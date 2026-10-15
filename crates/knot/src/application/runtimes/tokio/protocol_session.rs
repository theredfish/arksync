// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventHandler, EventId, EventRouter, Timestamp};
use arksync_protocol::knot::{KnotConfig, KnotEnvelope, KnotMessage};
use arksync_protocol::ArkSyncActor;
use tokio::sync::{mpsc, watch};
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use super::runtime::{timestamp_now, TokioKnotRuntimeConfig};
use super::TokioMessageLink;
use crate::application::{
    ConnectionState, KnotProtocolRuntime, KnotProtocolRuntimeError, MessageLink,
};

/// Latest local session state; configuration survives a link becoming unavailable.
#[derive(Clone, Debug, PartialEq)]
pub struct KnotSessionStatus {
    pub connection: ConnectionState,
    pub config: Option<KnotConfig>,
    pub pending_messages: usize,
}

/// Bounded application port for publishing messages and observing the session.
#[derive(Clone)]
pub struct KnotSessionHandle {
    outgoing: mpsc::Sender<(Timestamp, KnotMessage)>,
    status: watch::Receiver<KnotSessionStatus>,
}

impl KnotSessionHandle {
    /// Queueing is local acceptance, not acknowledgement by the Hub.
    #[expect(
        clippy::result_large_err,
        reason = "Return ownership of unsent messages so the caller can retry without cloning"
    )]
    pub async fn publish(
        &self,
        message: KnotMessage,
    ) -> Result<(), mpsc::error::SendError<KnotMessage>> {
        self.outgoing
            .send((timestamp_now(), message))
            .await
            .map_err(|mpsc::error::SendError((_, message))| mpsc::error::SendError(message))
    }

    #[expect(
        clippy::result_large_err,
        reason = "Return ownership of unsent messages so the caller can retry without cloning"
    )]
    pub fn try_publish_at(
        &self,
        message: KnotMessage,
        occurred_at: Timestamp,
    ) -> Result<(), mpsc::error::TrySendError<KnotMessage>> {
        self.outgoing
            .try_send((occurred_at, message))
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full((_, message)) => {
                    mpsc::error::TrySendError::Full(message)
                }
                mpsc::error::TrySendError::Closed((_, message)) => {
                    mpsc::error::TrySendError::Closed(message)
                }
            })
    }

    pub fn status(&self) -> watch::Receiver<KnotSessionStatus> {
        self.status.clone()
    }
}

/// Runs the portable protocol on Tokio without starting sensor or actuator drivers.
///
/// Both directions keep progressing when the bounded link is full. Subscribers
/// observe accepted inbound messages; hardware services have a separate lifetime.
pub struct TokioKnotSession {
    runtime: KnotProtocolRuntime,
    link: TokioMessageLink<KnotEnvelope>,
    outgoing: mpsc::Receiver<(Timestamp, KnotMessage)>,
    handle: KnotSessionHandle,
    status: watch::Sender<KnotSessionStatus>,
    events: EventRouter<KnotMessage, ArkSyncActor>,
    tick_interval: Duration,
}

impl TokioKnotSession {
    pub fn new(config: TokioKnotRuntimeConfig, link: TokioMessageLink<KnotEnvelope>) -> Self {
        let (outgoing, receiver) = mpsc::channel(config.retry_policy.capacity.max(1));
        let (status, status_rx) = watch::channel(KnotSessionStatus {
            connection: ConnectionState::Connecting,
            config: None,
            pending_messages: 0,
        });
        Self {
            tick_interval: Duration::from_millis(config.keep_alive_config.interval_ms().min(100)),
            runtime: KnotProtocolRuntime::new(
                config.hardware_uid,
                config.capabilities,
                config.retry_policy,
            )
            .with_keep_alive_config(config.keep_alive_config),
            link,
            outgoing: receiver,
            handle: KnotSessionHandle {
                outgoing,
                status: status_rx,
            },
            status,
            events: EventRouter::new(),
        }
    }

    pub fn handle(&self) -> KnotSessionHandle {
        self.handle.clone()
    }

    pub fn subscribe(
        &mut self,
        handler: impl EventHandler<KnotMessage, ArkSyncActor> + Send + 'static,
    ) {
        self.events.subscribe(handler);
    }

    pub async fn run(
        mut self,
        shutdown: CancellationToken,
    ) -> Result<(), KnotProtocolRuntimeError> {
        let result = self.run_session(shutdown).await;
        self.runtime.close();
        self.update_status();
        result
    }

    async fn run_session(
        &mut self,
        shutdown: CancellationToken,
    ) -> Result<(), KnotProtocolRuntimeError> {
        let started_at = Instant::now();
        self.runtime.start(EventId::new(), timestamp_now(), 0)?;
        let mut tick = tokio::time::interval(self.tick_interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            let now_ms = started_at.elapsed().as_millis() as u64;
            // Bound each send pass even if retries have a zero delay.
            for _ in 0..self.runtime.pending_message_count() {
                let Some(message) = self.runtime.next_due_message(now_ms).cloned() else {
                    break;
                };
                let id = message.id;
                match self.link.try_send(message) {
                    Ok(()) => self.runtime.mark_sent(id, now_ms),
                    Err(mpsc::error::TrySendError::Full(_)) => break,
                    Err(mpsc::error::TrySendError::Closed(_)) => return Ok(()),
                }
            }
            self.update_status();

            tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                incoming = self.link.receive() => {
                    let Ok(Some(message)) = incoming else { return Ok(()); };
                    let now_ms = started_at.elapsed().as_millis() as u64;
                    match self.runtime.receive(&message, EventId::new(), timestamp_now(), now_ms) {
                        Ok(()) => {
                            let report = self.events.publish(&message);
                            if report.rejected > 0 {
                                log::warn!("Knot session observer rejected message {}", message.id);
                            }
                        }
                        Err(error) => log::warn!("Knot rejected protocol message {}: {error:?}", message.id),
                    }
                }
                Some((occurred_at, message)) = self.outgoing.recv(), if self.runtime.can_publish() => {
                    self.runtime.publish(message, EventId::new(), occurred_at, started_at.elapsed().as_millis() as u64)?;
                }
                _ = tick.tick() => {
                    self.runtime.tick(EventId::new(), timestamp_now(), started_at.elapsed().as_millis() as u64)?;
                }
            }
        }
    }

    fn update_status(&self) {
        let next = KnotSessionStatus {
            connection: self.runtime.connection_state(),
            config: self.runtime.config().cloned(),
            pending_messages: self.runtime.pending_message_count(),
        };
        self.status.send_if_modified(|current| {
            if *current == next {
                return false;
            }
            *current = next;
            true
        });
    }
}
