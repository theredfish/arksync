// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::domain::SensorId;
use arksync_macros::UuidV4;
use core::fmt;
use derive_more::{Display, FromStr};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(UuidV4)]
pub struct DashboardId([u8; 16]);

#[derive(UuidV4)]
pub struct DashboardComponentId([u8; 16]);

#[derive(Clone, Copy, Debug, Display, FromStr, PartialEq, Eq, Serialize, Deserialize)]
#[display(rename_all = "snake_case")]
#[from_str(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DashboardComponentKind {
    Gauge,
    LineChart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DashboardValidationError {
    EmptyDashboardName,
    EmptyComponentTitle,
    InvalidRefreshInterval,
}

impl fmt::Display for DashboardValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DashboardValidationError::EmptyDashboardName => f.write_str("dashboard name is empty"),
            DashboardValidationError::EmptyComponentTitle => {
                f.write_str("dashboard component title is empty")
            }
            DashboardValidationError::InvalidRefreshInterval => {
                f.write_str("dashboard component refresh interval must be greater than zero")
            }
        }
    }
}

impl std::error::Error for DashboardValidationError {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Dashboard {
    pub id: DashboardId,
    pub name: String,
}

impl Dashboard {
    pub fn new(name: String) -> Result<Self, DashboardValidationError> {
        let name = name.trim().to_string();

        if name.is_empty() {
            return Err(DashboardValidationError::EmptyDashboardName);
        }

        Ok(Self {
            id: DashboardId::new(),
            name,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DashboardComponent {
    pub id: DashboardComponentId,
    pub dashboard_id: DashboardId,
    pub sensor_id: Option<SensorId>,
    pub component_kind: DashboardComponentKind,
    pub title: String,
    pub refresh_interval_ms: i32,
    pub config: Value,
}

impl DashboardComponent {
    pub fn new(
        dashboard_id: DashboardId,
        sensor_id: Option<SensorId>,
        component_kind: DashboardComponentKind,
        title: String,
        refresh_interval_ms: i32,
        config: Value,
    ) -> Result<Self, DashboardValidationError> {
        let title = title.trim().to_string();

        if title.is_empty() {
            return Err(DashboardValidationError::EmptyComponentTitle);
        }

        if refresh_interval_ms <= 0 {
            return Err(DashboardValidationError::InvalidRefreshInterval);
        }

        Ok(Self {
            id: DashboardComponentId::new(),
            dashboard_id,
            sensor_id,
            component_kind,
            title,
            refresh_interval_ms,
            config,
        })
    }
}
