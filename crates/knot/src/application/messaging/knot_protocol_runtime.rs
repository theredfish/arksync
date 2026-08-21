// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::{string::String, vec::Vec};
use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_knot_protocol::{
    KnotAck, KnotCapabilities, KnotConfig, KnotConfigApplied, KnotConfigRejected,
    KnotControlMessage, KnotEnvelope, KnotHello, KnotMessage, KnotMessageSource,
};

use crate::application::{KnotOutbox, KnotOutboxError, RetryPolicy};

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
        }
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
        if !matches!(envelope.source, KnotMessageSource::Hub { .. }) {
            return Err(KnotProtocolRuntimeError::UnexpectedSource);
        }

        let KnotMessage::Control(message) = &envelope.payload else {
            return Err(KnotProtocolRuntimeError::UnexpectedMessage);
        };

        match message {
            KnotControlMessage::Ack(KnotAck::Processed { event_id }) => {
                self.outbox.acknowledge(*event_id);
            }
            KnotControlMessage::Ack(KnotAck::Hello { event_id, config }) => {
                if self.outbox.acknowledge(*event_id) {
                    self.apply_config(
                        config.clone(),
                        envelope.id,
                        response_event_id,
                        occurred_at,
                        now_ms,
                    )?;
                }
            }
            KnotControlMessage::Configure(config) => {
                self.apply_config(
                    config.clone(),
                    envelope.id,
                    response_event_id,
                    occurred_at,
                    now_ms,
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
    ) -> Result<(), KnotProtocolRuntimeError> {
        let message = if self
            .config
            .as_ref()
            .is_some_and(|current| current.version > config.version)
        {
            KnotControlMessage::ConfigRejected(KnotConfigRejected {
                event_id: config_event_id,
                config_version: config.version,
                reason: "received stale Knot config version".into(),
            })
        } else {
            let config_version = config.version;
            self.config = Some(config);
            KnotControlMessage::ConfigApplied(KnotConfigApplied {
                event_id: config_event_id,
                config_version,
            })
        };

        let envelope = self.envelope(event_id, occurred_at, KnotMessage::Control(message));
        self.outbox.enqueue(envelope, now_ms)?;
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
            KnotMessageSource::Knot {
                hardware_uid: self.hardware_uid.clone(),
            },
            occurred_at,
            payload,
        )
    }
}
