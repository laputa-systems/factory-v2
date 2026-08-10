use society_kernel::{KernelDatabaseUrl, PostgresKernelStore};

#[test]
fn catalog_contains_the_complete_named_postgres_contract() {
    let Ok(value) = std::env::var("SOCIETY_POSTGRES_TEST_URL") else {
        return;
    };
    let url = KernelDatabaseUrl::parse(&value).expect("valid PostgreSQL test URL");
    let admin = PostgresKernelStore::connect(&url).expect("connect PostgreSQL test database");
    let schema = format!("society_contract_{}", std::process::id());
    admin
        .create_private_schema(&schema)
        .expect("create PostgreSQL contract schema");
    {
        let store = PostgresKernelStore::connect_in_schema(&url, &schema)
            .expect("connect PostgreSQL contract schema");
        store
            .migrate()
            .expect("apply PostgreSQL contract migration");
        let catalog = store.catalog_snapshot().expect("read PostgreSQL catalog");
        assert_eq!(catalog.table_count, 322);
        assert_eq!(catalog.foreign_key_count, 727);
        assert_eq!(catalog.partial_index_count, 8);
        assert_eq!(catalog.trigger_count, 24);
        assert!(catalog.check_constraint_count > 100);
    }
    admin
        .drop_private_schema(&schema)
        .expect("drop PostgreSQL contract schema");
}
