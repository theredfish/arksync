use crate::components::grid::{LayoutBuilder, Size};
use leptos::{html::Div, prelude::*};
use leptos_use::{
    use_element_bounding_with_options, UseElementBoundingOptions, UseElementBoundingReturn,
};

#[component]
pub fn GridLayout(children: Children, columns: u8) -> impl IntoView {
    let layout = RwSignal::new(LayoutBuilder::default().columns(columns).build());
    let last_size = RwSignal::new(Size::default());
    let grid_layout_node = NodeRef::<Div>::new();

    provide_context(layout);

    let UseElementBoundingReturn { width, height, .. } = use_element_bounding_with_options(
        grid_layout_node,
        UseElementBoundingOptions {
            reset: false,
            immediate: false,
            ..Default::default()
        },
    );

    Effect::new(move || {
        layout.update(|layout| {
            layout.size = Size {
                width: width.get(),
                height: height.get(),
            };
        });
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
            let prev_size = last_size.get_untracked();

            // width ratio
            if (width - last_w).abs() >= 50.0 {
                last_size.update(|last_size| {
                    last_size.width = *width;
                });

                if prev_size.width == 0. {
                    return;
                }

                let curr_size = last_size.get_untracked();
                let w_ratio = curr_size.width / prev_size.width;
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

                if prev_size.height == 0. {
                    return;
                }

                let curr_size = last_size.get_untracked();
                let h_ratio = curr_size.height / prev_size.height;
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
