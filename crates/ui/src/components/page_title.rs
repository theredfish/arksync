use leptos::prelude::*;
use leptos::IntoView;

#[component]
pub fn PageTitle(#[prop(into)] eyebrow: String, #[prop(into)] title: String) -> impl IntoView {
    view! {
        <div>
            <div class="font-mono text-[10px] uppercase tracking-[0.24em] text-sk-carbon-475">{eyebrow}</div>
            <h1 class="mt-2 text-4xl font-semibold text-sk-carbon-50">{title}</h1>
        </div>
    }
}
