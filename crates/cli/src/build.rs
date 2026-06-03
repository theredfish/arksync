// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use clap::{Args, Subcommand, ValueEnum};
use eyre::{bail, eyre, Result, WrapErr};
use std::env;
use std::ffi::OsString;
use std::process::Command;

#[derive(Debug, Subcommand)]
pub enum BuildCommand {
    Actuator(ActuatorBuildOptions),
    App(AppBuildOptions),
}

impl BuildCommand {
    pub fn exec(&self) -> Result<()> {
        match self {
            Self::Actuator(options) => build_actuator(options),
            Self::App(options) => build_app(options),
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

#[derive(Debug, Args)]
pub struct AppBuildOptions {
    #[arg(long, value_enum, help = "Tauri app build target")]
    target: AppBuildTarget,
    #[arg(long, value_enum, default_value_t = BuildProfile::Release, help = "Tauri build profile")]
    profile: BuildProfile,
    #[arg(long, help = "Skip Tauri bundle generation")]
    no_bundle: bool,
    #[arg(long, help = "Print the cargo tauri command without running it")]
    dry_run: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum AppBuildTarget {
    LinuxX64,
    LinuxArm64,
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

struct AppBuildPlan {
    label: &'static str,
    rust_target: &'static str,
    linker: Option<&'static str>,
    linker_env: &'static str,
}

impl AppBuildPlan {
    fn new(target: AppBuildTarget) -> Self {
        match target {
            AppBuildTarget::LinuxX64 => Self {
                label: "Linux x86_64",
                rust_target: "x86_64-unknown-linux-gnu",
                linker: None,
                linker_env: "",
            },
            AppBuildTarget::LinuxArm64 => Self {
                label: "Linux ARM64",
                rust_target: "aarch64-unknown-linux-gnu",
                linker: Some("aarch64-linux-gnu-gcc"),
                linker_env: "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER",
            },
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

fn build_app(options: &AppBuildOptions) -> Result<()> {
    ensure_command("cargo")?;
    ensure_command("cargo-tauri")?;
    ensure_command("rustc")?;
    ensure_command("trunk")?;

    let plan = AppBuildPlan::new(options.target);
    let mut args: Vec<OsString> = vec![
        "tauri".into(),
        "build".into(),
        "--ci".into(),
        "--target".into(),
        plan.rust_target.into(),
    ];

    if options.profile == BuildProfile::Dev {
        args.push("--debug".into());
    }

    if options.no_bundle {
        args.push("--no-bundle".into());
    }

    if options.dry_run {
        let prefix = "NO_COLOR=false";
        if let Some(linker) = plan.linker {
            println!(
                "{prefix} {}={} cargo {}",
                plan.linker_env,
                linker,
                join_args(&args)
            );
        } else {
            println!("{prefix} cargo {}", join_args(&args));
        }
        return Ok(());
    }

    ensure_rust_target_installed(plan.rust_target)?;
    if let Some(linker) = plan.linker {
        ensure_command(linker)?;
    }

    if options.target == AppBuildTarget::LinuxArm64 {
        ensure_linux_arm64_pkg_config_is_configured()?;
        println!(
            "building Tauri for Linux ARM64 from this host is best-effort: WebKitGTK, AppImage, \
             pkg-config and system library cross files must also be available for the target. \
             The publish workflow currently uses a native ubuntu-22.04-arm runner to avoid this."
        );
    }

    let mut command = Command::new("cargo");
    command.args(&args);
    command.env("NO_COLOR", "false");

    if let Some(linker) = plan.linker {
        command.env(plan.linker_env, linker);
    }

    let status = command
        .status()
        .wrap_err("failed to spawn cargo tauri build")?;

    if !status.success() {
        bail!("cargo tauri build failed with status {status}");
    }

    println!("built Tauri app for {}", plan.label);

    Ok(())
}

fn ensure_linux_arm64_pkg_config_is_configured() -> Result<()> {
    let has_pkg_config_wrapper = env::var_os("PKG_CONFIG_aarch64_unknown_linux_gnu").is_some()
        || env::var_os("PKG_CONFIG_aarch64-unknown-linux-gnu").is_some()
        || env::var_os("TARGET_PKG_CONFIG").is_some();
    let has_pkg_config_sysroot = env::var_os("PKG_CONFIG_SYSROOT_DIR_aarch64_unknown_linux_gnu")
        .is_some()
        || env::var_os("PKG_CONFIG_SYSROOT_DIR_aarch64-unknown-linux-gnu").is_some()
        || env::var_os("TARGET_PKG_CONFIG_SYSROOT_DIR").is_some()
        || env::var_os("PKG_CONFIG_SYSROOT_DIR").is_some();
    let allows_cross = env::var_os("PKG_CONFIG_ALLOW_CROSS_aarch64_unknown_linux_gnu").is_some()
        || env::var_os("PKG_CONFIG_ALLOW_CROSS_aarch64-unknown-linux-gnu").is_some()
        || env::var_os("TARGET_PKG_CONFIG_ALLOW_CROSS").is_some()
        || env::var_os("PKG_CONFIG_ALLOW_CROSS").is_some();

    if has_pkg_config_wrapper || has_pkg_config_sysroot || allows_cross {
        return Ok(());
    }

    bail!(
        "Linux ARM64 Tauri cross-build needs target pkg-config setup for GLib/GTK/WebKit. \
         Configure an ARM64 sysroot with PKG_CONFIG_SYSROOT_DIR and PKG_CONFIG_PATH, \
         or provide a cross pkg-config wrapper via PKG_CONFIG_aarch64_unknown_linux_gnu. \
         The publish workflow currently avoids this by building on a native ubuntu-22.04-arm runner."
    );
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
