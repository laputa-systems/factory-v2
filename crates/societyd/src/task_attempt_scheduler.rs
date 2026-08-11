//! Daemon-private coordinator contract for live study TaskAttempt children.
//!
//! The existing [`crate::pi_execution`] bridge is intentionally an Office
//! bridge and rejects `SessionKind::TaskAttempt`.  This module therefore does
//! not wrap or relabel that driver.  It defines the narrow state handoff a
//! future TaskAttempt Pi driver must satisfy: the driver owns process physics,
//! while this coordinator owns the study obligation/attempt join and the
//! ordering around the two already-typed study runtime transitions.
//!
//! No application descriptor, prompt bytes, JSON value, Office session, or
//! provider identity crosses this module.  The application remains outside
//! the daemon; the eventual scheduler will obtain its sealed plan through the
//! existing content-custody boundary before constructing a typed driver
//! request.

use std::collections::BTreeMap;

use society_kernel::{
    ActorAttemptId, CommandId, NativeChildId, NativeChildSpawnAdmissionId, StudyActorObligationId,
    StudyCommand,
};
use thiserror::Error;

use crate::supervision::MonotonicTick;

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
pub(crate) enum TaskAttemptScheduleState {
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
            StudyActorObligationId::new(11).unwrap(),
            ActorAttemptId::new(23).unwrap(),
        )
    }

    fn receipt(actor_attempt_id: ActorAttemptId) -> TaskAttemptSpawnReceipt {
        TaskAttemptSpawnReceipt {
            actor_attempt_id,
            native_child_id: NativeChildId::new(31).unwrap(),
            native_child_spawn_admission_id: NativeChildSpawnAdmissionId::new(37).unwrap(),
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
        let spawned = scheduler.spawn(&mut driver, key).unwrap();
        assert_eq!(driver.seen_key, Some(key));
        assert_eq!(
            scheduler.state(key).unwrap(),
            TaskAttemptScheduleState::Spawned
        );
        assert_eq!(scheduler.child(key).unwrap(), spawned);
        assert_eq!(
            scheduler.runtime_binding_command(key).unwrap(),
            StudyCommand::BindActorTaskAttemptRuntime {
                obligation_id: key.obligation_id,
                actor_attempt_id: key.actor_attempt_id,
                native_child_id: spawned.native_child_id,
                native_child_spawn_admission_id: spawned.native_child_spawn_admission_id,
            }
        );
        scheduler.record_runtime_bound(key).unwrap();
        assert_eq!(
            scheduler
                .drive(&mut driver, key, MonotonicTick::ZERO)
                .unwrap(),
            TaskAttemptScheduleState::RuntimeBound
        );
        assert_eq!(
            scheduler
                .drive(&mut driver, key, MonotonicTick::ZERO)
                .unwrap(),
            TaskAttemptScheduleState::Ready
        );
        assert_eq!(
            scheduler
                .drive(&mut driver, key, MonotonicTick::ZERO)
                .unwrap(),
            TaskAttemptScheduleState::Disposed
        );
        assert_eq!(
            scheduler
                .drive(&mut driver, key, MonotonicTick::ZERO)
                .unwrap(),
            TaskAttemptScheduleState::PhysicalReconciled
        );
        assert_eq!(
            scheduler.runtime_reconciliation_command(key).unwrap(),
            StudyCommand::ReconcileActorRuntime {
                obligation_id: key.obligation_id,
                native_child_id: spawned.native_child_id,
            }
        );
        scheduler.record_runtime_reconciled(key).unwrap();
        assert_eq!(
            scheduler.state(key).unwrap(),
            TaskAttemptScheduleState::Reconciled
        );
    }

    #[test]
    fn task_attempt_scheduler_rejects_off_by_one_identity_and_duplicate() {
        let key = key();
        let mut driver = FakeDriver {
            receipt: receipt(ActorAttemptId::new(29).unwrap()),
            progress: Vec::new(),
            seen_key: None,
        };
        let mut scheduler = TaskAttemptScheduler::default();
        assert_eq!(
            scheduler.spawn(&mut driver, key),
            Err(TaskAttemptSchedulerError::DriverIdentityMismatch)
        );
        assert_eq!(
            scheduler.state(key).unwrap(),
            TaskAttemptScheduleState::ContainmentRequired
        );
        assert_eq!(
            scheduler.spawn(&mut driver, key),
            Err(TaskAttemptSchedulerError::DuplicateSchedule)
        );
    }

    #[test]
    fn task_attempt_command_ids_are_distinct_and_typed() {
        let key = key();
        let binding = runtime_binding_command_id(key).unwrap();
        let reconciliation = runtime_reconciliation_command_id(key).unwrap();
        assert_ne!(binding, reconciliation);
        assert!(binding.as_str().contains("obligation-11/attempt-23"));
        assert!(reconciliation.as_str().ends_with("/reconcile"));
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
        scheduler.spawn(&mut driver, key).unwrap();
        scheduler.record_runtime_bound(key).unwrap();
        assert_eq!(
            scheduler.drive(&mut driver, key, MonotonicTick::ZERO),
            Err(TaskAttemptSchedulerError::UnexpectedDriverProgress)
        );
        assert_eq!(
            scheduler.state(key).unwrap(),
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
        scheduler.spawn(&mut driver, key).unwrap();
        scheduler.record_runtime_bound(key).unwrap();
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
