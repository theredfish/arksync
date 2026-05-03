// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::grid::core::collision::aabb::Aabb;
use crate::components::grid::core::item::GridItemData;
use crate::components::grid::core::size::Size;
use leptos_use::core::Position;

pub fn from_item(item: &GridItemData) -> Aabb {
    Aabb::new(
        item.grid_pos.col_start as f64,
        item.grid_pos.row_start as f64,
        item.span.col_span as f64,
        item.span.row_span as f64,
    )
}

pub fn from_drag(item: &GridItemData, drag_px_pos: Position, cell_size: Size) -> Aabb {
    Aabb::new(
        drag_px_pos.x / cell_size.width,
        drag_px_pos.y / cell_size.height,
        item.span.col_span as f64,
        item.span.row_span as f64,
    )
}

pub fn items_overlap(a: &GridItemData, b: &GridItemData) -> bool {
    from_item(a).overlap(&from_item(b)).is_some()
}

pub fn overlap_item(aabb: Aabb, item: &GridItemData) -> Option<(f64, f64)> {
    aabb.overlap(&from_item(item))
}
