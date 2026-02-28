use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};

use crate::ezo::driver::DriverError;
use crate::ezo::driver::{CommandTransport, Driver};
use crate::ezo::error::{Result, SensorError};

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

pub trait EzoSensor: Send + Sync + 'static {
    type DriverType: Driver;

    fn data(&self) -> &SensorInfo;
    fn driver(&self) -> &Mutex<Self::DriverType>;

    /// Measurement command for this sensor.
    fn measurement_command(&self) -> &'static [u8] {
        b"R"
    }

    /// EZO measurement command (`R`) parsed as `f64`.
    fn read_measurement(&self) -> Result<f64> {
        let response = self.read_command_response(self.measurement_command())?;

        response
            .trim()
            .parse::<f64>()
            .map_err(|err| SensorError::Driver(DriverError::Read(err.to_string())))
    }

    fn read_command_response(&self, command: &[u8]) -> Result<String> {
        let mut driver = self
            .driver()
            .lock()
            .map_err(|err| SensorError::Driver(DriverError::Read(err.to_string())))?;

        driver.send_command(command).map_err(Into::into)
    }
}

impl<T> Sensor for T
where
    T: EzoSensor,
{
    fn info(&self) -> &SensorInfo {
        EzoSensor::data(self)
    }

    fn read_measurement(&self) -> Result<f64> {
        EzoSensor::read_measurement(self)
    }
}
