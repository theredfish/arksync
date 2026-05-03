// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
