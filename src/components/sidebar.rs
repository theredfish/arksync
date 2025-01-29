use leptos::prelude::*;
use leptos::IntoView;

#[component]
pub fn Sidebar(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <div class={class}>
            <h2>"ArkSync"</h2>
        </div>
    }
}
