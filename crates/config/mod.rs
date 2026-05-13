// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod config_handler;

pub use config_handler::ConfigHandler;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handler_loads_config() {
        let handler = ConfigHandler::new(|| "mpl");

        assert_eq!(handler.load(), "mpl");
    }
}
