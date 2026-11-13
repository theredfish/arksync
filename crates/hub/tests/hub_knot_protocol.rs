// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_event_router::{EventEnvelope, EventId, Timestamp};
use arksync_hub::infrastructure::store::knot as knot_store;
use arksync_hub::{handle_knot_protocol_event, list_knots, setup_local_station, SensorRegistry};
use arksync_protocol::knot::{
    KnotAck, KnotCapabilities, KnotConfigApplied, KnotConfigRejected, KnotControlMessage,
    KnotEnvelope, KnotHello, KnotMeasurementUnit, KnotMessage, KnotPayload, KnotRejectionReason,
    KnotSensorConnection, KnotSensorDescriptor, KnotSensorKind, KnotSensorMeasurement,
    KnotSensorMeasurementBatch, KnotSensorMessage, KnotSerialPort,
};
use arksync_protocol::ArkSyncActor;

fn timestamp(unix_millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(unix_millis)
}

fn knot_event(
    event_id: EventId,
    hardware_uid: &str,
    message: impl Into<KnotMessage>,
) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        event_id,
        ArkSyncActor::Knot {
            hardware_uid: hardware_uid.to_string(),
        },
        timestamp(1_780_000_000_000),
        message.into(),
    )
}

fn hello(event_id: EventId, hardware_uid: &str) -> KnotEnvelope {
    knot_event(
        event_id,
        hardware_uid,
        KnotControlMessage::Hello(KnotHello {
            hardware_uid: hardware_uid.to_string(),
            capabilities: KnotCapabilities {
                gpio: true,
                uart: true,
                i2c: false,
                atlas_scientific_ezo: true,
            },
            last_applied_config_version: None,
        }),
    )
}

fn measurement(event_id: EventId, hardware_uid: &str) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        event_id,
        ArkSyncActor::Knot {
            hardware_uid: hardware_uid.to_string(),
        },
        timestamp(1_780_000_000_000),
        KnotMessage::Con(KnotPayload::Sensor(KnotSensorMessage::Measurements(
            KnotSensorMeasurementBatch {
                measurements: vec![KnotSensorMeasurement {
                    device_uid: "rkfTHk12L9ZCY39z".to_string(),
                    sensor: KnotSensorDescriptor {
                        hardware_uid: "DK0HFBFB".to_string(),
                        kind: KnotSensorKind::Temperature,
                        connection: KnotSensorConnection::Uart(KnotSerialPort {
                            port_name: "/dev/ttyUSB0".to_string(),
                            serial_number: "DK0HFBFB".to_string(),
                            baud_rate: 9_600,
                        }),
                        firmware: Some(2.15),
                    },
                    value: 28.8,
                    unit: KnotMeasurementUnit::Celsius,
                }],
            },
        ))),
    )
}

#[arksync_testing::test]
async fn duplicate_hello_registers_one_knot_and_returns_correlated_ack(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let hardware_uid = "test-knot-rpi-1";
    let hello_id = EventId::from_bytes([7; 16]);
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let mut sensor_registry = SensorRegistry::load(&pool).await?;
    let event = hello(hello_id, hardware_uid);

    let first = handle_knot_protocol_event(
        &pool,
        &event,
        EventId::from_bytes([8; 16]),
        timestamp(1_780_000_000_100),
        timestamp(1_780_000_000_100),
        &mut sensor_registry,
    )
    .await?
    .response
    .expect("Hello must produce a response");
    // Simulate a new session after the desired configuration has changed.
    let knot = knot_store::station_knot_by_hardware_uid(&pool, hardware_uid)
        .await
        .map_err(|error| eyre::eyre!("{error:?}"))?;
    knot_store::increment_station_knot_config_version(&pool, knot.id)
        .await
        .map_err(|error| eyre::eyre!("{error:?}"))?;
    sensor_registry = SensorRegistry::load(&pool).await?;
    let duplicate = handle_knot_protocol_event(
        &pool,
        &event,
        EventId::from_bytes([9; 16]),
        timestamp(1_780_000_000_200),
        timestamp(1_780_000_000_200),
        &mut sensor_registry,
    )
    .await?
    .response
    .expect("duplicate Hello must reproduce an ACK");

    let knots = list_knots(&pool).await?;
    let matching_knots = knots
        .iter()
        .filter(|knot| knot.hardware_uid == hardware_uid)
        .count();
    let receipt_count: i64 =
        sqlx::query_scalar("select count(*) from knot_message_receipts where event_id = $1")
            .bind(hello_id.uuid_v4())
            .fetch_one(&pool)
            .await?;

    assert_eq!(
        first, duplicate,
        "the exact original envelope must survive config changes and session restart"
    );
    let KnotMessage::Ack(KnotAck::Hello {
        event_id: first_ack_id,
        config: first_config,
    }) = first.payload
    else {
        panic!("expected Hello ACK");
    };
    let KnotMessage::Ack(KnotAck::Hello {
        event_id: duplicate_ack_id,
        config: duplicate_config,
    }) = duplicate.payload
    else {
        panic!("expected duplicate Hello ACK");
    };

    assert_eq!(matching_knots, 1);
    assert_eq!(receipt_count, 1);
    assert_eq!(first_ack_id, hello_id);
    assert_eq!(duplicate_ack_id, hello_id);
    assert_eq!(first_config, duplicate_config);

    Ok(())
}

#[arksync_testing::test]
async fn config_applied_updates_knot_state_once(pool: arksync_testing::PgPool) -> eyre::Result<()> {
    let hardware_uid = "test-knot-rpi-2";
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let mut sensor_registry = SensorRegistry::load(&pool).await?;
    let hello_event = hello(EventId::from_bytes([1; 16]), hardware_uid);
    handle_knot_protocol_event(
        &pool,
        &hello_event,
        EventId::from_bytes([2; 16]),
        timestamp(1_780_000_000_100),
        timestamp(1_780_000_000_100),
        &mut sensor_registry,
    )
    .await?;
    let applied_id = EventId::from_bytes([3; 16]);
    let applied_event = knot_event(
        applied_id,
        hardware_uid,
        KnotControlMessage::ConfigApplied(KnotConfigApplied {
            event_id: EventId::from_bytes([2; 16]),
            config_version: 1,
        }),
    );

    let response = handle_knot_protocol_event(
        &pool,
        &applied_event,
        EventId::from_bytes([4; 16]),
        timestamp(1_780_000_000_200),
        timestamp(1_780_000_000_200),
        &mut sensor_registry,
    )
    .await?
    .response
    .expect("ConfigApplied must be acknowledged");
    handle_knot_protocol_event(
        &pool,
        &applied_event,
        EventId::from_bytes([5; 16]),
        timestamp(1_780_000_000_300),
        timestamp(1_780_000_000_300),
        &mut sensor_registry,
    )
    .await?;
    let knot = list_knots(&pool)
        .await?
        .into_iter()
        .find(|knot| knot.hardware_uid == hardware_uid)
        .expect("Knot must exist");

    assert_eq!(knot.config_status, "applied");
    assert_eq!(knot.applied_config_version, Some(1));
    assert!(matches!(
        response.payload,
        KnotMessage::Ack(KnotAck::Processed { event_id })
            if event_id == applied_id
    ));

    Ok(())
}

#[arksync_testing::test]
async fn config_rejected_records_the_reason_and_is_acknowledged(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let hardware_uid = "test-knot-rpi-3";
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let mut sensor_registry = SensorRegistry::load(&pool).await?;
    handle_knot_protocol_event(
        &pool,
        &hello(EventId::from_bytes([10; 16]), hardware_uid),
        EventId::from_bytes([11; 16]),
        timestamp(1_780_000_000_100),
        timestamp(1_780_000_000_100),
        &mut sensor_registry,
    )
    .await?;
    let rejection_id = EventId::from_bytes([12; 16]);
    let rejection = knot_event(
        rejection_id,
        hardware_uid,
        KnotControlMessage::ConfigRejected(KnotConfigRejected {
            event_id: EventId::from_bytes([11; 16]),
            config_version: 1,
            reason: "GPIO 17 is unavailable".to_string(),
        }),
    );

    let response = handle_knot_protocol_event(
        &pool,
        &rejection,
        EventId::from_bytes([13; 16]),
        timestamp(1_780_000_000_200),
        timestamp(1_780_000_000_200),
        &mut sensor_registry,
    )
    .await?
    .response
    .expect("ConfigRejected must be acknowledged");
    let knot = list_knots(&pool)
        .await?
        .into_iter()
        .find(|knot| knot.hardware_uid == hardware_uid)
        .expect("Knot must exist");

    assert_eq!(knot.config_status, "rejected");
    assert_eq!(knot.config_error.as_deref(), Some("GPIO 17 is unavailable"));
    assert!(matches!(
        response.payload,
        KnotMessage::Ack(KnotAck::Processed { event_id })
            if event_id == rejection_id
    ));

    Ok(())
}

#[arksync_testing::test]
async fn duplicate_measurement_batch_has_one_durable_effect(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let mut sensor_registry = SensorRegistry::load(&pool).await?;
    let measurement_id = EventId::from_bytes([6; 16]);
    let event = measurement(measurement_id, "arksync-local-knot");

    let first = handle_knot_protocol_event(
        &pool,
        &event,
        EventId::from_bytes([7; 16]),
        timestamp(1_780_000_000_100),
        timestamp(1_780_000_000_100),
        &mut sensor_registry,
    )
    .await?;
    let duplicate = handle_knot_protocol_event(
        &pool,
        &event,
        EventId::from_bytes([8; 16]),
        timestamp(1_780_000_000_200),
        timestamp(1_780_000_000_200),
        &mut sensor_registry,
    )
    .await?;
    let measurement_count: i64 =
        sqlx::query_scalar("select count(*) from sensor_measurements where event_id = $1")
            .bind(measurement_id.uuid_v4())
            .fetch_one(&pool)
            .await?;
    let sensor_count: i64 =
        sqlx::query_scalar("select count(*) from sensors where device_uid = 'rkfTHk12L9ZCY39z'")
            .fetch_one(&pool)
            .await?;

    assert_eq!(first.sensor_events.len(), 1);
    assert!(duplicate.sensor_events.is_empty());
    assert_eq!(measurement_count, 1);
    assert_eq!(sensor_count, 1);
    assert!(matches!(
        duplicate.response.map(|response| response.payload),
        Some(KnotMessage::Ack(KnotAck::Processed { event_id })) if event_id == measurement_id
    ));

    Ok(())
}

async fn process(
    pool: &arksync_testing::PgPool,
    registry: &mut SensorRegistry,
    event: &KnotEnvelope,
) -> eyre::Result<KnotEnvelope> {
    Ok(handle_knot_protocol_event(
        pool,
        event,
        EventId::new(),
        timestamp(0),
        timestamp(0),
        registry,
    )
    .await?
    .response
    .expect("expected protocol response"))
}

fn applied(uid: &str, command: &KnotEnvelope, version: u64) -> KnotEnvelope {
    knot_event(
        EventId::new(),
        uid,
        KnotControlMessage::ConfigApplied(KnotConfigApplied {
            event_id: command.id,
            config_version: version,
        }),
    )
}

fn rejected(uid: &str, command: &KnotEnvelope, version: u64) -> KnotEnvelope {
    knot_event(
        EventId::new(),
        uid,
        KnotControlMessage::ConfigRejected(KnotConfigRejected {
            event_id: command.id,
            config_version: version,
            reason: "pin unavailable".into(),
        }),
    )
}

fn assert_processed(response: &KnotEnvelope, request: &KnotEnvelope) {
    assert_eq!(
        response.payload,
        KnotMessage::Ack(KnotAck::Processed {
            event_id: request.id
        })
    );
}

#[arksync_testing::test]
async fn configuration_results_are_correlated_and_never_regress_state(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let mut registry = SensorRegistry::load(&pool).await?;
    let uid = "versioned-knot";
    let v1 = process(&pool, &mut registry, &hello(EventId::new(), uid)).await?;
    let other_v1 = process(&pool, &mut registry, &hello(EventId::new(), uid)).await?;
    let record = knot_store::station_knot_by_hardware_uid(&pool, uid)
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))?;
    knot_store::increment_station_knot_config_version(&pool, record.id)
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))?;
    let v2 = process(&pool, &mut registry, &hello(EventId::new(), uid)).await?;

    // Version 1 can finish while 2 is pending, without claiming 2 is applied.
    let event = applied(uid, &v1, 1);
    assert_processed(&process(&pool, &mut registry, &event).await?, &event);
    let record = knot_store::station_knot_by_hardware_uid(&pool, uid)
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))?;
    assert_eq!(record.applied_config_version, Some(1));
    assert_eq!(record.config_status, "pending");

    let event = applied(uid, &v2, 2);
    assert_processed(&process(&pool, &mut registry, &event).await?, &event);
    for late in [applied(uid, &v1, 1), rejected(uid, &other_v1, 1)] {
        assert_processed(&process(&pool, &mut registry, &late).await?, &late);
    }
    // Unknown origin, wrong version, overflow, another Knot, and contradictory result.
    for invalid in [
        applied(uid, &hello(EventId::new(), uid), 2),
        applied(uid, &v2, 1),
        applied(uid, &v2, u64::MAX),
        applied("another-knot", &v2, 2),
        rejected(uid, &v2, 2),
    ] {
        let reply = process(&pool, &mut registry, &invalid).await?;
        assert_eq!(
            reply.payload,
            KnotMessage::Ack(KnotAck::Rejected {
                event_id: invalid.id,
                reason: KnotRejectionReason::ConfigurationConflict,
            })
        );
        let receipts: i64 =
            sqlx::query_scalar("select count(*) from knot_message_receipts where event_id = $1")
                .bind(invalid.id.uuid_v4())
                .fetch_one(&pool)
                .await?;
        assert_eq!(receipts, 0);
    }
    let record = knot_store::station_knot_by_hardware_uid(&pool, uid)
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))?;
    assert_eq!(record.config_version, 2);
    assert_eq!(record.applied_config_version, Some(2));
    assert_eq!(record.config_status, "applied");
    assert_eq!(record.config_error, None);

    // A rejected new version retains the last successfully applied version.
    knot_store::increment_station_knot_config_version(&pool, record.id)
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))?;
    let v3 = process(&pool, &mut registry, &hello(EventId::new(), uid)).await?;
    let event = rejected(uid, &v3, 3);
    assert_processed(&process(&pool, &mut registry, &event).await?, &event);
    let record = knot_store::station_knot_by_hardware_uid(&pool, uid)
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))?;
    assert_eq!(record.applied_config_version, Some(2));
    assert_eq!(record.config_status, "rejected");
    assert_eq!(record.config_error.as_deref(), Some("pin unavailable"));
    Ok(())
}

#[arksync_testing::test]
async fn startup_preserves_configuration_version_and_application_state(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let local = list_knots(&pool)
        .await?
        .into_iter()
        .find(|k| k.role == "local_hub")
        .unwrap();
    let mut registry = SensorRegistry::load(&pool).await?;
    knot_store::increment_station_knot_config_version(&pool, local.id)
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))?;
    let command = process(
        &pool,
        &mut registry,
        &hello(EventId::new(), &local.hardware_uid),
    )
    .await?;
    process(
        &pool,
        &mut registry,
        &applied(&local.hardware_uid, &command, 2),
    )
    .await?;
    for expected_status in ["applied", "rejected"] {
        if expected_status == "rejected" {
            knot_store::increment_station_knot_config_version(&pool, local.id)
                .await
                .map_err(|e| eyre::eyre!("{e:?}"))?;
            let command = process(
                &pool,
                &mut registry,
                &hello(EventId::new(), &local.hardware_uid),
            )
            .await?;
            process(
                &pool,
                &mut registry,
                &rejected(&local.hardware_uid, &command, 3),
            )
            .await?;
        }
        let before = knot_store::station_knot_by_hardware_uid(&pool, &local.hardware_uid)
            .await
            .map_err(|e| eyre::eyre!("{e:?}"))?;
        let mut txn = pool.begin().await?;
        setup_local_station(&mut txn).await?;
        txn.commit().await?;
        let after = knot_store::station_knot_by_hardware_uid(&pool, &local.hardware_uid)
            .await
            .map_err(|e| eyre::eyre!("{e:?}"))?;
        assert_eq!(after.config_version, before.config_version);
        assert_eq!(after.applied_config_version, Some(2));
        assert_eq!(after.config_status, expected_status);
        assert_eq!(after.config_error, before.config_error);
    }
    Ok(())
}

#[arksync_testing::test]
async fn invalid_batch_gets_terminal_rejection_and_rolls_back_every_effect(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let mut registry = SensorRegistry::load(&pool).await?;
    let mut event = measurement(EventId::new(), "arksync-local-knot");
    let KnotMessage::Con(KnotPayload::Sensor(KnotSensorMessage::Measurements(batch))) =
        &mut event.payload
    else {
        unreachable!()
    };
    let mut invalid = batch.measurements[0].clone();
    invalid.device_uid = "!".into();
    batch.measurements.push(invalid);
    let reply = process(&pool, &mut registry, &event).await?;
    assert_eq!(
        reply.payload,
        KnotMessage::Ack(KnotAck::Rejected {
            event_id: event.id,
            reason: KnotRejectionReason::InvalidPayload,
        })
    );
    for query in [
        "select count(*) from sensors",
        "select count(*) from sensor_measurements",
        "select count(*) from knot_message_receipts",
    ] {
        let count: i64 = sqlx::query_scalar(query).fetch_one(&pool).await?;
        assert_eq!(count, 0);
    }
    // A corrected message can register its sensor; no rolled-back ID leaked into the cache.
    let corrected = measurement(EventId::new(), "arksync-local-knot");
    assert_processed(
        &process(&pool, &mut registry, &corrected).await?,
        &corrected,
    );
    let unavailable = sqlx::postgres::PgPoolOptions::new()
        .connect_with((*pool.connect_options()).clone())
        .await?;
    unavailable.close().await;
    assert!(
        handle_knot_protocol_event(
            &unavailable,
            &corrected,
            EventId::new(),
            timestamp(0),
            timestamp(0),
            &mut registry
        )
        .await
        .is_err(),
        "database failures must remain retryable rather than becoming terminal rejections"
    );
    Ok(())
}

#[arksync_testing::test]
async fn concurrent_hello_duplicates_share_one_committed_reply(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let mut first_registry = SensorRegistry::load(&pool).await?;
    let mut second_registry = SensorRegistry::load(&pool).await?;
    let event = hello(EventId::new(), "concurrent-hello");
    let (first, second) = tokio::join!(
        handle_knot_protocol_event(
            &pool,
            &event,
            EventId::new(),
            timestamp(1),
            timestamp(1),
            &mut first_registry
        ),
        handle_knot_protocol_event(
            &pool,
            &event,
            EventId::new(),
            timestamp(2),
            timestamp(2),
            &mut second_registry
        ),
    );
    assert_eq!(first?.response, second?.response);
    let count: i64 =
        sqlx::query_scalar("select count(*) from knot_message_receipts where event_id = $1")
            .bind(event.id.uuid_v4())
            .fetch_one(&pool)
            .await?;
    assert_eq!(count, 1);
    Ok(())
}

#[arksync_testing::test]
async fn non_ingestion_is_silent_and_deduplicated(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let mut registry = SensorRegistry::load(&pool).await?;
    let mut event = measurement(EventId::new(), "arksync-local-knot");
    event.payload = KnotMessage::Non(event.payload.payload().unwrap().clone());
    let first = handle_knot_protocol_event(
        &pool,
        &event,
        EventId::new(),
        timestamp(1),
        timestamp(1),
        &mut registry,
    )
    .await?;
    let duplicate = handle_knot_protocol_event(
        &pool,
        &event,
        EventId::new(),
        timestamp(2),
        timestamp(2),
        &mut registry,
    )
    .await?;
    assert!(first.accepted);
    assert!(duplicate.accepted);
    assert!(first.response.is_none());
    assert!(duplicate.response.is_none());
    assert_eq!(first.sensor_events.len(), 1);
    assert!(duplicate.sensor_events.is_empty());
    let count: i64 =
        sqlx::query_scalar("select count(*) from sensor_measurements where event_id = $1")
            .bind(event.id.uuid_v4())
            .fetch_one(&pool)
            .await?;
    assert_eq!(count, 1);
    let mut invalid = hello(EventId::new(), "non-hello");
    invalid.payload = KnotMessage::Non(invalid.payload.payload().unwrap().clone());
    let result = handle_knot_protocol_event(
        &pool,
        &invalid,
        EventId::new(),
        timestamp(0),
        timestamp(0),
        &mut registry,
    )
    .await?;
    assert!(!result.accepted);
    assert!(result.response.is_none());
    assert!(!list_knots(&pool)
        .await?
        .iter()
        .any(|knot| knot.hardware_uid == "non-hello"));
    Ok(())
}

#[arksync_testing::test]
async fn conflicting_message_id_cannot_replay_an_unrelated_success(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let mut registry = SensorRegistry::load(&pool).await?;
    let original = measurement(EventId::new(), "arksync-local-knot");
    let first_reply = process(&pool, &mut registry, &original).await?;
    let mut changed = original.clone();
    let KnotMessage::Con(KnotPayload::Sensor(KnotSensorMessage::Measurements(batch))) =
        &mut changed.payload
    else {
        unreachable!()
    };
    batch.measurements[0].value = 99.0;
    for mut conflict in [changed, original.clone()] {
        if conflict == original {
            conflict.source = ArkSyncActor::Knot {
                hardware_uid: "other-knot".into(),
            };
        }
        assert_eq!(
            process(&pool, &mut registry, &conflict).await?.payload,
            KnotMessage::Ack(KnotAck::Rejected {
                event_id: original.id,
                reason: KnotRejectionReason::ConfigurationConflict
            })
        );
    }
    for invalid_number in [f64::NAN, f64::INFINITY] {
        let mut invalid = original.clone();
        let KnotMessage::Con(KnotPayload::Sensor(KnotSensorMessage::Measurements(batch))) =
            &mut invalid.payload
        else {
            unreachable!()
        };
        batch.measurements[0].sensor.firmware = Some(invalid_number);
        assert_eq!(
            process(&pool, &mut registry, &invalid).await?.payload,
            KnotMessage::Ack(KnotAck::Rejected {
                event_id: original.id,
                reason: KnotRejectionReason::InvalidPayload
            })
        );
    }
    assert_eq!(process(&pool, &mut registry, &original).await?, first_reply);
    let value: f64 =
        sqlx::query_scalar("select value from sensor_measurements where event_id = $1")
            .bind(original.id.uuid_v4())
            .fetch_one(&pool)
            .await?;
    assert_eq!(value, 28.8);
    Ok(())
}

#[arksync_testing::test]
async fn configure_ack_commits_its_result_without_producing_another_ack(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let mut registry = SensorRegistry::load(&pool).await?;
    let uid = "piggyback-config";
    let hello_ack = process(&pool, &mut registry, &hello(EventId::new(), uid)).await?;
    let KnotMessage::Ack(KnotAck::Hello { config, .. }) = hello_ack.payload else {
        unreachable!()
    };
    let command_id = EventId::new();
    assert!(knot_store::insert_knot_config_delivery(&pool, command_id, &config).await?);
    let mut event = knot_event(EventId::new(), uid, KnotControlMessage::Heartbeat);
    event.payload = KnotMessage::Ack(KnotAck::ConfigApplied {
        event_id: command_id,
        config_version: config.version,
    });
    for _ in 0..2 {
        let result = handle_knot_protocol_event(
            &pool,
            &event,
            EventId::new(),
            timestamp(0),
            timestamp(0),
            &mut registry,
        )
        .await?;
        assert!(result.accepted);
        assert_eq!(
            result.acknowledged.as_ref(),
            match &event.payload {
                KnotMessage::Ack(ack) => Some(ack),
                _ => None,
            }
        );
        assert!(
            result.response.is_none(),
            "an ACK must never be acknowledged"
        );
    }
    let record = knot_store::station_knot_by_hardware_uid(&pool, uid)
        .await
        .map_err(|error| eyre::eyre!("{error:?}"))?;
    assert_eq!(record.applied_config_version, Some(1));
    let mut invalid = event.clone();
    invalid.id = EventId::new();
    invalid.payload = KnotMessage::Ack(KnotAck::ConfigApplied {
        event_id: command_id,
        config_version: 999,
    });
    let result = handle_knot_protocol_event(
        &pool,
        &invalid,
        EventId::new(),
        timestamp(0),
        timestamp(0),
        &mut registry,
    )
    .await?;
    assert!(!result.accepted);
    assert!(result.acknowledged.is_none());
    assert!(result.response.is_none());
    Ok(())
}
