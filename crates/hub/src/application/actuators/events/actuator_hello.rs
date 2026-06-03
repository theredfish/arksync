// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::actuator_config_ack_for_knot_hardware_uid;
use arksync_knot::application::{KnotActuatorEvent, KnotHello};
use eyre::{eyre, Result, WrapErr};

pub(super) async fn handle_actuator_hello(
    hello: KnotHello,
    knot_event_tx: &tokio::sync::mpsc::Sender<KnotActuatorEvent>,
) -> Result<()> {
    let ack = actuator_config_ack_for_knot_hardware_uid(arksync_db::pool(), &hello.hardware_uid)
        .await
        .wrap_err("failed to load actuator config for Knot Hello")?;
    log::info!(
        "Hub ACKs local Knot actuator runtime hardware_uid={} knot_id={} actuator_configs={}",
        ack.hardware_uid,
        ack.knot_id,
        ack.actuator_configs.len()
    );
    knot_event_tx
        .send(KnotActuatorEvent::Ack(ack))
        .await
        .map_err(|_| eyre!("local Knot actuator event receiver dropped"))?;

    Ok(())
}
