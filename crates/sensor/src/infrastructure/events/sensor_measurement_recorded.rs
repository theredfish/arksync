// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::device_uid::DeviceUid;
use crate::infrastructure::events::{MeasuredSensor, SensorMeasurement};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SensorMeasurementRecorded {
    pub device_uid: DeviceUid,
    pub sensor: MeasuredSensor,
    pub measurement: SensorMeasurement,
}
