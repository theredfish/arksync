// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use chrono::Utc;
use std::sync::Mutex;

use crate::core::temperature::unit::{CelsiusUnit, Unit};
use crate::core::temperature::DynamicRange;

use crate::ezo::driver::{uart::UartDriver, Driver};
use crate::ezo::ezo_sensor::EzoSensor;
use crate::sensor::{SensorInfo, SensorName, SensorState, SensorStateReason};

const RTD_DISCONNECTED_VALUE: f64 = -1023.0;
const RTD_DISCONNECTED_EPSILON: f64 = 0.001;

pub struct Rtd<D: Driver> {
    data: Mutex<SensorInfo>,
    driver: Mutex<D>,
    temperature_unit: Unit,
}

impl<D: Driver + Send + 'static> EzoSensor for Rtd<D> {
    type DriverType = D;

    fn data(&self) -> &Mutex<SensorInfo> {
        &self.data
    }

    fn driver(&self) -> &Mutex<Self::DriverType> {
        &self.driver
    }

    fn data_range(&self) -> DynamicRange {
        match self.temperature_unit {
            Unit::Celsius(_) => DynamicRange::celsius(-126.0..1254.0),
            Unit::Fahrenheit(_) => DynamicRange::fahrenheit(-194.8..2289.2),
            Unit::Kelvin(_) => DynamicRange::kelvin(147.15..1527.15),
        }
    }

    fn check_measurement(&self, value: f64) -> Option<SensorStateReason> {
        if (value - RTD_DISCONNECTED_VALUE).abs() <= RTD_DISCONNECTED_EPSILON {
            return Some(SensorStateReason::InvalidMeasurement(value));
        }

        let (min, max) = match self.temperature_unit {
            Unit::Celsius(_) => (-126.0, 1254.0),
            Unit::Fahrenheit(_) => (-194.8, 2289.2),
            Unit::Kelvin(_) => (147.15, 1527.15),
        };

        if value < min || value > max {
            return Some(SensorStateReason::OutOfRange { value, min, max });
        }

        None
    }
}

impl<D: Driver> Rtd<D> {
    pub fn new(driver: D, firmware: f64) -> Self {
        let now = Utc::now();
        Self {
            data: Mutex::new(SensorInfo {
                firmware,
                name: SensorName::Unnamed,
                state: SensorState::Initializing,
                state_reason: SensorStateReason::Plugged,
                state_since: now,
                last_activity: now,
                consecutive_failures: 0,
                connection: driver.connection_info(),
            }),
            driver: Mutex::new(driver),
            temperature_unit: Unit::Celsius(CelsiusUnit),
        }
    }
}

impl Rtd<UartDriver> {
    pub fn from_uart(driver: UartDriver, firmware: f64) -> Self {
        Self::new(driver, firmware)
    }
}
