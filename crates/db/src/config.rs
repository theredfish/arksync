// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_config::ConfigHandler;
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| ConfigHandler::new(mpl).load());

#[derive(Clone, Debug)]
pub struct Config {
    pub pg_db: String,
    pub pg_host: String,
    pub pg_port: u16,
    pub pg_user: String,
    pub pg_password: String,
    pub pg_max_connections: u32,
}

fn mpl() -> Config {
    Config {
        pg_db: "arksync".to_string(),
        pg_host: "localhost".to_string(),
        pg_port: 5433,
        pg_user: "admin".to_string(),
        pg_password: "admin".to_string(),
        pg_max_connections: 5,
    }
}
