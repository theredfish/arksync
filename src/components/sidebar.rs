use leptos::prelude::*;
use leptos::IntoView;

#[component]
pub fn Sidebar(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <div class={class}>
            <p>"Sidebar"</p>
        </div>
    }
}
