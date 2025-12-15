use crate::components::grid::utils::draggable_item::{
    use_draggable_grid_item, UseDraggableGridItemOptions, UseDraggableGridItemReturn,
};
use crate::components::grid::utils::resizable_item::{
    use_resizable_grid_item, UseResizableGridItemOptions, UseResizableGridItemReturn,
};
use crate::components::grid::{GridItemData, GridItemPosition, Layout, Size, Span};
use crate::components::heroicons::ResizeIcon;
use leptos::html::Div;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_use::{use_element_bounding, UseElementBoundingReturn};

#[component]
pub fn GridItem(
    children: Children,
    id: u32,
    col_span: u32,
    row_span: u32,
    col_start: u32,
    row_start: u32,
    #[prop(optional)] label: String,
) -> impl IntoView {
    let layout = use_context::<RwSignal<Layout>>().expect("should retrieve the layout context");
    let metadata = RwSignal::new(GridItemData::default());
    let window = window();
    let grid_item_ref = NodeRef::<Div>::new();
    let drag_ref = NodeRef::<Div>::new();
    let resize_button_ref = NodeRef::<Div>::new();

    Effect::new(move || {
        // TODO: find a better way, this is causing a initial state immediately
        // updated from watch effects when layout is tracked.
        let Size {
            width: cell_w,
            height: cell_h,
        } = layout.get_untracked().cell_size;
        let item_data = GridItemData {
            id,
            position: GridItemPosition {
                col_start,
                row_start,
            },
            span: Span { row_span, col_span },
            size: Size {
                width: col_span as f64 * cell_w,
                height: row_span as f64 * cell_h,
            },
        };
        log!(
            "col_span: {col_span} and cell_w: {cell_w}. Computed width: {}",
            col_span as f64 * cell_w
        );
        metadata.set(item_data);

        layout.update(|layout| {
            layout.add_item(id, metadata);
        });
    });

    // Drag events
    let draggable_options = UseDraggableGridItemOptions {
        handle: Some(drag_ref),
        col_start,
        row_start,
        ..Default::default()
    };

    let UseDraggableGridItemReturn {
        left,
        top,
        transition: drag_transition,
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
        ..Default::default()
    };

    let UseResizableGridItemReturn {
        size: resize_size,
        transition: resize_transition,
    } = use_resizable_grid_item(grid_item_ref, resize_options);

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
        let (left, top, drag_transition) = (left.get(), top.get(), drag_transition.get());
        let resize_transition = resize_transition.get();
        let Size { width, height } = resize_size.get();
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
                    let GridItemPosition { col_start, row_start } = metadata.get().position;
                    // format!("position: {col_start};{row_start} | size: {width};{height} | left/top: : {}; {}", left.get(), top.get())
                }
                }
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
