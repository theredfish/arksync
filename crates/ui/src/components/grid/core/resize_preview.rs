use crate::components::grid::core::item::GridPosition;
use crate::components::grid::core::size::Size;
use crate::components::grid::core::span::Span;
use leptos_use::core::Position;

/// Visual preview of the grid rectangle targeted by a resize gesture.
///
/// The preview is UI-only. It mirrors the snapped size that will be submitted
/// to `Layout::resize_item_with_collision` when the resize ends, but it does
/// not update the collision grid while the pointer is still moving.
#[derive(Clone, Copy, Debug)]
pub struct ResizePreview {
    /// Item currently being resized.
    pub item_id: u32,
    /// Fixed top-left position used as the resize anchor.
    pub grid_pos: GridPosition,
    /// Snapped target span under the active resize gesture.
    pub span: Span,
}

impl ResizePreview {
    /// Creates a resize preview from an item id, anchor position, and target span.
    pub fn new(item_id: u32, grid_pos: GridPosition, span: Span) -> Self {
        Self {
            item_id,
            grid_pos,
            span,
        }
    }

    /// Converts the anchored grid span to a pixel rectangle.
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
