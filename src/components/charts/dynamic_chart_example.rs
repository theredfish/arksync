use charming::{
    component::{Axis, Title},
    element::{AxisType, Color, TextStyle},
    series::Line,
    Animation, Chart, ChartResize, Easing, Echarts, WasmRenderer,
};
use leptos::{html::Div, prelude::*};
use leptos_use::{use_element_size, use_interval_fn, utils::Pausable};
use std::cell::RefCell;
use std::rc::Rc;

#[component]
pub fn DynamicChartExample() -> impl IntoView {
    let chart_container = NodeRef::<Div>::new();
    let chart_node = NodeRef::<Div>::new();
    let chart_container_size = use_element_size(chart_container);
    let (chart_container_w, chart_container_h) =
        (chart_container_size.width, chart_container_size.height);

    let serie = RwSignal::new(vec![150, 230, 224, 218, 135, 147, 260]);
    let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::new(RefCell::new(None));

    let render_responsive_chart = move |width: f64, height: f64, serie: Vec<i32>| {
        let chart_instance: Rc<RefCell<Option<Echarts>>> = Rc::clone(&chart_instance);
        let mut chart_ref = chart_instance.borrow_mut();
        let width = if width == 0.0 { 300 } else { width as u32 };
        let height = if height == 0.0 { 150 } else { height as u32 };

        let chart_config = Chart::new()
            .title(
                Title::new()
                    .text("Dynamic and responsive chart".to_string())
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
            let renderer = WasmRenderer::new(width, height);
            let echarts = renderer.render("chart", &chart_config).unwrap();
            *chart_ref = Some(echarts);
        }
    };

    // Auto-rotate data
    let Pausable {
        pause,
        resume,
        is_active: _,
    } = use_interval_fn(
        move || {
            serie.update(|d| d.rotate_right(1));
        },
        1000,
    );

    Effect::watch(
        move || {
            (
                chart_container_w.get(),
                chart_container_h.get(),
                serie.get(),
            )
        },
        move |(width, height, serie): &(f64, f64, Vec<i32>), _prev, _| {
            render_responsive_chart(*width, *height, serie.clone());
        },
        false,
    );

    view! {
        <div node_ref=chart_container class="w-1/2 h-1/3">
            <div node_ref=chart_node id="chart"></div>
            <button on:click=move |_| pause()>"Pause"</button>
            <button on:click=move |_| resume()>"Resume"</button>
        </div>
    }
}
