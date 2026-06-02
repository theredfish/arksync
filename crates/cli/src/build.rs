// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::{Args, Subcommand, ValueEnum};
use eyre::{bail, eyre, Result, WrapErr};
use std::ffi::OsString;
use std::process::Command;

#[derive(Debug, Subcommand)]
pub enum BuildCommand {
    Actuator(ActuatorBuildOptions),
}

impl BuildCommand {
    pub fn exec(&self) -> Result<()> {
        match self {
            Self::Actuator(options) => build_actuator(options),
        }
    }
}

#[derive(Debug, Args)]
pub struct ActuatorBuildOptions {
    #[arg(long, value_enum, help = "Actuator build target")]
    target: ActuatorBuildTarget,
    #[arg(long, value_enum, default_value_t = BuildProfile::Dev, help = "Cargo build profile")]
    profile: BuildProfile,
    #[arg(long, help = "Print the cargo command without running it")]
    dry_run: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ActuatorBuildTarget {
    LinuxRpi,
    Esp32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum BuildProfile {
    Dev,
    Release,
}

impl BuildProfile {
    fn target_dir_name(self) -> &'static str {
        match self {
            Self::Dev => "debug",
            Self::Release => "release",
        }
    }
}

struct ActuatorBuildPlan {
    label: &'static str,
    rust_target: &'static str,
    feature: &'static str,
    linker: Option<&'static str>,
    linker_env: &'static str,
}

impl ActuatorBuildPlan {
    fn new(target: ActuatorBuildTarget) -> Self {
        match target {
            ActuatorBuildTarget::LinuxRpi => Self {
                label: "Linux Raspberry Pi",
                rust_target: "aarch64-unknown-linux-gnu",
                feature: "linux-gpio",
                linker: Some("aarch64-linux-gnu-gcc"),
                linker_env: "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER",
            },
            ActuatorBuildTarget::Esp32 => unreachable!("ESP32 actuator build is not wired yet"),
        }
    }
}

fn build_actuator(options: &ActuatorBuildOptions) -> Result<()> {
    ensure_command("cargo")?;
    ensure_command("rustc")?;

    if options.target == ActuatorBuildTarget::Esp32 {
        bail!(
            "ESP32 actuator build is not wired yet. It needs a dedicated Embassy/esp-hal firmware binary and ESP toolchain setup."
        );
    }

    let plan = ActuatorBuildPlan::new(options.target);
    let mut args: Vec<OsString> = vec![
        "build".into(),
        "-p".into(),
        "arksync-actuator".into(),
        "--bin".into(),
        "arksync-actuator".into(),
        "--features".into(),
        plan.feature.into(),
        "--target".into(),
        plan.rust_target.into(),
    ];

    if options.profile == BuildProfile::Release {
        args.push("--release".into());
    }

    if options.dry_run {
        if let Some(linker) = plan.linker {
            println!("{}={} cargo {}", plan.linker_env, linker, join_args(&args));
        } else {
            println!("cargo {}", join_args(&args));
        }
        return Ok(());
    }

    ensure_rust_target_installed(plan.rust_target)?;
    if let Some(linker) = plan.linker {
        ensure_command(linker)?;
    }

    let mut command = Command::new("cargo");
    command.args(&args);

    if let Some(linker) = plan.linker {
        command.env(plan.linker_env, linker);
    }

    let status = command.status().wrap_err("failed to spawn cargo build")?;

    if !status.success() {
        bail!("cargo build failed with status {status}");
    }

    println!(
        "built actuator probe for {} at target/{}/{}/arksync-actuator",
        plan.label,
        plan.rust_target,
        options.profile.target_dir_name()
    );

    Ok(())
}

fn ensure_command(command: &str) -> Result<()> {
    let output = Command::new(command)
        .arg("--version")
        .output()
        .map_err(|source| command_not_available_error(command, source))?;

    if !output.status.success() {
        bail!(
            "required command `{command}` failed with status {}",
            output.status
        );
    }

    Ok(())
}

fn command_not_available_error(command: &str, source: std::io::Error) -> eyre::Report {
    if command == "aarch64-linux-gnu-gcc" {
        return eyre!(
            "required cross-linker `{command}` is not available: {source}\n\
             Install an AArch64 Linux GNU cross compiler/linker and make sure `{command}` is in PATH."
        );
    }

    eyre!("required command `{command}` is not available: {source}")
}

fn ensure_rust_target_installed(target: &str) -> Result<()> {
    ensure_command("rustup")?;

    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .wrap_err("failed to list installed Rust targets")?;

    if !output.status.success() {
        bail!("rustup target list failed with status {}", output.status);
    }

    let installed_targets =
        String::from_utf8(output.stdout).wrap_err("rustup target list output is not UTF-8")?;

    if !installed_targets
        .lines()
        .any(|installed| installed == target)
    {
        bail!("Rust target `{target}` is not installed. Run `rustup target add {target}` first.");
    }

    Ok(())
}

fn join_args(args: &[OsString]) -> String {
    args.iter()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}
