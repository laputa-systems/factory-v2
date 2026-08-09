//! Daemon-only binding of a physical content seal to the kernel's global
//! `ContentObject` identity chain.
//!
//! `society-content` proves immutable byte identity. This module is the only
//! `societyd` code allowed to call that physical writer, and it does so before
//! issuing either kernel command. It deliberately has no media/schema,
//! provenance, retention, evaluator, or evidence fields: the resulting
//! `ContentObject` is global byte identity only.

#[cfg(test)]
use std::cell::Cell;

use society_content::{
    ContentDigest, ContentObjectStore, ContentSealDisposition, ContentSealLimit, ContentStoreError,
    ContentStoreRoot,
};
use society_kernel::{
    Capability, CommandBody, CommandDisposition, CommandId, CommandRequest, ContentIdentityState,
    ContentObjectId, KernelStore, PrincipalId, Sha256Digest, StoreError,
};
use thiserror::Error;

const MAX_OPERATION_LABEL_BYTES: usize = 80;
const RECORD_COMMAND_PREFIX: &str = "content-seal-v1/";
const RECORD_COMMAND_SUFFIX: &str = "/receipt";
const REGISTER_COMMAND_SUFFIX: &str = "/object";

/// An opaque, retry-stable identity for one physical-seal-to-kernel operation.
///
/// The expected digest and its two exact kernel-service command identities are
/// inseparable. A caller can therefore retry the same operation, but cannot
/// silently reuse its label with different command IDs. Reusing a label with
/// different bytes reaches the kernel as an idempotency conflict rather than
/// registering a second meaning for the same operation identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentSealOperationId {
    expected_digest: Sha256Digest,
    record_content_seal_receipt_command_id: CommandId,
    register_content_object_command_id: CommandId,
}

impl ContentSealOperationId {
    /// Constructs the identity from its closed label and expected byte digest.
    /// The generated command IDs are fixed v1 spellings, not caller-provided
    /// text, so their relation cannot drift across crash retries.
    pub(crate) fn parse(
        label: impl AsRef<str>,
        expected_digest: Sha256Digest,
    ) -> Result<Self, ContentSealOperationIdError> {
        let label = label.as_ref();
        if !valid_operation_label(label) {
            return Err(ContentSealOperationIdError::InvalidLabel);
        }
        let record_content_seal_receipt_command_id = CommandId::parse(format!(
            "{RECORD_COMMAND_PREFIX}{label}{RECORD_COMMAND_SUFFIX}"
        ))
        .map_err(|_| ContentSealOperationIdError::DerivedCommandId)?;
        let register_content_object_command_id = CommandId::parse(format!(
            "{RECORD_COMMAND_PREFIX}{label}{REGISTER_COMMAND_SUFFIX}"
        ))
        .map_err(|_| ContentSealOperationIdError::DerivedCommandId)?;
        Ok(Self {
            expected_digest,
            record_content_seal_receipt_command_id,
            register_content_object_command_id,
        })
    }

    pub(crate) const fn expected_digest(&self) -> Sha256Digest {
        self.expected_digest
    }

    pub(crate) fn record_content_seal_receipt_command_id(&self) -> &CommandId {
        &self.record_content_seal_receipt_command_id
    }

    pub(crate) fn register_content_object_command_id(&self) -> &CommandId {
        &self.register_content_object_command_id
    }
}

fn valid_operation_label(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > MAX_OPERATION_LABEL_BYTES
        || !bytes[0].is_ascii_alphanumeric()
        || !bytes.last().is_some_and(u8::is_ascii_alphanumeric)
    {
        return false;
    }
    bytes
        .iter()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub(crate) enum ContentSealOperationIdError {
    #[error("content seal operation label must be canonical lowercase ASCII")]
    InvalidLabel,
    #[error("content seal operation produced an invalid kernel command identity")]
    DerivedCommandId,
}

/// The only result of the daemon content writer. It carries global byte
/// identity and physical store disposition, never contextual semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ContentObjectRegistration {
    pub(crate) digest: Sha256Digest,
    pub(crate) content_object_id: ContentObjectId,
    pub(crate) physical_disposition: ContentSealDisposition,
}

/// The daemon-owned physical writer. It intentionally has no public/supervisor
/// wire representation; callers inside the resident authority must supply the
/// operation identity above.
pub(crate) struct ContentSealingAuthority {
    store: ContentObjectStore,
    limit: ContentSealLimit,
    #[cfg(test)]
    test_crash_seam: Cell<Option<ContentSealTestCrashSeam>>,
}

impl ContentSealingAuthority {
    pub(crate) fn open(
        root: ContentStoreRoot,
        limit: ContentSealLimit,
    ) -> Result<Self, ContentStoreError> {
        Ok(Self {
            store: ContentObjectStore::open(root)?,
            limit,
            #[cfg(test)]
            test_crash_seam: Cell::new(None),
        })
    }

    /// Physically seals first, then drives only the existing narrow kernel
    /// receipt-to-object transition. A physical failure returns before any
    /// ledger command can be created.
    pub(crate) fn seal_and_register(
        &self,
        kernel: &mut KernelStore,
        operation: &ContentSealOperationId,
        bytes: &[u8],
    ) -> Result<ContentObjectRegistration, ContentSealingError> {
        self.seal_and_register_inner(kernel, operation, bytes)
    }

    fn seal_and_register_inner(
        &self,
        kernel: &mut KernelStore,
        operation: &ContentSealOperationId,
        bytes: &[u8],
    ) -> Result<ContentObjectRegistration, ContentSealingError> {
        let supplied_digest = kernel_digest(ContentDigest::of_bytes(bytes));
        if supplied_digest != operation.expected_digest() {
            return Err(ContentSealingError::InputDigestMismatch);
        }

        let physical = self.store.seal_bytes(bytes, self.limit)?;
        let digest = kernel_digest(physical.digest);
        if digest != operation.expected_digest() {
            return Err(ContentSealingError::PhysicalDigestMismatch);
        }
        #[cfg(test)]
        if self.take_test_crash(ContentSealTestCrashSeam::AfterPhysicalSeal) {
            return Err(ContentSealingError::TestCrashAfterPhysicalSeal);
        }

        match kernel.content_identity_state(digest)? {
            ContentIdentityState::Registered {
                content_object_id, ..
            } => {
                return Ok(ContentObjectRegistration {
                    digest,
                    content_object_id,
                    physical_disposition: physical.disposition,
                });
            }
            ContentIdentityState::SealReceiptOnly {
                content_seal_receipt_id,
            } => {
                return self.register_after_receipt(
                    kernel,
                    operation,
                    digest,
                    physical.disposition,
                    content_seal_receipt_id,
                );
            }
            ContentIdentityState::Absent => {}
        }

        execute_kernel_service_command(
            kernel,
            operation.record_content_seal_receipt_command_id(),
            Capability::RecordContentSealReceipt,
            CommandBody::RecordContentSealReceipt { digest },
        )?;
        #[cfg(test)]
        if self.take_test_crash(ContentSealTestCrashSeam::AfterReceiptCommand) {
            return Err(ContentSealingError::TestCrashAfterReceiptCommand);
        }

        match kernel.content_identity_state(digest)? {
            ContentIdentityState::Registered {
                content_object_id, ..
            } => Ok(ContentObjectRegistration {
                digest,
                content_object_id,
                physical_disposition: physical.disposition,
            }),
            ContentIdentityState::SealReceiptOnly {
                content_seal_receipt_id,
            } => self.register_after_receipt(
                kernel,
                operation,
                digest,
                physical.disposition,
                content_seal_receipt_id,
            ),
            ContentIdentityState::Absent => Err(ContentSealingError::ReceiptNotMaterialized),
        }
    }

    fn register_after_receipt(
        &self,
        kernel: &mut KernelStore,
        operation: &ContentSealOperationId,
        digest: Sha256Digest,
        physical_disposition: ContentSealDisposition,
        content_seal_receipt_id: society_kernel::ContentSealReceiptId,
    ) -> Result<ContentObjectRegistration, ContentSealingError> {
        execute_kernel_service_command(
            kernel,
            operation.register_content_object_command_id(),
            Capability::RegisterContentObject,
            CommandBody::RegisterContentObject {
                content_seal_receipt_id,
            },
        )?;
        match kernel.content_identity_state(digest)? {
            ContentIdentityState::Registered {
                content_object_id, ..
            } => Ok(ContentObjectRegistration {
                digest,
                content_object_id,
                physical_disposition,
            }),
            ContentIdentityState::Absent | ContentIdentityState::SealReceiptOnly { .. } => {
                Err(ContentSealingError::ContentObjectNotMaterialized)
            }
        }
    }

    #[cfg(test)]
    fn seal_and_register_after_test_crash(
        &self,
        kernel: &mut KernelStore,
        operation: &ContentSealOperationId,
        bytes: &[u8],
        seam: ContentSealTestCrashSeam,
    ) -> Result<ContentObjectRegistration, ContentSealingError> {
        self.test_crash_seam.set(Some(seam));
        let result = self.seal_and_register_inner(kernel, operation, bytes);
        self.test_crash_seam.set(None);
        result
    }

    #[cfg(test)]
    fn take_test_crash(&self, expected: ContentSealTestCrashSeam) -> bool {
        if self.test_crash_seam.get() == Some(expected) {
            self.test_crash_seam.set(None);
            true
        } else {
            false
        }
    }
}

fn kernel_digest(digest: ContentDigest) -> Sha256Digest {
    Sha256Digest::from_bytes(*digest.as_bytes())
}

fn execute_kernel_service_command(
    kernel: &mut KernelStore,
    command_id: &CommandId,
    capability: Capability,
    body: CommandBody,
) -> Result<(), ContentSealingError> {
    let capability_grant_id = kernel
        .active_capability_grant(PrincipalId::KERNEL, capability)?
        .ok_or(ContentSealingError::KernelServiceCapabilityMissing { capability })?;
    let receipt = kernel.execute(CommandRequest {
        command_id: command_id.clone(),
        principal_id: PrincipalId::KERNEL,
        capability_grant_id,
        capability,
        expected_generation: society_kernel::ExpectedGeneration::NotApplicable,
        body,
    })?;
    match receipt.disposition {
        CommandDisposition::Accepted(_) => Ok(()),
        CommandDisposition::Rejected(rejection) => {
            Err(ContentSealingError::KernelCommandRejected {
                capability,
                rejection,
            })
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContentSealTestCrashSeam {
    AfterPhysicalSeal,
    AfterReceiptCommand,
}

#[derive(Debug, Error)]
pub(crate) enum ContentSealingError {
    #[error(transparent)]
    Physical(#[from] ContentStoreError),
    #[error(transparent)]
    Kernel(#[from] StoreError),
    #[error("supplied content bytes do not match the operation's expected digest")]
    InputDigestMismatch,
    #[error("physical seal digest does not match the operation's expected digest")]
    PhysicalDigestMismatch,
    #[error("the kernel service capability {capability:?} is absent")]
    KernelServiceCapabilityMissing { capability: Capability },
    #[error("kernel rejected daemon-only content command {capability:?}: {rejection:?}")]
    KernelCommandRejected {
        capability: Capability,
        rejection: society_kernel::Rejection,
    },
    #[error("accepted content seal receipt was not materialized")]
    ReceiptNotMaterialized,
    #[error("accepted content object registration was not materialized")]
    ContentObjectNotMaterialized,
    #[error("daemon restart recovery is fenced before content sealing can resume")]
    RecoveryFenced,
    #[cfg(test)]
    #[error("test crash seam after the physical content seal")]
    TestCrashAfterPhysicalSeal,
    #[cfg(test)]
    #[error("test crash seam after the receipt command")]
    TestCrashAfterReceiptCommand,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::{
        fs,
        io::Cursor,
        os::unix::fs::{PermissionsExt, symlink},
        path::{Path, PathBuf},
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use society_content::{ContentObjectStore, ContentSealLimit, ContentStoreRoot};
    use society_kernel::{
        CapabilityGrantId, CommandId, ContentObjectId, ContentSealReceiptId, KernelStore,
    };

    use super::*;

    fn temporary_parent(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let parent = PathBuf::from("/tmp").join(format!(
            "societyd-content-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        parent
    }

    fn limit() -> ContentSealLimit {
        ContentSealLimit::new(1024 * 1024).unwrap()
    }

    fn bind_after_parallel_exec_window(parent: &Path) -> crate::Daemon {
        // Other native-supervision tests fork in this same test process. A
        // child between fork and exec briefly carries every close-on-exec
        // descriptor, including this just-released flock. Bound the retry so
        // a real leaked lock still fails rather than weakening the ownership
        // assertion this recovery test relies on.
        for attempt in 0..100 {
            match crate::Daemon::bind(crate::DaemonConfig::new(parent)) {
                Ok(daemon) => return daemon,
                Err(crate::DaemonError::AlreadyRunning) if attempt < 99 => {
                    thread::sleep(Duration::from_millis(1));
                }
                Err(error) => panic!("resident rebind failed after bounded retry: {error}"),
            }
        }
        unreachable!("the bounded retry loop either returns or panics")
    }

    fn harness(
        label: &str,
        seal_limit: ContentSealLimit,
    ) -> (ContentSealingAuthority, KernelStore, PathBuf) {
        let parent = temporary_parent(label);
        let authority = ContentSealingAuthority::open(
            ContentStoreRoot::parse(parent.join("content")).unwrap(),
            seal_limit,
        )
        .unwrap();
        let kernel = KernelStore::open(parent.join("society.sqlite3")).unwrap();
        (authority, kernel, parent)
    }

    fn operation(label: &str, bytes: &[u8]) -> ContentSealOperationId {
        ContentSealOperationId::parse(label, Sha256Digest::of_bytes(bytes)).unwrap()
    }

    fn physical_path(parent: &std::path::Path, digest: Sha256Digest) -> PathBuf {
        let hex = format!("{digest:?}");
        parent
            .join("content")
            .join("sha256")
            .join(&hex[..2])
            .join(&hex[2..])
    }

    #[test]
    fn fresh_physical_seal_precedes_one_receipt_and_one_global_object() {
        let (authority, mut kernel, parent) = harness("fresh", limit());
        let bytes = b"immutable bytes are not an evidence claim";
        let operation = operation("fresh-content", bytes);

        let registered = authority
            .seal_and_register(&mut kernel, &operation, bytes)
            .unwrap();

        assert_eq!(registered.digest, Sha256Digest::of_bytes(bytes));
        assert_eq!(
            registered.content_object_id,
            ContentObjectId::new(1).unwrap()
        );
        assert_eq!(
            registered.physical_disposition,
            ContentSealDisposition::Created
        );
        assert_eq!(kernel.command_count().unwrap(), 2);
        assert_eq!(
            kernel.content_identity_state(registered.digest).unwrap(),
            ContentIdentityState::Registered {
                content_seal_receipt_id: ContentSealReceiptId::new(1).unwrap(),
                content_object_id: registered.content_object_id,
            }
        );
        drop(kernel);
        drop(authority);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn resident_daemon_owns_and_exercises_the_only_content_writer() {
        let parent = temporary_parent("resident-owner");
        let bytes = b"resident daemon content writer";
        let operation = operation("resident-owner", bytes);
        let mut daemon = crate::Daemon::bind(crate::DaemonConfig::new(&parent)).unwrap();

        let first = daemon.seal_content_object(&operation, bytes).unwrap();
        let retry = daemon.seal_content_object(&operation, bytes).unwrap();

        assert_eq!(first.content_object_id, retry.content_object_id);
        assert_eq!(
            retry.physical_disposition,
            ContentSealDisposition::AlreadyPresentVerified
        );
        drop(daemon);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn daemon_recovery_fence_refuses_content_completion_after_a_restart() {
        let parent = temporary_parent("recovery-fenced");
        let first_bytes = b"first daemon lifetime";
        let first_operation = operation("recovery-first", first_bytes);
        let mut first = crate::Daemon::bind(crate::DaemonConfig::new(&parent)).unwrap();
        first
            .seal_content_object(&first_operation, first_bytes)
            .unwrap();
        drop(first);

        let second_bytes = b"must not cross restart recovery fence";
        let second_operation = operation("recovery-second", second_bytes);
        let mut restarted = bind_after_parallel_exec_window(&parent);
        assert_eq!(restarted.startup_mode(), crate::StartupMode::RecoveryFenced);
        assert!(matches!(
            restarted.seal_content_object(&second_operation, second_bytes),
            Err(ContentSealingError::RecoveryFenced)
        ));
        drop(restarted);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn second_daemon_cannot_acquire_the_resident_content_store_writer() {
        let parent = temporary_parent("resident-exclusive");
        let daemon = crate::Daemon::bind(crate::DaemonConfig::new(&parent)).unwrap();

        assert!(matches!(
            crate::Daemon::bind(crate::DaemonConfig::new(&parent)),
            Err(crate::DaemonError::AlreadyRunning)
        ));
        assert!(matches!(
            ContentObjectStore::open(ContentStoreRoot::parse(parent.join("content")).unwrap()),
            Err(ContentStoreError::StoreAlreadyOwned)
        ));
        drop(daemon);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn resident_daemon_refuses_a_symlinked_content_root_before_socket_bind() {
        let parent = temporary_parent("unsafe-content-root");
        let target = parent.join("outside-content");
        fs::create_dir(&target).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
        symlink(&target, parent.join("content")).unwrap();

        assert!(matches!(
            crate::Daemon::bind(crate::DaemonConfig::new(&parent)),
            Err(crate::DaemonError::ContentStore(
                ContentStoreError::UnsafeStorageDirectory
            ))
        ));
        assert!(!parent.join("societyd.sock").exists());
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn exact_retry_reuses_both_the_physical_and_global_identity() {
        let (authority, mut kernel, parent) = harness("exact-retry", limit());
        let bytes = b"same exact retry";
        let operation = operation("exact-retry", bytes);
        let first = authority
            .seal_and_register(&mut kernel, &operation, bytes)
            .unwrap();
        let retry = authority
            .seal_and_register(&mut kernel, &operation, bytes)
            .unwrap();

        assert_eq!(retry.content_object_id, first.content_object_id);
        assert_eq!(
            retry.physical_disposition,
            ContentSealDisposition::AlreadyPresentVerified
        );
        assert_eq!(kernel.command_count().unwrap(), 2);
        drop(kernel);
        drop(authority);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn duplicate_bytes_from_a_distinct_operation_reuse_one_global_object() {
        let (authority, mut kernel, parent) = harness("duplicate", limit());
        let bytes = b"globally shared immutable bytes";
        let first_operation = operation("first-occurrence", bytes);
        let second_operation = operation("second-occurrence", bytes);
        let first = authority
            .seal_and_register(&mut kernel, &first_operation, bytes)
            .unwrap();
        let second = authority
            .seal_and_register(&mut kernel, &second_operation, bytes)
            .unwrap();

        assert_eq!(second.content_object_id, first.content_object_id);
        assert_eq!(kernel.command_count().unwrap(), 2);
        assert!(
            kernel
                .command_receipt(second_operation.record_content_seal_receipt_command_id())
                .unwrap()
                .is_none()
        );
        assert!(
            kernel
                .command_receipt(second_operation.register_content_object_command_id())
                .unwrap()
                .is_none()
        );
        drop(kernel);
        drop(authority);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn changed_bytes_reusing_one_operation_command_identity_conflict() {
        let (authority, mut kernel, parent) = harness("changed-bytes", limit());
        let original = b"the original operation bytes";
        let changed = b"a different byte identity";
        let original_operation = operation("one-operation", original);
        authority
            .seal_and_register(&mut kernel, &original_operation, original)
            .unwrap();
        let changed_operation = operation("one-operation", changed);

        assert!(matches!(
            authority.seal_and_register(&mut kernel, &changed_operation, changed),
            Err(ContentSealingError::Kernel(StoreError::IdempotencyConflict))
        ));
        assert_eq!(kernel.command_count().unwrap(), 2);
        drop(kernel);
        drop(authority);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn physical_limit_failure_never_creates_a_ledger_row() {
        let small_limit = ContentSealLimit::new(8).unwrap();
        let (authority, mut kernel, parent) = harness("limit", small_limit);
        let bytes = b"nine-bytes";
        let operation = operation("limit-failure", bytes);

        assert!(matches!(
            authority.seal_and_register(&mut kernel, &operation, bytes),
            Err(ContentSealingError::Physical(
                ContentStoreError::SealLimitExceeded
            ))
        ));
        assert_eq!(kernel.command_count().unwrap(), 0);
        assert_eq!(
            kernel
                .content_identity_state(Sha256Digest::of_bytes(bytes))
                .unwrap(),
            ContentIdentityState::Absent
        );
        drop(kernel);
        drop(authority);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn retry_after_physical_seal_crash_uses_the_same_kernel_operation_identity() {
        let (authority, mut kernel, parent) = harness("physical-crash", limit());
        let bytes = b"crash after physical seal";
        let operation = operation("physical-crash", bytes);

        assert!(matches!(
            authority.seal_and_register_after_test_crash(
                &mut kernel,
                &operation,
                bytes,
                ContentSealTestCrashSeam::AfterPhysicalSeal,
            ),
            Err(ContentSealingError::TestCrashAfterPhysicalSeal)
        ));
        assert_eq!(kernel.command_count().unwrap(), 0);
        assert!(
            authority
                .store
                .verify(ContentDigest::of_bytes(bytes))
                .is_ok()
        );

        let retry = authority
            .seal_and_register(&mut kernel, &operation, bytes)
            .unwrap();
        assert_eq!(retry.content_object_id, ContentObjectId::new(1).unwrap());
        assert_eq!(kernel.command_count().unwrap(), 2);
        drop(kernel);
        drop(authority);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn retry_after_receipt_command_crash_registers_the_existing_receipt() {
        let (authority, mut kernel, parent) = harness("receipt-crash", limit());
        let bytes = b"crash after receipt command";
        let operation = operation("receipt-crash", bytes);

        assert!(matches!(
            authority.seal_and_register_after_test_crash(
                &mut kernel,
                &operation,
                bytes,
                ContentSealTestCrashSeam::AfterReceiptCommand,
            ),
            Err(ContentSealingError::TestCrashAfterReceiptCommand)
        ));
        assert_eq!(kernel.command_count().unwrap(), 1);
        assert!(matches!(
            kernel
                .content_identity_state(Sha256Digest::of_bytes(bytes))
                .unwrap(),
            ContentIdentityState::SealReceiptOnly { .. }
        ));

        let retry = authority
            .seal_and_register(&mut kernel, &operation, bytes)
            .unwrap();
        assert_eq!(retry.content_object_id, ContentObjectId::new(1).unwrap());
        assert_eq!(kernel.command_count().unwrap(), 2);
        drop(kernel);
        drop(authority);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn tampered_digest_path_cannot_create_a_second_kernel_command() {
        let (authority, mut kernel, parent) = harness("tamper", limit());
        let bytes = b"tampered content identity";
        let first = operation("tamper-first", bytes);
        authority
            .seal_and_register(&mut kernel, &first, bytes)
            .unwrap();
        let path = physical_path(&parent, Sha256Digest::of_bytes(bytes));
        fs::write(&path, b"mutated bytes").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let second = operation("tamper-second", bytes);

        assert!(matches!(
            authority.seal_and_register(&mut kernel, &second, bytes),
            Err(ContentSealingError::Physical(
                ContentStoreError::StoredDigestMismatch
            ))
        ));
        assert_eq!(kernel.command_count().unwrap(), 2);
        assert!(
            kernel
                .command_receipt(second.record_content_seal_receipt_command_id())
                .unwrap()
                .is_none()
        );
        drop(kernel);
        drop(authority);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn symlinked_digest_path_cannot_create_a_second_kernel_command() {
        let (authority, mut kernel, parent) = harness("symlink", limit());
        let bytes = b"symlinked content identity";
        let first = operation("symlink-first", bytes);
        authority
            .seal_and_register(&mut kernel, &first, bytes)
            .unwrap();
        let path = physical_path(&parent, Sha256Digest::of_bytes(bytes));
        fs::remove_file(&path).unwrap();
        let target = parent.join("redirected-content");
        fs::write(&target, bytes).unwrap();
        symlink(&target, &path).unwrap();
        let second = operation("symlink-second", bytes);

        assert!(matches!(
            authority.seal_and_register(&mut kernel, &second, bytes),
            Err(ContentSealingError::Physical(
                ContentStoreError::UnsafeStoredObject
            ))
        ));
        assert_eq!(kernel.command_count().unwrap(), 2);
        assert!(
            kernel
                .command_receipt(second.record_content_seal_receipt_command_id())
                .unwrap()
                .is_none()
        );
        drop(kernel);
        drop(authority);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn public_and_supervisor_protocols_cannot_encode_content_writer_authority() {
        let command_id = CommandId::parse("valid-supervisor-bootstrap").unwrap();
        let supervisor_request = crate::protocol::SupervisorRequest::Execute {
            correlation: crate::protocol::CorrelationId::new(1).unwrap(),
            command: crate::protocol::ClientCommandRequest {
                command_id: command_id.clone(),
                principal_id: PrincipalId::KERNEL,
                capability_grant_id: CapabilityGrantId::new(1).unwrap(),
                capability: Capability::BootstrapSociety,
                expected_generation: society_kernel::ExpectedGeneration::NotApplicable,
                body: crate::protocol::ClientCommandBody::BootstrapSociety,
            },
        };
        let mut encoded_supervisor = Vec::new();
        crate::protocol::write_supervisor_request(&mut encoded_supervisor, &supervisor_request)
            .unwrap();

        // The command request uses a fixed layout. Mutating only this byte
        // preserves a valid existing body while asking for capability 62; the
        // closed supervisor decoder must reject that unsupported authority.
        let capability_offset = 4 + 2 + 1 + 8 + 4 + command_id.as_str().len() + 8 + 8;
        encoded_supervisor[capability_offset] = Capability::RecordContentSealReceipt as u8;
        assert!(matches!(
            crate::protocol::read_supervisor_request(&mut Cursor::new(&encoded_supervisor)),
            Err(crate::protocol::WireError::InvalidValue)
        ));

        // Client command-body tags are also closed. Kernel content command
        // kind 62 has no representable body variant on this protocol.
        encoded_supervisor[capability_offset] = Capability::BootstrapSociety as u8;
        let body_tag_offset = capability_offset + 1 + 1;
        encoded_supervisor[body_tag_offset] = Capability::RecordContentSealReceipt as u8;
        assert!(matches!(
            crate::protocol::read_supervisor_request(&mut Cursor::new(&encoded_supervisor)),
            Err(crate::protocol::WireError::UnknownTag)
        ));

        let mut forged_public = Vec::new();
        let payload = [
            (crate::protocol::PROTOCOL_VERSION >> 8) as u8,
            crate::protocol::PROTOCOL_VERSION as u8,
            0x41,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            1,
        ];
        forged_public.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        forged_public.extend_from_slice(&payload);
        assert!(matches!(
            crate::protocol::read_public_request(&mut Cursor::new(forged_public)),
            Err(crate::protocol::WireError::UnknownTag)
        ));
    }
}
