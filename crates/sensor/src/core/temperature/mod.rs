// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod dynamic;
mod dynamic_range;
pub mod unit;

pub use dynamic::*;
pub use dynamic_range::*;
pub use unit::TemperatureUnit;

use std::{cmp::Ordering, marker::PhantomData};

pub type Celsius = Temperature<unit::CelsiusUnit>;
pub type Kelvin = Temperature<unit::KelvinUnit>;
pub type Fahrenheit = Temperature<unit::FahrenheitUnit>;

#[derive(Debug)]
pub struct Temperature<Unit: TemperatureUnit>(f32, PhantomData<Unit>);

impl<Unit: TemperatureUnit> Clone for Temperature<Unit> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Unit: TemperatureUnit> Copy for Temperature<Unit> {}

impl<Unit1, Unit2> PartialEq<Temperature<Unit2>> for Temperature<Unit1>
where
    Unit1: TemperatureUnit,
    Unit2: TemperatureUnit,
{
    fn eq(&self, other: &Temperature<Unit2>) -> bool {
        // avoid precision loss, although partialeq on floats is probably a pretty bad idea anyways
        if const { Unit2::SCALING_FACTOR > Unit1::SCALING_FACTOR } {
            other.convert::<Unit1>().0 == self.0
        } else {
            self.convert::<Unit2>().0 == other.0
        }
    }
}

impl<Unit1, Unit2> PartialOrd<Temperature<Unit2>> for Temperature<Unit1>
where
    Unit1: TemperatureUnit,
    Unit2: TemperatureUnit,
{
    fn partial_cmp(&self, other: &Temperature<Unit2>) -> Option<std::cmp::Ordering> {
        // avoid precision loss
        if const { Unit2::SCALING_FACTOR > Unit1::SCALING_FACTOR } {
            other
                .convert::<Unit1>()
                .0
                .partial_cmp(&self.0)
                .map(Ordering::reverse)
        } else {
            self.convert::<Unit2>().0.partial_cmp(&other.0)
        }
    }
}

impl<Unit> Temperature<Unit>
where
    Unit: TemperatureUnit,
{
    pub fn new(value: f32) -> Self {
        Self(value, PhantomData)
    }

    pub fn convert<TargetUnit: TemperatureUnit>(self) -> Temperature<TargetUnit> {
        // this is essentially a f(g^-1(x)) composition in normal form. The factors get calculated at compile time, eliminating overhead.
        let result = self.0 * const { TargetUnit::SCALING_FACTOR / Unit::SCALING_FACTOR }
            + const {
                TargetUnit::ZERO_OFFSET
                    - Unit::ZERO_OFFSET * (TargetUnit::SCALING_FACTOR / Unit::SCALING_FACTOR)
            };

        Temperature(result, PhantomData)
    }
}

#[cfg(test)]
mod test {
    // TODO: implement test cases
}
