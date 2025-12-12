use crate::components::charts::{AirTemperatureGauge, WaterTemperatureChart};
use crate::components::grid::{GridItem, GridLayout};
use crate::components::sidebar::Sidebar;
use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <main class="bg-darcula-black text-gray-100 flex h-screen antialiased">
                <Sidebar class="w-2/12 bg-darcula-gray p-5" />
                <Routes fallback=|| "Not found.">
                    <Route path=path!("/") view=Home/>
                    <Route path=path!("/dashboards") view=Dashboards />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn Dashboards() -> impl IntoView {
    view! {
        <div class="flex-1 h-screen">
            <GridLayout columns=6 display_grid=true>
                <GridItem id=1 col_start=0 col_span=4 row_start=0 row_span=2 label="Air temperature".to_string()>
                    <AirTemperatureGauge />
                </GridItem>
                <GridItem id=2 col_start=2 col_span=5 row_start=4 row_span=4>
                    <WaterTemperatureChart />
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
        <p>Welcome to ArkSync</p>
    }
}
