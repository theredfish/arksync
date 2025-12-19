use charming::{theme::Theme, Echarts};
use eyre::{eyre, Result};
use js_sys::{Reflect, JSON};
use leptos::logging::log;
use std::sync::atomic::{AtomicBool, Ordering};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::theme::sealed::JsValueSealed;

type Themes = Vec<ArkSyncTheme>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArkSyncTheme {
    Westeros,
    Chalk,
    Roma,
    Wonderland,
    Walden,
}

pub struct ArkSyncThemeWrapper {
    pub name: &'static str,
    pub charming_theme: Theme,
    pub json: &'static str,
}

impl ArkSyncTheme {
    pub fn as_wrapper(&self) -> ArkSyncThemeWrapper {
        match self {
            ArkSyncTheme::Westeros => ArkSyncThemeWrapper {
                name: "westeros",
                charming_theme: Theme::Custom("westeros", ""),
                json: include_str!("../styles/chart_themes/westeros.json"),
            },
            ArkSyncTheme::Chalk => ArkSyncThemeWrapper {
                name: "chalk",
                charming_theme: Theme::Custom("chalk", ""),
                json: include_str!("../styles/chart_themes/chalk.json"),
            },
            ArkSyncTheme::Walden => ArkSyncThemeWrapper {
                name: "walden",
                charming_theme: Theme::Custom("walden", ""),
                json: include_str!("../styles/chart_themes/walden.json"),
            },
            ArkSyncTheme::Wonderland => ArkSyncThemeWrapper {
                name: "wonderland",
                charming_theme: Theme::Custom("wonderland", ""),
                json: include_str!("../styles/chart_themes/wonderland.json"),
            },
            ArkSyncTheme::Roma => ArkSyncThemeWrapper {
                name: "roma",
                charming_theme: Theme::Custom("roma", ""),
                json: include_str!("../styles/chart_themes/roma.json"),
            },
        }
    }
}

// pub static THEME_JSON: &str = include_str!("../styles/chart_themes/westeros.json");
// pub static WESTEROS_THEME: Theme = Theme::Custom("westeros", "");
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

mod sealed {
    pub trait JsValueSealed {}
}

trait JsValueExists: JsValueSealed {
    fn exists_or_err(&self, err_msg: &'static str) -> Result<JsValue>;
}

impl JsValueSealed for JsValue {}

impl JsValueExists for JsValue {
    fn exists_or_err(&self, err_msg: &'static str) -> Result<Self> {
        if self.is_undefined() || self.is_null() {
            log!("Something was undefined");
            // Do we get the error log somewhere when used on the frontend?
            eyre::bail!(err_msg)
        }

        Ok(self.clone())
    }
}

pub fn register_theme(themes: Themes) -> Result<()> {
    if !THEME_REGISTERED.swap(true, Ordering::SeqCst) {
        // Check if echarts is available
        ECHARTS.exists_or_err("ECharts is not loaded yet!")?;

        let Ok(register_theme) = Reflect::get(&ECHARTS, &JsValue::from_str("registerTheme")) else {
            eyre::bail!("ECharts registerTheme function not found!");
        };

        let register_theme =
            register_theme.exists_or_err("ECharts registerTheme function not found!")?;

        // Call register_theme function with THEME_JSON str
        for theme in themes {
            let theme_json =
                JSON::parse(theme.as_wrapper().json).map_err(|err| eyre!("Json parse error"))?;
            register_theme_js(theme.as_wrapper().name, theme_json);
        }
    }

    Ok(())
}
