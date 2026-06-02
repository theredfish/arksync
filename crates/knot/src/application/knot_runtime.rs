// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{KnotCommand, KnotCommandHandler, SerialSensor};
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotRuntimeError {
    SensorAlreadyListening,
    SensorNotListening,
}

#[derive(Default)]
pub struct KnotRuntime {
    listened_serial_sensors: Vec<SerialSensor>,
}

impl KnotRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn listened_serial_sensors(&self) -> &[SerialSensor] {
        &self.listened_serial_sensors
    }

    fn listen_sensor(&mut self, sensor: SerialSensor) -> Result<(), KnotRuntimeError> {
        if self
            .listened_serial_sensors
            .iter()
            .any(|listened_sensor| listened_sensor == &sensor)
        {
            return Err(KnotRuntimeError::SensorAlreadyListening);
        }

        self.listened_serial_sensors.push(sensor);

        Ok(())
    }

    fn stop_listening_sensor(&mut self, sensor: SerialSensor) -> Result<(), KnotRuntimeError> {
        let Some(index) = self
            .listened_serial_sensors
            .iter()
            .position(|listened_sensor| listened_sensor == &sensor)
        else {
            return Err(KnotRuntimeError::SensorNotListening);
        };

        self.listened_serial_sensors.remove(index);

        Ok(())
    }
}

impl KnotCommandHandler for KnotRuntime {
    type Error = KnotRuntimeError;

    fn handle(&mut self, command: KnotCommand) -> Result<(), Self::Error> {
        match command {
            KnotCommand::ListenSensor { sensor } => self.listen_sensor(sensor),
            KnotCommand::StopListeningSensor { sensor } => self.stop_listening_sensor(sensor),
        }
    }
}
