// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::{Arc, Mutex};

use arksync_bus::{EventHandler, EventId, EventRouter, Timestamp};
use arksync_protocol::knot::{KnotConfig, KnotEnvelope, KnotMessage, KnotObservationMessage};
use arksync_protocol::ArkSyncActor;
use tokio::sync::{mpsc, watch, Notify};
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use super::runtime::{timestamp_now, TokioKnotRuntimeConfig};
use super::TokioMessageLink;
use crate::application::{
    ConnectionState, KnotObservationError, KnotObservationPublisher, KnotProtocolRuntime,
    KnotProtocolRuntimeError, MessageLink, DEFAULT_OBSERVATION_CAPACITY,
};

/// Latest local session state; configuration survives a link becoming unavailable.
#[derive(Clone, Debug, PartialEq)]
pub struct KnotSessionStatus {
    pub connection: ConnectionState,
    pub config: Option<KnotConfig>,
    /// Reliable messages awaiting acknowledgement; excludes coalesced observations.
    pub pending_messages: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotObservationPublishError {
    Closed,
    Invalid(KnotObservationError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotPublishFailure {
    Full,
    Closed,
    UnexpectedMessage,
    Observation(KnotObservationError),
}

/// Failed admission retains the original message for the caller.
#[derive(Debug)]
pub struct KnotPublishError {
    pub message: KnotMessage,
    pub reason: KnotPublishFailure,
}

impl core::fmt::Display for KnotPublishError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "Knot publication failed: {:?}", self.reason)
    }
}

impl std::error::Error for KnotPublishError {}

struct ObservationMailbox {
    publisher: KnotObservationPublisher,
    closed: bool,
}

/// Bounded application port for publishing messages and observing the session.
#[derive(Clone)]
pub struct KnotSessionHandle {
    observation_session_id: EventId,
    observations: Arc<Mutex<ObservationMailbox>>,
    observation_ready: Arc<Notify>,
    outgoing: mpsc::Sender<(Timestamp, KnotMessage)>,
    status: watch::Receiver<KnotSessionStatus>,
}

impl KnotSessionHandle {
    pub fn observation_session_id(&self) -> EventId {
        self.observation_session_id
    }

    /// Coalesce live updates before transport admission; no ACK or durable receipt.
    /// The caller retains the message on failure. Inventory must precede measurements.
    pub fn try_observe_at(
        &self,
        message: &KnotObservationMessage,
        occurred_at: Timestamp,
    ) -> Result<(), KnotObservationPublishError> {
        let mut observations = self
            .observations
            .lock()
            .expect("observation mailbox poisoned");
        if observations.closed || self.outgoing.is_closed() {
            return Err(KnotObservationPublishError::Closed);
        }
        observations
            .publisher
            .publish_at(message, occurred_at)
            .map_err(KnotObservationPublishError::Invalid)?;
        drop(observations);
        self.observation_ready.notify_one();
        Ok(())
    }

    /// Local acceptance only. Reliable messages wait for queue capacity; observations
    /// are validated and coalesced immediately through the live publisher.
    #[expect(
        clippy::result_large_err,
        reason = "Return ownership of unsent messages so the caller can retry without cloning"
    )]
    pub async fn publish(&self, message: KnotMessage) -> Result<(), KnotPublishError> {
        if !message.requires_ack() {
            return self.try_publish_at(message, timestamp_now());
        }
        self.outgoing
            .send((timestamp_now(), message))
            .await
            .map_err(|mpsc::error::SendError((_, message))| KnotPublishError {
                message,
                reason: KnotPublishFailure::Closed,
            })
    }

    #[expect(
        clippy::result_large_err,
        reason = "Return ownership of unsent messages so the caller can retry without cloning"
    )]
    pub fn try_publish_at(
        &self,
        message: KnotMessage,
        occurred_at: Timestamp,
    ) -> Result<(), KnotPublishError> {
        if let KnotMessage::Observation(observation) = &message {
            return self
                .try_observe_at(observation, occurred_at)
                .map_err(|error| KnotPublishError {
                    message,
                    reason: match error {
                        KnotObservationPublishError::Closed => KnotPublishFailure::Closed,
                        KnotObservationPublishError::Invalid(error) => {
                            KnotPublishFailure::Observation(error)
                        }
                    },
                });
        }
        if !message.requires_ack() {
            return Err(KnotPublishError {
                message,
                reason: KnotPublishFailure::UnexpectedMessage,
            });
        }
        self.outgoing
            .try_send((occurred_at, message))
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full((_, message)) => KnotPublishError {
                    message,
                    reason: KnotPublishFailure::Full,
                },
                mpsc::error::TrySendError::Closed((_, message)) => KnotPublishError {
                    message,
                    reason: KnotPublishFailure::Closed,
                },
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
        let observation_session_id = EventId::new();
        let observations = Arc::new(Mutex::new(ObservationMailbox {
            publisher: KnotObservationPublisher::new(
                observation_session_id,
                DEFAULT_OBSERVATION_CAPACITY,
            ),
            closed: false,
        }));
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
                observation_session_id,
                observations,
                observation_ready: Arc::new(Notify::new()),
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
        self.handle
            .observations
            .lock()
            .expect("observation mailbox poisoned")
            .closed = true;
        self.runtime.close();
        self.update_status();
        result
    }

    async fn run_session(
        &mut self,
        shutdown: CancellationToken,
    ) -> Result<(), KnotProtocolRuntimeError> {
        let started_at = Instant::now();
        self.runtime
            .start(self.handle.observation_session_id, timestamp_now(), 0)?;
        let mut tick = tokio::time::interval(self.tick_interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut previous_connection = ConnectionState::Connecting;
        loop {
            if shutdown.is_cancelled() {
                return Ok(());
            }
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
            let connection = self.runtime.connection_state();
            if connection == ConnectionState::Connected && previous_connection != connection {
                self.handle
                    .observations
                    .lock()
                    .expect("observation mailbox poisoned")
                    .publisher
                    .resynchronize();
            }
            previous_connection = connection;
            if !self.flush_observation(now_ms) {
                return Ok(());
            }
            self.update_status();

            tokio::select! {
                biased;
                _ = shutdown.cancelled() => return Ok(()),
                _ = tick.tick() => {
                    self.runtime.tick(EventId::new(), timestamp_now(), started_at.elapsed().as_millis() as u64)?;
                }
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
                _ = self.handle.observation_ready.notified() => {}
            }
        }
    }

    fn flush_observation(&mut self, now_ms: u64) -> bool {
        if self.runtime.connection_state() != ConnectionState::Connected {
            return true;
        }
        let mut mailbox = self
            .handle
            .observations
            .lock()
            .expect("observation mailbox poisoned");
        let Some((occurred_at, message)) = mailbox.publisher.next_message(now_ms) else {
            return true;
        };
        let Ok(envelope) =
            self.runtime
                .observation_envelope(message.clone(), EventId::new(), occurred_at)
        else {
            return true;
        };
        match self.link.try_send(envelope) {
            Ok(()) => mailbox.publisher.mark_sent(&message, now_ms),
            Err(mpsc::error::TrySendError::Full(_)) => {}
            Err(mpsc::error::TrySendError::Closed(_)) => return false,
        }
        true
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
