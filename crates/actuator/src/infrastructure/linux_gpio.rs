// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::relay::{RelayError, RelayResult, RelaySpec, RelayState};

pub struct LinuxGpioRelayDriver {
    gpio_line: u32,
    request: std::sync::Mutex<gpiocdev::Request>,
}

impl LinuxGpioRelayDriver {
    pub fn new(spec: RelaySpec) -> RelayResult<Self> {
        use gpiocdev::line::Value;

        let gpio_line = u32::from(spec.gpio_bcm_pin);
        let mut request_builder = gpiocdev::Request::builder();
        request_builder
            .on_chip("/dev/gpiochip0")
            .with_line(gpio_line)
            .as_output(Value::Inactive);

        if spec.active_low {
            request_builder.as_active_low();
        }

        let request = request_builder.request().map_err(|source| {
            log::error!(
                "Failed to request GPIO{} from /dev/gpiochip0: {source:?}",
                spec.gpio_bcm_pin
            );
            RelayError::GpioRequest {
                gpio_bcm_pin: spec.gpio_bcm_pin,
            }
        })?;

        Ok(Self {
            gpio_line,
            request: std::sync::Mutex::new(request),
        })
    }

    pub fn apply(&self, state: RelayState) -> RelayResult<()> {
        use gpiocdev::line::Value;

        let request = self
            .request
            .lock()
            .map_err(|_| RelayError::GpioLockPoisoned)?;
        let value = if state.active {
            Value::Active
        } else {
            Value::Inactive
        };

        request.set_value(self.gpio_line, value).map_err(|source| {
            log::error!(
                "Failed to set GPIO{} to {value}: {source:?}",
                state.gpio_bcm_pin
            );
            RelayError::GpioSet {
                gpio_bcm_pin: state.gpio_bcm_pin,
            }
        })?;

        Ok(())
    }
}
