// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{migrations::Migrator, pool};

pub struct MplMigrator;

impl Migrator for MplMigrator {
    async fn run() -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!("./src/migrations/local").run(pool()).await
    }
}
