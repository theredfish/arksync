// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::grid::core::item::{GridItemData, GridPosition};
use crate::components::grid::core::layout::Layout;
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

    /// Creates a preview from the current drag pixel position.
    ///
    /// The drag preview follows the currently hovered grid slot. It does not
    /// try to predict whether the final placement will swap, push, or restore.
    pub fn from_drag(item: &GridItemData, drag_px_pos: Position, layout: &Layout) -> Self {
        let max_col_start = layout.columns.saturating_sub(item.span.col_span);
        let col_start =
            ((drag_px_pos.x / layout.cell_size.width).round() as usize).min(max_col_start);
        let row_start = (drag_px_pos.y / layout.cell_size.height).round() as usize;

        Self::new(
            item.id,
            GridPosition {
                col_start,
                row_start,
            },
            item.span,
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::grid::core::layout::LayoutBuilder;

    fn layout() -> Layout {
        LayoutBuilder::default()
            .columns(4)
            .rows(4)
            .cell_size(100.0, 50.0)
            .build()
    }

    fn item() -> GridItemData {
        GridItemData {
            id: 7,
            grid_pos: GridPosition {
                col_start: 0,
                row_start: 0,
            },
            span: Span {
                col_span: 2,
                row_span: 1,
            },
            ..GridItemData::default()
        }
    }

    #[test]
    fn from_drag_follows_the_hovered_rounded_grid_slot() {
        let preview = DropPreview::from_drag(&item(), Position { x: 149.0, y: 76.0 }, &layout());

        assert_eq!(preview.item_id, 7);
        assert_eq!(preview.grid_pos.col_start, 1);
        assert_eq!(preview.grid_pos.row_start, 2);
        assert_eq!(preview.span.col_span, 2);
        assert_eq!(preview.span.row_span, 1);
    }

    #[test]
    fn from_drag_clamps_the_preview_inside_available_columns() {
        let preview = DropPreview::from_drag(&item(), Position { x: 999.0, y: 0.0 }, &layout());

        assert_eq!(preview.grid_pos.col_start, 2);
        assert_eq!(preview.grid_pos.row_start, 0);
    }

    #[test]
    fn pixel_rect_converts_grid_preview_back_to_pixels() {
        let preview = DropPreview::new(
            7,
            GridPosition {
                col_start: 2,
                row_start: 3,
            },
            Span {
                col_span: 2,
                row_span: 4,
            },
        );

        let (position, size) = preview.pixel_rect(Size {
            width: 100.0,
            height: 50.0,
        });

        assert_eq!(position.x, 200.0);
        assert_eq!(position.y, 150.0);
        assert_eq!(size.width, 200.0);
        assert_eq!(size.height, 200.0);
    }
}
