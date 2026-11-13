// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_event_router::{EventEnvelope, EventId, Timestamp};
use arksync_knot::application::{
    ConnectionState, KeepAliveConfig, KnotOutboxError, KnotProtocolRuntime,
    KnotProtocolRuntimeError, RetryPolicy,
};
use arksync_protocol::knot::{
    KnotAck, KnotCapabilities, KnotConfig, KnotControlMessage, KnotEnvelope, KnotMessage,
    KnotPayload, KnotRejectionReason,
};
use arksync_protocol::ArkSyncActor;

fn timestamp(unix_millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(unix_millis)
}

fn runtime(policy: RetryPolicy) -> KnotProtocolRuntime {
    KnotProtocolRuntime::new(
        "knot-rpi-1".into(),
        KnotCapabilities {
            gpio: true,
            uart: true,
            i2c: false,
            atlas_scientific_ezo: true,
        },
        policy,
    )
}

fn config(version: u64) -> KnotConfig {
    KnotConfig {
        version,
        knot_id: [2; 16],
        sensor_bindings: vec![],
        actuator_configs: vec![],
    }
}

fn hub_envelope(payload: impl Into<KnotMessage>) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        EventId::from_bytes([9; 16]),
        ArkSyncActor::Hub { hub_id: [1; 16] },
        timestamp(1_780_000_000_100),
        payload.into(),
    )
}

#[test]
fn hello_is_sent_immediately_and_retried_with_the_same_event_id() {
    let hello_id = EventId::from_bytes([1; 16]);
    let mut runtime = runtime(RetryPolicy::default());
    runtime
        .start(hello_id, timestamp(1_780_000_000_000), 10_000)
        .unwrap();

    let first = runtime.due_messages(10_000);
    let before_retry = runtime.due_messages(10_999);
    let retry = runtime.due_messages(11_000);

    assert_eq!(first.len(), 1);
    assert!(before_retry.is_empty());
    assert_eq!(retry.len(), 1);
    assert_eq!(first[0].id, hello_id);
    assert_eq!(retry[0].id, hello_id);
    assert_eq!(retry[0], first[0]);
}

#[test]
fn retry_delay_grows_exponentially_and_stops_at_the_configured_maximum() {
    let mut runtime = runtime(RetryPolicy {
        initial_delay_ms: 1_000,
        max_delay_ms: 3_000,
        capacity: 1,
    });
    runtime
        .start(
            EventId::from_bytes([1; 16]),
            timestamp(1_780_000_000_000),
            0,
        )
        .unwrap();

    assert_eq!(runtime.due_messages(0).len(), 1);
    assert!(runtime.due_messages(999).is_empty());
    assert_eq!(runtime.due_messages(1_000).len(), 1);
    assert!(runtime.due_messages(2_999).is_empty());
    assert_eq!(runtime.due_messages(3_000).len(), 1);
    assert!(runtime.due_messages(5_999).is_empty());
    assert_eq!(runtime.due_messages(6_000).len(), 1);
}

#[test]
fn hello_ack_applies_config_and_queues_confirmation() {
    let hello_id = EventId::from_bytes([1; 16]);
    let confirmation_id = EventId::from_bytes([3; 16]);
    let mut runtime = runtime(RetryPolicy::default());
    runtime
        .start(hello_id, timestamp(1_780_000_000_000), 10_000)
        .unwrap();
    runtime.due_messages(10_000);

    runtime
        .receive(
            &hub_envelope(KnotAck::Hello {
                event_id: hello_id,
                config: config(7),
            }),
            confirmation_id,
            timestamp(1_780_000_000_200),
            10_100,
        )
        .unwrap();
    let confirmation = runtime.due_messages(10_100);

    assert_eq!(runtime.config().map(|config| config.version), Some(7));
    assert_eq!(confirmation.len(), 1);
    assert_eq!(confirmation[0].id, confirmation_id);
    assert!(matches!(
        confirmation[0].payload,
        KnotMessage::Con(KnotPayload::Control(KnotControlMessage::ConfigApplied(_)))
    ));
}

#[test]
fn processed_ack_removes_the_pending_message() {
    let hello_id = EventId::from_bytes([1; 16]);
    let confirmation_id = EventId::from_bytes([3; 16]);
    let mut runtime = runtime(RetryPolicy::default());
    runtime
        .start(hello_id, timestamp(1_780_000_000_000), 10_000)
        .unwrap();

    runtime
        .receive(
            &hub_envelope(KnotAck::Hello {
                event_id: hello_id,
                config: config(1),
            }),
            confirmation_id,
            timestamp(1_780_000_000_100),
            10_100,
        )
        .unwrap();

    runtime
        .receive(
            &hub_envelope(KnotAck::Processed {
                event_id: confirmation_id,
            }),
            EventId::from_bytes([2; 16]),
            timestamp(1_780_000_000_100),
            10_100,
        )
        .unwrap();

    assert_eq!(runtime.pending_message_count(), 0);
    assert!(runtime.due_messages(20_000).is_empty());
}

#[test]
fn a_processed_ack_cannot_replace_the_hello_config() {
    let mut runtime = runtime(RetryPolicy::default());
    let id = EventId::from_bytes([1; 16]);
    runtime.start(id, timestamp(0), 0).unwrap();
    let result = runtime.receive(
        &hub_envelope(KnotAck::Processed { event_id: id }),
        EventId::from_bytes([2; 16]),
        timestamp(1),
        1,
    );
    assert_eq!(result, Err(KnotProtocolRuntimeError::UnexpectedMessage));
    assert_eq!(runtime.pending_message_count(), 1);
    assert_eq!(runtime.connection_state(), ConnectionState::Connecting);
}

#[test]
fn hello_ack_reuses_the_request_slot_in_a_full_outbox() {
    let mut runtime = runtime(RetryPolicy {
        capacity: 1,
        ..RetryPolicy::default()
    });
    let hello_id = EventId::from_bytes([1; 16]);
    let applied_id = EventId::from_bytes([2; 16]);
    runtime.start(hello_id, timestamp(0), 0).unwrap();
    let ack = hub_envelope(KnotAck::Hello {
        event_id: hello_id,
        config: config(1),
    });
    runtime.receive(&ack, applied_id, timestamp(1), 1).unwrap();
    runtime
        .receive(&ack, EventId::from_bytes([3; 16]), timestamp(2), 2)
        .unwrap();

    assert_eq!(runtime.config(), Some(&config(1)));
    assert_eq!(runtime.pending_message_count(), 1);
    assert_eq!(runtime.next_due_message(2).unwrap().id, applied_id);
    assert_eq!(runtime.outbox_overflow_count(), 0);

    runtime
        .receive(
            &hub_envelope(KnotControlMessage::Configure(config(2))),
            EventId::from_bytes([4; 16]),
            timestamp(3),
            3,
        )
        .unwrap();
    assert_eq!(runtime.config(), Some(&config(2)));
    let ack = runtime.next_due_message(3).unwrap();
    assert!(matches!(
        ack.payload,
        KnotMessage::Ack(KnotAck::ConfigApplied {
            config_version: 2,
            ..
        })
    ));
    assert_eq!(
        runtime.pending_message_count(),
        2,
        "ACK has a separate slot from the reliable follow-up"
    );
}

#[test]
fn blocked_transport_does_not_advance_retry_time() {
    let mut runtime = runtime(RetryPolicy::default());
    let id = EventId::from_bytes([1; 16]);
    runtime.start(id, timestamp(0), 0).unwrap();
    assert_eq!(runtime.next_due_message(0).unwrap().id, id);
    assert_eq!(runtime.next_due_message(500).unwrap().id, id);
    runtime.mark_sent(id, 500);
    assert!(runtime.next_due_message(1499).is_none());
    assert_eq!(runtime.next_due_message(1500).unwrap().id, id);
}

#[test]
fn heartbeat_expires_and_recovers_only_with_a_correlated_ack() {
    let mut runtime = runtime(RetryPolicy::default())
        .with_keep_alive_config(KeepAliveConfig::new(100, 300).unwrap());
    let hello_id = EventId::from_bytes([1; 16]);
    let applied_id = EventId::from_bytes([2; 16]);
    let old_probe = EventId::from_bytes([3; 16]);
    let fresh_probe = EventId::from_bytes([4; 16]);
    runtime.start(hello_id, timestamp(0), 10_000).unwrap();
    runtime.tick(old_probe, timestamp(1), 10_001).unwrap();
    assert_eq!(runtime.connection_state(), ConnectionState::Connecting);
    runtime
        .receive(
            &hub_envelope(KnotAck::Hello {
                event_id: hello_id,
                config: config(1),
            }),
            applied_id,
            timestamp(10),
            10_010,
        )
        .unwrap();
    runtime
        .receive(
            &hub_envelope(KnotAck::Processed {
                event_id: applied_id,
            }),
            old_probe,
            timestamp(20),
            10_020,
        )
        .unwrap();
    runtime.tick(old_probe, timestamp(110), 10_110).unwrap();
    assert_eq!(runtime.pending_message_count(), 1);
    assert_eq!(
        runtime.next_due_message(10_110).unwrap().payload,
        KnotMessage::Con(KnotPayload::Control(KnotControlMessage::Heartbeat))
    );
    runtime.tick(fresh_probe, timestamp(320), 10_320).unwrap();
    assert_eq!(runtime.connection_state(), ConnectionState::Unavailable);
    assert_eq!(runtime.config(), Some(&config(1)));
    runtime.tick(fresh_probe, timestamp(410), 10_410).unwrap();
    assert_eq!(
        runtime.pending_message_count(),
        1,
        "expired probes must not accumulate"
    );

    runtime
        .receive(
            &hub_envelope(KnotAck::Processed {
                event_id: old_probe,
            }),
            applied_id,
            timestamp(420),
            10_420,
        )
        .unwrap();
    assert_eq!(runtime.connection_state(), ConnectionState::Unavailable);
    let mut ack = hub_envelope(KnotAck::Processed {
        event_id: fresh_probe,
    });
    ack.source = ArkSyncActor::Hub { hub_id: [99; 16] };
    assert_eq!(
        runtime.receive(&ack, applied_id, timestamp(421), 10_421),
        Err(KnotProtocolRuntimeError::UnexpectedSource)
    );
    ack.source = ArkSyncActor::Hub { hub_id: [1; 16] };
    runtime
        .receive(&ack, applied_id, timestamp(422), 10_422)
        .unwrap();
    assert_eq!(runtime.connection_state(), ConnectionState::Connected);
    assert_eq!(runtime.pending_message_count(), 0);

    runtime.close();
    runtime.tick(old_probe, timestamp(900), 10_900).unwrap();
    assert_eq!(runtime.connection_state(), ConnectionState::Closed);
    assert_eq!(runtime.config(), Some(&config(1)));
}

#[test]
fn retryable_rejection_reschedules_and_terminal_rejection_removes() {
    let hello_id = EventId::from_bytes([1; 16]);
    let mut runtime = runtime(RetryPolicy::default());
    runtime
        .start(hello_id, timestamp(1_780_000_000_000), 10_000)
        .unwrap();
    runtime.due_messages(10_000);

    runtime
        .receive(
            &hub_envelope(KnotAck::Rejected {
                event_id: hello_id,
                reason: KnotRejectionReason::TemporarilyUnavailable,
            }),
            EventId::from_bytes([2; 16]),
            timestamp(1_780_000_000_100),
            10_100,
        )
        .unwrap();
    assert_eq!(runtime.due_messages(10_100).len(), 1);

    runtime
        .receive(
            &hub_envelope(KnotAck::Rejected {
                event_id: hello_id,
                reason: KnotRejectionReason::ConfigurationConflict,
            }),
            EventId::from_bytes([3; 16]),
            timestamp(1_780_000_000_200),
            10_200,
        )
        .unwrap();
    assert_eq!(runtime.pending_message_count(), 0);
}

#[test]
fn full_outbox_is_explicit_and_observable() {
    let mut runtime = runtime(RetryPolicy {
        capacity: 1,
        ..RetryPolicy::default()
    });
    runtime
        .start(
            EventId::from_bytes([1; 16]),
            timestamp(1_780_000_000_000),
            10_000,
        )
        .unwrap();

    let result = runtime.start(
        EventId::from_bytes([2; 16]),
        timestamp(1_780_000_000_100),
        10_100,
    );

    assert_eq!(
        result,
        Err(KnotProtocolRuntimeError::Outbox(KnotOutboxError::Full))
    );
    assert_eq!(runtime.outbox_overflow_count(), 1);
}

#[test]
fn configure_message_replaces_config_and_correlates_confirmation() {
    let configure_id = EventId::from_bytes([4; 16]);
    let confirmation_id = EventId::from_bytes([5; 16]);
    let mut runtime = runtime(RetryPolicy::default());
    let configure = EventEnvelope::new_with_id(
        configure_id,
        ArkSyncActor::Hub { hub_id: [1; 16] },
        timestamp(1_780_000_000_000),
        KnotMessage::Con(KnotPayload::Control(KnotControlMessage::Configure(config(
            2,
        )))),
    );

    runtime
        .receive(
            &configure,
            confirmation_id,
            timestamp(1_780_000_000_100),
            10_000,
        )
        .unwrap();
    let confirmation = runtime.due_messages(10_000);

    assert_eq!(runtime.config().map(|config| config.version), Some(2));
    assert!(matches!(
        confirmation[0].payload,
        KnotMessage::Ack(KnotAck::ConfigApplied { event_id, config_version: 2 })
            if event_id == configure_id
    ));
}

#[test]
fn stale_config_is_rejected_without_replacing_the_current_config() {
    let current_config_event_id = EventId::from_bytes([4; 16]);
    let current_confirmation_id = EventId::from_bytes([5; 16]);
    let stale_config_event_id = EventId::from_bytes([6; 16]);
    let stale_confirmation_id = EventId::from_bytes([7; 16]);
    let mut runtime = runtime(RetryPolicy::default());

    runtime
        .receive(
            &EventEnvelope::new_with_id(
                current_config_event_id,
                ArkSyncActor::Hub { hub_id: [1; 16] },
                timestamp(1_780_000_000_000),
                KnotMessage::Con(KnotPayload::Control(KnotControlMessage::Configure(config(
                    2,
                )))),
            ),
            current_confirmation_id,
            timestamp(1_780_000_000_100),
            0,
        )
        .unwrap();
    runtime.due_messages(0);
    runtime
        .receive(
            &hub_envelope(KnotAck::Processed {
                event_id: current_confirmation_id,
            }),
            EventId::from_bytes([8; 16]),
            timestamp(1_780_000_000_200),
            100,
        )
        .unwrap();

    runtime
        .receive(
            &EventEnvelope::new_with_id(
                stale_config_event_id,
                ArkSyncActor::Hub { hub_id: [1; 16] },
                timestamp(1_780_000_000_300),
                KnotMessage::Con(KnotPayload::Control(KnotControlMessage::Configure(config(
                    1,
                )))),
            ),
            stale_confirmation_id,
            timestamp(1_780_000_000_400),
            200,
        )
        .unwrap();
    let rejection = runtime.due_messages(200);

    assert_eq!(runtime.config().map(|config| config.version), Some(2));
    assert!(matches!(
        &rejection[0].payload,
        KnotMessage::Ack(KnotAck::ConfigRejected { event_id, config_version: 1, reason })
            if *event_id == stale_config_event_id
                && reason == "received stale Knot config version"
    ));
}

#[test]
fn duplicate_configure_reuses_the_pending_confirmation_even_with_a_full_outbox() {
    let mut runtime = runtime(RetryPolicy {
        capacity: 1,
        ..RetryPolicy::default()
    });
    let command = hub_envelope(KnotControlMessage::Configure(config(2)));
    let confirmation_id = EventId::from_bytes([4; 16]);
    runtime
        .receive(&command, confirmation_id, timestamp(0), 0)
        .unwrap();
    let first = runtime.due_messages(0);
    runtime
        .receive(&command, EventId::from_bytes([5; 16]), timestamp(10), 10)
        .unwrap();
    assert_eq!(runtime.pending_message_count(), 1);
    assert_eq!(runtime.outbox_overflow_count(), 0);
    assert_eq!(runtime.due_messages(1_000), first);
}

#[test]
fn configuration_identity_and_content_cannot_change_without_a_new_version() {
    let mut runtime = runtime(RetryPolicy::default());
    runtime
        .receive(
            &hub_envelope(KnotControlMessage::Configure(config(2))),
            EventId::from_bytes([4; 16]),
            timestamp(0),
            0,
        )
        .unwrap();
    let mut changed_content = config(2);
    changed_content
        .sensor_bindings
        .push(arksync_protocol::knot::KnotSensorBinding {
            sensor_id: [3; 16],
            device_uid: "sensor".into(),
        });
    let mut changed_identity = config(3);
    changed_identity.knot_id = [7; 16];
    for (index, invalid) in [config(0), changed_content, changed_identity]
        .into_iter()
        .enumerate()
    {
        let response_id = EventId::from_bytes([10 + index as u8; 16]);
        let mut command = hub_envelope(KnotControlMessage::Configure(invalid));
        command.id = EventId::from_bytes([20 + index as u8; 16]);
        runtime
            .receive(&command, response_id, timestamp(0), 0)
            .unwrap();
        assert_eq!(runtime.config(), Some(&config(2)));
        let messages = runtime.due_messages(0);
        assert!(messages.iter().any(|message| message.id == response_id
            && matches!(
                message.payload,
                KnotMessage::Ack(KnotAck::ConfigRejected { .. })
            )));
    }
}

#[test]
fn non_is_sent_once_even_while_the_reliable_outbox_is_full() {
    let mut runtime = runtime(RetryPolicy {
        capacity: 1,
        ..RetryPolicy::default()
    });
    let hello_id = EventId::from_bytes([1; 16]);
    let applied_id = EventId::from_bytes([2; 16]);
    let non_id = EventId::from_bytes([3; 16]);
    runtime.start(hello_id, timestamp(0), 0).unwrap();
    runtime
        .receive(
            &hub_envelope(KnotAck::Hello {
                event_id: hello_id,
                config: config(1),
            }),
            applied_id,
            timestamp(1),
            1,
        )
        .unwrap();
    let non = KnotMessage::Non(KnotPayload::Sensor(
        arksync_protocol::knot::KnotSensorMessage::Measurements(
            arksync_protocol::knot::KnotSensorMeasurementBatch {
                measurements: vec![],
            },
        ),
    ));
    runtime
        .publish(non.clone(), non_id, timestamp(2), 2)
        .unwrap();
    assert!(!runtime.can_publish());
    assert!(!runtime.can_publish_non());
    runtime.mark_sent(applied_id, 2);
    assert_eq!(runtime.next_due_message(2).unwrap().id, non_id);
    assert_eq!(
        runtime.next_due_message(500).unwrap().payload,
        non,
        "a full link does not consume NON"
    );
    runtime.mark_sent(non_id, 500);
    assert!(runtime.can_publish_non());
    assert!(runtime.next_due_message(501).is_none());
    runtime
        .receive(
            &hub_envelope(KnotAck::Processed {
                event_id: applied_id,
            }),
            EventId::from_bytes([4; 16]),
            timestamp(600),
            600,
        )
        .unwrap();
    assert!(runtime.due_messages(60_000).is_empty());
}

#[test]
fn a_full_ack_queue_cannot_overwrite_configuration_or_its_cached_result() {
    let mut runtime = runtime(RetryPolicy {
        capacity: 1,
        ..RetryPolicy::default()
    });
    let first = hub_envelope(KnotControlMessage::Configure(config(1)));
    let mut second = hub_envelope(KnotControlMessage::Configure(config(2)));
    second.id = EventId::from_bytes([8; 16]);
    runtime
        .receive(&first, EventId::from_bytes([1; 16]), timestamp(0), 0)
        .unwrap();
    let reply = runtime.next_due_message(0).unwrap().clone();
    assert_eq!(
        runtime.receive(&second, EventId::from_bytes([2; 16]), timestamp(1), 1),
        Err(KnotProtocolRuntimeError::Outbox(KnotOutboxError::Full))
    );
    assert_eq!(runtime.config(), Some(&config(1)));
    runtime.mark_sent(reply.id, 1);
    assert!(
        runtime.next_due_message(30_000).is_none(),
        "ACKs are never retried on their own"
    );
    runtime
        .receive(
            &first,
            EventId::from_bytes([3; 16]),
            timestamp(30_001),
            30_001,
        )
        .unwrap();
    assert_eq!(runtime.due_messages(30_001), vec![reply]);
    runtime
        .receive(
            &second,
            EventId::from_bytes([4; 16]),
            timestamp(30_002),
            30_002,
        )
        .unwrap();
    assert_eq!(runtime.config(), Some(&config(2)));
}

#[test]
fn reusing_a_configure_id_with_different_content_does_not_replace_the_reply() {
    let mut runtime = runtime(RetryPolicy::default());
    let first = hub_envelope(KnotControlMessage::Configure(config(1)));
    runtime
        .receive(&first, EventId::from_bytes([1; 16]), timestamp(0), 0)
        .unwrap();
    let reply = runtime.due_messages(0);
    let conflicting = hub_envelope(KnotControlMessage::Configure(config(2)));
    assert_eq!(
        runtime.receive(&conflicting, EventId::from_bytes([2; 16]), timestamp(1), 1),
        Err(KnotProtocolRuntimeError::UnexpectedMessage)
    );
    assert_eq!(runtime.config(), Some(&config(1)));
    assert!(matches!(
        runtime.due_messages(1)[0].payload,
        KnotMessage::Ack(KnotAck::Rejected {
            reason: KnotRejectionReason::ConfigurationConflict,
            ..
        })
    ));
    runtime
        .receive(&first, EventId::from_bytes([3; 16]), timestamp(2), 2)
        .unwrap();
    assert_eq!(runtime.due_messages(2), reply);
}

#[test]
fn non_configure_is_silent_and_application_cannot_publish_ack() {
    let mut runtime = runtime(RetryPolicy::default());
    let message = KnotMessage::Non(KnotPayload::Control(KnotControlMessage::Configure(config(
        1,
    ))));
    assert_eq!(
        runtime.receive(
            &hub_envelope(message),
            EventId::from_bytes([1; 16]),
            timestamp(0),
            0
        ),
        Err(KnotProtocolRuntimeError::UnexpectedMessage)
    );
    assert_eq!(runtime.config(), None);
    assert!(runtime.due_messages(0).is_empty());
    assert_eq!(
        runtime.publish(
            KnotAck::Processed {
                event_id: EventId::from_bytes([2; 16])
            }
            .into(),
            EventId::from_bytes([3; 16]),
            timestamp(0),
            0
        ),
        Err(KnotProtocolRuntimeError::UnexpectedMessage)
    );
}

#[test]
fn a_valid_configure_retry_restores_presence_and_replays_its_ack() {
    let mut runtime = runtime(RetryPolicy::default())
        .with_keep_alive_config(KeepAliveConfig::new(100, 300).unwrap());
    let command = hub_envelope(KnotControlMessage::Configure(config(1)));
    runtime
        .receive(&command, EventId::from_bytes([1; 16]), timestamp(0), 0)
        .unwrap();
    let reply = runtime.due_messages(0)[0].clone();
    runtime
        .tick(EventId::from_bytes([2; 16]), timestamp(400), 400)
        .unwrap();
    assert_eq!(runtime.connection_state(), ConnectionState::Unavailable);
    runtime
        .receive(&command, EventId::from_bytes([3; 16]), timestamp(401), 401)
        .unwrap();
    assert_eq!(runtime.connection_state(), ConnectionState::Connected);
    assert_eq!(runtime.next_due_message(401), Some(&reply));
}

#[test]
fn an_unsupported_con_is_rejected_without_being_accepted_by_the_runtime() {
    let mut runtime = runtime(RetryPolicy::default());
    let command = hub_envelope(KnotControlMessage::Heartbeat);
    assert_eq!(
        runtime.receive(&command, EventId::from_bytes([1; 16]), timestamp(0), 0),
        Err(KnotProtocolRuntimeError::UnexpectedMessage)
    );
    assert!(matches!(runtime.due_messages(0)[0].payload,
        KnotMessage::Ack(KnotAck::Rejected { event_id, reason: KnotRejectionReason::UnsupportedMessage })
            if event_id == command.id));
}
