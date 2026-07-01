// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{domain::User, infrastructure::store::users as user_store};
use eyre::Result;
use sqlx::PgExecutor;

pub struct CreateUser {
    pub username: String,
    pub password: String,
}

pub async fn create_user(executor: impl PgExecutor<'_>, command: CreateUser) -> Result<User> {
    let user = User::new(command.username, command.password);
    user_store::insert_user(executor, &user).await?;

    Ok(user)
}
