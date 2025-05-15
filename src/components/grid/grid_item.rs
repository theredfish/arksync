use crate::components::grid::{GridItemData, Layout, Size};
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

#[component]
pub fn GridItem(
    children: Children,
    id: u32,
    width: u32,
    height: u32,
    x: f64,
    y: f64,
    #[prop(optional)] label: String,
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
    let drag_ref = NodeRef::<Div>::new();

    let resize_button_ref = NodeRef::<Div>::new();
    let resize_start_pos = RwSignal::new(None::<(i32, i32)>);
    let resize_movement = RwSignal::new((0, 0));

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
                resize_start_pos.set(Some((evt.client_x(), evt.client_y())));
            });

        // Resize stops: Update the metadata with the offset
        let _resize_stops_ev =
            use_event_listener(window.clone(), leptos::ev::pointerup, move |_| {
                if resize_start_pos.get().is_some() {
                    resize_start_pos.set(None);
                }
            });

        // Resize in progress: we update the offset (mouse_pos - client_pos)
        let _resize_ev = use_event_listener(window, leptos::ev::pointermove, move |evt| {
            if let Some((start_x, start_y)) = resize_start_pos.get() {
                let (offset_x, offset_y) = ((evt.client_x() - start_x), (evt.client_y() - start_y));
                log!("resize_movement : ({offset_x},{offset_y})");
                resize_movement.update(move |movement| {
                    *movement = (offset_x, offset_y);
                });
                resize_start_pos.update(move |pos| *pos = Some((evt.client_x(), evt.client_y())));
            }
        });

        Effect::watch(
            move || resize_movement.get(),
            move |(offset_x, offset_y): &(i32, i32), _, _| {
                metadata.update(|data| {
                    data.size.width = data.size.width + *offset_x as f64;
                    data.size.height = data.size.height + *offset_y as f64;
                });
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
                x: metadata.get_untracked().position.x,
                y: metadata.get_untracked().position.y,
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
        let transition = match (resize_start_pos.get()) {
            Some(_) => "width 0ms ease-in, height 0ms ease-in;",
            None => "width 250ms ease-in, height 250ms ease-in;",
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
