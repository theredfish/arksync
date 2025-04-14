use leptos::html::Div;
use leptos::{logging::log, prelude::*};
use leptos_use::use_event_listener;
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
            col_start: 1,
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

    let window = window();
    let resize_button_ref = NodeRef::<Div>::new();
    let resize_start_pos = RwSignal::new(None::<(i32, i32)>);
    let resize_offset = RwSignal::new((0, 0));

    // Track the element being resized
    let resizing_id = RwSignal::new(None::<i32>);
    let is_resizing = RwSignal::new(false);

    let _resize_starts_ev =
        use_event_listener(resize_button_ref, leptos::ev::pointerdown, move |evt| {
            evt.prevent_default();
            let (client_x, client_y) = (evt.client_x(), evt.client_y());
            resize_start_pos.set(Some((client_x, client_y)));
            log!("Resize started: {:#?}", resize_start_pos.get());
        });

    let _resize_stops_ev = use_event_listener(window.clone(), leptos::ev::pointerup, move |_| {
        if resize_start_pos.get().is_some() {
            resize_start_pos.set(None);
            log!("Resize stopped");
        }
    });

    let _resize_ev = use_event_listener(window, leptos::ev::pointermove, move |evt| {
        if let Some((start_pos_x, start_pos_y)) = resize_start_pos.get() {
            let (move_x, move_y) = (evt.client_x(), evt.client_y());
            let (offset_x, offset_y) = ((move_x - start_pos_x), (move_y - start_pos_y));

            resize_offset.set((offset_x, offset_y));
        }
    });

    // Handle mousedown on the resize handle
    let _resize = Effect::watch(
        move || {
            let (x, y) = resize_offset.get();
            (x, y)
        },
        move |(x, y): &(i32, i32), _prev, _| {
            log!("watching changes of move offset: ({x},{y})");
        },
        false,
    );

    view! {
        <div
            class="w-full grid grid-cols-12 gap-4"
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
                        data-id=id.to_string()
                    >
                        {format!("Div {id} - {col_start},{col_span} {:#?}",resize_start_pos.get())}
                        <div
                            node_ref=resize_button_ref
                            class="absolute bottom-0 right-0 w-4 h-4 bg-red-500 cursor-se-resize"
                            data-id=id.to_string()
                        ></div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
