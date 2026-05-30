// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_knot::application::{KnotCommand, SerialSensor};
use arksync_sensor::serial_port::SerialPortMetadata;

pub trait LocalKnotCommandHandler {
    type Error;

    fn handle(&mut self, command: KnotCommand) -> Result<(), Self::Error>;
}

pub fn serial_sensor_from_metadata(metadata: SerialPortMetadata) -> SerialSensor {
    SerialSensor {
        port_name: metadata.port_name,
        serial_number: metadata.serial_number,
        baud_rate: metadata.baud_rate,
    }
}
