// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Serialize};

use super::{KnotControlMessage, KnotSensorMessage};
use crate::ArkSyncEnvelope;

/// Versioned message envelope exchanged between a Hub and a Knot.
pub type KnotEnvelope = ArkSyncEnvelope<KnotMessage>;

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
