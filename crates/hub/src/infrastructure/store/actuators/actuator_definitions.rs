// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::domain::{
    Actuator, ActuatorBackend, ActuatorConnection, ActuatorKind, ActuatorProtocol,
    GpioActuatorConnection,
};
use arksync_utils::uuid::Uuid;
use core::str::FromStr;
use sqlx::FromRow;

#[derive(Clone, Debug)]
pub struct NewActuatorRecord {
    pub station_knot_id: Uuid,
    pub device_uid: String,
    pub display_name: Option<String>,
    pub kind: String,
    pub backend: String,
    pub protocol: String,
    pub config_version: i64,
    pub enabled: bool,
    pub gpio_pin: Option<i32>,
    pub pin_scheme: Option<String>,
    pub active_low: bool,
    pub channels: Option<i32>,
    pub model: Option<String>,
}

#[derive(Clone, Debug, FromRow)]
pub struct ActuatorRecord {
    pub id: Uuid,
    pub station_knot_id: Uuid,
    pub device_uid: String,
    pub display_name: Option<String>,
    pub kind: String,
    pub backend: String,
    pub protocol: String,
    pub config_version: i64,
    pub enabled: bool,
    pub gpio_pin: Option<i32>,
    pub pin_scheme: Option<String>,
    pub active_low: bool,
    pub channels: Option<i32>,
    pub model: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NewActuatorRuleRecord {
    pub actuator_id: Uuid,
    pub sensor_id: Uuid,
    pub name: String,
    pub config_version: i64,
    pub enabled: bool,
    pub threshold: f64,
    pub active_when_matched: bool,
    pub active_when_unmatched: bool,
}

#[derive(Clone, Debug, FromRow)]
pub struct ActuatorRuleRecord {
    pub id: Uuid,
    pub actuator_id: Uuid,
    pub sensor_id: Uuid,
    pub name: String,
    pub config_version: i64,
    pub enabled: bool,
    pub threshold: f64,
    pub active_when_matched: bool,
    pub active_when_unmatched: bool,
}

impl From<ActuatorRecord> for Actuator {
    fn from(record: ActuatorRecord) -> Self {
        let protocol = ActuatorProtocol::from_str(&record.protocol)
            .expect("actuator protocol should match database enum");
        let connection = match protocol {
            ActuatorProtocol::Gpio => ActuatorConnection::Gpio(GpioActuatorConnection {
                pin: record.gpio_pin.unwrap_or_default() as u16,
                pin_scheme: record.pin_scheme,
                active_low: record.active_low,
            }),
        };

        Self {
            id: record.id.into(),
            station_knot_id: record.station_knot_id.into(),
            device_uid: record.device_uid,
            display_name: record.display_name,
            kind: ActuatorKind::from_str(&record.kind)
                .expect("actuator kind should match database enum"),
            backend: ActuatorBackend::from_str(&record.backend)
                .expect("actuator backend should match database enum"),
            protocol,
            connection,
            config_version: record.config_version,
            enabled: record.enabled,
            channels: record.channels,
            model: record.model,
        }
    }
}
