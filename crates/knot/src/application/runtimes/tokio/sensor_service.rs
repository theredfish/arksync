// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::domain::KnotEventSource;
use arksync_bus::{EventBus, EventBusError, EventEnvelope, EventHandler, EventId, Timestamp};
use arksync_sensor::infrastructure::events::SensorEvent;
use arksync_sensor::services::SensorService;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

pub type KnotSensorEventEnvelope = EventEnvelope<SensorEvent, KnotEventSource>;

/// Knot sensor service used by std runtimes such as the hub runtime.
///
/// This adapter deliberately delegates to the existing arksync-sensor service
/// so the current UART detection and polling path keeps working while the Knot
/// becomes the owner of launching sensor logic.
pub struct TokioKnotSensorService {
    event_tx: mpsc::Sender<KnotSensorEventEnvelope>,
    source: KnotEventSource,
}

impl TokioKnotSensorService {
    pub fn new(source: KnotEventSource) -> Self {
        Self {
            event_tx: mpsc::channel(1).0,
            source,
        }
    }

    pub fn with_event_sender(
        event_tx: mpsc::Sender<KnotSensorEventEnvelope>,
        source: KnotEventSource,
    ) -> Self {
        Self { event_tx, source }
    }

    pub async fn run(self) {
        arksync_sensor::device_uid::rng::init_from_os_rng();
        let (sensor_event_tx, mut sensor_event_rx) = mpsc::channel(100);
        let mut sensor_bus = EventBus::new();
        sensor_bus.subscribe(TokioSensorEventHandler(sensor_event_tx));
        let sensor_service = SensorService::new().with_event_producer(sensor_bus.producer());
        let event_tx = self.event_tx;
        let source = self.source;
        let bridge = tokio::spawn(async move {
            let mut event_counter = 0_u128;
            let mut envelope_bus = EventBus::new();
            envelope_bus.subscribe(TokioEnvelopeHandler(event_tx));

            while let Some(sensor_envelope) = sensor_event_rx.recv().await {
                event_counter = event_counter.wrapping_add(1);
                log::debug!("Local Knot produced sensor event: {sensor_envelope:?}");

                let envelope = EventEnvelope::new_with_id(
                    event_id_from_counter(event_counter),
                    source.clone(),
                    timestamp_now(),
                    sensor_envelope.payload,
                );

                if envelope_bus.producer().publish(envelope).is_err() {
                    log::debug!("Local Knot sensor event receiver dropped");
                    break;
                }
            }
        });

        sensor_service.run().await;
        bridge.abort();
    }
}

fn event_id_from_counter(counter: u128) -> EventId {
    EventId::from_bytes(counter.to_be_bytes())
}

fn timestamp_now() -> Timestamp {
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Timestamp::from_unix_millis(unix_millis)
}

struct TokioEnvelopeHandler(mpsc::Sender<KnotSensorEventEnvelope>);

impl EventHandler<SensorEvent, KnotEventSource> for TokioEnvelopeHandler {
    fn handle(&mut self, event: KnotSensorEventEnvelope) -> Result<(), EventBusError> {
        self.0
            .try_send(event)
            .map_err(|_| EventBusError::HandlerRejected)
    }
}

struct TokioSensorEventHandler(mpsc::Sender<EventEnvelope<SensorEvent>>);

impl EventHandler<SensorEvent> for TokioSensorEventHandler {
    fn handle(&mut self, event: EventEnvelope<SensorEvent>) -> Result<(), EventBusError> {
        self.0
            .try_send(event)
            .map_err(|_| EventBusError::HandlerRejected)
    }
}
