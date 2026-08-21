// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::HubActuatorError;
use crate::domain::{
    Actuator, ActuatorBackend, ActuatorConnection, ActuatorKind, ActuatorProtocol,
    GpioActuatorConnection, RelayActuator, SensorId,
};
use crate::infrastructure::store::{
    actuators as actuator_store, knot as knot_store, sensors as sensor_store, ActuatorRecord,
    ActuatorRuleRecord, ActuatorStoreError, NewActuatorRecord, NewActuatorRuleRecord,
};
use arksync_actuator::application::protocol::{
    ActuatorBackend as ProtocolActuatorBackend, ActuatorConfig,
    ActuatorConnection as ProtocolActuatorConnection, ActuatorDescriptor,
    ActuatorKind as ProtocolActuatorKind, ActuatorProtocol as ProtocolActuatorProtocol,
    ActuatorRuleAssertion, ActuatorRuleConfig, ActuatorRuleEffect,
    GpioActuatorConnection as ProtocolGpioActuatorConnection, RuntimeStatus,
};
use arksync_knot::application::{LegacyKnotActuatorConfig, LegacyKnotSensorBinding};
use arksync_knot::domain::KnotId;
use arksync_knot_protocol::{
    KnotActuatorBackend, KnotActuatorConfig, KnotActuatorConnection, KnotActuatorDescriptor,
    KnotActuatorKind, KnotActuatorProtocol, KnotActuatorRule, KnotActuatorRuleAssertion,
    KnotActuatorRuleEffect, KnotConfig as ProtocolKnotConfig, KnotGpioActuatorConnection,
    KnotSensorBinding as ProtocolKnotSensorBinding,
};
use arksync_utils::uuid::Uuid;
use sqlx::{PgExecutor, PgTransaction};

const LOCAL_DEMO_RELAY_DEVICE_UID: &str = "rpi-gpio17-mist-relay";
const LOCAL_DEMO_RELAY_DISPLAY_NAME: &str = "Mist relay";
const LEGACY_LOCAL_DEMO_RULE_NAME: &str = "temperature_ge_36_mist_relay";
const LOCAL_DEMO_RULE_NAME: &str = "temperature_ge_40_mist_relay";

pub async fn list_actuators(
    executor: impl PgExecutor<'_>,
) -> Result<Vec<Actuator>, HubActuatorError> {
    let records = actuator_store::list_actuators(executor).await?;

    Ok(records.into_iter().map(Actuator::from).collect())
}

pub async fn insert_relay_actuator(
    executor: impl PgExecutor<'_>,
    actuator: RelayActuator,
) -> Result<Actuator, HubActuatorError> {
    let record = NewActuatorRecord {
        station_knot_id: actuator.station_knot_id.uuid_v4(),
        device_uid: actuator.device_uid,
        display_name: actuator.display_name,
        kind: ActuatorKind::Relay.to_string(),
        backend: actuator.backend.to_string(),
        protocol: ActuatorProtocol::Gpio.to_string(),
        config_version: actuator.config_version,
        enabled: actuator.enabled,
        gpio_pin: Some(i32::from(actuator.connection.pin)),
        pin_scheme: actuator.connection.pin_scheme,
        active_low: actuator.connection.active_low,
        channels: actuator.channels,
        model: actuator.model,
    };
    let actuator = actuator_store::insert_actuator(executor, &record).await?;

    Ok(actuator.into())
}

pub async fn actuator_config_ack_for_knot_hardware_uid(
    txn: &mut PgTransaction<'_>,
    hardware_uid: &str,
) -> Result<LegacyKnotActuatorConfig, HubActuatorError> {
    let knot = knot_store::station_knot_by_hardware_uid(&mut **txn, hardware_uid).await?;
    let actuator_records =
        actuator_store::list_actuators_by_station_knot_id(&mut **txn, knot.id).await?;
    let actuator_rules = actuator_store::list_actuator_rules_by_actuator_ids(
        &mut **txn,
        &actuator_ids(&actuator_records),
    )
    .await?;
    let sensor_bindings = sensor_store::list_sensors(&mut **txn)
        .await?
        .into_iter()
        .filter(|sensor| sensor.station_knot_id == knot.id)
        .map(|sensor| LegacyKnotSensorBinding {
            sensor_id: sensor.id.to_string(),
            device_uid: sensor.device_uid,
        })
        .collect();
    let actuator_configs = actuator_records
        .into_iter()
        .map(|record| {
            let rules = actuator_rules
                .iter()
                .filter(|rule| rule.actuator_id == record.id)
                .map(actuator_rule_config_from_record)
                .collect();

            actuator_config_from_actuator(&Actuator::from(record), rules)
        })
        .collect();

    Ok(LegacyKnotActuatorConfig {
        hardware_uid: knot.hardware_uid,
        knot_id: KnotId::from(knot.id),
        sensor_bindings,
        actuator_configs,
    })
}

pub async fn knot_protocol_config_for_hardware_uid(
    txn: &mut PgTransaction<'_>,
    hardware_uid: &str,
) -> Result<ProtocolKnotConfig, HubActuatorError> {
    let knot = knot_store::station_knot_by_hardware_uid(&mut **txn, hardware_uid).await?;
    let legacy_config = actuator_config_ack_for_knot_hardware_uid(txn, hardware_uid).await?;

    let sensor_bindings = legacy_config
        .sensor_bindings
        .into_iter()
        .map(|binding| {
            let sensor_id = Uuid::parse_str(&binding.sensor_id)?;

            Ok(ProtocolKnotSensorBinding {
                sensor_id: *sensor_id.as_bytes(),
                device_uid: binding.device_uid,
            })
        })
        .collect::<Result<Vec<_>, arksync_utils::uuid::Error>>()?;

    Ok(ProtocolKnotConfig {
        version: knot.config_version as u64,
        knot_id: *knot.id.as_bytes(),
        sensor_bindings,
        actuator_configs: legacy_config
            .actuator_configs
            .into_iter()
            .map(knot_actuator_config_from_legacy)
            .collect(),
    })
}

pub async fn ensure_local_demo_temperature_relay_rule(
    txn: &mut PgTransaction<'_>,
    station_knot_id: KnotId,
    sensor_id: SensorId,
) -> Result<Actuator, HubActuatorError> {
    let station_knot_id = station_knot_id.uuid_v4();
    let sensor_id = sensor_id.uuid_v4();
    let actuator = ensure_local_demo_relay_actuator(txn, station_knot_id).await?;

    ensure_local_demo_relay_rule(txn, actuator.id.uuid_v4(), sensor_id).await?;

    Ok(actuator)
}

pub async fn record_actuator_runtime_status(
    txn: &mut PgTransaction<'_>,
    status: &RuntimeStatus,
) -> Result<(), HubActuatorError> {
    for actuator in &status.actuators {
        actuator_store::update_actuator_runtime_status(
            &mut **txn,
            &actuator.config_id,
            actuator.version as i64,
            actuator.enabled,
        )
        .await?;
    }

    Ok(())
}

pub fn actuator_config_from_actuator(
    actuator: &Actuator,
    rules: Vec<ActuatorRuleConfig>,
) -> ActuatorConfig {
    ActuatorConfig {
        config_id: actuator.id.to_string(),
        version: actuator.config_version as u64,
        enabled: actuator.enabled,
        device_uid: actuator.device_uid.clone(),
        actuator: ActuatorDescriptor {
            id: actuator.id.to_string(),
            kind: actuator_kind_to_protocol(actuator.kind),
            backend: actuator_backend_to_protocol(actuator.backend),
            protocol: actuator_protocol_to_protocol(actuator.protocol),
            connection: actuator_connection_to_protocol(&actuator.connection),
            channels: actuator.channels,
            model: actuator.model.clone(),
        },
        rules,
    }
}

fn actuator_ids(actuators: &[ActuatorRecord]) -> Vec<Uuid> {
    actuators.iter().map(|actuator| actuator.id).collect()
}

fn actuator_rule_config_from_record(rule: &ActuatorRuleRecord) -> ActuatorRuleConfig {
    ActuatorRuleConfig {
        rule_id: rule.id.to_string(),
        version: rule.config_version as u64,
        enabled: rule.enabled,
        sensor_id: rule.sensor_id.to_string(),
        assertion: ActuatorRuleAssertion::GreaterThanOrEqual {
            threshold: rule.threshold,
        },
        effect: ActuatorRuleEffect::SetActiveWhenMatched {
            active_when_matched: rule.active_when_matched,
            active_when_unmatched: rule.active_when_unmatched,
        },
    }
}

async fn ensure_local_demo_relay_actuator(
    txn: &mut PgTransaction<'_>,
    station_knot_id: Uuid,
) -> Result<Actuator, HubActuatorError> {
    match actuator_store::actuator_by_station_knot_id_and_device_uid(
        &mut **txn,
        station_knot_id,
        LOCAL_DEMO_RELAY_DEVICE_UID,
    )
    .await
    {
        Ok(actuator) => return Ok(actuator.into()),
        Err(ActuatorStoreError::NotFound) => {}
        Err(err) => return Err(err.into()),
    }

    let actuator = RelayActuator {
        station_knot_id: KnotId::from(station_knot_id),
        device_uid: LOCAL_DEMO_RELAY_DEVICE_UID.to_string(),
        display_name: Some(LOCAL_DEMO_RELAY_DISPLAY_NAME.to_string()),
        backend: ActuatorBackend::LinuxGpiod,
        connection: GpioActuatorConnection {
            pin: 17,
            pin_scheme: Some("bcm".to_string()),
            active_low: true,
        },
        config_version: 1,
        enabled: true,
        channels: Some(2),
        model: Some("5V dual-channel relay module".to_string()),
    };

    insert_relay_actuator(&mut **txn, actuator).await
}

async fn ensure_local_demo_relay_rule(
    txn: &mut PgTransaction<'_>,
    actuator_id: Uuid,
    sensor_id: Uuid,
) -> Result<ActuatorRuleRecord, HubActuatorError> {
    actuator_store::delete_actuator_rule_by_actuator_sensor_name(
        &mut **txn,
        actuator_id,
        sensor_id,
        LEGACY_LOCAL_DEMO_RULE_NAME,
    )
    .await?;

    match actuator_store::actuator_rule_by_actuator_sensor_name(
        &mut **txn,
        actuator_id,
        sensor_id,
        LOCAL_DEMO_RULE_NAME,
    )
    .await
    {
        Ok(rule) => return Ok(rule),
        Err(ActuatorStoreError::NotFound) => {}
        Err(err) => return Err(err.into()),
    }

    let rule = NewActuatorRuleRecord {
        actuator_id,
        sensor_id,
        name: LOCAL_DEMO_RULE_NAME.to_string(),
        config_version: 1,
        enabled: true,
        threshold: 40.0,
        active_when_matched: true,
        active_when_unmatched: false,
    };

    Ok(actuator_store::insert_actuator_rule(&mut **txn, &rule).await?)
}

fn actuator_kind_to_protocol(kind: ActuatorKind) -> ProtocolActuatorKind {
    match kind {
        ActuatorKind::Relay => ProtocolActuatorKind::Relay,
    }
}

fn actuator_backend_to_protocol(backend: ActuatorBackend) -> ProtocolActuatorBackend {
    match backend {
        ActuatorBackend::LinuxGpiod => ProtocolActuatorBackend::LinuxGpiod,
        ActuatorBackend::EspGpio => ProtocolActuatorBackend::EspGpio,
    }
}

fn actuator_protocol_to_protocol(protocol: ActuatorProtocol) -> ProtocolActuatorProtocol {
    match protocol {
        ActuatorProtocol::Gpio => ProtocolActuatorProtocol::Gpio,
    }
}

fn actuator_connection_to_protocol(connection: &ActuatorConnection) -> ProtocolActuatorConnection {
    match connection {
        ActuatorConnection::Gpio(connection) => {
            ProtocolActuatorConnection::Gpio(ProtocolGpioActuatorConnection {
                pin: connection.pin,
                pin_scheme: connection.pin_scheme.clone(),
                active_low: connection.active_low,
            })
        }
    }
}

fn knot_actuator_config_from_legacy(config: ActuatorConfig) -> KnotActuatorConfig {
    KnotActuatorConfig {
        config_id: config.config_id,
        version: config.version,
        enabled: config.enabled,
        device_uid: config.device_uid,
        actuator: KnotActuatorDescriptor {
            id: config.actuator.id,
            kind: match config.actuator.kind {
                ProtocolActuatorKind::Relay => KnotActuatorKind::Relay,
            },
            backend: match config.actuator.backend {
                ProtocolActuatorBackend::LinuxGpiod => KnotActuatorBackend::LinuxGpiod,
                ProtocolActuatorBackend::EspGpio => KnotActuatorBackend::EspGpio,
            },
            protocol: match config.actuator.protocol {
                ProtocolActuatorProtocol::Gpio => KnotActuatorProtocol::Gpio,
            },
            connection: match config.actuator.connection {
                ProtocolActuatorConnection::Gpio(connection) => {
                    KnotActuatorConnection::Gpio(KnotGpioActuatorConnection {
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
            .map(|rule| KnotActuatorRule {
                rule_id: rule.rule_id,
                version: rule.version,
                enabled: rule.enabled,
                sensor_id: rule.sensor_id,
                assertion: match rule.assertion {
                    ActuatorRuleAssertion::GreaterThanOrEqual { threshold } => {
                        KnotActuatorRuleAssertion::GreaterThanOrEqual { threshold }
                    }
                },
                effect: match rule.effect {
                    ActuatorRuleEffect::SetActiveWhenMatched {
                        active_when_matched,
                        active_when_unmatched,
                    } => KnotActuatorRuleEffect::SetActiveWhenMatched {
                        active_when_matched,
                        active_when_unmatched,
                    },
                },
            })
            .collect(),
    }
}
