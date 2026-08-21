// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::knot_protocol_config_for_hardware_uid;
use crate::application::{actuator_config_ack_for_knot_hardware_uid, HubKnotError};
use crate::config::CONFIG;
use crate::infrastructure::store::{knot as knot_store, KnotRecord};
use arksync_knot::application::{KnotAck, KnotHello};
use arksync_knot_protocol::{KnotConfig as ProtocolKnotConfig, KnotHello as ProtocolKnotHello};
use arksync_utils::uuid::new_v4;
use sqlx::{PgExecutor, PgTransaction};

pub async fn list_knots(executor: impl PgExecutor<'_>) -> Result<Vec<KnotRecord>, HubKnotError> {
    Ok(knot_store::list_station_knots(executor).await?)
}

pub async fn ack_knot_hello(
    txn: &mut PgTransaction<'_>,
    hello: &KnotHello,
) -> Result<KnotAck, HubKnotError> {
    ensure_station_knot_for_hello(txn, hello).await?;
    let config = actuator_config_ack_for_knot_hardware_uid(txn, &hello.hardware_uid).await?;

    Ok(KnotAck { config })
}

pub async fn register_knot_hello(
    txn: &mut PgTransaction<'_>,
    hello: &ProtocolKnotHello,
) -> Result<ProtocolKnotConfig, HubKnotError> {
    ensure_station_knot_for_hardware_uid(txn, &hello.hardware_uid).await?;
    Ok(knot_protocol_config_for_hardware_uid(txn, &hello.hardware_uid).await?)
}

async fn ensure_station_knot_for_hello(
    txn: &mut PgTransaction<'_>,
    hello: &KnotHello,
) -> Result<KnotRecord, HubKnotError> {
    ensure_station_knot_for_hardware_uid(txn, &hello.hardware_uid).await
}

async fn ensure_station_knot_for_hardware_uid(
    txn: &mut PgTransaction<'_>,
    hardware_uid: &str,
) -> Result<KnotRecord, HubKnotError> {
    let role = if hardware_uid == CONFIG.local_knot_hardware_uid {
        "local_hub"
    } else {
        "remote_knot"
    };
    let knot = KnotRecord {
        id: new_v4(),
        hub_id: CONFIG.local_hub_id,
        name: default_knot_name(hardware_uid),
        hardware_uid: hardware_uid.to_string(),
        role: role.to_string(),
        status: "awake".to_string(),
        config_version: 1,
        applied_config_version: None,
        config_status: "pending".to_string(),
        config_error: None,
    };

    Ok(knot_store::find_or_insert_station_knot_by_hardware_uid(txn, &knot).await?)
}

fn default_knot_name(hardware_uid: &str) -> String {
    if hardware_uid == CONFIG.local_knot_hardware_uid {
        return CONFIG.local_knot_name.clone();
    }

    format!("ArkSync knot {hardware_uid}")
}
