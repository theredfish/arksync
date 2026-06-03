// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_knot::application::{KnotActuatorEvent, KnotActuatorEventEnvelope};
use eyre::Result;

use super::actuator_ack::handle_actuator_ack;
use super::actuator_hello::handle_actuator_hello;
use super::actuator_runtime_event::handle_actuator_runtime_event;

pub async fn handle_actuator_event(
    event: KnotActuatorEventEnvelope,
    knot_event_tx: &tokio::sync::mpsc::Sender<KnotActuatorEvent>,
) -> Result<()> {
    log::debug!("Hub received local Knot actuator event: {event:?}");

    match event.payload {
        KnotActuatorEvent::Hello(hello) => handle_actuator_hello(hello, knot_event_tx).await?,
        KnotActuatorEvent::Ack(config) => handle_actuator_ack(config),
        KnotActuatorEvent::Actuator(event) => handle_actuator_runtime_event(event).await?,
    }

    Ok(())
}
