// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::string::String;
use arksync_event_router::EventId;
use serde::{Deserialize, Serialize};

use super::KnotConfig;

/// Control-plane payloads used for handshake, configuration, and presence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnotControlMessage {
    Hello(KnotHello),
    ConfigApplied(KnotConfigApplied),
    ConfigRejected(KnotConfigRejected),
    Configure(KnotConfig),
    /// Presence probe. Its acknowledgement is correlated but not persisted.
    Heartbeat,
}

/// Hardware interfaces supported by a Knot runtime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotCapabilities {
    pub gpio: bool,
    pub uart: bool,
    pub i2c: bool,
    pub atlas_scientific_ezo: bool,
}

/// Presence announcement sent by a Knot when its protocol runtime starts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotHello {
    pub hardware_uid: String,
    pub capabilities: KnotCapabilities,
    pub last_applied_config_version: Option<u64>,
}

/// Processing acknowledgement, with an optional business result, correlated to the processed message's [`EventId`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnotAck {
    Processed {
        event_id: EventId,
    },
    Hello {
        event_id: EventId,
        config: KnotConfig,
    },
    ConfigApplied {
        event_id: EventId,
        config_version: u64,
    },
    ConfigRejected {
        event_id: EventId,
        config_version: u64,
        reason: String,
    },
    Rejected {
        event_id: EventId,
        reason: KnotRejectionReason,
    },
}

impl KnotAck {
    pub fn event_id(&self) -> EventId {
        match self {
            Self::Processed { event_id }
            | Self::Hello { event_id, .. }
            | Self::ConfigApplied { event_id, .. }
            | Self::ConfigRejected { event_id, .. }
            | Self::Rejected { event_id, .. } => *event_id,
        }
    }
}

/// Stable reason that determines whether a rejected message can be retried unchanged.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotRejectionReason {
    InvalidPayload,
    UnsupportedMessage,
    ConfigurationConflict,
    TemporarilyUnavailable,
}

impl KnotRejectionReason {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::TemporarilyUnavailable)
    }
}

/// Confirmation that a Knot accepted and loaded a Hub configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotConfigApplied {
    pub event_id: EventId,
    pub config_version: u64,
}

/// Confirmation that a Knot could not load a Hub configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotConfigRejected {
    pub event_id: EventId,
    pub config_version: u64,
    pub reason: String,
}
