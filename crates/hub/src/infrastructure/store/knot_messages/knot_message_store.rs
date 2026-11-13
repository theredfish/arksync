// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_event_router::EventId;
use arksync_protocol::knot::KnotEnvelope;
use arksync_protocol::ArkSyncActor;
use sqlx::{PgExecutor, PgTransaction};

use crate::infrastructure::store::KnotMessageStoreError;

pub enum KnotMessageClaim {
    New,
    Replay(Option<KnotEnvelope>),
    Conflict,
}

/// Claims the request and its effects in the caller's transaction. A concurrent
/// duplicate waits for the original transaction before reading its committed reply.
pub async fn claim_knot_message(
    txn: &mut PgTransaction<'_>,
    event: &KnotEnvelope,
    message_kind: &str,
) -> Result<KnotMessageClaim, KnotMessageStoreError> {
    let ArkSyncActor::Knot { hardware_uid } = &event.source else {
        return Ok(KnotMessageClaim::Conflict);
    };
    let request = serde_json::to_value(event)?;
    let inserted = sqlx::query(
        r#"
        insert into knot_message_receipts (event_id, source_hardware_uid, message_kind, request)
        values ($1, $2, $3, $4)
        on conflict (event_id) do nothing
        "#,
    )
    .bind(event.id.uuid_v4())
    .bind(hardware_uid)
    .bind(message_kind)
    .bind(&request)
    .execute(&mut **txn)
    .await?;
    if inserted.rows_affected() == 1 {
        return Ok(KnotMessageClaim::New);
    }
    let (previous, reply): (
        Option<serde_json::Value>,
        Option<sqlx::types::Json<KnotEnvelope>>,
    ) = sqlx::query_as(
        "select request, reply from knot_message_receipts where event_id = $1 for update",
    )
    .bind(event.id.uuid_v4())
    .fetch_one(&mut **txn)
    .await?;
    if previous.as_ref() != Some(&request) {
        return Ok(KnotMessageClaim::Conflict);
    }
    Ok(KnotMessageClaim::Replay(reply.map(|json| json.0)))
}

pub async fn save_knot_message_reply(
    executor: impl PgExecutor<'_>,
    event_id: EventId,
    reply: Option<&KnotEnvelope>,
) -> Result<(), sqlx::Error> {
    sqlx::query("update knot_message_receipts set reply = $2 where event_id = $1")
        .bind(event_id.uuid_v4())
        .bind(reply.map(sqlx::types::Json))
        .execute(executor)
        .await?;
    Ok(())
}
