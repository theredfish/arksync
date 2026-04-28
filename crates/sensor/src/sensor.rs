use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration, Instant};

use crate::error::{Result, SensorError};
use crate::i2c_bus::I2cConnection;
use crate::serial_port::SerialPortMetadata;

#[derive(Debug, Clone, Default)]
pub enum SensorName {
    #[default]
    Unnamed,
    Named(String),
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorState {
    Active,
    Degraded,
    #[default]
    Initializing,
    Unplugged,
    Unreachable,
}

#[derive(Debug, Clone)]
pub enum SensorStateReason {
    Plugged,
    Unplugged,
    MeasurementOk,
    InvalidMeasurement(f64),
    OutOfRange { value: f64, min: f64, max: f64 },
    ReadError(String),
    NoRecentActivity,
}

#[derive(Debug, Clone)]
pub enum SensorConnection {
    Uart(SerialPortMetadata),
    I2c(I2cConnection),
}

#[derive(Debug, Clone)]
pub struct SensorInfo {
    pub firmware: f64,
    pub name: SensorName,
    pub state: SensorState,
    pub state_reason: SensorStateReason,
    pub state_since: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub consecutive_failures: u32,
    pub connection: SensorConnection,
}

pub trait Sensor: Send + Sync + 'static {
    fn info(&self) -> SensorInfo;
    fn read_measurement(&self) -> Result<f64>;
    fn check_measurement(&self, value: f64) -> Option<SensorStateReason>;
    fn record_measurement(&self, value: f64);
    fn record_error(&self, err: &SensorError);
    fn mark_unplugged(&self);

    /// Spawn the main background task for this sensor.
    fn run(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            // This is based on Atlas Scientific read time, plus some time to not
            // be at the edge of the value disponibility
            let mut ticker = interval(Duration::from_millis(1200));
            let unreachable_retry_interval = Duration::from_secs(30);
            let mut last_unreachable_retry = Instant::now() - unreachable_retry_interval;

            loop {
                ticker.tick().await;

                let info = self.info();
                match info.state {
                    SensorState::Unplugged => continue,
                    SensorState::Unreachable => {
                        if last_unreachable_retry.elapsed() < unreachable_retry_interval {
                            continue;
                        }
                        last_unreachable_retry = Instant::now();
                    }
                    _ => {}
                }

                // TODO: Retry with backoff strategy: we allow some I/O error but after a specific threshold we start to update
                // the state of the sensor to Degraded then Unresponsive.
                match self.read_measurement() {
                    Ok(value) => {
                        self.record_measurement(value);
                        println!("Sensor reading: {value:.3}");
                    }
                    Err(err) => {
                        self.record_error(&err);
                        eprintln!("Sensor read error: {err:#?}");
                    }
                }
            }
        })
    }
}
