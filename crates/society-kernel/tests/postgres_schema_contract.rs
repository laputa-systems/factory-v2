use society_kernel::{KernelDatabaseUrl, POSTGRES_SCHEMA_REVISION, PostgresKernelStore};

#[test]
fn catalog_contains_the_complete_named_postgres_contract() {
    let Ok(value) = std::env::var("SOCIETY_POSTGRES_TEST_URL") else {
        return;
    };
    let url = KernelDatabaseUrl::parse(&value).expect("valid PostgreSQL test URL");
    let admin = PostgresKernelStore::connect(&url).expect("connect PostgreSQL test database");
    let catalog = admin.catalog_snapshot().expect("read PostgreSQL catalog");
    assert_eq!(
        catalog.schema_revision.as_deref(),
        Some(POSTGRES_SCHEMA_REVISION)
    );
    assert_eq!(catalog.table_count, 374);
    assert_eq!(catalog.foreign_key_count, 917);
    assert_eq!(catalog.partial_index_count, 8);
    assert_eq!(catalog.trigger_count, 24);
    assert!(catalog.check_constraint_count > 100);
}
