// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_actuator::infrastructure::events::ActuatorEvent;
use serde::{Deserialize, Serialize};

use crate::application::{KnotConfig, KnotHello};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnotActuatorEvent {
    Hello(KnotHello),
    Ack(KnotConfig),
    Actuator(ActuatorEvent),
}
