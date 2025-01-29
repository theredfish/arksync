use crate::components::charts::DynamicChartExample;
use crate::components::sidebar::Sidebar;
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="bg-darcula-black text-gray-100 flex w-screen h-screen">
            <Sidebar class="w-1/6 bg-darcula-gray p-5" />
            <div id="charts-container" class="w-full p-5">
                <DynamicChartExample />
            </div>
        </div>
    }
}
