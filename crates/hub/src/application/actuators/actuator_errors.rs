// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::infrastructure::store::{ActuatorStoreError, KnotStoreError, SensorStoreError};
use derive_more::From;
use std::fmt;

#[derive(Debug, From)]
pub enum HubActuatorError {
    KnotStore(KnotStoreError),
    ActuatorStore(ActuatorStoreError),
    SensorStore(SensorStoreError),
}

impl fmt::Display for HubActuatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HubActuatorError::KnotStore(err) => write!(f, "knot store error: {err:?}"),
            HubActuatorError::ActuatorStore(err) => write!(f, "actuator store error: {err}"),
            HubActuatorError::SensorStore(err) => write!(f, "sensor store error: {err}"),
        }
    }
}

impl std::error::Error for HubActuatorError {}
