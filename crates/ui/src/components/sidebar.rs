// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::heroicons::{
    CpuChipIcon, PresentationChartBarIcon, RectangleGroupIcon, ShieldExclamationIcon,
};
use leptos::prelude::*;
use leptos::IntoView;
use leptos_router::components::A;

#[component]
pub fn Sidebar(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <div class={class}>
            <div class="mb-7 rounded-md border border-sk-carbon-725 bg-sk-carbon-850 px-3 py-3">
                <div class="font-mono text-[10px] uppercase tracking-[0.2em] text-sk-carbon-450">"Station"</div>
                <A href="/" attr:class="mt-1 block truncate text-sm font-medium text-sk-carbon-100">"ArkSync"</A>
            </div>

            <ul class="space-y-1.5">
                <li><A href="dashboards" attr:class="block rounded-md px-3 py-2 text-sk-carbon-400 transition-colors hover:bg-sk-carbon-800 hover:text-sk-aqua-50 aria-[current=page]:bg-sk-carbon-800 aria-[current=page]:text-sk-aqua-50">
                    <span class="inline-flex items-center">
                        <PresentationChartBarIcon class="mr-2 h-5 w-5" />
                        "Dashboards"
                    </span>
                </A></li>

                <li><A href="alerts" attr:class="block rounded-md px-3 py-2 text-sk-carbon-400 transition-colors hover:bg-sk-carbon-800 hover:text-sk-aqua-50 aria-[current=page]:bg-sk-carbon-800 aria-[current=page]:text-sk-aqua-50">
                    <span class="inline-flex items-center">
                        <ShieldExclamationIcon class="mr-2 h-5 w-5" />
                        "Alerts"
                    </span>
                </A></li>

                <li><A href="sensors" attr:class="block rounded-md px-3 py-2 text-sk-carbon-400 transition-colors hover:bg-sk-carbon-800 hover:text-sk-aqua-50 aria-[current=page]:bg-sk-carbon-800 aria-[current=page]:text-sk-aqua-50">
                    <span class="inline-flex items-center">
                        <CpuChipIcon class="mr-2 h-5 w-5" />
                        "Sensors"
                    </span>
                </A></li>

                <li><A href="nodes" attr:class="block rounded-md px-3 py-2 text-sk-carbon-400 transition-colors hover:bg-sk-carbon-800 hover:text-sk-aqua-50 aria-[current=page]:bg-sk-carbon-800 aria-[current=page]:text-sk-aqua-50">
                    <span class="inline-flex items-center">
                        <RectangleGroupIcon class="mr-2 h-5 w-5" />
                        "Nodes"
                    </span>
                </A></li>
            </ul>
        </div>
    }
}
