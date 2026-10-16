// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use arksync_bus::{EventHandler, EventId, EventRouter, Timestamp};
use arksync_knot::application::{
    ConnectionState, KeepAliveConfig, KnotOutbox, MessageLink, PeerPresence, RetryPolicy,
    TokioMessageLink,
};
use arksync_knot::domain::KnotEventSource;
use arksync_protocol::knot::{KnotAck, KnotControlMessage, KnotEnvelope, KnotMessage};
use arksync_protocol::ArkSyncActor;
use arksync_sensor::infrastructure::events::SensorEvent;
use eyre::Result;
use sqlx::PgPool;
use tokio::sync::{mpsc, watch};
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::application::{handle_knot_protocol_event, SensorRegistry};
use crate::domain::HubId;
use crate::infrastructure::store::knot::insert_knot_config_delivery;

const PENDING_RESPONSE_CAPACITY: usize = 100;
const DATABASE_OPERATION_TIMEOUT: Duration = Duration::from_secs(5);

/// Live state of a single peer, separate from the Knot record in PostgreSQL.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HubSessionStatus {
    pub connection: ConnectionState,
    pub hardware_uid: Option<String>,
    pub knot_id: Option<[u8; 16]>,
    pub last_received_at: Option<Timestamp>,
}

#[derive(Clone)]
pub struct HubSessionHandle {
    hub_id: HubId,
    pool: PgPool,
    outgoing: mpsc::Sender<KnotEnvelope>,
    status: watch::Receiver<HubSessionStatus>,
}

impl HubSessionHandle {
    /// Register and queue a Configure command for retransmission until confirmed.
    /// Success accepts it locally; it does not confirm application by the Knot.
    pub async fn send(&self, message: KnotEnvelope) -> Result<()> {
        let KnotMessage::Control(KnotControlMessage::Configure(config)) = &message.payload else {
            return Err(eyre::eyre!(
                "Hub session commands must be Configure messages"
            ));
        };
        if message.source
            != (ArkSyncActor::Hub {
                hub_id: *self.hub_id.as_bytes(),
            })
            || self.status.borrow().knot_id != Some(config.knot_id)
        {
            return Err(eyre::eyre!(
                "configuration does not belong to the established session"
            ));
        }
        let permit =
            self.outgoing.reserve().await.map_err(|_| {
                eyre::eyre!("Hub session closed before configuration could be queued")
            })?;
        let registered = tokio::time::timeout(
            DATABASE_OPERATION_TIMEOUT,
            insert_knot_config_delivery(&self.pool, self.hub_id, message.id, config),
        )
        .await??;
        if !registered {
            return Err(eyre::eyre!(
                "configuration version or event ID conflicts with stored state"
            ));
        }
        permit.send(message);
        Ok(())
    }

    pub fn status(&self) -> watch::Receiver<HubSessionStatus> {
        self.status.clone()
    }
}

/// Hub-side session with one Knot, using the production transactional handlers.
///
/// Registration is durable; liveness and subscriptions belong to this session.
/// Accepted protocol messages and committed sensor projections are routed to
/// subscribers. Sends do not await transport capacity; reception is paused when
/// the bounded response queue fills, while cancellation and expiry remain active.
pub struct HubSession {
    hub_id: HubId,
    pool: PgPool,
    link: TokioMessageLink<KnotEnvelope>,
    sensors: SensorRegistry,
    presence: PeerPresence,
    keep_alive_config: KeepAliveConfig,
    status: watch::Sender<HubSessionStatus>,
    handle: HubSessionHandle,
    outgoing: mpsc::Receiver<KnotEnvelope>,
    events: EventRouter<KnotMessage, ArkSyncActor>,
    sensor_events: EventRouter<SensorEvent, KnotEventSource>,
}

impl HubSession {
    pub async fn new(
        pool: PgPool,
        hub_id: HubId,
        link: TokioMessageLink<KnotEnvelope>,
        keep_alive_config: KeepAliveConfig,
    ) -> Result<Self> {
        let sensors = SensorRegistry::load(&pool, hub_id).await?;
        let (outgoing, receiver) = mpsc::channel(PENDING_RESPONSE_CAPACITY);
        let (status, status_rx) = watch::channel(HubSessionStatus::default());
        Ok(Self {
            hub_id,
            pool: pool.clone(),
            link,
            sensors,
            keep_alive_config,
            presence: PeerPresence::default(),
            status,
            handle: HubSessionHandle {
                hub_id,
                pool,
                outgoing,
                status: status_rx,
            },
            outgoing: receiver,
            events: EventRouter::new(),
            sensor_events: EventRouter::new(),
        })
    }

    pub fn handle(&self) -> HubSessionHandle {
        self.handle.clone()
    }

    pub fn subscribe(
        &mut self,
        handler: impl EventHandler<KnotMessage, ArkSyncActor> + Send + 'static,
    ) {
        self.events.subscribe(handler);
    }

    pub fn subscribe_sensors(
        &mut self,
        handler: impl EventHandler<SensorEvent, KnotEventSource> + Send + 'static,
    ) {
        self.sensor_events.subscribe(handler);
    }

    pub async fn run(mut self, shutdown: CancellationToken) {
        self.run_session(shutdown).await;
        self.presence.close();
        self.update_connection();
    }

    async fn run_session(&mut self, shutdown: CancellationToken) {
        let started_at = Instant::now();
        let mut tick = tokio::time::interval(Duration::from_millis(
            self.keep_alive_config.interval_ms().min(100),
        ));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut pending = VecDeque::new();
        let mut configurations = KnotOutbox::new(RetryPolicy {
            capacity: PENDING_RESPONSE_CAPACITY,
            ..RetryPolicy::default()
        });

        loop {
            if shutdown.is_cancelled() {
                return;
            }
            while let Some(message) = pending.pop_front() {
                match self.link.try_send(message) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(message)) => {
                        pending.push_front(message);
                        break;
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => return,
                }
            }
            // ACKs have priority so a full Knot outbox can drain before Configure retries.
            if pending.is_empty() {
                let now_ms = started_at.elapsed().as_millis() as u64;
                for _ in 0..configurations.len() {
                    let Some(message) = configurations.next_due_message(now_ms).cloned() else {
                        break;
                    };
                    let id = message.id;
                    match self.link.try_send(message) {
                        Ok(()) => configurations.mark_sent(id, now_ms),
                        Err(mpsc::error::TrySendError::Full(_)) => break,
                        Err(mpsc::error::TrySendError::Closed(_)) => return,
                    }
                }
            }
            tokio::select! {
                _ = shutdown.cancelled() => return,
                incoming = self.link.receive(), if pending.len() < PENDING_RESPONSE_CAPACITY => {
                    let Ok(Some(message)) = incoming else { return; };
                    if let Some(response) = self.process_message(&message, started_at, &shutdown, &mut tick).await {
                        if matches!(response.payload, KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Processed { .. }))) {
                            match &message.payload {
                                KnotMessage::Control(KnotControlMessage::ConfigApplied(result)) => { configurations.acknowledge(result.event_id); }
                                KnotMessage::Control(KnotControlMessage::ConfigRejected(result)) => { configurations.acknowledge(result.event_id); }
                                _ => {}
                            }
                        }
                        pending.push_back(response);
                    }
                }
                Some(message) = self.outgoing.recv(), if configurations.has_capacity() => {
                    if let Err(error) = configurations.enqueue(message, started_at.elapsed().as_millis() as u64) {
                        log::warn!("Hub configuration already queued: {error:?}");
                    }
                }
                _ = tick.tick() => {
                    self.presence.expire(started_at.elapsed().as_millis() as u64, self.keep_alive_config);
                    self.update_connection();
                }
            }
        }
    }

    async fn process_message(
        &mut self,
        message: &KnotEnvelope,
        started_at: Instant,
        shutdown: &CancellationToken,
        tick: &mut tokio::time::Interval,
    ) -> Option<KnotEnvelope> {
        if !self.accepts_source(message) {
            log::warn!(
                "Hub ignored message {} outside the established Knot session",
                message.id
            );
            return None;
        }
        let received_at = timestamp_now();
        let received_at_ms = started_at.elapsed().as_millis() as u64;
        let operation = tokio::time::timeout(
            DATABASE_OPERATION_TIMEOUT,
            handle_knot_protocol_event(
                &self.pool,
                self.hub_id,
                message,
                EventId::new(),
                timestamp_now(),
                received_at,
                &mut self.sensors,
            ),
        );
        let result = {
            tokio::pin!(operation);
            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => return None,
                    result = &mut operation => break result,
                    _ = tick.tick() => {
                        self.presence.expire(started_at.elapsed().as_millis() as u64, self.keep_alive_config);
                        update_connection(&self.status, &self.presence);
                    }
                }
            }
        };
        let result = match result {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => {
                log::error!("Hub could not process message {}: {error:?}", message.id);
                return None;
            }
            Err(_) => {
                log::warn!(
                    "Hub database operation timed out for message {}",
                    message.id
                );
                return None;
            }
        };
        if let Some(KnotMessage::Control(KnotControlMessage::Ack(ack))) =
            result.response.as_ref().map(|r| &r.payload)
        {
            if let KnotAck::Hello { config, .. } = ack {
                let ArkSyncActor::Knot { hardware_uid } = &message.source else {
                    unreachable!()
                };
                self.status.send_modify(|status| {
                    status.hardware_uid = Some(hardware_uid.clone());
                    status.knot_id = Some(config.knot_id);
                });
            }
            self.presence.observe(received_at_ms);
            self.presence.expire(
                started_at.elapsed().as_millis() as u64,
                self.keep_alive_config,
            );
            self.status
                .send_modify(|status| status.last_received_at = Some(received_at));
            self.update_connection();
            if self.events.publish(message).rejected > 0 {
                log::warn!("Hub session observer rejected message {}", message.id);
            }
        }
        for event in result.sensor_events {
            if self.sensor_events.publish(&event).rejected > 0 {
                log::warn!("Hub sensor observer rejected message {}", event.id);
            }
        }
        result.response
    }

    fn accepts_source(&self, message: &KnotEnvelope) -> bool {
        let ArkSyncActor::Knot { hardware_uid } = &message.source else {
            return false;
        };
        match &self.status.borrow().hardware_uid {
            Some(expected) => expected == hardware_uid,
            None => {
                matches!(&message.payload, KnotMessage::Control(KnotControlMessage::Hello(hello)) if hello.hardware_uid == *hardware_uid)
            }
        }
    }

    fn update_connection(&self) {
        update_connection(&self.status, &self.presence);
    }
}

fn update_connection(status: &watch::Sender<HubSessionStatus>, presence: &PeerPresence) {
    status.send_if_modified(|status| {
        let next = presence.state();
        if status.connection == next {
            return false;
        }
        status.connection = next;
        true
    });
}

fn timestamp_now() -> Timestamp {
    Timestamp::from_unix_millis(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default(),
    )
}
