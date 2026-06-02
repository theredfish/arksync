// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::HubSensorError;
use crate::domain::{SensorId, SensorMeasurement, SensorMeasurementPoint, SensorTimeSeries};
use crate::infrastructure::store::{
    insert_sensor_measurement, latest_sensor_id, latest_sensor_measurement,
    list_sensor_measurements_since, SensorMeasurementRecord,
};
use arksync_bus::{EventId, Timestamp};
use arksync_knot::domain::KnotEventSource;
use arksync_sensor::infrastructure::events::{MeasurementUnit, SensorKind};
use sqlx::PgExecutor;

#[derive(Clone, Debug)]
pub struct SensorMeasurementInput {
    pub event_id: EventId,
    pub source: KnotEventSource,
    pub sensor_id: SensorId,
    pub kind: SensorKind,
    pub value: f64,
    pub unit: MeasurementUnit,
    pub measured_at: Timestamp,
    pub received_at: Timestamp,
}

pub async fn record_sensor_measurement<'e, E>(
    executor: E,
    input: SensorMeasurementInput,
) -> Result<SensorMeasurement, HubSensorError>
where
    E: PgExecutor<'e>,
{
    let measurement = SensorMeasurement {
        source: input.source,
        sensor_id: input.sensor_id,
        kind: input.kind,
        value: input.value,
        unit: input.unit,
        measured_at: input.measured_at,
        received_at: input.received_at,
    };
    let record = SensorMeasurementRecord::new(input.event_id, &measurement);

    insert_sensor_measurement(executor, &record).await?;

    Ok(measurement)
}

pub async fn load_sensor_time_series<'e, E>(
    executor: E,
    sensor_id: SensorId,
    window_start: Timestamp,
    window_end: Timestamp,
    limit: i64,
) -> Result<SensorTimeSeries, HubSensorError>
where
    E: PgExecutor<'e>,
{
    let records = list_sensor_measurements_since(
        executor,
        sensor_id.as_uuid(),
        window_start.unix_millis,
        limit,
    )
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
        sensor_id,
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
) -> Result<Option<SensorTimeSeries>, HubSensorError> {
    let Some(sensor_id) = latest_sensor_id(executor).await? else {
        return Ok(None);
    };

    load_sensor_time_series(executor, sensor_id.into(), window_start, window_end, limit)
        .await
        .map(Some)
}

pub async fn load_latest_sensor_measurement(
    executor: &sqlx::PgPool,
) -> Result<Option<SensorMeasurement>, HubSensorError> {
    let measurement = latest_sensor_measurement(executor).await?;

    Ok(measurement.map(SensorMeasurement::from))
}
