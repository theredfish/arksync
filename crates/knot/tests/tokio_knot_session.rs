// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![cfg(feature = "knot-tokio-runtime")]

use arksync_event_router::{EventEnvelope, EventId, Timestamp};
use arksync_knot::application::{
    local_tokio_message_link, ConnectionState, KeepAliveConfig, KnotPublishError, MessageLink,
    RetryPolicy, TokioKnotRuntimeConfig, TokioKnotSession,
};
use arksync_protocol::knot::{
    KnotAck, KnotCapabilities, KnotConfig, KnotControlMessage, KnotEnvelope, KnotMessage,
    KnotPayload, KnotSensorMeasurementBatch, KnotSensorMessage,
};
use arksync_protocol::ArkSyncActor;
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

fn hub_message(message: impl Into<KnotMessage>) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        EventId::new(),
        ArkSyncActor::Hub { hub_id: [1; 16] },
        Timestamp::from_unix_millis(0),
        message.into(),
    )
}

#[tokio::test(start_paused = true)]
async fn a_silent_peer_expires_and_a_full_link_does_not_prevent_shutdown() {
    timeout(Duration::from_secs(5), async {
        let (_silent_hub, knot) = local_tokio_message_link(1);
        let session = TokioKnotSession::new(
            TokioKnotRuntimeConfig {
                hardware_uid: "unanswered-knot".into(),
                capabilities: KnotCapabilities::default(),
                retry_policy: RetryPolicy {
                    initial_delay_ms: 10,
                    max_delay_ms: 100,
                    capacity: 1,
                },
                keep_alive_config: KeepAliveConfig::new(100, 300).unwrap(),
            },
            knot,
        );
        let handle = session.handle();
        let mut status = handle.status();
        let shutdown = CancellationToken::new();
        let mut tasks = JoinSet::new();
        tasks.spawn(session.run(shutdown.clone()));
        status
            .wait_for(|state| state.connection == ConnectionState::Unavailable)
            .await
            .unwrap();
        assert_eq!(status.borrow().pending_messages, 1);
        assert!(status.borrow().config.is_none());
        shutdown.cancel();
        tasks.join_next().await.unwrap().unwrap().unwrap();
        assert_eq!(status.borrow().connection, ConnectionState::Closed);
        assert!(handle
            .publish(KnotMessage::Con(KnotPayload::Control(
                KnotControlMessage::Heartbeat
            )))
            .await
            .is_err());
    })
    .await
    .expect("shutdown stalled on a full link");
}

#[tokio::test(start_paused = true)]
async fn saturated_link_still_receives_acks_and_keeps_config_after_closure() {
    timeout(Duration::from_secs(10), async {
        let (mut hub, knot) = local_tokio_message_link(1);
        let session = TokioKnotSession::new(
            TokioKnotRuntimeConfig {
                hardware_uid: "knot-without-hardware".into(),
                capabilities: KnotCapabilities::default(),
                retry_policy: RetryPolicy {
                    capacity: 8,
                    ..RetryPolicy::default()
                },
                keep_alive_config: KeepAliveConfig::new(1000, 3000).unwrap(),
            },
            knot,
        );
        let handle = session.handle();
        let mut status = handle.status();
        let mut tasks = JoinSet::new();
        tasks.spawn(session.run(CancellationToken::new()));

        let hello = hub.receive().await.unwrap().unwrap();
        let config = KnotConfig {
            version: 1,
            knot_id: [2; 16],
            sensor_bindings: vec![],
            actuator_configs: vec![],
        };
        hub.send(hub_message(KnotAck::Hello {
            event_id: hello.id,
            config: config.clone(),
        }))
        .await
        .unwrap();
        let applied = hub.receive().await.unwrap().unwrap();
        assert!(matches!(
            applied.payload,
            KnotMessage::Con(KnotPayload::Control(KnotControlMessage::ConfigApplied(_)))
        ));
        hub.send(hub_message(KnotAck::Processed {
            event_id: applied.id,
        }))
        .await
        .unwrap();
        status
            .wait_for(|status| {
                status.connection == ConnectionState::Connected && status.pending_messages == 0
            })
            .await
            .unwrap();

        let acquired_at = Timestamp::from_unix_millis(123);
        for _ in 0..4 {
            handle
                .try_publish_at(
                    KnotMessage::Con(KnotPayload::Sensor(KnotSensorMessage::Measurements(
                        KnotSensorMeasurementBatch {
                            measurements: vec![],
                        },
                    ))),
                    acquired_at,
                )
                .unwrap();
        }
        // Waiting for four pending messages ensures the one-slot transport is full.
        status
            .wait_for(|status| status.pending_messages == 4)
            .await
            .unwrap();
        let first = hub.receive().await.unwrap().unwrap();
        // An awaited flush of the whole batch would deadlock on the third message.
        hub.send(hub_message(KnotAck::Processed { event_id: first.id }))
            .await
            .unwrap();
        status
            .wait_for(|status| status.pending_messages == 3)
            .await
            .unwrap();
        assert_eq!(first.occurred_at, acquired_at);
        for _ in 0..3 {
            let message = hub.receive().await.unwrap().unwrap();
            assert_eq!(message.occurred_at, acquired_at);
            hub.send(hub_message(KnotAck::Processed {
                event_id: message.id,
            }))
            .await
            .unwrap();
        }
        status
            .wait_for(|status| status.pending_messages == 0)
            .await
            .unwrap();
        let probe = hub.receive().await.unwrap().unwrap();
        assert_eq!(
            probe.payload,
            KnotMessage::Con(KnotPayload::Control(KnotControlMessage::Heartbeat))
        );
        hub.send(hub_message(KnotAck::Processed { event_id: probe.id }))
            .await
            .unwrap();
        status
            .wait_for(|status| status.pending_messages == 0)
            .await
            .unwrap();

        drop(hub);
        tasks.join_next().await.unwrap().unwrap().unwrap();
        assert_eq!(status.borrow().connection, ConnectionState::Closed);
        assert_eq!(status.borrow().config, Some(config));
    })
    .await
    .expect("local session stalled");
}

#[tokio::test(start_paused = true)]
async fn non_publication_progresses_with_a_full_reliable_outbox_and_application_queue() {
    timeout(Duration::from_secs(3), async {
        let (mut hub, knot) = local_tokio_message_link(1);
        let session = TokioKnotSession::new(
            TokioKnotRuntimeConfig {
                hardware_uid: "best-effort-knot".into(),
                capabilities: KnotCapabilities::default(),
                retry_policy: RetryPolicy {
                    capacity: 1,
                    ..RetryPolicy::default()
                },
                keep_alive_config: KeepAliveConfig::default(),
            },
            knot,
        );
        let handle = session.handle();
        let shutdown = CancellationToken::new();
        let task = tokio::spawn(session.run(shutdown.clone()));
        let hello = hub.receive().await.unwrap().unwrap();
        hub.send(hub_message(KnotAck::Hello {
            event_id: hello.id,
            config: KnotConfig {
                version: 1,
                knot_id: [2; 16],
                sensor_bindings: vec![],
                actuator_configs: vec![],
            },
        }))
        .await
        .unwrap();
        let applied = hub.receive().await.unwrap().unwrap();
        assert!(applied.payload.requires_ack());
        // Leave ConfigApplied unconfirmed, filling the only reliable slot.
        let payload = KnotPayload::Sensor(KnotSensorMessage::Measurements(
            KnotSensorMeasurementBatch {
                measurements: vec![],
            },
        ));
        handle
            .try_publish_at(
                KnotMessage::Con(payload.clone()),
                Timestamp::from_unix_millis(1),
            )
            .unwrap();
        assert!(matches!(
            handle.try_publish_at(
                KnotMessage::Con(payload.clone()),
                Timestamp::from_unix_millis(2)
            ),
            Err(KnotPublishError::Full(_))
        ));
        handle
            .try_publish_at(
                KnotMessage::Non(payload.clone()),
                Timestamp::from_unix_millis(3),
            )
            .unwrap();
        let received = hub.receive().await.unwrap().unwrap();
        assert_eq!(received.payload, KnotMessage::Non(payload));
        assert_eq!(received.occurred_at, Timestamp::from_unix_millis(3));
        let ack = KnotMessage::Ack(KnotAck::Processed {
            event_id: applied.id,
        });
        assert!(
            matches!(handle.try_publish_at(ack.clone(), Timestamp::from_unix_millis(4)),
            Err(KnotPublishError::Unsupported(message)) if message == ack)
        );
        shutdown.cancel();
        task.await.unwrap().unwrap();
    })
    .await
    .expect("NON publication stalled behind the reliable queue");
}
