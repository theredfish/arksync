use crate::components::grid::{LayoutBuilder, Size};
use leptos::{html::Div, logging::log, prelude::*};
use leptos_use::{
    use_element_bounding, use_element_bounding_with_options, UseElementBoundingOptions,
    UseElementBoundingReturn,
};

#[component]
pub fn GridLayout(children: Children, columns: u8) -> impl IntoView {
    let grid_layout_node = NodeRef::<Div>::new();
    let last_size = RwSignal::new(Size::default());
    let layout = RwSignal::new(LayoutBuilder::default().build());

    provide_context(layout);

    let UseElementBoundingReturn { width, height, .. } = use_element_bounding(grid_layout_node);

    Effect::new(move || {
        layout.set(
            LayoutBuilder::default()
                .size(width.get(), height.get())
                .columns(columns)
                .cell_size(100., 100.)
                .build(),
        );
    });

    // UseElementBoundingReturn previous value for the width and the height
    // might be the same as the current one. For this reason we need to track
    // the last size value with a gap.
    Effect::watch(
        move || (width.get(), height.get()),
        move |(width, height): &(f64, f64), _, _| {
            let Size {
                width: last_w,
                height: last_h,
            } = last_size.get_untracked();
            log!("last width: {last_w} | width: {width}");

            // width ratio
            if (width - last_w).abs() >= 50.0 {
                last_size.update(|last_size| {
                    last_size.width = *width;
                });

                if last_w == 0. {
                    return;
                }

                let curr_size = last_size.get_untracked();
                let w_ratio = curr_size.width / last_w;
                let ratio = (w_ratio, 1.0);

                layout.update(|layout| {
                    layout.update_items_size(ratio);
                });
            }

            // height ratio
            if (height - last_h).abs() >= 50.0 {
                last_size.update(|last_size| {
                    last_size.height = *height;
                });

                if last_h == 0. {
                    return;
                }

                let curr_size = last_size.get_untracked();
                let h_ratio = curr_size.height / last_h;
                let ratio = (1.0, h_ratio);

                layout.update(|layout| {
                    layout.update_items_size(ratio);
                });
            }
        },
        false,
    );

    view! {
        <div node_ref=grid_layout_node class="relative h-full border border-red-300">
            { children() }
        </div>
    }
}
