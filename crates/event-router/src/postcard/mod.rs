// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Postcard support for event envelopes and bounded-context messages.

mod postcard_decode;
mod postcard_encode;

pub use postcard::Error;
pub use postcard_decode::*;
pub use postcard_encode::*;
