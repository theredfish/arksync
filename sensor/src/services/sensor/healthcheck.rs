use tokio::sync::mpsc::Sender;
use tokio::time::{interval, Duration};
use tokio_util::sync::CancellationToken;

use crate::services::sensor::SensorServiceCmd;

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
/// sensor and its last_activity before marking it as unhealthy or degraded.
/// This will help to sort sensors, have a backoff mechanism to check their
/// health again and avoid flooding the alert system.
pub async fn healthcheck(_cmd_tx: &Sender<SensorServiceCmd>, shutdown: CancellationToken) {
    let mut interval = interval(Duration::from_secs(15));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                println!("Health check: Sensor service is alive");
            }
            _ = shutdown.cancelled() => {
                println!("Health check: stopping");
                break;
            }
        }
    }
}
