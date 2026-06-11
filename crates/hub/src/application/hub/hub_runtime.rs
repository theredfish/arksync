// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{handle_actuator_event, handle_sensor_event, HubService, SensorRegistry};
use crate::config::CONFIG;
use arksync_bus::Timestamp;
use arksync_knot::application::{
    KnotActuatorEvent, KnotActuatorEventEnvelope, KnotSensorEventEnvelope, TokioKnotRuntime,
    TokioKnotRuntimeConfig, TokioKnotRuntimeEvent,
};
use arksync_knot::domain::{KnotEventSource, KnotId, ParentHubId};
use eyre::{Result, WrapErr};
use std::time::{SystemTime, UNIX_EPOCH};

/// Runtime for the local hub process.
///
/// The hub is the `std` application runner for the desktop/RPi MVP. It starts
/// the local Knot runtime, receives Knot sensor and actuator events, and routes
/// them to hub application handlers that project and persist the hub state.
///
/// The local hub is also a Knot from the system point of view: it has a local
/// Knot identity and can run sensors or GPIO actuators directly. Remote Knots
/// should eventually use the same event protocol through MQTT or another bus,
/// while this runtime keeps the local path in-process with Tokio channels.
pub struct HubRuntime;

impl HubRuntime {
    /// Starts the hub runtime and logs any fatal runtime error.
    ///
    /// This method is intentionally non-failing for the Tauri entrypoint: the
    /// error is captured with debug formatting so logs keep the full cause chain
    /// while the caller does not need to unwrap runtime internals.
    pub async fn run() {
        if let Err(err) = Self::try_run().await {
            log::error!("Hub runtime failed: {err:?}");
        }
    }

    async fn try_run() -> Result<()> {
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<HubKnotEvent>(100);
        let (knot_event_tx, mut knot_event_rx) =
            tokio::sync::mpsc::channel::<TokioKnotRuntimeEvent>(100);
        let (actuator_event_tx_to_knot, actuator_event_rx_from_hub) =
            tokio::sync::mpsc::channel::<KnotActuatorEvent>(100);
        let source = local_knot_source();
        let knot_runtime = tokio::spawn(async move {
            TokioKnotRuntime::run(
                TokioKnotRuntimeConfig {
                    source,
                    hardware_uid: CONFIG.local_knot_hardware_uid.clone(),
                },
                knot_event_tx,
                actuator_event_rx_from_hub,
            )
            .await;
        });
        let knot_forwarder = {
            let event_tx = event_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = knot_event_rx.recv().await {
                    let event = match event {
                        TokioKnotRuntimeEvent::Sensor(event) => HubKnotEvent::Sensor(event),
                        TokioKnotRuntimeEvent::Actuator(event) => HubKnotEvent::Actuator(event),
                    };

                    if event_tx.send(event).await.is_err() {
                        break;
                    }
                }
            })
        };
        let mut hub = HubService::new();
        let mut sensor_registry = SensorRegistry::load(arksync_db::pool())
            .await
            .wrap_err("failed to load hub sensor registry")?;

        while let Some(event) = event_rx.recv().await {
            let received_at = timestamp_now();

            match event {
                HubKnotEvent::Sensor(event) => {
                    handle_sensor_event(
                        event,
                        received_at,
                        &mut sensor_registry,
                        &mut hub,
                        &actuator_event_tx_to_knot,
                    )
                    .await?;
                }
                HubKnotEvent::Actuator(event) => {
                    handle_actuator_event(event, &actuator_event_tx_to_knot).await?;
                }
            }
        }

        knot_runtime
            .await
            .wrap_err("local Knot runtime task join failed")?;
        knot_forwarder
            .await
            .wrap_err("local Knot runtime forwarder task join failed")?;

        Ok(())
    }
}

enum HubKnotEvent {
    /// Event emitted by the local Knot sensor path.
    Sensor(KnotSensorEventEnvelope),
    /// Event emitted by the local Knot actuator path.
    Actuator(KnotActuatorEventEnvelope),
}

fn local_knot_source() -> KnotEventSource {
    // TODO: Replace these MVP constants with a provisioned identity bundle.
    // The hub install flow should expose an admin CLI/program such as
    // `sk init hub` that authenticates the station admin, generates or loads
    // the HubId + local KnotId, signs them with a certificate, and stores the
    // resulting identity bundle for the runtime to load at boot.
    KnotEventSource::Knot {
        parent_hub_id: ParentHubId::from(CONFIG.local_hub_id),
        knot_id: KnotId::from(CONFIG.local_knot_id),
    }
}

fn timestamp_now() -> Timestamp {
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Timestamp::from_unix_millis(unix_millis)
}
