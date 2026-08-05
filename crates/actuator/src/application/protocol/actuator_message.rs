// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Serialize};

use crate::application::protocol::{
    ActuatorStateChanged, AddActuator, ConfigApplied, ConfigRejected, DisableActuator,
    EnableActuator, RemoveActuator, RuntimeStatus,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActuatorMessage {
    AddActuator(AddActuator),
    EnableActuator(EnableActuator),
    DisableActuator(DisableActuator),
    RemoveActuator(RemoveActuator),
    ConfigApplied(ConfigApplied),
    ConfigRejected(ConfigRejected),
    RuntimeStatus(RuntimeStatus),
    ActuatorStateChanged(ActuatorStateChanged),
}
