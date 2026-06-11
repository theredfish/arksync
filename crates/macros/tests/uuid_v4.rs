// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_macros::UuidV4;

#[derive(UuidV4)]
struct TestId([u8; 16]);

#[test]
fn derives_uuid_roundtrip_helpers() {
    let uuid = arksync_utils::uuid::from_bytes([1; 16]);
    let id = TestId::from(uuid);

    assert_eq!(id.uuid_v4(), uuid);
    assert_eq!(id.as_bytes(), uuid.as_bytes());
    assert_eq!(TestId::from(uuid), id);
    assert_eq!(id.to_string().parse::<TestId>().unwrap(), id);
}

#[test]
fn derives_bytes_constructor() {
    let id = TestId::from_bytes([2; 16]);
    let uuid = id.uuid_v4();

    assert_eq!(uuid.get_version_num(), 4);
}
