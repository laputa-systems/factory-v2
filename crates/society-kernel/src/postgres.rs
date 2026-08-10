//! PostgreSQL configuration and the synchronous storage-boundary shell.
//!
//! The shell establishes the synchronous PostgreSQL execution shape: one
//! owned current-thread runtime and one bounded `PgPool`, with no runtime
//! construction per public method. Typed kernel transitions execute only
//! through this boundary.

use std::{fmt, str::FromStr, time::Duration};

use sqlx::{AssertSqlSafe, Row, SqlSafeStr, pool::PoolConnection};
use sqlx_core::migrate::{Migration, MigrationType, Migrator};
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgSslMode, Postgres};
use thiserror::Error;

/// A validated PostgreSQL connection URL whose `Display` implementation never
/// includes a password or other credential material.
#[derive(Clone)]
pub struct KernelDatabaseUrl {
    raw: String,
    options: PgConnectOptions,
}

impl KernelDatabaseUrl {
    pub fn parse(value: &str) -> Result<Self, PostgresStoreError> {
        if value.trim().is_empty() {
            return Err(PostgresStoreError::EmptyDatabaseUrl);
        }
        let options = PgConnectOptions::from_str(value)
            .map_err(|_| PostgresStoreError::InvalidDatabaseUrl)?;
        let local = options.get_socket().is_some()
            || matches!(options.get_host(), "localhost" | "127.0.0.1" | "::1");
        if !local
            && matches!(
                options.get_ssl_mode(),
                PgSslMode::Disable | PgSslMode::Allow | PgSslMode::Prefer
            )
        {
            return Err(PostgresStoreError::InsecureRemoteTls);
        }
        Ok(Self {
            raw: value.to_owned(),
            options,
        })
    }

    pub fn from_env(name: &str) -> Result<Self, PostgresStoreError> {
        let value = std::env::var(name)
            .map_err(|_| PostgresStoreError::MissingEnvironment(name.to_owned()))?;
        Self::parse(&value)
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub(crate) fn options(&self) -> PgConnectOptions {
        self.options.clone()
    }
}

impl fmt::Debug for KernelDatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KernelDatabaseUrl")
            .field("url", &self.to_string())
            .finish()
    }
}

impl fmt::Display for KernelDatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scheme = if self.raw.starts_with("postgresql://") {
            "postgresql://"
        } else {
            "postgres://"
        };
        let host = self.options.get_host();
        let port = self.options.get_port();
        let database = self.options.get_database().unwrap_or("");
        write!(formatter, "{scheme}{host}:{port}/{database}")
    }
}

#[derive(Debug, Error)]
pub enum PostgresStoreError {
    #[error("PostgreSQL database URL is empty")]
    EmptyDatabaseUrl,
    #[error("PostgreSQL database URL is invalid")]
    InvalidDatabaseUrl,
    #[error("remote PostgreSQL connections must require or verify TLS")]
    InsecureRemoteTls,
    #[error("required environment variable {0} is not set")]
    MissingEnvironment(String),
    #[error("could not build the PostgreSQL runtime")]
    Runtime(#[source] std::io::Error),
    #[error("PostgreSQL operation failed")]
    Database(#[source] sqlx::Error),
    #[error("PostgreSQL migration failed")]
    Migration(#[source] sqlx_core::migrate::MigrateError),
    #[error("PostgreSQL advisory lock is already held")]
    AdvisoryLockUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostgresCatalogSnapshot {
    pub table_count: i64,
    pub foreign_key_count: i64,
    pub partial_index_count: i64,
    pub trigger_count: i64,
    pub check_constraint_count: i64,
    pub migration_count: i64,
}

/// A database advisory lock held by one dedicated checked-out connection.
/// Keeping the connection inside this guard makes release a consequence of
/// the guard's lifetime rather than a pooled one-shot query.
pub struct PostgresAdvisoryLockGuard<'a> {
    store: &'a PostgresKernelStore,
    connection: Option<PoolConnection<Postgres>>,
    key: i64,
}

/// An advisory lock lease that owns the PostgreSQL store backing its dedicated
/// checked-out connection. This is the daemon-lifetime form; ownership avoids
/// a self-referential daemon struct while preserving connection-scoped unlock.
pub struct PostgresAdvisoryLockLease {
    store: PostgresKernelStore,
    connection: Option<PoolConnection<Postgres>>,
    key: i64,
}

// PostgreSQL migrations may be requested concurrently by independent test
// processes and by resident startup paths. Keep the single canonical
// migration runner serialized without making ordinary ledger operations
// share a process-local lock.
const MIGRATION_ADVISORY_LOCK_KEY: i64 = 0x0053_4f43_4945_5459;

impl Drop for PostgresAdvisoryLockLease {
    fn drop(&mut self) {
        let Some(mut connection) = self.connection.take() else {
            return;
        };
        let key = self.key;
        let _ = self.store.block_on(async move {
            sqlx::query("SELECT pg_advisory_unlock($1)")
                .bind(key)
                .execute(&mut *connection)
                .await
        });
    }
}

impl Drop for PostgresAdvisoryLockGuard<'_> {
    fn drop(&mut self) {
        let Some(mut connection) = self.connection.take() else {
            return;
        };
        let key = self.key;
        let _ = self.store.block_on(async move {
            sqlx::query("SELECT pg_advisory_unlock($1)")
                .bind(key)
                .execute(&mut *connection)
                .await
        });
    }
}

/// The owned synchronous PostgreSQL boundary used by the migrated kernel.
pub struct PostgresKernelStore {
    runtime: tokio::runtime::Runtime,
    pool: PgPool,
}

impl PostgresKernelStore {
    pub fn connect(url: &KernelDatabaseUrl) -> Result<Self, PostgresStoreError> {
        let store = Self::connect_with_options(url, None)?;
        store.migrate()?;
        Ok(store)
    }

    pub fn connect_in_schema(
        url: &KernelDatabaseUrl,
        schema: &str,
    ) -> Result<Self, PostgresStoreError> {
        if !is_safe_schema_name(schema) {
            return Err(PostgresStoreError::InvalidDatabaseUrl);
        }
        Self::connect_with_options(url, Some(schema))
    }

    fn connect_with_options(
        url: &KernelDatabaseUrl,
        schema: Option<&str>,
    ) -> Result<Self, PostgresStoreError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .map_err(PostgresStoreError::Runtime)?;
        let mut options = PgPoolOptions::new()
            .max_connections(8)
            .min_connections(0)
            .acquire_timeout(Duration::from_secs(10));
        if let Some(schema) = schema {
            let statement = format!("SET search_path TO {schema}");
            let after_connect_statement = statement.clone();
            options = options.after_connect(move |connection, _| {
                let statement = after_connect_statement.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(statement.as_str()))
                        .execute(&mut *connection)
                        .await?;
                    Ok(())
                })
            });
            options = options.before_acquire(move |connection, _| {
                let statement = statement.clone();
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(statement.as_str()))
                        .execute(&mut *connection)
                        .await?;
                    Ok(true)
                })
            });
        }
        let pool = runtime
            .block_on(options.connect_with(url.options()))
            .map_err(PostgresStoreError::Database)?;
        Ok(Self { runtime, pool })
    }

    pub fn migrate(&self) -> Result<(), PostgresStoreError> {
        let lock = self.acquire_advisory_lock(MIGRATION_ADVISORY_LOCK_KEY)?;
        let result = self
            .runtime
            .block_on(async {
                let migration = Migration::new(
                    1,
                    "kernel".into(),
                    MigrationType::Simple,
                    include_str!("../../../migrations/postgres/0001_kernel.sql").into_sql_str(),
                    false,
                );
                Migrator::with_migrations(vec![migration])
                    .run(&self.pool)
                    .await
            })
            .map_err(PostgresStoreError::Migration);
        drop(lock);
        result
    }

    /// Creates a private schema when necessary and applies the canonical
    /// migration through this migration-capable connection. Runtime roles can
    /// then connect to the already-migrated schema without DDL privileges.
    pub fn migrate_in_schema(
        url: &KernelDatabaseUrl,
        schema: &str,
    ) -> Result<(), PostgresStoreError> {
        let admin = Self::connect(url)?;
        admin.ensure_private_schema(schema)?;
        let scoped = Self::connect_in_schema(url, schema)?;
        scoped.migrate()
    }

    pub fn pool_size(&self) -> u32 {
        self.pool.size()
    }

    pub fn catalog_snapshot(&self) -> Result<PostgresCatalogSnapshot, PostgresStoreError> {
        self.block_on(async {
            let row = sqlx::query(
                "SELECT
                    (SELECT COUNT(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = current_schema() AND c.relkind = 'r' AND c.relname <> '_sqlx_migrations'),
                    (SELECT COUNT(*) FROM pg_constraint k JOIN pg_class c ON c.oid = k.conrelid JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = current_schema() AND k.contype = 'f'),
                    (SELECT COUNT(*) FROM pg_index i JOIN pg_class c ON c.oid = i.indrelid JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = current_schema() AND i.indpred IS NOT NULL),
                    (SELECT COUNT(*) FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = current_schema() AND NOT t.tgisinternal),
                    (SELECT COUNT(*) FROM pg_constraint k JOIN pg_class c ON c.oid = k.conrelid JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = current_schema() AND k.contype = 'c'),
                    (SELECT COUNT(*) FROM _sqlx_migrations)",
            )
            .fetch_one(&self.pool)
            .await
            .map_err(PostgresStoreError::Database)?;
            Ok(PostgresCatalogSnapshot {
                table_count: row.get(0),
                foreign_key_count: row.get(1),
                partial_index_count: row.get(2),
                trigger_count: row.get(3),
                check_constraint_count: row.get(4),
                migration_count: row.get(5),
            })
        })
    }

    pub fn create_private_schema(&self, schema: &str) -> Result<(), PostgresStoreError> {
        if !is_safe_schema_name(schema) {
            return Err(PostgresStoreError::InvalidDatabaseUrl);
        }
        self.block_on(async {
            sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}").as_str()))
                .execute(&self.pool)
                .await
                .map(|_| ())
                .map_err(PostgresStoreError::Database)
        })
    }

    pub(crate) fn ensure_private_schema(&self, schema: &str) -> Result<(), PostgresStoreError> {
        if !is_safe_schema_name(schema) {
            return Err(PostgresStoreError::InvalidDatabaseUrl);
        }
        self.block_on(async {
            sqlx::query(AssertSqlSafe(
                format!("CREATE SCHEMA IF NOT EXISTS {schema}").as_str(),
            ))
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(PostgresStoreError::Database)
        })
    }

    pub fn drop_private_schema(&self, schema: &str) -> Result<(), PostgresStoreError> {
        if !is_safe_schema_name(schema) {
            return Err(PostgresStoreError::InvalidDatabaseUrl);
        }
        self.block_on(async {
            sqlx::query(AssertSqlSafe(
                format!("DROP SCHEMA {schema} CASCADE").as_str(),
            ))
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(PostgresStoreError::Database)
        })
    }

    pub fn acquire_advisory_lock(
        &self,
        key: i64,
    ) -> Result<PostgresAdvisoryLockGuard<'_>, PostgresStoreError> {
        self.block_on(async {
            let mut connection = self
                .pool
                .acquire()
                .await
                .map_err(PostgresStoreError::Database)?;
            sqlx::query("SELECT pg_advisory_lock($1)")
                .bind(key)
                .execute(&mut *connection)
                .await
                .map_err(PostgresStoreError::Database)?;
            Ok(PostgresAdvisoryLockGuard {
                store: self,
                connection: Some(connection),
                key,
            })
        })
    }

    pub fn acquire_owned_advisory_lock(
        self,
        key: i64,
    ) -> Result<PostgresAdvisoryLockLease, PostgresStoreError> {
        let connection = self.block_on(async {
            let mut connection = self
                .pool
                .acquire()
                .await
                .map_err(PostgresStoreError::Database)?;
            sqlx::query("SELECT pg_advisory_lock($1)")
                .bind(key)
                .execute(&mut *connection)
                .await
                .map_err(PostgresStoreError::Database)?;
            Ok(connection)
        })?;
        Ok(PostgresAdvisoryLockLease {
            store: self,
            connection: Some(connection),
            key,
        })
    }

    pub fn try_acquire_advisory_lock(
        &self,
        key: i64,
    ) -> Result<Option<PostgresAdvisoryLockGuard<'_>>, PostgresStoreError> {
        self.block_on(async {
            let mut connection = self
                .pool
                .acquire()
                .await
                .map_err(PostgresStoreError::Database)?;
            let row = sqlx::query("SELECT pg_try_advisory_lock($1)")
                .bind(key)
                .fetch_one(&mut *connection)
                .await
                .map_err(PostgresStoreError::Database)?;
            if row.get::<bool, _>(0) {
                Ok(Some(PostgresAdvisoryLockGuard {
                    store: self,
                    connection: Some(connection),
                    key,
                }))
            } else {
                Ok(None)
            }
        })
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub(crate) fn block_on<F>(&self, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        self.runtime.block_on(future)
    }
}

fn is_safe_schema_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && value.as_bytes()[0].is_ascii_alphabetic()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_errors_are_redacted() {
        let error = KernelDatabaseUrl::parse("postgres://alice:secret@example.test/db")
            .expect_err("remote insecure URL must be rejected");
        assert_eq!(
            error.to_string(),
            "remote PostgreSQL connections must require or verify TLS"
        );
    }

    #[test]
    fn local_url_display_omits_credentials() {
        let url = KernelDatabaseUrl::parse("postgres://alice:secret@localhost:5432/postgres")
            .expect("local URL should parse");
        assert_eq!(url.to_string(), "postgres://localhost:5432/postgres");
        assert!(!format!("{url:?}").contains("secret"));
    }
}
