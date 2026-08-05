// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlx::PgPool;

use crate::{migrations::Migrator, pool};

static MPL_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./src/migrations/local");

pub struct MplMigrator;

impl MplMigrator {
    pub fn migrations() -> impl Iterator<Item = &'static sqlx::migrate::Migration> {
        MPL_MIGRATOR.iter()
    }

    pub async fn run_on(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
        MPL_MIGRATOR.run(pool).await
    }
}

impl Migrator for MplMigrator {
    async fn run() -> Result<(), sqlx::migrate::MigrateError> {
        Self::run_on(pool()).await
    }
}
