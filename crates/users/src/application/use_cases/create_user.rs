// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use eyre::Result;
use sqlx::PgPool;

use crate::{domain::User, stores::UserStore};

pub struct CreateUser {
    pub username: String,
    pub password: String,
}

pub struct CreateUserUseCase<'a> {
    store: UserStore<'a>,
}

impl<'a> CreateUserUseCase<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self {
            store: UserStore::new(pool),
        }
    }

    pub async fn execute(&self, cmd: CreateUser) -> Result<User> {
        let user = User::new(cmd.username, cmd.password);
        self.store.create(&user).await?;

        Ok(user)
    }
}
