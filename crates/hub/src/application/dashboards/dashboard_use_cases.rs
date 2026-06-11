// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::HubDashboardError;
use crate::domain::{Dashboard, DashboardComponent, DashboardId, Sensor, SensorId};
use crate::infrastructure::store::{dashboards as dashboard_store, sensors as sensor_store};
use sqlx::PgExecutor;

pub async fn list_dashboards<'e, E>(executor: E) -> Result<Vec<Dashboard>, HubDashboardError>
where
    E: PgExecutor<'e>,
{
    let records = dashboard_store::list_dashboards(executor).await?;

    Ok(records.into_iter().map(Dashboard::from).collect())
}

pub async fn dashboard_by_id<'e, E>(
    executor: E,
    dashboard_id: DashboardId,
) -> Result<Dashboard, HubDashboardError>
where
    E: PgExecutor<'e>,
{
    Ok(
        dashboard_store::dashboard_by_id(executor, dashboard_id.uuid_v4())
            .await?
            .into(),
    )
}

pub async fn dashboard_by_name<'e, E>(
    executor: E,
    name: impl AsRef<str>,
) -> Result<Dashboard, HubDashboardError>
where
    E: PgExecutor<'e>,
{
    Ok(dashboard_store::dashboard_by_name(executor, name.as_ref())
        .await?
        .into())
}

pub async fn create_dashboard<'e, E>(
    executor: E,
    name: impl AsRef<str>,
) -> Result<Dashboard, HubDashboardError>
where
    E: PgExecutor<'e>,
{
    let dashboard = Dashboard::new(name.as_ref().to_string())?;

    Ok(dashboard_store::insert_dashboard(executor, &dashboard)
        .await?
        .into())
}

pub async fn list_dashboard_components<'e, E>(
    executor: E,
    dashboard_id: DashboardId,
) -> Result<Vec<DashboardComponent>, HubDashboardError>
where
    E: PgExecutor<'e>,
{
    let records =
        dashboard_store::list_dashboard_components(executor, dashboard_id.uuid_v4()).await?;

    Ok(records.into_iter().map(DashboardComponent::from).collect())
}

pub async fn create_dashboard_component<'e, E>(
    executor: E,
    component: DashboardComponent,
) -> Result<DashboardComponent, HubDashboardError>
where
    E: PgExecutor<'e> + Copy,
{
    if let Some(sensor_id) = component.sensor_id {
        let sensor = sensor_store::sensor_by_id(executor, sensor_id.uuid_v4()).await?;
        if component.refresh_interval_ms < sensor.measurement_interval_ms {
            return Err(HubDashboardError::InvalidRefreshInterval {
                refresh_interval_ms: component.refresh_interval_ms,
                measurement_interval_ms: sensor.measurement_interval_ms,
            });
        }
    }

    Ok(
        dashboard_store::insert_dashboard_component(executor, &component)
            .await?
            .into(),
    )
}

pub async fn sensor_by_id<'e, E>(
    executor: E,
    sensor_id: SensorId,
) -> Result<Sensor, HubDashboardError>
where
    E: PgExecutor<'e>,
{
    Ok(sensor_store::sensor_by_id(executor, sensor_id.uuid_v4())
        .await?
        .into())
}

pub async fn sensor_by_device_uid<'e, E>(
    executor: E,
    device_uid: &str,
) -> Result<Sensor, HubDashboardError>
where
    E: PgExecutor<'e>,
{
    Ok(sensor_store::sensor_by_device_uid(executor, device_uid)
        .await?
        .into())
}
