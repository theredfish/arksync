// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::grid::core::drop_preview::DropPreview;
use crate::components::grid::core::item::{GridItemData, GridPosition};
use crate::components::grid::core::layout::Layout;
use crate::components::grid::core::resize_preview::ResizePreview;
use crate::components::grid::core::size::Size;
use crate::components::grid::core::span::Span;
use crate::components::grid::grid_layout::GridDebugInfo;
use crate::components::grid::utils::draggable_item::{
    use_draggable_grid_item, UseDraggableGridItemOptions, UseDraggableGridItemReturn,
};
use crate::components::grid::utils::resizable_item::{
    use_resizable_grid_item, UseResizableGridItemOptions, UseResizableGridItemReturn,
};
use crate::components::heroicons::ResizeIcon;
use leptos::html::Div;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_use::core::Position;
use std::sync::Arc;
use std::time::Duration;

pub(crate) const GRID_ITEM_GAP_PX: f64 = 12.0;
pub(crate) const GRID_ITEM_INSET_PX: f64 = GRID_ITEM_GAP_PX / 2.0;
const RESIZE_APPLY_TRANSITION_MS: u64 = 220;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum ResizeApplyAnimation {
    #[default]
    Idle,
    Primed {
        translate_x: f64,
        translate_y: f64,
        scale_x: f64,
        scale_y: f64,
    },
    Running,
}

impl ResizeApplyAnimation {
    fn is_active(self) -> bool {
        !matches!(self, Self::Idle)
    }
}

fn resize_inverse_transform(
    from_position: Position,
    from_size: Size,
    to_position: Position,
    to_size: Size,
) -> (f64, f64, f64, f64) {
    let from_width = (from_size.width - GRID_ITEM_GAP_PX).max(1.0);
    let from_height = (from_size.height - GRID_ITEM_GAP_PX).max(1.0);
    let to_width = (to_size.width - GRID_ITEM_GAP_PX).max(1.0);
    let to_height = (to_size.height - GRID_ITEM_GAP_PX).max(1.0);

    (
        from_position.x - to_position.x,
        from_position.y - to_position.y,
        from_width / to_width,
        from_height / to_height,
    )
}

#[component]
pub fn GridItem(
    children: Children,
    id: u32,
    col_span: usize,
    row_span: usize,
    col_start: usize,
    row_start: usize,
    #[prop(optional)] label: String,
    /// If true, adds item at top and pushes others down. If false, registers at specified position.
    #[prop(optional, default = false)]
    dynamic: bool,
    #[prop(optional)] debug: Option<bool>,
) -> impl IntoView {
    let layout = use_context::<RwSignal<Layout>>().expect("should retrieve the layout context");
    let drop_preview = use_context::<RwSignal<Option<DropPreview>>>()
        .expect("Drop preview context must be provided");
    let resize_preview = use_context::<RwSignal<Option<ResizePreview>>>()
        .expect("Resize preview context must be provided");
    let untracked_layout = layout.get_untracked();
    let grid_item_ref = NodeRef::<Div>::new();
    let drag_ref = NodeRef::<Div>::new();
    let resize_button_ref = NodeRef::<Div>::new();
    let resize_apply_animation = RwSignal::new(ResizeApplyAnimation::Idle);
    let resize_animation_epoch = RwSignal::new(0_u32);
    let debug = debug.unwrap_or_else(|| {
        use_context::<GridDebugInfo>()
            .map(|debug| debug.0)
            .unwrap_or(false)
    });

    // Initializing the grid item data
    let Size {
        width: cell_w,
        height: cell_h,
    } = untracked_layout.cell_size;

    let grid_item_data = RwSignal::new(GridItemData {
        id,
        grid_pos: GridPosition {
            col_start,
            row_start,
        },
        px_pos: Position {
            x: col_start as f64 * cell_w,
            y: row_start as f64 * cell_h,
        },
        span: Span { row_span, col_span },
        size: Size {
            width: col_span as f64 * cell_w,
            height: row_span as f64 * cell_h,
        },
    });

    layout.update(|layout| {
        // Check if item is already registered (prevent duplicate registration)
        if layout.items.contains_key(&id) {
            return;
        }

        log!("Update the layout with item {id}");

        if dynamic {
            // Dynamic items push others down and go to top
            layout.add_item_at_top(grid_item_data);
        } else {
            // Declarative items register at their specified position
            layout.register_item(grid_item_data);
        }
    });

    // Drag events
    let draggable_options = UseDraggableGridItemOptions {
        handle: Some(drag_ref),
        col_start,
        row_start,
        col_span,
        current_col_span: Arc::new(move || grid_item_data.get_untracked().span.col_span),
        on_drag_move: Arc::new(move |drag_px_pos| {
            let layout = layout.get_untracked();
            let item = grid_item_data.get_untracked();
            let next_preview = DropPreview::from_drag(&item, drag_px_pos, &layout);

            if drop_preview.get_untracked() != Some(next_preview) {
                drop_preview.set(Some(next_preview));
            }

            grid_item_data.update(|item| {
                item.px_pos = drag_px_pos;
            })
        }),
        on_drag_end: Arc::new(move |col_start, row_start, _snapped_px_pos, drag_px_pos| {
            drop_preview.set(None);
            layout.update(|layout| {
                layout.move_item_with_collision(grid_item_data, row_start, col_start, drag_px_pos);
            });
        }),
        ..Default::default()
    };

    // Compute position from grid_item_data signal (source of truth from layout)
    let UseDraggableGridItemReturn {
        // We can't use this position because the layout is also pushing back the
        // elements and update the items position data. Using this position would
        // prevent the UI to update on the pixel position updated from the layout.
        // TODO: see to remove this from the UseDraggableGridItemReturn? Or keep for API if open sourced?
        // position: drag_position,
        transition: drag_transition,
        is_dragging,
        ..
    } = use_draggable_grid_item(grid_item_ref, draggable_options);

    // Grid item resize
    let resize_options = UseResizableGridItemOptions {
        handle: Some(resize_button_ref),
        col_span,
        row_span,
        current_col_start: Arc::new(move || grid_item_data.get_untracked().grid_pos.col_start),
        current_col_span: Arc::new(move || grid_item_data.get_untracked().span.col_span),
        current_row_span: Arc::new(move || grid_item_data.get_untracked().span.row_span),
        on_resize_start: Arc::new(move |_| {
            resize_animation_epoch.update(|epoch| *epoch = epoch.wrapping_add(1));
            resize_apply_animation.set(ResizeApplyAnimation::Idle);

            let item = grid_item_data.get_untracked();
            resize_preview.set(Some(ResizePreview::new(item.id, item.grid_pos, item.span)));
        }),
        on_resize_move: Arc::new(move |size| {
            let layout = layout.get_untracked();
            let item = grid_item_data.get_untracked();
            let next_preview = ResizePreview::from_resize(&item, size, &layout);

            if resize_preview.get_untracked() != Some(next_preview) {
                resize_preview.set(Some(next_preview));
            }
        }),
        on_resize_end: Arc::new(move |size| {
            let previous_item = grid_item_data.get_untracked();

            resize_animation_epoch.update(|epoch| *epoch = epoch.wrapping_add(1));
            let animation_epoch = resize_animation_epoch.get_untracked();

            layout.update(|layout| {
                let cell_size = layout.cell_size;
                let col_span = (size.width / cell_size.width).round() as usize;
                let row_span = (size.height / cell_size.height).round() as usize;

                layout.resize_item_with_collision(grid_item_data, col_span, row_span);
            });

            let final_item = grid_item_data.get_untracked();
            let (translate_x, translate_y, scale_x, scale_y) = resize_inverse_transform(
                previous_item.px_pos,
                previous_item.size,
                final_item.px_pos,
                final_item.size,
            );
            let should_animate = translate_x.abs() > 0.01
                || translate_y.abs() > 0.01
                || (scale_x - 1.0).abs() > 0.0001
                || (scale_y - 1.0).abs() > 0.0001;

            if should_animate {
                // FLIP: the layout has been committed exactly once. Render the
                // final panel at its previous visual bounds for the first frame.
                resize_apply_animation.set(ResizeApplyAnimation::Primed {
                    translate_x,
                    translate_y,
                    scale_x,
                    scale_y,
                });

                // After collision/compaction, the preview becomes the destination
                // outline and stays underneath the animating panel.
                resize_preview.update(|preview| {
                    if let Some(preview) = preview.as_mut() {
                        preview.grid_pos = final_item.grid_pos;
                        preview.span = final_item.span;
                    }
                });
            }

            if should_animate {
                // Keep the primed transform for one painted frame. The second
                // frame starts a compositor-only transition to the committed size.
                request_animation_frame(move || {
                    request_animation_frame(move || {
                        if resize_animation_epoch.get_untracked() != animation_epoch {
                            return;
                        }

                        resize_apply_animation.set(ResizeApplyAnimation::Running);

                        set_timeout(
                            move || {
                                if resize_animation_epoch.get_untracked() == animation_epoch {
                                    resize_apply_animation.set(ResizeApplyAnimation::Idle);
                                    resize_preview.set(None);
                                }
                            },
                            Duration::from_millis(RESIZE_APPLY_TRANSITION_MS + 40),
                        );
                    });
                });
            } else {
                resize_preview.set(None);
                resize_apply_animation.set(ResizeApplyAnimation::Idle);
            }
        }),
        on_resize_cancel: Arc::new(move || {
            resize_animation_epoch.update(|epoch| *epoch = epoch.wrapping_add(1));
            resize_apply_animation.set(ResizeApplyAnimation::Idle);
            resize_preview.set(None);
        }),
        ..Default::default()
    };

    let UseResizableGridItemReturn { size: resize_size } =
        use_resizable_grid_item(grid_item_ref, resize_options);

    let style = move || {
        // let Size { width, height } = metadata.get().size;
        let drag_transition = drag_transition.get();
        let resize_apply_animation = resize_apply_animation.get();
        let Position { x: left, y: top } = grid_item_data.get().px_pos;
        // let Position { x: left, y: top } = drag_position.get();
        let Size { width, height } = grid_item_data.get().size;
        let visual_width = (width - GRID_ITEM_GAP_PX).max(0.0);
        let visual_height = (height - GRID_ITEM_GAP_PX).max(0.0);
        let visual_left = left + GRID_ITEM_INSET_PX;
        let visual_top = top + GRID_ITEM_INSET_PX;
        let z_index = if is_dragging.get() || resize_apply_animation.is_active() {
            1000
        } else {
            1
        };
        let (resize_transform, resize_transition, resize_will_change) =
            match resize_apply_animation {
                ResizeApplyAnimation::Idle => ("none".to_owned(), "transform 0ms linear", "auto"),
                ResizeApplyAnimation::Primed {
                    translate_x,
                    translate_y,
                    scale_x,
                    scale_y,
                } => (
                    format!(
                        "translate3d({translate_x:.3}px, {translate_y:.3}px, 0) scale3d({scale_x:.6}, {scale_y:.6}, 1)"
                    ),
                    "transform 0ms linear",
                    "transform",
                ),
                ResizeApplyAnimation::Running => (
                    "translate3d(0, 0, 0) scale3d(1, 1, 1)".to_owned(),
                    "transform 220ms cubic-bezier(0.22, 1, 0.36, 1)",
                    "transform",
                ),
            };

        format!(
            r#"width: {visual_width}px;
            height: {visual_height}px;
            transition: {drag_transition}, {resize_transition};
            transform: {resize_transform};
            transform-origin: top left;
            will-change: {resize_will_change};
            touch-action: none;
            left: {visual_left}px;
            top: {visual_top}px;
            z-index: {z_index};"#
        )
    };

    view! {
        <div
            node_ref=grid_item_ref
            style={style}
            class="arksync-panel absolute flex cursor-move flex-col overflow-hidden rounded-lg"
            data-id=id.to_string()
        >
            <div node_ref=drag_ref class="w-full border-b border-[var(--arksync-panel-border)] px-4 py-3 font-mono text-[10px] uppercase tracking-[0.18em] text-[var(--arksync-text-muted)]">
                { label }
            </div>
            {move || {
                debug.then(|| {
                    view! {
                        <div class="px-4 py-2 text-xs text-[var(--arksync-text-muted)]">
                            { move || {
                                let Size { width, height } = resize_size.get();
                                let GridPosition { col_start, row_start } = grid_item_data.get().grid_pos;
                                let Position { x: left, y: top } = grid_item_data.get().px_pos;
                                format!("position: {col_start};{row_start} | size: {width};{height} | left/top: : {left}; {top}")
                            }}
                        </div>
                    }
                })
            }}
            <div class="min-h-0 flex-1">
                { children() }
            </div>
            <div
                node_ref=resize_button_ref
                class="absolute bottom-0 right-0 cursor-se-resize"
                data-id=id.to_string()
            >
                <ResizeIcon class="h-6 w-6 text-[var(--arksync-text-muted)]" />
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_resize_scale_preserves_the_previous_visual_bounds() {
        let from_position = Position { x: 50.0, y: 20.0 };
        let to_position = Position { x: 10.0, y: 5.0 };
        let from_size = Size {
            width: 300.0,
            height: 200.0,
        };
        let to_size = Size {
            width: 500.0,
            height: 400.0,
        };
        let (translate_x, translate_y, scale_x, scale_y) =
            resize_inverse_transform(from_position, from_size, to_position, to_size);

        assert_eq!(translate_x, 40.0);
        assert_eq!(translate_y, 15.0);
        assert!(
            (scale_x * (to_size.width - GRID_ITEM_GAP_PX) - (from_size.width - GRID_ITEM_GAP_PX))
                .abs()
                < 0.001
        );
        assert!(
            (scale_y * (to_size.height - GRID_ITEM_GAP_PX) - (from_size.height - GRID_ITEM_GAP_PX))
                .abs()
                < 0.001
        );
    }
}
