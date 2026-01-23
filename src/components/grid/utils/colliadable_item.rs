// Takes a draggable signal
// Takes a resizable signal
// Updates the colliadable item position/size based on those signals
// Retrieve the GridItemData based on the item position or id
// Updates the grid layout with the new position/size
// Uses AABB collision detection to check for overlaps with other items

use crate::components::grid::core::item::GridItemData;
use crate::components::grid::core::layout::Layout;
use leptos::logging::log;
use leptos::prelude::*;

/// We need to avoid spreading this collision result if there is no movements
/// such as a draggable or resizable action
pub struct Collision {
    pub items: Option<Vec<GridItemData>>,
}

pub struct UseCollidableGridItemOptions {
    /// Grid item data signal
    pub grid_item: RwSignal<GridItemData>,
}

pub struct UseCollidableGridItemReturn {
    /// Whether the grid item is colliding with another item
    pub collision: Signal<Collision>,
}

pub fn use_collidable_grid_item(
    options: UseCollidableGridItemOptions,
) -> UseCollidableGridItemReturn {
    let UseCollidableGridItemOptions { grid_item } = options;

    log!("use_collidable_grid_item");

    Effect::watch(
        move || grid_item.get(),
        move |grid_item, previous_version, _| {
            let layout = use_context::<RwSignal<Layout>>()
                .expect("Layout context must be provided")
                .get_untracked();
            log!("update of the grid item detected");
        },
        false,
    );

    // Effect::watch(
    //     move || grid_item.get(),
    //     move |grid_item, previous_version, _| {
    //         log!("watch");
    //         let collision_id = grid_item.collision_id();
    //         let layout =
    //             use_context::<RwSignal<Layout>>().expect("Layout context must be provided");
    //         log!("grid_item: {grid_item:#?}; previous: {previous_version:#?};");

    //         // Detect changes
    //         if let Some(previous) = previous_version {
    //             let previous_collision_id = previous.collision_id();
    //             if collision_id == previous_collision_id {
    //                 return;
    //             }
    //         }
    //         // Detect if there a size change
    //     },
    //     false,
    // );

    // => a unique ID based on the position for each element
    // If the x direction is left => check for right corners in layout
    // If the x direction is right => check for left corners
    // If the y direcction is up => check for bottom corners
    // If the y direction is down => check for top corners*
    //
    // The drag events is triggering any direction, all are possibler
    // but the resize, is only starting from the right bottom corner. So the
    // only possible direction is: right and down

    // let other_item
    // let collides = grid_item.collides_with_optional(layout.item_from());
    // grid_item.collides_with(other)

    // UseCollidableGridItemReturn {
    //     collision,
    // }
    //
    UseCollidableGridItemReturn {
        collision: Signal::from(Collision { items: None }),
    }
}
