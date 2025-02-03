use crate::components::heroicons::PresentationChartBarIcon;
use leptos::prelude::*;
use leptos::IntoView;
use leptos_router::components::A;

#[component]
pub fn Sidebar(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <div class={class}>
            <ul class="space-y-3">
                <li><A href="/">"ArkSync"</A></li>
                <li><A href="dashboard">
                    <span class="inline-flex items-center">
                        <PresentationChartBarIcon class="h-6 w-6 mr-1" />
                        "Dashboard"
                    </span>
                </A></li>
            </ul>
        </div>
    }
}
