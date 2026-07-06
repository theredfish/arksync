// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use alloc::string::String;
use alloc::vec::Vec;

use crate::application::protocol::{
    ActuatorConfig, ActuatorRuleAssertion, ActuatorRuleConfig, ActuatorRuleEffect,
    RuleRuntimeStatus,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SensorValue {
    pub sensor_id: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActuatorDecision {
    pub config_id: String,
    pub actuator_id: String,
    pub rule_id: String,
    pub sensor_id: String,
    pub sensor_value: f64,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct RuntimeRule {
    config_id: String,
    actuator_id: String,
    rule: ActuatorRuleConfig,
    last_active: Option<bool>,
}

#[derive(Default)]
pub struct RuleEngine {
    rules: Vec<RuntimeRule>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replace_actuator_configs<'a>(
        &mut self,
        configs: impl IntoIterator<Item = &'a ActuatorConfig>,
    ) {
        let previous_rules = core::mem::take(&mut self.rules);

        self.rules = configs
            .into_iter()
            .flat_map(|config| {
                let previous_rules = &previous_rules;

                config.rules.iter().cloned().map(move |rule| {
                    let last_active = previous_rules
                        .iter()
                        .find(|runtime_rule| {
                            runtime_rule.config_id == config.config_id
                                && runtime_rule.rule.rule_id == rule.rule_id
                        })
                        .and_then(|runtime_rule| runtime_rule.last_active);

                    RuntimeRule {
                        config_id: config.config_id.clone(),
                        actuator_id: config.actuator.id.clone(),
                        rule,
                        last_active,
                    }
                })
            })
            .collect();
    }

    pub fn evaluate(&mut self, value: &SensorValue) -> Vec<ActuatorDecision> {
        self.rules
            .iter_mut()
            .filter_map(|runtime_rule| runtime_rule.evaluate(value))
            .collect()
    }

    pub fn statuses(&self) -> Vec<RuleRuntimeStatus> {
        self.rules
            .iter()
            .map(|runtime_rule| RuleRuntimeStatus {
                rule_id: runtime_rule.rule.rule_id.clone(),
                version: runtime_rule.rule.version,
                enabled: runtime_rule.rule.enabled,
            })
            .collect()
    }
}

impl RuntimeRule {
    fn evaluate(&mut self, value: &SensorValue) -> Option<ActuatorDecision> {
        if !self.rule.enabled || self.rule.sensor_id != value.sensor_id {
            return None;
        }

        let matched = self.rule.assertion.evaluate(value.value);
        let active = self.rule.effect.active_for_match(matched);

        if self.last_active == Some(active) {
            return None;
        }

        self.last_active = Some(active);

        Some(ActuatorDecision {
            config_id: self.config_id.clone(),
            actuator_id: self.actuator_id.clone(),
            rule_id: self.rule.rule_id.clone(),
            sensor_id: value.sensor_id.clone(),
            sensor_value: value.value,
            active,
        })
    }
}

impl ActuatorRuleAssertion {
    pub fn evaluate(&self, sensor_value: f64) -> bool {
        match self {
            Self::GreaterThanOrEqual { threshold } => sensor_value >= *threshold,
        }
    }
}

impl ActuatorRuleEffect {
    pub fn active_for_match(&self, matched: bool) -> bool {
        match self {
            Self::SetActiveWhenMatched {
                active_when_matched,
                active_when_unmatched,
            } => {
                if matched {
                    *active_when_matched
                } else {
                    *active_when_unmatched
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::protocol::{
        ActuatorBackend, ActuatorConnection, ActuatorDescriptor, ActuatorKind, ActuatorProtocol,
        GpioActuatorConnection,
    };
    use alloc::string::ToString;
    use alloc::vec;

    fn relay_config() -> ActuatorConfig {
        ActuatorConfig {
            config_id: "relay-config".to_string(),
            version: 1,
            enabled: true,
            device_uid: "relay-gpio17".to_string(),
            actuator: ActuatorDescriptor {
                id: "relay-1".to_string(),
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
            rules: vec![ActuatorRuleConfig {
                rule_id: "mist-overheat".to_string(),
                version: 1,
                enabled: true,
                sensor_id: "temperature-sensor".to_string(),
                assertion: ActuatorRuleAssertion::GreaterThanOrEqual { threshold: 40.0 },
                effect: ActuatorRuleEffect::SetActiveWhenMatched {
                    active_when_matched: true,
                    active_when_unmatched: false,
                },
            }],
        }
    }

    #[test]
    fn threshold_rule_switches_relay_on_and_off_on_transitions() {
        let config = relay_config();
        let mut engine = RuleEngine::new();
        engine.replace_actuator_configs([&config]);

        let decisions = engine.evaluate(&SensorValue {
            sensor_id: "temperature-sensor".to_string(),
            value: 40.0,
        });

        assert_eq!(decisions.len(), 1);
        assert!(decisions[0].active);

        let decisions = engine.evaluate(&SensorValue {
            sensor_id: "temperature-sensor".to_string(),
            value: 40.1,
        });

        assert!(decisions.is_empty());

        let decisions = engine.evaluate(&SensorValue {
            sensor_id: "temperature-sensor".to_string(),
            value: 39.9,
        });

        assert_eq!(decisions.len(), 1);
        assert!(!decisions[0].active);
    }
}
