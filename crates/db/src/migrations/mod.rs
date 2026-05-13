// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod migrator;
mod mpl_migrator;

pub use migrator::Migrator;
pub use mpl_migrator::MplMigrator;

pub async fn run() -> Result<(), sqlx::migrate::MigrateError> {
    MplMigrator::run().await
}
