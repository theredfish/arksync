use leptos::html::Div;
use leptos::prelude::*;
use leptos_use::{core::Position, use_draggable_with_options, UseDraggableOptions};
use std::sync::Arc;
use wasm_bindgen::JsCast;

use crate::components::grid::core::{layout::Layout, size::Size};

#[derive(Clone, Copy, Debug)]
pub enum DragState {
    Dragging(Position),
    DragEnded(Position),
}

pub struct UseDraggableGridItemOptions {
    /// The drag handle element (if None, the entire element is draggable)
    pub handle: Option<NodeRef<Div>>,
    /// Initial column position
    pub col_start: usize,
    /// Initial row position
    pub row_start: usize,
    /// Callback when drag starts
    pub on_drag_start: Arc<dyn Fn(Position) + Send + Sync>,
    /// Callback during dragging
    pub on_drag_move: Arc<dyn Fn(Position) + Send + Sync>,
    /// Callback when drag ends with final grid position
    pub on_drag_end: Arc<dyn Fn(usize, usize, Position) + Send + Sync>,
}

impl Default for UseDraggableGridItemOptions {
    fn default() -> Self {
        Self {
            handle: None,
            col_start: 0,
            row_start: 0,
            on_drag_start: Arc::new(|_| {}),
            on_drag_move: Arc::new(|_| {}),
            on_drag_end: Arc::new(|_, _, _| {}),
        }
    }
}

/// Return type for the draggable grid item hook
pub struct UseDraggableGridItemReturn {
    /// Computed position in pixels
    pub position: Signal<Position>,
    /// CSS transition string for drag animations
    pub transition: Signal<&'static str>,
}

pub fn use_draggable_grid_item(
    element_ref: NodeRef<Div>,
    options: UseDraggableGridItemOptions,
) -> UseDraggableGridItemReturn {
    let layout = use_context::<RwSignal<Layout>>().expect("Layout context must be provided");
    let drag_state = RwSignal::new(DragState::Dragging(Position::default()));
    let UseDraggableGridItemOptions {
        handle,
        col_start,
        row_start,
        on_drag_start,
        on_drag_move,
        on_drag_end,
    } = options;

    // Initialize position based on grid coordinates
    Effect::new(move || {
        let Size {
            width: cell_w,
            height: cell_h,
        } = layout.get_untracked().cell_size;

        let initial_position = Position {
            x: (options.col_start.saturating_sub(1)) as f64 * cell_w,
            y: (options.row_start.saturating_sub(1)) as f64 * cell_h,
        };

        drag_state.set(DragState::Dragging(initial_position));

        on_drag_start(initial_position);
    });

    // Setup draggable with leptos-use
    let _ = use_draggable_with_options(
        element_ref,
        UseDraggableOptions::default()
            .handle(options.handle)
            .initial_value({
                let pos = match drag_state.get_untracked() {
                    DragState::Dragging(p) | DragState::DragEnded(p) => p,
                };
                Position { x: pos.x, y: pos.y }
            })
            .on_move(move |drag_event| {
                drag_state.set(DragState::Dragging(drag_event.position));
                on_drag_move(drag_event.position);
            })
            .on_end(move |drag_event| {
                let cell_size = layout.get_untracked().cell_size;
                let drag_position = drag_event.position;

                // Snap to grid
                let (col_start, row_start, final_position) = {
                    let col = (drag_position.x / cell_size.width).round() as usize;
                    let row = (drag_position.y / cell_size.height).round() as usize;
                    let snapped_pos = Position {
                        x: col as f64 * cell_size.width,
                        y: row as f64 * cell_size.height,
                    };
                    (col, row, snapped_pos)
                };

                drag_state.set(DragState::DragEnded(final_position));

                on_drag_end(col_start, row_start, final_position);
            })
            .target_offset(move |event_target: web_sys::EventTarget| {
                let target: web_sys::HtmlElement = event_target.unchecked_into();
                let (x, y): (f64, f64) = (target.offset_left().into(), target.offset_top().into());
                (x, y)
            })
            .prevent_default(true),
    );

    let position = Signal::derive(move || match drag_state.get() {
        DragState::Dragging(p) | DragState::DragEnded(p) => p,
    });

    // Transition for smooth animations
    let transition = Signal::derive(move || match drag_state.get() {
        DragState::Dragging(_) => "left 0ms ease-in, top 0ms ease-in",
        DragState::DragEnded(_) => "left 250ms ease-in, top 250ms ease-in",
    });

    UseDraggableGridItemReturn {
        transition,
        position,
    }
}
