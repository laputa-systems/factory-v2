//! Synchronous SQLx execution primitives used by the typed kernel transition
//! functions. This is a storage-boundary adapter, not a repository or a
//! generic row-mapping layer: callers still own their SQL, fixed row shapes,
//! and domain decoding at the transition site.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
    sync::{Mutex, OnceLock},
};

use sqlx::{
    AssertSqlSafe, ColumnIndex, Row as SqlxRow, TypeInfo, ValueRef as SqlxValueRef,
    pool::PoolConnection,
    postgres::{PgArguments, PgConnection, PgRow, Postgres},
    query,
    query::Query,
};

use crate::postgres::{PostgresKernelStore, TEST_FIXTURE_ADVISORY_LOCK_KEY};

// Identity columns are stable across every isolated test database because
// they all come from the one checked-in schema. Cache the reflection result
// once per process; querying information_schema before every INSERT is a
// measurable part of founding-cycle test cost.
static IDENTITY_COLUMNS: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

#[derive(Debug)]
pub enum DbError {
    Sqlx(sqlx::Error),
    NoRows,
}

impl fmt::Display for DbError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlx(error) => write!(formatter, "{error}"),
            Self::NoRows => formatter.write_str("query returned no rows"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<sqlx::Error> for DbError {
    fn from(error: sqlx::Error) -> Self {
        Self::Sqlx(error)
    }
}

/// Values emitted by the existing `params!` call sites. The null variants
/// retain the PostgreSQL type so a null argument is still typed by SQLx.
pub enum SqlValue {
    I64(i64),
    Text(String),
    Bytes(Vec<u8>),
    Bool(bool),
    NullI64,
    NullText,
    NullBytes,
}

pub trait ToSqlValue {
    fn to_sql_value(self) -> SqlValue;
    fn null_value() -> SqlValue;
}

impl ToSqlValue for i64 {
    fn to_sql_value(self) -> SqlValue {
        SqlValue::I64(self)
    }
    fn null_value() -> SqlValue {
        SqlValue::NullI64
    }
}
impl ToSqlValue for i32 {
    fn to_sql_value(self) -> SqlValue {
        SqlValue::I64(i64::from(self))
    }
    fn null_value() -> SqlValue {
        SqlValue::NullI64
    }
}
impl ToSqlValue for usize {
    fn to_sql_value(self) -> SqlValue {
        SqlValue::I64(self as i64)
    }
    fn null_value() -> SqlValue {
        SqlValue::NullI64
    }
}
impl ToSqlValue for bool {
    fn to_sql_value(self) -> SqlValue {
        SqlValue::Bool(self)
    }
    fn null_value() -> SqlValue {
        SqlValue::NullI64
    }
}
impl ToSqlValue for String {
    fn to_sql_value(self) -> SqlValue {
        SqlValue::Text(self)
    }
    fn null_value() -> SqlValue {
        SqlValue::NullText
    }
}
impl ToSqlValue for &str {
    fn to_sql_value(self) -> SqlValue {
        SqlValue::Text(self.to_owned())
    }
    fn null_value() -> SqlValue {
        SqlValue::NullText
    }
}
impl ToSqlValue for Vec<u8> {
    fn to_sql_value(self) -> SqlValue {
        SqlValue::Bytes(self)
    }
    fn null_value() -> SqlValue {
        SqlValue::NullBytes
    }
}
impl ToSqlValue for &[u8] {
    fn to_sql_value(self) -> SqlValue {
        SqlValue::Bytes(self.to_owned())
    }
    fn null_value() -> SqlValue {
        SqlValue::NullBytes
    }
}
impl<const N: usize> ToSqlValue for [u8; N] {
    fn to_sql_value(self) -> SqlValue {
        SqlValue::Bytes(self.to_vec())
    }
    fn null_value() -> SqlValue {
        SqlValue::NullBytes
    }
}
impl<T: ToSqlValue + Clone> ToSqlValue for &T {
    fn to_sql_value(self) -> SqlValue {
        self.clone().to_sql_value()
    }
    fn null_value() -> SqlValue {
        T::null_value()
    }
}
impl<T: ToSqlValue> ToSqlValue for Option<T> {
    fn to_sql_value(self) -> SqlValue {
        self.map(ToSqlValue::to_sql_value)
            .unwrap_or_else(T::null_value)
    }
    fn null_value() -> SqlValue {
        T::null_value()
    }
}

#[derive(Default)]
pub struct Params(pub Vec<SqlValue>);

pub trait IntoParams {
    fn into_params(self) -> Params;
}

impl IntoParams for Params {
    fn into_params(self) -> Params {
        self
    }
}
impl<T: ToSqlValue, const N: usize> IntoParams for [T; N] {
    fn into_params(self) -> Params {
        Params(self.into_iter().map(ToSqlValue::to_sql_value).collect())
    }
}

macro_rules! params {
    () => { $crate::postgres_db::Params::default() };
    ($($value:expr),+ $(,)?) => {
        $crate::postgres_db::Params(vec![$($crate::postgres_db::ToSqlValue::to_sql_value($value)),+])
    };
}
pub(crate) use params;

#[macro_export]
macro_rules! test_params {
    () => { $crate::postgres_db::Params::default() };
    ($($value:expr),+ $(,)?) => {
        $crate::postgres_db::Params(vec![$($crate::postgres_db::ToSqlValue::to_sql_value($value)),+])
    };
}

fn bind_params<'q>(
    mut statement: Query<'q, Postgres, PgArguments>,
    params: Params,
) -> Query<'q, Postgres, PgArguments> {
    for value in params.0 {
        statement = match value {
            SqlValue::I64(value) => statement.bind(value),
            SqlValue::Text(value) => statement.bind(value),
            SqlValue::Bytes(value) => statement.bind(value),
            SqlValue::Bool(value) => statement.bind(value),
            SqlValue::NullI64 => statement.bind(None::<i64>),
            SqlValue::NullText => statement.bind(None::<String>),
            SqlValue::NullBytes => statement.bind(None::<Vec<u8>>),
        };
    }
    statement
}

pub struct Row {
    inner: PgRow,
}

impl Row {
    pub fn get<I, T>(&self, index: I) -> Result<T, DbError>
    where
        I: ColumnIndex<PgRow> + Copy,
        T: FromSql,
    {
        T::from_column(&self.inner, index)
    }

    pub fn get_ref<I>(&self, index: I) -> Result<ValueRef<'_>, DbError>
    where
        I: ColumnIndex<PgRow> + Copy,
    {
        let raw = self.inner.try_get_raw(index).map_err(DbError::Sqlx)?;
        if raw.is_null() {
            return Ok(ValueRef::Null);
        }
        let bytes = raw
            .as_bytes()
            .map_err(|error| DbError::Sqlx(sqlx::Error::Decode(error)))?;
        match raw.type_info().name() {
            "INT2" | "INT4" | "INT8" => self.get(index).map(ValueRef::Integer),
            "FLOAT4" | "FLOAT8" => self.get(index).map(ValueRef::Real),
            "BYTEA" => Ok(ValueRef::Blob(bytes)),
            _ => Ok(ValueRef::Text(bytes)),
        }
    }
}

pub enum ValueRef<'a> {
    Null,
    Integer(i64),
    Real(f64),
    Text(&'a [u8]),
    Blob(&'a [u8]),
}

pub trait FromSql: Sized {
    fn from_row(row: &Row, index: usize) -> Result<Self, DbError>;

    fn from_column<I>(row: &PgRow, index: I) -> Result<Self, DbError>
    where
        I: ColumnIndex<PgRow> + Copy;
}

impl FromSql for i64 {
    fn from_row(row: &Row, index: usize) -> Result<Self, DbError> {
        row.get(index)
    }

    fn from_column<I>(row: &PgRow, index: I) -> Result<Self, DbError>
    where
        I: ColumnIndex<PgRow> + Copy,
    {
        match row.try_get(index) {
            Ok(value) => Ok(value),
            Err(_) => match row.try_get::<i32, _>(index) {
                Ok(value) => Ok(i64::from(value)),
                Err(_) => row
                    .try_get::<bool, _>(index)
                    .map(i64::from)
                    .map_err(DbError::Sqlx),
            },
        }
    }
}
impl FromSql for i32 {
    fn from_row(row: &Row, index: usize) -> Result<Self, DbError> {
        row.get(index)
    }

    fn from_column<I>(row: &PgRow, index: I) -> Result<Self, DbError>
    where
        I: ColumnIndex<PgRow> + Copy,
    {
        match row.try_get(index) {
            Ok(value) => Ok(value),
            Err(_) => row
                .try_get::<i64, _>(index)
                .and_then(|value| {
                    i32::try_from(value).map_err(|error| sqlx::Error::Decode(error.into()))
                })
                .map_err(DbError::Sqlx),
        }
    }
}
impl FromSql for String {
    fn from_row(row: &Row, index: usize) -> Result<Self, DbError> {
        row.get(index)
    }

    fn from_column<I>(row: &PgRow, index: I) -> Result<Self, DbError>
    where
        I: ColumnIndex<PgRow> + Copy,
    {
        row.try_get(index).map_err(DbError::Sqlx)
    }
}
impl FromSql for Vec<u8> {
    fn from_row(row: &Row, index: usize) -> Result<Self, DbError> {
        row.get(index)
    }

    fn from_column<I>(row: &PgRow, index: I) -> Result<Self, DbError>
    where
        I: ColumnIndex<PgRow> + Copy,
    {
        row.try_get(index).map_err(DbError::Sqlx)
    }
}
impl<T> FromSql for Option<T>
where
    T: FromSql,
{
    fn from_row(row: &Row, index: usize) -> Result<Self, DbError> {
        let raw = row.inner.try_get_raw(index).map_err(DbError::Sqlx)?;
        if raw.is_null() {
            Ok(None)
        } else {
            T::from_column(&row.inner, index).map(Some)
        }
    }

    fn from_column<I>(row: &PgRow, index: I) -> Result<Self, DbError>
    where
        I: ColumnIndex<PgRow> + Copy,
    {
        let raw = row.try_get_raw(index).map_err(DbError::Sqlx)?;
        if raw.is_null() {
            Ok(None)
        } else {
            T::from_column(row, index).map(Some)
        }
    }
}

impl FromSql for bool {
    fn from_row(row: &Row, index: usize) -> Result<Self, DbError> {
        row.get(index)
    }

    fn from_column<I>(row: &PgRow, index: I) -> Result<Self, DbError>
    where
        I: ColumnIndex<PgRow> + Copy,
    {
        match row.try_get(index) {
            Ok(value) => Ok(value),
            Err(_) => row
                .try_get::<i64, _>(index)
                .map(|value| value != 0)
                .map_err(DbError::Sqlx),
        }
    }
}

impl FromSql for f64 {
    fn from_row(row: &Row, index: usize) -> Result<Self, DbError> {
        row.get(index)
    }

    fn from_column<I>(row: &PgRow, index: I) -> Result<Self, DbError>
    where
        I: ColumnIndex<PgRow> + Copy,
    {
        row.try_get(index).map_err(DbError::Sqlx)
    }
}

pub trait OptionalExtension<T> {
    fn optional(self) -> Result<Option<T>, DbError>;
}

impl<T> OptionalExtension<T> for Result<T, DbError> {
    fn optional(self) -> Result<Option<T>, DbError> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(DbError::NoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

pub struct Connection {
    backend: PostgresKernelStore,
    #[allow(dead_code)]
    cleanup: Option<TestCleanup>,
}

enum TestCleanup {
    Database {
        admin: PostgresKernelStore,
        database: String,
    },
}

impl Drop for TestCleanup {
    fn drop(&mut self) {
        match self {
            Self::Database { admin, database } => {
                if let Ok(_lock) = admin.acquire_advisory_lock(TEST_FIXTURE_ADVISORY_LOCK_KEY) {
                    let _ = admin.drop_database(database);
                }
            }
        }
    }
}

const TEST_TEMPLATE_DATABASE: &str = "society_test_template";
static NEXT_TEST_DATABASE: AtomicU64 = AtomicU64::new(1);
static TEST_TEMPLATE_READY: OnceLock<()> = OnceLock::new();
static TEST_TEMPLATE_INIT: OnceLock<Mutex<()>> = OnceLock::new();

/// Derive the deterministic private schema used by path-oriented test
/// fixtures. The path is only a stable fixture identity; no filesystem
/// database is opened.
pub fn test_schema_for_path(path: impl AsRef<std::path::Path>) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.as_ref().to_string_lossy().hash(&mut hasher);
    format!("society_test_path_{:016x}", hasher.finish())
}

fn ensure_test_template(
    url: &crate::postgres::KernelDatabaseUrl,
) -> Result<crate::postgres::KernelDatabaseUrl, DbError> {
    if TEST_TEMPLATE_READY.get().is_none() {
        let initializer = TEST_TEMPLATE_INIT.get_or_init(|| Mutex::new(())).lock();
        let _initializer = initializer.unwrap_or_else(|poisoned| poisoned.into_inner());
        if TEST_TEMPLATE_READY.get().is_none() {
            let source_database = url
                .options()
                .get_database()
                .unwrap_or("postgres")
                .to_owned();
            let admin_url = url.with_database("template1");
            let admin = PostgresKernelStore::connect(&admin_url)
                .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
            let lock = admin
                .acquire_advisory_lock(TEST_FIXTURE_ADVISORY_LOCK_KEY)
                .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
            let exists = admin
                .block_on(async {
                    sqlx::query("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)")
                        .bind(TEST_TEMPLATE_DATABASE)
                        .fetch_one(admin.pool())
                        .await
                })
                .map(|row| sqlx::Row::try_get::<bool, _>(&row, 0).unwrap_or(false))
                .map_err(DbError::Sqlx)?;
            if !exists {
                admin
                    .create_database_from_template(TEST_TEMPLATE_DATABASE, &source_database)
                    .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
            }
            drop(lock);
            let _ = TEST_TEMPLATE_READY.set(());
        }
    }
    Ok(url.with_database(TEST_TEMPLATE_DATABASE))
}

/// Materialize an empty test schema from the authoritative public schema.
/// PostgreSQL's `LIKE INCLUDING ALL` is substantially cheaper than replaying
/// the complete canonical DDL for every test case; foreign keys and triggers
/// are installed afterward because `LIKE` does not copy them.
fn clone_fresh_test_schema(admin: &PostgresKernelStore, schema: &str) -> Result<(), DbError> {
    let mut lock = admin
        .acquire_advisory_lock(TEST_FIXTURE_ADVISORY_LOCK_KEY)
        .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
    let clone_sql = format!(
        r#"
DO $society_clone$
DECLARE
    target_schema text := {schema_literal};
    table_row record;
    identity_row record;
    foreign_key_row record;
    trigger_row record;
    next_value bigint;
BEGIN
    FOR table_row IN
        SELECT tablename
        FROM pg_catalog.pg_tables
        WHERE schemaname = 'public'
        ORDER BY tablename
    LOOP
        EXECUTE format(
            'CREATE TABLE %I.%I (LIKE public.%I INCLUDING ALL)',
            target_schema, table_row.tablename, table_row.tablename
        );
        EXECUTE format(
            'INSERT INTO %I.%I SELECT * FROM public.%I',
            target_schema, table_row.tablename, table_row.tablename
        );
    END LOOP;

    FOR identity_row IN
        SELECT table_name, column_name
        FROM information_schema.columns
        WHERE table_schema = target_schema AND is_identity = 'YES'
        ORDER BY table_name, column_name
    LOOP
        EXECUTE format(
            'SELECT COALESCE(MAX(%I), 0) + 1 FROM %I.%I',
            identity_row.column_name, target_schema, identity_row.table_name
        ) INTO next_value;
        EXECUTE format(
            'ALTER TABLE %I.%I ALTER COLUMN %I RESTART WITH %s',
            target_schema, identity_row.table_name, identity_row.column_name, next_value
        );
    END LOOP;

    PERFORM set_config('search_path', format('%I, public', target_schema), true);

    FOR foreign_key_row IN
        SELECT child.relname AS table_name,
               constraint_row.conname AS constraint_name,
               pg_get_constraintdef(constraint_row.oid) AS definition
        FROM pg_catalog.pg_constraint constraint_row
        JOIN pg_catalog.pg_class child ON child.oid = constraint_row.conrelid
        JOIN pg_catalog.pg_namespace child_schema ON child_schema.oid = child.relnamespace
        WHERE constraint_row.contype = 'f' AND child_schema.nspname = 'public'
        ORDER BY constraint_row.oid
    LOOP
        EXECUTE format(
            'ALTER TABLE %I.%I ADD CONSTRAINT %I %s',
            target_schema,
            foreign_key_row.table_name,
            foreign_key_row.constraint_name,
            CASE
                WHEN position('REFERENCES public.' IN foreign_key_row.definition) > 0 THEN
                    replace(
                        foreign_key_row.definition,
                        'REFERENCES public.',
                        format('REFERENCES %I.', target_schema)
                    )
                ELSE
                    replace(
                        foreign_key_row.definition,
                        'REFERENCES ',
                        format('REFERENCES %I.', target_schema)
                    )
            END
        );
    END LOOP;

    FOR trigger_row IN
        SELECT pg_get_triggerdef(trigger_catalog_row.oid) AS definition
        FROM pg_catalog.pg_trigger trigger_catalog_row
        JOIN pg_catalog.pg_class class_row ON class_row.oid = trigger_catalog_row.tgrelid
        JOIN pg_catalog.pg_namespace namespace_row ON namespace_row.oid = class_row.relnamespace
        WHERE NOT trigger_catalog_row.tgisinternal AND namespace_row.nspname = 'public'
        ORDER BY trigger_catalog_row.oid
    LOOP
        EXECUTE replace(
            trigger_row.definition,
            ' ON public.',
            format(' ON %I.', target_schema)
        );
    END LOOP;
END
$society_clone$;
"#,
        schema_literal = sql_literal(schema),
    );
    admin.block_on(async {
        let connection = lock
            .connection
            .as_mut()
            .expect("test schema clone lock owns its PostgreSQL connection");
        sqlx::query(AssertSqlSafe(clone_sql.as_str()))
            .execute(&mut **connection)
            .await?;
        Ok::<(), sqlx::Error>(())
    })?;
    Ok(())
}

fn private_schema_exists(admin: &PostgresKernelStore, schema: &str) -> Result<bool, DbError> {
    admin
        .block_on(async {
            sqlx::query("SELECT EXISTS (SELECT 1 FROM pg_namespace WHERE nspname = $1)")
                .bind(schema)
                .fetch_one(admin.pool())
                .await
        })
        .map(|row| sqlx::Row::try_get::<bool, _>(&row, 0).unwrap_or(false))
        .map_err(DbError::Sqlx)
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn insert_table_name(sql: &str) -> Option<String> {
    let statement = sql.trim_start();
    let remainder = statement
        .strip_prefix("INSERT INTO")
        .or_else(|| statement.strip_prefix("insert into"))?
        .trim_start();
    let end = remainder
        .find(|character: char| character.is_ascii_whitespace() || character == '(')
        .unwrap_or(remainder.len());
    let table = remainder[..end].trim_matches('"');
    (!table.is_empty()).then(|| table.to_owned())
}

/// Clone one test schema into another private PostgreSQL schema.
pub fn clone_test_schema(
    source_path: impl AsRef<std::path::Path>,
    destination_path: impl AsRef<std::path::Path>,
) -> Result<(), DbError> {
    let url = crate::postgres::KernelDatabaseUrl::from_env("SOCIETY_POSTGRES_TEST_URL")
        .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
    let source_schema = test_schema_for_path(source_path);
    let destination_schema = test_schema_for_path(destination_path);
    let admin = PostgresKernelStore::connect_for_test(&url)
        .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
    let source = PostgresKernelStore::connect_in_schema_for_test(&url, &source_schema)
        .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
    let _ = admin.drop_private_schema(&destination_schema);
    admin
        .create_private_schema(&destination_schema)
        .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
    let tables = source.block_on(async {
        sqlx::query(
            "SELECT tablename FROM pg_catalog.pg_tables
             WHERE schemaname = current_schema() ORDER BY tablename",
        )
        .fetch_all(source.pool())
        .await
    })?;
    for table in tables {
        let table = table.try_get::<String, _>("tablename")?;
        let source_table = format!("{source_schema}.{}", quote_identifier(&table));
        let destination_table = format!("{destination_schema}.{}", quote_identifier(&table));
        admin.block_on(async {
            sqlx::query(AssertSqlSafe(
                format!(
                    "CREATE TABLE {destination_table} (LIKE {source_table} INCLUDING DEFAULTS INCLUDING GENERATED INCLUDING IDENTITY)"
                )
                .as_str(),
            ))
            .execute(admin.pool())
            .await?;
            sqlx::query(AssertSqlSafe(
                format!("INSERT INTO {destination_table} SELECT * FROM {source_table}").as_str(),
            ))
            .execute(admin.pool())
            .await
        })?;
    }
    Ok(())
}

impl Connection {
    pub fn connect(
        url: &crate::postgres::KernelDatabaseUrl,
    ) -> Result<Self, crate::postgres::PostgresStoreError> {
        Ok(Self {
            backend: PostgresKernelStore::connect_for_test(url)?,
            cleanup: None,
        })
    }

    pub fn connect_in_schema(
        url: &crate::postgres::KernelDatabaseUrl,
        schema: &str,
    ) -> Result<Self, crate::postgres::PostgresStoreError> {
        Ok(Self {
            backend: PostgresKernelStore::connect_in_schema(url, schema)?,
            cleanup: None,
        })
    }

    pub(crate) fn connect_test_with_url(
        url: &crate::postgres::KernelDatabaseUrl,
    ) -> Result<Self, DbError> {
        ensure_test_template(url)?;
        let ordinal = NEXT_TEST_DATABASE.fetch_add(1, Ordering::Relaxed);
        let database = format!("society_test_db_{}_{}", std::process::id(), ordinal);
        let admin = PostgresKernelStore::connect_for_test(&url.with_database("template1"))
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
        admin
            .create_database_from_template(&database, TEST_TEMPLATE_DATABASE)
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
        let database_url = url.with_database(&database);
        let scoped = PostgresKernelStore::connect_for_test(&database_url)
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
        Ok(Self {
            backend: scoped,
            cleanup: Some(TestCleanup::Database { admin, database }),
        })
    }

    pub fn connect_test() -> Result<Self, DbError> {
        let url = crate::postgres::KernelDatabaseUrl::from_env("SOCIETY_POSTGRES_TEST_URL")
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
        Self::connect_test_with_url(&url)
    }

    pub(crate) fn connect_test_path_with_url(
        _path: impl AsRef<std::path::Path>,
        url: &crate::postgres::KernelDatabaseUrl,
    ) -> Result<Self, DbError> {
        ensure_test_template(url)?;
        let schema = test_schema_for_path(_path);
        let admin = PostgresKernelStore::connect_for_test(url)
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
        if !private_schema_exists(&admin, &schema)? {
            admin
                .create_private_schema(&schema)
                .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
            clone_fresh_test_schema(&admin, &schema)?;
        }
        let scoped = PostgresKernelStore::connect_in_schema_for_test(url, &schema)
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
        Ok(Self {
            backend: scoped,
            cleanup: None,
        })
    }

    pub fn connect_test_path(_path: impl AsRef<std::path::Path>) -> Result<Self, DbError> {
        let url = crate::postgres::KernelDatabaseUrl::from_env("SOCIETY_POSTGRES_TEST_URL")
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(Box::new(error))))?;
        Self::connect_test_path_with_url(_path, &url)
    }

    pub fn query_row<P, F, T>(&self, sql: &str, params: P, mapper: F) -> Result<T, DbError>
    where
        P: IntoParams,
        F: FnOnce(&Row) -> Result<T, DbError>,
    {
        self.backend.block_on(async {
            let statement = bind_params(query(AssertSqlSafe(sql)), params.into_params());
            let row = statement
                .fetch_optional(self.backend.pool())
                .await?
                .ok_or(DbError::NoRows)?;
            mapper(&Row { inner: row })
        })
    }

    pub fn execute<P>(&self, sql: &str, params: P) -> Result<usize, DbError>
    where
        P: IntoParams,
    {
        self.backend.block_on(async {
            bind_params(query(AssertSqlSafe(sql)), params.into_params())
                .execute(self.backend.pool())
                .await
                .map(|result| result.rows_affected() as usize)
                .map_err(DbError::Sqlx)
        })
    }

    pub fn execute_batch(&self, sql: &str) -> Result<(), DbError> {
        for statement in sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            self.execute(statement, Params::default())?;
        }
        Ok(())
    }

    pub fn prepare(&self, sql: &str) -> Result<Statement<'_>, DbError> {
        Ok(Statement {
            source: StatementSource::Connection(self),
            sql: sql.to_owned(),
        })
    }

    pub(crate) fn ordered_table_scan(&self, table: &str) -> Result<(usize, String), DbError> {
        let rows = self.backend.block_on(async {
            sqlx::query(
                "SELECT column_name FROM information_schema.columns
                 WHERE table_schema = current_schema() AND table_name = $1
                 ORDER BY ordinal_position",
            )
            .bind(table)
            .fetch_all(self.backend.pool())
            .await
        })?;
        let columns = rows
            .iter()
            .map(|row| {
                row.try_get::<String, _>("column_name")
                    .map(|name| format!("\"{}\" ASC NULLS FIRST", name.replace('"', "\"\"")))
                    .map_err(DbError::Sqlx)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if columns.is_empty() {
            return Err(DbError::Sqlx(sqlx::Error::Configuration(
                format!("unknown table {table}").into(),
            )));
        }
        Ok((columns.len(), columns.join(", ")))
    }

    pub fn unchecked_transaction(&self) -> Result<Transaction<'_>, DbError> {
        let connection = self
            .backend
            .block_on(async { self.backend.pool().acquire().await })?;
        let transaction = Transaction {
            backend: &self.backend,
            connection: RefCell::new(Some(connection)),
            active: Cell::new(true),
            returned_identity: Cell::new(None),
            _marker: PhantomData,
        };
        transaction.execute("BEGIN", Params::default())?;
        Ok(transaction)
    }

    pub fn transaction_with_behavior(
        &self,
        _behavior: TransactionBehavior,
    ) -> Result<Transaction<'_>, DbError> {
        self.unchecked_transaction()
    }
}

pub struct Transaction<'a> {
    backend: &'a PostgresKernelStore,
    connection: RefCell<Option<PoolConnection<Postgres>>>,
    active: Cell<bool>,
    returned_identity: Cell<Option<i64>>,
    _marker: PhantomData<&'a mut PgConnection>,
}

impl Transaction<'_> {
    pub fn query_row<P, F, T>(&self, sql: &str, params: P, mapper: F) -> Result<T, DbError>
    where
        P: IntoParams,
        F: FnOnce(&Row) -> Result<T, DbError>,
    {
        let backend = self.backend;
        let mut connection = self.connection.borrow_mut();
        let connection = connection.as_mut().ok_or(DbError::NoRows)?;
        backend.block_on(async {
            let statement = bind_params(query(AssertSqlSafe(sql)), params.into_params());
            let row = statement
                .fetch_optional(&mut **connection)
                .await?
                .ok_or(DbError::NoRows)?;
            mapper(&Row { inner: row })
        })
    }

    pub fn execute<P>(&self, sql: &str, params: P) -> Result<usize, DbError>
    where
        P: IntoParams,
    {
        let backend = self.backend;
        let mut connection = self.connection.borrow_mut();
        let connection = connection.as_mut().ok_or(DbError::NoRows)?;
        backend
            .block_on(async {
                let insert_table = insert_table_name(sql);
                let identity_column = if let Some(table) = insert_table.as_deref() {
                    let cache = IDENTITY_COLUMNS.get_or_init(|| Mutex::new(HashMap::new()));
                    if let Some(cached) = cache
                        .lock()
                        .expect("identity-column cache lock poisoned")
                        .get(table)
                        .cloned()
                    {
                        cached
                    } else {
                        let discovered = sqlx::query(
                            "SELECT column_name FROM information_schema.columns
                                 WHERE table_schema = current_schema()
                                   AND table_name = $1 AND is_identity = 'YES'
                                 ORDER BY ordinal_position LIMIT 1",
                        )
                        .bind(table)
                        .fetch_optional(&mut **connection)
                        .await?
                        .map(|row| row.try_get::<String, _>(0))
                        .transpose()?;
                        cache
                            .lock()
                            .expect("identity-column cache lock poisoned")
                            .insert(table.to_owned(), discovered.clone());
                        discovered
                    }
                } else {
                    None
                };
                if let Some(identity_column) = identity_column {
                    let returning_sql =
                        format!("{sql} RETURNING {}", quote_identifier(&identity_column));
                    let row = bind_params(
                        query(AssertSqlSafe(returning_sql.as_str())),
                        params.into_params(),
                    )
                    .fetch_optional(&mut **connection)
                    .await?;
                    let Some(row) = row else {
                        self.returned_identity.set(None);
                        return Ok(0);
                    };
                    let identity = row.try_get::<i64, _>(0)?;
                    self.returned_identity.set(Some(identity));
                    Ok(1)
                } else {
                    if insert_table.is_some() {
                        self.returned_identity.set(None);
                    }
                    bind_params(query(AssertSqlSafe(sql)), params.into_params())
                        .execute(&mut **connection)
                        .await
                        .map(|result| result.rows_affected() as usize)
                }
            })
            .map_err(DbError::Sqlx)
    }

    pub fn execute_batch(&self, sql: &str) -> Result<(), DbError> {
        for statement in sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            self.execute(statement, Params::default())?;
        }
        Ok(())
    }

    pub fn prepare(&self, sql: &str) -> Result<Statement<'_>, DbError> {
        Ok(Statement {
            source: StatementSource::Transaction(self),
            sql: sql.to_owned(),
        })
    }

    pub fn commit(&self) -> Result<(), DbError> {
        self.execute("COMMIT", Params::default())?;
        self.active.set(false);
        let connection = self.connection.borrow_mut().take();
        self.backend.block_on(async move {
            drop(connection);
        });
        Ok(())
    }

    pub fn returned_identity(&self) -> Result<i64, DbError> {
        self.returned_identity.get().ok_or(DbError::NoRows)
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if self.active.get()
            && let Some(connection) = self.connection.borrow_mut().take()
        {
            let _ = self.backend.block_on(async move {
                let mut connection = connection;
                let result = sqlx::query("ROLLBACK").execute(&mut *connection).await;
                drop(connection);
                result
            });
        }
    }
}

#[derive(Clone, Copy)]
pub enum TransactionBehavior {
    Immediate,
}

pub struct Statement<'a> {
    source: StatementSource<'a>,
    sql: String,
}

enum StatementSource<'a> {
    Connection(&'a Connection),
    Transaction(&'a Transaction<'a>),
}

impl Statement<'_> {
    pub fn query_map<P, F, T>(&mut self, params: P, mapper: F) -> Result<MappedRows<T>, DbError>
    where
        P: IntoParams,
        F: Fn(&Row) -> Result<T, DbError>,
    {
        let rows = self.query_raw(params)?;
        Ok(MappedRows {
            rows: rows
                .into_iter()
                .map(|row| mapper(&row))
                .collect::<Vec<_>>()
                .into_iter(),
        })
    }

    pub fn query<P>(&mut self, params: P) -> Result<RawRows, DbError>
    where
        P: IntoParams,
    {
        Ok(RawRows {
            rows: self.query_raw(params)?.into_iter(),
            _marker: PhantomData,
        })
    }

    fn query_raw<P>(&self, params: P) -> Result<Vec<Row>, DbError>
    where
        P: IntoParams,
    {
        let sql = self.sql.as_str();
        match self.source {
            StatementSource::Connection(connection) => connection.backend.block_on(async {
                let statement = bind_params(query(AssertSqlSafe(sql)), params.into_params());
                let rows = statement.fetch_all(connection.backend.pool()).await?;
                Ok(rows.into_iter().map(|inner| Row { inner }).collect())
            }),
            StatementSource::Transaction(transaction) => {
                let mut connection = transaction.connection.borrow_mut();
                let connection = connection.as_mut().ok_or(DbError::NoRows)?;
                transaction.backend.block_on(async {
                    let statement = bind_params(query(AssertSqlSafe(sql)), params.into_params());
                    let rows = statement.fetch_all(&mut **connection).await?;
                    Ok(rows.into_iter().map(|inner| Row { inner }).collect())
                })
            }
        }
    }

    pub fn column_count(&self) -> usize {
        0
    }
}

pub struct MappedRows<T> {
    rows: std::vec::IntoIter<Result<T, DbError>>,
}

impl<T> Iterator for MappedRows<T> {
    type Item = Result<T, DbError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.rows.next()
    }
}

pub struct RawRows {
    rows: std::vec::IntoIter<Row>,
    _marker: PhantomData<()>,
}

impl RawRows {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<Row>, DbError> {
        Ok(self.rows.next())
    }
}
