// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::string::String;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActuatorConfig {
    pub config_id: String,
    pub version: u64,
    pub enabled: bool,
    pub device_uid: String,
    pub actuator: ActuatorDescriptor,
    pub rules: alloc::vec::Vec<ActuatorRuleConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActuatorDescriptor {
    pub id: String,
    pub kind: ActuatorKind,
    pub backend: ActuatorBackend,
    pub protocol: ActuatorProtocol,
    pub connection: ActuatorConnection,
    pub channels: Option<i32>,
    pub model: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActuatorKind {
    Relay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActuatorBackend {
    LinuxGpiod,
    EspGpio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActuatorProtocol {
    Gpio,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActuatorConnection {
    Gpio(GpioActuatorConnection),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GpioActuatorConnection {
    pub pin: u16,
    pub pin_scheme: Option<String>,
    pub active_low: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ActuatorRuleConfig {
    pub rule_id: String,
    pub version: u64,
    pub enabled: bool,
    pub sensor_id: String,
    pub assertion: ActuatorRuleAssertion,
    pub effect: ActuatorRuleEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActuatorRuleAssertion {
    GreaterThanOrEqual { threshold: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActuatorRuleEffect {
    SetActiveWhenMatched {
        active_when_matched: bool,
        active_when_unmatched: bool,
    },
}
