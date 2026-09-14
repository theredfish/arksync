// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::grid::core::drop_preview::DropPreview;
use crate::components::grid::core::layout::LayoutBuilder;
use crate::components::grid::core::resize_preview::ResizePreview;
use crate::components::grid::core::size::Size;
use crate::components::grid::grid_item::{GridItem, GRID_ITEM_GAP_PX, GRID_ITEM_INSET_PX};
use crate::components::page_layout::PageLayout;
use leptos::{html::Div, logging::log, prelude::*};
use leptos_use::{core::Position, use_element_size, UseElementSizeReturn};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GridDebugInfo(pub bool);

#[component]
pub fn GridLayout(
    children: Children,
    columns: usize,
    display_grid: bool,
    #[prop(optional, default = false)] debug: bool,
) -> impl IntoView {
    assert!(columns > 0, "The number of columns can't be zero");
    let grid_layout_node = NodeRef::<Div>::new();
    let UseElementSizeReturn { width, height } = use_element_size(grid_layout_node);
    let layout = RwSignal::new(
        LayoutBuilder::default()
            .columns(columns)
            .cell_size(100., 100.)
            .build(),
    );
    let drop_preview = RwSignal::new(None::<DropPreview>);
    let resize_preview = RwSignal::new(None::<ResizePreview>);

    provide_context(layout);
    provide_context(drop_preview);
    provide_context(resize_preview);
    provide_context(GridDebugInfo(debug));

    // Track dynamically added items
    let next_id = RwSignal::new(10_000u32);
    let grid_items = RwSignal::new(Vec::<u32>::new());

    // Handler to add new items
    let add_item = move |_| {
        let id = next_id.get_untracked();
        next_id.update(|n| *n += 1);
        grid_items.update(|items| items.push(id));
    };

    Effect::watch(
        move || (width.get(), height.get()),
        move |(width, height): &(f64, f64), _, _| {
            log!("Detected size change");
            // Update the cell size
            layout.update(|layout| {
                let cell_width = *width / columns as f64;
                layout.size = Size {
                    width: *width,
                    height: *height,
                };
                layout.cell_size = Size {
                    width: cell_width,
                    height: cell_width, // Same as width for square cells
                };
                layout.sync_items_to_grid();
            });
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
        <PageLayout
            eyebrow="Dashboards"
            title="Station overview"
            actions=move || view! {
                <button
                    on:click=add_item
                    class="rounded-md border border-[var(--arksync-action-border)] bg-[var(--arksync-primary)] px-4 py-2 font-mono text-xs uppercase tracking-[0.18em] text-[var(--arksync-primary-text)] shadow-sm transition-colors hover:bg-[var(--arksync-primary-hover)]"
                >
                    "Add Item"
                </button>
            }
        >
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
                                            class=move || format!("absolute {border_t_b} {border_l_r} border-[var(--arksync-panel-border)]")
                                            style={ move || {
                                                format!("left: {}px; top: {}px; width: {}px; height: {}px;", col * cell_w as usize, row * cell_h as usize, cell_w, cell_h)
                                            }}
                                        >
                                            {
                                                if row == 0 {
                                                    Some(view! { <span class="text-xs text-[var(--arksync-text-muted)]">{format!("{col}")}</span> })
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
                {
                    move || {
                        drop_preview.get().map(|preview| {
                            let layout = layout.get();
                            let (Position { x: left, y: top }, Size { width, height }) =
                                preview.pixel_rect(layout.cell_size);
                            let visual_left = left + GRID_ITEM_INSET_PX;
                            let visual_top = top + GRID_ITEM_INSET_PX;
                            let visual_width = (width - GRID_ITEM_GAP_PX).max(0.0);
                            let visual_height = (height - GRID_ITEM_GAP_PX).max(0.0);

                            view! {
                                <div
                                    class="pointer-events-none absolute rounded-lg border border-dashed border-[var(--arksync-preview-border)] bg-[var(--arksync-preview-bg)]"
                                    data-preview-for=preview.item_id.to_string()
                                    style=format!(
                                        "left: {visual_left}px; top: {visual_top}px; width: {visual_width}px; height: {visual_height}px; z-index: 999; transition: left 120ms ease-out, top 120ms ease-out, width 120ms ease-out, height 120ms ease-out;"
                                    )
                                />
                            }
                        })
                    }
                }
                {
                    let resize_preview_style = move || {
                        let Some(preview) = resize_preview.get() else {
                            return "left: 0; top: 0; width: 0; height: 0; opacity: 0; visibility: hidden; transform: none; transition: none; z-index: 999;".to_owned();
                        };

                        let layout = layout.get();
                        let source_span = preview.initial_span;
                        let (Position { x: left, y: top }, _) =
                            preview.pixel_rect(layout.cell_size);
                        let source_width = source_span.col_span as f64 * layout.cell_size.width;
                        let source_height = source_span.row_span as f64 * layout.cell_size.height;
                        let target_width = preview.span.col_span as f64 * layout.cell_size.width;
                        let target_height = preview.span.row_span as f64 * layout.cell_size.height;
                        let visual_left = left + GRID_ITEM_INSET_PX;
                        let visual_top = top + GRID_ITEM_INSET_PX;
                        let visual_source_width = (source_width - GRID_ITEM_GAP_PX).max(1.0);
                        let visual_source_height = (source_height - GRID_ITEM_GAP_PX).max(1.0);
                        let visual_target_width = (target_width - GRID_ITEM_GAP_PX).max(1.0);
                        let visual_target_height = (target_height - GRID_ITEM_GAP_PX).max(1.0);
                        let scale_x = visual_target_width / visual_source_width;
                        let scale_y = visual_target_height / visual_source_height;

                        format!(
                            "left: {visual_left}px; top: {visual_top}px; width: {visual_source_width}px; height: {visual_source_height}px; opacity: 1; visibility: visible; z-index: 999; transform-origin: top left; transform: translateZ(0) scale3d({scale_x:.6}, {scale_y:.6}, 1); transition: transform 140ms cubic-bezier(0.22, 1, 0.36, 1), opacity 80ms linear; will-change: transform; backface-visibility: hidden;"
                        )
                    };

                    view! {
                        <div
                            class="pointer-events-none absolute rounded-lg border border-dashed border-[var(--arksync-preview-border)] bg-[var(--arksync-preview-bg)]"
                            aria-hidden="true"
                            data-resize-preview-for=move || {
                                resize_preview
                                    .get()
                                    .map(|preview| preview.item_id.to_string())
                                    .unwrap_or_default()
                            }
                            style=resize_preview_style
                        />
                    }
                }
                <For
                    each=move || grid_items.get()
                    key=|id| *id
                    children=move |id| {
                        view! {
                            <GridItem
                                id=id
                                col_span=3
                                row_span=3
                                col_start=0
                                row_start=0
                                label=format!("Item {}", id)
                                dynamic=true
                                debug=debug
                            >
                                <div class="p-4 text-[var(--arksync-text-muted)]">
                                    "No data yet"
                                </div>
                            </GridItem>
                        }
                    }
                />
            </div>
        </PageLayout>
    }
}
