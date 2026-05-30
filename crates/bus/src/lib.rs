// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Shared EventBus infrastructure between ArkSync bounded contexts.
//!
//! Bounded contexts own their event payload definitions. This crate owns the
//! generic envelope, subscription, handler, and codec mechanics used to move
//! those events.

#![no_std]

extern crate alloc;

mod bus;
mod event;
mod ids;
pub mod postcard;

pub use bus::*;
pub use event::*;
pub use ids::*;
pub use postcard::*;

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::{String, ToString};
    use serde::{Deserialize, Serialize};

    fn timestamp(unix_millis: i64) -> Timestamp {
        Timestamp::from_unix_millis(unix_millis)
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum TestEvent {
        Observed { label: String },
        Ignored,
    }

    #[test]
    fn postcard_round_trips_event_envelope() {
        let event = EventEnvelope::new_with_id(
            EventId::new_with_random_bytes([1; 16]),
            "knot:test".to_string(),
            timestamp(1_779_840_000_000),
            TestEvent::Observed {
                label: "rtd".to_string(),
            },
        );
        let mut buffer = [0; 128];

        let encoded = event.encode_postcard(&mut buffer).unwrap();
        let decoded: EventEnvelope<TestEvent, String> =
            EventEnvelope::decode_postcard(encoded).unwrap();

        assert_eq!(decoded, event);
    }

    #[test]
    fn event_bus_delivers_matching_events() {
        let mut bus = EventBus::new();
        bus.subscribe_where(
            |event: &EventEnvelope<TestEvent>| matches!(event.payload, TestEvent::Observed { .. }),
            |_event: EventEnvelope<TestEvent>| Ok(()),
        );
        let event = EventEnvelope::new_with_id(
            EventId::new_with_random_bytes([1; 16]),
            (),
            timestamp(1_779_840_000_000),
            TestEvent::Observed {
                label: "rtd".to_string(),
            },
        );

        let delivered = bus.publish(event).unwrap();

        assert_eq!(delivered, 1);
    }

    #[test]
    fn event_bus_skips_filtered_events() {
        let mut bus = EventBus::new();
        bus.subscribe_where(
            |event: &EventEnvelope<TestEvent>| matches!(event.payload, TestEvent::Observed { .. }),
            |_event: EventEnvelope<TestEvent>| Ok(()),
        );
        let event = EventEnvelope::new_with_id(
            EventId::new_with_random_bytes([1; 16]),
            (),
            timestamp(1_779_840_000_000),
            TestEvent::Ignored,
        );

        let delivered = bus.publish(event).unwrap();

        assert_eq!(delivered, 0);
    }

    #[test]
    fn event_bus_delivers_all_events_with_unit_filter() {
        let mut bus = EventBus::new();
        bus.subscribe(|_event: EventEnvelope<TestEvent>| Ok(()));
        let event = EventEnvelope::new_with_id(
            EventId::new_with_random_bytes([1; 16]),
            (),
            timestamp(1_779_840_000_000),
            TestEvent::Ignored,
        );

        let delivered = bus.publish(event).unwrap();

        assert_eq!(delivered, 1);
    }

    #[test]
    fn postcard_reports_buffer_too_small() {
        let event = EventEnvelope::new_with_id(
            EventId::new_with_random_bytes([1; 16]),
            (),
            timestamp(1_779_840_000_000),
            TestEvent::Observed {
                label: "rtd".to_string(),
            },
        );
        let mut buffer = [0; 1];

        let result = event.encode_postcard(&mut buffer);

        assert!(matches!(result, Err(postcard::Error::SerializeBufferFull)));
    }

    #[cfg(feature = "uuid-v4")]
    #[test]
    fn event_envelope_new_generates_event_id() {
        let event = EventEnvelope::new((), timestamp(1_779_840_000_000), TestEvent::Ignored);

        assert_eq!(event.id.as_uuid().get_version_num(), 4);
    }
}
