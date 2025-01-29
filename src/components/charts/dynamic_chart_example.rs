use charming::{
    component::{Axis, Title},
    element::{AxisType, Color, TextStyle},
    series::Line,
    Animation, Chart, ChartResize, Easing, WasmRenderer,
};
use leptos::{html::Div, prelude::*};
use leptos_use::{use_element_size, use_interval_fn, utils::Pausable, UseElementSizeReturn};

#[component]
pub fn DynamicChartExample() -> impl IntoView {
    let chartContainer = NodeRef::<Div>::new();
    let chartNode: NodeRef<Div> = NodeRef::<Div>::new();
    let chartContainerSize: UseElementSizeReturn = use_element_size(chartContainer);
    let (chartContainerW, chartContainerH) = (chartContainerSize.width, chartContainerSize.height);

    // Chart
    let data = RwSignal::new(vec![150, 230, 224, 218, 135, 147, 260]);
    let action = Action::new(move |_input: &()| {
        let local = data.get();
        let chartContainerW = chartContainerW.get() as u32;
        let chartContainerH = if chartContainerH.get() == 0_f64 {
            400
        } else {
            chartContainerH.get() as u32
        };

        async move {
            let chart = Chart::new()
                .title(
                    Title::new()
                        .text(format!(
                            "Demo: Leptos + Charming {chartContainerW},{chartContainerH})"
                        ))
                        .text_style(TextStyle::new().color(Color::Value("#39344a".to_string()))),
                )
                .series(Line::new().data(local))
                .x_axis(
                    Axis::new()
                        .type_(AxisType::Category)
                        .data(vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]),
                )
                .y_axis(Axis::new().type_(AxisType::Value));

            let renderer = WasmRenderer::new(chartContainerW, chartContainerH);
            let echarts = renderer.render("chart", &chart).unwrap();

            WasmRenderer::resize_chart(
                &echarts,
                ChartResize {
                    width: chartContainerW,
                    height: chartContainerH,
                    silent: true,
                    animation: Some(Animation {
                        duration: 100,
                        easing: Some(Easing::ElasticIn),
                    }),
                },
            );
        }
    });

    let Pausable {
        pause,
        resume,
        is_active: _,
    } = use_interval_fn(
        move || {
            data.update(|d| d.rotate_right(1));
            action.dispatch(());
        },
        1000,
    );

    action.dispatch(());

    view! {
        <div class="flex flex-col">
            <div node_ref=chartContainer class="w-1/2 h-1/3">
                <div node_ref=chartNode id="chart"></div>
            </div>
            <button on:click=move |_| pause()>"Pause"</button>
            <button on:click=move |_| resume()>"Resume"</button>
        </div>
    }
}
