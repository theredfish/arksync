use crate::components::grid::core::{item::GridItemData, size::Size};
use leptos::prelude::RwSignal;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub size: Size,
    // TODO: see for detaching this global reactive state from the layout config
    // And use signals in the config so we can listen on different events
    pub items: HashMap<u32, RwSignal<GridItemData>>,
    pub columns: u32,
    pub cell_size: Size,
}

impl Layout {
    pub fn add_item(&mut self, id: u32, data: RwSignal<GridItemData>) {
        self.items.insert(id, data);
    }

    pub fn update_items_size(&mut self, (w_ratio, h_ratio): (f64, f64)) {
        // we used to update the grid items size here, so we can prepare the
        // collision detection and re-layout the items. But let's skip thos for
        // now since we are not using that feature yet.
    }
}

#[derive(Clone, Debug, Default)]
pub struct LayoutBuilder {
    pub size: Size,
    pub items: HashMap<u32, RwSignal<GridItemData>>,
    pub columns: u32,
    pub cell_size: Size,
}

impl LayoutBuilder {
    pub fn columns(mut self, quantity: u32) -> Self {
        self.columns = quantity;
        self
    }

    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.size = Size { width, height };
        self
    }

    pub fn cell_size(mut self, width: f64, height: f64) -> Self {
        self.cell_size = Size { width, height };
        self
    }

    pub fn build(self) -> Layout {
        Layout {
            size: self.size,
            items: self.items,
            columns: self.columns,
            cell_size: self.cell_size,
        }
    }
}
