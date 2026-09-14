// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use leptos::prelude::*;
use leptos::{html::Div, prelude::NodeRef};
use leptos_use::use_event_listener;
use std::default::Default;
use std::sync::Arc;
use wasm_bindgen::JsCast;

use crate::components::grid::core::layout::Layout;
use crate::components::grid::core::resize_preview::snap_resize_span;
use crate::components::grid::core::size::Size;

fn clamp_size_to_grid(layout: &Layout, col_start: usize, size: Size) -> Size {
    let cell_size = layout.cell_size;
    let max_col_span = layout.columns.saturating_sub(col_start).max(1);
    let max_width = max_col_span as f64 * cell_size.width;

    Size {
        width: size.width.clamp(cell_size.width, max_width),
        height: size.height.max(cell_size.height),
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ResizeState {
    Idle {
        last_item_size: Size,
    },
    Resizing {
        start_pos: (i32, i32),
        offset_x: i32,
        offset_y: i32,
        last_item_size: Size,
    },
    Ended {
        start_pos: (i32, i32),
        total_offset_x: i32,
        total_offset_y: i32,
        last_item_size: Size,
    },
}

impl ResizeState {
    fn last_item_size(&self) -> Size {
        match self {
            ResizeState::Idle { last_item_size } => *last_item_size,
            ResizeState::Resizing { last_item_size, .. } => *last_item_size,
            ResizeState::Ended { last_item_size, .. } => *last_item_size,
        }
    }
}

pub struct UseResizableGridItemOptions {
    /// The resize handle element, defaults to target
    pub handle: Option<NodeRef<Div>>,
    pub col_span: usize,
    pub row_span: usize,
    pub current_col_start: Arc<dyn Fn() -> usize + Send + Sync>,
    pub current_col_span: Arc<dyn Fn() -> usize + Send + Sync>,
    pub current_row_span: Arc<dyn Fn() -> usize + Send + Sync>,
    pub on_resize_start: Arc<dyn Fn(Size) + Send + Sync>,
    pub on_resize_move: Arc<dyn Fn(Size) + Send + Sync>,
    pub on_resize_end: Arc<dyn Fn(Size) + Send + Sync>,
    pub on_resize_cancel: Arc<dyn Fn() + Send + Sync>,
}

impl Default for UseResizableGridItemOptions {
    fn default() -> Self {
        Self {
            handle: None,
            col_span: 1,
            row_span: 1,
            current_col_start: Arc::new(|| 0),
            current_col_span: Arc::new(|| 1),
            current_row_span: Arc::new(|| 1),
            on_resize_start: Arc::new(|_| {}),
            on_resize_move: Arc::new(|_| {}),
            on_resize_end: Arc::new(|_| {}),
            on_resize_cancel: Arc::new(|| {}),
        }
    }
}

pub struct UseResizableGridItemReturn {
    /// Computed size in pixels of the grid item
    pub size: Signal<Size>,
}

pub fn use_resizable_grid_item(
    target: NodeRef<Div>,
    options: UseResizableGridItemOptions,
) -> UseResizableGridItemReturn {
    let layout = use_context::<RwSignal<Layout>>().expect("Layout context must be provided");
    let resize_state = RwSignal::new(ResizeState::Idle {
        last_item_size: Size::default(),
    });

    let UseResizableGridItemOptions {
        handle,
        col_span,
        row_span,
        current_col_start,
        current_col_span,
        current_row_span,
        on_resize_start,
        on_resize_move,
        on_resize_end,
        on_resize_cancel,
        ..
    } = options;

    Effect::new(move || {
        let Size {
            width: cell_w,
            height: cell_h,
        } = layout.get_untracked().cell_size;

        let item_size = Size {
            width: col_span as f64 * cell_w,
            height: row_span as f64 * cell_h,
        };

        resize_state.set(ResizeState::Idle {
            last_item_size: item_size,
        });
    });

    let handle = handle.unwrap_or(target);

    let current_col_start_for_move = Arc::clone(&current_col_start);
    let current_col_start_for_end = Arc::clone(&current_col_start);
    let current_col_start_for_size = Arc::clone(&current_col_start);
    let current_col_span_for_layout = Arc::clone(&current_col_span);
    let current_row_span_for_layout = Arc::clone(&current_row_span);

    let _resize_starts = use_event_listener(handle, leptos::ev::pointerdown, move |evt| {
        evt.prevent_default();
        if let Some(target) = evt.current_target() {
            let target: web_sys::Element = target.unchecked_into();
            let _ = target.set_pointer_capture(evt.pointer_id());
        }
        let cursor_pos = (evt.client_x(), evt.client_y());
        // Start from the current grid geometry, including fractional CSS
        // pixels, rather than a cached/rounded size from an earlier gesture.
        let cell_size = layout.get_untracked().cell_size;
        let last_item_size = Size {
            width: current_col_span() as f64 * cell_size.width,
            height: current_row_span() as f64 * cell_size.height,
        };
        resize_state.set(ResizeState::Resizing {
            start_pos: cursor_pos,
            offset_x: 0,
            offset_y: 0,
            last_item_size,
        });

        on_resize_start(last_item_size);
    });

    let _resize_in_progress = use_event_listener(handle, leptos::ev::pointermove, move |evt| {
        if let ResizeState::Resizing {
            start_pos,
            last_item_size,
            ..
        } = resize_state.get()
        {
            evt.prevent_default();
            let cursor_pos = (evt.client_x(), evt.client_y());
            let (offset_x, offset_y) = ((cursor_pos.0 - start_pos.0), (cursor_pos.1 - start_pos.1));

            resize_state.set(ResizeState::Resizing {
                start_pos,
                offset_x,
                offset_y,
                last_item_size,
            });

            let current_size = Size {
                width: last_item_size.width + offset_x as f64,
                height: last_item_size.height + offset_y as f64,
            };
            let current_size = clamp_size_to_grid(
                &layout.get_untracked(),
                current_col_start_for_move(),
                current_size,
            );

            on_resize_move(current_size);
        }
    });

    let _resize_stops = use_event_listener(handle, leptos::ev::pointerup, move |evt| {
        if let Some(target) = evt.current_target() {
            let target: web_sys::Element = target.unchecked_into();
            let _ = target.release_pointer_capture(evt.pointer_id());
        }

        if let ResizeState::Resizing {
            start_pos,
            last_item_size,
            ..
        } = resize_state.get()
        {
            let layout = layout.get_untracked();
            let cell_size = layout.cell_size;

            let total_offset_x = evt.client_x() - start_pos.0;
            let total_offset_y = evt.client_y() - start_pos.1;

            // Use the release coordinates even if no pointermove was
            // delivered for them, with the same snapping as the preview.
            let raw_size = clamp_size_to_grid(
                &layout,
                current_col_start_for_end(),
                Size {
                    width: last_item_size.width + total_offset_x as f64,
                    height: last_item_size.height + total_offset_y as f64,
                },
            );

            let snapped_col_span = snap_resize_span(raw_size.width, cell_size.width);
            let snapped_row_span = snap_resize_span(raw_size.height, cell_size.height);

            let snapped_size = clamp_size_to_grid(
                &layout,
                current_col_start_for_end(),
                Size {
                    width: snapped_col_span as f64 * cell_size.width,
                    height: snapped_row_span as f64 * cell_size.height,
                },
            );

            resize_state.set(ResizeState::Ended {
                start_pos,
                total_offset_x,
                total_offset_y,
                last_item_size: snapped_size,
            });

            on_resize_end(snapped_size);
        }
    });

    let _resize_cancels = use_event_listener(handle, leptos::ev::pointercancel, move |evt| {
        if let ResizeState::Resizing { last_item_size, .. } = resize_state.get_untracked() {
            if let Some(target) = evt.current_target() {
                let target: web_sys::Element = target.unchecked_into();
                let _ = target.release_pointer_capture(evt.pointer_id());
            }

            resize_state.set(ResizeState::Idle { last_item_size });
            on_resize_cancel();
        }
    });

    // Handle grid layout resize events
    Effect::watch(
        move || layout.get().cell_size,
        move |cell_size, _, _| {
            if !matches!(resize_state.get_untracked(), ResizeState::Resizing { .. }) {
                let expected_size = Size {
                    width: current_col_span_for_layout() as f64 * cell_size.width,
                    height: current_row_span_for_layout() as f64 * cell_size.height,
                };
                let expected_size =
                    clamp_size_to_grid(&layout.get_untracked(), current_col_start(), expected_size);

                if resize_state.get_untracked().last_item_size() != expected_size {
                    resize_state.set(ResizeState::Idle {
                        last_item_size: expected_size,
                    });
                }
            }
        },
        true,
    );

    let size = Signal::derive({
        move || match resize_state.get() {
            ResizeState::Idle { last_item_size } => last_item_size,
            ResizeState::Resizing {
                offset_x,
                offset_y,
                last_item_size,
                ..
            } => clamp_size_to_grid(
                &layout.get_untracked(),
                current_col_start_for_size(),
                Size {
                    width: last_item_size.width + offset_x as f64,
                    height: last_item_size.height + offset_y as f64,
                },
            ),
            ResizeState::Ended { last_item_size, .. } => last_item_size,
        }
    });

    UseResizableGridItemReturn { size }
}
