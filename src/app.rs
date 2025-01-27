use crate::components::charts::DynamicChartExample;
use crate::components::sidebar::Sidebar;
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="flex w-full h-full">
            <div class="w-1/2">
                <DynamicChartExample />
            </div>
            <Sidebar class="w-1/2 bg-gray-900 p-20" />
        </div>
    }
}
