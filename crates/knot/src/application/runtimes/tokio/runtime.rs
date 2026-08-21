// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::string::String;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use arksync_bus::{EventId, Timestamp};
use arksync_knot_protocol::{KnotCapabilities, KnotEnvelope, KnotMessage as ProtocolKnotMessage};
use arksync_sensor::infrastructure::events::SensorEvent;
use tokio::sync::mpsc;

use crate::application::runtimes::tokio::actuator_config_mapper::legacy_actuator_config;
use crate::application::runtimes::tokio::sensor_message_mapper::knot_sensor_message;
use crate::application::{
    KnotProtocolRuntime, LegacyKnotActuatorEnvelope, LegacyKnotActuatorMessage, MessageLink,
    RetryPolicy, TokioKnotActuatorInput, TokioKnotActuatorService, TokioKnotSensorService,
    TokioMessageLink,
};

/// Boot configuration for the Tokio Knot runtime.
pub struct TokioKnotRuntimeConfig {
    /// Stable hardware identity announced to the Hub and used for configuration.
    pub hardware_uid: String,
    /// Hardware capabilities exposed during the Hello handshake.
    pub capabilities: KnotCapabilities,
    /// Retry and capacity policy for messages awaiting Hub acknowledgement.
    pub retry_policy: RetryPolicy,
}

/// Tokio runner for the portable Knot protocol and local hardware services.
///
/// The runner supplies scheduling, wall-clock timestamps, generated EventIds,
/// and a Tokio message link. Sensor events enter the portable outbox before
/// transmission. Actuator events still use the legacy local channel until the
/// actuator protocol is migrated in a later change.
pub struct TokioKnotRuntime;

impl TokioKnotRuntime {
    pub async fn run(
        config: TokioKnotRuntimeConfig,
        mut protocol_link: TokioMessageLink<KnotEnvelope>,
        legacy_actuator_event_tx: mpsc::Sender<LegacyKnotActuatorEnvelope>,
    ) {
        let hardware_uid = config.hardware_uid.clone();
        let actuator_hardware_uid = hardware_uid.clone();
        let (sensor_event_tx, mut sensor_event_rx) = mpsc::channel(100);
        let (actuator_input_tx, actuator_input_rx) = mpsc::channel(100);
        let sensor_task = tokio::spawn(async move {
            TokioKnotSensorService::with_event_sender(sensor_event_tx)
                .run()
                .await;
        });
        let actuator_task = tokio::spawn(async move {
            TokioKnotActuatorService::with_channels(
                actuator_input_rx,
                legacy_actuator_event_tx,
                actuator_hardware_uid,
            )
            .run()
            .await;
        });

        let started_at = Instant::now();
        let mut protocol_runtime = KnotProtocolRuntime::new(
            hardware_uid.clone(),
            config.capabilities,
            config.retry_policy,
        );
        if let Err(error) = protocol_runtime.start(EventId::new(), timestamp_now(), 0) {
            log::error!("Knot failed to queue Hello message: {error:?}");
            sensor_task.abort();
            actuator_task.abort();
            return;
        }

        let mut retry_tick = tokio::time::interval(Duration::from_millis(100));
        loop {
            flush_due_messages(
                &mut protocol_runtime,
                &mut protocol_link,
                elapsed_millis(started_at),
            )
            .await;

            tokio::select! {
                Some(sensor_event) = sensor_event_rx.recv() => {
                    observe_actuator_sensor_value(&actuator_input_tx, &sensor_event.payload).await;
                    let occurred_at = sensor_event.occurred_at;
                    let message = ProtocolKnotMessage::Sensor(knot_sensor_message(sensor_event.payload));
                    if let Err(error) = protocol_runtime.publish(
                        message,
                        EventId::new(),
                        occurred_at,
                        elapsed_millis(started_at),
                    ) {
                        log::error!("Knot failed to queue sensor message: {error:?}");
                    }
                }
                message = protocol_link.receive() => {
                    let message = match message {
                        Ok(Some(message)) => message,
                        Ok(None) => break,
                        Err(error) => {
                            log::error!("Knot protocol link failed: {error:?}");
                            continue;
                        }
                    };
                    let previous_config = protocol_runtime.config().cloned();
                    if let Err(error) = protocol_runtime.receive(
                        &message,
                        EventId::new(),
                        timestamp_now(),
                        elapsed_millis(started_at),
                    ) {
                        log::debug!("Knot rejected protocol message: {error:?}");
                        continue;
                    }

                    let applied_config = protocol_runtime.config();
                    if applied_config != previous_config.as_ref() {
                        if let Some(config) = applied_config {
                            let config = legacy_actuator_config(hardware_uid.clone(), config);
                            let _ = actuator_input_tx
                                .send(TokioKnotActuatorInput::Message(
                                    LegacyKnotActuatorMessage::ApplyConfig(config),
                                ))
                                .await;
                        }
                    }
                }
                _ = retry_tick.tick() => {}
                else => break,
            }
        }

        sensor_task.abort();
        actuator_task.abort();
    }
}

async fn flush_due_messages(
    runtime: &mut KnotProtocolRuntime,
    link: &mut TokioMessageLink<KnotEnvelope>,
    now_ms: u64,
) {
    for message in runtime.due_messages(now_ms) {
        if link.send(message).await.is_err() {
            log::debug!("Knot protocol link receiver is unavailable");
            break;
        }
    }
}

async fn observe_actuator_sensor_value(
    actuator_input_tx: &mpsc::Sender<TokioKnotActuatorInput>,
    event: &SensorEvent,
) {
    let SensorEvent::SensorMeasurementRecorded(measurement) = event else {
        return;
    };

    let _ = actuator_input_tx
        .send(TokioKnotActuatorInput::SensorValue {
            device_uid: measurement.device_uid.to_string(),
            value: measurement.measurement.value,
        })
        .await;
}

fn timestamp_now() -> Timestamp {
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Timestamp::from_unix_millis(unix_millis)
}

fn elapsed_millis(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis() as u64
}
