// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "esp-gpio")]
pub mod esp_gpio;
#[cfg(feature = "linux-gpio")]
pub mod linux_gpio;
#[cfg(not(any(feature = "esp-gpio", feature = "linux-gpio")))]
pub mod simulated;
