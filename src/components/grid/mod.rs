mod grid_component;
mod grid_item;

#[derive(Clone, Debug)]
struct GridBounding {
    width: f64,
    height: f64,
}

#[derive(Clone, Debug)]
struct GridContext {
    storage: HashMap<i32, GridItemData>,
    boundaries: GridBounding,
}

#[derive(Clone, Debug)]
struct GridItemData {
    width: i32,
    height: i32,
    position: (f64, f64),
}

use std::collections::HashMap;

pub use self::grid_component::Grid;
pub use self::grid_item::GridItem;
