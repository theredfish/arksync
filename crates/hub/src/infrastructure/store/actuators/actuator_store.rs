// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlx::PgExecutor;
use std::fmt;

use crate::infrastructure::store::{ActuatorRecord, NewActuatorRecord};

#[derive(Debug)]
pub enum ActuatorStoreError {
    NotFound,
    AlreadyExists,
    Database(sqlx::Error),
}

impl fmt::Display for ActuatorStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("actuator not found"),
            Self::AlreadyExists => f.write_str("actuator already exists"),
            Self::Database(err) => write!(f, "actuator database error: {err}"),
        }
    }
}

impl std::error::Error for ActuatorStoreError {}

impl From<sqlx::Error> for ActuatorStoreError {
    fn from(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db_err) = &err {
            if db_err.is_unique_violation() {
                return Self::AlreadyExists;
            }
        }

        Self::Database(err)
    }
}

pub async fn list_actuators<'e, E>(executor: E) -> Result<Vec<ActuatorRecord>, ActuatorStoreError>
where
    E: PgExecutor<'e>,
{
    let actuators = sqlx::query_as(
        r#"
        select
            id,
            station_knot_id,
            device_uid,
            display_name,
            kind::text,
            backend::text,
            protocol::text,
            config_version,
            enabled,
            gpio_pin,
            pin_scheme,
            active_low,
            channels,
            model
        from actuators
        where deleted_at is null
        "#,
    )
    .fetch_all(executor)
    .await?;

    Ok(actuators)
}

pub async fn list_actuators_by_station_knot_id<'e, E>(
    executor: E,
    station_knot_id: arksync_utils::uuid::Uuid,
) -> Result<Vec<ActuatorRecord>, ActuatorStoreError>
where
    E: PgExecutor<'e>,
{
    let actuators = sqlx::query_as(
        r#"
        select
            id,
            station_knot_id,
            device_uid,
            display_name,
            kind::text,
            backend::text,
            protocol::text,
            config_version,
            enabled,
            gpio_pin,
            pin_scheme,
            active_low,
            channels,
            model
        from actuators
        where station_knot_id = $1
            and deleted_at is null
        "#,
    )
    .bind(station_knot_id)
    .fetch_all(executor)
    .await?;

    Ok(actuators)
}

pub async fn insert_actuator<'e, E>(
    executor: E,
    actuator: &NewActuatorRecord,
) -> Result<ActuatorRecord, ActuatorStoreError>
where
    E: PgExecutor<'e>,
{
    let inserted = sqlx::query_as(
        r#"
        insert into actuators (
            station_knot_id,
            device_uid,
            display_name,
            kind,
            backend,
            protocol,
            config_version,
            enabled,
            gpio_pin,
            pin_scheme,
            active_low,
            channels,
            model
        )
        values (
            $1,
            $2,
            $3,
            $4::actuator_kind,
            $5::actuator_backend,
            $6::actuator_protocol,
            $7,
            $8,
            $9,
            $10,
            $11,
            $12,
            $13
        )
        returning
            id,
            station_knot_id,
            device_uid,
            display_name,
            kind::text,
            backend::text,
            protocol::text,
            config_version,
            enabled,
            gpio_pin,
            pin_scheme,
            active_low,
            channels,
            model
        "#,
    )
    .bind(actuator.station_knot_id)
    .bind(&actuator.device_uid)
    .bind(&actuator.display_name)
    .bind(&actuator.kind)
    .bind(&actuator.backend)
    .bind(&actuator.protocol)
    .bind(actuator.config_version)
    .bind(actuator.enabled)
    .bind(actuator.gpio_pin)
    .bind(&actuator.pin_scheme)
    .bind(actuator.active_low)
    .bind(actuator.channels)
    .bind(&actuator.model)
    .fetch_one(executor)
    .await?;

    Ok(inserted)
}

pub async fn update_actuator_runtime_status<'e, E>(
    executor: E,
    actuator_id: &str,
    config_version: i64,
    enabled: bool,
) -> Result<(), ActuatorStoreError>
where
    E: PgExecutor<'e>,
{
    let result = sqlx::query(
        r#"
        update actuators
        set
            config_version = greatest(config_version, $2),
            status = case
                when $3 then 'active'::actuator_status
                else 'disabled'::actuator_status
            end
        where id = $1::uuid
            and deleted_at is null
        "#,
    )
    .bind(actuator_id)
    .bind(config_version)
    .bind(enabled)
    .execute(executor)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ActuatorStoreError::NotFound);
    }

    Ok(())
}
