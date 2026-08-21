// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_hub::{handle_knot_protocol_event, list_knots, setup_local_station, SensorRegistry};
use arksync_knot_protocol::{
    KnotAck, KnotCapabilities, KnotConfigApplied, KnotControlMessage, KnotEnvelope, KnotHello,
    KnotMeasurementUnit, KnotMessage, KnotMessageSource, KnotSensorConnection,
    KnotSensorDescriptor, KnotSensorKind, KnotSensorMeasurement, KnotSensorMeasurementBatch,
    KnotSensorMessage, KnotSerialPort,
};

fn timestamp(unix_millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(unix_millis)
}

fn knot_event(event_id: EventId, hardware_uid: &str, message: KnotControlMessage) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        event_id,
        KnotMessageSource::Knot {
            hardware_uid: hardware_uid.to_string(),
        },
        timestamp(1_780_000_000_000),
        KnotMessage::Control(message),
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
        KnotMessageSource::Knot {
            hardware_uid: hardware_uid.to_string(),
        },
        timestamp(1_780_000_000_000),
        KnotMessage::Sensor(KnotSensorMessage::Measurements(
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
        )),
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

    let KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Hello {
        event_id: first_ack_id,
        config: first_config,
    })) = first.payload
    else {
        panic!("expected Hello ACK");
    };
    let KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Hello {
        event_id: duplicate_ack_id,
        config: duplicate_config,
    })) = duplicate.payload
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
        KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Processed { event_id }))
            if event_id == applied_id
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
        Some(KnotMessage::Control(KnotControlMessage::Ack(
            KnotAck::Processed { event_id }
        ))) if event_id == measurement_id
    ));

    Ok(())
}
