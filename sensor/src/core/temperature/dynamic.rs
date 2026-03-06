use std::cmp::Ordering;

use crate::core::temperature::{Celsius, Fahrenheit, Kelvin, Temperature, TemperatureUnit};

// A temperatures with a unknown unit at compile time
//
// Can be either `Kelvin`, `Celsius` or `Fahrenheit`
//
// Try to use `Kelvin`, `Celsius` or `Fahrenheit` instead whenever possible.
#[derive(Debug, Clone, Copy)]
pub enum DynamicTemperature {
    Kelvin(Kelvin),
    Celsius(Celsius),
    Fahrenheit(Fahrenheit),
}

impl From<Kelvin> for DynamicTemperature {
    fn from(value: Kelvin) -> Self {
        Self::Kelvin(value)
    }
}
impl From<Celsius> for DynamicTemperature {
    fn from(value: Celsius) -> Self {
        Self::Celsius(value)
    }
}
impl From<Fahrenheit> for DynamicTemperature {
    fn from(value: Fahrenheit) -> Self {
        Self::Fahrenheit(value)
    }
}

impl<Unit> PartialEq<Temperature<Unit>> for DynamicTemperature
where
    Unit: TemperatureUnit,
{
    fn eq(&self, other: &Temperature<Unit>) -> bool {
        match self {
            Self::Kelvin(temperature) => temperature.eq(other),
            Self::Celsius(temperature) => temperature.eq(other),
            Self::Fahrenheit(temperature) => temperature.eq(other),
        }
    }
}

impl<Unit> PartialEq<DynamicTemperature> for Temperature<Unit>
where
    Unit: TemperatureUnit,
{
    fn eq(&self, other: &DynamicTemperature) -> bool {
        other == self
    }
}

impl PartialEq for DynamicTemperature {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Kelvin(temperature) => temperature.eq(other),
            Self::Celsius(temperature) => temperature.eq(other),
            Self::Fahrenheit(temperature) => temperature.eq(other),
        }
    }
}

impl<Unit> PartialOrd<Temperature<Unit>> for DynamicTemperature
where
    Unit: TemperatureUnit,
{
    fn partial_cmp(&self, other: &Temperature<Unit>) -> Option<std::cmp::Ordering> {
        match self {
            Self::Kelvin(temperature) => temperature.partial_cmp(other),
            Self::Celsius(temperature) => temperature.partial_cmp(other),
            Self::Fahrenheit(temperature) => temperature.partial_cmp(other),
        }
    }
}

impl<Unit> PartialOrd<DynamicTemperature> for Temperature<Unit>
where
    Unit: TemperatureUnit,
{
    fn partial_cmp(&self, other: &DynamicTemperature) -> Option<std::cmp::Ordering> {
        other.partial_cmp(self).map(Ordering::reverse)
    }
}

impl PartialOrd for DynamicTemperature {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            Self::Kelvin(temperature) => temperature.partial_cmp(other),
            Self::Celsius(temperature) => temperature.partial_cmp(other),
            Self::Fahrenheit(temperature) => temperature.partial_cmp(other),
        }
    }
}

impl DynamicTemperature {
    pub fn kelvin(value: f32) -> Self {
        Self::Kelvin(Temperature::new(value))
    }

    pub fn celsius(value: f32) -> Self {
        Self::Celsius(Temperature::new(value))
    }

    pub fn fahrenheit(value: f32) -> Self {
        Self::Fahrenheit(Temperature::new(value))
    }

    pub fn convert<Unit: TemperatureUnit>(self) -> Temperature<Unit> {
        match self {
            Self::Kelvin(val) => val.convert(),
            Self::Celsius(val) => val.convert(),
            Self::Fahrenheit(val) => val.convert(),
        }
    }
}

#[cfg(test)]
mod test {
    // TODO: implement test cases
}
