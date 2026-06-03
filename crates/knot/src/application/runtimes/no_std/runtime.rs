// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{KnotCommand, KnotCommandHandler, SerialSensor};
use alloc::vec::Vec;

/// Errors returned by the platform-agnostic Knot runtime command handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotRuntimeError {
    /// The requested serial sensor is already tracked by this Knot.
    SensorAlreadyListening,
    /// The requested serial sensor is not tracked by this Knot.
    SensorNotListening,
}

/// Platform-agnostic Knot runtime state.
///
/// This runtime is the small `no_std` application core for a Knot. It owns the
/// Knot-side command state that does not depend on Tokio, Embassy, Linux, or an
/// ESP32 HAL. Concrete runners wrap it with a platform adapter, such as the
/// Tokio runtime used by the local hub MVP or a future Embassy runtime on ESP32.
#[derive(Default)]
pub struct KnotRuntime {
    listened_serial_sensors: Vec<SerialSensor>,
}

impl KnotRuntime {
    /// Builds an empty Knot runtime state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the serial sensors currently tracked by this Knot runtime.
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
