// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlx::PgExecutor;

use crate::infrastructure::store::{NewSensorRecord, SensorRecord};
use std::fmt;

#[derive(Debug)]
pub enum SensorStoreError {
    NotFound,
    AlreadyExists,
    Database(sqlx::Error),
}

impl fmt::Display for SensorStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SensorStoreError::NotFound => f.write_str("sensor not found"),
            SensorStoreError::AlreadyExists => f.write_str("sensor already exists"),
            SensorStoreError::Database(_) => f.write_str("sensor database error"),
        }
    }
}

impl std::error::Error for SensorStoreError {}

impl From<sqlx::Error> for SensorStoreError {
    fn from(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(database_error) = &err {
            if database_error.constraint() == Some("sensors_station_knot_device_uid_unique") {
                return SensorStoreError::AlreadyExists;
            }
        }

        SensorStoreError::Database(err)
    }
}

pub async fn list_sensors<'e, E>(executor: E) -> Result<Vec<SensorRecord>, SensorStoreError>
where
    E: PgExecutor<'e>,
{
    let sensors = sqlx::query_as(
        r#"
        select
            id,
            station_knot_id,
            device_uid,
            display_name,
            kind::text as sensor_kind,
            driver::text as driver,
            protocol::text as protocol,
            connection,
            firmware,
            measurement_interval_ms
        from sensors
        where deleted_at is null
        order by created_at asc
        "#,
    )
    .fetch_all(executor)
    .await?;

    Ok(sensors)
}

pub async fn insert_sensor<'e, E>(
    executor: E,
    sensor: &NewSensorRecord,
) -> Result<SensorRecord, SensorStoreError>
where
    E: PgExecutor<'e>,
{
    let inserted = sqlx::query_as(
        r#"
        insert into sensors (
            station_knot_id,
            device_uid,
            display_name,
            kind,
            driver,
            protocol,
            connection,
            firmware,
            measurement_interval_ms,
            status,
            state_reason,
            last_activity_at
        )
        values (
            $1,
            $2,
            $3,
            $4::sensor_kind,
            $5::sensor_driver,
            $6::sensor_protocol,
            $7,
            $8,
            $9,
            'active',
            'measurement_received',
            now()
        )
        returning
            id,
            station_knot_id,
            device_uid,
            display_name,
            kind::text as sensor_kind,
            driver::text as driver,
            protocol::text as protocol,
            connection,
            firmware,
            measurement_interval_ms
        "#,
    )
    .bind(sensor.station_knot_id)
    .bind(&sensor.device_uid)
    .bind(&sensor.display_name)
    .bind(&sensor.sensor_kind)
    .bind(&sensor.driver)
    .bind(&sensor.protocol)
    .bind(&sensor.connection)
    .bind(sensor.firmware)
    .bind(sensor.measurement_interval_ms)
    .fetch_one(executor)
    .await?;

    Ok(inserted)
}
