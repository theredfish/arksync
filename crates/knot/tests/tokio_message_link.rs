// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![cfg(feature = "knot-tokio-runtime")]

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_knot::application::{
    local_tokio_message_link, KnotProtocolRuntime, MessageLink, RetryPolicy,
};
use arksync_protocol::knot::{
    KnotAck, KnotCapabilities, KnotConfig, KnotControlMessage, KnotEnvelope, KnotMessage,
};
use arksync_protocol::ArkSyncActor;

fn timestamp(unix_millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(unix_millis)
}

fn config() -> KnotConfig {
    KnotConfig {
        version: 1,
        knot_id: [2; 16],
        sensor_bindings: vec![],
        actuator_configs: vec![],
    }
}

#[tokio::test]
async fn local_link_exchanges_hello_config_and_confirmation() {
    let hello_id = EventId::from_bytes([1; 16]);
    let hello_ack_id = EventId::from_bytes([2; 16]);
    let confirmation_id = EventId::from_bytes([3; 16]);
    let mut runtime = KnotProtocolRuntime::new(
        "knot-rpi-1".into(),
        KnotCapabilities {
            gpio: true,
            uart: true,
            i2c: false,
            atlas_scientific_ezo: true,
        },
        RetryPolicy::default(),
    );
    let (mut hub_link, mut knot_link) = local_tokio_message_link::<KnotEnvelope>(4);

    runtime
        .start(hello_id, timestamp(1_780_000_000_000), 0)
        .unwrap();
    knot_link
        .send(runtime.due_messages(0).remove(0))
        .await
        .unwrap();

    let hello = hub_link.receive().await.unwrap().unwrap();
    assert_eq!(hello.id, hello_id);
    assert!(matches!(
        hello.payload,
        KnotMessage::Control(KnotControlMessage::Hello(_))
    ));

    hub_link
        .send(EventEnvelope::new_with_id(
            hello_ack_id,
            ArkSyncActor::Hub { hub_id: [1; 16] },
            timestamp(1_780_000_000_100),
            KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Hello {
                event_id: hello_id,
                config: config(),
            })),
        ))
        .await
        .unwrap();

    let hello_ack = knot_link.receive().await.unwrap().unwrap();
    runtime
        .receive(
            &hello_ack,
            confirmation_id,
            timestamp(1_780_000_000_200),
            200,
        )
        .unwrap();
    knot_link
        .send(runtime.due_messages(200).remove(0))
        .await
        .unwrap();

    let confirmation = hub_link.receive().await.unwrap().unwrap();
    assert_eq!(runtime.config(), Some(&config()));
    assert!(matches!(
        confirmation.payload,
        KnotMessage::Control(KnotControlMessage::ConfigApplied(applied))
            if applied.event_id == hello_ack_id && applied.config_version == 1
    ));
}
