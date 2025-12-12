mod sensor;
mod serial_port;

use sensor::Sensor;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration};

use serial_port::*;

// pub type SensorList = Arc<Mutex<HashMap<String, Sensor>>>;

enum ServiceCommand {
    AddSensor {
        uuid: String,
        sensor: Sensor,
    },
    RemoveSensor {
        uuid: String,
    },
    GetSensor {
        uuid: String,
        respond_to: oneshot::Sender<Option<Sensor>>,
    },
    GetActiveSensors {
        respond_to: oneshot::Sender<Vec<(String, Sensor)>>,
    },
}

pub struct CommandChannel {
    tx: mpsc::Sender<ServiceCommand>,
    rx: mpsc::Receiver<ServiceCommand>,
}

pub struct UartService {
    sensors: HashMap<String, Sensor>,
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

    fn handle_cmd(&self, cmd: ServiceCommand) {
        match cmd {
            ServiceCommand::AddSensor { uuid, sensor } => {
                println!("Adding sensor: {} - {:?}", uuid, sensor);
            }
            ServiceCommand::RemoveSensor { uuid } => {
                println!("Removing sensor: {}", uuid);
            }
            ServiceCommand::GetSensor { uuid, respond_to } => {
                println!("Getting sensor: {}", uuid);
                let _ = respond_to.send(None);
            }
            ServiceCommand::GetActiveSensors { respond_to } => {
                println!("Getting active sensors");
                let _ = respond_to.send(vec![]);
            }
        }
    }

    fn spawn_detector(&self, cmd_tx: Sender<ServiceCommand>) {
        tokio::spawn(async move {
            loop {
                println!("Checking for sensors...");
                let atlas_sc_ports = serial_port::find_atlas_sc_port();
                println!("{atlas_sc_ports:#?}");
                sleep(Duration::from_secs(60)).await;
            }
        });
    }

    fn spawn_reader_manager(&self, cmd_tx: Sender<ServiceCommand>) {
        tokio::spawn(async move {
            println!("Reading sensors...");
            sleep(Duration::from_secs(60)).await;
        });
    }

    fn spawn_healthcheck(&self, cmd_tx: Sender<ServiceCommand>) {
        tokio::spawn(async move {
            loop {
                let (respond_to, rx) = oneshot::channel();

                // TODO: warn on error
                let _ = cmd_tx
                    .send(ServiceCommand::GetActiveSensors { respond_to })
                    .await;

                if let Ok(sensors) = rx.await {
                    println!("Health check: {} sensors", sensors.len());
                }

                sleep(Duration::from_secs(20)).await;
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
