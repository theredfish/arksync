use leptos::prelude::*;

#[component]
pub fn Grid(children: Children) -> impl IntoView {
    view! {
        <div class="h-full grid grid-cols-12 grid-rows-[repeat(auto-fill,minmax(50px,auto))] gap-4">
            { children() }
        </div>
    }
}
