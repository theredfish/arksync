// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_actuator::infrastructure::events::{ActuatorEvent, AddActuator};
use arksync_actuator::services::ActuatorService;
use arksync_bus::{EventBus, EventBusError, EventEnvelope, EventHandler, EventId, Timestamp};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

use crate::application::{KnotActuatorEvent, KnotActuatorEventEnvelope, KnotHello};

pub struct TokioKnotActuatorService {
    event_rx: mpsc::Receiver<KnotActuatorEvent>,
    event_tx: mpsc::Sender<KnotActuatorEventEnvelope>,
    hardware_uid: String,
}

impl TokioKnotActuatorService {
    pub fn with_channels(
        event_rx: mpsc::Receiver<KnotActuatorEvent>,
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
        let mut actuator_service =
            ActuatorService::new().with_event_producer(actuator_bus.producer());
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
                Some(event) = self.event_rx.recv() => {
                    match event {
                        KnotActuatorEvent::Ack(config) => {
                            if config.hardware_uid != self.hardware_uid {
                                log::debug!(
                                    "Local Knot ignored actuator config ACK for hardware_uid={}",
                                    config.hardware_uid
                                );
                                continue;
                            }

                            log::info!(
                                "Local Knot received actuator config ACK knot_id={} configs={}",
                                config.knot_id,
                                config.actuator_configs.len()
                            );

                            for config in config.actuator_configs {
                                actuator_service.read_event(
                                    ActuatorEvent::AddActuator(AddActuator { config }),
                                    timestamp_now(),
                                );
                            }
                        }
                        KnotActuatorEvent::Actuator(command) => {
                            actuator_service.read_event(command, timestamp_now());
                        }
                        KnotActuatorEvent::Hello(_) => {}
                    }
                }
                Some(actuator_envelope) = actuator_event_rx.recv() => {
                    log::debug!("Local Knot produced actuator event: {actuator_envelope:?}");
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
