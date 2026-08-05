// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::domain::{Dashboard, DashboardComponent, DashboardComponentKind};
use arksync_utils::uuid::Uuid;
use core::str::FromStr;
use serde_json::Value;
use sqlx::FromRow;

#[derive(Clone, Debug, FromRow)]
pub struct DashboardRecord {
    pub id: Uuid,
    pub name: String,
}

#[derive(Clone, Debug, FromRow)]
pub struct DashboardComponentRecord {
    pub id: Uuid,
    pub dashboard_id: Uuid,
    pub sensor_id: Option<Uuid>,
    pub component_kind: String,
    pub title: String,
    pub refresh_interval_ms: i32,
    pub config: Value,
}

impl From<DashboardRecord> for Dashboard {
    fn from(record: DashboardRecord) -> Self {
        Self {
            id: record.id.into(),
            name: record.name,
        }
    }
}

impl From<DashboardComponentRecord> for DashboardComponent {
    fn from(record: DashboardComponentRecord) -> Self {
        Self {
            id: record.id.into(),
            dashboard_id: record.dashboard_id.into(),
            sensor_id: record.sensor_id.map(Into::into),
            component_kind: DashboardComponentKind::from_str(&record.component_kind)
                .expect("dashboard component kind should match database enum"),
            title: record.title,
            refresh_interval_ms: record.refresh_interval_ms,
            config: record.config,
        }
    }
}
