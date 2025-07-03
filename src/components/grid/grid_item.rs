use crate::components::grid::{GridItemData, GridItemPosition, Layout, Size, Span};
use crate::components::heroicons::ResizeIcon;
use leptos::html::Div;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_use::{
    core::Position, use_draggable_with_options, use_event_listener, UseDraggableOptions,
};
use leptos_use::{use_element_bounding, UseElementBoundingReturn};
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, Debug)]
enum ResizeState {
    Idle,
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

impl Default for ResizeState {
    fn default() -> Self {
        ResizeState::Idle
    }
}

#[derive(Clone, Copy, Debug)]
enum DragState {
    Dragging(Position),
    DragEnded(Position),
}

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
    let resize_state = RwSignal::new(ResizeState::default());
    // let position = RwSignal::new(Position::default());
    let drag_state = RwSignal::new(DragState::Dragging(Position::default()));

    Effect::new(move || {
        // TODO: find a better way, this is causing a initial state immediately
        // updated from watch effects when layout is tracked.
        let Size {
            width: cell_w,
            height: cell_h,
        } = layout.get_untracked().cell_size;
        let item_data = GridItemData {
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

    // resize events
    {
        let _resize_starts =
            use_event_listener(resize_button_ref, leptos::ev::pointerdown, move |evt| {
                evt.prevent_default();
                let cursor_pos = (evt.client_x(), evt.client_y());
                resize_state.set(ResizeState::Resizing {
                    start_pos: cursor_pos,
                    offset_x: 0,
                    offset_y: 0,
                    last_client_pos: cursor_pos,
                    last_item_size: metadata.get().size,
                });
            });

        let _resize_in_progress =
            use_event_listener(window.clone(), leptos::ev::pointermove, move |evt| {
                evt.prevent_default();
                if let ResizeState::Resizing {
                    start_pos,
                    last_client_pos,
                    last_item_size,
                    ..
                } = resize_state.get()
                {
                    let cursor_pos = (evt.client_x(), evt.client_y());
                    let (offset_x, offset_y) = (
                        (cursor_pos.0 - last_client_pos.0),
                        (cursor_pos.1 - last_client_pos.1),
                    );

                    resize_state.set(ResizeState::Resizing {
                        start_pos,
                        offset_x,
                        offset_y,
                        last_client_pos: cursor_pos,
                        last_item_size,
                    });
                }
            });

        let _resize_stops = use_event_listener(window, leptos::ev::pointerup, move |_| {
            if let ResizeState::Resizing {
                start_pos,
                last_client_pos,
                last_item_size,
                ..
            } = resize_state.get()
            {
                let total_offset_x = last_client_pos.0 - start_pos.0;
                let total_offset_y = last_client_pos.1 - start_pos.1;
                resize_state.set(ResizeState::Ended {
                    start_pos,
                    total_offset_x,
                    total_offset_y,
                    last_item_size,
                });
            }
        });

        // Handle pointer resize events
        Effect::watch(
            move || resize_state.get(),
            move |state, _, _| {
                match state {
                    ResizeState::Resizing {
                        offset_x, offset_y, ..
                    } => {
                        metadata.update(|data| {
                            data.size.width = data.size.width + (*offset_x as f64);
                            data.size.height = data.size.height + (*offset_y as f64);
                        });
                    }
                    ResizeState::Ended {
                        total_offset_x,
                        total_offset_y,
                        last_item_size,
                        ..
                    } => {
                        let cell_size = &layout.get_untracked().cell_size;
                        log!("layout cell size: {cell_size:?}");
                        metadata.update(|data| {
                            // Grid-snapping when resizing ends.
                            //
                            // If the last mouse position x is 253, and the resize started at 100px, then we get a movement
                            // of 153px. To stick the movement to the grid we need to know if we reached the middle of the
                            // last cell in which case we fill it, otherwise, we go back to the previous cell.
                            //
                            // Here the calcul for a grid cell width of 100px is: (153 / 100).round() -> 1.53.round() -> 2
                            // We move by 2 times: 2 * 100px => 200px. So we fill 2 new columns of 100px.
                            let snapped_w = (*total_offset_x as f64 / cell_size.width).round()
                                * cell_size.width;
                            let snapped_h = (*total_offset_y as f64 / cell_size.height).round()
                                * cell_size.height;

                            log!("snapped_w({snapped_w}), snapped_h({snapped_h})");
                            log!("[before] data.size.width: {}", data.size.width);

                            // TODO: calculate the max based on the item location, not only the layout size
                            data.size.width = last_item_size.width + snapped_w;
                            log!("[after] data.size.width: {}", data.size.width);
                            // TODO: creates a bug
                            //data.size.height = snapped_h;
                        });
                        resize_state.set(ResizeState::Idle);
                    }
                    ResizeState::Idle => {}
                }
            },
            false,
        );

        // Handle grid layout resize events
        Effect::watch(
            move || layout.get().cell_size,
            move |cell_size, _, _| {
                if matches!(resize_state.get_untracked(), ResizeState::Idle) {
                    metadata.update(|data| {
                        let expected_size = Size {
                            width: (col_span as f64 * cell_size.width).round(),
                            height: (row_span as f64 * cell_size.height).round(),
                        };
                        log!("expected_size: {:?}", expected_size);
                        if data.size != expected_size {
                            data.size = expected_size;
                        }
                    });
                }
            },
            true,
        );
    }

    // Drag events
    let _ = use_draggable_with_options(
        grid_item_ref,
        UseDraggableOptions::default()
            .handle(Some(drag_ref))
            .initial_value({
                let pos = match drag_state.get() {
                    DragState::Dragging(p) | DragState::DragEnded(p) => p,
                };
                Position {
                    x: pos.x * layout.get().cell_size.width,
                    y: pos.y * layout.get().cell_size.height,
                }
            })
            .on_move(move |drag_event| {
                drag_state.set(DragState::Dragging(drag_event.position));
                log!("position on_move: {:?}", drag_event.position);
            })
            .on_end(move |drag_event| {
                let cell_size = layout.get().cell_size;
                let drag_position = drag_event.position;
                let col_start = (drag_position.x / cell_size.width).round() as u32;
                let row_start = (drag_position.y / cell_size.height).round() as u32;
                let final_position = Position {
                    x: col_start as f64 * cell_size.width,
                    y: row_start as f64 * cell_size.height,
                };
                // position.set(final_position);
                drag_state.set(DragState::DragEnded(final_position));
                log!("position on_end: {:?}", final_position);

                metadata.update(|data| {
                    data.position.col_start = col_start;
                    data.position.row_start = row_start;
                    log!("Update grid item position: {:?}", data.position);
                });
            })
            .target_offset(move |event_target: web_sys::EventTarget| {
                let target: web_sys::HtmlElement = event_target.unchecked_into();
                let (x, y): (f64, f64) = (target.offset_left().into(), target.offset_top().into());

                (x, y)
            })
            .prevent_default(true),
    );

    // Absolute element width/height
    let UseElementBoundingReturn {
        width: item_width,
        height: item_height,
        ..
    } = use_element_bounding(grid_item_ref);

    // TODO: clamp dragging event.
    // Avoid issues where min > max.
    let left = move || {
        // let x = metadata.get().position.col_start * layout.get().cell_size.width as u32;
        let x = match drag_state.get() {
            DragState::Dragging(p) | DragState::DragEnded(p) => p.x,
        };
        // let grid_w = layout.get().size.width;
        //     let max = if grid_w <= 0. {
        //         0.
        //     } else {
        //         grid_w - item_width.get()
        //     };

        //     x.clamp(0., max.round())

        x
    };
    let top = move || {
        // let y = metadata.get().position.row_start * layout.get().cell_size.height as u32;
        let y = match drag_state.get() {
            DragState::Dragging(p) | DragState::DragEnded(p) => p.y,
        };
        // let grid_h = layout.get().size.height;
        // let max = if grid_h <= 0. {
        //     0.
        // } else {
        //     grid_h - item_height.get()
        // };

        // y.clamp(0.0, max.round())

        y
    };

    let style = move || {
        let Size { width, height } = metadata.get().size;
        let transition_resize = match resize_state.get() {
            ResizeState::Resizing { .. } => "width 0ms ease-in, height 0ms ease-in",
            _ => "width 250ms ease-in, height 250ms ease-in",
        };
        let transition_drag = match drag_state.get() {
            DragState::Dragging(_) => "left 0ms ease-in, top 0ms ease-in",
            DragState::DragEnded(_) => "left 250ms ease-in, top 250ms ease-in",
        };

        format!(
            r#"width: {width}px;
            height: {height}px;
            transition: {transition_resize}, {transition_drag};
            touch-action: none;
            left: {}px;
            top: {}px;"#,
            left(),
            top()
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
                    let GridItemPosition { col_start, row_start } = metadata.get().position;
                    let Size { width, height } = metadata.get().size;
                    format!("position: {col_start};{row_start} | size: {width};{height} | left/top: : {}; {}", left(), top())
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
