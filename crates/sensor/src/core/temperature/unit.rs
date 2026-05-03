// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// A temperature unit, defined by its zero offset and scaling factor relative to the absolute temperature (Kelvin).
pub trait TemperatureUnit {
    const ZERO_OFFSET: f32;
    const SCALING_FACTOR: f32;
}

pub enum Unit {
    Celsius(CelsiusUnit),
    Fahrenheit(FahrenheitUnit),
    Kelvin(KelvinUnit),
}

#[derive(Debug)]
pub struct KelvinUnit;
impl TemperatureUnit for KelvinUnit {
    const ZERO_OFFSET: f32 = 0.0;
    const SCALING_FACTOR: f32 = 1.0;
}

#[derive(Debug)]
pub struct CelsiusUnit;
impl TemperatureUnit for CelsiusUnit {
    const ZERO_OFFSET: f32 = -273.15;
    const SCALING_FACTOR: f32 = 1.0;
}

#[derive(Debug)]
pub struct FahrenheitUnit;
impl TemperatureUnit for FahrenheitUnit {
    const ZERO_OFFSET: f32 = -459.67;
    const SCALING_FACTOR: f32 = 1.8;
}
