// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_macros::UuidV4;

#[derive(UuidV4)]
pub struct KnotId([u8; 16]);

#[derive(UuidV4)]
pub struct ParentHubId([u8; 16]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_and_knot_ids_can_share_same_uuid_bytes() {
        let bytes = [1; 16];
        let knot_id = KnotId::from_bytes(bytes);
        let parent_hub_id = ParentHubId::from_bytes(bytes);

        assert_eq!(knot_id.as_bytes(), parent_hub_id.as_bytes());
        assert_eq!(knot_id.uuid_v4(), parent_hub_id.uuid_v4());
    }

    #[cfg(feature = "uuid-v4")]
    #[test]
    fn generates_knot_id() {
        let id = KnotId::new();
        let uuid = id.uuid_v4();

        assert_eq!(uuid.get_version_num(), 4);
    }
}
