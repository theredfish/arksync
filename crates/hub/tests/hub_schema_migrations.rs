// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

const OWNERSHIP_UP: &str =
    include_str!("../../db/src/migrations/local/20261016000000_knot_hub_ownership.up.sql");
const OWNERSHIP_DOWN: &str =
    include_str!("../../db/src/migrations/local/20261016000000_knot_hub_ownership.down.sql");

#[arksync_testing::test]
async fn ownership_migration_backfills_receipts_and_round_trips_existing_data(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    sqlx::raw_sql(OWNERSHIP_DOWN).execute(&mut *txn).await?;
    sqlx::raw_sql(r#"
        insert into users (username, password) values ('migration-test', 'unused');
        insert into station_hubs (user_id, name, hardware_uid)
            select id, 'Existing Hub', 'hub-before-migration' from users;
        insert into station_knots (station_hub_id, name, hardware_uid, role, config_version, applied_config_version, config_status)
            select id, 'Existing Knot', 'knot-before-migration', 'remote_knot', 7, 6, 'pending' from station_hubs;
        insert into knot_message_receipts (event_id, source_hardware_uid, message_kind)
            values ('00000000-0000-4000-8000-000000000001', 'knot-before-migration', 'hello');
    "#).execute(&mut *txn).await?;
    for _ in 0..2 {
        sqlx::raw_sql(OWNERSHIP_UP).execute(&mut *txn).await?;
        let row: (i64, i64, String) = sqlx::query_as(
            r#"
            select k.config_version, k.applied_config_version, r.message_kind
            from knot_message_receipts r join station_knots k
              on k.station_hub_id = r.hub_id and k.hardware_uid = r.source_hardware_uid
            where r.event_id = '00000000-0000-4000-8000-000000000001'
        "#,
        )
        .fetch_one(&mut *txn)
        .await?;
        assert_eq!(row, (7, 6, "hello".into()));
        sqlx::raw_sql(OWNERSHIP_DOWN).execute(&mut *txn).await?;
    }
    txn.rollback().await?;
    Ok(())
}
