//! Synchronous SQLx execution primitives used by the typed kernel transition
//! functions. This is a storage-boundary adapter, not a repository or a
//! generic row-mapping layer: callers still own their SQL, fixed row shapes,
//! and domain decoding at the transition site.

use std::{
    cell::{Cell, RefCell},
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use sqlx::{
    AssertSqlSafe, ColumnIndex, Row as SqlxRow, TypeInfo, ValueRef as SqlxValueRef,
    pool::PoolConnection,
    postgres::{PgArguments, PgConnection, PgRow, Postgres},
    query,
    query::Query,
};

use crate::postgres::PostgresKernelStore;

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
    () => { $crate::postgres_compat::Params::default() };
    ($($value:expr),+ $(,)?) => {
        $crate::postgres_compat::Params(vec![$($crate::postgres_compat::ToSqlValue::to_sql_value($value)),+])
    };
}
pub(crate) use params;

#[macro_export]
macro_rules! test_params {
    () => { $crate::postgres_compat::Params::default() };
    ($($value:expr),+ $(,)?) => {
        $crate::postgres_compat::Params(vec![$($crate::postgres_compat::ToSqlValue::to_sql_value($value)),+])
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
    cleanup: Option<TestSchemaCleanup>,
}

struct TestSchemaCleanup {
    admin: PostgresKernelStore,
    schema: String,
}

impl Drop for TestSchemaCleanup {
    fn drop(&mut self) {
        let _ = self.admin.drop_private_schema(&self.schema);
    }
}

static NEXT_TEST_SCHEMA: AtomicU64 = AtomicU64::new(1);

/// Derive the deterministic private schema used by path-oriented test
/// fixtures. The path is only a stable fixture identity; no filesystem
/// database is opened.
pub fn test_schema_for_path(path: impl AsRef<std::path::Path>) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.as_ref().to_string_lossy().hash(&mut hasher);
    format!("society_test_path_{:016x}", hasher.finish())
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
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

/// Clone a path-backed compatibility database into another private PostgreSQL
/// schema. This is a test-only bridge while filesystem snapshots are moved to
/// database-owned snapshots; it is not a production backup mechanism.
pub fn clone_for_test(
    source_path: impl AsRef<std::path::Path>,
    destination_path: impl AsRef<std::path::Path>,
) -> Result<(), DbError> {
    let url = crate::postgres::KernelDatabaseUrl::from_env("SOCIETY_POSTGRES_TEST_URL")
        .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
    let source_schema = test_schema_for_path(source_path);
    let destination_schema = test_schema_for_path(destination_path);
    let admin = PostgresKernelStore::connect(&url)
        .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
    let source = PostgresKernelStore::connect_in_schema(&url, &source_schema)
        .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
    let _ = admin.drop_private_schema(&destination_schema);
    admin
        .create_private_schema(&destination_schema)
        .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
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
            backend: PostgresKernelStore::connect(url)?,
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
        let ordinal = NEXT_TEST_SCHEMA.fetch_add(1, Ordering::Relaxed);
        let schema = format!("society_test_{}_{}", std::process::id(), ordinal);
        let admin = PostgresKernelStore::connect(url)
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
        admin
            .create_private_schema(&schema)
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
        let scoped = PostgresKernelStore::connect_in_schema(url, &schema)
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
        scoped
            .migrate()
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
        Ok(Self {
            backend: scoped,
            cleanup: Some(TestSchemaCleanup { admin, schema }),
        })
    }

    pub fn connect_test() -> Result<Self, DbError> {
        let url = crate::postgres::KernelDatabaseUrl::from_env("SOCIETY_POSTGRES_TEST_URL")
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
        Self::connect_test_with_url(&url)
    }

    pub(crate) fn connect_test_path_with_url(
        _path: impl AsRef<std::path::Path>,
        url: &crate::postgres::KernelDatabaseUrl,
    ) -> Result<Self, DbError> {
        let schema = test_schema_for_path(_path);
        let admin = PostgresKernelStore::connect(url)
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
        admin
            .ensure_private_schema(&schema)
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
        let scoped = PostgresKernelStore::connect_in_schema(url, &schema)
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
        scoped
            .migrate()
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
        Ok(Self {
            backend: scoped,
            cleanup: None,
        })
    }

    pub fn connect_test_path(_path: impl AsRef<std::path::Path>) -> Result<Self, DbError> {
        let url = crate::postgres::KernelDatabaseUrl::from_env("SOCIETY_POSTGRES_TEST_URL")
            .map_err(|error| DbError::Sqlx(sqlx::Error::Configuration(error.to_string().into())))?;
        Self::connect_test_path_with_url(_path, &url)
    }

    pub fn migrate(&self) -> Result<(), crate::postgres::PostgresStoreError> {
        self.backend.migrate()
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
                    sqlx::query(
                        "SELECT column_name FROM information_schema.columns
                             WHERE table_schema = current_schema()
                               AND table_name = $1 AND is_identity = 'YES'
                             ORDER BY ordinal_position LIMIT 1",
                    )
                    .bind(table)
                    .fetch_optional(&mut **connection)
                    .await?
                    .map(|row| row.try_get::<String, _>(0))
                    .transpose()?
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
