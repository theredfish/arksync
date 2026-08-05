// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::domain::DashboardValidationError;
use crate::infrastructure::store::{DashboardStoreError, SensorStoreError};
use derive_more::From;
use std::fmt;

#[derive(Debug, From)]
pub enum HubDashboardError {
    DashboardValidation(DashboardValidationError),
    DashboardStore(DashboardStoreError),
    SensorStore(SensorStoreError),
    InvalidRefreshInterval {
        refresh_interval_ms: i32,
        measurement_interval_ms: i32,
    },
}

impl fmt::Display for HubDashboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HubDashboardError::DashboardValidation(err) => {
                write!(f, "dashboard validation error: {err}")
            }
            HubDashboardError::DashboardStore(err) => write!(f, "dashboard store error: {err}"),
            HubDashboardError::SensorStore(err) => write!(f, "sensor store error: {err}"),
            HubDashboardError::InvalidRefreshInterval {
                refresh_interval_ms,
                measurement_interval_ms,
            } => write!(
                f,
                "dashboard component refresh interval {refresh_interval_ms}ms is lower than sensor measurement interval {measurement_interval_ms}ms"
            ),
        }
    }
}

impl std::error::Error for HubDashboardError {}
