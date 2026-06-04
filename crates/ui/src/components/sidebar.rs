// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::heroicons::{
    CpuChipIcon, MoonIcon, PresentationChartBarIcon, RectangleGroupIcon, ShieldExclamationIcon,
    SunIcon,
};
use leptos::prelude::*;
use leptos::IntoView;
use leptos_router::components::A;

#[component]
pub fn Sidebar(#[prop(into)] class: String, dark_theme: RwSignal<bool>) -> impl IntoView {
    view! {
        <div class=format!("{class} flex flex-col")>
            <div>
                <div class="mb-7 rounded-md border border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-bg)] px-3 py-3 transition-colors">
                    <div class="font-mono text-[10px] uppercase tracking-[0.2em] text-[var(--arksync-text-muted)]">"Station"</div>
                    <A href="/" attr:class="mt-1 block truncate text-sm font-medium text-[var(--arksync-text-strong)]">"ArkSync"</A>
                </div>

                <ul class="space-y-1.5">
                    <li><A href="dashboards" attr:class="block rounded-md px-3 py-2 text-[var(--arksync-text-muted)] transition-colors hover:bg-[var(--arksync-nav-active-bg)] hover:text-[var(--arksync-nav-active-text)] aria-[current=page]:bg-[var(--arksync-nav-active-bg)] aria-[current=page]:text-[var(--arksync-nav-active-text)]">
                        <span class="inline-flex items-center">
                            <PresentationChartBarIcon class="mr-2 h-5 w-5" />
                            "Dashboards"
                        </span>
                    </A></li>

                    <li><A href="alerts" attr:class="block rounded-md px-3 py-2 text-[var(--arksync-text-muted)] transition-colors hover:bg-[var(--arksync-nav-active-bg)] hover:text-[var(--arksync-nav-active-text)] aria-[current=page]:bg-[var(--arksync-nav-active-bg)] aria-[current=page]:text-[var(--arksync-nav-active-text)]">
                        <span class="inline-flex items-center">
                            <ShieldExclamationIcon class="mr-2 h-5 w-5" />
                            "Alerts"
                        </span>
                    </A></li>

                    <li><A href="sensors" attr:class="block rounded-md px-3 py-2 text-[var(--arksync-text-muted)] transition-colors hover:bg-[var(--arksync-nav-active-bg)] hover:text-[var(--arksync-nav-active-text)] aria-[current=page]:bg-[var(--arksync-nav-active-bg)] aria-[current=page]:text-[var(--arksync-nav-active-text)]">
                        <span class="inline-flex items-center">
                            <CpuChipIcon class="mr-2 h-5 w-5" />
                            "Sensors"
                        </span>
                    </A></li>

                    <li><A href="nodes" attr:class="block rounded-md px-3 py-2 text-[var(--arksync-text-muted)] transition-colors hover:bg-[var(--arksync-nav-active-bg)] hover:text-[var(--arksync-nav-active-text)] aria-[current=page]:bg-[var(--arksync-nav-active-bg)] aria-[current=page]:text-[var(--arksync-nav-active-text)]">
                        <span class="inline-flex items-center">
                            <RectangleGroupIcon class="mr-2 h-5 w-5" />
                            "Nodes"
                        </span>
                    </A></li>
                </ul>
            </div>

            <div class="mt-auto border-t border-[var(--arksync-panel-border)] pt-4">
                <ThemeSwitch dark_theme=dark_theme />
            </div>
        </div>
    }
}

#[component]
fn ThemeSwitch(dark_theme: RwSignal<bool>) -> impl IntoView {
    let label = move || {
        if dark_theme.get() {
            "Light theme"
        } else {
            "Dark theme"
        }
    };

    view! {
        <button
            type="button"
            aria-label=label
            title=label
            on:click=move |_| dark_theme.update(|enabled| *enabled = !*enabled)
            class="flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-[var(--arksync-text-muted)] transition-colors hover:bg-[var(--arksync-nav-active-bg)] hover:text-[var(--arksync-nav-active-text)]"
        >
            <span class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-bg)] text-[var(--arksync-text)]">
                {move || {
                    if dark_theme.get() {
                        view! { <SunIcon class="h-4 w-4" /> }.into_any()
                    } else {
                        view! { <MoonIcon class="h-4 w-4" /> }.into_any()
                    }
                }}
            </span>
            <span class="font-mono text-[10px] uppercase tracking-[0.18em]">
                {label}
            </span>
        </button>
    }
}
