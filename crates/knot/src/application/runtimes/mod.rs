// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod no_std;
#[cfg(any(
    feature = "tokio-knot-actuator-service",
    feature = "tokio-knot-runtime",
    feature = "tokio-knot-sensor-service"
))]
pub mod tokio;

pub use no_std::*;
#[cfg(any(
    feature = "tokio-knot-actuator-service",
    feature = "tokio-knot-runtime",
    feature = "tokio-knot-sensor-service"
))]
pub use tokio::*;
