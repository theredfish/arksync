// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Serialize};

use crate::infrastructure::events::{
    SensorMeasurementRecorded, SensorProvisioned, SensorProvisioningConflict, SerialSensorPlugged,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorEvent {
    SerialSensorPlugged(SerialSensorPlugged),
    SensorProvisioned(SensorProvisioned),
    SensorProvisioningConflict(SensorProvisioningConflict),
    SensorMeasurementRecorded(SensorMeasurementRecorded),
}
