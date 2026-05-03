// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod app;
mod components;
mod theme;

use app::*;
use leptos::prelude::{mount_to_body, view};

use crate::theme::{register_theme, ArkSyncTheme};

fn main() {
    console_error_panic_hook::set_once();

    let Ok(_) = register_theme(vec![
        ArkSyncTheme::Westeros,
        ArkSyncTheme::Chalk,
        ArkSyncTheme::Roma,
        ArkSyncTheme::Wonderland,
        ArkSyncTheme::Walden,
    ]) else {
        panic!("Failed to initialize the echarts themes");
    };

    mount_to_body(|| {
        view! {
            <App/>
        }
    })
}
