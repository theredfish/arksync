// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;

use crate::application::{insert_sensor, list_sensors, HubSensorError};
use crate::domain::{PluggedSensor, SensorId};
use arksync_utils::uuid::Uuid;
use sqlx::PgExecutor;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct SensorIdentityKey {
    station_knot_id: Uuid,
    device_uid: String,
}

#[derive(Default)]
pub struct SensorRegistry {
    sensors_by_device_uid: HashMap<SensorIdentityKey, SensorId>,
}

impl SensorRegistry {
    pub async fn load(executor: impl PgExecutor<'_>) -> Result<Self, HubSensorError> {
        let sensors = list_sensors(executor).await?;
        let sensors_by_device_uid = sensors
            .into_iter()
            .map(|sensor| {
                (
                    SensorIdentityKey {
                        station_knot_id: sensor.station_knot_id.uuid_v4(),
                        device_uid: sensor.device_uid,
                    },
                    sensor.id,
                )
            })
            .collect();

        Ok(Self {
            sensors_by_device_uid,
        })
    }

    pub async fn ensure_sensor_registered(
        &mut self,
        executor: impl PgExecutor<'_>,
        sensor: PluggedSensor,
    ) -> Result<SensorId, HubSensorError> {
        let key = SensorIdentityKey {
            station_knot_id: sensor.station_knot_id.uuid_v4(),
            device_uid: sensor.device_uid.as_ref().to_string(),
        };

        if let Some(sensor_id) = self.sensors_by_device_uid.get(&key) {
            return Ok(*sensor_id);
        }

        let registered_sensor = insert_sensor(executor, sensor).await?;
        self.sensors_by_device_uid.insert(key, registered_sensor.id);

        Ok(registered_sensor.id)
    }

    pub fn sensor_id(&self, station_knot_id: Uuid, device_uid: &str) -> Option<SensorId> {
        self.sensors_by_device_uid
            .get(&SensorIdentityKey {
                station_knot_id,
                device_uid: device_uid.to_string(),
            })
            .copied()
    }

    pub fn remember_sensor(
        &mut self,
        station_knot_id: Uuid,
        device_uid: String,
        sensor_id: SensorId,
    ) {
        self.sensors_by_device_uid.insert(
            SensorIdentityKey {
                station_knot_id,
                device_uid,
            },
            sensor_id,
        );
    }
}
