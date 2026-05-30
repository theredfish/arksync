// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::LocalKnotCommandHandler;
use crate::domain::{
    ObservedSerialSensor, RegisteredSensor, SensorId, SensorOverview, SensorRegistrationStatus,
};
use alloc::string::String;
use alloc::vec::Vec;
use arksync_bus::Timestamp;
use arksync_knot::application::KnotCommand;
use arksync_knot::domain::KnotEventSource;
use arksync_sensor::serial_port::SerialPortMetadata;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HubError {
    DuplicateSensorId,
    SensorAlreadyRegistered,
    SensorNotFound,
    KnotCommandRejected,
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

#[derive(Default)]
pub struct Hub {
    registered_sensors: Vec<RegisteredSensor>,
    observed_serial_sensors: Vec<ObservedSerialSensor>,
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

    pub fn handle_local_knot_command<Handler>(
        &mut self,
        handler: &mut Handler,
        command: KnotCommand,
    ) -> Result<(), HubError>
    where
        Handler: LocalKnotCommandHandler,
    {
        handler
            .handle(command)
            .map_err(|_| HubError::KnotCommandRejected)
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
}
