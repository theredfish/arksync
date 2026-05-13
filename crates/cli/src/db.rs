// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_db::MplMigrator;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    Reset,
}

impl DbCommand {
    pub async fn exec(&self) -> eyre::Result<()> {
        match self {
            Self::Reset => reset_db().await,
        }
    }
}

async fn reset_db() -> eyre::Result<()> {
    arksync_db::reset_public_schema::<MplMigrator>(arksync_db::pool()).await?;

    Ok(())
}
