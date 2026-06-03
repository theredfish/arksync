// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::SensorRegistry;
use arksync_sensor::infrastructure::events::SensorProvisioned;
use eyre::{Result, WrapErr};

use super::sensor_plugged::extract_plugged_sensor;
use super::HubSensorEventEnvelope;

pub(super) async fn handle_sensor_provisioned(
    event: &HubSensorEventEnvelope,
    provisioned: &SensorProvisioned,
    sensor_registry: &mut SensorRegistry,
) -> Result<()> {
    let plugged_sensor = extract_plugged_sensor(
        &event.source,
        provisioned.device_uid.clone(),
        &provisioned.sensor,
    );
    let sensor_id = sensor_registry
        .ensure_sensor_registered(arksync_db::pool(), plugged_sensor)
        .await
        .wrap_err("failed to register provisioned local Knot sensor")?;

    log::info!(
        "Hub registered local Knot sensor device_uid={} sensor_id={}",
        provisioned.device_uid,
        sensor_id
    );

    Ok(())
}
