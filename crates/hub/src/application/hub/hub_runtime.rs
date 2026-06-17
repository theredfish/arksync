// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{handle_knot_event, handle_sensor_event, HubService, SensorRegistry};
use crate::config::CONFIG;
use arksync_bus::{EventBus, EventBusError, EventEnvelope, EventHandler, Timestamp};
use arksync_knot::application::{
    KnotMessage, KnotMessageEnvelope, KnotSensorEventEnvelope, TokioKnotRuntime,
    TokioKnotRuntimeConfig, TokioKnotRuntimeEvent,
};
use arksync_knot::domain::{KnotEventSource, KnotId, ParentHubId};
use eyre::{eyre, Result, WrapErr};
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
        let (knot_event_tx, mut knot_event_rx) =
            tokio::sync::mpsc::channel::<TokioKnotRuntimeEvent>(100);
        let (knot_message_tx_to_knot, knot_message_rx_from_hub) =
            tokio::sync::mpsc::channel::<KnotMessage>(100);
        let (sensor_event_tx, mut sensor_event_rx) =
            tokio::sync::mpsc::channel::<KnotSensorEventEnvelope>(100);
        let (knot_message_event_tx, mut knot_message_event_rx) =
            tokio::sync::mpsc::channel::<KnotMessageEnvelope>(100);
        let mut hub_bus = EventBus::new();
        hub_bus.subscribe_where(
            |event: &EventEnvelope<HubKnotEvent>| matches!(event.payload, HubKnotEvent::Sensor(_)),
            HubSensorEventHandler(sensor_event_tx),
        );
        hub_bus.subscribe_where(
            |event: &EventEnvelope<HubKnotEvent>| matches!(event.payload, HubKnotEvent::Knot(_)),
            HubKnotMessageHandler(knot_message_event_tx),
        );
        let source = local_knot_source();
        let knot_runtime = tokio::spawn(async move {
            TokioKnotRuntime::run(
                TokioKnotRuntimeConfig {
                    source,
                    hardware_uid: CONFIG.local_knot_hardware_uid.clone(),
                },
                knot_event_tx,
                knot_message_rx_from_hub,
            )
            .await;
        });
        let mut hub = HubService::new();
        let mut sensor_registry = SensorRegistry::load(arksync_db::pool())
            .await
            .wrap_err("failed to load hub sensor registry")?;

        loop {
            tokio::select! {
                Some(event) = knot_event_rx.recv() => {
                    hub_bus
                        .publish(hub_event_envelope_from_tokio_event(event))
                        .map_err(|err| eyre!("hub runtime bus rejected Knot event: {err:?}"))?;
                }
                Some(event) = sensor_event_rx.recv() => {
                    let received_at = timestamp_now();
                    handle_sensor_event(
                        event,
                        received_at,
                        &mut sensor_registry,
                        &mut hub,
                        &knot_message_tx_to_knot,
                    )
                    .await?;
                }
                Some(event) = knot_message_event_rx.recv() => {
                    handle_knot_event(event, &knot_message_tx_to_knot).await?;
                }
                else => break,
            }
        }

        knot_runtime
            .await
            .wrap_err("local Knot runtime task join failed")?;

        Ok(())
    }
}

#[derive(Clone)]
enum HubKnotEvent {
    /// Event emitted by the local Knot sensor path.
    Sensor(KnotSensorEventEnvelope),
    /// Protocol message emitted by the local Knot runtime.
    Knot(KnotMessageEnvelope),
}

fn hub_event_envelope_from_tokio_event(
    event: TokioKnotRuntimeEvent,
) -> EventEnvelope<HubKnotEvent> {
    match event {
        TokioKnotRuntimeEvent::Sensor(event) => {
            EventEnvelope::new_with_id(event.id, (), event.occurred_at, HubKnotEvent::Sensor(event))
        }
        TokioKnotRuntimeEvent::Knot(event) => {
            EventEnvelope::new_with_id(event.id, (), event.occurred_at, HubKnotEvent::Knot(event))
        }
    }
}

struct HubSensorEventHandler(tokio::sync::mpsc::Sender<KnotSensorEventEnvelope>);

impl EventHandler<HubKnotEvent> for HubSensorEventHandler {
    fn handle(
        &mut self,
        event: EventEnvelope<HubKnotEvent>,
    ) -> core::result::Result<(), EventBusError> {
        let HubKnotEvent::Sensor(event) = event.payload else {
            return Ok(());
        };

        self.0
            .try_send(event)
            .map_err(|_| EventBusError::HandlerRejected)
    }
}

struct HubKnotMessageHandler(tokio::sync::mpsc::Sender<KnotMessageEnvelope>);

impl EventHandler<HubKnotEvent> for HubKnotMessageHandler {
    fn handle(
        &mut self,
        event: EventEnvelope<HubKnotEvent>,
    ) -> core::result::Result<(), EventBusError> {
        let HubKnotEvent::Knot(event) = event.payload else {
            return Ok(());
        };

        self.0
            .try_send(event)
            .map_err(|_| EventBusError::HandlerRejected)
    }
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
