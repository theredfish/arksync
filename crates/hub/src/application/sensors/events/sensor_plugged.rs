// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::{default_measurement_interval_ms, HubService};
use crate::domain::PluggedSensor;
use arksync_bus::Timestamp;
use arksync_knot::domain::KnotEventSource;
use arksync_sensor::device_uid::DeviceUid;
use arksync_sensor::infrastructure::events::MeasuredSensor;
use arksync_sensor::serial_port::SerialPortMetadata;

pub(super) fn handle_sensor_plugged(
    hub: &mut HubService,
    source: KnotEventSource,
    metadata: SerialPortMetadata,
    observed_at: Timestamp,
    received_at: Timestamp,
) {
    hub.observe_serial_sensor(source, metadata, observed_at, received_at);
}

pub(super) fn extract_plugged_sensor(
    source: &KnotEventSource,
    device_uid: DeviceUid,
    sensor: &MeasuredSensor,
) -> PluggedSensor {
    let KnotEventSource::Knot { knot_id, .. } = source;

    PluggedSensor {
        station_knot_id: *knot_id,
        device_uid,
        kind: sensor.kind,
        connection: sensor.connection.clone(),
        firmware: sensor.firmware,
        measurement_interval_ms: default_measurement_interval_ms(),
    }
}
