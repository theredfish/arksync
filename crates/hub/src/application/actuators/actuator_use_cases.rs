// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::HubActuatorError;
use crate::domain::{
    Actuator, ActuatorBackend, ActuatorConnection, ActuatorKind, ActuatorProtocol, RelayActuator,
};
use crate::infrastructure::store::{
    actuator_backend_as_str, actuator_kind_as_str, actuator_protocol_as_str,
    insert_actuator as store_insert_actuator, list_actuators as store_list_actuators,
    list_actuators_by_station_knot_id as store_list_actuators_by_station_knot_id,
    station_knot_by_hardware_uid,
    update_actuator_runtime_status as store_update_actuator_runtime_status, NewActuatorRecord,
};
use arksync_actuator::infrastructure::events::{
    ActuatorBackend as ActuatorEventBackend, ActuatorConfig,
    ActuatorConnection as ActuatorEventConnection, ActuatorDescriptor,
    ActuatorKind as ActuatorEventKind, ActuatorProtocol as ActuatorEventProtocol,
    GpioActuatorConnection as ActuatorEventGpioConnection, RuntimeStatus,
};
use arksync_knot::application::KnotConfig;
use arksync_knot::domain::KnotId;
use sqlx::PgExecutor;

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
    let actuators = store_list_actuators_by_station_knot_id(executor, knot.id).await?;
    let actuator_configs = actuators
        .into_iter()
        .map(Actuator::from)
        .map(|actuator| actuator_config_from_actuator(&actuator))
        .collect();

    Ok(KnotConfig {
        hardware_uid: knot.hardware_uid,
        knot_id: KnotId::new_with_uuid(knot.id),
        actuator_configs,
    })
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

pub fn actuator_config_from_actuator(actuator: &Actuator) -> ActuatorConfig {
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
    }
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
