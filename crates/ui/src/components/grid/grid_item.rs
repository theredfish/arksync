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
use std::sync::Arc;

const GRID_ITEM_GAP_PX: f64 = 12.0;
const GRID_ITEM_INSET_PX: f64 = GRID_ITEM_GAP_PX / 2.0;

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
            grid_item_data.update(|item| {
                item.px_pos = drag_px_pos;
                log!("Update item with new px_pos: {drag_px_pos:?}");
            })
        }),
        on_drag_end: Arc::new(move |col_start, row_start, _snapped_px_pos, drag_px_pos| {
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
        on_resize_move: Arc::new(move |size| {
            grid_item_data.update(|item| {
                item.size = size;
            });
        }),
        on_resize_end: Arc::new(move |size| {
            layout.update(|layout| {
                let cell_size = layout.cell_size;
                let col_span = (size.width / cell_size.width).round() as usize;
                let row_span = (size.height / cell_size.height).round() as usize;

                layout.resize_item_with_collision(grid_item_data, col_span, row_span);
            });
        }),
        ..Default::default()
    };

    let UseResizableGridItemReturn {
        size: resize_size,
        transition: resize_transition,
    } = use_resizable_grid_item(grid_item_ref, resize_options);

    let style = move || {
        // let Size { width, height } = metadata.get().size;
        let drag_transition = drag_transition.get();
        let Position { x: left, y: top } = grid_item_data.get().px_pos;
        // let Position { x: left, y: top } = drag_position.get();
        let resize_transition = resize_transition.get();
        let Size { width, height } = resize_size.get();
        let visual_width = (width - GRID_ITEM_GAP_PX).max(0.0);
        let visual_height = (height - GRID_ITEM_GAP_PX).max(0.0);
        let visual_left = left + GRID_ITEM_INSET_PX;
        let visual_top = top + GRID_ITEM_INSET_PX;
        let z_index = if is_dragging.get() { 1000 } else { 1 };

        log!("resize: {width};{height}");

        format!(
            r#"width: {visual_width}px;
            height: {visual_height}px;
            transition: {resize_transition}, {drag_transition};
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
            class="arksync-panel absolute cursor-move overflow-hidden rounded-lg"
            data-id=id.to_string()
        >
            <div node_ref=drag_ref class="w-full border-b border-sk-carbon-725 px-4 py-3 font-mono text-[10px] uppercase tracking-[0.18em] text-sk-carbon-500">
                { label }
            </div>
            <div class="px-4 py-2 text-xs text-sk-carbon-500">
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
                <ResizeIcon class="h-6 w-6 text-sk-carbon-450" />
            </div>
        </div>
    }
}
