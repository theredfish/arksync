// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventBus, EventEnvelope, EventId, Timestamp};
use arksync_hub::{
    HubSensorEventEnvelope, HubService, RegisterSensor, RemoveSensor, RenameSensor, SensorId,
    SensorRegistrationStatus,
};
use arksync_knot::domain::{KnotEventSource, KnotId, ParentHubId};
use arksync_sensor::infrastructure::events::{SensorEvent, SerialSensorPlugged};
use arksync_sensor::serial_port::{SerialPortMetadata, DEFAULT_BAUD_RATE};

fn timestamp(unix_millis: i64) -> Timestamp {
    Timestamp::from_unix_millis(unix_millis)
}

fn metadata(serial_number: &str) -> SerialPortMetadata {
    SerialPortMetadata {
        port_name: "/dev/ttyUSB0".to_string(),
        serial_number: serial_number.to_string(),
        baud_rate: DEFAULT_BAUD_RATE,
    }
}

fn source() -> KnotEventSource {
    KnotEventSource::Knot {
        parent_hub_id: ParentHubId::from_bytes([1; 16]),
        knot_id: KnotId::from_bytes([2; 16]),
    }
}

fn serial_sensor_plugged(serial_number: &str, occurred_at: i64) -> HubSensorEventEnvelope {
    EventEnvelope::new_with_id(
        EventId::from_bytes([3; 16]),
        source(),
        timestamp(occurred_at),
        SensorEvent::SerialSensorPlugged(SerialSensorPlugged {
            metadata: metadata(serial_number),
        }),
    )
}

#[test]
fn ingests_serial_sensor_plugged_events_into_overview_projection() {
    let mut hub = HubService::new();

    hub.handle_sensor_event(
        serial_sensor_plugged("rtd-serial-1", 1_779_840_000_000),
        timestamp(1_779_840_000_100),
    )
    .unwrap();

    let overview = hub.list_sensors_overview();

    assert_eq!(overview.len(), 1);
    assert_eq!(overview[0].sensor_id, None);
    assert_eq!(overview[0].display_name, "rtd-serial-1");
    assert_eq!(overview[0].status, SensorRegistrationStatus::Discovered);
    assert_eq!(
        overview[0].last_observed_at,
        Some(timestamp(1_779_840_000_000))
    );
    assert_eq!(
        overview[0].last_received_at,
        Some(timestamp(1_779_840_000_100))
    );
}

#[test]
fn register_rename_and_remove_sensor_update_overview_read_model() {
    let sensor_id = SensorId::from_bytes([4; 16]);
    let mut hub = HubService::new();

    hub.handle_sensor_event(
        serial_sensor_plugged("rtd-serial-1", 1_779_840_000_000),
        timestamp(1_779_840_000_100),
    )
    .unwrap();
    hub.register_sensor(RegisterSensor {
        sensor_id,
        display_name: "Water temperature".to_string(),
        metadata: metadata("rtd-serial-1"),
        registered_at: timestamp(1_779_840_000_200),
    })
    .unwrap();

    let overview = hub.list_sensors_overview();
    assert_eq!(overview[0].sensor_id, Some(sensor_id));
    assert_eq!(overview[0].display_name, "Water temperature");
    assert_eq!(overview[0].status, SensorRegistrationStatus::Registered);

    hub.rename_sensor(RenameSensor {
        sensor_id,
        display_name: "Tank temperature".to_string(),
    })
    .unwrap();

    assert_eq!(
        hub.list_sensors_overview()[0].display_name,
        "Tank temperature"
    );

    hub.remove_sensor(RemoveSensor { sensor_id }).unwrap();

    let overview = hub.list_sensors_overview();
    assert_eq!(overview[0].sensor_id, None);
    assert_eq!(overview[0].status, SensorRegistrationStatus::Discovered);
}

#[test]
fn event_bus_handler_is_the_hub_ingestion_boundary() {
    let mut bus = EventBus::new();
    let mut hub = HubService::new();
    bus.subscribe(move |event: HubSensorEventEnvelope| {
        hub.handle_sensor_event(event, timestamp(1_779_840_000_100))
            .map_err(|_| arksync_bus::EventBusError::HandlerRejected)
    });

    let delivered = bus
        .producer()
        .publish(serial_sensor_plugged("rtd-serial-1", 1_779_840_000_000))
        .unwrap();

    assert_eq!(delivered, 1);
}
