// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_actuator::infrastructure::events::ActuatorEvent;
use arksync_actuator::relay::{RelayDriver, RelayState, MIST_RELAY};
use arksync_bus::{EventBus, EventBusError, EventEnvelope, EventHandler, EventId, Timestamp};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

use crate::application::{KnotActuatorEvent, KnotActuatorEventEnvelope, KnotHello, KnotRuntime};

pub enum TokioKnotActuatorInput {
    Event(KnotActuatorEvent),
    SensorValue { device_uid: String, value: f64 },
}

pub struct TokioKnotActuatorService {
    event_rx: mpsc::Receiver<TokioKnotActuatorInput>,
    event_tx: mpsc::Sender<KnotActuatorEventEnvelope>,
    hardware_uid: String,
}

impl TokioKnotActuatorService {
    pub fn with_channels(
        event_rx: mpsc::Receiver<TokioKnotActuatorInput>,
        event_tx: mpsc::Sender<KnotActuatorEventEnvelope>,
        hardware_uid: String,
    ) -> Self {
        Self {
            event_rx,
            event_tx,
            hardware_uid,
        }
    }

    pub async fn run(mut self) {
        let (actuator_event_tx, mut actuator_event_rx) = mpsc::channel(100);
        let mut actuator_bus = EventBus::new();
        actuator_bus.subscribe(TokioActuatorEventHandler(actuator_event_tx));
        let mut knot_runtime = KnotRuntime::new()
            .with_actuator_hardware_uid(self.hardware_uid.clone())
            .with_actuator_event_producer(actuator_bus.producer());
        let mut event_counter = 0_u128;
        let mut envelope_bus = EventBus::new();
        envelope_bus.subscribe(TokioEnvelopeHandler(self.event_tx));
        publish_runtime_event(
            &mut envelope_bus,
            &mut event_counter,
            KnotActuatorEvent::Hello(KnotHello {
                hardware_uid: self.hardware_uid.clone(),
            }),
        );

        loop {
            tokio::select! {
                Some(input) = self.event_rx.recv() => {
                    match input {
                        TokioKnotActuatorInput::Event(event) => {
                            if let KnotActuatorEvent::Ack(config) = &event {
                                log::info!(
                                    "Local Knot received actuator config ACK knot_id={} configs={}",
                                    config.knot_id,
                                    config.actuator_configs.len()
                                );
                            }

                            if let Err(err) = knot_runtime.handle_actuator_event(event, timestamp_now()) {
                                log::debug!("Local Knot rejected actuator runtime event: {err:?}");
                            }
                        }
                        TokioKnotActuatorInput::SensorValue { device_uid, value } => {
                            knot_runtime.observe_actuator_sensor_device_value(
                                &device_uid,
                                value,
                                timestamp_now(),
                            );
                        }
                    }
                }
                Some(actuator_envelope) = actuator_event_rx.recv() => {
                    log::debug!("Local Knot produced actuator event: {actuator_envelope:?}");
                    apply_local_actuator_state(&actuator_envelope.payload);
                    publish_runtime_event(
                        &mut envelope_bus,
                        &mut event_counter,
                        KnotActuatorEvent::Actuator(actuator_envelope.payload),
                    );
                }
                else => break,
            }
        }
    }
}

fn apply_local_actuator_state(event: &ActuatorEvent) {
    let ActuatorEvent::ActuatorStateChanged(state) = event else {
        return;
    };

    // MVP: the local runtime has a single known relay on GPIO17. Once actuator
    // configs carry enough driver registry information, this should resolve the
    // driver from the applied actuator config instead of using MIST_RELAY.
    match RelayDriver::new(MIST_RELAY)
        .and_then(|driver| driver.apply(RelayState::new(MIST_RELAY, state.active)))
    {
        Ok(()) => {
            log::info!(
                "Local Knot applied relay state actuator_id={} rule_id={} active={}",
                state.actuator_id,
                state.rule_id,
                state.active
            );
        }
        Err(err) => {
            log::error!("Local Knot failed to apply relay state: {err:?}");
        }
    }
}

fn event_id_from_counter(counter: u128) -> EventId {
    EventId::new_with_random_bytes(counter.to_be_bytes())
}

fn publish_runtime_event(
    envelope_bus: &mut EventBus<KnotActuatorEvent>,
    event_counter: &mut u128,
    event: KnotActuatorEvent,
) {
    *event_counter = event_counter.wrapping_add(1);
    let envelope = EventEnvelope::new_with_id(
        event_id_from_counter(*event_counter),
        (),
        timestamp_now(),
        event,
    );

    if envelope_bus.producer().publish(envelope).is_err() {
        log::debug!("Local Knot actuator event receiver dropped");
    }
}

fn timestamp_now() -> Timestamp {
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Timestamp::from_unix_millis(unix_millis)
}

struct TokioEnvelopeHandler(mpsc::Sender<KnotActuatorEventEnvelope>);

impl EventHandler<KnotActuatorEvent> for TokioEnvelopeHandler {
    fn handle(&mut self, event: KnotActuatorEventEnvelope) -> Result<(), EventBusError> {
        self.0
            .try_send(event)
            .map_err(|_| EventBusError::HandlerRejected)
    }
}

struct TokioActuatorEventHandler(mpsc::Sender<EventEnvelope<ActuatorEvent>>);

impl EventHandler<ActuatorEvent> for TokioActuatorEventHandler {
    fn handle(&mut self, event: EventEnvelope<ActuatorEvent>) -> Result<(), EventBusError> {
        self.0
            .try_send(event)
            .map_err(|_| EventBusError::HandlerRejected)
    }
}
