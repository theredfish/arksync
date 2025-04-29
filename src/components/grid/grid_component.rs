use leptos::{logging::log, prelude::*};
use std::collections::HashMap;

use crate::components::grid::GridStorage;

#[component]
pub fn Grid(children: Children) -> impl IntoView {
    let grid_storage = GridStorage {
        items: HashMap::new(),
    };
    let update_grid = RwSignal::new(grid_storage);

    provide_context(update_grid);

    Effect::watch(
        move || update_grid.get(),
        move |storage: &GridStorage, _prev, _| log!("[Grid] storage: {storage:#?}"),
        false,
    );

    view! {
        <div class="relative h-full border border-red-300">
            { children() }
        </div>
    }
}
