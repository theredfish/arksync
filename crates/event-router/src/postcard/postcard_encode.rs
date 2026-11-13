// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::Serialize;

pub trait PostcardEncode {
    fn encode_postcard<'buffer>(
        &self,
        buffer: &'buffer mut [u8],
    ) -> Result<&'buffer mut [u8], postcard::Error>;
}

impl<T> PostcardEncode for T
where
    T: Serialize,
{
    fn encode_postcard<'buffer>(
        &self,
        buffer: &'buffer mut [u8],
    ) -> Result<&'buffer mut [u8], postcard::Error> {
        postcard::to_slice(self, buffer)
    }
}
