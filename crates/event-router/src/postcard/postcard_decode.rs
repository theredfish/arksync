// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::de::DeserializeOwned;

pub trait PostcardDecode: Sized {
    fn decode_postcard(bytes: &[u8]) -> Result<Self, postcard::Error>;
}

impl<T> PostcardDecode for T
where
    T: DeserializeOwned,
{
    fn decode_postcard(bytes: &[u8]) -> Result<Self, postcard::Error> {
        postcard::from_bytes(bytes)
    }
}
