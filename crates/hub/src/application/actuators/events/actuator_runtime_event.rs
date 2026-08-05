// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::record_actuator_runtime_status;
use arksync_actuator::application::protocol::ActuatorMessage;
use eyre::{Result, WrapErr};
use sqlx::PgPool;

pub(crate) async fn handle_actuator_runtime_event(
    pool: &PgPool,
    event: ActuatorMessage,
) -> Result<()> {
    match event {
        ActuatorMessage::ConfigApplied(applied) => {
            log::info!(
                "Local Knot applied actuator config config_id={} version={}",
                applied.config_id,
                applied.version
            );
        }
        ActuatorMessage::ConfigRejected(rejected) => {
            log::error!(
                "Local Knot rejected actuator config config_id={} version={} reason={}",
                rejected.config_id,
                rejected.version,
                rejected.reason
            );
        }
        ActuatorMessage::RuntimeStatus(status) => {
            log::debug!(
                "Local Knot actuator runtime status rules={} actuators={} last_seen_sensor_values={}",
                status.rules.len(),
                status.actuators.len(),
                status.last_seen_sensor_values.len()
            );
            let mut txn = pool
                .begin()
                .await
                .wrap_err("failed to begin actuator runtime status transaction")?;
            record_actuator_runtime_status(&mut txn, &status)
                .await
                .wrap_err("failed to record local Knot actuator runtime status")?;
            txn.commit()
                .await
                .wrap_err("failed to commit actuator runtime status transaction")?;
        }
        ActuatorMessage::ActuatorStateChanged(state) => {
            log::info!(
                "Local Knot actuator state changed actuator_id={} rule_id={} sensor_id={} value={} active={}",
                state.actuator_id,
                state.rule_id,
                state.sensor_id,
                state.sensor_value,
                state.active
            );
        }
        ActuatorMessage::AddActuator(_)
        | ActuatorMessage::EnableActuator(_)
        | ActuatorMessage::DisableActuator(_)
        | ActuatorMessage::RemoveActuator(_) => {}
    }

    Ok(())
}
