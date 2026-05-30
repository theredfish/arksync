// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::Hub;
use arksync_bus::Timestamp;
use arksync_knot::application::LocalKnotSensorEventEnvelope;
use arksync_knot::application::LocalKnotSensorService;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct LocalKnotRuntime;

impl LocalKnotRuntime {
    pub async fn run() {
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel::<LocalKnotSensorEventEnvelope>(100);
        let knot = tokio::spawn(async move {
            LocalKnotSensorService::with_event_sender(event_tx)
                .run()
                .await;
        });
        let mut hub = Hub::new();

        while let Some(event) = event_rx.recv().await {
            log::debug!("Hub received local Knot sensor event: {event:?}");

            if let Err(err) = hub.accept_sensor_event(event, timestamp_now()) {
                log::error!("Hub rejected local Knot sensor event: {err:?}");
                continue;
            }

            log::info!("Hub fake-persisted local Knot sensor event projection");
        }

        let _ = knot.await;
    }
}

fn timestamp_now() -> Timestamp {
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Timestamp::from_unix_millis(unix_millis)
}
