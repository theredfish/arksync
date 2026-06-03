// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::record_actuator_runtime_status;
use arksync_actuator::infrastructure::events::ActuatorEvent;
use eyre::{Result, WrapErr};

pub(super) async fn handle_actuator_runtime_event(event: ActuatorEvent) -> Result<()> {
    match event {
        ActuatorEvent::ConfigApplied(applied) => {
            log::info!(
                "Local Knot applied actuator config config_id={} version={}",
                applied.config_id,
                applied.version
            );
        }
        ActuatorEvent::ConfigRejected(rejected) => {
            log::error!(
                "Local Knot rejected actuator config config_id={} version={} reason={}",
                rejected.config_id,
                rejected.version,
                rejected.reason
            );
        }
        ActuatorEvent::RuntimeStatus(status) => {
            log::debug!(
                "Local Knot actuator runtime status rules={} actuators={} last_seen_sensor_values={}",
                status.rules.len(),
                status.actuators.len(),
                status.last_seen_sensor_values.len()
            );
            record_actuator_runtime_status(arksync_db::pool(), &status)
                .await
                .wrap_err("failed to record local Knot actuator runtime status")?;
        }
        ActuatorEvent::AddActuator(_)
        | ActuatorEvent::EnableActuator(_)
        | ActuatorEvent::DisableActuator(_)
        | ActuatorEvent::RemoveActuator(_) => {}
    }

    Ok(())
}
