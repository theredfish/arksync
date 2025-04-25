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
            <main class="bg-darcula-black text-gray-100 flex min-h-screen antialiased">
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
                <GridItem id=1 width=300 height=300 position_x=400 position_y=100>
                    <AirTemperatureGauge />
                </GridItem>
                <GridItem id=2 width=800 height=400 position_x=500 position_y=200>
                    <WaterTemperatureChart />
                </GridItem>
                <GridItem id=3 width=100 height=100 position_x=0 position_y=0>
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
