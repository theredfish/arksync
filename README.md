# ArkSync

An environmental control system for Raspberry Pi 4+. Curious about the project? Learn more on my [blog](https://theredfi.sh/).

## Getting Started

```bash
docker-compose up -d

docker exec influxdb influx setup \
  --username admin \
  --password sation_admin \
  --org station_knot \
  --bucket arksync_series \
  --force
```

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Documentation

ArkSync book: [stable](https://theredfish.github.io/arksync/version/stable/) |
[dev](https://theredfish.github.io/arksync/version/dev/).

Install the documentation tools with:

```bash
cargo install mdbook --version 0.5.4 --locked
cargo install mdbook-mermaid --version 0.17.0 --locked
```

Build the book with:

```bash
cargo sk book build
```

For a local preview with automatic rebuild and browser reload:

```bash
cargo sk book watch
```

Use `cargo sk book watch --headless` when running without a local browser. The preview address and
port can be changed with `--hostname` and `--port`.

## Troubleshooting

### GBM Buffer

> Failed to create GBM buffer of size 800x600: Invalid argument

WebKitGTK’s DMABUF renderer can conflict with NVIDIA drivers. You can disable it by setting an environment variable:

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 cargo tauri dev
```

### Nvidia explicit sync and Wayland

> Error 71 (Protocol error) dispatching to Wayland display.

```bash
__NV_DISABLE_EXPLICIT_SYNC=1 cargo tauri dev
```

## Influxdb

### Generate an admin token

This will be used for adding a new server to InfluxDB Explorer UI, and other
admin tasks later.

- `docker exec -it [container] bash`
- `influxdb3 create token --admin`

## Motion detection

This is the setup to have a motion detection camera feed on Rpi and a feed on
your Grafana dashboard:

- `sudo apt update && sudo apt install motion`
- Edit  `/etc/motion/motion.conf` and set `stream_localhost off`
- Also set `stream_quality 75` or less for best performances
- `sudo systemctl enable motion`
- `sudo systemctl start motion`
- http://<station_ip>:8081/

For adding the feed in Grafana add a `Text` panel with `Html` and add the iframe:

`<iframe src="http://<station_ip>:8081/" width="640" height="480"></iframe>`

## Deploying

### Actuator Probe

Build the Raspberry Pi relay probe from the desktop with:

```bash
cargo sk build actuator --target linux-rpi --profile dev
```

Use `--dry-run` to inspect the command without building:

```bash
cargo sk build actuator --target linux-rpi --profile dev --dry-run
```

This prints the `cargo build` command, including the target and linker environment:

```bash
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc cargo build -p arksync-actuator --bin arksync-actuator --features linux-gpio --target aarch64-unknown-linux-gnu
```

Prerequisites for Raspberry Pi 4 running a 64-bit OS:

```bash
rustup target add aarch64-unknown-linux-gnu
```

Install an AArch64 Linux GNU cross compiler/linker and make sure
`aarch64-linux-gnu-gcc` is available in `PATH`.

The generated binary is:

```bash
target/aarch64-unknown-linux-gnu/debug/arksync-actuator
```

### Tauri App

Build the local Linux desktop app:

```bash
cargo sk build app --target linux-x64
```

For a faster local check without packaging the AppImage:

```bash
cargo sk build app --target linux-x64 --profile dev --no-bundle
```

Inspect the command without building:

```bash
cargo sk build app --target linux-x64 --dry-run
```

An ARM64 target is available as a best-effort cross-build:

```bash
cargo sk build app --target linux-arm64 --profile dev --no-bundle
```

The publish workflow currently builds ARM on a native `ubuntu-22.04-arm` runner.
Cross-building the full Tauri app from x86_64 also requires target WebKitGTK,
AppImage, pkg-config and system library setup, so native ARM remains the
reliable release path for now.

### Development

- `cargo tauri signer generate -w ~/.tauri/arksync_dev.key`
- `export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=arksync-dev`
- `export TAURI_SIGNING_PRIVATE_KEY=${cat ~/.tauri/arksync_dev.key}`
