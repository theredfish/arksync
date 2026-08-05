// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_db::{setup_pool, Config, MplMigrator, CONFIG};
use eyre::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgExecutor;
pub use sqlx::PgPool;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::process;

const TEMPLATE_LOCK_NAMESPACE: &str = "arksync_testing_pg_tpl";

pub struct TestDatabase {
    database: Database,
    pool: PgPool,
}

impl TestDatabase {
    pub async fn setup(test_path: &str) -> Result<Self> {
        PostgresTestHarness::new()
            .database_for_test(test_path)
            .await
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn teardown(self) -> Result<()> {
        self.pool.close().await;

        PostgresTestHarness::new()
            .drop_database(&self.database)
            .await
    }
}

pub trait TestOutcome {
    fn succeeded(&self) -> bool;
}

impl TestOutcome for () {
    fn succeeded(&self) -> bool {
        true
    }
}

impl<T, E> TestOutcome for core::result::Result<T, E> {
    fn succeeded(&self) -> bool {
        self.is_ok()
    }
}

pub fn test_succeeded(result: &impl TestOutcome) -> bool {
    result.succeeded()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Database(String);

impl Database {
    fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    fn quoted(&self) -> String {
        format!(r#""{}""#, self.0.replace('"', r#""""#))
    }

    async fn exists(&self, executor: impl PgExecutor<'_>) -> Result<bool> {
        let row: Option<i32> = sqlx::query_scalar("select 1 from pg_database where datname = $1")
            .bind(&self.0)
            .fetch_optional(executor)
            .await?;

        Ok(row.is_some())
    }

    async fn drop_from(&self, executor: impl PgExecutor<'_>) -> Result<()> {
        sqlx::query(format!("drop database if exists {} with (force)", self.quoted()).as_str())
            .execute(executor)
            .await?;

        Ok(())
    }

    async fn create_empty_from(&self, executor: impl PgExecutor<'_>) -> Result<()> {
        sqlx::query(format!("create database {}", self.quoted()).as_str())
            .execute(executor)
            .await?;

        Ok(())
    }

    async fn create_from_template(
        &self,
        executor: impl PgExecutor<'_>,
        template: &TemplateDatabase,
    ) -> Result<()> {
        sqlx::query(
            format!(
                "create database {} template {}",
                self.quoted(),
                template.database.quoted()
            )
            .as_str(),
        )
        .execute(executor)
        .await
        .wrap_err("failed to create ArkSync test database from template")?;

        Ok(())
    }
}

struct TemplateDatabase {
    database: Database,
    fingerprint: String,
}

impl TemplateDatabase {
    fn from_mpl_migrator() -> Self {
        let fingerprint = migration_fingerprint();

        Self {
            database: Database::new(format!("_arksync_tpl_{fingerprint}")),
            fingerprint,
        }
    }

    fn build_database(&self) -> Database {
        Database::new(format!(
            "{}_building_{}",
            self.database.0,
            short_hash((process::id(), std::thread::current().name()))
        ))
    }

    fn lock_key(&self) -> i64 {
        i64::from_ne_bytes(hash((TEMPLATE_LOCK_NAMESPACE, &self.fingerprint)).to_ne_bytes())
    }
}

struct PostgresTestHarness {
    admin_pool: PgPool,
}

impl PostgresTestHarness {
    fn new() -> Self {
        Self {
            admin_pool: PgPoolOptions::new()
                .max_connections(1)
                .connect_lazy(&database_url(&CONFIG, "postgres"))
                .expect("postgres database url should be valid"),
        }
    }

    async fn database_for_test(&self, test_path: &str) -> Result<TestDatabase> {
        let test_database = Database::new(format!("_arksync_test_{}", short_hash(test_path)));

        self.drop_database(&test_database).await?;
        let template = self.ensure_template().await?;
        test_database
            .create_from_template(&self.admin_pool, &template)
            .await?;

        let pool = PgPoolOptions::new()
            .max_connections(CONFIG.pg_max_connections)
            .connect_lazy(&database_url(&CONFIG, &test_database.0))
            .wrap_err("failed to create ArkSync test pool")?;

        Ok(TestDatabase {
            database: test_database,
            pool,
        })
    }

    async fn drop_database(&self, database: &Database) -> Result<()> {
        database.drop_from(&self.admin_pool).await
    }

    async fn ensure_template(&self) -> Result<TemplateDatabase> {
        let template = TemplateDatabase::from_mpl_migrator();

        if template.database.exists(&self.admin_pool).await? {
            return Ok(template);
        }

        let mut admin_connection = self.admin_pool.acquire().await?;
        let lock_key = template.lock_key();
        sqlx::query("select pg_advisory_lock($1)")
            .bind(lock_key)
            .execute(&mut *admin_connection)
            .await?;

        let template_result = self
            .ensure_template_while_locked(&mut admin_connection, &template)
            .await;
        let unlock_result = sqlx::query("select pg_advisory_unlock($1)")
            .bind(lock_key)
            .execute(&mut *admin_connection)
            .await;

        template_result?;
        unlock_result?;

        Ok(template)
    }

    async fn ensure_template_while_locked(
        &self,
        admin_connection: &mut sqlx::PgConnection,
        template: &TemplateDatabase,
    ) -> Result<()> {
        if template.database.exists(&mut *admin_connection).await? {
            return Ok(());
        }

        let build_database = template.build_database();
        build_database.drop_from(&mut *admin_connection).await?;
        build_database
            .create_empty_from(&mut *admin_connection)
            .await?;

        let build_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&database_url(&CONFIG, &build_database.0))
            .wrap_err("failed to create ArkSync test template build pool")?;

        let build_result = async {
            setup_pool(&build_pool).await?;
            MplMigrator::run_on(&build_pool).await?;

            Ok::<_, eyre::Report>(())
        }
        .await;

        build_pool.close().await;

        if let Err(error) = build_result {
            build_database.drop_from(&mut *admin_connection).await?;
            return Err(error);
        }

        sqlx::query(
            format!(
                "alter database {} rename to {}",
                build_database.quoted(),
                template.database.quoted()
            )
            .as_str(),
        )
        .execute(&mut *admin_connection)
        .await?;

        Ok(())
    }
}

fn database_url(config: &Config, database_name: &str) -> String {
    format!(
        "postgres://{}:{}@{}:{}/{}",
        config.pg_user, config.pg_password, config.pg_host, config.pg_port, database_name
    )
}

fn migration_fingerprint() -> String {
    let mut hasher = DefaultHasher::new();

    for migration in
        MplMigrator::migrations().filter(|migration| !migration.migration_type.is_down_migration())
    {
        migration.version.hash(&mut hasher);
        migration.description.hash(&mut hasher);
        migration.checksum.hash(&mut hasher);
    }

    format!("{:016x}", hasher.finish())
}

fn short_hash(value: impl Hash) -> String {
    format!("{:016x}", hash(value))
}

fn hash(value: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}
