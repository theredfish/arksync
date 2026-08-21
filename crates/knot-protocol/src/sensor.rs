// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

/// Sensor messages emitted by a Knot toward its Hub.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnotSensorMessage {
    Plugged(KnotSensorPlugged),
    Provisioned(KnotSensorProvisioned),
    ProvisioningConflict(KnotSensorProvisioningConflict),
    Measurements(KnotSensorMeasurementBatch),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotSensorPlugged {
    pub connection: KnotSensorConnection,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnotSensorProvisioned {
    pub device_uid: String,
    pub sensor: KnotSensorDescriptor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnotSensorProvisioningConflict {
    pub reason: String,
    pub sensor: KnotSensorDescriptor,
}

/// Batch of sensor values processed atomically under one envelope [`arksync_bus::EventId`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnotSensorMeasurementBatch {
    pub measurements: Vec<KnotSensorMeasurement>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnotSensorMeasurement {
    pub device_uid: String,
    pub sensor: KnotSensorDescriptor,
    pub value: f64,
    pub unit: KnotMeasurementUnit,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnotSensorDescriptor {
    pub hardware_uid: String,
    pub kind: KnotSensorKind,
    pub connection: KnotSensorConnection,
    pub firmware: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotSensorConnection {
    Uart(KnotSerialPort),
    I2c { address: u8 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotSerialPort {
    pub port_name: String,
    pub serial_number: String,
    pub baud_rate: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotSensorKind {
    Temperature,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotMeasurementUnit {
    Celsius,
    Raw,
}
