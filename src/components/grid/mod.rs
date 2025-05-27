use leptos::logging::log;
use leptos::prelude::{RwSignal, Update};
use std::collections::HashMap;

mod grid_item;
mod grid_layout;

pub use self::grid_item::GridItem;
pub use self::grid_layout::GridLayout;

#[derive(Clone, Debug)]
pub struct GridItemPosition {
    col_start: u32,
    row_start: u32,
}

impl Default for GridItemPosition {
    fn default() -> Self {
        Self {
            col_start: 1,
            row_start: 1,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GridItemSpan {
    col_span: u32,
    row_span: u32,
}

impl Default for GridItemSpan {
    fn default() -> Self {
        Self {
            col_span: 1,
            row_span: 1,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd)]
pub struct Size {
    width: f64,
    height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Self {
        Size { width, height }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Layout {
    size: Size,
    items: HashMap<u32, RwSignal<GridItemData>>,
    columns: u8,
    cell_size: Size,
}

impl Layout {
    fn add_item(&mut self, id: u32, data: RwSignal<GridItemData>) {
        self.items.insert(id, data);
    }

    fn update_items_size(&mut self, (w_ratio, h_ratio): (f64, f64)) {
        for item in self.items.values() {
            item.update(|grid_item| {
                log!("update item size responsiveness");
                let Size { width, height } = grid_item.size;
                let new_size = Size::new(width * w_ratio, height * h_ratio);
                grid_item.size = new_size;
            });
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LayoutBuilder {
    size: Size,
    items: HashMap<u32, RwSignal<GridItemData>>,
    columns: u8,
    cell_size: Size,
}

impl LayoutBuilder {
    fn columns(mut self, quantity: u8) -> Self {
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

    fn build(self) -> Layout {
        Layout {
            size: self.size,
            items: self.items,
            columns: self.columns,
            cell_size: self.cell_size,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct GridItemData {
    position: GridItemPosition,
    span: GridItemSpan,
    size: Size,
}
