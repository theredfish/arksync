// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Size in pixels
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}
