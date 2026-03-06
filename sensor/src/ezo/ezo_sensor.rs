use std::sync::Mutex;

use crate::error::{Result, SensorError};
use crate::ezo::driver::DriverError;
use crate::ezo::driver::{CommandTransport, Driver};
use crate::sensor::{Sensor, SensorInfo};

pub trait EzoSensor: Send + Sync + 'static {
    type DriverType: Driver;

    fn data(&self) -> &SensorInfo;
    fn driver(&self) -> &Mutex<Self::DriverType>;
    fn data_range(&self) -> (f32, f32);

    /// Measurement command for this sensor.
    fn measurement_command(&self) -> &'static [u8] {
        b"R"
    }

    /// EZO measurement command (`R`) parsed as `f64`.
    fn read_measurement(&self) -> Result<f64> {
        let mut driver = self
            .driver()
            .lock()
            .map_err(|err| SensorError::source(DriverError::Read(err.to_string())))?;

        driver
            .send_command(self.measurement_command())
            .map_err(SensorError::source)?
            .trim()
            .parse::<f64>()
            .map_err(|err| SensorError::source(DriverError::Read(err.to_string())))
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
