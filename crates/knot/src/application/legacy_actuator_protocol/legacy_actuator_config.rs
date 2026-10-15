// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::{string::String, vec::Vec};
use arksync_actuator::application::protocol::ActuatorConfig;
use serde::{Deserialize, Serialize};

use crate::domain::KnotId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyKnotSensorBinding {
    pub sensor_id: String,
    pub device_uid: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegacyKnotActuatorConfig {
    pub hardware_uid: String,
    pub knot_id: KnotId,
    pub sensor_bindings: Vec<LegacyKnotSensorBinding>,
    pub actuator_configs: Vec<ActuatorConfig>,
}
