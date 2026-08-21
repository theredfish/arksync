// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnotConfig {
    pub version: u64,
    pub knot_id: [u8; 16],
    pub sensor_bindings: Vec<KnotSensorBinding>,
    pub actuator_configs: Vec<KnotActuatorConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotSensorBinding {
    pub sensor_id: [u8; 16],
    pub device_uid: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnotActuatorConfig {
    pub config_id: String,
    pub version: u64,
    pub enabled: bool,
    pub device_uid: String,
    pub actuator: KnotActuatorDescriptor,
    pub rules: Vec<KnotActuatorRule>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotActuatorDescriptor {
    pub id: String,
    pub kind: KnotActuatorKind,
    pub backend: KnotActuatorBackend,
    pub protocol: KnotActuatorProtocol,
    pub connection: KnotActuatorConnection,
    pub channels: Option<i32>,
    pub model: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotActuatorKind {
    Relay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotActuatorBackend {
    LinuxGpiod,
    EspGpio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotActuatorProtocol {
    Gpio,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotActuatorConnection {
    Gpio(KnotGpioActuatorConnection),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnotGpioActuatorConnection {
    pub pin: u16,
    pub pin_scheme: Option<String>,
    pub active_low: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnotActuatorRule {
    pub rule_id: String,
    pub version: u64,
    pub enabled: bool,
    pub sensor_id: String,
    pub assertion: KnotActuatorRuleAssertion,
    pub effect: KnotActuatorRuleEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum KnotActuatorRuleAssertion {
    GreaterThanOrEqual { threshold: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnotActuatorRuleEffect {
    SetActiveWhenMatched {
        active_when_matched: bool,
        active_when_unmatched: bool,
    },
}
