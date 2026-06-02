// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{Hub, HubError};
use arksync_bus::{EventEnvelope, Timestamp};
use arksync_knot::domain::KnotEventSource;
use arksync_sensor::infrastructure::events::{
    SensorEvent, SensorProvisioned, SensorProvisioningConflict, SerialSensorPlugged,
};

pub type HubSensorEventEnvelope = EventEnvelope<SensorEvent, KnotEventSource>;

impl Hub {
    pub fn accept_sensor_event(
        &mut self,
        event: HubSensorEventEnvelope,
        received_at: Timestamp,
    ) -> Result<(), HubError> {
        match event.payload {
            SensorEvent::SerialSensorPlugged(SerialSensorPlugged { metadata }) => {
                self.observe_serial_sensor(event.source, metadata, event.occurred_at, received_at);
                Ok(())
            }
            SensorEvent::SensorMeasurementRecorded(measurement) => {
                log::debug!(
                    "Hub accepted sensor measurement hardware_uid={} value={}",
                    measurement.sensor.hardware_uid,
                    measurement.measurement.value
                );

                self.record_sensor_measurement(
                    event.source,
                    measurement,
                    event.occurred_at,
                    received_at,
                );

                Ok(())
            }
            SensorEvent::SensorProvisioned(SensorProvisioned { device_uid, .. }) => {
                log::debug!("Hub accepted provisioned sensor device_uid={device_uid}");

                Ok(())
            }
            SensorEvent::SensorProvisioningConflict(SensorProvisioningConflict {
                reason, ..
            }) => {
                log::debug!("Hub accepted sensor provisioning conflict reason={reason}");

                Ok(())
            }
        }
    }
}
