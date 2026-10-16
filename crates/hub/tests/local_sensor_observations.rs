// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventHandlerError, EventId, Timestamp};
use arksync_hub::{setup_local_station, HubId, HubSession};
use arksync_knot::application::{
    local_tokio_message_link, KeepAliveConfig, MessageLink, RetryPolicy, TokioKnotRuntimeConfig,
    TokioKnotSession,
};
use arksync_protocol::knot::*;
use arksync_protocol::ArkSyncActor;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

fn hub_id() -> HubId {
    arksync_hub::config::CONFIG.local_hub_id.into()
}
fn time(value: i64) -> Timestamp {
    Timestamp::from_unix_millis(value)
}
fn inventory(session_id: EventId, revision: u64) -> KnotObservationMessage {
    KnotObservationMessage::Inventory(KnotSensorInventory {
        session_id,
        revision,
        sensors: vec![KnotObservedSensor {
            reference: KnotSensorRef(42),
            device_uid: None,
            sensor: KnotSensorDescriptor {
                hardware_uid: String::new(),
                kind: KnotSensorKind::Temperature,
                connection: KnotSensorConnection::I2c { address: 0x66 },
                firmware: None,
            },
            state: KnotSensorObservationState::Present,
        }],
    })
}
fn measurement(session_id: EventId, revision: u64, sequence: u64) -> KnotObservationMessage {
    KnotObservationMessage::Measurement(KnotLiveMeasurement {
        session_id,
        inventory_revision: revision,
        sensor: KnotSensorRef(42),
        sequence,
        acquired_at: time(123),
        value: Some(28.8),
        unit: KnotMeasurementUnit::Celsius,
        quality: KnotMeasurementQuality::Valid,
    })
}
fn envelope(hardware_uid: &str, payload: KnotMessage) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        EventId::new(),
        ArkSyncActor::Knot {
            hardware_uid: hardware_uid.into(),
        },
        time(456),
        payload,
    )
}
fn observe(hub: &mut HubSession) -> mpsc::Receiver<KnotEnvelope> {
    let (tx, rx) = mpsc::channel(16);
    hub.subscribe(move |message: &KnotEnvelope| {
        if matches!(message.payload, KnotMessage::Observation(_)) {
            tx.try_send(message.clone())
                .map_err(|_| EventHandlerError::Rejected)?;
        }
        Ok(())
    });
    rx
}
async fn setup(pool: &sqlx::PgPool) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    Ok(())
}

#[arksync_testing::test]
async fn real_sessions_deliver_unregistered_sensor_observations_without_durable_effects(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    setup(&pool).await?;
    timeout(Duration::from_secs(10), async {
        let mut previous_session = None;
        let mut previous_knot = None;
        for round in 0..2 {
            let (hub_link, knot_link) = local_tokio_message_link(1);
            let keep_alive_config = KeepAliveConfig::new(50, 2_000).unwrap();
            let mut hub =
                HubSession::new(pool.clone(), hub_id(), hub_link, keep_alive_config).await?;
            let mut received = observe(&mut hub);
            let knot = TokioKnotSession::new(
                TokioKnotRuntimeConfig {
                    hardware_uid: "observation-knot".into(),
                    capabilities: KnotCapabilities::default(),
                    retry_policy: RetryPolicy {
                        capacity: 1,
                        ..RetryPolicy::default()
                    },
                    keep_alive_config,
                },
                knot_link,
            );
            let handle = knot.handle();
            let mut status = handle.status();
            let session_id = handle.observation_session_id();
            assert_ne!(Some(session_id), previous_session);
            previous_session = Some(session_id);
            handle
                .try_observe_at(&inventory(session_id, 1), time(100))
                .unwrap();
            for sequence in 1..=50 {
                handle
                    .try_observe_at(&measurement(session_id, 1, sequence), time(200))
                    .unwrap();
            }
            let shutdown = CancellationToken::new();
            let _shutdown_on_drop = shutdown.clone().drop_guard();
            let mut tasks = JoinSet::new();
            tasks.spawn(hub.run(shutdown.clone()));
            let knot_shutdown = shutdown.clone();
            tasks.spawn(async move {
                knot.run(knot_shutdown).await.unwrap();
            });
            let snapshot = received.recv().await.unwrap();
            assert_eq!(
                snapshot.payload,
                KnotMessage::Observation(inventory(session_id, 1))
            );
            let sample = received.recv().await.unwrap();
            assert_eq!(
                sample.payload,
                KnotMessage::Observation(measurement(session_id, 1, 50))
            );
            assert_eq!(sample.occurred_at, time(200));
            status
                .wait_for(|status| status.config.is_some() && status.pending_messages == 0)
                .await?;
            let knot_id = status.borrow().config.as_ref().unwrap().knot_id;
            if let Some(previous) = previous_knot {
                assert_eq!(knot_id, previous);
            }
            previous_knot = Some(knot_id);
            let receipts: i64 = sqlx::query_scalar("select count(*) from knot_message_receipts")
                .fetch_one(&pool)
                .await?;
            assert_eq!(
                receipts,
                (round + 1) * 2,
                "only Hello and ConfigApplied produce receipts"
            );
            shutdown.cancel();
            while let Some(result) = tasks.join_next().await {
                result?;
            }
        }
        for table in ["sensors", "sensor_measurements", "actuators"] {
            let count: i64 = sqlx::query_scalar(&format!("select count(*) from {table}"))
                .fetch_one(&pool)
                .await?;
            assert_eq!(count, 0, "observation must not populate {table}");
        }
        eyre::Ok(())
    })
    .await??;
    Ok(())
}

#[arksync_testing::test]
async fn hub_filters_foreign_stale_and_unknown_observations_before_publishing(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    setup(&pool).await?;
    timeout(Duration::from_secs(10), async {
        let (hub_link, mut peer) = local_tokio_message_link(1);
        let mut hub =
            HubSession::new(pool.clone(), hub_id(), hub_link, KeepAliveConfig::default()).await?;
        let mut received = observe(&mut hub);
        let shutdown = CancellationToken::new();
        let _shutdown_on_drop = shutdown.clone().drop_guard();
        let mut tasks = JoinSet::new();
        tasks.spawn(hub.run(shutdown.clone()));
        let uid = "raw-observation-knot";
        let hello = envelope(
            uid,
            KnotMessage::Control(KnotControlMessage::Hello(KnotHello {
                hardware_uid: uid.into(),
                capabilities: KnotCapabilities::default(),
                last_applied_config_version: None,
            })),
        );
        peer.send(envelope(
            uid,
            KnotMessage::Observation(inventory(hello.id, 1)),
        ))
        .await
        .unwrap();
        peer.send(hello.clone()).await.unwrap();
        let ack = peer.receive().await.unwrap().unwrap();
        assert!(matches!(
            ack.payload,
            KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Hello { .. }))
        ));
        assert!(
            received.try_recv().is_err(),
            "observation before Hello must not be published"
        );
        for message in [inventory(hello.id, 1), measurement(hello.id, 1, 5)] {
            peer.send(envelope(uid, KnotMessage::Observation(message.clone())))
                .await
                .unwrap();
            assert_eq!(
                received.recv().await.unwrap().payload,
                KnotMessage::Observation(message)
            );
        }
        let KnotObservationMessage::Inventory(mut conflict) = inventory(hello.id, 1) else {
            unreachable!()
        };
        conflict.sensors.clear();
        for message in [
            measurement(hello.id, 1, 4),
            measurement(hello.id, 1, 5),
            inventory(EventId::new(), 99),
            measurement(EventId::new(), 1, 100),
            measurement(hello.id, 2, 6),
            KnotObservationMessage::Inventory(conflict),
        ] {
            peer.send(envelope(uid, KnotMessage::Observation(message)))
                .await
                .unwrap();
        }
        peer.send(envelope(
            "foreign-knot",
            KnotMessage::Observation(inventory(hello.id, 99)),
        ))
        .await
        .unwrap();
        // A delayed Hello must not rebind an already established endpoint.
        let mut old_hello = hello.clone();
        old_hello.id = EventId::new();
        peer.send(old_hello).await.unwrap();
        let heartbeat = envelope(uid, KnotMessage::Control(KnotControlMessage::Heartbeat));
        peer.send(heartbeat.clone()).await.unwrap();
        let barrier = peer.receive().await.unwrap().unwrap();
        assert_eq!(
            barrier.payload,
            KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Heartbeat {
                event_id: heartbeat.id
            }))
        );
        assert!(
            received.try_recv().is_err(),
            "rejected observations must not reach subscribers"
        );
        for message in [inventory(hello.id, 2), measurement(hello.id, 2, 1)] {
            peer.send(envelope(uid, KnotMessage::Observation(message.clone())))
                .await
                .unwrap();
            assert_eq!(
                received.recv().await.unwrap().payload,
                KnotMessage::Observation(message)
            );
        }
        let receipts: i64 = sqlx::query_scalar("select count(*) from knot_message_receipts")
            .fetch_one(&pool)
            .await?;
        assert_eq!(receipts, 1);
        shutdown.cancel();
        while let Some(result) = tasks.join_next().await {
            result?;
        }
        eyre::Ok(())
    })
    .await??;
    Ok(())
}
