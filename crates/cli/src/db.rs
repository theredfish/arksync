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
