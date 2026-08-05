// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration, Instant};

use crate::device_uid::DeviceUid;
use crate::error::{Result, SensorError};
use crate::i2c_bus::I2cConnection;
use crate::infrastructure::events::{
    MeasuredSensor, MeasurementUnit, SensorConnectionMetadata, SensorKind, SensorMeasurement,
    SensorMeasurementRecorded,
};
use crate::serial_port::SerialPortMetadata;

#[derive(Debug, Clone, Default)]
pub enum SensorName {
    #[default]
    Unnamed,
    Named(String),
}

pub const DEFAULT_MEASUREMENT_INTERVAL: Duration = Duration::from_millis(1200);

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
    pub device_uid: Option<DeviceUid>,
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
    fn ensure_device_uid(&self) -> Result<DeviceUid> {
        let info = self.info();

        Ok(info.device_uid.clone().unwrap_or_default())
    }

    fn record_measurement(&self, value: f64);
    fn record_error(&self, err: &SensorError);
    fn mark_unplugged(&self);

    /// Spawn the main background task for this sensor.
    fn run(
        self: Arc<Self>,
        measurement_tx: Option<mpsc::Sender<SensorMeasurementRecorded>>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            // This is based on Atlas Scientific read time, plus some time to not
            // be at the edge of the value disponibility
            let mut ticker = interval(DEFAULT_MEASUREMENT_INTERVAL);
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
                        if let Some(measurement_tx) = &measurement_tx {
                            let info = self.info();
                            if let Some(device_uid) = info.device_uid.clone() {
                                let _ = measurement_tx.try_send(SensorMeasurementRecorded {
                                    device_uid,
                                    sensor: measured_sensor_from_info(&info),
                                    measurement: SensorMeasurement {
                                        value,
                                        unit: measurement_unit_from_info(&info),
                                    },
                                });
                            }
                        }
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

pub(crate) fn measured_sensor_from_info(info: &SensorInfo) -> MeasuredSensor {
    match &info.connection {
        SensorConnection::Uart(metadata) => MeasuredSensor {
            hardware_uid: metadata.serial_number.clone(),
            kind: sensor_kind_from_info(info),
            connection: SensorConnectionMetadata::Uart(metadata.clone()),
            firmware: Some(info.firmware),
        },
        SensorConnection::I2c(connection) => MeasuredSensor {
            hardware_uid: format!("i2c:{:02x}", connection.address),
            kind: sensor_kind_from_info(info),
            connection: SensorConnectionMetadata::I2c {
                address: connection.address,
            },
            firmware: Some(info.firmware),
        },
    }
}

fn sensor_kind_from_info(_info: &SensorInfo) -> SensorKind {
    SensorKind::Temperature
}

fn measurement_unit_from_info(_info: &SensorInfo) -> MeasurementUnit {
    MeasurementUnit::Celsius
}
