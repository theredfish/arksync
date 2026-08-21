// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::string::String;
use arksync_bus::EventEnvelope;
use serde::{Deserialize, Serialize};

use crate::{KnotControlMessage, KnotSensorMessage};

/// Versioned message envelope exchanged between a Hub and a Knot.
pub type KnotEnvelope = EventEnvelope<KnotMessage, KnotMessageSource>;

/// Root payload for the bilateral Hub/Knot protocol.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnotMessage {
    Control(KnotControlMessage),
    Sensor(KnotSensorMessage),
}

impl KnotMessage {
    /// Whether successful processing of this message must be acknowledged.
    pub fn requires_ack(&self) -> bool {
        match self {
            Self::Control(message) => message.requires_ack(),
            Self::Sensor(_) => true,
        }
    }
}

/// Claimed origin of a protocol message.
///
/// A remote transport authenticates its peer independently. The source remains
/// useful for routing, auditing, and in-process links.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotMessageSource {
    Hub { hub_id: [u8; 16] },
    Knot { hardware_uid: String },
}
