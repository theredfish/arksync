// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::sensor::SensorConnection;
use crate::services::sensor::SensorServiceCmd;
use std::collections::HashSet;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::time::{interval, Duration as TokioDuration};
use tokio_util::sync::CancellationToken;

use crate::serial_port::{self};

/// Listen for unplugged sensors.
///
/// Compare the list of sensors'connection with OS connections for both
/// UART and I2C and mark stale sensors as unplugged.
pub async fn detect_unplugged_sensors(
    cmd_tx: &Sender<SensorServiceCmd>,
    shutdown: CancellationToken,
) {
    let mut interval = interval(TokioDuration::from_secs(2));

    loop {
        tokio::select! {
            _ = interval.tick() => {}
            _ = shutdown.cancelled() => {
                println!("Detector: stopping unplugged sensor scan");
                break;
            }
        }

        let available_asc_ports = serial_port::find_asc_port();
        let available_port_serials: HashSet<_> = available_asc_ports
            .iter()
            .map(|port| &port.serial_number)
            .collect();

        // Get current sensor list
        let (respond_to, rx) = oneshot::channel();
        let _ = cmd_tx
            .send(SensorServiceCmd::AllSensors { respond_to })
            .await;
        let current_sensors = rx.await;

        if let Ok(sensors) = current_sensors {
            let mut unplugged_sensors = Vec::new();

            for sensor in sensors.values() {
                let info = sensor.info();
                let connection_info = &info.connection;

                match connection_info {
                    SensorConnection::Uart(port_metadata) => {
                        if !available_port_serials.contains(&port_metadata.serial_number) {
                            println!(
                                "Detector: Sensor {} is unplugged, removing from registry",
                                port_metadata.serial_number
                            );
                            sensor.mark_unplugged();
                            unplugged_sensors.push(port_metadata.serial_number.clone());
                        }
                    }
                    SensorConnection::I2c(_) => {
                        unimplemented!("No I2C sensor handling yet");
                    }
                }
            }

            // Remove all unplugged sensors in a single batch to allow quick reconnect
            if !unplugged_sensors.is_empty() {
                let _ = cmd_tx
                    .send(SensorServiceCmd::RemoveSensors {
                        uuids: unplugged_sensors,
                    })
                    .await;
            }
        }
    }
}
