// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod echart;
mod sensor_gauge;
mod sensor_history_chart;

pub use self::sensor_gauge::SensorGauge;
pub use self::sensor_history_chart::SensorHistoryChart;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DashboardMetric {
    AirTemperature,
    WaterTemperature,
    WaterPh,
}

impl DashboardMetric {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::AirTemperature => "Air temperature",
            Self::WaterTemperature => "Water temperature",
            Self::WaterPh => "Water pH",
        }
    }

    pub(crate) fn unit(self) -> &'static str {
        match self {
            Self::AirTemperature | Self::WaterTemperature => "°C",
            Self::WaterPh => "pH",
        }
    }

    pub(crate) fn precision(self) -> u32 {
        match self {
            Self::AirTemperature | Self::WaterTemperature => 1,
            Self::WaterPh => 2,
        }
    }

    pub(crate) fn gauge_range(self) -> (f32, f32) {
        match self {
            Self::AirTemperature => (0.0, 50.0),
            Self::WaterTemperature => (0.0, 40.0),
            Self::WaterPh => (0.0, 14.0),
        }
    }

    pub(crate) fn history_range(self) -> (f32, f32) {
        match self {
            Self::AirTemperature => (23.0, 25.0),
            Self::WaterTemperature => (27.75, 28.25),
            Self::WaterPh => (6.77, 6.81),
        }
    }

    pub(crate) fn gauge_id(self) -> &'static str {
        match self {
            Self::AirTemperature => "air-temperature-gauge",
            Self::WaterTemperature => "water-temperature-gauge",
            Self::WaterPh => "water-ph-gauge",
        }
    }

    pub(crate) fn history_id(self) -> &'static str {
        match self {
            Self::AirTemperature => "air-temperature-history",
            Self::WaterTemperature => "water-temperature-history",
            Self::WaterPh => "water-ph-history",
        }
    }

    pub(crate) fn colors(self, dark_theme: bool) -> MetricColors {
        match (self, dark_theme) {
            (Self::AirTemperature, true) => MetricColors {
                gauge: "#2f756b",
                line: "#7ed0c2",
            },
            (Self::AirTemperature, false) => MetricColors {
                gauge: "#285f55",
                line: "#2f756b",
            },
            (Self::WaterTemperature, true) => MetricColors {
                gauge: "#3f78a8",
                line: "#7db9e8",
            },
            (Self::WaterTemperature, false) => MetricColors {
                gauge: "#32658f",
                line: "#3f78a8",
            },
            (Self::WaterPh, true) => MetricColors {
                gauge: "#a44f5b",
                line: "#ee8993",
            },
            (Self::WaterPh, false) => MetricColors {
                gauge: "#8f3f4b",
                line: "#b8515e",
            },
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct MetricColors {
    pub gauge: &'static str,
    pub line: &'static str,
}
