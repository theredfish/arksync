// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{actuator_config_ack_for_knot_hardware_uid, HubKnotError};
use crate::config::CONFIG;
use crate::infrastructure::store::{knot as knot_store, KnotRecord};
use arksync_knot::application::{KnotAck, KnotHello};
use arksync_utils::uuid::new_v4;

pub async fn list_knots(executor: &sqlx::PgPool) -> Result<Vec<KnotRecord>, HubKnotError> {
    Ok(knot_store::list_station_knots(executor).await?)
}

pub async fn ack_knot_hello(
    executor: &sqlx::PgPool,
    hello: &KnotHello,
) -> Result<KnotAck, HubKnotError> {
    ensure_station_knot_for_hello(executor, hello).await?;
    let config = actuator_config_ack_for_knot_hardware_uid(executor, &hello.hardware_uid).await?;

    Ok(KnotAck { config })
}

async fn ensure_station_knot_for_hello(
    executor: &sqlx::PgPool,
    hello: &KnotHello,
) -> Result<KnotRecord, HubKnotError> {
    let role = if hello.hardware_uid == CONFIG.local_knot_hardware_uid {
        "local_hub"
    } else {
        "remote_knot"
    };
    let knot = KnotRecord {
        id: new_v4(),
        hub_id: CONFIG.local_hub_id,
        name: default_knot_name(&hello.hardware_uid),
        hardware_uid: hello.hardware_uid.clone(),
        role: role.to_string(),
        status: "awake".to_string(),
    };

    Ok(knot_store::find_or_insert_station_knot_by_hardware_uid(executor, &knot).await?)
}

fn default_knot_name(hardware_uid: &str) -> String {
    if hardware_uid == CONFIG.local_knot_hardware_uid {
        return CONFIG.local_knot_name.clone();
    }

    format!("ArkSync knot {hardware_uid}")
}
