// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, Ident, ItemFn, Pat, PatIdent, PatType, Type};

const PG_POOL_TYPE: &str = "PgPool";

pub fn expand(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);

    if item.sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            item.sig.ident,
            "#[arksync_testing::test] only supports async test functions",
        )
        .to_compile_error()
        .into();
    }

    let mut pool_ident = None;
    let mut unsupported_input = None;

    for input in item.sig.inputs.iter() {
        if is_pg_pool_input(input) {
            if pool_ident.is_some() {
                return syn::Error::new_spanned(
                    input,
                    "#[arksync_testing::test] supports a single PgPool argument",
                )
                .to_compile_error()
                .into();
            }

            let Some(ident) = input_ident(input) else {
                return syn::Error::new_spanned(
                    input,
                    "PgPool argument must use a simple identifier pattern",
                )
                .to_compile_error()
                .into();
            };

            pool_ident = Some(ident);
            continue;
        }

        unsupported_input = Some(input.clone());
    }

    if let Some(input) = unsupported_input {
        return syn::Error::new_spanned(
            input,
            "#[arksync_testing::test] only supports an optional PgPool argument",
        )
        .to_compile_error()
        .into();
    }

    let fn_name = item.sig.ident.clone();
    let pool_setup = pool_ident.as_ref().map(|ident| {
        quote! {
            let __arksync_test_db = ::arksync_testing::TestDatabase::setup(
                concat!(module_path!(), "::", stringify!(#fn_name))
            )
            .await
            .expect("failed to set up ArkSync test database");
            let #ident = __arksync_test_db.pool().clone();
        }
    });
    let pool_cleanup = pool_ident.as_ref().map(|_| {
        quote! {
            if ::arksync_testing::test_succeeded(&__arksync_test_result) {
                __arksync_test_db
                    .teardown()
                    .await
                    .expect("failed to tear down ArkSync test database");
            }
        }
    });

    let attrs = item.attrs;
    let vis = item.vis;
    let sig = item.sig;
    let block = item.block;
    let name = &sig.ident;
    let output = &sig.output;

    quote! {
        #[::arksync_testing::tokio::test]
        #(#attrs)*
        #vis async fn #name() #output {
            #pool_setup
            let __arksync_test_result = { #block };
            #pool_cleanup
            __arksync_test_result
        }
    }
    .into()
}

fn is_pg_pool_input(input: &FnArg) -> bool {
    let FnArg::Typed(PatType { ty, .. }) = input else {
        return false;
    };
    let Type::Path(type_path) = ty.as_ref() else {
        return false;
    };

    type_path
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == PG_POOL_TYPE)
}

fn input_ident(input: &FnArg) -> Option<Ident> {
    let FnArg::Typed(PatType { pat, .. }) = input else {
        return None;
    };
    let Pat::Ident(PatIdent { ident, .. }) = pat.as_ref() else {
        return None;
    };

    Some(ident.clone())
}
