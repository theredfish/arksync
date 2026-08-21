// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_actuator::application::protocol::ActuatorMessage;
use arksync_actuator::relay::{RelayDriver, RelayState, MIST_RELAY};
use arksync_bus::{
    EventEnvelope, EventHandler, EventHandlerError, EventId, EventRouter, Timestamp,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

use crate::application::{KnotRuntime, LegacyKnotActuatorEnvelope, LegacyKnotActuatorMessage};

pub enum TokioKnotActuatorInput {
    Message(LegacyKnotActuatorMessage),
    SensorValue { device_uid: String, value: f64 },
}

pub struct TokioKnotActuatorService {
    event_rx: mpsc::Receiver<TokioKnotActuatorInput>,
    event_tx: mpsc::Sender<LegacyKnotActuatorEnvelope>,
    hardware_uid: String,
}

impl TokioKnotActuatorService {
    pub fn with_channels(
        event_rx: mpsc::Receiver<TokioKnotActuatorInput>,
        event_tx: mpsc::Sender<LegacyKnotActuatorEnvelope>,
        hardware_uid: String,
    ) -> Self {
        Self {
            event_rx,
            event_tx,
            hardware_uid,
        }
    }

    pub async fn run(mut self) {
        let relay_driver = local_relay_driver();
        let (actuator_event_tx, mut actuator_event_rx) = mpsc::channel(100);
        let mut actuator_router = EventRouter::new();
        actuator_router.subscribe(TokioActuatorEventHandler(actuator_event_tx));
        let mut knot_runtime = KnotRuntime::new()
            .with_hardware_uid(self.hardware_uid.clone())
            .with_actuator_event_publisher(actuator_router.publisher());
        let mut event_counter = 0_u128;
        let mut envelope_router = EventRouter::new();
        envelope_router.subscribe(TokioEnvelopeHandler(self.event_tx));
        loop {
            tokio::select! {
                Some(input) = self.event_rx.recv() => {
                    match input {
                        TokioKnotActuatorInput::Message(event) => {
                            if let LegacyKnotActuatorMessage::ApplyConfig(config) = &event {
                                log::info!(
                                    "Local Knot received actuator config ACK knot_id={} configs={}",
                                    config.knot_id,
                                    config.actuator_configs.len()
                                );
                            }

                            if let Err(err) = knot_runtime.handle_legacy_actuator_message(event, timestamp_now()) {
                                log::debug!("Local Knot rejected actuator runtime event: {err:?}");
                            }
                        }
                        TokioKnotActuatorInput::SensorValue { device_uid, value } => {
                            log::debug!(
                                "Local Knot actuator runtime observed sensor value device_uid={} value={}",
                                device_uid,
                                value
                            );
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
                    apply_local_actuator_state(relay_driver.as_ref(), &actuator_envelope.payload);
                    publish_runtime_event(
                        &mut envelope_router,
                        &mut event_counter,
                        LegacyKnotActuatorMessage::Actuator(actuator_envelope.payload),
                    );
                }
                else => break,
            }
        }
    }
}

fn local_relay_driver() -> Option<RelayDriver> {
    match RelayDriver::new(MIST_RELAY) {
        Ok(driver) => {
            log::info!(
                "Local Knot opened relay driver relay_id={} gpio_bcm_pin={} active_low={}",
                MIST_RELAY.id,
                MIST_RELAY.gpio_bcm_pin,
                MIST_RELAY.active_low
            );
            Some(driver)
        }
        Err(err) => {
            log::error!("Local Knot failed to open relay driver: {err:?}");
            None
        }
    }
}

fn apply_local_actuator_state(relay_driver: Option<&RelayDriver>, event: &ActuatorMessage) {
    let ActuatorMessage::ActuatorStateChanged(state) = event else {
        return;
    };
    let Some(relay_driver) = relay_driver else {
        log::error!(
            "Local Knot cannot apply relay state because the relay driver is unavailable actuator_id={} rule_id={} active={}",
            state.actuator_id,
            state.rule_id,
            state.active
        );
        return;
    };

    // MVP: the local runtime has a single known relay on GPIO17. Once actuator
    // configs carry enough driver registry information, this should resolve the
    // driver from the applied actuator config instead of using MIST_RELAY.
    log::info!(
        "Local Knot applying relay state actuator_id={} rule_id={} sensor_value={} active={}",
        state.actuator_id,
        state.rule_id,
        state.sensor_value,
        state.active
    );
    match relay_driver.apply(RelayState::new(MIST_RELAY, state.active)) {
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
    EventId::from_bytes(counter.to_be_bytes())
}

fn publish_runtime_event(
    envelope_router: &mut EventRouter<LegacyKnotActuatorMessage>,
    event_counter: &mut u128,
    event: LegacyKnotActuatorMessage,
) {
    *event_counter = event_counter.wrapping_add(1);
    let envelope = EventEnvelope::new_with_id(
        event_id_from_counter(*event_counter),
        (),
        timestamp_now(),
        event,
    );

    if envelope_router.publish(&envelope).rejected > 0 {
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

struct TokioEnvelopeHandler(mpsc::Sender<LegacyKnotActuatorEnvelope>);

impl EventHandler<LegacyKnotActuatorMessage> for TokioEnvelopeHandler {
    fn handle(&mut self, event: &LegacyKnotActuatorEnvelope) -> Result<(), EventHandlerError> {
        self.0
            .try_send(event.clone())
            .map_err(|_| EventHandlerError::Rejected)
    }
}

struct TokioActuatorEventHandler(mpsc::Sender<EventEnvelope<ActuatorMessage>>);

impl EventHandler<ActuatorMessage> for TokioActuatorEventHandler {
    fn handle(&mut self, event: &EventEnvelope<ActuatorMessage>) -> Result<(), EventHandlerError> {
        self.0
            .try_send(event.clone())
            .map_err(|_| EventHandlerError::Rejected)
    }
}
