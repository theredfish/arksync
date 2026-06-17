// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_actuator::infrastructure::events::ActuatorEvent;
use arksync_bus::EventEnvelope;
use serde::{Deserialize, Serialize};

use super::{ack::KnotAck, hello::KnotHello};

pub type KnotMessageEnvelope = EventEnvelope<KnotMessage>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnotMessage {
    Hello(KnotHello),
    Ack(KnotAck),
    Actuator(ActuatorEvent),
}
