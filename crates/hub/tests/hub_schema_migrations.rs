// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

const OWNERSHIP_UP: &str =
    include_str!("../../db/src/migrations/local/20261016000000_knot_hub_ownership.up.sql");
const OWNERSHIP_DOWN: &str =
    include_str!("../../db/src/migrations/local/20261016000000_knot_hub_ownership.down.sql");

const RENAME_UP: &str =
    include_str!("../../db/src/migrations/local/20261016010000_rename_hubs_and_knots.up.sql");
const RENAME_DOWN: &str =
    include_str!("../../db/src/migrations/local/20261016010000_rename_hubs_and_knots.down.sql");

#[arksync_testing::test]
async fn ownership_migration_backfills_receipts_and_round_trips_existing_data(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    sqlx::raw_sql(RENAME_DOWN).execute(&mut *txn).await?;
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

async fn contents(
    txn: &mut sqlx::PgTransaction<'_>,
    legacy: bool,
) -> eyre::Result<serde_json::Value> {
    let mut contents = serde_json::Map::new();
    for (table, old_table, owner) in [
        ("users", "users", None),
        ("hubs", "station_hubs", None),
        ("knots", "station_knots", Some(("station_hub_id", "hub_id"))),
        ("sensors", "sensors", Some(("station_knot_id", "knot_id"))),
        (
            "actuators",
            "actuators",
            Some(("station_knot_id", "knot_id")),
        ),
        ("actuator_rules", "actuator_rules", None),
        ("sensor_measurements", "sensor_measurements", None),
        (
            "knot_config_deliveries",
            "knot_config_deliveries",
            Some(("station_knot_id", "knot_id")),
        ),
        ("knot_message_receipts", "knot_message_receipts", None),
    ] {
        let source = if legacy { old_table } else { table };
        let row = match (legacy, owner) {
            (true, Some((old, new))) => {
                format!("to_jsonb(r) - '{old}' || jsonb_build_object('{new}', r.{old})")
            }
            _ => "to_jsonb(r)".to_string(),
        };
        let key = if matches!(table, "knot_config_deliveries" | "knot_message_receipts") {
            "event_id"
        } else {
            "id"
        };
        let value = sqlx::query_scalar(&format!(
            "select jsonb_agg({row} order by r.{key}) from {source} r"
        ))
        .fetch_one(&mut **txn)
        .await?;
        contents.insert(table.into(), value);
    }
    Ok(serde_json::Value::Object(contents))
}

#[arksync_testing::test]
async fn renaming_preserves_rows_relations_constraints_and_triggers(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    sqlx::raw_sql(RENAME_DOWN).execute(&mut *txn).await?;
    sqlx::raw_sql(r#"
        insert into users (username, password) values ('rename-test', 'unused');
        insert into station_hubs (user_id, name, hardware_uid)
            select id, 'Existing Hub', 'hub-before-rename' from users;
        insert into station_knots (station_hub_id, name, hardware_uid, role, status, config_version, applied_config_version, config_status)
            select id, 'Existing Knot', 'knot-before-rename', 'local_hub', 'sleeping', 7, 6, 'pending' from station_hubs;
        insert into sensors (station_knot_id, device_uid, kind, driver, protocol, connection)
            select id, 'existing-sensor', 'temperature', 'atlas_scientific_ezo', 'uart', '{"port_name":"/dev/ttyUSB0"}' from station_knots;
        insert into actuators (station_knot_id, device_uid, kind, backend, protocol, gpio_pin)
            select id, 'existing-relay', 'relay', 'linux_gpiod', 'gpio', 17 from station_knots;
        insert into actuator_rules (actuator_id, sensor_id, name, threshold)
            select a.id, s.id, 'existing-rule', 25.5 from actuators a cross join sensors s;
        insert into sensor_measurements (event_id, source_parent_hub_id, source_knot_id, sensor_id, sensor_kind, unit, value, measured_at, received_at)
            select gen_random_uuid(), k.station_hub_id, k.id, s.id, 'temperature', 'celsius', 28.8, now(), now()
            from sensors s join station_knots k on k.id = s.station_knot_id;
        insert into knot_config_deliveries (event_id, station_knot_id, config_version, config, outcome, error)
            select gen_random_uuid(), id, 5, '{"version":5}', 'rejected', 'GPIO unavailable' from station_knots;
        insert into knot_message_receipts (hub_id, event_id, source_hardware_uid, message_kind)
            select station_hub_id, gen_random_uuid(), hardware_uid, 'hello' from station_knots;
    "#).execute(&mut *txn).await?;
    let before = contents(&mut txn, true).await?;
    // Constraint OIDs and endpoints must survive, not just their names.
    let constraints_sql = r#"
        select jsonb_agg(jsonb_build_array(oid, contype, conrelid, confrelid, conkey, confkey) order by oid)
        from pg_constraint where connamespace = 'public'::regnamespace
    "#;
    let constraints: serde_json::Value = sqlx::query_scalar(constraints_sql)
        .fetch_one(&mut *txn)
        .await?;
    sqlx::raw_sql(RENAME_UP).execute(&mut *txn).await?;
    assert_eq!(contents(&mut txn, false).await?, before);
    sqlx::raw_sql(RENAME_DOWN).execute(&mut *txn).await?;
    assert_eq!(contents(&mut txn, true).await?, before);
    sqlx::raw_sql(RENAME_UP).execute(&mut *txn).await?;
    assert_eq!(contents(&mut txn, false).await?, before);
    let after_constraints: serde_json::Value = sqlx::query_scalar(constraints_sql)
        .fetch_one(&mut *txn)
        .await?;
    assert_eq!(after_constraints, constraints);

    for table in ["hubs", "knots"] {
        let updated: bool = sqlx::query_scalar(&format!("update {table} set name = 'Renamed', updated_at = '2000-01-01' returning updated_at = now()")).fetch_one(&mut *txn).await?;
        assert!(updated, "updated_at trigger must still work for {table}");
    }
    // Name-sensitive application error mapping must follow the renamed index.
    let mut savepoint = sqlx::Acquire::begin(&mut txn).await?;
    let duplicate = sqlx::query(
        r#"
        insert into sensors (knot_id, device_uid, kind, driver, protocol)
        select knot_id, device_uid, kind, driver, protocol from sensors
    "#,
    )
    .execute(&mut *savepoint)
    .await
    .unwrap_err();
    assert!(matches!(
        arksync_hub::infrastructure::store::SensorStoreError::from(duplicate),
        arksync_hub::infrastructure::store::SensorStoreError::AlreadyExists
    ));
    savepoint.rollback().await?;

    let mut savepoint = sqlx::Acquire::begin(&mut txn).await?;
    let duplicate_local = sqlx::query(
        r#"
        insert into knots (hub_id, name, hardware_uid, role)
        select hub_id, 'Second local Knot', 'another-uid', 'local_hub' from knots
    "#,
    )
    .execute(&mut *savepoint)
    .await
    .unwrap_err();
    assert_eq!(
        duplicate_local.as_database_error().unwrap().constraint(),
        Some("knots_one_local_knot_per_hub")
    );
    savepoint.rollback().await?;

    let mut savepoint = sqlx::Acquire::begin(&mut txn).await?;
    let orphan = sqlx::query("update knots set hub_id = gen_random_uuid()")
        .execute(&mut *savepoint)
        .await
        .unwrap_err();
    assert!(orphan
        .as_database_error()
        .unwrap()
        .is_foreign_key_violation());
    savepoint.rollback().await?;
    txn.rollback().await?;
    Ok(())
}
