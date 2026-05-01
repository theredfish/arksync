use leptos::html::Div;
use leptos::prelude::*;
use leptos_use::{core::Position, use_draggable_with_options, UseDraggableOptions};
use std::sync::Arc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

use crate::components::grid::core::{layout::Layout, size::Size};

// Keep this slightly above the drag transition duration so the released item
// stays above its neighbors until the snap-back animation has fully finished.
const DRAG_RELEASE_ELEVATION_MS: i32 = 280;

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
    /// Column span of the item, used to clamp horizontal dragging inside the grid
    pub col_span: usize,
    /// Current column span of the item, used after resize to keep drag bounds accurate
    pub current_col_span: Arc<dyn Fn() -> usize + Send + Sync>,
    /// Callback when drag starts
    pub on_drag_start: Arc<dyn Fn(Position) + Send + Sync>,
    /// Callback during dragging
    pub on_drag_move: Arc<dyn Fn(Position) + Send + Sync>,
    /// Callback when drag ends with final grid position
    pub on_drag_end: Arc<dyn Fn(usize, usize, Position, Position) + Send + Sync>,
}

impl Default for UseDraggableGridItemOptions {
    fn default() -> Self {
        Self {
            handle: None,
            col_start: 0,
            row_start: 0,
            col_span: 1,
            current_col_span: Arc::new(|| 1),
            on_drag_start: Arc::new(|_| {}),
            on_drag_move: Arc::new(|_| {}),
            on_drag_end: Arc::new(|_, _, _, _| {}),
        }
    }
}

/// Return type for the draggable grid item hook
pub struct UseDraggableGridItemReturn {
    /// Computed position in pixels
    pub position: Signal<Position>,
    /// CSS transition string for drag animations
    pub transition: Signal<&'static str>,
    /// Whether the item is actively being dragged
    pub is_dragging: Signal<bool>,
}

pub fn use_draggable_grid_item(
    element_ref: NodeRef<Div>,
    options: UseDraggableGridItemOptions,
) -> UseDraggableGridItemReturn {
    let layout = use_context::<RwSignal<Layout>>().expect("Layout context must be provided");
    let drag_state = RwSignal::new(DragState::Dragging(Position::default()));
    let is_dragging = RwSignal::new(false);
    let drag_elevation_epoch = RwSignal::new(0_u32);
    let UseDraggableGridItemOptions {
        handle,
        col_start,
        row_start,
        col_span,
        current_col_span,
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
            x: col_start as f64 * cell_w,
            y: row_start as f64 * cell_h,
        };

        drag_state.set(DragState::Dragging(initial_position));

        on_drag_start(initial_position);
    });

    let current_col_span_for_move = Arc::clone(&current_col_span);
    let current_col_span_for_end = Arc::clone(&current_col_span);

    // Setup draggable with leptos-use
    let _ = use_draggable_with_options(
        element_ref,
        UseDraggableOptions::default()
            .handle(handle)
            .initial_value({
                let pos = match drag_state.get_untracked() {
                    DragState::Dragging(p) | DragState::DragEnded(p) => p,
                };
                Position { x: pos.x, y: pos.y }
            })
            .on_move(move |drag_event| {
                drag_elevation_epoch.update(|epoch| *epoch = epoch.wrapping_add(1));

                let layout = layout.get_untracked();
                let max_col_start = layout.columns.saturating_sub(current_col_span_for_move());
                let max_x = max_col_start as f64 * layout.cell_size.width;
                let clamped_position = Position {
                    x: drag_event.position.x.clamp(0.0, max_x),
                    y: drag_event.position.y.max(0.0),
                };

                is_dragging.set(true);
                drag_state.set(DragState::Dragging(clamped_position));
                on_drag_move(clamped_position);
            })
            .on_end(move |drag_event| {
                drag_elevation_epoch.update(|epoch| *epoch = epoch.wrapping_add(1));
                let drag_end_epoch = drag_elevation_epoch.get_untracked();

                let layout = layout.get_untracked();
                let cell_size = layout.cell_size;
                let max_col_start = layout.columns.saturating_sub(current_col_span_for_end());
                let max_x = max_col_start as f64 * cell_size.width;
                let drag_position = Position {
                    x: drag_event.position.x.clamp(0.0, max_x),
                    y: drag_event.position.y.max(0.0),
                };

                // Snap to grid
                let (col_start, row_start, final_position) = {
                    let col =
                        ((drag_position.x / cell_size.width).round() as usize).min(max_col_start);
                    let row = (drag_position.y / cell_size.height).round() as usize;
                    let snapped_pos = Position {
                        x: col as f64 * cell_size.width,
                        y: row as f64 * cell_size.height,
                    };
                    (col, row, snapped_pos)
                };

                drag_state.set(DragState::DragEnded(final_position));

                on_drag_end(col_start, row_start, final_position, drag_position);

                // Release drag elevation after the CSS transition, not on pointerup.
                // Otherwise a snapping item can briefly pass behind another panel
                // during the return animation and make the interaction look broken.
                let release_drag_elevation = Closure::wrap(Box::new(move || {
                    if drag_elevation_epoch.get_untracked() == drag_end_epoch {
                        is_dragging.set(false);
                    }
                }) as Box<dyn FnMut()>);

                let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(
                    release_drag_elevation.as_ref().unchecked_ref(),
                    DRAG_RELEASE_ELEVATION_MS,
                );
                release_drag_elevation.forget();
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
        is_dragging: is_dragging.into(),
    }
}
