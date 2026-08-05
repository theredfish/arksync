// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::ack_knot_hello;
use arksync_knot::application::{KnotHello, KnotMessage};
use eyre::{eyre, Result, WrapErr};
use sqlx::PgPool;

pub(super) async fn handle_knot_hello(
    pool: &PgPool,
    hello: KnotHello,
    knot_message_tx: &tokio::sync::mpsc::Sender<KnotMessage>,
) -> Result<()> {
    let mut txn = pool
        .begin()
        .await
        .wrap_err("failed to begin Knot Hello transaction")?;
    let ack = ack_knot_hello(&mut txn, &hello)
        .await
        .wrap_err("failed to ACK Knot Hello")?;
    txn.commit()
        .await
        .wrap_err("failed to commit Knot Hello transaction")?;
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
