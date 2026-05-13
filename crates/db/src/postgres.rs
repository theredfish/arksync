// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::LazyLock;

use crate::CONFIG;

pub static PG_POOL: LazyLock<PgPool> = LazyLock::new(|| connect_db(&CONFIG.pg_db));

pub fn pool() -> &'static PgPool {
    &PG_POOL
}

pub fn connect_db(database_name: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(CONFIG.pg_max_connections)
        .connect_lazy(&database_url(database_name))
        .expect("postgres database url should be valid")
}

fn database_url(database_name: &str) -> String {
    format!(
        "postgres://{}:{}@{}:{}/{}",
        CONFIG.pg_user, CONFIG.pg_password, CONFIG.pg_host, CONFIG.pg_port, database_name
    )
}
