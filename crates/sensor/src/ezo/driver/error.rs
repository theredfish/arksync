// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::fmt;

#[derive(Debug)]
pub enum DriverError {
    Connection(String),
    UnknownDevice(String),
    Read(String),
    Write(String),
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::Connection(msg) => write!(f, "Connection error: {}", msg),
            DriverError::UnknownDevice(msg) => write!(f, "Unknown device: {}", msg),
            DriverError::Read(msg) => write!(f, "Read error: {}", msg),
            DriverError::Write(msg) => write!(f, "Write error: {}", msg),
        }
    }
}

impl std::error::Error for DriverError {}

pub type Result<T> = std::result::Result<T, DriverError>;
