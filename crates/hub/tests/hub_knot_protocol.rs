// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_hub::{handle_knot_protocol_event, list_knots, setup_local_station};
use arksync_knot_protocol::{
    KnotAck, KnotCapabilities, KnotConfigApplied, KnotControlMessage, KnotEnvelope, KnotHello,
    KnotMessage, KnotMessageSource,
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

#[arksync_testing::test]
async fn duplicate_hello_registers_one_knot_and_returns_correlated_ack(
    pool: arksync_testing::PgPool,
) -> eyre::Result<()> {
    let hardware_uid = "test-knot-rpi-1";
    let hello_id = EventId::from_bytes([7; 16]);
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;
    let event = hello(hello_id, hardware_uid);

    let first = handle_knot_protocol_event(
        &pool,
        &event,
        EventId::from_bytes([8; 16]),
        timestamp(1_780_000_000_100),
    )
    .await?
    .expect("Hello must produce a response");
    let duplicate = handle_knot_protocol_event(
        &pool,
        &event,
        EventId::from_bytes([9; 16]),
        timestamp(1_780_000_000_200),
    )
    .await?
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
    let hello_event = hello(EventId::from_bytes([1; 16]), hardware_uid);
    handle_knot_protocol_event(
        &pool,
        &hello_event,
        EventId::from_bytes([2; 16]),
        timestamp(1_780_000_000_100),
    )
    .await?;
    let applied_id = EventId::from_bytes([3; 16]);
    let applied_event = knot_event(
        applied_id,
        hardware_uid,
        KnotControlMessage::ConfigApplied(KnotConfigApplied { config_version: 1 }),
    );

    let response = handle_knot_protocol_event(
        &pool,
        &applied_event,
        EventId::from_bytes([4; 16]),
        timestamp(1_780_000_000_200),
    )
    .await?
    .expect("ConfigApplied must be acknowledged");
    handle_knot_protocol_event(
        &pool,
        &applied_event,
        EventId::from_bytes([5; 16]),
        timestamp(1_780_000_000_300),
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
