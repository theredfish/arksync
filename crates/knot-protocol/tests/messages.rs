// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::EventId;
use arksync_knot_protocol::{
    KnotAck, KnotControlMessage, KnotMessage, KnotNackReason, KnotSensorMeasurementBatch,
    KnotSensorMessage,
};

#[test]
fn only_messages_that_expect_processing_are_acknowledged() {
    let event_id = EventId::from_bytes([1; 16]);

    assert!(
        !KnotMessage::Control(KnotControlMessage::Ack(KnotAck::Processed { event_id }))
            .requires_ack()
    );
    assert!(KnotMessage::Sensor(KnotSensorMessage::Measurements(
        KnotSensorMeasurementBatch {
            measurements: vec![],
        }
    ))
    .requires_ack());
}

#[test]
fn nack_reason_defines_retryability() {
    assert!(KnotNackReason::TemporarilyUnavailable.is_retryable());
    assert!(!KnotNackReason::ConfigurationConflict.is_retryable());
}
