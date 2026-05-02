#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

impl Aabb {
    pub fn new(left: f64, top: f64, width: f64, height: f64) -> Self {
        Self {
            left,
            right: left + width,
            top,
            bottom: top + height,
        }
    }

    pub fn overlap(&self, other: &Self) -> Option<(f64, f64)> {
        let width = self.right.min(other.right) - self.left.max(other.left);
        let height = self.bottom.min(other.bottom) - self.top.max(other.top);

        if width <= 0.0 || height <= 0.0 {
            return None;
        }

        Some((width, height))
    }
}
