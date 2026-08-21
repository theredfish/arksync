// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::{EventEnvelope, EventHandler, EventHandlerError, EventRouter};
use arksync_sensor::infrastructure::events::SensorEvent;
use arksync_sensor::services::SensorService;
use tokio::sync::mpsc;

pub type KnotSensorEventEnvelope = EventEnvelope<SensorEvent>;

/// Knot sensor service used by std runtimes such as the hub runtime.
///
/// This adapter deliberately delegates to the existing arksync-sensor service
/// so the current UART detection and polling path keeps working while the Knot
/// becomes the owner of launching sensor logic.
pub struct TokioKnotSensorService {
    event_tx: mpsc::Sender<KnotSensorEventEnvelope>,
}

impl TokioKnotSensorService {
    pub fn new() -> Self {
        Self {
            event_tx: mpsc::channel(1).0,
        }
    }

    pub fn with_event_sender(event_tx: mpsc::Sender<KnotSensorEventEnvelope>) -> Self {
        Self { event_tx }
    }

    pub async fn run(self) {
        arksync_sensor::device_uid::rng::init_from_os_rng();
        let mut sensor_router = EventRouter::new();
        sensor_router.subscribe(TokioSensorEventHandler(self.event_tx));
        let sensor_service = SensorService::new().with_event_publisher(sensor_router.publisher());

        sensor_service.run().await;
    }
}

struct TokioSensorEventHandler(mpsc::Sender<EventEnvelope<SensorEvent>>);

impl EventHandler<SensorEvent> for TokioSensorEventHandler {
    fn handle(&mut self, event: &EventEnvelope<SensorEvent>) -> Result<(), EventHandlerError> {
        self.0
            .try_send(event.clone())
            .map_err(|_| EventHandlerError::Rejected)
    }
}
