// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Serialize};

use super::{KnotAck, KnotControlMessage, KnotSensorMessage};
use crate::ArkSyncEnvelope;

/// Versioned message envelope exchanged between a Hub and a Knot.
pub type KnotEnvelope = ArkSyncEnvelope<KnotMessage>;

/// Delivery semantics, independent of the business message family.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnotMessage {
    /// Retransmitted until an ACK confirms processing or rejects the message.
    Con(KnotPayload),
    /// Sent once, without an ACK or a response.
    Non(KnotPayload),
    /// Never acknowledged. May carry the result of processing a CON.
    Ack(KnotAck),
}

impl KnotMessage {
    pub fn requires_ack(&self) -> bool {
        matches!(self, Self::Con(_))
    }

    pub fn payload(&self) -> Option<&KnotPayload> {
        match self {
            Self::Con(payload) | Self::Non(payload) => Some(payload),
            Self::Ack(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnotPayload {
    Control(KnotControlMessage),
    Sensor(KnotSensorMessage),
}

impl From<KnotControlMessage> for KnotMessage {
    fn from(message: KnotControlMessage) -> Self {
        Self::Con(KnotPayload::Control(message))
    }
}

impl From<KnotSensorMessage> for KnotMessage {
    fn from(message: KnotSensorMessage) -> Self {
        Self::Con(KnotPayload::Sensor(message))
    }
}

impl From<KnotAck> for KnotMessage {
    fn from(ack: KnotAck) -> Self {
        Self::Ack(ack)
    }
}
