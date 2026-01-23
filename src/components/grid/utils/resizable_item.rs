use leptos::prelude::*;
use leptos::{html::Div, prelude::NodeRef};
use leptos_use::core::Position;
use leptos_use::use_event_listener;
use std::default::Default;
use std::sync::Arc;

use crate::components::grid::core::layout::Layout;
use crate::components::grid::core::size::Size;

#[derive(Clone, Copy, Debug)]
pub enum ResizeState {
    Idle {
        last_item_size: Size,
    },
    Resizing {
        start_pos: (i32, i32),
        offset_x: i32,
        offset_y: i32,
        last_client_pos: (i32, i32),
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
    pub on_resize_start: Arc<dyn Fn(Size) + Send + Sync>,
    pub on_resize_move: Arc<dyn Fn(Size) + Send + Sync>,
    pub on_resize_end: Arc<dyn Fn(Size) + Send + Sync>,
}

impl Default for UseResizableGridItemOptions {
    fn default() -> Self {
        Self {
            handle: None,
            col_span: 1,
            row_span: 1,
            on_resize_start: Arc::new(|_| {}),
            on_resize_move: Arc::new(|_| {}),
            on_resize_end: Arc::new(|_| {}),
        }
    }
}

pub struct UseResizableGridItemReturn {
    /// Computed size in pixels of the grid item
    pub size: Signal<Size>,
    /// CSS transition string for resize animations
    pub transition: Signal<&'static str>,
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
        on_resize_start,
        on_resize_move,
        on_resize_end,
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

    let _resize_starts = use_event_listener(handle, leptos::ev::pointerdown, move |evt| {
        evt.prevent_default();
        let cursor_pos = (evt.client_x(), evt.client_y());
        let last_item_size = resize_state.get().last_item_size();
        resize_state.set(ResizeState::Resizing {
            start_pos: cursor_pos,
            offset_x: 0,
            offset_y: 0,
            last_client_pos: cursor_pos,
            last_item_size,
        });

        on_resize_start(last_item_size);
        let on_resize_move = Arc::clone(&on_resize_move);
        let on_resize_end = Arc::clone(&on_resize_end);

        let _resize_in_progress =
            use_event_listener(window(), leptos::ev::pointermove, move |evt| {
                evt.prevent_default();
                if let ResizeState::Resizing {
                    start_pos,
                    last_client_pos,
                    last_item_size,
                    ..
                } = resize_state.get()
                {
                    let cursor_pos = (evt.client_x(), evt.client_y());
                    let (offset_x, offset_y) =
                        ((cursor_pos.0 - start_pos.0), (cursor_pos.1 - start_pos.1));

                    resize_state.set(ResizeState::Resizing {
                        start_pos,
                        offset_x,
                        offset_y,
                        last_client_pos: cursor_pos,
                        last_item_size,
                    });

                    let current_size = Size {
                        width: last_item_size.width + offset_x as f64,
                        height: last_item_size.height + offset_y as f64,
                    };

                    on_resize_move(current_size);
                }
            });

        let _resize_stops = use_event_listener(window(), leptos::ev::pointerup, move |_| {
            if let ResizeState::Resizing {
                start_pos,
                last_client_pos,
                last_item_size,
                ..
            } = resize_state.get()
            {
                let cell_size = layout.get_untracked().cell_size;

                let total_offset_x = last_client_pos.0 - start_pos.0;
                let total_offset_y = last_client_pos.1 - start_pos.1;

                // Grid-snapping when resizing ends.
                //
                // If the last mouse position x is 253, and the resize started at 100px, then we get a movement
                // of 153px. To stick the movement to the grid we need to know if we reached the middle of the
                // last cell in which case we fill it, otherwise, we go back to the previous cell.
                //
                // Here the calcul for a grid cell width of 100px is: (153 / 100).round() -> 1.53.round() -> 2

                // Calculate the raw new size (before snapping)
                let raw_width = last_item_size.width + total_offset_x as f64;
                let raw_height = last_item_size.height + total_offset_y as f64;

                let snapped_width = (raw_width / cell_size.width).round() * cell_size.width;
                let snapped_height = (raw_height / cell_size.height).round() * cell_size.height;

                let snapped_size = Size {
                    width: snapped_width,
                    height: snapped_height,
                };

                resize_state.set(ResizeState::Ended {
                    start_pos,
                    total_offset_x,
                    total_offset_y,
                    last_item_size: snapped_size,
                });

                on_resize_end(snapped_size);
            }
        });
    });

    // Handle grid layout resize events
    Effect::watch(
        move || layout.get().cell_size,
        move |cell_size, _, _| {
            if matches!(resize_state.get_untracked(), ResizeState::Idle { .. }) {
                // Only update if the size has actually changed
                if let ResizeState::Idle { last_item_size } = resize_state.get_untracked() {
                    let expected_size = Size {
                        width: (col_span as f64 * cell_size.width).round(),
                        height: (row_span as f64 * cell_size.height).round(),
                    };

                    if last_item_size != expected_size {
                        resize_state.set(ResizeState::Idle {
                            last_item_size: expected_size,
                        });
                    }
                }
            }
        },
        true,
    );

    let size = Signal::derive({
        move || {
            let cell_size = &layout.get_untracked().cell_size;

            match resize_state.get() {
                ResizeState::Idle { last_item_size } => last_item_size,
                ResizeState::Resizing {
                    offset_x,
                    offset_y,
                    last_item_size,
                    ..
                } => Size {
                    width: (last_item_size.width + offset_x as f64),
                    height: (last_item_size.height + offset_y as f64),
                },
                ResizeState::Ended { last_item_size, .. } => last_item_size,
            }
        }
    });

    let transition = Signal::derive(move || match resize_state.get() {
        ResizeState::Resizing { .. } => "width 0ms ease-in, height 0ms ease-in",
        _ => "width 250ms ease-in, height 250ms ease-in",
    });

    UseResizableGridItemReturn { size, transition }
}
