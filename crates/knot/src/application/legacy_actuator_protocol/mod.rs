// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Temporary local actuator contract retained until actuator wire migration.

mod legacy_actuator_config;
mod legacy_knot_actuator_message;

pub use legacy_actuator_config::*;
pub use legacy_knot_actuator_message::*;
