use crate::components::grid::core::collision::{grid as collision_grid, item_aabb};
use crate::components::grid::core::item::Axes;
use crate::components::grid::core::{
    item::{GridItemData, GridPosition},
    size::Size,
};
use leptos::{
    logging::log,
    prelude::{GetUntracked, RwSignal, Set, Update},
};
use leptos_use::core::Position;
use ndarray::{concatenate, Array2, Axis};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub size: Size,
    /// The collision grid storing the occupancy of each cell by item id
    pub collision_grid: Array2<Option<u32>>,
    /// The grid items collection stored as signals
    pub items: HashMap<u32, RwSignal<GridItemData>>,
    /// The number of rows in the grid
    pub rows: usize,
    /// The number of columns in the grid
    pub columns: usize,
    /// The size of each cell in the grid
    pub cell_size: Size,
}

impl Layout {
    fn ensure_rows(&mut self, required_rows: usize) {
        if required_rows <= self.rows {
            return;
        }

        let rows_to_add = required_rows - self.rows;
        let empty_rows = Array2::from_elem((rows_to_add, self.columns), None::<u32>);
        self.collision_grid =
            concatenate(Axis(0), &[self.collision_grid.view(), empty_rows.view()])
                .expect("Failed to concatenate empty rows at bottom");
        self.rows = required_rows;
    }

    fn clamp_position_for_item(&self, item: &GridItemData, row: usize, col: usize) -> GridPosition {
        let max_col = self.columns.saturating_sub(item.span.col_span);

        GridPosition {
            row_start: row,
            col_start: col.min(max_col),
        }
    }

    fn col_ranges_overlap(a_start: usize, a_span: usize, b_start: usize, b_span: usize) -> bool {
        a_start < b_start + b_span && b_start < a_start + a_span
    }

    fn restore_item_position(&mut self, item: RwSignal<GridItemData>, mut item_data: GridItemData) {
        item_data.grid_to_pixels(self.cell_size, Axes::XY);
        item.set(item_data);
        collision_grid::set_item(&mut self.collision_grid, &item_data);
    }

    fn try_swap_items(
        &mut self,
        moved_item: &GridItemData,
        old_position: GridPosition,
        colliding_ids: &[u32],
    ) -> Option<GridPosition> {
        if colliding_ids.len() != 1 {
            return None;
        }

        let colliding_id = colliding_ids[0];
        let Some(&colliding_item_signal) = self.items.get(&colliding_id) else {
            return None;
        };

        let mut colliding_item = colliding_item_signal.get_untracked();
        if old_position.col_start != moved_item.grid_pos.col_start {
            return None;
        }

        if !Self::col_ranges_overlap(
            old_position.col_start,
            moved_item.span.col_span,
            colliding_item.grid_pos.col_start,
            colliding_item.span.col_span,
        ) {
            return None;
        }

        let colliding_position = colliding_item.grid_pos;
        let (moved_swap_position, colliding_swap_position) =
            if old_position.row_start < colliding_position.row_start {
                (
                    GridPosition {
                        row_start: old_position.row_start + colliding_item.span.row_span,
                        col_start: moved_item.grid_pos.col_start,
                    },
                    GridPosition {
                        row_start: old_position.row_start,
                        col_start: colliding_position.col_start,
                    },
                )
            } else if old_position.row_start > colliding_position.row_start {
                (
                    GridPosition {
                        row_start: colliding_position.row_start,
                        col_start: moved_item.grid_pos.col_start,
                    },
                    GridPosition {
                        row_start: colliding_position.row_start + moved_item.span.row_span,
                        col_start: colliding_position.col_start,
                    },
                )
            } else {
                return None;
            };

        let mut moved_swap_item = *moved_item;
        moved_swap_item.grid_pos = moved_swap_position;
        colliding_item.grid_pos = colliding_swap_position;

        self.ensure_rows(
            (moved_swap_item.grid_pos.row_start + moved_swap_item.span.row_span)
                .max(colliding_item.grid_pos.row_start + colliding_item.span.row_span),
        );

        if item_aabb::items_overlap(&moved_swap_item, &colliding_item) {
            return None;
        }

        if !collision_grid::item_fits_ignoring(
            &self.collision_grid,
            &moved_swap_item,
            &[moved_item.id, colliding_id],
        ) || !collision_grid::item_fits_ignoring(
            &self.collision_grid,
            &colliding_item,
            &[moved_item.id, colliding_id],
        ) {
            return None;
        }

        collision_grid::clear_item(
            &mut self.collision_grid,
            &colliding_item_signal.get_untracked(),
        );
        colliding_item.grid_to_pixels(self.cell_size, Axes::XY);
        colliding_item_signal.set(colliding_item);
        collision_grid::set_item(&mut self.collision_grid, &colliding_item);

        Some(moved_swap_position)
    }

    fn insertion_row_for_collision(
        &self,
        moved_item: &GridItemData,
        colliding_ids: &[u32],
        drag_px_pos: Position,
    ) -> Option<usize> {
        let moved_top = drag_px_pos.y / self.cell_size.height;

        colliding_ids
            .iter()
            .filter_map(|id| self.items.get(id).map(|item| item.get_untracked()))
            .filter(|item| {
                Self::col_ranges_overlap(
                    moved_item.grid_pos.col_start,
                    moved_item.span.col_span,
                    item.grid_pos.col_start,
                    item.span.col_span,
                )
            })
            .min_by_key(|item| (item.grid_pos.row_start, item.grid_pos.col_start, item.id))
            .map(|item| {
                let item_mid = item.grid_pos.row_start as f64 + item.span.row_span as f64 / 2.0;

                if moved_top >= item_mid {
                    item.grid_pos.row_start + item.span.row_span
                } else {
                    item.grid_pos.row_start
                }
            })
    }

    fn compact_items_up_in_columns(
        &mut self,
        col_start: usize,
        col_span: usize,
        excluded_id: Option<u32>,
    ) {
        let item_ids = self.collect_item_ids_in_columns_from_row(0, col_start, col_span, u32::MAX);
        let mut items = item_ids
            .into_iter()
            .filter(|item_id| Some(*item_id) != excluded_id)
            .filter_map(|item_id| {
                self.items
                    .get(&item_id)
                    .map(|item_signal| (item_id, *item_signal, item_signal.get_untracked()))
            })
            .collect::<Vec<_>>();

        items.sort_by_key(|(_, _, item)| {
            (item.grid_pos.row_start, item.grid_pos.col_start, item.id)
        });

        for (_, item_signal, mut item) in items {
            collision_grid::clear_item(&mut self.collision_grid, &item);

            while item.grid_pos.row_start > 0 {
                let mut candidate = item;
                candidate.grid_pos.row_start -= 1;

                if !collision_grid::item_fits_ignoring(&self.collision_grid, &candidate, &[item.id])
                {
                    break;
                }

                item = candidate;
            }

            item.grid_to_pixels(self.cell_size, Axes::XY);
            item_signal.set(item);
            collision_grid::set_item(&mut self.collision_grid, &item);
        }
    }

    fn collect_item_ids_in_columns_from_row(
        &self,
        row_start: usize,
        col_start: usize,
        col_span: usize,
        excluded_id: u32,
    ) -> Vec<u32> {
        let mut ids = HashSet::new();
        let mut ranges_to_scan = vec![(row_start, col_start, col_span)];

        while let Some((row_start, col_start, col_span)) = ranges_to_scan.pop() {
            let col_end = (col_start + col_span).min(self.columns);

            for row_idx in row_start..self.collision_grid.nrows() {
                for col_idx in col_start..col_end {
                    let Some(item_id) = self.collision_grid[[row_idx, col_idx]] else {
                        continue;
                    };

                    if item_id == excluded_id || !ids.insert(item_id) {
                        continue;
                    }

                    if let Some(item_signal) = self.items.get(&item_id) {
                        let item = item_signal.get_untracked();
                        ranges_to_scan.push((
                            item.grid_pos.row_start,
                            item.grid_pos.col_start,
                            item.span.col_span,
                        ));
                    }
                }
            }
        }

        let mut ids = ids.into_iter().collect::<Vec<_>>();
        ids.sort_by_key(|id| {
            self.items
                .get(id)
                .map(|item| {
                    let item = item.get_untracked();
                    (item.grid_pos.row_start, item.grid_pos.col_start, item.id)
                })
                .unwrap_or((usize::MAX, usize::MAX, *id))
        });
        ids
    }

    fn shift_items_down(&mut self, item_ids: Vec<u32>, by_rows: usize) {
        if by_rows == 0 || item_ids.is_empty() {
            return;
        }

        let items_to_shift = item_ids
            .into_iter()
            .filter_map(|item_id| {
                self.items
                    .get(&item_id)
                    .map(|item_signal| (item_id, *item_signal, item_signal.get_untracked()))
            })
            .collect::<Vec<_>>();

        for (_, _, item) in &items_to_shift {
            collision_grid::clear_item(&mut self.collision_grid, item);
        }

        for (_, item_signal, mut item) in items_to_shift {
            item.grid_pos.row_start += by_rows;
            item.grid_to_pixels(self.cell_size, Axes::XY);
            self.ensure_rows(item.grid_pos.row_start + item.span.row_span);
            item_signal.set(item);
            collision_grid::set_item(&mut self.collision_grid, &item);
        }
    }

    fn set_item_after_local_push(
        &mut self,
        item: &GridItemData,
        mut row_start: usize,
        mut by_rows: usize,
    ) {
        self.ensure_rows(item.grid_pos.row_start + item.span.row_span);

        while !collision_grid::item_ids_for_item(&self.collision_grid, item).is_empty() {
            let item_ids = self.collect_item_ids_in_columns_from_row(
                row_start,
                item.grid_pos.col_start,
                item.span.col_span,
                item.id,
            );
            if item_ids.is_empty() || by_rows == 0 {
                break;
            }
            self.shift_items_down(item_ids, by_rows);

            row_start = item.grid_pos.row_start;
            by_rows = item.span.row_span;
        }

        collision_grid::set_item(&mut self.collision_grid, item);
    }

    /// Check for collisions with other items at a given position and span
    /// Returns a Vec of item IDs that would collide
    pub fn check_collision(&self, item: &GridItemData) -> Vec<u32> {
        collision_grid::item_ids_for_item(&self.collision_grid, item)
    }

    /// Ensure the grid has enough rows to accommodate all items
    /// Adds empty rows at the bottom if any item exceeds current grid bounds
    fn ensure_grid_capacity(&mut self) {
        // Find the maximum row end among all items
        let required_rows = self
            .items
            .values()
            .map(|item_signal| {
                let item = item_signal.get_untracked();
                item.grid_pos.row_start + item.span.row_span
            })
            .max()
            .unwrap_or(0);

        self.ensure_rows(required_rows);
    }

    /// Register an item at its specified position (for declarative items from JSX)
    /// Does not push other items or modify position
    /// If item is already registered, updates its position in the collision grid
    pub fn register_item(&mut self, item: RwSignal<GridItemData>) {
        let untracked_item = item.get_untracked();

        // Assert that col_span doesn't exceed grid columns
        assert!(
            untracked_item.span.col_span <= self.columns,
            "Item col_span ({}) exceeds grid columns ({})",
            untracked_item.span.col_span,
            self.columns
        );

        // If item is already registered, clear its old position from collision grid
        if let Some(old_item_signal) = self.items.get(&untracked_item.id) {
            let old_item = old_item_signal.get_untracked();
            collision_grid::clear_item(&mut self.collision_grid, &old_item);
        }

        // Ensure grid has enough rows for this item
        let item_end_row = untracked_item.grid_pos.row_start + untracked_item.span.row_span;
        self.ensure_rows(item_end_row);

        // Set item in collision grid at new position
        collision_grid::set_item(&mut self.collision_grid, &untracked_item);

        // Add/update item in the items HashMap
        self.items.insert(untracked_item.id, item);

        self.compact_items_up_in_columns(
            untracked_item.grid_pos.col_start,
            untracked_item.span.col_span,
            None,
        );
    }

    /// Add an item at the top-left, pushing all existing items down (for dynamic "Add Item" button)
    pub fn add_item_at_top(&mut self, item: RwSignal<GridItemData>) {
        let mut untracked_item = item.get_untracked();

        // Check if item is already in the layout - if so, don't add again
        if self.items.contains_key(&untracked_item.id) {
            return;
        }

        // Assert that col_span doesn't exceed grid columns
        assert!(
            untracked_item.span.col_span <= self.columns,
            "Item col_span ({}) exceeds grid columns ({})",
            untracked_item.span.col_span,
            self.columns
        );

        // Force position to top-left.
        untracked_item.grid_pos = GridPosition {
            row_start: 0,
            col_start: 0,
        };

        // Update the item signal with the new position
        item.set(untracked_item);

        // TODO: Make this more efficient, we should only retrieve the items that are just below our starting point
        // and our col_span
        // Get all existing items to push down
        let items_to_push: Vec<RwSignal<GridItemData>> = self.items.values().copied().collect();

        leptos::leptos_dom::log!("Items to push: {:?}", items_to_push.len());

        // Push all existing items down by row_span rows (starting from row 0)
        if !items_to_push.is_empty() {
            self.push_items_down(
                &items_to_push,
                0,
                untracked_item.span.row_span,
                (0, untracked_item.span.col_span),
            );
            // Ensure grid has enough rows after pushing items down
            self.ensure_grid_capacity();
        }

        // Create new rows at the top for the new item
        let mut new_rows = Vec::new();
        for _ in 0..untracked_item.span.row_span {
            let mut row = vec![None; self.columns];
            // Fill cells based on col_span (starting from col 0, 0-indexed)
            for cell in row
                .iter_mut()
                .take(untracked_item.span.col_span.min(self.columns))
            {
                *cell = Some(untracked_item.id);
            }
            new_rows.push(row);
        }

        // Convert to Array2
        let new_rows_array = Array2::from_shape_vec(
            (untracked_item.span.row_span, self.columns),
            new_rows.into_iter().flatten().collect(),
        )
        .expect("Failed to create Array2 for new rows");

        // Concatenate new rows at the top with existing grid (pushes everything down)
        self.collision_grid = concatenate(
            Axis(0),
            &[new_rows_array.view(), self.collision_grid.view()],
        )
        .expect("Failed to concatenate collision grid");

        // Update rows count
        self.rows += untracked_item.span.row_span;

        // Add item to the items HashMap
        self.items.insert(untracked_item.id, item);
    }

    /// Move an item to a new position, swapping when possible or pushing colliding items down.
    ///
    /// # Arguments
    /// * `item` - The item to move
    /// * `new_row_start` - New row position (0-indexed)
    /// * `new_col_start` - New column position (0-indexed)
    pub fn move_item_with_collision(
        &mut self,
        item: RwSignal<GridItemData>,
        new_row_start: usize,
        new_col_start: usize,
        drag_px_pos: Position,
    ) {
        let mut untracked_item = item.get_untracked();
        let old_position = untracked_item.grid_pos;
        let new_position =
            self.clamp_position_for_item(&untracked_item, new_row_start, new_col_start);

        // If the snapped grid position did not change, still restore the pixel
        // position because dragging may have written a raw sub-cell offset.
        if old_position.row_start == new_position.row_start
            && old_position.col_start == new_position.col_start
        {
            untracked_item.grid_to_pixels(self.cell_size, Axes::XY);
            item.set(untracked_item);
            return;
        }

        // Clear item from old position in collision grid
        collision_grid::clear_item(&mut self.collision_grid, &untracked_item);

        // Update item's position
        untracked_item.grid_pos = new_position;
        untracked_item.grid_to_pixels(self.cell_size, Axes::XY);

        // Ensure grid has enough capacity for the new position
        self.ensure_rows(untracked_item.grid_pos.row_start + untracked_item.span.row_span);

        let colliding_ids =
            collision_grid::item_ids_for_item(&self.collision_grid, &untracked_item);

        let mut placed_in_grid = false;
        if !colliding_ids.is_empty() {
            let moved_aabb = item_aabb::from_drag(&untracked_item, drag_px_pos, self.cell_size);
            let collision_candidates = collision_grid::item_ids_for_aabb(
                &self.collision_grid,
                moved_aabb,
                untracked_item.id,
            )
            .into_iter()
            .filter_map(|item_id| {
                self.items
                    .get(&item_id)
                    .map(|item_signal| item_signal.get_untracked())
            });
            let fine_collisions =
                item_aabb::drop_collisions(&untracked_item, moved_aabb, collision_candidates);
            let Some(drop_collision) = item_aabb::dominant_drop_collision(&fine_collisions) else {
                untracked_item.grid_pos = old_position;
                self.restore_item_position(item, untracked_item);
                return;
            };

            if let Some(swap_position) =
                self.try_swap_items(&untracked_item, old_position, &[drop_collision.item_id])
            {
                untracked_item.grid_pos = swap_position;
                untracked_item.grid_to_pixels(self.cell_size, Axes::XY);
                collision_grid::set_item(&mut self.collision_grid, &untracked_item);
                placed_in_grid = true;
            } else {
                if old_position.col_start == untracked_item.grid_pos.col_start {
                    untracked_item.grid_pos = old_position;
                    self.restore_item_position(item, untracked_item);
                    return;
                }

                if let Some(row_start) = self.insertion_row_for_collision(
                    &untracked_item,
                    &[drop_collision.item_id],
                    drag_px_pos,
                ) {
                    untracked_item.grid_pos.row_start = row_start;
                    untracked_item.grid_to_pixels(self.cell_size, Axes::XY);
                    self.ensure_rows(
                        untracked_item.grid_pos.row_start + untracked_item.span.row_span,
                    );
                }

                self.set_item_after_local_push(
                    &untracked_item,
                    untracked_item.grid_pos.row_start,
                    untracked_item.span.row_span,
                );
                placed_in_grid = true;
            }
        }

        // Update the moved item's signal
        item.set(untracked_item);
        if !placed_in_grid {
            self.set_item_after_local_push(
                &untracked_item,
                untracked_item.grid_pos.row_start,
                untracked_item.span.row_span,
            );
        }
        self.compact_items_up_in_columns(
            old_position.col_start,
            untracked_item.span.col_span,
            None,
        );
        self.compact_items_up_in_columns(
            untracked_item.grid_pos.col_start,
            untracked_item.span.col_span,
            None,
        );
    }

    pub fn resize_item_with_collision(
        &mut self,
        item: RwSignal<GridItemData>,
        col_span: usize,
        row_span: usize,
    ) {
        let mut untracked_item = item.get_untracked();
        let col_span = col_span.max(1).min(self.columns);
        let row_span = row_span.max(1);
        let old_item = untracked_item;

        untracked_item.span.col_span = col_span;
        untracked_item.span.row_span = row_span;
        untracked_item.grid_pos = self.clamp_position_for_item(
            &untracked_item,
            untracked_item.grid_pos.row_start,
            untracked_item.grid_pos.col_start,
        );
        untracked_item.size = Size {
            width: col_span as f64 * self.cell_size.width,
            height: row_span as f64 * self.cell_size.height,
        };

        collision_grid::clear_item(&mut self.collision_grid, &old_item);
        self.ensure_rows(untracked_item.grid_pos.row_start + untracked_item.span.row_span);

        let colliding_ids =
            collision_grid::item_ids_for_item(&self.collision_grid, &untracked_item);
        let mut placed_in_grid = false;
        if !colliding_ids.is_empty() {
            let old_row_end = old_item.grid_pos.row_start + old_item.span.row_span;
            let new_row_end = untracked_item.grid_pos.row_start + untracked_item.span.row_span;
            let has_side_collision = colliding_ids.iter().any(|colliding_id| {
                self.items
                    .get(colliding_id)
                    .map(|item| item.get_untracked().grid_pos.row_start < old_row_end)
                    .unwrap_or(false)
            });
            let (row_start, by_rows) = if has_side_collision {
                (
                    untracked_item.grid_pos.row_start,
                    untracked_item.span.row_span,
                )
            } else {
                (old_row_end, new_row_end.saturating_sub(old_row_end))
            };
            self.set_item_after_local_push(&untracked_item, row_start, by_rows);
            placed_in_grid = true;
        }

        item.set(untracked_item);
        if !placed_in_grid {
            self.set_item_after_local_push(
                &untracked_item,
                untracked_item.grid_pos.row_start,
                untracked_item.span.row_span,
            );
        }
        self.compact_items_up_in_columns(old_item.grid_pos.col_start, old_item.span.col_span, None);
        self.compact_items_up_in_columns(
            untracked_item.grid_pos.col_start,
            untracked_item.span.col_span,
            None,
        );
    }

    /// Push items down by a certain number of rows
    ///
    /// # Arguments
    /// * `items` - The items to push down
    /// * `row_start` - The row from which to start pushing
    /// * `by_rows` - The number of rows to push down by
    /// * `(start_col, end_col)` - The columns affecting the items to push
    fn push_items_down(
        &mut self,
        items: &[RwSignal<GridItemData>],
        row_start: usize,
        by_rows: usize,
        (start_col, end_col): (usize, usize),
    ) {
        for item_signal in items {
            item_signal.update(|item| {
                log!("Pushing from row {row_start} item down: {item:?}");
                let curr_row = item.grid_pos.row_start;

                // TODO: this could be computed as a util function
                let curr_col_end = item.grid_pos.col_start + item.span.col_span;
                let curr_col_start = item.grid_pos.col_start;
                if (curr_col_start >= start_col && curr_col_start < end_col)
                    || (curr_col_end > start_col && curr_col_end <= end_col)
                {
                    log!(
                        "({}: {curr_col_start} >= {start_col} && {curr_col_start} < {end_col})
                        || ({curr_col_end} > {start_col} && {curr_col_end} <= {end_col})",
                        item.id
                    );
                    if curr_row >= row_start {
                        item.grid_pos.row_start += by_rows;
                        item.grid_to_pixels(self.cell_size, Axes::Y);
                    }
                }
            });
        }

        // For add_item, the collision_grid update happens via row concatenation.
    }

    pub fn remove_item(&mut self, item: RwSignal<GridItemData>) {
        let untracked_item = item.get_untracked();

        // Clear the item from the collision grid
        collision_grid::clear_item(&mut self.collision_grid, &untracked_item);

        // Remove from items HashMap
        self.items.remove(&untracked_item.id);
    }

    pub fn sync_items_to_grid(&mut self) {
        self.collision_grid = Array2::from_elem((self.rows, self.columns), None::<u32>);
        let mut items = self
            .items
            .values()
            .map(|item_signal| {
                let item = item_signal.get_untracked();
                (
                    item.grid_pos.row_start,
                    item.grid_pos.col_start,
                    item.id,
                    *item_signal,
                )
            })
            .collect::<Vec<_>>();

        items.sort_by_key(|(row_start, col_start, id, _)| (*row_start, *col_start, *id));

        for (_, _, _, item_signal) in items {
            let mut item = item_signal.get_untracked();
            item.grid_pos = self.clamp_position_for_item(
                &item,
                item.grid_pos.row_start,
                item.grid_pos.col_start,
            );

            while !collision_grid::item_ids_for_item(&self.collision_grid, &item).is_empty() {
                item.grid_pos.row_start += 1;
                self.ensure_rows(item.grid_pos.row_start + item.span.row_span);
            }

            item.grid_to_pixels(self.cell_size, Axes::XY);
            item_signal.set(item);
            self.ensure_rows(item.grid_pos.row_start + item.span.row_span);
            collision_grid::set_item(&mut self.collision_grid, &item);
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LayoutBuilder {
    pub size: Size,
    pub collision_grid: Array2<Option<u32>>,
    pub items: HashMap<u32, RwSignal<GridItemData>>,
    pub columns: usize,
    pub rows: usize,
    pub cell_size: Size,
}

impl LayoutBuilder {
    pub fn rows(mut self, quantity: usize) -> Self {
        self.rows = quantity;
        self
    }

    pub fn columns(mut self, quantity: usize) -> Self {
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

    pub fn build(self) -> Layout {
        let collision_grid = Array2::from_elem((self.rows, self.columns), None::<u32>);

        Layout {
            size: self.size,
            collision_grid,
            items: self.items,
            rows: self.rows,
            columns: self.columns,
            cell_size: self.cell_size,
        }
    }
}
