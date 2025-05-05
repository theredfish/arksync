use leptos::prelude::{RwSignal, Update};
use leptos_use::core::Position;
use std::collections::HashMap;

mod grid_item;
mod grid_layout;

pub use self::grid_item::GridItem;
pub use self::grid_layout::GridLayout;

#[derive(Clone, Debug, Default)]
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
}

impl Layout {
    fn add_item(&mut self, id: u32, data: RwSignal<GridItemData>) {
        self.items.insert(id, data);
    }

    fn update_items_size(&mut self, (w_ratio, h_ratio): (f64, f64)) {
        for item in self.items.values() {
            item.update(|grid_item| {
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
}

impl LayoutBuilder {
    fn new() -> Self {
        LayoutBuilder {
            size: Size::default(),
            items: HashMap::new(),
            columns: u8::default(),
        }
    }

    fn columns(mut self, quantity: u8) -> Self {
        self.columns = quantity;
        self
    }

    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.size = Size { width, height };
        self
    }

    fn items(mut self, items: HashMap<u32, RwSignal<GridItemData>>) -> Self {
        self.items = items;
        self
    }

    fn build(self) -> Layout {
        Layout {
            size: self.size,
            items: self.items,
            columns: self.columns,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct GridItemData {
    size: Size,
    position: Position,
}
