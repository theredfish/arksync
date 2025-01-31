use leptos::prelude::*;
use leptos::IntoView;
use leptos_router::components::A;

#[component]
pub fn Sidebar(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <div class={class}>
            <ul>
                <li><A href="/">"ArkSync"</A></li>
                <li><A href="dashboard">"Dashboard"</A></li>
            </ul>
        </div>
    }
}
