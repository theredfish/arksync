// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{
    default_measurement_interval_ms, record_sensor_measurement, Hub, SensorMeasurementInput,
    SensorRegistry,
};
use crate::config::CONFIG;
use crate::domain::PluggedSensor;
use arksync_bus::Timestamp;
use arksync_knot::application::KnotSensorEventEnvelope;
use arksync_knot::application::KnotSensorService;
use arksync_knot::domain::{KnotEventSource, KnotId, ParentHubId};
use arksync_sensor::infrastructure::events::{MeasuredSensor, SensorEvent};
use eyre::{eyre, Result, WrapErr};
use std::time::{SystemTime, UNIX_EPOCH};

/// Runtime for the local hub process.
///
/// The hub is also a local Knot: it starts the Knot sensor service in a Tokio
/// task, receives sensor events from it, then projects and persists those
/// events in the hub boundary.
pub struct HubRuntime;

impl HubRuntime {
    pub async fn run() {
        if let Err(err) = Self::try_run().await {
            log::error!("Hub runtime failed: {err:?}");
        }
    }

    async fn try_run() -> Result<()> {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<KnotSensorEventEnvelope>(100);
        let source = local_knot_source();
        let knot = tokio::spawn(async move {
            KnotSensorService::with_event_sender(event_tx, source)
                .run()
                .await;
        });
        let mut hub = Hub::new();
        let mut sensor_registry = SensorRegistry::load(arksync_db::pool())
            .await
            .wrap_err("failed to load hub sensor registry")?;

        while let Some(event) = event_rx.recv().await {
            log::debug!("Hub received local Knot sensor event: {event:?}");
            let received_at = timestamp_now();

            match &event.payload {
                SensorEvent::SensorProvisioned(provisioned) => {
                    let plugged_sensor = plugged_sensor_from_event(
                        &event.source,
                        provisioned.device_uid.clone(),
                        &provisioned.sensor,
                    );
                    let sensor_id = sensor_registry
                        .ensure_sensor_registered(arksync_db::pool(), plugged_sensor)
                        .await
                        .wrap_err("failed to register provisioned local Knot sensor")?;

                    log::info!(
                        "Hub registered local Knot sensor device_uid={} sensor_id={}",
                        provisioned.device_uid,
                        sensor_id
                    );
                }
                SensorEvent::SensorMeasurementRecorded(sensor_measurement) => {
                    let plugged_sensor = plugged_sensor_from_event(
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
                }
                _ => {}
            }

            if let Err(err) = hub.accept_sensor_event(event, received_at) {
                return Err(eyre!("Hub rejected local Knot sensor event: {err:?}"));
            }

            log::debug!("Hub projected local Knot sensor event");
        }

        knot.await.wrap_err("local Knot task join failed")?;

        Ok(())
    }
}

fn plugged_sensor_from_event(
    source: &KnotEventSource,
    device_uid: arksync_sensor::device_uid::DeviceUid,
    sensor: &MeasuredSensor,
) -> PluggedSensor {
    let KnotEventSource::Knot { knot_id, .. } = source;

    PluggedSensor {
        station_knot_id: *knot_id,
        device_uid,
        kind: sensor.kind,
        connection: sensor.connection.clone(),
        firmware: sensor.firmware,
        measurement_interval_ms: default_measurement_interval_ms(),
    }
}

fn local_knot_source() -> KnotEventSource {
    // TODO: Replace these MVP constants with a provisioned identity bundle.
    // The hub install flow should expose an admin CLI/program such as
    // `sk init hub` that authenticates the station admin, generates or loads
    // the HubId + local KnotId, signs them with a certificate, and stores the
    // resulting identity bundle for the runtime to load at boot.
    KnotEventSource::Knot {
        parent_hub_id: ParentHubId::new_with_uuid(CONFIG.local_hub_id),
        knot_id: KnotId::new_with_uuid(CONFIG.local_knot_id),
    }
}

fn timestamp_now() -> Timestamp {
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Timestamp::from_unix_millis(unix_millis)
}
