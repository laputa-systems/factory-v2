//! Daemon-private bridge from durable Pi child/turn receipts to native physics.
//!
//! `PiSupervisor` owns only a live process group and transient pipe receipts.
//! This module owns neither a local wire command nor a new authority: it
//! translates one already-admitted Office child and Prompt through the
//! kernel's closed receipt chains, one committed transition at a time. In
//! particular it
//! never keeps a PostgreSQL transaction open while allocating a workspace,
//! spawning, reading or writing a pipe, signalling, waiting, or sealing
//! bytes.

use std::{
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
};

use society_content::ContentSealLimit;
use society_kernel::{
    ActorAttemptId, AdmissionGeneration, Blake3Digest as KernelDigest, BudgetReservationId,
    CanonicalPiSessionTranscriptPath, CanonicalWorkspacePath, Capability, ChildStreamKind,
    ChildStreamSealCompleteness, CommandBody, CommandDisposition, CommandId, CommandRequest,
    ContentObjectId, DirectChildWaitStatus, EventBody, EventId, ExecutionProfileId,
    ExpectedGeneration, KernelStore, NativeChildId, NativeChildPid, NativeChildSpawnAdmissionId,
    NativeWorkspaceId as KernelWorkspaceId, OfficeTurnId,
    OwnedProcessGroupId as KernelProcessGroupId, PiBoundarySessionIdentity, PiChildOwner,
    PiCorrelationIdentity, PiCumulativeUsage, PiOfficeSessionFirstUserPromptReceipt,
    PiOfficeSessionTranscriptReceipt, PiOfficeTurnAssistantOutcome, PiOfficeTurnDisposition,
    PiOfficeTurnTerminalEvidence, PiOfficeTurnTranscriptDisposition, PiOfficeTurnUsageFailure,
    PiOfficeTurnUsageUnavailableReason, PiOfficeTurnUsageUnknownReason, PiProtocolSequence,
    PiTaskAttemptAssistantOutcome, PiTaskAttemptDisposition, PiTaskAttemptTerminalEvidence,
    PiTaskAttemptTranscriptDisposition, PiTaskAttemptUsageFailure,
    PiTaskAttemptUsageUnavailableReason, PiTaskAttemptUsageUnknownReason, PiTokenCount,
    PrincipalId, ProcessExitCode, ProcessGroupLiveness as KernelLiveness, ProcessSignalNumber,
    ProviderCostBinary64, RootAuthorityOfficeSessionId, SpawnNonce as KernelSpawnNonce,
    SupervisedChildIdentity, SupervisorEpochId, SupervisorEpochIdentity,
};
use society_pi::{
    AbsolutePath, AssistantStopReason, BoundarySequence, CommandName, CommandResult,
    CorrelationIdentity, FinalAssistantOutcome, FirstUserPromptReceipt, InboundCommand,
    InboundFrame, OutboundEvent, ProjectedAgentEvent, PromptPayload, PromptPurpose,
    SessionIdentity, SessionKind, SettledClassification, TranscriptFlushReceiptV1, TurnDisposition,
    UsageObservation, UsageUnavailableReason,
};
use thiserror::Error;

use crate::{
    content::{
        ContentObjectRegistration, ContentSealOperationId, ContentSealingAuthority,
        ContentSealingError,
    },
    supervision::{
        ControlWriteDeadline, ControlWriteProgress, HandshakeDeadline, InertChildFacts,
        MonotonicTick, PeerFrameValidation, PiSpawnRequest, PiSupervisor, PostSpawnSetupFailure,
        PreCreateAdmissionGate, ReapStatus, SealedDecodedPeerFrame, SignalAction, SignalDelivery,
        SupervisedChildId, SupervisionError, SupervisionReceipt, TransientByteCount,
        TransientRetention, TransientStreamCapture,
    },
};

const COMMAND_PREFIX: &str = "pi-execution-v1/";
const OFFICE_TURN_COMMAND_PREFIX: &str = "pi-office-turn-v1/";
const OFFICE_SESSION_DISPOSE_COMMAND_PREFIX: &str = "pi-office-session-dispose-v1/";
const TASK_ATTEMPT_PROMPT_COMMAND_PREFIX: &str = "pi-task-attempt-prompt-v1/";
const TASK_ATTEMPT_SESSION_DISPOSE_COMMAND_PREFIX: &str = "pi-task-attempt-session-dispose-v1/";
const MAX_OPERATION_LABEL_BYTES: usize = 36;

#[cfg(feature = "test-support")]
type SpawnAdmissionTestHook =
    Box<dyn FnOnce(&mut KernelStore, society_kernel::OperatingCycleId) + Send>;

/// Stable daemon-internal identity for all kernel commands comprising one
/// Office child lifecycle.  A caller cannot supply individual command IDs,
/// so retrying a phase cannot silently alter its durable command relation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PiExecutionOperationId(String);

impl PiExecutionOperationId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, PiExecutionError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.is_empty()
            || bytes.len() > MAX_OPERATION_LABEL_BYTES
            || !bytes[0].is_ascii_alphanumeric()
            || !bytes.last().is_some_and(u8::is_ascii_alphanumeric)
            || !bytes
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        {
            return Err(PiExecutionError::InvalidOperationIdentity);
        }
        Ok(Self(value))
    }

    fn command_id(&self, command: PiExecutionCommand) -> Result<CommandId, PiExecutionError> {
        CommandId::parse(format!("{COMMAND_PREFIX}{}/{command}", self.0))
            .map_err(|_| PiExecutionError::InvalidOperationIdentity)
    }

    fn content_label(
        &self,
        child_process_id: NativeChildId,
        stream: ChildStreamKind,
    ) -> Result<String, PiExecutionError> {
        let label = format!(
            "pi-{}-c{}-{}",
            self.0,
            child_process_id.value(),
            stream_label(stream)
        );
        if label.len() > 80 {
            return Err(PiExecutionError::InvalidOperationIdentity);
        }
        Ok(label)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PiExecutionCommand {
    AdmitSpawn,
    RecordInertSpawn,
    RecordAdapterReady,
    AuthorizeCreate,
    RecordCreateDelivery,
    RecordSessionReady,
    RecordOfficeReady,
    RecordLiveness,
    RecordReap,
    RecordSignal { ordinal: usize },
    SealAdmittedControl,
    SealPhysicalStdin,
    SealStdout,
    SealStderr,
    Finalize,
    RecordNotSpawned,
}

impl std::fmt::Display for PiExecutionCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::AdmitSpawn => "admit-spawn",
            Self::RecordInertSpawn => "record-inert-spawn",
            Self::RecordAdapterReady => "record-adapter-ready",
            Self::AuthorizeCreate => "authorize-create",
            Self::RecordCreateDelivery => "record-create-delivery",
            Self::RecordSessionReady => "record-session-ready",
            Self::RecordOfficeReady => "record-office-ready",
            Self::RecordLiveness => "record-liveness",
            Self::RecordReap => "record-reap",
            Self::RecordSignal { ordinal } => return write!(formatter, "record-signal-{ordinal}"),
            Self::SealAdmittedControl => "seal-admitted-control",
            Self::SealPhysicalStdin => "seal-physical-stdin",
            Self::SealStdout => "seal-stdout",
            Self::SealStderr => "seal-stderr",
            Self::Finalize => "finalize",
            Self::RecordNotSpawned => "record-not-spawned",
        })
    }
}

/// Retry-stable command slots for one already-opened Office turn.  A session
/// lifecycle operation and an Office turn are intentionally distinct command
/// domains: a second turn cannot reuse an M5 child-receipt command identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PiOfficeTurnCommand {
    AuthorizePrompt,
    RecordPromptDelivery,
    RecordPromptAccepted,
    RecordKnownUsage { sequence: PiProtocolSequence },
    RecordUsageFailure { sequence: PiProtocolSequence },
    RecordTerminal,
    Settle,
}

/// Retry-stable command slots for the one TaskAssignment Prompt belonging to
/// one replaceable actor attempt. A task cannot reuse the Office-turn command
/// namespace: that would permit a root-authority session to collide with a
/// disposable actor's prompt by textual operation label alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PiTaskAttemptPromptCommand {
    AuthorizePrompt,
    RecordPromptDelivery,
    RecordPromptAccepted,
    RecordKnownUsage { sequence: PiProtocolSequence },
    RecordUsageFailure { sequence: PiProtocolSequence },
    RecordTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PiTaskAttemptSessionDisposeCommand {
    Authorize,
    RecordDelivery,
    RecordAccepted,
    RecordKnownUsage { sequence: PiProtocolSequence },
    RecordUsageFailure { sequence: PiProtocolSequence },
    RecordDisposed,
}

impl std::fmt::Display for PiTaskAttemptSessionDisposeCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Authorize => formatter.write_str("authorize"),
            Self::RecordDelivery => formatter.write_str("record-delivery"),
            Self::RecordAccepted => formatter.write_str("record-accepted"),
            Self::RecordKnownUsage { sequence } => {
                write!(formatter, "record-known-usage-{}", sequence.value())
            }
            Self::RecordUsageFailure { sequence } => {
                write!(formatter, "record-usage-failure-{}", sequence.value())
            }
            Self::RecordDisposed => formatter.write_str("record-disposed"),
        }
    }
}

impl std::fmt::Display for PiTaskAttemptPromptCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthorizePrompt => formatter.write_str("authorize-prompt"),
            Self::RecordPromptDelivery => formatter.write_str("record-prompt-delivery"),
            Self::RecordPromptAccepted => formatter.write_str("record-prompt-accepted"),
            Self::RecordKnownUsage { sequence } => {
                write!(formatter, "record-known-usage-{}", sequence.value())
            }
            Self::RecordUsageFailure { sequence } => {
                write!(formatter, "record-usage-failure-{}", sequence.value())
            }
            Self::RecordTerminal => formatter.write_str("record-terminal"),
        }
    }
}

impl std::fmt::Display for PiOfficeTurnCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthorizePrompt => formatter.write_str("authorize-prompt"),
            Self::RecordPromptDelivery => formatter.write_str("record-prompt-delivery"),
            Self::RecordPromptAccepted => formatter.write_str("record-prompt-accepted"),
            Self::RecordKnownUsage { sequence } => {
                write!(formatter, "record-known-usage-{}", sequence.value())
            }
            Self::RecordUsageFailure { sequence } => {
                write!(formatter, "record-usage-failure-{}", sequence.value())
            }
            Self::RecordTerminal => formatter.write_str("record-terminal"),
            Self::Settle => formatter.write_str("settle"),
        }
    }
}

/// Opaque daemon-internal identity for the M6 facts of one Office turn. It
/// derives every KERNEL-service command identity itself, so a caller cannot
/// splice a prompt authorization from one turn into another turn's terminal
/// or settlement command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PiOfficeTurnOperationId(String);

impl PiOfficeTurnOperationId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, PiExecutionError> {
        let value = value.into();
        // Keep exactly the established daemon operation spelling rather than
        // introducing a second ad-hoc identifier grammar.
        let _ = PiExecutionOperationId::parse(value.clone())?;
        Ok(Self(value))
    }

    fn command_id(&self, command: PiOfficeTurnCommand) -> Result<CommandId, PiExecutionError> {
        CommandId::parse(format!("{OFFICE_TURN_COMMAND_PREFIX}{}/{command}", self.0))
            .map_err(|_| PiExecutionError::InvalidOperationIdentity)
    }
}

/// Opaque resident identity for the single task Prompt receipt chain. It
/// shares the established operation-label grammar but derives command IDs in
/// a distinct namespace so retries cannot splice task and Office evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PiTaskAttemptPromptOperationId(String);

impl PiTaskAttemptPromptOperationId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, PiExecutionError> {
        let value = value.into();
        let _ = PiExecutionOperationId::parse(value.clone())?;
        Ok(Self(value))
    }

    fn command_id(
        &self,
        command: PiTaskAttemptPromptCommand,
    ) -> Result<CommandId, PiExecutionError> {
        CommandId::parse(format!(
            "{TASK_ATTEMPT_PROMPT_COMMAND_PREFIX}{}/{command}",
            self.0
        ))
        .map_err(|_| PiExecutionError::InvalidOperationIdentity)
    }
}

/// The task session's closing receipt chain has its own operation identity:
/// a transcript close must not replay as a task Prompt even for an identical
/// actor-attempt label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PiTaskAttemptSessionDisposeOperationId(String);

impl PiTaskAttemptSessionDisposeOperationId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, PiExecutionError> {
        let value = value.into();
        let _ = PiExecutionOperationId::parse(value.clone())?;
        Ok(Self(value))
    }

    fn command_id(
        &self,
        command: PiTaskAttemptSessionDisposeCommand,
    ) -> Result<CommandId, PiExecutionError> {
        CommandId::parse(format!(
            "{TASK_ATTEMPT_SESSION_DISPOSE_COMMAND_PREFIX}{}/{command}",
            self.0
        ))
        .map_err(|_| PiExecutionError::InvalidOperationIdentity)
    }

    fn transcript_content_operation(
        &self,
        digest: KernelDigest,
    ) -> Result<ContentSealOperationId, PiExecutionError> {
        let label = format!("pi-task-dispose-transcript-{}", self.0);
        ContentSealOperationId::parse(label, digest)
            .map_err(|_| PiExecutionError::InvalidOperationIdentity)
    }
}

/// Retry-stable command slots for one closing Root Authority Office session.
/// They deliberately do not share the M5 child or M6 turn command domains:
/// a transcript materialization cannot be replayed as a prior prompt fact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PiOfficeSessionDisposeCommand {
    Authorize,
    RecordDelivery,
    RecordAccepted,
    RecordKnownUsage { sequence: PiProtocolSequence },
    RecordUsageFailure { sequence: PiProtocolSequence },
    RecordDisposed,
}

impl std::fmt::Display for PiOfficeSessionDisposeCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Authorize => formatter.write_str("authorize"),
            Self::RecordDelivery => formatter.write_str("record-delivery"),
            Self::RecordAccepted => formatter.write_str("record-accepted"),
            Self::RecordKnownUsage { sequence } => {
                write!(formatter, "record-known-usage-{}", sequence.value())
            }
            Self::RecordUsageFailure { sequence } => {
                write!(formatter, "record-usage-failure-{}", sequence.value())
            }
            Self::RecordDisposed => formatter.write_str("record-disposed"),
        }
    }
}

/// Opaque daemon-internal identity for the entire closing session receipt
/// chain. The caller selects one canonical operation label, while this type
/// derives every kernel and content-seal command identity from it. Retrying a
/// live daemon therefore cannot splice a transcript from another session or
/// use a new command identity after physical disposal has begun.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PiOfficeSessionDisposeOperationId(String);

impl PiOfficeSessionDisposeOperationId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, PiExecutionError> {
        let value = value.into();
        let _ = PiExecutionOperationId::parse(value.clone())?;
        Ok(Self(value))
    }

    fn command_id(
        &self,
        command: PiOfficeSessionDisposeCommand,
    ) -> Result<CommandId, PiExecutionError> {
        CommandId::parse(format!(
            "{OFFICE_SESSION_DISPOSE_COMMAND_PREFIX}{}/{command}",
            self.0
        ))
        .map_err(|_| PiExecutionError::InvalidOperationIdentity)
    }

    fn transcript_content_operation(
        &self,
        digest: KernelDigest,
    ) -> Result<ContentSealOperationId, PiExecutionError> {
        let label = format!("pi-dispose-transcript-{}", self.0);
        ContentSealOperationId::parse(label, digest)
            .map_err(|_| PiExecutionError::InvalidOperationIdentity)
    }
}

const fn stream_label(stream: ChildStreamKind) -> &'static str {
    match stream {
        ChildStreamKind::AdmittedControl => "admitted-control",
        ChildStreamKind::PhysicalStdin => "physical-stdin",
        ChildStreamKind::Stdout => "stdout",
        ChildStreamKind::Stderr => "stderr",
    }
}

const fn stream_seal_command(stream: ChildStreamKind) -> PiExecutionCommand {
    match stream {
        ChildStreamKind::AdmittedControl => PiExecutionCommand::SealAdmittedControl,
        ChildStreamKind::PhysicalStdin => PiExecutionCommand::SealPhysicalStdin,
        ChildStreamKind::Stdout => PiExecutionCommand::SealStdout,
        ChildStreamKind::Stderr => PiExecutionCommand::SealStderr,
    }
}

/// Inputs already selected by trusted scheduling. The execution driver does
/// not discover a model, owner, capability, workspace, or command identity.
/// It merely turns this exact Office admission into a child receipt chain.
#[derive(Clone, Debug)]
pub(crate) struct OfficePiExecutionStart {
    pub(crate) operation: PiExecutionOperationId,
    pub(crate) operating_cycle_id: society_kernel::OperatingCycleId,
    pub(crate) office_session_id: RootAuthorityOfficeSessionId,
    pub(crate) budget_reservation_id: BudgetReservationId,
    pub(crate) execution_profile_id: ExecutionProfileId,
    pub(crate) expected_generation: AdmissionGeneration,
    pub(crate) supervisor_epoch_id: SupervisorEpochId,
    pub(crate) supervisor_epoch_identity: SupervisorEpochIdentity,
    pub(crate) spawn_request: PiSpawnRequest,
}

/// Inputs already selected by trusted scheduling for one replaceable actor
/// attempt. This is deliberately a separate type from
/// [`OfficePiExecutionStart`]: an actor child is owned by an `ActorAttempt`,
/// uses a `TaskAttempt` Pi session, and has no Office session or turn
/// authority to borrow.
#[derive(Clone, Debug)]
pub(crate) struct TaskAttemptPiExecutionStart {
    pub(crate) operation: PiExecutionOperationId,
    pub(crate) operating_cycle_id: society_kernel::OperatingCycleId,
    pub(crate) actor_attempt_id: ActorAttemptId,
    pub(crate) budget_reservation_id: BudgetReservationId,
    pub(crate) execution_profile_id: ExecutionProfileId,
    pub(crate) expected_generation: AdmissionGeneration,
    pub(crate) supervisor_epoch_id: SupervisorEpochId,
    pub(crate) supervisor_epoch_identity: SupervisorEpochIdentity,
    pub(crate) spawn_request: PiSpawnRequest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OfficePiExecutionPhase {
    SpawnRegistered,
    /// The exact PID/PGID was durable, then local peer/pipe setup failed and
    /// automatic containment began.  This handle admits only cancellation
    /// driving and receipt reconciliation; it can never reach AdapterReady.
    PostSpawnSetupContained,
    /// A registered child crossed a protocol/kernel/control error boundary.
    /// Its prior semantic phase is no longer usable for Office work; only
    /// deadline-driven containment and ordered physical reconciliation remain.
    BoundaryContainmentRequired,
    AdapterReadyRecorded,
    CreateAuthorized,
    CreateDelivered,
    SessionReadyRecorded,
    OfficeReadyRecorded,
    /// M6 prompt authorization exists, but its exact JSONL frame has not yet
    /// completed a physical pipe write. No other control may overtake it.
    OfficeTurnPromptDeliveryPending,
    /// The physical Prompt was delivered and the peer may now produce the
    /// accepted-result, usage, and terminal evidence chain.
    OfficeTurnPromptActive,
    /// A peer-valid non-ready outcome or accounting freeze leaves the Office
    /// turn durable and blocks further narrative mutation until a later
    /// dedicated closure/recovery transition exists.
    OfficeTurnTerminalBlocked,
    DisposeDeliveryPending,
    DisposeRequested,
    Disposed,
    DirectChildReapRecorded,
    LingeringCleanupRecorded,
    /// A distinct lingering-group kill was delivered while the group still
    /// existed. The retry-stable liveness command remains unspent until a
    /// later Absent/Inaccessible observation; another Present is transient
    /// process physics, not a new durable observation body.
    AwaitingLingeringGroupAbsence,
    Reconciled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TaskAttemptPiExecutionPhase {
    SpawnRegistered,
    PostSpawnSetupContained,
    BoundaryContainmentRequired,
    AdapterReadyRecorded,
    CreateAuthorized,
    CreateDelivered,
    SessionReadyRecorded,
    TaskPromptDeliveryPending,
    TaskPromptActive,
    /// Accounting could not be observed. The kernel freezes the exact
    /// reservation and the resident contains the child rather than treating
    /// missing usage as free or attempting disposal without a terminal.
    TaskPromptTerminalBlocked,
    TaskPromptTerminalRecorded,
    DisposeDeliveryPending,
    DisposeRequested,
    Disposed,
    DirectChildReapRecorded,
    LingeringCleanupRecorded,
    AwaitingLingeringGroupAbsence,
    Reconciled,
}

/// The daemon-private handle for exactly one registered Office child.  It has
/// no constructor outside the pre-spawn-to-registration transition.
#[derive(Clone, Debug)]
pub(crate) struct OfficePiExecutionChild {
    operation: PiExecutionOperationId,
    supervised_child_id: SupervisedChildId,
    child_process_id: NativeChildId,
    native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    office_session_id: RootAuthorityOfficeSessionId,
    /// Native paths remain daemon-private process custody facts. The kernel
    /// receives only a canonical transcript path after peer validation and a
    /// no-follow same-user file read below.
    workspace_directory: AbsolutePath,
    session_directory: AbsolutePath,
    pi_session_identity: PiBoundarySessionIdentity,
    spawn_nonce: KernelSpawnNonce,
    expected_generation: AdmissionGeneration,
    create_correlation: PiCorrelationIdentity,
    create_request_digest: KernelDigest,
    phase: OfficePiExecutionPhase,
}

/// Daemon-private custody for exactly one registered TaskAttempt child. The
/// type has no Office identity and its lifecycle has no Office-ready or
/// Office-turn phase; those are distinct root-authority capabilities.
#[derive(Clone, Debug)]
pub(crate) struct TaskAttemptPiExecutionChild {
    operation: PiExecutionOperationId,
    supervised_child_id: SupervisedChildId,
    child_process_id: NativeChildId,
    native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    actor_attempt_id: ActorAttemptId,
    workspace_directory: AbsolutePath,
    session_directory: AbsolutePath,
    pi_session_identity: PiBoundarySessionIdentity,
    spawn_nonce: KernelSpawnNonce,
    expected_generation: AdmissionGeneration,
    create_correlation: PiCorrelationIdentity,
    create_request_digest: KernelDigest,
    phase: TaskAttemptPiExecutionPhase,
}

/// Exact byte-bearing TaskAssignment prompt. The caller must arrange normal
/// content sealing/registration before this crosses the process boundary;
/// this type only proves that the physical prompt bytes match that digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SealedTaskAttemptPrompt {
    text: String,
    digest: KernelDigest,
}

impl SealedTaskAttemptPrompt {
    pub(crate) fn new(text: String, digest: KernelDigest) -> Result<Self, PiExecutionError> {
        if text.is_empty() || KernelDigest::of_bytes(text.as_bytes()) != digest {
            return Err(PiExecutionError::PromptContentDigestMismatch);
        }
        Ok(Self { text, digest })
    }
}

/// Inputs selected by trusted scheduling for the one actor-local Prompt. The
/// actor attempt and child come from the registered TaskAttempt handle, while
/// the application-owned prompt bytes have already entered immutable content
/// custody. No Office identity or synthetic-world field may enter here.
#[derive(Clone, Debug)]
pub(crate) struct TaskAttemptPiPromptStart {
    pub(crate) operation: PiTaskAttemptPromptOperationId,
    pub(crate) correlation_identity: PiCorrelationIdentity,
    pub(crate) prompt_content_object_id: ContentObjectId,
    pub(crate) prompt: SealedTaskAttemptPrompt,
    pub(crate) frontier_event_id: EventId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TaskAttemptPiPromptPhase {
    PromptDeliveryPending,
    AwaitingPromptAcceptance,
    AwaitingTerminalEvidence,
    UsageFrozen,
    TerminalRecorded,
}

/// Daemon-private custody for exactly one task assignment. This represents
/// runtime receipts only; the application still decides whether an actor's
/// task obligation completed through a separate typed study transition.
#[derive(Clone, Debug)]
pub(crate) struct TaskAttemptPiPrompt {
    operation: PiTaskAttemptPromptOperationId,
    correlation_identity: PiCorrelationIdentity,
    prompt_digest: KernelDigest,
    phase: TaskAttemptPiPromptPhase,
    accepted_sequence: Option<PiProtocolSequence>,
    agent_settled_sequence: Option<PiProtocolSequence>,
    latest_known_accounting_sequence: Option<PiProtocolSequence>,
    final_accounting_sequence: Option<PiProtocolSequence>,
}

/// Trusted scheduling selects the correlation and retry-stable operation
/// identity for the one close control. The child carries the immutable actor
/// attempt and session identities; a caller cannot replace either at dispose
/// time.
#[derive(Clone, Debug)]
pub(crate) struct TaskAttemptPiSessionDisposeStart {
    pub(crate) operation: PiTaskAttemptSessionDisposeOperationId,
    pub(crate) correlation_identity: PiCorrelationIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TaskAttemptPiSessionDisposePhase {
    DeliveryPending,
    AwaitingAcceptance,
    AwaitingFinalAccounting,
    AwaitingDisposed,
    UsageFrozen,
    Disposed,
}

/// Daemon-private state of the task session close. The exact child generation
/// is frozen from native admission; a later cycle change cannot silently
/// authorize a different actor session close.
#[derive(Clone, Debug)]
pub(crate) struct TaskAttemptPiSessionDispose {
    operation: PiTaskAttemptSessionDisposeOperationId,
    correlation_identity: PiCorrelationIdentity,
    expected_generation: AdmissionGeneration,
    phase: TaskAttemptPiSessionDisposePhase,
    accepted_sequence: Option<PiProtocolSequence>,
    final_accounting_sequence: Option<PiProtocolSequence>,
}

/// A task session shares the same physically verified transcript byte custody
/// as an Office session, but its terminal receipt is translated into the
/// separate task domain before it reaches the kernel.
impl VerifiedPiSessionTranscript {
    fn task_kernel_receipt_with_content(
        &self,
        sealed_content: Option<ContentObjectRegistration>,
    ) -> Result<society_kernel::PiTaskAttemptSessionTranscriptReceipt, PiExecutionError> {
        match self {
            Self::Materialized(request) => {
                let sealed_content =
                    sealed_content.ok_or(PiExecutionError::TranscriptContentMissing)?;
                if sealed_content.digest != request.session_file_digest {
                    return Err(PiExecutionError::TranscriptDigestMismatch);
                }
                let first_user_prompt = match request.first_user_prompt {
                    PiOfficeSessionFirstUserPromptReceipt::Absent => {
                        society_kernel::PiTaskAttemptFirstUserPromptReceipt::Absent
                    }
                    PiOfficeSessionFirstUserPromptReceipt::Verified { digest } => {
                        society_kernel::PiTaskAttemptFirstUserPromptReceipt::Verified { digest }
                    }
                };
                Ok(
                    society_kernel::PiTaskAttemptSessionTranscriptReceipt::Materialized {
                        session_file: request.session_file.clone(),
                        session_file_digest: request.session_file_digest,
                        transcript_content_object_id: sealed_content.content_object_id,
                        first_user_prompt,
                    },
                )
            }
            Self::UnmaterializedNoPrompt { session_file } => {
                if sealed_content.is_some() {
                    return Err(PiExecutionError::TranscriptContentUnexpected);
                }
                Ok(
                    society_kernel::PiTaskAttemptSessionTranscriptReceipt::UnmaterializedNoPrompt {
                        session_file: session_file.clone(),
                    },
                )
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct VerifiedTaskAttemptSessionDisposeTerminal {
    transcript: VerifiedPiSessionTranscript,
    disposed_sequence: PiProtocolSequence,
}

impl VerifiedTaskAttemptSessionDisposeTerminal {
    pub(crate) fn transcript(&self) -> &VerifiedPiSessionTranscript {
        &self.transcript
    }

    pub(crate) const fn disposed_sequence(&self) -> PiProtocolSequence {
        self.disposed_sequence
    }
}

#[derive(Clone, Debug)]
pub(crate) enum TaskAttemptPiSessionDisposeOutput {
    DeliveryRecorded,
    Accepted,
    KnownUsageRecorded,
    UsageFrozen,
    TranscriptReady(Box<VerifiedTaskAttemptSessionDisposeTerminal>),
    Disposed,
}

/// One peer-projected task-Prompt fact. Forum calls stay explicit so the
/// resident can perform the existing obligation-scoped tool transition before
/// returning a result to the host; no actor output is recast as task success.
#[derive(Clone, Debug)]
pub(crate) enum TaskAttemptPiPromptOutput {
    ControlInterleaving,
    PromptAccepted,
    ForumToolCall {
        correlation_identity: CorrelationIdentity,
        tool_call_identity: society_pi::ToolCallIdentity,
        tool_name: society_pi::ForumToolName,
        args: society_pi::SdkJsonValue,
    },
    KnownUsageRecorded,
    UsageFrozen,
    TerminalRecorded,
}

/// Exact byte-bearing Office prompt which was already physically sealed and
/// registered as a global `ContentObject`. The rendering is still supplied to
/// the host, but its digest cannot drift from the kernel authorization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SealedOfficePrompt {
    text: String,
    digest: KernelDigest,
}

impl SealedOfficePrompt {
    pub(crate) fn new(text: String, digest: KernelDigest) -> Result<Self, PiExecutionError> {
        if text.is_empty() || KernelDigest::of_bytes(text.as_bytes()) != digest {
            return Err(PiExecutionError::PromptContentDigestMismatch);
        }
        Ok(Self { text, digest })
    }
}

/// Inputs already selected by trusted Office scheduling. `societyd` does not
/// discover prompt bytes, frontier, correlation, or content identity; it
/// binds this exact pre-sealed prompt to the live session's M6 authority.
#[derive(Clone, Debug)]
pub(crate) struct OfficePiTurnStart {
    pub(crate) operation: PiOfficeTurnOperationId,
    pub(crate) office_turn_id: OfficeTurnId,
    pub(crate) correlation_identity: PiCorrelationIdentity,
    pub(crate) prompt_content_object_id: ContentObjectId,
    pub(crate) prompt: SealedOfficePrompt,
    pub(crate) frontier_event_id: EventId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OfficePiTurnPhase {
    PromptDeliveryPending,
    AwaitingPromptAcceptance,
    AwaitingTerminalEvidence,
    UsageFrozen,
    TerminalRecorded,
    Settled,
}

/// Daemon-private custody for exactly one M6 Office Prompt. It records only
/// host facts which the `BoundaryPeer` accepted from raw stdout; it does not
/// turn narrative output into a semantic submission.
#[derive(Clone, Debug)]
pub(crate) struct OfficePiTurn {
    operation: PiOfficeTurnOperationId,
    office_turn_id: OfficeTurnId,
    correlation_identity: PiCorrelationIdentity,
    prompt_digest: KernelDigest,
    phase: OfficePiTurnPhase,
    accepted_sequence: Option<PiProtocolSequence>,
    agent_settled_sequence: Option<PiProtocolSequence>,
    latest_known_accounting_sequence: Option<PiProtocolSequence>,
    final_accounting_sequence: Option<PiProtocolSequence>,
}

/// Inputs already selected by trusted Office scheduling for the one closing
/// Pi session. The session identity itself stays on the registered child; a
/// caller cannot replace it while reusing this correlation or operation.
#[derive(Clone, Debug)]
pub(crate) struct OfficePiSessionDisposeStart {
    pub(crate) operation: PiOfficeSessionDisposeOperationId,
    pub(crate) correlation_identity: PiCorrelationIdentity,
    /// The current Operating Cycle admission generation selected by trusted
    /// scheduling immediately before Dispose authorization. It is distinct
    /// from the child spawn generation: Quiesce advances the cycle before an
    /// idle Office session may close.
    pub(crate) expected_generation: AdmissionGeneration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OfficePiSessionDisposePhase {
    DeliveryPending,
    AwaitingAcceptance,
    AwaitingFinalAccounting,
    AwaitingDisposed,
    UsageFrozen,
    Disposed,
}

/// Daemon-private custody for the closing Pi session receipt chain. A typed
/// usage inability deliberately has no `Disposed` successor: Pi v1 fences on
/// that frame, and leaving the session open preserves the frozen parent for a
/// later recovery tranche.
#[derive(Clone, Debug)]
pub(crate) struct OfficePiSessionDispose {
    operation: PiOfficeSessionDisposeOperationId,
    correlation_identity: PiCorrelationIdentity,
    /// Frozen only after the kernel accepted `Authorize...`. Every later
    /// delivery/usage/terminal receipt must name this exact generation even
    /// if another cancellation transition advances the live cycle later.
    expected_generation: AdmissionGeneration,
    phase: OfficePiSessionDisposePhase,
    accepted_sequence: Option<PiProtocolSequence>,
    final_accounting_sequence: Option<PiProtocolSequence>,
}

/// A peer-validated materialized SessionManager transcript, opened by the
/// daemon under the session workspace's filesystem custody rules. It remains
/// an in-memory request for the daemon's sole content writer; this type
/// carries no content-object identity until that writer has physically sealed
/// the exact bytes.
#[derive(Clone, Debug)]
pub(crate) struct VerifiedPiSessionTranscriptSealRequest {
    content_operation: ContentSealOperationId,
    session_file: CanonicalPiSessionTranscriptPath,
    session_file_digest: KernelDigest,
    first_user_prompt: PiOfficeSessionFirstUserPromptReceipt,
    bytes: Vec<u8>,
}

impl VerifiedPiSessionTranscriptSealRequest {
    pub(crate) fn content_operation(&self) -> &ContentSealOperationId {
        &self.content_operation
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn kernel_receipt_with_content(
        &self,
        content_object_id: ContentObjectId,
        sealed_digest: KernelDigest,
    ) -> Result<PiOfficeSessionTranscriptReceipt, PiExecutionError> {
        if sealed_digest != self.session_file_digest {
            return Err(PiExecutionError::TranscriptDigestMismatch);
        }
        Ok(PiOfficeSessionTranscriptReceipt::Materialized {
            session_file: self.session_file.clone(),
            session_file_digest: self.session_file_digest,
            transcript_content_object_id: content_object_id,
            first_user_prompt: self.first_user_prompt,
        })
    }
}

/// The only two peer-valid transcript materialization states. In particular,
/// `UnmaterializedNoPrompt` cannot be passed to the content writer, so a
/// header-only or absent Pi session file never fabricates a ContentObject.
#[derive(Clone, Debug)]
pub(crate) enum VerifiedPiSessionTranscript {
    Materialized(VerifiedPiSessionTranscriptSealRequest),
    UnmaterializedNoPrompt {
        session_file: CanonicalPiSessionTranscriptPath,
    },
}

impl VerifiedPiSessionTranscript {
    fn kernel_receipt_with_content(
        &self,
        sealed_content: Option<ContentObjectRegistration>,
    ) -> Result<PiOfficeSessionTranscriptReceipt, PiExecutionError> {
        match self {
            Self::Materialized(request) => {
                let sealed_content =
                    sealed_content.ok_or(PiExecutionError::TranscriptContentMissing)?;
                request.kernel_receipt_with_content(
                    sealed_content.content_object_id,
                    sealed_content.digest,
                )
            }
            Self::UnmaterializedNoPrompt { session_file } => {
                if sealed_content.is_some() {
                    return Err(PiExecutionError::TranscriptContentUnexpected);
                }
                Ok(PiOfficeSessionTranscriptReceipt::UnmaterializedNoPrompt {
                    session_file: session_file.clone(),
                })
            }
        }
    }
}

/// One peer-accepted session-closing coordinate. Keeping the transcript and
/// its exact `Disposed` sequence together prevents a caller from sealing one
/// terminal file while attempting to commit another sequence.
#[derive(Clone, Debug)]
pub(crate) struct VerifiedPiSessionDisposeTerminal {
    transcript: VerifiedPiSessionTranscript,
    disposed_sequence: PiProtocolSequence,
}

impl VerifiedPiSessionDisposeTerminal {
    pub(crate) fn transcript(&self) -> &VerifiedPiSessionTranscript {
        &self.transcript
    }

    pub(crate) const fn disposed_sequence(&self) -> PiProtocolSequence {
        self.disposed_sequence
    }
}

/// One frame-at-a-time projection result for the closed Office-session
/// Dispose chain. It names only durable facts that can be recorded from a
/// peer-sealed raw stdout frame; it never turns transcript bytes into a
/// content object itself.
#[derive(Clone, Debug)]
pub(crate) enum OfficePiSessionDisposeOutput {
    DeliveryRecorded,
    Accepted,
    KnownUsageRecorded,
    UsageFrozen,
    /// The host's peer-validated terminal transcript has been opened under
    /// daemon filesystem custody. The daemon must seal materialized bytes
    /// through its sole content authority (or prove the unmaterialized arm)
    /// before asking this driver to record the kernel terminal.
    TranscriptReady(Box<VerifiedPiSessionDisposeTerminal>),
    Disposed,
}

/// The one-frame-at-a-time resident result of projecting peer evidence. It is
/// intentionally not a semantic submission or generic event log; its arms
/// correspond only to the M6 kernel facts this bridge may durably attest.
#[derive(Clone, Debug)]
pub(crate) enum OfficePiTurnOutput {
    ControlInterleaving,
    PromptAccepted,
    /// One peer-validated Forum request. The resident must execute its
    /// closed study transition and then return a result before polling the
    /// next host frame.
    ForumToolCall {
        correlation_identity: CorrelationIdentity,
        tool_call_identity: society_pi::ToolCallIdentity,
        tool_name: society_pi::ForumToolName,
        args: society_pi::SdkJsonValue,
    },
    KnownUsageRecorded,
    UsageFrozen,
    TerminalRecordedNonReady,
    SettledReady,
}

impl PartialEq for OfficePiTurnOutput {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ControlInterleaving, Self::ControlInterleaving)
            | (Self::PromptAccepted, Self::PromptAccepted)
            | (Self::KnownUsageRecorded, Self::KnownUsageRecorded)
            | (Self::UsageFrozen, Self::UsageFrozen)
            | (Self::TerminalRecordedNonReady, Self::TerminalRecordedNonReady)
            | (Self::SettledReady, Self::SettledReady) => true,
            (
                Self::ForumToolCall {
                    correlation_identity: left_correlation,
                    tool_call_identity: left_call,
                    tool_name: left_name,
                    args: left_args,
                },
                Self::ForumToolCall {
                    correlation_identity: right_correlation,
                    tool_call_identity: right_call,
                    tool_name: right_name,
                    args: right_args,
                },
            ) => {
                left_correlation == right_correlation
                    && left_call == right_call
                    && left_name == right_name
                    && society_pi::sdk_json_values_equal(left_args, right_args)
            }
            _ => false,
        }
    }
}

impl Eq for OfficePiTurnOutput {}

/// A native child exists, but the kernel rejected (or could not persist) its
/// first PID/PGID receipt. There is intentionally no `NativeChildId`: the
/// admission stays durably unresolved and cannot be rewritten as
/// `NotSpawned`. The current resident must only finish physical containment;
/// a restart remains RecoveryFenced because no later process can attach to
/// this unregistered native identity.
#[derive(Debug)]
pub(crate) struct UnregisteredPiChild {
    supervised_child_id: SupervisedChildId,
    native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    phase: UnregisteredPiChildPhase,
    transient_completion: Option<SupervisionReceipt>,
}

// Kept as a source-compatible alias for the daemon's existing containment
// import. The custody object itself is owner-neutral and carries no Office
// identity, so TaskAttempt and Office admissions share the same unresolved
// native-child safety path.
pub(crate) type UnregisteredOfficePiChild = UnregisteredPiChild;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UnregisteredPiChildPhase {
    ContainmentRequired,
    PhysicallyReaped,
}

impl UnregisteredPiChild {
    pub(crate) fn native_child_spawn_admission_id(&self) -> NativeChildSpawnAdmissionId {
        self.native_child_spawn_admission_id
    }

    pub(crate) fn transient_completion(&self) -> Option<&SupervisionReceipt> {
        self.transient_completion.as_ref()
    }
}

/// The first bridge transition has two non-ambiguous outcomes.  A physical
/// child is never collapsed into `RecordNativeChildNotSpawned`: callers receive
/// its registered handle and must drive/reconcile containment.
#[derive(Debug)]
pub(crate) enum OfficePiSpawnRegistration {
    Ready(OfficePiExecutionChild),
    PostSpawnSetupContained {
        child: OfficePiExecutionChild,
        failure: PostSpawnSetupFailure,
    },
    /// Registration succeeded, so the kernel can receive the later exact
    /// signal/reap/seal chain, but a non-setup local initialization error
    /// forced boundary containment before AdapterReady.
    RegisteredBoundaryContained {
        child: OfficePiExecutionChild,
        failure: SupervisionError,
    },
    /// The kernel has no child-process identity to receive later signal,
    /// liveness, stream-seal, or finalization commands. This typed outcome
    /// owns only the already-spawned native child until physical reaping.
    RegistrationUnresolved {
        // Keep the exceptional native-custody handle indirect: a normal
        // successful registration should not carry the transient supervisor
        // receipt buffer required only when the kernel rejected its first
        // child receipt.
        child: Box<UnregisteredPiChild>,
        failure: PiExecutionError,
    },
}

#[derive(Debug)]
pub(crate) enum TaskAttemptPiSpawnRegistration {
    Ready(TaskAttemptPiExecutionChild),
    PostSpawnSetupContained {
        child: TaskAttemptPiExecutionChild,
        failure: PostSpawnSetupFailure,
    },
    RegisteredBoundaryContained {
        child: TaskAttemptPiExecutionChild,
        failure: SupervisionError,
    },
    RegistrationUnresolved {
        child: Box<UnregisteredPiChild>,
        failure: PiExecutionError,
    },
}

impl OfficePiExecutionChild {
    pub(crate) fn child_process_id(&self) -> NativeChildId {
        self.child_process_id
    }

    pub(crate) fn native_child_spawn_admission_id(&self) -> NativeChildSpawnAdmissionId {
        self.native_child_spawn_admission_id
    }

    pub(crate) fn office_session_id(&self) -> RootAuthorityOfficeSessionId {
        self.office_session_id
    }

    pub(crate) fn phase(&self) -> &'static str {
        match self.phase {
            OfficePiExecutionPhase::SpawnRegistered => "spawn_registered",
            OfficePiExecutionPhase::PostSpawnSetupContained => "post_spawn_setup_contained",
            OfficePiExecutionPhase::BoundaryContainmentRequired => "boundary_containment_required",
            OfficePiExecutionPhase::AdapterReadyRecorded => "adapter_ready_recorded",
            OfficePiExecutionPhase::CreateAuthorized => "create_authorized",
            OfficePiExecutionPhase::CreateDelivered => "create_delivered",
            OfficePiExecutionPhase::SessionReadyRecorded => "session_ready_recorded",
            OfficePiExecutionPhase::OfficeReadyRecorded => "office_ready_recorded",
            OfficePiExecutionPhase::OfficeTurnPromptDeliveryPending => {
                "office_turn_prompt_delivery_pending"
            }
            OfficePiExecutionPhase::OfficeTurnPromptActive => "office_turn_prompt_active",
            OfficePiExecutionPhase::OfficeTurnTerminalBlocked => "office_turn_terminal_blocked",
            OfficePiExecutionPhase::DisposeDeliveryPending => "dispose_delivery_pending",
            OfficePiExecutionPhase::DisposeRequested => "dispose_requested",
            OfficePiExecutionPhase::Disposed => "disposed",
            OfficePiExecutionPhase::DirectChildReapRecorded => "direct_child_reap_recorded",
            OfficePiExecutionPhase::LingeringCleanupRecorded => "lingering_cleanup_recorded",
            OfficePiExecutionPhase::AwaitingLingeringGroupAbsence => {
                "awaiting_lingering_group_absence"
            }
            OfficePiExecutionPhase::Reconciled => "reconciled",
        }
    }
}

impl TaskAttemptPiExecutionChild {
    pub(crate) fn child_process_id(&self) -> NativeChildId {
        self.child_process_id
    }

    pub(crate) fn native_child_spawn_admission_id(&self) -> NativeChildSpawnAdmissionId {
        self.native_child_spawn_admission_id
    }

    pub(crate) fn actor_attempt_id(&self) -> ActorAttemptId {
        self.actor_attempt_id
    }

    pub(crate) fn phase(&self) -> &'static str {
        match self.phase {
            TaskAttemptPiExecutionPhase::SpawnRegistered => "spawn_registered",
            TaskAttemptPiExecutionPhase::PostSpawnSetupContained => "post_spawn_setup_contained",
            TaskAttemptPiExecutionPhase::BoundaryContainmentRequired => {
                "boundary_containment_required"
            }
            TaskAttemptPiExecutionPhase::AdapterReadyRecorded => "adapter_ready_recorded",
            TaskAttemptPiExecutionPhase::CreateAuthorized => "create_authorized",
            TaskAttemptPiExecutionPhase::CreateDelivered => "create_delivered",
            TaskAttemptPiExecutionPhase::SessionReadyRecorded => "session_ready_recorded",
            TaskAttemptPiExecutionPhase::TaskPromptDeliveryPending => {
                "task_prompt_delivery_pending"
            }
            TaskAttemptPiExecutionPhase::TaskPromptActive => "task_prompt_active",
            TaskAttemptPiExecutionPhase::TaskPromptTerminalBlocked => {
                "task_prompt_terminal_blocked"
            }
            TaskAttemptPiExecutionPhase::TaskPromptTerminalRecorded => {
                "task_prompt_terminal_recorded"
            }
            TaskAttemptPiExecutionPhase::DisposeDeliveryPending => "dispose_delivery_pending",
            TaskAttemptPiExecutionPhase::DisposeRequested => "dispose_requested",
            TaskAttemptPiExecutionPhase::Disposed => "disposed",
            TaskAttemptPiExecutionPhase::DirectChildReapRecorded => "direct_child_reap_recorded",
            TaskAttemptPiExecutionPhase::LingeringCleanupRecorded => "lingering_cleanup_recorded",
            TaskAttemptPiExecutionPhase::AwaitingLingeringGroupAbsence => {
                "awaiting_lingering_group_absence"
            }
            TaskAttemptPiExecutionPhase::Reconciled => "reconciled",
        }
    }
}

/// The resident-only process bridge.  It has no restart attach API: a new
/// daemon is RecoveryFenced and must use the kernel's separate parentage-loss
/// recovery receipts rather than pretending a `Child` can be reconstructed.
pub(crate) struct PiExecutionDriver {
    supervisor: PiSupervisor,
    /// Test-only scheduling seam: a real host can write SessionReady and
    /// disappear while the daemon commits that protocol fact. Production
    /// never delays this boundary; the seam makes that otherwise microscopic
    /// window reproducible against the provider-free native-host double.
    #[cfg(feature = "test-support")]
    pause_before_office_ready_liveness_for_test: Option<std::time::Duration>,
    /// Tests can advance the operating-cycle generation after durable
    /// admission but before native spawn returns. This is the exact race M5
    /// permits; production has no callback and instead reads the kernel's
    /// current generation immediately before the registration receipt.
    #[cfg(feature = "test-support")]
    after_spawn_admission_for_test: Option<SpawnAdmissionTestHook>,
    /// Test-only deterministic stand-in for a kernel rejection after native
    /// `exec` but before the first child PID/PGID receipt. It proves this
    /// seam fences the admission instead of inventing `NotSpawned`.
    #[cfg(feature = "test-support")]
    inert_registration_rejection_for_test: Option<society_kernel::Rejection>,
}

impl Default for PiExecutionDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl PiExecutionDriver {
    pub(crate) fn new() -> Self {
        Self {
            supervisor: PiSupervisor::new(),
            #[cfg(feature = "test-support")]
            pause_before_office_ready_liveness_for_test: None,
            #[cfg(feature = "test-support")]
            after_spawn_admission_for_test: None,
            #[cfg(feature = "test-support")]
            inert_registration_rejection_for_test: None,
        }
    }

    #[cfg(feature = "test-support")]
    fn with_supervisor_for_test(supervisor: PiSupervisor) -> Self {
        Self {
            supervisor,
            pause_before_office_ready_liveness_for_test: None,
            after_spawn_admission_for_test: None,
            inert_registration_rejection_for_test: None,
        }
    }

    #[cfg(feature = "test-support")]
    fn pause_before_office_ready_liveness_for_test(&mut self, duration: std::time::Duration) {
        self.pause_before_office_ready_liveness_for_test = Some(duration);
    }

    #[cfg(feature = "test-support")]
    fn after_spawn_admission_for_test(
        &mut self,
        callback: impl FnOnce(&mut KernelStore, society_kernel::OperatingCycleId) + Send + 'static,
    ) {
        self.after_spawn_admission_for_test = Some(Box::new(callback));
    }

    #[cfg(feature = "test-support")]
    fn force_next_control_write_pending_for_test(
        &mut self,
        child: &OfficePiExecutionChild,
    ) -> Result<(), SupervisionError> {
        self.supervisor
            .force_next_control_write_pending_for_test(&child.supervised_child_id)
    }

    #[cfg(test)]
    fn send_get_state_for_test(
        &mut self,
        child: &OfficePiExecutionChild,
        correlation: CorrelationIdentity,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        self.supervisor
            .send_get_state(&child.supervised_child_id, correlation, now, deadline)
            .map_err(PiExecutionError::Supervision)
    }

    #[cfg(feature = "test-support")]
    fn registered_child_count_for_test(&self) -> usize {
        self.supervisor.registered_child_count_for_test()
    }

    #[cfg(feature = "test-support")]
    fn reject_inert_registration_for_test(&mut self, rejection: society_kernel::Rejection) {
        self.inert_registration_rejection_for_test = Some(rejection);
    }

    /// Commits pre-spawn authority, performs only then the native inert spawn,
    /// and promptly commits the PID/PGID receipt before reading AdapterReady.
    pub(crate) fn admit_spawn_and_register(
        &mut self,
        store: &mut KernelStore,
        start: OfficePiExecutionStart,
    ) -> Result<OfficePiSpawnRegistration, PiExecutionError> {
        // A Root Authority Office owner and an Office session must be the
        // same closed boundary. Reject this before any kernel admission,
        // native workspace action, or host process exists; a TaskAttempt
        // payload cannot borrow the Office budget/session authority.
        if start.spawn_request.create_session.session_kind != SessionKind::RootAuthorityOffice {
            return Err(PiExecutionError::OfficeSessionKindRequired);
        }
        // This is deliberately before `AdmitPiChildSpawn`: malformed native
        // paths/profile/artifacts must not create an admission that no exact
        // process outcome can close. `spawn_native` repeats the same checks
        // immediately before exec and maps only that proven-absent TOCTOU
        // failure into `RecordNativeChildNotSpawned`.
        self.supervisor
            .preflight_spawn(&start.spawn_request)
            .map_err(PiExecutionError::Supervision)?;
        let expected_generation = ExpectedGeneration::Exact(start.expected_generation);
        let workspace_id = kernel_workspace_identity(&start.spawn_request)?;
        let workspace_path = kernel_workspace_path(&start.spawn_request)?;
        let pi_session_identity = kernel_session_identity(&start.spawn_request.session_identity)?;
        let spawn_nonce = kernel_spawn_nonce(&start.spawn_request.spawn_nonce)?;
        let create_correlation =
            kernel_correlation(&start.spawn_request.create_correlation_identity)?;
        let create_request_digest = canonical_create_request_digest(&start.spawn_request)?;

        let admitted = execute_kernel_command(
            store,
            &start.operation,
            PiExecutionCommand::AdmitSpawn,
            Capability::AdmitPiChildSpawn,
            expected_generation,
            CommandBody::AdmitPiChildSpawn {
                operating_cycle_id: start.operating_cycle_id,
                owner: PiChildOwner::RootAuthorityOfficeSession(start.office_session_id),
                budget_reservation_id: start.budget_reservation_id,
                execution_profile_id: start.execution_profile_id,
                native_workspace_id: workspace_id,
                canonical_workspace_path: workspace_path,
                supervisor_epoch_id: start.supervisor_epoch_id,
                supervisor_epoch_identity: start.supervisor_epoch_identity.clone(),
                pi_session_identity: pi_session_identity.clone(),
                spawn_nonce: spawn_nonce.clone(),
            },
        )?;
        let admission_id = match admitted {
            EventBody::PiChildSpawnAdmitted {
                native_child_spawn_admission_id,
                owner: PiChildOwner::RootAuthorityOfficeSession(session_id),
                budget_reservation_id,
            } if session_id == start.office_session_id
                && budget_reservation_id == start.budget_reservation_id =>
            {
                native_child_spawn_admission_id
            }
            _ => return Err(PiExecutionError::UnexpectedKernelEvent),
        };

        #[cfg(feature = "test-support")]
        if let Some(callback) = self.after_spawn_admission_for_test.take() {
            callback(store, start.operating_cycle_id);
        }

        let spawned = match self.supervisor.spawn_native(start.spawn_request.clone()) {
            Ok(facts) => facts,
            Err(spawn_error) => {
                if let Some(reason) = proven_not_spawned_reason(&spawn_error) {
                    let current_generation = store
                        .current_operating_cycle_admission_generation(start.operating_cycle_id)?;
                    execute_kernel_command(
                        store,
                        &start.operation,
                        PiExecutionCommand::RecordNotSpawned,
                        Capability::RecordNativeChildNotSpawned,
                        ExpectedGeneration::Exact(current_generation),
                        CommandBody::RecordNativeChildNotSpawned {
                            native_child_spawn_admission_id: admission_id,
                            reason,
                        },
                    )?;
                }
                return Err(PiExecutionError::Supervision(spawn_error));
            }
        };
        let (child_identity, direct_child_pid, process_group_id) = match (
            kernel_child_identity(&spawned.child_process_id),
            kernel_child_pid(spawned.host_process_id.value()),
            kernel_process_group_id(spawned.process_group_id.value()),
        ) {
            (Ok(child_identity), Ok(direct_child_pid), Ok(process_group_id)) => {
                (child_identity, direct_child_pid, process_group_id)
            }
            (child_identity, direct_child_pid, process_group_id) => {
                let failure = match child_identity {
                    Err(error) => error,
                    Ok(_) => match direct_child_pid {
                        Err(error) => error,
                        Ok(_) => match process_group_id {
                            Err(error) => error,
                            Ok(_) => unreachable!("successful conversion tuple was matched above"),
                        },
                    },
                };
                return Ok(self.unresolved_registration(
                    spawned.child_process_id,
                    admission_id,
                    failure,
                ));
            }
        };
        // Cancellation may have advanced the active cycle after the durable
        // admission but before native `exec` returned. M5 deliberately
        // permits this raced spawn only under that *current* generation,
        // where the kernel attaches it to the frozen cancellation target.
        let registration_generation =
            match store.current_operating_cycle_admission_generation(start.operating_cycle_id) {
                Ok(generation) => generation,
                Err(error) => {
                    return Ok(self.unresolved_registration(
                        spawned.child_process_id,
                        admission_id,
                        PiExecutionError::Kernel(error),
                    ));
                }
            };
        #[cfg(feature = "test-support")]
        let injected_registration_failure = self.inert_registration_rejection_for_test.take();
        #[cfg(not(feature = "test-support"))]
        let injected_registration_failure: Option<society_kernel::Rejection> = None;
        let registered = match injected_registration_failure {
            Some(rejection) => Err(PiExecutionError::KernelCommandRejected {
                capability: Capability::RecordInertChildSpawn,
                rejection,
            }),
            None => execute_kernel_command(
                store,
                &start.operation,
                PiExecutionCommand::RecordInertSpawn,
                Capability::RecordInertChildSpawn,
                ExpectedGeneration::Exact(registration_generation),
                CommandBody::RecordInertChildSpawn {
                    native_child_spawn_admission_id: admission_id,
                    child_identity,
                    direct_child_pid,
                    process_group_id,
                },
            ),
        };
        let registered = match registered {
            Ok(event) => event,
            Err(error) => {
                return Ok(self.unresolved_registration(
                    spawned.child_process_id,
                    admission_id,
                    error,
                ));
            }
        };
        let child_process_id = match registered {
            EventBody::InertPiChildSpawnRecorded {
                native_child_id: child_process_id,
                native_child_spawn_admission_id,
            } if native_child_spawn_admission_id == admission_id => child_process_id,
            _ => {
                return Ok(self.unresolved_registration(
                    spawned.child_process_id,
                    admission_id,
                    PiExecutionError::UnexpectedKernelEvent,
                ));
            }
        };
        let mut child = OfficePiExecutionChild {
            operation: start.operation,
            supervised_child_id: spawned.child_process_id,
            child_process_id,
            native_child_spawn_admission_id: admission_id,
            office_session_id: start.office_session_id,
            workspace_directory: start.spawn_request.workspace.directory().clone(),
            session_directory: start.spawn_request.create_session.session_directory.clone(),
            pi_session_identity,
            spawn_nonce,
            expected_generation: registration_generation,
            create_correlation,
            create_request_digest,
            phase: OfficePiExecutionPhase::SpawnRegistered,
        };
        match self
            .supervisor
            .finish_inert_setup(&child.supervised_child_id, MonotonicTick::ZERO)
        {
            Ok(()) => Ok(OfficePiSpawnRegistration::Ready(child)),
            Err(SupervisionError::PostSpawnSetup(failure)) => {
                child.phase = OfficePiExecutionPhase::PostSpawnSetupContained;
                Ok(OfficePiSpawnRegistration::PostSpawnSetupContained { child, failure })
            }
            Err(error) => {
                child.phase = OfficePiExecutionPhase::BoundaryContainmentRequired;
                self.contain(&child.supervised_child_id, MonotonicTick::ZERO);
                Ok(OfficePiSpawnRegistration::RegisteredBoundaryContained {
                    child,
                    failure: error,
                })
            }
        }
    }

    /// Opens the native child bridge for one replaceable actor. This path is
    /// intentionally parallel to the root-authority Office path but has a
    /// closed, non-interchangeable owner and session kind. In particular it
    /// never creates an Office session or an Office-ready receipt.
    pub(crate) fn admit_task_attempt_spawn_and_register(
        &mut self,
        store: &mut KernelStore,
        start: TaskAttemptPiExecutionStart,
    ) -> Result<TaskAttemptPiSpawnRegistration, PiExecutionError> {
        if start.spawn_request.create_session.session_kind != SessionKind::TaskAttempt {
            return Err(PiExecutionError::TaskAttemptSessionKindRequired);
        }
        self.supervisor
            .preflight_spawn(&start.spawn_request)
            .map_err(PiExecutionError::Supervision)?;
        let expected_generation = ExpectedGeneration::Exact(start.expected_generation);
        let workspace_id = kernel_workspace_identity(&start.spawn_request)?;
        let workspace_path = kernel_workspace_path(&start.spawn_request)?;
        let pi_session_identity = kernel_session_identity(&start.spawn_request.session_identity)?;
        let spawn_nonce = kernel_spawn_nonce(&start.spawn_request.spawn_nonce)?;
        let create_correlation =
            kernel_correlation(&start.spawn_request.create_correlation_identity)?;
        let create_request_digest = canonical_create_request_digest(&start.spawn_request)?;

        let admitted = execute_kernel_command(
            store,
            &start.operation,
            PiExecutionCommand::AdmitSpawn,
            Capability::AdmitPiChildSpawn,
            expected_generation,
            CommandBody::AdmitPiChildSpawn {
                operating_cycle_id: start.operating_cycle_id,
                owner: PiChildOwner::ActorAttempt(start.actor_attempt_id),
                budget_reservation_id: start.budget_reservation_id,
                execution_profile_id: start.execution_profile_id,
                native_workspace_id: workspace_id,
                canonical_workspace_path: workspace_path,
                supervisor_epoch_id: start.supervisor_epoch_id,
                supervisor_epoch_identity: start.supervisor_epoch_identity.clone(),
                pi_session_identity: pi_session_identity.clone(),
                spawn_nonce: spawn_nonce.clone(),
            },
        )?;
        let admission_id = match admitted {
            EventBody::PiChildSpawnAdmitted {
                native_child_spawn_admission_id,
                owner: PiChildOwner::ActorAttempt(actor_attempt_id),
                budget_reservation_id,
            } if actor_attempt_id == start.actor_attempt_id
                && budget_reservation_id == start.budget_reservation_id =>
            {
                native_child_spawn_admission_id
            }
            _ => return Err(PiExecutionError::UnexpectedKernelEvent),
        };

        #[cfg(feature = "test-support")]
        if let Some(callback) = self.after_spawn_admission_for_test.take() {
            callback(store, start.operating_cycle_id);
        }

        let spawned = match self.supervisor.spawn_native(start.spawn_request.clone()) {
            Ok(facts) => facts,
            Err(spawn_error) => {
                if let Some(reason) = proven_not_spawned_reason(&spawn_error) {
                    let current_generation = store
                        .current_operating_cycle_admission_generation(start.operating_cycle_id)?;
                    execute_kernel_command(
                        store,
                        &start.operation,
                        PiExecutionCommand::RecordNotSpawned,
                        Capability::RecordNativeChildNotSpawned,
                        ExpectedGeneration::Exact(current_generation),
                        CommandBody::RecordNativeChildNotSpawned {
                            native_child_spawn_admission_id: admission_id,
                            reason,
                        },
                    )?;
                }
                return Err(PiExecutionError::Supervision(spawn_error));
            }
        };
        let (child_identity, direct_child_pid, process_group_id) = match (
            kernel_child_identity(&spawned.child_process_id),
            kernel_child_pid(spawned.host_process_id.value()),
            kernel_process_group_id(spawned.process_group_id.value()),
        ) {
            (Ok(child_identity), Ok(direct_child_pid), Ok(process_group_id)) => {
                (child_identity, direct_child_pid, process_group_id)
            }
            (child_identity, direct_child_pid, process_group_id) => {
                let failure = match child_identity {
                    Err(error) => error,
                    Ok(_) => match direct_child_pid {
                        Err(error) => error,
                        Ok(_) => match process_group_id {
                            Err(error) => error,
                            Ok(_) => unreachable!("successful conversion tuple was matched above"),
                        },
                    },
                };
                return Ok(self.unresolved_task_attempt_registration(
                    spawned.child_process_id,
                    admission_id,
                    failure,
                ));
            }
        };
        let registration_generation =
            match store.current_operating_cycle_admission_generation(start.operating_cycle_id) {
                Ok(generation) => generation,
                Err(error) => {
                    return Ok(self.unresolved_task_attempt_registration(
                        spawned.child_process_id,
                        admission_id,
                        PiExecutionError::Kernel(error),
                    ));
                }
            };
        #[cfg(feature = "test-support")]
        let injected_registration_failure = self.inert_registration_rejection_for_test.take();
        #[cfg(not(feature = "test-support"))]
        let injected_registration_failure: Option<society_kernel::Rejection> = None;
        let registered = match injected_registration_failure {
            Some(rejection) => Err(PiExecutionError::KernelCommandRejected {
                capability: Capability::RecordInertChildSpawn,
                rejection,
            }),
            None => execute_kernel_command(
                store,
                &start.operation,
                PiExecutionCommand::RecordInertSpawn,
                Capability::RecordInertChildSpawn,
                ExpectedGeneration::Exact(registration_generation),
                CommandBody::RecordInertChildSpawn {
                    native_child_spawn_admission_id: admission_id,
                    child_identity,
                    direct_child_pid,
                    process_group_id,
                },
            ),
        };
        let registered = match registered {
            Ok(event) => event,
            Err(error) => {
                return Ok(self.unresolved_task_attempt_registration(
                    spawned.child_process_id,
                    admission_id,
                    error,
                ));
            }
        };
        let child_process_id = match registered {
            EventBody::InertPiChildSpawnRecorded {
                native_child_id: child_process_id,
                native_child_spawn_admission_id,
            } if native_child_spawn_admission_id == admission_id => child_process_id,
            _ => {
                return Ok(self.unresolved_task_attempt_registration(
                    spawned.child_process_id,
                    admission_id,
                    PiExecutionError::UnexpectedKernelEvent,
                ));
            }
        };
        let mut child = TaskAttemptPiExecutionChild {
            operation: start.operation,
            supervised_child_id: spawned.child_process_id,
            child_process_id,
            native_child_spawn_admission_id: admission_id,
            actor_attempt_id: start.actor_attempt_id,
            workspace_directory: start.spawn_request.workspace.directory().clone(),
            session_directory: start.spawn_request.create_session.session_directory.clone(),
            pi_session_identity,
            spawn_nonce,
            expected_generation: registration_generation,
            create_correlation,
            create_request_digest,
            phase: TaskAttemptPiExecutionPhase::SpawnRegistered,
        };
        match self
            .supervisor
            .finish_inert_setup(&child.supervised_child_id, MonotonicTick::ZERO)
        {
            Ok(()) => Ok(TaskAttemptPiSpawnRegistration::Ready(child)),
            Err(SupervisionError::PostSpawnSetup(failure)) => {
                child.phase = TaskAttemptPiExecutionPhase::PostSpawnSetupContained;
                Ok(TaskAttemptPiSpawnRegistration::PostSpawnSetupContained { child, failure })
            }
            Err(error) => {
                child.phase = TaskAttemptPiExecutionPhase::BoundaryContainmentRequired;
                self.contain(&child.supervised_child_id, MonotonicTick::ZERO);
                Ok(
                    TaskAttemptPiSpawnRegistration::RegisteredBoundaryContained {
                        child,
                        failure: error,
                    },
                )
            }
        }
    }

    fn unresolved_task_attempt_registration(
        &mut self,
        supervised_child_id: SupervisedChildId,
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
        failure: PiExecutionError,
    ) -> TaskAttemptPiSpawnRegistration {
        self.contain(&supervised_child_id, MonotonicTick::ZERO);
        TaskAttemptPiSpawnRegistration::RegistrationUnresolved {
            child: Box::new(UnregisteredPiChild {
                supervised_child_id,
                native_child_spawn_admission_id,
                phase: UnregisteredPiChildPhase::ContainmentRequired,
                transient_completion: None,
            }),
            failure,
        }
    }

    pub(crate) fn drive_task_attempt_boundary_containment(
        &mut self,
        child: &TaskAttemptPiExecutionChild,
        now: MonotonicTick,
    ) -> Result<(), PiExecutionError> {
        if !matches!(
            child.phase,
            TaskAttemptPiExecutionPhase::PostSpawnSetupContained
                | TaskAttemptPiExecutionPhase::BoundaryContainmentRequired
        ) {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        self.supervisor
            .drive_cancellation_without_reap(&child.supervised_child_id, now)
            .map_err(PiExecutionError::Supervision)?;
        Ok(())
    }

    pub(crate) fn observe_task_attempt_adapter_ready(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::SpawnRegistered {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let facts = match self.supervisor.observe_adapter_ready_at(
            &child.supervised_child_id,
            now,
            deadline,
        ) {
            Ok(None) => return Ok(false),
            Ok(Some(facts)) => facts,
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        if verify_task_attempt_adapter_facts(child, &facts).is_err() {
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(PiExecutionError::AdapterFactMismatch);
        }
        if let Err(error) = execute_kernel_command(
            store,
            &child.operation,
            PiExecutionCommand::RecordAdapterReady,
            Capability::RecordPiAdapterReady,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiAdapterReady {
                native_child_id: child.child_process_id,
                pi_session_identity: child.pi_session_identity.clone(),
                spawn_nonce: child.spawn_nonce.clone(),
            },
        ) {
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(error);
        }
        child.phase = TaskAttemptPiExecutionPhase::AdapterReadyRecorded;
        Ok(true)
    }

    pub(crate) fn authorize_and_begin_task_attempt_create(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::AdapterReadyRecorded {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let mut gate = KernelCreateAuthorizationGate::new(
            store,
            &child.operation,
            child.child_process_id,
            child.expected_generation,
            &child.create_correlation,
            child.create_request_digest,
        );
        let progress = self.supervisor.send_create_session(
            &child.supervised_child_id,
            &mut gate,
            now,
            deadline,
        );
        if let Err(error) = gate.finish() {
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(error);
        }
        child.phase = TaskAttemptPiExecutionPhase::CreateAuthorized;
        let progress = match progress {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        if progress == ControlWriteProgress::Delivered {
            self.record_task_attempt_create_delivery(store, child, now)?;
        }
        Ok(progress)
    }

    pub(crate) fn drive_task_attempt_create_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::CreateAuthorized {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let progress = match self
            .supervisor
            .drive_control_write(&child.supervised_child_id, now)
        {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        if progress == ControlWriteProgress::Delivered {
            self.record_task_attempt_create_delivery(store, child, now)?;
        }
        Ok(progress)
    }

    /// Records the ordinary Pi SessionReady fact for a TaskAttempt. A task
    /// has no Office-ready transition: once the peer handshake is accepted
    /// and the direct child is still live, the scheduler owns the next actor
    /// obligation transition.
    pub(crate) fn observe_task_attempt_session_ready(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::CreateDelivered {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let session_ready = match self.supervisor.observe_session_ready_at(
            &child.supervised_child_id,
            now,
            deadline,
        ) {
            Ok(ready) => ready,
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        if !session_ready {
            return Ok(false);
        }
        if self
            .supervisor
            .poll_direct_child_reap_at(&child.supervised_child_id)
            .map_err(PiExecutionError::Supervision)?
            .is_some()
        {
            child.phase = TaskAttemptPiExecutionPhase::BoundaryContainmentRequired;
            return Err(PiExecutionError::ExitedBeforeSessionReady);
        }
        let event = execute_kernel_command(
            store,
            &child.operation,
            PiExecutionCommand::RecordSessionReady,
            Capability::RecordPiSessionReady,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiSessionReady {
                native_child_id: child.child_process_id,
                pi_session_identity: child.pi_session_identity.clone(),
            },
        );
        match event {
            Ok(EventBody::PiSessionReadyRecorded {
                native_child_id, ..
            }) if native_child_id == child.child_process_id => {
                child.phase = TaskAttemptPiExecutionPhase::SessionReadyRecorded;
                Ok(true)
            }
            Ok(_) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    fn record_task_attempt_create_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        now: MonotonicTick,
    ) -> Result<(), PiExecutionError> {
        let event = execute_kernel_command(
            store,
            &child.operation,
            PiExecutionCommand::RecordCreateDelivery,
            Capability::RecordPiCreateSessionDelivery,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiCreateSessionDelivery {
                native_child_id: child.child_process_id,
                correlation_identity: child.create_correlation.clone(),
                create_request_digest: child.create_request_digest,
            },
        );
        if let Err(error) = event {
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(error);
        }
        child.phase = TaskAttemptPiExecutionPhase::CreateDelivered;
        Ok(())
    }

    /// Admits the one actor-local `TaskAssignment` Prompt before its JSONL
    /// frame enters the child pipe. A later retried operation retains the
    /// same command slots and exact content digest; it cannot turn a fresh
    /// task prompt into an implicit continuation of a replaced actor.
    pub(crate) fn authorize_and_begin_task_attempt_prompt(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        start: TaskAttemptPiPromptStart,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<(TaskAttemptPiPrompt, ControlWriteProgress), PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::SessionReadyRecorded {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let boundary_correlation = CorrelationIdentity::parse(start.correlation_identity.as_str())
            .map_err(|_| PiExecutionError::IdentityConversion)?;
        let authorized = execute_task_attempt_prompt_command(
            store,
            &start.operation,
            PiTaskAttemptPromptCommand::AuthorizePrompt,
            Capability::AuthorizePiTaskAttemptPrompt,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::AuthorizePiTaskAttemptPrompt {
                actor_attempt_id: child.actor_attempt_id,
                correlation_identity: start.correlation_identity.clone(),
                prompt_content_object_id: start.prompt_content_object_id,
                prompt_digest: start.prompt.digest,
                frontier_event_id: start.frontier_event_id,
            },
        )?;
        match authorized {
            EventBody::PiTaskAttemptPromptAuthorized {
                actor_attempt_id,
                native_child_id,
                correlation_identity,
                ..
            } if actor_attempt_id == child.actor_attempt_id
                && native_child_id == child.child_process_id
                && correlation_identity == start.correlation_identity => {}
            _ => {
                self.begin_task_attempt_boundary_containment(child, now);
                return Err(PiExecutionError::UnexpectedKernelEvent);
            }
        }
        let mut prompt = TaskAttemptPiPrompt {
            operation: start.operation,
            correlation_identity: start.correlation_identity,
            prompt_digest: start.prompt.digest,
            phase: TaskAttemptPiPromptPhase::PromptDeliveryPending,
            accepted_sequence: None,
            agent_settled_sequence: None,
            latest_known_accounting_sequence: None,
            final_accounting_sequence: None,
        };
        let progress = match self.supervisor.send_prompt(
            &child.supervised_child_id,
            boundary_correlation,
            PromptPayload {
                purpose: PromptPurpose::TaskAssignment,
                text: start.prompt.text,
            },
            now,
            deadline,
        ) {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        child.phase = match progress {
            ControlWriteProgress::Pending => TaskAttemptPiExecutionPhase::TaskPromptDeliveryPending,
            ControlWriteProgress::Delivered => TaskAttemptPiExecutionPhase::TaskPromptActive,
        };
        if progress == ControlWriteProgress::Delivered {
            self.record_task_attempt_prompt_delivery(store, child, &mut prompt, now)?;
        }
        Ok((prompt, progress))
    }

    /// Drains only the suffix of an already-authorized task Prompt. Until the
    /// entire frame reaches the host, no output may be attributed to it.
    pub(crate) fn drive_task_attempt_prompt_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        prompt: &mut TaskAttemptPiPrompt,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::TaskPromptDeliveryPending
            || prompt.phase != TaskAttemptPiPromptPhase::PromptDeliveryPending
        {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let progress = self
            .supervisor
            .drive_control_write(&child.supervised_child_id, now)
            .map_err(|error| {
                self.begin_task_attempt_boundary_containment(child, now);
                PiExecutionError::Supervision(error)
            })?;
        if progress == ControlWriteProgress::Delivered {
            child.phase = TaskAttemptPiExecutionPhase::TaskPromptActive;
            self.record_task_attempt_prompt_delivery(store, child, prompt, now)?;
        }
        Ok(progress)
    }

    /// Projects one strict-schema peer frame into the named task receipt
    /// chain. Forum calls remain a separate explicit output because their
    /// application transition must commit before the daemon returns a tool
    /// result to this actor.
    pub(crate) fn observe_task_attempt_prompt_output(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        prompt: &mut TaskAttemptPiPrompt,
        now: MonotonicTick,
    ) -> Result<Option<TaskAttemptPiPromptOutput>, PiExecutionError> {
        if !matches!(
            child.phase,
            TaskAttemptPiExecutionPhase::TaskPromptActive
                | TaskAttemptPiExecutionPhase::TaskPromptTerminalBlocked
        ) {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let output = match self
            .supervisor
            .observe_live_output_at(&child.supervised_child_id, now)
        {
            Ok(None) => return Ok(None),
            Ok(Some(output)) => output,
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        self.project_task_attempt_prompt_output(store, child, prompt, output, now)
            .map(Some)
    }

    /// Sends one already-committed Forum receipt to a task actor. This is the
    /// same bounded SDK control surface as an Office turn, but the task child
    /// and task prompt phases stay disjoint so a tool result cannot revive an
    /// actor after its terminal receipt.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn send_task_attempt_forum_tool_result(
        &mut self,
        child: &mut TaskAttemptPiExecutionChild,
        prompt: &TaskAttemptPiPrompt,
        tool_call_identity: society_pi::ToolCallIdentity,
        result: society_pi::SdkJsonValue,
        is_error: bool,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::TaskPromptActive
            || prompt.phase != TaskAttemptPiPromptPhase::AwaitingTerminalEvidence
        {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let result_correlation_identity = CorrelationIdentity::parse(format!(
            "forum-result-{}",
            blake3::hash(tool_call_identity.as_str().as_bytes()).to_hex()
        ))
        .map_err(|_| PiExecutionError::IdentityConversion)?;
        self.supervisor
            .send_forum_tool_result(
                &child.supervised_child_id,
                result_correlation_identity,
                tool_call_identity,
                result,
                is_error,
                now,
                deadline,
            )
            .map_err(|error| {
                self.begin_task_attempt_boundary_containment(child, now);
                PiExecutionError::Supervision(error)
            })
    }

    pub(crate) fn drive_task_attempt_forum_tool_result_delivery(
        &mut self,
        child: &mut TaskAttemptPiExecutionChild,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::TaskPromptActive {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        self.supervisor
            .drive_control_write(&child.supervised_child_id, now)
            .map_err(|error| {
                self.begin_task_attempt_boundary_containment(child, now);
                PiExecutionError::Supervision(error)
            })
    }

    fn record_task_attempt_prompt_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        prompt: &mut TaskAttemptPiPrompt,
        now: MonotonicTick,
    ) -> Result<(), PiExecutionError> {
        let event = execute_task_attempt_prompt_command(
            store,
            &prompt.operation,
            PiTaskAttemptPromptCommand::RecordPromptDelivery,
            Capability::RecordPiTaskAttemptPromptDelivery,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiTaskAttemptPromptDelivery {
                actor_attempt_id: child.actor_attempt_id,
                correlation_identity: prompt.correlation_identity.clone(),
                prompt_digest: prompt.prompt_digest,
            },
        );
        match event {
            Ok(EventBody::PiTaskAttemptPromptDelivered {
                actor_attempt_id,
                correlation_identity,
            }) if actor_attempt_id == child.actor_attempt_id
                && correlation_identity == prompt.correlation_identity =>
            {
                prompt.phase = TaskAttemptPiPromptPhase::AwaitingPromptAcceptance;
                Ok(())
            }
            Ok(_) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    fn project_task_attempt_prompt_output(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        prompt: &mut TaskAttemptPiPrompt,
        output: SealedDecodedPeerFrame,
        now: MonotonicTick,
    ) -> Result<TaskAttemptPiPromptOutput, PiExecutionError> {
        let sequence = kernel_protocol_sequence(output.frame().sequence)?;
        let expected_correlation = CorrelationIdentity::parse(prompt.correlation_identity.as_str())
            .map_err(|_| PiExecutionError::IdentityConversion)?;
        if output.frame().correlation_identity.as_ref() != Some(&expected_correlation) {
            if output.peer_became_fatal() {
                self.begin_task_attempt_boundary_containment(child, now);
            }
            return Ok(TaskAttemptPiPromptOutput::ControlInterleaving);
        }
        if let PeerFrameValidation::Rejected(error) = output.validation() {
            if matches!(
                (&output.frame().event, error),
                (
                    OutboundEvent::Settled { .. },
                    society_pi::PeerError::MissingTerminalEvidence
                )
            ) && prompt.final_accounting_sequence.is_none()
            {
                return self.record_task_attempt_usage_failure(
                    store,
                    child,
                    prompt,
                    sequence,
                    PiTaskAttemptUsageFailure::Unknown(
                        PiTaskAttemptUsageUnknownReason::MissingFinalUsageSnapshot,
                    ),
                    now,
                );
            }
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(PiExecutionError::PeerFatalWithoutAccountingFact);
        }
        match (&output.frame().event, output.observation()) {
            (
                OutboundEvent::CommandResult(CommandResult::Accepted {
                    command: CommandName::Prompt,
                    ..
                }),
                None,
            ) => {
                if prompt.phase != TaskAttemptPiPromptPhase::AwaitingPromptAcceptance {
                    self.begin_task_attempt_boundary_containment(child, now);
                    return Err(PiExecutionError::PromptEvidenceOrder);
                }
                let event = execute_task_attempt_prompt_command(
                    store,
                    &prompt.operation,
                    PiTaskAttemptPromptCommand::RecordPromptAccepted,
                    Capability::RecordPiTaskAttemptPromptAccepted,
                    ExpectedGeneration::Exact(child.expected_generation),
                    CommandBody::RecordPiTaskAttemptPromptAccepted {
                        actor_attempt_id: child.actor_attempt_id,
                        correlation_identity: prompt.correlation_identity.clone(),
                        command_result_sequence: sequence,
                    },
                );
                match event {
                    Ok(EventBody::PiTaskAttemptPromptAccepted {
                        actor_attempt_id,
                        correlation_identity,
                        command_result_sequence,
                    }) if actor_attempt_id == child.actor_attempt_id
                        && correlation_identity == prompt.correlation_identity
                        && command_result_sequence == sequence =>
                    {
                        prompt.accepted_sequence = Some(sequence);
                        prompt.phase = TaskAttemptPiPromptPhase::AwaitingTerminalEvidence;
                        Ok(TaskAttemptPiPromptOutput::PromptAccepted)
                    }
                    Ok(_) => {
                        self.begin_task_attempt_boundary_containment(child, now);
                        Err(PiExecutionError::UnexpectedKernelEvent)
                    }
                    Err(error) => {
                        self.begin_task_attempt_boundary_containment(child, now);
                        Err(error)
                    }
                }
            }
            (
                OutboundEvent::CommandResult(CommandResult::Rejected {
                    command: CommandName::Prompt,
                    ..
                }),
                None,
            ) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::PromptRejectedByHost)
            }
            (
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentSettled,
                },
                None,
            ) => {
                if prompt.phase != TaskAttemptPiPromptPhase::AwaitingTerminalEvidence
                    || prompt
                        .accepted_sequence
                        .is_none_or(|accepted| sequence <= accepted)
                {
                    self.begin_task_attempt_boundary_containment(child, now);
                    return Err(PiExecutionError::PromptEvidenceOrder);
                }
                prompt.agent_settled_sequence = Some(sequence);
                Ok(TaskAttemptPiPromptOutput::ControlInterleaving)
            }
            (
                OutboundEvent::ForumToolCall {
                    tool_call_identity,
                    tool_name,
                    args,
                },
                Some(society_pi::PeerObservation::ForumToolCall {
                    correlation_identity,
                    tool_call_identity: observed_tool_call_identity,
                    tool_name: observed_tool_name,
                    args: observed_args,
                }),
            ) if correlation_identity.as_str() == prompt.correlation_identity.as_str()
                && tool_call_identity == observed_tool_call_identity
                && tool_name == observed_tool_name
                && society_pi::sdk_json_values_equal(args, observed_args) =>
            {
                if prompt.phase != TaskAttemptPiPromptPhase::AwaitingTerminalEvidence {
                    self.begin_task_attempt_boundary_containment(child, now);
                    return Err(PiExecutionError::PromptEvidenceOrder);
                }
                Ok(TaskAttemptPiPromptOutput::ForumToolCall {
                    correlation_identity: correlation_identity.clone(),
                    tool_call_identity: tool_call_identity.clone(),
                    tool_name: *tool_name,
                    args: args.clone(),
                })
            }
            (
                OutboundEvent::UsageSnapshot {
                    usage: UsageObservation::Known(totals),
                },
                _,
            ) => {
                if prompt
                    .accepted_sequence
                    .is_none_or(|accepted| sequence <= accepted)
                {
                    self.begin_task_attempt_boundary_containment(child, now);
                    return Err(PiExecutionError::PromptEvidenceOrder);
                }
                let usage = kernel_cumulative_usage(totals)?;
                let event = execute_task_attempt_prompt_command(
                    store,
                    &prompt.operation,
                    PiTaskAttemptPromptCommand::RecordKnownUsage { sequence },
                    Capability::RecordPiTaskAttemptUsage,
                    ExpectedGeneration::Exact(child.expected_generation),
                    CommandBody::RecordPiTaskAttemptUsage {
                        actor_attempt_id: child.actor_attempt_id,
                        correlation_identity: prompt.correlation_identity.clone(),
                        protocol_sequence: sequence,
                        usage,
                    },
                );
                match event {
                    Ok(EventBody::PiTaskAttemptUsageRecorded {
                        actor_attempt_id,
                        protocol_sequence,
                        ..
                    }) if actor_attempt_id == child.actor_attempt_id
                        && protocol_sequence == sequence =>
                    {
                        prompt.latest_known_accounting_sequence = Some(sequence);
                        if prompt
                            .agent_settled_sequence
                            .is_some_and(|settled| sequence > settled)
                        {
                            prompt.final_accounting_sequence = Some(sequence);
                        }
                        Ok(TaskAttemptPiPromptOutput::KnownUsageRecorded)
                    }
                    Ok(_) => {
                        self.begin_task_attempt_boundary_containment(child, now);
                        Err(PiExecutionError::UnexpectedKernelEvent)
                    }
                    Err(error) => {
                        self.begin_task_attempt_boundary_containment(child, now);
                        Err(error)
                    }
                }
            }
            (
                OutboundEvent::UsageSnapshot {
                    usage: UsageObservation::Unavailable(reason),
                },
                Some(society_pi::PeerObservation::UsageUnavailable { reason: observed }),
            ) if reason == observed => self.record_task_attempt_usage_failure(
                store,
                child,
                prompt,
                sequence,
                kernel_task_attempt_usage_failure(*reason),
                now,
            ),
            (
                OutboundEvent::Settled {
                    classification,
                    final_assistant_outcome,
                },
                Some(society_pi::PeerObservation::TurnSettled(receipt)),
            ) => {
                let peer_became_fatal = output.peer_became_fatal();
                let result = self.record_task_attempt_terminal(
                    store,
                    child,
                    prompt,
                    sequence,
                    *classification,
                    final_assistant_outcome,
                    receipt,
                    now,
                );
                if peer_became_fatal {
                    self.begin_task_attempt_boundary_containment(child, now);
                }
                result
            }
            (OutboundEvent::Fatal { .. }, _) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::PeerFatalWithoutAccountingFact)
            }
            _ => {
                if output.peer_became_fatal() {
                    self.begin_task_attempt_boundary_containment(child, now);
                    return Err(PiExecutionError::PeerFatalWithoutAccountingFact);
                }
                Ok(TaskAttemptPiPromptOutput::ControlInterleaving)
            }
        }
    }

    fn record_task_attempt_usage_failure(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        prompt: &mut TaskAttemptPiPrompt,
        protocol_sequence: PiProtocolSequence,
        failure: PiTaskAttemptUsageFailure,
        now: MonotonicTick,
    ) -> Result<TaskAttemptPiPromptOutput, PiExecutionError> {
        let missing_final_usage = matches!(
            failure,
            PiTaskAttemptUsageFailure::Unknown(
                PiTaskAttemptUsageUnknownReason::MissingFinalUsageSnapshot
            )
        );
        if prompt.phase != TaskAttemptPiPromptPhase::AwaitingTerminalEvidence
            || prompt
                .accepted_sequence
                .is_none_or(|accepted| protocol_sequence <= accepted)
            || (missing_final_usage
                && prompt
                    .agent_settled_sequence
                    .is_none_or(|settled| protocol_sequence <= settled))
        {
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(PiExecutionError::PromptEvidenceOrder);
        }
        let event = execute_task_attempt_prompt_command(
            store,
            &prompt.operation,
            PiTaskAttemptPromptCommand::RecordUsageFailure {
                sequence: protocol_sequence,
            },
            Capability::RecordPiTaskAttemptUsageFailure,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiTaskAttemptUsageFailure {
                actor_attempt_id: child.actor_attempt_id,
                correlation_identity: prompt.correlation_identity.clone(),
                protocol_sequence,
                failure,
            },
        );
        match event {
            Ok(EventBody::PiTaskAttemptUsageFrozen {
                actor_attempt_id,
                failure: observed_failure,
                ..
            }) if actor_attempt_id == child.actor_attempt_id && observed_failure == failure => {
                prompt.final_accounting_sequence = Some(protocol_sequence);
                prompt.phase = TaskAttemptPiPromptPhase::UsageFrozen;
                child.phase = TaskAttemptPiExecutionPhase::TaskPromptTerminalBlocked;
                self.begin_task_attempt_boundary_containment(child, now);
                Ok(TaskAttemptPiPromptOutput::UsageFrozen)
            }
            Ok(_) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn record_task_attempt_terminal(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        prompt: &mut TaskAttemptPiPrompt,
        settled_sequence: PiProtocolSequence,
        classification: SettledClassification,
        final_assistant_outcome: &FinalAssistantOutcome,
        receipt: &society_pi::TurnReceipt,
        now: MonotonicTick,
    ) -> Result<TaskAttemptPiPromptOutput, PiExecutionError> {
        if prompt.phase != TaskAttemptPiPromptPhase::AwaitingTerminalEvidence
            || receipt.correlation_identity.as_str() != prompt.correlation_identity.as_str()
            || kernel_task_attempt_disposition(receipt.disposition)?
                != kernel_task_attempt_disposition_from_settled(
                    classification,
                    final_assistant_outcome,
                )?
            || kernel_task_attempt_assistant_outcome(&receipt.final_assistant_outcome)?
                != kernel_task_attempt_assistant_outcome(final_assistant_outcome)?
        {
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(PiExecutionError::PromptEvidenceOrder);
        }
        let disposition = kernel_task_attempt_disposition(receipt.disposition)?;
        let assistant_outcome =
            kernel_task_attempt_assistant_outcome(&receipt.final_assistant_outcome)?;
        let terminal_evidence = match assistant_outcome {
            PiTaskAttemptAssistantOutcome::ObservedStop
            | PiTaskAttemptAssistantOutcome::ObservedLength
            | PiTaskAttemptAssistantOutcome::ObservedError
            | PiTaskAttemptAssistantOutcome::ObservedAborted => {
                PiTaskAttemptTerminalEvidence::ObservedAssistant {
                    agent_settled_sequence: prompt
                        .agent_settled_sequence
                        .ok_or(PiExecutionError::PromptTerminalEvidenceMissing)?,
                    final_accounting_sequence: prompt
                        .final_accounting_sequence
                        .ok_or(PiExecutionError::PromptTerminalEvidenceMissing)?,
                }
            }
            PiTaskAttemptAssistantOutcome::SdkPromiseRejected
            | PiTaskAttemptAssistantOutcome::MissingFinalAssistantOutcome => {
                PiTaskAttemptTerminalEvidence::UnavailableAssistant {
                    final_known_usage_sequence: prompt
                        .latest_known_accounting_sequence
                        .ok_or(PiExecutionError::PromptTerminalEvidenceMissing)?,
                }
            }
        };
        if terminal_evidence
            .final_accounting_sequence()
            .value()
            .checked_add(1)
            != Some(settled_sequence.value())
        {
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(PiExecutionError::PromptEvidenceOrder);
        }
        let terminal = execute_task_attempt_prompt_command(
            store,
            &prompt.operation,
            PiTaskAttemptPromptCommand::RecordTerminal,
            Capability::RecordPiTaskAttemptTerminal,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiTaskAttemptTerminal {
                actor_attempt_id: child.actor_attempt_id,
                correlation_identity: prompt.correlation_identity.clone(),
                terminal_evidence,
                settled_sequence,
                disposition,
                assistant_outcome,
                transcript_disposition:
                    PiTaskAttemptTranscriptDisposition::DeferredUntilTaskAttemptSessionDispose,
            },
        );
        match terminal {
            Ok(EventBody::PiTaskAttemptTerminalRecorded {
                actor_attempt_id,
                disposition: observed_disposition,
                assistant_outcome: observed_outcome,
                ..
            }) if actor_attempt_id == child.actor_attempt_id
                && observed_disposition == disposition
                && observed_outcome == assistant_outcome =>
            {
                prompt.phase = TaskAttemptPiPromptPhase::TerminalRecorded;
                child.phase = TaskAttemptPiExecutionPhase::TaskPromptTerminalRecorded;
                Ok(TaskAttemptPiPromptOutput::TerminalRecorded)
            }
            Ok(_) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    /// Begins the one close control only after the actor's one task prompt
    /// has a durable terminal receipt. A failed authorization writes no host
    /// bytes, and an accepted authorization freezes the exact child/session
    /// relation before the asynchronous control write begins.
    pub(crate) fn begin_task_attempt_session_dispose(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        start: TaskAttemptPiSessionDisposeStart,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<(TaskAttemptPiSessionDispose, ControlWriteProgress), PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::TaskPromptTerminalRecorded {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let correlation = CorrelationIdentity::parse(start.correlation_identity.as_str())
            .map_err(|_| PiExecutionError::IdentityConversion)?;
        let authorized = execute_task_attempt_session_dispose_command(
            store,
            &start.operation,
            PiTaskAttemptSessionDisposeCommand::Authorize,
            Capability::AuthorizePiTaskAttemptSessionDispose,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::AuthorizePiTaskAttemptSessionDispose {
                actor_attempt_id: child.actor_attempt_id,
                correlation_identity: start.correlation_identity.clone(),
            },
        )?;
        match authorized {
            EventBody::PiTaskAttemptSessionDisposeAuthorized {
                actor_attempt_id,
                native_child_id,
                correlation_identity,
                authorized_generation,
            } if actor_attempt_id == child.actor_attempt_id
                && native_child_id == child.child_process_id
                && correlation_identity == start.correlation_identity
                && authorized_generation == child.expected_generation => {}
            _ => {
                self.begin_task_attempt_boundary_containment(child, now);
                return Err(PiExecutionError::UnexpectedKernelEvent);
            }
        }
        let mut dispose = TaskAttemptPiSessionDispose {
            operation: start.operation,
            correlation_identity: start.correlation_identity,
            expected_generation: child.expected_generation,
            phase: TaskAttemptPiSessionDisposePhase::DeliveryPending,
            accepted_sequence: None,
            final_accounting_sequence: None,
        };
        let progress = match self.supervisor.send_dispose(
            &child.supervised_child_id,
            correlation,
            society_pi::DisposeReason::CycleReconciliation,
            now,
            deadline,
        ) {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        child.phase = match progress {
            ControlWriteProgress::Pending => TaskAttemptPiExecutionPhase::DisposeDeliveryPending,
            ControlWriteProgress::Delivered => TaskAttemptPiExecutionPhase::DisposeRequested,
        };
        if progress == ControlWriteProgress::Delivered {
            self.record_task_attempt_session_dispose_delivery(store, child, &mut dispose, now)?;
        }
        Ok((dispose, progress))
    }

    pub(crate) fn drive_task_attempt_session_dispose_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        dispose: &mut TaskAttemptPiSessionDispose,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::DisposeDeliveryPending
            || dispose.phase != TaskAttemptPiSessionDisposePhase::DeliveryPending
        {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let progress = self
            .supervisor
            .drive_control_write(&child.supervised_child_id, now)
            .map_err(|error| {
                self.begin_task_attempt_boundary_containment(child, now);
                PiExecutionError::Supervision(error)
            })?;
        if progress == ControlWriteProgress::Delivered {
            child.phase = TaskAttemptPiExecutionPhase::DisposeRequested;
            self.record_task_attempt_session_dispose_delivery(store, child, dispose, now)?;
        }
        Ok(progress)
    }

    fn record_task_attempt_session_dispose_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        dispose: &mut TaskAttemptPiSessionDispose,
        now: MonotonicTick,
    ) -> Result<(), PiExecutionError> {
        let event = execute_task_attempt_session_dispose_command(
            store,
            &dispose.operation,
            PiTaskAttemptSessionDisposeCommand::RecordDelivery,
            Capability::RecordPiTaskAttemptSessionDisposeDelivery,
            ExpectedGeneration::Exact(dispose.expected_generation),
            CommandBody::RecordPiTaskAttemptSessionDisposeDelivery {
                actor_attempt_id: child.actor_attempt_id,
                correlation_identity: dispose.correlation_identity.clone(),
            },
        );
        match event {
            Ok(EventBody::PiTaskAttemptSessionDisposeDelivered {
                actor_attempt_id,
                native_child_id,
                correlation_identity,
            }) if actor_attempt_id == child.actor_attempt_id
                && native_child_id == child.child_process_id
                && correlation_identity == dispose.correlation_identity =>
            {
                dispose.phase = TaskAttemptPiSessionDisposePhase::AwaitingAcceptance;
                Ok(())
            }
            Ok(_) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    pub(crate) fn observe_task_attempt_session_dispose_output(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        dispose: &mut TaskAttemptPiSessionDispose,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
        transcript_seal_limit: ContentSealLimit,
    ) -> Result<Option<TaskAttemptPiSessionDisposeOutput>, PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::DisposeRequested
            || !matches!(
                dispose.phase,
                TaskAttemptPiSessionDisposePhase::AwaitingAcceptance
                    | TaskAttemptPiSessionDisposePhase::AwaitingFinalAccounting
                    | TaskAttemptPiSessionDisposePhase::AwaitingDisposed
            )
        {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let output = match self.supervisor.observe_disposal_output_at(
            &child.supervised_child_id,
            now,
            deadline,
        ) {
            Ok(None) => return Ok(None),
            Ok(Some(output)) => output,
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        self.project_task_attempt_session_dispose_output(
            store,
            child,
            dispose,
            output,
            now,
            transcript_seal_limit,
        )
        .map(Some)
    }

    fn project_task_attempt_session_dispose_output(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        dispose: &mut TaskAttemptPiSessionDispose,
        output: SealedDecodedPeerFrame,
        now: MonotonicTick,
        transcript_seal_limit: ContentSealLimit,
    ) -> Result<TaskAttemptPiSessionDisposeOutput, PiExecutionError> {
        let sequence = kernel_protocol_sequence(output.frame().sequence)?;
        let expected_correlation =
            CorrelationIdentity::parse(dispose.correlation_identity.as_str())
                .map_err(|_| PiExecutionError::IdentityConversion)?;
        if output.frame().correlation_identity.as_ref() != Some(&expected_correlation) {
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(PiExecutionError::DisposeEvidenceOrder);
        }
        if let PeerFrameValidation::Rejected(error) = output.validation() {
            if matches!(
                (&output.frame().event, error),
                (
                    OutboundEvent::Disposed { .. },
                    society_pi::PeerError::MissingTerminalEvidence
                )
            ) && dispose.phase == TaskAttemptPiSessionDisposePhase::AwaitingFinalAccounting
            {
                return self.record_task_attempt_session_dispose_usage_failure(
                    store,
                    child,
                    dispose,
                    sequence,
                    PiTaskAttemptUsageFailure::Unknown(
                        PiTaskAttemptUsageUnknownReason::MissingFinalUsageSnapshot,
                    ),
                    now,
                );
            }
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(PiExecutionError::PeerFatalWithoutAccountingFact);
        }
        match (&output.frame().event, output.observation()) {
            (
                OutboundEvent::CommandResult(CommandResult::Accepted {
                    command: CommandName::Dispose,
                    ..
                }),
                None,
            ) => {
                if dispose.phase != TaskAttemptPiSessionDisposePhase::AwaitingAcceptance {
                    self.begin_task_attempt_boundary_containment(child, now);
                    return Err(PiExecutionError::DisposeEvidenceOrder);
                }
                let event = execute_task_attempt_session_dispose_command(
                    store,
                    &dispose.operation,
                    PiTaskAttemptSessionDisposeCommand::RecordAccepted,
                    Capability::RecordPiTaskAttemptSessionDisposeAccepted,
                    ExpectedGeneration::Exact(dispose.expected_generation),
                    CommandBody::RecordPiTaskAttemptSessionDisposeAccepted {
                        actor_attempt_id: child.actor_attempt_id,
                        correlation_identity: dispose.correlation_identity.clone(),
                        command_result_sequence: sequence,
                    },
                );
                match event {
                    Ok(EventBody::PiTaskAttemptSessionDisposeAccepted {
                        actor_attempt_id,
                        correlation_identity,
                        command_result_sequence,
                    }) if actor_attempt_id == child.actor_attempt_id
                        && correlation_identity == dispose.correlation_identity
                        && command_result_sequence == sequence =>
                    {
                        dispose.accepted_sequence = Some(sequence);
                        dispose.phase = TaskAttemptPiSessionDisposePhase::AwaitingFinalAccounting;
                        Ok(TaskAttemptPiSessionDisposeOutput::Accepted)
                    }
                    Ok(_) => {
                        self.begin_task_attempt_boundary_containment(child, now);
                        Err(PiExecutionError::UnexpectedKernelEvent)
                    }
                    Err(error) => {
                        self.begin_task_attempt_boundary_containment(child, now);
                        Err(error)
                    }
                }
            }
            (
                OutboundEvent::CommandResult(CommandResult::Rejected {
                    command: CommandName::Dispose,
                    ..
                }),
                None,
            ) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::DisposeRejectedByHost)
            }
            (
                OutboundEvent::UsageSnapshot {
                    usage: UsageObservation::Known(totals),
                },
                _,
            ) => {
                if dispose.phase != TaskAttemptPiSessionDisposePhase::AwaitingFinalAccounting
                    || dispose.accepted_sequence.is_none_or(|accepted| {
                        accepted.value().checked_add(1) != Some(sequence.value())
                    })
                {
                    self.begin_task_attempt_boundary_containment(child, now);
                    return Err(PiExecutionError::DisposeEvidenceOrder);
                }
                let usage = kernel_cumulative_usage(totals)?;
                let event = execute_task_attempt_session_dispose_command(
                    store,
                    &dispose.operation,
                    PiTaskAttemptSessionDisposeCommand::RecordKnownUsage { sequence },
                    Capability::RecordPiTaskAttemptSessionDisposeUsage,
                    ExpectedGeneration::Exact(dispose.expected_generation),
                    CommandBody::RecordPiTaskAttemptSessionDisposeUsage {
                        actor_attempt_id: child.actor_attempt_id,
                        correlation_identity: dispose.correlation_identity.clone(),
                        protocol_sequence: sequence,
                        usage,
                    },
                );
                match event {
                    Ok(EventBody::PiTaskAttemptSessionDisposeUsageRecorded {
                        actor_attempt_id,
                        protocol_sequence,
                        ..
                    }) if actor_attempt_id == child.actor_attempt_id
                        && protocol_sequence == sequence =>
                    {
                        dispose.final_accounting_sequence = Some(sequence);
                        dispose.phase = TaskAttemptPiSessionDisposePhase::AwaitingDisposed;
                        Ok(TaskAttemptPiSessionDisposeOutput::KnownUsageRecorded)
                    }
                    Ok(_) => {
                        self.begin_task_attempt_boundary_containment(child, now);
                        Err(PiExecutionError::UnexpectedKernelEvent)
                    }
                    Err(error) => {
                        self.begin_task_attempt_boundary_containment(child, now);
                        Err(error)
                    }
                }
            }
            (
                OutboundEvent::UsageSnapshot {
                    usage: UsageObservation::Unavailable(reason),
                },
                Some(society_pi::PeerObservation::UsageUnavailable { reason: observed }),
            ) if reason == observed => self.record_task_attempt_session_dispose_usage_failure(
                store,
                child,
                dispose,
                sequence,
                kernel_task_attempt_usage_failure(*reason),
                now,
            ),
            (
                OutboundEvent::Disposed {
                    transcript_flush_receipt,
                },
                Some(society_pi::PeerObservation::Disposed),
            ) => {
                if dispose.phase != TaskAttemptPiSessionDisposePhase::AwaitingDisposed
                    || dispose
                        .final_accounting_sequence
                        .is_none_or(|usage| usage.value().checked_add(1) != Some(sequence.value()))
                {
                    self.begin_task_attempt_boundary_containment(child, now);
                    return Err(PiExecutionError::DisposeEvidenceOrder);
                }
                let transcript = project_task_attempt_session_transcript(
                    &dispose.operation,
                    &child.workspace_directory,
                    &child.session_directory,
                    transcript_flush_receipt,
                    transcript_seal_limit,
                );
                match transcript {
                    Ok(transcript) => Ok(TaskAttemptPiSessionDisposeOutput::TranscriptReady(
                        Box::new(VerifiedTaskAttemptSessionDisposeTerminal {
                            transcript,
                            disposed_sequence: sequence,
                        }),
                    )),
                    Err(error) => {
                        self.begin_task_attempt_boundary_containment(child, now);
                        Err(error)
                    }
                }
            }
            (OutboundEvent::Fatal { .. }, _) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::PeerFatalWithoutAccountingFact)
            }
            _ => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::DisposeEvidenceOrder)
            }
        }
    }

    fn record_task_attempt_session_dispose_usage_failure(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        dispose: &mut TaskAttemptPiSessionDispose,
        protocol_sequence: PiProtocolSequence,
        failure: PiTaskAttemptUsageFailure,
        now: MonotonicTick,
    ) -> Result<TaskAttemptPiSessionDisposeOutput, PiExecutionError> {
        if dispose.phase != TaskAttemptPiSessionDisposePhase::AwaitingFinalAccounting
            || dispose.accepted_sequence.is_none_or(|accepted| {
                accepted.value().checked_add(1) != Some(protocol_sequence.value())
            })
        {
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(PiExecutionError::DisposeEvidenceOrder);
        }
        let event = execute_task_attempt_session_dispose_command(
            store,
            &dispose.operation,
            PiTaskAttemptSessionDisposeCommand::RecordUsageFailure {
                sequence: protocol_sequence,
            },
            Capability::RecordPiTaskAttemptSessionDisposeUsageFailure,
            ExpectedGeneration::Exact(dispose.expected_generation),
            CommandBody::RecordPiTaskAttemptSessionDisposeUsageFailure {
                actor_attempt_id: child.actor_attempt_id,
                correlation_identity: dispose.correlation_identity.clone(),
                protocol_sequence,
                failure,
            },
        );
        match event {
            Ok(EventBody::PiTaskAttemptSessionDisposeUsageFrozen {
                actor_attempt_id,
                failure: observed_failure,
                ..
            }) if actor_attempt_id == child.actor_attempt_id && observed_failure == failure => {
                dispose.phase = TaskAttemptPiSessionDisposePhase::UsageFrozen;
                self.begin_task_attempt_boundary_containment(child, now);
                Ok(TaskAttemptPiSessionDisposeOutput::UsageFrozen)
            }
            Ok(_) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    /// Commits the task-domain transcript receipt after its materialized bytes
    /// (if any) pass through the daemon's one physical content authority.
    pub(crate) fn record_task_attempt_session_disposed(
        &mut self,
        store: &mut KernelStore,
        child: &mut TaskAttemptPiExecutionChild,
        dispose: &mut TaskAttemptPiSessionDispose,
        terminal: &VerifiedTaskAttemptSessionDisposeTerminal,
        sealed_content: Option<ContentObjectRegistration>,
        now: MonotonicTick,
    ) -> Result<TaskAttemptPiSessionDisposeOutput, PiExecutionError> {
        if child.phase != TaskAttemptPiExecutionPhase::DisposeRequested
            || dispose.phase != TaskAttemptPiSessionDisposePhase::AwaitingDisposed
            || dispose.final_accounting_sequence.is_none_or(|usage| {
                usage.value().checked_add(1) != Some(terminal.disposed_sequence.value())
            })
        {
            self.begin_task_attempt_boundary_containment(child, now);
            return Err(PiExecutionError::DisposeEvidenceOrder);
        }
        let transcript_receipt = terminal
            .transcript
            .task_kernel_receipt_with_content(sealed_content)?;
        let event = execute_task_attempt_session_dispose_command(
            store,
            &dispose.operation,
            PiTaskAttemptSessionDisposeCommand::RecordDisposed,
            Capability::RecordPiTaskAttemptSessionDisposed,
            ExpectedGeneration::Exact(dispose.expected_generation),
            CommandBody::RecordPiTaskAttemptSessionDisposed {
                actor_attempt_id: child.actor_attempt_id,
                correlation_identity: dispose.correlation_identity.clone(),
                disposed_sequence: terminal.disposed_sequence,
                transcript_receipt,
            },
        );
        match event {
            Ok(EventBody::PiTaskAttemptSessionDisposed {
                actor_attempt_id, ..
            }) if actor_attempt_id == child.actor_attempt_id => {
                dispose.phase = TaskAttemptPiSessionDisposePhase::Disposed;
                child.phase = TaskAttemptPiExecutionPhase::Disposed;
                Ok(TaskAttemptPiSessionDisposeOutput::Disposed)
            }
            Ok(_) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_task_attempt_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    fn begin_task_attempt_boundary_containment(
        &mut self,
        child: &mut TaskAttemptPiExecutionChild,
        now: MonotonicTick,
    ) {
        child.phase = TaskAttemptPiExecutionPhase::BoundaryContainmentRequired;
        self.contain(&child.supervised_child_id, now);
    }

    /// Advances the fixed emergency deadlines for a registered child whose
    /// protocol, kernel receipt, or local setup boundary failed. The caller
    /// retains the same child handle and later reconciles its direct
    /// wait/stream receipts; no successor or normal Office action is legal.
    pub(crate) fn drive_boundary_containment(
        &mut self,
        child: &OfficePiExecutionChild,
        now: MonotonicTick,
    ) -> Result<(), PiExecutionError> {
        if !matches!(
            child.phase,
            OfficePiExecutionPhase::PostSpawnSetupContained
                | OfficePiExecutionPhase::BoundaryContainmentRequired
        ) {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        self.supervisor
            .drive_cancellation_without_reap(&child.supervised_child_id, now)
            .map_err(PiExecutionError::Supervision)?;
        Ok(())
    }

    /// Drives physical containment for a child which exists natively but was
    /// never assigned a kernel `NativeChildId`. Its completed receipt is
    /// intentionally retained only as transient local evidence: no signal,
    /// wait, stream-seal, or finalization command can honestly name it.
    pub(crate) fn drive_unregistered_spawn_containment(
        &mut self,
        child: &mut UnregisteredPiChild,
        now: MonotonicTick,
    ) -> Result<bool, PiExecutionError> {
        if child.phase != UnregisteredPiChildPhase::ContainmentRequired {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let receipt = self
            .supervisor
            .drive_cancellation(&child.supervised_child_id, now)
            .map_err(PiExecutionError::Supervision)?;
        let Some(_) = receipt else {
            return Ok(false);
        };
        let completion = self
            .supervisor
            .take_reaped_receipt(&child.supervised_child_id)
            .ok_or(PiExecutionError::ReapReceiptLost)?;
        child.transient_completion = Some(completion);
        child.phase = UnregisteredPiChildPhase::PhysicallyReaped;
        Ok(true)
    }

    /// Observes and persists AdapterReady.  No session can be constructed
    /// while this transition has not committed.
    pub(crate) fn observe_adapter_ready(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::SpawnRegistered {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let facts = match self.supervisor.observe_adapter_ready_at(
            &child.supervised_child_id,
            now,
            deadline,
        ) {
            Ok(None) => return Ok(false),
            Ok(Some(facts)) => facts,
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        if let Err(error) = verify_adapter_facts(child, &facts) {
            self.begin_registered_boundary_containment(child, now);
            return Err(error);
        }
        let event = execute_kernel_command(
            store,
            &child.operation,
            PiExecutionCommand::RecordAdapterReady,
            Capability::RecordPiAdapterReady,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiAdapterReady {
                native_child_id: child.child_process_id,
                pi_session_identity: child.pi_session_identity.clone(),
                spawn_nonce: child.spawn_nonce.clone(),
            },
        );
        if let Err(error) = event {
            self.begin_registered_boundary_containment(child, now);
            return Err(error);
        }
        child.phase = OfficePiExecutionPhase::AdapterReadyRecorded;
        Ok(true)
    }

    /// Commits the final kernel authorization before the first byte of the
    /// CreateSession frame is eligible for a native pipe write.
    pub(crate) fn authorize_and_begin_create(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::AdapterReadyRecorded {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let mut gate = KernelCreateAuthorizationGate::new(
            store,
            &child.operation,
            child.child_process_id,
            child.expected_generation,
            &child.create_correlation,
            child.create_request_digest,
        );
        let progress = self.supervisor.send_create_session(
            &child.supervised_child_id,
            &mut gate,
            now,
            deadline,
        );
        if let Err(error) = gate.finish() {
            self.begin_registered_boundary_containment(child, now);
            return Err(error);
        }
        // The final authorization has committed even when the following
        // nonblocking pipe attempt reports an error. Preserve that durable
        // fact in the closed phase so reconciliation cannot pretend this was
        // merely an AdapterReady child.
        child.phase = OfficePiExecutionPhase::CreateAuthorized;
        let progress = match progress {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        if progress == ControlWriteProgress::Delivered {
            self.record_create_delivery(store, child, now)?;
        }
        Ok(progress)
    }

    /// Drains a previously admitted CreateSession frame.  A later command can
    /// never overtake this one inside `PiSupervisor`.
    pub(crate) fn drive_create_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::CreateAuthorized {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let progress = match self
            .supervisor
            .drive_control_write(&child.supervised_child_id, now)
        {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        if progress == ControlWriteProgress::Delivered {
            self.record_create_delivery(store, child, now)?;
        }
        Ok(progress)
    }

    /// Persists SessionReady only after a nonblocking direct-child poll has
    /// observed that the process has not already exited.  This is an
    /// observation boundary, not an impossible claim of atomic OS/PostgreSQL
    /// liveness across the following transaction.
    pub(crate) fn observe_session_ready(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::CreateDelivered {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let session_ready = match self.supervisor.observe_session_ready_at(
            &child.supervised_child_id,
            now,
            deadline,
        ) {
            Ok(ready) => ready,
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        if !session_ready {
            return Ok(false);
        }
        if self
            .supervisor
            .poll_direct_child_reap_at(&child.supervised_child_id)
            .map_err(PiExecutionError::Supervision)?
            .is_some()
        {
            child.phase = OfficePiExecutionPhase::BoundaryContainmentRequired;
            return Err(PiExecutionError::ExitedBeforeSessionReady);
        }
        let event = execute_kernel_command(
            store,
            &child.operation,
            PiExecutionCommand::RecordSessionReady,
            Capability::RecordPiSessionReady,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiSessionReady {
                native_child_id: child.child_process_id,
                pi_session_identity: child.pi_session_identity.clone(),
            },
        );
        if let Err(error) = event {
            self.begin_registered_boundary_containment(child, now);
            return Err(error);
        }
        child.phase = OfficePiExecutionPhase::SessionReadyRecorded;
        #[cfg(feature = "test-support")]
        if let Some(duration) = self.pause_before_office_ready_liveness_for_test.take() {
            std::thread::sleep(duration);
        }
        if self
            .supervisor
            .poll_direct_child_reap_at(&child.supervised_child_id)
            .map_err(PiExecutionError::Supervision)?
            .is_some()
        {
            // The Pi SessionReady receipt is a real earlier observation, but
            // the process no longer satisfies the separate Office-ready
            // liveness precondition. Its retained wait is reconciled through
            // the all-phase two-step reap path.
            child.phase = OfficePiExecutionPhase::BoundaryContainmentRequired;
            return Err(PiExecutionError::ExitedBeforeOfficeReady);
        }
        let office_ready = execute_kernel_command(
            store,
            &child.operation,
            PiExecutionCommand::RecordOfficeReady,
            Capability::RecordOfficeSessionReady,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordOfficeSessionReady {
                session_id: child.office_session_id,
            },
        );
        match office_ready {
            Ok(EventBody::RootAuthorityOfficeSessionStateChanged {
                session_id,
                state: society_kernel::OfficeSessionState::Ready,
            }) if session_id == child.office_session_id => {
                child.phase = OfficePiExecutionPhase::OfficeReadyRecorded;
            }
            Ok(_) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::UnexpectedKernelEvent);
            }
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(error);
            }
        }
        Ok(true)
    }

    /// Persists the exact M6 prompt authorization before the first Prompt
    /// byte can enter the host pipe. The content object/digest relation is
    /// checked by the kernel and the rendering/digest relation is checked
    /// again locally, so a caller cannot recombine registered bytes with a
    /// different native JSONL prompt.
    pub(crate) fn authorize_and_begin_office_turn_prompt(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        start: OfficePiTurnStart,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<(OfficePiTurn, ControlWriteProgress), PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::OfficeReadyRecorded {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        if start.prompt.digest != KernelDigest::of_bytes(start.prompt.text.as_bytes()) {
            return Err(PiExecutionError::PromptContentDigestMismatch);
        }
        let boundary_correlation = CorrelationIdentity::parse(start.correlation_identity.as_str())
            .map_err(|_| PiExecutionError::IdentityConversion)?;
        let authorized = execute_office_turn_command(
            store,
            &start.operation,
            PiOfficeTurnCommand::AuthorizePrompt,
            Capability::AuthorizePiOfficeTurnPrompt,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::AuthorizePiOfficeTurnPrompt {
                office_turn_id: start.office_turn_id,
                correlation_identity: start.correlation_identity.clone(),
                prompt_content_object_id: start.prompt_content_object_id,
                prompt_digest: start.prompt.digest,
                frontier_event_id: start.frontier_event_id,
            },
        )?;
        match authorized {
            EventBody::PiOfficeTurnPromptAuthorized {
                office_turn_id,
                native_child_id: child_process_id,
                correlation_identity,
                ..
            } if office_turn_id == start.office_turn_id
                && child_process_id == child.child_process_id
                && correlation_identity == start.correlation_identity => {}
            _ => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::UnexpectedKernelEvent);
            }
        }
        let mut turn = OfficePiTurn {
            operation: start.operation,
            office_turn_id: start.office_turn_id,
            correlation_identity: start.correlation_identity,
            prompt_digest: start.prompt.digest,
            phase: OfficePiTurnPhase::PromptDeliveryPending,
            accepted_sequence: None,
            agent_settled_sequence: None,
            latest_known_accounting_sequence: None,
            final_accounting_sequence: None,
        };
        let progress = match self.supervisor.send_prompt(
            &child.supervised_child_id,
            boundary_correlation,
            PromptPayload {
                purpose: PromptPurpose::OfficeTurn,
                text: start.prompt.text,
            },
            now,
            deadline,
        ) {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        child.phase = match progress {
            ControlWriteProgress::Pending => {
                OfficePiExecutionPhase::OfficeTurnPromptDeliveryPending
            }
            ControlWriteProgress::Delivered => OfficePiExecutionPhase::OfficeTurnPromptActive,
        };
        if progress == ControlWriteProgress::Delivered {
            self.record_office_turn_prompt_delivery(store, child, &mut turn, now)?;
        }
        Ok((turn, progress))
    }

    /// Drives a Prompt which was logically admitted and authorized but whose
    /// exact JSONL suffix has not yet drained to the host. No later control
    /// can overtake this frame in `PiSupervisor`.
    pub(crate) fn drive_office_turn_prompt_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        turn: &mut OfficePiTurn,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::OfficeTurnPromptDeliveryPending
            || turn.phase != OfficePiTurnPhase::PromptDeliveryPending
        {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let progress = match self
            .supervisor
            .drive_control_write(&child.supervised_child_id, now)
        {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        if progress == ControlWriteProgress::Delivered {
            child.phase = OfficePiExecutionPhase::OfficeTurnPromptActive;
            self.record_office_turn_prompt_delivery(store, child, turn, now)?;
        }
        Ok(progress)
    }

    /// Delivers one result for a peer-validated Forum call. The caller is
    /// responsible for committing the typed `ReadForum` or
    /// `PublishForumMessage` transition before invoking this method.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn send_office_forum_tool_result(
        &mut self,
        child: &mut OfficePiExecutionChild,
        turn: &OfficePiTurn,
        tool_call_identity: society_pi::ToolCallIdentity,
        result: society_pi::SdkJsonValue,
        is_error: bool,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if !matches!(
            child.phase,
            OfficePiExecutionPhase::OfficeTurnPromptActive
                | OfficePiExecutionPhase::OfficeTurnTerminalBlocked
        ) || !matches!(
            turn.phase,
            OfficePiTurnPhase::AwaitingPromptAcceptance
                | OfficePiTurnPhase::AwaitingTerminalEvidence
        ) {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        // A Forum result is a second inbound command, not a second frame of
        // the Prompt command. It therefore needs its own correlation so the
        // peer's duplicate-command guard cannot confuse the result with the
        // still-active turn. The digest keeps the derived identity bounded
        // even when an SDK emits a long tool-call label.
        let result_correlation_identity = CorrelationIdentity::parse(format!(
            "forum-result-{}",
            blake3::hash(tool_call_identity.as_str().as_bytes()).to_hex()
        ))
        .map_err(|_| PiExecutionError::IdentityConversion)?;
        self.supervisor
            .send_forum_tool_result(
                &child.supervised_child_id,
                result_correlation_identity,
                tool_call_identity,
                result,
                is_error,
                now,
                deadline,
            )
            .map_err(PiExecutionError::Supervision)
    }

    /// Continues a Forum result whose JSONL write was back-pressured.  The
    /// caller must not poll host output until this reaches `Delivered`; that
    /// ordering keeps the actor from issuing a second mutable tool request
    /// while the first result is still only physically partial.
    pub(crate) fn drive_office_forum_tool_result_delivery(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if !matches!(
            child.phase,
            OfficePiExecutionPhase::OfficeTurnPromptActive
                | OfficePiExecutionPhase::OfficeTurnTerminalBlocked
        ) {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        match self
            .supervisor
            .drive_control_write(&child.supervised_child_id, now)
        {
            Ok(progress) => Ok(progress),
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::Supervision(error))
            }
        }
    }

    /// Projects exactly one peer-sealed, schema-decoded stdout frame into the
    /// closed M6 receipt chain. Raw stdout remains transient sealed evidence
    /// in the supervisor; this method names only accepted Prompt, cumulative
    /// usage, typed accounting inability, and final terminal facts.
    pub(crate) fn observe_office_turn_output(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        turn: &mut OfficePiTurn,
        now: MonotonicTick,
    ) -> Result<Option<OfficePiTurnOutput>, PiExecutionError> {
        if !matches!(
            child.phase,
            OfficePiExecutionPhase::OfficeTurnPromptActive
                | OfficePiExecutionPhase::OfficeTurnTerminalBlocked
        ) {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let output = match self
            .supervisor
            .observe_live_output_at(&child.supervised_child_id, now)
        {
            Ok(None) => return Ok(None),
            Ok(Some(output)) => output,
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        self.project_office_turn_output(store, child, turn, output, now)
            .map(Some)
    }

    /// Authorizes the one closing Dispose control before any byte can enter
    /// the host pipe, then begins the nonblocking physical write. The kernel
    /// freezes the exact session/correlation/current-generation relation in
    /// its `AuthorizePiOfficeSessionDispose` command; this prevents a stale
    /// or wrong-cycle caller from physically closing a session and learning
    /// only afterwards that delivery was not durable authority.
    ///
    /// No content writer is involved here: transcript materialization is a
    /// later peer-sealed stdout fact.
    pub(crate) fn begin_office_session_dispose(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        start: OfficePiSessionDisposeStart,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<(OfficePiSessionDispose, ControlWriteProgress), PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::OfficeReadyRecorded {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let correlation = CorrelationIdentity::parse(start.correlation_identity.as_str())
            .map_err(|_| PiExecutionError::IdentityConversion)?;
        let authorized = execute_office_session_dispose_command(
            store,
            &start.operation,
            PiOfficeSessionDisposeCommand::Authorize,
            Capability::AuthorizePiOfficeSessionDispose,
            ExpectedGeneration::Exact(start.expected_generation),
            CommandBody::AuthorizePiOfficeSessionDispose {
                session_id: child.office_session_id,
                correlation_identity: start.correlation_identity.clone(),
            },
        );
        match authorized {
            Ok(EventBody::PiOfficeSessionDisposeAuthorized {
                session_id,
                native_child_id: child_process_id,
                correlation_identity,
                authorized_generation,
            }) if session_id == child.office_session_id
                && child_process_id == child.child_process_id
                && correlation_identity == start.correlation_identity
                && authorized_generation == start.expected_generation => {}
            Ok(_) => {
                // An accepted authorization with another child/session is an
                // internal cross-boundary contradiction. No Dispose byte has
                // been written, but this resident may no longer safely use
                // the live child for Office work.
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::UnexpectedKernelEvent);
            }
            Err(error) => {
                // A rejected/stale authorization has no native side effect.
                // In particular, do not turn a current Office session into a
                // cancellation merely because a caller offered old authority.
                return Err(error);
            }
        }
        let mut dispose = OfficePiSessionDispose {
            operation: start.operation,
            correlation_identity: start.correlation_identity,
            expected_generation: start.expected_generation,
            phase: OfficePiSessionDisposePhase::DeliveryPending,
            accepted_sequence: None,
            final_accounting_sequence: None,
        };
        let progress = match self.supervisor.send_dispose(
            &child.supervised_child_id,
            correlation,
            society_pi::DisposeReason::CycleReconciliation,
            now,
            deadline,
        ) {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        child.phase = match progress {
            ControlWriteProgress::Pending => OfficePiExecutionPhase::DisposeDeliveryPending,
            ControlWriteProgress::Delivered => OfficePiExecutionPhase::DisposeRequested,
        };
        if progress == ControlWriteProgress::Delivered {
            self.record_office_session_dispose_delivery(store, child, &mut dispose, now)?;
        }
        Ok((dispose, progress))
    }

    /// Drains the one already-authorized Dispose suffix. A `Pending` write is
    /// not a durable delivery and remains unable to observe host output.
    pub(crate) fn drive_office_session_dispose_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        dispose: &mut OfficePiSessionDispose,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::DisposeDeliveryPending
            || dispose.phase != OfficePiSessionDisposePhase::DeliveryPending
        {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let progress = match self
            .supervisor
            .drive_control_write(&child.supervised_child_id, now)
        {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        if progress == ControlWriteProgress::Delivered {
            child.phase = OfficePiExecutionPhase::DisposeRequested;
            self.record_office_session_dispose_delivery(store, child, dispose, now)?;
        }
        Ok(progress)
    }

    /// Projects exactly one peer-sealed Dispose stdout frame. Materialized
    /// transcript bytes are returned as a closed in-memory seal request; this
    /// driver never opens the daemon's content object store or fabricates an
    /// object identity.
    pub(crate) fn observe_office_session_dispose_output(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        dispose: &mut OfficePiSessionDispose,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
        transcript_seal_limit: ContentSealLimit,
    ) -> Result<Option<OfficePiSessionDisposeOutput>, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::DisposeRequested
            || !matches!(
                dispose.phase,
                OfficePiSessionDisposePhase::AwaitingAcceptance
                    | OfficePiSessionDisposePhase::AwaitingFinalAccounting
                    | OfficePiSessionDisposePhase::AwaitingDisposed
            )
        {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let output = match self.observe_disposal_output(child, now, deadline) {
            Ok(None) => return Ok(None),
            Ok(Some(output)) => output,
            Err(error) => return Err(error),
        };
        self.project_office_session_dispose_output(
            store,
            child,
            dispose,
            output,
            now,
            transcript_seal_limit,
        )
        .map(Some)
    }

    /// Completes a peer-validated Dispose transcript only after the daemon's
    /// sole physical content writer supplied the global object identity for a
    /// materialized receipt. The unmaterialized arm rejects any supplied
    /// object, making it impossible to invent content for a no-Prompt session.
    pub(crate) fn record_office_session_disposed(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        dispose: &mut OfficePiSessionDispose,
        terminal: &VerifiedPiSessionDisposeTerminal,
        sealed_content: Option<ContentObjectRegistration>,
        now: MonotonicTick,
    ) -> Result<OfficePiSessionDisposeOutput, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::DisposeRequested
            || dispose.phase != OfficePiSessionDisposePhase::AwaitingDisposed
            || dispose.final_accounting_sequence.is_none_or(|usage| {
                usage.value().checked_add(1) != Some(terminal.disposed_sequence.value())
            })
        {
            self.begin_registered_boundary_containment(child, now);
            return Err(PiExecutionError::DisposeEvidenceOrder);
        }
        let transcript_receipt = terminal
            .transcript
            .kernel_receipt_with_content(sealed_content)?;
        let event = execute_office_session_dispose_command(
            store,
            &dispose.operation,
            PiOfficeSessionDisposeCommand::RecordDisposed,
            Capability::RecordPiOfficeSessionDisposed,
            ExpectedGeneration::Exact(dispose.expected_generation),
            CommandBody::RecordPiOfficeSessionDisposed {
                session_id: child.office_session_id,
                correlation_identity: dispose.correlation_identity.clone(),
                disposed_sequence: terminal.disposed_sequence,
                transcript_receipt,
            },
        );
        match event {
            Ok(EventBody::PiOfficeSessionDisposed { session_id, .. })
                if session_id == child.office_session_id =>
            {
                dispose.phase = OfficePiSessionDisposePhase::Disposed;
                child.phase = OfficePiExecutionPhase::Disposed;
                Ok(OfficePiSessionDisposeOutput::Disposed)
            }
            Ok(_) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    /// Records the physical-write boundary only after `PiSupervisor` has
    /// drained the complete JSONL Dispose frame. A logical control admission
    /// or a partial pipe suffix is deliberately not a delivery receipt.
    fn record_office_session_dispose_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        dispose: &mut OfficePiSessionDispose,
        now: MonotonicTick,
    ) -> Result<(), PiExecutionError> {
        let event = execute_office_session_dispose_command(
            store,
            &dispose.operation,
            PiOfficeSessionDisposeCommand::RecordDelivery,
            Capability::RecordPiOfficeSessionDisposeDelivery,
            ExpectedGeneration::Exact(dispose.expected_generation),
            CommandBody::RecordPiOfficeSessionDisposeDelivery {
                session_id: child.office_session_id,
                correlation_identity: dispose.correlation_identity.clone(),
            },
        );
        match event {
            Ok(EventBody::PiOfficeSessionDisposeDelivered {
                session_id,
                native_child_id: child_process_id,
                correlation_identity,
            }) if session_id == child.office_session_id
                && child_process_id == child.child_process_id
                && correlation_identity == dispose.correlation_identity =>
            {
                dispose.phase = OfficePiSessionDisposePhase::AwaitingAcceptance;
                Ok(())
            }
            Ok(_) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                // A physical write without its matching durable receipt is a
                // safety boundary: no later host output can be attributed to
                // this resident's session close.
                self.begin_registered_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    /// Projects a single byte-sealed, strict-schema-decoded closing frame.
    /// The peer's semantic validation remains authoritative for lifecycle
    /// order. The kernel receives only the narrow named facts it can own:
    /// accepted Dispose, one final cumulative usage/failure, and a transcript
    /// candidate whose bytes are still held outside the content store.
    fn project_office_session_dispose_output(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        dispose: &mut OfficePiSessionDispose,
        output: SealedDecodedPeerFrame,
        now: MonotonicTick,
        transcript_seal_limit: ContentSealLimit,
    ) -> Result<OfficePiSessionDisposeOutput, PiExecutionError> {
        let sequence = kernel_protocol_sequence(output.frame().sequence)?;
        let expected_correlation =
            CorrelationIdentity::parse(dispose.correlation_identity.as_str())
                .map_err(|_| PiExecutionError::IdentityConversion)?;
        if output.frame().correlation_identity.as_ref() != Some(&expected_correlation) {
            self.begin_registered_boundary_containment(child, now);
            return Err(PiExecutionError::DisposeEvidenceOrder);
        }

        if let PeerFrameValidation::Rejected(error) = output.validation() {
            // A schema-valid Disposed immediately after acceptance is an
            // observed sequence but lacks the forced final accounting frame.
            // Preserve that exact inability rather than inventing a missing
            // Usage snapshot or accepting the terminal close.
            if matches!(
                (&output.frame().event, error),
                (
                    OutboundEvent::Disposed { .. },
                    society_pi::PeerError::MissingTerminalEvidence
                )
            ) && dispose.phase == OfficePiSessionDisposePhase::AwaitingFinalAccounting
            {
                return self.record_office_session_dispose_usage_failure(
                    store,
                    child,
                    dispose,
                    sequence,
                    PiOfficeTurnUsageFailure::Unknown(
                        PiOfficeTurnUsageUnknownReason::MissingFinalUsageSnapshot,
                    ),
                    now,
                );
            }
            self.begin_registered_boundary_containment(child, now);
            return Err(PiExecutionError::PeerFatalWithoutAccountingFact);
        }

        match (&output.frame().event, output.observation()) {
            (
                OutboundEvent::CommandResult(CommandResult::Accepted {
                    command: CommandName::Dispose,
                    ..
                }),
                None,
            ) => {
                if dispose.phase != OfficePiSessionDisposePhase::AwaitingAcceptance {
                    self.begin_registered_boundary_containment(child, now);
                    return Err(PiExecutionError::DisposeEvidenceOrder);
                }
                let event = execute_office_session_dispose_command(
                    store,
                    &dispose.operation,
                    PiOfficeSessionDisposeCommand::RecordAccepted,
                    Capability::RecordPiOfficeSessionDisposeAccepted,
                    ExpectedGeneration::Exact(dispose.expected_generation),
                    CommandBody::RecordPiOfficeSessionDisposeAccepted {
                        session_id: child.office_session_id,
                        correlation_identity: dispose.correlation_identity.clone(),
                        command_result_sequence: sequence,
                    },
                );
                match event {
                    Ok(EventBody::PiOfficeSessionDisposeAccepted {
                        session_id,
                        correlation_identity,
                        command_result_sequence,
                    }) if session_id == child.office_session_id
                        && correlation_identity == dispose.correlation_identity
                        && command_result_sequence == sequence =>
                    {
                        dispose.accepted_sequence = Some(sequence);
                        dispose.phase = OfficePiSessionDisposePhase::AwaitingFinalAccounting;
                        Ok(OfficePiSessionDisposeOutput::Accepted)
                    }
                    Ok(_) => {
                        self.begin_registered_boundary_containment(child, now);
                        Err(PiExecutionError::UnexpectedKernelEvent)
                    }
                    Err(error) => {
                        self.begin_registered_boundary_containment(child, now);
                        Err(error)
                    }
                }
            }
            (
                OutboundEvent::CommandResult(CommandResult::Rejected {
                    command: CommandName::Dispose,
                    ..
                }),
                None,
            ) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::DisposeRejectedByHost)
            }
            (
                OutboundEvent::UsageSnapshot {
                    usage: UsageObservation::Known(totals),
                },
                // Known totals remain terminal evidence even if the peer's
                // normalized delta is the explicit zero/idempotent delta.
                // The kernel independently verifies full cumulative
                // nondecrease across the session namespace.
                _,
            ) => {
                if dispose.phase != OfficePiSessionDisposePhase::AwaitingFinalAccounting
                    || dispose.accepted_sequence.is_none_or(|accepted| {
                        accepted.value().checked_add(1) != Some(sequence.value())
                    })
                {
                    self.begin_registered_boundary_containment(child, now);
                    return Err(PiExecutionError::DisposeEvidenceOrder);
                }
                let usage = kernel_cumulative_usage(totals)?;
                let event = execute_office_session_dispose_command(
                    store,
                    &dispose.operation,
                    PiOfficeSessionDisposeCommand::RecordKnownUsage { sequence },
                    Capability::RecordPiOfficeSessionDisposeUsage,
                    ExpectedGeneration::Exact(dispose.expected_generation),
                    CommandBody::RecordPiOfficeSessionDisposeUsage {
                        session_id: child.office_session_id,
                        correlation_identity: dispose.correlation_identity.clone(),
                        protocol_sequence: sequence,
                        usage,
                    },
                );
                match event {
                    Ok(EventBody::PiOfficeSessionDisposeUsageRecorded {
                        session_id,
                        protocol_sequence,
                        ..
                    }) if session_id == child.office_session_id
                        && protocol_sequence == sequence =>
                    {
                        dispose.final_accounting_sequence = Some(sequence);
                        dispose.phase = OfficePiSessionDisposePhase::AwaitingDisposed;
                        Ok(OfficePiSessionDisposeOutput::KnownUsageRecorded)
                    }
                    Ok(_) => {
                        self.begin_registered_boundary_containment(child, now);
                        Err(PiExecutionError::UnexpectedKernelEvent)
                    }
                    Err(error) => {
                        self.begin_registered_boundary_containment(child, now);
                        Err(error)
                    }
                }
            }
            (
                OutboundEvent::UsageSnapshot {
                    usage: UsageObservation::Unavailable(reason),
                },
                Some(society_pi::PeerObservation::UsageUnavailable { reason: observed }),
            ) if reason == observed => self.record_office_session_dispose_usage_failure(
                store,
                child,
                dispose,
                sequence,
                kernel_usage_failure(*reason),
                now,
            ),
            (
                OutboundEvent::Disposed {
                    transcript_flush_receipt,
                },
                Some(society_pi::PeerObservation::Disposed),
            ) => {
                if dispose.phase != OfficePiSessionDisposePhase::AwaitingDisposed
                    || dispose
                        .final_accounting_sequence
                        .is_none_or(|usage| usage.value().checked_add(1) != Some(sequence.value()))
                {
                    self.begin_registered_boundary_containment(child, now);
                    return Err(PiExecutionError::DisposeEvidenceOrder);
                }
                let transcript = project_verified_session_transcript(
                    &dispose.operation,
                    &child.workspace_directory,
                    &child.session_directory,
                    transcript_flush_receipt,
                    transcript_seal_limit,
                );
                match transcript {
                    Ok(transcript) => Ok(OfficePiSessionDisposeOutput::TranscriptReady(Box::new(
                        VerifiedPiSessionDisposeTerminal {
                            transcript,
                            disposed_sequence: sequence,
                        },
                    ))),
                    Err(error) => {
                        self.begin_registered_boundary_containment(child, now);
                        Err(error)
                    }
                }
            }
            (OutboundEvent::Fatal { .. }, _) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::PeerFatalWithoutAccountingFact)
            }
            _ => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::DisposeEvidenceOrder)
            }
        }
    }

    /// Records an exact accounting inability at the observed closing sequence,
    /// freezes the parent through the kernel, and starts physical containment.
    /// A frozen Dispose chain has no `Disposed` successor.
    fn record_office_session_dispose_usage_failure(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        dispose: &mut OfficePiSessionDispose,
        protocol_sequence: PiProtocolSequence,
        failure: PiOfficeTurnUsageFailure,
        now: MonotonicTick,
    ) -> Result<OfficePiSessionDisposeOutput, PiExecutionError> {
        if dispose.phase != OfficePiSessionDisposePhase::AwaitingFinalAccounting
            || dispose.accepted_sequence.is_none_or(|accepted| {
                accepted.value().checked_add(1) != Some(protocol_sequence.value())
            })
        {
            self.begin_registered_boundary_containment(child, now);
            return Err(PiExecutionError::DisposeEvidenceOrder);
        }
        let event = execute_office_session_dispose_command(
            store,
            &dispose.operation,
            PiOfficeSessionDisposeCommand::RecordUsageFailure {
                sequence: protocol_sequence,
            },
            Capability::RecordPiOfficeSessionDisposeUsageFailure,
            ExpectedGeneration::Exact(dispose.expected_generation),
            CommandBody::RecordPiOfficeSessionDisposeUsageFailure {
                session_id: child.office_session_id,
                correlation_identity: dispose.correlation_identity.clone(),
                protocol_sequence,
                failure,
            },
        );
        match event {
            Ok(EventBody::PiOfficeSessionDisposeUsageFrozen { session_id, .. })
                if session_id == child.office_session_id =>
            {
                dispose.phase = OfficePiSessionDisposePhase::UsageFrozen;
                self.begin_registered_boundary_containment(child, now);
                Ok(OfficePiSessionDisposeOutput::UsageFrozen)
            }
            Ok(_) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    /// Retired M5 fixture helper. Production has exactly one Office Dispose
    /// path: `begin_office_session_dispose`, whose kernel authorization
    /// precedes every physical control byte. Keep this only for the narrow
    /// pre-M7 process-physics regression fixtures until those are retired.
    #[cfg(test)]
    pub(crate) fn begin_dispose(
        &mut self,
        child: &mut OfficePiExecutionChild,
        correlation: CorrelationIdentity,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::OfficeReadyRecorded {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let progress = match self.supervisor.send_dispose(
            &child.supervised_child_id,
            correlation,
            society_pi::DisposeReason::CycleReconciliation,
            now,
            deadline,
        ) {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        child.phase = match progress {
            ControlWriteProgress::Delivered => OfficePiExecutionPhase::DisposeRequested,
            ControlWriteProgress::Pending => OfficePiExecutionPhase::DisposeDeliveryPending,
        };
        Ok(progress)
    }

    /// Retired M5 fixture helper paired with `begin_dispose` above.
    #[cfg(test)]
    pub(crate) fn drive_dispose_delivery(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::DisposeDeliveryPending {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let progress = match self
            .supervisor
            .drive_control_write(&child.supervised_child_id, now)
        {
            Ok(progress) => progress,
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::Supervision(error));
            }
        };
        if progress == ControlWriteProgress::Delivered {
            child.phase = OfficePiExecutionPhase::DisposeRequested;
        }
        Ok(progress)
    }

    /// Reads one peer-sealed disposal frame without advancing the daemon's
    /// child phase. The forthcoming durable session-dispose chain must commit
    /// the exact accepted/usage/transcript fact before `Disposed` becomes a
    /// daemon-visible terminal state; this native method deliberately keeps
    /// that commit boundary with its caller.
    pub(crate) fn observe_disposal_output(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<Option<SealedDecodedPeerFrame>, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::DisposeRequested {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        match self
            .supervisor
            .observe_disposal_output_at(&child.supervised_child_id, now, deadline)
        {
            Ok(output) => Ok(output),
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::Supervision(error))
            }
        }
    }

    /// Retired M5 fixture helper. Production projects the raw sealed frame
    /// through the M7 receipt chain before it may mark this child disposed.
    #[cfg(test)]
    pub(crate) fn observe_disposed(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, PiExecutionError> {
        let output = self.observe_disposal_output(child, now, deadline)?;
        let disposed = output
            .as_ref()
            .is_some_and(|frame| matches!(&frame.frame().event, OutboundEvent::Disposed { .. }));
        if disposed {
            child.phase = OfficePiExecutionPhase::Disposed;
        }
        Ok(disposed)
    }

    /// Reconciles a direct wait through the M5 ordering: durable direct-child
    /// reap, then (only if due) a distinct lingering-group cleanup signal,
    /// then a later liveness observation and bounded stream sealing. The
    /// process physics never needs a PostgreSQL transaction, and each durable
    /// transition completes before the next OS action.
    pub(crate) fn poll_reap_and_reconcile(
        &mut self,
        store: &mut KernelStore,
        content: &ContentSealingAuthority,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
    ) -> Result<bool, PiExecutionError> {
        if child.phase == OfficePiExecutionPhase::Reconciled {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let Some(direct_reap) = self
            .supervisor
            .poll_direct_child_reap_at(&child.supervised_child_id)
            .map_err(PiExecutionError::Supervision)?
        else {
            return Ok(false);
        };
        if child.phase != OfficePiExecutionPhase::DirectChildReapRecorded
            && child.phase != OfficePiExecutionPhase::LingeringCleanupRecorded
            && child.phase != OfficePiExecutionPhase::AwaitingLingeringGroupAbsence
        {
            self.record_pre_reap_signal_receipts(store, child, &direct_reap)?;
            self.record_direct_child_reap(store, child, &direct_reap)?;
            if direct_reap.prior_signal_receipts.iter().any(|receipt| {
                receipt.delivery == SignalDelivery::GroupInaccessible
                    || receipt.group_liveness_after_attempt
                        == crate::supervision::ProcessGroupLiveness::Inaccessible
            }) {
                // The kernel deliberately makes an inaccessible signal
                // observation terminal containment failure.  Keep the exact
                // already-recorded wait/signal facts; do not attempt a later
                // liveness/finalization transition that would falsely imply
                // renewed supervisory authority.
                return Err(PiExecutionError::AutomaticContainmentInaccessible);
            }
            if direct_reap.group_liveness_after_direct_child_reap
                == crate::supervision::ProcessGroupLiveness::Inaccessible
            {
                // `RecordDirectChildReap` itself has durably classified this
                // child as containment-failed. A group we cannot signal must
                // not be given a fictional lingering-KILL attempt.
                return Err(PiExecutionError::LingeringGroupInaccessible);
            }
            child.phase = OfficePiExecutionPhase::DirectChildReapRecorded;
        }

        if child.phase == OfficePiExecutionPhase::DirectChildReapRecorded {
            if let Some(signal) = self
                .supervisor
                .issue_lingering_group_cleanup(&child.supervised_child_id, now)
                .map_err(PiExecutionError::Supervision)?
            {
                self.record_signal_receipt(store, child, &signal, 2)?;
                match signal.group_liveness_after_attempt {
                    crate::supervision::ProcessGroupLiveness::Present => {
                        // The signal receipt is the only immediate durable
                        // fact. Do not spend the retry-stable later-liveness
                        // command on a Present body: a future Absent result
                        // must remain representable under that command ID.
                        child.phase = OfficePiExecutionPhase::AwaitingLingeringGroupAbsence;
                        return Ok(false);
                    }
                    crate::supervision::ProcessGroupLiveness::Inaccessible => {
                        return Err(PiExecutionError::LingeringGroupInaccessible);
                    }
                    crate::supervision::ProcessGroupLiveness::Absent => {}
                }
            }
            child.phase = OfficePiExecutionPhase::LingeringCleanupRecorded;
        }

        let liveness = self
            .supervisor
            .observe_group_liveness(&child.supervised_child_id)
            .map_err(PiExecutionError::Supervision)?;
        match liveness {
            crate::supervision::ProcessGroupLiveness::Present => {
                if child.phase == OfficePiExecutionPhase::AwaitingLingeringGroupAbsence {
                    return Ok(false);
                }
                // The earlier direct-reap/signal observation was Absent, so
                // a later Present group is a possible PID/PGID reuse rather
                // than a harmless retry. Make it durable exactly once; the
                // kernel classifies the physical identity as containment
                // failed and this bridge must not finalize it.
                self.record_liveness(store, child, liveness)?;
                return Err(PiExecutionError::ProcessGroupIdentityRegressed);
            }
            crate::supervision::ProcessGroupLiveness::Inaccessible => {
                self.record_liveness(store, child, liveness)?;
                return Err(PiExecutionError::LingeringGroupInaccessible);
            }
            crate::supervision::ProcessGroupLiveness::Absent => {
                self.record_liveness(store, child, liveness)?;
            }
        }
        let receipt = self
            .supervisor
            .complete_deferred_reap_at(&child.supervised_child_id)
            .map_err(PiExecutionError::Supervision)?;
        self.seal_and_finalize(store, content, child, &receipt)?;
        self.supervisor
            .take_reaped_receipt(&child.supervised_child_id)
            .ok_or(PiExecutionError::ReapReceiptLost)?;
        child.phase = OfficePiExecutionPhase::Reconciled;
        Ok(true)
    }

    fn record_office_turn_prompt_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        turn: &mut OfficePiTurn,
        now: MonotonicTick,
    ) -> Result<(), PiExecutionError> {
        let event = execute_office_turn_command(
            store,
            &turn.operation,
            PiOfficeTurnCommand::RecordPromptDelivery,
            Capability::RecordPiOfficeTurnPromptDelivery,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiOfficeTurnPromptDelivery {
                office_turn_id: turn.office_turn_id,
                correlation_identity: turn.correlation_identity.clone(),
                prompt_digest: turn.prompt_digest,
            },
        );
        match event {
            Ok(EventBody::PiOfficeTurnPromptDelivered {
                office_turn_id,
                correlation_identity,
            }) if office_turn_id == turn.office_turn_id
                && correlation_identity == turn.correlation_identity =>
            {
                turn.phase = OfficePiTurnPhase::AwaitingPromptAcceptance;
                Ok(())
            }
            Ok(_) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    fn project_office_turn_output(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        turn: &mut OfficePiTurn,
        output: SealedDecodedPeerFrame,
        now: MonotonicTick,
    ) -> Result<OfficePiTurnOutput, PiExecutionError> {
        let sequence = kernel_protocol_sequence(output.frame().sequence)?;
        let expected_correlation = CorrelationIdentity::parse(turn.correlation_identity.as_str())
            .map_err(|_| PiExecutionError::IdentityConversion)?;
        if output.frame().correlation_identity.as_ref() != Some(&expected_correlation) {
            // A peer-valid control result/snapshot may interleave after
            // `agent_settled`. It remains sealed raw stream evidence but it
            // cannot replace this Prompt's final accounting fact.
            if output.peer_became_fatal() {
                self.begin_registered_boundary_containment(child, now);
            }
            return Ok(OfficePiTurnOutput::ControlInterleaving);
        }

        if let PeerFrameValidation::Rejected(error) = output.validation() {
            // A schema-valid `Settled` frame with no persisted final usage is
            // still an exact host sequence. Preserve the closed Unknown
            // accounting outcome at that observed frame rather than inventing
            // an unobserved successor sequence or treating cost as zero. A
            // peer-rejected terminal after an existing final Known snapshot
            // instead remains a protocol containment fact; it cannot be
            // rewritten as an accounting failure.
            if matches!(
                (&output.frame().event, error),
                (
                    OutboundEvent::Settled { .. },
                    society_pi::PeerError::MissingTerminalEvidence
                )
            ) && turn.final_accounting_sequence.is_none()
            {
                return self.record_office_turn_usage_failure(
                    store,
                    child,
                    turn,
                    sequence,
                    PiOfficeTurnUsageFailure::Unknown(
                        PiOfficeTurnUsageUnknownReason::MissingFinalUsageSnapshot,
                    ),
                    now,
                );
            }
            self.begin_registered_boundary_containment(child, now);
            return Err(PiExecutionError::PeerFatalWithoutAccountingFact);
        }

        match (&output.frame().event, output.observation()) {
            (
                OutboundEvent::CommandResult(CommandResult::Accepted {
                    command: CommandName::Prompt,
                    ..
                }),
                None,
            ) => {
                if turn.phase != OfficePiTurnPhase::AwaitingPromptAcceptance {
                    self.begin_registered_boundary_containment(child, now);
                    return Err(PiExecutionError::PromptEvidenceOrder);
                }
                let event = execute_office_turn_command(
                    store,
                    &turn.operation,
                    PiOfficeTurnCommand::RecordPromptAccepted,
                    Capability::RecordPiOfficeTurnPromptAccepted,
                    ExpectedGeneration::Exact(child.expected_generation),
                    CommandBody::RecordPiOfficeTurnPromptAccepted {
                        office_turn_id: turn.office_turn_id,
                        correlation_identity: turn.correlation_identity.clone(),
                        command_result_sequence: sequence,
                    },
                );
                match event {
                    Ok(EventBody::PiOfficeTurnPromptAccepted {
                        office_turn_id,
                        correlation_identity,
                        command_result_sequence,
                    }) if office_turn_id == turn.office_turn_id
                        && correlation_identity == turn.correlation_identity
                        && command_result_sequence == sequence =>
                    {
                        turn.accepted_sequence = Some(sequence);
                        turn.phase = OfficePiTurnPhase::AwaitingTerminalEvidence;
                        Ok(OfficePiTurnOutput::PromptAccepted)
                    }
                    Ok(_) => {
                        self.begin_registered_boundary_containment(child, now);
                        Err(PiExecutionError::UnexpectedKernelEvent)
                    }
                    Err(error) => {
                        self.begin_registered_boundary_containment(child, now);
                        Err(error)
                    }
                }
            }
            (
                OutboundEvent::CommandResult(CommandResult::Rejected {
                    command: CommandName::Prompt,
                    ..
                }),
                None,
            ) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::PromptRejectedByHost)
            }
            (
                OutboundEvent::AgentEvent {
                    agent_event: ProjectedAgentEvent::AgentSettled,
                },
                None,
            ) => {
                if turn.phase != OfficePiTurnPhase::AwaitingTerminalEvidence
                    || turn
                        .accepted_sequence
                        .is_none_or(|accepted| sequence <= accepted)
                {
                    self.begin_registered_boundary_containment(child, now);
                    return Err(PiExecutionError::PromptEvidenceOrder);
                }
                turn.agent_settled_sequence = Some(sequence);
                Ok(OfficePiTurnOutput::ControlInterleaving)
            }
            (
                OutboundEvent::ForumToolCall {
                    tool_call_identity,
                    tool_name,
                    args,
                },
                Some(society_pi::PeerObservation::ForumToolCall {
                    correlation_identity,
                    tool_call_identity: observed_tool_call_identity,
                    tool_name: observed_tool_name,
                    args: observed_args,
                }),
            ) if correlation_identity.as_str() == turn.correlation_identity.as_str()
                && tool_call_identity == observed_tool_call_identity
                && tool_name == observed_tool_name
                && society_pi::sdk_json_values_equal(args, observed_args) =>
            {
                if turn.phase != OfficePiTurnPhase::AwaitingTerminalEvidence {
                    self.begin_registered_boundary_containment(child, now);
                    return Err(PiExecutionError::PromptEvidenceOrder);
                }
                Ok(OfficePiTurnOutput::ForumToolCall {
                    correlation_identity: correlation_identity.clone(),
                    tool_call_identity: tool_call_identity.clone(),
                    tool_name: *tool_name,
                    args: args.clone(),
                })
            }
            (
                OutboundEvent::UsageSnapshot {
                    usage: UsageObservation::Known(totals),
                },
                // A known cumulative snapshot remains exact Prompt evidence
                // even when the peer has no newly normalized delta to emit.
                // The M6 kernel owns the independent nondecreasing cumulative
                // check and needs the forced post-AgentSettled snapshot to
                // prove this turn's final accounting sequence.
                _,
            ) => {
                if turn
                    .accepted_sequence
                    .is_none_or(|accepted| sequence <= accepted)
                {
                    self.begin_registered_boundary_containment(child, now);
                    return Err(PiExecutionError::PromptEvidenceOrder);
                }
                let usage = kernel_cumulative_usage(totals)?;
                let event = execute_office_turn_command(
                    store,
                    &turn.operation,
                    PiOfficeTurnCommand::RecordKnownUsage { sequence },
                    Capability::RecordPiOfficeTurnUsage,
                    ExpectedGeneration::Exact(child.expected_generation),
                    CommandBody::RecordPiOfficeTurnUsage {
                        office_turn_id: turn.office_turn_id,
                        correlation_identity: turn.correlation_identity.clone(),
                        protocol_sequence: sequence,
                        usage,
                    },
                );
                match event {
                    Ok(EventBody::PiOfficeTurnUsageRecorded {
                        office_turn_id,
                        protocol_sequence,
                        ..
                    }) if office_turn_id == turn.office_turn_id
                        && protocol_sequence == sequence =>
                    {
                        turn.latest_known_accounting_sequence = Some(sequence);
                        if turn
                            .agent_settled_sequence
                            .is_some_and(|settled| sequence > settled)
                        {
                            turn.final_accounting_sequence = Some(sequence);
                        }
                        Ok(OfficePiTurnOutput::KnownUsageRecorded)
                    }
                    Ok(_) => {
                        self.begin_registered_boundary_containment(child, now);
                        Err(PiExecutionError::UnexpectedKernelEvent)
                    }
                    Err(error) => {
                        self.begin_registered_boundary_containment(child, now);
                        Err(error)
                    }
                }
            }
            (
                OutboundEvent::UsageSnapshot {
                    usage: UsageObservation::Unavailable(reason),
                },
                Some(society_pi::PeerObservation::UsageUnavailable { reason: observed }),
            ) if reason == observed => {
                if turn
                    .accepted_sequence
                    .is_none_or(|accepted| sequence <= accepted)
                {
                    self.begin_registered_boundary_containment(child, now);
                    return Err(PiExecutionError::PromptEvidenceOrder);
                }
                self.record_office_turn_usage_failure(
                    store,
                    child,
                    turn,
                    sequence,
                    kernel_usage_failure(*reason),
                    now,
                )
            }
            (
                OutboundEvent::Settled {
                    classification,
                    final_assistant_outcome,
                },
                Some(society_pi::PeerObservation::TurnSettled(receipt)),
            ) => {
                let peer_became_fatal = output.peer_became_fatal();
                let result = self.record_office_turn_terminal(
                    store,
                    child,
                    turn,
                    sequence,
                    *classification,
                    final_assistant_outcome,
                    receipt,
                    now,
                );
                if peer_became_fatal {
                    self.begin_registered_boundary_containment(child, now);
                }
                result
            }
            (OutboundEvent::Fatal { .. }, _) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::PeerFatalWithoutAccountingFact)
            }
            _ => {
                // The peer admits normal lifecycle/tool frames, but M6 has
                // no durable generic-event table. They remain raw sealed
                // evidence and cannot stand in for a named accounting fact.
                if output.peer_became_fatal() {
                    self.begin_registered_boundary_containment(child, now);
                    return Err(PiExecutionError::PeerFatalWithoutAccountingFact);
                }
                Ok(OfficePiTurnOutput::ControlInterleaving)
            }
        }
    }

    /// Records a named inability to account for an admitted Prompt at the
    /// exact host sequence that exposed it. This deliberately never invents a
    /// successor Usage frame: a peer-rejected but schema-valid `Settled`, for
    /// example, is the only available sequence for the closed Unknown fact.
    fn record_office_turn_usage_failure(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        turn: &mut OfficePiTurn,
        protocol_sequence: PiProtocolSequence,
        failure: PiOfficeTurnUsageFailure,
        now: MonotonicTick,
    ) -> Result<OfficePiTurnOutput, PiExecutionError> {
        // Pi may report a typed Unavailable snapshot immediately after the
        // accepted Prompt and before it can project `agent_settled`. That
        // snapshot is already the named reason to freeze; requiring invented
        // later terminal evidence would lose its durable accounting fact.
        // Conversely, the schema-valid Settled -> Unknown path is meaningful
        // only after the final agent lifecycle event it says lacks accounting.
        let missing_final_usage = matches!(
            failure,
            PiOfficeTurnUsageFailure::Unknown(
                PiOfficeTurnUsageUnknownReason::MissingFinalUsageSnapshot
            )
        );
        if turn.phase != OfficePiTurnPhase::AwaitingTerminalEvidence
            || turn
                .accepted_sequence
                .is_none_or(|accepted| protocol_sequence <= accepted)
            || (missing_final_usage
                && turn
                    .agent_settled_sequence
                    .is_none_or(|settled| protocol_sequence <= settled))
        {
            self.begin_registered_boundary_containment(child, now);
            return Err(PiExecutionError::PromptEvidenceOrder);
        }
        let event = execute_office_turn_command(
            store,
            &turn.operation,
            PiOfficeTurnCommand::RecordUsageFailure {
                sequence: protocol_sequence,
            },
            Capability::RecordPiOfficeTurnUsageFailure,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiOfficeTurnUsageFailure {
                office_turn_id: turn.office_turn_id,
                correlation_identity: turn.correlation_identity.clone(),
                protocol_sequence,
                failure,
            },
        );
        match event {
            Ok(EventBody::PiOfficeTurnUsageFrozen {
                office_turn_id,
                failure: observed_failure,
                ..
            }) if office_turn_id == turn.office_turn_id && observed_failure == failure => {
                turn.final_accounting_sequence = Some(protocol_sequence);
                turn.phase = OfficePiTurnPhase::UsageFrozen;
                self.begin_registered_boundary_containment(child, now);
                Ok(OfficePiTurnOutput::UsageFrozen)
            }
            Ok(_) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn record_office_turn_terminal(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        turn: &mut OfficePiTurn,
        settled_sequence: PiProtocolSequence,
        classification: SettledClassification,
        final_assistant_outcome: &FinalAssistantOutcome,
        receipt: &society_pi::TurnReceipt,
        now: MonotonicTick,
    ) -> Result<OfficePiTurnOutput, PiExecutionError> {
        if turn.phase != OfficePiTurnPhase::AwaitingTerminalEvidence
            || receipt.correlation_identity.as_str() != turn.correlation_identity.as_str()
            || kernel_turn_disposition(receipt.disposition)?
                != kernel_turn_disposition_from_settled(classification, final_assistant_outcome)?
            || kernel_assistant_outcome(&receipt.final_assistant_outcome)?
                != kernel_assistant_outcome(final_assistant_outcome)?
        {
            self.begin_registered_boundary_containment(child, now);
            return Err(PiExecutionError::PromptEvidenceOrder);
        }
        let disposition = kernel_turn_disposition(receipt.disposition)?;
        let assistant_outcome = kernel_assistant_outcome(&receipt.final_assistant_outcome)?;
        let terminal_evidence = match assistant_outcome {
            PiOfficeTurnAssistantOutcome::ObservedStop
            | PiOfficeTurnAssistantOutcome::ObservedLength
            | PiOfficeTurnAssistantOutcome::ObservedError
            | PiOfficeTurnAssistantOutcome::ObservedAborted => {
                let agent_settled_sequence = turn
                    .agent_settled_sequence
                    .ok_or(PiExecutionError::PromptTerminalEvidenceMissing)?;
                let final_accounting_sequence = turn
                    .final_accounting_sequence
                    .ok_or(PiExecutionError::PromptTerminalEvidenceMissing)?;
                PiOfficeTurnTerminalEvidence::ObservedAssistant {
                    agent_settled_sequence,
                    final_accounting_sequence,
                }
            }
            PiOfficeTurnAssistantOutcome::SdkPromiseRejected
            | PiOfficeTurnAssistantOutcome::MissingFinalAssistantOutcome => {
                PiOfficeTurnTerminalEvidence::UnavailableAssistant {
                    final_known_usage_sequence: turn
                        .latest_known_accounting_sequence
                        .ok_or(PiExecutionError::PromptTerminalEvidenceMissing)?,
                }
            }
        };
        if terminal_evidence
            .final_accounting_sequence()
            .value()
            .checked_add(1)
            != Some(settled_sequence.value())
        {
            self.begin_registered_boundary_containment(child, now);
            return Err(PiExecutionError::PromptEvidenceOrder);
        }
        let terminal = execute_office_turn_command(
            store,
            &turn.operation,
            PiOfficeTurnCommand::RecordTerminal,
            Capability::RecordPiOfficeTurnTerminal,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiOfficeTurnTerminal {
                office_turn_id: turn.office_turn_id,
                correlation_identity: turn.correlation_identity.clone(),
                terminal_evidence,
                settled_sequence,
                disposition,
                assistant_outcome,
                transcript_disposition:
                    PiOfficeTurnTranscriptDisposition::DeferredUntilOfficeSessionDispose,
            },
        );
        let terminal_receipt_id = match terminal {
            Ok(EventBody::PiOfficeTurnTerminalRecorded {
                pi_office_turn_terminal_receipt_id,
                office_turn_id,
                disposition: observed_disposition,
                assistant_outcome: observed_outcome,
            }) if office_turn_id == turn.office_turn_id
                && observed_disposition == disposition
                && observed_outcome == assistant_outcome =>
            {
                pi_office_turn_terminal_receipt_id
            }
            Ok(_) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(PiExecutionError::UnexpectedKernelEvent);
            }
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                return Err(error);
            }
        };
        turn.phase = OfficePiTurnPhase::TerminalRecorded;
        if disposition != PiOfficeTurnDisposition::Completed
            || assistant_outcome != PiOfficeTurnAssistantOutcome::ObservedStop
        {
            child.phase = OfficePiExecutionPhase::OfficeTurnTerminalBlocked;
            return Ok(OfficePiTurnOutput::TerminalRecordedNonReady);
        }
        let settled = execute_office_turn_command(
            store,
            &turn.operation,
            PiOfficeTurnCommand::Settle,
            Capability::SettleOfficeTurn,
            ExpectedGeneration::NotApplicable,
            CommandBody::SettleOfficeTurn {
                turn_id: turn.office_turn_id,
                terminal_receipt_id,
            },
        );
        match settled {
            Ok(EventBody::OfficeTurnSettled {
                turn_id,
                session_id,
                ..
            }) if turn_id == turn.office_turn_id && session_id == child.office_session_id => {
                turn.phase = OfficePiTurnPhase::Settled;
                child.phase = OfficePiExecutionPhase::OfficeReadyRecorded;
                Ok(OfficePiTurnOutput::SettledReady)
            }
            Ok(_) => {
                self.begin_registered_boundary_containment(child, now);
                Err(PiExecutionError::UnexpectedKernelEvent)
            }
            Err(error) => {
                self.begin_registered_boundary_containment(child, now);
                Err(error)
            }
        }
    }

    fn record_create_delivery(
        &mut self,
        store: &mut KernelStore,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
    ) -> Result<(), PiExecutionError> {
        let event = execute_kernel_command(
            store,
            &child.operation,
            PiExecutionCommand::RecordCreateDelivery,
            Capability::RecordPiCreateSessionDelivery,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordPiCreateSessionDelivery {
                native_child_id: child.child_process_id,
                correlation_identity: child.create_correlation.clone(),
                create_request_digest: child.create_request_digest,
            },
        );
        if let Err(error) = event {
            self.begin_registered_boundary_containment(child, now);
            return Err(error);
        }
        child.phase = OfficePiExecutionPhase::CreateDelivered;
        Ok(())
    }

    fn record_pre_reap_signal_receipts(
        &self,
        store: &mut KernelStore,
        child: &OfficePiExecutionChild,
        direct_reap: &crate::supervision::DirectChildReapFacts,
    ) -> Result<(), PiExecutionError> {
        for (ordinal, signal) in direct_reap.prior_signal_receipts.iter().enumerate() {
            if signal.action == SignalAction::LingeringGroupKill {
                return Err(PiExecutionError::SignalReceiptOrderingRequiresTwoPhaseReap);
            }
            self.record_signal_receipt(store, child, signal, ordinal)?;
        }
        Ok(())
    }

    fn record_direct_child_reap(
        &self,
        store: &mut KernelStore,
        child: &OfficePiExecutionChild,
        direct_reap: &crate::supervision::DirectChildReapFacts,
    ) -> Result<(), PiExecutionError> {
        if direct_reap.child_process_id != child.supervised_child_id {
            return Err(PiExecutionError::ReceiptIdentityMismatch);
        }
        let liveness = kernel_liveness(direct_reap.group_liveness_after_direct_child_reap);
        execute_kernel_command(
            store,
            &child.operation,
            PiExecutionCommand::RecordReap,
            Capability::RecordDirectChildReap,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordDirectChildReap {
                native_child_id: child.child_process_id,
                wait_status: kernel_wait_status(direct_reap.status)?,
                // The supervisor has not issued the *distinct* lingering
                // group policy action yet. Both observations therefore name
                // the same honest post-wait/basic-cleanup liveness fact.
                group_liveness_before_cleanup: liveness,
                group_liveness_after_cleanup: liveness,
            },
        )?;
        Ok(())
    }

    fn record_liveness(
        &self,
        store: &mut KernelStore,
        child: &OfficePiExecutionChild,
        liveness: crate::supervision::ProcessGroupLiveness,
    ) -> Result<(), PiExecutionError> {
        record_liveness_for_child(
            store,
            &child.operation,
            child.child_process_id,
            child.expected_generation,
            liveness,
        )?;
        Ok(())
    }

    fn seal_and_finalize(
        &mut self,
        store: &mut KernelStore,
        content: &ContentSealingAuthority,
        child: &OfficePiExecutionChild,
        receipt: &SupervisionReceipt,
    ) -> Result<(), PiExecutionError> {
        if receipt.child_process_id != child.supervised_child_id {
            return Err(PiExecutionError::ReceiptIdentityMismatch);
        }
        let _reap = receipt
            .reap
            .as_ref()
            .ok_or(PiExecutionError::MissingReapReceipt)?;
        self.seal_stream(
            store,
            content,
            child,
            ChildStreamKind::AdmittedControl,
            &receipt.transient_evidence.admitted_control,
        )?;
        self.seal_stream(
            store,
            content,
            child,
            ChildStreamKind::PhysicalStdin,
            &receipt.transient_evidence.stdin,
        )?;
        self.seal_stream(
            store,
            content,
            child,
            ChildStreamKind::Stdout,
            &receipt.transient_evidence.stdout,
        )?;
        self.seal_stream(
            store,
            content,
            child,
            ChildStreamKind::Stderr,
            &receipt.transient_evidence.stderr,
        )?;
        execute_kernel_command(
            store,
            &child.operation,
            PiExecutionCommand::Finalize,
            Capability::FinalizeChildProcess,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::FinalizeChildProcess {
                native_child_id: child.child_process_id,
            },
        )?;
        Ok(())
    }

    fn record_signal_receipt(
        &self,
        store: &mut KernelStore,
        child: &OfficePiExecutionChild,
        signal: &crate::supervision::SignalReceipt,
        ordinal: usize,
    ) -> Result<(), PiExecutionError> {
        record_signal_receipt_for_child(
            store,
            &child.operation,
            child.child_process_id,
            child.expected_generation,
            signal,
            ordinal,
        )
    }

    fn seal_stream(
        &self,
        store: &mut KernelStore,
        content: &ContentSealingAuthority,
        child: &OfficePiExecutionChild,
        stream_kind: ChildStreamKind,
        capture: &TransientStreamCapture,
    ) -> Result<(), PiExecutionError> {
        seal_stream_for_child(
            store,
            content,
            &child.operation,
            child.child_process_id,
            child.expected_generation,
            stream_kind,
            capture,
        )
    }

    fn begin_registered_boundary_containment(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
    ) {
        let child_process_id = child.supervised_child_id.clone();
        child.phase = OfficePiExecutionPhase::BoundaryContainmentRequired;
        self.contain(&child_process_id, now);
    }

    fn unresolved_registration(
        &mut self,
        supervised_child_id: SupervisedChildId,
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
        failure: PiExecutionError,
    ) -> OfficePiSpawnRegistration {
        // The durable admission exists but `RecordInertChildSpawn` does not,
        // so no kernel child identity may be fabricated. Contain the exact
        // native group now; its physical completion remains transient and the
        // admission deliberately stays unresolved for recovery fencing.
        self.contain(&supervised_child_id, MonotonicTick::ZERO);
        OfficePiSpawnRegistration::RegistrationUnresolved {
            child: Box::new(UnregisteredPiChild {
                supervised_child_id,
                native_child_spawn_admission_id,
                phase: UnregisteredPiChildPhase::ContainmentRequired,
                transient_completion: None,
            }),
            failure,
        }
    }

    fn contain(&mut self, child_process_id: &SupervisedChildId, now: MonotonicTick) {
        let _ = self
            .supervisor
            .contain_boundary_failure(child_process_id, now);
    }
}

impl PiExecutionDriver {
    // The task path uses the same low-level custody transitions as Office,
    // but keeps the owner-specific handle and lifecycle separate.
    fn record_task_attempt_pre_reap_signal_receipts(
        &self,
        store: &mut KernelStore,
        child: &TaskAttemptPiExecutionChild,
        direct_reap: &crate::supervision::DirectChildReapFacts,
    ) -> Result<(), PiExecutionError> {
        for (ordinal, signal) in direct_reap.prior_signal_receipts.iter().enumerate() {
            if signal.action == SignalAction::LingeringGroupKill {
                return Err(PiExecutionError::SignalReceiptOrderingRequiresTwoPhaseReap);
            }
            record_signal_receipt_for_child(
                store,
                &child.operation,
                child.child_process_id,
                child.expected_generation,
                signal,
                ordinal,
            )?;
        }
        Ok(())
    }

    fn record_task_attempt_direct_child_reap(
        &self,
        store: &mut KernelStore,
        child: &TaskAttemptPiExecutionChild,
        direct_reap: &crate::supervision::DirectChildReapFacts,
    ) -> Result<(), PiExecutionError> {
        if direct_reap.child_process_id != child.supervised_child_id {
            return Err(PiExecutionError::ReceiptIdentityMismatch);
        }
        let liveness = kernel_liveness(direct_reap.group_liveness_after_direct_child_reap);
        execute_kernel_command(
            store,
            &child.operation,
            PiExecutionCommand::RecordReap,
            Capability::RecordDirectChildReap,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordDirectChildReap {
                native_child_id: child.child_process_id,
                wait_status: kernel_wait_status(direct_reap.status)?,
                group_liveness_before_cleanup: liveness,
                group_liveness_after_cleanup: liveness,
            },
        )?;
        Ok(())
    }

    pub(crate) fn poll_task_attempt_reap_and_reconcile(
        &mut self,
        store: &mut KernelStore,
        content: &ContentSealingAuthority,
        child: &mut TaskAttemptPiExecutionChild,
        now: MonotonicTick,
    ) -> Result<bool, PiExecutionError> {
        if child.phase == TaskAttemptPiExecutionPhase::Reconciled {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let Some(direct_reap) = self
            .supervisor
            .poll_direct_child_reap_at(&child.supervised_child_id)
            .map_err(PiExecutionError::Supervision)?
        else {
            return Ok(false);
        };
        if child.phase != TaskAttemptPiExecutionPhase::DirectChildReapRecorded
            && child.phase != TaskAttemptPiExecutionPhase::LingeringCleanupRecorded
            && child.phase != TaskAttemptPiExecutionPhase::AwaitingLingeringGroupAbsence
        {
            self.record_task_attempt_pre_reap_signal_receipts(store, child, &direct_reap)?;
            self.record_task_attempt_direct_child_reap(store, child, &direct_reap)?;
            if direct_reap.prior_signal_receipts.iter().any(|receipt| {
                receipt.delivery == SignalDelivery::GroupInaccessible
                    || receipt.group_liveness_after_attempt
                        == crate::supervision::ProcessGroupLiveness::Inaccessible
            }) {
                return Err(PiExecutionError::AutomaticContainmentInaccessible);
            }
            if direct_reap.group_liveness_after_direct_child_reap
                == crate::supervision::ProcessGroupLiveness::Inaccessible
            {
                return Err(PiExecutionError::LingeringGroupInaccessible);
            }
            child.phase = TaskAttemptPiExecutionPhase::DirectChildReapRecorded;
        }

        if child.phase == TaskAttemptPiExecutionPhase::DirectChildReapRecorded {
            if let Some(signal) = self
                .supervisor
                .issue_lingering_group_cleanup(&child.supervised_child_id, now)
                .map_err(PiExecutionError::Supervision)?
            {
                record_signal_receipt_for_child(
                    store,
                    &child.operation,
                    child.child_process_id,
                    child.expected_generation,
                    &signal,
                    2,
                )?;
                match signal.group_liveness_after_attempt {
                    crate::supervision::ProcessGroupLiveness::Present => {
                        child.phase = TaskAttemptPiExecutionPhase::AwaitingLingeringGroupAbsence;
                        return Ok(false);
                    }
                    crate::supervision::ProcessGroupLiveness::Inaccessible => {
                        return Err(PiExecutionError::LingeringGroupInaccessible);
                    }
                    crate::supervision::ProcessGroupLiveness::Absent => {}
                }
            }
            child.phase = TaskAttemptPiExecutionPhase::LingeringCleanupRecorded;
        }

        let liveness = self
            .supervisor
            .observe_group_liveness(&child.supervised_child_id)
            .map_err(PiExecutionError::Supervision)?;
        match liveness {
            crate::supervision::ProcessGroupLiveness::Present => {
                if child.phase == TaskAttemptPiExecutionPhase::AwaitingLingeringGroupAbsence {
                    return Ok(false);
                }
                record_liveness_for_child(
                    store,
                    &child.operation,
                    child.child_process_id,
                    child.expected_generation,
                    liveness,
                )?;
                return Err(PiExecutionError::ProcessGroupIdentityRegressed);
            }
            crate::supervision::ProcessGroupLiveness::Inaccessible => {
                record_liveness_for_child(
                    store,
                    &child.operation,
                    child.child_process_id,
                    child.expected_generation,
                    liveness,
                )?;
                return Err(PiExecutionError::LingeringGroupInaccessible);
            }
            crate::supervision::ProcessGroupLiveness::Absent => {
                record_liveness_for_child(
                    store,
                    &child.operation,
                    child.child_process_id,
                    child.expected_generation,
                    liveness,
                )?;
            }
        }
        let receipt = self
            .supervisor
            .complete_deferred_reap_at(&child.supervised_child_id)
            .map_err(PiExecutionError::Supervision)?;
        seal_and_finalize_for_child(
            store,
            content,
            &child.operation,
            child.child_process_id,
            child.expected_generation,
            &child.supervised_child_id,
            &receipt,
        )?;
        self.supervisor
            .take_reaped_receipt(&child.supervised_child_id)
            .ok_or(PiExecutionError::ReapReceiptLost)?;
        child.phase = TaskAttemptPiExecutionPhase::Reconciled;
        Ok(true)
    }
}

fn record_signal_receipt_for_child(
    store: &mut KernelStore,
    operation: &PiExecutionOperationId,
    child_process_id: NativeChildId,
    expected_generation: AdmissionGeneration,
    signal: &crate::supervision::SignalReceipt,
    ordinal: usize,
) -> Result<(), PiExecutionError> {
    if signal.action == SignalAction::AbortControl {
        return Err(PiExecutionError::UnmodeledAbortControlReceipt);
    }
    let action = match signal.action {
        SignalAction::Terminate => society_kernel::ProcessSignalAction::Terminate,
        SignalAction::Kill => society_kernel::ProcessSignalAction::Kill,
        SignalAction::LingeringGroupKill => society_kernel::ProcessSignalAction::LingeringGroupKill,
        SignalAction::AbortControl => unreachable!("checked above"),
    };
    let delivery = match signal.delivery {
        SignalDelivery::TermSent
        | SignalDelivery::KillSent
        | SignalDelivery::LingeringGroupKillSent => {
            society_kernel::ProcessSignalDelivery::Delivered
        }
        SignalDelivery::AbsentBeforeSignal => {
            society_kernel::ProcessSignalDelivery::AbsentBeforeSignal
        }
        SignalDelivery::AbsentDuringSignal => {
            society_kernel::ProcessSignalDelivery::AbsentDuringSignal
        }
        SignalDelivery::GroupInaccessible => society_kernel::ProcessSignalDelivery::Inaccessible,
        SignalDelivery::AbortControlWritten => {
            return Err(PiExecutionError::UnmodeledAbortControlReceipt);
        }
    };
    execute_kernel_command(
        store,
        operation,
        PiExecutionCommand::RecordSignal { ordinal },
        Capability::RecordProcessSignalReceipt,
        ExpectedGeneration::Exact(expected_generation),
        CommandBody::RecordProcessSignalReceipt {
            native_child_id: child_process_id,
            action,
            delivery,
            observed_liveness: kernel_liveness(signal.group_liveness_after_attempt),
            cause: society_kernel::ProcessSignalCause::AutomaticBoundaryContainment,
        },
    )?;
    Ok(())
}

fn record_liveness_for_child(
    store: &mut KernelStore,
    operation: &PiExecutionOperationId,
    child_process_id: NativeChildId,
    expected_generation: AdmissionGeneration,
    liveness: crate::supervision::ProcessGroupLiveness,
) -> Result<(), PiExecutionError> {
    execute_kernel_command(
        store,
        operation,
        PiExecutionCommand::RecordLiveness,
        Capability::RecordChildProcessLiveness,
        ExpectedGeneration::Exact(expected_generation),
        CommandBody::RecordChildProcessLiveness {
            native_child_id: child_process_id,
            liveness: kernel_liveness(liveness),
        },
    )?;
    Ok(())
}

fn seal_stream_for_child(
    store: &mut KernelStore,
    content: &ContentSealingAuthority,
    operation: &PiExecutionOperationId,
    child_process_id: NativeChildId,
    expected_generation: AdmissionGeneration,
    stream_kind: ChildStreamKind,
    capture: &TransientStreamCapture,
) -> Result<(), PiExecutionError> {
    let retained_bytes = capture.retained_bytes();
    let content_operation = ContentSealOperationId::parse(
        operation.content_label(child_process_id, stream_kind)?,
        KernelDigest::of_bytes(retained_bytes),
    )
    .map_err(|_| PiExecutionError::InvalidOperationIdentity)?;
    let registration = content.seal_and_register(store, &content_operation, retained_bytes)?;
    execute_kernel_command(
        store,
        operation,
        stream_seal_command(stream_kind),
        Capability::RecordChildStreamSeal,
        ExpectedGeneration::Exact(expected_generation),
        CommandBody::RecordChildStreamSeal {
            native_child_id: child_process_id,
            stream_kind,
            full_observed_digest: kernel_digest_from_boundary(capture)?,
            retained_content_object_id: registration.content_object_id,
            completeness: kernel_stream_completeness(capture),
        },
    )?;
    Ok(())
}

fn seal_and_finalize_for_child(
    store: &mut KernelStore,
    content: &ContentSealingAuthority,
    operation: &PiExecutionOperationId,
    child_process_id: NativeChildId,
    expected_generation: AdmissionGeneration,
    supervised_child_id: &SupervisedChildId,
    receipt: &SupervisionReceipt,
) -> Result<(), PiExecutionError> {
    if receipt.child_process_id != *supervised_child_id {
        return Err(PiExecutionError::ReceiptIdentityMismatch);
    }
    let _reap = receipt
        .reap
        .as_ref()
        .ok_or(PiExecutionError::MissingReapReceipt)?;
    seal_stream_for_child(
        store,
        content,
        operation,
        child_process_id,
        expected_generation,
        ChildStreamKind::AdmittedControl,
        &receipt.transient_evidence.admitted_control,
    )?;
    seal_stream_for_child(
        store,
        content,
        operation,
        child_process_id,
        expected_generation,
        ChildStreamKind::PhysicalStdin,
        &receipt.transient_evidence.stdin,
    )?;
    seal_stream_for_child(
        store,
        content,
        operation,
        child_process_id,
        expected_generation,
        ChildStreamKind::Stdout,
        &receipt.transient_evidence.stdout,
    )?;
    seal_stream_for_child(
        store,
        content,
        operation,
        child_process_id,
        expected_generation,
        ChildStreamKind::Stderr,
        &receipt.transient_evidence.stderr,
    )?;
    execute_kernel_command(
        store,
        operation,
        PiExecutionCommand::Finalize,
        Capability::FinalizeChildProcess,
        ExpectedGeneration::Exact(expected_generation),
        CommandBody::FinalizeChildProcess {
            native_child_id: child_process_id,
        },
    )?;
    Ok(())
}

struct KernelCreateAuthorizationGate<'a> {
    store: &'a mut KernelStore,
    operation: &'a PiExecutionOperationId,
    child_process_id: NativeChildId,
    expected_generation: AdmissionGeneration,
    correlation: &'a PiCorrelationIdentity,
    create_request_digest: KernelDigest,
    outcome: Option<Result<(), PiExecutionError>>,
}

impl<'a> KernelCreateAuthorizationGate<'a> {
    fn new(
        store: &'a mut KernelStore,
        operation: &'a PiExecutionOperationId,
        child_process_id: NativeChildId,
        expected_generation: AdmissionGeneration,
        correlation: &'a PiCorrelationIdentity,
        create_request_digest: KernelDigest,
    ) -> Self {
        Self {
            store,
            operation,
            child_process_id,
            expected_generation,
            correlation,
            create_request_digest,
            outcome: None,
        }
    }

    fn finish(mut self) -> Result<(), PiExecutionError> {
        self.outcome
            .take()
            .ok_or(PiExecutionError::CreateGateNotInvoked)?
    }
}

impl PreCreateAdmissionGate for KernelCreateAuthorizationGate<'_> {
    fn recheck(&mut self, _: &InertChildFacts) -> Result<(), crate::supervision::AdmissionDenied> {
        let outcome = execute_kernel_command(
            self.store,
            self.operation,
            PiExecutionCommand::AuthorizeCreate,
            Capability::AuthorizePiCreateSession,
            ExpectedGeneration::Exact(self.expected_generation),
            CommandBody::AuthorizePiCreateSession {
                native_child_id: self.child_process_id,
                correlation_identity: self.correlation.clone(),
                create_request_digest: self.create_request_digest,
            },
        )
        .and_then(|event| match event {
            EventBody::PiCreateSessionAuthorized {
                native_child_id: child_process_id,
            } if child_process_id == self.child_process_id => Ok(()),
            _ => Err(PiExecutionError::UnexpectedKernelEvent),
        });
        self.outcome = Some(outcome);
        self.outcome
            .as_ref()
            .expect("outcome was assigned")
            .as_ref()
            .map_err(|_| crate::supervision::AdmissionDenied::StaleGeneration)
            .copied()
    }
}

fn execute_kernel_command(
    store: &mut KernelStore,
    operation: &PiExecutionOperationId,
    command: PiExecutionCommand,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> Result<EventBody, PiExecutionError> {
    let capability_grant_id = store
        .active_capability_grant(PrincipalId::KERNEL, capability)?
        .ok_or(PiExecutionError::KernelServiceCapabilityMissing { capability })?;
    let receipt = store.execute(CommandRequest {
        command_id: operation.command_id(command)?,
        principal_id: PrincipalId::KERNEL,
        capability_grant_id,
        capability,
        expected_generation,
        body,
    })?;
    let event_id = match receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        CommandDisposition::Rejected(rejection) => {
            return Err(PiExecutionError::KernelCommandRejected {
                capability,
                rejection,
            });
        }
    };
    Ok(store.ledger_event(event_id)?.body)
}

fn execute_office_turn_command(
    store: &mut KernelStore,
    operation: &PiOfficeTurnOperationId,
    command: PiOfficeTurnCommand,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> Result<EventBody, PiExecutionError> {
    let capability_grant_id = store
        .active_capability_grant(PrincipalId::KERNEL, capability)?
        .ok_or(PiExecutionError::KernelServiceCapabilityMissing { capability })?;
    let receipt = store.execute(CommandRequest {
        command_id: operation.command_id(command)?,
        principal_id: PrincipalId::KERNEL,
        capability_grant_id,
        capability,
        expected_generation,
        body,
    })?;
    let event_id = match receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        CommandDisposition::Rejected(rejection) => {
            return Err(PiExecutionError::KernelCommandRejected {
                capability,
                rejection,
            });
        }
    };
    Ok(store.ledger_event(event_id)?.body)
}

/// Executes one retry-stable KERNEL-service command in the actor-local Prompt
/// namespace. The namespace is deliberately distinct from Office turns even
/// though both chains project the same Pi SDK frame vocabulary.
fn execute_task_attempt_prompt_command(
    store: &mut KernelStore,
    operation: &PiTaskAttemptPromptOperationId,
    command: PiTaskAttemptPromptCommand,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> Result<EventBody, PiExecutionError> {
    let capability_grant_id = store
        .active_capability_grant(PrincipalId::KERNEL, capability)?
        .ok_or(PiExecutionError::KernelServiceCapabilityMissing { capability })?;
    let receipt = store.execute(CommandRequest {
        command_id: operation.command_id(command)?,
        principal_id: PrincipalId::KERNEL,
        capability_grant_id,
        capability,
        expected_generation,
        body,
    })?;
    let event_id = match receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        CommandDisposition::Rejected(rejection) => {
            return Err(PiExecutionError::KernelCommandRejected {
                capability,
                rejection,
            });
        }
    };
    Ok(store.ledger_event(event_id)?.body)
}

fn execute_task_attempt_session_dispose_command(
    store: &mut KernelStore,
    operation: &PiTaskAttemptSessionDisposeOperationId,
    command: PiTaskAttemptSessionDisposeCommand,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> Result<EventBody, PiExecutionError> {
    let capability_grant_id = store
        .active_capability_grant(PrincipalId::KERNEL, capability)?
        .ok_or(PiExecutionError::KernelServiceCapabilityMissing { capability })?;
    let receipt = store.execute(CommandRequest {
        command_id: operation.command_id(command)?,
        principal_id: PrincipalId::KERNEL,
        capability_grant_id,
        capability,
        expected_generation,
        body,
    })?;
    let event_id = match receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        CommandDisposition::Rejected(rejection) => {
            return Err(PiExecutionError::KernelCommandRejected {
                capability,
                rejection,
            });
        }
    };
    Ok(store.ledger_event(event_id)?.body)
}

/// Executes one retry-stable KERNEL-service command in the closing Office
/// session namespace. This remains separate from the M5 child and M6 turn
/// namespaces so a terminal session receipt cannot collide with a prior
/// Prompt operation merely because their textual labels match.
fn execute_office_session_dispose_command(
    store: &mut KernelStore,
    operation: &PiOfficeSessionDisposeOperationId,
    command: PiOfficeSessionDisposeCommand,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> Result<EventBody, PiExecutionError> {
    let capability_grant_id = store
        .active_capability_grant(PrincipalId::KERNEL, capability)?
        .ok_or(PiExecutionError::KernelServiceCapabilityMissing { capability })?;
    let receipt = store.execute(CommandRequest {
        command_id: operation.command_id(command)?,
        principal_id: PrincipalId::KERNEL,
        capability_grant_id,
        capability,
        expected_generation,
        body,
    })?;
    let event_id = match receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        CommandDisposition::Rejected(rejection) => {
            return Err(PiExecutionError::KernelCommandRejected {
                capability,
                rejection,
            });
        }
    };
    Ok(store.ledger_event(event_id)?.body)
}

fn kernel_protocol_sequence(
    value: BoundarySequence,
) -> Result<PiProtocolSequence, PiExecutionError> {
    i64::try_from(value.value())
        .ok()
        .and_then(|sequence| PiProtocolSequence::try_from(sequence).ok())
        .ok_or(PiExecutionError::IdentityConversion)
}

fn kernel_cumulative_usage(
    totals: &society_pi::UsageTotals,
) -> Result<PiCumulativeUsage, PiExecutionError> {
    let token = |value: u64| {
        i64::try_from(value)
            .ok()
            .and_then(|value| PiTokenCount::try_from(value).ok())
            .ok_or(PiExecutionError::UsageConversion)
    };
    let raw_cost = u64::from_str_radix(totals.provider_cost.binary64_big_endian_hex.as_str(), 16)
        .map_err(|_| PiExecutionError::UsageConversion)?;
    let provider_cost = ProviderCostBinary64::from_big_endian(raw_cost.to_be_bytes())
        .map_err(|_| PiExecutionError::UsageConversion)?;
    let ceiling_micro_usd = provider_cost
        .ceil_micro_usd()
        .map_err(|_| PiExecutionError::UsageConversion)?;
    let usage = PiCumulativeUsage {
        input_tokens: token(totals.input_tokens.value())?,
        output_tokens: token(totals.output_tokens.value())?,
        cache_read_tokens: token(totals.cache_read_tokens.value())?,
        cache_write_tokens: token(totals.cache_write_tokens.value())?,
        total_tokens: token(totals.total_tokens.value())?,
        provider_cost,
        ceiling_micro_usd,
    };
    if !usage.is_internally_consistent() {
        return Err(PiExecutionError::UsageConversion);
    }
    Ok(usage)
}

const fn kernel_usage_failure(reason: UsageUnavailableReason) -> PiOfficeTurnUsageFailure {
    let reason = match reason {
        UsageUnavailableReason::InvalidSdkUsage => {
            PiOfficeTurnUsageUnavailableReason::InvalidSdkUsage
        }
        UsageUnavailableReason::UsageRegressed => {
            PiOfficeTurnUsageUnavailableReason::UsageRegressed
        }
        UsageUnavailableReason::UsageInconsistent => {
            PiOfficeTurnUsageUnavailableReason::UsageInconsistent
        }
    };
    PiOfficeTurnUsageFailure::Unavailable(reason)
}

const fn kernel_task_attempt_usage_failure(
    reason: UsageUnavailableReason,
) -> PiTaskAttemptUsageFailure {
    let reason = match reason {
        UsageUnavailableReason::InvalidSdkUsage => {
            PiTaskAttemptUsageUnavailableReason::InvalidSdkUsage
        }
        UsageUnavailableReason::UsageRegressed => {
            PiTaskAttemptUsageUnavailableReason::UsageRegressed
        }
        UsageUnavailableReason::UsageInconsistent => {
            PiTaskAttemptUsageUnavailableReason::UsageInconsistent
        }
    };
    PiTaskAttemptUsageFailure::Unavailable(reason)
}

fn kernel_turn_disposition(
    value: TurnDisposition,
) -> Result<PiOfficeTurnDisposition, PiExecutionError> {
    match value {
        TurnDisposition::Pending => Err(PiExecutionError::PromptEvidenceOrder),
        TurnDisposition::Completed => Ok(PiOfficeTurnDisposition::Completed),
        TurnDisposition::Length => Ok(PiOfficeTurnDisposition::Length),
        TurnDisposition::Error => Ok(PiOfficeTurnDisposition::Error),
        TurnDisposition::Aborted => Ok(PiOfficeTurnDisposition::Aborted),
        TurnDisposition::Failed => Ok(PiOfficeTurnDisposition::Failed),
        TurnDisposition::ProtocolFailed => Ok(PiOfficeTurnDisposition::ProtocolFailed),
    }
}

fn kernel_task_attempt_disposition(
    value: TurnDisposition,
) -> Result<PiTaskAttemptDisposition, PiExecutionError> {
    match value {
        TurnDisposition::Pending => Err(PiExecutionError::PromptEvidenceOrder),
        TurnDisposition::Completed => Ok(PiTaskAttemptDisposition::Completed),
        TurnDisposition::Length => Ok(PiTaskAttemptDisposition::Length),
        TurnDisposition::Error => Ok(PiTaskAttemptDisposition::Error),
        TurnDisposition::Aborted => Ok(PiTaskAttemptDisposition::Aborted),
        TurnDisposition::Failed => Ok(PiTaskAttemptDisposition::Failed),
        TurnDisposition::ProtocolFailed => Ok(PiTaskAttemptDisposition::ProtocolFailed),
    }
}

fn kernel_turn_disposition_from_settled(
    classification: SettledClassification,
    outcome: &FinalAssistantOutcome,
) -> Result<PiOfficeTurnDisposition, PiExecutionError> {
    match (classification, outcome) {
        (
            SettledClassification::Completed,
            FinalAssistantOutcome::Observed {
                stop_reason: AssistantStopReason::Stop,
            },
        ) => Ok(PiOfficeTurnDisposition::Completed),
        (
            SettledClassification::Length,
            FinalAssistantOutcome::Observed {
                stop_reason: AssistantStopReason::Length,
            },
        ) => Ok(PiOfficeTurnDisposition::Length),
        (
            SettledClassification::Error,
            FinalAssistantOutcome::Observed {
                stop_reason: AssistantStopReason::Error,
            },
        ) => Ok(PiOfficeTurnDisposition::Error),
        (
            SettledClassification::Aborted,
            FinalAssistantOutcome::Observed {
                stop_reason: AssistantStopReason::Aborted | AssistantStopReason::Stop,
            },
        ) => Ok(PiOfficeTurnDisposition::Aborted),
        (
            SettledClassification::Failed,
            FinalAssistantOutcome::Unavailable {
                reason: society_pi::FinalAssistantUnavailableReason::SdkPromiseRejected,
            },
        ) => Ok(PiOfficeTurnDisposition::Failed),
        (
            SettledClassification::ProtocolFailed,
            FinalAssistantOutcome::Unavailable {
                reason: society_pi::FinalAssistantUnavailableReason::MissingFinalAssistantOutcome,
            },
        ) => Ok(PiOfficeTurnDisposition::ProtocolFailed),
        _ => Err(PiExecutionError::PromptEvidenceOrder),
    }
}

fn kernel_task_attempt_disposition_from_settled(
    classification: SettledClassification,
    outcome: &FinalAssistantOutcome,
) -> Result<PiTaskAttemptDisposition, PiExecutionError> {
    match (classification, outcome) {
        (
            SettledClassification::Completed,
            FinalAssistantOutcome::Observed {
                stop_reason: AssistantStopReason::Stop,
            },
        ) => Ok(PiTaskAttemptDisposition::Completed),
        (
            SettledClassification::Length,
            FinalAssistantOutcome::Observed {
                stop_reason: AssistantStopReason::Length,
            },
        ) => Ok(PiTaskAttemptDisposition::Length),
        (
            SettledClassification::Error,
            FinalAssistantOutcome::Observed {
                stop_reason: AssistantStopReason::Error,
            },
        ) => Ok(PiTaskAttemptDisposition::Error),
        (
            SettledClassification::Aborted,
            FinalAssistantOutcome::Observed {
                stop_reason: AssistantStopReason::Aborted | AssistantStopReason::Stop,
            },
        ) => Ok(PiTaskAttemptDisposition::Aborted),
        (
            SettledClassification::Failed,
            FinalAssistantOutcome::Unavailable {
                reason: society_pi::FinalAssistantUnavailableReason::SdkPromiseRejected,
            },
        ) => Ok(PiTaskAttemptDisposition::Failed),
        (
            SettledClassification::ProtocolFailed,
            FinalAssistantOutcome::Unavailable {
                reason: society_pi::FinalAssistantUnavailableReason::MissingFinalAssistantOutcome,
            },
        ) => Ok(PiTaskAttemptDisposition::ProtocolFailed),
        _ => Err(PiExecutionError::PromptEvidenceOrder),
    }
}

fn kernel_assistant_outcome(
    value: &FinalAssistantOutcome,
) -> Result<PiOfficeTurnAssistantOutcome, PiExecutionError> {
    match value {
        FinalAssistantOutcome::Observed {
            stop_reason: AssistantStopReason::Stop,
        } => Ok(PiOfficeTurnAssistantOutcome::ObservedStop),
        FinalAssistantOutcome::Observed {
            stop_reason: AssistantStopReason::Length,
        } => Ok(PiOfficeTurnAssistantOutcome::ObservedLength),
        FinalAssistantOutcome::Observed {
            stop_reason: AssistantStopReason::Error,
        } => Ok(PiOfficeTurnAssistantOutcome::ObservedError),
        FinalAssistantOutcome::Observed {
            stop_reason: AssistantStopReason::Aborted,
        } => Ok(PiOfficeTurnAssistantOutcome::ObservedAborted),
        FinalAssistantOutcome::Unavailable {
            reason: society_pi::FinalAssistantUnavailableReason::SdkPromiseRejected,
        } => Ok(PiOfficeTurnAssistantOutcome::SdkPromiseRejected),
        FinalAssistantOutcome::Unavailable {
            reason: society_pi::FinalAssistantUnavailableReason::MissingFinalAssistantOutcome,
        } => Ok(PiOfficeTurnAssistantOutcome::MissingFinalAssistantOutcome),
    }
}

fn kernel_task_attempt_assistant_outcome(
    value: &FinalAssistantOutcome,
) -> Result<PiTaskAttemptAssistantOutcome, PiExecutionError> {
    match value {
        FinalAssistantOutcome::Observed {
            stop_reason: AssistantStopReason::Stop,
        } => Ok(PiTaskAttemptAssistantOutcome::ObservedStop),
        FinalAssistantOutcome::Observed {
            stop_reason: AssistantStopReason::Length,
        } => Ok(PiTaskAttemptAssistantOutcome::ObservedLength),
        FinalAssistantOutcome::Observed {
            stop_reason: AssistantStopReason::Error,
        } => Ok(PiTaskAttemptAssistantOutcome::ObservedError),
        FinalAssistantOutcome::Observed {
            stop_reason: AssistantStopReason::Aborted,
        } => Ok(PiTaskAttemptAssistantOutcome::ObservedAborted),
        FinalAssistantOutcome::Unavailable {
            reason: society_pi::FinalAssistantUnavailableReason::SdkPromiseRejected,
        } => Ok(PiTaskAttemptAssistantOutcome::SdkPromiseRejected),
        FinalAssistantOutcome::Unavailable {
            reason: society_pi::FinalAssistantUnavailableReason::MissingFinalAssistantOutcome,
        } => Ok(PiTaskAttemptAssistantOutcome::MissingFinalAssistantOutcome),
    }
}

fn canonical_create_request_digest(
    request: &PiSpawnRequest,
) -> Result<KernelDigest, PiExecutionError> {
    let frame = InboundFrame {
        sequence: BoundarySequence::parse(1).map_err(PiExecutionError::BoundaryProtocol)?,
        session_identity: request.session_identity.clone(),
        correlation_identity: request.create_correlation_identity.clone(),
        command: InboundCommand::CreateSession(Box::new(request.create_session.clone())),
    };
    let line =
        society_pi::encode_inbound_jsonl(&frame).map_err(PiExecutionError::BoundaryProtocol)?;
    Ok(KernelDigest::of_bytes(line.as_bytes()))
}

fn kernel_workspace_identity(
    request: &PiSpawnRequest,
) -> Result<KernelWorkspaceId, PiExecutionError> {
    KernelWorkspaceId::parse(request.workspace.identity().as_str())
        .map_err(|_| PiExecutionError::IdentityConversion)
}

fn kernel_workspace_path(
    request: &PiSpawnRequest,
) -> Result<CanonicalWorkspacePath, PiExecutionError> {
    let path = request
        .workspace
        .directory()
        .as_path()
        .to_str()
        .ok_or(PiExecutionError::IdentityConversion)?;
    CanonicalWorkspacePath::parse(path).map_err(|_| PiExecutionError::IdentityConversion)
}

fn kernel_session_identity(
    identity: &SessionIdentity,
) -> Result<PiBoundarySessionIdentity, PiExecutionError> {
    PiBoundarySessionIdentity::parse(identity.as_str())
        .map_err(|_| PiExecutionError::IdentityConversion)
}

fn kernel_spawn_nonce(
    nonce: &society_pi::SpawnNonce,
) -> Result<KernelSpawnNonce, PiExecutionError> {
    KernelSpawnNonce::parse(nonce.as_str()).map_err(|_| PiExecutionError::IdentityConversion)
}

fn kernel_correlation(
    correlation: &CorrelationIdentity,
) -> Result<PiCorrelationIdentity, PiExecutionError> {
    PiCorrelationIdentity::parse(correlation.as_str())
        .map_err(|_| PiExecutionError::IdentityConversion)
}

fn kernel_child_identity(
    identity: &SupervisedChildId,
) -> Result<SupervisedChildIdentity, PiExecutionError> {
    SupervisedChildIdentity::parse(identity.as_str())
        .map_err(|_| PiExecutionError::IdentityConversion)
}

fn kernel_child_pid(value: u64) -> Result<NativeChildPid, PiExecutionError> {
    let value = i32::try_from(value).map_err(|_| PiExecutionError::IdentityConversion)?;
    NativeChildPid::try_from(value).map_err(|_| PiExecutionError::IdentityConversion)
}

fn kernel_process_group_id(value: libc::pid_t) -> Result<KernelProcessGroupId, PiExecutionError> {
    KernelProcessGroupId::try_from(value).map_err(|_| PiExecutionError::IdentityConversion)
}

/// `RecordNativeChildNotSpawned` is a durable assertion of physical absence, not
/// a catch-all spawn/setup error. Keep this mapping deliberately small: other
/// failures leave the already-admitted operation fenced for a later exact
/// recovery path rather than fabricating a negative child receipt.
fn proven_not_spawned_reason(
    error: &SupervisionError,
) -> Option<society_kernel::NativeChildNotSpawnedReason> {
    match error {
        SupervisionError::NativeSpawn(_) => {
            Some(society_kernel::NativeChildNotSpawnedReason::NativeSpawnFailed)
        }
        SupervisionError::ArtifactIsNotRegularFile | SupervisionError::ArtifactDigestDrift => {
            Some(society_kernel::NativeChildNotSpawnedReason::ArtifactQualificationFailed)
        }
        SupervisionError::InvalidSpawnRequest => {
            Some(society_kernel::NativeChildNotSpawnedReason::WorkspacePreparationFailed)
        }
        _ => None,
    }
}

fn verify_adapter_facts(
    child: &OfficePiExecutionChild,
    facts: &InertChildFacts,
) -> Result<(), PiExecutionError> {
    if facts.child_process_id != child.supervised_child_id
        || kernel_session_identity(&facts.session_identity)? != child.pi_session_identity
    {
        // `PiSupervisor` already validates the nonce it retained from the
        // immutable spawn request before this bridge receives `InertChildFacts`.
        // The kernel records that same nonce through `RecordPiAdapterReady`;
        // this bridge never fabricates one from another identity.
        return Err(PiExecutionError::AdapterFactMismatch);
    }
    Ok(())
}

fn verify_task_attempt_adapter_facts(
    child: &TaskAttemptPiExecutionChild,
    facts: &InertChildFacts,
) -> Result<(), PiExecutionError> {
    if facts.child_process_id != child.supervised_child_id
        || kernel_session_identity(&facts.session_identity)? != child.pi_session_identity
    {
        return Err(PiExecutionError::AdapterFactMismatch);
    }
    Ok(())
}

fn kernel_liveness(value: crate::supervision::ProcessGroupLiveness) -> KernelLiveness {
    match value {
        crate::supervision::ProcessGroupLiveness::Present => KernelLiveness::Present,
        crate::supervision::ProcessGroupLiveness::Absent => KernelLiveness::Absent,
        crate::supervision::ProcessGroupLiveness::Inaccessible => KernelLiveness::Inaccessible,
    }
}

fn kernel_wait_status(value: ReapStatus) -> Result<DirectChildWaitStatus, PiExecutionError> {
    match value {
        ReapStatus::Exited { code } => ProcessExitCode::try_from(code)
            .map(|exit_code| DirectChildWaitStatus::Exited { exit_code })
            .map_err(|_| PiExecutionError::InvalidWaitStatus),
        ReapStatus::Signaled { signal } => ProcessSignalNumber::try_from(signal)
            .map(|signal_number| DirectChildWaitStatus::Signaled { signal_number })
            .map_err(|_| PiExecutionError::InvalidWaitStatus),
        ReapStatus::Unknown => Ok(DirectChildWaitStatus::Unknown),
    }
}

fn kernel_stream_completeness(capture: &TransientStreamCapture) -> ChildStreamSealCompleteness {
    match (capture.retention, capture.observed_byte_count) {
        (TransientRetention::Complete, TransientByteCount::Exact(_)) => {
            ChildStreamSealCompleteness::Complete
        }
        (TransientRetention::CountOverflow, _) | (_, TransientByteCount::Overflowed) => {
            ChildStreamSealCompleteness::CountOverflow
        }
        (TransientRetention::PrefixBounded, TransientByteCount::Exact(_)) => {
            ChildStreamSealCompleteness::PrefixBounded
        }
    }
}

fn kernel_digest_from_boundary(
    capture: &TransientStreamCapture,
) -> Result<KernelDigest, PiExecutionError> {
    kernel_digest_from_hex(capture.blake3.as_str())
}

/// Converts only the already schema-validated lowercase BLAKE3 spelling from
/// the Pi boundary. The kernel stores bytes, so no display decimal/string is
/// carried into a durable transcript or stream receipt.
fn kernel_digest_from_hex(text: &str) -> Result<KernelDigest, PiExecutionError> {
    let mut bytes = [0_u8; 32];
    let text = text.as_bytes();
    let (pairs, remainder) = text.as_chunks::<2>();
    if !remainder.is_empty() || pairs.len() != bytes.len() {
        return Err(PiExecutionError::BoundaryDigestInvalid);
    }
    for (index, pair) in pairs.iter().enumerate() {
        let high = hex_nibble(pair[0]).ok_or(PiExecutionError::BoundaryDigestInvalid)?;
        let low = hex_nibble(pair[1]).ok_or(PiExecutionError::BoundaryDigestInvalid)?;
        bytes[index] = (high << 4) | low;
    }
    Ok(KernelDigest::from_bytes(bytes))
}

/// Converts a peer-accepted transcript flush receipt into one of the two
/// daemon-private content requests. The `BoundaryPeer` has already verified
/// session identity, configured session-file identity, session-directory
/// relation, header CWD, and the first Prompt rendering digest. This function
/// adds the separate native custody proof before *any* content writer sees
/// the bytes.
fn project_verified_session_transcript(
    operation: &PiOfficeSessionDisposeOperationId,
    workspace_directory: &AbsolutePath,
    session_directory: &AbsolutePath,
    receipt: &TranscriptFlushReceiptV1,
    seal_limit: ContentSealLimit,
) -> Result<VerifiedPiSessionTranscript, PiExecutionError> {
    match receipt {
        TranscriptFlushReceiptV1::Materialized {
            session_file,
            session_file_blake3,
            first_user_prompt,
            ..
        } => {
            let session_file = canonical_transcript_path(session_file)?;
            let expected_digest = kernel_digest_from_hex(session_file_blake3.as_str())?;
            let bytes = read_verified_transcript_bytes(
                workspace_directory,
                session_directory,
                session_file.as_str(),
                seal_limit,
            )?;
            if KernelDigest::of_bytes(&bytes) != expected_digest {
                return Err(PiExecutionError::TranscriptDigestMismatch);
            }
            let first_user_prompt = match first_user_prompt {
                FirstUserPromptReceipt::Absent => PiOfficeSessionFirstUserPromptReceipt::Absent,
                FirstUserPromptReceipt::Verified { digest } => {
                    PiOfficeSessionFirstUserPromptReceipt::Verified {
                        digest: kernel_digest_from_hex(digest.as_str())?,
                    }
                }
            };
            Ok(VerifiedPiSessionTranscript::Materialized(
                VerifiedPiSessionTranscriptSealRequest {
                    content_operation: operation.transcript_content_operation(expected_digest)?,
                    session_file,
                    session_file_digest: expected_digest,
                    first_user_prompt,
                    bytes,
                },
            ))
        }
        TranscriptFlushReceiptV1::UnmaterializedNoPrompt { session_file, .. } => {
            Ok(VerifiedPiSessionTranscript::UnmaterializedNoPrompt {
                session_file: canonical_transcript_path(session_file)?,
            })
        }
    }
}

/// TaskAttempt counterpart of [`project_verified_session_transcript`]. The
/// filesystem proof and received bytes are identical, but its content-seal
/// operation is a separate task-disposal namespace so retrying one actor close
/// can never reuse an Office transcript registration.
fn project_task_attempt_session_transcript(
    operation: &PiTaskAttemptSessionDisposeOperationId,
    workspace_directory: &AbsolutePath,
    session_directory: &AbsolutePath,
    receipt: &TranscriptFlushReceiptV1,
    seal_limit: ContentSealLimit,
) -> Result<VerifiedPiSessionTranscript, PiExecutionError> {
    match receipt {
        TranscriptFlushReceiptV1::Materialized {
            session_file,
            session_file_blake3,
            first_user_prompt,
            ..
        } => {
            let session_file = canonical_transcript_path(session_file)?;
            let expected_digest = kernel_digest_from_hex(session_file_blake3.as_str())?;
            let bytes = read_verified_transcript_bytes(
                workspace_directory,
                session_directory,
                session_file.as_str(),
                seal_limit,
            )?;
            if KernelDigest::of_bytes(&bytes) != expected_digest {
                return Err(PiExecutionError::TranscriptDigestMismatch);
            }
            let first_user_prompt = match first_user_prompt {
                FirstUserPromptReceipt::Absent => PiOfficeSessionFirstUserPromptReceipt::Absent,
                FirstUserPromptReceipt::Verified { digest } => {
                    PiOfficeSessionFirstUserPromptReceipt::Verified {
                        digest: kernel_digest_from_hex(digest.as_str())?,
                    }
                }
            };
            Ok(VerifiedPiSessionTranscript::Materialized(
                VerifiedPiSessionTranscriptSealRequest {
                    content_operation: operation.transcript_content_operation(expected_digest)?,
                    session_file,
                    session_file_digest: expected_digest,
                    first_user_prompt,
                    bytes,
                },
            ))
        }
        TranscriptFlushReceiptV1::UnmaterializedNoPrompt { session_file, .. } => {
            Ok(VerifiedPiSessionTranscript::UnmaterializedNoPrompt {
                session_file: canonical_transcript_path(session_file)?,
            })
        }
    }
}

fn canonical_transcript_path(
    session_file: &AbsolutePath,
) -> Result<CanonicalPiSessionTranscriptPath, PiExecutionError> {
    CanonicalPiSessionTranscriptPath::parse(session_file.as_str())
        .map_err(|_| PiExecutionError::TranscriptPathNotCanonical)
}

/// Opens a materialized Pi transcript once, without following the final path
/// component, and returns at most the exact content-store seal limit. The
/// caller never trusts a host-supplied filename alone: all paths must remain
/// canonical direct descendants of the native workspace/session directories
/// that this child received before CreateSession.
fn read_verified_transcript_bytes(
    workspace_directory: &AbsolutePath,
    session_directory: &AbsolutePath,
    session_file: &str,
    seal_limit: ContentSealLimit,
) -> Result<Vec<u8>, PiExecutionError> {
    let session_file = AbsolutePath::parse(session_file)
        .map_err(|_| PiExecutionError::TranscriptPathNotCanonical)?;
    if !session_directory.is_strict_descendant_of(workspace_directory)
        || !session_file.is_strict_descendant_of(session_directory)
    {
        return Err(PiExecutionError::TranscriptOutsideOwnedWorkspace);
    }

    let canonical_workspace = canonical_owned_directory(workspace_directory)?;
    let canonical_session_directory = canonical_owned_directory(session_directory)?;
    if !path_is_strict_descendant(&canonical_session_directory, &canonical_workspace) {
        return Err(PiExecutionError::TranscriptOutsideOwnedWorkspace);
    }

    // Reject a final symlink before canonicalizing.  Canonicalizing first
    // would make a symlink-to-an-in-tree file look like a normal receipt and
    // would weaken the explicit no-follow custody boundary below.
    let listed_before_open = fs::symlink_metadata(session_file.as_path())
        .map_err(PiExecutionError::TranscriptFilesystem)?;
    if !same_user_regular_file(&listed_before_open) {
        return Err(PiExecutionError::TranscriptFileUnsafe);
    }

    // A materialized receipt is supposed to report a resolved canonical path.
    // Require it directly rather than allowing an ancestor symlink to change
    // the document selected by a later content seal.
    let resolved_before_open =
        fs::canonicalize(session_file.as_path()).map_err(PiExecutionError::TranscriptFilesystem)?;
    let resolved_before_open = resolved_before_open
        .to_str()
        .ok_or(PiExecutionError::TranscriptPathNotCanonical)?;
    if resolved_before_open != session_file.as_str()
        || !path_is_strict_descendant(resolved_before_open, &canonical_session_directory)
    {
        return Err(PiExecutionError::TranscriptOutsideOwnedWorkspace);
    }

    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(session_file.as_path())
        .map_err(|error| match error.raw_os_error() {
            Some(libc::ELOOP) => PiExecutionError::TranscriptFileUnsafe,
            _ => PiExecutionError::TranscriptFilesystem(error),
        })?;
    let opened = file
        .metadata()
        .map_err(PiExecutionError::TranscriptFilesystem)?;
    if !same_user_regular_file(&opened) {
        return Err(PiExecutionError::TranscriptFileUnsafe);
    }

    // The path is checked again after opening. Comparing inode/device pins the
    // bytes we will read to the current no-follow directory entry rather than
    // a replaced same-named file selected during the earlier realpath check.
    let listed_after_open = fs::symlink_metadata(session_file.as_path())
        .map_err(PiExecutionError::TranscriptFilesystem)?;
    if !same_user_regular_file(&listed_after_open)
        || listed_after_open.dev() != opened.dev()
        || listed_after_open.ino() != opened.ino()
    {
        return Err(PiExecutionError::TranscriptFileUnsafe);
    }
    let resolved_after_open =
        fs::canonicalize(session_file.as_path()).map_err(PiExecutionError::TranscriptFilesystem)?;
    let resolved_after_open = resolved_after_open
        .to_str()
        .ok_or(PiExecutionError::TranscriptPathNotCanonical)?;
    if resolved_after_open != session_file.as_str()
        || !path_is_strict_descendant(resolved_after_open, &canonical_session_directory)
    {
        return Err(PiExecutionError::TranscriptOutsideOwnedWorkspace);
    }

    let read_limit = seal_limit
        .bytes()
        .checked_add(1)
        .ok_or(PiExecutionError::TranscriptLimitInvalid)?;
    let mut bytes = Vec::new();
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(PiExecutionError::TranscriptFilesystem)?;
    if u64::try_from(bytes.len()).map_err(|_| PiExecutionError::TranscriptLimitInvalid)?
        > seal_limit.bytes()
    {
        return Err(PiExecutionError::TranscriptSizeLimitExceeded);
    }
    Ok(bytes)
}

fn canonical_owned_directory(directory: &AbsolutePath) -> Result<String, PiExecutionError> {
    let listed = fs::symlink_metadata(directory.as_path())
        .map_err(PiExecutionError::TranscriptFilesystem)?;
    if listed.file_type().is_symlink() || !same_user_directory(&listed) {
        return Err(PiExecutionError::TranscriptOutsideOwnedWorkspace);
    }
    let resolved =
        fs::canonicalize(directory.as_path()).map_err(PiExecutionError::TranscriptFilesystem)?;
    let resolved = resolved
        .to_str()
        .ok_or(PiExecutionError::TranscriptPathNotCanonical)?
        .to_owned();
    if resolved != directory.as_str() {
        return Err(PiExecutionError::TranscriptOutsideOwnedWorkspace);
    }
    Ok(resolved)
}

fn path_is_strict_descendant(path: &str, base: &str) -> bool {
    path.starts_with(base) && path.as_bytes().get(base.len()) == Some(&b'/')
}

fn same_user_regular_file(metadata: &fs::Metadata) -> bool {
    metadata.is_file()
        && !metadata.file_type().is_symlink()
        && metadata.uid() == effective_uid()
        // The daemon owns transcript custody. A group/world-readable file
        // can leak the peer-projected transcript, and an extra hard link can
        // alias an unrelated same-user document into the owned session tree.
        && metadata.mode() & 0o077 == 0
        && metadata.nlink() == 1
}

fn same_user_directory(metadata: &fs::Metadata) -> bool {
    metadata.is_dir()
        && !metadata.file_type().is_symlink()
        && metadata.uid() == effective_uid()
        // NativeWorkspace creates these directories with 0700. Retaining
        // that proof here prevents a host from redirecting a valid lexical
        // path into a group/world-visible custody boundary.
        && metadata.mode() & 0o077 == 0
}

fn effective_uid() -> libc::uid_t {
    // SAFETY: `geteuid` has no preconditions and does not retain pointers.
    unsafe { libc::geteuid() }
}

const fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

#[derive(Debug, Error)]
pub(crate) enum PiExecutionError {
    #[error("daemon restart recovery is fenced before Pi process work can resume")]
    RecoveryFenced,
    #[error("Pi execution operation identity is not canonical")]
    InvalidOperationIdentity,
    #[error("daemon/runtime identities could not be converted to the exact kernel identities")]
    IdentityConversion,
    #[error("Pi boundary protocol construction failed: {0}")]
    BoundaryProtocol(#[from] society_pi::ProtocolError),
    #[error("Pi supervisor failed: {0}")]
    Supervision(#[from] SupervisionError),
    #[error("kernel failed: {0}")]
    Kernel(#[from] society_kernel::StoreError),
    #[error("content sealing failed: {0}")]
    Content(#[from] ContentSealingError),
    #[error("kernel service capability {capability:?} is not active")]
    KernelServiceCapabilityMissing { capability: Capability },
    #[error("kernel rejected daemon-only {capability:?}: {rejection:?}")]
    KernelCommandRejected {
        capability: Capability,
        rejection: society_kernel::Rejection,
    },
    #[error("accepted kernel command returned an unexpected event body")]
    UnexpectedKernelEvent,
    #[error("Office Pi child transition is invalid in its current phase")]
    InvalidLifecycle,
    #[error("an Office Pi child requires the exact RootAuthorityOffice session kind")]
    OfficeSessionKindRequired,
    #[error("a TaskAttempt Pi child requires the exact TaskAttempt session kind")]
    TaskAttemptSessionKindRequired,
    #[error("the supplied Office prompt text is empty or differs from its sealed digest")]
    PromptContentDigestMismatch,
    #[error("validated Pi usage could not be represented by the exact kernel accounting types")]
    UsageConversion,
    #[error("Pi Prompt evidence arrived outside the closed M6 order")]
    PromptEvidenceOrder,
    #[error("the final Prompt terminal lacks AgentSettled or final accounting evidence")]
    PromptTerminalEvidenceMissing,
    #[error("the Pi host rejected an already delivered Office Prompt")]
    PromptRejectedByHost,
    #[error("Pi Dispose evidence arrived outside the closed session-finalization order")]
    DisposeEvidenceOrder,
    #[error("the Pi host rejected an already delivered Office-session Dispose")]
    DisposeRejectedByHost,
    #[error("the Pi host became fatal without an exact M6 accounting-failure frame")]
    PeerFatalWithoutAccountingFact,
    #[error("peer-reported Pi transcript path is not canonical")]
    TranscriptPathNotCanonical,
    #[error("Pi transcript escaped the child-owned workspace/session directory")]
    TranscriptOutsideOwnedWorkspace,
    #[error("Pi transcript is not a same-user regular no-follow file")]
    TranscriptFileUnsafe,
    #[error("Pi transcript read failed: {0}")]
    TranscriptFilesystem(#[source] std::io::Error),
    #[error("Pi transcript exceeds the daemon content-seal limit")]
    TranscriptSizeLimitExceeded,
    #[error("Pi transcript limit could not be represented by the native reader")]
    TranscriptLimitInvalid,
    #[error("Pi transcript bytes do not match the peer-validated BLAKE3 receipt")]
    TranscriptDigestMismatch,
    #[error("materialized Pi transcript has no physical content-object registration")]
    TranscriptContentMissing,
    #[error("unmaterialized Pi transcript must not create a content object")]
    TranscriptContentUnexpected,
    #[error("Pi Create authorization gate was not called by the supervisor")]
    CreateGateNotInvoked,
    #[error("AdapterReady facts did not match the durable child identity")]
    AdapterFactMismatch,
    #[error("direct child exited before SessionReady could be recorded")]
    ExitedBeforeSessionReady,
    #[error(
        "direct child exited after Pi SessionReady but before Office readiness could be recorded"
    )]
    ExitedBeforeOfficeReady,
    #[error("supervisor receipt names another child")]
    ReceiptIdentityMismatch,
    #[error("supervisor completed without a direct-child wait receipt")]
    MissingReapReceipt,
    #[error("supervisor dropped a completed receipt before durable reconciliation")]
    ReapReceiptLost,
    #[error("direct-child wait status was outside the kernel's closed range")]
    InvalidWaitStatus,
    #[error(
        "a receipt contains an SDK Abort control delivery without a typed cancellation-propagation relation"
    )]
    UnmodeledAbortControlReceipt,
    #[error("signal receipts require a durable direct-reap-before-lingering-cleanup transition")]
    SignalReceiptOrderingRequiresTwoPhaseReap,
    #[error(
        "automatic containment recorded an inaccessible signal observation; the child is terminally containment-failed"
    )]
    AutomaticContainmentInaccessible,
    #[error("the owned process group became inaccessible; the kernel recorded containment failure")]
    LingeringGroupInaccessible,
    #[error(
        "the owned process group was observed absent and later present; the kernel recorded possible identity reuse"
    )]
    ProcessGroupIdentityRegressed,
    #[error("Pi boundary digest was not canonical lowercase BLAKE3")]
    BoundaryDigestInvalid,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
        path::{Path, PathBuf},
        process::Command,
        sync::{
            Mutex, MutexGuard, OnceLock,
            atomic::{AtomicU64, Ordering},
        },
        thread,
        time::Duration,
    };

    use society_content::{ContentSealLimit, ContentStoreRoot};
    use society_kernel::{
        ActorAttemptId, ActorConfigurationName, ActorConfigurationRevisionId, ActorInstanceId,
        ActorModelPolicy, AdmissionGeneration, ApplicationIdentity, ApplicationMissionInput,
        ApplicationName, ApplicationRevisionId, ApplicationRevisionOrdinal,
        Blake3Digest as KernelDigest, BudgetReservationId, Capability, CommandBody,
        CommandDisposition, CommandId, CommandRequest, ContextPackId, ContextPackPurpose,
        DevelopmentalAttractor, ExpectedGeneration, KernelStore, MissionPrinciple,
        MissionPrincipleKind, MissionPrincipleText, MissionPrinciples, MissionStatement,
        NorthStarBoundaryCommitmentQuestion, NorthStarChangeQuestion,
        NorthStarImprovementEvidenceQuestion, NorthStarQuestionSet, NorthStarRevisitQuestion,
        OfficeTurnId, OfficeTurnPurpose, OperatingCycleId, OperatingCycleTreatment,
        PiCorrelationIdentity, PiProtocolSequence, PrincipalDisplayName, PrincipalId, ProjectId,
        ProjectMilestoneName, ProjectName, ProjectNorthStarAlignment,
        ProjectNorthStarBoundaryCommitmentAnswer, ProjectNorthStarChangeAnswer,
        ProjectNorthStarImprovementEvidenceAnswer, ProjectNorthStarRevisitAnswer,
        ProjectObjectiveText, ProjectState, ProjectStopConditionText, Rejection,
        RootAuthorityOfficeSessionId, SocietyName, SupervisorEpochId, SupervisorEpochIdentity,
        TicketAcceptanceConditionText, TicketId, TicketTitle, UsdMicros, WorkAssignmentText,
        WorkItemId, WorkItemKind,
    };
    #[cfg(feature = "test-support")]
    use society_kernel::{CancellationMode, CancellationPropagationId, CancellationRequestId};
    use society_pi::{
        AbsolutePath, ActorModelPolicyV1, AdapterVersion, Blake3Digest, CacheWritePerMillionRateV1,
        CanonicalModelSlug, CompactionMode, CompactionPolicyV1, CorrelationIdentity,
        CreateSessionPayload, Disabled, EffectiveModelDescriptorV1, ForumSessionContractV1, Images,
        KnownPerMillionRateV1, ModelApi, ModelCatalogPolicyV1, ModelId, ModelInput, ModelSelection,
        NodeRuntimeVersion, NonNegativeInteger, OpenRouterBaseUrl, PiSdkVersion, PositiveInteger,
        ProjectTrust, Provider, QueueMode, RetryPolicyV1, RuntimeIdentity, SessionIdentity,
        SessionKind, SpawnNonce, ThinkingLevel, ToolProfile, TranscriptFlushReceiptV1, Transport,
        UsdPerMillionDecimal,
    };

    use super::{
        OfficePiExecutionStart, OfficePiSessionDisposeOutput, OfficePiSessionDisposeStart,
        OfficePiSpawnRegistration, OfficePiTurnOutput, OfficePiTurnStart, PiExecutionDriver,
        PiExecutionOperationId, PiOfficeSessionDisposeCommand, PiOfficeSessionDisposeOperationId,
        PiOfficeTurnCommand, PiOfficeTurnOperationId, PiTaskAttemptPromptCommand,
        PiTaskAttemptPromptOperationId, PiTaskAttemptSessionDisposeCommand,
        PiTaskAttemptSessionDisposeOperationId, SealedOfficePrompt, SealedTaskAttemptPrompt,
        TaskAttemptPiExecutionStart, TaskAttemptPiPromptOutput, TaskAttemptPiPromptStart,
        TaskAttemptPiSessionDisposeOutput, TaskAttemptPiSessionDisposeStart,
        TaskAttemptPiSpawnRegistration, VerifiedPiSessionTranscript,
        project_verified_session_transcript, read_verified_transcript_bytes,
    };

    #[test]
    fn task_attempt_operation_namespaces_cannot_alias_office_receipts() {
        let label = "same-operation-label";
        let office = PiOfficeTurnOperationId::parse(label).unwrap();
        let task_prompt = PiTaskAttemptPromptOperationId::parse(label).unwrap();
        let task_dispose = PiTaskAttemptSessionDisposeOperationId::parse(label).unwrap();
        assert_ne!(
            office
                .command_id(PiOfficeTurnCommand::AuthorizePrompt)
                .unwrap(),
            task_prompt
                .command_id(PiTaskAttemptPromptCommand::AuthorizePrompt)
                .unwrap()
        );
        assert_ne!(
            task_prompt
                .command_id(PiTaskAttemptPromptCommand::RecordTerminal)
                .unwrap(),
            task_dispose
                .command_id(PiTaskAttemptSessionDisposeCommand::RecordDisposed)
                .unwrap()
        );
    }

    #[test]
    fn sealed_task_assignment_rejects_digest_drift() {
        let text = "only exact task assignment bytes may reach Pi".to_owned();
        assert!(
            SealedTaskAttemptPrompt::new(text.clone(), KernelDigest::of_bytes(text.as_bytes()),)
                .is_ok()
        );
        assert!(SealedTaskAttemptPrompt::new(text, KernelDigest::of_bytes(b"different"),).is_err());
    }
    use crate::{
        content::{ContentSealOperationId, ContentSealingAuthority},
        supervision::{
            ControlWriteDeadline, HandshakeDeadline, MonotonicTick, NativeHostEnvironment,
            NativeWorkspace, NativeWorkspaceId, NativeWorkspaceRoot, PiSpawnRequest,
            QualifiedHostExecution, SupervisedChildId, VerifiedArtifact,
        },
    };

    #[test]
    fn task_attempt_path_rejects_root_office_session_before_admission() {
        let fixture = NativeFixture::new("task-attempt-office-session");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "task-attempt-office-session");
        let mut driver = PiExecutionDriver::new();
        let error = driver
            .admit_task_attempt_spawn_and_register(
                &mut store,
                TaskAttemptPiExecutionStart {
                    operation: PiExecutionOperationId::parse("task-attempt-office-session")
                        .unwrap(),
                    operating_cycle_id: office.cycle_id,
                    actor_attempt_id: society_kernel::ActorAttemptId::new(1).unwrap(),
                    budget_reservation_id: BudgetReservationId::new(1).unwrap(),
                    execution_profile_id:
                        society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
                    expected_generation: AdmissionGeneration::INITIAL,
                    supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
                    supervisor_epoch_identity: office.epoch_identity,
                    spawn_request: fixture.spawn_request(),
                },
            )
            .unwrap_err();
        assert!(matches!(
            error,
            super::PiExecutionError::TaskAttemptSessionKindRequired
        ));
        fixture.cleanup();
    }

    #[test]
    fn task_attempt_path_rejects_non_running_actor_owner_before_native_spawn() {
        let fixture = NativeFixture::new("task-attempt-invalid-owner");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "task-attempt-invalid-owner");
        let mut request = fixture.spawn_request();
        request.create_session.session_kind = SessionKind::TaskAttempt;
        let mut driver = PiExecutionDriver::new();
        let error = driver
            .admit_task_attempt_spawn_and_register(
                &mut store,
                TaskAttemptPiExecutionStart {
                    operation: PiExecutionOperationId::parse("task-attempt-invalid-owner").unwrap(),
                    operating_cycle_id: office.cycle_id,
                    // No ActorAttempt with this identity is running. The
                    // kernel must reject the closed ActorAttempt owner before
                    // this bridge asks the supervisor to spawn anything.
                    actor_attempt_id: society_kernel::ActorAttemptId::new(1).unwrap(),
                    budget_reservation_id: BudgetReservationId::new(1).unwrap(),
                    execution_profile_id:
                        society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
                    expected_generation: AdmissionGeneration::INITIAL,
                    supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
                    supervisor_epoch_identity: office.epoch_identity,
                    spawn_request: request,
                },
            )
            .unwrap_err();
        assert!(matches!(
            error,
            super::PiExecutionError::KernelCommandRejected {
                capability: Capability::AdmitPiChildSpawn,
                rejection: Rejection::ChildSpawnAdmissionInvalid,
            }
        ));
        fixture.cleanup();
    }

    /// A TaskAttempt session must prove the same physical suffix as an Office
    /// session without borrowing either Office identity or Office prompt
    /// authority. This is intentionally a native-child regression rather than
    /// a synthetic kernel receipt chain: it exercises the actual Create,
    /// TaskAssignment, final accounting, transcript custody, Dispose, and
    /// direct-child reconciliation boundaries an admitted live study runner
    /// will rely on.
    #[test]
    fn task_attempt_native_child_runs_prompt_dispose_and_reconciles() {
        let fixture = NativeFixture::new("task-attempt-full-lifecycle");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "task-attempt-full-lifecycle");
        let task = running_task_attempt(&mut store, &office);
        let mut driver = PiExecutionDriver::new();
        let start = TaskAttemptPiExecutionStart {
            operation: PiExecutionOperationId::parse("task-attempt-full-lifecycle").unwrap(),
            operating_cycle_id: office.cycle_id,
            actor_attempt_id: task.actor_attempt_id,
            budget_reservation_id: task.budget_reservation_id,
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            expected_generation: AdmissionGeneration::INITIAL,
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: office.epoch_identity.clone(),
            spawn_request: fixture.task_attempt_spawn_request(),
        };
        let mut child = match driver
            .admit_task_attempt_spawn_and_register(&mut store, start)
            .unwrap()
        {
            TaskAttemptPiSpawnRegistration::Ready(child) => child,
            other => panic!("task fixture must start a registered child: {other:?}"),
        };
        assert_eq!(child.actor_attempt_id(), task.actor_attempt_id);
        wait_for_task_attempt_adapter_ready(&mut driver, &mut store, &mut child);
        let create_progress = driver
            .authorize_and_begin_task_attempt_create(
                &mut store,
                &mut child,
                MonotonicTick::from_milliseconds(1),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        if create_progress == crate::supervision::ControlWriteProgress::Pending {
            drive_task_attempt_create_until_delivered(
                &mut driver,
                &mut store,
                &mut child,
                2,
                1_000,
            );
        }
        wait_for_task_attempt_session_ready(&mut driver, &mut store, &mut child);

        let prompt_text = "Complete one bounded disposable-actor task.";
        let prompt_content = seal_prompt_content(
            &mut store,
            &fixture,
            "task-attempt-full-prompt",
            prompt_text,
        );
        // Prompt authorization binds the exact current accepted ledger
        // frontier, which is the content-registration event just committed by
        // `seal_prompt_content`; an earlier actor-start event would permit a
        // stale prompt to overtake newly sealed content.
        let prompt_content_operation =
            ContentSealOperationId::parse("task-attempt-full-prompt", prompt_content.digest)
                .unwrap();
        let prompt_frontier_event_id = match store
            .command_receipt(prompt_content_operation.register_content_object_command_id())
            .unwrap()
            .unwrap()
            .disposition
        {
            CommandDisposition::Accepted(event_id) => event_id,
            other => panic!("task prompt content must have a registration event: {other:?}"),
        };
        let (mut prompt, prompt_progress) = driver
            .authorize_and_begin_task_attempt_prompt(
                &mut store,
                &mut child,
                TaskAttemptPiPromptStart {
                    operation: PiTaskAttemptPromptOperationId::parse("task-attempt-full-prompt")
                        .unwrap(),
                    correlation_identity: PiCorrelationIdentity::parse("task-attempt-full-prompt")
                        .unwrap(),
                    prompt_content_object_id: prompt_content.content_object_id,
                    prompt: SealedTaskAttemptPrompt::new(
                        prompt_text.to_owned(),
                        prompt_content.digest,
                    )
                    .unwrap(),
                    frontier_event_id: prompt_frontier_event_id,
                },
                MonotonicTick::from_milliseconds(1_001),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(2_000)),
            )
            .unwrap();
        if prompt_progress == crate::supervision::ControlWriteProgress::Pending {
            drive_task_attempt_prompt_until_delivered(
                &mut driver,
                &mut store,
                &mut child,
                &mut prompt,
                1_002,
                2_000,
            );
        }
        let mut terminal_recorded = false;
        for tick in 1_002..3_000 {
            let Some(output) = driver
                .observe_task_attempt_prompt_output(
                    &mut store,
                    &mut child,
                    &mut prompt,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            else {
                thread::sleep(Duration::from_millis(1));
                continue;
            };
            if matches!(output, TaskAttemptPiPromptOutput::TerminalRecorded) {
                terminal_recorded = true;
                break;
            }
        }
        assert!(
            terminal_recorded,
            "TaskAssignment must reach a durable terminal receipt"
        );
        assert_eq!(child.phase(), "task_prompt_terminal_recorded");

        let (mut dispose, dispose_progress) = driver
            .begin_task_attempt_session_dispose(
                &mut store,
                &mut child,
                TaskAttemptPiSessionDisposeStart {
                    operation: PiTaskAttemptSessionDisposeOperationId::parse(
                        "task-attempt-full-dispose",
                    )
                    .unwrap(),
                    correlation_identity: PiCorrelationIdentity::parse("task-attempt-full-dispose")
                        .unwrap(),
                },
                MonotonicTick::from_milliseconds(3_001),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(4_000)),
            )
            .unwrap();
        if dispose_progress == crate::supervision::ControlWriteProgress::Pending {
            drive_task_attempt_dispose_until_delivered(
                &mut driver,
                &mut store,
                &mut child,
                &mut dispose,
                3_002,
                4_000,
            );
        }
        let transcript_content = ContentSealingAuthority::open(
            ContentStoreRoot::parse(fixture.root.join("task-attempt-transcript-content")).unwrap(),
            ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
        )
        .unwrap();
        let mut disposed = false;
        for tick in 3_002..5_000 {
            let Some(output) = driver
                .observe_task_attempt_session_dispose_output(
                    &mut store,
                    &mut child,
                    &mut dispose,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(5_000)),
                    ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
                )
                .unwrap()
            else {
                thread::sleep(Duration::from_millis(1));
                continue;
            };
            if let TaskAttemptPiSessionDisposeOutput::TranscriptReady(terminal) = output {
                let sealed_content = match terminal.transcript() {
                    VerifiedPiSessionTranscript::Materialized(request) => Some(
                        transcript_content
                            .seal_and_register(
                                &mut store,
                                request.content_operation(),
                                request.bytes(),
                            )
                            .unwrap(),
                    ),
                    VerifiedPiSessionTranscript::UnmaterializedNoPrompt { .. } => {
                        panic!("a completed TaskAssignment must materialize a transcript")
                    }
                };
                assert!(matches!(
                    driver
                        .record_task_attempt_session_disposed(
                            &mut store,
                            &mut child,
                            &mut dispose,
                            &terminal,
                            sealed_content,
                            MonotonicTick::from_milliseconds(tick),
                        )
                        .unwrap(),
                    TaskAttemptPiSessionDisposeOutput::Disposed
                ));
                disposed = true;
                break;
            }
        }
        assert!(
            disposed,
            "TaskAttempt disposal must commit after transcript custody"
        );
        assert_eq!(child.phase(), "disposed");
        for tick in 5_000..6_000 {
            if driver
                .poll_task_attempt_reap_and_reconcile(
                    &mut store,
                    &transcript_content,
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(child.phase(), "reconciled");
        store.validate_replayed_materialized_state().unwrap();
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(1);
    static PROCESS_PHYSICS_FIXTURE_GUARD: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn provider_free_office_create_dispose_reap_records_ready_only_after_live_session() {
        let fixture = NativeFixture::new("m5-office-bridge");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m5-office-bridge");
        let start = OfficePiExecutionStart {
            operation: PiExecutionOperationId::parse("m5-office-bridge").unwrap(),
            operating_cycle_id: office.cycle_id,
            office_session_id: office.session_id,
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            expected_generation: AdmissionGeneration::INITIAL,
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: office.epoch_identity,
            spawn_request: fixture.spawn_request(),
        };
        let mut driver = PiExecutionDriver::new();
        let mut child = match driver.admit_spawn_and_register(&mut store, start).unwrap() {
            OfficePiSpawnRegistration::Ready(child) => child,
            other => panic!("ordinary provider-free host fixture must complete setup: {other:?}"),
        };

        rejected_open_office_turn(&mut store, office.session_id, "before-office-ready");

        let adapter_ready = driver
            .observe_adapter_ready(
                &mut store,
                &mut child,
                MonotonicTick::ZERO,
                HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        if !adapter_ready {
            for tick in 0..1_000 {
                if driver
                    .observe_adapter_ready(
                        &mut store,
                        &mut child,
                        MonotonicTick::from_milliseconds(tick),
                        HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
                    )
                    .unwrap()
                {
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }
        }

        let progress = driver
            .authorize_and_begin_create(
                &mut store,
                &mut child,
                MonotonicTick::from_milliseconds(1),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        if progress == crate::supervision::ControlWriteProgress::Pending {
            for tick in 2..1_000 {
                if driver
                    .drive_create_delivery(
                        &mut store,
                        &mut child,
                        MonotonicTick::from_milliseconds(tick),
                    )
                    .unwrap()
                    == crate::supervision::ControlWriteProgress::Delivered
                {
                    break;
                }
            }
        }
        for tick in 2..1_000 {
            if driver
                .observe_session_ready(
                    &mut store,
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
                )
                .unwrap()
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(child.phase(), "office_ready_recorded");

        let dispose_progress = driver
            .begin_dispose(
                &mut child,
                CorrelationIdentity::parse("dispose-office-bridge").unwrap(),
                MonotonicTick::from_milliseconds(1_001),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(2_000)),
            )
            .unwrap();
        if dispose_progress == crate::supervision::ControlWriteProgress::Pending {
            for tick in 1_002..2_000 {
                if driver
                    .drive_dispose_delivery(&mut child, MonotonicTick::from_milliseconds(tick))
                    .unwrap()
                    == crate::supervision::ControlWriteProgress::Delivered
                {
                    break;
                }
            }
        }
        for tick in 1_002..2_000 {
            if driver
                .observe_disposed(
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(2_000)),
                )
                .unwrap()
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }

        let content_root = fixture.root.join("content");
        let content = ContentSealingAuthority::open(
            ContentStoreRoot::parse(content_root).unwrap(),
            ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
        )
        .unwrap();
        let mut reconciled = false;
        for tick in 2_000..3_000 {
            if driver
                .poll_reap_and_reconcile(
                    &mut store,
                    &content,
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                reconciled = true;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(reconciled, "direct child must be reaped and sealed");
        assert_eq!(child.phase(), "reconciled");
        rejected_open_office_turn(&mut store, office.session_id, "after-child-finalization");
        fixture.cleanup();
    }

    #[test]
    fn m6_forum_call_waits_for_resident_result_before_terminal_evidence() {
        let mut fixture = NativeFixture::new("m6-forum-call");
        fixture.create.tool_profile = ToolProfile::ForumIsolatedV1;
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-forum-call");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m6-forum-call-child",
        );
        let prompt_text = "A resident must authorize each Forum call before it can settle.";
        let prompt_content = seal_prompt_content(
            &mut store,
            &fixture,
            "m6-forum-call-prompt-content",
            prompt_text,
        );
        let (turn_id, frontier_event_id) =
            open_office_turn(&mut store, office.session_id, "m6-forum-call-open-turn");
        let (mut turn, progress) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-forum-call-turn",
                    turn_id,
                    "m6-forum-call-correlation",
                    prompt_content.content_object_id,
                    prompt_content.digest,
                    prompt_text,
                    frontier_event_id,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        if progress == crate::supervision::ControlWriteProgress::Pending {
            drive_prompt_until_delivered(
                &mut driver,
                &mut store,
                &mut child,
                &mut turn,
                101,
                1_000,
            );
        }

        let mut saw_call = false;
        let mut saw_terminal = false;
        for tick in 101..3_000 {
            let Some(output) = driver
                .observe_office_turn_output(
                    &mut store,
                    &mut child,
                    &mut turn,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            else {
                thread::sleep(Duration::from_millis(1));
                continue;
            };
            match output {
                OfficePiTurnOutput::ForumToolCall {
                    correlation_identity,
                    tool_call_identity,
                    tool_name,
                    args,
                } => {
                    assert_eq!(correlation_identity.as_str(), "m6-forum-call-correlation");
                    assert_eq!(tool_call_identity.as_str(), "forum-call-1");
                    assert_eq!(tool_name, society_pi::ForumToolName::SocietyForumPost);
                    assert_eq!(
                        society_pi::decode_forum_tool_arguments(tool_name, &args).unwrap(),
                        society_pi::ForumToolArguments::Post {
                            message_kind: society_pi::ForumMessageKind::Finding,
                            body_utf8: "provider-free Forum bridge observation".to_owned(),
                            in_reply_to_message_id: None,
                            supersedes_message_id: None,
                        }
                    );
                    saw_call = true;
                    let result_progress = driver
                        .send_office_forum_tool_result(
                            &mut child,
                            &turn,
                            tool_call_identity,
                            society_pi::SdkJsonValue::String(
                                "resident-authorized-result".to_owned(),
                            ),
                            false,
                            MonotonicTick::from_milliseconds(tick),
                            ControlWriteDeadline::at(MonotonicTick::from_milliseconds(3_000)),
                        )
                        .unwrap();
                    if result_progress == crate::supervision::ControlWriteProgress::Pending {
                        for result_tick in tick..3_000 {
                            if driver
                                .drive_office_forum_tool_result_delivery(
                                    &mut child,
                                    MonotonicTick::from_milliseconds(result_tick),
                                )
                                .unwrap()
                                == crate::supervision::ControlWriteProgress::Delivered
                            {
                                break;
                            }
                            thread::sleep(Duration::from_millis(1));
                        }
                    }
                }
                OfficePiTurnOutput::SettledReady => {
                    saw_terminal = true;
                    break;
                }
                OfficePiTurnOutput::TerminalRecordedNonReady => break,
                _ => {}
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(
            saw_call,
            "the validated Forum request must reach the resident"
        );
        assert!(
            saw_terminal,
            "the host must not settle before its result is delivered"
        );

        let dispose_progress = driver
            .begin_dispose(
                &mut child,
                CorrelationIdentity::parse("m6-forum-call-dispose").unwrap(),
                MonotonicTick::from_milliseconds(3_001),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(4_000)),
            )
            .unwrap();
        if dispose_progress == crate::supervision::ControlWriteProgress::Pending {
            for tick in 3_002..4_000 {
                if driver
                    .drive_dispose_delivery(&mut child, MonotonicTick::from_milliseconds(tick))
                    .unwrap()
                    == crate::supervision::ControlWriteProgress::Delivered
                {
                    break;
                }
            }
        }
        for tick in 3_002..4_000 {
            if driver
                .observe_disposed(
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(4_000)),
                )
                .unwrap()
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        let content = ContentSealingAuthority::open(
            ContentStoreRoot::parse(fixture.root.join("m6-forum-call-content")).unwrap(),
            ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
        )
        .unwrap();
        for tick in 4_000..5_000 {
            if driver
                .poll_reap_and_reconcile(
                    &mut store,
                    &content,
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(child.phase(), "reconciled");
        fixture.cleanup();
    }

    #[test]
    fn m7_dispose_authorizes_at_quiesced_generation_then_records_unmaterialized_terminal() {
        let fixture = NativeFixture::new("m7-dispose-unmaterialized");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m7-dispose-unmaterialized");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m7-dispose-unmaterialized-child",
        );

        // Spawn/session readiness occurred at generation zero. Quiesce moves
        // the current cycle to generation one, which is the only authority
        // accepted for the closing Dispose chain.
        let dispose_generation = quiesce_office_cycle(&mut store, office.cycle_id);
        let (mut dispose, progress) = driver
            .begin_office_session_dispose(
                &mut store,
                &mut child,
                office_session_dispose_start(
                    "m7-dispose-unmaterialized",
                    "m7-dispose-unmaterialized-correlation",
                    dispose_generation,
                ),
                MonotonicTick::from_milliseconds(1_001),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(2_000)),
            )
            .unwrap();
        if progress == crate::supervision::ControlWriteProgress::Pending {
            for tick in 1_002..2_000 {
                if driver
                    .drive_office_session_dispose_delivery(
                        &mut store,
                        &mut child,
                        &mut dispose,
                        MonotonicTick::from_milliseconds(tick),
                    )
                    .unwrap()
                    == crate::supervision::ControlWriteProgress::Delivered
                {
                    break;
                }
            }
        }

        let mut saw_accepted = false;
        let mut saw_known = false;
        let mut disposed = false;
        for tick in 1_002..3_000 {
            let Some(output) = driver
                .observe_office_session_dispose_output(
                    &mut store,
                    &mut child,
                    &mut dispose,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(3_000)),
                    ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
                )
                .unwrap()
            else {
                thread::sleep(Duration::from_millis(1));
                continue;
            };
            match output {
                OfficePiSessionDisposeOutput::Accepted => saw_accepted = true,
                OfficePiSessionDisposeOutput::KnownUsageRecorded => saw_known = true,
                OfficePiSessionDisposeOutput::TranscriptReady(terminal) => {
                    assert!(matches!(
                        terminal.transcript(),
                        VerifiedPiSessionTranscript::UnmaterializedNoPrompt { .. }
                    ));
                    assert!(matches!(
                        driver
                            .record_office_session_disposed(
                                &mut store,
                                &mut child,
                                &mut dispose,
                                &terminal,
                                None,
                                MonotonicTick::from_milliseconds(tick),
                            )
                            .unwrap(),
                        OfficePiSessionDisposeOutput::Disposed
                    ));
                    disposed = true;
                    break;
                }
                other => panic!("unexpected M7 Dispose output: {other:?}"),
            }
        }
        assert!(saw_accepted);
        assert!(saw_known);
        assert!(disposed);
        assert_eq!(child.phase(), "disposed");

        let event_id = match store
            .command_receipt(
                &dispose
                    .operation
                    .command_id(PiOfficeSessionDisposeCommand::RecordDisposed)
                    .unwrap(),
            )
            .unwrap()
            .unwrap()
            .disposition
        {
            CommandDisposition::Accepted(event_id) => event_id,
            other => panic!("M7 terminal command must be accepted: {other:?}"),
        };
        assert!(matches!(
            store.ledger_event(event_id).unwrap().body,
            society_kernel::EventBody::PiOfficeSessionDisposed { session_id, .. }
                if session_id == office.session_id
        ));

        let content = ContentSealingAuthority::open(
            ContentStoreRoot::parse(fixture.root.join("m7-dispose-unmaterialized-content"))
                .unwrap(),
            ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
        )
        .unwrap();
        for tick in 3_000..4_000 {
            if driver
                .poll_reap_and_reconcile(
                    &mut store,
                    &content,
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(child.phase(), "reconciled");
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn m7_dispose_seals_the_peer_validated_materialized_transcript_before_terminal_commit() {
        let fixture = NativeFixture::new("m7-dispose-materialized");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m7-dispose-materialized");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m7-dispose-materialized-child",
        );

        let prompt_text = "the exact first Office prompt becomes transcript custody";
        let prompt_content = seal_prompt_content(
            &mut store,
            &fixture,
            "m7-dispose-materialized-prompt-content",
            prompt_text,
        );
        let (turn_id, frontier_event_id) = open_office_turn(
            &mut store,
            office.session_id,
            "m7-dispose-materialized-open-turn",
        );
        let (mut turn, prompt_progress) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m7-dispose-materialized-turn",
                    turn_id,
                    "m7-dispose-materialized-prompt-correlation",
                    prompt_content.content_object_id,
                    prompt_content.digest,
                    prompt_text,
                    frontier_event_id,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        if prompt_progress == crate::supervision::ControlWriteProgress::Pending {
            drive_prompt_until_delivered(
                &mut driver,
                &mut store,
                &mut child,
                &mut turn,
                101,
                1_000,
            );
        }
        assert!(
            drive_turn_until_terminal(&mut driver, &mut store, &mut child, &mut turn, 101, 2_000,)
                .contains(&OfficePiTurnOutput::SettledReady)
        );
        assert_eq!(child.phase(), "office_ready_recorded");

        let dispose_generation = quiesce_office_cycle(&mut store, office.cycle_id);
        let (mut dispose, progress) = driver
            .begin_office_session_dispose(
                &mut store,
                &mut child,
                office_session_dispose_start(
                    "m7-dispose-materialized",
                    "m7-dispose-materialized-correlation",
                    dispose_generation,
                ),
                MonotonicTick::from_milliseconds(2_001),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(3_000)),
            )
            .unwrap();
        if progress == crate::supervision::ControlWriteProgress::Pending {
            for tick in 2_002..3_000 {
                if driver
                    .drive_office_session_dispose_delivery(
                        &mut store,
                        &mut child,
                        &mut dispose,
                        MonotonicTick::from_milliseconds(tick),
                    )
                    .unwrap()
                    == crate::supervision::ControlWriteProgress::Delivered
                {
                    break;
                }
            }
        }

        let transcript_content = ContentSealingAuthority::open(
            ContentStoreRoot::parse(fixture.root.join("m7-dispose-materialized-content")).unwrap(),
            ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
        )
        .unwrap();
        let mut committed = false;
        for tick in 2_002..4_000 {
            let Some(output) = driver
                .observe_office_session_dispose_output(
                    &mut store,
                    &mut child,
                    &mut dispose,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(4_000)),
                    ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
                )
                .unwrap()
            else {
                thread::sleep(Duration::from_millis(1));
                continue;
            };
            if let OfficePiSessionDisposeOutput::TranscriptReady(terminal) = output {
                let sealed_content = match terminal.transcript() {
                    VerifiedPiSessionTranscript::Materialized(request) => {
                        // Crash seam: physical sealing may complete before
                        // the terminal Dispose command is even attempted.
                        // Retrying within this still-owned daemon uses the
                        // same derived operation identity and resolves to the
                        // one global ContentObject before terminal commit.
                        let physically_sealed = transcript_content
                            .seal_and_register(
                                &mut store,
                                request.content_operation(),
                                request.bytes(),
                            )
                            .unwrap();
                        let retry = transcript_content
                            .seal_and_register(
                                &mut store,
                                request.content_operation(),
                                request.bytes(),
                            )
                            .unwrap();
                        assert_eq!(retry.content_object_id, physically_sealed.content_object_id);
                        assert_eq!(retry.digest, physically_sealed.digest);
                        Some(retry)
                    }
                    VerifiedPiSessionTranscript::UnmaterializedNoPrompt { .. } => {
                        panic!("a completed Prompt must materialize the transcript")
                    }
                };
                assert!(matches!(
                    driver
                        .record_office_session_disposed(
                            &mut store,
                            &mut child,
                            &mut dispose,
                            &terminal,
                            sealed_content,
                            MonotonicTick::from_milliseconds(tick),
                        )
                        .unwrap(),
                    OfficePiSessionDisposeOutput::Disposed
                ));
                committed = true;
                break;
            }
        }
        assert!(
            committed,
            "materialized transcript must commit only after physical seal"
        );
        assert_eq!(child.phase(), "disposed");
        for tick in 4_000..5_000 {
            if driver
                .poll_reap_and_reconcile(
                    &mut store,
                    &transcript_content,
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(child.phase(), "reconciled");
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn m7_dispose_usage_unavailable_freezes_without_fabricating_a_disposed_session() {
        let fixture = NativeFixture::new("m7-dispose-usage-unavailable-ignore-term");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(
            &mut store,
            &fixture,
            "m7-dispose-usage-unavailable-ignore-term",
        );
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m7-dispose-usage-unavailable-child",
        );
        let generation = quiesce_office_cycle(&mut store, office.cycle_id);
        let (mut dispose, progress) = driver
            .begin_office_session_dispose(
                &mut store,
                &mut child,
                office_session_dispose_start(
                    "m7-dispose-usage-unavailable",
                    "m7-dispose-usage-unavailable-correlation",
                    generation,
                ),
                MonotonicTick::from_milliseconds(1_001),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(2_000)),
            )
            .unwrap();
        if progress == crate::supervision::ControlWriteProgress::Pending {
            for tick in 1_002..2_000 {
                if driver
                    .drive_office_session_dispose_delivery(
                        &mut store,
                        &mut child,
                        &mut dispose,
                        MonotonicTick::from_milliseconds(tick),
                    )
                    .unwrap()
                    == crate::supervision::ControlWriteProgress::Delivered
                {
                    break;
                }
            }
        }
        let mut froze = false;
        let mut frozen_at = 0;
        for tick in 1_002..3_000 {
            let output = driver
                .observe_office_session_dispose_output(
                    &mut store,
                    &mut child,
                    &mut dispose,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(3_000)),
                    ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
                )
                .unwrap();
            if matches!(output, Some(OfficePiSessionDisposeOutput::UsageFrozen)) {
                froze = true;
                frozen_at = tick;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(
            froze,
            "validated UsageUnavailable must freeze the parent first"
        );
        assert_eq!(child.phase(), "boundary_containment_required");
        assert!(
            store
                .command_receipt(
                    &dispose
                        .operation
                        .command_id(PiOfficeSessionDisposeCommand::RecordDisposed)
                        .unwrap(),
                )
                .unwrap()
                .is_none()
        );

        driver
            .drive_boundary_containment(&child, MonotonicTick::from_milliseconds(frozen_at + 1_000))
            .unwrap();
        driver
            .drive_boundary_containment(&child, MonotonicTick::from_milliseconds(frozen_at + 3_000))
            .unwrap();
        reconcile_child(
            &mut driver,
            &mut store,
            &fixture,
            &mut child,
            frozen_at + 3_001,
        );
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn m7_missing_forced_dispose_usage_freezes_at_the_observed_disposed_sequence() {
        let fixture = NativeFixture::new("m7-dispose-missing-final-usage");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m7-dispose-missing-final-usage");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m7-dispose-missing-final-usage-child",
        );
        let generation = quiesce_office_cycle(&mut store, office.cycle_id);
        let (mut dispose, progress) = driver
            .begin_office_session_dispose(
                &mut store,
                &mut child,
                office_session_dispose_start(
                    "m7-dispose-missing-final-usage",
                    "m7-dispose-missing-final-usage-correlation",
                    generation,
                ),
                MonotonicTick::from_milliseconds(1_001),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(2_000)),
            )
            .unwrap();
        if progress == crate::supervision::ControlWriteProgress::Pending {
            for tick in 1_002..2_000 {
                if driver
                    .drive_office_session_dispose_delivery(
                        &mut store,
                        &mut child,
                        &mut dispose,
                        MonotonicTick::from_milliseconds(tick),
                    )
                    .unwrap()
                    == crate::supervision::ControlWriteProgress::Delivered
                {
                    break;
                }
            }
        }
        let mut froze = false;
        for tick in 1_002..3_000 {
            let output = driver
                .observe_office_session_dispose_output(
                    &mut store,
                    &mut child,
                    &mut dispose,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(3_000)),
                    ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
                )
                .unwrap();
            if matches!(output, Some(OfficePiSessionDisposeOutput::UsageFrozen)) {
                froze = true;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(
            froze,
            "peer-rejected Disposed retains its exact Unknown failure"
        );
        assert_eq!(child.phase(), "boundary_containment_required");
        assert!(
            store
                .command_receipt(
                    &dispose
                        .operation
                        .command_id(PiOfficeSessionDisposeCommand::RecordDisposed)
                        .unwrap(),
                )
                .unwrap()
                .is_none()
        );
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn m7_stale_dispose_generation_is_rejected_before_any_host_control_write() {
        let fixture = NativeFixture::new("m7-dispose-stale-generation");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m7-dispose-stale-generation");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m7-dispose-stale-generation-child",
        );
        let current_generation = quiesce_office_cycle(&mut store, office.cycle_id);

        let stale = driver.begin_office_session_dispose(
            &mut store,
            &mut child,
            office_session_dispose_start(
                "m7-dispose-stale-generation",
                "m7-dispose-stale-generation-correlation",
                AdmissionGeneration::INITIAL,
            ),
            MonotonicTick::from_milliseconds(1_001),
            ControlWriteDeadline::at(MonotonicTick::from_milliseconds(2_000)),
        );
        assert!(matches!(
            stale,
            Err(super::PiExecutionError::KernelCommandRejected {
                capability: Capability::AuthorizePiOfficeSessionDispose,
                rejection: Rejection::StaleAdmissionGeneration,
            })
        ));
        // The authorizer was the only pre-write call. No stale Dispose frame
        // entered the host, so this child remains an otherwise live Office
        // session rather than an accidental native close.
        assert_eq!(child.phase(), "office_ready_recorded");

        // This distinct current-generation operation proves the preceding
        // failed authorization did not send/queue a hidden Dispose control.
        let (_dispose, progress) = driver
            .begin_office_session_dispose(
                &mut store,
                &mut child,
                office_session_dispose_start(
                    "m7-dispose-current-after-stale",
                    "m7-dispose-current-after-stale-correlation",
                    current_generation,
                ),
                MonotonicTick::from_milliseconds(1_002),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(2_000)),
            )
            .unwrap();
        assert!(matches!(
            progress,
            crate::supervision::ControlWriteProgress::Pending
                | crate::supervision::ControlWriteProgress::Delivered
        ));
        // The test has proven admission order; let Drop retain process
        // custody rather than deleting the workspace while a child exists.
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn transcript_reader_rejects_permissive_or_hard_linked_files_and_nonprivate_directories() {
        let fixture = NativeFixture::new("m7-transcript-custody");
        let workspace_directory = fixture.workspace.directory().clone();
        let session_directory = absolute(workspace_directory.as_path().join("sessions"));
        let transcript_path = session_directory.as_path().join("receipt.jsonl");
        fs::write(&transcript_path, b"private transcript").unwrap();
        fs::set_permissions(&transcript_path, fs::Permissions::from_mode(0o600)).unwrap();
        let transcript = absolute(transcript_path.clone());
        let limit = ContentSealLimit::new(4 * 1024).unwrap();

        assert_eq!(
            read_verified_transcript_bytes(
                &workspace_directory,
                &session_directory,
                transcript.as_str(),
                limit,
            )
            .unwrap(),
            b"private transcript"
        );

        fs::set_permissions(&transcript_path, fs::Permissions::from_mode(0o640)).unwrap();
        assert!(matches!(
            read_verified_transcript_bytes(
                &workspace_directory,
                &session_directory,
                transcript.as_str(),
                limit,
            ),
            Err(super::PiExecutionError::TranscriptFileUnsafe)
        ));

        fs::set_permissions(&transcript_path, fs::Permissions::from_mode(0o600)).unwrap();
        let alias = session_directory.as_path().join("receipt-alias.jsonl");
        fs::hard_link(&transcript_path, &alias).unwrap();
        assert!(matches!(
            read_verified_transcript_bytes(
                &workspace_directory,
                &session_directory,
                transcript.as_str(),
                limit,
            ),
            Err(super::PiExecutionError::TranscriptFileUnsafe)
        ));
        fs::remove_file(alias).unwrap();

        fs::write(&transcript_path, vec![b'x'; 4_097]).unwrap();
        fs::set_permissions(&transcript_path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(matches!(
            read_verified_transcript_bytes(
                &workspace_directory,
                &session_directory,
                transcript.as_str(),
                ContentSealLimit::new(4 * 1024).unwrap(),
            ),
            Err(super::PiExecutionError::TranscriptSizeLimitExceeded)
        ));
        fs::write(&transcript_path, b"private transcript").unwrap();
        fs::set_permissions(&transcript_path, fs::Permissions::from_mode(0o600)).unwrap();

        let alternate = session_directory.as_path().join("alternate-receipt.jsonl");
        fs::write(&alternate, b"private transcript").unwrap();
        fs::set_permissions(&alternate, fs::Permissions::from_mode(0o600)).unwrap();
        fs::remove_file(&transcript_path).unwrap();
        symlink(&alternate, &transcript_path).unwrap();
        assert!(matches!(
            read_verified_transcript_bytes(
                &workspace_directory,
                &session_directory,
                transcript.as_str(),
                limit,
            ),
            Err(super::PiExecutionError::TranscriptFileUnsafe)
        ));
        fs::remove_file(&transcript_path).unwrap();
        fs::write(&transcript_path, b"private transcript").unwrap();
        fs::set_permissions(&transcript_path, fs::Permissions::from_mode(0o600)).unwrap();

        let stale_digest = Blake3Digest::parse("0".repeat(64)).unwrap();
        let receipt = TranscriptFlushReceiptV1::Materialized {
            session_identity: SessionIdentity::parse("transcript-custody-session").unwrap(),
            session_file: transcript.clone(),
            session_file_blake3: stale_digest,
            header_cwd: workspace_directory.clone(),
            first_user_prompt: society_pi::FirstUserPromptReceipt::Absent,
        };
        assert!(matches!(
            project_verified_session_transcript(
                &PiOfficeSessionDisposeOperationId::parse("transcript-custody").unwrap(),
                &workspace_directory,
                &session_directory,
                &receipt,
                limit,
            ),
            Err(super::PiExecutionError::TranscriptDigestMismatch)
        ));

        fs::set_permissions(
            session_directory.as_path(),
            fs::Permissions::from_mode(0o750),
        )
        .unwrap();
        assert!(matches!(
            read_verified_transcript_bytes(
                &workspace_directory,
                &session_directory,
                transcript.as_str(),
                limit,
            ),
            Err(super::PiExecutionError::TranscriptOutsideOwnedWorkspace)
        ));
        fs::set_permissions(
            session_directory.as_path(),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();

        fs::set_permissions(
            workspace_directory.as_path(),
            fs::Permissions::from_mode(0o750),
        )
        .unwrap();
        assert!(matches!(
            read_verified_transcript_bytes(
                &workspace_directory,
                &session_directory,
                transcript.as_str(),
                limit,
            ),
            Err(super::PiExecutionError::TranscriptOutsideOwnedWorkspace)
        ));
        fs::set_permissions(
            workspace_directory.as_path(),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        fixture.cleanup();
    }

    #[test]
    fn provider_free_m6_prompt_projects_full_cumulative_usage_and_settles_only_stop() {
        let fixture = NativeFixture::new("m6-turn-stop");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-turn-stop");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(&mut driver, &mut store, &fixture, &office, "m6-child");

        let prompt_text = "sealed Office prompt one";
        let registered =
            seal_prompt_content(&mut store, &fixture, "m6-prompt-content", prompt_text);
        let (turn_one, frontier_one) =
            open_office_turn(&mut store, office.session_id, "m6-open-one");
        let start_one = office_turn_start(
            "m6-turn-one",
            turn_one,
            "m6-prompt-one",
            registered.content_object_id,
            registered.digest,
            prompt_text,
            frontier_one,
        );
        let (mut active_one, first_write) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                start_one,
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        if first_write == crate::supervision::ControlWriteProgress::Pending {
            drive_prompt_until_delivered(
                &mut driver,
                &mut store,
                &mut child,
                &mut active_one,
                101,
                1_000,
            );
        }
        let first_outputs = drive_turn_until_terminal(
            &mut driver,
            &mut store,
            &mut child,
            &mut active_one,
            101,
            2_000,
        );
        assert!(first_outputs.contains(&OfficePiTurnOutput::PromptAccepted));
        assert!(first_outputs.contains(&OfficePiTurnOutput::KnownUsageRecorded));
        assert_eq!(
            first_outputs.last(),
            Some(&OfficePiTurnOutput::SettledReady)
        );
        assert_eq!(child.phase(), "office_ready_recorded");

        let (turn_two, frontier_two) =
            open_office_turn(&mut store, office.session_id, "m6-open-two");
        let start_two = office_turn_start(
            "m6-turn-two",
            turn_two,
            "m6-prompt-two",
            registered.content_object_id,
            registered.digest,
            prompt_text,
            frontier_two,
        );
        let (mut active_two, second_write) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                start_two,
                MonotonicTick::from_milliseconds(2_001),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(3_000)),
            )
            .unwrap();
        if second_write == crate::supervision::ControlWriteProgress::Pending {
            drive_prompt_until_delivered(
                &mut driver,
                &mut store,
                &mut child,
                &mut active_two,
                2_002,
                3_000,
            );
        }
        let second_outputs = drive_turn_until_terminal(
            &mut driver,
            &mut store,
            &mut child,
            &mut active_two,
            2_002,
            4_000,
        );
        assert_eq!(
            second_outputs.last(),
            Some(&OfficePiTurnOutput::SettledReady)
        );
        let charged_deltas: Vec<_> = store
            .replay_ledger()
            .unwrap()
            .into_iter()
            .filter_map(|event| match event.body {
                society_kernel::EventBody::OfficeTurnSettled { charged_delta, .. } => {
                    Some(charged_delta)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            charged_deltas,
            [UsdMicros::new(4).unwrap(), UsdMicros::new(5).unwrap()]
        );
        drop(active_two);
        drop(active_one);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn provider_free_m6_same_total_final_known_usage_is_terminal_evidence_not_a_second_charge() {
        let fixture = NativeFixture::new("m6-known-before-and-final-same");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-known-before-and-final-same");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m6-same-known-child",
        );
        let prompt_text = "sealed Office prompt with same final cumulative usage";
        let registered =
            seal_prompt_content(&mut store, &fixture, "m6-same-known-content", prompt_text);
        let (turn_id, frontier) =
            open_office_turn(&mut store, office.session_id, "m6-same-known-open");
        let (mut turn, _) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-same-known-turn",
                    turn_id,
                    "m6-same-known-prompt",
                    registered.content_object_id,
                    registered.digest,
                    prompt_text,
                    frontier,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        let outputs =
            drive_turn_until_terminal(&mut driver, &mut store, &mut child, &mut turn, 101, 2_000);
        assert_eq!(
            outputs
                .iter()
                .filter(|output| **output == OfficePiTurnOutput::KnownUsageRecorded)
                .count(),
            2,
            "both the pre-terminal and forced same-total snapshots are durable cumulative facts"
        );
        assert_eq!(outputs.last(), Some(&OfficePiTurnOutput::SettledReady));
        let known_sequences: Vec<_> = store
            .replay_ledger()
            .unwrap()
            .into_iter()
            .filter_map(|event| match event.body {
                society_kernel::EventBody::PiOfficeTurnUsageRecorded {
                    office_turn_id,
                    protocol_sequence,
                    ..
                } if office_turn_id == turn_id => Some(protocol_sequence.value()),
                _ => None,
            })
            .collect();
        assert_eq!(known_sequences, [7, 9]);
        let charged: Vec<_> = store
            .replay_ledger()
            .unwrap()
            .into_iter()
            .filter_map(|event| match event.body {
                society_kernel::EventBody::OfficeTurnSettled {
                    turn_id: observed,
                    charged_delta,
                    ..
                } if observed == turn_id => Some(charged_delta),
                _ => None,
            })
            .collect();
        assert_eq!(charged, [UsdMicros::new(4).unwrap()]);
        drop(turn);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn provider_free_m6_known_error_records_terminal_but_never_returns_office_ready() {
        let fixture = NativeFixture::new("m6-prompt-error");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-prompt-error");
        let mut driver = PiExecutionDriver::new();
        let mut child =
            ready_office_child(&mut driver, &mut store, &fixture, &office, "m6-error-child");
        let prompt_text = "sealed Office error prompt";
        let registered = seal_prompt_content(&mut store, &fixture, "m6-error-content", prompt_text);
        let (turn_id, frontier) = open_office_turn(&mut store, office.session_id, "m6-error-open");
        let (mut turn, _) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-error-turn",
                    turn_id,
                    "m6-error-correlation",
                    registered.content_object_id,
                    registered.digest,
                    prompt_text,
                    frontier,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        let outputs =
            drive_turn_until_terminal(&mut driver, &mut store, &mut child, &mut turn, 101, 2_000);
        assert_eq!(
            outputs.last(),
            Some(&OfficePiTurnOutput::TerminalRecordedNonReady)
        );
        assert_eq!(child.phase(), "office_turn_terminal_blocked");
        drop(turn);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn provider_free_m6_sdk_promise_rejection_records_adjacent_known_terminal_without_agent_fact() {
        let fixture = NativeFixture::new("m6-sdk-promise-rejected-final-known");
        let mut store = KernelStore::connect_test().unwrap();
        let office =
            found_office_start(&mut store, &fixture, "m6-sdk-promise-rejected-final-known");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m6-sdk-rejection-child",
        );
        let prompt_text = "sealed Office prompt rejected by the SDK";
        let registered = seal_prompt_content(
            &mut store,
            &fixture,
            "m6-sdk-rejection-content",
            prompt_text,
        );
        let (turn_id, frontier) =
            open_office_turn(&mut store, office.session_id, "m6-sdk-rejection-open");
        let (mut turn, _) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-sdk-rejection-turn",
                    turn_id,
                    "m6-sdk-rejection-correlation",
                    registered.content_object_id,
                    registered.digest,
                    prompt_text,
                    frontier,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        let outputs =
            drive_turn_until_terminal(&mut driver, &mut store, &mut child, &mut turn, 101, 2_000);
        assert!(outputs.contains(&OfficePiTurnOutput::PromptAccepted));
        assert!(outputs.contains(&OfficePiTurnOutput::KnownUsageRecorded));
        assert_eq!(
            outputs.last(),
            Some(&OfficePiTurnOutput::TerminalRecordedNonReady)
        );
        assert_eq!(child.phase(), "office_turn_terminal_blocked");
        let events = store.replay_ledger().unwrap();
        assert!(events.iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::PiOfficeTurnTerminalRecorded {
                office_turn_id,
                disposition: society_kernel::PiOfficeTurnDisposition::Failed,
                assistant_outcome: society_kernel::PiOfficeTurnAssistantOutcome::SdkPromiseRejected,
                ..
            } if office_turn_id == turn_id
        )));
        assert!(!events.iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::PiOfficeTurnUsageFrozen {
                office_turn_id,
                ..
            } if office_turn_id == turn_id
        )));
        assert!(!events.iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::OfficeTurnSettled {
                turn_id: observed,
                ..
            } if observed == turn_id
        )));
        drop(turn);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn provider_free_m6_protocol_failure_records_terminal_then_contains_fatal_session() {
        let fixture = NativeFixture::new("m6-protocol-failed-final-known-ignore-term");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-protocol-failed-terminal");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m6-protocol-failed-child",
        );
        let prompt_text = "sealed Office prompt with invalid terminal assistant evidence";
        let registered = seal_prompt_content(
            &mut store,
            &fixture,
            "m6-protocol-failed-content",
            prompt_text,
        );
        let (turn_id, frontier) =
            open_office_turn(&mut store, office.session_id, "m6-protocol-failed-open");
        let (mut turn, _) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-protocol-failed-turn",
                    turn_id,
                    "m6-protocol-failed-correlation",
                    registered.content_object_id,
                    registered.digest,
                    prompt_text,
                    frontier,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        let outputs =
            drive_turn_until_terminal(&mut driver, &mut store, &mut child, &mut turn, 101, 2_000);
        assert_eq!(
            outputs.last(),
            Some(&OfficePiTurnOutput::TerminalRecordedNonReady)
        );
        assert_eq!(child.phase(), "boundary_containment_required");
        let events = store.replay_ledger().unwrap();
        assert!(events.iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::PiOfficeTurnTerminalRecorded {
                office_turn_id,
                disposition: society_kernel::PiOfficeTurnDisposition::ProtocolFailed,
                assistant_outcome: society_kernel::PiOfficeTurnAssistantOutcome::MissingFinalAssistantOutcome,
                ..
            } if office_turn_id == turn_id
        )));
        assert!(!events.iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::PiOfficeTurnUsageFrozen {
                office_turn_id,
                ..
            } if office_turn_id == turn_id
        )));
        driver
            .drive_boundary_containment(&child, MonotonicTick::from_milliseconds(1_200))
            .unwrap();
        driver
            .drive_boundary_containment(&child, MonotonicTick::from_milliseconds(3_200))
            .unwrap();
        reconcile_child(&mut driver, &mut store, &fixture, &mut child, 3_201);
        drop(turn);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn provider_free_m6_control_usage_after_agent_settled_cannot_replace_final_prompt_usage() {
        let fixture = NativeFixture::new("m6-control-interleave");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-control-interleave");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m6-control-child",
        );
        let prompt_text = "sealed Office interleaving prompt";
        let registered =
            seal_prompt_content(&mut store, &fixture, "m6-control-content", prompt_text);
        let (turn_id, frontier) =
            open_office_turn(&mut store, office.session_id, "m6-control-open");
        let (mut turn, _) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-control-turn",
                    turn_id,
                    "m6-control-prompt",
                    registered.content_object_id,
                    registered.digest,
                    prompt_text,
                    frontier,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        let mut observed_frames = 0;
        for tick in 101..1_000 {
            if driver
                .observe_office_turn_output(
                    &mut store,
                    &mut child,
                    &mut turn,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
                .is_some()
            {
                observed_frames += 1;
                if observed_frames == 4 {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            observed_frames, 4,
            "accepted plus terminal agent evidence must arrive"
        );
        assert_eq!(child.phase(), "office_turn_prompt_active");
        assert_eq!(
            driver
                .send_get_state_for_test(
                    &child,
                    CorrelationIdentity::parse("m6-control-get-state").unwrap(),
                    MonotonicTick::from_milliseconds(1_001),
                    ControlWriteDeadline::at(MonotonicTick::from_milliseconds(2_000)),
                )
                .unwrap(),
            crate::supervision::ControlWriteProgress::Delivered
        );
        let outputs =
            drive_turn_until_terminal(&mut driver, &mut store, &mut child, &mut turn, 1_002, 3_000);
        assert!(outputs.contains(&OfficePiTurnOutput::ControlInterleaving));
        assert_eq!(outputs.last(), Some(&OfficePiTurnOutput::SettledReady));
        drop(turn);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn pending_m6_prompt_cannot_be_observed_or_delivered_until_its_full_pipe_write() {
        let fixture = NativeFixture::new("m6-turn-stop");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-prompt-pending");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m6-pending-child",
        );
        let prompt_text = "sealed pending Office prompt";
        let registered =
            seal_prompt_content(&mut store, &fixture, "m6-pending-content", prompt_text);
        let (turn_id, frontier) =
            open_office_turn(&mut store, office.session_id, "m6-pending-open");
        driver
            .force_next_control_write_pending_for_test(&child)
            .unwrap();
        let (mut turn, progress) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-pending-turn",
                    turn_id,
                    "m6-pending-prompt",
                    registered.content_object_id,
                    registered.digest,
                    prompt_text,
                    frontier,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        assert_eq!(progress, crate::supervision::ControlWriteProgress::Pending);
        assert_eq!(child.phase(), "office_turn_prompt_delivery_pending");
        assert!(matches!(
            driver.observe_office_turn_output(
                &mut store,
                &mut child,
                &mut turn,
                MonotonicTick::from_milliseconds(101),
            ),
            Err(super::PiExecutionError::InvalidLifecycle)
        ));
        drive_prompt_until_delivered(&mut driver, &mut store, &mut child, &mut turn, 102, 1_000);
        assert_eq!(child.phase(), "office_turn_prompt_active");
        assert_eq!(
            drive_turn_until_terminal(&mut driver, &mut store, &mut child, &mut turn, 103, 2_000,)
                .last(),
            Some(&OfficePiTurnOutput::SettledReady)
        );
        drop(turn);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn cancellation_after_m6_authorization_fences_late_physical_delivery_without_office_ready() {
        let fixture = NativeFixture::new("m6-turn-stop");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-cancel-after-auth");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m6-cancel-child",
        );
        let prompt_text = "sealed cancellation-race Office prompt";
        let registered =
            seal_prompt_content(&mut store, &fixture, "m6-cancel-content", prompt_text);
        let (turn_id, frontier) = open_office_turn(&mut store, office.session_id, "m6-cancel-open");
        driver
            .force_next_control_write_pending_for_test(&child)
            .unwrap();
        let (mut turn, progress) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-cancel-turn",
                    turn_id,
                    "m6-cancel-prompt",
                    registered.content_object_id,
                    registered.digest,
                    prompt_text,
                    frontier,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        assert_eq!(progress, crate::supervision::ControlWriteProgress::Pending);
        let cancellation_capability = Capability::RequestCancellation;
        let cancellation = store
            .execute(CommandRequest {
                command_id: CommandId::parse("m6-cancel-request").unwrap(),
                principal_id: PrincipalId::new(3).unwrap(),
                capability_grant_id: store
                    .active_capability_grant(PrincipalId::new(3).unwrap(), cancellation_capability)
                    .unwrap()
                    .unwrap(),
                capability: cancellation_capability,
                expected_generation: ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
                body: CommandBody::RequestCancellation {
                    cycle_id: office.cycle_id,
                    mode: CancellationMode::GracefulCancel,
                },
            })
            .unwrap();
        assert!(matches!(
            cancellation.disposition,
            CommandDisposition::Accepted(_)
        ));
        accepted(
            &mut store,
            "m6-cancel-begin-propagation",
            PrincipalId::KERNEL,
            Capability::BeginCancellationPropagation,
            ExpectedGeneration::Exact(AdmissionGeneration::INITIAL.increment().unwrap()),
            CommandBody::BeginCancellationPropagation {
                cancellation_request_id: CancellationRequestId::new(1).unwrap(),
            },
        );
        let mut delivered = false;
        for tick in 101..1_000 {
            match driver.drive_office_turn_prompt_delivery(
                &mut store,
                &mut child,
                &mut turn,
                MonotonicTick::from_milliseconds(tick),
            ) {
                Ok(crate::supervision::ControlWriteProgress::Pending) => {
                    thread::sleep(Duration::from_millis(1));
                }
                Ok(crate::supervision::ControlWriteProgress::Delivered) => {
                    delivered = true;
                    break;
                }
                Err(error) => panic!("late physical Prompt delivery failed unexpectedly: {error}"),
            }
        }
        assert!(
            delivered,
            "the already authorized physical frame may race cancellation"
        );
        let mut late_settlement_error = None;
        for tick in 1_001..2_000 {
            match driver.observe_office_turn_output(
                &mut store,
                &mut child,
                &mut turn,
                MonotonicTick::from_milliseconds(tick),
            ) {
                Ok(_) => {}
                Err(error) => {
                    late_settlement_error = Some(error);
                    break;
                }
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(matches!(
            late_settlement_error,
            Some(super::PiExecutionError::KernelCommandRejected {
                capability: Capability::SettleOfficeTurn,
                rejection: Rejection::PiOfficeTurnNotReconciled,
            })
        ));
        assert_eq!(child.phase(), "boundary_containment_required");
        assert!(matches!(
            driver.observe_office_turn_output(
                &mut store,
                &mut child,
                &mut turn,
                MonotonicTick::from_milliseconds(102),
            ),
            Err(super::PiExecutionError::InvalidLifecycle)
        ));
        drop(turn);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn output_loss_after_m6_prompt_acceptance_is_contained_without_fabricating_usage_or_ready() {
        let fixture = NativeFixture::new("m6-exit-after-prompt-accepted");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-output-loss");
        let mut driver = PiExecutionDriver::new();
        let mut child =
            ready_office_child(&mut driver, &mut store, &fixture, &office, "m6-loss-child");
        let prompt_text = "sealed output-loss Office prompt";
        let registered = seal_prompt_content(&mut store, &fixture, "m6-loss-content", prompt_text);
        let (turn_id, frontier) = open_office_turn(&mut store, office.session_id, "m6-loss-open");
        let (mut turn, _) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-loss-turn",
                    turn_id,
                    "m6-loss-prompt",
                    registered.content_object_id,
                    registered.digest,
                    prompt_text,
                    frontier,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        let mut accepted = false;
        let mut output_lost = false;
        for tick in 101..2_000 {
            match driver.observe_office_turn_output(
                &mut store,
                &mut child,
                &mut turn,
                MonotonicTick::from_milliseconds(tick),
            ) {
                Ok(Some(OfficePiTurnOutput::PromptAccepted)) => accepted = true,
                Ok(_) => {}
                Err(super::PiExecutionError::Supervision(
                    crate::supervision::SupervisionError::OutputLost,
                )) => {
                    output_lost = true;
                    break;
                }
                Err(error) => panic!("unexpected output-loss projection error: {error}"),
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(accepted, "the exact host acceptance remains a durable fact");
        assert!(
            output_lost,
            "missing final usage must trigger containment, not a synthetic sequence"
        );
        assert_eq!(child.phase(), "boundary_containment_required");
        assert!(!store.replay_ledger().unwrap().iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::OfficeTurnSettled { turn_id: observed, .. }
                if observed == turn_id
        )));
        assert!(!store.replay_ledger().unwrap().iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::PiOfficeTurnUsageFrozen {
                office_turn_id: observed,
                ..
            } if observed == turn_id
        )));
        drop(turn);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn provider_free_m6_unavailable_usage_is_recorded_then_frozen_before_containment() {
        let fixture = NativeFixture::new("m6-usage-unavailable-ignore-term");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-usage-unavailable-ignore-term");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m6-frozen-child",
        );
        let prompt_text = "sealed Office unavailable prompt";
        let registered =
            seal_prompt_content(&mut store, &fixture, "m6-frozen-content", prompt_text);
        let (turn_id, frontier) = open_office_turn(&mut store, office.session_id, "m6-frozen-open");
        let (mut turn, _) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-frozen-turn",
                    turn_id,
                    "m6-frozen-correlation",
                    registered.content_object_id,
                    registered.digest,
                    prompt_text,
                    frontier,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        let mut frozen = false;
        for tick in 101..2_000 {
            if matches!(
                driver
                    .observe_office_turn_output(
                        &mut store,
                        &mut child,
                        &mut turn,
                        MonotonicTick::from_milliseconds(tick),
                    )
                    .unwrap(),
                Some(OfficePiTurnOutput::UsageFrozen)
            ) {
                frozen = true;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(
            frozen,
            "typed unavailable usage must reach the kernel before containment"
        );
        assert_eq!(child.phase(), "boundary_containment_required");
        assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::PiOfficeTurnUsageFrozen {
                office_turn_id,
                failure: society_kernel::PiOfficeTurnUsageFailure::Unavailable(
                    society_kernel::PiOfficeTurnUsageUnavailableReason::InvalidSdkUsage
                ),
                ..
            } if office_turn_id == turn_id
        )));
        drop(turn);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn provider_free_m6_pre_agent_settled_unavailable_freezes_the_observed_snapshot_then_reaps() {
        let fixture = NativeFixture::new("m6-usage-unavailable-pre-agent-settled-ignore-term");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-pre-agent-unavailable");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m6-pre-agent-unavailable-child",
        );
        let prompt_text = "sealed Office prompt with unavailable pre-terminal usage";
        let registered = seal_prompt_content(
            &mut store,
            &fixture,
            "m6-pre-agent-unavailable-content",
            prompt_text,
        );
        let (turn_id, frontier) = open_office_turn(
            &mut store,
            office.session_id,
            "m6-pre-agent-unavailable-open",
        );
        let (mut turn, _) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-pre-agent-unavailable-turn",
                    turn_id,
                    "m6-pre-agent-unavailable-prompt",
                    registered.content_object_id,
                    registered.digest,
                    prompt_text,
                    frontier,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();

        let mut accepted = false;
        let mut frozen = false;
        for tick in 101..2_000 {
            match driver
                .observe_office_turn_output(
                    &mut store,
                    &mut child,
                    &mut turn,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                Some(OfficePiTurnOutput::PromptAccepted) => accepted = true,
                Some(OfficePiTurnOutput::UsageFrozen) => {
                    frozen = true;
                    break;
                }
                Some(OfficePiTurnOutput::ControlInterleaving) | None => {
                    thread::sleep(Duration::from_millis(1));
                }
                Some(other) => panic!("unexpected pre-terminal output: {other:?}"),
            }
        }
        assert!(
            accepted,
            "the Prompt result must precede the frozen accounting fact"
        );
        assert!(
            frozen,
            "the pre-agent-settled Unavailable snapshot must be preserved"
        );
        assert_eq!(child.phase(), "boundary_containment_required");

        let unavailable_sequence = PiProtocolSequence::try_from(5).unwrap();
        let failure_command = turn
            .operation
            .command_id(PiOfficeTurnCommand::RecordUsageFailure {
                sequence: unavailable_sequence,
            })
            .unwrap();
        assert!(matches!(
            store.command_receipt(&failure_command).unwrap(),
            Some(society_kernel::CommandReceipt {
                disposition: CommandDisposition::Accepted(_),
                ..
            })
        ));
        assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::PiOfficeTurnUsageFrozen {
                office_turn_id,
                failure: society_kernel::PiOfficeTurnUsageFailure::Unavailable(
                    society_kernel::PiOfficeTurnUsageUnavailableReason::InvalidSdkUsage
                ),
                ..
            } if office_turn_id == turn_id
        )));
        assert!(!store.replay_ledger().unwrap().iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::PiOfficeTurnTerminalRecorded {
                office_turn_id: observed,
                ..
            } if observed == turn_id
        )));
        assert!(!store.replay_ledger().unwrap().iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::OfficeTurnSettled {
                turn_id: observed,
                ..
            } if observed == turn_id
        )));

        driver
            .drive_boundary_containment(&child, MonotonicTick::from_milliseconds(1_200))
            .unwrap();
        driver
            .drive_boundary_containment(&child, MonotonicTick::from_milliseconds(3_200))
            .unwrap();
        reconcile_child(&mut driver, &mut store, &fixture, &mut child, 3_201);
        for (ordinal, action) in [
            (0, society_kernel::ProcessSignalAction::Terminate),
            (1, society_kernel::ProcessSignalAction::Kill),
        ] {
            let command = child
                .operation
                .command_id(super::PiExecutionCommand::RecordSignal { ordinal })
                .unwrap();
            let receipt = store.command_receipt(&command).unwrap().unwrap();
            let CommandDisposition::Accepted(event_id) = receipt.disposition else {
                panic!("automatic containment signal must be durably accepted")
            };
            assert!(matches!(
                store.ledger_event(event_id).unwrap().body,
                society_kernel::EventBody::ProcessSignalReceiptRecorded {
                    action: observed,
                    ..
                } if observed == action
            ));
        }
        drop(turn);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn provider_free_m6_missing_final_usage_freezes_at_the_observed_settled_sequence() {
        let fixture = NativeFixture::new("m6-missing-final-usage");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m6-missing-final-usage");
        let mut driver = PiExecutionDriver::new();
        let mut child = ready_office_child(
            &mut driver,
            &mut store,
            &fixture,
            &office,
            "m6-missing-usage-child",
        );
        let prompt_text = "sealed Office prompt missing final usage";
        let registered = seal_prompt_content(
            &mut store,
            &fixture,
            "m6-missing-usage-content",
            prompt_text,
        );
        let (turn_id, frontier) =
            open_office_turn(&mut store, office.session_id, "m6-missing-usage-open");
        let (mut turn, _) = driver
            .authorize_and_begin_office_turn_prompt(
                &mut store,
                &mut child,
                office_turn_start(
                    "m6-missing-usage-turn",
                    turn_id,
                    "m6-missing-usage-prompt",
                    registered.content_object_id,
                    registered.digest,
                    prompt_text,
                    frontier,
                ),
                MonotonicTick::from_milliseconds(100),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        let mut frozen = false;
        for tick in 101..2_000 {
            match driver
                .observe_office_turn_output(
                    &mut store,
                    &mut child,
                    &mut turn,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                Some(OfficePiTurnOutput::UsageFrozen) => {
                    frozen = true;
                    break;
                }
                Some(_) | None => thread::sleep(Duration::from_millis(1)),
            }
        }
        assert!(
            frozen,
            "the schema-valid Settled must become a named Unknown"
        );
        assert_eq!(child.phase(), "boundary_containment_required");
        let frozen_events: Vec<_> = store
            .replay_ledger()
            .unwrap()
            .into_iter()
            .filter_map(|event| match event.body {
                society_kernel::EventBody::PiOfficeTurnUsageFrozen {
                    office_turn_id,
                    failure,
                    ..
                } if office_turn_id == turn_id => Some(failure),
                _ => None,
            })
            .collect();
        assert_eq!(frozen_events.len(), 1);
        assert_eq!(
            frozen_events[0],
            society_kernel::PiOfficeTurnUsageFailure::Unknown(
                society_kernel::PiOfficeTurnUsageUnknownReason::MissingFinalUsageSnapshot
            )
        );
        assert!(!store.replay_ledger().unwrap().iter().any(|event| matches!(
            event.body,
            society_kernel::EventBody::PiOfficeTurnTerminalRecorded {
                office_turn_id: observed,
                ..
            } if observed == turn_id
        )));
        drop(turn);
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn session_ready_then_direct_child_exit_refuses_office_ready_and_reconciles_wait() {
        let fixture = NativeFixture::new("m5-exit-after-session-ready");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m5-exit-after-session-ready");
        let start = OfficePiExecutionStart {
            operation: PiExecutionOperationId::parse("m5-exit-after-session-ready").unwrap(),
            operating_cycle_id: office.cycle_id,
            office_session_id: office.session_id,
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            expected_generation: AdmissionGeneration::INITIAL,
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: office.epoch_identity,
            spawn_request: fixture.spawn_request(),
        };
        let mut driver = PiExecutionDriver::new();
        let mut child = match driver.admit_spawn_and_register(&mut store, start).unwrap() {
            OfficePiSpawnRegistration::Ready(child) => child,
            other => panic!("ordinary provider-free host fixture must complete setup: {other:?}"),
        };
        wait_for_adapter_ready(&mut driver, &mut store, &mut child);
        let progress = driver
            .authorize_and_begin_create(
                &mut store,
                &mut child,
                MonotonicTick::from_milliseconds(1),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        if progress == crate::supervision::ControlWriteProgress::Pending {
            drive_create_until_delivered(&mut driver, &mut store, &mut child, 2, 1_000);
        }
        driver.pause_before_office_ready_liveness_for_test(Duration::from_millis(650));
        let outcome = loop {
            match driver.observe_session_ready(
                &mut store,
                &mut child,
                MonotonicTick::from_milliseconds(10),
                HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            ) {
                Ok(false) => thread::sleep(Duration::from_millis(1)),
                Ok(true) => panic!("a dead direct child must not make the Office ready"),
                Err(error) => break error,
            }
        };
        assert!(matches!(
            outcome,
            super::PiExecutionError::ExitedBeforeOfficeReady
        ));
        assert_eq!(child.phase(), "boundary_containment_required");
        let session_ready_command = child
            .operation
            .command_id(super::PiExecutionCommand::RecordSessionReady)
            .unwrap();
        assert!(matches!(
            store.command_receipt(&session_ready_command).unwrap(),
            Some(society_kernel::CommandReceipt {
                disposition: CommandDisposition::Accepted(_),
                ..
            })
        ));
        rejected_open_office_turn(&mut store, office.session_id, "dead-before-office-ready");

        let content = ContentSealingAuthority::open(
            ContentStoreRoot::parse(fixture.root.join("content")).unwrap(),
            ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
        )
        .unwrap();
        let mut reconciled = false;
        for tick in 20..1_000 {
            if driver
                .poll_reap_and_reconcile(
                    &mut store,
                    &content,
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                reconciled = true;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(
            reconciled,
            "an exited pre-Office child still needs receipts"
        );
        fixture.cleanup();
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn cancellation_between_admission_and_native_registration_uses_current_generation() {
        let fixture = NativeFixture::new("m5-generation-race-exit-before-ready");
        let mut store = KernelStore::connect_test().unwrap();
        let office =
            found_office_start(&mut store, &fixture, "m5-generation-race-exit-before-ready");
        let start = OfficePiExecutionStart {
            operation: PiExecutionOperationId::parse("m5-generation-race").unwrap(),
            operating_cycle_id: office.cycle_id,
            office_session_id: office.session_id,
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            expected_generation: AdmissionGeneration::INITIAL,
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: office.epoch_identity,
            spawn_request: fixture.spawn_request(),
        };
        let mut driver = PiExecutionDriver::new();
        driver.after_spawn_admission_for_test(|store, cycle_id| {
            accepted(
                store,
                "cancel-between-admit-and-register",
                PrincipalId::new(3).unwrap(),
                Capability::RequestCancellation,
                ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
                CommandBody::RequestCancellation {
                    cycle_id,
                    mode: CancellationMode::EmergencyStop,
                },
            );
            accepted(
                store,
                "snapshot-between-admit-and-register",
                PrincipalId::KERNEL,
                Capability::BeginCancellationPropagation,
                ExpectedGeneration::Exact(AdmissionGeneration::INITIAL.increment().unwrap()),
                CommandBody::BeginCancellationPropagation {
                    cancellation_request_id: CancellationRequestId::new(1).unwrap(),
                },
            );
        });
        let child = match driver.admit_spawn_and_register(&mut store, start).unwrap() {
            OfficePiSpawnRegistration::Ready(child) => child,
            other => panic!("the native child must register before any setup outcome: {other:?}"),
        };
        assert_eq!(
            child.expected_generation,
            AdmissionGeneration::INITIAL.increment().unwrap(),
            "the raced child must attach at the frozen cancellation generation"
        );
        assert_eq!(child.phase(), "spawn_registered");
        rejected(
            &mut store,
            "reject-reconcile-live-raced-child",
            PrincipalId::KERNEL,
            Capability::ReconcileCancellationPropagation,
            ExpectedGeneration::Exact(AdmissionGeneration::INITIAL.increment().unwrap()),
            CommandBody::ReconcileCancellationPropagation {
                cancellation_propagation_id: CancellationPropagationId::new(1).unwrap(),
            },
            Rejection::CancellationPropagationIncomplete,
        );
        // The fixture exits before AdapterReady, so this verifies the
        // registration did not merely succeed in memory before cleanup.
        let content = ContentSealingAuthority::open(
            ContentStoreRoot::parse(fixture.root.join("content")).unwrap(),
            ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
        )
        .unwrap();
        let mut child = child;
        for tick in 0..1_000 {
            if driver
                .poll_reap_and_reconcile(
                    &mut store,
                    &content,
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                assert_eq!(child.phase(), "reconciled");
                accepted(
                    &mut store,
                    "reconcile-finalized-raced-child",
                    PrincipalId::KERNEL,
                    Capability::ReconcileCancellationPropagation,
                    ExpectedGeneration::Exact(AdmissionGeneration::INITIAL.increment().unwrap()),
                    CommandBody::ReconcileCancellationPropagation {
                        cancellation_propagation_id: CancellationPropagationId::new(1).unwrap(),
                    },
                );
                fixture.cleanup();
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("raced registered child did not reach ordered cleanup")
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn task_attempt_payload_is_rejected_before_office_admission_or_native_spawn() {
        let mut fixture = NativeFixture::new("m5-reject-task-attempt-office");
        fixture.create.session_kind = SessionKind::TaskAttempt;
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m5-reject-task-attempt-office");
        let before_commands = store.command_count().unwrap();
        let start = OfficePiExecutionStart {
            operation: PiExecutionOperationId::parse("m5-reject-task-attempt-office").unwrap(),
            operating_cycle_id: office.cycle_id,
            office_session_id: office.session_id,
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            expected_generation: AdmissionGeneration::INITIAL,
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: office.epoch_identity,
            spawn_request: fixture.spawn_request(),
        };
        let mut driver = PiExecutionDriver::new();
        assert!(matches!(
            driver.admit_spawn_and_register(&mut store, start),
            Err(super::PiExecutionError::OfficeSessionKindRequired)
        ));
        assert_eq!(store.command_count().unwrap(), before_commands);
        assert_eq!(driver.registered_child_count_for_test(), 0);
        fixture.cleanup();
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn inert_registration_rejection_keeps_admission_unresolved_but_reaps_the_native_child() {
        let fixture = NativeFixture::new("m5-unresolved-registration-ignore-term");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(
            &mut store,
            &fixture,
            "m5-unresolved-registration-ignore-term",
        );
        let before_commands = store.command_count().unwrap();
        let start = OfficePiExecutionStart {
            operation: PiExecutionOperationId::parse("m5-unresolved-registration").unwrap(),
            operating_cycle_id: office.cycle_id,
            office_session_id: office.session_id,
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            expected_generation: AdmissionGeneration::INITIAL,
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: office.epoch_identity,
            spawn_request: fixture.spawn_request(),
        };
        let mut driver = PiExecutionDriver::new();
        driver.reject_inert_registration_for_test(Rejection::InvalidLifecycleTransition);
        let mut unresolved = match driver.admit_spawn_and_register(&mut store, start).unwrap() {
            OfficePiSpawnRegistration::RegistrationUnresolved { child, failure } => {
                assert!(matches!(
                    failure,
                    super::PiExecutionError::KernelCommandRejected {
                        capability: Capability::RecordInertChildSpawn,
                        rejection: Rejection::InvalidLifecycleTransition,
                    }
                ));
                child
            }
            other => panic!("native registration failure must retain containment: {other:?}"),
        };
        assert_eq!(unresolved.native_child_spawn_admission_id().value(), 1);
        // Only the pre-spawn admission exists. In particular no false
        // NotSpawned, PID/PGID, signal, or finalization receipt was written.
        assert_eq!(store.command_count().unwrap(), before_commands + 1);
        assert_eq!(driver.registered_child_count_for_test(), 1);

        let mut reaped = false;
        for tick in [1_000_u64, 3_000, 3_001, 3_002] {
            if driver
                .drive_unregistered_spawn_containment(
                    &mut unresolved,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                reaped = true;
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(reaped, "unregistered native child must still be reaped");
        assert!(unresolved.transient_completion().is_some());
        assert_eq!(store.command_count().unwrap(), before_commands + 1);
        assert_eq!(driver.registered_child_count_for_test(), 0);
        fixture.cleanup();
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn pending_dispose_cannot_be_observed_until_its_full_native_delivery() {
        let fixture = NativeFixture::new("m5-pending-dispose");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m5-pending-dispose");
        let start = OfficePiExecutionStart {
            operation: PiExecutionOperationId::parse("m5-pending-dispose").unwrap(),
            operating_cycle_id: office.cycle_id,
            office_session_id: office.session_id,
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            expected_generation: AdmissionGeneration::INITIAL,
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: office.epoch_identity,
            spawn_request: fixture.spawn_request(),
        };
        let mut driver = PiExecutionDriver::new();
        let mut child = match driver.admit_spawn_and_register(&mut store, start).unwrap() {
            OfficePiSpawnRegistration::Ready(child) => child,
            other => panic!("ordinary provider-free host fixture must complete setup: {other:?}"),
        };
        wait_for_adapter_ready(&mut driver, &mut store, &mut child);
        let progress = driver
            .authorize_and_begin_create(
                &mut store,
                &mut child,
                MonotonicTick::from_milliseconds(1),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        if progress == crate::supervision::ControlWriteProgress::Pending {
            drive_create_until_delivered(&mut driver, &mut store, &mut child, 2, 1_000);
        }
        wait_for_office_ready(&mut driver, &mut store, &mut child);

        driver
            .force_next_control_write_pending_for_test(&child)
            .unwrap();
        assert_eq!(
            driver
                .begin_dispose(
                    &mut child,
                    CorrelationIdentity::parse("pending-dispose").unwrap(),
                    MonotonicTick::from_milliseconds(1_001),
                    ControlWriteDeadline::at(MonotonicTick::from_milliseconds(2_000)),
                )
                .unwrap(),
            crate::supervision::ControlWriteProgress::Pending
        );
        assert_eq!(child.phase(), "dispose_delivery_pending");
        assert!(matches!(
            driver.observe_disposed(
                &mut child,
                MonotonicTick::from_milliseconds(1_001),
                HandshakeDeadline::at(MonotonicTick::from_milliseconds(2_000)),
            ),
            Err(super::PiExecutionError::InvalidLifecycle)
        ));
        assert_eq!(
            driver
                .drive_dispose_delivery(&mut child, MonotonicTick::from_milliseconds(1_002))
                .unwrap(),
            crate::supervision::ControlWriteProgress::Delivered
        );
        assert_eq!(child.phase(), "dispose_requested");
        for tick in 1_002..2_000 {
            if driver
                .observe_disposed(
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(2_000)),
                )
                .unwrap()
            {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(child.phase(), "disposed");
        reconcile_child(&mut driver, &mut store, &fixture, &mut child, 2_000);
        fixture.cleanup();
    }

    #[test]
    fn never_session_ready_boundary_error_drives_term_kill_then_ordered_reap() {
        let fixture = NativeFixture::new("m5-never-session-ready-ignore-term");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m5-never-session-ready-ignore-term");
        let operation = PiExecutionOperationId::parse("m5-never-session-ready").unwrap();
        let start = OfficePiExecutionStart {
            operation: operation.clone(),
            operating_cycle_id: office.cycle_id,
            office_session_id: office.session_id,
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            expected_generation: AdmissionGeneration::INITIAL,
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: office.epoch_identity,
            spawn_request: fixture.spawn_request(),
        };
        let mut driver = PiExecutionDriver::new();
        let mut child = match driver.admit_spawn_and_register(&mut store, start).unwrap() {
            OfficePiSpawnRegistration::Ready(child) => child,
            other => panic!("ordinary provider-free host fixture must complete setup: {other:?}"),
        };
        wait_for_adapter_ready(&mut driver, &mut store, &mut child);
        // The deterministic deadline clock advances much faster than a fresh
        // Node process can initialize. This private fixture marker is written
        // only after the TERM handler and keepalive are active; it prevents a
        // pre-handler TERM / voluntary EOF race from masquerading as an owned
        // process-group accessibility failure.
        let ready_marker = fixture
            .workspace
            .directory()
            .as_path()
            .join(".m5-never-session-ready-ready");
        for _ in 0..1_000 {
            if ready_marker.exists() {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(
            ready_marker.exists(),
            "never-SessionReady fixture never reached signal readiness"
        );
        let progress = driver
            .authorize_and_begin_create(
                &mut store,
                &mut child,
                MonotonicTick::from_milliseconds(1),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(100)),
            )
            .unwrap();
        if progress == crate::supervision::ControlWriteProgress::Pending {
            drive_create_until_delivered(&mut driver, &mut store, &mut child, 2, 100);
        }
        let failure = loop {
            match driver.observe_session_ready(
                &mut store,
                &mut child,
                MonotonicTick::from_milliseconds(20),
                HandshakeDeadline::at(MonotonicTick::from_milliseconds(20)),
            ) {
                Ok(false) => thread::sleep(Duration::from_millis(1)),
                Ok(true) => panic!("never-session-ready fixture cannot make Office Ready"),
                Err(error) => break error,
            }
        };
        assert!(matches!(
            failure,
            super::PiExecutionError::Supervision(
                crate::supervision::SupervisionError::HandshakeDeadlineExpired
            )
        ));
        assert_eq!(child.phase(), "boundary_containment_required");
        assert!(matches!(
            driver.begin_dispose(
                &mut child,
                CorrelationIdentity::parse("late-dispose").unwrap(),
                MonotonicTick::from_milliseconds(20),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(100)),
            ),
            Err(super::PiExecutionError::InvalidLifecycle)
        ));

        driver
            .drive_boundary_containment(&child, MonotonicTick::from_milliseconds(1_020))
            .unwrap();
        driver
            .drive_boundary_containment(&child, MonotonicTick::from_milliseconds(3_020))
            .unwrap();
        reconcile_child(&mut driver, &mut store, &fixture, &mut child, 3_021);
        for (ordinal, action) in [
            (0, society_kernel::ProcessSignalAction::Terminate),
            (1, society_kernel::ProcessSignalAction::Kill),
        ] {
            let command = operation
                .command_id(super::PiExecutionCommand::RecordSignal { ordinal })
                .unwrap();
            let receipt = store.command_receipt(&command).unwrap().unwrap();
            let CommandDisposition::Accepted(event_id) = receipt.disposition else {
                panic!("boundary signal must have a durable receipt")
            };
            assert!(matches!(
                store.ledger_event(event_id).unwrap().body,
                society_kernel::EventBody::ProcessSignalReceiptRecorded {
                    action: observed,
                    delivery: society_kernel::ProcessSignalDelivery::Delivered,
                    ..
                } if observed == action
            ));
        }
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    #[test]
    fn pre_adapter_and_pre_create_exits_still_enter_ordered_reap_seal_and_finalization() {
        for (label, observe_adapter_first) in [
            ("m5-exit-before-ready", false),
            ("m5-exit-after-ready", true),
        ] {
            let fixture = NativeFixture::new(label);
            let mut store = KernelStore::connect_test().unwrap();
            let office = found_office_start(&mut store, &fixture, label);
            let start = OfficePiExecutionStart {
                operation: PiExecutionOperationId::parse(format!("{label}-operation")).unwrap(),
                operating_cycle_id: office.cycle_id,
                office_session_id: office.session_id,
                budget_reservation_id: BudgetReservationId::new(1).unwrap(),
                execution_profile_id:
                    society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
                expected_generation: AdmissionGeneration::INITIAL,
                supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
                supervisor_epoch_identity: office.epoch_identity,
                spawn_request: fixture.spawn_request(),
            };
            let mut driver = PiExecutionDriver::new();
            let mut child = match driver.admit_spawn_and_register(&mut store, start).unwrap() {
                OfficePiSpawnRegistration::Ready(child) => child,
                other => {
                    panic!("ordinary provider-free host fixture must complete setup: {other:?}")
                }
            };
            if observe_adapter_first {
                wait_for_adapter_ready(&mut driver, &mut store, &mut child);
                // The provider-free double now exits before any durable
                // Create authorization or native CreateSession byte.
                thread::sleep(Duration::from_millis(50));
                assert_eq!(child.phase(), "adapter_ready_recorded");
            }
            reconcile_child(&mut driver, &mut store, &fixture, &mut child, 0);
            fixture.cleanup();
        }
    }

    #[test]
    fn owned_descendant_requires_direct_reap_then_lingering_kill_then_later_absence() {
        let fixture = NativeFixture::new("m5-owned-descendant-after-ready");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m5-owned-descendant-after-ready");
        let operation = PiExecutionOperationId::parse("m5-owned-descendant").unwrap();
        let start = OfficePiExecutionStart {
            operation: operation.clone(),
            operating_cycle_id: office.cycle_id,
            office_session_id: office.session_id,
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            expected_generation: AdmissionGeneration::INITIAL,
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: office.epoch_identity,
            spawn_request: fixture.spawn_request(),
        };
        let mut driver = PiExecutionDriver::new();
        let mut child = match driver.admit_spawn_and_register(&mut store, start).unwrap() {
            OfficePiSpawnRegistration::Ready(child) => child,
            other => panic!("ordinary provider-free host fixture must complete setup: {other:?}"),
        };
        wait_for_adapter_ready(&mut driver, &mut store, &mut child);
        thread::sleep(Duration::from_millis(25));
        reconcile_child(&mut driver, &mut store, &fixture, &mut child, 0);
        let command = operation
            .command_id(super::PiExecutionCommand::RecordSignal { ordinal: 2 })
            .unwrap();
        let receipt = store.command_receipt(&command).unwrap().unwrap();
        let CommandDisposition::Accepted(event_id) = receipt.disposition else {
            panic!("the owned descendant must have a durable lingering signal")
        };
        assert!(matches!(
            store.ledger_event(event_id).unwrap().body,
            society_kernel::EventBody::ProcessSignalReceiptRecorded {
                action: society_kernel::ProcessSignalAction::LingeringGroupKill,
                ..
            }
        ));
        fixture.cleanup();
    }

    #[cfg(feature = "test-support")]
    #[test]
    fn post_spawn_setup_failure_is_registered_then_contained_not_recorded_as_not_spawned() {
        let fixture = NativeFixture::new("m5-setup-fault-ignore-term");
        let mut store = KernelStore::connect_test().unwrap();
        let office = found_office_start(&mut store, &fixture, "m5-setup-fault-ignore-term");
        let start = OfficePiExecutionStart {
            operation: PiExecutionOperationId::parse("m5-setup-fault-ignore-term").unwrap(),
            operating_cycle_id: office.cycle_id,
            office_session_id: office.session_id,
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            expected_generation: AdmissionGeneration::INITIAL,
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: office.epoch_identity,
            spawn_request: fixture.spawn_request(),
        };
        let supervisor = crate::supervision::PiSupervisor::with_post_spawn_setup_fault_for_test(
            crate::supervision::PostSpawnSetupFailure::StdoutNonblocking,
        );
        let mut driver = PiExecutionDriver::with_supervisor_for_test(supervisor);
        let mut child = match driver.admit_spawn_and_register(&mut store, start).unwrap() {
            OfficePiSpawnRegistration::PostSpawnSetupContained { child, failure } => {
                assert_eq!(
                    failure,
                    crate::supervision::PostSpawnSetupFailure::StdoutNonblocking
                );
                child
            }
            other => panic!("injected post-spawn setup fault must be caller-visible: {other:?}"),
        };
        assert_eq!(child.phase(), "post_spawn_setup_contained");

        // The fault path closes stdin before it starts automatic containment.
        // This double deliberately ignores that EOF and TERM, so the test
        // waits for its private readiness marker before it advances both
        // documented emergency deadlines. Without that synchronization, a
        // voluntary EOF exit or a TERM sent before Node installs its handler
        // can turn this into a PID/PGID reuse race rather than testing ordered
        // automatic containment.
        let ready_marker = fixture
            .workspace
            .directory()
            .as_path()
            .join(".m5-setup-fault-ready");
        for _ in 0..1_000 {
            if ready_marker.exists() {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(
            ready_marker.exists(),
            "fault fixture never reached readiness"
        );
        driver
            .drive_boundary_containment(&child, MonotonicTick::from_milliseconds(1_000))
            .unwrap();
        driver
            .drive_boundary_containment(&child, MonotonicTick::from_milliseconds(3_000))
            .unwrap();
        let content = ContentSealingAuthority::open(
            ContentStoreRoot::parse(fixture.root.join("content")).unwrap(),
            ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
        )
        .unwrap();
        let mut reconciled = false;
        for tick in 3_000..4_000 {
            if driver
                .poll_reap_and_reconcile(
                    &mut store,
                    &content,
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                reconciled = true;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(reconciled);
        assert_eq!(child.phase(), "reconciled");
        for (ordinal, action) in [
            (0, society_kernel::ProcessSignalAction::Terminate),
            (1, society_kernel::ProcessSignalAction::Kill),
        ] {
            let command = child
                .operation
                .command_id(super::PiExecutionCommand::RecordSignal { ordinal })
                .unwrap();
            let receipt = store.command_receipt(&command).unwrap().unwrap();
            let CommandDisposition::Accepted(event_id) = receipt.disposition else {
                panic!("automatic containment signal must be durably accepted")
            };
            assert!(matches!(
                store.ledger_event(event_id).unwrap().body,
                society_kernel::EventBody::ProcessSignalReceiptRecorded {
                    action: observed,
                    delivery: society_kernel::ProcessSignalDelivery::Delivered,
                    ..
                } if observed == action
            ));
        }
        drop(child);
        drop(driver);
        fixture.cleanup();
    }

    fn wait_for_adapter_ready(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        child: &mut super::OfficePiExecutionChild,
    ) {
        for tick in 0..1_000 {
            if driver
                .observe_adapter_ready(
                    store,
                    child,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
                )
                .unwrap()
            {
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("provider-free host never reached AdapterReady")
    }

    fn drive_create_until_delivered(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        child: &mut super::OfficePiExecutionChild,
        first_tick: u64,
        deadline_tick: u64,
    ) {
        for tick in first_tick..deadline_tick {
            if driver
                .drive_create_delivery(store, child, MonotonicTick::from_milliseconds(tick))
                .unwrap()
                == crate::supervision::ControlWriteProgress::Delivered
            {
                return;
            }
        }
        panic!("provider-free CreateSession frame did not reach stdin")
    }

    fn wait_for_office_ready(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        child: &mut super::OfficePiExecutionChild,
    ) {
        for tick in 2..1_000 {
            if driver
                .observe_session_ready(
                    store,
                    child,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
                )
                .unwrap()
            {
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("provider-free host never reached Office Ready")
    }

    fn reconcile_child(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        fixture: &NativeFixture,
        child: &mut super::OfficePiExecutionChild,
        first_tick: u64,
    ) {
        let content = ContentSealingAuthority::open(
            ContentStoreRoot::parse(fixture.root.join("content")).unwrap(),
            ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
        )
        .unwrap();
        for tick in first_tick..first_tick.saturating_add(1_000) {
            if driver
                .poll_reap_and_reconcile(
                    store,
                    &content,
                    child,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                assert_eq!(child.phase(), "reconciled");
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("direct child did not reach ordered reconciliation")
    }

    fn wait_for_task_attempt_adapter_ready(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        child: &mut super::TaskAttemptPiExecutionChild,
    ) {
        for tick in 0..1_000 {
            if driver
                .observe_task_attempt_adapter_ready(
                    store,
                    child,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
                )
                .unwrap()
            {
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("provider-free host never reached TaskAttempt AdapterReady")
    }

    fn drive_task_attempt_create_until_delivered(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        child: &mut super::TaskAttemptPiExecutionChild,
        first_tick: u64,
        deadline_tick: u64,
    ) {
        for tick in first_tick..deadline_tick {
            if driver
                .drive_task_attempt_create_delivery(
                    store,
                    child,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
                == crate::supervision::ControlWriteProgress::Delivered
            {
                return;
            }
        }
        panic!("provider-free TaskAttempt CreateSession frame did not reach stdin")
    }

    fn wait_for_task_attempt_session_ready(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        child: &mut super::TaskAttemptPiExecutionChild,
    ) {
        for tick in 2..1_000 {
            if driver
                .observe_task_attempt_session_ready(
                    store,
                    child,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
                )
                .unwrap()
            {
                assert_eq!(child.phase(), "session_ready_recorded");
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("provider-free host never reached TaskAttempt SessionReady")
    }

    fn drive_task_attempt_prompt_until_delivered(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        child: &mut super::TaskAttemptPiExecutionChild,
        prompt: &mut super::TaskAttemptPiPrompt,
        first_tick: u64,
        deadline_tick: u64,
    ) {
        for tick in first_tick..deadline_tick {
            if driver
                .drive_task_attempt_prompt_delivery(
                    store,
                    child,
                    prompt,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
                == crate::supervision::ControlWriteProgress::Delivered
            {
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("provider-free TaskAssignment Prompt did not reach a complete physical write")
    }

    fn drive_task_attempt_dispose_until_delivered(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        child: &mut super::TaskAttemptPiExecutionChild,
        dispose: &mut super::TaskAttemptPiSessionDispose,
        first_tick: u64,
        deadline_tick: u64,
    ) {
        for tick in first_tick..deadline_tick {
            if driver
                .drive_task_attempt_session_dispose_delivery(
                    store,
                    child,
                    dispose,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
                == crate::supervision::ControlWriteProgress::Delivered
            {
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("provider-free TaskAttempt Dispose did not reach a complete physical write")
    }

    struct OfficeStart {
        cycle_id: OperatingCycleId,
        session_id: RootAuthorityOfficeSessionId,
        epoch_identity: SupervisorEpochIdentity,
    }

    /// The exact M3 identities needed for an actor-owned Pi child. Keeping
    /// the attempt and reservation together prevents the native TaskAttempt
    /// regression from accidentally borrowing the Office session's budget.
    struct RunningTaskAttempt {
        actor_attempt_id: ActorAttemptId,
        budget_reservation_id: BudgetReservationId,
    }

    fn office_bridge_application_mission() -> ApplicationMissionInput {
        ApplicationMissionInput {
            application_identity: ApplicationIdentity::parse("office-bridge-fixture").unwrap(),
            application_name: ApplicationName::parse("Office bridge fixture").unwrap(),
            revision_ordinal: ApplicationRevisionOrdinal::new(1).unwrap(),
            statement: MissionStatement::parse(
                "Exercise the bounded office execution bridge without a provider.",
            )
            .unwrap(),
            principles: MissionPrinciples::new(vec![MissionPrinciple {
                kind: MissionPrincipleKind::Boundary,
                text: MissionPrincipleText::parse("Keep execution authority bounded.").unwrap(),
            }])
            .unwrap(),
            north_star_questions: NorthStarQuestionSet {
                change: NorthStarChangeQuestion::parse("What bounded execution change is needed?")
                    .unwrap(),
                improvement_evidence: NorthStarImprovementEvidenceQuestion::parse(
                    "What receipt proves the execution improvement?",
                )
                .unwrap(),
                boundary_commitment: NorthStarBoundaryCommitmentQuestion::parse(
                    "Which execution boundary must remain intact?",
                )
                .unwrap(),
                revisit: NorthStarRevisitQuestion::parse(
                    "When should this execution mission be revisited?",
                )
                .unwrap(),
            },
            source_rendering_digest: KernelDigest::of_bytes(b"office-bridge-fixture-mission"),
        }
    }

    fn found_office_start(
        store: &mut KernelStore,
        fixture: &NativeFixture,
        label: &str,
    ) -> OfficeStart {
        let bootstrap = PrincipalId::BOOTSTRAP;
        accepted(
            store,
            "found-society",
            bootstrap,
            Capability::CreateSocietyIdentity,
            ExpectedGeneration::NotApplicable,
            CommandBody::CreateSocietyIdentity {
                name: SocietyName::parse("M5 office bridge").unwrap(),
            },
        );
        let mission_source = b"office-bridge-fixture-mission";
        let mission = office_bridge_application_mission();
        assert_eq!(
            mission.source_rendering_digest,
            KernelDigest::of_bytes(mission_source)
        );
        let authority = ContentSealingAuthority::open(
            ContentStoreRoot::parse(fixture.root.join("founding-mission-content")).unwrap(),
            ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
        )
        .unwrap();
        let operation = ContentSealOperationId::parse(
            format!("founding-source-{label}"),
            mission.source_rendering_digest,
        )
        .unwrap();
        authority
            .seal_and_register(store, &operation, mission_source)
            .unwrap();
        accepted(
            store,
            "found-founding-mission",
            bootstrap,
            Capability::InstallFoundingMission,
            ExpectedGeneration::NotApplicable,
            CommandBody::InstallFoundingMission { mission },
        );
        accepted(
            store,
            "found-office",
            bootstrap,
            Capability::InstallRootAuthorityOffice,
            ExpectedGeneration::NotApplicable,
            CommandBody::InstallRootAuthorityOffice,
        );
        accepted(
            store,
            "found-root_authority",
            bootstrap,
            Capability::AppointInitialRootAuthority,
            ExpectedGeneration::NotApplicable,
            CommandBody::AppointInitialRootAuthority {
                actor_display_name: PrincipalDisplayName::parse("Root Authority").unwrap(),
            },
        );
        accepted(
            store,
            "found-ceiling",
            bootstrap,
            Capability::SetR0HardCeiling,
            ExpectedGeneration::NotApplicable,
            CommandBody::SetR0HardCeiling {
                ceiling: UsdMicros::new(1_030_000).unwrap(),
            },
        );
        accepted(
            store,
            "found-bootstrap",
            bootstrap,
            Capability::BootstrapSociety,
            ExpectedGeneration::NotApplicable,
            CommandBody::BootstrapSociety,
        );
        accepted(
            store,
            "found-propose",
            bootstrap,
            Capability::ProposeOperatingCycle,
            ExpectedGeneration::NotApplicable,
            CommandBody::ProposeOperatingCycle {
                treatment: OperatingCycleTreatment::DeterministicPiHostFixtureV1,
                budget_ceiling: UsdMicros::new(1_000_000).unwrap(),
            },
        );
        let cycle_id = OperatingCycleId::new(1).unwrap();
        let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
        accepted(
            store,
            "found-admit",
            bootstrap,
            Capability::AdmitOperatingCycle,
            generation,
            CommandBody::AdmitOperatingCycle { cycle_id },
        );
        let root_authority = PrincipalId::new(3).unwrap();
        accepted(
            store,
            "office-start",
            root_authority,
            Capability::StartRootAuthorityOfficeSession,
            generation,
            CommandBody::StartRootAuthorityOfficeSession { cycle_id },
        );
        accepted(
            store,
            "office-reserve",
            root_authority,
            Capability::ReserveBudget,
            generation,
            CommandBody::ReserveBudget {
                cycle_id,
                amount: UsdMicros::new(10_000).unwrap(),
            },
        );
        let epoch_identity = SupervisorEpochIdentity::parse(format!("epoch-{label}")).unwrap();
        accepted(
            store,
            "office-epoch",
            PrincipalId::KERNEL,
            Capability::OpenSupervisorEpoch,
            ExpectedGeneration::NotApplicable,
            CommandBody::OpenSupervisorEpoch {
                supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
                supervisor_epoch_identity: epoch_identity.clone(),
            },
        );
        OfficeStart {
            cycle_id,
            session_id: RootAuthorityOfficeSessionId::new(1).unwrap(),
            epoch_identity,
        }
    }

    fn running_task_attempt(store: &mut KernelStore, office: &OfficeStart) -> RunningTaskAttempt {
        let root_authority = PrincipalId::new(3).unwrap();
        let actor_principal = PrincipalId::new(4).unwrap();
        let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
        let project_id = ProjectId::new(1).unwrap();
        accepted(
            store,
            "task-live-create-project",
            root_authority,
            Capability::CreateProject,
            generation,
            CommandBody::CreateProject {
                operating_cycle_id: office.cycle_id,
                project_name: ProjectName::parse("Task lifecycle proof").unwrap(),
                north_star_alignment: ProjectNorthStarAlignment {
                    application_revision_id: ApplicationRevisionId::new(1).unwrap(),
                    change_answer: ProjectNorthStarChangeAnswer::parse(
                        "Prove one bounded task lifecycle.",
                    )
                    .unwrap(),
                    improvement_evidence_answer: ProjectNorthStarImprovementEvidenceAnswer::parse(
                        "A native-child replay must validate.",
                    )
                    .unwrap(),
                    boundary_commitment_answer: ProjectNorthStarBoundaryCommitmentAnswer::parse(
                        "Do not borrow Office authority.",
                    )
                    .unwrap(),
                    revisit_answer: ProjectNorthStarRevisitAnswer::parse(
                        "Review after the actor session closes.",
                    )
                    .unwrap(),
                },
            },
        );
        accepted(
            store,
            "task-live-challenge-project",
            root_authority,
            Capability::TransitionProject,
            generation,
            CommandBody::TransitionProject {
                operating_cycle_id: office.cycle_id,
                project_id,
                target: ProjectState::Challenged,
            },
        );
        accepted(
            store,
            "task-live-charter-project",
            root_authority,
            Capability::CharterProject,
            generation,
            CommandBody::CharterProject {
                operating_cycle_id: office.cycle_id,
                project_id,
                objective: ProjectObjectiveText::parse("Exercise one bounded task attempt.")
                    .unwrap(),
                initial_milestone: ProjectMilestoneName::parse("Finish the task session").unwrap(),
                stop_condition: ProjectStopConditionText::parse(
                    "The native session is reconciled.",
                )
                .unwrap(),
            },
        );
        accepted(
            store,
            "task-live-activate-project",
            root_authority,
            Capability::TransitionProject,
            generation,
            CommandBody::TransitionProject {
                operating_cycle_id: office.cycle_id,
                project_id,
                target: ProjectState::Active,
            },
        );
        accepted(
            store,
            "task-live-create-ticket",
            root_authority,
            Capability::CreateTicket,
            generation,
            CommandBody::CreateTicket {
                operating_cycle_id: office.cycle_id,
                project_id,
                ticket_title: TicketTitle::parse("Run task actor").unwrap(),
                acceptance_condition: TicketAcceptanceConditionText::parse(
                    "The actor runtime is reconciled.",
                )
                .unwrap(),
                prerequisite_ticket_id: None,
            },
        );
        accepted(
            store,
            "task-live-register-config",
            root_authority,
            Capability::RegisterActorConfiguration,
            ExpectedGeneration::NotApplicable,
            CommandBody::RegisterActorConfiguration {
                configuration_name: ActorConfigurationName::parse("task actor").unwrap(),
                model_policy: ActorModelPolicy::PinnedDeepseekV4FlashHigh,
                primary_attractor: DevelopmentalAttractor::Challenge,
            },
        );
        accepted(
            store,
            "task-live-register-context",
            root_authority,
            Capability::RegisterContextPack,
            generation,
            CommandBody::RegisterContextPack {
                operating_cycle_id: office.cycle_id,
                purpose: ContextPackPurpose::TicketExecution,
                rendering_digest: KernelDigest::of_bytes(b"task-live-context"),
            },
        );
        accepted(
            store,
            "task-live-admit-actor",
            root_authority,
            Capability::AdmitActorInstance,
            generation,
            CommandBody::AdmitActorInstance {
                operating_cycle_id: office.cycle_id,
                actor_configuration_revision_id: ActorConfigurationRevisionId::new(1).unwrap(),
                execution_profile_id:
                    society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
                actor_display_name: PrincipalDisplayName::parse("task actor").unwrap(),
            },
        );
        accepted(
            store,
            "task-live-admit-ticket",
            root_authority,
            Capability::AdmitTicket,
            generation,
            CommandBody::AdmitTicket {
                operating_cycle_id: office.cycle_id,
                ticket_id: TicketId::new(1).unwrap(),
            },
        );
        accepted(
            store,
            "task-live-register-work",
            root_authority,
            Capability::RegisterWorkItem,
            generation,
            CommandBody::RegisterWorkItem {
                operating_cycle_id: office.cycle_id,
                ticket_id: TicketId::new(1).unwrap(),
                actor_instance_id: ActorInstanceId::new(1).unwrap(),
                context_pack_id: ContextPackId::new(1).unwrap(),
                work_kind: WorkItemKind::TicketExecution,
                adversarial_review_id: None,
                assignment: WorkAssignmentText::parse("Run the bounded task actor.").unwrap(),
            },
        );
        accepted(
            store,
            "task-live-claim-work",
            actor_principal,
            Capability::ClaimWorkItem,
            generation,
            CommandBody::ClaimWorkItem {
                operating_cycle_id: office.cycle_id,
                work_item_id: WorkItemId::new(1).unwrap(),
            },
        );
        let capability = Capability::StartActorAttempt;
        let receipt = store
            .execute(CommandRequest {
                command_id: CommandId::parse("task-live-start-attempt").unwrap(),
                principal_id: root_authority,
                capability_grant_id: store
                    .active_capability_grant(root_authority, capability)
                    .unwrap()
                    .unwrap(),
                capability,
                expected_generation: generation,
                body: CommandBody::StartActorAttempt {
                    operating_cycle_id: office.cycle_id,
                    work_item_id: WorkItemId::new(1).unwrap(),
                    reservation_amount: UsdMicros::new(5_000).unwrap(),
                },
            })
            .unwrap();
        let CommandDisposition::Accepted(_) = receipt.disposition else {
            panic!("task actor attempt must be admitted: {receipt:?}")
        };
        RunningTaskAttempt {
            actor_attempt_id: ActorAttemptId::new(1).unwrap(),
            // The founding helper reserved one Office budget first; this M3
            // attempt owns the next reservation and the kernel checks that
            // exact relation during TaskAttempt child admission.
            budget_reservation_id: BudgetReservationId::new(2).unwrap(),
        }
    }

    fn accepted(
        store: &mut KernelStore,
        id: &str,
        principal: PrincipalId,
        capability: Capability,
        expected_generation: ExpectedGeneration,
        body: CommandBody,
    ) {
        let grant = store
            .active_capability_grant(principal, capability)
            .unwrap()
            .unwrap();
        let receipt = store
            .execute(CommandRequest {
                command_id: CommandId::parse(id).unwrap(),
                principal_id: principal,
                capability_grant_id: grant,
                capability,
                expected_generation,
                body,
            })
            .unwrap();
        assert!(
            matches!(receipt.disposition, CommandDisposition::Accepted(_)),
            "{id}: {receipt:?}"
        );
    }

    fn rejected(
        store: &mut KernelStore,
        id: &str,
        principal: PrincipalId,
        capability: Capability,
        expected_generation: ExpectedGeneration,
        body: CommandBody,
        expected_rejection: Rejection,
    ) {
        let grant = store
            .active_capability_grant(principal, capability)
            .unwrap()
            .unwrap();
        let receipt = store
            .execute(CommandRequest {
                command_id: CommandId::parse(id).unwrap(),
                principal_id: principal,
                capability_grant_id: grant,
                capability,
                expected_generation,
                body,
            })
            .unwrap();
        assert_eq!(
            receipt.disposition,
            CommandDisposition::Rejected(expected_rejection),
            "{id}: {receipt:?}"
        );
    }

    fn rejected_open_office_turn(
        store: &mut KernelStore,
        session_id: RootAuthorityOfficeSessionId,
        command_id: &str,
    ) {
        let capability = Capability::OpenOfficeTurn;
        let receipt = store
            .execute(CommandRequest {
                command_id: CommandId::parse(command_id).unwrap(),
                principal_id: PrincipalId::new(3).unwrap(),
                capability_grant_id: store
                    .active_capability_grant(PrincipalId::new(3).unwrap(), capability)
                    .unwrap()
                    .unwrap(),
                capability,
                expected_generation: ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
                body: CommandBody::OpenOfficeTurn {
                    session_id,
                    purpose: OfficeTurnPurpose::OrdinaryWork,
                },
            })
            .unwrap();
        assert_eq!(
            receipt.disposition,
            CommandDisposition::Rejected(Rejection::InvalidLifecycleTransition)
        );
    }

    fn ready_office_child(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        fixture: &NativeFixture,
        office: &OfficeStart,
        operation_label: &str,
    ) -> super::OfficePiExecutionChild {
        let start = OfficePiExecutionStart {
            operation: PiExecutionOperationId::parse(operation_label).unwrap(),
            operating_cycle_id: office.cycle_id,
            office_session_id: office.session_id,
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            expected_generation: AdmissionGeneration::INITIAL,
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: office.epoch_identity.clone(),
            spawn_request: fixture.spawn_request(),
        };
        let mut child = match driver.admit_spawn_and_register(store, start).unwrap() {
            OfficePiSpawnRegistration::Ready(child) => child,
            other => panic!("M6 fixture must start a registered child: {other:?}"),
        };
        wait_for_adapter_ready(driver, store, &mut child);
        let progress = driver
            .authorize_and_begin_create(
                store,
                &mut child,
                MonotonicTick::from_milliseconds(1),
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap();
        if progress == crate::supervision::ControlWriteProgress::Pending {
            drive_create_until_delivered(driver, store, &mut child, 2, 1_000);
        }
        for tick in 2..1_000 {
            if driver
                .observe_session_ready(
                    store,
                    &mut child,
                    MonotonicTick::from_milliseconds(tick),
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
                )
                .unwrap()
            {
                assert_eq!(child.phase(), "office_ready_recorded");
                return child;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("provider-free fixture did not become Office ready")
    }

    fn seal_prompt_content(
        store: &mut KernelStore,
        fixture: &NativeFixture,
        operation_label: &str,
        text: &str,
    ) -> crate::content::ContentObjectRegistration {
        let authority = ContentSealingAuthority::open(
            ContentStoreRoot::parse(fixture.root.join("m6-prompt-content")).unwrap(),
            ContentSealLimit::new(4 * 1024 * 1024).unwrap(),
        )
        .unwrap();
        let digest = KernelDigest::of_bytes(text.as_bytes());
        let operation = ContentSealOperationId::parse(operation_label, digest).unwrap();
        authority
            .seal_and_register(store, &operation, text.as_bytes())
            .unwrap()
    }

    fn open_office_turn(
        store: &mut KernelStore,
        session_id: RootAuthorityOfficeSessionId,
        command_id: &str,
    ) -> (OfficeTurnId, society_kernel::EventId) {
        let capability = Capability::OpenOfficeTurn;
        let receipt = store
            .execute(CommandRequest {
                command_id: CommandId::parse(command_id).unwrap(),
                principal_id: PrincipalId::new(3).unwrap(),
                capability_grant_id: store
                    .active_capability_grant(PrincipalId::new(3).unwrap(), capability)
                    .unwrap()
                    .unwrap(),
                capability,
                expected_generation: ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
                body: CommandBody::OpenOfficeTurn {
                    session_id,
                    purpose: OfficeTurnPurpose::OrdinaryWork,
                },
            })
            .unwrap();
        let CommandDisposition::Accepted(event_id) = receipt.disposition else {
            panic!("M6 Office turn must open: {receipt:?}")
        };
        let turn_id = match store.ledger_event(event_id).unwrap().body {
            society_kernel::EventBody::OfficeTurnOpened { turn_id, .. } => turn_id,
            other => panic!("M6 open returned unexpected event: {other:?}"),
        };
        (turn_id, event_id)
    }

    fn quiesce_office_cycle(
        store: &mut KernelStore,
        cycle_id: OperatingCycleId,
    ) -> AdmissionGeneration {
        accepted(
            store,
            "m7-quiesce-office-cycle",
            PrincipalId::new(3).unwrap(),
            Capability::QuiesceOperatingCycle,
            ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
            CommandBody::QuiesceOperatingCycle { cycle_id },
        );
        AdmissionGeneration::INITIAL.increment().unwrap()
    }

    fn office_session_dispose_start(
        operation: &str,
        correlation: &str,
        expected_generation: AdmissionGeneration,
    ) -> OfficePiSessionDisposeStart {
        OfficePiSessionDisposeStart {
            operation: PiOfficeSessionDisposeOperationId::parse(operation).unwrap(),
            correlation_identity: society_kernel::PiCorrelationIdentity::parse(correlation)
                .unwrap(),
            expected_generation,
        }
    }

    fn office_turn_start(
        operation: &str,
        office_turn_id: OfficeTurnId,
        correlation: &str,
        prompt_content_object_id: society_kernel::ContentObjectId,
        digest: KernelDigest,
        text: &str,
        frontier_event_id: society_kernel::EventId,
    ) -> OfficePiTurnStart {
        OfficePiTurnStart {
            operation: PiOfficeTurnOperationId::parse(operation).unwrap(),
            office_turn_id,
            correlation_identity: society_kernel::PiCorrelationIdentity::parse(correlation)
                .unwrap(),
            prompt_content_object_id,
            prompt: SealedOfficePrompt::new(text.to_owned(), digest).unwrap(),
            frontier_event_id,
        }
    }

    fn drive_prompt_until_delivered(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        child: &mut super::OfficePiExecutionChild,
        turn: &mut super::OfficePiTurn,
        start_tick: u64,
        deadline_tick: u64,
    ) {
        for tick in start_tick..deadline_tick {
            if driver
                .drive_office_turn_prompt_delivery(
                    store,
                    child,
                    turn,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
                == crate::supervision::ControlWriteProgress::Delivered
            {
                return;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("M6 Prompt did not reach a complete physical write")
    }

    fn drive_turn_until_terminal(
        driver: &mut PiExecutionDriver,
        store: &mut KernelStore,
        child: &mut super::OfficePiExecutionChild,
        turn: &mut super::OfficePiTurn,
        start_tick: u64,
        deadline_tick: u64,
    ) -> Vec<OfficePiTurnOutput> {
        let mut observed = Vec::new();
        for tick in start_tick..deadline_tick {
            if let Some(output) = driver
                .observe_office_turn_output(
                    store,
                    child,
                    turn,
                    MonotonicTick::from_milliseconds(tick),
                )
                .unwrap()
            {
                if matches!(
                    &output,
                    OfficePiTurnOutput::SettledReady | OfficePiTurnOutput::TerminalRecordedNonReady
                ) {
                    observed.push(output);
                    return observed;
                }
                observed.push(output);
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("M6 host did not emit a terminal peer chain")
    }

    struct NativeFixture {
        /// Synthetic monotonic deadlines intentionally advance independently
        /// of host scheduling. Serializing only these native-host fixture
        /// lifetimes prevents unrelated Node startup/PGID churn in Rust's
        /// parallel unit runner from changing a process-physics assertion.
        /// Dedicated supervision integration tests remain the explicit
        /// concurrency judge.
        _process_physics_guard: MutexGuard<'static, ()>,
        root: PathBuf,
        workspace: NativeWorkspace,
        session: SessionIdentity,
        nonce: SpawnNonce,
        host: QualifiedHostExecution,
        create: CreateSessionPayload,
    }

    impl NativeFixture {
        fn new(label: &str) -> Self {
            let process_physics_guard = PROCESS_PHYSICS_FIXTURE_GUARD
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let nonce = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "society-pi-execution-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&root).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
            let workspace_root = root.join("workspaces");
            fs::create_dir(&workspace_root).unwrap();
            fs::set_permissions(&workspace_root, fs::Permissions::from_mode(0o700)).unwrap();
            let workspace = NativeWorkspaceRoot::open_owned(&workspace_root)
                .unwrap()
                .allocate(NativeWorkspaceId::parse(format!("workspace-{nonce}")).unwrap())
                .unwrap();
            let agent = workspace.directory().as_path().join("agent");
            let session_dir = workspace.directory().as_path().join("sessions");
            fs::create_dir(&agent).unwrap();
            fs::create_dir(&session_dir).unwrap();
            fs::set_permissions(&agent, fs::Permissions::from_mode(0o700)).unwrap();
            fs::set_permissions(&session_dir, fs::Permissions::from_mode(0o700)).unwrap();
            let auth = agent.join("auth.json");
            let models = agent.join("models.json");
            fs::write(&auth, "{}").unwrap();
            let catalog_json = models_json();
            fs::write(&models, &catalog_json).unwrap();
            let node = node_executable();
            let double = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join("tests/supervision/pi-host-double.mjs");
            let node_digest = digest_file(&node);
            let double_digest = digest_file(&double);
            let host = QualifiedHostExecution {
                node_executable: VerifiedArtifact::inspect(&node, node_digest.clone()).unwrap(),
                adapter_entrypoint: VerifiedArtifact::inspect(&double, double_digest.clone())
                    .unwrap(),
                lockfile: VerifiedArtifact::inspect(&double, double_digest.clone()).unwrap(),
                pi_transitive_package_set: VerifiedArtifact::inspect(
                    &double,
                    double_digest.clone(),
                )
                .unwrap(),
                runtime: RuntimeIdentity {
                    node_version: NodeRuntimeVersion::parse(node_version()).unwrap(),
                    adapter_version: AdapterVersion::V1,
                    pi_sdk_version: PiSdkVersion::V0830,
                    node_executable_blake3: node_digest,
                    lockfile_blake3: double_digest.clone(),
                    adapter_build_blake3: double_digest.clone(),
                    pi_transitive_package_set_blake3: double_digest,
                },
            };
            let prompt = "Founding Mission\nM5 provider-free Office bootstrap".to_owned();
            let create = CreateSessionPayload {
                session_kind: SessionKind::RootAuthorityOffice,
                cwd: workspace.directory().clone(),
                agent_directory: absolute(agent),
                auth_path: absolute(auth),
                models_path: absolute(models),
                session_directory: absolute(session_dir),
                system_prompt_digest: digest_bytes(prompt.as_bytes()),
                system_prompt: prompt,
                model: ModelSelection {
                    provider: Provider::OpenRouter,
                    model_id: ModelId::DeepseekV4Flash0731,
                    thinking_level: ThinkingLevel::High,
                },
                model_catalog: {
                    let mut catalog = model_catalog();
                    catalog.catalog_blake3 = digest_bytes(catalog_json.as_bytes());
                    catalog
                },
                tool_profile: ToolProfile::ReadExecuteV1,
                settings: settings(),
                forum_contract: ForumSessionContractV1::forum_enabled_v1().unwrap(),
            };
            Self {
                _process_physics_guard: process_physics_guard,
                root,
                workspace,
                session: SessionIdentity::parse(format!("session-{label}-{nonce}")).unwrap(),
                nonce: SpawnNonce::parse(format!("spawn-{label}-{nonce}")).unwrap(),
                host,
                create,
            }
        }

        fn spawn_request(&self) -> PiSpawnRequest {
            PiSpawnRequest {
                child_process_id: SupervisedChildId::parse(format!(
                    "child-{}",
                    self.session.as_str()
                ))
                .unwrap(),
                workspace: self.workspace.clone(),
                session_identity: self.session.clone(),
                spawn_nonce: self.nonce.clone(),
                host_execution: self.host.clone(),
                environment: NativeHostEnvironment::EmptyV1,
                create_correlation_identity: CorrelationIdentity::parse("create-office-bridge")
                    .unwrap(),
                create_session: self.create.clone(),
            }
        }

        fn task_attempt_spawn_request(&self) -> PiSpawnRequest {
            let mut request = self.spawn_request();
            request.create_correlation_identity =
                CorrelationIdentity::parse("create-task-attempt-bridge").unwrap();
            request.create_session.session_kind = SessionKind::TaskAttempt;
            request.create_session.tool_profile = ToolProfile::ForumIsolatedV1;
            request
        }

        fn cleanup(self) {
            fs::remove_dir_all(self.root).unwrap();
        }
    }

    fn model_catalog() -> ModelCatalogPolicyV1 {
        ModelCatalogPolicyV1 {
            catalog_blake3: Blake3Digest::parse("a".repeat(64)).unwrap(),
            effective_model: EffectiveModelDescriptorV1 {
                provider: Provider::OpenRouter,
                base_url: OpenRouterBaseUrl::ApiV1,
                api: ModelApi::OpenAiCompletions,
                model_id: ModelId::DeepseekV4Flash0731,
                canonical_slug: CanonicalModelSlug::DeepseekV4Flash20260731,
                input: ModelInput::TextOnly,
                context_window: PositiveInteger::parse(1_048_576).unwrap(),
                max_tokens: PositiveInteger::parse(384_000).unwrap(),
                input_usd_per_million: rate("0.09"),
                output_usd_per_million: rate("0.18"),
                cache_read_usd_per_million: rate("0.018"),
                cache_write_usd_per_million: CacheWritePerMillionRateV1::Absent,
            },
        }
    }
    fn models_json() -> String {
        "{\"providers\":{\"openrouter\":{\"baseUrl\":\"https://openrouter.ai/api/v1\",\"api\":\"openai-completions\",\"models\":[{\"id\":\"deepseek/deepseek-v4-flash-0731\",\"name\":\"admitted\",\"reasoning\":true,\"input\":[\"text\"],\"contextWindow\":1048576,\"maxTokens\":384000,\"cost\":{\"input\":0.00000009,\"output\":0.00000018,\"cacheRead\":0.000000018,\"cacheWrite\":0}}]}}}"
            .to_owned()
    }
    fn rate(value: &str) -> KnownPerMillionRateV1 {
        KnownPerMillionRateV1 {
            usd_per_million: UsdPerMillionDecimal::parse(value).unwrap(),
        }
    }
    fn settings() -> ActorModelPolicyV1 {
        ActorModelPolicyV1 {
            retry: RetryPolicyV1 {
                max_retries: NonNegativeInteger::parse(2).unwrap(),
                base_delay_milliseconds: NonNegativeInteger::parse(2_000).unwrap(),
                provider_timeout_milliseconds: PositiveInteger::parse(300_000).unwrap(),
                provider_max_retries: NonNegativeInteger::parse(1).unwrap(),
                provider_max_retry_delay_milliseconds: PositiveInteger::parse(30_000).unwrap(),
            },
            compaction: CompactionPolicyV1 {
                mode: CompactionMode::Enabled,
                reserve_tokens: NonNegativeInteger::parse(16_384).unwrap(),
                keep_recent_tokens: NonNegativeInteger::parse(20_000).unwrap(),
            },
            steering_mode: QueueMode::OneAtATime,
            follow_up_mode: QueueMode::OneAtATime,
            transport: Transport::Sse,
            project_trust: ProjectTrust::Never,
            install_telemetry: Disabled::Disabled,
            analytics: Disabled::Disabled,
            images: Images::Blocked,
        }
    }
    fn node_executable() -> PathBuf {
        let output = Command::new("node")
            .args(["-p", "process.execPath"])
            .output()
            .unwrap();
        assert!(output.status.success());
        fs::canonicalize(String::from_utf8(output.stdout).unwrap().trim()).unwrap()
    }
    fn node_version() -> String {
        let output = Command::new("node").arg("--version").output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }
    fn absolute(path: PathBuf) -> AbsolutePath {
        AbsolutePath::parse(fs::canonicalize(path).unwrap().to_str().unwrap()).unwrap()
    }
    fn digest_file(path: &Path) -> Blake3Digest {
        digest_bytes(&fs::read(path).unwrap())
    }
    fn digest_bytes(bytes: &[u8]) -> Blake3Digest {
        let mut rendered = String::with_capacity(64);
        for byte in blake3::hash(bytes).as_bytes() {
            use std::fmt::Write as _;
            write!(&mut rendered, "{byte:02x}").unwrap();
        }
        Blake3Digest::parse(rendered).unwrap()
    }
}
