// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![cfg(feature = "knot-tokio-runtime")]

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_knot::application::{
    local_tokio_message_link, ConnectionState, KeepAliveConfig, KnotObservationError,
    KnotPublishFailure, MessageLink, RetryPolicy, TokioKnotRuntimeConfig, TokioKnotSession,
};
use arksync_protocol::knot::{
    KnotAck, KnotCapabilities, KnotConfig, KnotControlMessage, KnotEnvelope, KnotMessage,
    KnotSensorMeasurementBatch, KnotSensorMessage,
};
use arksync_protocol::knot::{
    KnotLiveMeasurement, KnotMeasurementQuality, KnotMeasurementUnit, KnotObservationMessage,
    KnotObservedSensor, KnotSensorConnection, KnotSensorDescriptor, KnotSensorInventory,
    KnotSensorKind, KnotSensorObservationState, KnotSensorRef,
};
use arksync_protocol::ArkSyncActor;
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration};
use tokio_util::sync::CancellationToken;

fn hub_message(message: KnotControlMessage) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        EventId::new(),
        ArkSyncActor::Hub { hub_id: [1; 16] },
        Timestamp::from_unix_millis(0),
        KnotMessage::Control(message),
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
            .publish(KnotMessage::Control(KnotControlMessage::Heartbeat))
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
        hub.send(hub_message(KnotControlMessage::Ack(KnotAck::Hello {
            event_id: hello.id,
            config: config.clone(),
        })))
        .await
        .unwrap();
        let applied = hub.receive().await.unwrap().unwrap();
        assert!(matches!(
            applied.payload,
            KnotMessage::Control(KnotControlMessage::ConfigApplied(_))
        ));
        hub.send(hub_message(KnotControlMessage::Ack(KnotAck::Processed {
            event_id: applied.id,
        })))
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
                    KnotMessage::Sensor(KnotSensorMessage::Measurements(
                        KnotSensorMeasurementBatch {
                            measurements: vec![],
                        },
                    )),
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
        hub.send(hub_message(KnotControlMessage::Ack(KnotAck::Processed {
            event_id: first.id,
        })))
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
            hub.send(hub_message(KnotControlMessage::Ack(KnotAck::Processed {
                event_id: message.id,
            })))
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
            KnotMessage::Control(KnotControlMessage::Heartbeat)
        );
        hub.send(hub_message(KnotControlMessage::Ack(KnotAck::Heartbeat {
            event_id: probe.id,
        })))
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
async fn live_updates_coalesce_on_a_full_link_while_control_and_periodic_inventory_progress() {
    timeout(Duration::from_secs(12), async {
        let (mut hub, knot) = local_tokio_message_link(1);
        let session = TokioKnotSession::new(
            TokioKnotRuntimeConfig {
                hardware_uid: "live-knot".into(),
                capabilities: KnotCapabilities::default(),
                retry_policy: RetryPolicy {
                    capacity: 1,
                    ..RetryPolicy::default()
                },
                keep_alive_config: KeepAliveConfig::new(1_000, 10_000).unwrap(),
            },
            knot,
        );
        let handle = session.handle();
        let mut status = handle.status();
        let session_id = handle.observation_session_id();
        let inventory = KnotObservationMessage::Inventory(KnotSensorInventory {
            session_id,
            revision: 1,
            sensors: vec![KnotObservedSensor {
                reference: KnotSensorRef(1),
                device_uid: None,
                sensor: KnotSensorDescriptor {
                    hardware_uid: "".into(),
                    kind: KnotSensorKind::Temperature,
                    connection: KnotSensorConnection::I2c { address: 0x66 },
                    firmware: None,
                },
                state: KnotSensorObservationState::Present,
            }],
        });
        handle
            .try_publish_at(
                KnotMessage::Observation(inventory.clone()),
                Timestamp::from_unix_millis(10),
            )
            .unwrap();
        for sequence in 1..=200 {
            handle
                .try_observe_at(
                    &KnotObservationMessage::Measurement(KnotLiveMeasurement {
                        session_id,
                        inventory_revision: 1,
                        sensor: KnotSensorRef(1),
                        sequence,
                        acquired_at: Timestamp::from_unix_millis(sequence as i64),
                        value: Some(sequence as f64),
                        unit: KnotMeasurementUnit::Celsius,
                        quality: KnotMeasurementQuality::Valid,
                    }),
                    Timestamp::from_unix_millis(sequence as i64),
                )
                .unwrap();
        }
        let shutdown = CancellationToken::new();
        let _shutdown_on_drop = shutdown.clone().drop_guard();
        let mut tasks = JoinSet::new();
        tasks.spawn(session.run(shutdown.clone()));
        let hello = hub.receive().await.unwrap().unwrap();
        assert_eq!(hello.id, session_id);
        assert!(matches!(
            hello.payload,
            KnotMessage::Control(KnotControlMessage::Hello(_))
        ));
        let mut config = KnotConfig {
            version: 1,
            knot_id: [2; 16],
            sensor_bindings: vec![],
            actuator_configs: vec![],
        };
        hub.send(hub_message(KnotControlMessage::Ack(KnotAck::Hello {
            event_id: hello.id,
            config: config.clone(),
        })))
        .await
        .unwrap();
        let applied = hub.receive().await.unwrap().unwrap();
        assert!(
            matches!(
                applied.payload,
                KnotMessage::Control(KnotControlMessage::ConfigApplied(_))
            ),
            "configuration confirmation has priority over live observations"
        );
        // Leave the only reliable slot occupied: live traffic must still progress.
        let snapshot = hub.receive().await.unwrap().unwrap();
        assert_eq!(
            snapshot.payload,
            KnotMessage::Observation(inventory.clone())
        );
        assert_eq!(status.borrow().pending_messages, 1);
        hub.send(hub_message(KnotControlMessage::Ack(KnotAck::Processed {
            event_id: applied.id,
        })))
        .await
        .unwrap();
        config.version = 2;
        let configure = hub_message(KnotControlMessage::Configure(config.clone()));
        let configure_id = configure.id;
        hub.send(configure).await.unwrap();
        let mut measurements = 0;
        let mut configured = false;
        while measurements == 0 || !configured {
            let message = hub.receive().await.unwrap().unwrap();
            match &message.payload {
                KnotMessage::Observation(KnotObservationMessage::Measurement(reading)) => {
                    measurements += 1;
                    assert_eq!(reading.sequence, 200);
                    assert_eq!(reading.acquired_at, Timestamp::from_unix_millis(200));
                }
                KnotMessage::Control(KnotControlMessage::ConfigApplied(result)) => {
                    assert_eq!(result.event_id, configure_id);
                    assert_eq!(result.config_version, 2);
                    configured = true;
                    hub.send(hub_message(KnotControlMessage::Ack(KnotAck::Processed {
                        event_id: message.id,
                    })))
                    .await
                    .unwrap();
                }
                other => panic!("unexpected message during live/control exchange: {other:?}"),
            }
        }
        status
            .wait_for(|state| state.pending_messages == 0 && state.config == Some(config.clone()))
            .await
            .unwrap();
        // Periodic snapshots repair a lost snapshot, but a sent reading never retries.
        loop {
            let message = hub.receive().await.unwrap().unwrap();
            match message.payload {
                KnotMessage::Observation(KnotObservationMessage::Inventory(_)) => {
                    assert_eq!(message.payload, KnotMessage::Observation(inventory.clone()));
                    assert_eq!(message.occurred_at, snapshot.occurred_at);
                    break;
                }
                KnotMessage::Control(KnotControlMessage::Heartbeat) => {
                    hub.send(hub_message(KnotControlMessage::Ack(KnotAck::Heartbeat {
                        event_id: message.id,
                    })))
                    .await
                    .unwrap();
                }
                other => panic!("live measurement must not retry: {other:?}"),
            }
        }
        assert_eq!(measurements, 1);
        shutdown.cancel();
        tasks.join_next().await.unwrap().unwrap().unwrap();
        assert!(handle
            .try_observe_at(&inventory, Timestamp::from_unix_millis(20))
            .is_err());
    })
    .await
    .expect("live traffic blocked control or shutdown");
}

#[tokio::test]
async fn publication_errors_preserve_messages_and_live_admission_does_not_use_the_reliable_queue() {
    let (_hub, knot) = local_tokio_message_link(1);
    let session = TokioKnotSession::new(
        TokioKnotRuntimeConfig {
            hardware_uid: "admission".into(),
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
    let reliable = KnotMessage::Control(KnotControlMessage::Heartbeat);
    handle
        .try_publish_at(reliable.clone(), Timestamp::from_unix_millis(0))
        .unwrap();
    let error = handle
        .try_publish_at(reliable.clone(), Timestamp::from_unix_millis(0))
        .unwrap_err();
    assert_eq!(error.reason, KnotPublishFailure::Full);
    assert_eq!(error.message, reliable);
    let inventory =
        KnotMessage::Observation(KnotObservationMessage::Inventory(KnotSensorInventory {
            session_id: handle.observation_session_id(),
            revision: 1,
            sensors: vec![],
        }));
    handle.publish(inventory.clone()).await.unwrap();
    let mut invalid = inventory.clone();
    let KnotMessage::Observation(KnotObservationMessage::Inventory(ref mut data)) = invalid else {
        unreachable!()
    };
    data.revision = 0;
    let error = handle.publish(invalid.clone()).await.unwrap_err();
    assert_eq!(
        error.reason,
        KnotPublishFailure::Observation(KnotObservationError::InvalidInventory)
    );
    assert_eq!(error.message, invalid);
    drop(session);
    let error = handle.publish(inventory.clone()).await.unwrap_err();
    assert_eq!(error.reason, KnotPublishFailure::Closed);
    assert_eq!(error.message, inventory);
}
