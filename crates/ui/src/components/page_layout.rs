// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::page_title::PageTitle;
use leptos::children::ViewFn;
use leptos::prelude::*;
use leptos::IntoView;

#[component]
pub fn PageLayout(
    #[prop(into)] eyebrow: String,
    #[prop(into)] title: String,
    children: Children,
    #[prop(optional, into)] actions: Option<ViewFn>,
) -> impl IntoView {
    view! {
        <div class="flex h-full flex-col">
            <header class="flex shrink-0 items-center justify-between gap-4 border-b border-[var(--arksync-panel-border)] bg-[var(--arksync-app-bg)] px-8 py-4">
                <PageTitle eyebrow=eyebrow title=title />
                <div class="flex items-center gap-2">
                    {move || actions.clone().map(|actions| actions.run())}
                </div>
            </header>

            <div class="relative flex-1 overflow-y-auto overflow-x-hidden px-8 pb-8 pt-5">
                {children()}
            </div>
        </div>
    }
}
