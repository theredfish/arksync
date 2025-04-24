use crate::components::charts::{AirTemperatureGauge, WaterTemperatureChart};
use crate::components::grid::{Grid, GridItem};
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
                <GridItem id=1 width=2 height=5 position_x=1 position_y=5>
                    <AirTemperatureGauge />
                </GridItem>
                <GridItem id=2 width=2 height=12 position_x=10 position_y=5>
                    <WaterTemperatureChart />
                </GridItem>
                <GridItem id=3 width=1 height=3 position_x=0 position_y=3>
                    No data yet
                </GridItem>
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
