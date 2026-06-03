// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{
    record_sensor_measurement, HubService, SensorMeasurementInput, SensorRegistry,
};
use arksync_bus::Timestamp;
use arksync_sensor::infrastructure::events::SensorMeasurementRecorded;
use eyre::{Result, WrapErr};

use super::sensor_plugged::extract_plugged_sensor;
use super::HubSensorEventEnvelope;

pub(super) async fn handle_sensor_measurement_recorded(
    event: &HubSensorEventEnvelope,
    sensor_measurement: &SensorMeasurementRecorded,
    received_at: Timestamp,
    sensor_registry: &mut SensorRegistry,
    hub: &mut HubService,
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
    hub.record_sensor_measurement(
        event.source.clone(),
        sensor_id,
        sensor_measurement.clone(),
        event.occurred_at,
        received_at,
    );

    Ok(())
}
