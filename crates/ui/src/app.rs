// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::charts::{AirTemperatureGauge, WaterTemperatureChart};
use crate::components::grid::{GridItem, GridLayout};
use crate::components::page_layout::PageLayout;
use crate::components::sidebar::Sidebar;
use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

#[component]
pub fn App() -> impl IntoView {
    let dark_theme = RwSignal::new(true);
    let theme_class = move || {
        if dark_theme.get() {
            "theme-dark"
        } else {
            "theme-light"
        }
    };

    view! {
        <Router>
            <main class=move || format!("{} h-screen overflow-hidden bg-[var(--arksync-app-bg)] text-[var(--arksync-text)] antialiased transition-colors", theme_class())>
                <div class="flex h-full w-full overflow-hidden">
                    <Sidebar
                        class="theme-sidebar w-64 shrink-0 border-r border-[var(--arksync-panel-border)] bg-[var(--arksync-sidebar-bg)] px-5 py-5 transition-colors"
                        dark_theme=dark_theme
                    />
                    <section class="flex min-w-0 flex-1 flex-col bg-[var(--arksync-app-bg)] text-[var(--arksync-text)] transition-colors">
                        <div class="min-h-0 flex-1">
                            <Routes fallback=|| "Not found.">
                                <Route path=path!("/") view=Home/>
                                <Route path=path!("/dashboards") view=move || view! { <Dashboards dark_theme=dark_theme /> } />
                            </Routes>
                        </div>
                    </section>
                </div>
            </main>
        </Router>
    }
}

#[component]
pub fn Dashboards(dark_theme: RwSignal<bool>) -> impl IntoView {
    view! {
        <div class="h-full">
            <GridLayout columns=12 display_grid=false>
                <GridItem id=1 col_start=0 col_span=4 row_start=0 row_span=2 label="Air temperature".to_string()>
                    <AirTemperatureGauge dark_theme=dark_theme />
                </GridItem>
                <GridItem id=2 col_start=2 col_span=5 row_start=4 row_span=4 label="Air Temp. History".to_string()>
                    <WaterTemperatureChart dark_theme=dark_theme />
                </GridItem>
                // <GridItem id=3 col_start=0 col_span=3 row_start=0 row_span=4>
                //     No data yet
                // </GridItem>
            </GridLayout>
        </div>
    }
}

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <PageLayout eyebrow="Station" title="ArkSync">
            <p class="mt-3 max-w-xl text-[var(--arksync-text-muted)]">
                "Environmental monitoring and regulation system."
            </p>
        </PageLayout>
    }
}
