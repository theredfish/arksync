// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use leptos::prelude::*;
use leptos_use::use_interval_fn;
use serde::Deserialize;

#[cfg(not(feature = "demo"))]
use futures_util::StreamExt as _;
#[cfg(feature = "demo")]
use std::{cell::Cell, rc::Rc};
#[cfg(not(feature = "demo"))]
use tauri_sys::{core::invoke_result, event::listen};
#[cfg(not(feature = "demo"))]
use wasm_bindgen_futures::spawn_local;

pub const DEMO_MODE: bool = cfg!(feature = "demo");

#[cfg(not(feature = "demo"))]
const STALE_MEASUREMENT_TIMEOUT_MS: f64 = 16_000.0;
#[cfg(feature = "demo")]
const DEMO_REFRESH_INTERVAL_MS: u64 = 5_000;
#[cfg(feature = "demo")]
const AIR_TEMPERATURE_LOOP: [f32; 8] = [23.6, 23.9, 24.2, 24.5, 24.7, 24.4, 24.0, 23.7];
#[cfg(feature = "demo")]
const WATER_TEMPERATURE_LOOP: [f32; 8] = [27.8, 27.9, 28.0, 28.1, 28.2, 28.1, 28.0, 27.9];
#[cfg(feature = "demo")]
const WATER_PH_LOOP: [f32; 8] = [6.78, 6.80, 6.80, 6.80, 6.80, 6.78, 6.78, 6.78];

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorSnapshot {
    pub name: String,
    pub latest: f32,
    pub labels: Vec<String>,
    pub values: Vec<f32>,
    /// Unix milliseconds; display labels are not suitable for a time axis.
    #[serde(default)]
    pub timestamps: Vec<f64>,
}

#[cfg(not(feature = "demo"))]
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSensorOverview {
    pub device_uid: String,
    pub display_name: Option<String>,
    pub kind: String,
    pub protocol: String,
    pub address: String,
    pub connection_detail: String,
    pub firmware: Option<f64>,
    pub measurement_interval_ms: i32,
    pub latest_value: Option<f32>,
    pub connected: bool,
}

#[derive(Clone, Copy)]
pub struct AppDataSources {
    pub air_temperature: RwSignal<Option<SensorSnapshot>>,
    pub water_temperature: RwSignal<Option<SensorSnapshot>>,
    pub water_ph: RwSignal<Option<SensorSnapshot>>,
}

impl AppDataSources {
    pub fn new() -> Self {
        let sources = Self {
            air_temperature: RwSignal::new(None),
            water_temperature: RwSignal::new(None),
            water_ph: RwSignal::new(None),
        };
        initialize_dashboard_sources(sources);

        sources
    }
}

#[cfg(feature = "demo")]
fn initialize_dashboard_sources(sources: AppDataSources) {
    let sequence = Rc::new(Cell::new(0_u32));
    initialize_demo_snapshots(sources);

    use_interval_fn(
        move || {
            let next = (sequence.get() + 1) % AIR_TEMPERATURE_LOOP.len() as u32;
            sequence.set(next);
            advance_demo_snapshots(sources, next as usize);
        },
        DEMO_REFRESH_INTERVAL_MS,
    );
}

#[cfg(feature = "demo")]
fn initialize_demo_snapshots(sources: AppDataSources) {
    sources.air_temperature.set(Some(demo_sensor_snapshot(
        "Air temperature — Canopy",
        24.0,
        0.62,
        0.18,
        1,
        23.0,
        25.0,
        AIR_TEMPERATURE_LOOP[0],
    )));
    sources.water_temperature.set(Some(demo_sensor_snapshot(
        "Water temperature — Tank 01",
        28.0,
        0.14,
        0.04,
        1,
        27.8,
        28.2,
        WATER_TEMPERATURE_LOOP[0],
    )));
    sources.water_ph.set(Some(demo_sensor_snapshot(
        "Water pH — Tank 01",
        6.78,
        0.0,
        0.0,
        2,
        6.78,
        6.80,
        WATER_PH_LOOP[0],
    )));
}

#[cfg(feature = "demo")]
fn advance_demo_snapshots(sources: AppDataSources, sequence: usize) {
    advance_demo_snapshot(sources.air_temperature, AIR_TEMPERATURE_LOOP[sequence], 1);
    advance_demo_snapshot(
        sources.water_temperature,
        WATER_TEMPERATURE_LOOP[sequence],
        1,
    );
    advance_demo_snapshot(sources.water_ph, WATER_PH_LOOP[sequence], 2);
}

#[cfg(not(feature = "demo"))]
fn initialize_dashboard_sources(sources: AppDataSources) {
    initialize_live_source("air_temperature_sensor", sources.air_temperature);
    initialize_live_source("water_temperature_sensor", sources.water_temperature);
    initialize_live_source("water_ph_sensor", sources.water_ph);
}

#[cfg(not(feature = "demo"))]
fn initialize_live_source(event_name: &'static str, data: RwSignal<Option<SensorSnapshot>>) {
    let last_measurement_at = RwSignal::new(None::<f64>);

    Effect::new(move |_| {
        spawn_local(async move {
            let mut stream = match listen::<SensorSnapshot>(event_name).await {
                Ok(stream) => stream,
                Err(err) => {
                    leptos::logging::error!("Failed to subscribe to {event_name}: {err}");
                    return;
                }
            };

            if let Err(err) = invoke_result::<(), String>(event_name, &()).await {
                leptos::logging::error!("Failed to start {event_name}: {err}");
                return;
            }

            while let Some(sensor_data) = stream.next().await {
                last_measurement_at.set(Some(js_sys::Date::now()));
                data.set(Some(sensor_data.payload));
            }
        });
    });

    use_interval_fn(
        move || {
            let Some(last_measurement_at) = last_measurement_at.get_untracked() else {
                return;
            };

            if js_sys::Date::now() - last_measurement_at > STALE_MEASUREMENT_TIMEOUT_MS {
                data.set(None);
            }
        },
        1_000,
    );
}

#[cfg(feature = "demo")]
fn demo_sensor_snapshot(
    name: &str,
    center: f32,
    primary_amplitude: f32,
    secondary_amplitude: f32,
    precision: u32,
    minimum: f32,
    maximum: f32,
    latest: f32,
) -> SensorSnapshot {
    const POINT_COUNT: usize = 61;
    const HISTORY_CYCLES: f32 = 1.35;

    let now = js_sys::Date::now();
    let mut values = (0..POINT_COUNT)
        .map(|index| {
            let history_progress = index as f32 / (POINT_COUNT - 1) as f32;
            let phase = history_progress * std::f32::consts::TAU * HISTORY_CYCLES;
            let value = center
                + phase.sin() * primary_amplitude
                + (phase * 0.43).cos() * secondary_amplitude;
            round_to_precision(value.clamp(minimum, maximum), precision)
        })
        .collect::<Vec<_>>();
    if let Some(last_value) = values.last_mut() {
        *last_value = latest;
    }

    let timestamps = (0..POINT_COUNT)
        .map(|index| {
            let samples_ago = POINT_COUNT - 1 - index;
            now - samples_ago as f64 * DEMO_REFRESH_INTERVAL_MS as f64
        })
        .collect::<Vec<_>>();
    let labels = timestamps
        .iter()
        .map(|timestamp| format_time(*timestamp))
        .collect();

    SensorSnapshot {
        name: name.to_string(),
        latest: values.last().copied().unwrap_or(latest),
        labels,
        values,
        timestamps,
    }
}

#[cfg(feature = "demo")]
fn advance_demo_snapshot(
    source: RwSignal<Option<SensorSnapshot>>,
    next_value: f32,
    precision: u32,
) {
    source.update(|snapshot| {
        let Some(snapshot) = snapshot else {
            return;
        };

        let next_value = round_to_precision(next_value, precision);
        if snapshot.values.is_empty() {
            snapshot.values.push(next_value);
        } else {
            snapshot.values.rotate_left(1);
            if let Some(last_value) = snapshot.values.last_mut() {
                *last_value = next_value;
            }
        }

        let now = js_sys::Date::now();
        if snapshot.timestamps.is_empty() {
            snapshot.timestamps.push(now);
        } else {
            snapshot.timestamps.rotate_left(1);
            if let Some(last_timestamp) = snapshot.timestamps.last_mut() {
                *last_timestamp = now;
            }
        }
        let next_label = format_time(now);
        if snapshot.labels.is_empty() {
            snapshot.labels.push(next_label);
        } else {
            snapshot.labels.rotate_left(1);
            if let Some(last_label) = snapshot.labels.last_mut() {
                *last_label = next_label;
            }
        }

        // The gauge and the history are two views over this same sample.
        snapshot.latest = snapshot.values.last().copied().unwrap_or(next_value);
    });
}

#[cfg(feature = "demo")]
fn format_time(timestamp_ms: f64) -> String {
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(timestamp_ms));
    format!(
        "{:02}:{:02}:{:02}",
        date.get_hours(),
        date.get_minutes(),
        date.get_seconds()
    )
}

#[cfg(feature = "demo")]
fn round_to_precision(value: f32, precision: u32) -> f32 {
    let multiplier = 10_f32.powi(precision as i32);
    (value * multiplier).round() / multiplier
}

#[cfg(not(feature = "demo"))]
pub async fn load_live_sensors() -> Result<Vec<LiveSensorOverview>, String> {
    invoke_result("sensors_overview", &()).await
}
