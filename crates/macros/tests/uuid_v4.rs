// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_macros::UuidV4;

#[derive(UuidV4)]
struct TestId([u8; 16]);

#[test]
fn derives_uuid_roundtrip_helpers() {
    let uuid = arksync_utils::uuid::from_random_bytes([1; 16]);
    let id = TestId::new_with_uuid(uuid);

    assert_eq!(id.as_uuid(), uuid);
    assert_eq!(id.as_bytes(), uuid.as_bytes());
    assert_eq!(TestId::from(uuid), id);
}

#[test]
fn derives_random_bytes_constructor() {
    let id = TestId::new_with_random_bytes([2; 16]);

    assert_eq!(id.as_uuid().get_version_num(), 4);
}
