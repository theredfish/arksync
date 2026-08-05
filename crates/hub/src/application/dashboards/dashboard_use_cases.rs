// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::HubDashboardError;
use crate::domain::{Dashboard, DashboardComponent, DashboardId, Sensor, SensorId};
use crate::infrastructure::store::{dashboards as dashboard_store, sensors as sensor_store};
use sqlx::{PgExecutor, PgTransaction};

pub async fn list_dashboards(
    executor: impl PgExecutor<'_>,
) -> Result<Vec<Dashboard>, HubDashboardError> {
    let records = dashboard_store::list_dashboards(executor).await?;

    Ok(records.into_iter().map(Dashboard::from).collect())
}

pub async fn dashboard_by_id(
    executor: impl PgExecutor<'_>,
    dashboard_id: DashboardId,
) -> Result<Dashboard, HubDashboardError> {
    Ok(
        dashboard_store::dashboard_by_id(executor, dashboard_id.uuid_v4())
            .await?
            .into(),
    )
}

pub async fn dashboard_by_name(
    executor: impl PgExecutor<'_>,
    name: impl AsRef<str>,
) -> Result<Dashboard, HubDashboardError> {
    Ok(dashboard_store::dashboard_by_name(executor, name.as_ref())
        .await?
        .into())
}

pub async fn create_dashboard(
    executor: impl PgExecutor<'_>,
    name: impl AsRef<str>,
) -> Result<Dashboard, HubDashboardError> {
    let dashboard = Dashboard::new(name.as_ref().to_string())?;

    Ok(dashboard_store::insert_dashboard(executor, &dashboard)
        .await?
        .into())
}

pub async fn list_dashboard_components(
    executor: impl PgExecutor<'_>,
    dashboard_id: DashboardId,
) -> Result<Vec<DashboardComponent>, HubDashboardError> {
    let records =
        dashboard_store::list_dashboard_components(executor, dashboard_id.uuid_v4()).await?;

    Ok(records.into_iter().map(DashboardComponent::from).collect())
}

pub async fn create_dashboard_component(
    txn: &mut PgTransaction<'_>,
    component: DashboardComponent,
) -> Result<DashboardComponent, HubDashboardError> {
    if let Some(sensor_id) = component.sensor_id {
        let sensor = sensor_store::sensor_by_id(&mut **txn, sensor_id.uuid_v4()).await?;
        if component.refresh_interval_ms < sensor.measurement_interval_ms {
            return Err(HubDashboardError::InvalidRefreshInterval {
                refresh_interval_ms: component.refresh_interval_ms,
                measurement_interval_ms: sensor.measurement_interval_ms,
            });
        }
    }

    Ok(
        dashboard_store::insert_dashboard_component(&mut **txn, &component)
            .await?
            .into(),
    )
}

pub async fn sensor_by_id(
    executor: impl PgExecutor<'_>,
    sensor_id: SensorId,
) -> Result<Sensor, HubDashboardError> {
    Ok(sensor_store::sensor_by_id(executor, sensor_id.uuid_v4())
        .await?
        .into())
}

pub async fn sensor_by_device_uid(
    executor: impl PgExecutor<'_>,
    device_uid: &str,
) -> Result<Sensor, HubDashboardError> {
    Ok(sensor_store::sensor_by_device_uid(executor, device_uid)
        .await?
        .into())
}
