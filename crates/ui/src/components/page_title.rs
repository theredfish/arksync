// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use leptos::prelude::*;
use leptos::IntoView;

#[component]
pub fn PageTitle(#[prop(into)] eyebrow: String, #[prop(into)] title: String) -> impl IntoView {
    view! {
        <div class="flex min-w-0 items-center gap-2 font-mono text-[10px] uppercase tracking-[0.22em] text-[var(--arksync-text-muted)]">
            <span>{eyebrow}</span>
            <span class="text-[var(--arksync-panel-muted)]">"/"</span>
            <span class="truncate text-[var(--arksync-text-strong)]">{title}</span>
        </div>
    }
}
