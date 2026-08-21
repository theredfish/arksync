// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_knot::application::{
    KnotOutboxError, KnotProtocolRuntime, KnotProtocolRuntimeError, RetryPolicy,
};
use arksync_knot_protocol::{
    KnotAck, KnotCapabilities, KnotConfig, KnotControlMessage, KnotEnvelope, KnotMessage,
    KnotMessageSource, KnotNack, KnotNackReason,
};

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

fn hub_envelope(payload: KnotControlMessage) -> KnotEnvelope {
    EventEnvelope::new_with_id(
        EventId::from_bytes([9; 16]),
        KnotMessageSource::Hub { hub_id: [1; 16] },
        timestamp(1_780_000_000_100),
        KnotMessage::Control(payload),
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
            &hub_envelope(KnotControlMessage::Ack(KnotAck::Hello {
                event_id: hello_id,
                config: config(7),
            })),
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
        KnotMessage::Control(KnotControlMessage::ConfigApplied(_))
    ));
}

#[test]
fn processed_ack_removes_the_pending_message() {
    let hello_id = EventId::from_bytes([1; 16]);
    let mut runtime = runtime(RetryPolicy::default());
    runtime
        .start(hello_id, timestamp(1_780_000_000_000), 10_000)
        .unwrap();

    runtime
        .receive(
            &hub_envelope(KnotControlMessage::Ack(KnotAck::Processed {
                event_id: hello_id,
            })),
            EventId::from_bytes([2; 16]),
            timestamp(1_780_000_000_100),
            10_100,
        )
        .unwrap();

    assert_eq!(runtime.pending_message_count(), 0);
    assert!(runtime.due_messages(20_000).is_empty());
}

#[test]
fn retryable_nack_reschedules_and_terminal_nack_removes() {
    let hello_id = EventId::from_bytes([1; 16]);
    let mut runtime = runtime(RetryPolicy::default());
    runtime
        .start(hello_id, timestamp(1_780_000_000_000), 10_000)
        .unwrap();
    runtime.due_messages(10_000);

    runtime
        .receive(
            &hub_envelope(KnotControlMessage::Nack(KnotNack {
                event_id: hello_id,
                reason: KnotNackReason::TemporarilyUnavailable,
            })),
            EventId::from_bytes([2; 16]),
            timestamp(1_780_000_000_100),
            10_100,
        )
        .unwrap();
    assert_eq!(runtime.due_messages(10_100).len(), 1);

    runtime
        .receive(
            &hub_envelope(KnotControlMessage::Nack(KnotNack {
                event_id: hello_id,
                reason: KnotNackReason::ConfigurationConflict,
            })),
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
        KnotMessageSource::Hub { hub_id: [1; 16] },
        timestamp(1_780_000_000_000),
        KnotMessage::Control(KnotControlMessage::Configure(config(2))),
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
        KnotMessage::Control(KnotControlMessage::ConfigApplied(applied))
            if applied.event_id == configure_id && applied.config_version == 2
    ));
}
