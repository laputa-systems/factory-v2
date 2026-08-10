//! Immutable, digest-addressed byte sealing for society forensic content.
//!
//! This crate has one semantic operation: seal exact bytes under BLAKE3. A
//! successful receipt establishes physical byte identity only. It does not
//! admit evidence, create graph knowledge, assign a forensic role, or emit
//! provenance. Those require separate typed kernel commands.

use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
    },
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use blake3::Hasher;
use thiserror::Error;

const DIRECTORY_MODE: u32 = 0o700;
const OBJECT_MODE: u32 = 0o600;
const DIGEST_HEX_LENGTH: usize = 64;
static NEXT_TEMPORARY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ContentDigest([u8; 32]);

impl ContentDigest {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    pub fn parse(value: &str) -> Result<Self, ContentDigestError> {
        if value.len() != DIGEST_HEX_LENGTH
            || !value
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        {
            return Err(ContentDigestError::NotCanonicalBlake3);
        }
        let mut bytes = [0_u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let offset = index * 2;
            *byte = (hex_nibble(value.as_bytes()[offset])? << 4)
                | hex_nibble(value.as_bytes()[offset + 1])?;
        }
        Ok(Self(bytes))
    }

    pub fn to_hex(self) -> String {
        let mut rendered = String::with_capacity(DIGEST_HEX_LENGTH);
        for byte in self.0 {
            use fmt::Write as _;
            write!(&mut rendered, "{byte:02x}").expect("writing to String cannot fail");
        }
        rendered
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ContentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ContentDigest")
            .field(&self.to_hex())
            .finish()
    }
}

impl fmt::Display for ContentDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

fn hex_nibble(byte: u8) -> Result<u8, ContentDigestError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(ContentDigestError::NotCanonicalBlake3),
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum ContentDigestError {
    #[error("content digest must be 64 lowercase BLAKE3 hexadecimal characters")]
    NotCanonicalBlake3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentStoreRoot(PathBuf);

impl ContentStoreRoot {
    pub fn parse(path: impl Into<PathBuf>) -> Result<Self, ContentStoreRootError> {
        let path = path.into();
        let bytes = path.as_os_str().as_bytes();
        if !path.is_absolute()
            || path.parent().is_none()
            || bytes.contains(&0)
            || (bytes.len() > 1 && bytes.ends_with(b"/"))
            || bytes
                .split(|byte| *byte == b'/')
                .skip(1)
                .any(|component| component.is_empty() || component == b"." || component == b"..")
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::CurDir | Component::ParentDir | Component::Prefix(_)
                )
            })
        {
            return Err(ContentStoreRootError::NotCanonicalAbsolutePath);
        }
        Ok(Self(path))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum ContentStoreRootError {
    #[error("content-store root must be a canonical non-root absolute Unix path")]
    NotCanonicalAbsolutePath,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentSealLimit(u64);

impl ContentSealLimit {
    pub const fn new(bytes: u64) -> Option<Self> {
        if bytes == 0 { None } else { Some(Self(bytes)) }
    }

    pub const fn bytes(self) -> u64 {
        self.0
    }
}

/// Caller-selected transfer bound for releasing one physically verified
/// object from the content store. It is distinct from the ingest limit:
/// reading an existing object must not silently inherit the bound of an
/// unrelated seal operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentReadLimit(u64);

impl ContentReadLimit {
    pub const fn new(bytes: u64) -> Option<Self> {
        if bytes == 0 { None } else { Some(Self(bytes)) }
    }

    pub const fn bytes(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentSealDisposition {
    Created,
    AlreadyPresentVerified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentSealReceipt {
    pub digest: ContentDigest,
    pub disposition: ContentSealDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentVerificationReceipt {
    pub digest: ContentDigest,
}

/// Receipt for one bounded stream copy whose complete bytes matched the
/// requested physical BLAKE3 identity. It is not provenance, executable
/// qualification, or evidence admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentReadReceipt {
    digest: ContentDigest,
    byte_count: u64,
}

impl ContentReadReceipt {
    pub const fn digest(&self) -> ContentDigest {
        self.digest
    }

    pub const fn byte_count(&self) -> u64 {
        self.byte_count
    }
}

#[derive(Clone, Debug)]
pub struct ContentObjectStore {
    root: ContentStoreRoot,
    digest_root: PathBuf,
    incoming_root: PathBuf,
    _ownership_lock: Arc<File>,
}

impl ContentObjectStore {
    /// Opens the store under an exclusively held writer lock.
    ///
    /// The parent is expected to be a private runtime root in the custody of
    /// `societyd`. Mode/owner/symlink checks detect accidents and stale state;
    /// they do not claim containment against a hostile same-UID process racing
    /// ancestor-directory replacement.
    pub fn open(root: ContentStoreRoot) -> Result<Self, ContentStoreError> {
        let parent = root
            .as_path()
            .parent()
            .ok_or(ContentStoreError::UnsafeStoreRoot)?;
        validate_directory(parent)?;
        create_or_validate_directory(root.as_path())?;
        let ownership_lock = Arc::new(acquire_ownership_lock(root.as_path())?);
        let digest_root = root.as_path().join("blake3");
        let incoming_root = root.as_path().join(".incoming");
        create_or_validate_directory(&digest_root)?;
        create_or_validate_directory(&incoming_root)?;
        recover_incoming(&incoming_root)?;
        sync_directory(root.as_path())?;
        Ok(Self {
            root,
            digest_root,
            incoming_root,
            _ownership_lock: ownership_lock,
        })
    }

    pub fn root(&self) -> &ContentStoreRoot {
        &self.root
    }

    pub fn seal_bytes(
        &self,
        bytes: &[u8],
        limit: ContentSealLimit,
    ) -> Result<ContentSealReceipt, ContentStoreError> {
        self.seal_reader(&mut &*bytes, limit)
    }

    pub fn seal_reader(
        &self,
        reader: &mut impl Read,
        limit: ContentSealLimit,
    ) -> Result<ContentSealReceipt, ContentStoreError> {
        let temporary_id = NEXT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
        if temporary_id == u64::MAX {
            return Err(ContentStoreError::TemporaryIdentityExhausted);
        }
        let temporary_path = self
            .incoming_root
            .join(format!("ingest-{}-{temporary_id}", std::process::id()));
        let mut temporary = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(OBJECT_MODE)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&temporary_path)?;
        let cleanup = TemporaryArtifact::new(temporary_path.clone());
        validate_object_metadata(&temporary.metadata()?)?;

        let mut hasher = Hasher::new();
        let mut observed = 0_u64;
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            observed = observed
                .checked_add(read as u64)
                .ok_or(ContentStoreError::SealLimitExceeded)?;
            if observed > limit.bytes() {
                return Err(ContentStoreError::SealLimitExceeded);
            }
            hasher.update(&buffer[..read]);
            temporary.write_all(&buffer[..read])?;
        }
        temporary.sync_all()?;
        let digest = ContentDigest(*hasher.finalize().as_bytes());
        let final_path = self.object_path(digest)?;

        let disposition = match fs::hard_link(&temporary_path, &final_path) {
            Ok(()) => {
                sync_directory(
                    final_path
                        .parent()
                        .ok_or(ContentStoreError::UnsafeStorageDirectory)?,
                )?;
                ContentSealDisposition::Created
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                self.verify(digest)?;
                ContentSealDisposition::AlreadyPresentVerified
            }
            Err(error) => return Err(ContentStoreError::Io(error)),
        };
        drop(temporary);
        cleanup.remove()?;
        sync_directory(&self.incoming_root)?;
        Ok(ContentSealReceipt {
            digest,
            disposition,
        })
    }

    pub fn verify(
        &self,
        digest: ContentDigest,
    ) -> Result<ContentVerificationReceipt, ContentStoreError> {
        let path = self.object_path(digest)?;
        let mut object = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|error| match error.raw_os_error() {
                Some(libc::ELOOP) => ContentStoreError::UnsafeStoredObject,
                _ => ContentStoreError::Io(error),
            })?;
        validate_object_metadata(&object.metadata()?)?;
        let mut hasher = Hasher::new();
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let read = object.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        if ContentDigest(*hasher.finalize().as_bytes()) != digest {
            return Err(ContentStoreError::StoredDigestMismatch);
        }
        Ok(ContentVerificationReceipt { digest })
    }

    /// Copies one immutable object under an explicit bound while recomputing
    /// the requested digest on the same no-follow file handle. The caller must
    /// discard its destination if this method returns an error because a
    /// digest mismatch is knowable only after the complete stream. A success
    /// receipt proves the copied byte count and physical identity only; later
    /// code must establish semantic role and executable/profile authority.
    pub fn copy_verified_to(
        &self,
        digest: ContentDigest,
        limit: ContentReadLimit,
        destination: &mut impl Write,
    ) -> Result<ContentReadReceipt, ContentStoreError> {
        let path = self.object_path(digest)?;
        let mut object = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|error| match error.raw_os_error() {
                Some(libc::ELOOP) => ContentStoreError::UnsafeStoredObject,
                _ => ContentStoreError::Io(error),
            })?;
        validate_object_metadata(&object.metadata()?)?;
        let mut hasher = Hasher::new();
        let mut observed = 0_u64;
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let read = object.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            observed = observed
                .checked_add(read as u64)
                .ok_or(ContentStoreError::ReadLimitExceeded)?;
            if observed > limit.bytes() {
                return Err(ContentStoreError::ReadLimitExceeded);
            }
            hasher.update(&buffer[..read]);
            destination.write_all(&buffer[..read])?;
        }
        if ContentDigest(*hasher.finalize().as_bytes()) != digest {
            return Err(ContentStoreError::StoredDigestMismatch);
        }
        Ok(ContentReadReceipt {
            digest,
            byte_count: observed,
        })
    }

    fn object_path(&self, digest: ContentDigest) -> Result<PathBuf, ContentStoreError> {
        let hex = digest.to_hex();
        let shard = self.digest_root.join(&hex[..2]);
        create_or_validate_directory(&shard)?;
        Ok(shard.join(&hex[2..]))
    }
}

#[derive(Debug, Error)]
pub enum ContentStoreError {
    #[error("content-store I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("content-store root or parent is not a private owned directory")]
    UnsafeStoreRoot,
    #[error("another content-store writer already owns this root")]
    StoreAlreadyOwned,
    #[error("content-store internal directory is not a private owned directory")]
    UnsafeStorageDirectory,
    #[error("stored content is not a private owned regular file")]
    UnsafeStoredObject,
    #[error("stored bytes do not match their digest identity")]
    StoredDigestMismatch,
    #[error("input exceeded the admitted content seal limit")]
    SealLimitExceeded,
    #[error("stored content exceeded the caller's verified-read limit")]
    ReadLimitExceeded,
    #[error("temporary content identity space is exhausted")]
    TemporaryIdentityExhausted,
}

fn create_or_validate_directory(path: &Path) -> Result<(), ContentStoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => validate_directory_metadata(&metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            use std::os::unix::fs::DirBuilderExt as _;
            let mut builder = fs::DirBuilder::new();
            builder.mode(DIRECTORY_MODE);
            let created = match builder.create(path) {
                Ok(()) => true,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => false,
                Err(error) => return Err(ContentStoreError::Io(error)),
            };
            validate_directory_metadata(&fs::symlink_metadata(path)?)?;
            if created {
                sync_directory(
                    path.parent()
                        .ok_or(ContentStoreError::UnsafeStorageDirectory)?,
                )?;
            }
            Ok(())
        }
        Err(error) => Err(ContentStoreError::Io(error)),
    }
}

fn validate_directory(path: &Path) -> Result<(), ContentStoreError> {
    let metadata = fs::symlink_metadata(path).map_err(ContentStoreError::Io)?;
    validate_directory_metadata(&metadata).map_err(|error| match error {
        ContentStoreError::UnsafeStorageDirectory => ContentStoreError::UnsafeStoreRoot,
        other => other,
    })
}

fn validate_directory_metadata(metadata: &fs::Metadata) -> Result<(), ContentStoreError> {
    if !metadata.file_type().is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != effective_uid()
        || metadata.permissions().mode() & 0o777 != DIRECTORY_MODE
    {
        return Err(ContentStoreError::UnsafeStorageDirectory);
    }
    Ok(())
}

fn validate_object_metadata(metadata: &fs::Metadata) -> Result<(), ContentStoreError> {
    if !metadata.file_type().is_file()
        || metadata.file_type().is_symlink()
        || metadata.uid() != effective_uid()
        || metadata.permissions().mode() & 0o777 != OBJECT_MODE
    {
        return Err(ContentStoreError::UnsafeStoredObject);
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), ContentStoreError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn acquire_ownership_lock(root: &Path) -> Result<File, ContentStoreError> {
    let path = root.join(".store.lock");
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .mode(OBJECT_MODE)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|error| match error.raw_os_error() {
            Some(libc::ELOOP) => ContentStoreError::UnsafeStoredObject,
            _ => ContentStoreError::Io(error),
        })?;
    validate_object_metadata(&lock.metadata()?)?;
    lock.sync_all()?;
    // SAFETY: `lock` owns this live descriptor for the entire store lifetime;
    // `flock` changes only its advisory lock state and receives no pointers.
    let outcome = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if outcome != 0 {
        let error = io::Error::last_os_error();
        return if error.kind() == io::ErrorKind::WouldBlock {
            Err(ContentStoreError::StoreAlreadyOwned)
        } else {
            Err(ContentStoreError::Io(error))
        };
    }
    sync_directory(root)?;
    Ok(lock)
}

fn recover_incoming(incoming_root: &Path) -> Result<(), ContentStoreError> {
    for entry in fs::read_dir(incoming_root)? {
        let entry = entry?;
        let name = entry.file_name();
        if !is_temporary_artifact_name(name.as_bytes()) {
            return Err(ContentStoreError::UnsafeStoredObject);
        }
        validate_object_metadata(&fs::symlink_metadata(entry.path())?)?;
        fs::remove_file(entry.path())?;
    }
    sync_directory(incoming_root)
}

fn is_temporary_artifact_name(bytes: &[u8]) -> bool {
    let Ok(name) = std::str::from_utf8(bytes) else {
        return false;
    };
    let mut fields = name.split('-');
    let (Some(prefix), Some(process), Some(identity), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return false;
    };
    let (Ok(process_id), Ok(temporary_id)) = (process.parse::<u32>(), identity.parse::<u64>())
    else {
        return false;
    };
    prefix == "ingest"
        && process_id > 0
        && temporary_id > 0
        && temporary_id < u64::MAX
        && process == process_id.to_string()
        && identity == temporary_id.to_string()
}

fn effective_uid() -> libc::uid_t {
    // SAFETY: `geteuid` has no pointer arguments or memory preconditions.
    unsafe { libc::geteuid() }
}

struct TemporaryArtifact {
    path: Option<PathBuf>,
}

impl TemporaryArtifact {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn remove(mut self) -> Result<(), ContentStoreError> {
        if let Some(path) = self.path.as_ref() {
            fs::remove_file(path)?;
            self.path = None;
        }
        Ok(())
    }
}

impl Drop for TemporaryArtifact {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = fs::remove_file(path);
        }
    }
}
