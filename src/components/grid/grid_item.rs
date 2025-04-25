use crate::components::grid::{GridItemData, GridStorage};
use leptos::html::Div;
use leptos::logging::log;
use leptos::prelude::*;
use leptos_use::{
    core::Position, use_draggable_with_options, use_event_listener, UseDraggableOptions,
};

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
    let component_ref = NodeRef::<Div>::new();
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
    let draggable_return = use_draggable_with_options(
        component_ref,
        UseDraggableOptions::default()
            .initial_value(Position {
                x: metadata.get().position.0,
                y: metadata.get().position.1,
            })
            .prevent_default(true),
    );

    let styles = move || {
        let GridItemData { width, height, .. } = metadata.get();
        let (pos_x, pos_y) = (draggable_return.x.get(), draggable_return.y.get());

        format!(
            r#"
            width: {width}px;
            height: {height}px;
            transition: width 0.2s ease-out, height 0.2s ease-in;
            transform: translate({pos_x}px, {pos_y}px)
        "#
        )
    };

    view! {
        <div
            node_ref=component_ref
            style={styles}
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
