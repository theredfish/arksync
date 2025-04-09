use leptos::{logging::log, prelude::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
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
    col_span: i32, // Base span (adjusted by viewport)
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

    // Track the element being resized
    let resizing_id = RwSignal::new(None::<i32>);
    let resize_start_x = RwSignal::new(0);
    let resize_start_y = RwSignal::new(0);

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

    // Drag over handler
    let on_drag_over = move |ev: DragEvent| {
        ev.prevent_default();
    };

    // Drop handler
    let on_drop = move |ev: DragEvent| {
        ev.prevent_default();
        if let Ok(id) = ev
            .data_transfer()
            .unwrap()
            .get_data("text/plain")
            .unwrap()
            .parse::<i32>()
        {
            let client_x = ev.client_x();
            let client_y = ev.client_y();
            // Simplified: Map mouse coords to grid position (assuming 12-column grid)
            let new_col_start = ((client_x / 50) as i32).max(1).min(12); // Rough estimate: 50px per column
            let new_row_start = ((client_y / 50) as i32).max(1); // Rough estimate: 50px per row
            elements.update(|elems| {
                if let Some(elem) = elems.iter_mut().find(|e| e.id == id) {
                    elem.col_start = new_col_start;
                    elem.row_start = new_row_start;
                }
            });
        }
    };

    // Resize handlers
    let on_resize_start = move |ev: MouseEvent| {
        ev.prevent_default();
        if let Some(target) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        {
            if let Ok(id) = target
                .get_attribute("data-id")
                .unwrap_or_default()
                .parse::<i32>()
            {
                resizing_id.set(Some(id));
                resize_start_x.set(ev.client_x());
                resize_start_y.set(ev.client_y());
            }
        }
    };

    let on_resize_move = move |ev: MouseEvent| {
        if let Some(id) = resizing_id.get() {
            let delta_x = ev.client_x() - resize_start_x.get();
            let delta_y = ev.client_y() - resize_start_y.get();
            log!("Resize move {id}: x({delta_x}),y({delta_y})");
            elements.update(|elems| {
                if let Some(elem) = elems.iter_mut().find(|e| e.id == id) {
                    // Update spans based on mouse movement (50px grid units)
                    elem.col_span = (elem.col_span + delta_x).clamp(1, 12);
                    // .max(1)
                    // .min(12 - elem.col_start + 1);
                    log!("div col-span: {}", elem.col_span);
                    elem.row_span = (elem.row_span + delta_y).clamp(1, 12);
                    log!("div row-span: {}", elem.row_span);
                }
            });
            resize_start_x.set(ev.client_x());
            resize_start_y.set(ev.client_y());
        }
    };

    let on_resize_end = move |_| {
        resizing_id.set(None);
    };

    view! {
        <div
            class="w-full grid grid-cols-12 gap-4"
            on:dragover=on_drag_over
            on:drop=on_drop
            on:mousemove=on_resize_move
            on:mouseup=on_resize_end
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
                            class="absolute bottom-0 right-0 w-4 h-4 bg-gray-500 cursor-se-resize"
                            data-id=id.to_string()
                            on:mousedown=on_resize_start
                        ></div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
