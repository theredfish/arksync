// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::domain::{Dashboard, DashboardComponent};
use crate::infrastructure::store::{DashboardComponentRecord, DashboardRecord};
use arksync_utils::uuid::Uuid;
use sqlx::PgExecutor;
use std::fmt;

#[derive(Debug)]
pub enum DashboardStoreError {
    NotFound,
    AlreadyExists,
    Database(sqlx::Error),
}

impl fmt::Display for DashboardStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DashboardStoreError::NotFound => f.write_str("dashboard not found"),
            DashboardStoreError::AlreadyExists => f.write_str("dashboard already exists"),
            DashboardStoreError::Database(err) => write!(f, "dashboard database error: {err}"),
        }
    }
}

impl std::error::Error for DashboardStoreError {}

impl From<sqlx::Error> for DashboardStoreError {
    fn from(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(database_error) = &err {
            if database_error.is_unique_violation() {
                return DashboardStoreError::AlreadyExists;
            }
        }

        DashboardStoreError::Database(err)
    }
}

pub async fn list_dashboards(
    executor: impl PgExecutor<'_>,
) -> Result<Vec<DashboardRecord>, DashboardStoreError> {
    let dashboards = sqlx::query_as(
        r#"
        select
            id,
            name
        from dashboards
        where deleted_at is null
        order by created_at asc
        "#,
    )
    .fetch_all(executor)
    .await?;

    Ok(dashboards)
}

pub async fn dashboard_by_id(
    executor: impl PgExecutor<'_>,
    dashboard_id: Uuid,
) -> Result<DashboardRecord, DashboardStoreError> {
    let dashboard = sqlx::query_as(
        r#"
        select
            id,
            name
        from dashboards
        where id = $1
            and deleted_at is null
        "#,
    )
    .bind(dashboard_id)
    .fetch_optional(executor)
    .await?;

    dashboard.ok_or(DashboardStoreError::NotFound)
}

pub async fn dashboard_by_name(
    executor: impl PgExecutor<'_>,
    name: &str,
) -> Result<DashboardRecord, DashboardStoreError> {
    let dashboard = sqlx::query_as(
        r#"
        select
            id,
            name
        from dashboards
        where name = $1
            and deleted_at is null
        "#,
    )
    .bind(name)
    .fetch_optional(executor)
    .await?;

    dashboard.ok_or(DashboardStoreError::NotFound)
}

pub async fn insert_dashboard(
    executor: impl PgExecutor<'_>,
    dashboard: &Dashboard,
) -> Result<DashboardRecord, DashboardStoreError> {
    let inserted = sqlx::query_as(
        r#"
        insert into dashboards (
            id,
            name
        )
        values (
            $1,
            $2
        )
        returning
            id,
            name
        "#,
    )
    .bind(dashboard.id.uuid_v4())
    .bind(&dashboard.name)
    .fetch_one(executor)
    .await?;

    Ok(inserted)
}

pub async fn list_dashboard_components(
    executor: impl PgExecutor<'_>,
    dashboard_id: Uuid,
) -> Result<Vec<DashboardComponentRecord>, DashboardStoreError> {
    let components = sqlx::query_as(
        r#"
        select
            id,
            dashboard_id,
            sensor_id,
            component_kind::text as component_kind,
            title,
            refresh_interval_ms,
            config
        from dashboard_components
        where dashboard_id = $1
            and deleted_at is null
        order by created_at asc
        "#,
    )
    .bind(dashboard_id)
    .fetch_all(executor)
    .await?;

    Ok(components)
}

pub async fn insert_dashboard_component(
    executor: impl PgExecutor<'_>,
    component: &DashboardComponent,
) -> Result<DashboardComponentRecord, DashboardStoreError> {
    let inserted = sqlx::query_as(
        r#"
        insert into dashboard_components (
            id,
            dashboard_id,
            sensor_id,
            component_kind,
            title,
            refresh_interval_ms,
            config
        )
        values (
            $1,
            $2,
            $3,
            $4::dashboard_component_kind,
            $5,
            $6,
            $7
        )
        returning
            id,
            dashboard_id,
            sensor_id,
            component_kind::text as component_kind,
            title,
            refresh_interval_ms,
            config
        "#,
    )
    .bind(component.id.uuid_v4())
    .bind(component.dashboard_id.uuid_v4())
    .bind(component.sensor_id.map(|sensor_id| sensor_id.uuid_v4()))
    .bind(component.component_kind.to_string())
    .bind(&component.title)
    .bind(component.refresh_interval_ms)
    .bind(&component.config)
    .fetch_one(executor)
    .await?;

    Ok(inserted)
}
