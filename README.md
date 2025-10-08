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