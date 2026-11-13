// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::application::HubActuatorError;
use crate::infrastructure::store::{KnotMessageStoreError, KnotStoreError};
use derive_more::From;
use std::fmt;

#[derive(Debug, From)]
pub enum HubKnotError {
    KnotStore(KnotStoreError),
    KnotMessageStore(KnotMessageStoreError),
    HubActuator(HubActuatorError),
}

impl fmt::Display for HubKnotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HubKnotError::KnotStore(err) => write!(f, "knot store error: {err:?}"),
            HubKnotError::KnotMessageStore(err) => {
                write!(f, "knot message store error: {err:?}")
            }
            HubKnotError::HubActuator(err) => write!(f, "hub actuator error: {err}"),
        }
    }
}

impl std::error::Error for HubKnotError {}
