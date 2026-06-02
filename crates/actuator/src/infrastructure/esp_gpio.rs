// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::relay::{RelayError, RelayResult, RelaySpec, RelayState};

pub struct EspGpioRelayDriver;

impl EspGpioRelayDriver {
    pub fn new(_spec: RelaySpec) -> RelayResult<Self> {
        Err(RelayError::BackendUnavailable)
    }

    pub fn apply(&self, _state: RelayState) -> RelayResult<()> {
        Err(RelayError::BackendUnavailable)
    }
}
