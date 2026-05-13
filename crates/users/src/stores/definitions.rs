// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlx::FromRow;
use uuid::Uuid;

use crate::domain::User;

#[derive(Clone, Debug, FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub username: String,
    pub password: String,
}

impl From<&User> for UserRecord {
    fn from(user: &User) -> Self {
        Self {
            id: user.id(),
            username: user.username().to_string(),
            password: user.password().to_string(),
        }
    }
}
