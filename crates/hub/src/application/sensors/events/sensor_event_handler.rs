// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{HubError, HubService, SensorRegistry};
use arksync_bus::Timestamp;
use arksync_knot::application::LegacyKnotActuatorMessage;
use arksync_sensor::infrastructure::events::SensorEvent;
use eyre::{eyre, Result};
use sqlx::PgPool;

use super::sensor_measurement_recorded::handle_sensor_measurement_recorded;
use super::sensor_plugged::handle_sensor_plugged;
use super::sensor_provisioned::handle_sensor_provisioned;
use super::sensor_provisioning_conflict::handle_sensor_provisioning_conflict;
use super::HubSensorEventEnvelope;

impl HubService {
    pub fn handle_sensor_event(
        &mut self,
        event: HubSensorEventEnvelope,
        received_at: Timestamp,
    ) -> Result<(), HubError> {
        let source = event.source;
        let occurred_at = event.occurred_at;

        match event.payload {
            SensorEvent::SerialSensorPlugged(event) => {
                handle_sensor_plugged(self, source, event.metadata, occurred_at, received_at);
                Ok(())
            }
            SensorEvent::SensorMeasurementRecorded(measurement) => {
                log::debug!(
                    "Hub handled sensor measurement device_uid={} value={}",
                    measurement.device_uid,
                    measurement.measurement.value
                );

                Ok(())
            }
            SensorEvent::SensorProvisioned(provisioned) => {
                log::debug!(
                    "Hub handled provisioned sensor device_uid={}",
                    provisioned.device_uid
                );

                Ok(())
            }
            SensorEvent::SensorProvisioningConflict(conflict) => {
                handle_sensor_provisioning_conflict(&conflict);
                Ok(())
            }
        }
    }
}

pub async fn handle_sensor_event(
    pool: &PgPool,
    event: HubSensorEventEnvelope,
    received_at: Timestamp,
    sensor_registry: &mut SensorRegistry,
    hub: &mut HubService,
    knot_event_tx: &tokio::sync::mpsc::Sender<LegacyKnotActuatorMessage>,
) -> Result<()> {
    log::debug!("Hub received local Knot sensor event: {event:?}");

    match &event.payload {
        SensorEvent::SensorProvisioned(provisioned) => {
            handle_sensor_provisioned(pool, &event, provisioned, sensor_registry).await?;
        }
        SensorEvent::SensorMeasurementRecorded(sensor_measurement) => {
            handle_sensor_measurement_recorded(
                pool,
                &event,
                sensor_measurement,
                received_at,
                sensor_registry,
                hub,
                knot_event_tx,
            )
            .await?;
        }
        SensorEvent::SerialSensorPlugged(_) | SensorEvent::SensorProvisioningConflict(_) => {}
    }

    if let Err(err) = hub.handle_sensor_event(event, received_at) {
        return Err(eyre!("Hub rejected local Knot sensor event: {err:?}"));
    }

    log::debug!("Hub projected local Knot sensor event");

    Ok(())
}
