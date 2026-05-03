use crate::components::grid::core::item::GridPosition;
use crate::components::grid::core::size::Size;
use crate::components::grid::core::span::Span;
use leptos_use::core::Position;

/// Visual preview of the grid slot currently targeted by a drag gesture.
///
/// The preview is UI-only: it does not reserve cells in the collision grid and
/// does not mutate item positions. The actual placement is still resolved by
/// `Layout` when the drag ends.
#[derive(Clone, Copy, Debug)]
pub struct DropPreview {
    /// Item currently driving the preview.
    pub item_id: u32,
    /// Snapped grid position under the active drag.
    pub grid_pos: GridPosition,
    /// Span used to size the preview rectangle.
    pub span: Span,
}

impl DropPreview {
    /// Creates a preview from an item id, snapped grid position, and current span.
    pub fn new(item_id: u32, grid_pos: GridPosition, span: Span) -> Self {
        Self {
            item_id,
            grid_pos,
            span,
        }
    }

    /// Converts the snapped grid position to a pixel rectangle.
    pub fn pixel_rect(self, cell_size: Size) -> (Position, Size) {
        (
            Position {
                x: self.grid_pos.col_start as f64 * cell_size.width,
                y: self.grid_pos.row_start as f64 * cell_size.height,
            },
            Size {
                width: self.span.col_span as f64 * cell_size.width,
                height: self.span.row_span as f64 * cell_size.height,
            },
        )
    }
}
