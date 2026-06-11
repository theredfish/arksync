// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_knot::domain::KnotId;
use arksync_macros::UuidV4;
use derive_more::{Display, FromStr};
use serde::{Deserialize, Serialize};

#[derive(UuidV4)]
pub struct ActuatorId([u8; 16]);

#[derive(Clone, Copy, Debug, Display, FromStr, PartialEq, Eq, Serialize, Deserialize)]
#[display(rename_all = "snake_case")]
#[from_str(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActuatorKind {
    Relay,
}

#[derive(Clone, Copy, Debug, Display, FromStr, PartialEq, Eq, Serialize, Deserialize)]
#[display(rename_all = "snake_case")]
#[from_str(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActuatorBackend {
    LinuxGpiod,
    EspGpio,
}

#[derive(Clone, Copy, Debug, Display, FromStr, PartialEq, Eq, Serialize, Deserialize)]
#[display(rename_all = "snake_case")]
#[from_str(rename_all = "snake_case")]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RelayActuator {
    pub station_knot_id: KnotId,
    pub device_uid: String,
    pub display_name: Option<String>,
    pub backend: ActuatorBackend,
    pub connection: GpioActuatorConnection,
    pub config_version: i64,
    pub enabled: bool,
    pub channels: Option<i32>,
    pub model: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Actuator {
    pub id: ActuatorId,
    pub station_knot_id: KnotId,
    pub device_uid: String,
    pub display_name: Option<String>,
    pub kind: ActuatorKind,
    pub backend: ActuatorBackend,
    pub protocol: ActuatorProtocol,
    pub connection: ActuatorConnection,
    pub config_version: i64,
    pub enabled: bool,
    pub channels: Option<i32>,
    pub model: Option<String>,
}
