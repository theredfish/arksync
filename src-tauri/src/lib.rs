// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_bus::Timestamp;
use arksync_hub::SensorTimeSeries;
use chrono::{Local, TimeZone};
use serde::Serialize;
use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter};
use tauri_plugin_log::{Builder as TauriLog, Target, TargetKind};
use tokio::time::{interval, Duration};

pub static SENSORS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));
const TEMPERATURE_SERIES_WINDOW: Duration = Duration::from_secs(30 * 60);
const TEMPERATURE_SERIES_LIMIT: i64 = 1_500;

pub fn builder() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::<tauri::Wry>::default()
        .setup(|app| {
            tauri::async_runtime::block_on(async {
                arksync_db::run().await?;
                arksync_hub::setup_local_station(arksync_db::pool()).await?;

                eyre::Ok(())
            })
            .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;

            tauri::async_runtime::spawn(arksync_hub::HubRuntime::run());
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
            water_temperature_sensor
        ])
}

pub fn run(context: tauri::Context) {
    builder().run(context).expect("Failed to run ArkSync");
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TemperatureSeriesData {
    name: String,
    labels: Vec<String>,
    value: Vec<f32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TemperatureGaugeData {
    name: String,
    value: f32,
}

#[tauri::command]
async fn water_temperature_sensor(app: AppHandle) {
    let sensor_name = "water_temperature_sensor";
    let mut sensors = SENSORS.lock().unwrap();

    if sensors.contains(sensor_name) {
        log::info!("Sensor '{sensor_name}' already registered.");
        return;
    }

    sensors.insert(sensor_name.to_string());

    tauri::async_runtime::spawn(async move {
        log::info!("Spawning sensor '{sensor_name}'...");

        let mut interval = interval(Duration::from_secs(30));
        loop {
            interval.tick().await;

            let sensor_data =
                load_temperature_series(TEMPERATURE_SERIES_WINDOW, TEMPERATURE_SERIES_LIMIT).await;

            log::debug!("{sensor_data:#?}");

            app.emit("water_temperature_sensor", &sensor_data).unwrap();
        }
    });
}

#[tauri::command]
async fn air_temperature_sensor(app: AppHandle) {
    let sensor_name = "air_temperature_sensor";
    let mut sensors = SENSORS.lock().unwrap();

    if sensors.contains(sensor_name) {
        log::info!("Sensor '{sensor_name}' is already registered.");
        return;
    }

    sensors.insert(sensor_name.to_string());

    tauri::async_runtime::spawn(async move {
        log::info!("Spawning sensor '{sensor_name}'...");

        let mut interval = interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            let sensor_data = load_temperature_gauge().await;

            log::debug!("{sensor_data:#?}");

            app.emit("air_temperature_sensor", &sensor_data).unwrap();
        }
    });
}

async fn load_temperature_series(window: Duration, limit: i64) -> TemperatureSeriesData {
    let Some(series) = load_latest_time_series(window, limit).await else {
        return TemperatureSeriesData {
            name: "Water Temperature (C°)".to_string(),
            labels: Vec::new(),
            value: Vec::new(),
        };
    };

    TemperatureSeriesData {
        name: "Water Temperature (C°)".to_string(),
        labels: measurement_labels(&series),
        value: series
            .points
            .iter()
            .map(|point| point.value as f32)
            .collect(),
    }
}

async fn load_temperature_gauge() -> TemperatureGaugeData {
    let value = arksync_hub::load_latest_sensor_measurement(arksync_db::pool())
        .await
        .map_err(|err| {
            log::error!("Failed to load latest sensor measurement: {err:?}");
            err
        })
        .ok()
        .flatten()
        .map(|measurement| round_to_tenth(measurement.value as f32))
        .unwrap_or_default();

    TemperatureGaugeData {
        name: "Air Temperature (C°)".to_string(),
        value,
    }
}

fn round_to_tenth(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}

async fn load_latest_time_series(window: Duration, limit: i64) -> Option<SensorTimeSeries> {
    let window_end = timestamp_now();
    let window_start =
        Timestamp::from_unix_millis(window_end.unix_millis - window.as_millis() as i64);

    match arksync_hub::load_latest_sensor_time_series(
        arksync_db::pool(),
        window_start,
        window_end,
        limit,
    )
    .await
    {
        Ok(series) => series,
        Err(err) => {
            log::error!("Failed to load latest sensor time series: {err:?}");
            None
        }
    }
}

fn measurement_labels(series: &SensorTimeSeries) -> Vec<String> {
    series
        .points
        .iter()
        .map(|point| format_timestamp(point.measured_at))
        .collect()
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
