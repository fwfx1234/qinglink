use super::*;
use qinglink_storage::{
    AppPaths,
    database::{DatabaseService, DatabaseSpec},
};

fn store() -> (TokenStore, tempfile::TempDir) {
    let directory = tempfile::tempdir().expect("tempdir");
    let database = std::sync::Arc::new(DatabaseService::new(AppPaths::for_test(directory.path())));
    database
        .register_database(DatabaseSpec::app(DB_KEY, "qinglink.db"))
        .expect("register");
    (TokenStore::new(&database), directory)
}

#[test]
fn stores_only_hash_and_validates_token() {
    let (store, _directory) = store();
    let token = store.create_permanent_token("phone").expect("token");
    assert!(store.validate(&token));
    let conn = store.connection().expect("connection");
    let raw_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tokens_v2 WHERE token_hash = ?1",
            [&token],
            |row| row.get(0),
        )
        .expect("query");
    assert_eq!(raw_count, 0);
}

#[test]
fn same_name_legacy_revoke_is_isolated_and_invalidates_cache() {
    let (store, _directory) = store();
    let first = store.create_permanent_token("same-name").unwrap();
    let second = store.create_permanent_token("same-name").unwrap();
    assert!(store.validate(&first));
    assert!(store.validate(&second));
    let first_hash = token_hash(&first);
    assert_eq!(store.revoke_legacy(&first_hash), vec![first_hash.clone()]);
    assert!(!store.validate(&first));
    assert!(store.validate(&second));
    assert_eq!(store.list_devices(0, 100).len(), 1);
}

#[test]
fn same_name_paired_revoke_targets_only_one_device() {
    let (store, _directory) = store();
    let first = store.create_paired_token("device-a", "same-name").unwrap();
    let second = store.create_paired_token("device-b", "same-name").unwrap();
    assert!(store.validate(&first));
    assert!(store.validate(&second));
    let hashes = store.revoke_device("device-a");
    assert_eq!(hashes, vec![token_hash(&first)]);
    assert!(!store.validate(&first));
    assert!(store.validate(&second));
}
