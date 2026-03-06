use chrono::Utc;
use std::fmt;
use std::sync::Mutex;

use crate::ezo::driver::{uart::UartDriver, Driver};
use crate::ezo::ezo_sensor::EzoSensor;
use crate::sensor::{SensorInfo, SensorName, SensorState};

pub struct Rtd<D: Driver> {
    data: SensorInfo,
    driver: Mutex<D>,
}

impl<D: Driver + Send + 'static> EzoSensor for Rtd<D> {
    type DriverType = D;

    fn data(&self) -> &SensorInfo {
        &self.data
    }

    fn driver(&self) -> &Mutex<Self::DriverType> {
        &self.driver
    }

    fn data_range(&self) -> (f32, f32) {
        (-126.0, 1254.0)
    }
}

impl<D: Driver> Rtd<D> {
    pub fn new(driver: D, firmware: f64) -> Self {
        Self {
            data: SensorInfo {
                firmware,
                name: SensorName::Unnamed,
                state: SensorState::Initializing,
                last_activity: Utc::now(),
                connection: driver.connection_info(),
            },
            driver: Mutex::new(driver),
        }
    }
}

impl Rtd<UartDriver> {
    pub fn from_uart(driver: UartDriver, firmware: f64) -> Self {
        Self::new(driver, firmware)
    }
}
