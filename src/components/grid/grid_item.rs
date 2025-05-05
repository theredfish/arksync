use crate::components::grid::{GridItemData, Layout, Size};
use leptos::html::Div;
use leptos::prelude::*;
use leptos_use::{
    core::Position, use_draggable_with_options, use_event_listener, UseDraggableOptions,
    UseDraggableReturn,
};
use leptos_use::{use_element_bounding, UseElementBoundingReturn};
use wasm_bindgen::JsCast;

#[component]
pub fn GridItem(
    children: Children,
    id: u32,
    width: u32,
    height: u32,
    x: f64,
    y: f64,
) -> impl IntoView {
    let layout = use_context::<RwSignal<Layout>>().expect("should retrieve the layout context");
    let metadata = RwSignal::new(GridItemData {
        size: Size {
            width: width.into(),
            height: height.into(),
        },
        position: Position { x, y },
    });
    let window = window();
    let grid_item_ref = NodeRef::<Div>::new();
    let resize_button_ref = NodeRef::<Div>::new();

    let resize_start_pos = RwSignal::new(None::<(i32, i32)>);
    let resize_offset = RwSignal::new((0, 0));

    Effect::new(move |_| {
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
                let (client_x, client_y) = (evt.client_x(), evt.client_y());
                resize_start_pos.set(Some((client_x, client_y)));
            });

        // Resize stops: Update the metadata with the offset
        let _resize_stops_ev =
            use_event_listener(window.clone(), leptos::ev::pointerup, move |_| {
                // Pointerup event isn't associated to a resize event for this component
                if resize_start_pos.get().is_none() {
                    return;
                }

                let offset = resize_offset.get();
                resize_start_pos.set(None);

                let prev_metadata = metadata.get_untracked();
                metadata.update(|data| {
                    data.size.width = prev_metadata.size.width + offset.0 as f64;
                    data.size.height = prev_metadata.size.height + offset.1 as f64;
                });
            });

        // Resize in progress: we update the offset (mouse_pos - client_pos)
        let _resize_ev = use_event_listener(window, leptos::ev::pointermove, move |evt| {
            if let Some((start_pos_x, start_pos_y)) = resize_start_pos.get() {
                let (move_x, move_y) = (evt.client_x(), evt.client_y());
                let (offset_x, offset_y) = ((move_x - start_pos_x), (move_y - start_pos_y));

                resize_offset.set((offset_x, offset_y));
            }
        });
    }

    // Drag events
    let UseDraggableReturn { x, y, .. } = use_draggable_with_options(
        grid_item_ref,
        UseDraggableOptions::default()
            .initial_value(Position {
                x: metadata.get_untracked().position.x,
                y: metadata.get_untracked().position.y,
            })
            .target_offset(move |event_target: web_sys::EventTarget| {
                let target: web_sys::HtmlElement = event_target.unchecked_into();
                let (x, y): (f64, f64) = (target.offset_left().into(), target.offset_top().into());

                (x, y)
            })
            // .on_start(move |_| {
            //     // TODO: see to sync drag event with the resize event
            //     if resize_start_pos.get().is_some() {
            //         return false;
            //     }
            //     true
            // })
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

        format!(
            r#"width: {width}px;
            height: {height}px;
            transition: width 0.2s ease-out, height 0.2s ease-in;
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
            class="absolute p-4 cursor-move border-2 border-gray-500"
            data-id=id.to_string()
        >
            { children() }
            <div
                node_ref=resize_button_ref
                class="absolute bottom-0 right-0 w-4 h-4 bg-red-500 cursor-se-resize"
                data-id=id.to_string()
            ></div>
        </div>
    }
}
