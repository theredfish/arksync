use crate::components::grid::{GridItemData, GridStorage};
use crate::core::tailwind::*;
use leptos::html::Div;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_use::use_event_listener;

fn into_tailwind(col_start: i32, col_span: i32, row_start: i32, row_span: i32) -> String {
    let md_col_span = MD_COL_SPAN_MAP.get(&col_span).unwrap_or(&"md:col-span-12");
    // start-span needs to be redefined when col-span is used for a specific
    // viewport. https://github.com/tailwindlabs/tailwindcss/issues/2989#issuecomment-738764372
    let md_col_start = MD_COL_START_MAP
        .get(&col_start)
        .unwrap_or(&"md:col-start-1");

    let row_start_str = match row_start {
        0 => "row-start-auto".to_string(),
        _ => format!("row-start-{row_start}"),
    };
    let md_row_start = MD_ROW_START_MAP
        .get(&row_start)
        .unwrap_or(&"md:row-start-1");
    let md_row_span = MD_ROW_SPAN_MAP.get(&row_span).unwrap_or(&"md:row-span-12");

    format!("col-start-{col_start} {md_col_start} col-span-{col_span} {md_col_span} {row_start_str} {md_row_start} row-span-{row_span} {md_row_span}")
}

#[component]
pub fn GridItem(
    children: Children,
    id: i32,
    width: i32,
    height: i32,
    position_x: i32,
    position_y: i32,
) -> impl IntoView {
    // Context management with metadata
    let update_grid =
        use_context::<RwSignal<GridStorage>>().expect("to have found the setter provided");

    let metadata = RwSignal::new(GridItemData {
        width,
        height,
        position: (position_x, position_y),
    });

    // Rendering effect
    Effect::new(move |_| {
        let metadata = metadata.read_untracked();
        log!("[GridItem][{id}]: {:#?}", metadata.clone());
        update_grid.update(|grid| {
            grid.items.insert(id, metadata.clone());
        });
    });

    let width = RwSignal::new(width);
    let height = RwSignal::new(height);
    let position_x = RwSignal::new(position_x);
    let position_y = RwSignal::new(position_y);

    let window = window();
    let resize_button_ref = NodeRef::<Div>::new();
    let resize_start_pos = RwSignal::new(None::<(i32, i32)>);
    let resize_offset = RwSignal::new((0, 0));

    // Resize event listeners
    let _resize_starts_ev =
        use_event_listener(resize_button_ref, leptos::ev::pointerdown, move |evt| {
            evt.prevent_default();
            let (client_x, client_y) = (evt.client_x(), evt.client_y());
            resize_start_pos.set(Some((client_x, client_y)));
        });

    let _resize_stops_ev = use_event_listener(window.clone(), leptos::ev::pointerup, move |_| {
        // Pointerup event isn't associated to a resize event for this component
        if resize_start_pos.get().is_none() {
            return;
        }

        log!("[GridItem][{id}] _resize_stops_ev");
        resize_start_pos.set(None);

        metadata.update(|data| {
            data.width = width.get();
            data.height = height.get();
        });

        update_grid.update(|grid| {
            grid.items.insert(id, metadata.get().clone());
        });
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
            let resize_start_pos_untracked = resize_start_pos.get_untracked();

            if x.abs() > 100 {
                let new_col_span = (height.get_untracked() + (x / x.abs())).clamp(1, 12);
                height.set(new_col_span);

                if let Some((resize_start_x, resize_start_y)) = resize_start_pos_untracked {
                    // Update the resize start position to the new one
                    resize_start_pos.set(Some((resize_start_x + x, resize_start_y)));
                    // Reset the offset
                    resize_offset.set((0, 0));
                }
            }

            if y.abs() > 100 {
                let new_row_span = (position_y.get_untracked() + (y / y.abs())).clamp(0, 12);
                position_y.set(new_row_span);

                if let Some((resize_start_x, resize_start_y)) = resize_start_pos_untracked {
                    // Update the resize start position to the new one
                    resize_start_pos.set(Some((resize_start_x, resize_start_y + y)));
                    // Reset the offset
                    resize_offset.set((0, 0));
                }
            }
        },
        false,
    );

    view! {
        <div
            class=move || {
                format!("relative p-4 cursor-move border-2 border-gray-500 {}",
                into_tailwind(width.get(), height.get(), position_x.get(), position_y.get()))
            }
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
