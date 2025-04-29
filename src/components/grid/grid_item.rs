use std::sync::Arc;

use crate::components::grid::{GridItemData, GridStorage};
use leptos::html::Div;
use leptos::prelude::*;
use leptos_use::{
    core::Position, use_draggable_with_options, use_element_bounding, use_event_listener,
    UseDraggableOptions, UseDraggableReturn, UseElementBoundingReturn,
};
use wasm_bindgen::JsCast;

#[component]
pub fn GridItem(
    children: Children,
    id: i32,
    width: i32,
    height: i32,
    position_x: f64,
    position_y: f64,
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
        // log!("[GridItem][{id}]: {:#?}", metadata.clone());
        update_grid.update(|grid| {
            grid.items.insert(id, metadata.clone());
        });
    });

    let window = window();
    let grid_item_ref = NodeRef::<Div>::new();
    let resize_button_ref = NodeRef::<Div>::new();

    // resize events
    {
        let resize_start_pos = RwSignal::new(None::<(i32, i32)>);
        let resize_offset = RwSignal::new((0, 0));

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
                    data.width = prev_metadata.width + offset.0;
                    data.height = prev_metadata.height + offset.1;
                });

                update_grid.update(|grid| {
                    grid.items.insert(id, metadata.get().clone());
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

    // TODO: pass width/height signals from parent so we can listen on the
    // responsive changes. Or observe the resize event maybe.
    let UseElementBoundingReturn {
        width: absolute_parent_w,
        height: absolute_parent_h,
        ..
    } = use_element_bounding(absolute_parent_el);

    let UseDraggableReturn { x, y, .. } = use_draggable_with_options(
        grid_item_ref,
        UseDraggableOptions::default()
            .initial_value(Position {
                // x: metadata.get().position.0,
                // y: metadata.get().position.1,
                x: 40.,
                y: 40.,
            })
            .target_offset(Arc::new(move |event_target| {
                let target: web_sys::HtmlElement = event_target.unchecked_into();
                let (x, y): (f64, f64) = (target.offset_left().into(), target.offset_top().into());

                (x, y)
            }))
            .prevent_default(true),
    );

    // TODO: adapt with reactive parent information
    let left = move || {
        let max = absolute_parent_w.get() - x.get();
        x.get().clamp(0.0, max.round())
    };
    let top = move || {
        let max = absolute_parent_h.get() - y.get();
        y.get().clamp(0.0, max.round())
    };

    let styles = move || {
        let GridItemData { width, height, .. } = metadata.get();

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
            style={styles}
            class="absolute p-4 cursor-move border-2 border-gray-500"
            data-id=id.to_string()
        >
            { move || x.get() };{ move || y.get() }
            { children() }
            <div
                node_ref=resize_button_ref
                class="absolute bottom-0 right-0 w-4 h-4 bg-red-500 cursor-se-resize"
                data-id=id.to_string()
            ></div>
        </div>
    }
}
