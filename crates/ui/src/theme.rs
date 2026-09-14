// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[derive(Clone, Copy, Debug)]
pub struct ArkSyncChartColors {
    pub background: &'static str,
    pub text: &'static str,
    pub muted_text: &'static str,
    pub grid: &'static str,
}

impl ArkSyncChartColors {
    pub fn from_dark_theme(dark_theme: bool) -> Self {
        if dark_theme {
            Self {
                background: "#2c2b31",
                text: "#ffffff",
                muted_text: "#d8d2df",
                grid: "#5a5563",
            }
        } else {
            Self {
                background: "#fafafa",
                text: "#050908",
                muted_text: "#26332d",
                grid: "#9aa59e",
            }
        }
    }
}
