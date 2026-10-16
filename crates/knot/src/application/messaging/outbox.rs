// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::vec::Vec;
use arksync_bus::EventId;
use arksync_protocol::knot::{KnotEnvelope, KnotMessage, KnotNack};

pub const DEFAULT_OUTBOX_CAPACITY: usize = 256;
pub const DEFAULT_INITIAL_RETRY_DELAY_MS: u64 = 1_000;
pub const DEFAULT_MAX_RETRY_DELAY_MS: u64 = 30_000;

/// Capacity and exponential retry delays for a volatile Hub/Knot outbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub capacity: usize,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_delay_ms: DEFAULT_INITIAL_RETRY_DELAY_MS,
            max_delay_ms: DEFAULT_MAX_RETRY_DELAY_MS,
            capacity: DEFAULT_OUTBOX_CAPACITY,
        }
    }
}

/// Failure to enqueue a message that requires acknowledgement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotOutboxError {
    Full,
    DuplicateEventId,
}

#[derive(Clone, Debug)]
struct PendingMessage {
    envelope: KnotEnvelope,
    attempts: u32,
    next_attempt_at_ms: u64,
}

/// Bounded in-memory queue for envelopes awaiting peer confirmation.
pub struct KnotOutbox {
    pending: Vec<PendingMessage>,
    policy: RetryPolicy,
    overflow_count: u64,
}

impl KnotOutbox {
    pub fn new(policy: RetryPolicy) -> Self {
        Self {
            pending: Vec::new(),
            policy,
            overflow_count: 0,
        }
    }

    pub fn enqueue(&mut self, envelope: KnotEnvelope, now_ms: u64) -> Result<(), KnotOutboxError> {
        if self
            .pending
            .iter()
            .any(|pending| pending.envelope.id == envelope.id)
        {
            return Err(KnotOutboxError::DuplicateEventId);
        }
        if self.pending.len() >= self.policy.capacity {
            self.overflow_count = self.overflow_count.saturating_add(1);
            return Err(KnotOutboxError::Full);
        }

        self.pending.push(PendingMessage {
            envelope,
            attempts: 0,
            next_attempt_at_ms: now_ms,
        });

        Ok(())
    }

    pub fn due_messages(&mut self, now_ms: u64) -> Vec<KnotEnvelope> {
        let policy = self.policy;
        let mut messages: Vec<_> = self
            .pending
            .iter_mut()
            .filter_map(|pending| {
                if pending.next_attempt_at_ms > now_ms {
                    return None;
                }

                pending.attempts = pending.attempts.saturating_add(1);
                pending.next_attempt_at_ms =
                    now_ms.saturating_add(retry_delay_ms(policy, pending.attempts));

                Some(pending.envelope.clone())
            })
            .collect();
        messages.sort_by_key(|envelope| !matches!(envelope.payload, KnotMessage::Control(_)));
        messages
    }

    /// Peeks without advancing retry time. A full transport has not sent anything.
    pub fn next_due_message(&self, now_ms: u64) -> Option<&KnotEnvelope> {
        self.pending
            .iter()
            .filter(|pending| pending.next_attempt_at_ms <= now_ms)
            .min_by_key(|pending| !matches!(pending.envelope.payload, KnotMessage::Control(_)))
            .map(|pending| &pending.envelope)
    }

    pub fn mark_sent(&mut self, event_id: EventId, now_ms: u64) {
        if let Some(pending) = self.pending.iter_mut().find(|p| p.envelope.id == event_id) {
            pending.attempts = pending.attempts.saturating_add(1);
            pending.next_attempt_at_ms =
                now_ms.saturating_add(retry_delay_ms(self.policy, pending.attempts));
        }
    }

    /// Replaces a confirmed request with its follow-up without needing another slot.
    pub fn replace(
        &mut self,
        request_id: EventId,
        envelope: KnotEnvelope,
        now_ms: u64,
    ) -> Result<(), KnotOutboxError> {
        if self.pending.iter().any(|p| p.envelope.id == envelope.id) {
            return Err(KnotOutboxError::DuplicateEventId);
        }
        if let Some(pending) = self
            .pending
            .iter_mut()
            .find(|p| p.envelope.id == request_id)
        {
            *pending = PendingMessage {
                envelope,
                attempts: 0,
                next_attempt_at_ms: now_ms,
            };
            return Ok(());
        }
        self.enqueue(envelope, now_ms)
    }

    pub fn has_capacity(&self) -> bool {
        self.pending.len() < self.policy.capacity
    }

    pub fn contains_payload(&self, payload: &KnotMessage) -> bool {
        self.pending
            .iter()
            .any(|pending| pending.envelope.payload == *payload)
    }

    pub fn acknowledge(&mut self, event_id: EventId) -> bool {
        self.remove(event_id)
    }

    pub fn reject(&mut self, nack: &KnotNack, now_ms: u64) -> bool {
        if !nack.reason.is_retryable() {
            return self.remove(nack.event_id);
        }

        let Some(pending) = self
            .pending
            .iter_mut()
            .find(|pending| pending.envelope.id == nack.event_id)
        else {
            return false;
        };

        pending.next_attempt_at_ms = now_ms;
        true
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub fn overflow_count(&self) -> u64 {
        self.overflow_count
    }

    fn remove(&mut self, event_id: EventId) -> bool {
        let Some(index) = self
            .pending
            .iter()
            .position(|pending| pending.envelope.id == event_id)
        else {
            return false;
        };

        self.pending.remove(index);
        true
    }
}

fn retry_delay_ms(policy: RetryPolicy, attempts: u32) -> u64 {
    let exponent = attempts.saturating_sub(1).min(63);
    policy
        .initial_delay_ms
        .saturating_mul(1_u64 << exponent)
        .min(policy.max_delay_ms)
}
