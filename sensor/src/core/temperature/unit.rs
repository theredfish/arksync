pub trait TemperatureUnit {
    const KELVIN_OFFSET: f32;
    const KELVIN_FACTOR: f32;
}

#[derive(Debug)]
pub struct Kelvin;
impl TemperatureUnit for Kelvin {
    const KELVIN_OFFSET: f32 = 0.0;
    const KELVIN_FACTOR: f32 = 1.0;
}

#[derive(Debug)]
pub struct Celsius;
impl TemperatureUnit for Celsius {
    const KELVIN_OFFSET: f32 = -273.15;
    const KELVIN_FACTOR: f32 = 1.0;
}
#[derive(Debug)]
pub struct Fahrenheit;
impl TemperatureUnit for Fahrenheit {
    const KELVIN_OFFSET: f32 = -459.67;
    const KELVIN_FACTOR: f32 = 1.8;
}
