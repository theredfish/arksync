// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[derive(Debug)]
pub enum KnotMessageStoreError {
    Database(sqlx::Error),
    Serialization(serde_json::Error),
}

impl From<sqlx::Error> for KnotMessageStoreError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<serde_json::Error> for KnotMessageStoreError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serialization(value)
    }
}
