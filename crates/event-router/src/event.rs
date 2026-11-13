// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Generic event envelopes shared by event routers and transport adapters.

use crate::EventId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Timestamp {
    pub unix_millis: i64,
}

impl Timestamp {
    pub fn from_unix_millis(unix_millis: i64) -> Self {
        Self { unix_millis }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EventEnvelope<E, S = ()> {
    pub id: EventId,
    pub source: S,
    pub occurred_at: Timestamp,
    pub payload: E,
}

impl<E, S> EventEnvelope<E, S> {
    pub fn new_with_id(id: EventId, source: S, occurred_at: Timestamp, payload: E) -> Self {
        Self {
            id,
            source,
            occurred_at,
            payload,
        }
    }
}

#[cfg(feature = "uuid-v4")]
impl<E, S> EventEnvelope<E, S> {
    pub fn new(source: S, occurred_at: Timestamp, payload: E) -> Self {
        Self::new_with_id(EventId::new(), source, occurred_at, payload)
    }
}
