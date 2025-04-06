use std::{cell::RefCell, rc::Rc};

use charming::{
    element::Tooltip,
    series::{Gauge, GaugeDetail, GaugeProgress},
    Animation, Chart, ChartResize, Easing, Echarts, WasmRenderer,
};
use futures_util::StreamExt as _;
use leptos::{attr::Id, html::Div, prelude::*};
use leptos::{logging::log, IntoView};
use leptos_use::use_element_size;
use serde::Deserialize;
use tauri_sys::event::listen;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SensorData {
    name: String,
    value: f32,
}

#[component]
pub fn AirTemperatureGauge() -> impl IntoView {
    let chart_container = NodeRef::<Div>::new();
    let chart_node = NodeRef::<Div>::new();
    let chart_container_size = use_element_size(chart_container);
    let (chart_container_w, chart_container_h) =
        (chart_container_size.width, chart_container_size.height);

    let sensor_value = RwSignal::new(0.0);
    let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::new(RefCell::new(None));

    let render_responsive_chart = move |width: f64, height: f64, serie: f32| {
        let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::clone(&chart_instance);
        let mut chart_ref = chart_instance.borrow_mut();
        let width = if width == 0.0 { 300 } else { width as u32 };
        let height = if height == 0.0 { 150 } else { height as u32 };

        let chart_config = Chart::new()
            .tooltip(Tooltip::new().formatter("{a} <br/>{b} : {c}%"))
            .series(
                Gauge::new()
                    .name("Pressure")
                    .progress(GaugeProgress::new().show(true))
                    .detail(
                        GaugeDetail::new()
                            .formatter("{value}")
                            .value_animation(true),
                    )
                    .data(vec![(serie, "Water C°")]),
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
            let renderer = WasmRenderer::new(width, height);
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
            )
        },
        move |(width, height, sensor_value): &(f64, f64, f32), _prev, _| {
            render_responsive_chart(*width, *height, *sensor_value);
        },
        false,
    );

    view! {
        <div node_ref=chart_container class="w-full h-full border border-red-100">
            <div node_ref=chart_node id="air-temperature-gauge"></div>
        </div>
    }
}
