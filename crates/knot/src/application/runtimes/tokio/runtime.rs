// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{
    KnotActuatorEvent, KnotActuatorEventEnvelope, KnotSensorEventEnvelope, TokioKnotActuatorInput,
    TokioKnotActuatorService, TokioKnotSensorService,
};
use crate::domain::KnotEventSource;
use arksync_sensor::infrastructure::events::SensorEvent;
use std::string::String;
use tokio::sync::mpsc;

/// Event emitted by the Tokio Knot runtime toward its parent runtime.
///
/// The Tokio adapter runs sensor and actuator loops concurrently, then forwards
/// both streams through this single channel so the hub can consume one Knot
/// event stream.
pub enum TokioKnotRuntimeEvent {
    /// Sensor event produced by the Knot sensor service.
    Sensor(KnotSensorEventEnvelope),
    /// Actuator event produced by the Knot actuator service.
    Actuator(KnotActuatorEventEnvelope),
}

/// Boot configuration for the Tokio Knot runtime.
///
/// The `source` identifies the logical Knot in emitted events. The
/// `hardware_uid` is the physical identity used during the actuator handshake
/// so the hub can load the persisted configuration for this Knot.
pub struct TokioKnotRuntimeConfig {
    pub source: KnotEventSource,
    pub hardware_uid: String,
}

/// Tokio-based Knot runner used by the local hub MVP.
///
/// This is not the platform-agnostic Knot core. It is the `std`/Tokio adapter
/// that starts the sensor and actuator services as Tokio tasks, bridges their
/// events into one stream, and receives actuator configuration events from the
/// hub. A future ESP32 binary should provide an Embassy equivalent around the
/// same no_std application concepts instead of depending on this type.
pub struct TokioKnotRuntime;

impl TokioKnotRuntime {
    /// Starts the Tokio sensor and actuator loops and forwards their events.
    ///
    /// `event_tx` is the outbound stream consumed by the parent hub runtime.
    /// `actuator_event_rx` is the inbound stream used by the hub to acknowledge
    /// the Knot hello handshake and send actuator configuration changes.
    pub async fn run(
        config: TokioKnotRuntimeConfig,
        event_tx: mpsc::Sender<TokioKnotRuntimeEvent>,
        actuator_event_rx: mpsc::Receiver<KnotActuatorEvent>,
    ) {
        let (sensor_event_tx, mut sensor_event_rx) = mpsc::channel::<KnotSensorEventEnvelope>(100);
        let (actuator_event_tx, mut actuator_event_rx_from_knot) =
            mpsc::channel::<KnotActuatorEventEnvelope>(100);
        let (actuator_input_tx, actuator_input_rx) = mpsc::channel::<TokioKnotActuatorInput>(100);
        let sensor_source = config.source.clone();
        let sensor_knot = tokio::spawn(async move {
            TokioKnotSensorService::with_event_sender(sensor_event_tx, sensor_source)
                .run()
                .await;
        });
        let actuator_knot = tokio::spawn(async move {
            TokioKnotActuatorService::with_channels(
                actuator_input_rx,
                actuator_event_tx,
                config.hardware_uid,
            )
            .run()
            .await;
        });
        let actuator_input_forwarder = {
            let actuator_input_tx = actuator_input_tx.clone();
            tokio::spawn(async move {
                let mut actuator_event_rx = actuator_event_rx;

                while let Some(event) = actuator_event_rx.recv().await {
                    if actuator_input_tx
                        .send(TokioKnotActuatorInput::Event(event))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            })
        };
        let sensor_forwarder = {
            let event_tx = event_tx.clone();
            let actuator_input_tx = actuator_input_tx.clone();
            tokio::spawn(async move {
                while let Some(event) = sensor_event_rx.recv().await {
                    if let SensorEvent::SensorMeasurementRecorded(measurement) = &event.payload {
                        let _ = actuator_input_tx
                            .send(TokioKnotActuatorInput::SensorValue {
                                device_uid: measurement.device_uid.to_string(),
                                value: measurement.measurement.value,
                            })
                            .await;
                    }

                    if event_tx
                        .send(TokioKnotRuntimeEvent::Sensor(event))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            })
        };
        let actuator_forwarder = tokio::spawn(async move {
            while let Some(event) = actuator_event_rx_from_knot.recv().await {
                if event_tx
                    .send(TokioKnotRuntimeEvent::Actuator(event))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        let _ = sensor_knot.await;
        let _ = actuator_knot.await;
        let _ = actuator_input_forwarder.await;
        let _ = sensor_forwarder.await;
        let _ = actuator_forwarder.await;
    }
}
