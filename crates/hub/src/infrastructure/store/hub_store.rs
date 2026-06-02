// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlx::PgExecutor;

use crate::infrastructure::store::{HubRecord, SystemUserRecord};

pub async fn upsert_system_user<'e, E>(
    executor: E,
    user: &SystemUserRecord,
) -> Result<(), sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        r#"
        insert into users (
            id,
            username,
            password
        )
        values (
            $1,
            $2,
            $3
        )
        on conflict (id) do update
        set
            username = excluded.username,
            password = excluded.password,
            deleted_at = null
        "#,
    )
    .bind(user.id)
    .bind(&user.username)
    .bind(&user.password)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn upsert_station_hub<'e, E>(executor: E, hub: &HubRecord) -> Result<(), sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        r#"
        insert into station_hubs (
            id,
            user_id,
            name,
            hardware_uid
        )
        values (
            $1,
            $2,
            $3,
            $4
        )
        on conflict (id) do update
        set
            user_id = excluded.user_id,
            name = excluded.name,
            hardware_uid = excluded.hardware_uid,
            deleted_at = null
        "#,
    )
    .bind(hub.id)
    .bind(hub.user_id)
    .bind(&hub.name)
    .bind(&hub.hardware_uid)
    .execute(executor)
    .await?;

    Ok(())
}
