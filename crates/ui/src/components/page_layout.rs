use crate::components::page_title::PageTitle;
use leptos::children::ViewFn;
use leptos::prelude::*;
use leptos::IntoView;

#[component]
pub fn PageLayout(
    #[prop(into)] eyebrow: String,
    #[prop(into)] title: String,
    children: Children,
    #[prop(optional, into)] actions: ViewFn,
) -> impl IntoView {
    view! {
        <div class="flex h-full flex-col">
            <header class="flex items-center justify-between gap-4 px-8 pb-3 pt-8">
                <PageTitle eyebrow=eyebrow title=title />
                {actions.run()}
            </header>

            <div class="relative flex-1 overflow-auto px-8 pb-8 pt-3">
                {children()}
            </div>
        </div>
    }
}
