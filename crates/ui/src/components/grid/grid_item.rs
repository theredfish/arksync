use crate::components::grid::core::item::{GridItemData, GridPosition};
use crate::components::grid::core::layout::Layout;
use crate::components::grid::core::size::Size;
use crate::components::grid::core::span::Span;
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
use leptos_use::{use_element_bounding, UseElementBoundingReturn};
use std::sync::Arc;

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
) -> impl IntoView {
    let layout = use_context::<RwSignal<Layout>>().expect("should retrieve the layout context");
    let untracked_layout = layout.get_untracked();
    let window = window();
    let grid_item_ref = NodeRef::<Div>::new();
    let drag_ref = NodeRef::<Div>::new();
    let resize_button_ref = NodeRef::<Div>::new();

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
            x: col_start as f64 * cell_h,
            y: row_start as f64 * cell_w,
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
        on_drag_move: Arc::new(move |drag_px_pos| {
            grid_item_data.update(|item| {
                item.px_pos = drag_px_pos;
                log!("Update item with new px_pos: {drag_px_pos:?}");
            })
        }),
        on_drag_end: Arc::new(move |col_start, row_start, snapped_px_pos| {
            grid_item_data.update(|item| {
                item.px_pos = snapped_px_pos;
                item.grid_pos = GridPosition {
                    col_start,
                    row_start,
                };
            });

            // Move item with collision detection and push other items down
            layout.update(|layout| {
                // TODO: drag & detect collisions
                // layout.move_item_with_collision(grid_item_data, new_row, new_col);
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
        ..
    } = use_draggable_grid_item(grid_item_ref, draggable_options);

    // Absolute element width/height
    let UseElementBoundingReturn {
        width: item_width,
        height: item_height,
        ..
    } = use_element_bounding(grid_item_ref);

    // Grid item resize
    let resize_options = UseResizableGridItemOptions {
        handle: Some(resize_button_ref),
        col_span,
        row_span,
        on_resize_end: Arc::new(move |size| {
            grid_item_data.update(|item| {
                item.size = size;
            });
        }),
        ..Default::default()
    };

    let UseResizableGridItemReturn {
        size: resize_size,
        transition: resize_transition,
    } = use_resizable_grid_item(grid_item_ref, resize_options);

    // TODO: Handle collisions

    // TODO: clamp dragging event.
    // Avoid issues where min > max.
    // let left = move || {
    //     // let x = metadata.get().position.col_start * layout.get().cell_size.width as u32;
    //     match drag_state.get() {
    //         DragState::Dragging(p) | DragState::DragEnded(p) => p.x,
    //     }
    //     // let grid_w = layout.get().size.width;
    //     //     let max = if grid_w <= 0. {
    //     //         0.
    //     //     } else {
    //     //         grid_w - item_width.get()
    //     //     };

    //     //     x.clamp(0., max.round())
    // };
    // let top = move || {
    //     // let y = metadata.get().position.row_start * layout.get().cell_size.height as u32;
    //     match drag_state.get() {
    //         DragState::Dragging(p) | DragState::DragEnded(p) => p.y,
    //     }
    //     // let grid_h = layout.get().size.height;
    //     // let max = if grid_h <= 0. {
    //     //     0.
    //     // } else {
    //     //     grid_h - item_height.get()
    //     // };

    //     // y.clamp(0.0, max.round())
    // };

    let style = move || {
        // let Size { width, height } = metadata.get().size;
        let drag_transition = drag_transition.get();
        let Position { x: left, y: top } = grid_item_data.get().px_pos;
        // let Position { x: left, y: top } = drag_position.get();
        let resize_transition = resize_transition.get();
        let Size { width, height } = resize_size.get();

        log!("resize: {width};{height}");

        format!(
            r#"width: {width}px;
            height: {height}px;
            transition: {resize_transition}, {drag_transition};
            touch-action: none;
            left: {left}px;
            top: {top}px;"#
        )
    };

    view! {
        <div
            node_ref=grid_item_ref
            style={style}
            class="absolute cursor-move border-2 border-gray-500"
            data-id=id.to_string()
        >
            <div node_ref=drag_ref class="w-full p-2">
                { label }
            </div>
            <div>
                { move || {
                    let Size { width, height } = resize_size.get();
                    let GridPosition { col_start, row_start } = grid_item_data.get().grid_pos;
                    let Position { x: left, y: top } = grid_item_data.get().px_pos;
                    format!("position: {col_start};{row_start} | size: {width};{height} | left/top: : {left}; {top}")
                }}
            </div>
            { children() }
            <div
                node_ref=resize_button_ref
                class="absolute bottom-0 right-0 cursor-se-resize"
                data-id=id.to_string()
            >
                <ResizeIcon class="h-6 w-6" />
            </div>
        </div>
    }
}
