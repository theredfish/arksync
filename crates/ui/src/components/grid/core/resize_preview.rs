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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResizePreview {
    /// Item currently being resized.
    pub item_id: u32,
    /// Fixed top-left position used as the resize anchor.
    pub grid_pos: GridPosition,
    /// Span used as the fixed compositor surface while the preview animates.
    pub initial_span: Span,
    /// Snapped target span under the active resize gesture.
    pub span: Span,
}

impl ResizePreview {
    /// Creates a resize preview from an item id, anchor position, and target span.
    pub fn new(item_id: u32, grid_pos: GridPosition, span: Span) -> Self {
        Self {
            item_id,
            grid_pos,
            initial_span: span,
            span,
        }
    }

    /// Creates a preview from the current resize pixel size.
    ///
    /// This keeps the preview math outside the component callbacks. The span is
    /// snapped to the nearest cell, then clamped to the remaining columns.
    pub fn from_resize(item: &GridItemData, size: Size, layout: &Layout) -> Self {
        let max_col_span = layout
            .columns
            .saturating_sub(item.grid_pos.col_start)
            .max(1);
        let col_span = snap_resize_span(size.width, layout.cell_size.width).min(max_col_span);
        let row_span = snap_resize_span(size.height, layout.cell_size.height);

        Self {
            item_id: item.id,
            grid_pos: item.grid_pos,
            initial_span: item.span,
            span: Span { row_span, col_span },
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

/// Snaps a resize dimension to the grid boundary closest to the pointer.
///
/// A small movement (or a fractional CSS pixel) must not add or remove an
/// entire cell. Preview and release share this half-cell threshold so the
/// target stays within half a cell of the requested size, except at bounds.
pub fn snap_resize_span(raw_px: f64, cell_px: f64) -> usize {
    ((raw_px / cell_px).round() as usize).max(1)
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
    fn small_pointer_movements_do_not_jump_a_cell() {
        for raw_px in [199.0, 200.0, 201.0] {
            assert_eq!(snap_resize_span(raw_px, 100.0), 2);
        }
        assert_eq!(snap_resize_span(1.0, 100.0), 1);
    }

    #[test]
    fn resize_snaps_at_half_cell_boundaries_in_both_directions() {
        for (raw_px, expected_span) in [
            (249.0, 2),
            (250.0, 3),
            (351.0, 4),
            (349.0, 3),
            (200.0, 2),
            (150.0, 2),
            (149.0, 1),
        ] {
            assert_eq!(snap_resize_span(raw_px, 100.0), expected_span);
        }
    }

    #[test]
    fn fractional_cells_keep_the_untouched_axis_at_its_original_span() {
        let layout = LayoutBuilder::default()
            .columns(12)
            .cell_size(111.7, 111.7)
            .build();
        let item = GridItemData {
            span: Span {
                col_span: 4,
                row_span: 3,
            },
            ..item()
        };

        for offset in [-1.0, 0.0, 1.0] {
            let preview = ResizePreview::from_resize(
                &item,
                Size {
                    width: 4.0 * layout.cell_size.width + offset,
                    height: 3.0 * layout.cell_size.height,
                },
                &layout,
            );
            assert_eq!(preview.span, item.span);
            assert_eq!(preview.initial_span, item.span);
        }

        let preview = ResizePreview::from_resize(
            &item,
            Size {
                width: 4.0 * layout.cell_size.width + 60.0,
                height: 3.0 * layout.cell_size.height,
            },
            &layout,
        );
        assert_eq!(preview.span.col_span, 5);
        assert_eq!(preview.span.row_span, 3);
    }

    #[test]
    fn from_resize_anchors_position_and_clamps_to_remaining_columns() {
        let preview = ResizePreview::from_resize(
            &item(),
            Size {
                width: 999.0,
                height: 176.0,
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
