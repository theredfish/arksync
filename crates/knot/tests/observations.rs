// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventId, Timestamp};
use arksync_knot::application::{
    KnotObservationCursor, KnotObservationError, KnotObservationPublisher, KnotProtocolRuntime,
    KnotProtocolRuntimeError, RetryPolicy, DEFAULT_INVENTORY_INTERVAL_MS,
};
use arksync_protocol::knot::*;
use arksync_protocol::ArkSyncActor;

fn session() -> EventId {
    EventId::from_bytes([1; 16])
}
fn time(value: i64) -> Timestamp {
    Timestamp::from_unix_millis(value)
}
fn inventory(revision: u64, references: &[u32]) -> KnotObservationMessage {
    KnotObservationMessage::Inventory(KnotSensorInventory {
        session_id: session(),
        revision,
        sensors: references
            .iter()
            .map(|reference| KnotObservedSensor {
                reference: KnotSensorRef(*reference),
                device_uid: None,
                sensor: KnotSensorDescriptor {
                    hardware_uid: String::new(),
                    kind: KnotSensorKind::Temperature,
                    connection: KnotSensorConnection::I2c { address: 0x66 },
                    firmware: None,
                },
                state: KnotSensorObservationState::Present,
            })
            .collect(),
    })
}
fn sample(revision: u64, reference: u32, sequence: u64) -> KnotObservationMessage {
    KnotObservationMessage::Measurement(KnotLiveMeasurement {
        session_id: session(),
        inventory_revision: revision,
        sensor: KnotSensorRef(reference),
        sequence,
        acquired_at: time(100 - sequence as i64),
        value: Some(sequence as f64),
        unit: KnotMeasurementUnit::Celsius,
        quality: KnotMeasurementQuality::Valid,
    })
}

#[test]
fn receiver_requires_current_session_inventory_and_increasing_per_sensor_sequences() {
    let mut cursor = KnotObservationCursor::new(session(), 2);
    assert_eq!(
        cursor.accept(&sample(1, 1, 1)),
        Err(KnotObservationError::MissingInventory)
    );
    cursor.accept(&inventory(1, &[1, 2])).unwrap();
    cursor.accept(&sample(1, 1, 5)).unwrap();
    cursor.accept(&sample(1, 2, 1)).unwrap();
    // An identical periodic snapshot must not reset ordering.
    cursor.accept(&inventory(1, &[1, 2])).unwrap();
    assert_eq!(
        cursor.accept(&sample(1, 1, 5)),
        Err(KnotObservationError::StaleMeasurement)
    );
    assert_eq!(
        cursor.accept(&sample(1, 1, 4)),
        Err(KnotObservationError::StaleMeasurement)
    );
    cursor.accept(&sample(1, 1, 6)).unwrap(); // acquisition clock moved backwards
    assert_eq!(
        cursor.accept(&inventory(1, &[1])),
        Err(KnotObservationError::ConflictingInventory)
    );
    assert_eq!(
        cursor.accept(&inventory(2, &[1, 2, 3])),
        Err(KnotObservationError::CapacityExceeded)
    );
    assert_eq!(
        cursor.accept(&inventory(2, &[1, 1])),
        Err(KnotObservationError::InvalidInventory)
    );
    cursor.accept(&inventory(2, &[2])).unwrap();
    assert_eq!(
        cursor.accept(&sample(1, 2, 100)),
        Err(KnotObservationError::WrongInventoryRevision)
    );
    assert_eq!(
        cursor.accept(&sample(2, 1, 1)),
        Err(KnotObservationError::UnknownSensor)
    );
    assert_eq!(
        cursor.accept(&inventory(1, &[1, 2])),
        Err(KnotObservationError::StaleInventory)
    );
    cursor.accept(&sample(2, 2, 1)).unwrap();
    let mut old = inventory(99, &[]);
    let KnotObservationMessage::Inventory(ref mut old_inventory) = old else {
        unreachable!()
    };
    old_inventory.session_id = EventId::from_bytes([9; 16]);
    assert_eq!(cursor.accept(&old), Err(KnotObservationError::WrongSession));
    cursor.accept(&sample(2, 2, 2)).unwrap();
}

#[test]
fn invalid_values_do_not_advance_the_cursor_and_missing_readings_remain_explicit() {
    let mut cursor = KnotObservationCursor::new(session(), 1);
    cursor.accept(&inventory(1, &[1])).unwrap();
    for value in [None, Some(f64::NAN), Some(f64::INFINITY)] {
        let KnotObservationMessage::Measurement(mut invalid) = sample(1, 1, 1) else {
            unreachable!()
        };
        invalid.value = value;
        assert_eq!(
            cursor.accept(&KnotObservationMessage::Measurement(invalid)),
            Err(KnotObservationError::InvalidMeasurement)
        );
    }
    cursor.accept(&sample(1, 1, 1)).unwrap();
    let KnotObservationMessage::Measurement(mut missing) = sample(1, 1, 2) else {
        unreachable!()
    };
    missing.value = None;
    missing.quality = KnotMeasurementQuality::Unavailable;
    cursor
        .accept(&KnotObservationMessage::Measurement(missing))
        .unwrap();
    cursor.accept(&inventory(2, &[])).unwrap();
    assert_eq!(
        cursor.accept(&sample(2, 1, 1)),
        Err(KnotObservationError::UnknownSensor)
    );
}

#[test]
fn publisher_coalesces_before_transport_and_repeats_inventory_without_retrying_measurements() {
    let mut publisher = KnotObservationPublisher::new(session(), 2);
    publisher
        .publish_at(&inventory(1, &[1, 2]), time(10))
        .unwrap();
    for seq in 1..=100 {
        publisher
            .publish_at(&sample(1, 1, seq), time(seq as i64))
            .unwrap();
    }
    publisher.publish_at(&sample(1, 2, 1), time(200)).unwrap();
    assert_eq!(publisher.pending_measurement_count(), 2);
    let (_, first) = publisher.next_message(0).unwrap();
    assert_eq!(first, inventory(1, &[1, 2]));
    assert_eq!(
        publisher.next_message(1).unwrap().1,
        first,
        "full transport must not consume an inventory"
    );
    publisher.mark_sent(&first, 2);
    let (occurred_at, latest) = publisher.next_message(3).unwrap();
    assert_eq!(latest, sample(1, 1, 100));
    assert_eq!(occurred_at, time(100));
    publisher.publish_at(&sample(1, 1, 101), time(101)).unwrap();
    publisher.mark_sent(&latest, 4);
    assert_eq!(
        publisher.pending_measurement_count(),
        2,
        "a concurrent newer value must survive completion of the older send"
    );
    for now in 5..7 {
        let (_, message) = publisher.next_message(now).unwrap();
        publisher.mark_sent(&message, now);
    }
    assert!(publisher.next_message(8).is_none());
    assert_eq!(
        publisher
            .next_message(DEFAULT_INVENTORY_INTERVAL_MS + 2)
            .unwrap(),
        (time(10), first.clone())
    );
    publisher.mark_sent(&first, DEFAULT_INVENTORY_INTERVAL_MS + 2);
    assert!(publisher
        .next_message(DEFAULT_INVENTORY_INTERVAL_MS + 3)
        .is_none());
    publisher.resynchronize();
    assert_eq!(
        publisher
            .next_message(DEFAULT_INVENTORY_INTERVAL_MS + 4)
            .unwrap()
            .1,
        first
    );
    publisher.publish_at(&sample(1, 1, 102), time(102)).unwrap();
    publisher.publish_at(&inventory(2, &[]), time(300)).unwrap();
    assert_eq!(
        publisher.pending_measurement_count(),
        0,
        "new inventory discards obsolete pending readings"
    );
}

#[test]
fn observations_only_leave_an_established_session_and_never_enter_its_outbox() {
    let mut runtime = KnotProtocolRuntime::new(
        "knot".into(),
        KnotCapabilities::default(),
        RetryPolicy::default(),
    );
    runtime.start(session(), time(0), 0).unwrap();
    assert_eq!(
        runtime.observation_envelope(inventory(1, &[1]), EventId::from_bytes([2; 16]), time(1)),
        Err(KnotProtocolRuntimeError::UnexpectedMessage)
    );
    let ack = EventEnvelope::new_with_id(
        EventId::from_bytes([2; 16]),
        ArkSyncActor::Hub { hub_id: [3; 16] },
        time(2),
        KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Hello {
            event_id: session(),
            config: KnotConfig {
                version: 1,
                knot_id: [4; 16],
                sensor_bindings: vec![],
                actuator_configs: vec![],
            },
        })),
    );
    runtime
        .receive(&ack, EventId::from_bytes([5; 16]), time(3), 3)
        .unwrap();
    let pending = runtime.pending_message_count();
    let envelope = runtime
        .observation_envelope(inventory(1, &[1]), EventId::from_bytes([6; 16]), time(4))
        .unwrap();
    assert!(!envelope.payload.requires_ack());
    assert_eq!(runtime.pending_message_count(), pending);
    assert!(runtime
        .publish(envelope.payload, envelope.id, time(4), 4)
        .is_err());
    runtime.close();
    assert!(runtime
        .observation_envelope(inventory(1, &[1]), EventId::from_bytes([7; 16]), time(5))
        .is_err());
}

#[test]
fn periodic_inventory_repairs_a_lost_snapshot_without_accepting_orphan_measurements() {
    let mut publisher = KnotObservationPublisher::new(session(), 1);
    let mut receiver = KnotObservationCursor::new(session(), 1);
    publisher.publish_at(&inventory(1, &[1]), time(1)).unwrap();
    let (_, lost) = publisher.next_message(0).unwrap();
    publisher.mark_sent(&lost, 0); // transport accepted, receiver never got it
    publisher.publish_at(&sample(1, 1, 1), time(2)).unwrap();
    let (_, orphan) = publisher.next_message(1).unwrap();
    publisher.mark_sent(&orphan, 1);
    assert_eq!(
        receiver.accept(&orphan),
        Err(KnotObservationError::MissingInventory)
    );
    let (_, repeated) = publisher
        .next_message(DEFAULT_INVENTORY_INTERVAL_MS)
        .unwrap();
    receiver.accept(&repeated).unwrap();
    publisher.mark_sent(&repeated, DEFAULT_INVENTORY_INTERVAL_MS);
    publisher.publish_at(&sample(1, 1, 2), time(3)).unwrap();
    receiver
        .accept(
            &publisher
                .next_message(DEFAULT_INVENTORY_INTERVAL_MS + 1)
                .unwrap()
                .1,
        )
        .unwrap();
}

#[test]
fn due_control_messages_have_priority_over_queued_durable_sensor_batches() {
    let mut outbox = arksync_knot::application::KnotOutbox::new(RetryPolicy::default());
    let source = ArkSyncActor::Knot {
        hardware_uid: "knot".into(),
    };
    let sensor_id = EventId::from_bytes([4; 16]);
    let control_id = EventId::from_bytes([5; 16]);
    outbox
        .enqueue(
            EventEnvelope::new_with_id(
                sensor_id,
                source.clone(),
                time(0),
                KnotMessage::Sensor(KnotSensorMessage::Measurements(
                    KnotSensorMeasurementBatch {
                        measurements: vec![],
                    },
                )),
            ),
            0,
        )
        .unwrap();
    outbox
        .enqueue(
            EventEnvelope::new_with_id(
                control_id,
                source,
                time(1),
                KnotMessage::Control(KnotControlMessage::Heartbeat),
            ),
            0,
        )
        .unwrap();
    assert_eq!(outbox.next_due_message(0).unwrap().id, control_id);
    let due = outbox.due_messages(0);
    assert_eq!(
        due.iter().map(|message| message.id).collect::<Vec<_>>(),
        vec![control_id, sensor_id]
    );
}
