// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::{string::String, vec::Vec};
use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_protocol::knot::{
    KnotAck, KnotCapabilities, KnotConfig, KnotConfigApplied, KnotConfigRejected,
    KnotControlMessage, KnotEnvelope, KnotHello, KnotMessage,
};
use arksync_protocol::ArkSyncActor;

use crate::application::{
    ConnectionState, KeepAliveConfig, KnotOutbox, KnotOutboxError, PeerPresence, RetryPolicy,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotProtocolRuntimeError {
    Outbox(KnotOutboxError),
    UnexpectedSource,
    UnexpectedMessage,
}

impl From<KnotOutboxError> for KnotProtocolRuntimeError {
    fn from(value: KnotOutboxError) -> Self {
        Self::Outbox(value)
    }
}

/// Portable Hub/Knot messaging state machine.
///
/// The runtime has no executor, clock, channel, socket, or database dependency.
/// A concrete runner supplies timestamps and EventIds, sends due envelopes, and
/// passes received envelopes back into this state machine.
pub struct KnotProtocolRuntime {
    hardware_uid: String,
    capabilities: KnotCapabilities,
    config: Option<KnotConfig>,
    outbox: KnotOutbox,
    hello_id: Option<EventId>,
    hub_id: Option<[u8; 16]>,
    heartbeat: Option<(EventId, u64)>,
    last_heartbeat_at_ms: u64,
    presence: PeerPresence,
    keep_alive_config: KeepAliveConfig,
}

impl KnotProtocolRuntime {
    pub fn new(
        hardware_uid: String,
        capabilities: KnotCapabilities,
        retry_policy: RetryPolicy,
    ) -> Self {
        Self {
            hardware_uid,
            capabilities,
            config: None,
            outbox: KnotOutbox::new(retry_policy),
            hello_id: None,
            hub_id: None,
            heartbeat: None,
            last_heartbeat_at_ms: 0,
            presence: PeerPresence::default(),
            keep_alive_config: KeepAliveConfig::default(),
        }
    }

    pub fn with_keep_alive_config(mut self, config: KeepAliveConfig) -> Self {
        self.keep_alive_config = config;
        self
    }

    pub fn start(
        &mut self,
        event_id: EventId,
        occurred_at: Timestamp,
        now_ms: u64,
    ) -> Result<(), KnotProtocolRuntimeError> {
        let envelope = self.envelope(
            event_id,
            occurred_at,
            KnotMessage::Control(KnotControlMessage::Hello(KnotHello {
                hardware_uid: self.hardware_uid.clone(),
                capabilities: self.capabilities,
                last_applied_config_version: self.config.as_ref().map(|config| config.version),
            })),
        );

        self.outbox.enqueue(envelope, now_ms)?;
        self.hello_id = Some(event_id);
        self.presence = PeerPresence::new(now_ms);
        Ok(())
    }

    pub fn publish(
        &mut self,
        message: KnotMessage,
        event_id: EventId,
        occurred_at: Timestamp,
        now_ms: u64,
    ) -> Result<(), KnotProtocolRuntimeError> {
        if !message.requires_ack() {
            return Err(KnotProtocolRuntimeError::UnexpectedMessage);
        }

        let envelope = self.envelope(event_id, occurred_at, message);
        self.outbox.enqueue(envelope, now_ms)?;
        Ok(())
    }

    pub fn receive(
        &mut self,
        envelope: &KnotEnvelope,
        response_event_id: EventId,
        occurred_at: Timestamp,
        now_ms: u64,
    ) -> Result<(), KnotProtocolRuntimeError> {
        if self.presence.state() == ConnectionState::Closed {
            return Err(KnotProtocolRuntimeError::UnexpectedMessage);
        }
        let ArkSyncActor::Hub { hub_id } = envelope.source else {
            return Err(KnotProtocolRuntimeError::UnexpectedSource);
        };
        if self.hub_id.is_some_and(|expected| expected != hub_id) {
            return Err(KnotProtocolRuntimeError::UnexpectedSource);
        }

        let KnotMessage::Control(message) = &envelope.payload else {
            return Err(KnotProtocolRuntimeError::UnexpectedMessage);
        };

        match message {
            KnotControlMessage::Ack(KnotAck::Processed { event_id }) => {
                if self.hello_id == Some(*event_id)
                    || self.heartbeat.is_some_and(|(id, _)| id == *event_id)
                {
                    return Err(KnotProtocolRuntimeError::UnexpectedMessage);
                }
                if self.outbox.acknowledge(*event_id) {
                    self.presence.observe(now_ms);
                }
            }
            KnotControlMessage::Ack(KnotAck::Hello { event_id, config }) => {
                if self.hello_id == Some(*event_id) {
                    self.apply_config(
                        config.clone(),
                        envelope.id,
                        response_event_id,
                        occurred_at,
                        now_ms,
                        Some(*event_id),
                    )?;
                    self.hello_id = None;
                    self.hub_id = Some(hub_id);
                    self.last_heartbeat_at_ms = now_ms;
                    self.presence.observe(now_ms);
                }
            }
            KnotControlMessage::Ack(KnotAck::Heartbeat { event_id }) => {
                if self.heartbeat.is_some_and(|(id, _)| id == *event_id)
                    && self.outbox.acknowledge(*event_id)
                {
                    self.heartbeat = None;
                    self.presence.observe(now_ms);
                }
            }
            KnotControlMessage::Configure(config) => {
                self.apply_config(
                    config.clone(),
                    envelope.id,
                    response_event_id,
                    occurred_at,
                    now_ms,
                    None,
                )?;
            }
            KnotControlMessage::Nack(nack) => {
                self.outbox.reject(nack, now_ms);
            }
            _ => return Err(KnotProtocolRuntimeError::UnexpectedMessage),
        }

        Ok(())
    }

    pub fn due_messages(&mut self, now_ms: u64) -> Vec<KnotEnvelope> {
        self.outbox.due_messages(now_ms)
    }

    pub fn config(&self) -> Option<&KnotConfig> {
        self.config.as_ref()
    }

    pub fn connection_state(&self) -> ConnectionState {
        self.presence.state()
    }

    pub fn close(&mut self) {
        self.presence.close();
    }

    /// Maintains one bounded presence probe after the Hello handshake.
    pub fn tick(
        &mut self,
        event_id: EventId,
        occurred_at: Timestamp,
        now_ms: u64,
    ) -> Result<(), KnotProtocolRuntimeError> {
        if self.presence.state() == ConnectionState::Closed {
            return Ok(());
        }
        self.presence.expire(now_ms, self.keep_alive_config);
        if let Some((id, started_at)) = self.heartbeat {
            if now_ms.saturating_sub(started_at) >= self.keep_alive_config.timeout_ms() {
                self.outbox.acknowledge(id);
                self.heartbeat = None;
            }
        }
        if self.hub_id.is_some()
            && self.heartbeat.is_none()
            && now_ms.saturating_sub(self.last_heartbeat_at_ms)
                >= self.keep_alive_config.interval_ms()
            && self.outbox.has_capacity()
        {
            self.publish(
                KnotMessage::Control(KnotControlMessage::Heartbeat),
                event_id,
                occurred_at,
                now_ms,
            )?;
            self.heartbeat = Some((event_id, now_ms));
            self.last_heartbeat_at_ms = now_ms;
        }
        Ok(())
    }

    pub fn next_due_message(&self, now_ms: u64) -> Option<&KnotEnvelope> {
        self.outbox.next_due_message(now_ms)
    }

    pub fn mark_sent(&mut self, event_id: EventId, now_ms: u64) {
        self.outbox.mark_sent(event_id, now_ms);
    }

    pub fn can_publish(&self) -> bool {
        self.outbox.has_capacity()
    }

    pub fn pending_message_count(&self) -> usize {
        self.outbox.len()
    }

    pub fn outbox_overflow_count(&self) -> u64 {
        self.outbox.overflow_count()
    }

    fn apply_config(
        &mut self,
        config: KnotConfig,
        config_event_id: EventId,
        event_id: EventId,
        occurred_at: Timestamp,
        now_ms: u64,
        replaces: Option<EventId>,
    ) -> Result<(), KnotProtocolRuntimeError> {
        let rejection = if config.version == 0 {
            Some("Knot config version must be positive")
        } else if let Some(current) = &self.config {
            if current.knot_id != config.knot_id {
                Some("Knot config cannot change the assigned Knot identity")
            } else if current.version > config.version {
                Some("received stale Knot config version")
            } else if current.version == config.version && current != &config {
                Some("Knot config changed without a new version")
            } else {
                None
            }
        } else {
            None
        };
        let message = if let Some(reason) = rejection {
            KnotControlMessage::ConfigRejected(KnotConfigRejected {
                event_id: config_event_id,
                config_version: config.version,
                reason: reason.into(),
            })
        } else {
            let config_version = config.version;
            KnotControlMessage::ConfigApplied(KnotConfigApplied {
                event_id: config_event_id,
                config_version,
            })
        };

        let payload = KnotMessage::Control(message);
        // Retransmitting a command must not consume another confirmation slot.
        if replaces.is_none() && self.outbox.contains_payload(&payload) {
            return Ok(());
        }
        let envelope = self.envelope(event_id, occurred_at, payload);
        if let Some(request_id) = replaces {
            self.outbox.replace(request_id, envelope, now_ms)?;
        } else {
            self.outbox.enqueue(envelope, now_ms)?;
        }
        if rejection.is_none() {
            self.config = Some(config);
        }
        Ok(())
    }

    fn envelope(
        &self,
        event_id: EventId,
        occurred_at: Timestamp,
        payload: KnotMessage,
    ) -> KnotEnvelope {
        EventEnvelope::new_with_id(
            event_id,
            ArkSyncActor::Knot {
                hardware_uid: self.hardware_uid.clone(),
            },
            occurred_at,
            payload,
        )
    }
}
