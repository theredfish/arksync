// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_hub::infrastructure::store::{
    hub as hub_store, knot as knot_store, HubRecord, SystemUserRecord,
};
use arksync_hub::{
    handle_knot_protocol_event, knot_protocol_config_for_hardware_uid, list_knots, HubId,
    KnotProtocolEventResult, SensorRegistry,
};
use arksync_protocol::knot::{
    KnotAck, KnotCapabilities, KnotConfig, KnotConfigApplied, KnotConfigRejected,
    KnotControlMessage, KnotEnvelope, KnotHello, KnotMeasurementUnit, KnotMessage, KnotNackReason,
    KnotSensorConnection, KnotSensorDescriptor, KnotSensorKind, KnotSensorMeasurement,
    KnotSensorMeasurementBatch, KnotSensorMessage, KnotSerialPort,
};
use arksync_protocol::ArkSyncActor;
use arksync_utils::uuid::new_v4;
use sqlx::PgPool;

async fn hubs(pool: &PgPool) -> eyre::Result<[HubId; 2]> {
    let user = SystemUserRecord {
        id: new_v4(),
        username: "ownership-test".into(),
        password: "unused".into(),
    };
    hub_store::upsert_system_user(pool, &user).await?;
    let hubs = [HubId::new(), HubId::new()];
    for (index, id) in hubs.iter().enumerate() {
        hub_store::upsert_station_hub(
            pool,
            &HubRecord {
                id: id.uuid_v4(),
                user_id: user.id,
                name: format!("Hub {index}"),
                hardware_uid: format!("hub-{index}"),
            },
        )
        .await?;
    }
    Ok(hubs)
}

fn event(uid: &str, message: KnotControlMessage) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        EventId::new(),
        ArkSyncActor::Knot {
            hardware_uid: uid.into(),
        },
        Timestamp::from_unix_millis(1_780_000_000_000),
        KnotMessage::Control(message),
    )
}

fn hello(uid: &str) -> KnotEnvelope {
    event(
        uid,
        KnotControlMessage::Hello(KnotHello {
            hardware_uid: uid.into(),
            capabilities: KnotCapabilities::default(),
            last_applied_config_version: None,
        }),
    )
}

fn measurement(uid: &str) -> KnotEnvelope {
    let mut envelope = hello(uid);
    envelope.payload = KnotMessage::Sensor(KnotSensorMessage::Measurements(
        KnotSensorMeasurementBatch {
            measurements: vec![KnotSensorMeasurement {
                device_uid: "rkfTHk12L9ZCY39z".into(),
                sensor: KnotSensorDescriptor {
                    hardware_uid: "DK0HFBFB".into(),
                    kind: KnotSensorKind::Temperature,
                    connection: KnotSensorConnection::Uart(KnotSerialPort {
                        port_name: "/dev/ttyUSB0".into(),
                        serial_number: "DK0HFBFB".into(),
                        baud_rate: 9_600,
                    }),
                    firmware: Some(2.15),
                },
                value: 28.8,
                unit: KnotMeasurementUnit::Celsius,
            }],
        },
    ));
    envelope
}

async fn handle(
    pool: &PgPool,
    hub: HubId,
    event: &KnotEnvelope,
) -> eyre::Result<KnotProtocolEventResult> {
    let mut registry = SensorRegistry::load(pool, hub).await?;
    let now = Timestamp::from_unix_millis(1_780_000_000_100);
    let result =
        handle_knot_protocol_event(pool, hub, event, EventId::new(), now, now, &mut registry)
            .await?;
    assert_eq!(
        result.response.as_ref().unwrap().source,
        ArkSyncActor::Hub {
            hub_id: *hub.as_bytes()
        }
    );
    Ok(result)
}

fn config(result: &KnotProtocolEventResult) -> &KnotConfig {
    let KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Hello { config, .. })) =
        &result.response.as_ref().unwrap().payload
    else {
        panic!("expected Hello ACK")
    };
    config
}

fn assert_nack(result: &KnotProtocolEventResult, reason: KnotNackReason) {
    let KnotMessage::Control(KnotControlMessage::Nack(nack)) =
        &result.response.as_ref().unwrap().payload
    else {
        panic!("expected NACK")
    };
    assert_eq!(nack.reason, reason);
    assert!(result.sensor_events.is_empty());
}

#[arksync_testing::test]
async fn hub_owns_many_knots_and_foreign_hello_cannot_transfer_them(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let [a, b] = hubs(&pool).await?;
    let first = handle(&pool, a, &hello("a-1")).await?;
    handle(&pool, a, &hello("a-2")).await?;
    handle(&pool, b, &hello("b-1")).await?;
    let rejoin = handle(&pool, a, &hello("a-1")).await?;
    assert_eq!(config(&first), config(&rejoin));
    assert_nack(
        &handle(&pool, b, &hello("a-1")).await?,
        KnotNackReason::ConfigurationConflict,
    );
    let mut a_knots = list_knots(&pool, a).await?;
    assert_eq!(a_knots.len(), 2);
    assert!(a_knots.iter().all(|knot| knot.hub_id == a.uuid_v4()));
    let b_knots = list_knots(&pool, b).await?;
    assert_eq!(b_knots.len(), 1);
    assert_eq!(b_knots[0].hardware_uid, "b-1");

    let mut knot = a_knots.pop().unwrap();
    knot.hub_id = b.uuid_v4();
    knot.name = "attempted reassignment".into();
    assert!(knot_store::upsert_station_knot(&pool, &knot).await.is_err());
    let unchanged = list_knots(&pool, a)
        .await?
        .into_iter()
        .find(|row| row.id == knot.id)
        .unwrap();
    assert_ne!(unchanged.name, knot.name);
    assert_eq!(unchanged.hub_id, a.uuid_v4());
    Ok(())
}

#[arksync_testing::test]
async fn foreign_messages_cannot_read_config_settle_deliveries_or_poison_receipts(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let [a, b] = hubs(&pool).await?;
    let registered = handle(&pool, a, &hello("owned")).await?;
    let desired = config(&registered);
    let delivery_id = registered.response.as_ref().unwrap().id;
    let mut txn = pool.begin().await?;
    assert!(knot_protocol_config_for_hardware_uid(&mut txn, b, "owned")
        .await
        .is_err());
    txn.rollback().await?;
    assert!(!knot_store::insert_knot_config_delivery(&pool, b, EventId::new(), desired).await?);
    assert!(matches!(
        knot_store::increment_station_knot_config_version(
            &pool,
            b,
            arksync_utils::uuid::Uuid::from_bytes(desired.knot_id)
        )
        .await,
        Err(knot_store::KnotStoreError::NotFound)
    ));
    assert_nack(
        &handle(&pool, b, &event("owned", KnotControlMessage::Heartbeat)).await?,
        KnotNackReason::InvalidPayload,
    );
    let applied = event(
        "owned",
        KnotControlMessage::ConfigApplied(KnotConfigApplied {
            event_id: delivery_id,
            config_version: desired.version,
        }),
    );
    let rejected = event(
        "owned",
        KnotControlMessage::ConfigRejected(KnotConfigRejected {
            event_id: delivery_id,
            config_version: desired.version,
            reason: "foreign rejection".into(),
        }),
    );
    for message in [&applied, &rejected] {
        assert_nack(
            &handle(&pool, b, message).await?,
            KnotNackReason::ConfigurationConflict,
        );
    }
    let knot = list_knots(&pool, a).await?.pop().unwrap();
    assert_eq!(knot.applied_config_version, None);
    assert_eq!(knot.config_status, "pending");
    let outcome: Option<String> =
        sqlx::query_scalar("select outcome from knot_config_deliveries where event_id = $1")
            .bind(delivery_id.uuid_v4())
            .fetch_one(&pool)
            .await?;
    assert_eq!(outcome, None);
    let sample = measurement("owned");
    assert_nack(
        &handle(&pool, b, &sample).await?,
        KnotNackReason::InvalidPayload,
    );
    let receipts: i64 = sqlx::query_scalar("select count(*) from knot_message_receipts")
        .fetch_one(&pool)
        .await?;
    assert_eq!(receipts, 1, "only the legitimate Hello may have a receipt");
    assert!(!handle(&pool, a, &sample).await?.sensor_events.is_empty());
    assert_nack(
        &handle(&pool, b, &sample).await?,
        KnotNackReason::InvalidPayload,
    );
    assert!(handle(&pool, a, &sample).await?.sensor_events.is_empty());
    let own_registry = SensorRegistry::load(&pool, a).await?;
    let foreign_registry = SensorRegistry::load(&pool, b).await?;
    assert!(own_registry
        .sensor_id(knot.id, "rkfTHk12L9ZCY39z")
        .is_some());
    assert!(foreign_registry
        .sensor_id(knot.id, "rkfTHk12L9ZCY39z")
        .is_none());
    handle(&pool, a, &applied).await?;
    assert_eq!(list_knots(&pool, a).await?[0].config_status, "applied");
    let measurements: i64 = sqlx::query_scalar("select count(*) from sensor_measurements")
        .fetch_one(&pool)
        .await?;
    assert_eq!(measurements, 1);
    Ok(())
}

#[arksync_testing::test]
async fn event_ids_are_deduplicated_per_knot_and_hub(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let [a, b] = hubs(&pool).await?;
    let hello_id = EventId::new();
    let measurement_id = EventId::new();
    for (hub, uid) in [(a, "a-1"), (a, "a-2"), (b, "b-1")] {
        let mut greeting = hello(uid);
        greeting.id = hello_id;
        handle(&pool, hub, &greeting).await?;
        let mut sample = measurement(uid);
        sample.id = measurement_id;
        assert!(!handle(&pool, hub, &sample).await?.sensor_events.is_empty());
        assert!(handle(&pool, hub, &sample).await?.sensor_events.is_empty());
    }
    let receipts: i64 = sqlx::query_scalar("select count(*) from knot_message_receipts")
        .fetch_one(&pool)
        .await?;
    let measurements: i64 = sqlx::query_scalar("select count(*) from sensor_measurements")
        .fetch_one(&pool)
        .await?;
    let sources: i64 =
        sqlx::query_scalar("select count(distinct source_knot_id) from sensor_measurements")
            .fetch_one(&pool)
            .await?;
    assert_eq!(receipts, 6);
    assert_eq!(measurements, 3);
    assert_eq!(sources, 3);
    Ok(())
}

#[arksync_testing::test]
async fn concurrent_hello_preserves_one_identity_and_one_owner(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let [a, b] = hubs(&pool).await?;
    let greeting = hello("same-hub-race");
    let (first, second) = tokio::join!(handle(&pool, a, &greeting), handle(&pool, a, &greeting));
    assert_eq!(config(&first?), config(&second?));
    let greeting = hello("cross-hub-race");
    let (first, second) = tokio::join!(handle(&pool, a, &greeting), handle(&pool, b, &greeting));
    let results = [first?, second?];
    let accepted = results
        .iter()
        .filter(|result| {
            matches!(
                result.response.as_ref().unwrap().payload,
                KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Hello { .. }))
            )
        })
        .count();
    assert_eq!(accepted, 1);
    for result in &results {
        if matches!(
            result.response.as_ref().unwrap().payload,
            KnotMessage::Control(KnotControlMessage::Nack(_))
        ) {
            assert_nack(result, KnotNackReason::ConfigurationConflict);
        }
    }
    let count: i64 = sqlx::query_scalar("select count(*) from station_knots")
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 2);
    Ok(())
}

#[arksync_testing::test]
async fn deleted_knots_keep_their_identity_and_are_not_reclaimed_by_hello(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let [a, b] = hubs(&pool).await?;
    handle(&pool, a, &hello("deleted")).await?;
    sqlx::query("update station_knots set deleted_at = now() where hardware_uid = 'deleted'")
        .execute(&pool)
        .await?;
    for hub in [a, b] {
        assert!(list_knots(&pool, hub).await?.is_empty());
        assert_nack(
            &handle(&pool, hub, &hello("deleted")).await?,
            KnotNackReason::ConfigurationConflict,
        );
        assert_nack(
            &handle(&pool, hub, &event("deleted", KnotControlMessage::Heartbeat)).await?,
            KnotNackReason::InvalidPayload,
        );
    }
    let owner: arksync_utils::uuid::Uuid = sqlx::query_scalar(
        "select station_hub_id from station_knots where hardware_uid = 'deleted'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(owner, a.uuid_v4());
    Ok(())
}

#[arksync_testing::test]
async fn concurrent_local_sessions_share_a_hub_without_sharing_knot_identity(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    use arksync_hub::HubSession;
    use arksync_knot::application::{
        local_tokio_message_link, ConnectionState, KeepAliveConfig, RetryPolicy,
        TokioKnotRuntimeConfig, TokioKnotSession,
    };
    use tokio::task::JoinSet;
    use tokio::time::{timeout, Duration};
    use tokio_util::sync::CancellationToken;

    let [a, b] = hubs(&pool).await?;
    timeout(Duration::from_secs(10), async {
        let shutdown = CancellationToken::new();
        let _shutdown_on_drop = shutdown.clone().drop_guard();
        let mut tasks = JoinSet::new();
        let mut statuses = Vec::new();
        for (hub_id, uid) in [(a, "session-a-1"), (a, "session-a-2"), (b, "session-b-1")] {
            let (hub_link, knot_link) = local_tokio_message_link(1);
            let keep_alive_config = KeepAliveConfig::new(50, 2_000).unwrap();
            let hub = HubSession::new(pool.clone(), hub_id, hub_link, keep_alive_config).await?;
            let knot = TokioKnotSession::new(
                TokioKnotRuntimeConfig {
                    hardware_uid: uid.into(),
                    capabilities: KnotCapabilities::default(),
                    retry_policy: RetryPolicy::default(),
                    keep_alive_config,
                },
                knot_link,
            );
            statuses.push((hub_id, uid, knot.handle().status()));
            tasks.spawn(hub.run(shutdown.clone()));
            let knot_shutdown = shutdown.clone();
            tasks.spawn(async move {
                knot.run(knot_shutdown).await.expect("Knot session failed");
            });
        }
        for (hub_id, uid, status) in &mut statuses {
            while status.borrow().connection != ConnectionState::Connected {
                status.changed().await?;
            }
            let config = status
                .borrow()
                .config
                .clone()
                .expect("connected Knot has a config");
            let knots = list_knots(&pool, *hub_id).await?;
            let knot = knots.iter().find(|knot| knot.hardware_uid == *uid).unwrap();
            assert_eq!(knot.id.as_bytes(), &config.knot_id);
        }
        assert_eq!(list_knots(&pool, a).await?.len(), 2);
        assert_eq!(list_knots(&pool, b).await?.len(), 1);
        shutdown.cancel();
        while let Some(result) = tasks.join_next().await {
            result?;
        }
        eyre::Ok(())
    })
    .await??;
    Ok(())
}
