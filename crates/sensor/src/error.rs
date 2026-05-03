// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum SensorError {
    Message(String),
    Source(Box<dyn Error + Send + Sync>),
}

impl SensorError {
    pub fn message(msg: impl Into<String>) -> Self {
        SensorError::Message(msg.into())
    }

    pub fn source<E>(err: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        SensorError::Source(Box::new(err))
    }
}

impl fmt::Display for SensorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SensorError::Message(msg) => write!(f, "{msg}"),
            SensorError::Source(err) => write!(f, "{err}"),
        }
    }
}

impl Error for SensorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SensorError::Message(_) => None,
            SensorError::Source(err) => Some(err.as_ref()),
        }
    }
}

pub type Result<T> = std::result::Result<T, SensorError>;
