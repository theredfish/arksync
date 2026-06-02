// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::HubSensorError;
use crate::domain::{
    ObservedSerialSensor, PluggedSensor, RegisteredSensor, Sensor, SensorId, SensorMeasurement,
    SensorOverview, SensorRegistrationStatus,
};
use crate::infrastructure::store::{
    insert_sensor as store_insert_sensor, list_sensors as store_list_sensors, sensor_kind_as_str,
    NewSensorRecord,
};
use arksync_bus::Timestamp;
use arksync_knot::domain::KnotEventSource;
use arksync_sensor::infrastructure::events::{SensorConnectionMetadata, SensorMeasurementRecorded};
use arksync_sensor::sensor::DEFAULT_MEASUREMENT_INTERVAL;
use arksync_sensor::serial_port::SerialPortMetadata;
use sqlx::PgExecutor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubError {
    DuplicateSensorId,
    SensorAlreadyRegistered,
    SensorNotFound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterSensor {
    pub sensor_id: SensorId,
    pub display_name: String,
    pub metadata: SerialPortMetadata,
    pub registered_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameSensor {
    pub sensor_id: SensorId,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoveSensor {
    pub sensor_id: SensorId,
}

pub async fn list_sensors<'e, E>(executor: E) -> Result<Vec<Sensor>, HubSensorError>
where
    E: PgExecutor<'e>,
{
    let records = store_list_sensors(executor).await?;

    Ok(records.into_iter().map(Sensor::from).collect())
}

pub async fn insert_sensor<'e, E>(
    executor: E,
    sensor: PluggedSensor,
) -> Result<Sensor, HubSensorError>
where
    E: PgExecutor<'e>,
{
    let record = NewSensorRecord {
        station_knot_id: sensor.station_knot_id.as_uuid(),
        device_uid: sensor.device_uid.as_ref().to_string(),
        display_name: None,
        sensor_kind: sensor_kind_as_str(sensor.kind).to_string(),
        driver: "atlas_scientific_ezo".to_string(),
        protocol: sensor_protocol_as_str(&sensor.connection).to_string(),
        connection: serde_json::to_value(&sensor.connection)
            .expect("sensor connection metadata should serialize"),
        firmware: sensor.firmware,
        measurement_interval_ms: sensor.measurement_interval_ms,
    };

    let sensor = store_insert_sensor(executor, &record).await?;

    Ok(sensor.into())
}

#[derive(Default)]
pub struct Hub {
    registered_sensors: Vec<RegisteredSensor>,
    observed_serial_sensors: Vec<ObservedSerialSensor>,
    sensor_measurements: Vec<SensorMeasurement>,
}

impl Hub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_sensor(&mut self, command: RegisterSensor) -> Result<(), HubError> {
        if self
            .registered_sensors
            .iter()
            .any(|sensor| sensor.id == command.sensor_id)
        {
            return Err(HubError::DuplicateSensorId);
        }

        if self
            .registered_sensors
            .iter()
            .any(|sensor| sensor.metadata == command.metadata)
        {
            return Err(HubError::SensorAlreadyRegistered);
        }

        self.registered_sensors.push(RegisteredSensor {
            id: command.sensor_id,
            display_name: command.display_name,
            metadata: command.metadata,
            registered_at: command.registered_at,
        });

        Ok(())
    }

    pub fn rename_sensor(&mut self, command: RenameSensor) -> Result<(), HubError> {
        let sensor = self
            .registered_sensors
            .iter_mut()
            .find(|sensor| sensor.id == command.sensor_id)
            .ok_or(HubError::SensorNotFound)?;

        sensor.display_name = command.display_name;

        Ok(())
    }

    pub fn remove_sensor(&mut self, command: RemoveSensor) -> Result<(), HubError> {
        let Some(index) = self
            .registered_sensors
            .iter()
            .position(|sensor| sensor.id == command.sensor_id)
        else {
            return Err(HubError::SensorNotFound);
        };

        self.registered_sensors.remove(index);

        Ok(())
    }

    pub fn list_sensors_overview(&self) -> Vec<SensorOverview> {
        let mut overview = Vec::new();

        for observed in &self.observed_serial_sensors {
            let registered = self
                .registered_sensors
                .iter()
                .find(|sensor| sensor.metadata == observed.metadata);

            overview.push(SensorOverview::from_observed(observed, registered));
        }

        for registered in &self.registered_sensors {
            if self
                .observed_serial_sensors
                .iter()
                .any(|observed| observed.metadata == registered.metadata)
            {
                continue;
            }

            overview.push(SensorOverview {
                sensor_id: Some(registered.id),
                display_name: registered.display_name.clone(),
                metadata: registered.metadata.clone(),
                status: SensorRegistrationStatus::Registered,
                first_observed_at: None,
                last_observed_at: None,
                last_received_at: None,
            });
        }

        overview
    }

    pub fn list_sensor_measurements(&self) -> Vec<SensorMeasurement> {
        self.sensor_measurements.clone()
    }

    pub(crate) fn observe_serial_sensor(
        &mut self,
        source: KnotEventSource,
        metadata: SerialPortMetadata,
        observed_at: Timestamp,
        received_at: Timestamp,
    ) {
        if let Some(observed) = self
            .observed_serial_sensors
            .iter_mut()
            .find(|sensor| sensor.metadata == metadata)
        {
            observed.source = source;
            observed.last_observed_at = observed_at;
            observed.last_received_at = received_at;
            return;
        }

        self.observed_serial_sensors.push(ObservedSerialSensor {
            source,
            metadata,
            first_observed_at: observed_at,
            last_observed_at: observed_at,
            last_received_at: received_at,
        });
    }

    pub(crate) fn record_sensor_measurement(
        &mut self,
        source: KnotEventSource,
        sensor_id: SensorId,
        event: SensorMeasurementRecorded,
        measured_at: Timestamp,
        received_at: Timestamp,
    ) {
        self.sensor_measurements.push(SensorMeasurement {
            source,
            sensor_id,
            kind: event.sensor.kind,
            value: event.measurement.value,
            unit: event.measurement.unit,
            measured_at,
            received_at,
        });
    }
}

pub fn default_measurement_interval_ms() -> i32 {
    DEFAULT_MEASUREMENT_INTERVAL.as_millis() as i32
}

fn sensor_protocol_as_str(connection: &SensorConnectionMetadata) -> &'static str {
    match connection {
        SensorConnectionMetadata::Uart(_) => "uart",
        SensorConnectionMetadata::I2c { .. } => "i2c",
    }
}
