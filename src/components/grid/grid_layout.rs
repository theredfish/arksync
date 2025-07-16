use crate::components::grid::{LayoutBuilder, Size};
use leptos::{html::Div, prelude::*};
use leptos_use::{use_element_bounding, UseElementBoundingReturn};

#[component]
pub fn GridLayout(children: Children, columns: u32, display_grid: bool) -> impl IntoView {
    assert!(columns > 0, "The number of columns can't be zero");
    let grid_layout_node = NodeRef::<Div>::new();
    let last_size = RwSignal::new(Size::default());
    let UseElementBoundingReturn { width, height, .. } = use_element_bounding(grid_layout_node);
    let layout = RwSignal::new(
        LayoutBuilder::default()
            .columns(columns)
            .cell_size(100., 100.)
            .build(),
    );

    provide_context(layout);

    // UseElementBoundingReturn previous value for the width and the height
    // might be the same as the current one. For this reason we need to track
    // the last size value with a gap.
    Effect::watch(
        move || (width.get(), height.get()),
        move |(width, height): &(f64, f64), _, _| {
            // Update the cell size
            let border_pixels = columns as f64;
            layout.update(|layout| {
                layout.cell_size = Size {
                    width: (*width / (columns) as f64) - border_pixels,
                    height: 100.,
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
        let rows = (height.get() / cell_h).floor() as u32;

        (0..columns)
            .flat_map(move |x| (0..rows).map(move |y| (x, y)))
            .collect::<Vec<_>>()
    });

    view! {
        <div node_ref=grid_layout_node class="relative h-full">
            {
                move || {
                    if display_grid {
                        Some(draw_grid
                            .get()
                            .into_iter()
                            .map(|(col, row)| {
                                let Size { width: cell_w, height: cell_h } = layout.get().cell_size;
                                let rows = (height.get() / cell_h).floor() as u32;
                                let border_t_b = if row == rows - 1 { "border-t border-b" } else { "border-t" };
                                let border_l_r = if col == columns -1 { "border-l border-r" } else { "border-l" };

                                view! {
                                    <div
                                        class=move || format!("absolute {border_t_b} {border_l_r} border-gray-800")
                                        style={ move || {
                                            format!("left: {}px; top: {}px; width: {}px; height: {}px;", col * cell_w as u32, row * cell_h as u32, cell_w, cell_h)
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
            {children()}
        </div>
    }
}
