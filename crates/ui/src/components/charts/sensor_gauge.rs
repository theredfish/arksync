// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use leptos::prelude::*;

use super::{
    echart::{ChartKind, EChart},
    DashboardMetric,
};
use crate::data_sources::SensorSnapshot;

#[component]
pub fn SensorGauge(
    dark_theme: RwSignal<bool>,
    data: RwSignal<Option<SensorSnapshot>>,
    metric: DashboardMetric,
) -> impl IntoView {
    view! { <EChart dark_theme data metric kind=ChartKind::Gauge /> }
}
