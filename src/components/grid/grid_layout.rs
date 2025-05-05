use crate::components::grid::{Layout, LayoutBuilder, Size};
use leptos::{html::Div, prelude::*};
use leptos_use::{use_element_bounding, UseElementBoundingReturn};

#[component]
pub fn GridLayout(children: Children, columns: u8) -> impl IntoView {
    let grid_layout = NodeRef::<Div>::new();
    let layout_ctx = RwSignal::new(LayoutBuilder::default().columns(columns).build());
    provide_context(layout_ctx);

    let UseElementBoundingReturn { width, height, .. } = use_element_bounding(grid_layout);
    Effect::watch(
        move || (width.get(), height.get()),
        move |(width, height): &(f64, f64), _, _| {
            layout_ctx.update(|layout| {
                layout.size.set(Size {
                    width: *width,
                    height: *height,
                })
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
