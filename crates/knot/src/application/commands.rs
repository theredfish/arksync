// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::string::String;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SerialSensor {
    pub port_name: String,
    pub serial_number: String,
    pub baud_rate: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnotCommand {
    ListenSensor { sensor: SerialSensor },
    StopListeningSensor { sensor: SerialSensor },
}

pub trait KnotCommandHandler {
    type Error;

    fn handle(&mut self, command: KnotCommand) -> Result<(), Self::Error>;
}
