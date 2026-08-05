// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub use uuid::{Error, Uuid};

pub fn from_bytes(bytes: [u8; 16]) -> Uuid {
    uuid::Builder::from_bytes(bytes)
        .with_variant(uuid::Variant::RFC4122)
        .with_version(uuid::Version::Random)
        .into_uuid()
}

#[cfg(feature = "uuid-v4")]
pub fn new_v4() -> Uuid {
    Uuid::new_v4()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_uuid_from_bytes() {
        let uuid = from_bytes([1; 16]);

        assert_eq!(uuid.as_bytes().len(), 16);
        assert_eq!(uuid.get_version_num(), 4);
    }

    #[cfg(feature = "uuid-v4")]
    #[test]
    fn generates_uuid_v4() {
        let uuid = new_v4();

        assert_eq!(uuid.get_version_num(), 4);
    }
}
