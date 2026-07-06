// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::string::String;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActuatorStateChanged {
    pub config_id: String,
    pub actuator_id: String,
    pub rule_id: String,
    pub sensor_id: String,
    pub sensor_value: f64,
    pub active: bool,
}
