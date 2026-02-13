use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub enum SensorName {
    #[default]
    Unnamed,
    Named(String),
}

#[derive(Default, Debug, Clone, Copy)]
pub enum SensorState {
    Active,
    Degraded,
    #[default]
    Initializing,
    Unreachable,
}

pub struct SensorData {
    pub firmware: f64,
    pub name: SensorName,
    pub state: SensorState,
    pub last_activity: DateTime<Utc>,
}
