use crate::components::grid::core::item::GridPosition;

/// Decision returned after a drag drop collides with an existing grid item.
///
/// This enum only describes the placement strategy. `Layout` remains
/// responsible for mutating item signals and the collision grid.
#[derive(Clone, Copy, Debug)]
pub enum DropPlacement {
    /// Reject the drop and restore the moved item to its previous grid position.
    Restore,
    /// Place the moved item at its computed swap position and move the target
    /// item into the space opened by the drag.
    Swap {
        /// The item that was selected as the unique swap target.
        target_id: u32,
        /// Final grid position for the moved item after the swap succeeds.
        moved_position: GridPosition,
    },
    /// Insert the moved item near the collided target and push the local
    /// collision chain down until the moved item fits.
    InsertWithPush {
        /// The item that was selected as the insertion anchor.
        target_id: u32,
        /// Optional row chosen from the target midpoint. When absent, the
        /// caller keeps the moved item's snapped row.
        row_start: Option<usize>,
    },
}

/// Resolves the high-level placement strategy for a colliding drag drop.
///
/// The collision adapter chooses the dominant target, `Layout` tests whether a
/// strict vertical swap is possible, and this function decides whether the drop
/// should restore, swap, or insert with a local push.
pub fn resolve_collision_drop(
    old_position: GridPosition,
    moved_position: GridPosition,
    target_id: Option<u32>,
    swap_position: Option<GridPosition>,
    insertion_row: Option<usize>,
) -> DropPlacement {
    let Some(target_id) = target_id else {
        return DropPlacement::Restore;
    };

    if let Some(moved_position) = swap_position {
        return DropPlacement::Swap {
            target_id,
            moved_position,
        };
    }

    // Downward moves inside the same column must not push the target by default.
    // Without a valid swap, they behave like Grafana-style rejected drops.
    let same_column_drop = old_position.col_start == moved_position.col_start;
    let moving_up = moved_position.row_start < old_position.row_start;

    if same_column_drop && !moving_up {
        DropPlacement::Restore
    } else {
        DropPlacement::InsertWithPush {
            target_id,
            row_start: insertion_row,
        }
    }
}
