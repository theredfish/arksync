// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_macros::UuidV4;

#[derive(UuidV4)]
pub struct HubId([u8; 16]);

#[derive(UuidV4)]
pub struct SensorId([u8; 16]);

#[derive(UuidV4)]
pub struct ActuatorId([u8; 16]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_hub_id_from_random_bytes() {
        let id = HubId::new_with_random_bytes([1; 16]);

        assert_eq!(id.as_uuid().get_version_num(), 4);
    }

    #[test]
    fn builds_sensor_id_from_random_bytes() {
        let id = SensorId::new_with_random_bytes([2; 16]);

        assert_eq!(id.as_uuid().get_version_num(), 4);
    }

    #[test]
    fn builds_actuator_id_from_random_bytes() {
        let id = ActuatorId::new_with_random_bytes([3; 16]);

        assert_eq!(id.as_uuid().get_version_num(), 4);
    }

    #[cfg(feature = "uuid-v4")]
    #[test]
    fn generates_hub_id() {
        let id = HubId::new();

        assert_eq!(id.as_uuid().get_version_num(), 4);
    }
}
