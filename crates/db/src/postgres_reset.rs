// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sqlx::PgExecutor;

use crate::Migrator;

pub async fn reset_public_schema<M>(executor: impl PgExecutor<'_>) -> eyre::Result<()>
where
    M: Migrator,
{
    executor
        .execute(
            r#"
            drop schema if exists public cascade;
            create schema public;
            "#,
        )
        .await?;

    crate::setup().await?;
    M::run().await?;

    Ok(())
}
