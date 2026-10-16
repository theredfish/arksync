// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, Timestamp};
use arksync_knot::domain::{KnotEventSource, KnotHubId, KnotId};
use arksync_protocol::knot::{
    KnotEnvelope, KnotMeasurementUnit, KnotMessage, KnotSensorConnection, KnotSensorDescriptor,
    KnotSensorKind, KnotSensorMessage,
};
use arksync_protocol::ArkSyncActor;
use arksync_sensor::device_uid::DeviceUid;
use arksync_sensor::infrastructure::events::{
    MeasuredSensor, MeasurementUnit, SensorConnectionMetadata, SensorEvent, SensorMeasurement,
    SensorMeasurementRecorded, SensorProvisioned, SensorProvisioningConflict, SerialSensorPlugged,
};
use arksync_sensor::serial_port::SerialPortMetadata;
use eyre::{eyre, Result, WrapErr};
use sqlx::{PgPool, PgTransaction};

use crate::application::{
    default_measurement_interval_ms, insert_sensor, record_sensor_measurement,
    HubSensorEventEnvelope, SensorMeasurementInput, SensorRegistry,
};
use crate::domain::{HubId, PluggedSensor, SensorId};
use crate::infrastructure::store::{
    insert_knot_message_receipt, knot as knot_store, sensors as sensor_store, KnotRecord,
    SensorStoreError,
};

pub(super) async fn handle_knot_sensor_protocol(
    pool: &PgPool,
    hub_id: HubId,
    event: &KnotEnvelope,
    received_at: Timestamp,
    sensor_registry: &mut SensorRegistry,
) -> Result<Vec<HubSensorEventEnvelope>> {
    let ArkSyncActor::Knot { hardware_uid } = &event.source else {
        return Err(InvalidSensorPayload.into());
    };
    let KnotMessage::Sensor(message) = &event.payload else {
        return Err(InvalidSensorPayload.into());
    };
    let event_id = event.id;
    let occurred_at = event.occurred_at;
    let mut txn = pool
        .begin()
        .await
        .wrap_err("failed to begin Knot sensor message transaction")?;
    let knot = knot_store::station_knot_by_hardware_uid(&mut *txn, hub_id, hardware_uid)
        .await
        .map_err(|error| match error {
            knot_store::KnotStoreError::NotFound => eyre::Report::new(InvalidSensorPayload),
            error => eyre!("failed to load Knot for sensor message: {error:?}"),
        })?;
    let is_new = insert_knot_message_receipt(
        &mut *txn,
        hub_id,
        event_id,
        hardware_uid,
        sensor_message_kind(message),
    )
    .await
    .map_err(|error| eyre!("Knot message store error: {error:?}"))?;

    if !is_new {
        txn.commit()
            .await
            .wrap_err("failed to commit duplicate Knot sensor message transaction")?;
        return Ok(Vec::new());
    }

    let source = knot_source(&knot);
    let mut projected_events = Vec::new();
    let mut remembered_sensors = Vec::new();

    match message {
        KnotSensorMessage::Plugged(plugged) => {
            if let KnotSensorConnection::Uart(metadata) = &plugged.connection {
                projected_events.push(EventEnvelope::new_with_id(
                    event_id,
                    source,
                    occurred_at,
                    SensorEvent::SerialSensorPlugged(SerialSensorPlugged {
                        metadata: serial_port(metadata),
                    }),
                ));
            }
        }
        KnotSensorMessage::Provisioned(provisioned) => {
            let device_uid = device_uid(&provisioned.device_uid)?;
            let sensor = measured_sensor(&provisioned.sensor);
            let sensor_id = ensure_sensor(
                &mut txn,
                sensor_registry,
                &knot,
                device_uid.clone(),
                &sensor,
            )
            .await?;
            remembered_sensors.push((device_uid.to_string(), sensor_id));
            projected_events.push(EventEnvelope::new_with_id(
                event_id,
                source,
                occurred_at,
                SensorEvent::SensorProvisioned(SensorProvisioned { device_uid, sensor }),
            ));
        }
        KnotSensorMessage::ProvisioningConflict(conflict) => {
            projected_events.push(EventEnvelope::new_with_id(
                event_id,
                source,
                occurred_at,
                SensorEvent::SensorProvisioningConflict(SensorProvisioningConflict {
                    reason: conflict.reason.clone(),
                    sensor: measured_sensor(&conflict.sensor),
                }),
            ));
        }
        KnotSensorMessage::Measurements(batch) => {
            for measurement in &batch.measurements {
                let device_uid = device_uid(&measurement.device_uid)?;
                let sensor = measured_sensor(&measurement.sensor);
                let sensor_id = ensure_sensor(
                    &mut txn,
                    sensor_registry,
                    &knot,
                    device_uid.clone(),
                    &sensor,
                )
                .await?;
                record_sensor_measurement(
                    &mut *txn,
                    SensorMeasurementInput {
                        event_id,
                        source: source.clone(),
                        sensor_id,
                        kind: sensor.kind,
                        value: measurement.value,
                        unit: measurement_unit(measurement.unit),
                        measured_at: occurred_at,
                        received_at,
                    },
                )
                .await
                .wrap_err("failed to persist Knot sensor measurement")?;
                remembered_sensors.push((device_uid.to_string(), sensor_id));
                projected_events.push(EventEnvelope::new_with_id(
                    event_id,
                    source.clone(),
                    occurred_at,
                    SensorEvent::SensorMeasurementRecorded(SensorMeasurementRecorded {
                        device_uid,
                        sensor: sensor.clone(),
                        measurement: SensorMeasurement {
                            value: measurement.value,
                            unit: measurement_unit(measurement.unit),
                        },
                    }),
                ));
            }
        }
    }

    txn.commit()
        .await
        .wrap_err("failed to commit Knot sensor message transaction")?;

    for (device_uid, sensor_id) in remembered_sensors {
        sensor_registry.remember_sensor(knot.id, device_uid, sensor_id);
    }

    Ok(projected_events)
}

async fn ensure_sensor(
    txn: &mut PgTransaction<'_>,
    registry: &SensorRegistry,
    knot: &KnotRecord,
    device_uid: DeviceUid,
    sensor: &MeasuredSensor,
) -> Result<SensorId> {
    if let Some(sensor_id) = registry.sensor_id(knot.id, device_uid.as_ref()) {
        return Ok(sensor_id);
    }

    match sensor_store::sensor_by_station_knot_id_and_device_uid(
        &mut **txn,
        knot.id,
        device_uid.as_ref(),
    )
    .await
    {
        Ok(record) => return Ok(record.id.into()),
        Err(SensorStoreError::NotFound) => {}
        Err(error) => return Err(eyre!("failed to load sensor identity: {error:?}")),
    }

    let inserted = insert_sensor(
        &mut **txn,
        PluggedSensor {
            station_knot_id: KnotId::from(knot.id),
            device_uid,
            kind: sensor.kind,
            connection: sensor.connection.clone(),
            firmware: sensor.firmware,
            measurement_interval_ms: default_measurement_interval_ms(),
        },
    )
    .await
    .wrap_err("failed to register sensor from Knot message")?;

    Ok(inserted.id)
}

fn knot_source(knot: &KnotRecord) -> KnotEventSource {
    KnotEventSource::Knot {
        hub_id: KnotHubId::from(knot.hub_id),
        knot_id: KnotId::from(knot.id),
    }
}

fn measured_sensor(sensor: &KnotSensorDescriptor) -> MeasuredSensor {
    MeasuredSensor {
        hardware_uid: sensor.hardware_uid.clone(),
        kind: match sensor.kind {
            KnotSensorKind::Temperature => {
                arksync_sensor::infrastructure::events::SensorKind::Temperature
            }
            KnotSensorKind::Custom => arksync_sensor::infrastructure::events::SensorKind::Custom,
        },
        connection: match &sensor.connection {
            KnotSensorConnection::Uart(metadata) => {
                SensorConnectionMetadata::Uart(serial_port(metadata))
            }
            KnotSensorConnection::I2c { address } => {
                SensorConnectionMetadata::I2c { address: *address }
            }
        },
        firmware: sensor.firmware,
    }
}

fn serial_port(metadata: &arksync_protocol::knot::KnotSerialPort) -> SerialPortMetadata {
    SerialPortMetadata {
        port_name: metadata.port_name.clone(),
        serial_number: metadata.serial_number.clone(),
        baud_rate: metadata.baud_rate,
    }
}

fn measurement_unit(unit: KnotMeasurementUnit) -> MeasurementUnit {
    match unit {
        KnotMeasurementUnit::Celsius => MeasurementUnit::Celsius,
        KnotMeasurementUnit::Raw => MeasurementUnit::Raw,
    }
}

#[derive(Debug)]
pub(super) struct InvalidSensorPayload;

impl core::fmt::Display for InvalidSensorPayload {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("invalid Knot sensor payload")
    }
}

impl std::error::Error for InvalidSensorPayload {}

fn device_uid(value: &str) -> Result<DeviceUid> {
    DeviceUid::try_from(value).map_err(|_| InvalidSensorPayload.into())
}

fn sensor_message_kind(message: &KnotSensorMessage) -> &'static str {
    match message {
        KnotSensorMessage::Plugged(_) => "sensor_plugged",
        KnotSensorMessage::Provisioned(_) => "sensor_provisioned",
        KnotSensorMessage::ProvisioningConflict(_) => "sensor_provisioning_conflict",
        KnotSensorMessage::Measurements(_) => "sensor_measurements",
    }
}
