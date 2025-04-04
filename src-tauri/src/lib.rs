use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_log::{Builder as TauriLog, Target, TargetKind};
use tokio::time::{interval, Duration};

pub fn run() {
    tauri::Builder::default()
        .plugin(
            TauriLog::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::Stderr),
                    Target::new(TargetKind::Webview),
                ])
                .level(log::LevelFilter::Debug)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            // Directly test logging here
            println!("✅ println! Logging from setup()");
            log::debug!("✅ log::debug! Logging from setup()");
            log::info!("ℹ️ info log from setup()");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![air_temperature_sensor])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SensorData<'a> {
    name: &'a str,
    value: [f32; 7],
}

#[tauri::command]
async fn air_temperature_sensor(app: AppHandle) {
    log::debug!("air_temperature_sensor");
    log::info!("air_temperature_sensor");
    tauri::async_runtime::spawn(async move {
        let mut interval = interval(Duration::from_secs(5));
        loop {
            let mut rng = StdRng::from_os_rng();
            interval.tick().await;
            let mut air_temp_series: [f32; 7] = [0.0; 7];

            for i in 0..7 {
                air_temp_series[i] = rng.random_range(8.02..80.);
            }

            // TODO: retrieve the data from GPIO
            let sensor_data = SensorData {
                name: "Air Temperature (C°)",
                value: air_temp_series,
            };

            log::debug!("{sensor_data:#?}");

            app.emit("sensor-data", &sensor_data).unwrap();
        }
    });
}
