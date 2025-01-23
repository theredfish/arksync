use charming::{
    component::{Axis, Title},
    element::AxisType,
    series::Line,
    Chart, WasmRenderer,
};
use leptos::prelude::*;
use leptos_use::use_interval_fn;
use leptos_use::utils::Pausable;

#[component]
pub fn DynamicChartExample() -> impl IntoView {
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

            let renderer = WasmRenderer::new(600, 400);
            renderer.render("chart", &chart).unwrap();
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
        <div>
            <div id="chart"></div>
            <button on:click=move |_| pause()>"Pause"</button>
            <button on:click=move |_| resume()>"Resume"</button>
        </div>
    }
}
