// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlx::PgExecutor;

use crate::infrastructure::store::SensorMeasurementRecord;

pub async fn insert_sensor_measurement<'e, E>(
    executor: E,
    record: &SensorMeasurementRecord,
) -> Result<(), sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        r#"
        insert into sensor_measurements (
            event_id,
            source_parent_hub_id,
            source_knot_id,
            hardware_uid,
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
    .bind(&record.hardware_uid)
    .bind(&record.sensor_kind)
    .bind(&record.unit)
    .bind(record.value)
    .bind(record.measured_at_unix_millis)
    .bind(record.received_at_unix_millis)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn list_sensor_measurements_since<'e, E>(
    executor: E,
    hardware_uid: &str,
    since_unix_millis: i64,
    limit: i64,
) -> Result<Vec<SensorMeasurementRecord>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query_as(
        r#"
        select *
        from (
            select
                id,
                event_id,
                source_parent_hub_id,
                source_knot_id,
                hardware_uid,
                sensor_kind::text as sensor_kind,
                unit,
                value,
                (extract(epoch from measured_at) * 1000)::bigint as measured_at_unix_millis,
                (extract(epoch from received_at) * 1000)::bigint as received_at_unix_millis
            from sensor_measurements
            where hardware_uid = $1
                and measured_at >= to_timestamp($2::double precision / 1000.0)
            order by measured_at desc
            limit $3
        ) recent_measurements
        order by measured_at_unix_millis asc
        "#,
    )
    .bind(hardware_uid)
    .bind(since_unix_millis)
    .bind(limit)
    .fetch_all(executor)
    .await
}

pub async fn latest_sensor_hardware_uid<'e, E>(executor: E) -> Result<Option<String>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query_scalar(
        r#"
        select hardware_uid
        from sensor_measurements
        order by measured_at desc
        limit 1
        "#,
    )
    .fetch_optional(executor)
    .await
}

pub async fn latest_sensor_measurement<'e, E>(
    executor: E,
) -> Result<Option<SensorMeasurementRecord>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query_as(
        r#"
        select
            id,
            event_id,
            source_parent_hub_id,
            source_knot_id,
            hardware_uid,
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
    .await
}
