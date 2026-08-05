// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{domain::User, infrastructure::store::UserRecord};
use eyre::Result;
use sqlx::PgExecutor;

pub async fn insert_user(executor: impl PgExecutor<'_>, user: &User) -> Result<()> {
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
    .execute(executor)
    .await?;

    Ok(())
}
