// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::detect_plugged_sensors_task;
use crate::infrastructure::events::{
    SensorEvent, SensorMeasurementRecorded, SensorProvisioned, SensorProvisioningConflict,
    SerialSensorPlugged,
};
use crate::sensor::Sensor;
use crate::sensor::SensorConnection;
use crate::services::sensor::{detect_unplugged_sensors, healthcheck};
use arksync_bus::{EventEnvelope, EventProducer, Timestamp};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

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
pub struct SensorService<'bus> {
    sensors: SensorList,
    sensor_tasks: HashMap<String, JoinHandle<()>>,
    cmd_channel: CommandChannel,
    measurement_tx: mpsc::Sender<SensorMeasurementRecorded>,
    measurement_rx: mpsc::Receiver<SensorMeasurementRecorded>,
    event_producer: Option<EventProducer<'bus, SensorEvent>>,
    event_counter: u128,
}

impl Default for SensorService<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'bus> SensorService<'bus> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(100);
        let (measurement_tx, measurement_rx) = mpsc::channel(100);

        Self {
            sensors: HashMap::new(),
            sensor_tasks: HashMap::new(),
            cmd_channel: CommandChannel { tx, rx },
            measurement_tx,
            measurement_rx,
            event_producer: None,
            event_counter: 0,
        }
    }

    pub fn with_event_producer(mut self, event_producer: EventProducer<'bus, SensorEvent>) -> Self {
        self.event_producer = Some(event_producer);
        self
    }

    /// Main supervisor loop - maintains sensor registry
    pub async fn run(mut self) {
        let cmd_tx = self.cmd_channel.tx.clone();
        let shutdown = CancellationToken::new();
        println!("Sensor service started - maintaining sensor registry");

        let main_loop = {
            let shutdown = shutdown.clone();

            async move {
                loop {
                    tokio::select! {
                        Some(cmd) = self.cmd_channel.rx.recv() => {
                            self.handle_cmd(cmd);
                        }

                        Some(measurement) = self.measurement_rx.recv() => {
                            self.emit_measurement_recorded_event(measurement);
                        }

                        _ = tokio::signal::ctrl_c() => {
                            println!("Shutting down sensor registry...");
                            shutdown.cancel();
                            self.abort_all_sensor_tasks();
                            break;
                        }

                        _ = shutdown.cancelled() => {
                            self.abort_all_sensor_tasks();
                            break;
                        }
                    }
                }
            }
        };

        // TODO: check for mutex contention across awaits
        tokio::join!(
            main_loop,
            healthcheck(&cmd_tx, shutdown.clone()),
            detect_plugged_sensors_task(&cmd_tx, shutdown.clone()),
            detect_unplugged_sensors(&cmd_tx, shutdown)
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

                    let device_uid = match sensor.ensure_device_uid() {
                        Ok(device_uid) => device_uid,
                        Err(err) => {
                            self.emit_sensor_provisioning_conflict_event(
                                sensor.as_ref(),
                                err.to_string(),
                            );
                            continue;
                        }
                    };

                    self.emit_sensor_provisioned_event(sensor.as_ref(), device_uid);
                    self.emit_sensor_added_event(sensor.as_ref());
                    let task = Arc::clone(&sensor).run(Some(self.measurement_tx.clone()));
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
                let _ = respond_to.send(Arc::new(self.sensors.clone()));
            }
        }
    }

    fn abort_all_sensor_tasks(&mut self) {
        for (_, task) in self.sensor_tasks.drain() {
            task.abort();
        }
    }

    fn emit_sensor_added_event(&mut self, sensor: &dyn Sensor) {
        let Some(event_producer) = &mut self.event_producer else {
            return;
        };

        let info = sensor.info();
        let SensorConnection::Uart(metadata) = info.connection else {
            return;
        };

        log::debug!(
            "Sensor service produced SerialSensorPlugged for serial_number={}",
            metadata.serial_number
        );

        let _ = event_producer.publish(EventEnvelope::new_with_id(
            sensor_event_id(&metadata.serial_number),
            (),
            Timestamp::from_unix_millis(0),
            SensorEvent::SerialSensorPlugged(SerialSensorPlugged { metadata }),
        ));
    }

    fn emit_sensor_provisioned_event(
        &mut self,
        sensor: &dyn Sensor,
        device_uid: crate::device_uid::DeviceUid,
    ) {
        let Some(event_producer) = &mut self.event_producer else {
            return;
        };

        let info = sensor.info();
        let measured_sensor = crate::sensor::measured_sensor_from_info(&info);

        log::debug!("Sensor service produced SensorProvisioned device_uid={device_uid}");

        let _ = event_producer.publish(EventEnvelope::new_with_id(
            sensor_event_id(device_uid.as_ref()),
            (),
            timestamp_now(),
            SensorEvent::SensorProvisioned(SensorProvisioned {
                device_uid,
                sensor: measured_sensor,
            }),
        ));
    }

    fn emit_sensor_provisioning_conflict_event(&mut self, sensor: &dyn Sensor, reason: String) {
        self.event_counter = self.event_counter.wrapping_add(1);
        let Some(event_producer) = &mut self.event_producer else {
            return;
        };

        let info = sensor.info();
        let measured_sensor = crate::sensor::measured_sensor_from_info(&info);

        log::debug!("Sensor service produced SensorProvisioningConflict reason={reason}");

        let _ = event_producer.publish(EventEnvelope::new_with_id(
            event_id_from_counter(self.event_counter),
            (),
            timestamp_now(),
            SensorEvent::SensorProvisioningConflict(SensorProvisioningConflict {
                reason,
                sensor: measured_sensor,
            }),
        ));
    }

    fn emit_measurement_recorded_event(&mut self, measurement: SensorMeasurementRecorded) {
        self.event_counter = self.event_counter.wrapping_add(1);
        let Some(event_producer) = &mut self.event_producer else {
            return;
        };

        log::debug!(
            "Sensor service produced SensorMeasurementRecorded for device_uid={} value={}",
            measurement.device_uid,
            measurement.measurement.value
        );

        let _ = event_producer.publish(EventEnvelope::new_with_id(
            event_id_from_counter(self.event_counter),
            (),
            timestamp_now(),
            SensorEvent::SensorMeasurementRecorded(measurement),
        ));
    }
}

fn event_id_from_counter(counter: u128) -> arksync_bus::EventId {
    arksync_bus::EventId::new_with_random_bytes(counter.to_be_bytes())
}

fn timestamp_now() -> Timestamp {
    let unix_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Timestamp::from_unix_millis(unix_millis)
}

fn sensor_event_id(serial_number: &str) -> arksync_bus::EventId {
    let mut bytes = [0; 16];

    for (index, byte) in serial_number.as_bytes().iter().take(16).enumerate() {
        bytes[index] = *byte;
    }

    arksync_bus::EventId::new_with_random_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_uid::DeviceUid;
    use crate::error::{Result, SensorError};
    use crate::sensor::{SensorInfo, SensorName, SensorState, SensorStateReason};
    use crate::serial_port::{SerialPortMetadata, DEFAULT_BAUD_RATE};
    use chrono::Utc;

    struct MockSensor {
        metadata: SerialPortMetadata,
    }

    impl MockSensor {
        fn new(serial_number: &str) -> Self {
            Self {
                metadata: SerialPortMetadata {
                    port_name: "/dev/ttyUSB0".to_string(),
                    serial_number: serial_number.to_string(),
                    baud_rate: DEFAULT_BAUD_RATE,
                },
            }
        }
    }

    impl Sensor for MockSensor {
        fn info(&self) -> SensorInfo {
            let now = Utc::now();

            SensorInfo {
                firmware: 1.0,
                name: SensorName::Unnamed,
                device_uid: Some(DeviceUid::try_from("A1B2C3D4E5F6G7H8").unwrap()),
                state: SensorState::Active,
                state_reason: SensorStateReason::Plugged,
                state_since: now,
                last_activity: now,
                consecutive_failures: 0,
                connection: SensorConnection::Uart(self.metadata.clone()),
            }
        }

        fn read_measurement(&self) -> Result<f64> {
            Err(SensorError::message("mock sensor does not read"))
        }

        fn check_measurement(&self, _value: f64) -> Option<SensorStateReason> {
            None
        }

        fn record_measurement(&self, _value: f64) {}

        fn record_error(&self, _err: &SensorError) {}

        fn mark_unplugged(&self) {}
    }

    #[tokio::test]
    async fn emits_sensor_plugged_event_when_sensor_is_added() {
        let (event_tx, mut event_rx) = mpsc::channel(2);
        let mut bus = arksync_bus::EventBus::new();
        bus.subscribe(move |event: EventEnvelope<SensorEvent>| {
            event_tx
                .try_send(event)
                .map_err(|_| arksync_bus::EventBusError::HandlerRejected)
        });
        let mut service = SensorService::new().with_event_producer(bus.producer());
        let sensor = Arc::new(MockSensor::new("rtd-serial-1")) as Arc<dyn Sensor>;

        service.handle_cmd(SensorServiceCmd::AddSensors {
            sensors: vec![("rtd-serial-1".to_string(), sensor)],
        });

        let provisioned_event = event_rx.recv().await.unwrap();
        let plugged_event = event_rx.recv().await.unwrap();
        service.abort_all_sensor_tasks();

        assert!(matches!(
            provisioned_event.payload,
            SensorEvent::SensorProvisioned(SensorProvisioned { device_uid, .. })
                if device_uid.as_ref() == "A1B2C3D4E5F6G7H8"
        ));
        assert!(matches!(
            plugged_event.payload,
            SensorEvent::SerialSensorPlugged(SerialSensorPlugged { metadata })
                if metadata.serial_number == "rtd-serial-1"
        ));
    }
}
