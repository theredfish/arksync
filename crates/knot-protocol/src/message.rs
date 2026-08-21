// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::string::String;
use arksync_bus::EventEnvelope;
use serde::{Deserialize, Serialize};

use crate::{KnotControlMessage, KnotSensorMessage};

pub type KnotEnvelope = EventEnvelope<KnotMessage, KnotMessageSource>;

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotMessageSource {
    Hub { hub_id: [u8; 16] },
    Knot { hardware_uid: String },
}
