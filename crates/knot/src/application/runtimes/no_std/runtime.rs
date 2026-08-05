// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{KnotCommand, KnotCommandHandler, SerialSensor};
use alloc::vec::Vec;
use core::marker::PhantomData;
#[cfg(feature = "knot-nostd-runtime")]
use {
    crate::application::{KnotConfig, KnotMessage, KnotSensorBinding},
    alloc::string::String,
    arksync_actuator::application::protocol::{ActuatorConfig, ActuatorMessage, AddActuator},
    arksync_actuator::services::ActuatorService,
    arksync_bus::{EventProducer, Timestamp},
};

/// Errors returned by the platform-agnostic Knot runtime command handler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnotRuntimeError {
    /// The requested serial sensor is already tracked by this Knot.
    SensorAlreadyListening,
    /// The requested serial sensor is not tracked by this Knot.
    SensorNotListening,
    /// The runtime received a config before its Knot hardware UID was set.
    #[cfg(feature = "knot-nostd-runtime")]
    KnotHardwareUidNotConfigured,
    /// The actuator config targets another Knot hardware UID.
    #[cfg(feature = "knot-nostd-runtime")]
    KnotHardwareUidMismatch,
}

/// Platform-agnostic Knot runtime state.
///
/// This runtime is the small `no_std` application core for a Knot. It owns the
/// Knot-side command state that does not depend on Tokio, Embassy, Linux, or an
/// ESP32 HAL. Concrete runners wrap it with a platform adapter, such as the
/// Tokio runtime used by the local hub MVP or a future Embassy runtime on ESP32.
#[derive(Default)]
pub struct KnotRuntime<'bus> {
    listened_serial_sensors: Vec<SerialSensor>,
    _bus: PhantomData<&'bus ()>,
    #[cfg(feature = "knot-nostd-runtime")]
    hardware_uid: Option<String>,
    #[cfg(feature = "knot-nostd-runtime")]
    sensor_bindings: Vec<KnotSensorBinding>,
    #[cfg(feature = "knot-nostd-runtime")]
    actuator_service: ActuatorService<'bus>,
}

impl KnotRuntime<'_> {
    /// Builds an empty Knot runtime state.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'bus> KnotRuntime<'bus> {
    /// Returns the serial sensors currently tracked by this Knot runtime.
    pub fn listened_serial_sensors(&self) -> &[SerialSensor] {
        &self.listened_serial_sensors
    }

    /// Stores this Knot's hardware UID used to validate configuration ACKs.
    #[cfg(feature = "knot-nostd-runtime")]
    pub fn with_hardware_uid(mut self, hardware_uid: String) -> Self {
        self.hardware_uid = Some(hardware_uid);
        self
    }

    /// Connects actuator runtime output to an event producer.
    ///
    /// The no_std runtime owns the actuator config state. The concrete runtime
    /// provides the producer so emitted actuator status can be transported by
    /// Tokio, Embassy, MQTT, or another bus adapter.
    #[cfg(feature = "knot-nostd-runtime")]
    pub fn with_actuator_event_producer(
        mut self,
        event_producer: EventProducer<'bus, ActuatorMessage>,
    ) -> Self {
        self.actuator_service = self.actuator_service.with_event_producer(event_producer);
        self
    }

    /// Returns actuator configs currently applied in the runtime.
    #[cfg(feature = "knot-nostd-runtime")]
    pub fn actuator_configs(&self) -> &[ActuatorConfig] {
        self.actuator_service.configs()
    }

    /// Observes one sensor value for actuator rule evaluation.
    ///
    /// The caller must provide the hub-stable `sensor_id` referenced by the
    /// actuator rules. Concrete runtimes are responsible for mapping local
    /// sensor events, such as a device UID, to this stable identifier from their
    /// applied Knot config.
    #[cfg(feature = "knot-nostd-runtime")]
    pub fn observe_actuator_sensor_value(
        &mut self,
        sensor_id: String,
        value: f64,
        occurred_at: Timestamp,
    ) {
        self.actuator_service
            .observe_sensor_value(sensor_id, value, occurred_at);
    }

    /// Observes one local sensor value by device UID for actuator rule evaluation.
    ///
    /// The runtime uses the hub-provided sensor bindings from `KnotConfig` to
    /// translate the physical sensor device UID into the stable `sensor_id`
    /// referenced by rules.
    #[cfg(feature = "knot-nostd-runtime")]
    pub fn observe_actuator_sensor_device_value(
        &mut self,
        device_uid: &str,
        value: f64,
        occurred_at: Timestamp,
    ) {
        let Some(binding) = self
            .sensor_bindings
            .iter()
            .find(|binding| binding.device_uid == device_uid)
        else {
            #[cfg(feature = "log")]
            log::debug!(
                "Knot actuator runtime ignored sensor value with no binding device_uid={} value={}",
                device_uid,
                value
            );
            return;
        };

        #[cfg(feature = "log")]
        log::debug!(
            "Knot actuator runtime mapped sensor device_uid={} to sensor_id={} value={}",
            device_uid,
            binding.sensor_id,
            value
        );
        self.actuator_service
            .observe_sensor_value(binding.sensor_id.clone(), value, occurred_at);
    }

    /// Applies one Knot protocol message to the no_std runtime state.
    ///
    /// `Ack` applies the hub-provided actuator configuration for this Knot,
    /// `Actuator` applies a direct actuator command such as enable/disable, and
    /// `Hello` is ignored because it is produced by concrete runtimes.
    #[cfg(feature = "knot-nostd-runtime")]
    pub fn handle_knot_message(
        &mut self,
        event: KnotMessage,
        occurred_at: Timestamp,
    ) -> Result<(), KnotRuntimeError> {
        match event {
            KnotMessage::Ack(ack) => self.apply_actuator_config(ack.config, occurred_at),
            KnotMessage::Actuator(message) => {
                self.actuator_service.handle_message(message, occurred_at);
                Ok(())
            }
            KnotMessage::Hello(_) => Ok(()),
        }
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

    #[cfg(feature = "knot-nostd-runtime")]
    fn apply_actuator_config(
        &mut self,
        config: KnotConfig,
        occurred_at: Timestamp,
    ) -> Result<(), KnotRuntimeError> {
        let Some(hardware_uid) = &self.hardware_uid else {
            return Err(KnotRuntimeError::KnotHardwareUidNotConfigured);
        };

        if config.hardware_uid != *hardware_uid {
            return Err(KnotRuntimeError::KnotHardwareUidMismatch);
        }

        #[cfg(feature = "log")]
        log::info!(
            "Knot actuator runtime applying config knot_hardware_uid={} sensor_bindings={} actuator_configs={}",
            config.hardware_uid,
            config.sensor_bindings.len(),
            config.actuator_configs.len()
        );

        self.sensor_bindings = config.sensor_bindings;

        for config in config.actuator_configs {
            #[cfg(feature = "log")]
            log::info!(
                "Knot actuator runtime applies actuator config config_id={} rules={}",
                config.config_id,
                config.rules.len()
            );
            self.actuator_service.handle_message(
                ActuatorMessage::AddActuator(AddActuator { config }),
                occurred_at,
            );
        }

        Ok(())
    }
}

impl KnotCommandHandler for KnotRuntime<'_> {
    type Error = KnotRuntimeError;

    fn handle(&mut self, command: KnotCommand) -> Result<(), Self::Error> {
        match command {
            KnotCommand::ListenSensor { sensor } => self.listen_sensor(sensor),
            KnotCommand::StopListeningSensor { sensor } => self.stop_listening_sensor(sensor),
        }
    }
}
