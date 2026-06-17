// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::actuators::handle_actuator_runtime_event;
use arksync_knot::application::{KnotMessage, KnotMessageEnvelope};
use eyre::Result;

use super::knot_hello::handle_knot_hello;

pub async fn handle_knot_event(
    event: KnotMessageEnvelope,
    knot_message_tx: &tokio::sync::mpsc::Sender<KnotMessage>,
) -> Result<()> {
    log::debug!("Hub received local Knot message: {event:?}");

    match event.payload {
        KnotMessage::Hello(hello) => handle_knot_hello(hello, knot_message_tx).await?,
        KnotMessage::Ack(ack) => {
            log::debug!(
                "Hub ignored Knot ACK echo hardware_uid={} knot_id={}",
                ack.config.hardware_uid,
                ack.config.knot_id
            );
        }
        KnotMessage::Actuator(event) => handle_actuator_runtime_event(event).await?,
    }

    Ok(())
}
