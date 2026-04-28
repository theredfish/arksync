use arksync_sensor::services::SensorService;

#[tokio::main]
async fn main() {
    println!("Starting ArkSync Sensor Service...");
    SensorService::new().run().await;
}
