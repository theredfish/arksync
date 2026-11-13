// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_event_router::EventId;
use arksync_protocol::knot::{
    KnotAck, KnotMessage, KnotPayload, KnotRejectionReason, KnotSensorMeasurementBatch,
    KnotSensorMessage,
};

#[test]
fn only_messages_that_expect_processing_are_acknowledged() {
    let event_id = EventId::from_bytes([1; 16]);

    assert!(!KnotMessage::Ack(KnotAck::Processed { event_id }).requires_ack());
    assert!(
        !KnotMessage::Non(KnotPayload::Sensor(KnotSensorMessage::Measurements(
            KnotSensorMeasurementBatch {
                measurements: vec![]
            }
        )))
        .requires_ack()
    );
    assert!(
        KnotMessage::Con(KnotPayload::Sensor(KnotSensorMessage::Measurements(
            KnotSensorMeasurementBatch {
                measurements: vec![],
            }
        )))
        .requires_ack()
    );
}

#[test]
fn rejection_reason_defines_retryability() {
    assert!(KnotRejectionReason::TemporarilyUnavailable.is_retryable());
    assert!(!KnotRejectionReason::ConfigurationConflict.is_retryable());
}
