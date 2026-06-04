// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{cell::RefCell, rc::Rc};

use charming::{
    element::{
        AxisLabel, AxisLine, AxisLineStyle, Color, Easing, ItemStyle, JsFunction, Pointer,
        SplitLine, Tooltip,
    },
    series::{Gauge, GaugeDetail, GaugeProgress, GaugeTitle},
    Animation, Chart, ChartResize, Echarts, WasmRenderer,
};
use futures_util::StreamExt as _;
use leptos::{html::Div, prelude::*};
use leptos::{logging::log, IntoView};
use leptos_use::use_element_size;
use serde::Deserialize;
use tauri_sys::event::listen;
use wasm_bindgen_futures::spawn_local;

use crate::theme::{ArkSyncChartColors, ArkSyncTheme};

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SensorData {
    name: String,
    value: f32,
}

#[component]
pub fn AirTemperatureGauge(dark_theme: RwSignal<bool>) -> impl IntoView {
    let chart_container = NodeRef::<Div>::new();
    let chart_node = NodeRef::<Div>::new();
    let chart_container_size = use_element_size(chart_container);
    let (chart_container_w, chart_container_h) =
        (chart_container_size.width, chart_container_size.height);

    let sensor_value = RwSignal::new(0.0);
    let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::new(RefCell::new(None));

    let render_responsive_chart = move |width: f64, height: f64, serie: f32, dark_theme: bool| {
        let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::clone(&chart_instance);
        let mut chart_ref = chart_instance.borrow_mut();
        let width = if width == 0.0 { 300 } else { width as u32 };
        let height = if height == 0.0 { 150 } else { height as u32 };
        let chart_theme = ArkSyncChartColors::from_dark_theme(dark_theme);

        let chart_config = Chart::new()
            .background_color(Color::Value(chart_theme.background.to_string()))
            .tooltip(Tooltip::new().formatter("{a} <br/>{b} : {c} °C"))
            .series(
                Gauge::new()
                    .name("Temperature")
                    .axis_line(
                        AxisLine::new().line_style(
                            AxisLineStyle::new()
                                .color((1.0, Color::Value(chart_theme.grid.to_string())))
                                .width(12.0),
                        ),
                    )
                    .axis_label(
                        AxisLabel::new()
                            .color(Color::Value(chart_theme.muted_text.to_string()))
                            .font_size(12.0),
                    )
                    .split_line(
                        SplitLine::new().line_style(
                            charming::element::LineStyle::new()
                                .color(Color::Value(chart_theme.grid.to_string())),
                        ),
                    )
                    .pointer(
                        Pointer::new()
                            .item_style(
                                ItemStyle::new()
                                    .color(Color::Value(chart_theme.pointer.to_string())),
                            )
                            .width(6.0),
                    )
                    .progress(GaugeProgress::new().show(true).width(12.0).item_style(
                        ItemStyle::new().color(Color::Value(chart_theme.gauge_fill.to_string())),
                    ))
                    .title(GaugeTitle::new().show(false))
                    .detail(
                        GaugeDetail::new()
                            .color(Color::Value(chart_theme.text.to_string()))
                            .font_size(30.0)
                            .formatter(JsFunction::new_with_args(
                                "value",
                                "return value.toFixed(1).replace('.', ',') + ' °C';",
                            ))
                            .value_animation(true),
                    )
                    .data(vec![(round_to_tenth(serie), "")]),
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
            log!(
                "Rendering air temperature gauge with size: {}x{}",
                width,
                height
            );

            let theme = if dark_theme {
                ArkSyncTheme::Chalk
            } else {
                ArkSyncTheme::Walden
            }
            .as_wrapper()
            .charming_theme;
            let renderer = WasmRenderer::new(width, height).theme(theme);
            let echarts = renderer
                .render("air-temperature-gauge", &chart_config)
                .unwrap();

            *chart_ref = Some(echarts);
        }
    };

    Effect::new(move |_| {
        spawn_local(async move {
            let _ = tauri_sys::core::invoke::<()>("air_temperature_sensor", &()).await;
        });

        spawn_local(async move {
            let event_name = "air_temperature_sensor";
            let mut stream = match listen::<SensorData>(event_name).await {
                Ok(s) => s,
                Err(e) => {
                    log!("Failed to subscribe to air_temperature_sensor: {}", e);
                    return;
                }
            };

            while let Some(sensor_data) = stream.next().await {
                sensor_value.set(sensor_data.payload.value);
            }
        });
    });

    Effect::watch(
        move || {
            (
                chart_container_w.get(),
                chart_container_h.get(),
                sensor_value.get(),
                dark_theme.get(),
            )
        },
        move |(width, height, sensor_value, dark_theme): &(f64, f64, f32, bool), _prev, _| {
            render_responsive_chart(*width, *height, *sensor_value, *dark_theme);
        },
        false,
    );

    view! {
        <div node_ref=chart_container class="w-full h-full">
            <div node_ref=chart_node id="air-temperature-gauge"></div>
        </div>
    }
}

fn round_to_tenth(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}
