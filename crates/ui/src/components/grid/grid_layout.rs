use crate::components::grid::core::layout::LayoutBuilder;
use crate::components::grid::core::size::Size;
use crate::components::grid::grid_item::GridItem;
use leptos::{html::Div, logging::log, prelude::*};
use leptos_use::{
    use_element_bounding_with_options, UseElementBoundingOptions, UseElementBoundingReturn,
};

#[component]
pub fn GridLayout(children: Children, columns: usize, display_grid: bool) -> impl IntoView {
    assert!(columns > 0, "The number of columns can't be zero");
    let grid_layout_node = NodeRef::<Div>::new();
    let last_size = RwSignal::new(Size::default());
    let UseElementBoundingReturn { width, height, .. } = use_element_bounding_with_options(
        grid_layout_node,
        UseElementBoundingOptions::default().immediate(true),
    );
    let layout = RwSignal::new(
        LayoutBuilder::default()
            .columns(columns)
            .cell_size(100., 100.)
            .build(),
    );

    provide_context(layout);

    // Track dynamically added items
    let next_id = RwSignal::new(1u32);
    let grid_items = RwSignal::new(Vec::<u32>::new());

    // Handler to add new items
    let add_item = move |_| {
        let id = next_id.get_untracked();
        next_id.update(|n| *n += 1);
        grid_items.update(|items| items.push(id));
    };

    // UseElementBoundingReturn previous value for the width and the height
    // might be the same as the current one. For this reason we need to track
    // the last size value with a gap.
    Effect::watch(
        move || (width.get(), height.get()),
        move |(width, height): &(f64, f64), _, _| {
            log!("Detected size change");
            // Update the cell size
            let border_pixels = columns as f64;
            layout.update(|layout| {
                let cell_width = (*width / (columns) as f64) - border_pixels;
                layout.cell_size = Size {
                    width: cell_width,
                    height: cell_width, // Same as width for square cells
                }
            });

            let Size {
                width: last_w,
                height: last_h,
            } = last_size.get_untracked();

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
                    // TODO: items responsiveness and layout position
                    // and DO IT ONCE not after each ratio calculation
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

    let draw_grid = Memo::new(move |_| {
        let layout = layout.get();
        let Size { height: cell_h, .. } = layout.cell_size;
        let rows = (height.get() / cell_h).floor() as usize;

        (0..columns)
            .flat_map(move |x| (0..rows).map(move |y| (x, y)))
            .collect::<Vec<_>>()
    });

    view! {
        <div class="flex flex-col h-full">
            // Header section for action buttons
            <div class="flex items-center gap-2 px-4 py-3 bg-darcula-gray border-b border-gray-700">
                <button
                    on:click=add_item
                    class="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded shadow text-sm"
                >
                    "Add Item"
                </button>
                // Space for future action buttons
            </div>

            // Grid content area
            <div class="flex-1 relative overflow-auto">
                <div node_ref=grid_layout_node class="relative h-full">
                {
                    move || {
                        if display_grid {
                            Some(draw_grid
                                .get()
                                .into_iter()
                                .map(|(col, row)| {
                                    let Size { width: cell_w, height: cell_h } = layout.get().cell_size;
                                    let rows = (height.get() / cell_h).floor() as usize;
                                    let border_t_b = if row == rows - 1 { "border-t border-b" } else { "border-t" };
                                    let border_l_r = if col == columns -1 { "border-l border-r" } else { "border-l" };

                                    view! {
                                        <div
                                            class=move || format!("absolute {border_t_b} {border_l_r} border-gray-800")
                                            style={ move || {
                                                format!("left: {}px; top: {}px; width: {}px; height: {}px;", col * cell_w as usize, row * cell_h as usize, cell_w, cell_h)
                                            }}
                                        >
                                            {
                                                if row == 0 {
                                                    Some(view! { <span class="text-xs text-gray-400">{format!("{col}")}</span> })
                                                } else {
                                                    None
                                                }
                                            }
                                        </div>
                                    }
                                }).collect_view())
                        } else {
                            None
                        }
                    }
                }
                // {children()}
                <For
                    each=move || grid_items.get()
                    key=|id| *id
                    children=move |id| {
                        view! {
                            <GridItem
                                id=id
                                col_span=3
                                row_span=3
                                // FIXME: this is just for debugging
                                col_start=1
                                row_start=0
                                label=format!("Item {}", id)
                                dynamic=true
                            >
                                <div class="p-4 text-gray-500">
                                    "No data yet"
                                </div>
                            </GridItem>
                        }
                    }
                />
                </div>
            </div>
        </div>
    }
}
