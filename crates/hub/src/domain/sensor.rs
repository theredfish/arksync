// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::Timestamp;
use arksync_knot::domain::{KnotEventSource, KnotId};
use arksync_macros::UuidV4;
use arksync_sensor::device_uid::DeviceUid;
use arksync_sensor::infrastructure::events::{
    MeasurementUnit, SensorConnectionMetadata, SensorKind,
};
use arksync_sensor::serial_port::SerialPortMetadata;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(UuidV4)]
pub struct SensorId([u8; 16]);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorRegistrationStatus {
    Discovered,
    Registered,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RegisteredSensor {
    pub id: SensorId,
    pub display_name: String,
    pub metadata: SerialPortMetadata,
    pub registered_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ObservedSerialSensor {
    pub source: KnotEventSource,
    pub metadata: SerialPortMetadata,
    pub first_observed_at: Timestamp,
    pub last_observed_at: Timestamp,
    pub last_received_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SensorOverview {
    pub sensor_id: Option<SensorId>,
    pub display_name: String,
    pub metadata: SerialPortMetadata,
    pub status: SensorRegistrationStatus,
    pub first_observed_at: Option<Timestamp>,
    pub last_observed_at: Option<Timestamp>,
    pub last_received_at: Option<Timestamp>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PluggedSensor {
    pub station_knot_id: KnotId,
    pub device_uid: DeviceUid,
    pub kind: SensorKind,
    pub connection: SensorConnectionMetadata,
    pub firmware: Option<f64>,
    pub measurement_interval_ms: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Sensor {
    pub id: SensorId,
    pub station_knot_id: KnotId,
    pub device_uid: String,
    pub display_name: Option<String>,
    pub kind: SensorKind,
    pub driver: String,
    pub protocol: String,
    pub connection: Value,
    pub firmware: Option<f64>,
    pub measurement_interval_ms: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SensorMeasurement {
    pub source: KnotEventSource,
    pub sensor_id: SensorId,
    pub kind: SensorKind,
    pub value: f64,
    pub unit: MeasurementUnit,
    pub measured_at: Timestamp,
    pub received_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SensorMeasurementPoint {
    pub measured_at: Timestamp,
    pub value: f64,
    pub unit: MeasurementUnit,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SensorTimeSeries {
    pub sensor_id: SensorId,
    pub window_start: Timestamp,
    pub window_end: Timestamp,
    pub points: Vec<SensorMeasurementPoint>,
}

impl SensorOverview {
    pub fn from_observed(
        observed: &ObservedSerialSensor,
        registered: Option<&RegisteredSensor>,
    ) -> Self {
        let sensor_id = registered.map(|sensor| sensor.id);
        let display_name = registered
            .map(|sensor| sensor.display_name.clone())
            .unwrap_or_else(|| observed.metadata.serial_number.to_string());
        let status = if registered.is_some() {
            SensorRegistrationStatus::Registered
        } else {
            SensorRegistrationStatus::Discovered
        };

        Self {
            sensor_id,
            display_name,
            metadata: observed.metadata.clone(),
            status,
            first_observed_at: Some(observed.first_observed_at),
            last_observed_at: Some(observed.last_observed_at),
            last_received_at: Some(observed.last_received_at),
        }
    }
}
