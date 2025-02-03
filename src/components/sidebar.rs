use crate::components::heroicons::{
    CpuChip, PresentationChartBarIcon, RectangleGroup, ShieldExclamation,
};
use leptos::prelude::*;
use leptos::IntoView;
use leptos_router::components::A;

#[component]
pub fn Sidebar(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <div class={class}>
            <ul class="space-y-5">
                <li><A href="/">"ArkSync"</A></li>
                <li><A href="dashboards">
                    <span class="inline-flex items-center">
                        <PresentationChartBarIcon class="h-6 w-6 mr-1" />
                        "Dashboards"
                    </span>
                </A></li>

                <li><A href="alerts">
                    <span class="inline-flex items-center">
                        <ShieldExclamation class="h-6 w-6 mr-1" />
                        "Alerts"
                    </span>
                </A></li>

                <li><A href="sensors">
                    <span class="inline-flex items-center">
                        <CpuChip class="h-6 w-6 mr-1" />
                        "Sensors"
                    </span>
                </A></li>

                <li><A href="nodes">
                    <span class="inline-flex items-center">
                        <RectangleGroup class="h-6 w-6 mr-1" />
                        "Nodes"
                    </span>
                </A></li>
            </ul>
        </div>
    }
}
