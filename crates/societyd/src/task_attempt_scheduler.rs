//! Daemon-owned coordinator and in-process runner for live study TaskAttempt children.
//!
//! The existing [`crate::pi_execution`] bridge is intentionally an Office
//! bridge and rejects `SessionKind::TaskAttempt`.  This module therefore does
//! not wrap or relabel that driver. It defines the narrow state handoff around
//! the resident TaskAttempt Pi driver: the driver owns process physics, while
//! this coordinator owns the study obligation/attempt join and the ordering
//! around the two already-typed study runtime transitions.
//!
//! No application descriptor, prompt bytes, JSON value, Office session, or
//! provider identity crosses this module.  The application remains outside
//! the daemon; the scheduler obtains its sealed plan through the existing
//! content-custody boundary and opens a typed kernel launch claim before this
//! runner materializes a native child.

use std::collections::BTreeMap;

use society_kernel::{
    ActorAttemptId, AdmissionGeneration, Blake3Digest, BudgetReservationId, CommandId,
    ContentObjectId, EventId, ExecutionProfileId, NativeChildId, NativeChildSpawnAdmissionId,
    OperatingCycleId, PiCorrelationIdentity, StudyActorObligationId,
    StudyActorTaskAttemptLaunchClaim, StudyActorTaskAttemptLaunchId, StudyCommand,
    StudyTransitionReceipt, SupervisorEpochId, SupervisorEpochIdentity,
};
use society_pi::{CorrelationIdentity, ForumToolName, SdkJsonValue, ToolCallIdentity};
use thiserror::Error;

use crate::{
    Daemon,
    pi_execution::{
        PiExecutionError, PiExecutionOperationId, PiTaskAttemptPromptOperationId,
        PiTaskAttemptSessionDisposeOperationId, SealedTaskAttemptPrompt,
        TaskAttemptPiExecutionChild, TaskAttemptPiExecutionStart, TaskAttemptPiPrompt,
        TaskAttemptPiPromptOutput, TaskAttemptPiPromptStart, TaskAttemptPiSessionDispose,
        TaskAttemptPiSessionDisposeOutput, TaskAttemptPiSessionDisposeStart,
        TaskAttemptPiSpawnRegistration,
    },
    supervision::{
        ControlWriteDeadline, ControlWriteProgress, HandshakeDeadline, MonotonicTick,
        PiSpawnRequest,
    },
};

const COMMAND_PREFIX: &str = "study-task-attempt-v1";

/// The exact generic identities which join one study obligation to one
/// replaceable actor attempt.  Keeping both IDs in one value prevents a
/// scheduler from accidentally binding a successor attempt to a predecessor
/// obligation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct TaskAttemptScheduleKey {
    pub(crate) obligation_id: StudyActorObligationId,
    pub(crate) actor_attempt_id: ActorAttemptId,
}

impl TaskAttemptScheduleKey {
    pub(crate) const fn new(
        obligation_id: StudyActorObligationId,
        actor_attempt_id: ActorAttemptId,
    ) -> Self {
        Self {
            obligation_id,
            actor_attempt_id,
        }
    }
}

/// Spawn input selected by trusted scheduling.  It is deliberately only a
/// typed join key: execution profile, workspace, budget, Pi boundary identity,
/// and spawn nonce must come from the kernel admission/driver seam rather than
/// from a generic scheduler payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TaskAttemptSpawnRequest {
    pub(crate) key: TaskAttemptScheduleKey,
}

/// Native custody identities returned by a TaskAttempt driver after it has
/// committed the exact child PID/PGID receipt.  The coordinator never accepts
/// an Office session identity as a substitute for `actor_attempt_id`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TaskAttemptSpawnReceipt {
    pub(crate) actor_attempt_id: ActorAttemptId,
    pub(crate) native_child_id: NativeChildId,
    pub(crate) native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
}

/// Physical progress is intentionally narrower than actor semantics.  The
/// driver may report readiness, disposal, and final native reconciliation, but
/// it cannot report a Forum finding, measurement, truth value, or actor
/// completion through this coordinator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskAttemptPiProgress {
    Waiting,
    Ready,
    Disposed,
    /// The driver has entered its fixed physical containment suffix. It must
    /// continue reporting that suffix until the native child is reconciled;
    /// this is never an actor-semantic failure or a not-spawned rewrite.
    ContainmentRequired,
    Reconciled,
}

/// Closed driver failure vocabulary.  A driver must contain/reconcile a child
/// before returning one of these failures; the coordinator never silently
/// converts a spawned child into `NotSpawned`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskAttemptDriverFailure {
    PreflightRejected,
    NativeSpawnFailed,
    BoundaryContainmentRequired,
    ProtocolFailed,
    PhysicalReconciliationFailed,
}

/// The future TaskAttempt Pi driver contract.  It is crate-private and has no
/// wire representation.  A concrete implementation must use the key to derive
/// its own operation identities and return the exact native custody IDs which
/// the kernel admitted for that ActorAttempt.
pub(crate) trait TaskAttemptPiDriver {
    fn spawn_task_attempt(
        &mut self,
        request: TaskAttemptSpawnRequest,
    ) -> Result<TaskAttemptSpawnReceipt, TaskAttemptDriverFailure>;

    fn drive_task_attempt(
        &mut self,
        child: TaskAttemptSpawnReceipt,
        now: MonotonicTick,
    ) -> Result<TaskAttemptPiProgress, TaskAttemptDriverFailure>;
}

/// Durable coordinator phase.  `PhysicalReconciled` is deliberately distinct
/// from `Reconciled`: the latter is reached only after the kernel accepts the
/// typed `ReconcileActorRuntime` transition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskAttemptScheduleState {
    SpawnPending,
    Spawned,
    RuntimeBound,
    Ready,
    Disposed,
    ContainmentRequired,
    PhysicalReconciled,
    Reconciled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TaskAttemptScheduleEntry {
    child: Option<TaskAttemptSpawnReceipt>,
    state: TaskAttemptScheduleState,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub(crate) enum TaskAttemptSchedulerError {
    #[error("the study obligation/actor-attempt key is already scheduled")]
    DuplicateSchedule,
    #[error("the study obligation/actor-attempt key is not scheduled")]
    UnknownSchedule,
    #[error("the TaskAttempt schedule is not in the required lifecycle phase")]
    InvalidLifecycle,
    #[error("the TaskAttempt driver returned a child owned by another actor attempt")]
    DriverIdentityMismatch,
    #[error("the TaskAttempt driver returned progress outside the closed lifecycle")]
    UnexpectedDriverProgress,
    #[error("the TaskAttempt driver failed: {0:?}")]
    Driver(TaskAttemptDriverFailure),
    #[error("the derived TaskAttempt kernel command identity is invalid")]
    InvalidCommandIdentity,
}

/// Single-threaded daemon-private state for live TaskAttempt schedules.  The
/// daemon control loop is the sole owner; this type intentionally has no
/// `Send`/wire/serialization contract and no recovery attach behavior.
#[derive(Default)]
pub(crate) struct TaskAttemptScheduler {
    schedules: BTreeMap<TaskAttemptScheduleKey, TaskAttemptScheduleEntry>,
}

impl TaskAttemptScheduler {
    /// Starts one exact TaskAttempt driver child.  The entry is retained as
    /// `SpawnPending` while the driver performs its pre-spawn and native
    /// registration work, so a duplicate request cannot race it in the same
    /// daemon loop.
    pub(crate) fn spawn<D: TaskAttemptPiDriver>(
        &mut self,
        driver: &mut D,
        key: TaskAttemptScheduleKey,
    ) -> Result<TaskAttemptSpawnReceipt, TaskAttemptSchedulerError> {
        if self.schedules.contains_key(&key) {
            return Err(TaskAttemptSchedulerError::DuplicateSchedule);
        }
        self.schedules.insert(
            key,
            TaskAttemptScheduleEntry {
                child: None,
                state: TaskAttemptScheduleState::SpawnPending,
            },
        );
        let receipt = match driver.spawn_task_attempt(TaskAttemptSpawnRequest { key }) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.schedules.remove(&key);
                return Err(TaskAttemptSchedulerError::Driver(error));
            }
        };
        if receipt.actor_attempt_id != key.actor_attempt_id {
            // Keep the exact returned custody IDs associated with this entry;
            // a future driver integration must use them for containment rather
            // than losing a child whose owner identity was malformed.
            let entry = self
                .schedules
                .get_mut(&key)
                .ok_or(TaskAttemptSchedulerError::UnknownSchedule)?;
            entry.child = Some(receipt);
            entry.state = TaskAttemptScheduleState::ContainmentRequired;
            return Err(TaskAttemptSchedulerError::DriverIdentityMismatch);
        }
        let entry = self
            .schedules
            .get_mut(&key)
            .ok_or(TaskAttemptSchedulerError::UnknownSchedule)?;
        entry.child = Some(receipt);
        entry.state = TaskAttemptScheduleState::Spawned;
        Ok(receipt)
    }

    /// Inserts a receipt produced by the resident's concrete Pi bridge. The
    /// original generic `spawn` method remains for provider-free coordinator
    /// tests; this method is the production handoff and never accepts a
    /// caller-supplied native identity unrelated to the schedule key.
    pub(crate) fn record_spawned(
        &mut self,
        key: TaskAttemptScheduleKey,
        receipt: TaskAttemptSpawnReceipt,
    ) -> Result<(), TaskAttemptSchedulerError> {
        if self.schedules.contains_key(&key) {
            return Err(TaskAttemptSchedulerError::DuplicateSchedule);
        }
        if receipt.actor_attempt_id != key.actor_attempt_id {
            return Err(TaskAttemptSchedulerError::DriverIdentityMismatch);
        }
        self.schedules.insert(
            key,
            TaskAttemptScheduleEntry {
                child: Some(receipt),
                state: TaskAttemptScheduleState::Spawned,
            },
        );
        Ok(())
    }

    pub(crate) fn record_containment_required(
        &mut self,
        key: TaskAttemptScheduleKey,
    ) -> Result<(), TaskAttemptSchedulerError> {
        let entry = self.entry_mut(key)?;
        if !matches!(
            entry.state,
            TaskAttemptScheduleState::Spawned
                | TaskAttemptScheduleState::RuntimeBound
                | TaskAttemptScheduleState::Ready
                | TaskAttemptScheduleState::Disposed
        ) {
            return Err(TaskAttemptSchedulerError::InvalidLifecycle);
        }
        entry.state = TaskAttemptScheduleState::ContainmentRequired;
        Ok(())
    }

    /// Builds the only kernel command which may establish the study/runtime
    /// join.  The caller must submit this exact value through the daemon's
    /// resident `KernelStore` and call [`Self::record_runtime_bound`] only
    /// after an accepted `ActorTaskAttemptRuntimeBound` receipt.
    pub(crate) fn runtime_binding_command(
        &self,
        key: TaskAttemptScheduleKey,
    ) -> Result<StudyCommand, TaskAttemptSchedulerError> {
        let entry = self.entry(key)?;
        if entry.state != TaskAttemptScheduleState::Spawned {
            return Err(TaskAttemptSchedulerError::InvalidLifecycle);
        }
        let child = entry
            .child
            .ok_or(TaskAttemptSchedulerError::InvalidLifecycle)?;
        Ok(StudyCommand::BindActorTaskAttemptRuntime {
            obligation_id: key.obligation_id,
            actor_attempt_id: key.actor_attempt_id,
            native_child_id: child.native_child_id,
            native_child_spawn_admission_id: child.native_child_spawn_admission_id,
        })
    }

    /// Marks the study/runtime join after the resident kernel transition was
    /// accepted.  This is separate from command construction so a rejected
    /// binding cannot advance the local scheduler.
    pub(crate) fn record_runtime_bound(
        &mut self,
        key: TaskAttemptScheduleKey,
    ) -> Result<(), TaskAttemptSchedulerError> {
        let entry = self.entry_mut(key)?;
        if entry.state != TaskAttemptScheduleState::Spawned {
            return Err(TaskAttemptSchedulerError::InvalidLifecycle);
        }
        entry.state = TaskAttemptScheduleState::RuntimeBound;
        Ok(())
    }

    /// Records the generic SessionReady boundary after the resident has
    /// accepted it from the exact child.  Keeping this transition here makes
    /// the concrete runner use the same closed lifecycle as the coordinator
    /// test double; it cannot skip directly from binding to disposal.
    pub(crate) fn record_ready(
        &mut self,
        key: TaskAttemptScheduleKey,
    ) -> Result<(), TaskAttemptSchedulerError> {
        let entry = self.entry_mut(key)?;
        if entry.state != TaskAttemptScheduleState::RuntimeBound {
            return Err(TaskAttemptSchedulerError::InvalidLifecycle);
        }
        entry.state = TaskAttemptScheduleState::Ready;
        Ok(())
    }

    /// Records the task-session Dispose receipt after the daemon has committed
    /// the exact transcript/usage chain.
    pub(crate) fn record_disposed(
        &mut self,
        key: TaskAttemptScheduleKey,
    ) -> Result<(), TaskAttemptSchedulerError> {
        let entry = self.entry_mut(key)?;
        if entry.state != TaskAttemptScheduleState::Ready {
            return Err(TaskAttemptSchedulerError::InvalidLifecycle);
        }
        entry.state = TaskAttemptScheduleState::Disposed;
        Ok(())
    }

    /// Records the native physical suffix only after the process bridge has
    /// durably reconciled the direct child and owned process group.
    pub(crate) fn record_physical_reconciled(
        &mut self,
        key: TaskAttemptScheduleKey,
    ) -> Result<(), TaskAttemptSchedulerError> {
        let entry = self.entry_mut(key)?;
        if entry.state != TaskAttemptScheduleState::Disposed
            && entry.state != TaskAttemptScheduleState::ContainmentRequired
        {
            return Err(TaskAttemptSchedulerError::InvalidLifecycle);
        }
        entry.state = TaskAttemptScheduleState::PhysicalReconciled;
        Ok(())
    }

    /// Drives one nonblocking physical step.  Actor Forum calls and semantic
    /// obligation completion remain separate typed coordinator operations; no
    /// application result can be smuggled in through `TaskAttemptPiProgress`.
    pub(crate) fn drive<D: TaskAttemptPiDriver>(
        &mut self,
        driver: &mut D,
        key: TaskAttemptScheduleKey,
        now: MonotonicTick,
    ) -> Result<TaskAttemptScheduleState, TaskAttemptSchedulerError> {
        let entry = self.entry(key)?;
        let child = entry
            .child
            .ok_or(TaskAttemptSchedulerError::InvalidLifecycle)?;
        if !matches!(
            entry.state,
            TaskAttemptScheduleState::RuntimeBound
                | TaskAttemptScheduleState::Ready
                | TaskAttemptScheduleState::Disposed
                | TaskAttemptScheduleState::ContainmentRequired
        ) {
            return Err(TaskAttemptSchedulerError::InvalidLifecycle);
        }
        let progress = driver
            .drive_task_attempt(child, now)
            .map_err(TaskAttemptSchedulerError::Driver)?;
        let entry = self.entry_mut(key)?;
        match (entry.state, progress) {
            (TaskAttemptScheduleState::RuntimeBound, TaskAttemptPiProgress::Waiting) => {}
            (TaskAttemptScheduleState::RuntimeBound, TaskAttemptPiProgress::Ready) => {
                entry.state = TaskAttemptScheduleState::Ready
            }
            (TaskAttemptScheduleState::Ready, TaskAttemptPiProgress::Waiting)
            | (TaskAttemptScheduleState::Ready, TaskAttemptPiProgress::Ready) => {}
            (TaskAttemptScheduleState::Ready, TaskAttemptPiProgress::Disposed) => {
                entry.state = TaskAttemptScheduleState::Disposed
            }
            (TaskAttemptScheduleState::Disposed, TaskAttemptPiProgress::Waiting)
            | (TaskAttemptScheduleState::Disposed, TaskAttemptPiProgress::Disposed) => {}
            (TaskAttemptScheduleState::Disposed, TaskAttemptPiProgress::Reconciled) => {
                entry.state = TaskAttemptScheduleState::PhysicalReconciled
            }
            (
                TaskAttemptScheduleState::RuntimeBound
                | TaskAttemptScheduleState::Ready
                | TaskAttemptScheduleState::Disposed,
                TaskAttemptPiProgress::ContainmentRequired,
            ) => entry.state = TaskAttemptScheduleState::ContainmentRequired,
            (
                TaskAttemptScheduleState::ContainmentRequired,
                TaskAttemptPiProgress::Waiting | TaskAttemptPiProgress::ContainmentRequired,
            ) => {}
            (TaskAttemptScheduleState::ContainmentRequired, TaskAttemptPiProgress::Reconciled) => {
                entry.state = TaskAttemptScheduleState::PhysicalReconciled
            }
            _ => return Err(TaskAttemptSchedulerError::UnexpectedDriverProgress),
        }
        Ok(entry.state)
    }

    /// Builds the typed kernel command which closes the study/runtime join
    /// after the TaskAttempt driver's physical reconciliation.  The caller
    /// must submit it through the daemon resident store before calling
    /// [`Self::record_runtime_reconciled`].
    pub(crate) fn runtime_reconciliation_command(
        &self,
        key: TaskAttemptScheduleKey,
    ) -> Result<StudyCommand, TaskAttemptSchedulerError> {
        let entry = self.entry(key)?;
        if entry.state != TaskAttemptScheduleState::PhysicalReconciled {
            return Err(TaskAttemptSchedulerError::InvalidLifecycle);
        }
        let child = entry
            .child
            .ok_or(TaskAttemptSchedulerError::InvalidLifecycle)?;
        Ok(StudyCommand::ReconcileActorRuntime {
            obligation_id: key.obligation_id,
            native_child_id: child.native_child_id,
        })
    }

    /// Marks the runtime reconciliation only after the resident kernel accepts
    /// `ReconcileActorRuntime`.  The resulting `Reconciled` state is the point
    /// at which a later coordinator may complete/fail the study obligation.
    pub(crate) fn record_runtime_reconciled(
        &mut self,
        key: TaskAttemptScheduleKey,
    ) -> Result<(), TaskAttemptSchedulerError> {
        let entry = self.entry_mut(key)?;
        if entry.state != TaskAttemptScheduleState::PhysicalReconciled {
            return Err(TaskAttemptSchedulerError::InvalidLifecycle);
        }
        entry.state = TaskAttemptScheduleState::Reconciled;
        Ok(())
    }

    pub(crate) fn state(
        &self,
        key: TaskAttemptScheduleKey,
    ) -> Result<TaskAttemptScheduleState, TaskAttemptSchedulerError> {
        Ok(self.entry(key)?.state)
    }

    pub(crate) fn child(
        &self,
        key: TaskAttemptScheduleKey,
    ) -> Result<TaskAttemptSpawnReceipt, TaskAttemptSchedulerError> {
        self.entry(key)?
            .child
            .ok_or(TaskAttemptSchedulerError::InvalidLifecycle)
    }

    fn entry(
        &self,
        key: TaskAttemptScheduleKey,
    ) -> Result<&TaskAttemptScheduleEntry, TaskAttemptSchedulerError> {
        self.schedules
            .get(&key)
            .ok_or(TaskAttemptSchedulerError::UnknownSchedule)
    }

    fn entry_mut(
        &mut self,
        key: TaskAttemptScheduleKey,
    ) -> Result<&mut TaskAttemptScheduleEntry, TaskAttemptSchedulerError> {
        self.schedules
            .get_mut(&key)
            .ok_or(TaskAttemptSchedulerError::UnknownSchedule)
    }
}

/// Stable command identity for the resident's runtime-binding transition.
/// Command IDs are idempotent and include both sides of the typed join.
pub(crate) fn runtime_binding_command_id(
    key: TaskAttemptScheduleKey,
) -> Result<CommandId, TaskAttemptSchedulerError> {
    CommandId::parse(format!(
        "{COMMAND_PREFIX}/obligation-{}/attempt-{}/bind",
        key.obligation_id.value(),
        key.actor_attempt_id.value()
    ))
    .map_err(|_| TaskAttemptSchedulerError::InvalidCommandIdentity)
}

/// Stable command identity for the resident's runtime-reconciliation
/// transition.  It is a distinct command from binding so replay can prove the
/// physical suffix was accepted before study closure.
pub(crate) fn runtime_reconciliation_command_id(
    key: TaskAttemptScheduleKey,
) -> Result<CommandId, TaskAttemptSchedulerError> {
    CommandId::parse(format!(
        "{COMMAND_PREFIX}/obligation-{}/attempt-{}/reconcile",
        key.obligation_id.value(),
        key.actor_attempt_id.value()
    ))
    .map_err(|_| TaskAttemptSchedulerError::InvalidCommandIdentity)
}

/// Stable operation identity selected by a trusted application/daemon
/// composition. It is deliberately the same compact grammar used by the
/// resident Pi bridge; callers cannot inject a command namespace or a
/// filesystem path through this value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskAttemptRunnerOperationId(String);

impl TaskAttemptRunnerOperationId {
    pub fn parse(value: impl Into<String>) -> Result<Self, TaskAttemptRunnerError> {
        let value = value.into();
        if !valid_operation_label(&value) {
            return Err(TaskAttemptRunnerError::InvalidOperationIdentity);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Exact generic admission selected by the daemon's kernel-claim consumer.
///
/// This is deliberately crate-private.  A `PiSpawnRequest` carries executable,
/// workspace, environment, and session-boundary choices, so accepting it from
/// an application would let the application manufacture native custody.  The
/// eventual public entry point must construct this value only from an opaque
/// durable launch claim.
#[derive(Clone, Debug)]
pub(crate) struct TaskAttemptExecutionStart {
    pub(crate) obligation_id: StudyActorObligationId,
    pub(crate) operating_cycle_id: OperatingCycleId,
    pub(crate) actor_attempt_id: ActorAttemptId,
    pub(crate) budget_reservation_id: BudgetReservationId,
    pub(crate) execution_profile_id: ExecutionProfileId,
    pub(crate) expected_generation: AdmissionGeneration,
    pub(crate) supervisor_epoch_id: SupervisorEpochId,
    pub(crate) supervisor_epoch_identity: SupervisorEpochIdentity,
    pub(crate) spawn_request: PiSpawnRequest,
}

/// One digest-bound TaskAttempt prompt. Prompt bytes must have already been
/// sealed through [`crate::StudyAdmissionAuthority`] (or an equivalent
/// resident content authority); the runner checks that the supplied bytes
/// agree with the digest before crossing the Pi process boundary.
#[derive(Clone, Debug)]
pub struct TaskAttemptPromptRequest {
    pub operation: TaskAttemptRunnerOperationId,
    pub correlation_identity: PiCorrelationIdentity,
    pub prompt_content_object_id: ContentObjectId,
    pub prompt_digest: Blake3Digest,
    pub prompt: String,
    pub frontier_event_id: EventId,
}

/// Retry-stable identity for the one TaskAttempt session Dispose control.
#[derive(Clone, Debug)]
pub struct TaskAttemptDisposeRequest {
    pub operation: TaskAttemptRunnerOperationId,
    pub correlation_identity: PiCorrelationIdentity,
}

/// Public Forum observation returned by the daemon-owned runner. The runner
/// never exposes a process handle, workspace path, raw transcript, or generic
/// actor result. A Forum call must be handed back to
/// [`TaskAttemptRunner::route_forum_tool_call`] before the actor receives its
/// result.
#[derive(Clone, Debug)]
pub enum TaskAttemptPromptEvent {
    ControlInterleaving,
    PromptAccepted,
    ForumToolCall {
        correlation_identity: CorrelationIdentity,
        tool_call_identity: ToolCallIdentity,
        tool_name: ForumToolName,
        args: SdkJsonValue,
    },
    KnownUsageRecorded,
    UsageFrozen,
    TerminalRecorded,
}

/// Public Dispose observation. Transcript bytes remain daemon-owned and are
/// sealed/registered before the runner emits `Disposed`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskAttemptDisposeEvent {
    DeliveryRecorded,
    Accepted,
    KnownUsageRecorded,
    /// Accounting is unavailable. The caller must use
    /// [`TaskAttemptRunner::drive_containment`] when no later transcript
    /// terminal is available; the observation itself does not mutate the
    /// schedule entry.
    UsageFrozen,
    Disposed,
}

/// Errors intentionally expose lifecycle classes rather than private process
/// or PostgreSQL implementation details. The daemon retains the exact typed
/// failure internally and keeps the child in its containment suffix whenever
/// a native boundary has already been crossed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum TaskAttemptRunnerError {
    #[error("TaskAttempt operation identity is not canonical")]
    InvalidOperationIdentity,
    #[error("TaskAttempt input is invalid for the closed daemon boundary")]
    InvalidInput,
    #[error("the daemon is recovery-fenced")]
    RecoveryFenced,
    #[error("the TaskAttempt lifecycle is not in the required phase")]
    InvalidLifecycle,
    #[error("the kernel rejected or could not persist the TaskAttempt transition")]
    Kernel,
    #[error("native process supervision failed")]
    Process,
    #[error("the Pi boundary protocol failed")]
    Protocol,
    #[error("daemon content custody failed")]
    Content,
    #[error("native containment or reconciliation is required")]
    ContainmentRequired,
    #[error("the TaskAttempt scheduler rejected the lifecycle transition")]
    Scheduler,
    #[error("no active kernel launch claim exists for this study obligation")]
    LaunchClaimMissing,
    #[error("resident pinned Pi launch profile rejected the TaskAttempt claim")]
    LaunchProfileUnavailable,
}

/// Failure to open a runner from the daemon-owned durable launch projection.
/// The public error intentionally does not expose a PostgreSQL connection or
/// query details.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum TaskAttemptRunnerOpenError {
    #[error("the daemon is recovery-fenced")]
    RecoveryFenced,
    #[error("no active kernel launch claim exists for this study obligation")]
    LaunchClaimMissing,
    #[error("the kernel launch-claim projection could not be read")]
    Kernel,
}

impl From<TaskAttemptSchedulerError> for TaskAttemptRunnerError {
    fn from(error: TaskAttemptSchedulerError) -> Self {
        match error {
            TaskAttemptSchedulerError::DriverIdentityMismatch
            | TaskAttemptSchedulerError::UnexpectedDriverProgress
            | TaskAttemptSchedulerError::Driver(
                TaskAttemptDriverFailure::BoundaryContainmentRequired,
            )
            | TaskAttemptSchedulerError::Driver(
                TaskAttemptDriverFailure::PhysicalReconciliationFailed,
            ) => Self::ContainmentRequired,
            TaskAttemptSchedulerError::InvalidCommandIdentity => Self::InvalidInput,
            TaskAttemptSchedulerError::DuplicateSchedule
            | TaskAttemptSchedulerError::UnknownSchedule
            | TaskAttemptSchedulerError::InvalidLifecycle
            | TaskAttemptSchedulerError::Driver(_) => Self::Scheduler,
        }
    }
}

impl From<PiExecutionError> for TaskAttemptRunnerError {
    fn from(error: PiExecutionError) -> Self {
        match error {
            PiExecutionError::RecoveryFenced => Self::RecoveryFenced,
            PiExecutionError::InvalidOperationIdentity
            | PiExecutionError::IdentityConversion
            | PiExecutionError::TaskAttemptSessionKindRequired
            | PiExecutionError::PromptContentDigestMismatch
            | PiExecutionError::TranscriptContentUnexpected
            | PiExecutionError::TranscriptContentMissing => Self::InvalidInput,
            PiExecutionError::InvalidLifecycle => Self::InvalidLifecycle,
            PiExecutionError::BoundaryProtocol(_) => Self::Protocol,
            PiExecutionError::Supervision(_) => Self::Process,
            PiExecutionError::Kernel(_)
            | PiExecutionError::KernelServiceCapabilityMissing { .. }
            | PiExecutionError::KernelCommandRejected { .. }
            | PiExecutionError::UnexpectedKernelEvent => Self::Kernel,
            PiExecutionError::Content(_) => Self::Content,
            PiExecutionError::AutomaticContainmentInaccessible
            | PiExecutionError::LingeringGroupInaccessible
            | PiExecutionError::ProcessGroupIdentityRegressed
            | PiExecutionError::MissingReapReceipt
            | PiExecutionError::ReapReceiptLost
            | PiExecutionError::ReceiptIdentityMismatch
            | PiExecutionError::SignalReceiptOrderingRequiresTwoPhaseReap => {
                Self::ContainmentRequired
            }
            _ => Self::Protocol,
        }
    }
}

/// The concrete resident TaskAttempt runner. It is single-threaded and
/// borrows the daemon for its whole lifetime, so the application cannot hold
/// a PostgreSQL connection, content writer, or native child handle beside it.
/// Every transition is performed by the resident's existing typed Pi bridge.
/// A public instance is opened only from a kernel launch claim. Its public
/// `spawn_from_launch_claim` operation asks the daemon to materialize the
/// pinned host/session profile and private workspace; the raw `spawn` seam is
/// retained only for resident-internal tests and composition.
pub struct TaskAttemptRunner<'daemon> {
    daemon: &'daemon mut Daemon,
    operation: TaskAttemptRunnerOperationId,
    scheduler: TaskAttemptScheduler,
    key: Option<TaskAttemptScheduleKey>,
    child: Option<TaskAttemptPiExecutionChild>,
    unregistered_child: Option<crate::pi_execution::UnregisteredPiChild>,
    prompt: Option<TaskAttemptPiPrompt>,
    pending_forum_call: Option<TaskAttemptPiPromptOutput>,
    dispose: Option<TaskAttemptPiSessionDispose>,
    launch_claim: Option<StudyActorTaskAttemptLaunchClaim>,
    actor_completion_recorded: bool,
}

impl<'daemon> TaskAttemptRunner<'daemon> {
    pub(crate) fn new(
        daemon: &'daemon mut Daemon,
        operation: TaskAttemptRunnerOperationId,
    ) -> Self {
        Self {
            daemon,
            operation,
            scheduler: TaskAttemptScheduler::default(),
            key: None,
            child: None,
            unregistered_child: None,
            prompt: None,
            pending_forum_call: None,
            dispose: None,
            launch_claim: None,
            actor_completion_recorded: false,
        }
    }

    pub(crate) fn new_from_launch_claim(
        daemon: &'daemon mut Daemon,
        operation: TaskAttemptRunnerOperationId,
        launch_claim: StudyActorTaskAttemptLaunchClaim,
    ) -> Self {
        let mut runner = Self::new(daemon, operation);
        runner.launch_claim = Some(launch_claim);
        runner
    }

    /// The durable claim identity which opened this runner.  It is an
    /// immutable kernel fact, not an actor/process handle or a spawn input.
    pub fn launch_claim_id(&self) -> Option<StudyActorTaskAttemptLaunchId> {
        self.launch_claim
            .as_ref()
            .map(StudyActorTaskAttemptLaunchClaim::launch_claim_id)
    }

    /// Starts the claimed TaskAttempt using only resident-owned native
    /// materialization. The application supplies no executable, workspace,
    /// session, environment, or child identity; those values are derived
    /// from the immutable kernel launch projection and the pinned daemon
    /// profile.
    pub fn spawn_from_launch_claim(
        &mut self,
    ) -> Result<TaskAttemptScheduleState, TaskAttemptRunnerError> {
        let claim = self
            .launch_claim
            .as_ref()
            .ok_or(TaskAttemptRunnerError::LaunchClaimMissing)?
            .clone();
        let start = self
            .daemon
            .task_attempt_execution_start_from_claim(&claim)
            .map_err(|_| TaskAttemptRunnerError::LaunchProfileUnavailable)?;
        self.spawn(start)
    }

    /// Starts one admitted actor attempt. The daemon rejects Office session
    /// kinds and retains every native identity internally; the returned
    /// value contains only the generic schedule state.
    pub(crate) fn spawn(
        &mut self,
        start: TaskAttemptExecutionStart,
    ) -> Result<TaskAttemptScheduleState, TaskAttemptRunnerError> {
        if self.key.is_some() || self.child.is_some() {
            return Err(TaskAttemptRunnerError::Scheduler);
        }
        if start.spawn_request.create_session.session_kind != society_pi::SessionKind::TaskAttempt {
            return Err(TaskAttemptRunnerError::InvalidInput);
        }
        if start.spawn_request.child_process_id.as_str().is_empty() {
            return Err(TaskAttemptRunnerError::InvalidInput);
        }
        let key = TaskAttemptScheduleKey::new(start.obligation_id, start.actor_attempt_id);
        let operation = PiExecutionOperationId::parse(self.operation.as_str().to_owned())
            .map_err(TaskAttemptRunnerError::from)?;
        let registration = self
            .daemon
            .admit_task_attempt_pi_child(TaskAttemptPiExecutionStart {
                operation,
                operating_cycle_id: start.operating_cycle_id,
                actor_attempt_id: start.actor_attempt_id,
                budget_reservation_id: start.budget_reservation_id,
                execution_profile_id: start.execution_profile_id,
                expected_generation: start.expected_generation,
                supervisor_epoch_id: start.supervisor_epoch_id,
                supervisor_epoch_identity: start.supervisor_epoch_identity,
                spawn_request: start.spawn_request,
            })
            .map_err(TaskAttemptRunnerError::from)?;
        let child = match registration {
            TaskAttemptPiSpawnRegistration::Ready(child) => child,
            TaskAttemptPiSpawnRegistration::PostSpawnSetupContained { child, .. }
            | TaskAttemptPiSpawnRegistration::RegisteredBoundaryContained { child, .. } => {
                let receipt = TaskAttemptSpawnReceipt {
                    actor_attempt_id: child.actor_attempt_id(),
                    native_child_id: child.child_process_id(),
                    native_child_spawn_admission_id: child.native_child_spawn_admission_id(),
                };
                self.scheduler
                    .record_spawned(key, receipt)
                    .map_err(TaskAttemptRunnerError::from)?;
                self.scheduler
                    .record_containment_required(key)
                    .map_err(TaskAttemptRunnerError::from)?;
                self.key = Some(key);
                self.child = Some(child);
                return Err(TaskAttemptRunnerError::ContainmentRequired);
            }
            TaskAttemptPiSpawnRegistration::RegistrationUnresolved { child, .. } => {
                self.key = Some(key);
                self.unregistered_child = Some(*child);
                return Err(TaskAttemptRunnerError::ContainmentRequired);
            }
        };
        let receipt = TaskAttemptSpawnReceipt {
            actor_attempt_id: child.actor_attempt_id(),
            native_child_id: child.child_process_id(),
            native_child_spawn_admission_id: child.native_child_spawn_admission_id(),
        };
        self.scheduler
            .record_spawned(key, receipt)
            .map_err(TaskAttemptRunnerError::from)?;
        self.key = Some(key);
        self.child = Some(child);
        Ok(TaskAttemptScheduleState::Spawned)
    }

    /// Commits the exact obligation/child runtime join. The command identity
    /// is derived by the daemon scheduler; the application supplies neither a
    /// child identity nor a generic command body.
    pub fn bind_runtime(&mut self) -> Result<StudyTransitionReceipt, TaskAttemptRunnerError> {
        let key = self.key.ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        if self
            .scheduler
            .state(key)
            .map_err(TaskAttemptRunnerError::from)?
            != TaskAttemptScheduleState::Spawned
        {
            return Err(TaskAttemptRunnerError::InvalidLifecycle);
        }
        let child = self
            .child
            .as_ref()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        let command_id = runtime_binding_command_id(key).map_err(TaskAttemptRunnerError::from)?;
        let receipt = self
            .daemon
            .bind_study_actor_task_attempt_runtime_for_child(command_id, key.obligation_id, child)
            .map_err(TaskAttemptRunnerError::from)?;
        self.scheduler
            .record_runtime_bound(key)
            .map_err(TaskAttemptRunnerError::from)?;
        Ok(receipt)
    }

    pub fn state(&self) -> Option<TaskAttemptScheduleState> {
        self.key.and_then(|key| self.scheduler.state(key).ok())
    }

    /// Advances the fixed containment suffix after a registered child crosses
    /// a protocol or setup boundary. This method is also the only operation
    /// available after a native registration could not be persisted; in that
    /// case the runner can finish physical containment but cannot invent a
    /// durable child identity or close the study runtime join.
    pub fn drive_containment(
        &mut self,
        now: MonotonicTick,
    ) -> Result<bool, TaskAttemptRunnerError> {
        if let Some(child) = self.unregistered_child.as_mut() {
            let done = self
                .daemon
                .drive_unregistered_task_attempt_pi_containment(child, now)
                .map_err(TaskAttemptRunnerError::from)?;
            if done {
                self.unregistered_child = None;
            }
            return Ok(done);
        }
        let key = self.key.ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        if !matches!(
            self.scheduler
                .state(key)
                .map_err(TaskAttemptRunnerError::from)?,
            TaskAttemptScheduleState::ContainmentRequired
        ) {
            self.scheduler
                .record_containment_required(key)
                .map_err(TaskAttemptRunnerError::from)?;
        }
        let child = self
            .child
            .as_ref()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        self.daemon
            .drive_task_attempt_pi_boundary_containment(child, now)
            .map_err(TaskAttemptRunnerError::from)?;
        Ok(false)
    }

    pub fn observe_adapter_ready(
        &mut self,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, TaskAttemptRunnerError> {
        let child = self
            .child
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        self.daemon
            .observe_task_attempt_pi_adapter_ready(child, now, deadline)
            .map_err(TaskAttemptRunnerError::from)
    }

    pub fn begin_create(
        &mut self,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, TaskAttemptRunnerError> {
        let child = self
            .child
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        self.daemon
            .authorize_and_begin_task_attempt_pi_create(child, now, deadline)
            .map_err(TaskAttemptRunnerError::from)
    }

    pub fn drive_create(
        &mut self,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, TaskAttemptRunnerError> {
        let child = self
            .child
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        self.daemon
            .drive_task_attempt_pi_create_delivery(child, now)
            .map_err(TaskAttemptRunnerError::from)
    }

    pub fn observe_session_ready(
        &mut self,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, TaskAttemptRunnerError> {
        let key = self.key.ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        if self
            .scheduler
            .state(key)
            .map_err(TaskAttemptRunnerError::from)?
            != TaskAttemptScheduleState::RuntimeBound
        {
            return Err(TaskAttemptRunnerError::InvalidLifecycle);
        }
        let child = self
            .child
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        let ready = self
            .daemon
            .observe_task_attempt_pi_session_ready(child, now, deadline)
            .map_err(TaskAttemptRunnerError::from)?;
        if ready {
            self.scheduler
                .record_ready(key)
                .map_err(TaskAttemptRunnerError::from)?;
        }
        Ok(ready)
    }

    pub fn begin_prompt(
        &mut self,
        request: TaskAttemptPromptRequest,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, TaskAttemptRunnerError> {
        let key = self.key.ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        if self
            .scheduler
            .state(key)
            .map_err(TaskAttemptRunnerError::from)?
            != TaskAttemptScheduleState::Ready
        {
            return Err(TaskAttemptRunnerError::InvalidLifecycle);
        }
        let child = self
            .child
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        let operation =
            PiTaskAttemptPromptOperationId::parse(request.operation.as_str().to_owned())
                .map_err(TaskAttemptRunnerError::from)?;
        let prompt = SealedTaskAttemptPrompt::new(request.prompt, request.prompt_digest)
            .map_err(TaskAttemptRunnerError::from)?;
        let (prompt_state, progress) = self
            .daemon
            .authorize_and_begin_task_attempt_pi_prompt(
                child,
                TaskAttemptPiPromptStart {
                    operation,
                    correlation_identity: request.correlation_identity,
                    prompt_content_object_id: request.prompt_content_object_id,
                    prompt,
                    frontier_event_id: request.frontier_event_id,
                },
                now,
                deadline,
            )
            .map_err(TaskAttemptRunnerError::from)?;
        self.prompt = Some(prompt_state);
        Ok(progress)
    }

    pub fn drive_prompt(
        &mut self,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, TaskAttemptRunnerError> {
        let child = self
            .child
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        let prompt = self
            .prompt
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        self.daemon
            .drive_task_attempt_pi_prompt_delivery(child, prompt, now)
            .map_err(TaskAttemptRunnerError::from)
    }

    /// Polls one peer frame. Forum calls are retained inside the runner until
    /// `route_forum_tool_call` commits their typed study transition.
    pub fn observe_prompt(
        &mut self,
        now: MonotonicTick,
    ) -> Result<Option<TaskAttemptPromptEvent>, TaskAttemptRunnerError> {
        if self.pending_forum_call.is_some() {
            return Err(TaskAttemptRunnerError::InvalidLifecycle);
        }
        let output = {
            let child = self
                .child
                .as_mut()
                .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
            let prompt = self
                .prompt
                .as_mut()
                .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
            self.daemon
                .observe_task_attempt_pi_prompt_output(child, prompt, now)
                .map_err(TaskAttemptRunnerError::from)?
        };
        let Some(output) = output else {
            return Ok(None);
        };
        let event = match &output {
            TaskAttemptPiPromptOutput::ControlInterleaving => {
                TaskAttemptPromptEvent::ControlInterleaving
            }
            TaskAttemptPiPromptOutput::PromptAccepted => TaskAttemptPromptEvent::PromptAccepted,
            TaskAttemptPiPromptOutput::ForumToolCall {
                correlation_identity,
                tool_call_identity,
                tool_name,
                args,
            } => {
                self.pending_forum_call = Some(output.clone());
                TaskAttemptPromptEvent::ForumToolCall {
                    correlation_identity: correlation_identity.clone(),
                    tool_call_identity: tool_call_identity.clone(),
                    tool_name: *tool_name,
                    args: args.clone(),
                }
            }
            TaskAttemptPiPromptOutput::KnownUsageRecorded => {
                TaskAttemptPromptEvent::KnownUsageRecorded
            }
            TaskAttemptPiPromptOutput::UsageFrozen => TaskAttemptPromptEvent::UsageFrozen,
            TaskAttemptPiPromptOutput::TerminalRecorded => TaskAttemptPromptEvent::TerminalRecorded,
        };
        Ok(Some(event))
    }

    /// Executes the generic obligation-scoped Forum transition and stages its
    /// result on the exact actor child. Applications cannot substitute a
    /// semantic result or another obligation here.
    pub fn route_forum_tool_call(
        &mut self,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, TaskAttemptRunnerError> {
        let key = self.key.ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        let output = self
            .pending_forum_call
            .take()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        let child = self
            .child
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        let prompt = self
            .prompt
            .as_ref()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        self.daemon
            .handle_task_attempt_pi_forum_tool_call(
                key.obligation_id,
                child,
                prompt,
                output,
                now,
                deadline,
            )
            .map_err(TaskAttemptRunnerError::from)
    }

    pub fn drive_forum_tool_result(
        &mut self,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, TaskAttemptRunnerError> {
        let child = self
            .child
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        self.daemon
            .drive_task_attempt_pi_forum_tool_result_delivery(child, now)
            .map_err(TaskAttemptRunnerError::from)
    }

    pub fn begin_dispose(
        &mut self,
        request: TaskAttemptDisposeRequest,
        now: MonotonicTick,
        deadline: ControlWriteDeadline,
    ) -> Result<ControlWriteProgress, TaskAttemptRunnerError> {
        let key = self.key.ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        if self
            .scheduler
            .state(key)
            .map_err(TaskAttemptRunnerError::from)?
            != TaskAttemptScheduleState::Ready
        {
            return Err(TaskAttemptRunnerError::InvalidLifecycle);
        }
        let child = self
            .child
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        let operation =
            PiTaskAttemptSessionDisposeOperationId::parse(request.operation.as_str().to_owned())
                .map_err(TaskAttemptRunnerError::from)?;
        let (dispose, progress) = self
            .daemon
            .begin_task_attempt_pi_session_dispose(
                child,
                TaskAttemptPiSessionDisposeStart {
                    operation,
                    correlation_identity: request.correlation_identity,
                },
                now,
                deadline,
            )
            .map_err(TaskAttemptRunnerError::from)?;
        self.dispose = Some(dispose);
        Ok(progress)
    }

    pub fn drive_dispose(
        &mut self,
        now: MonotonicTick,
    ) -> Result<ControlWriteProgress, TaskAttemptRunnerError> {
        let child = self
            .child
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        let dispose = self
            .dispose
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        self.daemon
            .drive_task_attempt_pi_session_dispose_delivery(child, dispose, now)
            .map_err(TaskAttemptRunnerError::from)
    }

    pub fn observe_dispose(
        &mut self,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<Option<TaskAttemptDisposeEvent>, TaskAttemptRunnerError> {
        let key = self.key.ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        let output = {
            let child = self
                .child
                .as_mut()
                .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
            let dispose = self
                .dispose
                .as_mut()
                .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
            self.daemon
                .observe_task_attempt_pi_session_dispose_output(child, dispose, now, deadline)
                .map_err(TaskAttemptRunnerError::from)?
        };
        let Some(output) = output else {
            return Ok(None);
        };
        let event = match output {
            TaskAttemptPiSessionDisposeOutput::DeliveryRecorded => {
                TaskAttemptDisposeEvent::DeliveryRecorded
            }
            TaskAttemptPiSessionDisposeOutput::Accepted => TaskAttemptDisposeEvent::Accepted,
            TaskAttemptPiSessionDisposeOutput::KnownUsageRecorded => {
                TaskAttemptDisposeEvent::KnownUsageRecorded
            }
            TaskAttemptPiSessionDisposeOutput::UsageFrozen => {
                // Keep the public observation separate from the schedule
                // transition.  A generic bridge may expose an accounting
                // freeze before it emits its verified transcript terminal;
                // advancing to `ContainmentRequired` here would make that
                // terminal fail the scheduler's `Ready` -> `Disposed` edge.
                // The current Pi v1 bridge treats this output as terminal and
                // enters native containment internally, so its caller must
                // invoke `drive_containment` next; that operation advances
                // the durable scheduler suffix exactly once.
                TaskAttemptDisposeEvent::UsageFrozen
            }
            TaskAttemptPiSessionDisposeOutput::TranscriptReady(terminal) => {
                let child = self
                    .child
                    .as_mut()
                    .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
                let dispose = self
                    .dispose
                    .as_mut()
                    .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
                self.daemon
                    .record_task_attempt_pi_session_disposed(child, dispose, &terminal, now)
                    .map_err(TaskAttemptRunnerError::from)?;
                self.scheduler
                    .record_disposed(key)
                    .map_err(TaskAttemptRunnerError::from)?;
                TaskAttemptDisposeEvent::Disposed
            }
            TaskAttemptPiSessionDisposeOutput::Disposed => TaskAttemptDisposeEvent::Disposed,
        };
        Ok(Some(event))
    }

    /// Drives the fixed direct-child/process-group suffix. Once it returns
    /// `true`, the runner has committed the generic runtime-reconciliation
    /// transition and no native custody remains in the application.
    pub fn reconcile(&mut self, now: MonotonicTick) -> Result<bool, TaskAttemptRunnerError> {
        let key = self.key.ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        if self.unregistered_child.is_some() {
            return Err(TaskAttemptRunnerError::ContainmentRequired);
        }
        if !matches!(
            self.scheduler
                .state(key)
                .map_err(TaskAttemptRunnerError::from)?,
            TaskAttemptScheduleState::Disposed | TaskAttemptScheduleState::ContainmentRequired
        ) {
            return Err(TaskAttemptRunnerError::InvalidLifecycle);
        }
        let child = self
            .child
            .as_mut()
            .ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        let done = self
            .daemon
            .reconcile_reaped_task_attempt_pi_child(child, now)
            .map_err(TaskAttemptRunnerError::from)?;
        if !done {
            return Ok(false);
        }
        self.scheduler
            .record_physical_reconciled(key)
            .map_err(TaskAttemptRunnerError::from)?;
        let command_id =
            runtime_reconciliation_command_id(key).map_err(TaskAttemptRunnerError::from)?;
        self.daemon
            .reconcile_study_actor_task_attempt_runtime_for_child(
                command_id,
                key.obligation_id,
                child,
            )
            .map_err(TaskAttemptRunnerError::from)?;
        self.scheduler
            .record_runtime_reconciled(key)
            .map_err(TaskAttemptRunnerError::from)?;
        Ok(true)
    }

    /// Records the exact terminal study obligation after the owned native
    /// child and its runtime binding have both reconciled.  The caller may
    /// choose only the already pre-registered study-budget charge; it cannot
    /// name an obligation, actor attempt, child, or command identity.  A
    /// terminal Pi transcript is evidence of task termination, not evidence
    /// that a role produced a valid application decision record.
    pub fn complete_actor_obligation(
        &mut self,
        charged_budget: society_kernel::StudyBudgetUnits,
    ) -> Result<StudyTransitionReceipt, TaskAttemptRunnerError> {
        let key = self.key.ok_or(TaskAttemptRunnerError::InvalidLifecycle)?;
        if self.actor_completion_recorded
            || self
                .scheduler
                .state(key)
                .map_err(TaskAttemptRunnerError::from)?
                != TaskAttemptScheduleState::Reconciled
        {
            return Err(TaskAttemptRunnerError::InvalidLifecycle);
        }
        let command_id = actor_completion_command_id(key).map_err(TaskAttemptRunnerError::from)?;
        let receipt = self
            .daemon
            .bind_study_actor_runtime(
                command_id,
                StudyCommand::CompleteActorObligation {
                    obligation_id: key.obligation_id,
                    charged_budget,
                },
            )
            .map_err(TaskAttemptRunnerError::from)?;
        self.actor_completion_recorded = true;
        Ok(receipt)
    }
}

/// Stable command identity for the terminal study-obligation receipt.  It is
/// distinct from native reconciliation: a durable process suffix must exist
/// before the application can close the corresponding actor lifetime.
fn actor_completion_command_id(
    key: TaskAttemptScheduleKey,
) -> Result<CommandId, TaskAttemptSchedulerError> {
    CommandId::parse(format!(
        "{COMMAND_PREFIX}/obligation-{}/attempt-{}/complete",
        key.obligation_id.value(),
        key.actor_attempt_id.value()
    ))
    .map_err(|_| TaskAttemptSchedulerError::InvalidCommandIdentity)
}

fn valid_operation_label(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 36
        && bytes[0].is_ascii_alphanumeric()
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeDriver {
        receipt: TaskAttemptSpawnReceipt,
        progress: Vec<TaskAttemptPiProgress>,
        seen_key: Option<TaskAttemptScheduleKey>,
    }

    impl TaskAttemptPiDriver for FakeDriver {
        fn spawn_task_attempt(
            &mut self,
            request: TaskAttemptSpawnRequest,
        ) -> Result<TaskAttemptSpawnReceipt, TaskAttemptDriverFailure> {
            self.seen_key = Some(request.key);
            Ok(self.receipt)
        }

        fn drive_task_attempt(
            &mut self,
            _child: TaskAttemptSpawnReceipt,
            _now: MonotonicTick,
        ) -> Result<TaskAttemptPiProgress, TaskAttemptDriverFailure> {
            Ok(self.progress.remove(0))
        }
    }

    fn key() -> TaskAttemptScheduleKey {
        TaskAttemptScheduleKey::new(
            StudyActorObligationId::new(11).expect("test fixture"),
            ActorAttemptId::new(23).expect("test fixture"),
        )
    }

    fn receipt(actor_attempt_id: ActorAttemptId) -> TaskAttemptSpawnReceipt {
        TaskAttemptSpawnReceipt {
            actor_attempt_id,
            native_child_id: NativeChildId::new(31).expect("test fixture"),
            native_child_spawn_admission_id: NativeChildSpawnAdmissionId::new(37)
                .expect("test fixture"),
        }
    }

    #[test]
    fn task_attempt_lifecycle_requires_kernel_boundaries() {
        let key = key();
        let mut driver = FakeDriver {
            receipt: receipt(key.actor_attempt_id),
            progress: vec![
                TaskAttemptPiProgress::Waiting,
                TaskAttemptPiProgress::Ready,
                TaskAttemptPiProgress::Disposed,
                TaskAttemptPiProgress::Reconciled,
            ],
            seen_key: None,
        };
        let mut scheduler = TaskAttemptScheduler::default();
        let spawned = scheduler.spawn(&mut driver, key).expect("test fixture");
        assert_eq!(driver.seen_key, Some(key));
        assert_eq!(
            scheduler.state(key).expect("test fixture"),
            TaskAttemptScheduleState::Spawned
        );
        assert_eq!(scheduler.child(key).expect("test fixture"), spawned);
        assert_eq!(
            scheduler
                .runtime_binding_command(key)
                .expect("test fixture"),
            StudyCommand::BindActorTaskAttemptRuntime {
                obligation_id: key.obligation_id,
                actor_attempt_id: key.actor_attempt_id,
                native_child_id: spawned.native_child_id,
                native_child_spawn_admission_id: spawned.native_child_spawn_admission_id,
            }
        );
        scheduler.record_runtime_bound(key).expect("test fixture");
        assert_eq!(
            scheduler
                .drive(&mut driver, key, MonotonicTick::ZERO)
                .expect("test fixture"),
            TaskAttemptScheduleState::RuntimeBound
        );
        assert_eq!(
            scheduler
                .drive(&mut driver, key, MonotonicTick::ZERO)
                .expect("test fixture"),
            TaskAttemptScheduleState::Ready
        );
        assert_eq!(
            scheduler
                .drive(&mut driver, key, MonotonicTick::ZERO)
                .expect("test fixture"),
            TaskAttemptScheduleState::Disposed
        );
        assert_eq!(
            scheduler
                .drive(&mut driver, key, MonotonicTick::ZERO)
                .expect("test fixture"),
            TaskAttemptScheduleState::PhysicalReconciled
        );
        assert_eq!(
            scheduler
                .runtime_reconciliation_command(key)
                .expect("test fixture"),
            StudyCommand::ReconcileActorRuntime {
                obligation_id: key.obligation_id,
                native_child_id: spawned.native_child_id,
            }
        );
        scheduler
            .record_runtime_reconciled(key)
            .expect("test fixture");
        assert_eq!(
            scheduler.state(key).expect("test fixture"),
            TaskAttemptScheduleState::Reconciled
        );
    }

    #[test]
    fn task_attempt_scheduler_rejects_off_by_one_identity_and_duplicate() {
        let key = key();
        let mut driver = FakeDriver {
            receipt: receipt(ActorAttemptId::new(29).expect("test fixture")),
            progress: Vec::new(),
            seen_key: None,
        };
        let mut scheduler = TaskAttemptScheduler::default();
        assert_eq!(
            scheduler.spawn(&mut driver, key),
            Err(TaskAttemptSchedulerError::DriverIdentityMismatch)
        );
        assert_eq!(
            scheduler.state(key).expect("test fixture"),
            TaskAttemptScheduleState::ContainmentRequired
        );
        assert_eq!(
            scheduler.spawn(&mut driver, key),
            Err(TaskAttemptSchedulerError::DuplicateSchedule)
        );
    }

    #[test]
    fn concrete_resident_receipt_handoff_preserves_closed_lifecycle() {
        let key = key();
        let mut scheduler = TaskAttemptScheduler::default();
        scheduler
            .record_spawned(key, receipt(key.actor_attempt_id))
            .expect("resident receipt must admit once");
        scheduler
            .record_runtime_bound(key)
            .expect("kernel runtime binding must precede readiness");
        scheduler
            .record_ready(key)
            .expect("SessionReady must precede task disposal");
        scheduler
            .record_disposed(key)
            .expect("Dispose must precede native reconciliation");
        scheduler
            .record_physical_reconciled(key)
            .expect("native reconciliation must precede the kernel suffix");
        scheduler
            .record_runtime_reconciled(key)
            .expect("kernel runtime reconciliation must close the join");
        assert_eq!(
            scheduler.state(key).expect("test fixture"),
            TaskAttemptScheduleState::Reconciled
        );
    }

    #[test]
    fn dispose_accounting_freeze_does_not_preempt_verified_transcript_terminal() {
        let key = key();
        let mut scheduler = TaskAttemptScheduler::default();
        scheduler
            .record_spawned(key, receipt(key.actor_attempt_id))
            .expect("resident receipt must admit once");
        scheduler
            .record_runtime_bound(key)
            .expect("kernel runtime binding must precede readiness");
        scheduler
            .record_ready(key)
            .expect("SessionReady must precede disposal");

        // `observe_dispose` reports UsageFrozen without changing this
        // schedule entry.  If a bridge subsequently supplies a verified
        // transcript terminal, the normal Ready -> Disposed transition stays
        // legal.  A bridge which makes the freeze terminal instead calls the
        // explicit containment suffix below.
        assert_eq!(
            scheduler.state(key).expect("test fixture"),
            TaskAttemptScheduleState::Ready
        );
        scheduler
            .record_disposed(key)
            .expect("a verified transcript terminal must close normally");
        assert_eq!(
            scheduler.state(key).expect("test fixture"),
            TaskAttemptScheduleState::Disposed
        );

        let containment_key = TaskAttemptScheduleKey::new(
            StudyActorObligationId::new(12).expect("test fixture"),
            ActorAttemptId::new(24).expect("test fixture"),
        );
        scheduler
            .record_spawned(containment_key, receipt(containment_key.actor_attempt_id))
            .expect("resident receipt must admit once");
        scheduler
            .record_containment_required(containment_key)
            .expect("terminal accounting freeze must use the explicit suffix");
        assert_eq!(
            scheduler.state(containment_key).expect("test fixture"),
            TaskAttemptScheduleState::ContainmentRequired
        );
    }

    #[test]
    fn concrete_resident_receipt_handoff_rejects_foreign_actor_attempt() {
        let key = key();
        let mut scheduler = TaskAttemptScheduler::default();
        assert_eq!(
            scheduler.record_spawned(
                key,
                receipt(ActorAttemptId::new(999).expect("test fixture")),
            ),
            Err(TaskAttemptSchedulerError::DriverIdentityMismatch)
        );
        assert_eq!(
            scheduler.state(key),
            Err(TaskAttemptSchedulerError::UnknownSchedule)
        );
    }

    #[test]
    fn task_attempt_command_ids_are_distinct_and_typed() {
        let key = key();
        let binding = runtime_binding_command_id(key).expect("test fixture");
        let reconciliation = runtime_reconciliation_command_id(key).expect("test fixture");
        let completion = actor_completion_command_id(key).expect("test fixture");
        assert_ne!(binding, reconciliation);
        assert_ne!(reconciliation, completion);
        assert!(binding.as_str().contains("obligation-11/attempt-23"));
        assert!(reconciliation.as_str().ends_with("/reconcile"));
        assert!(completion.as_str().ends_with("/complete"));
    }

    #[test]
    fn task_attempt_driver_progress_cannot_skip_ready_or_reconciliation() {
        let key = key();
        let mut driver = FakeDriver {
            receipt: receipt(key.actor_attempt_id),
            progress: vec![TaskAttemptPiProgress::Disposed],
            seen_key: None,
        };
        let mut scheduler = TaskAttemptScheduler::default();
        scheduler.spawn(&mut driver, key).expect("test fixture");
        scheduler.record_runtime_bound(key).expect("test fixture");
        assert_eq!(
            scheduler.drive(&mut driver, key, MonotonicTick::ZERO),
            Err(TaskAttemptSchedulerError::UnexpectedDriverProgress)
        );
        assert_eq!(
            scheduler.state(key).expect("test fixture"),
            TaskAttemptScheduleState::RuntimeBound
        );
    }

    #[test]
    fn task_attempt_containment_is_a_physical_suffix_before_reconciliation() {
        let key = key();
        let mut driver = FakeDriver {
            receipt: receipt(key.actor_attempt_id),
            progress: vec![
                TaskAttemptPiProgress::ContainmentRequired,
                TaskAttemptPiProgress::Waiting,
                TaskAttemptPiProgress::Reconciled,
            ],
            seen_key: None,
        };
        let mut scheduler = TaskAttemptScheduler::default();
        scheduler.spawn(&mut driver, key).expect("test fixture");
        scheduler.record_runtime_bound(key).expect("test fixture");
        assert_eq!(
            scheduler.drive(&mut driver, key, MonotonicTick::ZERO),
            Ok(TaskAttemptScheduleState::ContainmentRequired)
        );
        assert_eq!(
            scheduler.drive(&mut driver, key, MonotonicTick::ZERO),
            Ok(TaskAttemptScheduleState::ContainmentRequired)
        );
        assert_eq!(
            scheduler.drive(&mut driver, key, MonotonicTick::ZERO),
            Ok(TaskAttemptScheduleState::PhysicalReconciled)
        );
        assert!(scheduler.runtime_binding_command(key).is_err());
        assert!(scheduler.runtime_reconciliation_command(key).is_ok());
    }
}
