// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlx::PgExecutor;

use crate::infrastructure::store::SensorMeasurementRecord;
use std::fmt;

#[derive(Debug)]
pub enum SensorMeasurementStoreError {
    Database(sqlx::Error),
}

impl fmt::Display for SensorMeasurementStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SensorMeasurementStoreError::Database(_) => {
                f.write_str("sensor measurement database error")
            }
        }
    }
}

impl std::error::Error for SensorMeasurementStoreError {}

impl From<sqlx::Error> for SensorMeasurementStoreError {
    fn from(err: sqlx::Error) -> Self {
        SensorMeasurementStoreError::Database(err)
    }
}

pub async fn insert_sensor_measurement(
    executor: impl PgExecutor<'_>,
    record: &SensorMeasurementRecord,
) -> Result<(), SensorMeasurementStoreError> {
    sqlx::query(
        r#"
        insert into sensor_measurements (
            event_id,
            source_parent_hub_id,
            source_knot_id,
            sensor_id,
            sensor_kind,
            unit,
            value,
            measured_at,
            received_at
        )
        values (
            $1,
            $2,
            $3,
            $4,
            $5::sensor_kind,
            $6,
            $7,
            to_timestamp($8::double precision / 1000.0),
            to_timestamp($9::double precision / 1000.0)
        )
        "#,
    )
    .bind(record.event_id)
    .bind(record.source_parent_hub_id)
    .bind(record.source_knot_id)
    .bind(record.sensor_id)
    .bind(&record.sensor_kind)
    .bind(&record.unit)
    .bind(record.value)
    .bind(record.measured_at_unix_millis)
    .bind(record.received_at_unix_millis)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn list_sensor_measurements_since(
    executor: impl PgExecutor<'_>,
    sensor_id: arksync_utils::uuid::Uuid,
    since_unix_millis: i64,
    limit: i64,
) -> Result<Vec<SensorMeasurementRecord>, SensorMeasurementStoreError> {
    let measurements = sqlx::query_as(
        r#"
        select *
        from (
            select
                id,
                event_id,
                source_parent_hub_id,
                source_knot_id,
                sensor_id,
                sensor_kind::text as sensor_kind,
                unit,
                value,
                (extract(epoch from measured_at) * 1000)::bigint as measured_at_unix_millis,
                (extract(epoch from received_at) * 1000)::bigint as received_at_unix_millis
            from sensor_measurements
            where sensor_id = $1
                and measured_at >= to_timestamp($2::double precision / 1000.0)
            order by measured_at desc
            limit $3
        ) recent_measurements
        order by measured_at_unix_millis asc
        "#,
    )
    .bind(sensor_id)
    .bind(since_unix_millis)
    .bind(limit)
    .fetch_all(executor)
    .await?;

    Ok(measurements)
}

pub async fn latest_sensor_id(
    executor: impl PgExecutor<'_>,
) -> Result<Option<arksync_utils::uuid::Uuid>, SensorMeasurementStoreError> {
    let sensor_id = sqlx::query_scalar(
        r#"
        select sensor_id
        from sensor_measurements
        order by measured_at desc
        limit 1
        "#,
    )
    .fetch_optional(executor)
    .await?;

    Ok(sensor_id)
}

pub async fn latest_sensor_measurement(
    executor: impl PgExecutor<'_>,
) -> Result<Option<SensorMeasurementRecord>, SensorMeasurementStoreError> {
    let measurement = sqlx::query_as(
        r#"
        select
            id,
            event_id,
            source_parent_hub_id,
            source_knot_id,
            sensor_id,
            sensor_kind::text as sensor_kind,
            unit,
            value,
            (extract(epoch from measured_at) * 1000)::bigint as measured_at_unix_millis,
            (extract(epoch from received_at) * 1000)::bigint as received_at_unix_millis
        from sensor_measurements
        order by measured_at desc
        limit 1
        "#,
    )
    .fetch_optional(executor)
    .await?;

    Ok(measurement)
}
