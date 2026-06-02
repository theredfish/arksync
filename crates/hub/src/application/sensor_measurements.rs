// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::HubSensorEventEnvelope;
use crate::domain::{SensorMeasurement, SensorMeasurementPoint, SensorTimeSeries};
use crate::infrastructure::store::{
    insert_sensor_measurement, latest_sensor_hardware_uid, latest_sensor_measurement,
    list_sensor_measurements_since, SensorMeasurementRecord,
};
use arksync_bus::Timestamp;
use arksync_sensor::infrastructure::events::SensorEvent;
use sqlx::PgExecutor;

pub async fn persist_sensor_measurement<'e, E>(
    executor: E,
    event: &HubSensorEventEnvelope,
    received_at: Timestamp,
) -> Result<Option<SensorMeasurement>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    let SensorEvent::SensorMeasurementRecorded(measurement) = &event.payload else {
        return Ok(None);
    };

    let measurement = SensorMeasurement {
        source: event.source.clone(),
        hardware_uid: measurement.sensor.hardware_uid.clone(),
        kind: measurement.sensor.kind,
        value: measurement.measurement.value,
        unit: measurement.measurement.unit,
        measured_at: event.occurred_at,
        received_at,
    };
    let record = SensorMeasurementRecord::new(event.id, &measurement);

    insert_sensor_measurement(executor, &record).await?;

    Ok(Some(measurement))
}

pub async fn load_sensor_time_series<'e, E>(
    executor: E,
    hardware_uid: &str,
    window_start: Timestamp,
    window_end: Timestamp,
    limit: i64,
) -> Result<SensorTimeSeries, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    let records =
        list_sensor_measurements_since(executor, hardware_uid, window_start.unix_millis, limit)
            .await?;
    let points = records
        .into_iter()
        .map(SensorMeasurement::from)
        .filter(|measurement| measurement.measured_at <= window_end)
        .map(|measurement| SensorMeasurementPoint {
            measured_at: measurement.measured_at,
            value: measurement.value,
            unit: measurement.unit,
        })
        .collect();

    Ok(SensorTimeSeries {
        hardware_uid: hardware_uid.to_string(),
        window_start,
        window_end,
        points,
    })
}

pub async fn load_latest_sensor_time_series(
    executor: &sqlx::PgPool,
    window_start: Timestamp,
    window_end: Timestamp,
    limit: i64,
) -> Result<Option<SensorTimeSeries>, sqlx::Error> {
    let Some(hardware_uid) = latest_sensor_hardware_uid(executor).await? else {
        return Ok(None);
    };

    load_sensor_time_series(executor, &hardware_uid, window_start, window_end, limit)
        .await
        .map(Some)
}

pub async fn load_latest_sensor_measurement(
    executor: &sqlx::PgPool,
) -> Result<Option<SensorMeasurement>, sqlx::Error> {
    latest_sensor_measurement(executor)
        .await
        .map(|record| record.map(SensorMeasurement::from))
}
