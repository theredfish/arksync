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
        let knot_id = KnotId::new_with_random_bytes(bytes);
        let parent_hub_id = ParentHubId::new_with_random_bytes(bytes);

        assert_eq!(knot_id.as_bytes(), parent_hub_id.as_bytes());
        assert_eq!(knot_id.as_uuid(), parent_hub_id.as_uuid());
    }

    #[cfg(feature = "uuid-v4")]
    #[test]
    fn generates_knot_id() {
        let id = KnotId::new();

        assert_eq!(id.as_uuid().get_version_num(), 4);
    }
}
