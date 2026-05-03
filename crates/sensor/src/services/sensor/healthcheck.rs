// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use chrono::Utc;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;

use crate::sensor::SensorState;
use crate::services::sensor::SensorServiceCmd;

const HEALTHCHECK_INTERVAL_SECS: u64 = 15;

/// The sensor service healthcheck.
///
/// This healthcheck is in charge of monitoring registered sensors and alerting
/// when some of them are unhealthy. Since sensors themselves can't say when they
/// are unhealthy we need an external way to detect it.
///
/// However sensors also have their own healthcheck via the status command and
/// are in charge of updating their state. This offers different healthcheck
/// layers.
///
/// The service healthcheck will mainly focus on checking the state of the
/// sensor and its last_activity before deciding when it should be removed from
/// the registry.
///
/// Note: Unplugged sensors are removed immediately by the unplugged sensor
/// detector, but this healthcheck serves as a safety net to catch any edge cases
/// where sensors remain in an Unplugged state without being removed.
pub async fn healthcheck(cmd_tx: &Sender<SensorServiceCmd>, shutdown: CancellationToken) {
    let mut interval = interval(Duration::from_secs(HEALTHCHECK_INTERVAL_SECS));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let (respond_to, rx) = oneshot::channel();
                if cmd_tx
                    .send(SensorServiceCmd::AllSensors { respond_to })
                    .await
                    .is_err()
                {
                    eprintln!("Health check: sensor registry is unavailable");
                    continue;
                }

                let Ok(sensors) = rx.await else {
                    eprintln!("Health check: failed to receive sensor snapshot");
                    continue;
                };

                let now = Utc::now();
                let mut sensors_to_remove = Vec::new();

                for (uuid, sensor) in sensors.iter() {
                    let info = sensor.info();
                    let state_age = now.signed_duration_since(info.state_since).num_seconds();
                    let inactivity = now.signed_duration_since(info.last_activity).num_seconds();

                    println!(
                        "Health check: sensor {uuid} state={:?} reason={:?} state_age={}s inactivity={}s failures={}",
                        info.state,
                        info.state_reason,
                        state_age,
                        inactivity,
                        info.consecutive_failures
                    );

                    if info.state == SensorState::Unplugged {
                        sensors_to_remove.push(uuid.clone())
                    }
                }

                if !sensors_to_remove.is_empty() {
                    println!(
                        "Health check: removing stale sensors {:?}",
                        sensors_to_remove
                    );
                    let _ = cmd_tx
                        .send(SensorServiceCmd::RemoveSensors {
                            uuids: sensors_to_remove,
                        })
                        .await;
                }
            }
            _ = shutdown.cancelled() => {
                println!("Health check: stopping");
                break;
            }
        }
    }
}
