use crate::components::charts::{AirTemperatureGauge, WaterTemperatureChart};
use crate::components::grid::Grid;
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
        <Grid />
        // <div class="w-full grid grid-cols-12 gap-4 p-3 border border-green-600">
        //     <div id="charts-container-air-temp-gauge" class="col-span-4 row-span-2 p-1 border border-red-600" draggable="true">
        //         <AirTemperatureGauge />
        //     </div>
        //     <div id="charts-container-water-temp-gauge" class="col-span-8 row-span-3 p-1 border border-blue-600" draggable="true">
        //         <WaterTemperatureChart />
        //     </div>

        //     <div id="charts-container-air-temp-gauge2" class="p-1 border border-red-600">
        //         <div class="w-full h-full">more data</div>
        //     </div>
        //     <div id="charts-container-water-temp-gauge2" class="p-1 border border-blue-600">
        //         <div class="w-full h-full">more data</div>
        //     </div>

        //     // HERE NEW ROW
        //     <div id="charts-container-air-temp-gauge3" class="p-1 border border-red-600">
        //         <div class="w-full h-full">more data</div>
        //     </div>
        //     <div id="charts-container-water-temp-gauge3" class="p-1 border border-blue-600">
        //         <div class="w-full h-full">more data</div>
        //     </div>
        //     <div id="charts-container-air-temp-gauge3" class="p-1 border border-red-600">
        //         <div class="w-full h-full">more data</div>
        //     </div>
        //     <div id="charts-container-water-temp-gauge3" class="p-1 border border-blue-600">
        //         <div class="w-full h-full">more data</div>
        //     </div>
        // </div>
    }
}

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <p>Welcome to ArkSync</p>
    }
}
