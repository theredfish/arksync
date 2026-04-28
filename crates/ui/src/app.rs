use crate::components::charts::{AirTemperatureGauge, WaterTemperatureChart};
use crate::components::grid::{GridItem, GridLayout};
use crate::components::page_layout::PageLayout;
use crate::components::sidebar::Sidebar;
use crate::theme::ArkSyncTheme;
use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <main class="h-screen overflow-hidden bg-sk-carbon-840 text-sk-carbon-150 antialiased">
                <div class="flex h-full w-full overflow-hidden">
                    <Sidebar class="w-64 shrink-0 border-r border-sk-carbon-700 bg-sk-carbon-975 px-5 py-5" />
                    <section class="min-w-0 flex-1 bg-sk-carbon-840 text-sk-carbon-150">
                        <Routes fallback=|| "Not found.">
                            <Route path=path!("/") view=Home/>
                            <Route path=path!("/dashboards") view=Dashboards />
                        </Routes>
                    </section>
                </div>
            </main>
        </Router>
    }
}

#[component]
pub fn Dashboards() -> impl IntoView {
    view! {
        <div class="h-full">
            <GridLayout columns=12 display_grid=false>
                <GridItem id=1 col_start=0 col_span=4 row_start=0 row_span=2 label="Air temperature".to_string()>
                    <AirTemperatureGauge theme=ArkSyncTheme::Walden />
                </GridItem>
                <GridItem id=2 col_start=2 col_span=5 row_start=4 row_span=4>
                    <WaterTemperatureChart theme=ArkSyncTheme::Walden />
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
            <p class="mt-3 max-w-xl text-sk-carbon-300">
                "Centralisez les mesures, les alertes et les automatisations de votre station."
            </p>
        </PageLayout>
    }
}
