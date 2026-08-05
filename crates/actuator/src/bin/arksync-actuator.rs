// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_actuator::relay::{RelayDriver, RelayState, MIST_RELAY};
use std::thread;
use std::time::Duration;

const TOGGLE_INTERVAL: Duration = Duration::from_secs(5);

fn main() {
    let driver = match RelayDriver::new(MIST_RELAY) {
        Ok(driver) => driver,
        Err(error) => {
            eprintln!(
                "failed to start actuator probe for relay '{}' on GPIO{}: {error:?}",
                MIST_RELAY.id, MIST_RELAY.gpio_bcm_pin
            );
            std::process::exit(1);
        }
    };
    let mut active = false;

    println!(
        "starting actuator probe for relay '{}' on GPIO{}; toggling every {}s",
        MIST_RELAY.id,
        MIST_RELAY.gpio_bcm_pin,
        TOGGLE_INTERVAL.as_secs()
    );

    loop {
        active = !active;
        let state = RelayState::new(MIST_RELAY, active);

        if let Err(error) = driver.apply(state) {
            eprintln!(
                "failed to switch relay '{}' on GPIO{}: {error:?}",
                MIST_RELAY.id, MIST_RELAY.gpio_bcm_pin
            );
            std::process::exit(1);
        }

        println!(
            "relay '{}' switched {} with {} level",
            MIST_RELAY.id,
            if state.active { "ON" } else { "OFF" },
            state.level
        );

        thread::sleep(TOGGLE_INTERVAL);
    }
}
