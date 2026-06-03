// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::infrastructure::store::{SensorMeasurementStoreError, SensorStoreError};
use derive_more::From;
use std::fmt;

#[derive(Debug, From)]
pub enum HubSensorError {
    SensorStore(SensorStoreError),
    SensorMeasurementStore(SensorMeasurementStoreError),
}

impl fmt::Display for HubSensorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HubSensorError::SensorStore(err) => write!(f, "sensor store error: {err}"),
            HubSensorError::SensorMeasurementStore(err) => {
                write!(f, "sensor measurement store error: {err}")
            }
        }
    }
}

impl std::error::Error for HubSensorError {}
