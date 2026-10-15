// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::string::String;
use std::time::{SystemTime, UNIX_EPOCH};

use arksync_bus::Timestamp;
use arksync_protocol::knot::{KnotCapabilities, KnotEnvelope, KnotMessage as ProtocolKnotMessage};
use arksync_sensor::infrastructure::events::SensorEvent;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::application::runtimes::tokio::actuator_config_mapper::legacy_actuator_config;
use crate::application::runtimes::tokio::sensor_message_mapper::knot_sensor_message;
use crate::application::{
    KeepAliveConfig, LegacyKnotActuatorEnvelope, LegacyKnotActuatorMessage, RetryPolicy,
    TokioKnotActuatorInput, TokioKnotActuatorService, TokioKnotSensorService, TokioKnotSession,
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
    /// Application heartbeat interval and communication timeout for this session.
    pub keep_alive_config: KeepAliveConfig,
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
        protocol_link: TokioMessageLink<KnotEnvelope>,
        legacy_actuator_event_tx: mpsc::Sender<LegacyKnotActuatorEnvelope>,
    ) {
        Self::run_until(
            config,
            protocol_link,
            legacy_actuator_event_tx,
            CancellationToken::new(),
        )
        .await;
    }

    /// Starts hardware independently from the session; only explicit shutdown stops it.
    pub async fn run_until(
        config: TokioKnotRuntimeConfig,
        protocol_link: TokioMessageLink<KnotEnvelope>,
        legacy_actuator_event_tx: mpsc::Sender<LegacyKnotActuatorEnvelope>,
        shutdown: CancellationToken,
    ) {
        let _shutdown_on_drop = shutdown.clone().drop_guard();
        let hardware_uid = config.hardware_uid.clone();
        let actuator_hardware_uid = hardware_uid.clone();
        let session = TokioKnotSession::new(config, protocol_link);
        let session_handle = session.handle();
        let mut status = session_handle.status();
        let mut session_task = tokio::spawn(session.run(shutdown.clone()));
        let mut session_running = true;
        let (sensor_event_tx, mut sensor_event_rx) = mpsc::channel(100);
        let (actuator_input_tx, actuator_input_rx) = mpsc::channel(100);
        let mut hardware_tasks = JoinSet::new();
        hardware_tasks.spawn(async move {
            TokioKnotSensorService::with_event_sender(sensor_event_tx)
                .run()
                .await;
        });
        hardware_tasks.spawn(async move {
            TokioKnotActuatorService::with_channels(
                actuator_input_rx,
                legacy_actuator_event_tx,
                actuator_hardware_uid,
            )
            .run()
            .await;
        });

        let mut last_config = None;
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                Some(sensor_event) = sensor_event_rx.recv() => {
                    observe_actuator_sensor_value(&actuator_input_tx, &sensor_event.payload).await;
                    let message = ProtocolKnotMessage::Sensor(knot_sensor_message(sensor_event.payload));
                    // Hardware execution must not wait for space in the protocol outbox.
                    if let Err(error) = session_handle.try_publish_at(message, sensor_event.occurred_at) {
                        log::warn!("Knot could not queue sensor message: {error}");
                    }
                }
                Ok(()) = status.changed(), if session_running => {
                    let applied_config = status.borrow_and_update().config.clone();
                    if let Some(config) = applied_config {
                        if last_config.as_ref() != Some(&config) {
                            let legacy = legacy_actuator_config(hardware_uid.clone(), &config);
                            let _ = actuator_input_tx
                                .send(TokioKnotActuatorInput::Message(
                                    LegacyKnotActuatorMessage::ApplyConfig(legacy),
                                ))
                                .await;
                            last_config = Some(config);
                        }
                    }
                }
                result = &mut session_task, if session_running => {
                    log::warn!("Knot protocol session stopped; hardware remains running: {result:?}");
                    session_running = false;
                }
                else => break,
            }
        }

        hardware_tasks.shutdown().await;
        if session_running {
            session_task.abort();
            let _ = session_task.await;
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

pub(super) fn timestamp_now() -> Timestamp {
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Timestamp::from_unix_millis(unix_millis)
}
