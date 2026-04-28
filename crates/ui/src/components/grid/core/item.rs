use leptos_use::core::Position;

use crate::components::grid::core::{size::Size, span::Span};

#[derive(Clone, Copy, Debug, Default)]
pub struct GridPosition {
    pub col_start: usize,
    pub row_start: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GridItemData {
    pub id: u32,
    pub px_pos: Position,
    pub grid_pos: GridPosition,
    pub span: Span,
    pub size: Size,
}

pub enum Axes {
    X,
    Y,
    XY,
}

impl GridItemData {
    /// Converts from grid coordinated to pixel coordinates.
    ///
    /// Only update the axes based on the parameter `axes_to_update`.
    pub fn grid_to_pixels(&mut self, cell_size: Size, axes_to_update: Axes) {
        let Size { width, height } = cell_size;
        let x = (self.grid_pos.col_start) as f64 * width;
        let y = (self.grid_pos.row_start) as f64 * height;

        self.px_pos = match axes_to_update {
            Axes::X => Position {
                x,
                y: self.px_pos.y,
            },
            Axes::Y => Position {
                x: self.px_pos.x,
                y,
            },
            Axes::XY => Position { x, y },
        };
    }

    /// Unused but might be helpful for later
    pub fn _pixels_to_grid(&mut self, cell_width: f64, cell_height: f64) {
        let row_start = ((self.px_pos.x / cell_height).round() as usize).max(1);
        let col_start = ((self.px_pos.y / cell_width).round() as usize).max(1);

        self.grid_pos = GridPosition {
            col_start,
            row_start,
        };
    }
}
