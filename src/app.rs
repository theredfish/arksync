use crate::components::charts::DynamicChartExample;
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
                <Sidebar class="w-1/6 bg-darcula-gray p-5" />
                <Routes fallback=|| "Not found.">
                    <Route path=path!("/") view=Home/>
                    <Route path=path!("/dashboard") view=Dashboard />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn Dashboard() -> impl IntoView {
    view! {
        <div id="charts-container" class="w-full p-5">
            <DynamicChartExample />
        </div>
    }
}

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <p>Welcome to ArkSync</p>
    }
}
