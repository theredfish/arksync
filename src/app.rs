use crate::components::charts::{AirTemperatureGauge, WaterTemperatureChart};
use crate::components::grid::{Grid, GridElement};
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
            <main class="bg-darcula-black text-gray-100 flex min-h-screen">
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
        <div class="w-full p-4">
            <Grid>
                <GridElement id=1 col_start=2 col_span=5 row_start=1 row_span=5>
                    <AirTemperatureGauge />
                </GridElement>
                <GridElement id=2 col_start=2 col_span=12 row_start=10 row_span=5>
                    <WaterTemperatureChart />
                </GridElement>
                <GridElement id=3 col_start=1 col_span=3 row_start=0 row_span=3>No data yet</GridElement>
            </Grid>
        </div>
    }
}

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <p>Welcome to ArkSync</p>
    }
}
