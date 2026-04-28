#[derive(Clone, Copy, Debug)]
pub struct Span {
    pub col_span: usize,
    pub row_span: usize,
}

impl Default for Span {
    fn default() -> Self {
        Self {
            col_span: 1,
            row_span: 1,
        }
    }
}
