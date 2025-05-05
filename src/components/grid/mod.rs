mod grid_item;
mod grid_layout;

#[derive(Clone, Debug, Default)]
pub struct Size {
    width: f64,
    height: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Layout {
    size: RwSignal<Size>,
    items: RwSignal<HashMap<u32, GridItemData>>,
    columns: u8,
}

impl Layout {
    fn add_item(&mut self, id: u32, data: GridItemData) {
        self.items.update(|items| {
            items.insert(id, data);
        });
    }
}

#[derive(Clone, Debug, Default)]
pub struct LayoutBuilder {
    size: Size,
    items: HashMap<u32, GridItemData>,
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

    fn size(mut self, width: f64, height: f64) -> Self {
        self.size = Size { width, height };
        self
    }

    fn items(mut self, items: HashMap<u32, GridItemData>) -> Self {
        self.items = items;
        self
    }

    fn build(self) -> Layout {
        Layout {
            size: RwSignal::new(self.size),
            items: RwSignal::new(self.items),
            columns: self.columns,
        }
    }
}

#[derive(Clone, Debug)]
struct GridItemData {
    size: Size,
    position: Position,
}

use std::collections::HashMap;

use leptos::prelude::{RwSignal, Update};
use leptos_use::core::Position;

pub use self::grid_item::GridItem;
pub use self::grid_layout::GridLayout;
