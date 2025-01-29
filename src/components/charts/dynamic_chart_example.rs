use charming::{
    component::{Axis, Title},
    element::{AxisType, Color, TextStyle},
    series::Line,
    Animation, Chart, ChartResize, Easing, Echarts, WasmRenderer,
};
use leptos::{html::Div, prelude::*};
use leptos_use::{use_element_size, use_interval_fn, utils::Pausable, UseElementSizeReturn};
use std::cell::RefCell;
use std::rc::Rc;

#[component]
pub fn DynamicChartExample() -> impl IntoView {
    let chart_container = NodeRef::<Div>::new();
    let chart_node = NodeRef::<Div>::new();
    let chart_container_size = use_element_size(chart_container);
    let (chart_container_w, chart_container_h) =
        (chart_container_size.width, chart_container_size.height);

    let data = RwSignal::new(vec![150, 230, 224, 218, 135, 147, 260]);
    let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::new(RefCell::new(None));

    let update_chart = {
        let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::clone(&chart_instance);

        move || {
            let local_data = move || data.get();
            let width = move || chart_container_w.get() as u32;
            let height = move || {
                if chart_container_h.get() == 0.0 {
                    400
                } else {
                    chart_container_h.get() as u32
                }
            };

            let chart = Chart::new()
                .title(
                    Title::new()
                        .text(format!(
                            "Demo: Leptos + Charming ({},{})",
                            width(),
                            height()
                        ))
                        .text_style(TextStyle::new().color(Color::Value("#39344a".to_string()))),
                )
                .series(Line::new().data(local_data()))
                .x_axis(
                    Axis::new()
                        .type_(AxisType::Category)
                        .data(vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]),
                )
                .y_axis(Axis::new().type_(AxisType::Value));

            let renderer = WasmRenderer::new(width(), height());

            let mut chart_ref = chart_instance.borrow_mut();
            if let Some(echarts) = chart_ref.as_ref() {
                // Resize if chart exists
                WasmRenderer::resize_chart(
                    echarts,
                    ChartResize {
                        width: width(),
                        height: height(),
                        silent: true,
                        animation: Some(Animation {
                            duration: 100,
                            easing: Some(Easing::ElasticIn),
                        }),
                    },
                );
            } else {
                // Create new chart and store instance
                let echarts = renderer.render("chart", &chart).unwrap();
                *chart_ref = Some(echarts);
            }
        }
    };

    Effect::watch(
        move || chart_node.get(), // Dependency function
        {
            let update_chart = update_chart.clone();

            move |node: &Option<_>, _, _| {
                if node.is_some() {
                    update_chart();
                }
            }
        },
        true, // Run immediately
    );

    Effect::watch(
        move || (chart_container_w.get(), chart_container_h.get()),
        move |_new, _prev, param| {
            if let Some(()) = param {
                update_chart();
            }
        },
        false, // `immediate` flag
    );

    // Auto-rotate data
    // let Pausable {
    //     pause,
    //     resume,
    //     is_active: _,
    // } = use_interval_fn(
    //     move || {
    //         data.update(|d| d.rotate_right(1));
    //         update_chart();
    //     },
    //     1000,
    // );

    view! {
        <div class="flex flex-col">
            <div node_ref=chart_container class="w-1/2 h-1/3">
                <div node_ref=chart_node id="chart"></div>
            </div>
            // <button on:click=move |_| pause()>"Pause"</button>
            // <button on:click=move |_| resume()>"Resume"</button>
        </div>
    }
}
