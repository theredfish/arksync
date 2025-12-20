use charming::{
    component::{Axis, Title},
    element::{AxisType, Color, Easing, TextStyle},
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

use crate::theme::ArkSyncTheme;

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SensorData {
    name: String,
    value: [f32; 7],
}

#[component]
pub fn WaterTemperatureChart(#[prop(optional)] theme: Option<ArkSyncTheme>) -> impl IntoView {
    let chart_container = NodeRef::<Div>::new();
    let chart_node = NodeRef::<Div>::new();
    let chart_container_size = use_element_size(chart_container);
    let (chart_container_w, chart_container_h) =
        (chart_container_size.width, chart_container_size.height);

    let sensor_values = RwSignal::new(vec![0.0; 7]);
    let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::new(RefCell::new(None));

    let render_responsive_chart = move |width: f64, height: f64, serie: Vec<f32>| {
        let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::clone(&chart_instance);
        let mut chart_ref = chart_instance.borrow_mut();
        let width = if width == 0.0 { 300 } else { width as u32 };
        let height = if height == 0.0 { 150 } else { height as u32 };

        let chart_config = Chart::new()
            .title(
                Title::new()
                    .text("Water Temperature (C°)".to_string())
                    .text_style(TextStyle::new().color(Color::Value("#39344a".to_string()))),
            )
            .series(Line::new().data(serie))
            .x_axis(
                Axis::new()
                    .type_(AxisType::Category)
                    .data(vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]),
            )
            .y_axis(Axis::new().type_(AxisType::Value));

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
            let theme = theme.unwrap_or_default().as_wrapper().charming_theme;
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
                sensor_values.set(sensor_data.payload.value.to_vec());
            }
        });
    });

    Effect::watch(
        move || {
            (
                chart_container_w.get(),
                chart_container_h.get(),
                sensor_values.get(),
            )
        },
        move |(width, height, sensor_values): &(f64, f64, Vec<f32>), _prev, _| {
            render_responsive_chart(*width, *height, sensor_values.to_vec());
        },
        false,
    );

    view! {
        <div node_ref=chart_container class="w-full h-full">
            <div node_ref=chart_node id="water-temparature-gauge"></div>
        </div>
    }
}
