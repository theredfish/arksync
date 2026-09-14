// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::build::{ensure_command, join_args};
use clap::Args;
use eyre::{bail, Result, WrapErr};
use std::{ffi::OsString, process::Command};

#[derive(Debug, Args)]
pub struct DevOptions {
    #[arg(long, help = "Use generated demo data instead of live Hub data")]
    demo: bool,
    #[arg(long, help = "Print the cargo tauri command without running it")]
    dry_run: bool,
}

impl DevOptions {
    pub fn exec(&self) -> Result<()> {
        ensure_command("cargo")?;
        ensure_command("cargo-tauri")?;
        ensure_command("trunk")?;

        let mut args: Vec<OsString> = vec!["tauri".into(), "dev".into()];
        if self.demo {
            args.push("--features".into());
            args.push("demo".into());
        }

        if self.dry_run {
            let mut environment = vec!["NO_COLOR=false"];
            if self.demo {
                environment.push("TRUNK_BUILD_FEATURES=demo");
            }
            println!("{} cargo {}", environment.join(" "), join_args(&args));
            return Ok(());
        }

        let mut command = Command::new("cargo");
        command.args(&args);
        command.env("NO_COLOR", "false");
        if self.demo {
            // Tauri forwards Cargo features to the backend only. Trunk needs
            // this explicit feature list to compile the matching UI variant.
            command.env("TRUNK_BUILD_FEATURES", "demo");
        } else {
            command.env_remove("TRUNK_BUILD_FEATURES");
        }

        let status = command
            .status()
            .wrap_err("failed to spawn cargo tauri dev")?;
        if !status.success() {
            bail!("cargo tauri dev failed with status {status}");
        }

        Ok(())
    }
}
