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
