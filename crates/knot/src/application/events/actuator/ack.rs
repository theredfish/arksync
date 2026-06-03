// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_actuator::infrastructure::events::ActuatorConfig;
use serde::{Deserialize, Serialize};
use std::string::String;
use std::vec::Vec;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KnotConfig {
    pub hardware_uid: String,
    pub knot_id: crate::domain::KnotId,
    pub actuator_configs: Vec<ActuatorConfig>,
}
