//! Daemon-private bridge from durable M5 child receipts to native Pi physics.
//!
//! `PiSupervisor` owns only a live process group and transient pipe receipts.
//! This module owns neither a local wire command nor a new authority: it
//! translates one already-admitted Office child through the kernel's closed
//! M5 receipt chain, one committed transition at a time.  In particular it
//! never keeps a SQLite transaction open while allocating a workspace,
//! spawning, reading or writing a pipe, signalling, waiting, or sealing
//! bytes.

use society_kernel::{
    AdmissionGeneration, BudgetReservationId, CanonicalWorkspacePath, Capability, ChildProcessId,
    ChildStreamKind, ChildStreamSealCompleteness, CommandBody, CommandDisposition, CommandId,
    CommandRequest, DirectChildWaitStatus, EventBody, ExecutionProfileId, ExpectedGeneration,
    GrandArchitectOfficeSessionId, KernelStore, NativeChildPid,
    NativeWorkspaceId as KernelWorkspaceId, OwnedProcessGroupId as KernelProcessGroupId,
    PiBoundarySessionIdentity, PiChildOwner, PiChildSpawnAdmissionId, PiCorrelationIdentity,
    PrincipalId, ProcessExitCode, ProcessGroupLiveness as KernelLiveness, ProcessSignalNumber,
    Sha256Digest as KernelDigest, SpawnNonce as KernelSpawnNonce, SupervisedChildIdentity,
    SupervisorEpochId, SupervisorEpochIdentity,
};
use society_pi::{
    BoundarySequence, CorrelationIdentity, InboundCommand, InboundFrame, SessionIdentity,
    SessionKind,
};
use thiserror::Error;

use crate::{
    content::{ContentSealOperationId, ContentSealingAuthority, ContentSealingError},
    supervision::{
        ControlWriteDeadline, ControlWriteProgress, HandshakeDeadline, InertChildFacts,
        MonotonicTick, PiSpawnRequest, PiSupervisor, PostSpawnSetupFailure, PreCreateAdmissionGate,
        ReapStatus, SignalAction, SignalDelivery, SupervisedChildId, SupervisionError,
        SupervisionReceipt, TransientByteCount, TransientRetention, TransientStreamCapture,
    },
};

const COMMAND_PREFIX: &str = "pi-execution-v1/";
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
        child_process_id: ChildProcessId,
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

/// Inputs already selected by trusted scheduling.  The execution driver does
/// not discover a model, owner, capability, workspace, or command identity.
/// It merely turns this exact Office admission into a child receipt chain.
#[derive(Clone, Debug)]
pub(crate) struct OfficePiExecutionStart {
    pub(crate) operation: PiExecutionOperationId,
    pub(crate) operating_cycle_id: society_kernel::OperatingCycleId,
    pub(crate) office_session_id: GrandArchitectOfficeSessionId,
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

/// The daemon-private handle for exactly one registered Office child.  It has
/// no constructor outside the pre-spawn-to-registration transition.
#[derive(Clone, Debug)]
pub(crate) struct OfficePiExecutionChild {
    operation: PiExecutionOperationId,
    supervised_child_id: SupervisedChildId,
    child_process_id: ChildProcessId,
    office_session_id: GrandArchitectOfficeSessionId,
    pi_session_identity: PiBoundarySessionIdentity,
    spawn_nonce: KernelSpawnNonce,
    expected_generation: AdmissionGeneration,
    create_correlation: PiCorrelationIdentity,
    create_request_digest: KernelDigest,
    phase: OfficePiExecutionPhase,
}

/// A native child exists, but the kernel rejected (or could not persist) its
/// first PID/PGID receipt. There is intentionally no `ChildProcessId`: the
/// admission stays durably unresolved and cannot be rewritten as
/// `NotSpawned`. The current resident must only finish physical containment;
/// a restart remains RecoveryFenced because no later process can attach to
/// this unregistered native identity.
#[derive(Debug)]
pub(crate) struct UnregisteredOfficePiChild {
    supervised_child_id: SupervisedChildId,
    pi_child_spawn_admission_id: PiChildSpawnAdmissionId,
    phase: UnregisteredOfficePiChildPhase,
    transient_completion: Option<SupervisionReceipt>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UnregisteredOfficePiChildPhase {
    ContainmentRequired,
    PhysicallyReaped,
}

impl UnregisteredOfficePiChild {
    pub(crate) fn pi_child_spawn_admission_id(&self) -> PiChildSpawnAdmissionId {
        self.pi_child_spawn_admission_id
    }

    pub(crate) fn transient_completion(&self) -> Option<&SupervisionReceipt> {
        self.transient_completion.as_ref()
    }
}

/// The first bridge transition has two non-ambiguous outcomes.  A physical
/// child is never collapsed into `RecordPiChildNotSpawned`: callers receive
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
        child: Box<UnregisteredOfficePiChild>,
        failure: PiExecutionError,
    },
}

impl OfficePiExecutionChild {
    pub(crate) fn child_process_id(&self) -> ChildProcessId {
        self.child_process_id
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
        // A Grand Architect Office owner and an Office session must be the
        // same closed boundary. Reject this before any kernel admission,
        // native workspace action, or host process exists; a TaskAttempt
        // payload cannot borrow the Office budget/session authority.
        if start.spawn_request.create_session.session_kind != SessionKind::GrandArchitectOffice {
            return Err(PiExecutionError::OfficeSessionKindRequired);
        }
        // This is deliberately before `AdmitPiChildSpawn`: malformed native
        // paths/profile/artifacts must not create an admission that no exact
        // process outcome can close. `spawn_native` repeats the same checks
        // immediately before exec and maps only that proven-absent TOCTOU
        // failure into `RecordPiChildNotSpawned`.
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
                owner: PiChildOwner::GrandArchitectOfficeSession(start.office_session_id),
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
                pi_child_spawn_admission_id,
                owner: PiChildOwner::GrandArchitectOfficeSession(session_id),
                budget_reservation_id,
            } if session_id == start.office_session_id
                && budget_reservation_id == start.budget_reservation_id =>
            {
                pi_child_spawn_admission_id
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
                        Capability::RecordPiChildNotSpawned,
                        ExpectedGeneration::Exact(current_generation),
                        CommandBody::RecordPiChildNotSpawned {
                            pi_child_spawn_admission_id: admission_id,
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
                    pi_child_spawn_admission_id: admission_id,
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
                child_process_id,
                pi_child_spawn_admission_id,
            } if pi_child_spawn_admission_id == admission_id => child_process_id,
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
            office_session_id: start.office_session_id,
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
    /// never assigned a kernel `ChildProcessId`. Its completed receipt is
    /// intentionally retained only as transient local evidence: no signal,
    /// wait, stream-seal, or finalization command can honestly name it.
    pub(crate) fn drive_unregistered_spawn_containment(
        &mut self,
        child: &mut UnregisteredOfficePiChild,
        now: MonotonicTick,
    ) -> Result<bool, PiExecutionError> {
        if child.phase != UnregisteredOfficePiChildPhase::ContainmentRequired {
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
        child.phase = UnregisteredOfficePiChildPhase::PhysicallyReaped;
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
                child_process_id: child.child_process_id,
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
        let mut gate = KernelCreateAuthorizationGate::new(store, child);
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
    /// observation boundary, not an impossible claim of atomic OS/SQLite
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
                child_process_id: child.child_process_id,
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
            Ok(EventBody::GrandArchitectOfficeSessionStateChanged {
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

    /// This bounded bridge performs no Prompt.  It disposes the provider-free
    /// session and then leaves reaping/sealing to the caller's nonblocking
    /// control-loop ticks.
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

    /// Drains the already-admitted Dispose frame without permitting another
    /// command to overtake it. `Disposed` observation is illegal until this
    /// returns `Delivered` and changes the closed phase.
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

    pub(crate) fn observe_disposed(
        &mut self,
        child: &mut OfficePiExecutionChild,
        now: MonotonicTick,
        deadline: HandshakeDeadline,
    ) -> Result<bool, PiExecutionError> {
        if child.phase != OfficePiExecutionPhase::DisposeRequested {
            return Err(PiExecutionError::InvalidLifecycle);
        }
        let disposed =
            match self
                .supervisor
                .observe_disposed_at(&child.supervised_child_id, now, deadline)
            {
                Ok(disposed) => disposed,
                Err(error) => {
                    self.begin_registered_boundary_containment(child, now);
                    return Err(PiExecutionError::Supervision(error));
                }
            };
        if disposed {
            child.phase = OfficePiExecutionPhase::Disposed;
        }
        Ok(disposed)
    }

    /// Reconciles a direct wait through the M5 ordering: durable direct-child
    /// reap, then (only if due) a distinct lingering-group cleanup signal,
    /// then a later liveness observation and bounded stream sealing. The
    /// process physics never needs a SQLite transaction, and each durable
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
                child_process_id: child.child_process_id,
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
                child_process_id: child.child_process_id,
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
        execute_kernel_command(
            store,
            &child.operation,
            PiExecutionCommand::RecordLiveness,
            Capability::RecordChildProcessLiveness,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordChildProcessLiveness {
                child_process_id: child.child_process_id,
                liveness: kernel_liveness(liveness),
            },
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
                child_process_id: child.child_process_id,
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
        // This bounded bridge does not yet execute a typed kernel cancellation
        // propagation, so an SDK Abort control receipt has no valid durable
        // command relation here. It must never be silently omitted.
        if signal.action == SignalAction::AbortControl {
            return Err(PiExecutionError::UnmodeledAbortControlReceipt);
        }
        let action = match signal.action {
            SignalAction::Terminate => society_kernel::ProcessSignalAction::Terminate,
            SignalAction::Kill => society_kernel::ProcessSignalAction::Kill,
            SignalAction::LingeringGroupKill => {
                society_kernel::ProcessSignalAction::LingeringGroupKill
            }
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
            SignalDelivery::GroupInaccessible => {
                society_kernel::ProcessSignalDelivery::Inaccessible
            }
            SignalDelivery::AbortControlWritten => {
                return Err(PiExecutionError::UnmodeledAbortControlReceipt);
            }
        };
        let command = PiExecutionCommand::RecordSignal { ordinal };
        execute_kernel_command(
            store,
            &child.operation,
            command,
            Capability::RecordProcessSignalReceipt,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordProcessSignalReceipt {
                child_process_id: child.child_process_id,
                action,
                delivery,
                observed_liveness: kernel_liveness(signal.group_liveness_after_attempt),
                cause: society_kernel::ProcessSignalCause::AutomaticBoundaryContainment,
            },
        )?;
        Ok(())
    }

    fn seal_stream(
        &self,
        store: &mut KernelStore,
        content: &ContentSealingAuthority,
        child: &OfficePiExecutionChild,
        stream_kind: ChildStreamKind,
        capture: &TransientStreamCapture,
    ) -> Result<(), PiExecutionError> {
        let retained_bytes = capture.retained_bytes();
        let operation = ContentSealOperationId::parse(
            child
                .operation
                .content_label(child.child_process_id, stream_kind)?,
            KernelDigest::of_bytes(retained_bytes),
        )
        .map_err(|_| PiExecutionError::InvalidOperationIdentity)?;
        let registration = content.seal_and_register(store, &operation, retained_bytes)?;
        execute_kernel_command(
            store,
            &child.operation,
            stream_seal_command(stream_kind),
            Capability::RecordChildStreamSeal,
            ExpectedGeneration::Exact(child.expected_generation),
            CommandBody::RecordChildStreamSeal {
                child_process_id: child.child_process_id,
                stream_kind,
                full_observed_digest: kernel_digest_from_boundary(capture)?,
                retained_content_object_id: registration.content_object_id,
                completeness: kernel_stream_completeness(capture),
            },
        )?;
        Ok(())
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
        pi_child_spawn_admission_id: PiChildSpawnAdmissionId,
        failure: PiExecutionError,
    ) -> OfficePiSpawnRegistration {
        // The durable admission exists but `RecordInertChildSpawn` does not,
        // so no kernel child identity may be fabricated. Contain the exact
        // native group now; its physical completion remains transient and the
        // admission deliberately stays unresolved for recovery fencing.
        self.contain(&supervised_child_id, MonotonicTick::ZERO);
        OfficePiSpawnRegistration::RegistrationUnresolved {
            child: Box::new(UnregisteredOfficePiChild {
                supervised_child_id,
                pi_child_spawn_admission_id,
                phase: UnregisteredOfficePiChildPhase::ContainmentRequired,
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

struct KernelCreateAuthorizationGate<'a> {
    store: &'a mut KernelStore,
    operation: &'a PiExecutionOperationId,
    child_process_id: ChildProcessId,
    expected_generation: AdmissionGeneration,
    correlation: &'a PiCorrelationIdentity,
    create_request_digest: KernelDigest,
    outcome: Option<Result<(), PiExecutionError>>,
}

impl<'a> KernelCreateAuthorizationGate<'a> {
    fn new(store: &'a mut KernelStore, child: &'a OfficePiExecutionChild) -> Self {
        Self {
            store,
            operation: &child.operation,
            child_process_id: child.child_process_id,
            expected_generation: child.expected_generation,
            correlation: &child.create_correlation,
            create_request_digest: child.create_request_digest,
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
                child_process_id: self.child_process_id,
                correlation_identity: self.correlation.clone(),
                create_request_digest: self.create_request_digest,
            },
        )
        .and_then(|event| match event {
            EventBody::PiCreateSessionAuthorized { child_process_id }
                if child_process_id == self.child_process_id =>
            {
                Ok(())
            }
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

/// `RecordPiChildNotSpawned` is a durable assertion of physical absence, not
/// a catch-all spawn/setup error. Keep this mapping deliberately small: other
/// failures leave the already-admitted operation fenced for a later exact
/// recovery path rather than fabricating a negative child receipt.
fn proven_not_spawned_reason(
    error: &SupervisionError,
) -> Option<society_kernel::PiChildNotSpawnedReason> {
    match error {
        SupervisionError::NativeSpawn(_) => {
            Some(society_kernel::PiChildNotSpawnedReason::NativeSpawnFailed)
        }
        SupervisionError::ArtifactIsNotRegularFile | SupervisionError::ArtifactDigestDrift => {
            Some(society_kernel::PiChildNotSpawnedReason::ArtifactQualificationFailed)
        }
        SupervisionError::InvalidSpawnRequest => {
            Some(society_kernel::PiChildNotSpawnedReason::WorkspacePreparationFailed)
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
    let mut bytes = [0_u8; 32];
    let text = capture.sha256.as_str().as_bytes();
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
    #[error("an Office Pi child requires the exact GrandArchitectOffice session kind")]
    OfficeSessionKindRequired,
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
    #[error("Pi boundary digest was not canonical lowercase SHA-256")]
    BoundaryDigestInvalid,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
        thread,
        time::Duration,
    };

    use sha2::{Digest, Sha256};
    use society_content::{ContentSealLimit, ContentStoreRoot};
    use society_kernel::{
        AdmissionGeneration, BudgetReservationId, CancellationMode, CancellationPropagationId,
        CancellationRequestId, Capability, CommandBody, CommandDisposition, CommandId,
        CommandRequest, ExpectedGeneration, GrandArchitectOfficeSessionId, KernelStore,
        OfficeTurnPurpose, OperatingCycleId, OperatingCycleTreatment, PrincipalDisplayName,
        PrincipalId, Rejection, Sha256Digest as KernelDigest, SocietyName, SupervisorEpochId,
        SupervisorEpochIdentity, UsdMicros,
    };
    use society_pi::{
        AbsolutePath, ActorModelPolicyV1, AdapterVersion, CacheWritePerMillionRateV1,
        CanonicalModelSlug, CompactionMode, CompactionPolicyV1, CorrelationIdentity,
        CreateSessionPayload, Disabled, EffectiveModelDescriptorV1, Images, KnownPerMillionRateV1,
        ModelApi, ModelCatalogPolicyV1, ModelId, ModelInput, ModelSelection, NodeRuntimeVersion,
        NonNegativeInteger, OpenRouterBaseUrl, PiSdkVersion, PositiveInteger, ProjectTrust,
        Provider, QueueMode, RetryPolicyV1, RuntimeIdentity, SessionIdentity, SessionKind,
        Sha256Digest, SpawnNonce, ThinkingLevel, ToolProfile, Transport, UsdPerMillionDecimal,
    };

    use super::{
        OfficePiExecutionStart, OfficePiSpawnRegistration, PiExecutionDriver,
        PiExecutionOperationId,
    };
    use crate::{
        content::ContentSealingAuthority,
        supervision::{
            ControlWriteDeadline, HandshakeDeadline, MonotonicTick, NativeHostEnvironment,
            NativeWorkspace, NativeWorkspaceId, NativeWorkspaceRoot, PiSpawnRequest,
            QualifiedHostExecution, SupervisedChildId, VerifiedArtifact,
        },
    };

    static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn provider_free_office_create_dispose_reap_records_ready_only_after_live_session() {
        let fixture = NativeFixture::new("m5-office-bridge");
        let mut store = KernelStore::open_in_memory().unwrap();
        let office = found_office_start(&mut store, "m5-office-bridge");
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

        assert!(
            !driver
                .observe_adapter_ready(
                    &mut store,
                    &mut child,
                    MonotonicTick::ZERO,
                    HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
                )
                .unwrap()
        );
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

    #[cfg(feature = "test-support")]
    #[test]
    fn session_ready_then_direct_child_exit_refuses_office_ready_and_reconciles_wait() {
        let fixture = NativeFixture::new("m5-exit-after-session-ready");
        let mut store = KernelStore::open_in_memory().unwrap();
        let office = found_office_start(&mut store, "m5-exit-after-session-ready");
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
        driver.pause_before_office_ready_liveness_for_test(Duration::from_millis(20));
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
        let mut store = KernelStore::open_in_memory().unwrap();
        let office = found_office_start(&mut store, "m5-generation-race-exit-before-ready");
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
        let mut store = KernelStore::open_in_memory().unwrap();
        let office = found_office_start(&mut store, "m5-reject-task-attempt-office");
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
        let mut store = KernelStore::open_in_memory().unwrap();
        let office = found_office_start(&mut store, "m5-unresolved-registration-ignore-term");
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
        assert_eq!(unresolved.pi_child_spawn_admission_id().value(), 1);
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
        let mut store = KernelStore::open_in_memory().unwrap();
        let office = found_office_start(&mut store, "m5-pending-dispose");
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
        let mut store = KernelStore::open_in_memory().unwrap();
        let office = found_office_start(&mut store, "m5-never-session-ready-ignore-term");
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
                    ..
                } if observed == action
            ));
        }
        fixture.cleanup();
    }

    #[test]
    fn pre_adapter_and_pre_create_exits_still_enter_ordered_reap_seal_and_finalization() {
        for (label, observe_adapter_first) in [
            ("m5-exit-before-ready", false),
            ("m5-exit-after-ready", true),
        ] {
            let fixture = NativeFixture::new(label);
            let mut store = KernelStore::open_in_memory().unwrap();
            let office = found_office_start(&mut store, label);
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
        let mut store = KernelStore::open_in_memory().unwrap();
        let office = found_office_start(&mut store, "m5-owned-descendant-after-ready");
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
        let fixture = NativeFixture::new("m5-registered-setup-failure");
        let mut store = KernelStore::open_in_memory().unwrap();
        let office = found_office_start(&mut store, "m5-registered-setup-failure");
        let start = OfficePiExecutionStart {
            operation: PiExecutionOperationId::parse("m5-registered-setup-failure").unwrap(),
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

        // The emergency schedule is deterministic. Its Term/Kill receipts
        // are retained and then persisted before the direct wait, proving the
        // spawned child did not fall through the NotSpawned shortcut.
        driver
            .drive_boundary_containment(&child, MonotonicTick::from_milliseconds(1_000))
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

    struct OfficeStart {
        cycle_id: OperatingCycleId,
        session_id: GrandArchitectOfficeSessionId,
        epoch_identity: SupervisorEpochIdentity,
    }

    fn found_office_start(store: &mut KernelStore, label: &str) -> OfficeStart {
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
        accepted(
            store,
            "found-seed",
            bootstrap,
            Capability::InstallFoundingUniverseSeed,
            ExpectedGeneration::NotApplicable,
            CommandBody::InstallFoundingUniverseSeed {
                rendering_digest: KernelDigest::of_bytes(b"m5-office-bridge"),
            },
        );
        accepted(
            store,
            "found-office",
            bootstrap,
            Capability::InstallGrandArchitectOffice,
            ExpectedGeneration::NotApplicable,
            CommandBody::InstallGrandArchitectOffice,
        );
        accepted(
            store,
            "found-architect",
            bootstrap,
            Capability::AppointInitialGrandArchitect,
            ExpectedGeneration::NotApplicable,
            CommandBody::AppointInitialGrandArchitect {
                actor_display_name: PrincipalDisplayName::parse("Grand Architect").unwrap(),
            },
        );
        accepted(
            store,
            "found-ceiling",
            bootstrap,
            Capability::SetR0HardCeiling,
            ExpectedGeneration::NotApplicable,
            CommandBody::SetR0HardCeiling {
                ceiling: UsdMicros::VS001_SOCIETY_HARD_CEILING,
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
                treatment: OperatingCycleTreatment::Vs001DeterministicV1,
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
        let architect = PrincipalId::new(3).unwrap();
        accepted(
            store,
            "office-start",
            architect,
            Capability::StartGrandArchitectOfficeSession,
            generation,
            CommandBody::StartGrandArchitectOfficeSession { cycle_id },
        );
        accepted(
            store,
            "office-reserve",
            architect,
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
            session_id: GrandArchitectOfficeSessionId::new(1).unwrap(),
            epoch_identity,
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
        session_id: GrandArchitectOfficeSessionId,
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

    struct NativeFixture {
        root: PathBuf,
        workspace: NativeWorkspace,
        session: SessionIdentity,
        nonce: SpawnNonce,
        host: QualifiedHostExecution,
        create: CreateSessionPayload,
    }

    impl NativeFixture {
        fn new(label: &str) -> Self {
            let nonce = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "xsh-society-{label}-{}-{nonce}",
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
                    node_executable_sha256: node_digest,
                    lockfile_sha256: double_digest.clone(),
                    adapter_build_sha256: double_digest.clone(),
                    pi_transitive_package_set_sha256: double_digest,
                },
            };
            let prompt = "Universe Seed\nM5 provider-free Office bootstrap".to_owned();
            let create = CreateSessionPayload {
                session_kind: SessionKind::GrandArchitectOffice,
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
                    catalog.catalog_sha256 = digest_bytes(catalog_json.as_bytes());
                    catalog
                },
                tool_profile: ToolProfile::ReadSourceV1,
                settings: settings(),
            };
            Self {
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

        fn cleanup(self) {
            fs::remove_dir_all(self.root).unwrap();
        }
    }

    fn model_catalog() -> ModelCatalogPolicyV1 {
        ModelCatalogPolicyV1 {
            catalog_sha256: Sha256Digest::parse("a".repeat(64)).unwrap(),
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
    fn digest_file(path: &Path) -> Sha256Digest {
        digest_bytes(&fs::read(path).unwrap())
    }
    fn digest_bytes(bytes: &[u8]) -> Sha256Digest {
        let mut rendered = String::with_capacity(64);
        for byte in Sha256::digest(bytes) {
            use std::fmt::Write as _;
            write!(&mut rendered, "{byte:02x}").unwrap();
        }
        Sha256Digest::parse(rendered).unwrap()
    }
}
