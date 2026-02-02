mod commands;
mod sensor;
mod serial_port;

use chrono::{Duration as ChronoDuration, Utc};
use sensor::{Sensor, SensorState};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration as TokioDuration};

pub type SensorList = HashMap<String, Sensor>;

enum ServiceCommand {
    RemoveSensors {
        uuids: Vec<String>,
    },
    FindSensor {
        serial_number: String,
        respond_to: oneshot::Sender<Option<Sensor>>,
    },
    AllSensors {
        respond_to: oneshot::Sender<Arc<SensorList>>,
    },
    UpsertSensors {
        sensors: Vec<(String, Sensor)>,
    },
}

pub struct CommandChannel {
    tx: mpsc::Sender<ServiceCommand>,
    rx: mpsc::Receiver<ServiceCommand>,
}

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

    /// Supervisor loop
    pub async fn run(mut self) {
        let cmd_tx = self.cmd_channel.tx.clone();

        // Spawn background tasks
        self.spawn_detector(cmd_tx.clone());
        self.spawn_healthcheck(cmd_tx.clone());
        self.spawn_reader_manager(cmd_tx.clone());

        // Main supervisor loop
        loop {
            tokio::select! {
                Some(cmd) = self.cmd_channel.rx.recv() => {
                    self.handle_cmd(cmd);
                }

                // Graceful termination signal (Ctrl+C)
                _ = tokio::signal::ctrl_c() => {
                    println!("Shutting down...");
                    break;
                }
            }
        }
    }

    fn handle_cmd(&mut self, cmd: ServiceCommand) {
        match cmd {
            ServiceCommand::RemoveSensors { uuids } => {
                println!("RemoveSensors: {} sensors", uuids.len());
                for uuid in uuids {
                    self.sensors.remove(&uuid);
                }
            }
            ServiceCommand::FindSensor {
                serial_number,
                respond_to,
            } => {
                println!("FindSensor: {}", serial_number);
                let sensor = self.sensors.get(&serial_number);
                let _ = respond_to.send(sensor.cloned());
            }
            ServiceCommand::AllSensors { respond_to } => {
                println!("AllSensors: {:#?}", self.sensors);
                let _ = respond_to.send(Arc::new(self.sensors.clone()));
            }
            ServiceCommand::UpsertSensors { sensors } => {
                println!("UpsertSensors: {} sensors", sensors.len());
                for (uuid, mut sensor) in sensors {
                    sensor.last_activity = Utc::now();
                    sensor.state = SensorState::Active;
                    self.sensors.insert(uuid, sensor);
                }
            }
        }
    }

    fn spawn_detector(&self, cmd_tx: Sender<ServiceCommand>) {
        tokio::spawn(async move {
            loop {
                println!("Checking for sensors...");
                let asc_ports = serial_port::find_asc_port();
                println!("Found {} ASC ports: {:#?}", asc_ports.len(), asc_ports);

                // Get current sensor list
                let (respond_to, rx) = oneshot::channel();
                let _ = cmd_tx.send(ServiceCommand::AllSensors { respond_to }).await;

                if let Ok(current_sensors) = rx.await {
                    // Find new sensors (not in the current list)
                    let mut new_sensors: Vec<(String, Sensor)> = Vec::new();

                    for port in asc_ports.iter() {
                        if !current_sensors.contains_key(&port.serial_number) {
                            // Try to connect and query device info
                            match Sensor::from_device(port.clone()) {
                                Ok(sensor) => {
                                    println!(
                                        "Successfully connected to sensor: {:?} v{} ({})",
                                        sensor.kind, sensor.firmware, port.serial_number
                                    );
                                    new_sensors.push((port.serial_number.clone(), sensor));
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Failed to connect to sensor {}: {}",
                                        port.serial_number, e
                                    );
                                }
                            }
                        }
                    }

                    if !new_sensors.is_empty() {
                        println!("Adding {} new sensors", new_sensors.len());
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

    fn spawn_reader_manager(&self, _cmd_tx: Sender<ServiceCommand>) {
        tokio::spawn(async move {
            println!("Reading sensors...");
            sleep(TokioDuration::from_secs(60)).await;
        });
    }

    fn spawn_healthcheck(&self, cmd_tx: Sender<ServiceCommand>) {
        tokio::spawn(async move {
            loop {
                let (respond_to, rx) = oneshot::channel();

                // Request snapshot of all sensors
                let _ = cmd_tx.send(ServiceCommand::AllSensors { respond_to }).await;

                if let Ok(sensors) = rx.await {
                    println!("Health check: {} sensors", sensors.len());

                    let now = Utc::now();
                    let timeout_duration = ChronoDuration::minutes(2);

                    // Collect sensors to remove: Unreachable AND last_activity > 2 minutes
                    let sensors_to_remove: Vec<String> = sensors
                        .iter()
                        .filter(|(_, sensor)| {
                            matches!(sensor.state, SensorState::Unreachable)
                                && (now - sensor.last_activity) > timeout_duration
                        })
                        .map(|(uuid, _)| uuid.clone())
                        .collect();

                    // Collect sensors to update: all others
                    let sensors_to_update: Vec<(String, Sensor)> = sensors
                        .iter()
                        .filter(|(uuid, _)| !sensors_to_remove.contains(uuid))
                        .map(|(uuid, sensor)| (uuid.clone(), sensor.clone()))
                        .collect();

                    // Remove timed-out unreachable sensors
                    if !sensors_to_remove.is_empty() {
                        println!("Removing {} unreachable sensors", sensors_to_remove.len());
                        let _ = cmd_tx
                            .send(ServiceCommand::RemoveSensors {
                                uuids: sensors_to_remove,
                            })
                            .await;
                    }

                    // Update active sensors with current timestamp and Active state
                    if !sensors_to_update.is_empty() {
                        let _ = cmd_tx
                            .send(ServiceCommand::UpsertSensors {
                                sensors: sensors_to_update,
                            })
                            .await;
                    }
                }

                sleep(TokioDuration::from_secs(20)).await;
            }
        });
    }
}

// - A UartSensorService: this service is in charge of managing all kind of
// Atlas Scientific sensors.
//
// - A task to manage sensors among existing ones with filtering. When a new
// sensor is detected, it is added to the list. When the current USB ports don't
// include one of the sensors, it is removed from the list. This task only track
// some kind of unique information.
//
// - A task to healthcheck sensors and update their status. Healthy, Unhealthy,
// Lost. The status tracks the last status updated_at which helps to determine
// the state machine for passing from one state to another.
//
// - A task per healthy sensor to handle sensor values. A cancellation token
// should be used to terminate the task when a sensor status is Lost.
//
// - A task to handle commands sent to sensors and handle responses.

#[tokio::main]
async fn main() {
    UartService::new().run().await
}
