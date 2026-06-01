// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::{fmt, str::FromStr};
use rand::distr::{Alphanumeric, Distribution};
use serde::{Deserialize, Serialize};

pub mod rng;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DeviceUid(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceUidError {
    Invalid,
}

impl DeviceUid {
    pub const LEN: usize = 16;

    pub fn new() -> Self {
        rng::with(|rng| {
            let uid = (0..Self::LEN)
                .map(|_| Alphanumeric.sample(rng) as char)
                .collect();

            Self(uid)
        })
    }

    pub fn is_valid(value: &str) -> bool {
        value.len() == Self::LEN && value.chars().all(|char| char.is_ascii_alphanumeric())
    }
}

impl AsRef<str> for DeviceUid {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<DeviceUid> for String {
    fn from(value: DeviceUid) -> Self {
        value.0
    }
}

impl fmt::Display for DeviceUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl fmt::Display for DeviceUidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceUidError::Invalid => f.write_str("invalid ArkSync device UID"),
        }
    }
}

impl FromStr for DeviceUid {
    type Err = DeviceUidError;

    fn from_str(value: &str) -> core::result::Result<Self, Self::Err> {
        Self::try_from(value)
    }
}

impl TryFrom<&str> for DeviceUid {
    type Error = DeviceUidError;

    fn try_from(value: &str) -> core::result::Result<Self, Self::Error> {
        if Self::is_valid(value) {
            Ok(Self(value.to_string()))
        } else {
            Err(DeviceUidError::Invalid)
        }
    }
}

impl TryFrom<String> for DeviceUid {
    type Error = DeviceUidError;

    fn try_from(value: String) -> core::result::Result<Self, Self::Error> {
        if Self::is_valid(&value) {
            Ok(Self(value))
        } else {
            Err(DeviceUidError::Invalid)
        }
    }
}
