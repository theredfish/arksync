// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod sensor_event_handler;
mod sensor_measurement_recorded;
mod sensor_plugged;
mod sensor_provisioned;
mod sensor_provisioning_conflict;

use arksync_bus::EventEnvelope;
use arksync_knot::domain::KnotEventSource;
use arksync_sensor::infrastructure::events::SensorEvent;

pub type HubSensorEventEnvelope = EventEnvelope<SensorEvent, KnotEventSource>;

pub use sensor_event_handler::*;
