mod grid_item;
mod grid_layout;

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

pub use self::grid_item::GridItem;
pub use self::grid_layout::GridLayout;
