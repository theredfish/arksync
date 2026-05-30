// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use proc_macro::TokenStream;

mod uuid_v4_macro;

#[proc_macro_derive(UuidV4)]
pub fn derive_uuid_v4(input: TokenStream) -> TokenStream {
    uuid_v4_macro::derive(input)
}
