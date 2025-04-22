use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::Serialize;
use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
};
use tauri::{AppHandle, Emitter};
use tauri_plugin_log::{Builder as TauriLog, Target, TargetKind};
use tokio::time::{interval, Duration};

pub fn run() {
    tauri::Builder::default()
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

static SPAWNED_SENSORS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VecSensorData<'a> {
    name: &'a str,
    value: [f32; 7],
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SensorData<'a> {
    name: &'a str,
    value: f32,
}

#[tauri::command]
async fn water_temperature_sensor(app: AppHandle) {
    let sensor_name = "water_temperature_sensor";
    let mut spawned = SPAWNED_SENSORS.lock().unwrap();

    if spawned.contains(sensor_name) {
        log::info!("Sensor '{sensor_name}' is already spawned.");
        return;
    }

    spawned.insert(sensor_name.to_string());

    tauri::async_runtime::spawn(async move {
        log::info!("Spawning sensor '{sensor_name}'...");

        let mut interval = interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            let mut rng = StdRng::from_os_rng();
            let mut air_temp_series: [f32; 7] = [0.0; 7];

            for air_temp_metric in &mut air_temp_series {
                *air_temp_metric = rng.random_range(8.02..80.0);
            }

            // TODO: retrieve the data from GPIO
            let sensor_data = VecSensorData {
                name: "Water Temperature (C°)",
                value: air_temp_series,
            };

            log::debug!("{sensor_data:#?}");

            app.emit("water_temperature_sensor", &sensor_data).unwrap();
        }
    });
}

#[tauri::command]
async fn air_temperature_sensor(app: AppHandle) {
    let sensor_name = "air_temperature_sensor";
    let mut spawned = SPAWNED_SENSORS.lock().unwrap();

    if spawned.contains(sensor_name) {
        log::info!("Sensor '{sensor_name}' is already spawned.");
        return;
    }

    spawned.insert(sensor_name.to_string());

    tauri::async_runtime::spawn(async move {
        log::info!("Spawning sensor '{sensor_name}'...");

        let mut interval = interval(Duration::from_secs(5));
        loop {
            interval.tick().await;

            let mut rng = StdRng::from_os_rng();
            let water_temp_series = rng.random_range(8.02..80.0);

            // TODO: retrieve the data from GPIO
            let sensor_data = SensorData {
                name: "Air Temperature (C°)",
                value: water_temp_series,
            };

            log::debug!("{sensor_data:#?}");

            app.emit("air_temperature_sensor", &sensor_data).unwrap();
        }
    });
}
