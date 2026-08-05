// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Snapshot of the actuator runtime state emitted by a Knot.
///
/// This event is observational: it lets the hub persist or display what the
/// Knot is currently running without making the actuator crate depend on hub
/// storage or UI concepts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeStatus {
    /// Rule states currently known by the Knot runtime.
    pub rules: Vec<RuleRuntimeStatus>,
    /// Actuator states currently known by the Knot runtime.
    pub actuators: Vec<ActuatorRuntimeStatus>,
    /// Last sensor values seen by the runtime and relevant to rule evaluation.
    pub last_seen_sensor_values: Vec<SensorValueSnapshot>,
}

/// Runtime status for one automation rule.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuleRuntimeStatus {
    /// Stable rule identifier from the hub configuration.
    pub rule_id: String,
    /// Configuration version currently applied by the Knot.
    pub version: u64,
    /// Whether this rule is currently enabled in the runtime.
    pub enabled: bool,
}

/// Runtime status for one actuator configuration applied on a Knot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActuatorRuntimeStatus {
    /// Stable actuator configuration identifier from the hub.
    pub config_id: String,
    /// Configuration version currently applied by the Knot.
    pub version: u64,
    /// Whether this actuator is currently enabled in the runtime.
    pub enabled: bool,
}

/// Last sensor value remembered by the runtime for regulation decisions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SensorValueSnapshot {
    /// Stable sensor identifier from the hub configuration.
    pub sensor_id: String,
    /// Last numeric value observed for this sensor.
    pub value: f64,
}
