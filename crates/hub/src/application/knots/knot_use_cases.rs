// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{knot_protocol_config_for_hardware_uid, HubKnotError};
use crate::domain::HubId;
use crate::infrastructure::store::{knot as knot_store, KnotRecord};
use arksync_protocol::knot::{KnotConfig as ProtocolKnotConfig, KnotHello as ProtocolKnotHello};
use arksync_utils::uuid::new_v4;
use sqlx::{PgExecutor, PgTransaction};

pub async fn list_knots(
    executor: impl PgExecutor<'_>,
    hub_id: HubId,
) -> Result<Vec<KnotRecord>, HubKnotError> {
    Ok(knot_store::list_knots(executor, hub_id).await?)
}

pub async fn register_knot_hello(
    txn: &mut PgTransaction<'_>,
    hub_id: HubId,
    hello: &ProtocolKnotHello,
) -> Result<ProtocolKnotConfig, HubKnotError> {
    ensure_knot_for_hardware_uid(txn, hub_id, &hello.hardware_uid).await?;
    Ok(knot_protocol_config_for_hardware_uid(txn, hub_id, &hello.hardware_uid).await?)
}

async fn ensure_knot_for_hardware_uid(
    txn: &mut PgTransaction<'_>,
    hub_id: HubId,
    hardware_uid: &str,
) -> Result<KnotRecord, HubKnotError> {
    let knot = KnotRecord {
        id: new_v4(),
        hub_id: hub_id.uuid_v4(),
        name: format!("ArkSync knot {hardware_uid}"),
        hardware_uid: hardware_uid.to_string(),
        role: "remote_knot".to_string(),
        status: "awake".to_string(),
        config_version: 1,
        applied_config_version: None,
        config_status: "pending".to_string(),
        config_error: None,
    };

    Ok(knot_store::find_or_insert_knot_by_hardware_uid(&mut **txn, &knot).await?)
}
