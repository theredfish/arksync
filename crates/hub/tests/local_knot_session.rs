// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventHandlerError, EventId, Timestamp};
use arksync_hub::infrastructure::store::knot as knot_store;
use arksync_hub::{
    knot_protocol_config_for_hardware_uid, list_knots, setup_local_station, HubSession,
};
use arksync_knot::application::{
    local_tokio_message_link, ConnectionState, KeepAliveConfig, MessageLink, RetryPolicy,
    TokioKnotRuntimeConfig, TokioKnotSession,
};
use arksync_protocol::knot::{
    KnotAck, KnotCapabilities, KnotControlMessage, KnotEnvelope, KnotHello, KnotMessage,
};
use arksync_protocol::ArkSyncActor;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

fn local_hub_id() -> arksync_hub::HubId {
    arksync_hub::config::CONFIG.local_hub_id.into()
}

fn envelope(source: ArkSyncActor, message: KnotControlMessage) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        EventId::new(),
        source,
        Timestamp::from_unix_millis(1_780_000_000_000),
        KnotMessage::Control(message),
    )
}

async fn next_event(
    events: &mut mpsc::Receiver<KnotEnvelope>,
    matches: impl Fn(&KnotEnvelope) -> bool,
) -> KnotEnvelope {
    loop {
        let event = events.recv().await.expect("session observer closed");
        if matches(&event) {
            return event;
        }
    }
}

#[arksync_testing::test]
async fn local_sessions_register_exchange_config_and_rejoin_without_hardware(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;

    timeout(Duration::from_secs(15), async {
        let hardware_uid = "integration-local-knot";
        let keep_alive_config = KeepAliveConfig::new(50, 2_000).unwrap();
        let mut first_knot_id = None;

        for round in 0..2 {
            // Use the production sessions and handlers, not direct handler calls.
            let (hub_link, knot_link) = local_tokio_message_link(1);
            let mut hub = HubSession::new(pool.clone(), local_hub_id(), hub_link, keep_alive_config).await?;
            let hub_handle = hub.handle();
            let hub_status = hub_handle.status();
            let (hub_events_tx, mut hub_events) = mpsc::channel(64);
            hub.subscribe(move |event: &KnotEnvelope| {
                hub_events_tx.try_send(event.clone()).map_err(|_| EventHandlerError::Rejected)
            });
            let mut knot = TokioKnotSession::new(TokioKnotRuntimeConfig {
                hardware_uid: hardware_uid.into(),
                capabilities: KnotCapabilities::default(),
                retry_policy: RetryPolicy { capacity: 1, ..RetryPolicy::default() },
                keep_alive_config,
            }, knot_link);
            let knot_handle = knot.handle();
            let knot_status = knot_handle.status();
            let (knot_events_tx, mut knot_events) = mpsc::channel(64);
            knot.subscribe(move |event: &KnotEnvelope| {
                knot_events_tx.try_send(event.clone()).map_err(|_| EventHandlerError::Rejected)
            });
            let shutdown = CancellationToken::new();
            let _shutdown_on_drop = shutdown.clone().drop_guard();
            let mut tasks = JoinSet::new();
            tasks.spawn(hub.run(shutdown.clone()));
            let knot_shutdown = shutdown.clone();
            tasks.spawn(async move { knot.run(knot_shutdown).await.expect("Knot session failed"); });

            let hello = next_event(&mut hub_events, |event| matches!(event.payload,
                KnotMessage::Control(KnotControlMessage::Hello(_)))).await;
            let hello_ack = next_event(&mut knot_events, |event| matches!(event.payload,
                KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Hello { event_id, .. })) if event_id == hello.id)).await;
            let KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Hello { config, .. })) = &hello_ack.payload else { unreachable!() };
            assert_eq!(config.version, round + 1);
            assert!(config.sensor_bindings.is_empty());
            assert!(config.actuator_configs.is_empty());
            if let Some(id) = first_knot_id { assert_eq!(config.knot_id, id); }
            first_knot_id = Some(config.knot_id);

            let applied = next_event(&mut hub_events, |event| matches!(event.payload,
                KnotMessage::Control(KnotControlMessage::ConfigApplied(applied)) if applied.event_id == hello_ack.id)).await;
            next_event(&mut knot_events, |event| matches!(event.payload,
                KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Processed { event_id })) if event_id == applied.id)).await;

            let knots = list_knots(&pool, local_hub_id()).await?;
            let matching: Vec<_> = knots.iter().filter(|knot| knot.hardware_uid == hardware_uid).collect();
            assert_eq!(matching.len(), 1);
            let registered = matching[0];
            assert_eq!(registered.id.as_bytes(), &config.knot_id);
            assert_eq!(registered.config_status, "applied");
            assert_eq!(registered.applied_config_version, Some(config.version as i64));

            let receipts_before: i64 = sqlx::query_scalar("select count(*) from knot_message_receipts")
                .fetch_one(&pool).await?;
            for _ in 0..3 {
                let heartbeat = next_event(&mut hub_events, |event| event.payload == KnotMessage::Control(KnotControlMessage::Heartbeat)).await;
                next_event(&mut knot_events, |event| matches!(event.payload,
                    KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Heartbeat { event_id })) if event_id == heartbeat.id)).await;
            }
            let receipts_after: i64 = sqlx::query_scalar("select count(*) from knot_message_receipts")
                .fetch_one(&pool).await?;
            assert_eq!(receipts_before, receipts_after, "presence must not create durable receipts");
            assert_eq!(hub_status.borrow().connection, ConnectionState::Connected);
            assert_eq!(knot_status.borrow().connection, ConnectionState::Connected);

            if round == 0 {
                let mut txn = pool.begin().await?;
                knot_store::increment_knot_config_version(&mut *txn, local_hub_id(), registered.id)
                    .await.map_err(|error| eyre::eyre!("config version: {error:?}"))?;
                let config = knot_protocol_config_for_hardware_uid(&mut txn, local_hub_id(), hardware_uid).await?;
                txn.commit().await?;
                let configure = envelope(hello_ack.source.clone(), KnotControlMessage::Configure(config));
                let configure_id = configure.id;
                hub_handle.send(configure).await?;
                let applied = next_event(&mut hub_events, |event| matches!(event.payload,
                    KnotMessage::Control(KnotControlMessage::ConfigApplied(applied))
                        if applied.event_id == configure_id && applied.config_version == 2)).await;
                next_event(&mut knot_events, |event| matches!(event.payload,
                    KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Processed { event_id })) if event_id == applied.id)).await;
            }

            shutdown.cancel();
            while let Some(result) = tasks.join_next().await { result?; }
            assert_eq!(hub_status.borrow().connection, ConnectionState::Closed);
            assert_eq!(knot_status.borrow().connection, ConnectionState::Closed);
            assert_eq!(knot_status.borrow().config.as_ref().unwrap().version, 2);
        }
        let sensors: i64 = sqlx::query_scalar("select count(*) from sensors").fetch_one(&pool).await?;
        let measurements: i64 = sqlx::query_scalar("select count(*) from sensor_measurements").fetch_one(&pool).await?;
        assert_eq!(sensors, 0);
        assert_eq!(measurements, 0);
        eyre::Ok(())
    }).await??;
    Ok(())
}

#[arksync_testing::test]
async fn hub_session_handles_duplicate_hello_presence_expiry_and_recovery(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;

    timeout(Duration::from_secs(10), async {
        let hardware_uid = "idle-local-knot";
        let source = ArkSyncActor::Knot {
            hardware_uid: hardware_uid.into(),
        };
        let (hub_link, mut peer) = local_tokio_message_link(1);
        let hub = HubSession::new(
            pool.clone(),
            local_hub_id(),
            hub_link,
            KeepAliveConfig::new(50, 300).unwrap(),
        )
        .await?;
        let mut status = hub.handle().status();
        let shutdown = CancellationToken::new();
        let _shutdown_on_drop = shutdown.clone().drop_guard();
        let mut tasks = JoinSet::new();
        tasks.spawn(hub.run(shutdown.clone()));
        let hello = envelope(
            source.clone(),
            KnotControlMessage::Hello(KnotHello {
                hardware_uid: hardware_uid.into(),
                capabilities: KnotCapabilities::default(),
                last_applied_config_version: None,
            }),
        );
        peer.send(hello.clone()).await.unwrap();
        let first = peer.receive().await.unwrap().unwrap();
        peer.send(hello.clone()).await.unwrap();
        let duplicate = peer.receive().await.unwrap().unwrap();
        assert_eq!(first.payload, duplicate.payload);
        let receipts: i64 =
            sqlx::query_scalar("select count(*) from knot_message_receipts where event_id = $1")
                .bind(hello.id.uuid_v4())
                .fetch_one(&pool)
                .await?;
        assert_eq!(receipts, 1);

        let heartbeat = envelope(source.clone(), KnotControlMessage::Heartbeat);
        peer.send(heartbeat.clone()).await.unwrap();
        let reply = peer.receive().await.unwrap().unwrap();
        assert_eq!(
            reply.payload,
            KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Heartbeat {
                event_id: heartbeat.id
            }))
        );
        status
            .wait_for(|state| state.connection == ConnectionState::Unavailable)
            .await?;
        let id = status.borrow().knot_id;
        assert!(id.is_some(), "expiry must not discard registration");

        // A different peer cannot take over an already bound endpoint.
        let last_received_at = status.borrow().last_received_at;
        peer.send(envelope(
            ArkSyncActor::Knot {
                hardware_uid: "another-peer".into(),
            },
            KnotControlMessage::Heartbeat,
        ))
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(status.borrow().connection, ConnectionState::Unavailable);
        assert_eq!(status.borrow().last_received_at, last_received_at);

        let heartbeat = envelope(source, KnotControlMessage::Heartbeat);
        peer.send(heartbeat.clone()).await.unwrap();
        let reply = peer.receive().await.unwrap().unwrap();
        assert_eq!(
            reply.payload,
            KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Heartbeat {
                event_id: heartbeat.id
            }))
        );
        status
            .wait_for(|state| state.connection == ConnectionState::Connected)
            .await?;
        assert_eq!(status.borrow().knot_id, id);
        assert_eq!(
            list_knots(&pool, local_hub_id())
                .await?
                .iter()
                .filter(|knot| knot.hardware_uid == hardware_uid)
                .count(),
            1
        );

        drop(peer);
        tasks.join_next().await.unwrap()?;
        assert_eq!(status.borrow().connection, ConnectionState::Closed);
        eyre::Ok(())
    })
    .await??;
    Ok(())
}

#[arksync_testing::test]
async fn configure_is_retried_after_a_full_knot_outbox_without_overwriting_messages(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    timeout(Duration::from_secs(10), async {
        let uid = "saturated-knot";
        // Interpose two local endpoints to control delivery order between production sessions.
        let (hub_link, mut hub_peer) = local_tokio_message_link(2);
        let (mut knot_peer, knot_link) = local_tokio_message_link(2);
        let hub = HubSession::new(pool.clone(), local_hub_id(), hub_link, KeepAliveConfig::default()).await?;
        let hub_handle = hub.handle();
        let knot = TokioKnotSession::new(TokioKnotRuntimeConfig {
            hardware_uid: uid.into(),
            capabilities: KnotCapabilities::default(),
            retry_policy: RetryPolicy { capacity: 1, ..RetryPolicy::default() },
            keep_alive_config: KeepAliveConfig::default(),
        }, knot_link);
        let mut status = knot.handle().status();
        let shutdown = CancellationToken::new();
        let _guard = shutdown.clone().drop_guard();
        let mut tasks = JoinSet::new();
        tasks.spawn(hub.run(shutdown.clone()));
        let knot_shutdown = shutdown.clone();
        tasks.spawn(async move { knot.run(knot_shutdown).await.unwrap(); });

        let hello = knot_peer.receive().await.unwrap().unwrap();
        hub_peer.send(hello).await.unwrap();
        let hello_ack = hub_peer.receive().await.unwrap().unwrap();
        knot_peer.send(hello_ack.clone()).await.unwrap();
        let first_confirmation = knot_peer.receive().await.unwrap().unwrap();
        let record = knot_store::knot_by_hardware_uid(&pool, local_hub_id(), uid).await
            .map_err(|e| eyre::eyre!("{e:?}"))?;
        let mut txn = pool.begin().await?;
        knot_store::increment_knot_config_version(&mut *txn, local_hub_id(), record.id).await
            .map_err(|e| eyre::eyre!("{e:?}"))?;
        let config = knot_protocol_config_for_hardware_uid(&mut txn, local_hub_id(), uid).await?;
        txn.commit().await?;
        let command = envelope(hello_ack.source.clone(), KnotControlMessage::Configure(config));
        hub_handle.send(command.clone()).await?;
        let mut conflicting_command = command.clone();
        let KnotMessage::Control(KnotControlMessage::Configure(config)) = &mut conflicting_command.payload else { unreachable!() };
        config.sensor_bindings.push(arksync_protocol::knot::KnotSensorBinding { sensor_id: [3; 16], device_uid: "sensor".into() });
        assert!(hub_handle.send(conflicting_command).await.is_err(), "an existing event ID cannot overwrite a queued command");

        let sent = hub_peer.receive().await.unwrap().unwrap();
        assert_eq!(sent, command);
        let invalid_confirmation = envelope(
            ArkSyncActor::Knot { hardware_uid: uid.into() },
            KnotControlMessage::ConfigApplied(arksync_protocol::knot::KnotConfigApplied {
                event_id: command.id, config_version: 1,
            }),
        );
        hub_peer.send(invalid_confirmation.clone()).await.unwrap();
        let nack = hub_peer.receive().await.unwrap().unwrap();
        assert_eq!(nack.payload, KnotMessage::Control(KnotControlMessage::Nack(arksync_protocol::knot::KnotNack {
            event_id: invalid_confirmation.id,
            reason: arksync_protocol::knot::KnotNackReason::ConfigurationConflict,
        })));
        // Configure arrives while the only Knot slot still holds ConfigApplied(v1).
        knot_peer.send(sent).await.unwrap();
        hub_peer.send(first_confirmation.clone()).await.unwrap();
        let ack = hub_peer.receive().await.unwrap().unwrap();
        assert_eq!(ack.payload, KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Processed { event_id: first_confirmation.id })));
        knot_peer.send(ack).await.unwrap();
        status.wait_for(|state| state.pending_messages == 0).await?;
        assert_eq!(status.borrow().config.as_ref().unwrap().version, 1);

        let retry = hub_peer.receive().await.unwrap().unwrap();
        assert_eq!(retry, command, "retry must preserve content, timestamp and EventId");
        knot_peer.send(retry).await.unwrap();
        let confirmation = knot_peer.receive().await.unwrap().unwrap();
        assert!(matches!(confirmation.payload, KnotMessage::Control(KnotControlMessage::ConfigApplied(result))
            if result.event_id == command.id && result.config_version == 2));
        hub_peer.send(confirmation.clone()).await.unwrap();
        let ack = hub_peer.receive().await.unwrap().unwrap();
        assert_eq!(ack.payload, KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Processed { event_id: confirmation.id })));
        knot_peer.send(ack).await.unwrap();
        status.wait_for(|state| state.pending_messages == 0 && state.config.as_ref().is_some_and(|c| c.version == 2)).await?;
        let record = knot_store::knot_by_hardware_uid(&pool, local_hub_id(), uid).await
            .map_err(|e| eyre::eyre!("{e:?}"))?;
        assert_eq!(record.applied_config_version, Some(2));
        assert_eq!(record.config_status, "applied");
        assert!(timeout(Duration::from_millis(2_200), hub_peer.receive()).await.is_err(),
            "a committed confirmation must stop Hub retries");
        shutdown.cancel();
        while let Some(result) = tasks.join_next().await { result?; }
        eyre::Ok(())
    }).await??;
    Ok(())
}

#[arksync_testing::test]
async fn database_waits_do_not_block_presence_expiry_or_shutdown(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    timeout(Duration::from_secs(5), async {
        let single = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_with((*pool.connect_options()).clone())
            .await?;
        let (hub_link, mut peer) = local_tokio_message_link(2);
        let hub = HubSession::new(
            single.clone(),
            local_hub_id(),
            hub_link,
            KeepAliveConfig::new(50, 150).unwrap(),
        )
        .await?;
        let mut status = hub.handle().status();
        let shutdown = CancellationToken::new();
        let _guard = shutdown.clone().drop_guard();
        let mut tasks = JoinSet::new();
        tasks.spawn(hub.run(shutdown.clone()));
        let uid = "database-wait-knot";
        let source = ArkSyncActor::Knot {
            hardware_uid: uid.into(),
        };
        peer.send(envelope(
            source.clone(),
            KnotControlMessage::Hello(KnotHello {
                hardware_uid: uid.into(),
                capabilities: KnotCapabilities::default(),
                last_applied_config_version: None,
            }),
        ))
        .await
        .unwrap();
        peer.receive().await.unwrap().unwrap();
        assert_eq!(status.borrow().connection, ConnectionState::Connected);
        let held = single.acquire().await?;
        peer.send(envelope(source, KnotControlMessage::Heartbeat))
            .await
            .unwrap();
        timeout(
            Duration::from_secs(1),
            status.wait_for(|state| state.connection == ConnectionState::Unavailable),
        )
        .await??;
        shutdown.cancel();
        timeout(Duration::from_millis(200), tasks.join_next())
            .await?
            .unwrap()?;
        assert_eq!(status.borrow().connection, ConnectionState::Closed);
        drop(held);
        single.close().await;
        eyre::Ok(())
    })
    .await??;
    Ok(())
}

#[arksync_testing::test]
async fn cancellation_during_sql_rolls_back_configuration_confirmation(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    timeout(Duration::from_secs(5), async {
        let (link, mut peer) = local_tokio_message_link(2);
        let hub = HubSession::new(
            pool.clone(),
            local_hub_id(),
            link,
            KeepAliveConfig::new(50, 200).unwrap(),
        )
        .await?;
        let mut status = hub.handle().status();
        let shutdown = CancellationToken::new();
        let _guard = shutdown.clone().drop_guard();
        let mut tasks = JoinSet::new();
        tasks.spawn(hub.run(shutdown.clone()));
        let uid = "database-lock-knot";
        let source = ArkSyncActor::Knot {
            hardware_uid: uid.into(),
        };
        peer.send(envelope(
            source.clone(),
            KnotControlMessage::Hello(KnotHello {
                hardware_uid: uid.into(),
                capabilities: KnotCapabilities::default(),
                last_applied_config_version: None,
            }),
        ))
        .await
        .unwrap();
        let hello_ack = peer.receive().await.unwrap().unwrap();
        let mut lock = pool.begin().await?;
        sqlx::query("select id from knots where hardware_uid = $1 for update")
            .bind(uid)
            .fetch_one(&mut *lock)
            .await?;
        let confirmation = envelope(
            source,
            KnotControlMessage::ConfigApplied(arksync_protocol::knot::KnotConfigApplied {
                event_id: hello_ack.id,
                config_version: 1,
            }),
        );
        peer.send(confirmation.clone()).await.unwrap();
        status
            .wait_for(|state| state.connection == ConnectionState::Unavailable)
            .await?;
        shutdown.cancel();
        timeout(Duration::from_millis(200), tasks.join_next())
            .await?
            .unwrap()?;
        lock.rollback().await?;
        // Wait for the cancelled transaction to release its delivery row.
        let mut check = pool.begin().await?;
        let outcome: Option<String> = sqlx::query_scalar(
            "select outcome from knot_config_deliveries where event_id = $1 for update",
        )
        .bind(hello_ack.id.uuid_v4())
        .fetch_one(&mut *check)
        .await?;
        assert_eq!(outcome, None);
        let receipts: i64 =
            sqlx::query_scalar("select count(*) from knot_message_receipts where event_id = $1")
                .bind(confirmation.id.uuid_v4())
                .fetch_one(&mut *check)
                .await?;
        assert_eq!(receipts, 0);
        check.commit().await?;
        assert!(
            peer.receive().await.unwrap().is_none(),
            "no ACK can escape a cancelled transaction"
        );
        eyre::Ok(())
    })
    .await??;
    Ok(())
}

#[arksync_testing::test]
async fn database_deadline_releases_the_session_without_acknowledging_unfinished_work(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    timeout(Duration::from_secs(9), async {
        let single = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_with((*pool.connect_options()).clone())
            .await?;
        let (link, mut peer) = local_tokio_message_link(2);
        let hub = HubSession::new(
            single.clone(),
            local_hub_id(),
            link,
            KeepAliveConfig::new(50, 200).unwrap(),
        )
        .await?;
        let shutdown = CancellationToken::new();
        let _guard = shutdown.clone().drop_guard();
        let mut tasks = JoinSet::new();
        tasks.spawn(hub.run(shutdown.clone()));
        let uid = "database-timeout-knot";
        let source = ArkSyncActor::Knot {
            hardware_uid: uid.into(),
        };
        peer.send(envelope(
            source.clone(),
            KnotControlMessage::Hello(KnotHello {
                hardware_uid: uid.into(),
                capabilities: KnotCapabilities::default(),
                last_applied_config_version: None,
            }),
        ))
        .await
        .unwrap();
        peer.receive().await.unwrap().unwrap();
        let held = single.acquire().await?;
        let first = envelope(source.clone(), KnotControlMessage::Heartbeat);
        peer.send(first.clone()).await.unwrap();
        // Longer than the handler's deadline, shorter than the pool acquisition timeout.
        tokio::time::sleep(Duration::from_millis(5_500)).await;
        let retry = envelope(source, KnotControlMessage::Heartbeat);
        peer.send(retry.clone()).await.unwrap();
        drop(held);
        let response = peer.receive().await.unwrap().unwrap();
        assert_eq!(
            response.payload,
            KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Heartbeat {
                event_id: retry.id,
            })),
            "the first operation expired and must never be acknowledged"
        );
        shutdown.cancel();
        tasks.join_next().await.unwrap()?;
        single.close().await;
        eyre::Ok(())
    })
    .await??;
    Ok(())
}
