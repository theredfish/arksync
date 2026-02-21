mod commands;
mod ezo;
mod serial_port;

use ezo::driver::uart::UartDriver;
use ezo::driver::{DeviceType, Driver};
use ezo::rtd::Rtd;
use ezo::sensor::Sensor;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration as TokioDuration};

pub type SensorList = HashMap<String, Arc<dyn Sensor>>;

enum ServiceCommand {
    /// Add or update sensors in the registry
    UpsertSensors {
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
    cmd_channel: CommandChannel,
}

impl UartService {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);

        Self {
            sensors: HashMap::new(),
            cmd_channel: CommandChannel { tx, rx },
        }
    }

    /// Main supervisor loop - maintains sensor registry
    pub async fn run(mut self) {
        let cmd_tx = self.cmd_channel.tx.clone();

        // Spawn detector task (detects new sensors and adds them to registry)
        self.detect_sensors(cmd_tx.clone());

        println!("UartService started - maintaining sensor registry");

        // Main event loop - just manages the HashMap
        loop {
            tokio::select! {
                Some(cmd) = self.cmd_channel.rx.recv() => {
                    self.handle_cmd(cmd);
                }

                _ = tokio::signal::ctrl_c() => {
                    println!("Shutting down sensor registry...");
                    break;
                }
            }
        }
    }

    /// Handle commands to maintain sensor list
    fn handle_cmd(&mut self, cmd: ServiceCommand) {
        match cmd {
            ServiceCommand::UpsertSensors { sensors } => {
                println!("Registry: Upserting {} sensors", sensors.len());
                for (uuid, sensor) in sensors {
                    self.sensors.insert(uuid, sensor);
                }
                println!("Registry: Total sensors = {}", self.sensors.len());
            }

            ServiceCommand::RemoveSensors { uuids } => {
                println!("Registry: Removing {} sensors", uuids.len());
                for uuid in &uuids {
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

    /// Detector task - finds new USB sensors and adds them to registry
    fn detect_sensors(&self, cmd_tx: Sender<ServiceCommand>) {
        tokio::spawn(async move {
            loop {
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
                            let sensor = create_sensor_from_port(&port);
                            println!(
                                "Detector: Created sensor {}: {:#?}",
                                port.serial_number,
                                sensor.as_ref().map(|s| s.data())
                            );

                            match sensor {
                                Ok(sensor) => {
                                    let data = sensor.data();
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
                            .send(ServiceCommand::UpsertSensors {
                                sensors: new_sensors,
                            })
                            .await;
                    }
                }

                sleep(TokioDuration::from_secs(5)).await;
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
    port: &serial_port::SerialPort,
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
