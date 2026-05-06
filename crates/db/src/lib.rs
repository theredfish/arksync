// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod config;
pub mod migrations;
mod postgres;
mod postgres_reset;
mod postgres_setup;

pub use config::{Config, CONFIG};
pub use postgres::{connect_db, pool, PG_POOL};
pub use postgres_reset::reset_public_schema;
pub use postgres_setup::setup;

pub async fn run() -> eyre::Result<()> {
    setup().await?;
    migrations::run().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pg_pool_is_available_lazily() {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        runtime.block_on(async {
            let _ = pool();
        });
    }
}
