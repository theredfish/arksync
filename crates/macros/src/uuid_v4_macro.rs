// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;

    let valid_shape = matches!(
        input.data,
        Data::Struct(ref data)
            if matches!(
                data.fields,
                Fields::Unnamed(ref fields) if fields.unnamed.len() == 1
            )
    );

    if !valid_shape {
        return syn::Error::new_spanned(
            ident,
            "UuidV4 can only be derived for tuple structs with one [u8; 16] field",
        )
        .to_compile_error()
        .into();
    }

    quote! {
        impl #ident {
            #[cfg(feature = "uuid-v4")]
            pub fn new() -> Self {
                Self::new_with_uuid(::arksync_utils::uuid::new_v4())
            }

            pub fn new_with_uuid(uuid: ::arksync_utils::uuid::Uuid) -> Self {
                Self(*uuid.as_bytes())
            }

            pub fn new_with_random_bytes(bytes: [u8; 16]) -> Self {
                Self::new_with_uuid(::arksync_utils::uuid::from_random_bytes(bytes))
            }

            pub fn as_uuid(&self) -> ::arksync_utils::uuid::Uuid {
                ::arksync_utils::uuid::Uuid::from_bytes(self.0)
            }

            pub fn as_bytes(&self) -> &[u8; 16] {
                &self.0
            }
        }

        impl From<::arksync_utils::uuid::Uuid> for #ident {
            fn from(value: ::arksync_utils::uuid::Uuid) -> Self {
                Self::new_with_uuid(value)
            }
        }

        impl Clone for #ident {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl Copy for #ident {}

        impl core::fmt::Debug for #ident {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_tuple(stringify!(#ident)).field(&self.as_uuid()).finish()
            }
        }

        impl core::fmt::Display for #ident {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Display::fmt(&self.as_uuid(), f)
            }
        }

        impl PartialEq for #ident {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl Eq for #ident {}

        impl core::hash::Hash for #ident {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                core::hash::Hash::hash(&self.0, state);
            }
        }

        impl ::serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                ::serde::Serialize::serialize(&self.0, serializer)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                <[u8; 16] as ::serde::Deserialize>::deserialize(deserializer).map(Self)
            }
        }
    }
    .into()
}
