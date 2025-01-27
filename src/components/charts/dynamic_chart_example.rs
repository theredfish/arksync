use charming::{
    component::{Axis, Title},
    element::AxisType,
    series::Line,
    Animation, Chart, ChartResize, Easing, WasmRenderer,
};
use leptos::{html::Div, prelude::*};
use leptos_use::utils::Pausable;
use leptos_use::{use_interval_fn, use_resize_observer};

#[component]
pub fn DynamicChartExample() -> impl IntoView {
    let chartNode = NodeRef::<Div>::new();
    let (size, set_size) = signal((600_f64, 400_f64));

    use_resize_observer(chartNode, move |entries, _observer| {
        let rect = entries[0].content_rect();
        set_size.set((rect.width(), rect.height()));
    });

    // Chart
    let data = RwSignal::new(vec![150, 230, 224, 218, 135, 147, 260]);
    let action = Action::new(move |_input: &()| {
        let local = data.get();
        async move {
            let chart = Chart::new()
                .title(Title::new().text("Demo: Leptos + Charming"))
                .x_axis(
                    Axis::new()
                        .type_(AxisType::Category)
                        .data(vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]),
                )
                .y_axis(Axis::new().type_(AxisType::Value))
                .series(Line::new().data(local));

            let (width, height) = size.get();
            let renderer = WasmRenderer::new(width as u32, height as u32);
            let echarts = renderer.render("chart", &chart).unwrap();
            // let chart_size: ChartResize =
            WasmRenderer::resize_chart(
                &echarts,
                ChartResize {
                    width: width as u32,
                    height: height as u32,
                    silent: true,
                    animation: Some(Animation {
                        duration: 250,
                        easing: Some(Easing::Linear),
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
            <div class="flex-grow border-2 border-red-700 w-1/2 p-20">
                <div node_ref=chartNode id="chart"></div>
            </div>
            <div>
                { move || format!("{},{}", size.get().0, size.get().1) }
            </div>
            <button on:click=move |_| pause()>"Pause"</button>
            <button on:click=move |_| resume()>"Resume"</button>
        </div>
    }
}
