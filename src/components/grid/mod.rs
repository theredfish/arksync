use leptos::prelude::RwSignal;
use std::collections::HashMap;

mod grid_item;
mod grid_layout;
mod utils;

pub use self::grid_item::GridItem;
pub use self::grid_layout::GridLayout;

#[derive(Clone, Copy, Debug)]
pub struct GridItemPosition {
    pub col_start: u32,
    pub row_start: u32,
}

impl Default for GridItemPosition {
    fn default() -> Self {
        Self {
            col_start: 1,
            row_start: 1,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Span {
    pub col_span: u32,
    pub row_span: u32,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            col_span: 1,
            row_span: 1,
        }
    }
}

/// Size in pixels
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Self {
        Size { width, height }
    }
}

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
    fn add_item(&mut self, id: u32, data: RwSignal<GridItemData>) {
        self.items.insert(id, data);
    }

    fn update_items_size(&mut self, (w_ratio, h_ratio): (f64, f64)) {
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
    fn columns(mut self, quantity: u32) -> Self {
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

#[derive(Clone, Copy, Debug, Default)]
pub struct GridItemData {
    pub id: u32,
    pub position: GridItemPosition,
    pub span: Span,
    pub size: Size,
}

impl GridItemData {
    pub fn min_x(&self) -> f64 {
        // 2 => minx = 2-1 * 100 => 100
        (self.position.col_start - 1) as f64 * self.size.width
    }

    pub fn max_x(&self) -> f64 {
        (self.size.width * self.span.col_span as f64) + self.min_x()
    }

    pub fn min_y(&self) -> f64 {
        (self.position.row_start - 1) as f64 * self.size.height
    }

    pub fn max_y(&self) -> f64 {
        (self.size.height * self.span.row_span as f64) + self.min_y()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_grid_item_data_min_max() {
        let item = GridItemData {
            id: 1,
            position: GridItemPosition {
                col_start: 2,
                row_start: 3,
            },
            span: Span {
                col_span: 2,
                row_span: 2,
            },
            size: Size {
                width: 100.0,
                height: 100.0,
            },
        };

        assert_eq!(item.min_x(), 100.0, "incorrect min_x calculation");
        assert_eq!(item.max_x(), 300.0, "incorrect max_x calculation");
        assert_eq!(item.min_y(), 200.0, "incorrect min_y calculation");
        assert_eq!(item.max_y(), 400.0, "incorrect max_y calculation");
    }

    #[test]
    fn test_grid_item_data_min_max_at_top_left_edge() {
        let item = GridItemData {
            id: 1,
            position: GridItemPosition {
                col_start: 1,
                row_start: 1,
            },
            span: Span {
                col_span: 2,
                row_span: 2,
            },
            size: Size {
                width: 100.0,
                height: 100.0,
            },
        };

        assert_eq!(item.min_x(), 0.0, "incorrect min_x calculation");
        assert_eq!(item.max_x(), 200.0, "incorrect max_x calculation");
        assert_eq!(item.min_y(), 0.0, "incorrect min_y calculation");
        assert_eq!(item.max_y(), 200.0, "incorrect max_y calculation");
    }
}
