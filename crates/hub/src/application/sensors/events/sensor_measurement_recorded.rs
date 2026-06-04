// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{
    actuator_config_ack_for_knot_hardware_uid, ensure_local_demo_temperature_relay_rule,
    record_sensor_measurement, HubService, SensorMeasurementInput, SensorRegistry,
};
use crate::config::CONFIG;
use arksync_bus::Timestamp;
use arksync_knot::application::KnotActuatorEvent;
use arksync_knot::domain::KnotEventSource;
use arksync_sensor::infrastructure::events::SensorMeasurementRecorded;
use eyre::{eyre, Result, WrapErr};
use std::sync::atomic::{AtomicBool, Ordering};

use super::sensor_plugged::extract_plugged_sensor;
use super::HubSensorEventEnvelope;

pub(super) async fn handle_sensor_measurement_recorded(
    event: &HubSensorEventEnvelope,
    sensor_measurement: &SensorMeasurementRecorded,
    received_at: Timestamp,
    sensor_registry: &mut SensorRegistry,
    hub: &mut HubService,
    knot_event_tx: &tokio::sync::mpsc::Sender<KnotActuatorEvent>,
) -> Result<()> {
    let plugged_sensor = extract_plugged_sensor(
        &event.source,
        sensor_measurement.device_uid.clone(),
        &sensor_measurement.sensor,
    );
    let sensor_id = sensor_registry
        .ensure_sensor_registered(arksync_db::pool(), plugged_sensor)
        .await
        .wrap_err("failed to register measured local Knot sensor")?;
    let input = SensorMeasurementInput {
        event_id: event.id,
        source: event.source.clone(),
        sensor_id,
        kind: sensor_measurement.sensor.kind,
        value: sensor_measurement.measurement.value,
        unit: sensor_measurement.measurement.unit,
        measured_at: event.occurred_at,
        received_at,
    };
    let measurement = record_sensor_measurement(arksync_db::pool(), input)
        .await
        .wrap_err("failed to persist sensor measurement")?;

    log::info!(
        "Hub persisted sensor measurement device_uid={} sensor_id={} value={}",
        sensor_measurement.device_uid,
        measurement.sensor_id,
        measurement.value
    );

    maybe_refresh_local_demo_actuator_config(&event.source, sensor_id, knot_event_tx).await?;

    hub.record_sensor_measurement(
        event.source.clone(),
        sensor_id,
        sensor_measurement.clone(),
        event.occurred_at,
        received_at,
    );

    Ok(())
}

async fn maybe_refresh_local_demo_actuator_config(
    source: &KnotEventSource,
    sensor_id: crate::domain::SensorId,
    knot_event_tx: &tokio::sync::mpsc::Sender<KnotActuatorEvent>,
) -> Result<()> {
    static LOCAL_DEMO_ACTUATOR_CONFIG_REFRESHED: AtomicBool = AtomicBool::new(false);

    if LOCAL_DEMO_ACTUATOR_CONFIG_REFRESHED.load(Ordering::Relaxed) {
        return Ok(());
    }

    let KnotEventSource::Knot { knot_id, .. } = source;

    if knot_id.as_uuid() != CONFIG.local_knot_id {
        return Ok(());
    }

    ensure_local_demo_temperature_relay_rule(arksync_db::pool(), *knot_id, sensor_id)
        .await
        .wrap_err("failed to ensure local demo relay rule")?;

    let ack = actuator_config_ack_for_knot_hardware_uid(
        arksync_db::pool(),
        &CONFIG.local_knot_hardware_uid,
    )
    .await
    .wrap_err("failed to refresh local Knot actuator config after sensor measurement")?;
    log::info!(
        "Hub refreshes local Knot actuator config after sensor measurement sensor_id={} actuator_configs={} sensor_bindings={}",
        sensor_id,
        ack.actuator_configs.len(),
        ack.sensor_bindings.len()
    );

    knot_event_tx
        .send(KnotActuatorEvent::Ack(ack))
        .await
        .map_err(|_| eyre!("local Knot actuator event receiver dropped"))?;
    LOCAL_DEMO_ACTUATOR_CONFIG_REFRESHED.store(true, Ordering::Relaxed);

    Ok(())
}
