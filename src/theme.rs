use charming::{theme::Theme, Echarts};
use leptos::logging::log;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

pub static THEME_JS: &str = include_str!("../styles/chart_themes/purple-passion.js");
pub static THEME: Theme = Theme::Custom("purple-passion", THEME_JS);
static THEME_REGISTERED: AtomicBool = AtomicBool::new(false);

#[wasm_bindgen]
extern "C" {
    // Access the global echarts object
    #[wasm_bindgen(js_namespace = window, js_name = echarts)]
    static ECHARTS: JsValue;

    // Or access it via the type directly
    #[wasm_bindgen(js_namespace = echarts, js_name = registerTheme)]
    fn register_theme_js(name: &str, theme: JsValue);
}

pub fn register_theme() {
    if !THEME_REGISTERED.swap(true, Ordering::SeqCst) {
        // Check if echarts is available
        if ECHARTS.is_undefined() {
            log!("ECharts is not loaded yet!");
            return;
        }

        log!("ECharts global object found");

        // Now eval the theme JS - echarts should be available
        match js_sys::eval(THEME_JS) {
            Ok(_) => log!("Theme registered successfully!"),
            Err(e) => log!("Failed to register theme: {:?}", e),
        }
    }
}
