// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::domain::HubId;
use sqlx::PgExecutor;

use crate::infrastructure::store::KnotRecord;

#[derive(Debug)]
pub enum KnotStoreError {
    NotFound,
    OwnershipConflict,
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

pub async fn upsert_knot(
    executor: impl PgExecutor<'_>,
    knot: &KnotRecord,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        r#"
        insert into knots (
            id,
            hub_id,
            name,
            hardware_uid,
            role,
            status,
            config_version,
            applied_config_version,
            config_status,
            config_error
        )
        values (
            $1,
            $2,
            $3,
            $4,
            $5::knot_role,
            $6::knot_status,
            $7,
            $8,
            $9,
            $10
        )
        on conflict (id) do update
        set
            name = excluded.name,
            role = excluded.role,
            status = excluded.status,
            deleted_at = null
        where knots.hub_id = excluded.hub_id
          and knots.hardware_uid = excluded.hardware_uid
        "#,
    )
    .bind(knot.id)
    .bind(knot.hub_id)
    .bind(&knot.name)
    .bind(&knot.hardware_uid)
    .bind(&knot.role)
    .bind(&knot.status)
    .bind(knot.config_version)
    .bind(knot.applied_config_version)
    .bind(&knot.config_status)
    .bind(&knot.config_error)
    .execute(executor)
    .await?;

    if result.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }
    Ok(())
}

pub async fn find_or_insert_knot_by_hardware_uid(
    executor: impl PgExecutor<'_>,
    knot: &KnotRecord,
) -> Result<KnotRecord, KnotStoreError> {
    let knot = sqlx::query_as(
        r#"
        insert into knots (
            id,
            hub_id,
            name,
            hardware_uid,
            role,
            status,
            config_version,
            applied_config_version,
            config_status,
            config_error
        )
        values (
            $1,
            $2,
            $3,
            $4,
            $5::knot_role,
            $6::knot_status,
            $7,
            $8,
            $9,
            $10
        )
        on conflict (hardware_uid) do update
        set hardware_uid = excluded.hardware_uid
        where knots.hub_id = excluded.hub_id
          and knots.deleted_at is null
        returning
            id,
            hub_id,
            name,
            hardware_uid,
            role::text as role,
            status::text as status,
            config_version,
            applied_config_version,
            config_status,
            config_error
        "#,
    )
    .bind(knot.id)
    .bind(knot.hub_id)
    .bind(&knot.name)
    .bind(&knot.hardware_uid)
    .bind(&knot.role)
    .bind(&knot.status)
    .bind(knot.config_version)
    .bind(knot.applied_config_version)
    .bind(&knot.config_status)
    .bind(&knot.config_error)
    .fetch_optional(executor)
    .await?;

    knot.ok_or(KnotStoreError::OwnershipConflict)
}

pub async fn list_knots(
    executor: impl PgExecutor<'_>,
    hub_id: HubId,
) -> Result<Vec<KnotRecord>, KnotStoreError> {
    let knots = sqlx::query_as(
        r#"
        select
            id,
            hub_id,
            name,
            hardware_uid,
            role::text as role,
            status::text as status,
            config_version,
            applied_config_version,
            config_status,
            config_error
        from knots
        where hub_id = $1 and deleted_at is null
        order by created_at asc
        "#,
    )
    .bind(hub_id.uuid_v4())
    .fetch_all(executor)
    .await?;

    Ok(knots)
}

pub async fn knot_by_hardware_uid(
    executor: impl PgExecutor<'_>,
    hub_id: HubId,
    hardware_uid: &str,
) -> Result<KnotRecord, KnotStoreError> {
    let knot = sqlx::query_as(
        r#"
        select
            id,
            hub_id,
            name,
            hardware_uid,
            role::text as role,
            status::text as status,
            config_version,
            applied_config_version,
            config_status,
            config_error
        from knots
        where hardware_uid = $1
            and hub_id = $2
            and deleted_at is null
        "#,
    )
    .bind(hardware_uid)
    .bind(hub_id.uuid_v4())
    .fetch_one(executor)
    .await?;

    Ok(knot)
}

pub async fn increment_knot_config_version(
    executor: impl PgExecutor<'_>,
    hub_id: HubId,
    knot_id: arksync_utils::uuid::Uuid,
) -> Result<i64, KnotStoreError> {
    let version = sqlx::query_scalar(
        r#"
        update knots
        set
            config_version = config_version + 1,
            config_status = 'pending',
            config_error = null
        where id = $1
            and hub_id = $2
            and deleted_at is null
        returning config_version
        "#,
    )
    .bind(knot_id)
    .bind(hub_id.uuid_v4())
    .fetch_optional(executor)
    .await?;

    version.ok_or(KnotStoreError::NotFound)
}
