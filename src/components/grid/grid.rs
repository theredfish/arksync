use leptos::html::Div;
use leptos::{logging::log, prelude::*};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::{collections::HashMap, i32};
use wasm_bindgen::prelude::*;
use web_sys::{DragEvent, MouseEvent};

// Static arrays of Tailwind classes pairs.
// This is used to overcome Tailwind limitations when
// parsing source files with viewports. This is preferred
// than a safelist that will increase the size of the output.
static MD_COL_SPAN_MAP: LazyLock<HashMap<i32, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    map.insert(1, "md:col-span-1");
    map.insert(2, "md:col-span-2");
    map.insert(3, "md:col-span-3");
    map.insert(4, "md:col-span-4");
    map.insert(5, "md:col-span-5");
    map.insert(6, "md:col-span-6");
    map.insert(7, "md:col-span-7");
    map.insert(8, "md:col-span-8");
    map.insert(9, "md:col-span-9");
    map.insert(10, "md:col-span-10");
    map.insert(11, "md:col-span-11");
    map.insert(12, "md:col-span-12");

    map
});

static MD_COL_START_MAP: LazyLock<HashMap<i32, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    map.insert(1, "md:col-start-1");
    map.insert(2, "md:col-start-2");
    map.insert(3, "md:col-start-3");
    map.insert(4, "md:col-start-4");
    map.insert(5, "md:col-start-5");
    map.insert(6, "md:col-start-6");
    map.insert(7, "md:col-start-7");
    map.insert(8, "md:col-start-8");
    map.insert(9, "md:col-start-9");
    map.insert(10, "md:col-start-10");
    map.insert(11, "md:col-start-11");
    map.insert(12, "md:col-start-12");

    map
});

trait IntoTailwind {
    fn into_tailwind(&self) -> String;
}

// Struct to hold element properties
#[derive(Clone, Serialize, Deserialize)]
pub struct GridElement {
    id: i32,
    col_start: i32,
    col_span: i32,
    row_start: i32,
    row_span: i32,
}

impl IntoTailwind for GridElement {
    fn into_tailwind(&self) -> String {
        let GridElement {
            col_start,
            col_span,
            row_start,
            row_span,
            ..
        } = self;

        let md_col_span = MD_COL_SPAN_MAP.get(col_span).unwrap_or(&"md:col-span-12");
        // start-span needs to be redefined when col-span is used for a specific
        // viewport. https://github.com/tailwindlabs/tailwindcss/issues/2989#issuecomment-738764372
        let md_start_span = MD_COL_START_MAP.get(col_start).unwrap_or(&"md:col-start-1");

        format!(
            "col-start-{col_start} {md_col_span} {md_start_span}
            row-start-{row_start} row-span-{row_span}"
        )
    }
}

#[component]
pub fn Grid() -> impl IntoView {
    // Reactive signal for grid elements
    let elements = RwSignal::new(vec![
        GridElement {
            id: 1,
            col_start: 2,
            col_span: 4,
            row_start: 1,
            row_span: 4,
        },
        GridElement {
            id: 2,
            col_start: 1,
            col_span: 2,
            row_start: 1,
            row_span: 1,
        },
        GridElement {
            id: 3,
            col_start: 3,
            col_span: 2,
            row_start: 3,
            row_span: 3,
        },
    ]);

    let resize_button_ref = NodeRef::<Div>::new();

    // Track the element being resized
    let resizing_id = RwSignal::new(None::<i32>);
    let resize_start_pos = RwSignal::new(None::<(i32, i32)>);
    let is_resizing = RwSignal::new(false);

    // Drag start handler
    let on_drag_start = move |ev: DragEvent| {
        if let Some(target) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            if let Ok(id) = target
                .get_attribute("data-id")
                .unwrap_or_default()
                .parse::<i32>()
            {
                ev.data_transfer()
                    .unwrap()
                    .set_data("text/plain", &id.to_string())
                    .unwrap();
            }
        }
    };

    // Handle mousedown on the resize handle
    Effect::new(move || {
        let handle = resize_button_ref.get().expect("resize handle should exist");
        let mousedown_handler = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            ev.prevent_default();
            if !is_resizing.get() {
                resize_start_pos.set(Some((ev.client_x(), ev.client_y())));
                is_resizing.set(true);
            }
        });

        handle
            .add_event_listener_with_callback(
                "mousedown",
                mousedown_handler.as_ref().unchecked_ref(),
            )
            .expect("mousedown listener should be attached");

        let mousedown_handler_ref: &JsValue = mousedown_handler.as_ref().unchecked_ref();

        // on_cleanup(move || {
        //     handle
        //         .remove_event_listener_with_callback(
        //             "mousedown",
        //             ,
        //         )
        //         .expect("mousedown listener should be removed");
        // });
    });

    view! {
        <div
            class="w-full grid grid-cols-12 gap-4"
            // on:dragover=on_drag_over
            // on:drop=on_drop
            // on:mousemove=on_resize_move
            // on:mouseup=on_resize_end
        >
            {move || elements.get().into_iter().map(|elem| {
                // Responsive column span: full width on mobile, custom on desktop

                let id = elem.id;
                let col_start = elem.col_start;
                let col_span = elem.col_span;

                view! {
                    <div
                        class=format!(
                            "relative p-4 bg-blue-200 cursor-move {}",
                            elem.into_tailwind()
                        )
                        // class="relative p-4 bg-blue-200 cursor-move col-start-2 col-span-4"
                        draggable=true
                        data-id=id.to_string()
                        on:dragstart=on_drag_start
                    >
                        {format!("Div {id} - {col_start},{col_span}")}
                        <div
                            node_ref=resize_button_ref
                            class="absolute bottom-0 right-0 w-4 h-4 bg-gray-500 cursor-se-resize"
                            data-id=id.to_string()
                        ></div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
