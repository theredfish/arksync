// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{
    EventEnvelope, EventHandler, EventHandlerError, EventId, EventRouter, PostcardDecode,
    PostcardEncode, Timestamp,
};
use arksync_sensor::infrastructure::events::{SensorEvent, SerialSensorPlugged};
use arksync_sensor::serial_port::{SerialPortMetadata, DEFAULT_BAUD_RATE};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Sender};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TestSource {
    Knot { label: String },
}

type SensorEventEnvelope = EventEnvelope<SensorEvent, TestSource>;
type SensorEventChannel = Channel<CriticalSectionRawMutex, SensorEventEnvelope, 1>;

static SENSOR_EVENTS: SensorEventChannel = Channel::new();
static KNOT_EVENTS: SensorEventChannel = Channel::new();

struct ChannelHandler(Sender<'static, CriticalSectionRawMutex, SensorEventEnvelope, 1>);

impl EventHandler<SensorEvent, TestSource> for ChannelHandler {
    fn handle(&mut self, event: &SensorEventEnvelope) -> Result<(), EventHandlerError> {
        self.0
            .try_send(event.clone())
            .map_err(|_| EventHandlerError::Rejected)
    }
}

struct TokioHandler(tokio::sync::mpsc::Sender<SensorEventEnvelope>);

impl EventHandler<SensorEvent, TestSource> for TokioHandler {
    fn handle(&mut self, event: &SensorEventEnvelope) -> Result<(), EventHandlerError> {
        self.0
            .try_send(event.clone())
            .map_err(|_| EventHandlerError::Rejected)
    }
}

fn sensor_event() -> SensorEventEnvelope {
    EventEnvelope::new_with_id(
        EventId::from_bytes([1; 16]),
        TestSource::Knot {
            label: "rtd-knot".to_string(),
        },
        Timestamp::from_unix_millis(1_779_840_000_000),
        SensorEvent::SerialSensorPlugged(SerialSensorPlugged {
            metadata: SerialPortMetadata {
                port_name: "/dev/ttyUSB0".to_string(),
                serial_number: "rtd-serial-1".to_string(),
                baud_rate: DEFAULT_BAUD_RATE,
            },
        }),
    )
}

#[test]
fn sends_sensor_event_through_embassy_local_event_router() {
    let mut router = EventRouter::new();
    router.subscribe_where(
        |event: &SensorEventEnvelope| {
            matches!(
                event.payload,
                SensorEvent::SerialSensorPlugged(SerialSensorPlugged { .. })
            )
        },
        ChannelHandler(SENSOR_EVENTS.sender()),
    );
    let event = sensor_event();

    let report = router.publish(&event);
    let received = SENSOR_EVENTS.receiver().try_receive().unwrap();

    assert_eq!(report.delivered, 1);
    assert_eq!(received, event);
}

#[tokio::test]
async fn sends_sensor_event_through_tokio_local_event_router() {
    let (hub_tx, mut hub_rx) = tokio::sync::mpsc::channel(1);
    let mut router = EventRouter::new();
    router.subscribe_where(
        |event: &SensorEventEnvelope| {
            matches!(
                event.payload,
                SensorEvent::SerialSensorPlugged(SerialSensorPlugged { .. })
            )
        },
        TokioHandler(hub_tx),
    );
    let event = sensor_event();

    let report = router.publish(&event);
    let received = hub_rx.recv().await.unwrap();

    assert_eq!(report.delivered, 1);
    assert_eq!(received, event);
}

#[test]
fn postcard_round_trips_sensor_event_envelope() {
    let event = sensor_event();
    let mut buffer = [0; 256];

    let encoded = event.encode_postcard(&mut buffer).unwrap();
    let decoded = SensorEventEnvelope::decode_postcard(encoded).unwrap();

    assert_eq!(decoded, event);
}

#[tokio::test]
async fn bridges_embassy_knot_channel_to_tokio_hub_channel() {
    let (hub_tx, mut hub_rx) = tokio::sync::mpsc::channel(1);
    let bridge = tokio::spawn(async move {
        let event = KNOT_EVENTS.receiver().receive().await;
        hub_tx
            .send(event)
            .await
            .map_err(|_| EventHandlerError::Rejected)
    });

    let mut knot_router = EventRouter::new();
    knot_router.subscribe_where(
        |event: &SensorEventEnvelope| {
            matches!(
                event.payload,
                SensorEvent::SerialSensorPlugged(SerialSensorPlugged { .. })
            )
        },
        ChannelHandler(KNOT_EVENTS.sender()),
    );
    let event = sensor_event();

    let report = knot_router.publish(&event);
    let received = hub_rx.recv().await.unwrap();
    let bridged = bridge.await.unwrap();

    assert_eq!(report.delivered, 1);
    assert_eq!(received, event);
    assert_eq!(bridged, Ok(()));
}
