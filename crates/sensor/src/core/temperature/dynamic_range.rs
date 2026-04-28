use std::ops::Range;

use crate::core::temperature::{Celsius, Fahrenheit, Kelvin, Temperature, TemperatureUnit};

// A range of temperatures with a unknown unit at compile time
//
// Can be either `Kelvin`, `Celsius` or `Fahrenheit`
//
// Try to use `Range<Kelvin>`, `Range<Celsius>` or `Range<Fahrenheit>` instead whenever possible.
#[derive(Debug, Clone)]
pub enum DynamicRange {
    Kelvin(Range<Kelvin>),
    Celsius(Range<Celsius>),
    Fahrenheit(Range<Fahrenheit>),
}

impl From<Range<Kelvin>> for DynamicRange {
    fn from(value: Range<Kelvin>) -> Self {
        Self::Kelvin(value)
    }
}

impl From<Range<Celsius>> for DynamicRange {
    fn from(value: Range<Celsius>) -> Self {
        Self::Celsius(value)
    }
}

impl From<Range<Fahrenheit>> for DynamicRange {
    fn from(value: Range<Fahrenheit>) -> Self {
        Self::Fahrenheit(value)
    }
}

impl DynamicRange {
    pub fn kelvin(value: Range<f32>) -> Self {
        Self::Kelvin(Range {
            start: Temperature::new(value.start),
            end: Temperature::new(value.end),
        })
    }

    pub fn celsius(value: Range<f32>) -> Self {
        Self::Celsius(Range {
            start: Temperature::new(value.start),
            end: Temperature::new(value.end),
        })
    }

    pub fn fahrenheit(value: Range<f32>) -> Self {
        Self::Fahrenheit(Range {
            start: Temperature::new(value.start),
            end: Temperature::new(value.end),
        })
    }

    pub fn convert<Unit: TemperatureUnit>(self) -> Range<Temperature<Unit>> {
        match self {
            Self::Kelvin(val) => Range {
                start: val.start.convert(),
                end: val.end.convert(),
            },
            Self::Celsius(val) => Range {
                start: val.start.convert(),
                end: val.end.convert(),
            },
            Self::Fahrenheit(val) => Range {
                start: val.start.convert(),
                end: val.end.convert(),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::core::temperature::unit::FahrenheitUnit;

    #[test]
    #[ignore = "for dev"]
    fn test_conversion() {
        println!(
            "{:?}",
            DynamicRange::celsius(-126.0..1254.0).convert::<FahrenheitUnit>()
        );
    }
}
