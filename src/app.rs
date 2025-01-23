use crate::components::charts::DynamicChartExample;
use crate::components::sidebar::Sidebar;
use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div>
            <Sidebar class="bg-gray-900 p-20" />
            <DynamicChartExample />
        </div>
    }
}
