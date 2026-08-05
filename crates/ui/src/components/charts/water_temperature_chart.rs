// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use charming::{
    component::Axis,
    element::{
        AxisLabel, AxisLine, AxisLineStyle, AxisType, Color, Easing, ItemStyle, LineStyle,
        SplitLine,
    },
    series::Line,
    Animation, Chart, ChartResize, Echarts, WasmRenderer,
};
use futures_util::StreamExt;
use leptos::{html::Div, logging::log, prelude::*};
use leptos_use::use_element_size;
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use tauri_sys::event::listen;
use wasm_bindgen_futures::spawn_local;

use crate::theme::{ArkSyncChartColors, ArkSyncTheme};

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SensorData {
    name: String,
    labels: Vec<String>,
    value: Vec<f32>,
}

#[component]
pub fn WaterTemperatureChart(dark_theme: RwSignal<bool>) -> impl IntoView {
    let chart_container = NodeRef::<Div>::new();
    let chart_node = NodeRef::<Div>::new();
    let chart_container_size = use_element_size(chart_container);
    let (chart_container_w, chart_container_h) =
        (chart_container_size.width, chart_container_size.height);

    let sensor_values = RwSignal::new(None::<Vec<f32>>);
    let sensor_labels = RwSignal::new(None::<Vec<String>>);
    let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::new(RefCell::new(None));

    let render_responsive_chart = move |width: f64,
                                        height: f64,
                                        serie: Vec<f32>,
                                        labels: Vec<String>,
                                        dark_theme: bool| {
        let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::clone(&chart_instance);
        let mut chart_ref = chart_instance.borrow_mut();
        let width = if width == 0.0 { 300 } else { width as u32 };
        let height = if height == 0.0 { 150 } else { height as u32 };
        let chart_theme = ArkSyncChartColors::from_dark_theme(dark_theme);

        let chart_config = Chart::new()
            .background_color(Color::Value(chart_theme.background.to_string()))
            .series(
                Line::new()
                    .show_symbol(false)
                    .line_style(
                        LineStyle::new()
                            .color(Color::Value(chart_theme.line.to_string()))
                            .width(2.0),
                    )
                    .item_style(ItemStyle::new().color(Color::Value(chart_theme.line.to_string())))
                    .data(serie),
            )
            .x_axis(
                Axis::new()
                    .type_(AxisType::Category)
                    .axis_label(
                        AxisLabel::new()
                            .interval(axis_label_interval(labels.len()))
                            .color(Color::Value(chart_theme.muted_text.to_string())),
                    )
                    .axis_line(
                        AxisLine::new().line_style(
                            AxisLineStyle::new()
                                .color((1.0, Color::Value(chart_theme.grid.to_string()))),
                        ),
                    )
                    .data(labels),
            )
            .y_axis(
                Axis::new()
                    .type_(AxisType::Value)
                    .axis_label(
                        AxisLabel::new().color(Color::Value(chart_theme.muted_text.to_string())),
                    )
                    .axis_line(
                        AxisLine::new().line_style(
                            AxisLineStyle::new()
                                .color((1.0, Color::Value(chart_theme.grid.to_string()))),
                        ),
                    )
                    .split_line(SplitLine::new().line_style(
                        LineStyle::new().color(Color::Value(chart_theme.grid.to_string())),
                    )),
            );

        if let Some(echarts) = chart_ref.as_ref() {
            WasmRenderer::update(echarts, &chart_config);
            // Resize if chart exists
            WasmRenderer::resize_chart(
                echarts,
                ChartResize {
                    width,
                    height,
                    silent: true,
                    animation: Some(Animation {
                        duration: 150,
                        easing: Some(Easing::Linear),
                    }),
                },
            );
        } else {
            let theme = if dark_theme {
                ArkSyncTheme::Chalk
            } else {
                ArkSyncTheme::Walden
            }
            .as_wrapper()
            .charming_theme;
            let renderer = WasmRenderer::new(width, height).theme(theme);
            let echarts = renderer
                .render("water-temparature-gauge", &chart_config)
                .unwrap();
            *chart_ref = Some(echarts);
        }
    };

    Effect::new(move |_| {
        spawn_local(async move {
            let _ = tauri_sys::core::invoke::<()>("water_temperature_sensor", &()).await;
        });

        spawn_local(async move {
            let event_name = "water_temperature_sensor";
            let mut stream = match listen::<SensorData>(event_name).await {
                Ok(s) => s,
                Err(e) => {
                    log!("Failed to subscribe to water_temperature_sensor: {}", e);
                    return;
                }
            };

            while let Some(sensor_data) = stream.next().await {
                sensor_labels.set(Some(sensor_data.payload.labels));
                sensor_values.set(Some(sensor_data.payload.value));
            }
        });
    });

    Effect::watch(
        move || {
            (
                chart_container_w.get(),
                chart_container_h.get(),
                sensor_values.get(),
                sensor_labels.get(),
                dark_theme.get(),
            )
        },
        move |(width, height, sensor_values, sensor_labels, dark_theme): &(
            f64,
            f64,
            Option<Vec<f32>>,
            Option<Vec<String>>,
            bool,
        ),
              _prev,
              _| {
            if let (Some(sensor_values), Some(sensor_labels)) = (sensor_values, sensor_labels) {
                render_responsive_chart(
                    *width,
                    *height,
                    sensor_values.to_vec(),
                    sensor_labels.to_vec(),
                    *dark_theme,
                );
            }
        },
        false,
    );

    view! {
        <div node_ref=chart_container class="relative w-full h-full">
            <div
                node_ref=chart_node
                id="water-temparature-gauge"
                class=move || if sensor_values.get().is_some() { "h-full" } else { "hidden h-full" }
            ></div>
            {move || {
                sensor_values.get().is_none().then(|| {
                    view! {
                        <div class="arksync-no-data pointer-events-none absolute inset-0 flex items-center justify-center font-mono text-xs uppercase tracking-[0.22em] text-[var(--arksync-text-muted)]">
                            "No Data"
                        </div>
                    }
                })
            }}
        </div>
    }
}

fn axis_label_interval(label_count: usize) -> f64 {
    if label_count <= 8 {
        0.0
    } else {
        (label_count / 8) as f64
    }
}
