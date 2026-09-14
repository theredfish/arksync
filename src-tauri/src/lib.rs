// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::Timestamp;
use arksync_hub::SensorTimeSeries;
use arksync_sensor::infrastructure::events::SensorKind;
use chrono::{Local, TimeZone};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter};
use tauri_plugin_log::{Builder as TauriLog, Target, TargetKind};
use tokio::time::{interval, Duration};

pub static SENSORS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
const SENSOR_SERIES_WINDOW: Duration = Duration::from_secs(30 * 60);
const SENSOR_SERIES_LIMIT: i64 = 1_500;
const SENSOR_LIVE_WINDOW: Duration = Duration::from_secs(15);

pub fn builder() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::<tauri::Wry>::default()
        // WebKitGTK can create its GL context halfway through GTK's first frame on
        // NVIDIA/Wayland. Arm the window's GL context before that frame so DMA-BUF
        // and explicit sync remain enabled without triggering Wayland error 71.
        .plugin(tauri_plugin_wayland_nvidia_quirk::init())
        .setup(|app| {
            #[cfg(not(feature = "demo"))]
            {
                tauri::async_runtime::block_on(async {
                    arksync_db::run().await?;
                    let mut txn = arksync_db::pool().begin().await?;
                    arksync_hub::setup_local_station(&mut txn).await?;
                    txn.commit().await?;

                    eyre::Ok(())
                })
                .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;

                tauri::async_runtime::spawn(arksync_hub::HubRuntime::run());
            }
            Ok(())
        })
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            TauriLog::new()
                .targets([
                    Target::new(TargetKind::Stderr),
                    Target::new(TargetKind::Webview),
                ])
                .level(log::LevelFilter::Debug)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            air_temperature_sensor,
            water_temperature_sensor,
            water_ph_sensor,
            sensors_overview
        ])
}

pub fn run(context: tauri::Context) {
    builder().run(context).expect("Failed to run ArkSync");
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SensorSeriesData {
    name: String,
    latest: f32,
    labels: Vec<String>,
    values: Vec<f32>,
    timestamps: Vec<i64>,
}

#[derive(Clone, Copy, Debug)]
enum DashboardMetric {
    AirTemperature,
    WaterTemperature,
    WaterPh,
}

impl DashboardMetric {
    fn event_name(self) -> &'static str {
        match self {
            Self::AirTemperature => "air_temperature_sensor",
            Self::WaterTemperature => "water_temperature_sensor",
            Self::WaterPh => "water_ph_sensor",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::AirTemperature => "Air temperature",
            Self::WaterTemperature => "Water temperature",
            Self::WaterPh => "Water pH",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SensorOverviewData {
    device_uid: String,
    display_name: Option<String>,
    kind: String,
    protocol: String,
    address: String,
    connection_detail: String,
    firmware: Option<f64>,
    measurement_interval_ms: i32,
    latest_value: Option<f32>,
    connected: bool,
}

#[tauri::command]
async fn air_temperature_sensor(app: AppHandle) {
    start_dashboard_sensor(app, DashboardMetric::AirTemperature);
}

#[tauri::command]
async fn water_temperature_sensor(app: AppHandle) {
    start_dashboard_sensor(app, DashboardMetric::WaterTemperature);
}

#[tauri::command]
async fn water_ph_sensor(app: AppHandle) {
    start_dashboard_sensor(app, DashboardMetric::WaterPh);
}

fn start_dashboard_sensor(app: AppHandle, metric: DashboardMetric) {
    let event_name = metric.event_name();
    let mut sensors = SENSORS.lock().unwrap();

    if sensors.contains(event_name) {
        log::info!("Dashboard source '{event_name}' is already registered.");
        return;
    }

    sensors.insert(event_name.to_string());
    drop(sensors);

    tauri::async_runtime::spawn(async move {
        log::info!("Spawning dashboard source '{event_name}'...");

        let mut interval = interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            if let Some(sensor_data) =
                load_dashboard_sensor_data(metric, SENSOR_SERIES_WINDOW, SENSOR_SERIES_LIMIT).await
            {
                log::debug!("{sensor_data:#?}");

                app.emit(event_name, &sensor_data).unwrap();
            }
        }
    });
}

#[tauri::command]
async fn sensors_overview() -> Result<Vec<SensorOverviewData>, String> {
    let sensors = arksync_hub::list_sensors(arksync_db::pool())
        .await
        .map_err(|err| {
            log::error!("Failed to load sensors overview: {err:?}");
            err.to_string()
        })?;
    let latest_measurement = arksync_hub::load_latest_sensor_measurement(arksync_db::pool())
        .await
        .map_err(|err| {
            log::error!("Failed to load latest sensor measurement: {err:?}");
            err.to_string()
        })?;

    Ok(sensors
        .into_iter()
        .map(|sensor| {
            let live_measurement = latest_measurement.as_ref().filter(|measurement| {
                measurement.sensor_id == sensor.id && timestamp_is_live(measurement.measured_at)
            });
            let (address, connection_detail) =
                connection_labels(&sensor.protocol, &sensor.connection);

            SensorOverviewData {
                device_uid: sensor.device_uid,
                display_name: sensor.display_name,
                kind: match sensor.kind {
                    SensorKind::Temperature => "temperature",
                    SensorKind::Custom => "custom",
                }
                .to_string(),
                protocol: sensor.protocol,
                address,
                connection_detail,
                firmware: sensor.firmware,
                measurement_interval_ms: sensor.measurement_interval_ms,
                latest_value: live_measurement
                    .map(|measurement| round_to_tenth(measurement.value as f32)),
                connected: live_measurement.is_some(),
            }
        })
        .collect())
}

async fn load_dashboard_sensor_data(
    metric: DashboardMetric,
    window: Duration,
    limit: i64,
) -> Option<SensorSeriesData> {
    let sensors = arksync_hub::list_sensors(arksync_db::pool())
        .await
        .map_err(|err| {
            log::error!("Failed to select {} source: {err:?}", metric.display_name());
            err
        })
        .ok()?;
    let sensor = select_dashboard_sensor(metric, &sensors)?;
    let window_end = timestamp_now();
    let window_start =
        Timestamp::from_unix_millis(window_end.unix_millis - window.as_millis() as i64);
    let series = arksync_hub::load_sensor_time_series(
        arksync_db::pool(),
        sensor.id,
        window_start,
        window_end,
        limit,
    )
    .await
    .map_err(|err| {
        log::error!(
            "Failed to load {} time series: {err:?}",
            metric.display_name()
        );
        err
    })
    .ok()?;
    if !series_has_live_measurement(&series) {
        return None;
    }
    let values = series
        .points
        .iter()
        .map(|point| round_to_tenth(point.value as f32))
        .collect::<Vec<_>>();
    let latest = values.last().copied()?;

    Some(SensorSeriesData {
        name: sensor
            .display_name
            .clone()
            .unwrap_or_else(|| metric.display_name().to_string()),
        latest,
        labels: measurement_labels(&series),
        values,
        timestamps: series
            .points
            .iter()
            .map(|point| point.measured_at.unix_millis)
            .collect(),
    })
}

fn select_dashboard_sensor(
    metric: DashboardMetric,
    sensors: &[arksync_hub::Sensor],
) -> Option<&arksync_hub::Sensor> {
    let name_matches = |sensor: &&arksync_hub::Sensor, needle: &str| {
        sensor
            .display_name
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains(needle)
    };

    match metric {
        DashboardMetric::AirTemperature => sensors
            .iter()
            .filter(|sensor| sensor.kind == SensorKind::Temperature)
            .find(|sensor| name_matches(sensor, "air") || name_matches(sensor, "canopy"))
            .or_else(|| {
                sensors
                    .iter()
                    .filter(|sensor| sensor.kind == SensorKind::Temperature)
                    .find(|sensor| !name_matches(sensor, "water"))
            })
            .or_else(|| {
                sensors
                    .iter()
                    .find(|sensor| sensor.kind == SensorKind::Temperature)
            }),
        DashboardMetric::WaterTemperature => sensors
            .iter()
            .filter(|sensor| sensor.kind == SensorKind::Temperature)
            .find(|sensor| name_matches(sensor, "water")),
        DashboardMetric::WaterPh => sensors.iter().find(|sensor| name_matches(sensor, "ph")),
    }
}

fn connection_labels(protocol: &str, connection: &Value) -> (String, String) {
    let payload = connection.get(protocol).unwrap_or(connection);

    match protocol {
        "uart" => {
            let port = payload
                .get("port_name")
                .and_then(Value::as_str)
                .unwrap_or("Not reported")
                .to_string();
            let baud_rate = payload
                .get("baud_rate")
                .and_then(Value::as_u64)
                .map(|baud_rate| format!("{baud_rate} baud"))
                .unwrap_or_else(|| "Baud rate not reported".to_string());
            (port, baud_rate)
        }
        "i2c" => {
            let address = payload
                .get("address")
                .and_then(Value::as_u64)
                .map(|address| format!("0x{address:02X}"))
                .unwrap_or_else(|| "Not reported".to_string());
            (address, "Local I2C bus".to_string())
        }
        _ => ("Not reported".to_string(), "Unknown protocol".to_string()),
    }
}

fn round_to_tenth(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

fn measurement_labels(series: &SensorTimeSeries) -> Vec<String> {
    series
        .points
        .iter()
        .map(|point| format_timestamp(point.measured_at))
        .collect()
}

fn series_has_live_measurement(series: &SensorTimeSeries) -> bool {
    series
        .points
        .last()
        .map(|point| timestamp_is_live(point.measured_at))
        .unwrap_or(false)
}

fn timestamp_is_live(timestamp: Timestamp) -> bool {
    let now = timestamp_now();
    let live_window_start =
        Timestamp::from_unix_millis(now.unix_millis - SENSOR_LIVE_WINDOW.as_millis() as i64);

    timestamp >= live_window_start && timestamp <= now
}

fn format_timestamp(timestamp: Timestamp) -> String {
    Local
        .timestamp_millis_opt(timestamp.unix_millis)
        .single()
        .map(|datetime| datetime.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "--:--:--".to_string())
}

fn timestamp_now() -> Timestamp {
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();

    Timestamp::from_unix_millis(unix_millis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eyre::Result;

    #[test]
    fn build_app() -> Result<()> {
        builder();

        Ok(())
    }
}
