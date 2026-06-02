// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlx::PgExecutor;

use crate::infrastructure::store::KnotRecord;

pub async fn upsert_station_knot<'e, E>(executor: E, knot: &KnotRecord) -> Result<(), sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query(
        r#"
        insert into station_knots (
            id,
            station_hub_id,
            name,
            hardware_uid,
            role,
            status
        )
        values (
            $1,
            $2,
            $3,
            $4,
            'local_hub',
            'awake'
        )
        on conflict (id) do update
        set
            station_hub_id = excluded.station_hub_id,
            name = excluded.name,
            hardware_uid = excluded.hardware_uid,
            role = 'local_hub',
            status = 'awake',
            deleted_at = null
        "#,
    )
    .bind(knot.id)
    .bind(knot.hub_id)
    .bind(&knot.name)
    .bind(&knot.hardware_uid)
    .execute(executor)
    .await?;

    Ok(())
}
