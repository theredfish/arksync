use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};

use crate::error::Result;
use crate::i2c_bus::I2cConnection;
use crate::serial_port::SerialPortMetadata;

#[derive(Debug, Clone, Default)]
pub enum SensorName {
    #[default]
    Unnamed,
    Named(String),
}

#[derive(Default, Debug, Clone, Copy)]
pub enum SensorState {
    Active,
    Degraded,
    #[default]
    Initializing,
    Unreachable,
}

#[derive(Debug)]
pub enum SensorConnection {
    Uart(SerialPortMetadata),
    I2c(I2cConnection),
}

#[derive(Debug)]
pub struct SensorInfo {
    pub firmware: f64,
    pub name: SensorName,
    pub state: SensorState,
    pub last_activity: DateTime<Utc>,
    pub connection: SensorConnection,
}

pub trait Sensor: Send + Sync + 'static {
    fn info(&self) -> &SensorInfo;
    fn read_measurement(&self) -> Result<f64>;

    /// Spawn the main background task for this sensor.
    fn run(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            // This is based on Atlas Scientific read time, plus some time to not
            // be at the edge of the value disponibility
            let mut ticker = interval(Duration::from_millis(1200));

            loop {
                ticker.tick().await;

                // TODO: Retry with backoff strategy: we allow some I/O error but after a specific threshold we start to update
                // the state of the sensor to Degraded then Unresponsive.
                match self.read_measurement() {
                    Ok(value) => {
                        println!("Sensor reading: {value:.3}");
                        // TODO: If it's successful then we update the last activity so the supervisor healthcheck get an
                        // up-to-date information. As long as we can read, then we are active.
                    }
                    Err(err) => eprintln!("Sensor read error: {err:#?}"),
                }
            }
        })
    }
}
