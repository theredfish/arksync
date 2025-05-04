use crate::components::grid::{GridBounding, GridContext};
use leptos::{html::Div, prelude::*};
use leptos_use::{use_element_bounding, UseElementBoundingReturn};
use std::collections::HashMap;

#[component]
pub fn GridLayout(children: Children) -> impl IntoView {
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

    let UseElementBoundingReturn { width, height, .. } = use_element_bounding(grid_layout);
    Effect::watch(
        move || (width.get(), height.get()),
        move |(width, height): &(f64, f64), _, _| {
            grid_ctx.update(|ctx| {
                ctx.boundaries = GridBounding {
                    width: *width,
                    height: *height,
                };
            });
        },
        false,
    );

    view! {
        <div node_ref=grid_layout class="relative h-full border border-red-300">
            { children() }
        </div>
    }
}
