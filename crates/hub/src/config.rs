// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_config::ConfigHandler;
use arksync_utils::uuid::Uuid;
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| ConfigHandler::new(mpl).load());

#[derive(Clone, Debug)]
pub struct Config {
    pub local_system_user_id: Uuid,
    pub local_system_username: String,
    pub local_system_password: String,
    pub local_hub_id: Uuid,
    pub local_hub_name: String,
    pub local_hub_hardware_uid: String,
    pub local_knot_id: Uuid,
    pub local_knot_name: String,
    pub local_knot_hardware_uid: String,
}

fn mpl() -> Config {
    Config {
        local_system_user_id: local_system_user_id(),
        local_system_username: "arksync-system".to_string(),
        local_system_password: "not-used".to_string(),
        local_hub_id: local_hub_id(),
        local_hub_name: "ArkSync local hub".to_string(),
        local_hub_hardware_uid: "arksync-local-hub".to_string(),
        local_knot_id: local_knot_id(),
        local_knot_name: "ArkSync local knot".to_string(),
        local_knot_hardware_uid: "arksync-local-knot".to_string(),
    }
}

fn local_system_user_id() -> Uuid {
    Uuid::from_bytes([
        0x03, 0x03, 0x03, 0x03, 0x03, 0x03, 0x43, 0x03, 0x83, 0x03, 0x03, 0x03, 0x03, 0x03, 0x03,
        0x03,
    ])
}

fn local_hub_id() -> Uuid {
    Uuid::from_bytes([
        0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x41, 0x01, 0x81, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01,
        0x01,
    ])
}

fn local_knot_id() -> Uuid {
    Uuid::from_bytes([
        0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x42, 0x02, 0x82, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,
        0x02,
    ])
}
