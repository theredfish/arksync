/// A temperature unit, defined by its zero offset and scaling factor relative to the absolute temperature (Kelvin).
pub trait TemperatureUnit {
    const ZERO_OFFSET: f32;
    const SCALING_FACTOR: f32;
}

#[derive(Debug)]
pub struct Kelvin;
impl TemperatureUnit for Kelvin {
    const ZERO_OFFSET: f32 = 0.0;
    const SCALING_FACTOR: f32 = 1.0;
}

#[derive(Debug)]
pub struct Celsius;
impl TemperatureUnit for Celsius {
    const ZERO_OFFSET: f32 = -273.15;
    const SCALING_FACTOR: f32 = 1.0;
}
#[derive(Debug)]
pub struct Fahrenheit;
impl TemperatureUnit for Fahrenheit {
    const ZERO_OFFSET: f32 = -459.67;
    const SCALING_FACTOR: f32 = 1.8;
}
