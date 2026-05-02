use crate::components::grid::core::collision::aabb::Aabb;
use crate::components::grid::core::item::GridItemData;
use crate::components::grid::core::size::Size;
use leptos_use::core::Position;

const MIN_DROP_AXIS_OVERLAP_RATIO: f64 = 0.35;
const DOMINANT_DROP_OVERLAP_RATIO: f64 = 1.25;

#[derive(Clone, Copy, Debug)]
pub struct DropCollision {
    pub item_id: u32,
    overlap_area: f64,
    horizontal_overlap_ratio: f64,
    vertical_overlap_ratio: f64,
}

impl DropCollision {
    fn is_actionable(self) -> bool {
        self.horizontal_overlap_ratio >= MIN_DROP_AXIS_OVERLAP_RATIO
            && self.vertical_overlap_ratio >= MIN_DROP_AXIS_OVERLAP_RATIO
    }
}

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

pub fn drop_collisions(
    moved_item: &GridItemData,
    moved_aabb: Aabb,
    candidates: impl IntoIterator<Item = GridItemData>,
) -> Vec<DropCollision> {
    let mut collisions = candidates
        .into_iter()
        .filter_map(|item| {
            let (overlap_width, overlap_height) = overlap_item(moved_aabb, &item)?;

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
