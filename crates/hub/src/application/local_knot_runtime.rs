// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{persist_sensor_measurement, Hub};
use arksync_bus::Timestamp;
use arksync_knot::application::LocalKnotSensorEventEnvelope;
use arksync_knot::application::LocalKnotSensorService;
use arksync_knot::domain::{KnotEventSource, KnotId, ParentHubId};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct LocalKnotRuntime;

impl LocalKnotRuntime {
    pub async fn run() {
        let (event_tx, mut event_rx) =
            tokio::sync::mpsc::channel::<LocalKnotSensorEventEnvelope>(100);
        let source = local_knot_source();
        let knot = tokio::spawn(async move {
            LocalKnotSensorService::with_event_sender(event_tx, source)
                .run()
                .await;
        });
        let mut hub = Hub::new();

        while let Some(event) = event_rx.recv().await {
            log::debug!("Hub received local Knot sensor event: {event:?}");
            let received_at = timestamp_now();

            match persist_sensor_measurement(arksync_db::pool(), &event, received_at).await {
                Ok(Some(measurement)) => {
                    log::info!(
                        "Hub persisted sensor measurement hardware_uid={} value={}",
                        measurement.hardware_uid,
                        measurement.value
                    );
                }
                Ok(None) => {}
                Err(err) => {
                    log::error!("Hub failed to persist sensor measurement: {err:?}");
                }
            }

            if let Err(err) = hub.accept_sensor_event(event, received_at) {
                log::error!("Hub rejected local Knot sensor event: {err:?}");
                continue;
            }

            log::debug!("Hub projected local Knot sensor event");
        }

        let _ = knot.await;
    }
}

fn local_knot_source() -> KnotEventSource {
    // TODO: Replace these MVP constants with a provisioned identity bundle.
    // The hub install flow should expose an admin CLI/program such as
    // `sk init hub` that authenticates the station admin, generates or loads
    // the HubId + local KnotId, signs them with a certificate, and stores the
    // resulting identity bundle for the runtime to load at boot.
    const LOCAL_PARENT_HUB_ID_RANDOM_BYTES: [u8; 16] = [1; 16];
    const LOCAL_KNOT_ID_RANDOM_BYTES: [u8; 16] = [2; 16];

    KnotEventSource::Knot {
        parent_hub_id: ParentHubId::new_with_random_bytes(LOCAL_PARENT_HUB_ID_RANDOM_BYTES),
        knot_id: KnotId::new_with_random_bytes(LOCAL_KNOT_ID_RANDOM_BYTES),
    }
}

fn timestamp_now() -> Timestamp {
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Timestamp::from_unix_millis(unix_millis)
}
