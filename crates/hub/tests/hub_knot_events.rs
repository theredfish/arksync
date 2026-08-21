// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventHandlerError, EventId, EventRouter, Timestamp};
use arksync_hub::{handle_knot_event, list_knots, setup_local_station};
use arksync_knot::application::{KnotCapabilities, KnotHello, KnotMessage, KnotMessageEnvelope};

fn timestamp(unix_millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(unix_millis)
}

fn knot_hello_event(hardware_uid: &str) -> KnotMessageEnvelope {
    EventEnvelope::new_with_id(
        EventId::from_bytes([7; 16]),
        (),
        timestamp(1_780_000_000_000),
        KnotMessage::Hello(KnotHello {
            hardware_uid: hardware_uid.to_string(),
            capabilities: KnotCapabilities {
                gpio: true,
                uart: true,
                i2c: false,
                atlas_scientific_ezo: true,
            },
        }),
    )
}

#[arksync_testing::test]
async fn knot_hello_event_registers_knot(pool: arksync_testing::PgPool) -> eyre::Result<()> {
    let hardware_uid = "test-knot-rpi-1";
    let mut txn = pool.begin().await?;
    setup_local_station(&mut txn).await?;
    txn.commit().await?;

    let (knot_event_tx, mut knot_event_rx) = tokio::sync::mpsc::channel(1);
    let (knot_message_tx, mut knot_message_rx) = tokio::sync::mpsc::channel(1);
    let mut router = EventRouter::new();
    router.subscribe(move |event: &KnotMessageEnvelope| {
        knot_event_tx
            .try_send(event.clone())
            .map_err(|_| EventHandlerError::Rejected)
    });

    let event = knot_hello_event(hardware_uid);
    let report = router.publish(&event);
    let event = knot_event_rx
        .recv()
        .await
        .expect("Knot hello event should be routed by the EventRouter");

    handle_knot_event(&pool, event, &knot_message_tx).await?;

    let ack = knot_message_rx
        .recv()
        .await
        .expect("Hub should ACK the Knot hello");
    let KnotMessage::Ack(ack) = ack else {
        panic!("expected Knot ACK");
    };
    let knots = list_knots(&pool).await?;
    let remote_knot = knots
        .iter()
        .find(|knot| knot.hardware_uid == hardware_uid)
        .expect("remote Knot should be registered from Hello");

    assert_eq!(report.delivered, 1);
    assert_eq!(ack.config.hardware_uid, hardware_uid);
    assert_eq!(remote_knot.role, "remote_knot");
    assert_eq!(remote_knot.status, "awake");

    Ok(())
}
