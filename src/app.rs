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
            <GridLayout columns=10>
                <GridItem id=1 width=300 height=300 x=400. y=100.>
                    <AirTemperatureGauge />
                </GridItem>
                // <GridItem id=2 width=800 height=400 x=800. y=500.>
                //     <WaterTemperatureChart />
                // </GridItem>
                // <GridItem id=3 width=100 height=100 x=0. y=0.>
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
