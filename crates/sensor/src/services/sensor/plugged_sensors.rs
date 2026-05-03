// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ezo::driver::uart::UartDriver;
use crate::ezo::driver::{DeviceType, Driver};
use crate::ezo::rtd::Rtd;
use crate::sensor::Sensor;
use crate::services::sensor::SensorServiceCmd;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::time::{interval, Duration as TokioDuration};
use tokio_util::sync::CancellationToken;

use crate::serial_port::{self, SerialPortMetadata};

/// Listen for plugged sensors.
///
/// Finds new USB sensors and adds them to registry.
pub async fn detect_plugged_sensors_task(
    cmd_tx: &Sender<SensorServiceCmd>,
    shutdown: CancellationToken,
) {
    let mut interval = interval(TokioDuration::from_secs(2));

    loop {
        tokio::select! {
            _ = interval.tick() => {}
            _ = shutdown.cancelled() => {
                println!("Detector: stopping plugged sensor scan");
                break;
            }
        }

        println!("Detector: Scanning for sensors...");
        let asc_ports = serial_port::find_asc_port();

        if !asc_ports.is_empty() {
            println!("Detector: Found {} ASC ports", asc_ports.len());
        }

        // Get current sensor list
        let (respond_to, rx) = oneshot::channel();
        let _ = cmd_tx
            .send(SensorServiceCmd::AllSensors { respond_to })
            .await;
        let current_sensors = rx.await;

        if let Ok(current_sensors) = current_sensors {
            let mut new_sensors: Vec<(String, Arc<dyn Sensor>)> = Vec::new();

            for port in asc_ports.iter() {
                if !current_sensors.contains_key(&port.serial_number) {
                    let sensor = create_sensor_from_port(port);
                    println!(
                        "Detector: Created sensor {}: {:#?}",
                        port.serial_number,
                        sensor.as_ref().map(|s| s.info())
                    );

                    match sensor {
                        Ok(sensor) => {
                            let data = sensor.info();
                            println!("Detector: Created sensor - firmware v{}", data.firmware);
                            new_sensors.push((port.serial_number.clone(), sensor));
                        }
                        Err(e) => {
                            eprintln!(
                                "Detector: Failed to create sensor {}: {}",
                                port.serial_number, e
                            );
                        }
                    }
                }
            }

            if !new_sensors.is_empty() {
                let _ = cmd_tx
                    .send(SensorServiceCmd::AddSensors {
                        sensors: new_sensors,
                    })
                    .await;
            }
        }
    }
}

/// Factory function to create a sensor from a serial port
///
/// This function:
/// 1. Creates a temporary UART driver
/// 2. Queries device info to determine sensor type
/// 3. Creates the appropriate sensor with the correct driver
/// 4. Returns it as Arc<dyn Sensor>
fn create_sensor_from_port(
    port: &SerialPortMetadata,
) -> Result<Arc<dyn Sensor>, Box<dyn std::error::Error>> {
    // Create temporary driver to query device type
    let mut uart_driver = UartDriver::new(port)?;
    let device_info = uart_driver.device_info()?;

    println!(
        "Factory: Detected {:?} sensor v{}",
        device_info.device_type, device_info.firmware_version
    );

    // Create appropriate sensor based on device type
    match device_info.device_type {
        DeviceType::Rtd => {
            let rtd = Rtd::<UartDriver>::from_uart(uart_driver, device_info.firmware_version);
            Ok(Arc::new(rtd) as Arc<dyn Sensor>)
        }
    }
}
