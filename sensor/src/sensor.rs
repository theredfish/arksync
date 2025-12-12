#[derive(Debug, Clone, Copy)]
pub enum SensorKind {
    Rtd,
}

pub enum SensorName {
    Unnamed,
    Named(String),
}

#[derive(Debug)]
pub struct Sensor {
    pub kind: SensorKind,
    pub firmware: f64,
    pub name: Option<String>,
}
