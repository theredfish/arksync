// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::fmt;
use serde::Serialize;

#[cfg(feature = "esp-gpio")]
pub use crate::infrastructure::esp_gpio::EspGpioRelayDriver as RelayDriver;
#[cfg(feature = "linux-gpio")]
pub use crate::infrastructure::linux_gpio::LinuxGpioRelayDriver as RelayDriver;
#[cfg(not(any(feature = "esp-gpio", feature = "linux-gpio")))]
pub use crate::infrastructure::simulated::SimulatedRelayDriver as RelayDriver;

pub const MIST_RELAY: RelaySpec = RelaySpec {
    id: "mist_relay",
    model: "5V dual-channel relay module",
    channels: 2,
    coil_voltage_vdc: 5.0,
    contact_current_a: 10.0,
    contact_type: "2NO 2NC",
    gpio_bcm_pin: 17,
    active_low: true,
};

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelaySpec {
    pub id: &'static str,
    pub model: &'static str,
    pub channels: u8,
    pub coil_voltage_vdc: f32,
    pub contact_current_a: f32,
    pub contact_type: &'static str,
    pub gpio_bcm_pin: u8,
    pub active_low: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayState {
    pub id: &'static str,
    pub gpio_bcm_pin: u8,
    pub active_low: bool,
    pub active: bool,
    pub level: &'static str,
}

impl RelayState {
    pub fn new(spec: RelaySpec, active: bool) -> Self {
        Self {
            id: spec.id,
            gpio_bcm_pin: spec.gpio_bcm_pin,
            active_low: spec.active_low,
            active,
            level: match (spec.active_low, active) {
                (true, true) | (false, false) => "low",
                (true, false) | (false, true) => "high",
            },
        }
    }
}

pub type RelayResult<T> = Result<T, RelayError>;

#[derive(Debug)]
pub enum RelayError {
    BackendUnavailable,
    GpioRequest { gpio_bcm_pin: u8 },
    GpioSet { gpio_bcm_pin: u8 },
    GpioLockPoisoned,
}

impl fmt::Display for RelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BackendUnavailable => f.write_str("relay backend is not available"),
            Self::GpioRequest { gpio_bcm_pin } => {
                write!(
                    f,
                    "failed to request GPIO{gpio_bcm_pin} from /dev/gpiochip0"
                )
            }
            Self::GpioSet { gpio_bcm_pin } => {
                write!(f, "failed to set GPIO{gpio_bcm_pin}")
            }
            Self::GpioLockPoisoned => f.write_str("GPIO lock is poisoned"),
        }
    }
}
