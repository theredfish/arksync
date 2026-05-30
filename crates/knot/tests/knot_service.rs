// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_knot::application::{
    KnotCommand, KnotCommandHandler, KnotService, KnotServiceError, SerialSensor,
};

fn serial_sensor(serial_number: &str) -> SerialSensor {
    SerialSensor {
        port_name: "/dev/ttyUSB0".to_string(),
        serial_number: serial_number.to_string(),
        baud_rate: 9_600,
    }
}

#[test]
fn knot_service_handles_sensor_listening_commands() {
    let sensor = serial_sensor("rtd-serial-1");
    let mut service = KnotService::new();

    service
        .handle(KnotCommand::ListenSensor {
            sensor: sensor.clone(),
        })
        .unwrap();

    assert_eq!(service.listened_serial_sensors(), &[sensor]);
}

#[test]
fn knot_service_rejects_duplicate_listening_commands() {
    let sensor = serial_sensor("rtd-serial-1");
    let mut service = KnotService::new();

    service
        .handle(KnotCommand::ListenSensor {
            sensor: sensor.clone(),
        })
        .unwrap();
    let result = service.handle(KnotCommand::ListenSensor { sensor });

    assert_eq!(result, Err(KnotServiceError::SensorAlreadyListening));
}
