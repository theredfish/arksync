mod app;
mod components;
mod core;

use app::*;
use leptos::prelude::{mount_to_body, view};

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <App/>
        }
    })
}
