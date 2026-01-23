use crate::components::grid::core::item::Axes;
use crate::components::grid::core::{
    item::{GridItemData, GridPosition},
    size::Size,
};
use leptos::{
    logging::log,
    prelude::{GetUntracked, RwSignal, Set, Update},
};
use ndarray::{concatenate, Array2, Axis};
use std::collections::HashMap;

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
    /// Set an item in the collision grid based on its position and span
    fn set_item_in_grid(&mut self, item: &GridItemData) {
        let row_start = item.grid_pos.row_start;
        let col_start = item.grid_pos.col_start;

        for row_offset in 0..item.span.row_span {
            for col_offset in 0..item.span.col_span {
                let row_idx = row_start + row_offset;
                let col_idx = col_start + col_offset;

                if row_idx < self.collision_grid.nrows() && col_idx < self.collision_grid.ncols() {
                    self.collision_grid[[row_idx, col_idx]] = Some(item.id);
                }
            }
        }
    }

    /// Clear an item from the collision grid based on its position and span
    fn clear_item_from_grid(&mut self, item: &GridItemData) {
        let row_start = item.grid_pos.row_start;
        let col_start = item.grid_pos.col_start;

        for row_offset in 0..item.span.row_span {
            for col_offset in 0..item.span.col_span {
                let row_idx = row_start + row_offset;
                let col_idx = col_start + col_offset;

                if row_idx < self.collision_grid.nrows() && col_idx < self.collision_grid.ncols() {
                    // Only clear if this cell actually contains this item
                    if self.collision_grid[[row_idx, col_idx]] == Some(item.id) {
                        self.collision_grid[[row_idx, col_idx]] = None;
                    }
                }
            }
        }
    }

    /// Update an item's position in the collision grid (clear old, set new)
    ///
    /// Note: Currently unused in add_item (which uses concatenation for efficiency),
    /// but will be essential for drag/resize operations where items move without
    /// adding new rows to the grid.
    fn update_item_in_grid(&mut self, old_item: &GridItemData, new_item: &GridItemData) {
        self.clear_item_from_grid(old_item);
        self.set_item_in_grid(new_item);
    }

    /// Check for collisions with other items at a given position and span
    /// Returns a Vec of item IDs that would collide
    pub fn check_collision(&self, item: &GridItemData) -> Vec<u32> {
        let mut colliding_ids = std::collections::HashSet::new();

        let row_start = item.grid_pos.row_start;
        let col_start = item.grid_pos.col_start;

        for row_offset in 0..item.span.row_span {
            for col_offset in 0..item.span.col_span {
                let row_idx = row_start + row_offset;
                let col_idx = col_start + col_offset;

                if row_idx < self.collision_grid.nrows() && col_idx < self.collision_grid.ncols() {
                    if let Some(occupant_id) = self.collision_grid[[row_idx, col_idx]] {
                        // Don't consider collision with itself
                        if occupant_id != item.id {
                            colliding_ids.insert(occupant_id);
                        }
                    }
                }
            }
        }

        colliding_ids.into_iter().collect()
    }

    /// Ensure the grid has enough rows to accommodate all items
    /// Adds empty rows at the bottom if any item exceeds current grid bounds
    fn ensure_grid_capacity(&mut self) {
        // Find the maximum row end among all items
        let max_row_end = self
            .items
            .values()
            .map(|item_signal| {
                let item = item_signal.get_untracked();
                // Calculate the last row this item occupies (1-indexed)
                item.grid_pos.row_start + item.span.row_span - 1
            })
            .max()
            .unwrap_or(0);

        // If items exceed current grid, add rows at the bottom
        if max_row_end > self.rows {
            let rows_to_add = max_row_end - self.rows;
            let empty_rows = Array2::from_elem((rows_to_add, self.columns), None::<u32>);

            self.collision_grid =
                concatenate(Axis(0), &[self.collision_grid.view(), empty_rows.view()])
                    .expect("Failed to concatenate empty rows at bottom");

            self.rows = max_row_end;
        }
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
            self.clear_item_from_grid(&old_item);
        }

        // Ensure grid has enough rows for this item
        let item_end_row = untracked_item.grid_pos.row_start + untracked_item.span.row_span - 1;
        if item_end_row > self.rows {
            let rows_to_add = item_end_row - self.rows;
            let empty_rows = Array2::from_elem((rows_to_add, self.columns), None::<u32>);

            self.collision_grid =
                concatenate(Axis(0), &[self.collision_grid.view(), empty_rows.view()])
                    .expect("Failed to concatenate empty rows at bottom");

            self.rows = item_end_row;
        }

        // Set item in collision grid at new position
        self.set_item_in_grid(&untracked_item);

        // Add/update item in the items HashMap
        self.items.insert(untracked_item.id, item);
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

        // Force position to top-left (1-indexed: row 1, col 1)
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
                // FIXME: this is just for debugging
                (1, untracked_item.span.col_span),
            );
            // Ensure grid has enough rows after pushing items down
            self.ensure_grid_capacity();
        }

        // Create new rows at the top for the new item
        let mut new_rows = Vec::new();
        for _ in 0..untracked_item.span.row_span {
            let mut row = vec![None; self.columns];
            // Fill cells based on col_span (starting from col 0, 0-indexed)
            for col_idx in 0..untracked_item.span.col_span.min(self.columns) {
                row[col_idx] = Some(untracked_item.id);
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

    /// Move an item to a new position, pushing colliding items down
    ///
    /// # Arguments
    /// * `item` - The item to move
    /// * `new_row_start` - New row position (1-indexed)
    /// * `new_col_start` - New column position (1-indexed)
    pub fn move_item_with_collision(
        &mut self,
        item: RwSignal<GridItemData>,
        new_row_start: usize,
        new_col_start: usize,
    ) {
        let mut untracked_item = item.get_untracked();
        let old_position = untracked_item.grid_pos.clone();

        // If position hasn't changed, nothing to do
        if old_position.row_start == new_row_start && old_position.col_start == new_col_start {
            return;
        }

        // Clear item from old position in collision grid
        self.clear_item_from_grid(&untracked_item);

        // Update item's position
        untracked_item.grid_pos = GridPosition {
            row_start: new_row_start,
            col_start: new_col_start,
        };

        // Check for collisions at new position
        let item_end_row = new_row_start + untracked_item.span.row_span - 1;
        let item_end_col = new_col_start + untracked_item.span.col_span - 1;

        // Ensure grid has enough capacity for the new position
        if item_end_row > self.rows {
            let rows_to_add = item_end_row - self.rows;
            let empty_rows = Array2::from_elem((rows_to_add, self.columns), None::<u32>);
            self.collision_grid =
                concatenate(Axis(0), &[self.collision_grid.view(), empty_rows.view()])
                    .expect("Failed to concatenate empty rows");
            self.rows = item_end_row;
        }

        // Find all items that collide with the new position
        let mut colliding_items = Vec::new();
        for row in (new_row_start - 1)..item_end_row.min(self.rows) {
            for col in (new_col_start - 1)..item_end_col.min(self.columns) {
                if let Some(colliding_id) = self.collision_grid[[row, col]] {
                    // Don't consider the item itself as a collision
                    if colliding_id != untracked_item.id {
                        if let Some(&colliding_item_signal) = self.items.get(&colliding_id) {
                            if !colliding_items
                                .iter()
                                .any(|&item_sig: &RwSignal<GridItemData>| {
                                    item_sig.get_untracked().id == colliding_id
                                })
                            {
                                colliding_items.push(colliding_item_signal);
                            }
                        }
                    }
                }
            }
        }

        // Calculate how far down to push colliding items
        // They need to be pushed to at least below the new item's bottom edge
        if !colliding_items.is_empty() {
            // Push colliding items down to make room
            let push_to_row = item_end_row; // 1-indexed position where colliding items should start

            for colliding_item_signal in &colliding_items {
                let colliding_item = colliding_item_signal.get_untracked();

                // Clear the colliding item from its old position
                self.clear_item_from_grid(&colliding_item);

                // Calculate new position (push below the moved item)
                let colliding_new_row = push_to_row + 1;

                // Update the colliding item's position
                colliding_item_signal.update(|item| {
                    item.grid_pos.row_start = colliding_new_row;
                });
            }

            // Ensure grid has enough capacity after pushing items
            self.ensure_grid_capacity();

            // Re-register all colliding items at their new positions
            for colliding_item_signal in &colliding_items {
                let colliding_item = colliding_item_signal.get_untracked();
                self.set_item_in_grid(&colliding_item);
            }
        }

        // Update the moved item's signal
        item.set(untracked_item);

        // Place the moved item in the collision grid at its new position
        self.set_item_in_grid(&untracked_item);
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
                    // FIXME: there is a bug where curr_col_end isn't updated after drag event.
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

        // Note: For add_item, the collision_grid update happens via concatenation,
        // so we don't manually update it here. For drag/resize events, use
        // update_item_in_grid() or clear_item_from_grid() + set_item_in_grid()
        // to handle collision_grid updates explicitly.
    }

    pub fn remove_item(&mut self, item: RwSignal<GridItemData>) {
        let untracked_item = item.get_untracked();

        // Clear the item from the collision grid
        self.clear_item_from_grid(&untracked_item);

        // Remove from items HashMap
        self.items.remove(&untracked_item.id);
    }

    pub fn update_items_size(&mut self, (w_ratio, h_ratio): (f64, f64)) {
        // we used to update the grid items size here, so we can prepare the
        // collision detection and re-layout the items. But let's skip thos for
        // now since we are not using that feature yet.
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
