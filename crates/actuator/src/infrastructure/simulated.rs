// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::relay::{RelayResult, RelaySpec, RelayState};

pub struct SimulatedRelayDriver {
    spec: RelaySpec,
}

impl SimulatedRelayDriver {
    pub fn new(spec: RelaySpec) -> RelayResult<Self> {
        log::warn!(
            "GPIO debug loop is running without Raspberry Pi GPIO access; state changes will only be logged."
        );
        Ok(Self { spec })
    }

    pub fn apply(&self, state: RelayState) -> RelayResult<()> {
        log::debug!(
            "Simulated relay '{}' on GPIO{} -> {} ({})",
            state.id,
            self.spec.gpio_bcm_pin,
            if state.active { "ON" } else { "OFF" },
            state.level
        );

        Ok(())
    }
}
