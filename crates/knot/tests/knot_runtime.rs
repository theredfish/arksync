// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_knot::application::{
    KnotCommand, KnotCommandHandler, KnotRuntime, KnotRuntimeError, SerialSensor,
};
#[cfg(feature = "knot-nostd-runtime")]
use {
    arksync_actuator::infrastructure::events::{
        ActuatorBackend, ActuatorConfig, ActuatorConnection, ActuatorDescriptor, ActuatorKind,
        ActuatorProtocol, GpioActuatorConnection,
    },
    arksync_bus::Timestamp,
    arksync_knot::application::{KnotAck, KnotConfig, KnotMessage},
    arksync_knot::domain::KnotId,
};

fn serial_sensor(serial_number: &str) -> SerialSensor {
    SerialSensor {
        port_name: "/dev/ttyUSB0".to_string(),
        serial_number: serial_number.to_string(),
        baud_rate: 9_600,
    }
}

#[test]
fn knot_runtime_handles_sensor_listening_commands() {
    let sensor = serial_sensor("rtd-serial-1");
    let mut runtime = KnotRuntime::new();

    runtime
        .handle(KnotCommand::ListenSensor {
            sensor: sensor.clone(),
        })
        .unwrap();

    assert_eq!(runtime.listened_serial_sensors(), &[sensor]);
}

#[test]
fn knot_runtime_rejects_duplicate_listening_commands() {
    let sensor = serial_sensor("rtd-serial-1");
    let mut runtime = KnotRuntime::new();

    runtime
        .handle(KnotCommand::ListenSensor {
            sensor: sensor.clone(),
        })
        .unwrap();
    let result = runtime.handle(KnotCommand::ListenSensor { sensor });

    assert_eq!(result, Err(KnotRuntimeError::SensorAlreadyListening));
}

#[cfg(feature = "knot-nostd-runtime")]
fn actuator_config(config_id: &str) -> ActuatorConfig {
    ActuatorConfig {
        config_id: config_id.to_string(),
        version: 1,
        enabled: true,
        device_uid: "relay-gpio17".to_string(),
        actuator: ActuatorDescriptor {
            id: "actuator-1".to_string(),
            kind: ActuatorKind::Relay,
            backend: ActuatorBackend::LinuxGpiod,
            protocol: ActuatorProtocol::Gpio,
            connection: ActuatorConnection::Gpio(GpioActuatorConnection {
                pin: 17,
                pin_scheme: Some("bcm".to_string()),
                active_low: true,
            }),
            channels: None,
            model: None,
        },
        rules: vec![],
    }
}

#[cfg(feature = "knot-nostd-runtime")]
fn timestamp() -> Timestamp {
    Timestamp::from_unix_millis(1_780_000_000_000)
}

#[cfg(feature = "knot-nostd-runtime")]
#[test]
fn knot_runtime_applies_actuator_config_ack() {
    let config = actuator_config("relay-config-1");
    let mut runtime = KnotRuntime::new().with_actuator_hardware_uid("knot-rpi-1".to_string());

    runtime
        .handle_knot_message(
            KnotMessage::Ack(KnotAck {
                config: KnotConfig {
                    hardware_uid: "knot-rpi-1".to_string(),
                    knot_id: KnotId::from_bytes([2; 16]),
                    sensor_bindings: vec![],
                    actuator_configs: vec![config.clone()],
                },
            }),
            timestamp(),
        )
        .unwrap();

    assert_eq!(runtime.actuator_configs(), &[config]);
}

#[cfg(feature = "knot-nostd-runtime")]
#[test]
fn knot_runtime_rejects_actuator_config_for_another_hardware_uid() {
    let mut runtime = KnotRuntime::new().with_actuator_hardware_uid("knot-rpi-1".to_string());
    let result = runtime.handle_knot_message(
        KnotMessage::Ack(KnotAck {
            config: KnotConfig {
                hardware_uid: "another-knot".to_string(),
                knot_id: KnotId::from_bytes([2; 16]),
                sensor_bindings: vec![],
                actuator_configs: vec![actuator_config("relay-config-1")],
            },
        }),
        timestamp(),
    );

    assert_eq!(result, Err(KnotRuntimeError::ActuatorHardwareUidMismatch));
    assert!(runtime.actuator_configs().is_empty());
}
