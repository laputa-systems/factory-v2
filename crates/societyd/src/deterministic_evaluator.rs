//! Daemon-private deterministic evaluator process driver.
//!
//! There is deliberately no local protocol or supervisor scheduler request,
//! Pi session, or semantic evidence mutation here. The daemon-private
//! coordinator constructs [`DeterministicEvaluatorAdmission`] only from the
//! kernel's exact native-child admission/experiment binding. This driver then
//! owns native process physics and allows physical output sealing only after
//! direct reaping and owned-group containment have completed.

use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
};

use society_content::ContentReadLimit;
use society_kernel::{
    Blake3Digest, CanonicalWorkspacePath, Capability, ChildStreamKind, ChildStreamSealCompleteness,
    CommandBody, CommandDisposition, CommandId, CommandRequest,
    DeterministicEvaluatorScheduleClaim, DeterministicEvaluatorScheduleClaimRequest,
    DeterministicExperimentId, DirectChildWaitStatus, EvaluatorRevisionId, EventBody,
    ExpectedGeneration, InputManifestId, KernelStore, NativeChildId, NativeChildPid,
    NativeChildSpawnAdmissionId, NativeWorkspaceId as KernelWorkspaceId,
    OwnedProcessGroupId as KernelProcessGroupId, PrincipalId, ProcessExitCode,
    ProcessGroupLiveness as KernelProcessGroupLiveness,
    ProcessSignalAction as KernelProcessSignalAction,
    ProcessSignalCause as KernelProcessSignalCause,
    ProcessSignalDelivery as KernelProcessSignalDelivery, ProcessSignalNumber,
    SupervisedChildIdentity, SupervisorEpochId, SupervisorEpochIdentity,
};
use thiserror::Error;

use crate::{
    content::{
        ContentObjectRegistration, ContentSealOperationId, ContentSealOperationIdError,
        ContentSealingAuthority, ContentSealingError,
    },
    native_child::{
        NativeChildDeadline, NativeChildDirectReapFacts, NativeChildEnvironment,
        NativeChildExecution, NativeChildReceipt, NativeChildSpawnOutcome, NativeChildSpawnRequest,
        NativeChildSupervisor,
    },
    supervision::{
        MonotonicTick, NativeWorkspace, ProcessGroupLiveness, ReapStatus, SignalAction,
        SignalDelivery, SignalReceipt, SupervisedChildId, SupervisionError, TransientRetention,
        VerifiedArtifact,
    },
};

/// Materialization is intentionally bounded separately from stream capture:
/// a registered content object is immutable but it still must not make the
/// resident allocate an unbounded evaluator or input artifact. This matches
/// the daemon's fixed 64 MiB content-seal ceiling while retaining no generic
/// application file-size knob.
const MAX_MATERIALIZED_EVALUATOR_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const MATERIALIZED_EVALUATOR_FILE: &str = "evaluator";
const MATERIALIZED_INPUT_MANIFEST_FILE: &str = "input-manifest";

/// Exact local files copied from the content store for one already-admitted
/// evaluator binding. Their paths are fresh direct children of the daemon
/// workspace; neither a caller nor an application provides a filesystem path
/// to the evaluator process boundary.
#[derive(Clone, Debug)]
pub(crate) struct MaterializedDeterministicEvaluatorArtifacts {
    pub(crate) evaluator: VerifiedArtifact,
    pub(crate) input_manifest: VerifiedArtifact,
}

/// Releases kernel-bound sealed bytes into fixed filenames in a newly
/// allocated daemon workspace, then re-inspects those exact local files. The
/// copied evaluator alone receives an executable mode; the input manifest is
/// data and remains non-executable. This is physical byte custody, not an
/// evaluator qualification, an application execution result, or evidence.
pub(crate) fn materialize_sealed_evaluator_artifacts(
    authority: &ContentSealingAuthority,
    workspace: &NativeWorkspace,
    evaluator_digest: Blake3Digest,
    input_manifest_digest: Blake3Digest,
) -> Result<MaterializedDeterministicEvaluatorArtifacts, DeterministicEvaluatorMaterializationError>
{
    let Some(read_limit) = ContentReadLimit::new(MAX_MATERIALIZED_EVALUATOR_ARTIFACT_BYTES) else {
        return Err(DeterministicEvaluatorMaterializationError::InvalidReadLimit);
    };
    let evaluator = materialize_artifact(
        authority,
        workspace,
        MATERIALIZED_EVALUATOR_FILE,
        evaluator_digest,
        read_limit,
        0o700,
    )?;
    let input_manifest = match materialize_artifact(
        authority,
        workspace,
        MATERIALIZED_INPUT_MANIFEST_FILE,
        input_manifest_digest,
        read_limit,
        0o600,
    ) {
        Ok(artifact) => artifact,
        Err(error) => {
            fs::remove_file(evaluator.path().as_path()).map_err(|cleanup| {
                DeterministicEvaluatorMaterializationError::PriorArtifactCleanup { cleanup }
            })?;
            return Err(error);
        }
    };
    Ok(MaterializedDeterministicEvaluatorArtifacts {
        evaluator,
        input_manifest,
    })
}

fn materialize_artifact(
    authority: &ContentSealingAuthority,
    workspace: &NativeWorkspace,
    name: &str,
    expected_digest: Blake3Digest,
    read_limit: ContentReadLimit,
    mode: u32,
) -> Result<VerifiedArtifact, DeterministicEvaluatorMaterializationError> {
    let path = workspace.directory().as_path().join(name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)?;
    let mut cleanup = FreshMaterializedArtifact::new(path.clone());
    // `create_new` made this a fresh child of the already private workspace;
    // set the exact final mode rather than inheriting a process umask.
    file.set_permissions(fs::Permissions::from_mode(mode))?;
    authority
        .copy_verified_content_to(expected_digest, read_limit, &mut file)
        .map_err(DeterministicEvaluatorMaterializationError::Content)?;
    file.sync_all()?;
    let expected_digest = protocol_digest_from_kernel(expected_digest)
        .map_err(|_| DeterministicEvaluatorMaterializationError::InvalidDigest)?;
    let artifact = VerifiedArtifact::inspect(path, expected_digest)
        .map_err(DeterministicEvaluatorMaterializationError::Artifact)?;
    cleanup.keep();
    Ok(artifact)
}

/// Until an artifact has been fully copied, synced, and re-inspected, no
/// partial destination survives a failed materialization boundary. The
/// enclosing workspace remains retired by the coordinator after that failure;
/// it is never retried as a caller-selected path or reused by another
/// admission.
struct FreshMaterializedArtifact {
    path: std::path::PathBuf,
    keep: bool,
}

impl FreshMaterializedArtifact {
    fn new(path: std::path::PathBuf) -> Self {
        Self { path, keep: false }
    }

    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for FreshMaterializedArtifact {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// The daemon-local projection of one already durable kernel admission. The
/// identity fields are retained together so the private coordinator cannot
/// splice an evaluator executable or input treatment from another experiment.
/// The fields stay private: arbitrary paths, program identities, and argv do
/// not cross a public daemon boundary.
#[derive(Clone, Debug)]
pub(crate) struct DeterministicEvaluatorAdmission {
    native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    deterministic_experiment_id: DeterministicExperimentId,
    evaluator_revision_id: EvaluatorRevisionId,
    input_manifest_id: InputManifestId,
    child_process_id: SupervisedChildId,
    workspace: NativeWorkspace,
    evaluator_artifact: VerifiedArtifact,
    input_manifest_artifact: VerifiedArtifact,
}

/// No public construction API is exposed. The private coordinator is the one
/// narrow, auditable join point from a kernel claim to a generic direct
/// executor.
impl DeterministicEvaluatorAdmission {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_kernel_admission(
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
        deterministic_experiment_id: DeterministicExperimentId,
        evaluator_revision_id: EvaluatorRevisionId,
        input_manifest_id: InputManifestId,
        child_process_id: SupervisedChildId,
        workspace: NativeWorkspace,
        evaluator_artifact: VerifiedArtifact,
        input_manifest_artifact: VerifiedArtifact,
    ) -> Self {
        Self {
            native_child_spawn_admission_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            child_process_id,
            workspace,
            evaluator_artifact,
            input_manifest_artifact,
        }
    }
}

#[derive(Default)]
pub(crate) struct DeterministicEvaluatorDriver {
    native_children: NativeChildSupervisor,
    admitted_children: BTreeMap<SupervisedChildId, NativeChildSpawnAdmissionId>,
}

impl DeterministicEvaluatorDriver {
    /// Rechecks direct executable/evaluator digests and spawns only the exact
    /// empty-environment direct-executable treatment. The caller records the
    /// returned native facts using the admission ID before any deadline tick.
    pub(crate) fn spawn_admitted(
        &mut self,
        kernel: &society_kernel::KernelStore,
        admission: DeterministicEvaluatorAdmission,
    ) -> Result<NativeChildSpawnOutcome, SupervisionError> {
        let kernel_admission = kernel
            .deterministic_evaluator_native_child_admission(
                admission.native_child_spawn_admission_id,
            )
            .map_err(|_| SupervisionError::InvalidNativeChildRequest)?
            .ok_or(SupervisionError::InvalidNativeChildRequest)?;
        if kernel_admission.deterministic_experiment_id() != admission.deterministic_experiment_id
            || kernel_admission.evaluator_revision_id() != admission.evaluator_revision_id
            || kernel_admission.input_manifest_id() != admission.input_manifest_id
        {
            return Err(SupervisionError::InvalidNativeChildRequest);
        }
        reject_script_or_digest_drift(
            &admission.evaluator_artifact,
            Some(kernel_admission.evaluator_digest()),
        )?;
        reject_script_or_digest_drift(
            &admission.input_manifest_artifact,
            Some(kernel_admission.input_manifest_digest()),
        )?;
        let _binding = (
            admission.native_child_spawn_admission_id,
            admission.deterministic_experiment_id,
            admission.evaluator_revision_id,
            admission.input_manifest_id,
        );
        let child_process_id = admission.child_process_id.clone();
        let native_child_spawn_admission_id = admission.native_child_spawn_admission_id;
        let execution = NativeChildExecution::direct_evaluator(
            admission.evaluator_artifact,
            admission.input_manifest_artifact,
        )?;
        let outcome = self.native_children.spawn(NativeChildSpawnRequest {
            child_process_id: admission.child_process_id,
            workspace: admission.workspace,
            execution,
            environment: NativeChildEnvironment::EmptyV1,
        })?;
        // A post-spawn setup failure is still an admitted containment subject.
        // Retain the binding so its later reaped streams cannot be spliced
        // into another evaluator admission.
        self.admitted_children
            .insert(child_process_id, native_child_spawn_admission_id);
        Ok(outcome)
    }

    pub(crate) fn drive_at(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
        deadline: NativeChildDeadline,
    ) -> Result<(), SupervisionError> {
        self.native_children
            .drive_at(child_process_id, now, deadline)
    }

    /// Polls only the direct child. The coordinator records this receipt
    /// before it attempts a later lingering-group cleanup, because wait(2)
    /// cannot speak for any descendants which remain in the owned group.
    pub(crate) fn poll_direct_child_reap(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<Option<NativeChildDirectReapFacts>, SupervisionError> {
        self.native_children
            .poll_direct_child_reap(child_process_id)
    }

    /// Uses the one post-reap lingering cleanup attempt owned by the generic
    /// native custody nucleus. It is not evaluator cancellation semantics.
    pub(crate) fn issue_lingering_group_cleanup(
        &mut self,
        child_process_id: &SupervisedChildId,
        now: MonotonicTick,
    ) -> Result<Option<SignalReceipt>, SupervisionError> {
        self.native_children
            .issue_lingering_group_cleanup(child_process_id, now)
    }

    /// Returns complete retained streams only after direct reaping and owned
    /// group absence. A caller must have first recorded the direct-reap fact
    /// and any preceding cleanup signal in the kernel.
    pub(crate) fn complete_deferred_reap(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<NativeChildReceipt, SupervisionError> {
        self.native_children
            .complete_deferred_reap(child_process_id)
    }

    /// Observes the currently owned group after a lingering cleanup attempt.
    /// The result is deliberately a physical fact which the coordinator must
    /// project separately; it never implies a child reaped or finalized.
    pub(crate) fn observe_group_liveness(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<ProcessGroupLiveness, SupervisionError> {
        self.native_children
            .observe_group_liveness(child_process_id)
    }

    /// Returns signal history without consuming it. The coordinator records
    /// any new receipt immediately, while this driver still retains the same
    /// history for its final post-reap custody receipt.
    pub(crate) fn signal_receipts(
        &mut self,
        child_process_id: &SupervisedChildId,
    ) -> Result<Vec<SignalReceipt>, SupervisionError> {
        self.native_children.signal_receipts(child_process_id)
    }

    /// The content writer sees only complete retained stdout after the generic
    /// core has reaped the direct child and found the owned group absent. No
    /// evaluator evidence command is emitted here: the coordinator projects
    /// only the derived forensic/evaluation receipts after this physical
    /// seal/register result exists.
    pub(crate) fn seal_reaped_streams(
        &self,
        authority: &ContentSealingAuthority,
        kernel: &mut society_kernel::KernelStore,
        receipt: &NativeChildReceipt,
    ) -> Result<DeterministicEvaluatorSealedStreams, DeterministicEvaluatorError> {
        let native_child_spawn_admission_id = self
            .admitted_children
            .get(&receipt.child_process_id)
            .copied()
            .ok_or(DeterministicEvaluatorError::ReceiptNotAdmitted)?;
        if receipt.stdout.retention != TransientRetention::Complete
            || receipt.stderr.retention != TransientRetention::Complete
        {
            return Err(DeterministicEvaluatorError::OutputNotComplete);
        }
        let stdout_operation = ContentSealOperationId::native_child_stream(
            native_child_spawn_admission_id,
            ChildStreamKind::Stdout,
            kernel_digest(receipt.stdout.blake3())?,
        )?;
        let stderr_operation = ContentSealOperationId::native_child_stream(
            native_child_spawn_admission_id,
            ChildStreamKind::Stderr,
            kernel_digest(receipt.stderr.blake3())?,
        )?;
        let stdout = authority
            .seal_and_register(kernel, &stdout_operation, receipt.stdout.retained_bytes())
            .map_err(DeterministicEvaluatorError::Content)?;
        let stderr = authority
            .seal_and_register(kernel, &stderr_operation, receipt.stderr.retained_bytes())
            .map_err(DeterministicEvaluatorError::Content)?;
        Ok(DeterministicEvaluatorSealedStreams { stdout, stderr })
    }
}

const SCHEDULE_COMMAND_PREFIX: &str = "deterministic-evaluator-schedule-v1/";
const CHILD_COMMAND_PREFIX: &str = "deterministic-evaluator-child-v1/";

/// Trusted private inputs for exactly one scheduler claim. The root is an
/// already-opened daemon-owned directory, but this start carries only a
/// prospective direct-child identity. The coordinator derives the exact
/// candidate path without creating it, claims first, and calls `allocate`
/// only after a kernel `SpawnAuthorized` response. There is intentionally no
/// application input, evaluator path, environment, or output selection.
#[derive(Clone, Debug)]
pub(crate) struct DeterministicEvaluatorScheduleStart {
    workspace_root: crate::supervision::NativeWorkspaceRoot,
    workspace_identity: crate::supervision::NativeWorkspaceId,
    supervisor_epoch_id: SupervisorEpochId,
    supervisor_epoch_identity: SupervisorEpochIdentity,
}

impl DeterministicEvaluatorScheduleStart {
    pub(crate) fn new(
        workspace_root: crate::supervision::NativeWorkspaceRoot,
        workspace_identity: crate::supervision::NativeWorkspaceId,
        supervisor_epoch_id: SupervisorEpochId,
        supervisor_epoch_identity: SupervisorEpochIdentity,
    ) -> Self {
        Self {
            workspace_root,
            workspace_identity,
            supervisor_epoch_id,
            supervisor_epoch_identity,
        }
    }
}

/// A daemon-private owner for the only generic evaluator lifecycle. It
/// invokes the kernel claim before it materializes either artifact, records
/// the PID/PGID immediately after `exec`, and projects only native custody,
/// sealed stream, forensic-occurrence, and deterministic-receipt facts.
/// It has no local protocol form, scheduler wire request, application result,
/// or semantic evidence admission.
#[derive(Default)]
pub(crate) struct DeterministicEvaluatorCoordinator {
    driver: DeterministicEvaluatorDriver,
}

/// A claim either introduced no new native work, repeated an already resolved
/// operation, or registered one exact child for later monotonic driving.
#[derive(Clone, Debug)]
pub(crate) enum DeterministicEvaluatorScheduleOutcome {
    NoEligibleExperiment,
    AlreadyClaimed {
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    },
    Spawned(DeterministicEvaluatorChild),
}

/// The durable/physical coordinates retained between nonblocking drives. No
/// artifact path, caller output, or application conclusion is exposed here.
#[derive(Clone, Debug)]
pub(crate) struct DeterministicEvaluatorChild {
    native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    native_child_id: NativeChildId,
    supervised_child_id: SupervisedChildId,
    operating_cycle_id: society_kernel::OperatingCycleId,
    deterministic_experiment_id: DeterministicExperimentId,
    evaluator_revision_id: EvaluatorRevisionId,
    input_manifest_id: InputManifestId,
    projected_signal_count: usize,
    direct_child_reap_recorded: bool,
    lingering_cleanup_attempted: bool,
    post_cleanup_liveness_recorded: bool,
    completed: bool,
}

/// A direct child can still need a later liveness observation after the one
/// owned lingering cleanup signal. Only the completed arm seals streams and
/// creates the generic forensic/evaluation receipts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeterministicEvaluatorReconciliation {
    StillRunning,
    AwaitingOwnedGroupAbsence,
    ContainmentBlocked,
    Completed(DeterministicEvaluatorCompletion),
}

/// Exact generic receipt coordinates produced after complete stdout/stderr
/// sealing. This reports no evaluator truth, application success, or semantic
/// evidence admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeterministicEvaluatorCompletion {
    pub(crate) native_child_id: NativeChildId,
    pub(crate) forensic_manifest_id: society_kernel::ForensicManifestId,
    pub(crate) deterministic_evaluation_receipt_id:
        society_kernel::DeterministicEvaluationReceiptId,
    pub(crate) stdout: ContentObjectRegistration,
    pub(crate) stderr: ContentObjectRegistration,
}

impl DeterministicEvaluatorCoordinator {
    /// Claims exactly one oldest eligible experiment. The durable kernel claim
    /// is the authorization boundary: workspace allocation, materialization,
    /// artifact checks, and native spawn all occur only after it succeeds.
    pub(crate) fn claim_materialize_and_spawn(
        &mut self,
        kernel: &mut KernelStore,
        authority: &ContentSealingAuthority,
        start: DeterministicEvaluatorScheduleStart,
    ) -> Result<DeterministicEvaluatorScheduleOutcome, DeterministicEvaluatorCoordinatorError> {
        let workspace_id = KernelWorkspaceId::parse(start.workspace_identity.as_str())
            .map_err(|_| DeterministicEvaluatorCoordinatorError::IdentityConversion)?;
        let workspace_path =
            prospective_workspace_path(&start.workspace_root, &start.workspace_identity)?;
        let command_id = schedule_claim_command_id(&start.workspace_identity)?;
        let claim = kernel.claim_registered_deterministic_evaluator(
            DeterministicEvaluatorScheduleClaimRequest::new(
                command_id,
                workspace_id,
                workspace_path,
                start.supervisor_epoch_id,
                start.supervisor_epoch_identity,
            ),
        )?;
        let Some(claim) = claim else {
            return Ok(DeterministicEvaluatorScheduleOutcome::NoEligibleExperiment);
        };
        let admission = match claim {
            DeterministicEvaluatorScheduleClaim::AlreadyClaimed {
                native_child_spawn_admission_id,
            } => {
                return Ok(DeterministicEvaluatorScheduleOutcome::AlreadyClaimed {
                    native_child_spawn_admission_id,
                });
            }
            DeterministicEvaluatorScheduleClaim::SpawnAuthorized(admission) => admission,
        };
        let admission_id = admission.native_child_spawn_admission_id();
        let workspace = match start.workspace_root.allocate(start.workspace_identity) {
            Ok(workspace) => workspace,
            Err(error) => {
                self.record_not_spawned(
                    kernel,
                    admission_id,
                    admission.operating_cycle_id(),
                    society_kernel::NativeChildNotSpawnedReason::WorkspacePreparationFailed,
                )?;
                return Err(DeterministicEvaluatorCoordinatorError::Supervision(error));
            }
        };
        if admission.native_workspace_id().as_str() != workspace.identity().as_str()
            || admission.canonical_workspace_path().as_str()
                != canonical_workspace_path(&workspace)?.as_str()
        {
            self.record_not_spawned(
                kernel,
                admission_id,
                admission.operating_cycle_id(),
                society_kernel::NativeChildNotSpawnedReason::WorkspacePreparationFailed,
            )?;
            return Err(DeterministicEvaluatorCoordinatorError::ClaimWorkspaceMismatch);
        }
        let materialized = match materialize_sealed_evaluator_artifacts(
            authority,
            &workspace,
            admission.evaluator_digest(),
            admission.input_manifest_digest(),
        ) {
            Ok(materialized) => materialized,
            Err(error) => {
                self.record_not_spawned(
                    kernel,
                    admission_id,
                    admission.operating_cycle_id(),
                    materialization_not_spawned_reason(&error),
                )?;
                return Err(DeterministicEvaluatorCoordinatorError::Materialization(
                    error,
                ));
            }
        };
        let supervised_child_id = SupervisedChildId::parse(format!(
            "deterministic-evaluator-child-{}",
            admission_id.value()
        ))
        .map_err(|_| DeterministicEvaluatorCoordinatorError::IdentityConversion)?;
        let driver_admission = DeterministicEvaluatorAdmission::from_kernel_admission(
            admission_id,
            admission.deterministic_experiment_id(),
            admission.evaluator_revision_id(),
            admission.input_manifest_id(),
            supervised_child_id.clone(),
            workspace,
            materialized.evaluator,
            materialized.input_manifest,
        );
        let spawned = match self.driver.spawn_admitted(kernel, driver_admission) {
            Ok(outcome) => outcome,
            Err(error) => {
                if let Some(reason) = supervisor_not_spawned_reason(&error) {
                    self.record_not_spawned(
                        kernel,
                        admission_id,
                        admission.operating_cycle_id(),
                        reason,
                    )?;
                }
                return Err(DeterministicEvaluatorCoordinatorError::Supervision(error));
            }
        };
        let facts = match &spawned {
            NativeChildSpawnOutcome::Ready(facts)
            | NativeChildSpawnOutcome::RegisteredSetupFailure { facts, .. } => facts,
        };
        if facts.child_process_id != supervised_child_id {
            return Err(DeterministicEvaluatorCoordinatorError::SpawnFactMismatch);
        }
        let native_child_id =
            self.record_spawn(kernel, admission_id, admission.operating_cycle_id(), facts)?;
        let mut child = DeterministicEvaluatorChild {
            native_child_spawn_admission_id: admission_id,
            native_child_id,
            supervised_child_id,
            operating_cycle_id: admission.operating_cycle_id(),
            deterministic_experiment_id: admission.deterministic_experiment_id(),
            evaluator_revision_id: admission.evaluator_revision_id(),
            input_manifest_id: admission.input_manifest_id(),
            projected_signal_count: 0,
            direct_child_reap_recorded: false,
            lingering_cleanup_attempted: false,
            post_cleanup_liveness_recorded: false,
            completed: false,
        };
        // A pipe setup failure occurred after `exec`, so it remains a child
        // custody subject rather than becoming a false not-spawned record.
        // Its initial kill receipt is durable before this method returns.
        self.record_new_signal_receipts(kernel, &mut child)?;
        Ok(DeterministicEvaluatorScheduleOutcome::Spawned(child))
    }

    /// Drives fixed output/deadline containment and immediately commits any
    /// new physical signal attempts. The driver never sleeps; the resident
    /// supplies its monotonic control-loop tick.
    pub(crate) fn drive_at(
        &mut self,
        kernel: &mut KernelStore,
        child: &mut DeterministicEvaluatorChild,
        now: MonotonicTick,
        deadline: NativeChildDeadline,
    ) -> Result<(), DeterministicEvaluatorCoordinatorError> {
        let outcome = self
            .driver
            .drive_at(&child.supervised_child_id, now, deadline);
        self.record_new_signal_receipts(kernel, child)?;
        outcome.map_err(DeterministicEvaluatorCoordinatorError::Supervision)
    }

    /// Records the direct wait fact before a possible lingering-group cleanup,
    /// then seals only complete reaped output. A `Present` group stays owned
    /// and must be driven again; an inaccessible group is a containment block.
    pub(crate) fn reconcile_at(
        &mut self,
        kernel: &mut KernelStore,
        authority: &ContentSealingAuthority,
        child: &mut DeterministicEvaluatorChild,
        now: MonotonicTick,
    ) -> Result<DeterministicEvaluatorReconciliation, DeterministicEvaluatorCoordinatorError> {
        if child.completed {
            return Err(DeterministicEvaluatorCoordinatorError::InvalidLifecycle);
        }
        self.record_new_signal_receipts(kernel, child)?;
        let mut liveness_after_reap = None;
        if !child.direct_child_reap_recorded {
            let Some(reap) = self
                .driver
                .poll_direct_child_reap(&child.supervised_child_id)?
            else {
                return Ok(DeterministicEvaluatorReconciliation::StillRunning);
            };
            self.record_new_signal_receipts(kernel, child)?;
            self.record_direct_reap(kernel, child, &reap)?;
            child.direct_child_reap_recorded = true;
            liveness_after_reap = Some(reap.group_liveness_after_direct_child_reap);
        }
        let liveness = liveness_after_reap.unwrap_or({
            // The already recorded direct reap has one physical group state;
            // subsequent calls observe the still-owned group without inventing
            // a second wait receipt.
            ProcessGroupLiveness::Present
        });
        if liveness == ProcessGroupLiveness::Absent {
            return self.seal_and_complete(kernel, authority, child);
        }
        if liveness == ProcessGroupLiveness::Inaccessible {
            // The direct-reap receipt has already put the kernel child into
            // containment failure. A later signal cannot repair or relabel
            // that fact, so retain the owned handle without emitting a
            // rejected post-terminal receipt.
            return Ok(DeterministicEvaluatorReconciliation::ContainmentBlocked);
        }
        if !child.lingering_cleanup_attempted {
            let signal = self
                .driver
                .issue_lingering_group_cleanup(&child.supervised_child_id, now)?;
            child.lingering_cleanup_attempted = true;
            self.record_new_signal_receipts(kernel, child)?;
            if let Some(signal) = signal {
                match signal.group_liveness_after_attempt {
                    // Preserve the signal's immediate Present observation
                    // without spending the one post-cleanup liveness command.
                    // A later Absent fact must use that stable command ID.
                    ProcessGroupLiveness::Present => {
                        return Ok(DeterministicEvaluatorReconciliation::AwaitingOwnedGroupAbsence);
                    }
                    ProcessGroupLiveness::Inaccessible => {
                        return Ok(DeterministicEvaluatorReconciliation::ContainmentBlocked);
                    }
                    ProcessGroupLiveness::Absent => {}
                }
            }
        }
        let liveness = self
            .driver
            .observe_group_liveness(&child.supervised_child_id)?;
        match liveness {
            ProcessGroupLiveness::Absent => {
                self.record_post_cleanup_liveness(kernel, child, liveness)?;
                self.seal_and_complete(kernel, authority, child)
            }
            ProcessGroupLiveness::Present => {
                Ok(DeterministicEvaluatorReconciliation::AwaitingOwnedGroupAbsence)
            }
            ProcessGroupLiveness::Inaccessible => {
                self.record_post_cleanup_liveness(kernel, child, liveness)?;
                Ok(DeterministicEvaluatorReconciliation::ContainmentBlocked)
            }
        }
    }

    fn record_not_spawned(
        &self,
        kernel: &mut KernelStore,
        admission_id: NativeChildSpawnAdmissionId,
        operating_cycle_id: society_kernel::OperatingCycleId,
        reason: society_kernel::NativeChildNotSpawnedReason,
    ) -> Result<(), DeterministicEvaluatorCoordinatorError> {
        let event = execute_child_command(
            kernel,
            admission_id,
            DeterministicEvaluatorChildCommand::RecordNotSpawned,
            Capability::RecordNativeChildNotSpawned,
            current_expected_generation(kernel, operating_cycle_id)?,
            CommandBody::RecordNativeChildNotSpawned {
                native_child_spawn_admission_id: admission_id,
                reason,
            },
        )?;
        match event {
            EventBody::NativeChildSpawnInvalidated {
                native_child_spawn_admission_id,
                reason: recorded_reason,
            } if native_child_spawn_admission_id == admission_id && recorded_reason == reason => {
                Ok(())
            }
            _ => Err(DeterministicEvaluatorCoordinatorError::UnexpectedKernelEvent),
        }
    }

    fn record_spawn(
        &self,
        kernel: &mut KernelStore,
        admission_id: NativeChildSpawnAdmissionId,
        operating_cycle_id: society_kernel::OperatingCycleId,
        facts: &crate::native_child::NativeChildSpawnFacts,
    ) -> Result<NativeChildId, DeterministicEvaluatorCoordinatorError> {
        let child_identity = SupervisedChildIdentity::parse(facts.child_process_id.as_str())
            .map_err(|_| DeterministicEvaluatorCoordinatorError::IdentityConversion)?;
        let direct_child_pid = kernel_child_pid(facts.host_process_id.value())?;
        let process_group_id = KernelProcessGroupId::try_from(facts.process_group_id.value())
            .map_err(|_| DeterministicEvaluatorCoordinatorError::IdentityConversion)?;
        let event = execute_child_command(
            kernel,
            admission_id,
            DeterministicEvaluatorChildCommand::RecordSpawn,
            Capability::RecordDeterministicEvaluatorNativeChildSpawn,
            current_expected_generation(kernel, operating_cycle_id)?,
            CommandBody::RecordDeterministicEvaluatorNativeChildSpawn {
                native_child_spawn_admission_id: admission_id,
                child_identity,
                direct_child_pid,
                process_group_id,
            },
        )?;
        match event {
            EventBody::DeterministicEvaluatorNativeChildSpawnRecorded {
                native_child_id,
                native_child_spawn_admission_id,
            } if native_child_spawn_admission_id == admission_id => Ok(native_child_id),
            _ => Err(DeterministicEvaluatorCoordinatorError::UnexpectedKernelEvent),
        }
    }

    fn record_new_signal_receipts(
        &mut self,
        kernel: &mut KernelStore,
        child: &mut DeterministicEvaluatorChild,
    ) -> Result<(), DeterministicEvaluatorCoordinatorError> {
        let receipts = self.driver.signal_receipts(&child.supervised_child_id)?;
        if child.projected_signal_count > receipts.len() {
            return Err(DeterministicEvaluatorCoordinatorError::SignalHistoryRegressed);
        }
        for (ordinal, receipt) in receipts
            .iter()
            .enumerate()
            .skip(child.projected_signal_count)
        {
            let event = execute_child_command(
                kernel,
                child.native_child_spawn_admission_id,
                DeterministicEvaluatorChildCommand::RecordSignal { ordinal },
                Capability::RecordProcessSignalReceipt,
                current_expected_generation(kernel, child.operating_cycle_id)?,
                CommandBody::RecordProcessSignalReceipt {
                    native_child_id: child.native_child_id,
                    action: kernel_signal_action(receipt.action)?,
                    delivery: kernel_signal_delivery(receipt.delivery)?,
                    observed_liveness: kernel_liveness(receipt.group_liveness_after_attempt),
                    cause: KernelProcessSignalCause::AutomaticBoundaryContainment,
                },
            )?;
            match event {
                EventBody::ProcessSignalReceiptRecorded {
                    native_child_id, ..
                } if native_child_id == child.native_child_id => {}
                _ => return Err(DeterministicEvaluatorCoordinatorError::UnexpectedKernelEvent),
            }
            child.projected_signal_count = ordinal + 1;
        }
        Ok(())
    }

    fn record_direct_reap(
        &self,
        kernel: &mut KernelStore,
        child: &DeterministicEvaluatorChild,
        reap: &NativeChildDirectReapFacts,
    ) -> Result<(), DeterministicEvaluatorCoordinatorError> {
        if reap.child_process_id != child.supervised_child_id {
            return Err(DeterministicEvaluatorCoordinatorError::ReapFactMismatch);
        }
        let event = execute_child_command(
            kernel,
            child.native_child_spawn_admission_id,
            DeterministicEvaluatorChildCommand::RecordDirectReap,
            Capability::RecordDirectChildReap,
            current_expected_generation(kernel, child.operating_cycle_id)?,
            CommandBody::RecordDirectChildReap {
                native_child_id: child.native_child_id,
                wait_status: kernel_wait_status(reap.status)?,
                group_liveness_before_cleanup: kernel_liveness(
                    reap.group_liveness_after_direct_child_reap,
                ),
                group_liveness_after_cleanup: kernel_liveness(
                    reap.group_liveness_after_direct_child_reap,
                ),
            },
        )?;
        match event {
            EventBody::DirectChildReaped {
                native_child_id, ..
            } if native_child_id == child.native_child_id => Ok(()),
            _ => Err(DeterministicEvaluatorCoordinatorError::UnexpectedKernelEvent),
        }
    }

    /// Records the later post-cleanup observation which closes a direct-reap
    /// `Present` group. The cleanup signal's immediate liveness is already a
    /// separate receipt, so this command is reserved for a later `Absent` or
    /// `Inaccessible` physical fact rather than overwritten by retries while
    /// the group remains present.
    fn record_post_cleanup_liveness(
        &self,
        kernel: &mut KernelStore,
        child: &mut DeterministicEvaluatorChild,
        liveness: ProcessGroupLiveness,
    ) -> Result<(), DeterministicEvaluatorCoordinatorError> {
        if child.post_cleanup_liveness_recorded {
            return Ok(());
        }
        let event = execute_child_command(
            kernel,
            child.native_child_spawn_admission_id,
            DeterministicEvaluatorChildCommand::RecordPostCleanupLiveness,
            Capability::RecordChildProcessLiveness,
            current_expected_generation(kernel, child.operating_cycle_id)?,
            CommandBody::RecordChildProcessLiveness {
                native_child_id: child.native_child_id,
                liveness: kernel_liveness(liveness),
            },
        )?;
        match event {
            EventBody::ChildProcessLivenessObserved {
                native_child_id,
                liveness: recorded_liveness,
                ..
            } if native_child_id == child.native_child_id
                && recorded_liveness == kernel_liveness(liveness) =>
            {
                child.post_cleanup_liveness_recorded = true;
                Ok(())
            }
            _ => Err(DeterministicEvaluatorCoordinatorError::UnexpectedKernelEvent),
        }
    }

    fn seal_and_complete(
        &mut self,
        kernel: &mut KernelStore,
        authority: &ContentSealingAuthority,
        child: &mut DeterministicEvaluatorChild,
    ) -> Result<DeterministicEvaluatorReconciliation, DeterministicEvaluatorCoordinatorError> {
        let receipt = self
            .driver
            .complete_deferred_reap(&child.supervised_child_id)?;
        if receipt.child_process_id != child.supervised_child_id
            || receipt.group_liveness_after_cleanup != ProcessGroupLiveness::Absent
        {
            return Err(DeterministicEvaluatorCoordinatorError::ReapFactMismatch);
        }
        let sealed = self
            .driver
            .seal_reaped_streams(authority, kernel, &receipt)?;
        self.record_stream_seal(
            kernel,
            child,
            ChildStreamKind::Stdout,
            kernel_digest(receipt.stdout.blake3())?,
            sealed.stdout.content_object_id,
        )?;
        self.record_stream_seal(
            kernel,
            child,
            ChildStreamKind::Stderr,
            kernel_digest(receipt.stderr.blake3())?,
            sealed.stderr.content_object_id,
        )?;
        let event = execute_child_command(
            kernel,
            child.native_child_spawn_admission_id,
            DeterministicEvaluatorChildCommand::FinalizeChild,
            Capability::FinalizeChildProcess,
            current_expected_generation(kernel, child.operating_cycle_id)?,
            CommandBody::FinalizeChildProcess {
                native_child_id: child.native_child_id,
            },
        )?;
        match event {
            EventBody::ChildProcessFinalized {
                native_child_id, ..
            } if native_child_id == child.native_child_id => {}
            _ => return Err(DeterministicEvaluatorCoordinatorError::UnexpectedKernelEvent),
        }
        let event = execute_child_command(
            kernel,
            child.native_child_spawn_admission_id,
            DeterministicEvaluatorChildCommand::RegisterForensicManifest,
            Capability::RegisterDeterministicEvaluatorForensicManifest,
            current_expected_generation(kernel, child.operating_cycle_id)?,
            CommandBody::RegisterDeterministicEvaluatorForensicManifest {
                operating_cycle_id: child.operating_cycle_id,
                native_child_spawn_admission_id: child.native_child_spawn_admission_id,
            },
        )?;
        let (forensic_manifest_id, output_content_object_id) = match event {
            EventBody::DeterministicEvaluatorForensicManifestRegistered {
                forensic_manifest_id,
                deterministic_experiment_id,
                native_child_spawn_admission_id,
                evaluator_output_content_object_id,
                ..
            } if deterministic_experiment_id == child.deterministic_experiment_id
                && native_child_spawn_admission_id == child.native_child_spawn_admission_id
                && evaluator_output_content_object_id == sealed.stdout.content_object_id =>
            {
                (forensic_manifest_id, evaluator_output_content_object_id)
            }
            _ => return Err(DeterministicEvaluatorCoordinatorError::UnexpectedKernelEvent),
        };
        let event = execute_child_command(
            kernel,
            child.native_child_spawn_admission_id,
            DeterministicEvaluatorChildCommand::RecordEvaluationReceipt,
            Capability::RecordDeterministicEvaluationReceipt,
            current_expected_generation(kernel, child.operating_cycle_id)?,
            CommandBody::RecordDeterministicEvaluationReceipt {
                operating_cycle_id: child.operating_cycle_id,
                deterministic_experiment_id: child.deterministic_experiment_id,
                evaluator_revision_id: child.evaluator_revision_id,
                input_manifest_id: child.input_manifest_id,
                forensic_manifest_id,
                evaluator_output_content_object_id: output_content_object_id,
            },
        )?;
        let deterministic_evaluation_receipt_id = match event {
            EventBody::DeterministicEvaluationReceiptRecorded {
                deterministic_evaluation_receipt_id,
                deterministic_experiment_id,
            } if deterministic_experiment_id == child.deterministic_experiment_id => {
                deterministic_evaluation_receipt_id
            }
            _ => return Err(DeterministicEvaluatorCoordinatorError::UnexpectedKernelEvent),
        };
        child.completed = true;
        Ok(DeterministicEvaluatorReconciliation::Completed(
            DeterministicEvaluatorCompletion {
                native_child_id: child.native_child_id,
                forensic_manifest_id,
                deterministic_evaluation_receipt_id,
                stdout: sealed.stdout,
                stderr: sealed.stderr,
            },
        ))
    }

    fn record_stream_seal(
        &self,
        kernel: &mut KernelStore,
        child: &DeterministicEvaluatorChild,
        stream_kind: ChildStreamKind,
        full_observed_digest: Blake3Digest,
        retained_content_object_id: society_kernel::ContentObjectId,
    ) -> Result<(), DeterministicEvaluatorCoordinatorError> {
        let command = match stream_kind {
            ChildStreamKind::Stdout => DeterministicEvaluatorChildCommand::SealStdout,
            ChildStreamKind::Stderr => DeterministicEvaluatorChildCommand::SealStderr,
            ChildStreamKind::AdmittedControl | ChildStreamKind::PhysicalStdin => {
                return Err(DeterministicEvaluatorCoordinatorError::InvalidStreamKind);
            }
        };
        let event = execute_child_command(
            kernel,
            child.native_child_spawn_admission_id,
            command,
            Capability::RecordChildStreamSeal,
            current_expected_generation(kernel, child.operating_cycle_id)?,
            CommandBody::RecordChildStreamSeal {
                native_child_id: child.native_child_id,
                stream_kind,
                full_observed_digest,
                retained_content_object_id,
                completeness: ChildStreamSealCompleteness::Complete,
            },
        )?;
        match event {
            EventBody::ChildStreamSealed {
                native_child_id,
                stream_kind: recorded_stream,
                completeness: ChildStreamSealCompleteness::Complete,
                ..
            } if native_child_id == child.native_child_id && recorded_stream == stream_kind => {
                Ok(())
            }
            _ => Err(DeterministicEvaluatorCoordinatorError::UnexpectedKernelEvent),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeterministicEvaluatorChildCommand {
    RecordSpawn,
    RecordNotSpawned,
    RecordSignal { ordinal: usize },
    RecordDirectReap,
    RecordPostCleanupLiveness,
    SealStdout,
    SealStderr,
    RegisterForensicManifest,
    RecordEvaluationReceipt,
    FinalizeChild,
}

impl std::fmt::Display for DeterministicEvaluatorChildCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RecordSpawn => formatter.write_str("record-spawn"),
            Self::RecordNotSpawned => formatter.write_str("record-not-spawned"),
            Self::RecordSignal { ordinal } => write!(formatter, "record-signal-{ordinal}"),
            Self::RecordDirectReap => formatter.write_str("record-direct-reap"),
            Self::RecordPostCleanupLiveness => formatter.write_str("record-post-cleanup-liveness"),
            Self::SealStdout => formatter.write_str("seal-stdout"),
            Self::SealStderr => formatter.write_str("seal-stderr"),
            Self::RegisterForensicManifest => formatter.write_str("register-forensic-manifest"),
            Self::RecordEvaluationReceipt => formatter.write_str("record-evaluation-receipt"),
            Self::FinalizeChild => formatter.write_str("finalize-child"),
        }
    }
}

fn schedule_claim_command_id(
    workspace_identity: &crate::supervision::NativeWorkspaceId,
) -> Result<CommandId, DeterministicEvaluatorCoordinatorError> {
    CommandId::parse(format!(
        "{SCHEDULE_COMMAND_PREFIX}{}/claim",
        workspace_identity.as_str()
    ))
    .map_err(|_| DeterministicEvaluatorCoordinatorError::IdentityConversion)
}

fn execute_child_command(
    kernel: &mut KernelStore,
    admission_id: NativeChildSpawnAdmissionId,
    command: DeterministicEvaluatorChildCommand,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> Result<EventBody, DeterministicEvaluatorCoordinatorError> {
    let capability_grant_id = kernel
        .active_capability_grant(PrincipalId::KERNEL, capability)?
        .ok_or(
            DeterministicEvaluatorCoordinatorError::KernelServiceCapabilityMissing { capability },
        )?;
    let command_id = CommandId::parse(format!(
        "{CHILD_COMMAND_PREFIX}{}/{command}",
        admission_id.value()
    ))
    .map_err(|_| DeterministicEvaluatorCoordinatorError::IdentityConversion)?;
    let receipt = kernel.execute(CommandRequest {
        command_id,
        principal_id: PrincipalId::KERNEL,
        capability_grant_id,
        capability,
        expected_generation,
        body,
    })?;
    let event_id = match receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        CommandDisposition::Rejected(rejection) => {
            return Err(
                DeterministicEvaluatorCoordinatorError::KernelCommandRejected {
                    capability,
                    rejection,
                },
            );
        }
    };
    Ok(kernel.ledger_event(event_id)?.body)
}

fn current_expected_generation(
    kernel: &KernelStore,
    operating_cycle_id: society_kernel::OperatingCycleId,
) -> Result<ExpectedGeneration, DeterministicEvaluatorCoordinatorError> {
    Ok(ExpectedGeneration::Exact(
        kernel.current_operating_cycle_admission_generation(operating_cycle_id)?,
    ))
}

fn canonical_workspace_path(
    workspace: &NativeWorkspace,
) -> Result<CanonicalWorkspacePath, DeterministicEvaluatorCoordinatorError> {
    let path = workspace
        .directory()
        .as_path()
        .to_str()
        .ok_or(DeterministicEvaluatorCoordinatorError::IdentityConversion)?;
    CanonicalWorkspacePath::parse(path)
        .map_err(|_| DeterministicEvaluatorCoordinatorError::IdentityConversion)
}

/// Forms the one path that `NativeWorkspaceRoot::allocate` will later create,
/// from a canonical already-owned root and a closed workspace identity. It
/// performs no filesystem mutation or candidate lookup, so an unsuccessful
/// claim cannot leave an empty workspace behind.
fn prospective_workspace_path(
    root: &crate::supervision::NativeWorkspaceRoot,
    identity: &crate::supervision::NativeWorkspaceId,
) -> Result<CanonicalWorkspacePath, DeterministicEvaluatorCoordinatorError> {
    let candidate = root.directory().as_path().join(identity.as_str());
    let candidate = candidate
        .to_str()
        .ok_or(DeterministicEvaluatorCoordinatorError::IdentityConversion)?;
    CanonicalWorkspacePath::parse(candidate)
        .map_err(|_| DeterministicEvaluatorCoordinatorError::IdentityConversion)
}

fn kernel_child_pid(
    host_process_id: u64,
) -> Result<NativeChildPid, DeterministicEvaluatorCoordinatorError> {
    let process_id = i32::try_from(host_process_id)
        .map_err(|_| DeterministicEvaluatorCoordinatorError::IdentityConversion)?;
    NativeChildPid::try_from(process_id)
        .map_err(|_| DeterministicEvaluatorCoordinatorError::IdentityConversion)
}

fn kernel_liveness(liveness: ProcessGroupLiveness) -> KernelProcessGroupLiveness {
    match liveness {
        ProcessGroupLiveness::Present => KernelProcessGroupLiveness::Present,
        ProcessGroupLiveness::Absent => KernelProcessGroupLiveness::Absent,
        ProcessGroupLiveness::Inaccessible => KernelProcessGroupLiveness::Inaccessible,
    }
}

fn kernel_signal_action(
    action: SignalAction,
) -> Result<KernelProcessSignalAction, DeterministicEvaluatorCoordinatorError> {
    match action {
        SignalAction::Terminate => Ok(KernelProcessSignalAction::Terminate),
        SignalAction::Kill => Ok(KernelProcessSignalAction::Kill),
        SignalAction::LingeringGroupKill => Ok(KernelProcessSignalAction::LingeringGroupKill),
        SignalAction::AbortControl => Err(DeterministicEvaluatorCoordinatorError::UnexpectedSignal),
    }
}

fn kernel_signal_delivery(
    delivery: SignalDelivery,
) -> Result<KernelProcessSignalDelivery, DeterministicEvaluatorCoordinatorError> {
    match delivery {
        SignalDelivery::TermSent
        | SignalDelivery::KillSent
        | SignalDelivery::LingeringGroupKillSent => Ok(KernelProcessSignalDelivery::Delivered),
        SignalDelivery::AbsentBeforeSignal => Ok(KernelProcessSignalDelivery::AbsentBeforeSignal),
        SignalDelivery::AbsentDuringSignal => Ok(KernelProcessSignalDelivery::AbsentDuringSignal),
        SignalDelivery::GroupInaccessible => Ok(KernelProcessSignalDelivery::Inaccessible),
        SignalDelivery::AbortControlWritten => {
            Err(DeterministicEvaluatorCoordinatorError::UnexpectedSignal)
        }
    }
}

fn kernel_wait_status(
    status: ReapStatus,
) -> Result<DirectChildWaitStatus, DeterministicEvaluatorCoordinatorError> {
    match status {
        ReapStatus::Exited { code } => ProcessExitCode::try_from(code)
            .map(|exit_code| DirectChildWaitStatus::Exited { exit_code })
            .map_err(|_| DeterministicEvaluatorCoordinatorError::InvalidWaitStatus),
        ReapStatus::Signaled { signal } => ProcessSignalNumber::try_from(signal)
            .map(|signal_number| DirectChildWaitStatus::Signaled { signal_number })
            .map_err(|_| DeterministicEvaluatorCoordinatorError::InvalidWaitStatus),
        ReapStatus::Unknown => Ok(DirectChildWaitStatus::Unknown),
    }
}

fn materialization_not_spawned_reason(
    error: &DeterministicEvaluatorMaterializationError,
) -> society_kernel::NativeChildNotSpawnedReason {
    match error {
        DeterministicEvaluatorMaterializationError::Content(_)
        | DeterministicEvaluatorMaterializationError::InvalidDigest
        | DeterministicEvaluatorMaterializationError::Artifact(_) => {
            society_kernel::NativeChildNotSpawnedReason::ArtifactQualificationFailed
        }
        DeterministicEvaluatorMaterializationError::InvalidReadLimit
        | DeterministicEvaluatorMaterializationError::Io(_)
        | DeterministicEvaluatorMaterializationError::PriorArtifactCleanup { .. } => {
            society_kernel::NativeChildNotSpawnedReason::WorkspacePreparationFailed
        }
    }
}

fn supervisor_not_spawned_reason(
    error: &SupervisionError,
) -> Option<society_kernel::NativeChildNotSpawnedReason> {
    match error {
        SupervisionError::NativeSpawn(_) => {
            Some(society_kernel::NativeChildNotSpawnedReason::NativeSpawnFailed)
        }
        SupervisionError::ArtifactIsNotRegularFile
        | SupervisionError::ArtifactDigestDrift
        | SupervisionError::InvalidNativeChildRequest => {
            Some(society_kernel::NativeChildNotSpawnedReason::ArtifactQualificationFailed)
        }
        SupervisionError::InvalidSpawnRequest
        | SupervisionError::UnsafeWorkspace
        | SupervisionError::UnsafeWorkspaceRoot
        | SupervisionError::WorkspaceAlreadyExists => {
            Some(society_kernel::NativeChildNotSpawnedReason::WorkspacePreparationFailed)
        }
        _ => None,
    }
}

#[derive(Debug, Error)]
pub(crate) enum DeterministicEvaluatorCoordinatorError {
    #[error(transparent)]
    Kernel(#[from] society_kernel::StoreError),
    #[error("trusted evaluator workspace/process identity could not cross the kernel boundary")]
    IdentityConversion,
    #[error("kernel claim did not retain the daemon-owned workspace identity/path")]
    ClaimWorkspaceMismatch,
    #[error("native evaluator spawn facts did not retain their supervised child identity")]
    SpawnFactMismatch,
    #[error("native evaluator reap facts did not retain their supervised child identity")]
    ReapFactMismatch,
    #[error("native evaluator signal history regressed while the child remained owned")]
    SignalHistoryRegressed,
    #[error("deterministic evaluator received a Pi-only signal receipt")]
    UnexpectedSignal,
    #[error("direct wait status could not be represented by the kernel")]
    InvalidWaitStatus,
    #[error("deterministic evaluator stream kind is not stdout or stderr")]
    InvalidStreamKind,
    #[error("deterministic evaluator child is already completed")]
    InvalidLifecycle,
    #[error("kernel service capability {capability:?} is absent")]
    KernelServiceCapabilityMissing { capability: Capability },
    #[error(
        "kernel rejected deterministic evaluator custody command {capability:?}: {rejection:?}"
    )]
    KernelCommandRejected {
        capability: Capability,
        rejection: society_kernel::Rejection,
    },
    #[error("kernel accepted an unexpected deterministic evaluator event")]
    UnexpectedKernelEvent,
    #[error("daemon could not materialize kernel-bound evaluator artifacts")]
    Materialization(#[source] DeterministicEvaluatorMaterializationError),
    #[error("generic native evaluator custody failed")]
    Supervision(#[from] SupervisionError),
    #[error("reaped evaluator streams could not be physically sealed")]
    Sealing(#[from] DeterministicEvaluatorError),
}

fn kernel_digest(digest: &society_pi::Blake3Digest) -> Result<Blake3Digest, SupervisionError> {
    Ok(Blake3Digest::from_bytes(protocol_digest_bytes(digest)?))
}

fn protocol_digest_from_kernel(
    digest: Blake3Digest,
) -> Result<society_pi::Blake3Digest, SupervisionError> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(64);
    for byte in digest.as_bytes() {
        text.push(char::from(HEX[(byte >> 4) as usize]));
        text.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    society_pi::Blake3Digest::parse(text).map_err(SupervisionError::Protocol)
}

fn reject_script_or_digest_drift(
    artifact: &VerifiedArtifact,
    expected: Option<society_kernel::Blake3Digest>,
) -> Result<(), SupervisionError> {
    if let Some(expected) = expected
        && protocol_digest_bytes(artifact.expected_blake3())? != expected.as_bytes()
    {
        return Err(SupervisionError::InvalidNativeChildRequest);
    }
    let mut file = std::fs::File::open(artifact.path().as_path())?;
    let mut prefix = [0_u8; 2];
    let count = file.read(&mut prefix)?;
    if count == 2 && prefix == *b"#!" {
        return Err(SupervisionError::InvalidNativeChildRequest);
    }
    artifact.verify_current_identity()
}

fn protocol_digest_bytes(digest: &society_pi::Blake3Digest) -> Result<[u8; 32], SupervisionError> {
    let text = digest.as_str().as_bytes();
    if text.len() != 64 {
        return Err(SupervisionError::InvalidNativeChildRequest);
    }
    let mut output = [0_u8; 32];
    let (pairs, []) = text.as_chunks::<2>() else {
        return Err(SupervisionError::InvalidNativeChildRequest);
    };
    for (index, pair) in pairs.iter().enumerate() {
        let nibble = |value: u8| match value {
            b'0'..=b'9' => Ok(value - b'0'),
            b'a'..=b'f' => Ok(value - b'a' + 10),
            _ => Err(SupervisionError::InvalidNativeChildRequest),
        };
        output[index] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Ok(output)
}

/// Both native output directions are physically sealed before a future kernel
/// stream/evidence command can refer to either object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DeterministicEvaluatorSealedStreams {
    pub(crate) stdout: ContentObjectRegistration,
    pub(crate) stderr: ContentObjectRegistration,
}

#[derive(Debug, Error)]
pub(crate) enum DeterministicEvaluatorError {
    #[error("reaped evaluator receipt was not produced by this admission")]
    ReceiptNotAdmitted,
    #[error("evaluator stdout was not completely captured after reaping")]
    OutputNotComplete,
    #[error("evaluator stream operation identity could not be derived")]
    OperationIdentity(#[from] ContentSealOperationIdError),
    #[error("evaluator stream digest was not canonical BLAKE3")]
    Digest(#[from] SupervisionError),
    #[error("reaped evaluator output could not be physically sealed")]
    Content(#[source] ContentSealingError),
}

#[derive(Debug, Error)]
pub(crate) enum DeterministicEvaluatorMaterializationError {
    #[error("verified evaluator artifact read could not be bounded")]
    InvalidReadLimit,
    #[error("kernel BLAKE3 digest could not form a verified local artifact identity")]
    InvalidDigest,
    #[error("sealed evaluator bytes could not be released from daemon content custody")]
    Content(#[from] ContentSealingError),
    #[error("daemon-owned evaluator workspace I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error(
        "failed to remove the first artifact after second-artifact materialization failed: {cleanup}"
    )]
    PriorArtifactCleanup { cleanup: std::io::Error },
    #[error("materialized evaluator artifact did not retain its exact verified identity")]
    Artifact(#[source] SupervisionError),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use society_content::{ContentSealLimit, ContentStoreRoot};
    use society_kernel::{
        AdmissionGeneration, ApplicationIdentity, ApplicationMissionInput, ApplicationName,
        ApplicationRevisionId, ApplicationRevisionOrdinal, Capability, CommandBody,
        CommandDisposition, CommandId, CommandRequest, GraphRevisionBody, GraphRevisionId,
        HypothesisRevisionText, MissionPrinciple, MissionPrincipleKind, MissionPrincipleText,
        MissionPrinciples, MissionStatement, NorthStarBoundaryCommitmentQuestion,
        NorthStarChangeQuestion, NorthStarImprovementEvidenceQuestion, NorthStarQuestionSet,
        NorthStarRevisitQuestion, OperatingCycleId, OperatingCycleTreatment, PrincipalDisplayName,
        PrincipalId, ProjectId, ProjectMilestoneName, ProjectName, ProjectNorthStarAlignment,
        ProjectNorthStarBoundaryCommitmentAnswer, ProjectNorthStarChangeAnswer,
        ProjectNorthStarImprovementEvidenceAnswer, ProjectNorthStarRevisitAnswer,
        ProjectObjectiveText, ProjectState, ProjectStopConditionText, SocietyName,
        SupervisorEpochId, SupervisorEpochIdentity, TicketAcceptanceConditionText, TicketId,
        TicketTitle, UsdMicros,
    };

    use crate::supervision::{NativeWorkspaceId, NativeWorkspaceRoot};

    use super::*;

    static NEXT_ARTIFACT: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn admitted_artifact_check_binds_raw_digest_and_rejects_shebang_programs() {
        let (direct, direct_path) = artifact(b"\x7fELFfixture");
        assert!(
            reject_script_or_digest_drift(&direct, Some(Blake3Digest::of_bytes(b"\x7fELFfixture")))
                .is_ok()
        );
        assert!(matches!(
            reject_script_or_digest_drift(
                &direct,
                Some(Blake3Digest::of_bytes(b"different fixture"))
            ),
            Err(SupervisionError::InvalidNativeChildRequest)
        ));
        fs::remove_file(direct_path).unwrap();

        let (script, script_path) = artifact(b"#!/bin/sh\nexit 0\n");
        assert!(matches!(
            reject_script_or_digest_drift(
                &script,
                Some(Blake3Digest::of_bytes(b"#!/bin/sh\nexit 0\n"))
            ),
            Err(SupervisionError::InvalidNativeChildRequest)
        ));
        fs::remove_file(script_path).unwrap();
    }

    #[test]
    fn materialization_releases_only_sealed_bytes_into_fixed_private_workspace_paths() {
        let parent = temporary_parent("materialize");
        let content_root = parent.join("content");
        let mut kernel =
            society_kernel::KernelStore::connect_test_path(parent.join("society.pg-test-schema"))
                .unwrap();
        let authority = ContentSealingAuthority::open(
            ContentStoreRoot::parse(content_root).unwrap(),
            ContentSealLimit::new(MAX_MATERIALIZED_EVALUATOR_ARTIFACT_BYTES).unwrap(),
        )
        .unwrap();
        let evaluator_bytes = fs::read("/usr/bin/true").unwrap();
        let input_bytes = b"sealed deterministic input manifest";
        let evaluator_digest = Blake3Digest::of_bytes(&evaluator_bytes);
        let input_digest = Blake3Digest::of_bytes(input_bytes);
        authority
            .seal_and_register(
                &mut kernel,
                &ContentSealOperationId::parse("materialized-evaluator", evaluator_digest).unwrap(),
                &evaluator_bytes,
            )
            .unwrap();
        authority
            .seal_and_register(
                &mut kernel,
                &ContentSealOperationId::parse("materialized-input", input_digest).unwrap(),
                input_bytes,
            )
            .unwrap();
        let workspace_root_path = parent.join("workspaces");
        fs::create_dir(&workspace_root_path).unwrap();
        fs::set_permissions(&workspace_root_path, fs::Permissions::from_mode(0o700)).unwrap();
        let workspace = NativeWorkspaceRoot::open_owned(&workspace_root_path)
            .unwrap()
            .allocate(NativeWorkspaceId::parse("deterministic-materialize").unwrap())
            .unwrap();

        let materialized = materialize_sealed_evaluator_artifacts(
            &authority,
            &workspace,
            evaluator_digest,
            input_digest,
        )
        .unwrap();

        assert_eq!(
            materialized.evaluator.path().as_path(),
            workspace
                .directory()
                .as_path()
                .join(MATERIALIZED_EVALUATOR_FILE)
        );
        assert_eq!(
            materialized.input_manifest.path().as_path(),
            workspace
                .directory()
                .as_path()
                .join(MATERIALIZED_INPUT_MANIFEST_FILE)
        );
        assert_eq!(
            fs::read(materialized.evaluator.path().as_path()).unwrap(),
            evaluator_bytes
        );
        assert_eq!(
            fs::read(materialized.input_manifest.path().as_path()).unwrap(),
            input_bytes
        );
        assert_eq!(
            fs::metadata(materialized.evaluator.path().as_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(materialized.input_manifest.path().as_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert!(matches!(
            materialize_sealed_evaluator_artifacts(
                &authority,
                &workspace,
                evaluator_digest,
                input_digest,
            ),
            Err(DeterministicEvaluatorMaterializationError::Io(error))
                if error.kind() == std::io::ErrorKind::AlreadyExists
        ));
        let failed_workspace = NativeWorkspaceRoot::open_owned(&workspace_root_path)
            .unwrap()
            .allocate(NativeWorkspaceId::parse("deterministic-materialize-failed").unwrap())
            .unwrap();
        assert!(
            materialize_sealed_evaluator_artifacts(
                &authority,
                &failed_workspace,
                evaluator_digest,
                Blake3Digest::of_bytes(b"not a sealed input manifest"),
            )
            .is_err()
        );
        assert!(
            !failed_workspace
                .directory()
                .as_path()
                .join(MATERIALIZED_EVALUATOR_FILE)
                .exists()
        );
        assert!(
            !failed_workspace
                .directory()
                .as_path()
                .join(MATERIALIZED_INPUT_MANIFEST_FILE)
                .exists()
        );

        drop(kernel);
        drop(authority);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn provider_free_coordinator_projects_later_owned_group_absence_before_sealing() {
        let parent = temporary_parent("coordinator");
        let mut kernel =
            society_kernel::KernelStore::connect_test_path(parent.join("society.pg-test-schema"))
                .unwrap();
        let authority = ContentSealingAuthority::open(
            ContentStoreRoot::parse(parent.join("content")).unwrap(),
            ContentSealLimit::new(MAX_MATERIALIZED_EVALUATOR_ARTIFACT_BYTES).unwrap(),
        )
        .unwrap();
        let mission_digest = Blake3Digest::of_bytes(b"private evaluator coordinator mission");
        authority
            .seal_and_register(
                &mut kernel,
                &ContentSealOperationId::parse("coordinator-mission", mission_digest).unwrap(),
                b"private evaluator coordinator mission",
            )
            .unwrap();
        let (root_authority, cycle) = founded_evaluator_cycle(&mut kernel, mission_digest);
        let project = active_project(&mut kernel, root_authority, cycle);
        let generation = society_kernel::ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
        accepted(
            &mut kernel,
            "coordinator-ticket",
            root_authority,
            Capability::CreateTicket,
            generation,
            CommandBody::CreateTicket {
                operating_cycle_id: cycle,
                project_id: project,
                ticket_title: TicketTitle::parse("Run one sealed deterministic evaluator").unwrap(),
                acceptance_condition: TicketAcceptanceConditionText::parse(
                    "The native receipt chain is complete.",
                )
                .unwrap(),
                prerequisite_ticket_id: None,
            },
        );
        accepted(
            &mut kernel,
            "coordinator-graph",
            root_authority,
            Capability::AddGraphObjectRevision,
            generation,
            CommandBody::AddGraphObjectRevision {
                operating_cycle_id: cycle,
                project_id: project,
                causal_episode_id: None,
                graph_object_id: None,
                body: GraphRevisionBody::Hypothesis {
                    hypothesis: HypothesisRevisionText::parse(
                        "A sealed direct evaluator can have complete native custody.",
                    )
                    .unwrap(),
                },
            },
        );
        let graph_revision = GraphRevisionId::new(1).unwrap();
        accepted(
            &mut kernel,
            "coordinator-graph-commit",
            root_authority,
            Capability::CommitGraphRevision,
            generation,
            CommandBody::CommitGraphRevision {
                operating_cycle_id: cycle,
                graph_revision_id: graph_revision,
            },
        );
        // The compiled test evaluator ignores the closed fixed argv grammar,
        // starts one owned descendant, waits for that descendant to report
        // that it is live, and then exits. This forces a direct reaping fact
        // before the separate LingeringGroupKill and a later group-absence
        // observation; neither a provider nor application runtime is used.
        let evaluator_bytes = delayed_descendant_evaluator(&parent);
        let evaluator_digest = Blake3Digest::of_bytes(&evaluator_bytes);
        let evaluator = authority
            .seal_and_register(
                &mut kernel,
                &ContentSealOperationId::parse("coordinator-evaluator", evaluator_digest).unwrap(),
                &evaluator_bytes,
            )
            .unwrap();
        let input_bytes = b"sealed coordinator input";
        let input_digest = Blake3Digest::of_bytes(input_bytes);
        let input = authority
            .seal_and_register(
                &mut kernel,
                &ContentSealOperationId::parse("coordinator-input", input_digest).unwrap(),
                input_bytes,
            )
            .unwrap();
        accepted(
            &mut kernel,
            "coordinator-experiment",
            root_authority,
            Capability::RegisterDeterministicExperiment,
            generation,
            CommandBody::RegisterDeterministicExperiment {
                operating_cycle_id: cycle,
                project_id: project,
                ticket_id: TicketId::new(1).unwrap(),
                target_graph_revision_id: graph_revision,
                evaluator_content_object_id: evaluator.content_object_id,
                input_manifest_content_object_id: input.content_object_id,
            },
        );
        let epoch_identity = SupervisorEpochIdentity::parse("coordinator-epoch").unwrap();
        accepted(
            &mut kernel,
            "coordinator-epoch",
            PrincipalId::KERNEL,
            Capability::OpenSupervisorEpoch,
            society_kernel::ExpectedGeneration::NotApplicable,
            CommandBody::OpenSupervisorEpoch {
                supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
                supervisor_epoch_identity: epoch_identity.clone(),
            },
        );
        let workspace_root_path = parent.join("workspaces");
        fs::create_dir(&workspace_root_path).unwrap();
        fs::set_permissions(&workspace_root_path, fs::Permissions::from_mode(0o700)).unwrap();
        let workspace_root = NativeWorkspaceRoot::open_owned(&workspace_root_path).unwrap();
        let workspace_identity = NativeWorkspaceId::parse("coordinator-workspace").unwrap();
        let workspace_directory = workspace_root
            .directory()
            .as_path()
            .join(workspace_identity.as_str());
        assert!(
            !workspace_directory.exists(),
            "the prospective workspace must not exist before the durable claim"
        );
        let mut coordinator = DeterministicEvaluatorCoordinator::default();
        let mut child = match coordinator
            .claim_materialize_and_spawn(
                &mut kernel,
                &authority,
                DeterministicEvaluatorScheduleStart::new(
                    workspace_root.clone(),
                    workspace_identity.clone(),
                    SupervisorEpochId::new(1).unwrap(),
                    epoch_identity,
                ),
            )
            .unwrap()
        {
            DeterministicEvaluatorScheduleOutcome::Spawned(child) => child,
            other => panic!("expected one kernel-authorized evaluator spawn, got {other:?}"),
        };
        assert!(
            workspace_directory.is_dir(),
            "only the accepted claim may allocate its exact bound workspace"
        );
        assert_eq!(
            fs::read(workspace_directory.join(MATERIALIZED_EVALUATOR_FILE)).unwrap(),
            evaluator_bytes
        );
        assert_eq!(
            fs::read(workspace_directory.join(MATERIALIZED_INPUT_MANIFEST_FILE)).unwrap(),
            input_bytes
        );

        let deadline = NativeChildDeadline::new(
            MonotonicTick::from_milliseconds(5_000),
            MonotonicTick::from_milliseconds(10_000),
        );
        let mut observed_awaiting_owned_group_absence = false;
        let completion = (0..1_000)
            .find_map(|tick| {
                coordinator
                    .drive_at(
                        &mut kernel,
                        &mut child,
                        MonotonicTick::from_milliseconds(tick),
                        deadline,
                    )
                    .unwrap();
                match coordinator
                    .reconcile_at(
                        &mut kernel,
                        &authority,
                        &mut child,
                        MonotonicTick::from_milliseconds(tick),
                    )
                    .unwrap()
                {
                    DeterministicEvaluatorReconciliation::Completed(completion) => Some(completion),
                    DeterministicEvaluatorReconciliation::StillRunning => {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        None
                    }
                    DeterministicEvaluatorReconciliation::AwaitingOwnedGroupAbsence => {
                        observed_awaiting_owned_group_absence = true;
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        None
                    }
                    DeterministicEvaluatorReconciliation::ContainmentBlocked => {
                        panic!("owned descendant must not leave an inaccessible group")
                    }
                }
            })
            .expect("provider-free direct evaluator must complete under the fixed deadline");

        assert!(
            observed_awaiting_owned_group_absence,
            "the owned descendant must remain between direct reap and later absence"
        );
        let signal_command = CommandId::parse(format!(
            "{CHILD_COMMAND_PREFIX}{}/{}",
            child.native_child_spawn_admission_id.value(),
            DeterministicEvaluatorChildCommand::RecordSignal { ordinal: 0 },
        ))
        .unwrap();
        let CommandDisposition::Accepted(signal_event_id) = kernel
            .command_receipt(&signal_command)
            .unwrap()
            .unwrap()
            .disposition
        else {
            panic!("owned descendant must have one durable lingering cleanup signal")
        };
        assert!(matches!(
            kernel.ledger_event(signal_event_id).unwrap().body,
            EventBody::ProcessSignalReceiptRecorded {
                native_child_id,
                action: society_kernel::ProcessSignalAction::LingeringGroupKill,
                cause: society_kernel::ProcessSignalCause::AutomaticBoundaryContainment,
                ..
            } if native_child_id == child.native_child_id
        ));
        let liveness_command = CommandId::parse(format!(
            "{CHILD_COMMAND_PREFIX}{}/{}",
            child.native_child_spawn_admission_id.value(),
            DeterministicEvaluatorChildCommand::RecordPostCleanupLiveness,
        ))
        .unwrap();
        let CommandDisposition::Accepted(liveness_event_id) = kernel
            .command_receipt(&liveness_command)
            .unwrap()
            .unwrap()
            .disposition
        else {
            panic!("later owned-group absence must be durably recorded")
        };
        assert!(matches!(
            kernel.ledger_event(liveness_event_id).unwrap().body,
            EventBody::ChildProcessLivenessObserved {
                native_child_id,
                liveness: society_kernel::ProcessGroupLiveness::Absent,
                ..
            } if native_child_id == child.native_child_id
        ));

        assert_eq!(completion.stdout.digest, Blake3Digest::of_bytes(b""));
        assert_eq!(completion.stderr.digest, Blake3Digest::of_bytes(b""));
        assert_eq!(
            completion.stdout.content_object_id,
            completion.stderr.content_object_id
        );
        assert_eq!(completion.native_child_id, child.native_child_id);
        kernel.validate_replayed_materialized_state().unwrap();

        let no_eligible_identity = NativeWorkspaceId::parse("coordinator-no-eligible").unwrap();
        let no_eligible_directory = workspace_root
            .directory()
            .as_path()
            .join(no_eligible_identity.as_str());
        let workspace_entries_before_no_eligible = workspace_entry_count(&workspace_root_path);
        assert!(matches!(
            coordinator
                .claim_materialize_and_spawn(
                    &mut kernel,
                    &authority,
                    DeterministicEvaluatorScheduleStart::new(
                        workspace_root.clone(),
                        no_eligible_identity,
                        SupervisorEpochId::new(1).unwrap(),
                        SupervisorEpochIdentity::parse("coordinator-epoch").unwrap(),
                    ),
                )
                .unwrap(),
            DeterministicEvaluatorScheduleOutcome::NoEligibleExperiment
        ));
        assert!(
            !no_eligible_directory.exists()
                && workspace_entry_count(&workspace_root_path)
                    == workspace_entries_before_no_eligible,
            "a no-eligible claim must not allocate its prospective workspace"
        );

        let workspace_entries_before_already_claimed = workspace_entry_count(&workspace_root_path);
        assert!(matches!(
            coordinator
                .claim_materialize_and_spawn(
                    &mut kernel,
                    &authority,
                    DeterministicEvaluatorScheduleStart::new(
                        workspace_root,
                        workspace_identity,
                        SupervisorEpochId::new(1).unwrap(),
                        SupervisorEpochIdentity::parse("coordinator-epoch").unwrap(),
                    ),
                )
                .unwrap(),
            DeterministicEvaluatorScheduleOutcome::AlreadyClaimed { .. }
        ));
        assert_eq!(
            workspace_entry_count(&workspace_root_path),
            workspace_entries_before_already_claimed,
            "an already-claimed operation must not allocate again"
        );

        drop(authority);
        drop(kernel);
        fs::remove_dir_all(parent).unwrap();
    }

    fn artifact(bytes: &[u8]) -> (VerifiedArtifact, std::path::PathBuf) {
        let unique = NEXT_ARTIFACT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("societyd-evaluator-artifact-{unique}"));
        fs::write(&path, bytes).unwrap();
        let digest = society_pi::Blake3Digest::parse(blake3::hash(bytes).to_string()).unwrap();
        let artifact = VerifiedArtifact::inspect(&path, digest).unwrap();
        (artifact, path)
    }

    fn delayed_descendant_evaluator(parent: &std::path::Path) -> Vec<u8> {
        let source = parent.join("delayed-descendant-evaluator.c");
        let executable = parent.join("delayed-descendant-evaluator");
        fs::write(
            &source,
            r#"
#include <sys/types.h>
#include <unistd.h>

int main(void) {
    int ready[2];
    if (pipe(ready) != 0) return 2;
    pid_t descendant = fork();
    if (descendant < 0) return 3;
    if (descendant == 0) {
        close(ready[0]);
        if (write(ready[1], "r", 1) != 1) _exit(4);
        close(ready[1]);
        usleep(5000000);
        _exit(0);
    }
    close(ready[1]);
    char byte;
    if (read(ready[0], &byte, 1) != 1) return 5;
    close(ready[0]);
    return 0;
}
"#,
        )
        .unwrap();
        let output = Command::new("cc")
            .arg("-O0")
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "C fixture compiler failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bytes = fs::read(&executable).unwrap();
        fs::remove_file(source).unwrap();
        fs::remove_file(executable).unwrap();
        bytes
    }

    fn workspace_entry_count(root: &std::path::Path) -> usize {
        fs::read_dir(root).unwrap().count()
    }

    fn temporary_parent(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let parent = std::env::temp_dir().join(format!(
            "societyd-deterministic-evaluator-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        parent
    }

    fn accepted(
        kernel: &mut KernelStore,
        command_id: &str,
        principal_id: PrincipalId,
        capability: Capability,
        expected_generation: society_kernel::ExpectedGeneration,
        body: CommandBody,
    ) {
        let capability_grant_id = kernel
            .active_capability_grant(principal_id, capability)
            .unwrap()
            .unwrap();
        let receipt = kernel
            .execute(CommandRequest {
                command_id: CommandId::parse(command_id).unwrap(),
                principal_id,
                capability_grant_id,
                capability,
                expected_generation,
                body,
            })
            .unwrap();
        assert!(
            matches!(receipt.disposition, CommandDisposition::Accepted(_)),
            "{command_id}: {receipt:?}"
        );
    }

    fn founded_evaluator_cycle(
        kernel: &mut KernelStore,
        mission_digest: Blake3Digest,
    ) -> (PrincipalId, OperatingCycleId) {
        let bootstrap = PrincipalId::BOOTSTRAP;
        accepted(
            kernel,
            "coordinator-society",
            bootstrap,
            Capability::CreateSocietyIdentity,
            society_kernel::ExpectedGeneration::NotApplicable,
            CommandBody::CreateSocietyIdentity {
                name: SocietyName::parse("Coordinator test society").unwrap(),
            },
        );
        accepted(
            kernel,
            "coordinator-mission",
            bootstrap,
            Capability::InstallFoundingMission,
            society_kernel::ExpectedGeneration::NotApplicable,
            CommandBody::InstallFoundingMission {
                mission: ApplicationMissionInput {
                    application_identity: ApplicationIdentity::parse("coordinator-test").unwrap(),
                    application_name: ApplicationName::parse("Coordinator Test").unwrap(),
                    revision_ordinal: ApplicationRevisionOrdinal::new(1).unwrap(),
                    statement: MissionStatement::parse(
                        "Prove one generic private evaluator custody chain.",
                    )
                    .unwrap(),
                    principles: MissionPrinciples::new(vec![MissionPrinciple {
                        kind: MissionPrincipleKind::Purpose,
                        text: MissionPrincipleText::parse(
                            "Keep evaluator process custody bounded and legible.",
                        )
                        .unwrap(),
                    }])
                    .unwrap(),
                    north_star_questions: NorthStarQuestionSet {
                        change: NorthStarChangeQuestion::parse("What changed?").unwrap(),
                        improvement_evidence: NorthStarImprovementEvidenceQuestion::parse(
                            "What receipt proves custody?",
                        )
                        .unwrap(),
                        boundary_commitment: NorthStarBoundaryCommitmentQuestion::parse(
                            "Which authority remains closed?",
                        )
                        .unwrap(),
                        revisit: NorthStarRevisitQuestion::parse("When is replay checked?")
                            .unwrap(),
                    },
                    source_rendering_digest: mission_digest,
                },
            },
        );
        accepted(
            kernel,
            "coordinator-office",
            bootstrap,
            Capability::InstallRootAuthorityOffice,
            society_kernel::ExpectedGeneration::NotApplicable,
            CommandBody::InstallRootAuthorityOffice,
        );
        accepted(
            kernel,
            "coordinator-root-authority",
            bootstrap,
            Capability::AppointInitialRootAuthority,
            society_kernel::ExpectedGeneration::NotApplicable,
            CommandBody::AppointInitialRootAuthority {
                actor_display_name: PrincipalDisplayName::parse("Coordinator Root").unwrap(),
            },
        );
        accepted(
            kernel,
            "coordinator-ceiling",
            bootstrap,
            Capability::SetR0HardCeiling,
            society_kernel::ExpectedGeneration::NotApplicable,
            CommandBody::SetR0HardCeiling {
                ceiling: UsdMicros::new(1_000_000).unwrap(),
            },
        );
        accepted(
            kernel,
            "coordinator-bootstrap",
            bootstrap,
            Capability::BootstrapSociety,
            society_kernel::ExpectedGeneration::NotApplicable,
            CommandBody::BootstrapSociety,
        );
        accepted(
            kernel,
            "coordinator-propose",
            bootstrap,
            Capability::ProposeOperatingCycle,
            society_kernel::ExpectedGeneration::NotApplicable,
            CommandBody::ProposeOperatingCycle {
                treatment: OperatingCycleTreatment::DeterministicEvaluatorFixtureV1,
                budget_ceiling: UsdMicros::new(500_000).unwrap(),
            },
        );
        let cycle = OperatingCycleId::new(1).unwrap();
        accepted(
            kernel,
            "coordinator-admit",
            bootstrap,
            Capability::AdmitOperatingCycle,
            society_kernel::ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
            CommandBody::AdmitOperatingCycle { cycle_id: cycle },
        );
        (PrincipalId::new(3).unwrap(), cycle)
    }

    fn active_project(
        kernel: &mut KernelStore,
        root_authority: PrincipalId,
        cycle: OperatingCycleId,
    ) -> ProjectId {
        let generation = society_kernel::ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
        accepted(
            kernel,
            "coordinator-office-session",
            root_authority,
            Capability::StartRootAuthorityOfficeSession,
            generation,
            CommandBody::StartRootAuthorityOfficeSession { cycle_id: cycle },
        );
        accepted(
            kernel,
            "coordinator-project",
            root_authority,
            Capability::CreateProject,
            generation,
            CommandBody::CreateProject {
                operating_cycle_id: cycle,
                project_name: ProjectName::parse("Coordinator project").unwrap(),
                north_star_alignment: ProjectNorthStarAlignment {
                    application_revision_id: ApplicationRevisionId::new(1).unwrap(),
                    change_answer: ProjectNorthStarChangeAnswer::parse(
                        "Run one sealed direct evaluator.",
                    )
                    .unwrap(),
                    improvement_evidence_answer: ProjectNorthStarImprovementEvidenceAnswer::parse(
                        "Native receipts are replay-valid.",
                    )
                    .unwrap(),
                    boundary_commitment_answer: ProjectNorthStarBoundaryCommitmentAnswer::parse(
                        "No evaluator result becomes semantic evidence here.",
                    )
                    .unwrap(),
                    revisit_answer: ProjectNorthStarRevisitAnswer::parse(
                        "After the process is reaped.",
                    )
                    .unwrap(),
                },
            },
        );
        let project = ProjectId::new(1).unwrap();
        accepted(
            kernel,
            "coordinator-project-challenged",
            root_authority,
            Capability::TransitionProject,
            generation,
            CommandBody::TransitionProject {
                operating_cycle_id: cycle,
                project_id: project,
                target: ProjectState::Challenged,
            },
        );
        accepted(
            kernel,
            "coordinator-project-charter",
            root_authority,
            Capability::CharterProject,
            generation,
            CommandBody::CharterProject {
                operating_cycle_id: cycle,
                project_id: project,
                objective: ProjectObjectiveText::parse("Exercise direct evaluator custody.")
                    .unwrap(),
                initial_milestone: ProjectMilestoneName::parse("Seal native output").unwrap(),
                stop_condition: ProjectStopConditionText::parse("The receipt chain closes.")
                    .unwrap(),
            },
        );
        accepted(
            kernel,
            "coordinator-project-active",
            root_authority,
            Capability::TransitionProject,
            generation,
            CommandBody::TransitionProject {
                operating_cycle_id: cycle,
                project_id: project,
                target: ProjectState::Active,
            },
        );
        project
    }
}
