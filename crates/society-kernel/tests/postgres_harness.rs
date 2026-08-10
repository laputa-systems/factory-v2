use society_kernel::{KernelDatabaseUrl, PostgresKernelStore};

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
