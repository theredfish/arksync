use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};

use crate::error::Result;

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

#[derive(Debug, Clone)]
pub struct SensorInfo {
    pub firmware: f64,
    pub name: SensorName,
    pub state: SensorState,
    pub last_activity: DateTime<Utc>,
}

pub trait Sensor: Send + Sync + 'static {
    fn info(&self) -> &SensorInfo;
    fn read_measurement(&self) -> Result<f64>;

    /// Spawn the main background task for this sensor.
    fn run(self: Arc<Self>) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(1500));

            loop {
                ticker.tick().await;

                match self.read_measurement() {
                    Ok(value) => println!("Sensor reading: {value:.3}"),
                    Err(err) => eprintln!("Sensor read error: {err}"),
                }
            }
        })
    }
}
