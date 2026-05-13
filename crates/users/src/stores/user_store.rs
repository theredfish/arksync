// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use eyre::Result;
use sqlx::PgPool;

use crate::{domain::User, stores::UserRecord};

pub struct UserStore<'a> {
    pool: &'a PgPool,
}

impl<'a> UserStore<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, user: &User) -> Result<()> {
        let user_record = UserRecord::from(user);

        sqlx::query(
            r#"
            INSERT INTO users (id, username, password)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(user_record.id)
        .bind(user_record.username)
        .bind(user_record.password)
        .execute(self.pool)
        .await?;

        Ok(())
    }
}
