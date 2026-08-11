use std::time::{SystemTime, UNIX_EPOCH};

use society_kernel::{
    KernelDatabaseUrl, KernelStore, POSTGRES_SCHEMA_REVISION, PostgresKernelStore,
    PostgresStoreError,
};

fn test_url() -> Option<KernelDatabaseUrl> {
    let value = std::env::var("SOCIETY_POSTGRES_TEST_URL").ok()?;
    Some(
        KernelDatabaseUrl::parse(&value)
            .unwrap_or_else(|error| panic!("SOCIETY_POSTGRES_TEST_URL is invalid: {error}")),
    )
}

#[test]
fn dedicated_advisory_lock_excludes_another_store() {
    let Some(url) = test_url() else { return };
    let first = PostgresKernelStore::connect(&url).expect("connect first PostgreSQL store");
    let second = PostgresKernelStore::connect(&url).expect("connect second PostgreSQL store");
    let key = 0x0053_4f43_4945_5459_i64;
    let first_guard = first
        .acquire_advisory_lock(key)
        .expect("acquire first dedicated advisory lock");
    assert!(
        second
            .try_acquire_advisory_lock(key)
            .expect("probe second dedicated advisory lock")
            .is_none()
    );
    drop(first_guard);
    let second_guard = second
        .try_acquire_advisory_lock(key)
        .expect("acquire released dedicated advisory lock")
        .expect("lock should be released with the dedicated connection");
    drop(second_guard);
}

#[test]
fn production_connection_rejects_unbootstrapped_schema() {
    let Some(url) = test_url() else { return };
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos();
    let schema = format!("society_schema_guard_{nonce}");
    let admin = PostgresKernelStore::connect(&url).expect("connect PostgreSQL test database");
    admin
        .create_private_schema(&schema)
        .expect("create unbootstrapped schema");
    let error = match PostgresKernelStore::connect_in_schema(&url, &schema) {
        Ok(_) => panic!("unbootstrapped schema must not be accepted"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        PostgresStoreError::SchemaRevisionMismatch {
            expected: POSTGRES_SCHEMA_REVISION,
            actual: None,
        }
    ));
    admin
        .drop_private_schema(&schema)
        .expect("drop unbootstrapped schema");
}

#[test]
fn test_schema_clone_preserves_postgres_contract_objects() {
    let Some(url) = test_url() else { return };
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos();
    let source_path = std::env::temp_dir().join(format!("society-clone-source-{nonce}"));
    let destination_path = std::env::temp_dir().join(format!("society-clone-destination-{nonce}"));
    let source_schema = KernelStore::test_schema_for_path(&source_path);
    let destination_schema = KernelStore::test_schema_for_path(&destination_path);

    let source = KernelStore::connect_test_path(&source_path).expect("create source fixture");
    drop(source);
    society_kernel::postgres_db::clone_test_schema(&source_path, &destination_path)
        .expect("clone source fixture");

    let source_catalog = PostgresKernelStore::connect_in_schema(&url, &source_schema)
        .expect("connect source schema")
        .catalog_snapshot()
        .expect("read source catalog");
    assert_eq!(
        source_catalog.schema_revision.as_deref(),
        Some(POSTGRES_SCHEMA_REVISION)
    );

    let admin = PostgresKernelStore::connect(&url).expect("connect PostgreSQL test database");
    // A fork must own its trigger functions.  Dropping the source schema is
    // the migration-sensitive judge: a destination trigger still bound to a
    // source function would be removed by PostgreSQL's dependency cascade.
    admin
        .drop_private_schema(&source_schema)
        .expect("drop source schema after clone");
    let destination_catalog = PostgresKernelStore::connect_in_schema(&url, &destination_schema)
        .expect("connect destination schema after source removal")
        .catalog_snapshot()
        .expect("read destination catalog after source removal");
    assert_eq!(destination_catalog, source_catalog);
    assert!(destination_catalog.foreign_key_count > 0);
    assert!(destination_catalog.trigger_count > 0);

    admin
        .drop_private_schema(&destination_schema)
        .expect("drop destination schema");
}
