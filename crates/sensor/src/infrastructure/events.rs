// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Published sensor events consumed by runtime adapters.

use crate::device_uid::DeviceUid;
use crate::serial_port::SerialPortMetadata;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorEvent {
    SerialSensorPlugged(SerialSensorPlugged),
    SensorProvisioned(SensorProvisioned),
    SensorProvisioningConflict(SensorProvisioningConflict),
    SensorMeasurementRecorded(SensorMeasurementRecorded),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SerialSensorPlugged {
    pub metadata: SerialPortMetadata,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SensorProvisioned {
    pub device_uid: DeviceUid,
    pub sensor: MeasuredSensor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SensorProvisioningConflict {
    pub reason: String,
    pub sensor: MeasuredSensor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SensorMeasurementRecorded {
    pub device_uid: DeviceUid,
    pub sensor: MeasuredSensor,
    pub measurement: SensorMeasurement,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MeasuredSensor {
    pub hardware_uid: String,
    pub kind: SensorKind,
    pub connection: SensorConnectionMetadata,
    pub firmware: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorConnectionMetadata {
    Uart(SerialPortMetadata),
    I2c { address: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorKind {
    Temperature,
    Custom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SensorMeasurement {
    pub value: f64,
    pub unit: MeasurementUnit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementUnit {
    Celsius,
    Raw,
}
