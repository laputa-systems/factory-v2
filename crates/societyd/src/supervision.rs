//! Native-host process ownership for the Pi SDK boundary.
//!
//! This module deliberately stops at process physics. It prepares an owned
//! workspace, verifies the qualified host artifacts, owns one process group,
//! captures raw pipe directions in bounded transient buffers, and returns
//! typed receipts. The returned facts
//! are **inputs** to the later kernel transaction; this module makes no PostgreSQL
//! write, durable charge, admission decision, or successor admission.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::CString,
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    os::fd::AsRawFd,
    os::unix::fs::MetadataExt,
    path::Path,
    process::{ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use blake3::Hasher;
use society_pi::{
    AbortPayload, AbortReason, AbsolutePath, Blake3Digest, BoundaryPeer, BoundarySequence,
    CorrelationIdentity, CreateSessionPayload, DisposePayload, DisposeReason, HostProcessId,
    InboundCommand, InboundFrame, MAX_JSONL_FRAME_BYTES, OutboundFrame, PeerError, PeerObservation,
    PeerPhase, PromptPayload, Provider, RuntimeIdentity, SessionIdentity, SpawnNonce,
    ToolCallIdentity, decode_outbound_jsonl, encode_inbound_jsonl,
    model_thinking_level_is_admitted,
};
use thiserror::Error;

use crate::native_child::{NativeSignalGroupOutcome, spawn_owned_native_child};

const MAX_HANDSHAKE_FRAMES: usize = 8;
const WORKSPACE_MODE: libc::mode_t = 0o700;
const MAX_TRANSIENT_STREAM_BYTES: usize = 8 * MAX_JSONL_FRAME_BYTES;

/// The resident's stable identity for one supervised child before the kernel
/// assigns its numeric `NativeChildId`. It is never inferred from a PID: PIDs
/// can be reused while the logical child must remain unique through its
/// receipt.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SupervisedChildId(String);

impl SupervisedChildId {
    pub fn parse(value: impl Into<String>) -> Result<Self, SupervisionError> {
        let value = value.into();
        if !is_domain_identifier(&value) {
            return Err(SupervisionError::InvalidChildIdentity);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A workspace identity is distinct from a filesystem path. The former is an
/// audit subject; the latter is an OS boundary validated at preparation time.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NativeWorkspaceId(String);

impl NativeWorkspaceId {
    pub fn parse(value: impl Into<String>) -> Result<Self, SupervisionError> {
        let value = value.into();
        if !is_domain_identifier(&value) {
            return Err(SupervisionError::InvalidWorkspaceIdentity);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A pre-existing daemon-owned root under which this subsystem may allocate
/// fresh direct children. Opening it is observational: it never creates or
/// chmods a caller-selected path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeWorkspaceRoot {
    directory: AbsolutePath,
}

impl NativeWorkspaceRoot {
    /// Validates an already-provisioned private root. Symlink spellings are
    /// rejected rather than canonicalized into an ambiguous authority path.
    pub fn open_owned(directory: impl AsRef<Path>) -> Result<Self, SupervisionError> {
        let directory = directory.as_ref();
        if fs::symlink_metadata(directory)?.file_type().is_symlink() {
            return Err(SupervisionError::UnsafeWorkspaceRoot);
        }
        let canonical = fs::canonicalize(directory)?;
        let metadata = fs::metadata(&canonical)?;
        // SAFETY: `geteuid` reads this process's credential without touching
        // memory, locks, descriptors, or process state.
        let daemon_uid = unsafe { libc::geteuid() };
        if !metadata.is_dir() || metadata.uid() != daemon_uid || metadata.mode() & 0o077 != 0 {
            return Err(SupervisionError::UnsafeWorkspaceRoot);
        }
        let directory = absolute_path_from_path(&canonical)?;
        Ok(Self { directory })
    }

    pub fn directory(&self) -> &AbsolutePath {
        &self.directory
    }

    /// Allocates exactly one brand-new direct child. `mkdir(0700)` is atomic
    /// with respect to pre-existing/symlink targets and avoids chmod on any
    /// caller-selected existing directory.
    pub fn allocate(
        &self,
        identity: NativeWorkspaceId,
    ) -> Result<NativeWorkspace, SupervisionError> {
        let candidate = self.directory.as_path().join(identity.as_str());
        let candidate_text = candidate
            .to_str()
            .ok_or(SupervisionError::InvalidSpawnRequest)?;
        let candidate_c =
            CString::new(candidate_text).map_err(|_| SupervisionError::InvalidSpawnRequest)?;
        // SAFETY: the C string is NUL-terminated and lives across the call;
        // `mkdir` reads it and creates one direct child with mode 0700.
        if unsafe { libc::mkdir(candidate_c.as_ptr(), WORKSPACE_MODE) } != 0 {
            return match io::Error::last_os_error().kind() {
                io::ErrorKind::AlreadyExists => Err(SupervisionError::WorkspaceAlreadyExists),
                _ => Err(SupervisionError::Io(io::Error::last_os_error())),
            };
        }
        let canonical = fs::canonicalize(&candidate)?;
        let metadata = fs::metadata(&canonical)?;
        // SAFETY: `geteuid` observes process credentials only.
        let daemon_uid = unsafe { libc::geteuid() };
        let canonical = absolute_path_from_path(&canonical)?;
        if !metadata.is_dir()
            || metadata.uid() != daemon_uid
            || metadata.mode() & 0o077 != 0
            || !canonical.is_strict_descendant_of(&self.directory)
        {
            return Err(SupervisionError::UnsafeWorkspace);
        }
        Ok(NativeWorkspace {
            identity,
            directory: canonical,
        })
    }
}

/// A fresh direct child of [`NativeWorkspaceRoot`], not a native sandbox.
/// Its contents remain under the ordinary host account.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeWorkspace {
    identity: NativeWorkspaceId,
    directory: AbsolutePath,
}

impl NativeWorkspace {
    pub fn identity(&self) -> &NativeWorkspaceId {
        &self.identity
    }

    pub fn directory(&self) -> &AbsolutePath {
        &self.directory
    }
}

/// A process group intentionally created for one direct host child.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OwnedProcessGroupId(libc::pid_t);

impl OwnedProcessGroupId {
    pub(crate) fn from_host_process_id(pid: HostProcessId) -> Result<Self, SupervisionError> {
        let pid =
            i32::try_from(pid.value()).map_err(|_| SupervisionError::InvalidProcessIdentity)?;
        if pid <= 0 {
            return Err(SupervisionError::InvalidProcessIdentity);
        }
        Ok(Self(pid))
    }

    pub const fn value(self) -> libc::pid_t {
        self.0
    }
}

/// A monotonic tick supplied by the daemon's control loop. Tests advance this
/// value directly; this subsystem never sleeps while holding child ownership.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MonotonicTick(u64);

impl MonotonicTick {
    pub const ZERO: Self = Self(0);

    pub const fn from_milliseconds(value: u64) -> Self {
        Self(value)
    }

    pub const fn milliseconds(self) -> u64 {
        self.0
    }

    fn checked_add(self, duration: CancellationDelay) -> Result<Self, SupervisionError> {
        self.0
            .checked_add(duration.value())
            .map(Self)
            .ok_or(SupervisionError::DeadlineOverflow)
    }
}

/// A control-loop deadline for a handshake observation. The supervisor never
/// blocks a daemon thread waiting for a silent adapter; callers poll at a
/// monotonic tick and expiry starts owned-child containment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandshakeDeadline {
    expires_at: MonotonicTick,
}

impl HandshakeDeadline {
    pub const fn at(expires_at: MonotonicTick) -> Self {
        Self { expires_at }
    }

    pub const fn expires_at(self) -> MonotonicTick {
        self.expires_at
    }
}

/// Deadline for a single ordered stdin frame. A frame is admitted to the
/// Rust peer before its first byte reaches the child, so an incomplete write
/// must either finish under this deadline or fail closed; later frames can
/// never overtake it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlWriteDeadline {
    expires_at: MonotonicTick,
}

impl ControlWriteDeadline {
    pub const fn at(expires_at: MonotonicTick) -> Self {
        Self { expires_at }
    }

    pub const fn expires_at(self) -> MonotonicTick {
        self.expires_at
    }
}

/// Progress of one strictly ordered adapter-control frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlWriteProgress {
    Delivered,
    Pending,
}

/// One exact host stdout frame which the [`BoundaryPeer`] has already sealed
/// and which the supervisor has strictly schema-decoded. The raw frame is
/// retained here because the resident M6 bridge must attest the host's actual
/// sequence and correlation to the kernel; [`PeerObservation`] alone
/// intentionally omits those transport coordinates. Semantic peer validation
/// remains an explicit closed outcome rather than an implication of this type
/// name. This is still transient process evidence, never a new wire protocol
/// or a durable fact by itself.
#[derive(Clone, Debug)]
pub struct SealedDecodedPeerFrame {
    frame: OutboundFrame,
    observation: Option<PeerObservation>,
    validation: PeerFrameValidation,
    /// The exact frame was valid and sealed, but its semantic effect fenced
    /// the peer (for example a typed unavailable-usage observation). The
    /// supervisor has already begun bounded containment; callers may still
    /// persist the frame's named durable consequence before reconciling the
    /// owned process group.
    peer_became_fatal: bool,
}

impl SealedDecodedPeerFrame {
    pub const fn frame(&self) -> &OutboundFrame {
        &self.frame
    }

    pub const fn observation(&self) -> Option<&PeerObservation> {
        self.observation.as_ref()
    }

    pub const fn validation(&self) -> &PeerFrameValidation {
        &self.validation
    }

    pub const fn peer_became_fatal(&self) -> bool {
        self.peer_became_fatal
    }
}

/// The raw stdout frame had a closed schema and was sealed by `BoundaryPeer`.
/// A semantic rejection is distinct from malformed transport: it lets the
/// daemon preserve a named accounting-failure consequence when the M6 kernel
/// permits one, while remaining terminally fenced at the peer.
#[derive(Clone, Debug)]
pub enum PeerFrameValidation {
    Accepted,
    Rejected(PeerError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CancellationDelay(u64);

impl CancellationDelay {
    pub const fn milliseconds(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancellationMode {
    Quiesce,
    GracefulCancel,
    EmergencyStop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancellationReason {
    OperatorStop,
    WallBudgetExpired,
    BudgetGuardrail,
    ProtocolContainment,
    DaemonRecovery,
}

/// Closed cancellation identity, supplied by the kernel/control plane rather
/// than synthesized from a signal or trace message.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CancellationRequestId(String);

impl CancellationRequestId {
    pub fn parse(value: impl Into<String>) -> Result<Self, SupervisionError> {
        let value = value.into();
        if !is_domain_identifier(&value) {
            return Err(SupervisionError::InvalidCancellationIdentity);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The externally admitted cancellation fact. There is no generic reason map
/// or arbitrary deadline override at this trusted boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CancellationRequest {
    pub cancellation_request_id: CancellationRequestId,
    pub mode: CancellationMode,
    pub reason: CancellationReason,
    pub observed_admission_generation: u64,
    pub abort_correlation_identity: CorrelationIdentity,
}

/// Explains whether a shutdown lineage came from an admitted kernel request
/// or from this boundary's fail-closed transport containment. The latter is
/// an in-memory safety action, not a fabricated durable cancellation command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancellationOrigin {
    ExplicitRequest,
    AutomaticBoundaryContainment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CancellationModeRevision {
    pub from: CancellationMode,
    pub to: CancellationMode,
    pub observed_at: MonotonicTick,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CancellationDeadlines {
    pub cooperative_abort_wait: CancellationDelay,
    pub terminate_wait: CancellationDelay,
}

impl CancellationDeadlines {
    pub const fn for_mode(mode: CancellationMode) -> Option<Self> {
        match mode {
            CancellationMode::Quiesce => None,
            CancellationMode::GracefulCancel => Some(Self {
                cooperative_abort_wait: CancellationDelay::milliseconds(5_000),
                terminate_wait: CancellationDelay::milliseconds(5_000),
            }),
            CancellationMode::EmergencyStop => Some(Self {
                cooperative_abort_wait: CancellationDelay::milliseconds(1_000),
                terminate_wait: CancellationDelay::milliseconds(2_000),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChildLifecycle {
    /// `fork`/`exec` succeeded and this supervisor owns the direct PID/PGID,
    /// but no fallible pipe or Pi-peer setup has been attempted yet.  The
    /// resident must durably register these facts before asking the host to
    /// participate in the adapter protocol.
    NativeSpawnRegistered,
    AwaitingAdapterReady,
    InertVerified,
    CreateSent,
    SessionReady,
    Quiescing,
    AwaitingCooperativeAbort,
    AwaitingTermination,
    AwaitingKill,
    Reaped,
    Contained,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessGroupLiveness {
    Absent,
    Present,
    /// POSIX reported a group but denied signal-zero. This supervisor never
    /// guesses it still owns that group after reaping the direct-child PID.
    Inaccessible,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChildTerminalDisposition {
    NotRunning,
    CompletedBeforeDelivery,
    CooperativelyAborted,
    Terminated,
    Killed,
    ContainmentFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalDelivery {
    AbortControlWritten,
    TermSent,
    KillSent,
    /// The direct child exited while a member of the owned process group was
    /// still alive. The supervisor sent one final SIGKILL before it touched
    /// potentially pipe-blocking stdout/stderr drains.
    LingeringGroupKillSent,
    /// A probe found a group the daemon cannot signal. This is explicit
    /// negative delivery evidence, not a successful cleanup claim.
    GroupInaccessible,
    /// The group was absent during the pre-send liveness probe; no signal was
    /// attempted.
    AbsentBeforeSignal,
    /// The group passed the pre-send probe but disappeared before `kill(2)`
    /// could deliver the signal; no signal was delivered.
    AbsentDuringSignal,
}

/// The supervisor intent behind a signal/control receipt.  Negative delivery
/// outcomes still retain this action, so the durable kernel never has to
/// guess whether an absent group was being terminated, killed, or cleaned up
/// after a direct-child exit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalAction {
    AbortControl,
    Terminate,
    Kill,
    LingeringGroupKill,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SignalGroupOutcome {
    AbsentBeforeSignal,
    InaccessibleBeforeSignal,
    AbsentDuringSignal,
    InaccessibleDuringSignal,
    Delivered {
        group_liveness_after_delivery: ProcessGroupLiveness,
    },
}

impl SignalGroupOutcome {
    const fn was_delivered(self) -> bool {
        matches!(self, Self::Delivered { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignalReceipt {
    pub action: SignalAction,
    pub delivery: SignalDelivery,
    pub observed_at: MonotonicTick,
    /// Liveness observed after the signal attempt or negative preflight. A
    /// negative delivery never pretends a signal was delivered.
    pub group_liveness_after_attempt: ProcessGroupLiveness,
}

/// Whether the later kernel receives every transient byte observed at this
/// pipe, or only a bounded prefix plus a digest of the full observed stream.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TransientRetention {
    #[default]
    Complete,
    PrefixBounded,
    /// More than `u64::MAX` bytes were observed. The digest is useful only as
    /// a bounded transient diagnostic; this receipt makes no exact-size claim
    /// and is terminally contained.
    CountOverflow,
}

/// An exact byte count is evidence only while arithmetic has not overflowed.
/// The later kernel must never coerce `Overflowed` to a cap, zero, or charge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransientByteCount {
    Exact(u64),
    Overflowed,
}

/// Raw transient pipe evidence handed to the later content-sealing authority.
/// This subsystem computes a digest but deliberately does **not** call that a
/// sealed content object or durable evidence record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransientStreamCapture {
    pub observed_byte_count: TransientByteCount,
    pub blake3: Blake3Digest,
    pub retention: TransientRetention,
    retained_bytes: Vec<u8>,
}

impl TransientStreamCapture {
    pub fn retained_bytes(&self) -> &[u8] {
        &self.retained_bytes
    }

    pub fn into_retained_bytes(self) -> Vec<u8> {
        self.retained_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransientExecutionEvidence {
    /// Full logical control frames accepted by the Rust peer. These bytes are
    /// not a claim that the native stdin pipe accepted them.
    pub admitted_control: TransientStreamCapture,
    /// Bytes actually accepted by successful native `write(2)` calls.
    pub stdin: TransientStreamCapture,
    pub stdout: TransientStreamCapture,
    pub stderr: TransientStreamCapture,
    pub logically_admitted_inbound_frame_count: u64,
    pub physically_delivered_inbound_frame_count: u64,
    pub outbound_frame_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReapStatus {
    Exited {
        code: i32,
    },
    Signaled {
        signal: i32,
    },
    /// POSIX supplied neither a numeric exit code nor a terminating signal.
    /// The receipt never invents signal zero as evidence.
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReapReceipt {
    pub child_process_id: SupervisedChildId,
    pub host_process_id: HostProcessId,
    pub process_group_id: OwnedProcessGroupId,
    pub status: ReapStatus,
    /// Direct-child reaping does not prove descendants were contained. This is
    /// the pre-cleanup process-group observation; the final field records the
    /// result after the one owned-group cleanup attempt.
    pub group_liveness_before_cleanup: ProcessGroupLiveness,
    pub group_liveness_after_reap: ProcessGroupLiveness,
}

/// The direct child has been collected by `wait(2)`, while its owned process
/// group remains a separate live subject. This is intentionally a one-shot
/// pre-lingering-cleanup fact: the resident persists it before issuing a
/// policy-driven group kill, so no later durable receipt invents a `Present`
/// observation after the signal was delivered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectChildReapFacts {
    pub child_process_id: SupervisedChildId,
    pub host_process_id: HostProcessId,
    pub process_group_id: OwnedProcessGroupId,
    pub status: ReapStatus,
    pub group_liveness_after_direct_child_reap: ProcessGroupLiveness,
    /// Exact TERM/KILL negative/delivery facts observed before `wait(2)`. A
    /// later lingering-group cleanup is intentionally absent here because it
    /// occurs only after this direct-reap fact is durable.
    pub prior_signal_receipts: Vec<SignalReceipt>,
}

/// This receipt is intentionally not a durable object. The later kernel is
/// responsible for sealing/persisting it and for deciding whether known usage
/// may be charged or an admission generation may change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupervisionReceipt {
    pub child_process_id: SupervisedChildId,
    pub session_identity: SessionIdentity,
    pub workspace_identity: NativeWorkspaceId,
    pub workspace_directory: AbsolutePath,
    pub terminal_disposition: ChildTerminalDisposition,
    pub reap: Option<ReapReceipt>,
    pub cancellation_deliveries: Vec<SignalReceipt>,
    pub cancellation_origin: Option<CancellationOrigin>,
    pub cancellation_mode_revisions: Vec<CancellationModeRevision>,
    /// The peer-validated canonical SessionManager path, if `SessionReady`
    /// occurred. It is a transient location fact only: the kernel later owns
    /// opening, sealing, and deciding whether its contents are admissible.
    pub canonical_session_file: Option<AbsolutePath>,
    /// Bounded raw bytes and full-stream transient digests. The kernel later
    /// chooses whether/how to content-seal them; no durable evidence exists
    /// merely because this receipt was returned.
    pub transient_evidence: TransientExecutionEvidence,
    pub peer_state: PiPeerReceiptState,
}

/// A reaping receipt must distinguish an adapter peer that was never
/// constructed from an adapter which reached `AwaitingAdapterReady`.  Calling
/// the former phase "awaiting" would fabricate protocol evidence after a
/// post-spawn setup failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PiPeerReceiptState {
    NotInitialized,
    Observed(PeerPhase),
}

/// Exact, pre-Create artifact identities for the Node host. Paths are
/// canonical regular files and digests are rechecked immediately before
/// `exec`; no ambient command lookup is used. In particular,
/// `pi_transitive_package_set` identifies a supplied package-set manifest. It
/// does **not** prove an arbitrary adapter entrypoint imports that manifest;
/// the adapter's v1 runtime report and the separately pinned host qualification
/// remain the evidence for Pi 0.84 behavior.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualifiedHostExecution {
    pub node_executable: VerifiedArtifact,
    pub adapter_entrypoint: VerifiedArtifact,
    pub lockfile: VerifiedArtifact,
    /// Identity prerequisite for the supplied package-set manifest, not a
    /// native proof of the adapter's dynamic import graph.
    pub pi_transitive_package_set: VerifiedArtifact,
    pub runtime: RuntimeIdentity,
}

impl QualifiedHostExecution {
    pub(crate) fn verify_before_spawn(&self) -> Result<(), SupervisionError> {
        self.runtime
            .assert_v1()
            .map_err(SupervisionError::Protocol)?;
        self.node_executable
            .verify_matches(&self.runtime.node_executable_blake3)?;
        self.adapter_entrypoint
            .verify_matches(&self.runtime.adapter_build_blake3)?;
        self.lockfile
            .verify_matches(&self.runtime.lockfile_blake3)?;
        self.pi_transitive_package_set
            .verify_matches(&self.runtime.pi_transitive_package_set_blake3)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedArtifact {
    path: AbsolutePath,
    expected_blake3: Blake3Digest,
}

impl VerifiedArtifact {
    pub fn inspect(
        path: impl AsRef<Path>,
        expected_blake3: Blake3Digest,
    ) -> Result<Self, SupervisionError> {
        let canonical = fs::canonicalize(path)?;
        let metadata = fs::metadata(&canonical)?;
        if !metadata.is_file() {
            return Err(SupervisionError::ArtifactIsNotRegularFile);
        }
        let path = absolute_path_from_path(&canonical)?;
        let artifact = Self {
            path,
            expected_blake3,
        };
        artifact.verify_matches(&artifact.expected_blake3)?;
        Ok(artifact)
    }

    pub fn path(&self) -> &AbsolutePath {
        &self.path
    }

    pub(crate) const fn expected_blake3(&self) -> &Blake3Digest {
        &self.expected_blake3
    }

    fn verify_matches(&self, expected: &Blake3Digest) -> Result<(), SupervisionError> {
        let observed = digest_file(self.path.as_path())?;
        if &observed != expected || &self.expected_blake3 != expected {
            return Err(SupervisionError::ArtifactDigestDrift);
        }
        Ok(())
    }

    /// Rechecks the canonical file against the identity established by
    /// [`Self::inspect`]. Native execution calls this immediately before
    /// `exec`, after its side-effect-free admission preflight has succeeded.
    pub(crate) fn verify_current_identity(&self) -> Result<(), SupervisionError> {
        self.verify_matches(&self.expected_blake3)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PiSpawnRequest {
    pub child_process_id: SupervisedChildId,
    pub workspace: NativeWorkspace,
    pub session_identity: SessionIdentity,
    pub spawn_nonce: SpawnNonce,
    pub host_execution: QualifiedHostExecution,
    /// The current pinned host contract grants no inherited environment values.
    /// The empty process environment is intentional evidence, not an omitted
    /// default; future allowlists must become a new closed version.
    pub environment: NativeHostEnvironment,
    pub create_correlation_identity: CorrelationIdentity,
    pub create_session: CreateSessionPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeHostEnvironment {
    EmptyV1,
}

/// A narrow synchronous seam for the kernel's final transactional recheck.
/// It is deliberately invoked only after Rust has recorded the child PID in
/// memory and validated the inert AdapterReady frame, immediately before the
/// first command which can create a Pi session.
pub trait PreCreateAdmissionGate {
    fn recheck(&mut self, facts: &InertChildFacts) -> Result<(), AdmissionDenied>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InertChildFacts {
    pub child_process_id: SupervisedChildId,
    pub session_identity: SessionIdentity,
    pub host_process_id: HostProcessId,
    pub process_group_id: OwnedProcessGroupId,
    pub workspace_identity: NativeWorkspaceId,
    pub workspace_directory: AbsolutePath,
    pub runtime: RuntimeIdentity,
    pub environment: NativeHostEnvironment,
}

/// Facts known immediately after `fork`/`exec` ownership succeeds, before
/// the untrusted host has emitted any adapter protocol.  The daemon records
/// these exact native identities promptly so a silent inert host is still a
/// durable containment subject; `AdapterReady` remains a separate later
/// proof of the host's claimed session/nonce/runtime identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpawnedChildFacts {
    pub child_process_id: SupervisedChildId,
    pub session_identity: SessionIdentity,
    pub spawn_nonce: SpawnNonce,
    pub host_process_id: HostProcessId,
    pub process_group_id: OwnedProcessGroupId,
    pub workspace_identity: NativeWorkspaceId,
    pub workspace_directory: AbsolutePath,
    pub runtime: RuntimeIdentity,
    pub environment: NativeHostEnvironment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionDenied {
    StaleGeneration,
    CancellationObserved,
    CapabilityAbsent,
    ReservationAbsent,
}

#[derive(Debug, Error)]
pub enum SupervisionError {
    #[error("process supervision I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("native Pi host exec did not create a child: {0}")]
    NativeSpawn(#[source] io::Error),
    #[error("Pi boundary rejected a frame: {0}")]
    Peer(#[from] PeerError),
    #[error("Pi protocol construction failed: {0}")]
    Protocol(#[from] society_pi::ProtocolError),
    #[error("child identity is invalid")]
    InvalidChildIdentity,
    #[error("workspace identity is invalid")]
    InvalidWorkspaceIdentity,
    #[error("cancellation identity is invalid")]
    InvalidCancellationIdentity,
    #[error("process identity is invalid")]
    InvalidProcessIdentity,
    #[error("fresh workspace allocation did not remain a private direct child")]
    UnsafeWorkspace,
    #[error("workspace root is not an existing private directory owned by this daemon user")]
    UnsafeWorkspaceRoot,
    #[error("workspace identity already has an allocated direct child")]
    WorkspaceAlreadyExists,
    #[error("qualified artifact is not a regular file")]
    ArtifactIsNotRegularFile,
    #[error("qualified artifact digest drifted")]
    ArtifactDigestDrift,
    #[error("spawn request does not exactly bind its workspace/profile")]
    InvalidSpawnRequest,
    #[error("native child request does not exactly bind its closed profile/artifacts")]
    InvalidNativeChildRequest,
    #[error("registered native child has no stdout pipe")]
    MissingNativeChildStdout,
    #[error("registered native child has no stderr pipe")]
    MissingNativeChildStderr,
    #[error("native child exceeded the fixed bounded output capture")]
    NativeChildOutputLimitExceeded,
    #[error("native child output capture could not be observed")]
    NativeChildOutputCaptureFailed,
    #[error("child id was already supervised; successors are never automatic")]
    DuplicateChildIdentity,
    #[error("child operation is invalid in its closed lifecycle")]
    InvalidLifecycle,
    #[error("a registered native Pi child could not complete post-spawn setup: {0}")]
    PostSpawnSetup(PostSpawnSetupFailure),
    #[error("the final pre-Create admission recheck denied this inert child")]
    AdmissionDenied(AdmissionDenied),
    #[error("stdout record exceeded the closed Pi frame bound")]
    OutboundFrameTooLarge,
    #[error("host stdout ended in an unterminated JSONL record")]
    UnterminatedOutboundRecord,
    #[error("host stdout ended before an evidenced terminal frame")]
    OutputLost,
    #[error("process group operation failed: {0}")]
    ProcessGroup(io::Error),
    #[error("monotonic cancellation deadline overflowed")]
    DeadlineOverflow,
    #[error("stderr capture thread did not complete")]
    StderrCaptureFailed,
    #[error("a contained child is still live; drive its bounded cancellation before reaping")]
    ContainmentAwaitingDrive,
    #[error("Pi host handshake did not become ready before its typed deadline")]
    HandshakeDeadlineExpired,
    #[error("the ordered Pi control write did not drain before its typed deadline")]
    ControlWriteDeadlineExpired,
    #[error("the Pi control pipe accepted a zero-byte write")]
    ControlWriteZero,
}

/// Closed classification for an error after the direct PID/PGID became an
/// owned native fact.  These are intentionally distinct from
/// `NativeSpawn`: the resident must record the child before it contains and
/// reaps this outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostSpawnSetupFailure {
    BoundaryPeer,
    MissingStdinPipe,
    StdinNonblocking,
    MissingStdoutPipe,
    StdoutNonblocking,
}

impl std::fmt::Display for PostSpawnSetupFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::BoundaryPeer => "boundary peer construction",
            Self::MissingStdinPipe => "missing child stdin pipe",
            Self::StdinNonblocking => "stdin nonblocking configuration",
            Self::MissingStdoutPipe => "missing child stdout pipe",
            Self::StdoutNonblocking => "stdout nonblocking configuration",
        })
    }
}

impl From<AdmissionDenied> for SupervisionError {
    fn from(value: AdmissionDenied) -> Self {
        Self::AdmissionDenied(value)
    }
}

/// The in-memory process registry. It is intentionally single-owner and has
/// no clone/automatic-restart API: a successor needs a new kernel admission.
pub struct PiSupervisor {
    children: BTreeMap<SupervisedChildId, ManagedPiChild>,
    historical_child_ids: BTreeSet<SupervisedChildId>,
    #[cfg(feature = "test-support")]
    post_spawn_setup_fault: Option<PostSpawnSetupFailure>,
}

impl Default for PiSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl PiSupervisor {
    pub fn new() -> Self {
        Self {
            children: BTreeMap::new(),
            historical_child_ids: BTreeSet::new(),
            #[cfg(feature = "test-support")]
            post_spawn_setup_fault: None,
        }
    }

    /// Provider-free test seam for the otherwise platform-dependent pipe and
    /// peer initialization failures. It is compiled only with the explicit
    /// `test-support` feature and has no daemon/wire representation.
    #[cfg(feature = "test-support")]
    pub fn with_post_spawn_setup_fault_for_test(failure: PostSpawnSetupFailure) -> Self {
        Self {
            children: BTreeMap::new(),
            historical_child_ids: BTreeSet::new(),
            post_spawn_setup_fault: Some(failure),
        }
    }

    /// Verifies exact artifacts, creates a native process group, and performs
    /// the remaining local setup as a convenience for standalone supervision
    /// users.  The resident M5 bridge instead uses [`Self::spawn_native`] and
    /// [`Self::finish_inert_setup`] separately so it can commit the native
    /// PID/PGID receipt before any fallible pipe or Pi-peer setup.
    pub fn spawn_inert(
        &mut self,
        request: PiSpawnRequest,
    ) -> Result<SpawnedChildFacts, SupervisionError> {
        let facts = self.spawn_native(request)?;
        // The registered child remains owned by this supervisor in automatic
        // containment if setup fails. A standalone caller may drive/reap it;
        // its Drop path remains a final physical containment backstop.
        self.finish_inert_setup(&facts.child_process_id, MonotonicTick::ZERO)?;
        Ok(facts)
    }

    /// Side-effect-free validation for the resident's durable pre-spawn
    /// admission. It deliberately performs the same path/artifact checks as
    /// `spawn_native`, then the latter rechecks immediately before `exec` to
    /// close the TOCTOU window. A failure here occurs before any ledger
    /// admission or native process exists.
    pub fn preflight_spawn(&self, request: &PiSpawnRequest) -> Result<(), SupervisionError> {
        if self
            .historical_child_ids
            .contains(&request.child_process_id)
        {
            return Err(SupervisionError::DuplicateChildIdentity);
        }
        validate_spawn_request(request)?;
        request.host_execution.verify_before_spawn()?;
        Ok(())
    }

    /// Creates and registers the native direct child before any operation
    /// which can fail while configuring pipes or constructing the Pi peer.
    /// The returned identities are the exact facts the resident must durably
    /// record before it calls [`Self::finish_inert_setup`].
    pub fn spawn_native(
        &mut self,
        request: PiSpawnRequest,
    ) -> Result<SpawnedChildFacts, SupervisionError> {
        self.preflight_spawn(&request)?;

        let mut command = Command::new(request.host_execution.node_executable.path().as_path());
        command
            .arg(request.host_execution.adapter_entrypoint.path().as_path())
            .arg("--session-identity")
            .arg(request.session_identity.as_str())
            .arg("--spawn-nonce")
            .arg(request.spawn_nonce.as_str())
            .arg("--node-executable-blake3")
            .arg(
                request
                    .host_execution
                    .runtime
                    .node_executable_blake3
                    .as_str(),
            )
            .arg("--lockfile-blake3")
            .arg(request.host_execution.runtime.lockfile_blake3.as_str())
            .arg("--adapter-build-blake3")
            .arg(request.host_execution.runtime.adapter_build_blake3.as_str())
            .arg("--pi-transitive-package-set-blake3")
            .arg(
                request
                    .host_execution
                    .runtime
                    .pi_transitive_package_set_blake3
                    .as_str(),
            )
            .current_dir(request.workspace.directory().as_path())
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut native_child = spawn_owned_native_child(command)?;
        // `Stdio::piped` makes these handles expected, but retaining their
        // optional shape until `finish_inert_setup` means a hypothetical
        // platform/runtime violation is a *registered* setup failure rather
        // than an orphaned physical child.
        let stdin = native_child.child.stdin.take();
        let stdout = native_child.child.stdout.take().map(BufReader::new);
        let stderr_capture = native_child.child.stderr.take().map(spawn_stderr_capture);
        let child_process_id = request.child_process_id.clone();
        self.historical_child_ids.insert(child_process_id.clone());
        self.children.insert(
            child_process_id.clone(),
            ManagedPiChild {
                request,
                native_child,
                stdin,
                stdout,
                stderr_capture,
                peer: None,
                lifecycle: ChildLifecycle::NativeSpawnRegistered,
                next_inbound_sequence: 1,
                stdin_capture: StreamCapture::default(),
                admitted_control_capture: StreamCapture::default(),
                stdout_capture: StreamCapture::default(),
                stdout_partial_record: Vec::new(),
                pending_control: None,
                #[cfg(feature = "test-support")]
                force_next_control_write_pending_for_test: false,
                pending_direct_reap: None,
                physically_delivered_inbound_frame_count: 0,
                cancellation: None,
                deliveries: Vec::new(),
                completed_receipt: None,
            },
        );
        self.children
            .get(&child_process_id)
            .map(ManagedPiChild::spawned_facts)
            .ok_or(SupervisionError::InvalidLifecycle)
    }

    /// Completes local pipe/Pi-peer setup only after the caller has made the
    /// native `SpawnedChildFacts` durable.  A failure starts automatic owned
    /// containment but leaves the child in the registry for typed
    /// cancellation, reaping, stream sealing, and final reconciliation.
    pub fn finish_inert_setup(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<(), SupervisionError> {
        #[cfg(feature = "test-support")]
        let injected_fault = self.post_spawn_setup_fault.take();
        let child = self.child_mut(child_process_id)?;
        if child.lifecycle != ChildLifecycle::NativeSpawnRegistered {
            return Err(SupervisionError::InvalidLifecycle);
        }
        #[cfg(feature = "test-support")]
        if let Some(failure) = injected_fault {
            child.start_automatic_boundary_containment(now)?;
            return Err(SupervisionError::PostSpawnSetup(failure));
        }
        if let Err(failure) = child.finish_inert_setup() {
            child.start_automatic_boundary_containment(now)?;
            return Err(SupervisionError::PostSpawnSetup(failure));
        }
        child.lifecycle = ChildLifecycle::AwaitingAdapterReady;
        Ok(())
    }

    pub fn lifecycle(&self, child_process_id: &SupervisedChildId) -> Option<ChildLifecycle> {
        self.children
            .get(child_process_id)
            .map(|child| child.lifecycle)
    }

    #[cfg(feature = "test-support")]
    pub fn registered_child_count_for_test(&self) -> usize {
        self.children.len()
    }

    /// Injects one non-writing `WouldBlock` equivalent after logical peer
    /// admission. This is test-only process physics: it proves callers do
    /// not confuse a pending control suffix with a delivered native frame.
    #[cfg(feature = "test-support")]
    pub fn force_next_control_write_pending_for_test(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<(), SupervisionError> {
        self.child_mut(child_process_id)?
            .force_next_control_write_pending_for_test = true;
        Ok(())
    }

    /// Records that the host boundary is no longer trustworthy, closes its
    /// control writer, and starts the fixed EmergencyStop escalation. This is
    /// deliberately automatic containment, not a synthetic kernel command.
    pub fn contain_boundary_failure(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<(), SupervisionError> {
        self.child_mut(child_process_id)?
            .start_automatic_boundary_containment(now)
    }

    /// Polls for the inert AdapterReady record and proves it belongs to the
    /// direct child/pgroup just spawned by this supervisor. `Ok(None)` is a
    /// normal not-ready poll before `deadline`; expiry itself contains the
    /// owned child rather than leaving a silent host unowned.
    pub fn observe_adapter_ready_at(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<Option<InertChildFacts>, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        if child.lifecycle != ChildLifecycle::AwaitingAdapterReady {
            return Err(SupervisionError::InvalidLifecycle);
        }
        match child.read_one_outbound() {
            Ok(OutboundRead::NotReady) => {
                if now >= deadline.expires_at() {
                    child.start_automatic_boundary_containment(now)?;
                    return Err(SupervisionError::HandshakeDeadlineExpired);
                }
                return Ok(None);
            }
            Ok(OutboundRead::Observation(_)) => {
                if child.peer()?.phase() == PeerPhase::Fatal {
                    child.start_automatic_boundary_containment(now)?;
                    return Err(SupervisionError::Peer(PeerError::Fatal));
                }
            }
            Err(error) => {
                child.start_automatic_boundary_containment(now)?;
                return Err(error);
            }
        }
        let owns_expected_group = match child.owns_expected_process_group() {
            Ok(owns_group) => owns_group,
            Err(error) => {
                child.start_automatic_boundary_containment(now)?;
                return Err(error);
            }
        };
        if child.peer()?.phase() != PeerPhase::Inert || !owns_expected_group {
            child.start_automatic_boundary_containment(now)?;
            return Err(SupervisionError::InvalidLifecycle);
        }
        child.lifecycle = ChildLifecycle::InertVerified;
        Ok(Some(child.inert_facts()))
    }

    /// Runs the final synchronous admission gate and only then writes the
    /// first session-creating command to the owned stdin pipe.
    pub fn send_create_session<G: PreCreateAdmissionGate>(
        &mut self,
        child_process_id: &SupervisedChildId,
        gate: &mut G,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, SupervisionError> {
        let facts = {
            let child = self.child_mut(child_process_id)?;
            if child.lifecycle != ChildLifecycle::InertVerified {
                return Err(SupervisionError::InvalidLifecycle);
            }
            child.inert_facts()
        };
        if let Err(denial) = gate.recheck(&facts) {
            // The host is still inert. Closing the sole control writer makes
            // the v1 adapter exit without ever constructing an AgentSession.
            let child = self.child_mut(child_process_id)?;
            child.stdin.take();
            child.lifecycle = ChildLifecycle::Quiescing;
            return Err(denial.into());
        }
        let child = self.child_mut(child_process_id)?;
        let frame = child.next_frame(
            child.request.create_correlation_identity.clone(),
            InboundCommand::CreateSession(Box::new(child.request.create_session.clone())),
        )?;
        let progress =
            match child.stage_inbound(frame, PendingControlCommand::CreateSession, now, deadline) {
                Ok(progress) => progress,
                Err(error) => {
                    child.start_automatic_boundary_containment(now)?;
                    return Err(error);
                }
            };
        child.lifecycle = ChildLifecycle::CreateSent;
        Ok(progress)
    }

    /// Drains the finite CreateSession handshake. The peer validates command
    /// correlation, runtime/profile equality, and effective SessionReady.
    pub fn observe_session_ready_at(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        if child.lifecycle != ChildLifecycle::CreateSent {
            return Err(SupervisionError::InvalidLifecycle);
        }
        if child.pending_control.is_some() {
            return Ok(false);
        }
        for _ in 0..MAX_HANDSHAKE_FRAMES {
            match child.read_one_outbound() {
                Ok(OutboundRead::NotReady) => {
                    if now >= deadline.expires_at() {
                        child.start_automatic_boundary_containment(now)?;
                        return Err(SupervisionError::HandshakeDeadlineExpired);
                    }
                    return Ok(false);
                }
                Ok(OutboundRead::Observation(_)) => {
                    if child.peer()?.phase() == PeerPhase::Fatal {
                        child.start_automatic_boundary_containment(now)?;
                        return Err(SupervisionError::Peer(PeerError::Fatal));
                    }
                }
                Err(error) => {
                    child.start_automatic_boundary_containment(now)?;
                    return Err(error);
                }
            }
            if child.peer()?.phase() == PeerPhase::Ready {
                child.lifecycle = ChildLifecycle::SessionReady;
                return Ok(true);
            }
        }
        child.start_automatic_boundary_containment(now)?;
        Err(SupervisionError::InvalidLifecycle)
    }

    /// Stages one already-authorized Office Prompt on the same nonblocking
    /// control path as CreateSession and Dispose.  This function has no
    /// kernel authority: its caller must first persist the M6 prompt
    /// authorization, then record delivery only after `Delivered`.
    pub fn send_prompt(
        &mut self,
        child_process_id: &SupervisedChildId,
        correlation_identity: CorrelationIdentity,
        payload: PromptPayload,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        if child.lifecycle != ChildLifecycle::SessionReady {
            return Err(SupervisionError::InvalidLifecycle);
        }
        let frame = child.next_frame(correlation_identity, InboundCommand::Prompt(payload))?;
        match child.stage_inbound(frame, PendingControlCommand::Prompt, now, deadline) {
            Ok(progress) => Ok(progress),
            Err(error) => {
                child.start_automatic_boundary_containment(now)?;
                Err(error)
            }
        }
    }

    /// Returns one durable-result-bearing Forum call to the host. The caller
    /// must first commit the corresponding typed study transition; this
    /// method only owns ordered native pipe delivery and peer validation.
    #[allow(clippy::too_many_arguments)]
    pub fn send_forum_tool_result(
        &mut self,
        child_process_id: &SupervisedChildId,
        correlation_identity: CorrelationIdentity,
        tool_call_identity: ToolCallIdentity,
        result: society_pi::SdkJsonValue,
        is_error: bool,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        if child.lifecycle != ChildLifecycle::SessionReady {
            return Err(SupervisionError::InvalidLifecycle);
        }
        let frame = child.next_frame(
            correlation_identity,
            InboundCommand::ForumToolResult(society_pi::ForumToolResultPayload {
                tool_call_identity,
                result,
                is_error,
            }),
        )?;
        match child.stage_inbound(frame, PendingControlCommand::ForumToolResult, now, deadline) {
            Ok(progress) => Ok(progress),
            Err(error) => {
                child.start_automatic_boundary_containment(now)?;
                Err(error)
            }
        }
    }

    /// Stages an observation-only GetState control while a session is live.
    /// It carries no kernel authority and exists so the resident control loop
    /// can observe a host without pretending its resulting usage snapshot is
    /// the active Prompt's final accounting proof.
    #[cfg(test)]
    pub(crate) fn send_get_state(
        &mut self,
        child_process_id: &SupervisedChildId,
        correlation_identity: CorrelationIdentity,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        if child.lifecycle != ChildLifecycle::SessionReady {
            return Err(SupervisionError::InvalidLifecycle);
        }
        let frame = child.next_frame(correlation_identity, InboundCommand::GetState)?;
        match child.stage_inbound(frame, PendingControlCommand::GetState, now, deadline) {
            Ok(progress) => Ok(progress),
            Err(error) => {
                child.start_automatic_boundary_containment(now)?;
                Err(error)
            }
        }
    }

    pub fn send_dispose(
        &mut self,
        child_process_id: &SupervisedChildId,
        correlation_identity: CorrelationIdentity,
        reason: DisposeReason,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        if child.lifecycle != ChildLifecycle::SessionReady {
            return Err(SupervisionError::InvalidLifecycle);
        }
        let frame = child.next_frame(
            correlation_identity,
            InboundCommand::Dispose(DisposePayload { reason }),
        )?;
        let progress =
            match child.stage_inbound(frame, PendingControlCommand::Dispose, now, deadline) {
                Ok(progress) => progress,
                Err(error) => {
                    child.start_automatic_boundary_containment(now)?;
                    return Err(error);
                }
            };
        child.lifecycle = ChildLifecycle::Quiescing;
        Ok(progress)
    }

    pub fn observe_disposed_at(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, SupervisionError> {
        let observed = self.observe_disposal_output_at(child_process_id, now, deadline)?;
        if observed.is_none() {
            return Ok(false);
        }
        Ok(self.child_mut(child_process_id)?.peer()?.phase() == PeerPhase::Disposed)
    }

    /// Reads exactly one disposal-phase stdout frame without discarding its
    /// peer-sealed transport coordinates. The daemon's final Office-session
    /// accounting bridge needs the accepted Dispose result, final cumulative
    /// usage, and typed transcript receipt in their actual sequence; a bool
    /// `Disposed` result cannot honestly reconstruct those facts later.
    ///
    /// This remains native process physics only. A caller must commit any
    /// durable session-terminal receipt before it treats the returned frame as
    /// authoritative, and malformed/output-loss cases still begin bounded
    /// containment here.
    pub fn observe_disposal_output_at(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<Option<SealedDecodedPeerFrame>, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        if child.lifecycle != ChildLifecycle::Quiescing {
            return Err(SupervisionError::InvalidLifecycle);
        }
        if child.pending_control.is_some() {
            return Ok(None);
        }
        match child.read_one_outbound() {
            Ok(OutboundRead::NotReady) => {
                if now >= deadline.expires_at() {
                    child.start_automatic_boundary_containment(now)?;
                    return Err(SupervisionError::HandshakeDeadlineExpired);
                }
                Ok(None)
            }
            Ok(OutboundRead::Observation(observation)) => {
                if observation.peer_became_fatal() {
                    child.start_automatic_boundary_containment(now)?;
                }
                if child.peer()?.phase() == PeerPhase::Disposed {
                    child.stdin.take();
                }
                Ok(Some(*observation))
            }
            Err(error) => {
                child.start_automatic_boundary_containment(now)?;
                Err(error)
            }
        }
    }

    /// Observes one non-handshake output frame while a session or cancellation
    /// is live. This makes malformed/oversize stream containment explicit to
    /// the control loop without pretending the frame was a normal handshake.
    pub fn observe_live_output_at(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<Option<SealedDecodedPeerFrame>, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        match child.read_one_outbound() {
            Ok(OutboundRead::NotReady) => Ok(None),
            Ok(OutboundRead::Observation(observation)) => {
                if observation.peer_became_fatal() {
                    child.start_automatic_boundary_containment(now)?;
                }
                Ok(Some(*observation))
            }
            Err(error) => {
                child.start_automatic_boundary_containment(now)?;
                Err(error)
            }
        }
    }

    /// Advances the one admitted stdin frame without blocking the daemon. A
    /// `Pending` result leaves the exact byte suffix in place; expiry fences
    /// the host and starts emergency containment. The caller must drive this
    /// alongside handshake/cancellation ticks.
    pub fn drive_control_write(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        if child.pending_control.is_none() {
            return Err(SupervisionError::InvalidLifecycle);
        }
        match child.flush_pending_control(now) {
            Ok(progress) => Ok(progress),
            Err(error) => {
                child.start_automatic_boundary_containment(now)?;
                Err(error)
            }
        }
    }

    /// Starts a typed cancellation lineage. Quiesce never signals a running
    /// child. Graceful/Emergency cancellation writes the SDK Abort control
    /// first when the session handshake completed; deadline escalation occurs
    /// only through [`Self::drive_cancellation`].
    pub fn request_cancellation(
        &mut self,
        child_process_id: &SupervisedChildId,
        request: CancellationRequest,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        // A partially delivered CreateSession is not an inert operation: if
        // its suffix later reaches the host after cancellation, it could begin
        // paid work. Every cancellation therefore closes the writer and drops
        // the pending suffix before it can be resumed.
        child.discard_pending_control_before_cancellation();
        if let Some(existing) = child.cancellation.as_mut() {
            upgrade_or_replay_cancellation(existing, request, now)?;
            return Ok(ControlWriteProgress::Delivered);
        }
        if matches!(
            child.lifecycle,
            ChildLifecycle::Reaped | ChildLifecycle::Contained
        ) {
            return Err(SupervisionError::InvalidLifecycle);
        }
        let mode = request.mode;
        let Some(deadlines) = CancellationDeadlines::for_mode(mode) else {
            // Quiesce is an admission fence, not a signal. Keep an already
            // ready session live so the caller may still send its explicit
            // close/dispose control; this supervisor exposes no work-admit
            // method while a cancellation lineage exists.
            child.cancellation = Some(CancellationProgress::quiesce(request));
            return Ok(ControlWriteProgress::Delivered);
        };
        let abort_deadline = now.checked_add(deadlines.cooperative_abort_wait)?;
        let kill_deadline = abort_deadline.checked_add(deadlines.terminate_wait)?;
        let stage_abort = child.lifecycle == ChildLifecycle::SessionReady;
        let abort_reason = abort_reason_for(request.reason);
        let abort_correlation_identity = request.abort_correlation_identity.clone();
        child.lifecycle = ChildLifecycle::AwaitingCooperativeAbort;
        child.cancellation = Some(CancellationProgress {
            explicit_request: Some(request),
            origin: CancellationOrigin::ExplicitRequest,
            mode,
            mode_revisions: Vec::new(),
            abort_deadline,
            kill_deadline,
            abort_control_written: false,
            term_sent: false,
            term_delivered: false,
            kill_sent: false,
            kill_delivered: false,
        });
        if stage_abort {
            let frame = child.next_frame(
                abort_correlation_identity,
                InboundCommand::Abort(AbortPayload {
                    reason: abort_reason,
                }),
            )?;
            match child.stage_inbound(
                frame,
                PendingControlCommand::Abort,
                now,
                ControlWriteDeadline::at(abort_deadline),
            ) {
                Ok(progress) => Ok(progress),
                Err(error) => {
                    child.start_automatic_boundary_containment(now)?;
                    Err(error)
                }
            }
        } else {
            Ok(ControlWriteProgress::Delivered)
        }
    }

    /// Advances deterministic cancellation physics without sleeping. The
    /// daemon control loop calls this with its monotonic tick; tests can cover
    /// each race by choosing exact tick values.
    pub fn drive_cancellation(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<Option<SupervisionReceipt>, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        if let Some(receipt) = child.try_reap(now)? {
            return Ok(Some(receipt));
        }
        child.advance_cancellation_without_reap(now)?;
        Ok(None)
    }

    /// Advances only TERM/KILL deadlines. The M5 bridge uses this after a
    /// post-spawn setup failure so its later explicit direct-child wait can
    /// be durably recorded before any lingering-group cleanup occurs.
    pub fn drive_cancellation_without_reap(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<(), SupervisionError> {
        self.child_mut(child_process_id)?
            .advance_cancellation_without_reap(now)
    }

    /// Polls the direct child and retains its actual wait status without
    /// touching a still-live owned process group. The caller must first make
    /// this fact durable, then call the distinct lingering-cleanup methods
    /// below. This is the only path suitable for M5's direct-reap-before-
    /// lingering-signal ledger ordering.
    pub fn poll_direct_child_reap_at(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<Option<DirectChildReapFacts>, SupervisionError> {
        self.child_mut(child_process_id)?.poll_direct_child_reap()
    }

    /// Issues the one policy cleanup signal for a group that was still
    /// present/inaccessible after its direct child had already been reaped.
    /// It returns the exact signal/negative-delivery observation for a later
    /// durable receipt; `Absent` needs no signal and returns `None`.
    pub fn issue_lingering_group_cleanup(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<Option<SignalReceipt>, SupervisionError> {
        self.child_mut(child_process_id)?
            .issue_lingering_group_cleanup(now)
    }

    /// Reads group liveness after the distinct lingering cleanup signal. This
    /// observation is intentionally separate from the direct-child wait fact.
    pub fn observe_group_liveness(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<ProcessGroupLiveness, SupervisionError> {
        self.child_mut(child_process_id)?.group_liveness()
    }

    /// Once the daemon has recorded the direct wait and any required signal/
    /// liveness facts, completes bounded pipe capture and produces the stable
    /// receipt for content sealing/finalization. A still-live/inaccessible
    /// group is not drained or finalized.
    pub fn complete_deferred_reap_at(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<SupervisionReceipt, SupervisionError> {
        self.child_mut(child_process_id)?.complete_deferred_reap()
    }

    /// Polls the direct child without blocking the resident control loop.
    /// Once a reap receipt exists, this returns its stable copy while keeping
    /// ownership in the supervisor; only `take_reaped_receipt` releases that
    /// child after the daemon has durably recorded the receipt chain.
    pub fn poll_reap_at(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<Option<SupervisionReceipt>, SupervisionError> {
        self.child_mut(child_process_id)?.try_reap(now)
    }

    /// Waits/reaps the direct child and returns all partial pipe evidence even
    /// when cancellation or stdout loss prevented normal settlement.
    pub fn wait_and_reap(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<SupervisionReceipt, SupervisionError> {
        self.wait_and_reap_at(child_process_id, MonotonicTick::ZERO)
    }

    /// Reaps an already-exited child. A child that entered automatic boundary
    /// containment is never waited on indefinitely: the control loop must
    /// drive its fixed TERM/KILL deadlines and call this again.
    pub fn wait_and_reap_at(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<SupervisionReceipt, SupervisionError> {
        let child = self.child_mut(child_process_id)?;
        child.wait_and_reap_at(now)
    }

    pub fn take_reaped_receipt(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Option<SupervisionReceipt> {
        self.children
            .get(child_process_id)
            .and_then(|child| child.completed_receipt.as_ref())?;
        self.children
            .remove(child_process_id)
            .and_then(|mut child| child.completed_receipt.take())
    }

    fn child_mut(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<&mut ManagedPiChild, SupervisionError> {
        self.children
            .get_mut(child_process_id)
            .ok_or(SupervisionError::InvalidLifecycle)
    }
}

struct ManagedPiChild {
    request: PiSpawnRequest,
    native_child: crate::native_child::OwnedNativeChildProcess,
    stdin: Option<ChildStdin>,
    stdout: Option<BufReader<ChildStdout>>,
    stderr_capture: Option<StderrCaptureTask>,
    /// Constructed only after `SpawnedChildFacts` has been durably recorded.
    /// A setup-contained child therefore has no fabricated protocol phase.
    peer: Option<BoundaryPeer>,
    lifecycle: ChildLifecycle,
    next_inbound_sequence: u64,
    stdin_capture: StreamCapture,
    admitted_control_capture: StreamCapture,
    stdout_capture: StreamCapture,
    stdout_partial_record: Vec<u8>,
    pending_control: Option<PendingControlWrite>,
    #[cfg(feature = "test-support")]
    force_next_control_write_pending_for_test: bool,
    pending_direct_reap: Option<PendingDirectChildReap>,
    physically_delivered_inbound_frame_count: u64,
    cancellation: Option<CancellationProgress>,
    deliveries: Vec<SignalReceipt>,
    completed_receipt: Option<SupervisionReceipt>,
}

enum OutboundRead {
    NotReady,
    /// The exceptional M6 path needs the full schema-decoded frame, which can
    /// contain raw JSON evidence. Heap-own it so the normal polling enum stays
    /// small and does not retain that payload on every `NotReady` result.
    Observation(Box<SealedDecodedPeerFrame>),
}

/// The Rust peer has admitted these exact bytes, but the native pipe has not
/// necessarily accepted all of them. The bytes stay private/transient and are
/// never rewritten, coalesced with another frame, or overtaken.
struct PendingControlWrite {
    bytes: Vec<u8>,
    next_byte_offset: usize,
    deadline: ControlWriteDeadline,
    command: PendingControlCommand,
}

/// Owns the actual `ExitStatus` after `wait(2)` until the daemon has made the
/// direct-child receipt durable and either observed no lingering group or
/// issued its distinct policy cleanup action.
struct PendingDirectChildReap {
    status: ExitStatus,
    group_liveness_after_direct_child_reap: ProcessGroupLiveness,
    prior_signal_receipts: Vec<SignalReceipt>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingControlCommand {
    CreateSession,
    Prompt,
    ForumToolResult,
    #[cfg(test)]
    GetState,
    Abort,
    Dispose,
}

/// The reader updates a shared transient snapshot after every read. If an
/// escaped descendant keeps stderr open, completion never waits indefinitely
/// for that foreign process; it returns the snapshot as bounded partial input
/// for later kernel sealing.
struct StderrCaptureTask {
    snapshot: Arc<Mutex<StreamCapture>>,
    worker: JoinHandle<io::Result<()>>,
}

struct CancellationProgress {
    explicit_request: Option<CancellationRequest>,
    origin: CancellationOrigin,
    mode: CancellationMode,
    mode_revisions: Vec<CancellationModeRevision>,
    abort_deadline: MonotonicTick,
    kill_deadline: MonotonicTick,
    abort_control_written: bool,
    /// Whether a TERM/KILL attempt was completed (including a negative
    /// absence/inaccessibility observation) so escalation cannot loop.
    term_sent: bool,
    term_delivered: bool,
    kill_sent: bool,
    kill_delivered: bool,
}

impl CancellationProgress {
    fn quiesce(request: CancellationRequest) -> Self {
        Self {
            explicit_request: Some(request),
            origin: CancellationOrigin::ExplicitRequest,
            mode: CancellationMode::Quiesce,
            mode_revisions: Vec::new(),
            abort_deadline: MonotonicTick::ZERO,
            kill_deadline: MonotonicTick::ZERO,
            abort_control_written: false,
            term_sent: false,
            term_delivered: false,
            kill_sent: false,
            kill_delivered: false,
        }
    }

    fn automatic_boundary_containment(now: MonotonicTick) -> Result<Self, SupervisionError> {
        let deadlines = CancellationDeadlines::for_mode(CancellationMode::EmergencyStop)
            .ok_or(SupervisionError::InvalidLifecycle)?;
        let abort_deadline = now.checked_add(deadlines.cooperative_abort_wait)?;
        Ok(Self {
            explicit_request: None,
            origin: CancellationOrigin::AutomaticBoundaryContainment,
            mode: CancellationMode::EmergencyStop,
            mode_revisions: Vec::new(),
            abort_deadline,
            kill_deadline: abort_deadline.checked_add(deadlines.terminate_wait)?,
            abort_control_written: false,
            term_sent: false,
            term_delivered: false,
            kill_sent: false,
            kill_delivered: false,
        })
    }
}

impl ManagedPiChild {
    /// Advances only the deadline-driven TERM/KILL portion of cancellation.
    /// It deliberately does not wait for the direct child or touch lingering
    /// process-group cleanup: M5 first makes the direct-child wait durable,
    /// then applies the separately authorized lingering-group policy.
    fn advance_cancellation_without_reap(
        &mut self,
        now: MonotonicTick,
    ) -> Result<(), SupervisionError> {
        let Some(progress) = self.cancellation.as_ref() else {
            return Err(SupervisionError::InvalidLifecycle);
        };
        if matches!(progress.mode, CancellationMode::Quiesce) {
            return Ok(());
        }
        let send_term = !progress.term_sent && now >= progress.abort_deadline;
        if send_term {
            let outcome = self.signal_group(libc::SIGTERM)?;
            let delivered = outcome.was_delivered();
            self.deliveries.push(signal_receipt(
                outcome,
                SignalAction::Terminate,
                SignalDelivery::TermSent,
                now,
            ));
            let progress = self
                .cancellation
                .as_mut()
                .ok_or(SupervisionError::InvalidLifecycle)?;
            progress.term_sent = true;
            progress.term_delivered = delivered;
            self.lifecycle = ChildLifecycle::AwaitingTermination;
        }
        let send_kill = self.cancellation.as_ref().is_some_and(|progress| {
            progress.term_sent && !progress.kill_sent && now >= progress.kill_deadline
        });
        if send_kill {
            let outcome = self.signal_group(libc::SIGKILL)?;
            let delivered = outcome.was_delivered();
            self.deliveries.push(signal_receipt(
                outcome,
                SignalAction::Kill,
                SignalDelivery::KillSent,
                now,
            ));
            let progress = self
                .cancellation
                .as_mut()
                .ok_or(SupervisionError::InvalidLifecycle)?;
            progress.kill_sent = true;
            progress.kill_delivered = delivered;
            self.lifecycle = ChildLifecycle::AwaitingKill;
        }
        Ok(())
    }

    fn finish_inert_setup(&mut self) -> Result<(), PostSpawnSetupFailure> {
        // `QualifiedHostExecution::verify_before_spawn` has already asserted
        // the runtime's closed v1 identity.  Construction is still fallible
        // by the peer API, so it belongs after durable native registration.
        let peer = BoundaryPeer::new(
            self.request.session_identity.clone(),
            self.native_child.native_process.host_process_id(),
            self.request.spawn_nonce.clone(),
            self.request.host_execution.runtime.clone(),
        )
        .map_err(|_| PostSpawnSetupFailure::BoundaryPeer)?;
        let stdin = self
            .stdin
            .as_ref()
            .ok_or(PostSpawnSetupFailure::MissingStdinPipe)?;
        set_nonblocking_stdin(stdin).map_err(|_| PostSpawnSetupFailure::StdinNonblocking)?;
        let stdout = self
            .stdout
            .as_ref()
            .ok_or(PostSpawnSetupFailure::MissingStdoutPipe)?;
        set_nonblocking_stdout(stdout.get_ref())
            .map_err(|_| PostSpawnSetupFailure::StdoutNonblocking)?;
        self.peer = Some(peer);
        Ok(())
    }

    fn peer(&self) -> Result<&BoundaryPeer, SupervisionError> {
        self.peer.as_ref().ok_or(SupervisionError::InvalidLifecycle)
    }

    fn peer_mut(&mut self) -> Result<&mut BoundaryPeer, SupervisionError> {
        self.peer.as_mut().ok_or(SupervisionError::InvalidLifecycle)
    }

    fn peer_receipt_state(&self) -> PiPeerReceiptState {
        self.peer
            .as_ref()
            .map_or(PiPeerReceiptState::NotInitialized, |peer| {
                PiPeerReceiptState::Observed(peer.phase())
            })
    }

    fn spawned_facts(&self) -> SpawnedChildFacts {
        SpawnedChildFacts {
            child_process_id: self.request.child_process_id.clone(),
            session_identity: self.request.session_identity.clone(),
            spawn_nonce: self.request.spawn_nonce.clone(),
            host_process_id: self.native_child.native_process.host_process_id(),
            process_group_id: self.native_child.native_process.process_group_id(),
            workspace_identity: self.request.workspace.identity().clone(),
            workspace_directory: self.request.workspace.directory().clone(),
            runtime: self.request.host_execution.runtime.clone(),
            environment: self.request.environment,
        }
    }

    fn inert_facts(&self) -> InertChildFacts {
        InertChildFacts {
            child_process_id: self.request.child_process_id.clone(),
            session_identity: self.request.session_identity.clone(),
            host_process_id: self.native_child.native_process.host_process_id(),
            process_group_id: self.native_child.native_process.process_group_id(),
            workspace_identity: self.request.workspace.identity().clone(),
            workspace_directory: self.request.workspace.directory().clone(),
            runtime: self.request.host_execution.runtime.clone(),
            environment: self.request.environment,
        }
    }

    fn next_frame(
        &mut self,
        correlation_identity: CorrelationIdentity,
        command: InboundCommand,
    ) -> Result<InboundFrame, SupervisionError> {
        if self.pending_control.is_some() {
            return Err(SupervisionError::InvalidLifecycle);
        }
        let sequence = BoundarySequence::parse(self.next_inbound_sequence)?;
        self.next_inbound_sequence = self
            .next_inbound_sequence
            .checked_add(1)
            .ok_or(SupervisionError::InvalidLifecycle)?;
        Ok(InboundFrame {
            sequence,
            session_identity: self.request.session_identity.clone(),
            correlation_identity,
            command,
        })
    }

    fn stage_inbound(
        &mut self,
        frame: InboundFrame,
        command: PendingControlCommand,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, SupervisionError> {
        let line = encode_inbound_jsonl(&frame)?;
        self.peer_mut()?
            .admit_inbound_jsonl_bytes(line.as_bytes())?;
        // Peer admission is a logical fact. It stays separate from native
        // pipe evidence: a partial write/cancellation must not make physical
        // stdin bytes appear to contain a complete frame.
        self.admitted_control_capture.observe(line.as_bytes());
        self.admitted_control_capture.observe(b"\n");
        let mut bytes = line.into_bytes();
        bytes.push(b'\n');
        self.pending_control = Some(PendingControlWrite {
            bytes,
            next_byte_offset: 0,
            deadline,
            command,
        });
        self.flush_pending_control(now)
    }

    fn flush_pending_control(
        &mut self,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, SupervisionError> {
        let deadline = self
            .pending_control
            .as_ref()
            .ok_or(SupervisionError::InvalidLifecycle)?
            .deadline;
        if now >= deadline.expires_at() {
            return Err(SupervisionError::ControlWriteDeadlineExpired);
        }
        #[cfg(feature = "test-support")]
        if self.force_next_control_write_pending_for_test {
            self.force_next_control_write_pending_for_test = false;
            return Ok(ControlWriteProgress::Pending);
        }
        loop {
            let (next_byte_offset, byte_count) = {
                let pending = self
                    .pending_control
                    .as_ref()
                    .ok_or(SupervisionError::InvalidLifecycle)?;
                (pending.next_byte_offset, pending.bytes.len())
            };
            if next_byte_offset == byte_count {
                let pending = self
                    .pending_control
                    .take()
                    .ok_or(SupervisionError::InvalidLifecycle)?;
                self.physically_delivered_inbound_frame_count = self
                    .physically_delivered_inbound_frame_count
                    .checked_add(1)
                    .ok_or(SupervisionError::InvalidLifecycle)?;
                self.record_control_delivery(pending.command, now)?;
                return Ok(ControlWriteProgress::Delivered);
            }
            let write_result = {
                let pending = self
                    .pending_control
                    .as_ref()
                    .ok_or(SupervisionError::InvalidLifecycle)?;
                let stdin = self
                    .stdin
                    .as_mut()
                    .ok_or(SupervisionError::InvalidLifecycle)?;
                stdin.write(&pending.bytes[pending.next_byte_offset..])
            };
            match write_result {
                Ok(0) => return Err(SupervisionError::ControlWriteZero),
                Ok(written) => {
                    let written_slice = {
                        let pending = self
                            .pending_control
                            .as_ref()
                            .ok_or(SupervisionError::InvalidLifecycle)?;
                        pending.bytes[pending.next_byte_offset..pending.next_byte_offset + written]
                            .to_vec()
                    };
                    self.stdin_capture.observe(&written_slice);
                    let pending = self
                        .pending_control
                        .as_mut()
                        .ok_or(SupervisionError::InvalidLifecycle)?;
                    pending.next_byte_offset = pending
                        .next_byte_offset
                        .checked_add(written)
                        .ok_or(SupervisionError::InvalidLifecycle)?;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(ControlWriteProgress::Pending);
                }
                Err(error) => {
                    self.stdin.take();
                    return Err(error.into());
                }
            }
        }
    }

    fn record_control_delivery(
        &mut self,
        command: PendingControlCommand,
        now: MonotonicTick,
    ) -> Result<(), SupervisionError> {
        if command != PendingControlCommand::Abort {
            return Ok(());
        }
        let liveness = self.group_liveness()?;
        let cancellation = self
            .cancellation
            .as_mut()
            .ok_or(SupervisionError::InvalidLifecycle)?;
        cancellation.abort_control_written = true;
        self.deliveries.push(SignalReceipt {
            action: SignalAction::AbortControl,
            delivery: SignalDelivery::AbortControlWritten,
            observed_at: now,
            group_liveness_after_attempt: liveness,
        });
        Ok(())
    }

    fn read_one_outbound(&mut self) -> Result<OutboundRead, SupervisionError> {
        let stdout = self
            .stdout
            .as_mut()
            .ok_or(SupervisionError::InvalidLifecycle)?;
        let record = read_bounded_record(
            stdout,
            &mut self.stdout_capture,
            &mut self.stdout_partial_record,
        )?;
        let record = match record {
            StreamRead::NotReady => return Ok(OutboundRead::NotReady),
            StreamRead::Eof => {
                let _ = self.peer_mut()?.observe_stdout_eof();
                return Err(SupervisionError::OutputLost);
            }
            StreamRead::Frame(record) => record,
        };
        // `BoundaryPeer` seals and validates the exact bytes *before* the
        // paired decode below. In particular, invalid UTF-8 must still be
        // sealed and terminally fenced by the peer rather than escaping
        // through a lossy or pre-validation parser path.
        let observation = self.peer_mut()?.observe_outbound_jsonl_bytes(&record);
        // This second strict decode exposes the same already-validated frame
        // to the daemon so M6 can attest exact sequence, correlation, full
        // cumulative totals, and raw binary64 cost. It never replaces the
        // peer's raw sealing/validation authority above.
        let record_text = std::str::from_utf8(&record)
            .map_err(|_| PeerError::Protocol(society_pi::ProtocolError::InvalidUtf8))?;
        let frame = decode_outbound_jsonl(record_text)?;
        let peer_became_fatal = self.peer()?.phase() == PeerPhase::Fatal;
        let (observation, validation) = match observation {
            Ok(observation) => (observation, PeerFrameValidation::Accepted),
            Err(error) => (None, PeerFrameValidation::Rejected(error)),
        };
        Ok(OutboundRead::Observation(Box::new(
            SealedDecodedPeerFrame {
                frame,
                observation,
                validation,
                peer_became_fatal,
            },
        )))
    }

    fn start_automatic_boundary_containment(
        &mut self,
        now: MonotonicTick,
    ) -> Result<(), SupervisionError> {
        if self.completed_receipt.is_some() || self.lifecycle == ChildLifecycle::Reaped {
            return Ok(());
        }
        self.stdin.take();
        self.pending_control.take();
        if self.cancellation.as_ref().is_some_and(|progress| {
            progress.origin == CancellationOrigin::AutomaticBoundaryContainment
        }) {
            self.lifecycle = ChildLifecycle::AwaitingCooperativeAbort;
            return Ok(());
        }
        // A corrupt host is not allowed to inherit a harmless Quiesce or a
        // longer graceful deadline. Preserve the prior mode only as a typed
        // transition fact; the live physics becomes the fixed emergency path.
        let prior_mode = self.cancellation.as_ref().map(|progress| progress.mode);
        let mut automatic = CancellationProgress::automatic_boundary_containment(now)?;
        if let Some(prior_mode) = prior_mode.filter(|mode| *mode != automatic.mode) {
            automatic.mode_revisions.push(CancellationModeRevision {
                from: prior_mode,
                to: automatic.mode,
                observed_at: now,
            });
        }
        self.cancellation = Some(automatic);
        self.lifecycle = ChildLifecycle::AwaitingCooperativeAbort;
        Ok(())
    }

    fn discard_pending_control_before_cancellation(&mut self) {
        if self.pending_control.is_some() {
            self.pending_control.take();
            self.stdin.take();
        }
    }

    fn owns_expected_process_group(&self) -> Result<bool, SupervisionError> {
        // SAFETY: `getpgid` only observes the direct child PID retained by
        // `std::process::Child`; no signal is delivered. The PID has not been
        // reaped, so it cannot be reused while this handle remains live.
        self.native_child.native_process.owns_expected_group()
    }

    fn group_liveness(&self) -> Result<ProcessGroupLiveness, SupervisionError> {
        // SAFETY: negative PGID targets only the process group this object
        // created. Signal zero probes liveness without delivering a signal.
        self.native_child.native_process.liveness()
    }

    fn signal_group(&self, signal: libc::c_int) -> Result<SignalGroupOutcome, SupervisionError> {
        Ok(match self.native_child.native_process.signal(signal)? {
            NativeSignalGroupOutcome::AbsentBeforeSignal => SignalGroupOutcome::AbsentBeforeSignal,
            NativeSignalGroupOutcome::InaccessibleBeforeSignal => {
                SignalGroupOutcome::InaccessibleBeforeSignal
            }
            NativeSignalGroupOutcome::AbsentDuringSignal => SignalGroupOutcome::AbsentDuringSignal,
            NativeSignalGroupOutcome::InaccessibleDuringSignal => {
                SignalGroupOutcome::InaccessibleDuringSignal
            }
            NativeSignalGroupOutcome::Delivered {
                group_liveness_after_delivery,
            } => SignalGroupOutcome::Delivered {
                group_liveness_after_delivery,
            },
        })
    }

    fn poll_direct_child_reap(&mut self) -> Result<Option<DirectChildReapFacts>, SupervisionError> {
        if let Some(pending) = &self.pending_direct_reap {
            return Ok(Some(DirectChildReapFacts {
                child_process_id: self.request.child_process_id.clone(),
                host_process_id: self.native_child.native_process.host_process_id(),
                process_group_id: self.native_child.native_process.process_group_id(),
                status: reap_status(pending.status),
                group_liveness_after_direct_child_reap: pending
                    .group_liveness_after_direct_child_reap,
                prior_signal_receipts: pending.prior_signal_receipts.clone(),
            }));
        }
        if self.completed_receipt.is_some() {
            return Err(SupervisionError::InvalidLifecycle);
        }
        let Some(status) = self.native_child.poll_direct_wait()? else {
            return Ok(None);
        };
        let group_liveness_after_direct_child_reap = self.group_liveness()?;
        self.pending_direct_reap = Some(PendingDirectChildReap {
            status,
            group_liveness_after_direct_child_reap,
            prior_signal_receipts: self.deliveries.clone(),
        });
        Ok(Some(DirectChildReapFacts {
            child_process_id: self.request.child_process_id.clone(),
            host_process_id: self.native_child.native_process.host_process_id(),
            process_group_id: self.native_child.native_process.process_group_id(),
            status: reap_status(status),
            group_liveness_after_direct_child_reap,
            prior_signal_receipts: self.deliveries.clone(),
        }))
    }

    fn issue_lingering_group_cleanup(
        &mut self,
        now: MonotonicTick,
    ) -> Result<Option<SignalReceipt>, SupervisionError> {
        let pending = self
            .pending_direct_reap
            .as_ref()
            .ok_or(SupervisionError::InvalidLifecycle)?;
        if pending.group_liveness_after_direct_child_reap == ProcessGroupLiveness::Absent {
            return Ok(None);
        }
        // A duplicate invocation would create a second semantically identical
        // cleanup attempt. The caller persists the returned receipt before it
        // polls/finishes, so exactly one action is permitted for this wait.
        if self
            .deliveries
            .iter()
            .any(|receipt| receipt.action == SignalAction::LingeringGroupKill)
        {
            return Err(SupervisionError::InvalidLifecycle);
        }
        let outcome = self.signal_group(libc::SIGKILL)?;
        let receipt = signal_receipt(
            outcome,
            SignalAction::LingeringGroupKill,
            SignalDelivery::LingeringGroupKillSent,
            now,
        );
        self.deliveries.push(receipt.clone());
        Ok(Some(receipt))
    }

    fn complete_deferred_reap(&mut self) -> Result<SupervisionReceipt, SupervisionError> {
        if let Some(receipt) = self.completed_receipt.clone() {
            return Ok(receipt);
        }
        let pending = self
            .pending_direct_reap
            .take()
            .ok_or(SupervisionError::InvalidLifecycle)?;
        let group_liveness_after_reap = self.group_liveness()?;
        if group_liveness_after_reap != ProcessGroupLiveness::Absent {
            self.pending_direct_reap = Some(pending);
            return Err(SupervisionError::ContainmentAwaitingDrive);
        }
        self.complete_reap_after_group_absent(
            pending.status,
            pending.group_liveness_after_direct_child_reap,
            group_liveness_after_reap,
        )
    }

    fn try_reap(
        &mut self,
        now: MonotonicTick,
    ) -> Result<Option<SupervisionReceipt>, SupervisionError> {
        if let Some(receipt) = self.completed_receipt.clone() {
            return Ok(Some(receipt));
        }
        let Some(status) = self.native_child.poll_direct_wait()? else {
            return Ok(None);
        };
        Ok(Some(self.complete_reap(status, now)?))
    }

    fn wait_and_reap_at(
        &mut self,
        now: MonotonicTick,
    ) -> Result<SupervisionReceipt, SupervisionError> {
        if let Some(receipt) = self.completed_receipt.clone() {
            return Ok(receipt);
        }
        if self.cancellation.as_ref().is_some_and(|progress| {
            progress.origin == CancellationOrigin::AutomaticBoundaryContainment
        }) {
            if let Some(receipt) = self.try_reap(now)? {
                return Ok(receipt);
            }
            return Err(SupervisionError::ContainmentAwaitingDrive);
        }
        let status = self.native_child.wait_direct()?;
        self.complete_reap(status, now)
    }

    fn complete_reap(
        &mut self,
        status: ExitStatus,
        now: MonotonicTick,
    ) -> Result<SupervisionReceipt, SupervisionError> {
        if let Some(receipt) = self.completed_receipt.clone() {
            return Ok(receipt);
        }
        self.stdin.take();
        // Do this before draining either pipe. A direct child can exit while
        // a descendant still owns stdout/stderr; draining first would permit
        // that descendant to block the single daemon supervisor forever.
        let group_liveness_before_cleanup = self.group_liveness()?;
        if group_liveness_before_cleanup != ProcessGroupLiveness::Absent {
            let outcome = self.signal_group(libc::SIGKILL)?;
            self.deliveries.push(signal_receipt(
                outcome,
                SignalAction::LingeringGroupKill,
                SignalDelivery::LingeringGroupKillSent,
                now,
            ));
        }
        let group_liveness_after_reap = self.group_liveness()?;
        self.complete_reap_after_group_absent(
            status,
            group_liveness_before_cleanup,
            group_liveness_after_reap,
        )
    }

    fn complete_reap_after_group_absent(
        &mut self,
        status: ExitStatus,
        group_liveness_before_cleanup: ProcessGroupLiveness,
        group_liveness_after_reap: ProcessGroupLiveness,
    ) -> Result<SupervisionReceipt, SupervisionError> {
        if let Some(receipt) = self.completed_receipt.clone() {
            return Ok(receipt);
        }
        let stdout_contained = self.drain_stdout_after_exit()?;
        // Closing stdin after a stale pre-Create recheck is the one expected
        // inert EOF: no Pi session existed, so it cannot produce Disposed.
        let (eof_contained, peer_fatal) = if let Some(peer) = self.peer.as_mut() {
            let expected_inert_control_eof = peer.phase() == PeerPhase::Inert
                && self.lifecycle == ChildLifecycle::Quiescing
                && self.cancellation.is_none();
            (
                !expected_inert_control_eof && peer.observe_stdout_eof().is_err(),
                peer.phase() == PeerPhase::Fatal,
            )
        } else {
            // A post-spawn setup failure has no adapter-peer evidence. Its
            // automatic containment is recorded separately below.
            (false, false)
        };
        let stderr = self.take_stderr_snapshot_after_reap()?;
        let inexact_transient_count = self.stdin_capture.count_overflowed
            || self.stdout_capture.count_overflowed
            || stderr.count_overflowed;
        let forced_cancellation = self
            .cancellation
            .as_ref()
            .is_some_and(|cancellation| cancellation.term_delivered || cancellation.kill_delivered);
        let automatic_containment = self.cancellation.as_ref().is_some_and(|cancellation| {
            cancellation.origin == CancellationOrigin::AutomaticBoundaryContainment
        });
        let terminal_disposition = if stdout_contained
            || automatic_containment
            || inexact_transient_count
            || group_liveness_after_reap != ProcessGroupLiveness::Absent
            || (eof_contained && !forced_cancellation)
            || (peer_fatal && !forced_cancellation)
        {
            ChildTerminalDisposition::ContainmentFailed
        } else {
            terminal_disposition(
                self.cancellation.as_ref(),
                &status,
                group_liveness_after_reap,
            )
        };
        let reap = ReapReceipt {
            child_process_id: self.request.child_process_id.clone(),
            host_process_id: self.native_child.native_process.host_process_id(),
            process_group_id: self.native_child.native_process.process_group_id(),
            status: reap_status(status),
            group_liveness_before_cleanup,
            group_liveness_after_reap,
        };
        let receipt = SupervisionReceipt {
            child_process_id: self.request.child_process_id.clone(),
            session_identity: self.request.session_identity.clone(),
            workspace_identity: self.request.workspace.identity().clone(),
            workspace_directory: self.request.workspace.directory().clone(),
            terminal_disposition,
            reap: Some(reap),
            cancellation_deliveries: self.deliveries.clone(),
            cancellation_origin: self
                .cancellation
                .as_ref()
                .map(|cancellation| cancellation.origin),
            cancellation_mode_revisions: self
                .cancellation
                .as_ref()
                .map_or_else(Vec::new, |cancellation| cancellation.mode_revisions.clone()),
            canonical_session_file: self.peer.as_ref().and_then(|peer| {
                peer.configuration()
                    .map(|configuration| configuration.session_file.clone())
            }),
            transient_evidence: TransientExecutionEvidence {
                admitted_control: self.admitted_control_capture.transient_capture(),
                stdin: self.stdin_capture.transient_capture(),
                stdout: self.stdout_capture.transient_capture(),
                stderr: stderr.transient_capture(),
                logically_admitted_inbound_frame_count: self
                    .peer
                    .as_ref()
                    .map_or(0, |peer| peer.inbound_seals().len() as u64),
                physically_delivered_inbound_frame_count: self
                    .physically_delivered_inbound_frame_count,
                outbound_frame_count: self
                    .peer
                    .as_ref()
                    .map_or(0, |peer| peer.outbound_seals().len() as u64),
            },
            peer_state: self.peer_receipt_state(),
        };
        self.lifecycle = if terminal_disposition == ChildTerminalDisposition::ContainmentFailed {
            ChildLifecycle::Contained
        } else {
            ChildLifecycle::Reaped
        };
        self.completed_receipt = Some(receipt.clone());
        Ok(receipt)
    }

    /// Returns whether the stream was protocol-contained. It never discards a
    /// malformed raw record merely because the direct child has already
    /// exited: the raw capture and peer phase remain evidence for the kernel.
    fn drain_stdout_after_exit(&mut self) -> Result<bool, SupervisionError> {
        let stdout = self
            .stdout
            .as_mut()
            .ok_or(SupervisionError::InvalidLifecycle)?;
        // A peerless child did not complete the normal nonblocking pipe
        // setup. Convert the owned descriptor before draining so an escaped
        // descendant cannot block this post-containment cleanup.
        if self.peer.is_none() {
            set_nonblocking_stdout(stdout.get_ref())?;
        }
        let mut contained = false;
        loop {
            match read_bounded_record(
                stdout,
                &mut self.stdout_capture,
                &mut self.stdout_partial_record,
            ) {
                Ok(StreamRead::Frame(record)) => {
                    if self
                        .peer
                        .as_mut()
                        .is_none_or(|peer| peer.observe_outbound_jsonl_bytes(&record).is_err())
                    {
                        contained = true;
                    }
                }
                Ok(StreamRead::Eof) => break,
                Ok(StreamRead::NotReady) => {
                    // The direct child is reaped, so a foreign escaped
                    // descendant must still own this pipe. Record the bounded
                    // prefix and return rather than blocking its supervisor.
                    self.stdout_capture.retention = TransientRetention::PrefixBounded;
                    contained = true;
                    break;
                }
                Err(SupervisionError::OutboundFrameTooLarge) => {
                    self.lifecycle = ChildLifecycle::Contained;
                    contained = true;
                    break;
                }
                Err(SupervisionError::UnterminatedOutboundRecord) => {
                    self.lifecycle = ChildLifecycle::Contained;
                    contained = true;
                    break;
                }
                Err(error) => return Err(error),
            }
        }
        Ok(contained)
    }

    fn take_stderr_snapshot_after_reap(&mut self) -> Result<StreamCapture, SupervisionError> {
        let task = self
            .stderr_capture
            .take()
            .ok_or(SupervisionError::StderrCaptureFailed)?;
        if task.worker.is_finished() {
            task.worker
                .join()
                .map_err(|_| SupervisionError::StderrCaptureFailed)??;
            return task
                .snapshot
                .lock()
                .map_err(|_| SupervisionError::StderrCaptureFailed)
                .map(|capture| capture.clone());
        }
        // Drop detaches the reader. It owns only an OS pipe and an Arc snapshot;
        // an escaped process may keep it live, but it cannot block reaping or
        // admit any successor from this supervisor.
        let snapshot = task
            .snapshot
            .lock()
            .map_err(|_| SupervisionError::StderrCaptureFailed)
            .map(|capture| capture.clone())?;
        drop(task.worker);
        Ok(snapshot.with_prefix_bounded())
    }
}

impl Drop for ManagedPiChild {
    fn drop(&mut self) {
        if self.completed_receipt.is_some()
            || matches!(
                self.lifecycle,
                ChildLifecycle::Reaped | ChildLifecycle::Contained
            )
        {
            return;
        }
        // There is deliberately no "detached" supervised child state. If a
        // daemon control-path error drops this owner, close admissions and
        // make the same conservative last-resort group kill before reaping
        // the direct child. Crash recovery is still required after a process
        // crash; Rust cannot restore a lost parent/wait status.
        self.stdin.take();
        // SAFETY: while `Child` remains unreaped its PID cannot be reused;
        // this group was created by `pre_exec(setpgid(0, 0))` for this child.
        let _ = self.native_child.native_process.signal(libc::SIGKILL);
        let _ = self.native_child.wait_direct();
    }
}

#[derive(Clone, Default)]
struct StreamCapture {
    observed_byte_count: u64,
    count_overflowed: bool,
    hasher: Hasher,
    retained_bytes: Vec<u8>,
    retention: TransientRetention,
}

impl StreamCapture {
    fn observe(&mut self, bytes: &[u8]) {
        match self.observed_byte_count.checked_add(bytes.len() as u64) {
            Some(count) if !self.count_overflowed => self.observed_byte_count = count,
            _ => {
                self.count_overflowed = true;
                self.retention = TransientRetention::CountOverflow;
            }
        }
        self.hasher.update(bytes);
        let remaining = MAX_TRANSIENT_STREAM_BYTES.saturating_sub(self.retained_bytes.len());
        let retained = bytes.len().min(remaining);
        self.retained_bytes.extend_from_slice(&bytes[..retained]);
        if retained != bytes.len() {
            self.retention = TransientRetention::PrefixBounded;
        }
    }

    fn transient_capture(&self) -> TransientStreamCapture {
        TransientStreamCapture {
            observed_byte_count: if self.count_overflowed {
                TransientByteCount::Overflowed
            } else {
                TransientByteCount::Exact(self.observed_byte_count)
            },
            blake3: digest_to_type(self.hasher.clone().finalize().as_bytes()),
            retention: self.retention,
            retained_bytes: self.retained_bytes.clone(),
        }
    }

    fn with_prefix_bounded(mut self) -> Self {
        if !self.count_overflowed {
            self.retention = TransientRetention::PrefixBounded;
        }
        self
    }
}

fn spawn_stderr_capture(mut stderr: ChildStderr) -> StderrCaptureTask {
    let snapshot = Arc::new(Mutex::new(StreamCapture::default()));
    let shared_snapshot = Arc::clone(&snapshot);
    let worker = thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            let count = stderr.read(&mut buffer)?;
            if count == 0 {
                return Ok(());
            }
            let mut capture = shared_snapshot
                .lock()
                .map_err(|_| io::Error::other("stderr capture mutex poisoned"))?;
            capture.observe(&buffer[..count]);
        }
    });
    StderrCaptureTask { snapshot, worker }
}

enum StreamRead {
    Frame(Vec<u8>),
    NotReady,
    Eof,
}

fn read_bounded_record(
    reader: &mut BufReader<ChildStdout>,
    capture: &mut StreamCapture,
    partial: &mut Vec<u8>,
) -> Result<StreamRead, SupervisionError> {
    loop {
        let available = match reader.fill_buf() {
            Ok(available) => available,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                return if partial.is_empty() {
                    Ok(StreamRead::NotReady)
                } else {
                    // No complete record exists yet. Preserve what was read
                    // in the BufReader until the next nonblocking poll.
                    Ok(StreamRead::NotReady)
                };
            }
            Err(error) => return Err(error.into()),
        };
        if available.is_empty() {
            return if partial.is_empty() {
                Ok(StreamRead::Eof)
            } else {
                Err(SupervisionError::UnterminatedOutboundRecord)
            };
        }
        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        let bytes = &available[..take];
        capture.observe(bytes);
        let newline = bytes.last() == Some(&b'\n');
        if partial.len().saturating_add(bytes.len()) > MAX_JSONL_FRAME_BYTES + 1 {
            reader.consume(take);
            partial.clear();
            return Err(SupervisionError::OutboundFrameTooLarge);
        }
        partial.extend_from_slice(bytes);
        reader.consume(take);
        if newline {
            partial.pop();
            return Ok(StreamRead::Frame(std::mem::take(partial)));
        }
    }
}

fn set_nonblocking_stdout(stdout: &ChildStdout) -> Result<(), SupervisionError> {
    set_nonblocking_file_descriptor(stdout.as_raw_fd())
}

fn set_nonblocking_stdin(stdin: &ChildStdin) -> Result<(), SupervisionError> {
    set_nonblocking_file_descriptor(stdin.as_raw_fd())
}

fn set_nonblocking_file_descriptor(file_descriptor: libc::c_int) -> Result<(), SupervisionError> {
    // SAFETY: fcntl reads/modifies only the status flags of this owned pipe
    // descriptor. The descriptor remains owned by its Child stdio handle.
    let flags = unsafe { libc::fcntl(file_descriptor, libc::F_GETFL) };
    if flags < 0 {
        return Err(SupervisionError::Io(io::Error::last_os_error()));
    }
    // SAFETY: as above; O_NONBLOCK is the only flag added.
    if unsafe { libc::fcntl(file_descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } != 0 {
        return Err(SupervisionError::Io(io::Error::last_os_error()));
    }
    Ok(())
}

fn validate_spawn_request(request: &PiSpawnRequest) -> Result<(), SupervisionError> {
    if request.create_session.cwd != *request.workspace.directory()
        || !request
            .create_session
            .agent_directory
            .is_strict_descendant_of(request.workspace.directory())
        || !request
            .create_session
            .session_directory
            .is_strict_descendant_of(request.workspace.directory())
        || !request
            .create_session
            .auth_path
            .is_strict_descendant_of(&request.create_session.agent_directory)
        || !request
            .create_session
            .models_path
            .is_strict_descendant_of(&request.create_session.agent_directory)
        || request.create_session.model.provider != Provider::OpenRouter
        || request.create_session.model.model_id
            != request
                .create_session
                .model_catalog
                .effective_model
                .model_id
        || !model_thinking_level_is_admitted(
            request.create_session.model.model_id,
            request.create_session.model.thinking_level,
        )
        || digest_bytes(request.create_session.system_prompt.as_bytes())
            != request.create_session.system_prompt_digest
    {
        return Err(SupervisionError::InvalidSpawnRequest);
    }
    request
        .create_session
        .model_catalog
        .assert_pinned()
        .map_err(SupervisionError::Protocol)?;
    request
        .create_session
        .settings
        .assert_pinned()
        .map_err(SupervisionError::Protocol)?;
    // The host receives ordinary native paths, so lexical containment alone
    // is insufficient: a symlink could redirect auth/models outside the owned
    // workspace between construction and Pi's ResourceLoader. Resolve each
    // existing target immediately before spawn and require the same owned
    // directory relation the protocol declares.
    validate_existing_directory(request.workspace.directory(), request.workspace.directory())?;
    validate_existing_directory(
        &request.create_session.agent_directory,
        request.workspace.directory(),
    )?;
    validate_existing_directory(
        &request.create_session.session_directory,
        request.workspace.directory(),
    )?;
    validate_existing_regular_file(
        &request.create_session.auth_path,
        &request.create_session.agent_directory,
    )?;
    validate_existing_regular_file(
        &request.create_session.models_path,
        &request.create_session.agent_directory,
    )?;
    if digest_file(request.create_session.models_path.as_path())?
        != request.create_session.model_catalog.catalog_blake3
    {
        return Err(SupervisionError::InvalidSpawnRequest);
    }
    Ok(())
}

fn validate_existing_directory(
    candidate: &AbsolutePath,
    owned_base: &AbsolutePath,
) -> Result<(), SupervisionError> {
    let candidate = fs::canonicalize(candidate.as_path())?;
    let owned_base = fs::canonicalize(owned_base.as_path())?;
    let metadata = fs::metadata(&candidate)?;
    let candidate = absolute_path_from_path(&candidate)?;
    let owned_base = absolute_path_from_path(&owned_base)?;
    if !metadata.is_dir()
        || (candidate != owned_base && !candidate.is_strict_descendant_of(&owned_base))
    {
        return Err(SupervisionError::InvalidSpawnRequest);
    }
    Ok(())
}

fn validate_existing_regular_file(
    candidate: &AbsolutePath,
    owned_base: &AbsolutePath,
) -> Result<(), SupervisionError> {
    let candidate = fs::canonicalize(candidate.as_path())?;
    let owned_base = fs::canonicalize(owned_base.as_path())?;
    let metadata = fs::metadata(&candidate)?;
    let candidate = absolute_path_from_path(&candidate)?;
    let owned_base = absolute_path_from_path(&owned_base)?;
    if !metadata.is_file() || !candidate.is_strict_descendant_of(&owned_base) {
        return Err(SupervisionError::InvalidSpawnRequest);
    }
    Ok(())
}

fn abort_reason_for(reason: CancellationReason) -> AbortReason {
    match reason {
        CancellationReason::OperatorStop | CancellationReason::WallBudgetExpired => {
            AbortReason::GracefulCancellation
        }
        CancellationReason::BudgetGuardrail => AbortReason::BudgetGuardrail,
        CancellationReason::ProtocolContainment => AbortReason::EmergencyStop,
        CancellationReason::DaemonRecovery => AbortReason::DaemonRecovery,
    }
}

fn upgrade_or_replay_cancellation(
    existing: &mut CancellationProgress,
    incoming: CancellationRequest,
    now: MonotonicTick,
) -> Result<(), SupervisionError> {
    let Some(previous) = existing.explicit_request.as_ref() else {
        return Err(SupervisionError::InvalidLifecycle);
    };
    if previous == &incoming {
        return Ok(());
    }
    if previous.cancellation_request_id != incoming.cancellation_request_id
        || existing.mode != CancellationMode::GracefulCancel
        || incoming.mode != CancellationMode::EmergencyStop
    {
        return Err(SupervisionError::InvalidLifecycle);
    }
    let deadlines = CancellationDeadlines::for_mode(CancellationMode::EmergencyStop)
        .ok_or(SupervisionError::InvalidLifecycle)?;
    let emergency_abort_deadline = now.checked_add(deadlines.cooperative_abort_wait)?;
    let emergency_kill_deadline = emergency_abort_deadline.checked_add(deadlines.terminate_wait)?;
    existing.mode_revisions.push(CancellationModeRevision {
        from: existing.mode,
        to: CancellationMode::EmergencyStop,
        observed_at: now,
    });
    existing.mode = CancellationMode::EmergencyStop;
    existing.explicit_request = Some(incoming);
    if !existing.term_sent {
        existing.abort_deadline = existing.abort_deadline.min(emergency_abort_deadline);
    }
    if !existing.kill_sent {
        existing.kill_deadline = existing.kill_deadline.min(emergency_kill_deadline);
    }
    Ok(())
}

fn terminal_disposition(
    cancellation: Option<&CancellationProgress>,
    status: &ExitStatus,
    liveness: ProcessGroupLiveness,
) -> ChildTerminalDisposition {
    if liveness != ProcessGroupLiveness::Absent {
        return ChildTerminalDisposition::ContainmentFailed;
    }
    let Some(cancellation) = cancellation else {
        return ChildTerminalDisposition::NotRunning;
    };
    if cancellation.kill_delivered {
        return ChildTerminalDisposition::Killed;
    }
    if cancellation.term_delivered {
        return ChildTerminalDisposition::Terminated;
    }
    if cancellation.abort_control_written && status.success() {
        ChildTerminalDisposition::CooperativelyAborted
    } else {
        ChildTerminalDisposition::CompletedBeforeDelivery
    }
}

fn signal_receipt(
    outcome: SignalGroupOutcome,
    action: SignalAction,
    delivered: SignalDelivery,
    observed_at: MonotonicTick,
) -> SignalReceipt {
    let (delivery, group_liveness_after_attempt) = match outcome {
        SignalGroupOutcome::AbsentBeforeSignal => (
            SignalDelivery::AbsentBeforeSignal,
            ProcessGroupLiveness::Absent,
        ),
        SignalGroupOutcome::InaccessibleBeforeSignal
        | SignalGroupOutcome::InaccessibleDuringSignal => (
            SignalDelivery::GroupInaccessible,
            ProcessGroupLiveness::Inaccessible,
        ),
        SignalGroupOutcome::AbsentDuringSignal => (
            SignalDelivery::AbsentDuringSignal,
            ProcessGroupLiveness::Absent,
        ),
        SignalGroupOutcome::Delivered {
            group_liveness_after_delivery,
        } => (delivered, group_liveness_after_delivery),
    };
    SignalReceipt {
        action,
        delivery,
        observed_at,
        group_liveness_after_attempt,
    }
}

fn reap_status(status: ExitStatus) -> ReapStatus {
    use std::os::unix::process::ExitStatusExt;

    match (status.code(), status.signal()) {
        (Some(code), _) => ReapStatus::Exited { code },
        (None, Some(signal)) => ReapStatus::Signaled { signal },
        (None, None) => ReapStatus::Unknown,
    }
}

fn digest_file(path: &Path) -> Result<Blake3Digest, SupervisionError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok(digest_to_type(hasher.finalize().as_bytes()));
        }
        hasher.update(&buffer[..count]);
    }
}

fn digest_bytes(bytes: &[u8]) -> Blake3Digest {
    digest_to_type(blake3::hash(bytes).as_bytes())
}

fn digest_to_type(digest: impl AsRef<[u8]>) -> Blake3Digest {
    let mut output = String::with_capacity(64);
    for byte in digest.as_ref() {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    Blake3Digest::parse(output).expect("BLAKE3 formatter produces lowercase hex")
}

fn absolute_path_from_path(path: &Path) -> Result<AbsolutePath, SupervisionError> {
    let value = path
        .to_str()
        .ok_or(SupervisionError::InvalidSpawnRequest)?
        .to_owned();
    AbsolutePath::parse(value).map_err(SupervisionError::Protocol)
}

fn is_domain_identifier(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || !bytes[0].is_ascii_alphanumeric()
        || !bytes.last().is_some_and(u8::is_ascii_alphanumeric)
    {
        return false;
    }
    bytes
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn cancellation_deadlines_are_fixed_by_closed_mode() {
        let graceful = CancellationDeadlines::for_mode(CancellationMode::GracefulCancel).unwrap();
        assert_eq!(graceful.cooperative_abort_wait.value(), 5_000);
        assert_eq!(graceful.terminate_wait.value(), 5_000);
        let emergency = CancellationDeadlines::for_mode(CancellationMode::EmergencyStop).unwrap();
        assert_eq!(emergency.cooperative_abort_wait.value(), 1_000);
        assert_eq!(emergency.terminate_wait.value(), 2_000);
        assert_eq!(
            CancellationDeadlines::for_mode(CancellationMode::Quiesce),
            None
        );
    }

    #[test]
    fn opaque_identifiers_are_not_generic_empty_or_path_strings() {
        assert!(SupervisedChildId::parse("child-001").is_ok());
        assert!(SupervisedChildId::parse("../child").is_err());
        assert!(NativeWorkspaceId::parse("workspace-001").is_ok());
        assert!(CancellationRequestId::parse("cancel-001").is_ok());
    }

    #[test]
    fn transient_count_overflow_is_never_saturated_into_exact_evidence() {
        let mut capture = StreamCapture {
            observed_byte_count: u64::MAX,
            ..StreamCapture::default()
        };
        capture.observe(b"x");
        let transient = capture.transient_capture();
        assert_eq!(
            transient.observed_byte_count,
            TransientByteCount::Overflowed
        );
        assert_eq!(transient.retention, TransientRetention::CountOverflow);
    }

    #[test]
    fn signal_receipts_distinguish_absence_from_a_delivered_signal_that_exited_fast() {
        let delivered = signal_receipt(
            SignalGroupOutcome::Delivered {
                group_liveness_after_delivery: ProcessGroupLiveness::Absent,
            },
            SignalAction::Terminate,
            SignalDelivery::TermSent,
            MonotonicTick::ZERO,
        );
        assert_eq!(delivered.delivery, SignalDelivery::TermSent);
        assert_eq!(
            delivered.group_liveness_after_attempt,
            ProcessGroupLiveness::Absent
        );

        let absent = signal_receipt(
            SignalGroupOutcome::AbsentBeforeSignal,
            SignalAction::Terminate,
            SignalDelivery::TermSent,
            MonotonicTick::ZERO,
        );
        assert_eq!(absent.delivery, SignalDelivery::AbsentBeforeSignal);

        let inaccessible = signal_receipt(
            SignalGroupOutcome::InaccessibleDuringSignal,
            SignalAction::Kill,
            SignalDelivery::KillSent,
            MonotonicTick::ZERO,
        );
        assert_eq!(inaccessible.delivery, SignalDelivery::GroupInaccessible);
    }
}
