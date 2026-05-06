// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use eyre::Result;

use crate::pool;

pub async fn reset_public_schema() -> Result<()> {
    sqlx::query("drop schema if exists public cascade")
        .execute(pool())
        .await?;

    sqlx::query("create schema public").execute(pool()).await?;

    Ok(())
}
