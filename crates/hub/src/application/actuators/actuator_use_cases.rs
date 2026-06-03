// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::HubActuatorError;
use crate::domain::{
    Actuator, ActuatorBackend, ActuatorConnection, ActuatorKind, ActuatorProtocol,
    GpioActuatorConnection, RelayActuator, SensorId,
};
use crate::infrastructure::store::{
    actuator_backend_as_str, actuator_by_station_knot_id_and_device_uid, actuator_kind_as_str,
    actuator_protocol_as_str, actuator_rule_by_actuator_sensor_name,
    delete_actuator_rule_by_actuator_sensor_name, insert_actuator as store_insert_actuator,
    insert_actuator_rule, list_actuator_rules_by_actuator_ids,
    list_actuators as store_list_actuators,
    list_actuators_by_station_knot_id as store_list_actuators_by_station_knot_id,
    list_sensors as store_list_sensors, station_knot_by_hardware_uid,
    update_actuator_runtime_status as store_update_actuator_runtime_status, ActuatorRecord,
    ActuatorRuleRecord, ActuatorStoreError, NewActuatorRecord, NewActuatorRuleRecord,
};
use arksync_actuator::infrastructure::events::{
    ActuatorBackend as ActuatorEventBackend, ActuatorConfig,
    ActuatorConnection as ActuatorEventConnection, ActuatorDescriptor,
    ActuatorKind as ActuatorEventKind, ActuatorProtocol as ActuatorEventProtocol,
    ActuatorRuleAssertion, ActuatorRuleConfig, ActuatorRuleEffect,
    GpioActuatorConnection as ActuatorEventGpioConnection, RuntimeStatus,
};
use arksync_knot::application::{KnotConfig, KnotSensorBinding};
use arksync_knot::domain::KnotId;
use arksync_utils::uuid::Uuid;
use sqlx::PgExecutor;

const LOCAL_DEMO_RELAY_DEVICE_UID: &str = "rpi-gpio17-mist-relay";
const LOCAL_DEMO_RELAY_DISPLAY_NAME: &str = "Mist relay";
const LEGACY_LOCAL_DEMO_RULE_NAME: &str = "temperature_ge_36_mist_relay";
const LOCAL_DEMO_RULE_NAME: &str = "temperature_ge_40_mist_relay";

pub async fn list_actuators<'e, E>(executor: E) -> Result<Vec<Actuator>, HubActuatorError>
where
    E: PgExecutor<'e>,
{
    let records = store_list_actuators(executor).await?;

    Ok(records.into_iter().map(Actuator::from).collect())
}

pub async fn insert_relay_actuator<'e, E>(
    executor: E,
    actuator: RelayActuator,
) -> Result<Actuator, HubActuatorError>
where
    E: PgExecutor<'e>,
{
    let record = NewActuatorRecord {
        station_knot_id: actuator.station_knot_id.as_uuid(),
        device_uid: actuator.device_uid,
        display_name: actuator.display_name,
        kind: actuator_kind_as_str(ActuatorKind::Relay).to_string(),
        backend: actuator_backend_as_str(actuator.backend).to_string(),
        protocol: actuator_protocol_as_str(ActuatorProtocol::Gpio).to_string(),
        config_version: actuator.config_version,
        enabled: actuator.enabled,
        gpio_pin: Some(i32::from(actuator.connection.pin)),
        pin_scheme: actuator.connection.pin_scheme,
        active_low: actuator.connection.active_low,
        channels: actuator.channels,
        model: actuator.model,
    };
    let actuator = store_insert_actuator(executor, &record).await?;

    Ok(actuator.into())
}

pub async fn actuator_config_ack_for_knot_hardware_uid(
    executor: &sqlx::PgPool,
    hardware_uid: &str,
) -> Result<KnotConfig, HubActuatorError> {
    let knot = station_knot_by_hardware_uid(executor, hardware_uid).await?;
    let actuator_records = store_list_actuators_by_station_knot_id(executor, knot.id).await?;
    let actuator_rules =
        list_actuator_rules_by_actuator_ids(executor, &actuator_ids(&actuator_records)).await?;
    let sensor_bindings = store_list_sensors(executor)
        .await?
        .into_iter()
        .filter(|sensor| sensor.station_knot_id == knot.id)
        .map(|sensor| KnotSensorBinding {
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

    Ok(KnotConfig {
        hardware_uid: knot.hardware_uid,
        knot_id: KnotId::new_with_uuid(knot.id),
        sensor_bindings,
        actuator_configs,
    })
}

pub async fn ensure_local_demo_temperature_relay_rule(
    executor: &sqlx::PgPool,
    station_knot_id: KnotId,
    sensor_id: SensorId,
) -> Result<Actuator, HubActuatorError> {
    let station_knot_id = station_knot_id.as_uuid();
    let sensor_id = sensor_id.as_uuid();
    let actuator = ensure_local_demo_relay_actuator(executor, station_knot_id).await?;

    ensure_local_demo_relay_rule(executor, actuator.id.as_uuid(), sensor_id).await?;

    Ok(actuator)
}

pub async fn record_actuator_runtime_status(
    executor: &sqlx::PgPool,
    status: &RuntimeStatus,
) -> Result<(), HubActuatorError> {
    for actuator in &status.actuators {
        store_update_actuator_runtime_status(
            executor,
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
            kind: actuator_kind_to_event(actuator.kind),
            backend: actuator_backend_to_event(actuator.backend),
            protocol: actuator_protocol_to_event(actuator.protocol),
            connection: actuator_connection_to_event(&actuator.connection),
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
    executor: &sqlx::PgPool,
    station_knot_id: Uuid,
) -> Result<Actuator, HubActuatorError> {
    match actuator_by_station_knot_id_and_device_uid(
        executor,
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
        station_knot_id: KnotId::new_with_uuid(station_knot_id),
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

    insert_relay_actuator(executor, actuator).await
}

async fn ensure_local_demo_relay_rule(
    executor: &sqlx::PgPool,
    actuator_id: Uuid,
    sensor_id: Uuid,
) -> Result<ActuatorRuleRecord, HubActuatorError> {
    delete_actuator_rule_by_actuator_sensor_name(
        executor,
        actuator_id,
        sensor_id,
        LEGACY_LOCAL_DEMO_RULE_NAME,
    )
    .await?;

    match actuator_rule_by_actuator_sensor_name(
        executor,
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

    Ok(insert_actuator_rule(executor, &rule).await?)
}

fn actuator_kind_to_event(kind: ActuatorKind) -> ActuatorEventKind {
    match kind {
        ActuatorKind::Relay => ActuatorEventKind::Relay,
    }
}

fn actuator_backend_to_event(backend: ActuatorBackend) -> ActuatorEventBackend {
    match backend {
        ActuatorBackend::LinuxGpiod => ActuatorEventBackend::LinuxGpiod,
        ActuatorBackend::EspGpio => ActuatorEventBackend::EspGpio,
    }
}

fn actuator_protocol_to_event(protocol: ActuatorProtocol) -> ActuatorEventProtocol {
    match protocol {
        ActuatorProtocol::Gpio => ActuatorEventProtocol::Gpio,
    }
}

fn actuator_connection_to_event(connection: &ActuatorConnection) -> ActuatorEventConnection {
    match connection {
        ActuatorConnection::Gpio(connection) => {
            ActuatorEventConnection::Gpio(ActuatorEventGpioConnection {
                pin: connection.pin,
                pin_scheme: connection.pin_scheme.clone(),
                active_low: connection.active_low,
            })
        }
    }
}
