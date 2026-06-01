// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use chrono::Utc;
use std::sync::Mutex;

use crate::core::temperature::DynamicRange;
use crate::error::{Result, SensorError};
use crate::ezo::driver::DriverError;
use crate::ezo::driver::{CommandTransport, Driver};
use crate::sensor::{
    generated_device_uid_from_info, is_arksync_device_uid, Sensor, SensorInfo, SensorState,
    SensorStateReason,
};

const UNREACHABLE_FAILURE_THRESHOLD: u32 = 3;

pub trait EzoSensor: Send + Sync + 'static {
    type DriverType: Driver;

    fn data(&self) -> &Mutex<SensorInfo>;
    fn driver(&self) -> &Mutex<Self::DriverType>;
    fn data_range(&self) -> DynamicRange;

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

    fn check_measurement(&self, _value: f64) -> Option<SensorStateReason> {
        None
    }

    fn ensure_device_uid(&self) -> Result<String> {
        let info = self
            .data()
            .lock()
            .map_err(|err| SensorError::message(err.to_string()))?
            .clone();

        if let Some(device_uid) = info.device_uid {
            return Ok(device_uid);
        }

        let mut driver = self
            .driver()
            .lock()
            .map_err(|err| SensorError::source(DriverError::Read(err.to_string())))?;

        match driver.device_name().map_err(SensorError::source)? {
            Some(device_uid) if is_arksync_device_uid(&device_uid) => {
                self.record_device_uid(&device_uid);
                Ok(device_uid)
            }
            Some(rejected_name) => Err(SensorError::message(format!(
                "sensor name '{rejected_name}' is not an ArkSync device UID"
            ))),
            None => {
                let device_uid = generated_device_uid_from_info(&info);
                driver
                    .set_device_name(&device_uid)
                    .map_err(SensorError::source)?;
                let confirmed = driver.device_name().map_err(SensorError::source)?;

                if confirmed.as_deref() != Some(device_uid.as_str()) {
                    return Err(SensorError::message(format!(
                        "failed to confirm ArkSync device UID '{device_uid}'"
                    )));
                }

                self.record_device_uid(&device_uid);
                Ok(device_uid)
            }
        }
    }

    fn record_device_uid(&self, device_uid: &str) {
        let mut data = self.data().lock().expect("sensor info mutex poisoned");
        data.device_uid = Some(device_uid.to_string());
    }
}

impl<T> Sensor for T
where
    T: EzoSensor,
{
    fn info(&self) -> SensorInfo {
        self.data()
            .lock()
            .expect("sensor info mutex poisoned")
            .clone()
    }

    fn read_measurement(&self) -> Result<f64> {
        EzoSensor::read_measurement(self)
    }

    fn check_measurement(&self, value: f64) -> Option<SensorStateReason> {
        EzoSensor::check_measurement(self, value)
    }

    fn ensure_device_uid(&self) -> Result<String> {
        EzoSensor::ensure_device_uid(self)
    }

    fn record_measurement(&self, value: f64) {
        let mut data = self.data().lock().expect("sensor info mutex poisoned");
        let now = Utc::now();

        if let Some(reason) = EzoSensor::check_measurement(self, value) {
            data.consecutive_failures += 1;
            let next_state = if data.consecutive_failures >= UNREACHABLE_FAILURE_THRESHOLD {
                SensorState::Unreachable
            } else {
                SensorState::Degraded
            };

            if data.state != next_state {
                data.state_since = now;
            }
            data.state = next_state;
            data.state_reason = reason;
            return;
        }

        data.last_activity = now;

        if !matches!(data.state, SensorState::Active) {
            data.state_since = now;
        }
        data.state = SensorState::Active;
        data.state_reason = SensorStateReason::MeasurementOk;
        data.consecutive_failures = 0;
    }

    fn record_error(&self, err: &SensorError) {
        let mut data = self.data().lock().expect("sensor info mutex poisoned");
        let now = Utc::now();
        data.consecutive_failures += 1;

        let next_state = if data.consecutive_failures >= UNREACHABLE_FAILURE_THRESHOLD {
            SensorState::Unreachable
        } else {
            SensorState::Degraded
        };

        if data.state != next_state {
            data.state_since = now;
        }

        data.state = next_state;
        data.state_reason = SensorStateReason::ReadError(err.to_string());
    }

    fn mark_unplugged(&self) {
        let mut data = self.data().lock().expect("sensor info mutex poisoned");
        if data.state != SensorState::Unplugged {
            data.state = SensorState::Unplugged;
            data.state_reason = SensorStateReason::Unplugged;
            data.state_since = Utc::now();
        }
    }
}
