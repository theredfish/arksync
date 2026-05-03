// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::grid::core::item::{GridItemData, GridPosition};
use crate::components::grid::core::layout::Layout;
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

    /// Creates a preview from the current resize pixel size.
    ///
    /// This keeps the preview math outside the component callbacks. The span is
    /// snapped in the resize direction, then clamped to the remaining columns.
    pub fn from_resize(item: &GridItemData, size: Size, layout: &Layout) -> Self {
        let max_col_span = layout
            .columns
            .saturating_sub(item.grid_pos.col_start)
            .max(1);
        let col_span =
            directional_snap_span(size.width, item.span.col_span, layout.cell_size.width)
                .min(max_col_span);
        let row_span =
            directional_snap_span(size.height, item.span.row_span, layout.cell_size.height);

        Self::new(item.id, item.grid_pos, Span { row_span, col_span })
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

/// Snaps a resize dimension in the direction of the pointer movement.
///
/// Growing targets the next grid cell and shrinking targets the previous one,
/// which makes previews and final sizes follow the resize direction instead of
/// waiting for a half-cell `round()` threshold.
pub fn directional_snap_span(raw_px: f64, current_span: usize, cell_px: f64) -> usize {
    let raw_span = raw_px / cell_px;
    let current_span = current_span.max(1) as f64;

    let snapped_span = if raw_span > current_span {
        raw_span.ceil()
    } else if raw_span < current_span {
        raw_span.floor()
    } else {
        current_span
    };

    (snapped_span as usize).max(1)
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
            id: 11,
            grid_pos: GridPosition {
                col_start: 1,
                row_start: 2,
            },
            span: Span {
                col_span: 2,
                row_span: 3,
            },
            ..GridItemData::default()
        }
    }

    #[test]
    fn directional_snap_span_announces_growth_and_shrink_in_pointer_direction() {
        assert_eq!(directional_snap_span(201.0, 2, 100.0), 3);
        assert_eq!(directional_snap_span(199.0, 2, 100.0), 1);
        assert_eq!(directional_snap_span(200.0, 2, 100.0), 2);
        assert_eq!(directional_snap_span(1.0, 2, 100.0), 1);
    }

    #[test]
    fn from_resize_anchors_position_and_clamps_to_remaining_columns() {
        let preview = ResizePreview::from_resize(
            &item(),
            Size {
                width: 999.0,
                height: 151.0,
            },
            &layout(),
        );

        assert_eq!(preview.item_id, 11);
        assert_eq!(preview.grid_pos.col_start, 1);
        assert_eq!(preview.grid_pos.row_start, 2);
        assert_eq!(preview.span.col_span, 3);
        assert_eq!(preview.span.row_span, 4);
    }

    #[test]
    fn pixel_rect_converts_resize_preview_back_to_pixels() {
        let preview = ResizePreview::new(
            11,
            GridPosition {
                col_start: 1,
                row_start: 2,
            },
            Span {
                col_span: 3,
                row_span: 4,
            },
        );

        let (position, size) = preview.pixel_rect(Size {
            width: 100.0,
            height: 50.0,
        });

        assert_eq!(position.x, 100.0);
        assert_eq!(position.y, 100.0);
        assert_eq!(size.width, 300.0);
        assert_eq!(size.height, 200.0);
    }
}
