// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![no_std]

extern crate alloc;

#[cfg(all(feature = "linux-gpio", feature = "esp-gpio"))]
compile_error!("features `linux-gpio` and `esp-gpio` cannot be enabled at the same time");

#[cfg(feature = "linux-gpio")]
extern crate std;

pub mod infrastructure;
pub mod relay;
pub mod rule_engine;
pub mod services;
