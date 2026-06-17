// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::domain::KnotId;
use alloc::string::String;
use alloc::vec::Vec;
use arksync_actuator::infrastructure::events::ActuatorConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KnotSensorBinding {
    pub sensor_id: String,
    pub device_uid: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KnotConfig {
    pub hardware_uid: String,
    pub knot_id: KnotId,
    pub sensor_bindings: Vec<KnotSensorBinding>,
    pub actuator_configs: Vec<ActuatorConfig>,
}
