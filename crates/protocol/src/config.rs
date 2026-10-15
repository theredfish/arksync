// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_config::ConfigHandler;

const FRAME_MAGIC: [u8; 3] = *b"ARK";
const PROTOCOL_VERSION: u8 = 1;
const SUPPORTED_VERSIONS: &[u8] = &[PROTOCOL_VERSION];

pub static CONFIG: ConfigHandler<Config> = ConfigHandler::new(mpl);

/// Wire-format configuration used by the ArkSync protocol codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Fixed bytes identifying an ArkSync protocol frame.
    pub frame_magic: [u8; 3],
    /// Wire version written by the frame encoder.
    pub protocol_version: u8,
    /// Wire versions that the current frame decoder can read.
    pub supported_versions: &'static [u8],
}

impl Config {
    pub const fn frame_header_len(&self) -> usize {
        self.frame_magic.len() + 1
    }

    pub fn supports_version(&self, version: u8) -> bool {
        self.supported_versions.contains(&version)
    }
}

fn mpl() -> Config {
    Config {
        frame_magic: FRAME_MAGIC,
        protocol_version: PROTOCOL_VERSION,
        supported_versions: SUPPORTED_VERSIONS,
    }
}
