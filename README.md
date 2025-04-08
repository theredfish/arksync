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