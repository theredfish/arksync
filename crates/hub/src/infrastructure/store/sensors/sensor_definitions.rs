// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::domain::Sensor;
use arksync_sensor::infrastructure::events::SensorKind;
use arksync_utils::uuid::Uuid;
use serde_json::Value;
use sqlx::FromRow;

#[derive(Clone, Debug)]
pub struct NewSensorRecord {
    pub station_knot_id: Uuid,
    pub device_uid: String,
    pub display_name: Option<String>,
    pub sensor_kind: String,
    pub driver: String,
    pub protocol: String,
    pub connection: Value,
    pub firmware: Option<f64>,
    pub measurement_interval_ms: i32,
}

#[derive(Clone, Debug, FromRow)]
pub struct SensorRecord {
    pub id: Uuid,
    pub station_knot_id: Uuid,
    pub device_uid: String,
    pub display_name: Option<String>,
    pub sensor_kind: String,
    pub driver: String,
    pub protocol: String,
    pub connection: Value,
    pub firmware: Option<f64>,
    pub measurement_interval_ms: i32,
}

impl From<SensorRecord> for Sensor {
    fn from(record: SensorRecord) -> Self {
        Self {
            id: record.id.into(),
            station_knot_id: record.station_knot_id.into(),
            device_uid: record.device_uid,
            display_name: record.display_name,
            kind: sensor_kind_from_str(&record.sensor_kind),
            driver: record.driver,
            protocol: record.protocol,
            connection: record.connection,
            firmware: record.firmware,
            measurement_interval_ms: record.measurement_interval_ms,
        }
    }
}

pub fn sensor_kind_as_str(kind: SensorKind) -> &'static str {
    match kind {
        SensorKind::Temperature => "temperature",
        SensorKind::Custom => "custom",
    }
}

pub fn sensor_kind_from_str(kind: &str) -> SensorKind {
    match kind {
        "temperature" => SensorKind::Temperature,
        _ => SensorKind::Custom,
    }
}
