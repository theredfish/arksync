use super::detect_plugged_sensors_task;
use crate::sensor::Sensor;
use crate::services::sensor::detect_unplugged_sensors;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

/// A sensor list compatible with both UART and I2C protocols.
pub type SensorList = HashMap<String, Arc<dyn Sensor>>;

pub enum SensorServiceCmd {
    /// Add sensors in the registry (no replacement)
    AddSensors {
        sensors: Vec<(String, Arc<dyn Sensor>)>,
    },
    /// Remove sensors from the registry
    RemoveSensors { uuids: Vec<String> },
    #[expect(unused)]
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
    tx: mpsc::Sender<SensorServiceCmd>,
    rx: mpsc::Receiver<SensorServiceCmd>,
}

/// Supervisor service that maintains the list of sensors
pub struct SensorService {
    sensors: SensorList,
    sensor_tasks: HashMap<String, JoinHandle<()>>,
    cmd_channel: CommandChannel,
}

impl Default for SensorService {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorService {
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

        println!("UartService started - maintaining sensor registry");

        let main_loop = async move {
            loop {
                tokio::select! {
                    Some(cmd) = self.cmd_channel.rx.recv() => {
                        self.handle_cmd(cmd);
                    }

                    // healthcheck => if this one dies then we stop the process

                    _ = tokio::signal::ctrl_c() => {
                        println!("Shutting down sensor registry...");
                        self.abort_all_sensor_tasks();
                        break;
                    }
                }
            }
        };

        // TODO: check for mutex contention across awaits
        tokio::join!(
            main_loop,
            detect_plugged_sensors_task(cmd_tx.clone()),
            detect_unplugged_sensors(cmd_tx.clone())
        );
    }

    /// Handle commands to maintain sensor list
    fn handle_cmd(&mut self, cmd: SensorServiceCmd) {
        match cmd {
            SensorServiceCmd::AddSensors { sensors } => {
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

            SensorServiceCmd::RemoveSensors { uuids } => {
                println!("Registry: Removing {} sensors", uuids.len());
                for uuid in &uuids {
                    if let Some(task) = self.sensor_tasks.remove(uuid) {
                        task.abort();
                    }
                    self.sensors.remove(uuid);
                }
                println!("Registry: Total sensors = {}", self.sensors.len());
            }

            SensorServiceCmd::FindSensor {
                serial_number,
                respond_to,
            } => {
                let sensor = self.sensors.get(&serial_number).cloned();
                let _ = respond_to.send(sensor);
            }

            SensorServiceCmd::AllSensors { respond_to } => {
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
}
