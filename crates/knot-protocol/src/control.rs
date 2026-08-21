// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::string::String;
use arksync_bus::EventId;
use serde::{Deserialize, Serialize};

use crate::KnotConfig;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnotControlMessage {
    Hello(KnotHello),
    Ack(KnotAck),
    Nack(KnotNack),
    ConfigApplied(KnotConfigApplied),
    ConfigRejected(KnotConfigRejected),
    Configure(KnotConfig),
}

impl KnotControlMessage {
    pub fn requires_ack(&self) -> bool {
        !matches!(self, Self::Ack(_) | Self::Nack(_))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotCapabilities {
    pub gpio: bool,
    pub uart: bool,
    pub i2c: bool,
    pub atlas_scientific_ezo: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotHello {
    pub hardware_uid: String,
    pub capabilities: KnotCapabilities,
    pub last_applied_config_version: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnotAck {
    Processed {
        event_id: EventId,
    },
    Hello {
        event_id: EventId,
        config: KnotConfig,
    },
}

impl KnotAck {
    pub fn event_id(&self) -> EventId {
        match self {
            Self::Processed { event_id } | Self::Hello { event_id, .. } => *event_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotNack {
    pub event_id: EventId,
    pub reason: KnotNackReason,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotNackReason {
    InvalidPayload,
    UnsupportedMessage,
    ConfigurationConflict,
    TemporarilyUnavailable,
}

impl KnotNackReason {
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::TemporarilyUnavailable)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotConfigApplied {
    pub event_id: EventId,
    pub config_version: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotConfigRejected {
    pub event_id: EventId,
    pub config_version: u64,
    pub reason: String,
}
