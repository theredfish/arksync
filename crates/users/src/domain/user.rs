// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct User {
    id: Uuid,
    username: String,
    password: String,
}

impl User {
    pub fn new(username: String, password: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            password,
        }
    }

    pub(crate) fn id(&self) -> Uuid {
        self.id
    }

    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    pub(crate) fn password(&self) -> &str {
        &self.password
    }
}
