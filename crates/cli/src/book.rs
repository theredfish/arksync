// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::{Args, Subcommand};
use eyre::{bail, eyre, Result, WrapErr};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

const MDBOOK_INSTALL_COMMAND: &str = "cargo install mdbook --version 0.5.4 --locked";
const MDBOOK_MERMAID_INSTALL_COMMAND: &str =
    "cargo install mdbook-mermaid --version 0.17.0 --locked";
const MDBOOK_VERSION: &str = "0.5.4";
const MDBOOK_MERMAID_VERSION: &str = "0.17.0";

#[derive(Debug, Subcommand)]
pub enum BookCommand {
    /// Build the ArkSync book.
    Build,
    /// Serve the book, rebuild it on changes, and refresh the browser.
    Watch(BookWatchOptions),
}

impl BookCommand {
    pub fn exec(&self) -> Result<()> {
        ensure_book_tools()?;

        match self {
            Self::Build => build(),
            Self::Watch(options) => watch(options),
        }
    }
}

#[derive(Debug, Args)]
pub struct BookWatchOptions {
    #[arg(
        long,
        default_value = "127.0.0.1",
        help = "Address used by the preview server"
    )]
    hostname: String,
    #[arg(long, default_value_t = 3000, help = "Port used by the preview server")]
    port: u16,
    #[arg(long, help = "Run the preview server without opening a browser")]
    headless: bool,
}

fn build() -> Result<()> {
    let book_dir = book_dir();
    run_mdbook([OsString::from("build"), book_dir.clone().into_os_string()])?;
    println!("built ArkSync book at {}", book_dir.join("book").display());

    Ok(())
}

fn watch(options: &BookWatchOptions) -> Result<()> {
    let mut args = vec![
        OsString::from("serve"),
        book_dir().into_os_string(),
        OsString::from("--hostname"),
        OsString::from(&options.hostname),
        OsString::from("--port"),
        OsString::from(options.port.to_string()),
    ];

    if !options.headless {
        args.push(OsString::from("--open"));
    }

    println!(
        "watching ArkSync book at http://{}:{}",
        options.hostname, options.port
    );
    run_mdbook(args)
}

fn ensure_book_tools() -> Result<()> {
    ensure_command_version("mdbook", MDBOOK_VERSION, MDBOOK_INSTALL_COMMAND)?;
    ensure_command_version(
        "mdbook-mermaid",
        MDBOOK_MERMAID_VERSION,
        MDBOOK_MERMAID_INSTALL_COMMAND,
    )
}

fn ensure_command_version(
    command: &str,
    expected_version: &str,
    install_command: &str,
) -> Result<()> {
    let output = Command::new(command)
        .arg("--version")
        .output()
        .map_err(|source| {
            eyre!(
                "required command `{command}` is not available: {source}\n\
                 Install it with `{install_command}`."
            )
        })?;

    if !output.status.success() {
        bail!(
            "required command `{command}` failed with status {}\n\
             Reinstall it with `{install_command}`.",
            output.status
        );
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    let actual_version = version_output
        .split_whitespace()
        .last()
        .map(|version| version.trim_start_matches('v'))
        .ok_or_else(|| eyre!("`{command} --version` returned no version"))?;

    if actual_version != expected_version {
        bail!(
            "required command `{command}` has version {actual_version}, but ArkSync requires \
             {expected_version}\nInstall it with `{install_command}`."
        );
    }

    Ok(())
}

fn run_mdbook<I, S>(args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let status = Command::new("mdbook")
        .args(args)
        .status()
        .wrap_err("failed to spawn mdbook")?;

    if !status.success() {
        bail!("mdbook failed with status {status}");
    }

    Ok(())
}

fn book_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("arksync-cli must remain under the workspace crates directory")
        .join("docs")
}
