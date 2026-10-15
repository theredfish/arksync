// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::EventId;
use arksync_protocol::knot::KnotConfig;
use arksync_utils::uuid::Uuid;
use sqlx::{PgExecutor, PgTransaction};

/// Register the exact configuration before sending it. Reusing an ID with
/// different content or issuing a version other than the desired one fails.
pub async fn insert_knot_config_delivery(
    executor: impl PgExecutor<'_>,
    event_id: EventId,
    config: &KnotConfig,
) -> Result<bool, sqlx::Error> {
    let Ok(version) = i64::try_from(config.version) else {
        return Ok(false);
    };
    let result = sqlx::query(
        r#"
        insert into knot_config_deliveries (event_id, station_knot_id, config_version, config)
        select $1, id, $3, $4 from station_knots
        where id = $2 and config_version = $3 and deleted_at is null
        on conflict (event_id) do update set event_id = excluded.event_id
        where knot_config_deliveries.station_knot_id = excluded.station_knot_id
          and knot_config_deliveries.config = excluded.config
        "#,
    )
    .bind(event_id.uuid_v4())
    .bind(Uuid::from_bytes(config.knot_id))
    .bind(version)
    .bind(sqlx::types::Json(config))
    .execute(executor)
    .await?;
    Ok(result.rows_affected() == 1)
}

#[derive(sqlx::FromRow)]
struct ConfigDelivery {
    station_knot_id: Uuid,
    config_version: i64,
    outcome: Option<String>,
    error: Option<String>,
}

/// Correlate and settle one delivery, preserving monotonic applied versions.
/// A stale but legitimate confirmation is accepted without replacing newer state.
pub async fn settle_knot_config_delivery(
    txn: &mut PgTransaction<'_>,
    event_id: EventId,
    hardware_uid: &str,
    version: u64,
    rejection: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let delivery: Option<ConfigDelivery> = sqlx::query_as(
        r#"
        select d.station_knot_id, d.config_version, d.outcome, d.error
        from knot_config_deliveries d
        join station_knots k on k.id = d.station_knot_id
        where d.event_id = $1 and k.hardware_uid = $2 and k.deleted_at is null
        for update of d
        "#,
    )
    .bind(event_id.uuid_v4())
    .bind(hardware_uid)
    .fetch_optional(&mut **txn)
    .await?;
    let Some(delivery) = delivery else {
        return Ok(false);
    };
    if u64::try_from(delivery.config_version).ok() != Some(version) {
        return Ok(false);
    }
    let outcome = if rejection.is_some() {
        "rejected"
    } else {
        "applied"
    };
    if let Some(previous) = delivery.outcome {
        return Ok(previous == outcome && delivery.error.as_deref() == rejection);
    }
    sqlx::query("update knot_config_deliveries set outcome = $2, error = $3 where event_id = $1")
        .bind(event_id.uuid_v4())
        .bind(outcome)
        .bind(rejection)
        .execute(&mut **txn)
        .await?;
    sqlx::query(
        r#"
        update station_knots
        set applied_config_version = case when $3 = 'applied'
                then greatest(applied_config_version, $2) else applied_config_version end,
            config_status = case when config_version = $2
                and (applied_config_version is null or applied_config_version < $2 or $3 = 'applied')
                then $3 else config_status end,
            config_error = case when config_version = $2
                and (applied_config_version is null or applied_config_version < $2 or $3 = 'applied')
                then $4 else config_error end
        where id = $1 and config_version >= $2 and deleted_at is null
        "#,
    )
    .bind(delivery.station_knot_id)
    .bind(delivery.config_version)
    .bind(outcome)
    .bind(rejection)
    .execute(&mut **txn)
    .await?;
    Ok(true)
}
