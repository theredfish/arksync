// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::domain::SensorMeasurement;
use arksync_bus::{EventId, Timestamp};
use arksync_knot::domain::KnotEventSource;
use arksync_sensor::infrastructure::events::{MeasurementUnit, SensorKind};
use arksync_utils::uuid::Uuid;
use sqlx::FromRow;

#[derive(Clone, Debug)]
pub struct SystemUserRecord {
    pub id: Uuid,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug)]
pub struct HubRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub hardware_uid: String,
}

#[derive(Clone, Debug)]
pub struct KnotRecord {
    pub id: Uuid,
    pub hub_id: Uuid,
    pub name: String,
    pub hardware_uid: String,
}

#[derive(Clone, Debug, FromRow)]
pub struct SensorMeasurementRecord {
    pub id: Uuid,
    pub event_id: Uuid,
    pub source_parent_hub_id: Uuid,
    pub source_knot_id: Uuid,
    pub hardware_uid: String,
    pub sensor_kind: String,
    pub unit: String,
    pub value: f64,
    pub measured_at_unix_millis: i64,
    pub received_at_unix_millis: i64,
}

impl SensorMeasurementRecord {
    pub fn new(event_id: EventId, measurement: &SensorMeasurement) -> Self {
        let KnotEventSource::Knot {
            parent_hub_id,
            knot_id,
        } = measurement.source;

        Self {
            id: Uuid::nil(),
            event_id: event_id.as_uuid(),
            source_parent_hub_id: parent_hub_id.as_uuid(),
            source_knot_id: knot_id.as_uuid(),
            hardware_uid: measurement.hardware_uid.clone(),
            sensor_kind: sensor_kind_as_str(measurement.kind).to_string(),
            unit: measurement_unit_as_str(measurement.unit).to_string(),
            value: measurement.value,
            measured_at_unix_millis: measurement.measured_at.unix_millis,
            received_at_unix_millis: measurement.received_at.unix_millis,
        }
    }
}

impl From<SensorMeasurementRecord> for SensorMeasurement {
    fn from(record: SensorMeasurementRecord) -> Self {
        Self {
            source: KnotEventSource::Knot {
                parent_hub_id: record.source_parent_hub_id.into(),
                knot_id: record.source_knot_id.into(),
            },
            hardware_uid: record.hardware_uid,
            kind: sensor_kind_from_str(&record.sensor_kind),
            value: record.value,
            unit: measurement_unit_from_str(&record.unit),
            measured_at: Timestamp::from_unix_millis(record.measured_at_unix_millis),
            received_at: Timestamp::from_unix_millis(record.received_at_unix_millis),
        }
    }
}

fn sensor_kind_as_str(kind: SensorKind) -> &'static str {
    match kind {
        SensorKind::Temperature => "temperature",
        SensorKind::Custom => "custom",
    }
}

fn sensor_kind_from_str(kind: &str) -> SensorKind {
    match kind {
        "temperature" => SensorKind::Temperature,
        _ => SensorKind::Custom,
    }
}

fn measurement_unit_as_str(unit: MeasurementUnit) -> &'static str {
    match unit {
        MeasurementUnit::Celsius => "celsius",
        MeasurementUnit::Raw => "raw",
    }
}

fn measurement_unit_from_str(unit: &str) -> MeasurementUnit {
    match unit {
        "celsius" => MeasurementUnit::Celsius,
        _ => MeasurementUnit::Raw,
    }
}
