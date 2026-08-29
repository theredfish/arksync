// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_actuator::application::protocol::ActuatorMessage;
use arksync_bus::EventEnvelope;
use serde::{Deserialize, Serialize};

use crate::application::LegacyKnotActuatorConfig;

pub type LegacyKnotActuatorEnvelope = EventEnvelope<LegacyKnotActuatorMessage>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LegacyKnotActuatorMessage {
    ApplyConfig(LegacyKnotActuatorConfig),
    Actuator(ActuatorMessage),
}
