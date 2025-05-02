use crate::components::grid::{GridBounding, GridContext};
use leptos::{html::Div, logging::log, prelude::*};
use leptos_use::{use_element_bounding, UseElementBoundingReturn};
use std::collections::HashMap;

#[component]
pub fn Grid(children: Children) -> impl IntoView {
    let grid_layout = NodeRef::<Div>::new();
    let grid_context = GridContext {
        storage: HashMap::new(),
        boundaries: GridBounding {
            width: 0.,
            height: 0.,
        },
    };
    let grid_ctx = RwSignal::new(grid_context);

    provide_context(grid_ctx);

    Effect::new(move || {
        let UseElementBoundingReturn { width, height, .. } = use_element_bounding(grid_layout);
        grid_ctx.update(|ctx| {
            ctx.boundaries = GridBounding {
                width: width.get_untracked(),
                height: height.get_untracked(),
            };
        });
    });

    // Effect::watch(
    //     move || grid_ctx.get(),
    //     move |ctx: &GridContext, _prev, _| log!("[Grid] context: {ctx:#?}"),
    //     false,
    // );

    view! {
        <div node_ref=grid_layout class="relative h-full border border-red-300">
            { children() }
        </div>
    }
}
