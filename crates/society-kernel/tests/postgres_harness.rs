use std::sync::atomic::{AtomicU64, Ordering};

use society_kernel::{KernelDatabaseUrl, PostgresKernelStore};

static NEXT_SCHEMA: AtomicU64 = AtomicU64::new(1);

fn test_url() -> Option<KernelDatabaseUrl> {
    let value = std::env::var("SOCIETY_POSTGRES_TEST_URL").ok()?;
    Some(
        KernelDatabaseUrl::parse(&value)
            .unwrap_or_else(|error| panic!("SOCIETY_POSTGRES_TEST_URL is invalid: {error}")),
    )
}

fn fresh_schema() -> String {
    format!(
        "society_test_{}_{}",
        std::process::id(),
        NEXT_SCHEMA.fetch_add(1, Ordering::Relaxed)
    )
}

#[test]
fn fresh_and_second_migration_are_idempotent() {
    let Some(url) = test_url() else { return };
    let admin = PostgresKernelStore::connect(&url).expect("connect PostgreSQL test database");
    let schema = fresh_schema();
    admin
        .create_private_schema(&schema)
        .expect("create private PostgreSQL test schema");
    {
        let store = PostgresKernelStore::connect_in_schema(&url, &schema)
            .expect("connect private PostgreSQL test schema");
        store.migrate().expect("apply fresh PostgreSQL migration");
        store.migrate().expect("reapply PostgreSQL migration");
        let snapshot = store.catalog_snapshot().expect("read PostgreSQL catalog");
        assert_eq!(snapshot.table_count, 322);
        assert_eq!(snapshot.foreign_key_count, 727);
        assert_eq!(snapshot.partial_index_count, 8);
        assert_eq!(snapshot.trigger_count, 24);
        assert_eq!(snapshot.migration_count, 1);
        assert!(snapshot.check_constraint_count > 100);
    }
    admin
        .drop_private_schema(&schema)
        .expect("drop private PostgreSQL test schema");
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
