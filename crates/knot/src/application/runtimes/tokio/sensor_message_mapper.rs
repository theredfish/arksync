// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_knot_protocol::{
    KnotMeasurementUnit, KnotSensorConnection, KnotSensorDescriptor, KnotSensorKind,
    KnotSensorMeasurement, KnotSensorMeasurementBatch, KnotSensorMessage, KnotSensorPlugged,
    KnotSensorProvisioned, KnotSensorProvisioningConflict, KnotSerialPort,
};
use arksync_sensor::infrastructure::events::{
    MeasuredSensor, MeasurementUnit, SensorConnectionMetadata, SensorEvent, SensorKind,
};
use arksync_sensor::serial_port::SerialPortMetadata;

pub(super) fn knot_sensor_message(event: SensorEvent) -> KnotSensorMessage {
    match event {
        SensorEvent::SerialSensorPlugged(plugged) => {
            KnotSensorMessage::Plugged(KnotSensorPlugged {
                connection: KnotSensorConnection::Uart(serial_port(plugged.metadata)),
            })
        }
        SensorEvent::SensorProvisioned(provisioned) => {
            KnotSensorMessage::Provisioned(KnotSensorProvisioned {
                device_uid: provisioned.device_uid.to_string(),
                sensor: sensor_descriptor(provisioned.sensor),
            })
        }
        SensorEvent::SensorProvisioningConflict(conflict) => {
            KnotSensorMessage::ProvisioningConflict(KnotSensorProvisioningConflict {
                reason: conflict.reason,
                sensor: sensor_descriptor(conflict.sensor),
            })
        }
        SensorEvent::SensorMeasurementRecorded(recorded) => {
            KnotSensorMessage::Measurements(KnotSensorMeasurementBatch {
                measurements: alloc::vec![KnotSensorMeasurement {
                    device_uid: recorded.device_uid.to_string(),
                    sensor: sensor_descriptor(recorded.sensor),
                    value: recorded.measurement.value,
                    unit: measurement_unit(recorded.measurement.unit),
                }],
            })
        }
    }
}

fn sensor_descriptor(sensor: MeasuredSensor) -> KnotSensorDescriptor {
    KnotSensorDescriptor {
        hardware_uid: sensor.hardware_uid,
        kind: match sensor.kind {
            SensorKind::Temperature => KnotSensorKind::Temperature,
            SensorKind::Custom => KnotSensorKind::Custom,
        },
        connection: match sensor.connection {
            SensorConnectionMetadata::Uart(metadata) => {
                KnotSensorConnection::Uart(serial_port(metadata))
            }
            SensorConnectionMetadata::I2c { address } => KnotSensorConnection::I2c { address },
        },
        firmware: sensor.firmware,
    }
}

fn serial_port(metadata: SerialPortMetadata) -> KnotSerialPort {
    KnotSerialPort {
        port_name: metadata.port_name,
        serial_number: metadata.serial_number,
        baud_rate: metadata.baud_rate,
    }
}

fn measurement_unit(unit: MeasurementUnit) -> KnotMeasurementUnit {
    match unit {
        MeasurementUnit::Celsius => KnotMeasurementUnit::Celsius,
        MeasurementUnit::Raw => KnotMeasurementUnit::Raw,
    }
}
