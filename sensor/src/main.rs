mod error;
mod ezo;
mod i2c_bus;
mod sensor;
mod serial_port;

use ezo::driver::uart::UartDriver;
use ezo::driver::{DeviceType, Driver};
use ezo::rtd::Rtd;
use sensor::Sensor;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration as TokioDuration};

use crate::sensor::SensorConnection;
use crate::serial_port::SerialPortMetadata;

/// A sensor list compatible with both UART and I2C protocols.
pub type SensorList = HashMap<String, Arc<dyn Sensor>>;

#[allow(unused)]
enum ServiceCommand {
    /// Add sensors in the registry (no replacement)
    AddSensors {
        sensors: Vec<(String, Arc<dyn Sensor>)>,
    },
    /// Remove sensors from the registry
    RemoveSensors { uuids: Vec<String> },
    /// Get a specific sensor by serial number
    FindSensor {
        serial_number: String,
        respond_to: oneshot::Sender<Option<Arc<dyn Sensor>>>,
    },
    /// Get all sensors (snapshot)
    AllSensors {
        respond_to: oneshot::Sender<Arc<SensorList>>,
    },
}

pub struct CommandChannel {
    tx: mpsc::Sender<ServiceCommand>,
    rx: mpsc::Receiver<ServiceCommand>,
}

/// Supervisor service that maintains the list of sensors
pub struct UartService {
    sensors: SensorList,
    sensor_tasks: HashMap<String, JoinHandle<()>>,
    cmd_channel: CommandChannel,
}

impl Default for UartService {
    fn default() -> Self {
        Self::new()
    }
}

impl UartService {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);

        Self {
            sensors: HashMap::new(),
            sensor_tasks: HashMap::new(),
            cmd_channel: CommandChannel { tx, rx },
        }
    }

    /// Main supervisor loop - maintains sensor registry
    pub async fn run(mut self) {
        let cmd_tx = self.cmd_channel.tx.clone();

        // Spawn tasks to detect (un)plugged sensors and update the registry
        self.detect_plugged_sensors(cmd_tx.clone());
        self.detect_unplugged_sensors(cmd_tx.clone());

        println!("UartService started - maintaining sensor registry");

        // Main event loop - just manages the HashMap
        loop {
            tokio::select! {
                Some(cmd) = self.cmd_channel.rx.recv() => {
                    self.handle_cmd(cmd);
                }

                _ = tokio::signal::ctrl_c() => {
                    println!("Shutting down sensor registry...");
                    self.abort_all_sensor_tasks();
                    break;
                }
            }
        }
    }

    /// Handle commands to maintain sensor list
    fn handle_cmd(&mut self, cmd: ServiceCommand) {
        match cmd {
            ServiceCommand::AddSensors { sensors } => {
                println!("Registry: Adding up to {} sensors", sensors.len());
                for (uuid, sensor) in sensors {
                    if self.sensors.contains_key(&uuid) {
                        println!("Registry: Sensor {uuid} already exists, skipping add");
                        continue;
                    }

                    let task = Arc::clone(&sensor).run();
                    self.sensor_tasks.insert(uuid.clone(), task);
                    self.sensors.insert(uuid, sensor);
                }
                println!("Registry: Total sensors = {}", self.sensors.len());
            }

            ServiceCommand::RemoveSensors { uuids } => {
                println!("Registry: Removing {} sensors", uuids.len());
                for uuid in &uuids {
                    if let Some(task) = self.sensor_tasks.remove(uuid) {
                        task.abort();
                    }
                    self.sensors.remove(uuid);
                }
                println!("Registry: Total sensors = {}", self.sensors.len());
            }

            ServiceCommand::FindSensor {
                serial_number,
                respond_to,
            } => {
                let sensor = self.sensors.get(&serial_number).cloned();
                let _ = respond_to.send(sensor);
            }

            ServiceCommand::AllSensors { respond_to } => {
                println!(
                    "Registry: Providing snapshot of all sensors ({} total)",
                    self.sensors.len()
                );
                let _ = respond_to.send(Arc::new(self.sensors.clone()));
            }
        }
    }

    fn abort_all_sensor_tasks(&mut self) {
        for (_, task) in self.sensor_tasks.drain() {
            task.abort();
        }
    }

    /// Listen for plugged sensors.
    ///
    /// Finds new USB sensors and adds them to registry.
    fn detect_plugged_sensors(&self, cmd_tx: Sender<ServiceCommand>) {
        tokio::spawn(async move {
            let mut interval = interval(TokioDuration::from_secs(2));

            loop {
                interval.tick().await;

                println!("Detector: Scanning for sensors...");
                let asc_ports = serial_port::find_asc_port();

                if !asc_ports.is_empty() {
                    println!("Detector: Found {} ASC ports", asc_ports.len());
                }

                // Get current sensor list
                let (respond_to, rx) = oneshot::channel();
                let _ = cmd_tx.send(ServiceCommand::AllSensors { respond_to }).await;
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
                                    println!(
                                        "Detector: Created sensor - firmware v{}",
                                        data.firmware
                                    );
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
                            .send(ServiceCommand::AddSensors {
                                sensors: new_sensors,
                            })
                            .await;
                    }
                }
            }
        });
    }

    /// Listen for unplugged sensors.
    ///
    /// Compare the list of sensors'connection with OS connections for both
    /// UART and I2C and remove stale sensor from the list.
    fn detect_unplugged_sensors(&self, cmd_tx: Sender<ServiceCommand>) {
        tokio::spawn(async move {
            let mut interval = interval(TokioDuration::from_secs(2));

            loop {
                interval.tick().await;

                let available_asc_ports = serial_port::find_asc_port();
                let available_port_serials: HashSet<_> = available_asc_ports
                    .iter()
                    .map(|port| &port.serial_number)
                    .collect();

                // Get current sensor list
                let (respond_to, rx) = oneshot::channel();
                let _ = cmd_tx.send(ServiceCommand::AllSensors { respond_to }).await;
                let current_sensors = rx.await;

                if let Ok(sensors) = current_sensors {
                    for sensor in sensors.values() {
                        let connection_info = &sensor.info().connection;

                        match connection_info {
                            SensorConnection::Uart(port_metadata) => {
                                if !available_port_serials.contains(&port_metadata.serial_number) {
                                    println!(
                                        "Detector: Sensor {} is unplugged, removing from registry",
                                        port_metadata.serial_number
                                    );
                                    let _ = cmd_tx
                                        .send(ServiceCommand::RemoveSensors {
                                            uuids: vec![port_metadata.serial_number.clone()],
                                        })
                                        .await;
                                }
                            }
                            SensorConnection::I2c(_) => {
                                unimplemented!("No I2C sensor handling yet");
                            }
                        }
                    }
                }
            }
        });
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

#[tokio::main]
async fn main() {
    println!("Starting ArkSync Sensor Service...");
    UartService::new().run().await;
}
