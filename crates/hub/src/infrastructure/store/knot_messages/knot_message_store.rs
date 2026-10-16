// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::EventId;
use sqlx::PgExecutor;

use crate::domain::HubId;
use crate::infrastructure::store::KnotMessageStoreError;

pub async fn insert_knot_message_receipt(
    executor: impl PgExecutor<'_>,
    hub_id: HubId,
    event_id: EventId,
    source_hardware_uid: &str,
    message_kind: &str,
) -> Result<bool, KnotMessageStoreError> {
    let result = sqlx::query(
        r#"
        insert into knot_message_receipts (
            event_id,
            source_hardware_uid,
            message_kind,
            hub_id
        )
        values ($1, $2, $3, $4)
        on conflict (hub_id, source_hardware_uid, event_id) do nothing
        "#,
    )
    .bind(event_id.uuid_v4())
    .bind(source_hardware_uid)
    .bind(message_kind)
    .bind(hub_id.uuid_v4())
    .execute(executor)
    .await?;

    Ok(result.rows_affected() == 1)
}
