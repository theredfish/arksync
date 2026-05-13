// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[derive(Clone, Copy)]
pub struct ConfigHandler<T> {
    load: fn() -> T,
}

impl<T> ConfigHandler<T> {
    pub const fn new(load: fn() -> T) -> Self {
        Self { load }
    }

    pub fn load(&self) -> T {
        (self.load)()
    }
}
