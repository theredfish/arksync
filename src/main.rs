mod app;
mod components;
mod theme;

use app::*;
use leptos::prelude::{mount_to_body, view};

use crate::theme::register_theme;

fn main() {
    console_error_panic_hook::set_once();

    register_theme();

    mount_to_body(|| {
        view! {
            <App/>
        }
    })
}
