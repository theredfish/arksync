use crate::components::grid::core::collision::{aabb::Aabb, item_aabb};
use crate::components::grid::core::item::GridItemData;
use crate::components::grid::core::item::GridPosition;

/// Minimum overlap needed on each axis before a dragged item can affect another
/// item. Small edge contacts are ignored so they do not accidentally push or
/// swap panels.
const MIN_DROP_AXIS_OVERLAP_RATIO: f64 = 0.35;

/// Ratio required for the strongest collision to be treated as the only useful
/// target. Ambiguous overlaps restore the item instead of guessing.
const DOMINANT_DROP_OVERLAP_RATIO: f64 = 1.25;

/// Fine collision result between the dragged item and one candidate item.
///
/// The collision grid first limits candidates by occupied cells. This type
/// stores the AABB overlap details used to decide whether one candidate is
/// strong enough to drive a drop placement.
#[derive(Clone, Copy, Debug)]
pub struct DropCollision {
    /// Candidate item touched by the dragged item.
    pub item_id: u32,
    /// AABB overlap area in grid-cell units.
    overlap_area: f64,
    /// Horizontal overlap normalized by the smaller item width.
    horizontal_overlap_ratio: f64,
    /// Vertical overlap normalized by the smaller item height.
    vertical_overlap_ratio: f64,
}

impl DropCollision {
    /// Returns true when the overlap is intentional enough to affect layout.
    fn is_actionable(self) -> bool {
        self.horizontal_overlap_ratio >= MIN_DROP_AXIS_OVERLAP_RATIO
            && self.vertical_overlap_ratio >= MIN_DROP_AXIS_OVERLAP_RATIO
    }
}

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

/// Builds sorted, actionable drop collisions from already filtered candidates.
///
/// Candidates should come from the collision grid, which keeps this pass small.
/// AABB is only used here to refine those candidates and rank the dominant
/// interaction.
pub fn drop_collisions(
    moved_item: &GridItemData,
    moved_aabb: Aabb,
    candidates: impl IntoIterator<Item = GridItemData>,
) -> Vec<DropCollision> {
    let mut collisions = candidates
        .into_iter()
        .filter_map(|item| {
            let (overlap_width, overlap_height) = item_aabb::overlap_item(moved_aabb, &item)?;

            Some(DropCollision {
                item_id: item.id,
                overlap_area: overlap_width * overlap_height,
                horizontal_overlap_ratio: overlap_width
                    / (moved_item.span.col_span.min(item.span.col_span) as f64),
                vertical_overlap_ratio: overlap_height
                    / (moved_item.span.row_span.min(item.span.row_span) as f64),
            })
        })
        .filter(|collision| collision.is_actionable())
        .collect::<Vec<_>>();

    collisions.sort_by(|a, b| {
        b.overlap_area
            .partial_cmp(&a.overlap_area)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.item_id.cmp(&b.item_id))
    });
    collisions
}

/// Selects the strongest collision only when it clearly dominates the next one.
///
/// This prevents diagonal or broad overlaps from choosing an arbitrary target
/// when the dragged item is interacting with multiple panels at similar depth.
pub fn dominant_drop_collision(collisions: &[DropCollision]) -> Option<DropCollision> {
    let first = *collisions.first()?;
    let Some(second) = collisions.get(1) else {
        return Some(first);
    };

    if first.overlap_area >= second.overlap_area * DOMINANT_DROP_OVERLAP_RATIO {
        Some(first)
    } else {
        None
    }
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
