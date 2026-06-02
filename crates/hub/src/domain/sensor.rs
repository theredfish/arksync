// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::domain::SensorId;
use arksync_bus::Timestamp;
use arksync_knot::domain::KnotEventSource;
use arksync_sensor::infrastructure::events::{MeasurementUnit, SensorKind};
use arksync_sensor::serial_port::SerialPortMetadata;
use serde::{Deserialize, Serialize};

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
pub struct SensorMeasurement {
    pub source: KnotEventSource,
    pub hardware_uid: String,
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
    pub hardware_uid: String,
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
