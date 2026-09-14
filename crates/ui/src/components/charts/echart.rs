// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use leptos::{html::Div, prelude::*};
use serde::Serialize;
use wasm_bindgen::{prelude::*, JsCast};

use super::DashboardMetric;
use crate::{data_sources::SensorSnapshot, theme::ArkSyncChartColors};

#[wasm_bindgen]
extern "C" {
    // The chart bundle is loaded before the WASM module in index.html. Keeping
    // this boundary deliberately small lets the Rust component own lifecycle
    // and data flow while JavaScript owns the imperative ECharts instance.
    #[wasm_bindgen(js_namespace = ArkSyncCharts, catch)]
    fn mount(host: &web_sys::HtmlElement, kind: &str, config: &JsValue) -> Result<(), JsValue>;
    #[wasm_bindgen(js_namespace = ArkSyncCharts, catch)]
    fn configure(host: &web_sys::HtmlElement, config: &JsValue) -> Result<(), JsValue>;
    #[wasm_bindgen(js_namespace = ArkSyncCharts, catch)]
    fn update(host: &web_sys::HtmlElement, snapshot: &JsValue) -> Result<(), JsValue>;
    #[wasm_bindgen(js_namespace = ArkSyncCharts, js_name = resetZoom)]
    fn reset_zoom(host: &web_sys::HtmlElement);
    #[wasm_bindgen(js_namespace = ArkSyncCharts)]
    fn dispose(host: &web_sys::HtmlElement);
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum ChartKind {
    Gauge,
    History,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChartConfig {
    label: &'static str,
    unit: &'static str,
    precision: u32,
    minimum: f32,
    maximum: f32,
    history_minimum: f32,
    history_maximum: f32,
    dark: bool,
    background: &'static str,
    text: &'static str,
    muted_text: &'static str,
    grid: &'static str,
    color: &'static str,
}

fn configuration(metric: DashboardMetric, kind: ChartKind, dark: bool) -> JsValue {
    let palette = ArkSyncChartColors::from_dark_theme(dark);
    let colors = metric.colors(dark);
    let (minimum, maximum) = metric.gauge_range();
    let (history_minimum, history_maximum) = metric.history_range();
    serde_wasm_bindgen::to_value(&ChartConfig {
        label: metric.label(),
        unit: metric.unit(),
        precision: metric.precision(),
        minimum,
        maximum,
        history_minimum,
        history_maximum,
        dark,
        background: palette.background,
        text: palette.text,
        muted_text: palette.muted_text,
        grid: palette.grid,
        color: if kind == ChartKind::Gauge {
            colors.gauge
        } else {
            colors.line
        },
    })
    .expect("Chart configuration is serializable")
}

#[derive(Serialize)]
struct HistoryData<'a> {
    timestamps: &'a [f64],
    values: &'a [f32],
}

#[component]
pub(super) fn EChart(
    dark_theme: RwSignal<bool>,
    data: RwSignal<Option<SensorSnapshot>>,
    metric: DashboardMetric,
    kind: ChartKind,
) -> impl IntoView {
    let node = NodeRef::<Div>::new();
    let host = StoredValue::new_local(None::<web_sys::HtmlElement>);
    let ready = RwSignal::new(false);
    let failed = RwSignal::new(false);

    // Mounting does not subscribe to data, theme or grid dimensions. Those
    // update through the two effects below without recreating an ECharts
    // instance, which preserves a history chart's current time selection.
    Effect::new(move |_| {
        let Some(element) = node.get() else { return };
        let element: web_sys::HtmlElement = element.unchecked_ref::<web_sys::HtmlElement>().clone();
        let chart_type = if kind == ChartKind::Gauge {
            "gauge"
        } else {
            "history"
        };
        let config = configuration(metric, kind, dark_theme.get_untracked());
        match mount(&element, chart_type, &config) {
            Ok(()) => {
                host.set_value(Some(element));
                ready.set(true);
            }
            Err(error) => {
                leptos::logging::error!("Could not initialize ECharts: {error:?}");
                failed.set(true);
            }
        }
    });

    Effect::new(move |_| {
        if !ready.get() {
            return;
        }
        let config = configuration(metric, kind, dark_theme.get());
        host.with_value(|host| {
            if let Some(host) = host {
                if let Err(error) = configure(host, &config) {
                    leptos::logging::error!("Could not configure ECharts: {error:?}");
                }
            }
        });
    });

    Effect::new(move |_| {
        if !ready.get() {
            return;
        }
        data.with(|snapshot| {
            let payload = match (kind, snapshot) {
                (ChartKind::Gauge, Some(snapshot)) => JsValue::from_f64(snapshot.latest as f64),
                (ChartKind::History, Some(snapshot)) => {
                    serde_wasm_bindgen::to_value(&HistoryData {
                        timestamps: &snapshot.timestamps,
                        values: &snapshot.values,
                    })
                    .expect("History data is serializable")
                }
                (_, None) => JsValue::NULL,
            };
            host.with_value(|host| {
                if let Some(host) = host {
                    if let Err(error) = update(host, &payload) {
                        leptos::logging::error!("Could not update ECharts: {error:?}");
                    }
                }
            });
        });
    });

    on_cleanup(move || {
        if let Some(Some(host)) = host.try_get_value() {
            dispose(&host);
        }
    });

    let id = if kind == ChartKind::Gauge {
        metric.gauge_id()
    } else {
        metric.history_id()
    };
    view! {
        <div class="relative h-full w-full cursor-default">
            <div
                node_ref=node
                id=id
                class="absolute inset-0"
                data-chart-kind=if kind == ChartKind::Gauge { "gauge" } else { "history" }
                role="img"
                aria-label=move || format!("{} {}", metric.label(), if kind == ChartKind::Gauge { "gauge" } else { "history" })
            ></div>
            {(kind == ChartKind::History).then(|| view! {
                <button
                    type="button"
                    class="absolute right-3 top-1 rounded border border-[var(--arksync-panel-border)] px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-[var(--arksync-text-muted)] hover:text-[var(--arksync-text)]"
                    title="Reset the selected interval and follow incoming measurements. Zoom with the slider, pinch, or Ctrl + mouse wheel."
                    aria-label=format!("Return {} history to live view", metric.label())
                    on:click=move |_| host.with_value(|host| { if let Some(host) = host { reset_zoom(host); } })
                >"Live"</button>
            })}
            <Show when=move || failed.get() || data.with(|snapshot| snapshot.is_none())>
                <div class="arksync-no-data pointer-events-none absolute inset-0 flex items-center justify-center font-mono text-xs uppercase tracking-[0.22em] text-[var(--arksync-text-muted)]">
                    {move || if failed.get() { "Chart unavailable" } else { "No Data" }}
                </div>
            </Show>
        </div>
    }
}
