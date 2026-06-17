// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::ack_knot_hello;
use arksync_knot::application::{KnotHello, KnotMessage};
use eyre::{eyre, Result, WrapErr};

pub(super) async fn handle_knot_hello(
    hello: KnotHello,
    knot_message_tx: &tokio::sync::mpsc::Sender<KnotMessage>,
) -> Result<()> {
    let ack = ack_knot_hello(arksync_db::pool(), &hello)
        .await
        .wrap_err("failed to ACK Knot Hello")?;
    log::info!(
        "Hub ACKs Knot hello hardware_uid={} knot_id={} actuator_configs={} sensor_bindings={}",
        ack.config.hardware_uid,
        ack.config.knot_id,
        ack.config.actuator_configs.len(),
        ack.config.sensor_bindings.len()
    );
    knot_message_tx
        .send(KnotMessage::Ack(ack))
        .await
        .map_err(|_| eyre!("local Knot message receiver dropped"))?;

    Ok(())
}
