//! PostgreSQL configuration and the synchronous storage-boundary shell.
//!
//! The shell establishes the synchronous PostgreSQL execution shape: one
//! bounded `PgPool` driven by the workspace's async-std executor, with no
//! executor construction per public method. Typed kernel transitions execute
//! only through this boundary.

use std::{fmt, str::FromStr, time::Duration};

use sqlx::{AssertSqlSafe, Row, pool::PoolConnection};
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgSslMode, Postgres};
use thiserror::Error;

/// Exact marker applied by the authoritative fresh PostgreSQL bootstrap.
pub const POSTGRES_SCHEMA_REVISION: &str = "society-kernel-postgres-schema-v20";

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

    pub(crate) fn with_database(&self, database: &str) -> Self {
        Self {
            raw: self.raw.clone(),
            options: self.options.clone().database(database),
        }
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
    #[error("PostgreSQL operation failed")]
    Database(#[source] sqlx::Error),
    #[error(
        "PostgreSQL schema revision mismatch: expected {expected}, found {actual:?}; apply the canonical fresh bootstrap"
    )]
    SchemaRevisionMismatch {
        expected: &'static str,
        actual: Option<String>,
    },
    #[error("PostgreSQL advisory lock is already held")]
    AdvisoryLockUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PostgresCatalogSnapshot {
    pub schema_revision: Option<String>,
    pub table_count: i64,
    pub foreign_key_count: i64,
    pub partial_index_count: i64,
    pub trigger_count: i64,
    pub check_constraint_count: i64,
}

/// A database advisory lock held by one dedicated checked-out connection.
/// Keeping the connection inside this guard makes release a consequence of
/// the guard's lifetime rather than a pooled one-shot query.
pub struct PostgresAdvisoryLockGuard<'a> {
    store: &'a PostgresKernelStore,
    pub(crate) connection: Option<PoolConnection<Postgres>>,
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

// Test schema cloning and cleanup are serialized because each operation owns
// the complete PostgreSQL catalog shape for one private schema.
pub(crate) const TEST_FIXTURE_ADVISORY_LOCK_KEY: i64 = 0x0000_5343_4c4f_4e45;

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

/// The owned synchronous PostgreSQL boundary for the authoritative schema.
pub struct PostgresKernelStore {
    pool: PgPool,
}

impl PostgresKernelStore {
    pub fn connect(url: &KernelDatabaseUrl) -> Result<Self, PostgresStoreError> {
        Self::connect_with_options(url, None, 8, true)
    }

    pub fn connect_in_schema(
        url: &KernelDatabaseUrl,
        schema: &str,
    ) -> Result<Self, PostgresStoreError> {
        Self::connect_in_schema_with_max_connections(url, schema, 8, true)
    }

    pub(crate) fn connect_for_test(url: &KernelDatabaseUrl) -> Result<Self, PostgresStoreError> {
        // Test administration connects to `template1` and to the source
        // database while materializing isolated fixtures. Those connections
        // deliberately bypass the application-schema guard; the scoped
        // fixture connection is checked by `ensure_test_template` and its
        // private-schema clone path before any test transition runs.
        Self::connect_with_options(url, None, 2, false)
    }

    pub(crate) fn connect_in_schema_for_test(
        url: &KernelDatabaseUrl,
        schema: &str,
    ) -> Result<Self, PostgresStoreError> {
        Self::connect_in_schema_with_max_connections(url, schema, 2, false)
    }

    fn connect_in_schema_with_max_connections(
        url: &KernelDatabaseUrl,
        schema: &str,
        max_connections: u32,
        validate_schema_revision: bool,
    ) -> Result<Self, PostgresStoreError> {
        if !is_safe_schema_name(schema) {
            return Err(PostgresStoreError::InvalidDatabaseUrl);
        }
        Self::connect_with_options(url, Some(schema), max_connections, validate_schema_revision)
    }

    fn connect_with_options(
        url: &KernelDatabaseUrl,
        schema: Option<&str>,
        max_connections: u32,
        validate_schema_revision: bool,
    ) -> Result<Self, PostgresStoreError> {
        let mut options = PgPoolOptions::new()
            .max_connections(max_connections)
            .min_connections(0)
            .acquire_timeout(Duration::from_secs(10));
        if let Some(schema) = schema {
            let statement = format!("SET search_path TO \"{schema}\"");
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
        let pool = async_std::task::block_on(options.connect_with(url.options()))
            .map_err(PostgresStoreError::Database)?;
        if validate_schema_revision {
            let actual = async_std::task::block_on(async {
                let row = sqlx::query(
                    "SELECT obj_description(namespace.oid, 'pg_namespace')
                       FROM pg_namespace AS namespace
                      WHERE namespace.nspname = current_schema()",
                )
                .fetch_one(&pool)
                .await
                .map_err(PostgresStoreError::Database)?;
                row.try_get::<Option<String>, _>(0)
                    .map_err(PostgresStoreError::Database)
            })?;
            if actual.as_deref() != Some(POSTGRES_SCHEMA_REVISION) {
                return Err(PostgresStoreError::SchemaRevisionMismatch {
                    expected: POSTGRES_SCHEMA_REVISION,
                    actual,
                });
            }
        }
        Ok(Self { pool })
    }

    pub fn pool_size(&self) -> u32 {
        self.pool.size()
    }

    pub fn catalog_snapshot(&self) -> Result<PostgresCatalogSnapshot, PostgresStoreError> {
        self.block_on(async {
            let row = sqlx::query(
                "SELECT
                    (SELECT obj_description(n.oid, 'pg_namespace')
                       FROM pg_namespace AS n WHERE n.nspname = current_schema()),
                    (SELECT COUNT(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = current_schema() AND c.relkind = 'r'),
                    (SELECT COUNT(*) FROM pg_constraint k JOIN pg_class c ON c.oid = k.conrelid JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = current_schema() AND k.contype = 'f'),
                    (SELECT COUNT(*) FROM pg_index i JOIN pg_class c ON c.oid = i.indrelid JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = current_schema() AND i.indpred IS NOT NULL),
                    (SELECT COUNT(*) FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = current_schema() AND NOT t.tgisinternal),
                    (SELECT COUNT(*) FROM pg_constraint k JOIN pg_class c ON c.oid = k.conrelid JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = current_schema() AND k.contype = 'c')",
            )
            .fetch_one(&self.pool)
            .await
            .map_err(PostgresStoreError::Database)?;
            Ok(PostgresCatalogSnapshot {
                schema_revision: row.get(0),
                table_count: row.get(1),
                foreign_key_count: row.get(2),
                partial_index_count: row.get(3),
                trigger_count: row.get(4),
                check_constraint_count: row.get(5),
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

    pub(crate) fn create_database_from_template(
        &self,
        database: &str,
        template: &str,
    ) -> Result<(), PostgresStoreError> {
        if !is_safe_schema_name(database) || !is_safe_schema_name(template) {
            return Err(PostgresStoreError::InvalidDatabaseUrl);
        }
        self.block_on(async {
            sqlx::query(AssertSqlSafe(
                format!("CREATE DATABASE \"{database}\" TEMPLATE \"{template}\"").as_str(),
            ))
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(PostgresStoreError::Database)
        })
    }

    pub(crate) fn drop_database(&self, database: &str) -> Result<(), PostgresStoreError> {
        if !is_safe_schema_name(database) {
            return Err(PostgresStoreError::InvalidDatabaseUrl);
        }
        self.block_on(async {
            sqlx::query(AssertSqlSafe(
                format!("DROP DATABASE IF EXISTS \"{database}\"").as_str(),
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
        async_std::task::block_on(future)
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
