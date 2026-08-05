// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use eyre::Result;
use sqlx::PgPool;

use crate::pool;

pub async fn setup() -> Result<()> {
    setup_pool(pool()).await
}

pub async fn setup_pool(pool: &PgPool) -> Result<()> {
    setup_public_schema_permissions(pool).await?;
    setup_extensions(pool).await?;

    Ok(())
}

async fn setup_public_schema_permissions(pool: &PgPool) -> Result<()> {
    sqlx::query("grant all on schema public to public")
        .execute(pool)
        .await?;
    sqlx::query("grant all on schema public to current_user")
        .execute(pool)
        .await?;

    Ok(())
}

async fn setup_extensions(pool: &PgPool) -> Result<()> {
    sqlx::query("create extension if not exists pgcrypto")
        .execute(pool)
        .await?;
    sqlx::query("create extension if not exists pg_partman")
        .execute(pool)
        .await?;

    Ok(())
}
