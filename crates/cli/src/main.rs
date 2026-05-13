// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::{Parser, Subcommand};

mod db;

#[derive(Debug, Parser)]
#[command(name = "sk")]
#[command(about = "Station Knot command line tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(subcommand)]
    Db(db::DbCommand),
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Db(cmd) => cmd.exec().await,
    }
}
