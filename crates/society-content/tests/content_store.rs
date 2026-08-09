//! Observable physical seal invariants; none of these tests confer semantic
//! evidence or provenance on the stored bytes.

#![allow(clippy::unwrap_used)]

use std::{
    env, fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Barrier},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use society_content::{
    ContentDigest, ContentObjectStore, ContentSealDisposition, ContentSealLimit, ContentStoreError,
    ContentStoreRoot,
};

fn temporary_parent(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = PathBuf::from("/tmp").join(format!(
        "society-content-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

fn open_store(parent: &Path) -> ContentObjectStore {
    ContentObjectStore::open(ContentStoreRoot::parse(parent.join("objects")).unwrap()).unwrap()
}

fn limit() -> ContentSealLimit {
    ContentSealLimit::new(1024 * 1024).unwrap()
}

fn object_path(parent: &Path, digest: ContentDigest) -> PathBuf {
    let hex = digest.to_hex();
    parent
        .join("objects")
        .join("blake3")
        .join(&hex[..2])
        .join(&hex[2..])
}

#[test]
fn seals_exact_bytes_once_and_reopen_verifies_the_same_identity() {
    let parent = temporary_parent("seal-reopen");
    let bytes = b"forensic bytes are not knowledge";
    let store = open_store(&parent);
    let created = store.seal_bytes(bytes, limit()).unwrap();
    assert_eq!(created.disposition, ContentSealDisposition::Created);
    assert_eq!(created.digest, ContentDigest::of_bytes(bytes));
    assert_eq!(
        fs::read(object_path(&parent, created.digest)).unwrap(),
        bytes
    );

    drop(store);
    let reopened = open_store(&parent);
    let repeated = reopened.seal_bytes(bytes, limit()).unwrap();
    assert_eq!(
        repeated.disposition,
        ContentSealDisposition::AlreadyPresentVerified
    );
    assert_eq!(
        reopened.verify(created.digest).unwrap().digest,
        created.digest
    );
    assert!(
        fs::read_dir(parent.join("objects/.incoming"))
            .unwrap()
            .next()
            .is_none()
    );
    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn root_has_one_writer_and_reopen_removes_only_valid_stale_ingests() {
    let parent = temporary_parent("ownership-recovery");
    let store = open_store(&parent);
    assert!(matches!(
        ContentObjectStore::open(store.root().clone()),
        Err(ContentStoreError::StoreAlreadyOwned)
    ));
    drop(store);

    let stale = parent.join("objects/.incoming/ingest-999-1");
    fs::write(&stale, b"uncommitted partial bytes").unwrap();
    fs::set_permissions(&stale, fs::Permissions::from_mode(0o600)).unwrap();
    let reopened = open_store(&parent);
    assert!(!stale.exists());
    drop(reopened);

    let foreign = parent.join("objects/.incoming/ingest-not-a-store-identity");
    fs::write(&foreign, b"must not be deleted by recovery").unwrap();
    fs::set_permissions(&foreign, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(matches!(
        ContentObjectStore::open(ContentStoreRoot::parse(parent.join("objects")).unwrap()),
        Err(ContentStoreError::UnsafeStoredObject)
    ));
    assert!(foreign.exists());
    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn cross_process_store_ownership_is_exclusive() {
    let parent = temporary_parent("cross-process-ownership");
    let store = open_store(&parent);
    let status = Command::new(env::current_exe().unwrap())
        .arg("--exact")
        .arg("content_store_lock_probe")
        .arg("--nocapture")
        .env("SOCIETY_CONTENT_LOCK_PROBE_ROOT", store.root().as_path())
        .status()
        .unwrap();
    assert!(status.success());
    drop(store);
    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn content_store_lock_probe() {
    let Some(root) = env::var_os("SOCIETY_CONTENT_LOCK_PROBE_ROOT") else {
        return;
    };
    assert!(matches!(
        ContentObjectStore::open(ContentStoreRoot::parse(root).unwrap()),
        Err(ContentStoreError::StoreAlreadyOwned)
    ));
}

#[test]
fn tampered_or_redirected_digest_location_never_returns_a_seal_receipt() {
    let parent = temporary_parent("tamper");
    let store = open_store(&parent);
    let bytes = b"sealed original";
    let receipt = store.seal_bytes(bytes, limit()).unwrap();
    let path = object_path(&parent, receipt.digest);
    fs::write(&path, b"tampered bytes").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(matches!(
        store.seal_bytes(bytes, limit()),
        Err(ContentStoreError::StoredDigestMismatch)
    ));

    fs::remove_file(&path).unwrap();
    let target = parent.join("redirect-target");
    fs::write(&target, bytes).unwrap();
    symlink(&target, &path).unwrap();
    assert!(matches!(
        store.verify(receipt.digest),
        Err(ContentStoreError::UnsafeStoredObject)
    ));
    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn admitted_limit_failure_removes_partial_ingest_and_creates_no_object() {
    let parent = temporary_parent("bounded");
    let store = open_store(&parent);
    let bytes = vec![b'x'; 65];
    let admitted = ContentSealLimit::new(64).unwrap();
    assert!(matches!(
        store.seal_bytes(&bytes, admitted),
        Err(ContentStoreError::SealLimitExceeded)
    ));
    assert!(
        fs::read_dir(parent.join("objects/.incoming"))
            .unwrap()
            .next()
            .is_none()
    );
    let hex = ContentDigest::of_bytes(&bytes).to_hex();
    assert!(
        !parent
            .join("objects/blake3")
            .join(&hex[..2])
            .join(&hex[2..])
            .exists()
    );
    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn concurrent_identical_seals_have_one_creator_and_verified_followers() {
    let parent = temporary_parent("concurrent");
    let store = Arc::new(open_store(&parent));
    let barrier = Arc::new(Barrier::new(8));
    let mut joins = Vec::new();
    for _ in 0..8 {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        joins.push(thread::spawn(move || {
            barrier.wait();
            store
                .seal_bytes(b"one immutable concurrent object", limit())
                .unwrap()
                .disposition
        }));
    }
    let dispositions: Vec<_> = joins.into_iter().map(|join| join.join().unwrap()).collect();
    assert_eq!(
        dispositions
            .iter()
            .filter(|value| **value == ContentSealDisposition::Created)
            .count(),
        1
    );
    assert_eq!(
        dispositions
            .iter()
            .filter(|value| **value == ContentSealDisposition::AlreadyPresentVerified)
            .count(),
        7
    );
    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn root_and_internal_directory_indirection_are_rejected() {
    let parent = temporary_parent("unsafe-root");
    let unsafe_parent = parent.join("unsafe-parent");
    fs::create_dir(&unsafe_parent).unwrap();
    fs::set_permissions(&unsafe_parent, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        ContentObjectStore::open(ContentStoreRoot::parse(unsafe_parent.join("objects")).unwrap()),
        Err(ContentStoreError::UnsafeStoreRoot)
    ));

    let store = open_store(&parent);
    let root = store.root().clone();
    drop(store);
    let digest_root = parent.join("objects/blake3");
    fs::remove_dir(&digest_root).unwrap();
    symlink(parent.join("objects/.incoming"), &digest_root).unwrap();
    assert!(matches!(
        ContentObjectStore::open(root),
        Err(ContentStoreError::UnsafeStorageDirectory)
    ));
    fs::remove_dir_all(parent).unwrap();
}

#[test]
fn digest_and_root_text_require_one_canonical_spelling() {
    assert_eq!(
        ContentDigest::of_bytes(b"").to_hex(),
        "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
    );
    assert_eq!(
        ContentDigest::of_bytes(b"abc").to_hex(),
        "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
    );
    let digest = ContentDigest::of_bytes(b"canonical");
    assert_eq!(ContentDigest::parse(&digest.to_hex()).unwrap(), digest);
    assert!(ContentDigest::parse(&digest.to_hex().to_uppercase()).is_err());
    assert!(ContentStoreRoot::parse("relative/objects").is_err());
    assert!(ContentStoreRoot::parse("/tmp/objects/").is_err());
    assert!(ContentStoreRoot::parse("/tmp/../objects").is_err());
}
