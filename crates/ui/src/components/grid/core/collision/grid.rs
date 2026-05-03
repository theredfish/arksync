// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::grid::core::collision::aabb::Aabb;
use crate::components::grid::core::item::GridItemData;
use ndarray::Array2;
use std::collections::HashSet;

pub fn item_ids_for_item(collision_grid: &Array2<Option<u32>>, item: &GridItemData) -> Vec<u32> {
    let row_end = item.grid_pos.row_start + item.span.row_span;
    let col_end = item.grid_pos.col_start + item.span.col_span;

    collect_item_ids(
        collision_grid,
        item.grid_pos.row_start,
        row_end,
        item.grid_pos.col_start,
        col_end,
        item.id,
    )
}

pub fn item_ids_for_aabb(
    collision_grid: &Array2<Option<u32>>,
    aabb: Aabb,
    excluded_id: u32,
) -> Vec<u32> {
    let row_start = aabb.top.floor().max(0.0) as usize;
    let row_end = aabb.bottom.ceil().max(0.0) as usize;
    let col_start = aabb.left.floor().max(0.0) as usize;
    let col_end = aabb.right.ceil().max(0.0) as usize;

    collect_item_ids(
        collision_grid,
        row_start,
        row_end,
        col_start,
        col_end,
        excluded_id,
    )
}

pub fn item_fits_ignoring(
    collision_grid: &Array2<Option<u32>>,
    item: &GridItemData,
    ignored_ids: &[u32],
) -> bool {
    let ignored_ids = ignored_ids.iter().copied().collect::<HashSet<_>>();
    let row_end = item.grid_pos.row_start + item.span.row_span;
    let col_end = item.grid_pos.col_start + item.span.col_span;

    for row_idx in item.grid_pos.row_start..row_end.min(collision_grid.nrows()) {
        for col_idx in item.grid_pos.col_start..col_end.min(collision_grid.ncols()) {
            if let Some(occupant_id) = collision_grid[[row_idx, col_idx]] {
                if !ignored_ids.contains(&occupant_id) {
                    return false;
                }
            }
        }
    }

    true
}

pub fn set_item(collision_grid: &mut Array2<Option<u32>>, item: &GridItemData) {
    let row_start = item.grid_pos.row_start;
    let row_end = row_start + item.span.row_span;
    let col_start = item.grid_pos.col_start;
    let col_end = col_start + item.span.col_span;

    for row_idx in row_start..row_end {
        for col_idx in col_start..col_end {
            if row_idx < collision_grid.nrows() && col_idx < collision_grid.ncols() {
                collision_grid[[row_idx, col_idx]] = Some(item.id);
            }
        }
    }
}

pub fn clear_item(collision_grid: &mut Array2<Option<u32>>, item: &GridItemData) {
    let row_start = item.grid_pos.row_start;
    let row_end = row_start + item.span.row_span;
    let col_start = item.grid_pos.col_start;
    let col_end = col_start + item.span.col_span;

    for row_idx in row_start..row_end {
        for col_idx in col_start..col_end {
            if row_idx < collision_grid.nrows()
                && col_idx < collision_grid.ncols()
                && collision_grid[[row_idx, col_idx]] == Some(item.id)
            {
                collision_grid[[row_idx, col_idx]] = None;
            }
        }
    }
}

fn collect_item_ids(
    collision_grid: &Array2<Option<u32>>,
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
    excluded_id: u32,
) -> Vec<u32> {
    let mut colliding_ids = HashSet::new();

    for row_idx in row_start..row_end.min(collision_grid.nrows()) {
        for col_idx in col_start..col_end.min(collision_grid.ncols()) {
            if let Some(occupant_id) = collision_grid[[row_idx, col_idx]] {
                if occupant_id != excluded_id {
                    colliding_ids.insert(occupant_id);
                }
            }
        }
    }

    colliding_ids.into_iter().collect()
}
