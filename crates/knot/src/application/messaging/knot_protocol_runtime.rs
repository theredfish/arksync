// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::{collections::VecDeque, string::String, vec::Vec};
use arksync_event_router::{EventEnvelope, EventId, Timestamp};
use arksync_protocol::knot::{
    KnotAck, KnotCapabilities, KnotConfig, KnotConfigApplied, KnotConfigRejected,
    KnotControlMessage, KnotEnvelope, KnotHello, KnotMessage, KnotPayload, KnotRejectionReason,
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
    acknowledgements: VecDeque<KnotEnvelope>,
    non_confirmable: VecDeque<KnotEnvelope>,
    queue_capacity: usize,
    last_configuration: Option<(KnotEnvelope, KnotEnvelope)>,
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
            acknowledgements: VecDeque::new(),
            non_confirmable: VecDeque::new(),
            queue_capacity: retry_policy.capacity.max(1),
            last_configuration: None,
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
            KnotControlMessage::Hello(KnotHello {
                hardware_uid: self.hardware_uid.clone(),
                capabilities: self.capabilities,
                last_applied_config_version: self.config.as_ref().map(|config| config.version),
            })
            .into(),
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
        let envelope = self.envelope(event_id, occurred_at, message);
        match &envelope.payload {
            KnotMessage::Con(_) => self.outbox.enqueue(envelope, now_ms)?,
            KnotMessage::Non(KnotPayload::Sensor(_)) => {
                if self.non_confirmable.len() >= self.queue_capacity {
                    return Err(KnotOutboxError::Full.into());
                }
                if self
                    .non_confirmable
                    .iter()
                    .any(|pending| pending.id == event_id)
                {
                    return Err(KnotOutboxError::DuplicateEventId.into());
                }
                self.non_confirmable.push_back(envelope);
            }
            _ => return Err(KnotProtocolRuntimeError::UnexpectedMessage),
        }
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

        match &envelope.payload {
            KnotMessage::Ack(KnotAck::Processed { event_id }) => {
                if self.hello_id == Some(*event_id) {
                    return Err(KnotProtocolRuntimeError::UnexpectedMessage);
                }
                if self.outbox.acknowledge(*event_id) {
                    if self.heartbeat.is_some_and(|(id, _)| id == *event_id) {
                        self.heartbeat = None;
                    }
                    self.presence.observe(now_ms);
                }
            }
            KnotMessage::Ack(KnotAck::Hello { event_id, config }) => {
                if self.hello_id == Some(*event_id) {
                    self.apply_hello_config(
                        config.clone(),
                        envelope.id,
                        response_event_id,
                        occurred_at,
                        now_ms,
                        *event_id,
                    )?;
                    self.hello_id = None;
                    self.hub_id = Some(hub_id);
                    self.last_heartbeat_at_ms = now_ms;
                    self.presence.observe(now_ms);
                }
            }
            KnotMessage::Ack(KnotAck::Rejected { event_id, reason }) => {
                self.outbox.reject(*event_id, reason, now_ms);
            }
            KnotMessage::Ack(_) => return Err(KnotProtocolRuntimeError::UnexpectedMessage),
            KnotMessage::Con(KnotPayload::Control(KnotControlMessage::Configure(config))) => {
                if let Some((request, reply)) = &self.last_configuration {
                    if request.id == envelope.id {
                        if request == envelope {
                            self.queue_ack(reply.clone())?;
                            self.presence.observe(now_ms);
                        } else {
                            let reply = self.envelope(
                                response_event_id,
                                occurred_at,
                                KnotAck::Rejected {
                                    event_id: envelope.id,
                                    reason: KnotRejectionReason::ConfigurationConflict,
                                }
                                .into(),
                            );
                            self.queue_ack(reply)?;
                            return Err(KnotProtocolRuntimeError::UnexpectedMessage);
                        }
                        return Ok(());
                    }
                }
                let rejection = self.config_rejection(config);
                let ack = match rejection {
                    Some(reason) => KnotAck::ConfigRejected {
                        event_id: envelope.id,
                        config_version: config.version,
                        reason: reason.into(),
                    },
                    None => KnotAck::ConfigApplied {
                        event_id: envelope.id,
                        config_version: config.version,
                    },
                };
                let reply = self.envelope(response_event_id, occurred_at, ack.into());
                self.queue_ack(reply.clone())?;
                if rejection.is_none() {
                    self.config = Some(config.clone());
                }
                self.last_configuration = Some((envelope.clone(), reply));
                self.hub_id = Some(hub_id);
                self.presence.observe(now_ms);
            }
            KnotMessage::Con(_) => {
                let reply = self.envelope(
                    response_event_id,
                    occurred_at,
                    KnotAck::Rejected {
                        event_id: envelope.id,
                        reason: KnotRejectionReason::UnsupportedMessage,
                    }
                    .into(),
                );
                self.queue_ack(reply)?;
                return Err(KnotProtocolRuntimeError::UnexpectedMessage);
            }
            KnotMessage::Non(_) => return Err(KnotProtocolRuntimeError::UnexpectedMessage),
        }
        Ok(())
    }

    /// Convenience for runners that treat every returned envelope as successfully sent.
    pub fn due_messages(&mut self, now_ms: u64) -> Vec<KnotEnvelope> {
        let mut messages: Vec<_> = self.acknowledgements.drain(..).collect();
        messages.extend(self.outbox.due_messages(now_ms));
        if self.hub_id.is_some() {
            messages.extend(self.non_confirmable.drain(..));
        }
        messages
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
                KnotControlMessage::Heartbeat.into(),
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
        self.acknowledgements
            .front()
            .or_else(|| self.outbox.next_due_message(now_ms))
            .or_else(|| self.hub_id.and_then(|_| self.non_confirmable.front()))
    }

    pub fn mark_sent(&mut self, event_id: EventId, now_ms: u64) {
        if self
            .acknowledgements
            .front()
            .is_some_and(|event| event.id == event_id)
        {
            self.acknowledgements.pop_front();
        } else if self
            .non_confirmable
            .front()
            .is_some_and(|event| event.id == event_id)
        {
            self.non_confirmable.pop_front();
        } else {
            self.outbox.mark_sent(event_id, now_ms);
        }
    }

    pub fn can_publish(&self) -> bool {
        self.outbox.has_capacity()
    }

    pub fn can_publish_non(&self) -> bool {
        self.non_confirmable.len() < self.queue_capacity
    }

    pub fn pending_message_count(&self) -> usize {
        self.outbox.len() + self.acknowledgements.len() + self.non_confirmable.len()
    }

    pub fn outbox_overflow_count(&self) -> u64 {
        self.outbox.overflow_count()
    }

    fn queue_ack(&mut self, reply: KnotEnvelope) -> Result<(), KnotProtocolRuntimeError> {
        if self
            .acknowledgements
            .iter()
            .any(|pending| pending == &reply)
        {
            return Ok(());
        }
        if self.acknowledgements.len() >= self.queue_capacity {
            return Err(KnotOutboxError::Full.into());
        }
        self.acknowledgements.push_back(reply);
        Ok(())
    }

    fn config_rejection(&self, config: &KnotConfig) -> Option<&'static str> {
        if config.version == 0 {
            Some("Knot config version must be positive")
        } else if let Some(current) = &self.config {
            if current.knot_id != config.knot_id {
                Some("Knot config cannot change the assigned Knot identity")
            } else if current.version > config.version {
                Some("received stale Knot config version")
            } else if current.version == config.version && current != config {
                Some("Knot config changed without a new version")
            } else {
                None
            }
        } else {
            None
        }
    }

    fn apply_hello_config(
        &mut self,
        config: KnotConfig,
        config_event_id: EventId,
        event_id: EventId,
        occurred_at: Timestamp,
        now_ms: u64,
        request_id: EventId,
    ) -> Result<(), KnotProtocolRuntimeError> {
        let rejection = self.config_rejection(&config);
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

        let envelope = self.envelope(event_id, occurred_at, message.into());
        // Reserve the reliable follow-up before retiring the Hello carrying its configuration.
        self.outbox.replace(request_id, envelope, now_ms)?;
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
