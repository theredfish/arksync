// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::application::protocol::{
    ActuatorConfig, ActuatorMessage, ActuatorRuntimeStatus, ActuatorStateChanged, AddActuator,
    ConfigApplied, ConfigRejected, RemoveActuator, RuntimeStatus,
};
use crate::rule_engine::{RuleEngine, SensorValue};
use arksync_bus::{EventEnvelope, EventId, EventPublisher, Timestamp};

pub struct ActuatorService<'bus> {
    configs: Vec<ActuatorConfig>,
    rule_engine: RuleEngine,
    event_publisher: Option<EventPublisher<'bus, ActuatorMessage>>,
    event_counter: u128,
}

impl Default for ActuatorService<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'bus> ActuatorService<'bus> {
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
            rule_engine: RuleEngine::new(),
            event_publisher: None,
            event_counter: 0,
        }
    }

    pub fn with_event_publisher(
        mut self,
        event_publisher: EventPublisher<'bus, ActuatorMessage>,
    ) -> Self {
        self.event_publisher = Some(event_publisher);
        self
    }

    pub fn configs(&self) -> &[ActuatorConfig] {
        &self.configs
    }

    pub fn handle_message(&mut self, message: ActuatorMessage, occurred_at: Timestamp) {
        match message {
            ActuatorMessage::AddActuator(command) => self.add_actuator(command, occurred_at),
            ActuatorMessage::EnableActuator(command) => {
                self.set_enabled(command.config_id, command.version, true, occurred_at);
            }
            ActuatorMessage::DisableActuator(command) => {
                self.set_enabled(command.config_id, command.version, false, occurred_at);
            }
            ActuatorMessage::RemoveActuator(command) => self.remove_actuator(command, occurred_at),
            ActuatorMessage::ConfigApplied(_)
            | ActuatorMessage::ConfigRejected(_)
            | ActuatorMessage::RuntimeStatus(_)
            | ActuatorMessage::ActuatorStateChanged(_) => {}
        }
    }

    pub fn observe_sensor_value(
        &mut self,
        sensor_id: alloc::string::String,
        value: f64,
        occurred_at: Timestamp,
    ) {
        let decisions = self.rule_engine.evaluate(&SensorValue { sensor_id, value });
        if decisions.is_empty() {
            return;
        }

        for decision in decisions {
            self.emit(
                occurred_at,
                ActuatorMessage::ActuatorStateChanged(ActuatorStateChanged {
                    config_id: decision.config_id,
                    actuator_id: decision.actuator_id,
                    rule_id: decision.rule_id,
                    sensor_id: decision.sensor_id,
                    sensor_value: decision.sensor_value,
                    active: decision.active,
                }),
            );
        }

        self.emit_runtime_status(occurred_at);
    }

    fn add_actuator(&mut self, command: AddActuator, occurred_at: Timestamp) {
        let config_id = command.config.config_id.clone();
        let version = command.config.version;

        if let Some(config) = self
            .configs
            .iter_mut()
            .find(|config| config.config_id == config_id)
        {
            if config.version > version {
                self.reject_config(
                    config_id,
                    version,
                    "received stale actuator config version",
                    occurred_at,
                );
                return;
            }

            *config = command.config;
        } else {
            self.configs.push(command.config);
        }

        self.reload_rules();
        self.apply_config(config_id, version, occurred_at);
    }

    fn set_enabled(
        &mut self,
        config_id: alloc::string::String,
        version: u64,
        enabled: bool,
        occurred_at: Timestamp,
    ) {
        let Some(config) = self
            .configs
            .iter_mut()
            .find(|config| config.config_id == config_id)
        else {
            self.reject_config(config_id, version, "actuator config not found", occurred_at);
            return;
        };

        if config.version > version {
            self.reject_config(
                config_id,
                version,
                "received stale actuator config version",
                occurred_at,
            );
            return;
        }

        config.version = version;
        config.enabled = enabled;
        self.reload_rules();
        self.apply_config(config_id, version, occurred_at);
    }

    fn remove_actuator(&mut self, command: RemoveActuator, occurred_at: Timestamp) {
        let Some(index) = self
            .configs
            .iter()
            .position(|config| config.config_id == command.config_id)
        else {
            self.reject_config(
                command.config_id,
                command.version,
                "actuator config not found",
                occurred_at,
            );
            return;
        };

        let removed = self.configs.remove(index);
        self.reload_rules();
        self.apply_config(removed.config_id, command.version, occurred_at);
    }

    fn apply_config(
        &mut self,
        config_id: alloc::string::String,
        version: u64,
        occurred_at: Timestamp,
    ) {
        self.emit(
            occurred_at,
            ActuatorMessage::ConfigApplied(ConfigApplied { config_id, version }),
        );
        self.emit_runtime_status(occurred_at);
    }

    fn reject_config(
        &mut self,
        config_id: alloc::string::String,
        version: u64,
        reason: &str,
        occurred_at: Timestamp,
    ) {
        self.emit(
            occurred_at,
            ActuatorMessage::ConfigRejected(ConfigRejected {
                config_id,
                version,
                reason: reason.to_string(),
            }),
        );
        self.emit_runtime_status(occurred_at);
    }

    fn emit_runtime_status(&mut self, occurred_at: Timestamp) {
        self.emit(
            occurred_at,
            ActuatorMessage::RuntimeStatus(RuntimeStatus {
                rules: self.rule_engine.statuses(),
                actuators: self
                    .configs
                    .iter()
                    .map(|config| ActuatorRuntimeStatus {
                        config_id: config.config_id.clone(),
                        version: config.version,
                        enabled: config.enabled,
                    })
                    .collect(),
                last_seen_sensor_values: Vec::new(),
            }),
        );
    }

    fn reload_rules(&mut self) {
        self.rule_engine.replace_actuator_configs(&self.configs);
    }

    fn emit(&mut self, occurred_at: Timestamp, event: ActuatorMessage) {
        self.event_counter = self.event_counter.wrapping_add(1);
        let Some(event_publisher) = &mut self.event_publisher else {
            return;
        };
        let _ = event_publisher.publish(EventEnvelope::new_with_id(
            EventId::from_bytes(self.event_counter.to_be_bytes()),
            (),
            occurred_at,
            event,
        ));
    }
}
