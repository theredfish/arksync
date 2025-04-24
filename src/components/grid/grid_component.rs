use std::collections::HashMap;

use leptos::{logging::log, prelude::*};

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
        <div class="h-full grid grid-cols-12 grid-rows-[repeat(auto-fill,minmax(50px,auto))] gap-4">
            { children() }
        </div>
    }
}
