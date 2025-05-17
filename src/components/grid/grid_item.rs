use crate::components::grid::{GridItemData, GridItemPosition, GridItemSpan, Layout, Size};
use crate::components::heroicons::ResizeIcon;
use leptos::html::Div;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_use::{
    core::Position, use_draggable_with_options, use_event_listener, UseDraggableOptions,
    UseDraggableReturn,
};
use leptos_use::{use_element_bounding, UseElementBoundingReturn};
use wasm_bindgen::JsCast;

#[derive(Clone, Default)]
pub struct ResizeMovement {
    offset_x: i32,
    offset_y: i32,
    last_client_pos: (i32, i32),
    last_item_size: Size,
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
    let resize_start_pos = RwSignal::new((0, 0));
    let resize_movement = RwSignal::new(ResizeMovement::default());
    let is_resizing = RwSignal::new(false);

    Effect::new(move |_| {
        let Size {
            width: cell_w,
            height: cell_h,
        } = layout.get_untracked().cell_size;
        let item_data = GridItemData {
            position: GridItemPosition {
                col_start,
                row_start,
            },
            span: GridItemSpan { row_span, col_span },
            size: Size {
                width: col_span as f64 * cell_w,
                height: row_span as f64 * cell_h,
            },
        };
        metadata.set(item_data);

        layout.update(|layout| {
            layout.add_item(id, metadata); // on the heap with a RwLock
        });
    });

    // TODO: bug: the last resize operation is saved and when the button is clicked (no mouse movement), the the last
    // operation is applied again.
    // TODO: When I click the resize button my component move to the center of the screen and it's certainly connected
    // to the draggable options and the initial value.
    // resize events
    {
        // Resize starts: set the resize start position with the mouse position
        let _resize_starts_ev =
            use_event_listener(resize_button_ref, leptos::ev::pointerdown, move |evt| {
                evt.prevent_default();
                let cursor_pos = (evt.client_x(), evt.client_y());
                resize_start_pos.set(cursor_pos);
                resize_movement.update(|movement| {
                    movement.last_client_pos = cursor_pos;
                    movement.last_item_size = metadata.get().size;
                });
                is_resizing.set(true);
            });

        // Resize stops: Update the metadata with the offset
        let _resize_stops_ev =
            use_event_listener(window.clone(), leptos::ev::pointerup, move |_| {
                if is_resizing.get() {
                    is_resizing.set(false);
                }
            });

        // Resize in progress: we update the offset (mouse_pos - client_pos)
        let _resize_ev = use_event_listener(window, leptos::ev::pointermove, move |evt| {
            if is_resizing.get() {
                // The total offset from the beginning

                let last_client_pos = resize_movement.get().last_client_pos;

                log!("last_client_pos: {last_client_pos:#?}");
                let (curr_pos_x, curr_pos_y) = (evt.client_x(), evt.client_y());
                let (offset_x, offset_y) = (
                    (curr_pos_x - last_client_pos.0),
                    (curr_pos_y - last_client_pos.1),
                );

                let last_client_pos = (evt.client_x(), evt.client_y());

                resize_movement.update(move |movement| {
                    movement.offset_x = offset_x;
                    movement.offset_y = offset_y;
                    movement.last_client_pos = last_client_pos;
                });
            }
        });

        Effect::watch(
            move || {
                (
                    resize_movement.get(),
                    resize_start_pos.get(),
                    is_resizing.get(),
                )
            },
            move |(resize_movement, resize_start_pos, is_resizing): &(
                ResizeMovement,
                (i32, i32),
                bool,
            ),
                  _,
                  _| {
                let ResizeMovement {
                    offset_x,
                    offset_y,
                    last_client_pos,
                    last_item_size,
                } = resize_movement;

                if *is_resizing {
                    metadata.update(|data| {
                        data.size.width = data.size.width + (*offset_x as f64);
                        data.size.height = data.size.height + *offset_y as f64;
                    });
                }

                // We stopped to resize
                if !*is_resizing {
                    log!("We stopped");
                    let (total_offset_x, total_offset_y) = (
                        (last_client_pos.0 - resize_start_pos.0),
                        (last_client_pos.1 - resize_start_pos.1),
                    );
                    log!("total_offset: {total_offset_x},{total_offset_y}");
                    let cell_size = layout.get_untracked().cell_size;

                    // 350px => 1 | 2 | 3 ^ | 4
                    // 350 / 100 => 3.5 => 3 => fallback to the column 3
                    // last step: we need to convert back to pixels (* cell_size)

                    // 200 (2 cols).
                    // ==> 250 ==> offset total should be 200
                    metadata.update(|data| {
                        let clamped_w =
                            ((total_offset_x as f64) / cell_size.width).round() * cell_size.width;
                        let clamped_h =
                            (total_offset_y as f64 / cell_size.height).round() * cell_size.height;

                        log!("clamped_w({clamped_w}), clamped_h({clamped_h})");
                        log!("[before] data.size.width: {}", data.size.width);

                        let layout = layout.get_untracked();
                        // TODO: calculate the max based on the item location, not only the layout size
                        data.size.width = (last_item_size.width + clamped_w)
                            .clamp(cell_size.width, layout.size.width);
                        log!("[after] data.size.width: {}", data.size.width);
                        // data.size.height = clamped_h;
                    });
                }
            },
            false,
        );
    }

    // Drag events
    let UseDraggableReturn { x, y, .. } = use_draggable_with_options(
        grid_item_ref,
        UseDraggableOptions::default()
            .handle(Some(drag_ref))
            .initial_value(Position {
                x: metadata.get_untracked().position.col_start as f64
                    * layout.get_untracked().cell_size.width,
                y: metadata.get_untracked().position.row_start as f64
                    * layout.get_untracked().cell_size.height,
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

    let left = move || {
        let x = x.get();
        let grid_w = layout.get().size.width;
        let max = if grid_w <= 0. {
            0.
        } else {
            grid_w - item_width.get()
        };

        x.clamp(0., max.round())
    };
    let top = move || {
        let y = y.get();
        let grid_h = layout.get().size.height;
        let max = if grid_h <= 0. {
            0.
        } else {
            grid_h - item_height.get()
        };

        y.clamp(0.0, max.round())
    };

    let style = move || {
        let Size { width, height } = metadata.get().size;
        let transition = if is_resizing.get() {
            "width 0ms ease-in, height 0ms ease-in;"
        } else {
            "width 250ms ease-in, height 250ms ease-in;"
        };

        format!(
            r#"width: {width}px;
            height: {height}px;
            transition: {transition}
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
