use crate::components::grid::GridItemData;

trait Aabb {
    fn collides_with(&self, other: &Self) -> bool;
}

impl Aabb for GridItemData {
    fn collides_with(&self, other: &Self) -> bool {
        !(self.max_x() <= other.min_x()
            || self.min_x() >= other.max_x()
            || self.max_y() <= other.min_y()
            || self.min_y() >= other.max_y())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::grid::*;

    /// Test AABB collision detection on the **x-axis**.
    ///
    /// # Starting layout
    /// ```
    ///          (1,1) -> (2,1)
    /// ================|===============
    ///     item1       | item2
    /// ================|===============
    /// ```
    /// # Item1 collides in item 2
    ///
    /// ```
    ///           (1,1) -> (2,1) item1
    /// ================|===============
    ///                 | item1, item2
    /// ================|===============
    ///```
    ///
    /// # Item2 collides in item 1
    ///
    /// ```
    ///          (1,1) <- (2,1)
    /// ================|===============
    ///    item1, item2 |
    /// ================|===============
    /// ```
    ///
    #[test]
    fn test_aabb_collision_on_x_axis() {
        let size = Size {
            width: 100.0,
            height: 100.0,
        };
        let span = Span {
            col_span: 1,
            row_span: 1,
        };

        let mut item1 = GridItemData {
            position: GridItemPosition {
                col_start: 1,
                row_start: 1,
            },
            span,
            size,
        };

        let mut item2 = GridItemData {
            position: GridItemPosition {
                col_start: 2,
                row_start: 1,
            },
            span,
            size,
        };

        assert!(
            !item1.collides_with(&item2),
            "Item1 should not collide with Item2"
        );
        assert!(
            !item2.collides_with(&item1),
            "Item2 should not collide with Item1"
        );

        item2.position.col_start = 1;

        assert!(
            item1.collides_with(&item2),
            "Item1 should collide with Item2"
        );
        assert!(
            item2.collides_with(&item1),
            "Item2 should collide with Item1"
        );

        item2.position.col_start = 2;
        item1.position.col_start = 2;

        assert!(
            item1.collides_with(&item2),
            "Item1 should collide with Item2"
        );
        assert!(
            item2.collides_with(&item1),
            "Item2 should collide with Item1"
        );
    }

    /// Test AABB collision detection on the **y-axis**.
    ///
    /// # Starting layout
    /// ```
    ///       (1,1)
    /// ================
    ///     item1
    /// ================
    ///       (1,2)
    /// ================
    ///     item2
    /// ================
    /// ```
    /// # Item1 collides in item 2
    /// ```
    ///       (1,2)
    /// ================
    ///     item1, item2
    /// ================
    /// ```
    /// # Item2 collides in item 1
    /// ```
    ///       (1,1)
    /// ================
    ///     item1, item2
    /// ================
    /// ```
    #[test]
    fn test_aabb_collision_on_y_axis() {
        let size = Size {
            width: 100.0,
            height: 100.0,
        };
        let span = Span {
            col_span: 1,
            row_span: 1,
        };

        let mut item1 = GridItemData {
            position: GridItemPosition {
                col_start: 1,
                row_start: 1,
            },
            span,
            size,
        };

        let mut item2 = GridItemData {
            position: GridItemPosition {
                col_start: 1,
                row_start: 2,
            },
            span,
            size,
        };

        assert!(
            !item1.collides_with(&item2),
            "Item1 should not collide with Item2"
        );
        assert!(
            !item2.collides_with(&item1),
            "Item2 should not collide with Item1"
        );

        item2.position.row_start = 1;

        assert!(
            item1.collides_with(&item2),
            "Item1 should collide with Item2"
        );
        assert!(
            item2.collides_with(&item1),
            "Item2 should collide with Item1"
        );

        item2.position.row_start = 2;
        item1.position.row_start = 2;

        assert!(
            item1.collides_with(&item2),
            "Item1 should collide with Item2"
        );
        assert!(
            item2.collides_with(&item1),
            "Item2 should collide with Item1"
        );
    }
}
