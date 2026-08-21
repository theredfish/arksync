// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_actuator::application::protocol::{
    ActuatorBackend, ActuatorConfig, ActuatorConnection, ActuatorDescriptor, ActuatorKind,
    ActuatorProtocol, ActuatorRuleAssertion, ActuatorRuleConfig, ActuatorRuleEffect,
    GpioActuatorConnection,
};
use arksync_knot_protocol::{
    KnotActuatorBackend, KnotActuatorConfig, KnotActuatorConnection, KnotActuatorKind,
    KnotActuatorProtocol, KnotActuatorRuleAssertion, KnotActuatorRuleEffect,
    KnotConfig as ProtocolKnotConfig,
};

use crate::application::{LegacyKnotActuatorConfig, LegacyKnotSensorBinding};
use crate::domain::KnotId;

pub(super) fn legacy_actuator_config(
    hardware_uid: String,
    config: &ProtocolKnotConfig,
) -> LegacyKnotActuatorConfig {
    LegacyKnotActuatorConfig {
        hardware_uid,
        knot_id: KnotId::from_bytes(config.knot_id),
        sensor_bindings: config
            .sensor_bindings
            .iter()
            .map(|binding| LegacyKnotSensorBinding {
                sensor_id: arksync_utils::uuid::Uuid::from_bytes(binding.sensor_id).to_string(),
                device_uid: binding.device_uid.clone(),
            })
            .collect(),
        actuator_configs: config
            .actuator_configs
            .iter()
            .cloned()
            .map(legacy_actuator)
            .collect(),
    }
}

fn legacy_actuator(config: KnotActuatorConfig) -> ActuatorConfig {
    ActuatorConfig {
        config_id: config.config_id,
        version: config.version,
        enabled: config.enabled,
        device_uid: config.device_uid,
        actuator: ActuatorDescriptor {
            id: config.actuator.id,
            kind: match config.actuator.kind {
                KnotActuatorKind::Relay => ActuatorKind::Relay,
            },
            backend: match config.actuator.backend {
                KnotActuatorBackend::LinuxGpiod => ActuatorBackend::LinuxGpiod,
                KnotActuatorBackend::EspGpio => ActuatorBackend::EspGpio,
            },
            protocol: match config.actuator.protocol {
                KnotActuatorProtocol::Gpio => ActuatorProtocol::Gpio,
            },
            connection: match config.actuator.connection {
                KnotActuatorConnection::Gpio(connection) => {
                    ActuatorConnection::Gpio(GpioActuatorConnection {
                        pin: connection.pin,
                        pin_scheme: connection.pin_scheme,
                        active_low: connection.active_low,
                    })
                }
            },
            channels: config.actuator.channels,
            model: config.actuator.model,
        },
        rules: config
            .rules
            .into_iter()
            .map(|rule| ActuatorRuleConfig {
                rule_id: rule.rule_id,
                version: rule.version,
                enabled: rule.enabled,
                sensor_id: rule.sensor_id,
                assertion: match rule.assertion {
                    KnotActuatorRuleAssertion::GreaterThanOrEqual { threshold } => {
                        ActuatorRuleAssertion::GreaterThanOrEqual { threshold }
                    }
                },
                effect: match rule.effect {
                    KnotActuatorRuleEffect::SetActiveWhenMatched {
                        active_when_matched,
                        active_when_unmatched,
                    } => ActuatorRuleEffect::SetActiveWhenMatched {
                        active_when_matched,
                        active_when_unmatched,
                    },
                },
            })
            .collect(),
    }
}
