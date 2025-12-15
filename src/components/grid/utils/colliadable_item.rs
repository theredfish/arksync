// Takes a draggable signal
// Takes a resizable signal
// Updates the colliadable item position/size based on those signals
// Retrieve the GridItemData based on the item position or id
// Updates the grid layout with the new position/size
// Uses AABB collision detection to check for overlaps with other items

use crate::aabb::Aabb;
use crate::components::grid::{GridItemData, Layout};
use leptos::prelude::*;

pub struct UseCollidableGridItemOptions {
    /// Grid item data signal
    pub grid_item: Signal<GridItemData>,
}

pub struct UseCollidableGridItemReturn {
    /// Whether the grid item is colliding with another item
    pub colliding: Signal<bool>,
}

pub fn use_collidable_grid_item(
    options: UseCollidableGridItemOptions,
) -> UseCollidableGridItemReturn {
    let layout = use_context::<RwSignal<Layout>>()
        .expect("Layout context must be provided")
        .get();
    let grid_item = options.grid_item.get();

    // let other_item
    // let collides = grid_item.collides_with_optional(layout.item_from());
    // grid_item.collides_with(other)

    UseCollidableGridItemReturn {
        colliding: Signal::from(false),
    }
}
