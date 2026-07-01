// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlx::{PgExecutor, PgTransaction};

use crate::infrastructure::store::KnotRecord;

#[derive(Debug)]
pub enum KnotStoreError {
    NotFound,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for KnotStoreError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound,
            err => Self::Database(err),
        }
    }
}

pub async fn upsert_station_knot(
    executor: impl PgExecutor<'_>,
    knot: &KnotRecord,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        insert into station_knots (
            id,
            station_hub_id,
            name,
            hardware_uid,
            role,
            status
        )
        values (
            $1,
            $2,
            $3,
            $4,
            $5::station_knot_role,
            $6::station_knot_status
        )
        on conflict (id) do update
        set
            station_hub_id = excluded.station_hub_id,
            name = excluded.name,
            hardware_uid = excluded.hardware_uid,
            role = excluded.role,
            status = excluded.status,
            deleted_at = null
        "#,
    )
    .bind(knot.id)
    .bind(knot.hub_id)
    .bind(&knot.name)
    .bind(&knot.hardware_uid)
    .bind(&knot.role)
    .bind(&knot.status)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn insert_station_knot(
    executor: impl PgExecutor<'_>,
    knot: &KnotRecord,
) -> Result<KnotRecord, KnotStoreError> {
    let knot = sqlx::query_as(
        r#"
        insert into station_knots (
            id,
            station_hub_id,
            name,
            hardware_uid,
            role,
            status
        )
        values (
            $1,
            $2,
            $3,
            $4,
            $5::station_knot_role,
            $6::station_knot_status
        )
        returning
            id,
            station_hub_id as hub_id,
            name,
            hardware_uid,
            role::text as role,
            status::text as status
        "#,
    )
    .bind(knot.id)
    .bind(knot.hub_id)
    .bind(&knot.name)
    .bind(&knot.hardware_uid)
    .bind(&knot.role)
    .bind(&knot.status)
    .fetch_one(executor)
    .await?;

    Ok(knot)
}

pub async fn list_station_knots(
    executor: impl PgExecutor<'_>,
) -> Result<Vec<KnotRecord>, KnotStoreError> {
    let knots = sqlx::query_as(
        r#"
        select
            id,
            station_hub_id as hub_id,
            name,
            hardware_uid,
            role::text as role,
            status::text as status
        from station_knots
        where deleted_at is null
        order by created_at asc
        "#,
    )
    .fetch_all(executor)
    .await?;

    Ok(knots)
}

pub async fn station_knot_by_hardware_uid(
    executor: impl PgExecutor<'_>,
    hardware_uid: &str,
) -> Result<KnotRecord, KnotStoreError> {
    let knot = sqlx::query_as(
        r#"
        select
            id,
            station_hub_id as hub_id,
            name,
            hardware_uid,
            role::text as role,
            status::text as status
        from station_knots
        where hardware_uid = $1
            and deleted_at is null
        "#,
    )
    .bind(hardware_uid)
    .fetch_one(executor)
    .await?;

    Ok(knot)
}

pub async fn find_or_insert_station_knot_by_hardware_uid(
    txn: &mut PgTransaction<'_>,
    knot: &KnotRecord,
) -> Result<KnotRecord, KnotStoreError> {
    match station_knot_by_hardware_uid(&mut **txn, &knot.hardware_uid).await {
        Ok(knot) => Ok(knot),
        Err(KnotStoreError::NotFound) => insert_station_knot(&mut **txn, knot).await,
        Err(err) => Err(err),
    }
}
