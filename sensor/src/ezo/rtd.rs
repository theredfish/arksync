use chrono::Utc;
use std::sync::Mutex;

use crate::ezo::sensor::*;
use crate::ezo::{
    driver::{uart::UartDriver, Driver},
    sensor::SensorData,
};

pub struct Rtd<D: Driver> {
    data: Mutex<SensorData>,
    driver: Mutex<D>,
}

impl Sensor for Rtd<UartDriver> {
    fn data(&self) -> SensorData {
        self.data.lock().unwrap().clone()
    }
}

impl Rtd<UartDriver> {
    pub fn from_uart(driver: UartDriver, firmware: f64) -> Self {
        Self {
            data: Mutex::new(SensorData {
                firmware,
                name: SensorName::Unnamed,
                state: SensorState::Initializing,
                last_activity: Utc::now(),
            }),
            driver: Mutex::new(driver),
        }
    }
}
