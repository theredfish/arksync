use crate::components::charts::{AirTemperatureGauge, WaterTemperatureChart};
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
            <main class="bg-darcula-black text-gray-100 flex w-screen h-screen">
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
        <div class="flex flex-wrap w-full p-3 border border-green-600">
            <div id="charts-container-air-temp-gauge" class="w-1/2 lg:w-1/4 h-1/3 p-1 border border-red-600">
                <AirTemperatureGauge />
            </div>
            <div id="charts-container-water-temp-gauge" class="w-1/2 lg:w-1/4 h-1/3 p-1 border border-blue-600">
                <WaterTemperatureChart />
            </div>

            <div id="charts-container-air-temp-gauge2" class="w-1/2 lg:w-1/4 h-1/3 p-1 border border-red-600">
                more data
            </div>
            <div id="charts-container-water-temp-gauge2" class="w-1/2 lg:w-1/4 h-1/3 p-1 border border-blue-600">
                more data
            </div>

            // HERE NEW ROW
            <div id="charts-container-air-temp-gauge3" class="w-1/2 lg:w-1/4 h-1/3 p-1 border border-red-600">
                more data
            </div>
            <div id="charts-container-water-temp-gauge3" class="w-1/2 lg:w-1/4 h-1/3 p-1 border border-blue-600">
                more data
            </div>
        </div>
    }
}

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <p>Welcome to ArkSync</p>
    }
}
