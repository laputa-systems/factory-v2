use std::path::Path;

use rusqlite::{
    Connection, OptionalExtension, Transaction, TransactionBehavior, params,
    types::{FromSql, ValueRef},
};
use thiserror::Error;

use crate::{
    ActorAttemptCancellationReason, ActorAttemptId, ActorAttemptState, ActorAttemptTerminalKind,
    ActorConfigurationId, ActorConfigurationRevisionId, ActorInstanceId, ActorInstanceState,
    ActorModelPolicy, AdmissionGeneration, AdversarialReviewId, AdversarialReviewState,
    ApplicationId, ApplicationIdentity, ApplicationMissionInput, ApplicationName,
    ApplicationRevisionId, ApplicationRevisionOrdinal, Blake3Digest, BudgetEnvelopeId,
    BudgetFreezeReason, BudgetReservationId, BudgetReservationState, CancellationMode,
    CancellationPropagationId, CancellationPropagationState,
    CancellationPropagationTargetDisposition, CancellationRequestId, CancellationState,
    CanonicalPiSessionTranscriptPath, CanonicalWorkspacePath, Capability, CausalEpisodeId,
    ChildProcessState, ChildRecoveryObservation, ChildStreamKind, ChildStreamSealCompleteness,
    ChildTerminalDisposition, CommandBody, CommandDisposition, CommandId, CommandKind,
    CommandReceipt, CommandRequest, ContentMediaSchemaContract, ContentObjectId,
    ContentSealReceiptId, ContextPackId, ContextPackPurpose, CostObservation, CostPostmortemCause,
    CostPostmortemId, CostPostmortemResolution, CostPostmortemState, CostUnavailableReason,
    CostUnknownReason, DeterministicEvaluationReceiptId, DeterministicExperimentId,
    DeterministicExperimentState, DevelopmentalAttractor, DirectChildWaitStatus, EpisodeState,
    EvaluatorRevisionId, EventBody, EventId, EventKind, EvidenceAdmissionId,
    EvidenceLimitationText, EvidenceSemanticRole, ExecutionProfileId, ExecutionProfileKind,
    ExecutionProfileReadiness, ExpectedGeneration, ForensicManifestCapturePolicy,
    ForensicManifestId, FoundingMissionId, GraphEdgeId, GraphEdgeKind, GraphObjectId,
    GraphObjectKind, GraphRevisionBody, GraphRevisionId, GraphRevisionState,
    HypothesisRevisionText, InputManifestId, LedgerEvent, MissionPrinciple, MissionPrincipleKind,
    MissionPrincipleText, MissionPrinciples, MissionStatement, NativeChildId,
    NativeChildLivenessObservationId, NativeChildNotSpawnedReason, NativeChildOwner,
    NativeChildPid, NativeChildReapReceiptId, NativeChildRecoveryReceiptId,
    NativeChildSpawnAdmissionId, NativeChildSpawnAdmissionState, NativeChildStreamSealId,
    NativeWorkspaceId, NorthStarBoundaryCommitmentQuestion, NorthStarChangeQuestion,
    NorthStarImprovementEvidenceQuestion, NorthStarQuestionSet, NorthStarRevisitQuestion,
    ObservationRevisionText, OfficeId, OfficeKind, OfficeOccupancyId, OfficeSessionState,
    OfficeSessionTerminalState, OfficeTurnId, OfficeTurnPurpose, OfficeTurnState, OperatingCycleId,
    OperatingCycleState, OperatingCycleTreatment, OutcomeObligationDisposition,
    OutcomeObligationId, OutcomeObligationState, OwnedProcessGroupId, PiAbortControlReceiptId,
    PiAbortControlWriteOutcome, PiBoundarySessionIdentity, PiChildOwner, PiChildSessionState,
    PiCorrelationIdentity, PiCumulativeUsage, PiOfficeSessionDisposeBudgetDisposition,
    PiOfficeSessionDisposeReceiptId, PiOfficeSessionFirstUserPromptReceipt,
    PiOfficeSessionTranscriptReceipt, PiOfficeTurnAssistantOutcome, PiOfficeTurnDisposition,
    PiOfficeTurnPromptAuthorizationId, PiOfficeTurnTerminalEvidence, PiOfficeTurnTerminalReceiptId,
    PiOfficeTurnTranscriptDisposition, PiOfficeTurnUsageFailure, PiOfficeTurnUsageFailureId,
    PiOfficeTurnUsageReceiptId, PiOfficeTurnUsageUnavailableReason, PiOfficeTurnUsageUnknownReason,
    PiProtocolSequence, PiSessionId, PiTokenCount, PostmortemActionKind,
    PostmortemActionProposalId, PostmortemCausalClaimId, PostmortemCausalClaimKind, PostmortemId,
    PostmortemState, PrincipalId, PrincipalKind, ProcessExitCode, ProcessGroupLiveness,
    ProcessSignalAction, ProcessSignalCause, ProcessSignalDelivery, ProcessSignalNumber,
    ProcessSignalReceiptId, ProjectId, ProjectMilestoneId, ProjectMilestoneState,
    ProjectNorthStarAlignment, ProjectNorthStarBoundaryCommitmentAnswer,
    ProjectNorthStarChangeAnswer, ProjectNorthStarImprovementEvidenceAnswer,
    ProjectNorthStarRevisitAnswer, ProjectState, ProviderCostBinary64, Rejection,
    RetentionAccessClass, ReviewChallengeId, ReviewChallengeResponseState, ReviewChallengeSeverity,
    ReviewDispositionKind, ReviewResolutionKind, RootAuthorityOfficeSessionId, SocietyId,
    SocietyName, SpawnNonce, SupervisedChildIdentity, SupervisorEpochId, SupervisorEpochIdentity,
    TicketId, TicketState, UsdMicros, WorkItemId, WorkItemKind, WorkItemState, WorkLeaseId,
    WorkLeaseState,
};

const CURRENT_SCHEMA: &str = include_str!("../../../migrations/0001_kernel.sql");
// Historical prototype schemas used versions one through thirteen. The collapsed
// fresh schema deliberately occupies a noncolliding identity, so an old
// ledger cannot be mistaken for current trusted physics.
const CURRENT_SCHEMA_VERSION: i64 = 15;

struct PiChildSpawnAdmissionInput<'a> {
    operating_cycle_id: OperatingCycleId,
    owner: PiChildOwner,
    budget_reservation_id: BudgetReservationId,
    execution_profile_id: ExecutionProfileId,
    native_workspace_id: &'a NativeWorkspaceId,
    canonical_workspace_path: &'a CanonicalWorkspacePath,
    supervisor_epoch_id: SupervisorEpochId,
    supervisor_epoch_identity: &'a SupervisorEpochIdentity,
    pi_session_identity: &'a PiBoundarySessionIdentity,
    spawn_nonce: &'a SpawnNonce,
}

struct ChildStreamSealInput {
    child_id: NativeChildId,
    stream: ChildStreamKind,
    full_digest: Blake3Digest,
    retained: ContentObjectId,
    completeness: ChildStreamSealCompleteness,
}

struct ProcessSignalReceiptInput {
    child_id: NativeChildId,
    action: ProcessSignalAction,
    delivery: ProcessSignalDelivery,
    liveness: ProcessGroupLiveness,
    cause: ProcessSignalCause,
}

struct PiAbortControlDeliveryInput<'a> {
    child_id: NativeChildId,
    propagation_id: CancellationPropagationId,
    correlation: &'a PiCorrelationIdentity,
    abort_digest: Blake3Digest,
    outcome: PiAbortControlWriteOutcome,
}

struct PiOfficeTurnPromptAuthorizationInput<'a> {
    expected_generation: ExpectedGeneration,
    office_turn_id: OfficeTurnId,
    correlation_identity: &'a PiCorrelationIdentity,
    prompt_content_object_id: ContentObjectId,
    prompt_digest: Blake3Digest,
    frontier_event_id: EventId,
}

struct PiOfficeTurnTerminalInput<'a> {
    office_turn_id: OfficeTurnId,
    correlation_identity: &'a PiCorrelationIdentity,
    terminal_evidence: PiOfficeTurnTerminalEvidence,
    settled_sequence: PiProtocolSequence,
    disposition: PiOfficeTurnDisposition,
    assistant_outcome: PiOfficeTurnAssistantOutcome,
    transcript_disposition: PiOfficeTurnTranscriptDisposition,
}

type StoredPiChildAdmissionCommand = (
    i64,
    Option<i64>,
    Option<i64>,
    i64,
    i64,
    String,
    String,
    i64,
    String,
    String,
    String,
);

type PiOfficeTurnUsageSqlRow = (i64, i64, i64, i64, i64, Vec<u8>, i64, i64);
type PiOfficeTurnSettlementSqlRow = (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64);
type PiOfficeSessionDisposeBindingSqlRow = (i64, i64, i64, i64, i64, i64, i64);
type PiOfficeSessionDisposeTerminalSqlRow = (
    i64,
    i64,
    String,
    i64,
    i64,
    i64,
    i64,
    i64,
    i64,
    i64,
    Vec<u8>,
    i64,
    i64,
    i64,
    i64,
);
type PiOfficeSessionTranscriptReceiptSqlValues = (
    i64,
    String,
    Option<Vec<u8>>,
    Option<i64>,
    Option<i64>,
    Option<Vec<u8>>,
);
type PiOfficeSessionDisposedCommandSqlRow = (
    i64,
    String,
    i64,
    i64,
    String,
    Option<Vec<u8>>,
    Option<i64>,
    Option<i64>,
    Option<Vec<u8>>,
);

const COMMAND_BODY_TABLES: [&str; 100] = [
    "command_create_society_identity",
    "command_install_root_authority_office",
    "command_install_founding_mission",
    "command_appoint_initial_root_authority",
    "command_set_r0_hard_ceiling",
    "command_bootstrap_society",
    "command_propose_operating_cycle",
    "command_admit_operating_cycle",
    "command_start_root_authority_office_session",
    "command_record_office_session_ready",
    "command_open_office_turn",
    "command_settle_office_turn",
    "command_quiesce_operating_cycle",
    "command_record_cycle_drained",
    "command_resume_operating_cycle",
    "command_reconcile_operating_cycle",
    "command_close_operating_cycle",
    "command_reserve_budget",
    "command_reconcile_budget",
    "command_request_cancellation",
    "command_reconcile_cancellation",
    "command_record_office_session_terminal",
    "command_close_cost_postmortem",
    "command_create_project",
    "command_charter_project",
    "command_transition_project",
    "command_complete_project_milestone",
    "command_reopen_project",
    "command_create_ticket",
    "command_transition_ticket",
    "command_add_graph_object_revision",
    "command_commit_graph_revision",
    "command_add_graph_edge",
    "command_create_episode",
    "command_transition_episode",
    "command_reopen_episode",
    "command_request_adversarial_review",
    "command_submit_review_challenge",
    "command_respond_to_review_challenge",
    "command_disposition_review_challenge",
    "command_resolve_adversarial_review",
    "command_trigger_postmortem",
    "command_record_postmortem_causal_claim",
    "command_propose_postmortem_action",
    "command_close_postmortem",
    "command_assign_adversarial_reviewer",
    "command_register_actor_configuration",
    "command_register_context_pack",
    "command_admit_actor_instance",
    "command_admit_ticket",
    "command_register_work_item",
    "command_claim_work_item",
    "command_start_actor_attempt",
    "command_attest_actor_attempt_terminal",
    "command_validate_ticket_attempt",
    "command_retry_actor_attempt",
    "command_complete_ticket",
    "command_expire_work_lease",
    "command_cancel_actor_attempt",
    "command_register_outcome_obligation",
    "command_resolve_outcome_obligation",
    "command_record_content_seal_receipt",
    "command_register_content_object",
    "command_register_forensic_manifest",
    "command_register_deterministic_experiment",
    "command_record_deterministic_evaluation_receipt",
    "command_admit_deterministic_evidence",
    "command_finalize_deterministic_experiment",
    "command_admit_pi_child_spawn",
    "command_record_inert_pi_child_spawn",
    "command_record_pi_adapter_ready",
    "command_authorize_pi_create_session",
    "command_record_pi_create_session_delivery",
    "command_record_pi_session_ready",
    "command_record_child_stream_seal",
    "command_record_child_process_liveness",
    "command_record_process_signal_receipt",
    "command_record_direct_child_reap",
    "command_record_child_recovery",
    "command_finalize_child_process",
    "command_begin_cancellation_propagation",
    "command_reconcile_cancellation_propagation",
    "command_open_supervisor_epoch",
    "command_record_pi_abort_control_delivery",
    "command_record_native_child_not_spawned",
    "command_authorize_pi_office_turn_prompt",
    "command_record_pi_office_turn_prompt_delivery",
    "command_record_pi_office_turn_prompt_accepted",
    "command_record_pi_office_turn_usage",
    "command_record_pi_office_turn_usage_failure",
    "command_record_pi_office_turn_terminal",
    "command_authorize_pi_office_session_dispose",
    "command_record_pi_office_session_dispose_delivery",
    "command_record_pi_office_session_dispose_accepted",
    "command_record_pi_office_session_dispose_usage",
    "command_record_pi_office_session_dispose_usage_failure",
    "command_record_pi_office_session_disposed",
    "command_admit_deterministic_evaluator_native_child",
    "command_record_deterministic_evaluator_native_child_spawn",
    "command_register_deterministic_evaluator_forensic_manifest",
];

const EVENT_BODY_TABLES: [&str; 94] = [
    "event_society_identity_created",
    "event_root_authority_office_installed",
    "event_founding_mission_installed",
    "event_root_authority_appointed",
    "event_r0_hard_ceiling_set",
    "event_society_bootstrapped",
    "event_operating_cycle_proposed",
    "event_operating_cycle_state_changed",
    "event_root_authority_office_session_started",
    "event_root_authority_office_session_state_changed",
    "event_office_turn_opened",
    "event_office_turn_settled",
    "event_budget_reserved",
    "event_budget_reconciled",
    "event_budget_admission_frozen",
    "event_cancellation_requested",
    "event_cancellation_reconciled",
    "event_cost_postmortem_closed",
    "event_project_created",
    "event_project_chartered",
    "event_project_state_changed",
    "event_project_milestone_completed",
    "event_ticket_created",
    "event_ticket_state_changed",
    "event_graph_object_revision_added",
    "event_graph_revision_committed",
    "event_graph_edge_added",
    "event_episode_created",
    "event_episode_state_changed",
    "event_adversarial_review_requested",
    "event_review_challenge_submitted",
    "event_review_challenge_responded",
    "event_review_challenge_dispositioned",
    "event_adversarial_review_resolved",
    "event_postmortem_triggered",
    "event_postmortem_causal_claim_recorded",
    "event_postmortem_action_proposed",
    "event_postmortem_closed",
    "event_adversarial_reviewer_assigned",
    "event_actor_configuration_registered",
    "event_context_pack_registered",
    "event_actor_instance_admitted",
    "event_ticket_admitted",
    "event_work_item_registered",
    "event_work_item_claimed",
    "event_actor_attempt_started",
    "event_actor_attempt_terminal_attested",
    "event_ticket_attempt_validated",
    "event_actor_attempt_retry_prepared",
    "event_ticket_completed",
    "event_work_lease_expired",
    "event_actor_attempt_cancellation_requested",
    "event_outcome_obligation_registered",
    "event_outcome_obligation_resolved",
    "event_content_seal_receipt_recorded",
    "event_content_object_registered",
    "event_forensic_manifest_registered",
    "event_deterministic_experiment_registered",
    "event_deterministic_evaluation_receipt_recorded",
    "event_deterministic_evidence_admitted",
    "event_deterministic_experiment_finalized",
    "event_pi_child_spawn_admitted",
    "event_inert_pi_child_spawn_recorded",
    "event_pi_adapter_ready_recorded",
    "event_pi_create_session_authorized",
    "event_pi_create_session_delivery_recorded",
    "event_pi_session_ready_recorded",
    "event_child_stream_sealed",
    "event_child_process_liveness_observed",
    "event_process_signal_receipt_recorded",
    "event_direct_child_reaped",
    "event_child_recovery_observed",
    "event_child_process_finalized",
    "event_cancellation_propagation_begun",
    "event_cancellation_propagation_reconciled",
    "event_supervisor_epoch_opened",
    "event_cancellation_propagation_containment_failed",
    "event_pi_abort_control_delivery_recorded",
    "event_native_child_spawn_invalidated",
    "event_pi_office_turn_prompt_authorized",
    "event_pi_office_turn_prompt_delivered",
    "event_pi_office_turn_prompt_accepted",
    "event_pi_office_turn_usage_recorded",
    "event_pi_office_turn_usage_frozen",
    "event_pi_office_turn_terminal_recorded",
    "event_pi_office_session_dispose_authorized",
    "event_pi_office_session_dispose_delivered",
    "event_pi_office_session_dispose_accepted",
    "event_pi_office_session_dispose_usage_recorded",
    "event_pi_office_session_dispose_usage_frozen",
    "event_pi_office_session_disposed",
    "event_deterministic_evaluator_native_child_admitted",
    "event_deterministic_evaluator_native_child_spawn_recorded",
    "event_deterministic_evaluator_forensic_manifest_registered",
];

const GRAPH_REVISION_BODY_TABLES: [&str; 2] = ["observation_revisions", "hypothesis_revisions"];

/// The SQLite implementation of trusted physics. `societyd` will be its only
/// production owner; this crate deliberately accepts an already-opened local
/// connection only through its own constructors so schema bootstrap and foreign-key
/// enforcement cannot be skipped accidentally.
pub struct KernelStore {
    connection: Connection,
}

type DeterministicEvaluatorAdmissionSqlRow = (
    i64,
    i64,
    i64,
    i64,
    i64,
    String,
    String,
    i64,
    String,
    i64,
    Vec<u8>,
    Vec<u8>,
    i64,
    i64,
);

type DeterministicEvaluatorSpawnAdmissionSqlRow = (
    i64,
    i64,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
);

type DeterministicEvaluatorScheduleClaimSqlRow = (
    i64,
    Option<i64>,
    i64,
    i64,
    i64,
    Option<String>,
    Option<String>,
    Option<i64>,
    Option<String>,
    Option<i64>,
);

/// Exact daemon-private execution binding for an admitted deterministic
/// evaluator child. The digests are resolved from sealed content identities,
/// so callers do not re-supply mutable evaluator/input tuples.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicEvaluatorNativeChildAdmission {
    native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    operating_cycle_id: OperatingCycleId,
    deterministic_experiment_id: DeterministicExperimentId,
    evaluator_revision_id: EvaluatorRevisionId,
    input_manifest_id: InputManifestId,
    execution_profile_id: ExecutionProfileId,
    native_workspace_id: NativeWorkspaceId,
    canonical_workspace_path: CanonicalWorkspacePath,
    supervisor_epoch_id: SupervisorEpochId,
    supervisor_epoch_identity: SupervisorEpochIdentity,
    admission_generation: AdmissionGeneration,
    evaluator_digest: Blake3Digest,
    input_manifest_digest: Blake3Digest,
    evaluator_content_object_id: ContentObjectId,
    input_manifest_content_object_id: ContentObjectId,
}

impl DeterministicEvaluatorNativeChildAdmission {
    pub const fn native_child_spawn_admission_id(&self) -> NativeChildSpawnAdmissionId {
        self.native_child_spawn_admission_id
    }
    pub const fn operating_cycle_id(&self) -> OperatingCycleId {
        self.operating_cycle_id
    }
    pub const fn deterministic_experiment_id(&self) -> DeterministicExperimentId {
        self.deterministic_experiment_id
    }
    pub const fn evaluator_revision_id(&self) -> EvaluatorRevisionId {
        self.evaluator_revision_id
    }
    pub const fn input_manifest_id(&self) -> InputManifestId {
        self.input_manifest_id
    }
    pub const fn execution_profile_id(&self) -> ExecutionProfileId {
        self.execution_profile_id
    }
    pub fn native_workspace_id(&self) -> &NativeWorkspaceId {
        &self.native_workspace_id
    }
    pub fn canonical_workspace_path(&self) -> &CanonicalWorkspacePath {
        &self.canonical_workspace_path
    }
    pub const fn supervisor_epoch_id(&self) -> SupervisorEpochId {
        self.supervisor_epoch_id
    }
    pub fn supervisor_epoch_identity(&self) -> &SupervisorEpochIdentity {
        &self.supervisor_epoch_identity
    }
    pub const fn admission_generation(&self) -> AdmissionGeneration {
        self.admission_generation
    }
    pub const fn evaluator_digest(&self) -> Blake3Digest {
        self.evaluator_digest
    }
    pub const fn input_manifest_digest(&self) -> Blake3Digest {
        self.input_manifest_digest
    }
    pub const fn evaluator_content_object_id(&self) -> ContentObjectId {
        self.evaluator_content_object_id
    }
    pub const fn input_manifest_content_object_id(&self) -> ContentObjectId {
        self.input_manifest_content_object_id
    }
}

/// One idempotent resident scheduler operation. It does not carry an
/// experiment, evaluator, input, output, or executable identity: the kernel
/// chooses the oldest currently eligible registered experiment and derives
/// its fixture execution profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicEvaluatorScheduleClaimRequest {
    command_id: CommandId,
    native_workspace_id: NativeWorkspaceId,
    canonical_workspace_path: CanonicalWorkspacePath,
    supervisor_epoch_id: SupervisorEpochId,
    supervisor_epoch_identity: SupervisorEpochIdentity,
}

impl DeterministicEvaluatorScheduleClaimRequest {
    pub fn new(
        command_id: CommandId,
        native_workspace_id: NativeWorkspaceId,
        canonical_workspace_path: CanonicalWorkspacePath,
        supervisor_epoch_id: SupervisorEpochId,
        supervisor_epoch_identity: SupervisorEpochIdentity,
    ) -> Self {
        Self {
            command_id,
            native_workspace_id,
            canonical_workspace_path,
            supervisor_epoch_id,
            supervisor_epoch_identity,
        }
    }
}

/// The exact durable result of a scheduler claim. `admission` remains the
/// post-claim verification query used by native custody; the two content
/// identities let the daemon materialize only the sealed evaluator and input
/// bytes that this claim resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeterministicEvaluatorScheduleClaim {
    /// The child has not been spawned or invalidated and may be passed to the
    /// daemon's exact materialization/spawn custody bridge.
    SpawnAuthorized(Box<DeterministicEvaluatorNativeChildAdmission>),
    /// The same idempotent operation already made a durable admission, but it
    /// is no longer spawn-authoritative (it was spawned or invalidated). This
    /// is normal retry information, never evidence of ledger corruption.
    AlreadyClaimed {
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    },
}

impl DeterministicEvaluatorScheduleClaim {
    pub fn admission(&self) -> Option<&DeterministicEvaluatorNativeChildAdmission> {
        match self {
            Self::SpawnAuthorized(admission) => Some(admission),
            Self::AlreadyClaimed { .. } => None,
        }
    }

    pub const fn native_child_spawn_admission_id(&self) -> NativeChildSpawnAdmissionId {
        match self {
            Self::SpawnAuthorized(admission) => admission.native_child_spawn_admission_id(),
            Self::AlreadyClaimed {
                native_child_spawn_admission_id,
            } => *native_child_spawn_admission_id,
        }
    }

    pub const fn evaluator_content_object_id(&self) -> Option<ContentObjectId> {
        match self {
            Self::SpawnAuthorized(admission) => Some(admission.evaluator_content_object_id()),
            Self::AlreadyClaimed { .. } => None,
        }
    }

    pub const fn input_manifest_content_object_id(&self) -> Option<ContentObjectId> {
        match self {
            Self::SpawnAuthorized(admission) => Some(admission.input_manifest_content_object_id()),
            Self::AlreadyClaimed { .. } => None,
        }
    }
}

/// The only durable recovery states for one physical BLAKE3 identity.
///
/// A content object cannot exist without the receipt that attests its physical
/// seal, so this closed result makes that impossible state unrepresentable to
/// the daemon. It carries byte identity only; occurrence, schema, retention,
/// provenance, and evidence meaning remain outside this query.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentIdentityState {
    Absent,
    SealReceiptOnly {
        content_seal_receipt_id: ContentSealReceiptId,
    },
    Registered {
        content_seal_receipt_id: ContentSealReceiptId,
        content_object_id: ContentObjectId,
    },
}

/// The only outcomes of a side-effect-free founding-mission preflight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallFoundingMissionPreflight {
    /// A new request may proceed to physical sealing and object registration.
    Ready,
    /// The command already has a durable result; no physical seal is needed.
    ExistingReceipt(CommandReceipt),
    /// The daemon must call `execute` with its original request so the
    /// rejection and typed body become durable operational history.
    RejectionRequiresExecution(Rejection),
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error("database has unsupported schema version {0}")]
    UnsupportedSchemaVersion(i64),
    #[error("command id was already used with a different typed request")]
    IdempotencyConflict,
    #[error("deterministic evaluator schedule claim rejected: {0:?}")]
    DeterministicEvaluatorScheduleClaimRejected(Rejection),
    #[error("operating cycle {0:?} was not found")]
    OperatingCycleNotFound(OperatingCycleId),
    #[error("ledger event {0:?} was not found")]
    LedgerEventNotFound(EventId),
    #[error("ledger corruption: {0}")]
    LedgerCorruption(&'static str),
    #[error("stored integer does not represent a valid domain value")]
    InvalidStoredValue,
}

#[derive(Clone, Copy)]
struct CycleRow {
    society_id: SocietyId,
    mission_id: FoundingMissionId,
    occupancy_id: OfficeOccupancyId,
    _treatment: OperatingCycleTreatment,
    state: OperatingCycleState,
    generation: AdmissionGeneration,
}

/// The exact immutable assignment/context binding of a WorkItem. The tuple is
/// private to the store because callers must not fabricate an execution
/// context outside the typed command path.
type WorkItemRow = (
    TicketId,
    ActorInstanceId,
    ContextPackId,
    WorkItemKind,
    Option<AdversarialReviewId>,
    WorkItemState,
    Option<ActorAttemptId>,
);

enum CapabilityGrantLookup {
    Active {
        grant_id: i64,
        office_occupancy_id: Option<OfficeOccupancyId>,
        actor_instance_id: Option<ActorInstanceId>,
    },
    Inactive,
}

impl KernelStore {
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Self::from_connection(Connection::open(path)?)
    }

    fn from_connection(connection: Connection) -> Result<Self, StoreError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let schema_version: i64 =
            connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        match schema_version {
            0 => {
                // This prototype has one current, atomic schema bootstrap. It
                // does not claim historical-ledger compatibility; fresh creation
                // either commits the whole trusted schema or does not become a
                // current-version database.
                bootstrap_current_schema(&connection)?;
            }
            CURRENT_SCHEMA_VERSION => {}
            other => return Err(StoreError::UnsupportedSchemaVersion(other)),
        }
        let foreign_key_violations: i64 =
            connection.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })?;
        if foreign_key_violations != 0 {
            return Err(StoreError::LedgerCorruption(
                "current schema has foreign-key violations",
            ));
        }
        Ok(Self { connection })
    }

    /// Resolves one active, exact capability grant for a principal. Callers
    /// must carry this identity back in `CommandRequest`; the kernel will
    /// revalidate it transactionally at command acceptance.
    pub fn active_capability_grant(
        &self,
        principal_id: PrincipalId,
        capability: Capability,
    ) -> Result<Option<crate::CapabilityGrantId>, StoreError> {
        self.connection
            .query_row(
                "SELECT capability_grant_id FROM capability_grants
                 WHERE principal_id = ?1 AND capability_kind = ?2 AND grant_state = 1",
                params![principal_id.value(), capability as i64],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .map(crate::CapabilityGrantId::try_from)
            .transpose()
            .map_err(|_| StoreError::InvalidStoredValue)
    }

    pub fn deterministic_evaluator_native_child_admission(
        &self,
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    ) -> Result<Option<DeterministicEvaluatorNativeChildAdmission>, StoreError> {
        let exists: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM native_child_spawn_admissions WHERE native_child_spawn_admission_id = ?1)",
            [native_child_spawn_admission_id.value()],
            |row| row.get::<_, i64>(0),
        )? != 0;
        if !exists {
            return Ok(None);
        }
        let row: Option<DeterministicEvaluatorAdmissionSqlRow> = self.connection.query_row(
            "SELECT admission.operating_cycle_id, admission.deterministic_experiment_id,
                    admission.evaluator_revision_id, admission.input_manifest_id,
                    admission.execution_profile_id, workspace.native_workspace_id,
                    workspace.canonical_workspace_path, admission.supervisor_epoch_id,
                    epoch.supervisor_epoch_identity, admission.admission_generation,
                    evaluator_seal.digest, input_seal.digest
                    , evaluator.content_object_id, input.content_object_id
               FROM native_child_spawn_admissions admission
               JOIN workspaces workspace ON workspace.workspace_id = admission.workspace_id
               JOIN supervisor_epochs epoch ON epoch.supervisor_epoch_id = admission.supervisor_epoch_id
               JOIN operating_cycles cycle ON cycle.operating_cycle_id = admission.operating_cycle_id
               JOIN execution_profiles profile ON profile.execution_profile_id = admission.execution_profile_id
               JOIN evaluator_revisions evaluator ON evaluator.evaluator_revision_id = admission.evaluator_revision_id
               JOIN content_objects evaluator_object ON evaluator_object.content_object_id = evaluator.content_object_id
               JOIN content_seal_receipts evaluator_seal ON evaluator_seal.content_seal_receipt_id = evaluator_object.content_seal_receipt_id
               JOIN input_manifests input ON input.input_manifest_id = admission.input_manifest_id
               JOIN content_objects input_object ON input_object.content_object_id = input.content_object_id
               JOIN content_seal_receipts input_seal ON input_seal.content_seal_receipt_id = input_object.content_seal_receipt_id
              WHERE admission.native_child_spawn_admission_id = ?1
                AND admission.actor_attempt_id IS NULL AND admission.root_authority_office_session_id IS NULL
                AND admission.deterministic_experiment_id IS NOT NULL
                AND admission.evaluator_revision_id IS NOT NULL AND admission.input_manifest_id IS NOT NULL
                AND admission.budget_reservation_id IS NULL
                AND admission.lifecycle_state = 1
                AND cycle.treatment = 4 AND cycle.lifecycle_state = 3
                AND cycle.admission_generation = admission.admission_generation
                AND profile.profile_kind = 3 AND profile.readiness = 1
                AND NOT EXISTS(SELECT 1 FROM pi_child_spawn_sidecars sidecar WHERE sidecar.native_child_spawn_admission_id = admission.native_child_spawn_admission_id)
                AND EXISTS(SELECT 1 FROM deterministic_experiments experiment
                            WHERE experiment.deterministic_experiment_id = admission.deterministic_experiment_id
                              AND experiment.operating_cycle_id = admission.operating_cycle_id
                              AND experiment.evaluator_revision_id = admission.evaluator_revision_id
                              AND experiment.input_manifest_id = admission.input_manifest_id
                              AND experiment.lifecycle_state = 1)",
            [native_child_spawn_admission_id.value()],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?,r.get(12)?,r.get(13)?)),
        ).optional()?;
        let Some(row) = row else {
            // A structurally exact evaluator admission can legitimately stop
            // being spawn-authoritative after spawn, invalidation, a cycle
            // generation advance, or experiment finalization. That is not
            // ledger corruption; callers must simply not execute from it.
            let unavailable: bool = self.connection.query_row(
                "SELECT EXISTS(
                     SELECT 1 FROM native_child_spawn_admissions admission
                     JOIN deterministic_experiments experiment
                       ON experiment.deterministic_experiment_id = admission.deterministic_experiment_id
                      AND experiment.operating_cycle_id = admission.operating_cycle_id
                      AND experiment.evaluator_revision_id = admission.evaluator_revision_id
                      AND experiment.input_manifest_id = admission.input_manifest_id
                     WHERE admission.native_child_spawn_admission_id = ?1
                       AND admission.actor_attempt_id IS NULL
                       AND admission.root_authority_office_session_id IS NULL
                       AND admission.deterministic_experiment_id IS NOT NULL
                       AND admission.evaluator_revision_id IS NOT NULL
                       AND admission.input_manifest_id IS NOT NULL
                       AND admission.budget_reservation_id IS NULL
                       AND NOT EXISTS(SELECT 1 FROM pi_child_spawn_sidecars sidecar
                                      WHERE sidecar.native_child_spawn_admission_id = admission.native_child_spawn_admission_id)
                 )",
                [native_child_spawn_admission_id.value()],
                |row| row.get::<_, i64>(0),
            )? != 0;
            if unavailable {
                return Ok(None);
            }
            return Err(StoreError::LedgerCorruption(
                "evaluator native-child admission violates its exact execution binding",
            ));
        };
        Ok(Some(DeterministicEvaluatorNativeChildAdmission {
            native_child_spawn_admission_id,
            operating_cycle_id: OperatingCycleId::try_from(row.0)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            deterministic_experiment_id: DeterministicExperimentId::try_from(row.1)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            evaluator_revision_id: EvaluatorRevisionId::try_from(row.2)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            input_manifest_id: InputManifestId::try_from(row.3)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            execution_profile_id: ExecutionProfileId::try_from(row.4)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            native_workspace_id: NativeWorkspaceId::parse(row.5)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            canonical_workspace_path: CanonicalWorkspacePath::parse(row.6)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            supervisor_epoch_id: SupervisorEpochId::try_from(row.7)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            supervisor_epoch_identity: SupervisorEpochIdentity::parse(row.8)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            admission_generation: AdmissionGeneration::try_from(row.9)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            evaluator_digest: digest_from_stored_bytes(&row.10)?,
            input_manifest_digest: digest_from_stored_bytes(&row.11)?,
            evaluator_content_object_id: ContentObjectId::try_from(row.12)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            input_manifest_content_object_id: ContentObjectId::try_from(row.13)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        }))
    }

    /// Claims the oldest eligible registered deterministic experiment and
    /// records its existing native-child admission before the daemon may
    /// materialize a workspace or spawn. Registration is the sole scheduling
    /// authorization: this input deliberately has no application authority,
    /// evaluator/input identity, output identity, or executable path.
    pub fn claim_registered_deterministic_evaluator(
        &mut self,
        claim: DeterministicEvaluatorScheduleClaimRequest,
    ) -> Result<Option<DeterministicEvaluatorScheduleClaim>, StoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(admission_id) =
            existing_deterministic_evaluator_schedule_claim(&transaction, &claim)?
        {
            transaction.commit()?;
            return Ok(Some(
                self.deterministic_evaluator_native_child_admission(admission_id)?
                    .map(Box::new)
                    .map(DeterministicEvaluatorScheduleClaim::SpawnAuthorized)
                    .unwrap_or(DeterministicEvaluatorScheduleClaim::AlreadyClaimed {
                        native_child_spawn_admission_id: admission_id,
                    }),
            ));
        }

        let epoch_matches: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM supervisor_epochs
               WHERE supervisor_epoch_id = ?1 AND supervisor_epoch_identity = ?2)",
            params![
                claim.supervisor_epoch_id.value(),
                claim.supervisor_epoch_identity.as_str(),
            ],
            |row| row.get::<_, i64>(0),
        )? != 0;
        if !epoch_matches {
            return Err(StoreError::DeterministicEvaluatorScheduleClaimRejected(
                Rejection::ChildSpawnAdmissionInvalid,
            ));
        }

        let candidate: Option<(i64, i64, i64, i64, i64)> = transaction
            .query_row(
                "SELECT experiment.deterministic_experiment_id,
                        experiment.operating_cycle_id,
                        experiment.evaluator_revision_id,
                        experiment.input_manifest_id,
                        cycle.admission_generation
                   FROM deterministic_experiments experiment
                   JOIN operating_cycles cycle
                     ON cycle.operating_cycle_id = experiment.operating_cycle_id
                  WHERE experiment.lifecycle_state = ?1
                    AND cycle.lifecycle_state = ?2
                    AND cycle.treatment = ?3
                    AND NOT EXISTS(
                        SELECT 1 FROM native_child_spawn_admissions admission
                         WHERE admission.deterministic_experiment_id = experiment.deterministic_experiment_id
                    )
                  ORDER BY experiment.deterministic_experiment_id ASC
                  LIMIT 1",
                params![
                    DeterministicExperimentState::Registered as i64,
                    OperatingCycleState::Running as i64,
                    OperatingCycleTreatment::DeterministicEvaluatorFixtureV1 as i64,
                ],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()?;
        let Some((experiment, cycle, evaluator, input, generation)) = candidate else {
            transaction.commit()?;
            return Ok(None);
        };

        let capability_grant_id: i64 = transaction
            .query_row(
                "SELECT capability_grant_id FROM capability_grants
                  WHERE principal_id = ?1 AND capability_kind = ?2 AND grant_state = 1",
                params![
                    PrincipalId::KERNEL.value(),
                    Capability::AdmitDeterministicEvaluatorNativeChild as i64,
                ],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::LedgerCorruption(
                "kernel service lacks deterministic evaluator admission capability",
            ))?;
        let request = CommandRequest {
            command_id: claim.command_id.clone(),
            principal_id: PrincipalId::KERNEL,
            capability_grant_id: crate::CapabilityGrantId::try_from(capability_grant_id)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            capability: Capability::AdmitDeterministicEvaluatorNativeChild,
            expected_generation: ExpectedGeneration::Exact(
                AdmissionGeneration::try_from(generation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            ),
            body: CommandBody::AdmitDeterministicEvaluatorNativeChild {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_experiment_id: DeterministicExperimentId::try_from(experiment)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_revision_id: EvaluatorRevisionId::try_from(evaluator)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                input_manifest_id: InputManifestId::try_from(input)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                execution_profile_id:
                    ExecutionProfileId::DETERMINISTIC_EVALUATOR_PROCESS_FIXTURE_V1,
                native_workspace_id: claim.native_workspace_id.clone(),
                canonical_workspace_path: claim.canonical_workspace_path.clone(),
                supervisor_epoch_id: claim.supervisor_epoch_id,
                supervisor_epoch_identity: claim.supervisor_epoch_identity.clone(),
            },
        };
        let receipt = Self::execute_in_transaction(&transaction, request)?;
        let CommandDisposition::Accepted(_) = receipt.disposition else {
            transaction.commit()?;
            let CommandDisposition::Rejected(rejection) = receipt.disposition else {
                unreachable!("command dispositions are closed")
            };
            return Err(StoreError::DeterministicEvaluatorScheduleClaimRejected(
                rejection,
            ));
        };
        let admission_id: i64 = transaction.query_row(
            "SELECT native_child_spawn_admission_id FROM native_child_spawn_admissions
              WHERE deterministic_experiment_id = ?1",
            [experiment],
            |row| row.get(0),
        )?;
        let admission_id = NativeChildSpawnAdmissionId::try_from(admission_id)
            .map_err(|_| StoreError::InvalidStoredValue)?;
        transaction.commit()?;
        self.deterministic_evaluator_native_child_admission(admission_id)?
            .map(Box::new)
            .map(DeterministicEvaluatorScheduleClaim::SpawnAuthorized)
            .ok_or(StoreError::LedgerCorruption(
                "accepted evaluator schedule claim has no live admission",
            ))
            .map(Some)
    }

    /// Accepts a closed command exactly once. An equal duplicate returns its
    /// original receipt; a changed request using the same command identity is
    /// rejected before any state transition is reconsidered.
    pub fn execute(&mut self, request: CommandRequest) -> Result<CommandReceipt, StoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let receipt = Self::execute_in_transaction(&transaction, request)?;
        transaction.commit()?;
        Ok(receipt)
    }

    /// Runs the closed-command append inside an already-exclusive transaction.
    /// The resident scheduler uses this to select a durable queue member and
    /// record its existing native-child admission as one atomic transition.
    fn execute_in_transaction(
        transaction: &Transaction<'_>,
        request: CommandRequest,
    ) -> Result<CommandReceipt, StoreError> {
        let fingerprint = request_fingerprint(&request);

        if let Some((stored_fingerprint, status, event_id, rejection)) = transaction
            .query_row(
                "SELECT request_fingerprint, command_status, accepted_event_id, rejection_code
                 FROM commands WHERE command_id = ?1",
                [request.command_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                    ))
                },
            )
            .optional()?
        {
            if stored_fingerprint.as_slice() != fingerprint.as_bytes() {
                return Err(StoreError::IdempotencyConflict);
            }
            let receipt = match status {
                1 => CommandReceipt {
                    disposition: CommandDisposition::Accepted(
                        EventId::try_from(event_id.ok_or(StoreError::LedgerCorruption(
                            "accepted command has no event",
                        ))?)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                    ),
                    idempotent: true,
                },
                2 => CommandReceipt {
                    disposition: CommandDisposition::Rejected(rejection_from_i64(
                        rejection.ok_or(StoreError::LedgerCorruption(
                            "rejected command has no rejection code",
                        ))?,
                    )?),
                    idempotent: true,
                },
                _ => return Err(StoreError::LedgerCorruption("unknown command status")),
            };
            return Ok(receipt);
        }

        // A newly received command begins in a durable rejected placeholder
        // state. The savepoint below guarantees an unsuccessful transition
        // leaves its exact typed input visible while rolling back all material
        // state changes. Successful commands overwrite this placeholder in the
        // same enclosing transaction.
        transaction.execute(
            "INSERT INTO commands(command_id, principal_id, capability_grant_id, capability_kind, expected_generation,
                                  command_kind, request_fingerprint, command_status, rejection_code,
                                  accepted_event_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 2, ?8, NULL)",
            params![
                request.command_id.as_str(),
                request.principal_id.value(),
                request.capability_grant_id.value(),
                request.capability as i64,
                expected_generation_to_sql(request.expected_generation),
                request.body.kind() as i64,
                fingerprint.as_bytes().as_slice(),
                Rejection::SubjectNotFound as i64,
            ],
        )?;
        let command_row_id = transaction.last_insert_rowid();
        // Rejections also retain their exact typed request. A missing source
        // binding is therefore represented as NULL in that command material;
        // accepted installations must carry the resolved object below.
        let source_content_object_id = match &request.body {
            CommandBody::InstallFoundingMission { mission } => {
                mission_source_content_object_id(transaction, mission.source_rendering_digest).ok()
            }
            _ => None,
        };
        insert_command_body(
            transaction,
            command_row_id,
            &request.body,
            source_content_object_id,
        )?;

        transaction.execute_batch("SAVEPOINT apply_command")?;
        let transition = apply_command(transaction, command_row_id, &request);
        let receipt = match transition? {
            Ok(event_body) => {
                let event_id = insert_event(
                    transaction,
                    command_row_id,
                    &request.command_id,
                    &event_body,
                )?;
                transaction.execute(
                    "UPDATE commands
                     SET command_status = 1, rejection_code = NULL, accepted_event_id = ?1
                     WHERE command_row_id = ?2",
                    params![event_id.value(), command_row_id],
                )?;
                transaction.execute_batch("RELEASE apply_command")?;
                CommandReceipt {
                    disposition: CommandDisposition::Accepted(event_id),
                    idempotent: false,
                }
            }
            Err(rejection) => {
                transaction.execute_batch("ROLLBACK TO apply_command; RELEASE apply_command")?;
                transaction.execute(
                    "UPDATE commands SET rejection_code = ?1 WHERE command_row_id = ?2",
                    params![rejection as i64, command_row_id],
                )?;
                CommandReceipt {
                    disposition: CommandDisposition::Rejected(rejection),
                    idempotent: false,
                }
            }
        };
        Ok(receipt)
    }

    /// Checks whether a founding mission command may proceed to physical
    /// source sealing. This never records a command or consumes a capability.
    /// A fresh predicted rejection is deliberately returned separately so the
    /// daemon can call `execute` and preserve rejected-command history.
    pub fn preflight_install_founding_mission(
        &self,
        request: &CommandRequest,
    ) -> Result<InstallFoundingMissionPreflight, StoreError> {
        let fingerprint = request_fingerprint(request);
        if let Some((stored_fingerprint, status, event_id, rejection)) = self
            .connection
            .query_row(
                "SELECT request_fingerprint, command_status, accepted_event_id, rejection_code
                   FROM commands WHERE command_id = ?1",
                [request.command_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                    ))
                },
            )
            .optional()?
        {
            if stored_fingerprint.as_slice() != fingerprint.as_bytes() {
                return Err(StoreError::IdempotencyConflict);
            }
            let receipt = match status {
                1 => CommandReceipt {
                    disposition: CommandDisposition::Accepted(
                        EventId::try_from(event_id.ok_or(StoreError::LedgerCorruption(
                            "accepted command has no event",
                        ))?)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                    ),
                    idempotent: true,
                },
                2 => CommandReceipt {
                    disposition: CommandDisposition::Rejected(rejection_from_i64(
                        rejection.ok_or(StoreError::LedgerCorruption(
                            "rejected command has no rejection code",
                        ))?,
                    )?),
                    idempotent: true,
                },
                _ => return Err(StoreError::LedgerCorruption("unknown command status")),
            };
            return Ok(InstallFoundingMissionPreflight::ExistingReceipt(receipt));
        }

        let transaction = self.connection.unchecked_transaction()?;
        let result = preflight_founding_mission_request(&transaction, request)?;
        drop(transaction);
        Ok(match result {
            Ok(()) => InstallFoundingMissionPreflight::Ready,
            Err(rejection) => {
                InstallFoundingMissionPreflight::RejectionRequiresExecution(rejection)
            }
        })
    }

    /// Validates and decodes the append-only event ledger through its named
    /// bodies and stored fingerprints. `validate_replayed_materialized_state`
    /// performs the separate fresh-state reconstruction and comparison.
    pub fn replay_ledger(&self) -> Result<Vec<LedgerEvent>, StoreError> {
        verify_command_bodies(&self.connection)?;
        verify_application_mission_source_bindings(&self.connection)?;
        verify_graph_revision_bodies(&self.connection)?;
        let mut statement = self.connection.prepare(
            "SELECT e.event_id, c.command_id, e.event_kind, e.event_sequence
             FROM events e
             JOIN commands c ON c.command_row_id = e.command_row_id
             ORDER BY e.event_sequence ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;
        let mut events = Vec::new();
        for (index, row) in rows.enumerate() {
            let (event_id, command_id, event_kind, event_sequence) = row?;
            if event_sequence != (index + 1) as i64 {
                return Err(StoreError::LedgerCorruption(
                    "event sequence is not contiguous from one",
                ));
            }
            let command_id =
                CommandId::parse(command_id).map_err(|_| StoreError::InvalidStoredValue)?;
            events.push(LedgerEvent {
                event_id: EventId::try_from(event_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                body: decode_event_body(&self.connection, event_id, event_kind, &command_id)?,
                command_id,
            });
        }
        Ok(events)
    }

    /// Reads one accepted event through its exact command and named event body.
    ///
    /// This is the narrow daemon bridge: it verifies the linked command's
    /// typed request fingerprint and command/event receipt relation, then the
    /// requested event's one-to-one body and fingerprint. It intentionally
    /// does not scan unrelated events or establish whole-ledger sequence
    /// continuity; `replay_ledger` remains the ledger-wide verifier.
    pub fn ledger_event(&self, event_id: EventId) -> Result<LedgerEvent, StoreError> {
        let row = self
            .connection
            .query_row(
                "SELECT e.command_row_id, c.command_id, e.event_kind, e.event_sequence,
                        c.command_status, c.accepted_event_id
                 FROM events e
                 LEFT JOIN commands c ON c.command_row_id = e.command_row_id
                 WHERE e.event_id = ?1",
                [event_id.value()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or(StoreError::LedgerEventNotFound(event_id))?;
        let (
            command_row_id,
            command_id,
            event_kind,
            event_sequence,
            command_status,
            accepted_event_id,
        ) = row;

        if event_sequence <= 0 {
            return Err(StoreError::LedgerCorruption(
                "event sequence is not positive",
            ));
        }
        let command_id = command_id.ok_or(StoreError::LedgerCorruption(
            "event references a missing command",
        ))?;
        let command_status = command_status.ok_or(StoreError::LedgerCorruption(
            "event references a missing command",
        ))?;
        verify_command_body(&self.connection, command_row_id)?;
        if command_status != 1 || accepted_event_id != Some(event_id.value()) {
            return Err(StoreError::LedgerCorruption(
                "requested event is not the command's accepted receipt",
            ));
        }
        let command_id =
            CommandId::parse(command_id).map_err(|_| StoreError::InvalidStoredValue)?;
        let body = decode_event_body(&self.connection, event_id.value(), event_kind, &command_id)?;
        Ok(LedgerEvent {
            event_id,
            command_id,
            body,
        })
    }

    /// Reconstructs this bounded kernel's materialized state by re-executing
    /// its verified typed command ledger into a fresh SQLite store, then
    /// compares every current material table/field through a deterministic
    /// digest. This catches mutable-state tampering without pretending that
    /// body-table cardinality alone is replay.
    pub fn validate_replayed_materialized_state(&self) -> Result<Blake3Digest, StoreError> {
        let expected_events = self.replay_ledger()?;
        let commands = replay_command_requests(&self.connection)?;
        let mut reconstructed = Self::open_in_memory()?;
        for (request, expected_disposition) in commands {
            let receipt = reconstructed.execute(request)?;
            if receipt.disposition != expected_disposition {
                return Err(StoreError::LedgerCorruption(
                    "replayed command receipt differs from durable receipt",
                ));
            }
        }
        if reconstructed.replay_ledger()? != expected_events {
            return Err(StoreError::LedgerCorruption(
                "replayed events differ from durable event ledger",
            ));
        }
        let actual = materialized_state_digest(&self.connection)?;
        let rebuilt = materialized_state_digest(&reconstructed.connection)?;
        if actual != rebuilt {
            return Err(StoreError::LedgerCorruption(
                "materialized state differs from fresh replay",
            ));
        }
        Ok(actual)
    }

    pub fn command_count(&self) -> Result<i64, StoreError> {
        Ok(self
            .connection
            .query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))?)
    }

    /// Reads the current admission generation for one exact Operating Cycle.
    ///
    /// The resident supervisor uses this only to attribute an already-spawned
    /// inert child at the generation current when its physical receipt lands;
    /// it is not a work-authorization decision or a generic cycle query.
    pub fn current_operating_cycle_admission_generation(
        &self,
        operating_cycle_id: OperatingCycleId,
    ) -> Result<AdmissionGeneration, StoreError> {
        let generation = self
            .connection
            .query_row(
                "SELECT admission_generation FROM operating_cycles WHERE operating_cycle_id = ?1",
                [operating_cycle_id.value()],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .ok_or(StoreError::OperatingCycleNotFound(operating_cycle_id))?;
        AdmissionGeneration::try_from(generation).map_err(|_| {
            StoreError::LedgerCorruption("operating cycle has invalid admission generation")
        })
    }

    /// Reads the one closed recovery state for an already physically sealed
    /// digest. This is intentionally not a generic row query: it models only
    /// the daemon's receipt-to-global-object transition.
    pub fn content_identity_state(
        &self,
        digest: Blake3Digest,
    ) -> Result<ContentIdentityState, StoreError> {
        let row = self
            .connection
            .query_row(
                "SELECT receipt.content_seal_receipt_id, object.content_object_id
                   FROM content_seal_receipts AS receipt
              LEFT JOIN content_objects AS object
                     ON object.content_seal_receipt_id = receipt.content_seal_receipt_id
                  WHERE receipt.digest = ?1",
                [digest.as_bytes().as_slice()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .optional()?;
        let Some((receipt, object)) = row else {
            return Ok(ContentIdentityState::Absent);
        };
        let content_seal_receipt_id =
            ContentSealReceiptId::try_from(receipt).map_err(|_| StoreError::InvalidStoredValue)?;
        match object {
            Some(object) => Ok(ContentIdentityState::Registered {
                content_seal_receipt_id,
                content_object_id: ContentObjectId::try_from(object)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }),
            None => Ok(ContentIdentityState::SealReceiptOnly {
                content_seal_receipt_id,
            }),
        }
    }

    /// Replays a previously accepted or rejected command receipt by its stable
    /// correlation identity. Rejection is durable operational history but has
    /// no transition event, so it is intentionally queried apart from events.
    pub fn command_receipt(
        &self,
        command_id: &CommandId,
    ) -> Result<Option<CommandReceipt>, StoreError> {
        let row = self
            .connection
            .query_row(
                "SELECT command_status, accepted_event_id, rejection_code
                 FROM commands WHERE command_id = ?1",
                [command_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                },
            )
            .optional()?;
        row.map(|(status, event_id, rejection)| match status {
            1 => Ok(CommandReceipt {
                disposition: CommandDisposition::Accepted(
                    EventId::try_from(event_id.ok_or(StoreError::LedgerCorruption(
                        "accepted command has no event",
                    ))?)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ),
                idempotent: true,
            }),
            2 => Ok(CommandReceipt {
                disposition: CommandDisposition::Rejected(rejection_from_i64(rejection.ok_or(
                    StoreError::LedgerCorruption("rejected command has no rejection code"),
                )?)?),
                idempotent: true,
            }),
            _ => Err(StoreError::LedgerCorruption("unknown command status")),
        })
        .transpose()
    }
}

fn existing_deterministic_evaluator_schedule_claim(
    transaction: &Transaction<'_>,
    claim: &DeterministicEvaluatorScheduleClaimRequest,
) -> Result<Option<NativeChildSpawnAdmissionId>, StoreError> {
    let row: Option<DeterministicEvaluatorScheduleClaimSqlRow> = transaction
        .query_row(
            "SELECT command.command_status, command.rejection_code,
                    command.principal_id, command.capability_kind, command.command_kind,
                    body.native_workspace_id, body.canonical_workspace_path,
                    body.supervisor_epoch_id, body.supervisor_epoch_identity,
                    admission.native_child_spawn_admission_id
               FROM commands command
          LEFT JOIN command_admit_deterministic_evaluator_native_child body
                 ON body.command_row_id = command.command_row_id
          LEFT JOIN native_child_spawn_admissions admission
                 ON admission.admitted_by_command_id = command.command_row_id
              WHERE command.command_id = ?1",
            [claim.command_id.as_str()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            },
        )
        .optional()?;
    let Some((
        status,
        rejection,
        principal,
        capability,
        command_kind,
        workspace,
        path,
        epoch,
        identity,
        admission,
    )) = row
    else {
        return Ok(None);
    };
    let same_claim = principal == PrincipalId::KERNEL.value()
        && capability == Capability::AdmitDeterministicEvaluatorNativeChild as i64
        && command_kind == CommandKind::AdmitDeterministicEvaluatorNativeChild as i64
        && workspace.as_deref() == Some(claim.native_workspace_id.as_str())
        && path.as_deref() == Some(claim.canonical_workspace_path.as_str())
        && epoch == Some(claim.supervisor_epoch_id.value())
        && identity.as_deref() == Some(claim.supervisor_epoch_identity.as_str());
    if !same_claim {
        return Err(StoreError::IdempotencyConflict);
    }
    match status {
        1 => NativeChildSpawnAdmissionId::try_from(admission.ok_or(
            StoreError::LedgerCorruption("accepted evaluator schedule claim has no admission"),
        )?)
        .map(Some)
        .map_err(|_| StoreError::InvalidStoredValue),
        2 => Err(StoreError::DeterministicEvaluatorScheduleClaimRejected(
            rejection_from_i64(rejection.ok_or(StoreError::LedgerCorruption(
                "rejected evaluator schedule claim has no rejection",
            ))?)?,
        )),
        _ => Err(StoreError::LedgerCorruption(
            "unknown evaluator schedule claim command status",
        )),
    }
}

fn preflight_founding_mission_request(
    transaction: &Transaction<'_>,
    request: &CommandRequest,
) -> Result<Result<(), Rejection>, StoreError> {
    let CommandBody::InstallFoundingMission { .. } = &request.body else {
        return Ok(Err(Rejection::CapabilityMismatch));
    };
    if request.capability != Capability::InstallFoundingMission {
        return Ok(Err(Rejection::CapabilityMismatch));
    }
    if request.expected_generation != ExpectedGeneration::NotApplicable {
        return Ok(Err(Rejection::InvalidExpectedGeneration));
    }
    let (grant_id, office_occupancy_id, actor_instance_id) = match capability_grant(
        transaction,
        request.principal_id,
        request.capability,
        request.capability_grant_id,
    )? {
        Some(CapabilityGrantLookup::Active {
            grant_id,
            office_occupancy_id,
            actor_instance_id,
        }) => (grant_id, office_occupancy_id, actor_instance_id),
        Some(CapabilityGrantLookup::Inactive) => {
            return Ok(Err(Rejection::CapabilityNoLongerActive));
        }
        None => return Ok(Err(Rejection::CapabilityNotGranted)),
    };
    if request.principal_id != PrincipalId::BOOTSTRAP && request.principal_id != PrincipalId::KERNEL
    {
        let active = match (office_occupancy_id, actor_instance_id) {
            (Some(_), None) => grant_has_active_occupancy(transaction, grant_id)?,
            (None, Some(_)) => grant_has_active_actor_instance(transaction, grant_id)?,
            _ => false,
        };
        if !active {
            return Ok(Err(Rejection::CapabilityNoLongerActive));
        }
        let target_occupancy_id = match command_target_occupancy(transaction, &request.body) {
            Ok(target_occupancy_id) => target_occupancy_id,
            Err(rejection) => return Ok(Err(rejection)),
        };
        if let Some(target_occupancy_id) = target_occupancy_id
            && office_occupancy_id != Some(target_occupancy_id)
        {
            return Ok(Err(Rejection::CapabilityNoLongerActive));
        }
    }
    if qualification_treatment_fences_request(transaction, request.principal_id, &request.body)? {
        return Ok(Err(Rejection::QualificationTreatmentRestricted));
    }
    let has_founding_mission = exists(
        transaction,
        "SELECT 1 FROM founding_missions WHERE society_id = (SELECT society_id FROM societies LIMIT 1)",
    )
    .map_err(|_| StoreError::LedgerCorruption("cannot read founding mission preflight"))?;
    if only_society_id(transaction).is_err() || has_founding_mission {
        return Ok(Err(Rejection::FoundingInvariant));
    }
    Ok(Ok(()))
}

/// `PiSdkQualificationV1` is a bootstrap-only native lab treatment. It has
/// no Root Authority office work, discovery, or Actor execution surface: the
/// future qualification command may be added only as a kernel-owned typed
/// fact. This guard is intentionally centralized before command dispatch so
/// a newly added cycle-scoped command cannot accidentally turn the paid lab
/// into an ordinary Operating Cycle.
fn qualification_treatment_fences_request(
    transaction: &Transaction<'_>,
    principal_id: PrincipalId,
    body: &CommandBody,
) -> Result<bool, StoreError> {
    if matches!(
        body,
        CommandBody::ProposeOperatingCycle {
            treatment: OperatingCycleTreatment::PiSdkQualificationV1,
            ..
        }
    ) {
        return Ok(principal_id != PrincipalId::BOOTSTRAP);
    }

    if matches!(body, CommandBody::RegisterActorConfiguration { .. }) {
        let qualification_cycle_exists: i64 = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM operating_cycles
             WHERE treatment = ?1 AND lifecycle_state NOT IN (7, 10, 11))",
            [OperatingCycleTreatment::PiSdkQualificationV1 as i64],
            |row| row.get(0),
        )?;
        return Ok(qualification_cycle_exists != 0
            && principal_id != PrincipalId::BOOTSTRAP
            && principal_id != PrincipalId::KERNEL);
    }

    let Some(cycle_id) = command_operating_cycle_for_treatment(transaction, body)? else {
        return Ok(false);
    };
    let treatment: Option<i64> = transaction
        .query_row(
            "SELECT treatment FROM operating_cycles WHERE operating_cycle_id = ?1",
            [cycle_id.value()],
            |row| row.get(0),
        )
        .optional()?;
    if treatment != Some(OperatingCycleTreatment::PiSdkQualificationV1 as i64) {
        return Ok(false);
    }

    let permitted = match principal_id {
        PrincipalId::BOOTSTRAP => matches!(body, CommandBody::AdmitOperatingCycle { .. }),
        PrincipalId::KERNEL => matches!(
            body,
            CommandBody::RecordCycleDrained { .. }
                | CommandBody::RecordOfficeSessionReady { .. }
                | CommandBody::RecordOfficeSessionTerminal { .. }
                | CommandBody::SettleOfficeTurn { .. }
                | CommandBody::ReconcileBudget { .. }
                | CommandBody::ReconcileCancellation { .. }
                | CommandBody::AttestActorAttemptTerminal { .. }
                | CommandBody::ExpireWorkLease { .. }
                | CommandBody::CancelActorAttempt { .. }
        ),
        _ => false,
    };
    Ok(!permitted)
}

fn command_operating_cycle_for_treatment(
    transaction: &Transaction<'_>,
    body: &CommandBody,
) -> Result<Option<OperatingCycleId>, StoreError> {
    let direct = match body {
        CommandBody::AdmitOperatingCycle { cycle_id }
        | CommandBody::StartRootAuthorityOfficeSession { cycle_id }
        | CommandBody::QuiesceOperatingCycle { cycle_id }
        | CommandBody::RecordCycleDrained { cycle_id }
        | CommandBody::ResumeOperatingCycle { cycle_id }
        | CommandBody::ReconcileOperatingCycle { cycle_id }
        | CommandBody::CloseOperatingCycle { cycle_id }
        | CommandBody::ReserveBudget { cycle_id, .. }
        | CommandBody::RequestCancellation { cycle_id, .. } => Some(*cycle_id),
        CommandBody::CreateProject {
            operating_cycle_id, ..
        }
        | CommandBody::CharterProject {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionProject {
            operating_cycle_id, ..
        }
        | CommandBody::CompleteProjectMilestone {
            operating_cycle_id, ..
        }
        | CommandBody::ReopenProject {
            operating_cycle_id, ..
        }
        | CommandBody::CreateTicket {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionTicket {
            operating_cycle_id, ..
        }
        | CommandBody::AddGraphObjectRevision {
            operating_cycle_id, ..
        }
        | CommandBody::CommitGraphRevision {
            operating_cycle_id, ..
        }
        | CommandBody::AddGraphEdge {
            operating_cycle_id, ..
        }
        | CommandBody::CreateEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::ReopenEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::RequestAdversarialReview {
            operating_cycle_id, ..
        }
        | CommandBody::AssignAdversarialReviewer {
            operating_cycle_id, ..
        }
        | CommandBody::SubmitReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::RespondToReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::DispositionReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::ResolveAdversarialReview {
            operating_cycle_id, ..
        }
        | CommandBody::TriggerPostmortem {
            operating_cycle_id, ..
        }
        | CommandBody::RecordPostmortemCausalClaim {
            operating_cycle_id, ..
        }
        | CommandBody::ProposePostmortemAction {
            operating_cycle_id, ..
        }
        | CommandBody::ClosePostmortem {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterContextPack {
            operating_cycle_id, ..
        }
        | CommandBody::AdmitActorInstance {
            operating_cycle_id, ..
        }
        | CommandBody::AdmitTicket {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterWorkItem {
            operating_cycle_id, ..
        }
        | CommandBody::ClaimWorkItem {
            operating_cycle_id, ..
        }
        | CommandBody::StartActorAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::ValidateTicketAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::RetryActorAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::CompleteTicket {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterOutcomeObligation {
            operating_cycle_id, ..
        }
        | CommandBody::ResolveOutcomeObligation {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterForensicManifest {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterDeterministicEvaluatorForensicManifest {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id, ..
        }
        | CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id, ..
        }
        | CommandBody::AdmitDeterministicEvidence {
            operating_cycle_id, ..
        }
        | CommandBody::FinalizeDeterministicExperiment {
            operating_cycle_id, ..
        }
        | CommandBody::AdmitDeterministicEvaluatorNativeChild {
            operating_cycle_id, ..
        } => Some(*operating_cycle_id),
        _ => None,
    };
    if direct.is_some() {
        return Ok(direct);
    }

    let cycle_id: Option<i64> = match body {
        CommandBody::RecordOfficeSessionReady { session_id }
        | CommandBody::RecordOfficeSessionTerminal { session_id, .. }
        | CommandBody::OpenOfficeTurn { session_id, .. } => transaction
            .query_row(
                "SELECT operating_cycle_id FROM root_authority_office_sessions
                 WHERE root_authority_office_session_id = ?1",
                [session_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::SettleOfficeTurn { turn_id, .. } => transaction
            .query_row(
                "SELECT s.operating_cycle_id FROM office_turns t
                 JOIN root_authority_office_sessions s
                   ON s.root_authority_office_session_id = t.root_authority_office_session_id
                 WHERE t.office_turn_id = ?1",
                [turn_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::ReconcileBudget {
            reservation_id, ..
        } => transaction
            .query_row(
                "SELECT operating_cycle_id FROM budget_reservations WHERE budget_reservation_id = ?1",
                [reservation_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        } => transaction
            .query_row(
                "SELECT operating_cycle_id FROM cancellation_requests
                 WHERE cancellation_request_id = ?1",
                [cancellation_request_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::CloseCostPostmortem { postmortem_id, .. } => transaction
            .query_row(
                "SELECT operating_cycle_id FROM cost_postmortems WHERE postmortem_id = ?1",
                [postmortem_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id, ..
        }
        | CommandBody::CancelActorAttempt {
            actor_attempt_id, ..
        } => transaction
            .query_row(
                "SELECT operating_cycle_id FROM attempts WHERE actor_attempt_id = ?1",
                [actor_attempt_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::ExpireWorkLease { work_lease_id } => transaction
            .query_row(
                "SELECT a.operating_cycle_id FROM leases l
                 JOIN actor_instances a ON a.actor_instance_id = l.actor_instance_id
                 WHERE l.work_lease_id = ?1",
                [work_lease_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        _ => None,
    };
    cycle_id
        .map(OperatingCycleId::try_from)
        .transpose()
        .map_err(|_| StoreError::InvalidStoredValue)
}

fn bootstrap_current_schema(connection: &Connection) -> Result<(), StoreError> {
    if let Err(error) = connection.execute_batch(CURRENT_SCHEMA) {
        let _ = connection.execute_batch("ROLLBACK");
        return Err(error.into());
    }
    connection.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}

fn apply_command(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    request: &CommandRequest,
) -> Result<Result<EventBody, Rejection>, StoreError> {
    if request.capability != request.body.required_capability() {
        return Ok(Err(Rejection::CapabilityMismatch));
    }
    if matches!(
        request.body,
        CommandBody::AdmitOperatingCycle { .. }
            | CommandBody::StartRootAuthorityOfficeSession { .. }
            | CommandBody::RecordOfficeSessionReady { .. }
            | CommandBody::RecordOfficeSessionTerminal { .. }
            | CommandBody::OpenOfficeTurn { .. }
            | CommandBody::QuiesceOperatingCycle { .. }
            | CommandBody::ResumeOperatingCycle { .. }
            | CommandBody::ReconcileOperatingCycle { .. }
            | CommandBody::CloseOperatingCycle { .. }
            | CommandBody::ReserveBudget { .. }
            | CommandBody::RequestCancellation { .. }
            | CommandBody::CloseCostPostmortem { .. }
            | CommandBody::CreateProject { .. }
            | CommandBody::CharterProject { .. }
            | CommandBody::TransitionProject { .. }
            | CommandBody::CompleteProjectMilestone { .. }
            | CommandBody::ReopenProject { .. }
            | CommandBody::CreateTicket { .. }
            | CommandBody::TransitionTicket { .. }
            | CommandBody::AddGraphObjectRevision { .. }
            | CommandBody::CommitGraphRevision { .. }
            | CommandBody::AddGraphEdge { .. }
            | CommandBody::CreateEpisode { .. }
            | CommandBody::TransitionEpisode { .. }
            | CommandBody::ReopenEpisode { .. }
            | CommandBody::RequestAdversarialReview { .. }
            | CommandBody::AssignAdversarialReviewer { .. }
            | CommandBody::SubmitReviewChallenge { .. }
            | CommandBody::RespondToReviewChallenge { .. }
            | CommandBody::DispositionReviewChallenge { .. }
            | CommandBody::ResolveAdversarialReview { .. }
            | CommandBody::TriggerPostmortem { .. }
            | CommandBody::RecordPostmortemCausalClaim { .. }
            | CommandBody::ProposePostmortemAction { .. }
            | CommandBody::ClosePostmortem { .. }
            | CommandBody::RegisterContextPack { .. }
            | CommandBody::AdmitActorInstance { .. }
            | CommandBody::AdmitTicket { .. }
            | CommandBody::RegisterWorkItem { .. }
            | CommandBody::ClaimWorkItem { .. }
            | CommandBody::StartActorAttempt { .. }
            | CommandBody::ValidateTicketAttempt { .. }
            | CommandBody::RetryActorAttempt { .. }
            | CommandBody::CompleteTicket { .. }
            | CommandBody::RegisterOutcomeObligation { .. }
            | CommandBody::ResolveOutcomeObligation { .. }
            | CommandBody::RegisterForensicManifest { .. }
            | CommandBody::RegisterDeterministicEvaluatorForensicManifest { .. }
            | CommandBody::RegisterDeterministicExperiment { .. }
            | CommandBody::RecordDeterministicEvaluationReceipt { .. }
            | CommandBody::AdmitDeterministicEvidence { .. }
            | CommandBody::FinalizeDeterministicExperiment { .. }
            | CommandBody::AdmitPiChildSpawn { .. }
            | CommandBody::RecordInertChildSpawn { .. }
            | CommandBody::RecordPiAdapterReady { .. }
            | CommandBody::AuthorizePiCreateSession { .. }
            | CommandBody::RecordPiCreateSessionDelivery { .. }
            | CommandBody::RecordPiSessionReady { .. }
            | CommandBody::RecordPiAbortControlDelivery { .. }
            | CommandBody::RecordChildStreamSeal { .. }
            | CommandBody::RecordChildProcessLiveness { .. }
            | CommandBody::RecordProcessSignalReceipt { .. }
            | CommandBody::RecordDirectChildReap { .. }
            | CommandBody::RecordChildRecovery { .. }
            | CommandBody::FinalizeChildProcess { .. }
            | CommandBody::BeginCancellationPropagation { .. }
            | CommandBody::ReconcileCancellationPropagation { .. }
            | CommandBody::RecordNativeChildNotSpawned { .. }
            | CommandBody::AdmitDeterministicEvaluatorNativeChild { .. }
            | CommandBody::RecordDeterministicEvaluatorNativeChildSpawn { .. }
            | CommandBody::AuthorizePiOfficeTurnPrompt { .. }
            | CommandBody::RecordPiOfficeTurnPromptDelivery { .. }
            | CommandBody::RecordPiOfficeTurnPromptAccepted { .. }
            | CommandBody::RecordPiOfficeTurnUsage { .. }
            | CommandBody::RecordPiOfficeTurnUsageFailure { .. }
            | CommandBody::RecordPiOfficeTurnTerminal { .. }
            | CommandBody::AuthorizePiOfficeSessionDispose { .. }
            | CommandBody::RecordPiOfficeSessionDisposeDelivery { .. }
            | CommandBody::RecordPiOfficeSessionDisposeAccepted { .. }
            | CommandBody::RecordPiOfficeSessionDisposeUsage { .. }
            | CommandBody::RecordPiOfficeSessionDisposeUsageFailure { .. }
            | CommandBody::RecordPiOfficeSessionDisposed { .. }
    ) != matches!(request.expected_generation, ExpectedGeneration::Exact(_))
    {
        return Ok(Err(Rejection::InvalidExpectedGeneration));
    }
    let (grant_id, office_occupancy_id, actor_instance_id) = match capability_grant(
        transaction,
        request.principal_id,
        request.capability,
        request.capability_grant_id,
    )? {
        Some(CapabilityGrantLookup::Active {
            grant_id,
            office_occupancy_id,
            actor_instance_id,
        }) => (grant_id, office_occupancy_id, actor_instance_id),
        Some(CapabilityGrantLookup::Inactive) => {
            return Ok(Err(Rejection::CapabilityNoLongerActive));
        }
        None => return Ok(Err(Rejection::CapabilityNotGranted)),
    };
    if request.principal_id != PrincipalId::BOOTSTRAP && request.principal_id != PrincipalId::KERNEL
    {
        let active = match (office_occupancy_id, actor_instance_id) {
            (Some(_), None) => grant_has_active_occupancy(transaction, grant_id)?,
            (None, Some(_)) => grant_has_active_actor_instance(transaction, grant_id)?,
            _ => false,
        };
        if !active {
            return Ok(Err(Rejection::CapabilityNoLongerActive));
        }
    }
    if request.principal_id != PrincipalId::BOOTSTRAP && request.principal_id != PrincipalId::KERNEL
    {
        let target_occupancy_id = match command_target_occupancy(transaction, &request.body) {
            Ok(target_occupancy_id) => target_occupancy_id,
            Err(rejection) => return Ok(Err(rejection)),
        };
        if let Some(target_occupancy_id) = target_occupancy_id
            && office_occupancy_id != Some(target_occupancy_id)
        {
            return Ok(Err(Rejection::CapabilityNoLongerActive));
        }
    }
    if qualification_treatment_fences_request(transaction, request.principal_id, &request.body)? {
        return Ok(Err(Rejection::QualificationTreatmentRestricted));
    }

    let result = match &request.body {
        CommandBody::CreateSocietyIdentity { name } => {
            create_society(transaction, command_row_id, name)
        }
        CommandBody::InstallRootAuthorityOffice => {
            install_root_authority_office(transaction, command_row_id)
        }
        CommandBody::InstallFoundingMission { mission } => {
            install_founding_mission(transaction, command_row_id, mission)
        }
        CommandBody::AppointInitialRootAuthority { actor_display_name } => {
            appoint_initial_root_authority(transaction, command_row_id, actor_display_name.as_str())
        }
        CommandBody::SetR0HardCeiling { ceiling } => {
            set_r0_hard_ceiling(transaction, command_row_id, *ceiling)
        }
        CommandBody::BootstrapSociety => bootstrap_society(transaction, command_row_id),
        CommandBody::ProposeOperatingCycle {
            treatment,
            budget_ceiling,
        } => propose_operating_cycle(transaction, command_row_id, *treatment, *budget_ceiling),
        CommandBody::AdmitOperatingCycle { cycle_id } => admit_operating_cycle(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::StartRootAuthorityOfficeSession { cycle_id } => start_office_session(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::RecordOfficeSessionReady { session_id } => record_office_session_ready(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
        ),
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state,
        } => record_office_session_terminal(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
            *terminal_state,
        ),
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose,
        } => open_office_turn(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
            *purpose,
        ),
        CommandBody::SettleOfficeTurn {
            turn_id,
            terminal_receipt_id,
        } => settle_office_turn(transaction, command_row_id, *turn_id, *terminal_receipt_id),
        CommandBody::QuiesceOperatingCycle { cycle_id } => quiesce_cycle(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::RecordCycleDrained { cycle_id } => {
            record_cycle_drained(transaction, command_row_id, *cycle_id)
        }
        CommandBody::ResumeOperatingCycle { cycle_id } => resume_cycle(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::ReconcileOperatingCycle { cycle_id } => begin_reconciliation(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::CloseOperatingCycle { cycle_id } => close_cycle(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::ReserveBudget { cycle_id, amount } => reserve_budget(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
            *amount,
        ),
        CommandBody::ReconcileBudget {
            reservation_id,
            observation,
        } => reconcile_budget(transaction, command_row_id, *reservation_id, *observation),
        CommandBody::RequestCancellation { cycle_id, mode } => request_cancellation(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
            *mode,
        ),
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        } => reconcile_cancellation(transaction, command_row_id, *cancellation_request_id),
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution,
        } => close_cost_postmortem(
            transaction,
            command_row_id,
            request.expected_generation,
            *postmortem_id,
            *resolution,
        ),
        CommandBody::CreateProject {
            operating_cycle_id,
            project_name,
            north_star_alignment,
        } => create_project(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            project_name.as_str(),
            north_star_alignment,
        ),
        CommandBody::CharterProject {
            operating_cycle_id,
            project_id,
            objective,
            initial_milestone,
            stop_condition,
        } => charter_project(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            objective.as_str(),
            initial_milestone.as_str(),
            stop_condition.as_str(),
        ),
        CommandBody::TransitionProject {
            operating_cycle_id,
            project_id,
            target,
        } => transition_project(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *target,
        ),
        CommandBody::CompleteProjectMilestone {
            operating_cycle_id,
            project_milestone_id,
        } => complete_project_milestone(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_milestone_id,
        ),
        CommandBody::ReopenProject {
            operating_cycle_id,
            project_id,
        } => reopen_project(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
        ),
        CommandBody::CreateTicket {
            operating_cycle_id,
            project_id,
            ticket_title,
            acceptance_condition,
            prerequisite_ticket_id,
        } => create_ticket(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            ticket_title.as_str(),
            acceptance_condition.as_str(),
            *prerequisite_ticket_id,
        ),
        CommandBody::TransitionTicket {
            operating_cycle_id,
            ticket_id,
            target,
        } => transition_ticket(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *ticket_id,
            *target,
        ),
        CommandBody::AddGraphObjectRevision {
            operating_cycle_id,
            project_id,
            causal_episode_id,
            graph_object_id,
            body,
        } => add_graph_object_revision(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *causal_episode_id,
            *graph_object_id,
            body,
        ),
        CommandBody::CommitGraphRevision {
            operating_cycle_id,
            graph_revision_id,
        } => commit_graph_revision(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *graph_revision_id,
        ),
        CommandBody::AddGraphEdge {
            operating_cycle_id,
            project_id,
            from_graph_revision_id,
            to_graph_revision_id,
            edge_kind,
        } => add_graph_edge(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *from_graph_revision_id,
            *to_graph_revision_id,
            *edge_kind,
        ),
        CommandBody::CreateEpisode {
            operating_cycle_id,
            project_id,
        } => create_episode(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
        ),
        CommandBody::TransitionEpisode {
            operating_cycle_id,
            causal_episode_id,
            target,
        } => transition_episode(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *causal_episode_id,
            *target,
        ),
        CommandBody::ReopenEpisode {
            operating_cycle_id,
            causal_episode_id,
        } => reopen_episode(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *causal_episode_id,
        ),
        CommandBody::RequestAdversarialReview {
            operating_cycle_id,
            project_id,
            target_graph_revision_id,
        } => request_adversarial_review(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *target_graph_revision_id,
        ),
        CommandBody::AssignAdversarialReviewer {
            operating_cycle_id,
            adversarial_review_id,
            reviewer_principal_id,
            reviewer_actor_instance_id,
            reviewer_actor_attempt_id,
        } => assign_adversarial_reviewer(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *adversarial_review_id,
            *reviewer_principal_id,
            *reviewer_actor_instance_id,
            *reviewer_actor_attempt_id,
        ),
        CommandBody::SubmitReviewChallenge {
            operating_cycle_id,
            adversarial_review_id,
            target_graph_revision_id,
            author_principal_id,
            severity,
            failure_hypothesis,
        } => submit_review_challenge(
            transaction,
            command_row_id,
            *author_principal_id,
            request.expected_generation,
            *operating_cycle_id,
            *adversarial_review_id,
            *target_graph_revision_id,
            *severity,
            failure_hypothesis.as_str(),
        ),
        CommandBody::RespondToReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            response,
        } => respond_to_review_challenge(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *review_challenge_id,
            response.as_str(),
        ),
        CommandBody::DispositionReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            disposition,
        } => disposition_review_challenge(
            transaction,
            command_row_id,
            request.principal_id,
            request.expected_generation,
            *operating_cycle_id,
            *review_challenge_id,
            *disposition,
        ),
        CommandBody::ResolveAdversarialReview {
            operating_cycle_id,
            adversarial_review_id,
            resolution,
        } => resolve_adversarial_review(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *adversarial_review_id,
            *resolution,
        ),
        CommandBody::TriggerPostmortem {
            operating_cycle_id,
            project_id,
            causal_episode_id,
        } => trigger_postmortem(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *causal_episode_id,
        ),
        CommandBody::RecordPostmortemCausalClaim {
            operating_cycle_id,
            postmortem_id,
            claim_kind,
            claim,
        } => record_postmortem_causal_claim(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *postmortem_id,
            *claim_kind,
            claim.as_str(),
        ),
        CommandBody::ProposePostmortemAction {
            operating_cycle_id,
            postmortem_id,
            action_kind,
            action,
        } => propose_postmortem_action(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *postmortem_id,
            *action_kind,
            action.as_str(),
        ),
        CommandBody::ClosePostmortem {
            operating_cycle_id,
            postmortem_id,
        } => close_postmortem(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *postmortem_id,
        ),
        CommandBody::RegisterActorConfiguration {
            configuration_name,
            model_policy,
            primary_attractor,
        } => register_actor_configuration(
            transaction,
            command_row_id,
            configuration_name.as_str(),
            *model_policy,
            *primary_attractor,
        ),
        CommandBody::RegisterContextPack {
            operating_cycle_id,
            purpose,
            rendering_digest,
        } => register_context_pack(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *purpose,
            *rendering_digest,
        ),
        CommandBody::AdmitActorInstance {
            operating_cycle_id,
            actor_configuration_revision_id,
            execution_profile_id,
            actor_display_name,
        } => admit_actor_instance(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *actor_configuration_revision_id,
            *execution_profile_id,
            actor_display_name.as_str(),
        ),
        CommandBody::AdmitTicket {
            operating_cycle_id,
            ticket_id,
        } => admit_ticket(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *ticket_id,
        ),
        CommandBody::RegisterWorkItem {
            operating_cycle_id,
            ticket_id,
            actor_instance_id,
            context_pack_id,
            work_kind,
            adversarial_review_id,
            assignment,
        } => register_work_item(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *ticket_id,
            *actor_instance_id,
            *context_pack_id,
            *work_kind,
            *adversarial_review_id,
            assignment.as_str(),
        ),
        CommandBody::ClaimWorkItem {
            operating_cycle_id,
            work_item_id,
        } => claim_work_item(
            transaction,
            command_row_id,
            request.principal_id,
            request.expected_generation,
            *operating_cycle_id,
            *work_item_id,
        ),
        CommandBody::StartActorAttempt {
            operating_cycle_id,
            work_item_id,
            reservation_amount,
        } => start_actor_attempt(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *work_item_id,
            *reservation_amount,
        ),
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id,
            terminal_kind,
        } => attest_actor_attempt_terminal(
            transaction,
            command_row_id,
            *actor_attempt_id,
            *terminal_kind,
        ),
        CommandBody::ValidateTicketAttempt {
            operating_cycle_id,
            actor_attempt_id,
        } => validate_ticket_attempt(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *actor_attempt_id,
        ),
        CommandBody::RetryActorAttempt {
            operating_cycle_id,
            actor_attempt_id,
        } => retry_actor_attempt(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *actor_attempt_id,
        ),
        CommandBody::CompleteTicket {
            operating_cycle_id,
            actor_attempt_id,
        } => complete_ticket(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *actor_attempt_id,
        ),
        CommandBody::ExpireWorkLease { work_lease_id } => {
            expire_work_lease(transaction, command_row_id, *work_lease_id)
        }
        CommandBody::CancelActorAttempt {
            actor_attempt_id,
            reason,
        } => cancel_actor_attempt(transaction, command_row_id, *actor_attempt_id, *reason),
        CommandBody::RegisterOutcomeObligation {
            operating_cycle_id,
            project_id,
            obligation,
        } => register_outcome_obligation(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            obligation.as_str(),
        ),
        CommandBody::ResolveOutcomeObligation {
            operating_cycle_id,
            outcome_obligation_id,
            disposition,
        } => resolve_outcome_obligation(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *outcome_obligation_id,
            *disposition,
        ),
        CommandBody::RecordContentSealReceipt { digest } => {
            record_content_seal_receipt(transaction, command_row_id, *digest)
        }
        CommandBody::RegisterContentObject {
            content_seal_receipt_id,
        } => register_content_object(transaction, command_row_id, *content_seal_receipt_id),
        CommandBody::RegisterForensicManifest {
            operating_cycle_id,
            producing_deterministic_experiment_id,
            capture_policy,
            retention_access_class,
            evaluator_output_content_object_id,
        } => register_forensic_manifest(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *producing_deterministic_experiment_id,
            *capture_policy,
            *retention_access_class,
            *evaluator_output_content_object_id,
        ),
        CommandBody::RegisterDeterministicEvaluatorForensicManifest {
            operating_cycle_id,
            native_child_spawn_admission_id,
        } => register_deterministic_evaluator_forensic_manifest(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *native_child_spawn_admission_id,
        ),
        CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id,
            project_id,
            ticket_id,
            target_graph_revision_id,
            evaluator_content_object_id,
            input_manifest_content_object_id,
        } => register_deterministic_experiment(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *ticket_id,
            *target_graph_revision_id,
            *evaluator_content_object_id,
            *input_manifest_content_object_id,
        ),
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            forensic_manifest_id,
            evaluator_output_content_object_id,
        } => record_deterministic_evaluation_receipt(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *deterministic_experiment_id,
            *evaluator_revision_id,
            *input_manifest_id,
            *forensic_manifest_id,
            *evaluator_output_content_object_id,
        ),
        CommandBody::AdmitDeterministicEvidence {
            operating_cycle_id,
            deterministic_evaluation_receipt_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            evaluator_output_content_object_id,
            related_graph_revision_id,
            semantic_role,
            applicability,
            limitation,
        } => admit_deterministic_evidence(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *deterministic_evaluation_receipt_id,
            *deterministic_experiment_id,
            *evaluator_revision_id,
            *input_manifest_id,
            *evaluator_output_content_object_id,
            *related_graph_revision_id,
            *semantic_role,
            *applicability,
            limitation,
        ),
        CommandBody::FinalizeDeterministicExperiment {
            operating_cycle_id,
            deterministic_experiment_id,
        } => finalize_deterministic_experiment(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *deterministic_experiment_id,
        ),
        CommandBody::AdmitDeterministicEvaluatorNativeChild {
            operating_cycle_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            execution_profile_id,
            native_workspace_id,
            canonical_workspace_path,
            supervisor_epoch_id,
            supervisor_epoch_identity,
        } => admit_deterministic_evaluator_native_child(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *deterministic_experiment_id,
            *evaluator_revision_id,
            *input_manifest_id,
            *execution_profile_id,
            native_workspace_id,
            canonical_workspace_path,
            *supervisor_epoch_id,
            supervisor_epoch_identity,
        ),
        CommandBody::RecordDeterministicEvaluatorNativeChildSpawn {
            native_child_spawn_admission_id,
            child_identity,
            direct_child_pid,
            process_group_id,
        } => record_deterministic_evaluator_native_child_spawn(
            transaction,
            command_row_id,
            request.expected_generation,
            *native_child_spawn_admission_id,
            child_identity,
            *direct_child_pid,
            *process_group_id,
        ),
        CommandBody::OpenSupervisorEpoch {
            supervisor_epoch_id,
            supervisor_epoch_identity,
        } => open_supervisor_epoch(
            transaction,
            command_row_id,
            *supervisor_epoch_id,
            supervisor_epoch_identity,
        ),
        CommandBody::AdmitPiChildSpawn {
            operating_cycle_id,
            owner,
            budget_reservation_id,
            execution_profile_id,
            native_workspace_id,
            canonical_workspace_path,
            supervisor_epoch_id,
            supervisor_epoch_identity,
            pi_session_identity,
            spawn_nonce,
        } => admit_pi_child_spawn(
            transaction,
            command_row_id,
            request.expected_generation,
            PiChildSpawnAdmissionInput {
                operating_cycle_id: *operating_cycle_id,
                owner: *owner,
                budget_reservation_id: *budget_reservation_id,
                execution_profile_id: *execution_profile_id,
                native_workspace_id,
                canonical_workspace_path,
                supervisor_epoch_id: *supervisor_epoch_id,
                supervisor_epoch_identity,
                pi_session_identity,
                spawn_nonce,
            },
        ),
        CommandBody::RecordInertChildSpawn {
            native_child_spawn_admission_id,
            child_identity,
            direct_child_pid,
            process_group_id,
        } => record_inert_child_spawn(
            transaction,
            command_row_id,
            request.expected_generation,
            *native_child_spawn_admission_id,
            child_identity,
            *direct_child_pid,
            *process_group_id,
        ),
        CommandBody::RecordPiAdapterReady {
            native_child_id,
            pi_session_identity,
            spawn_nonce,
        } => record_pi_adapter_ready(
            transaction,
            command_row_id,
            request.expected_generation,
            *native_child_id,
            pi_session_identity,
            spawn_nonce,
        ),
        CommandBody::AuthorizePiCreateSession {
            native_child_id,
            correlation_identity,
            create_request_digest,
        } => authorize_pi_create_session(
            transaction,
            command_row_id,
            request.expected_generation,
            *native_child_id,
            correlation_identity,
            *create_request_digest,
        ),
        CommandBody::RecordPiCreateSessionDelivery {
            native_child_id,
            correlation_identity,
            create_request_digest,
        } => record_pi_create_session_delivery(
            transaction,
            command_row_id,
            request.expected_generation,
            *native_child_id,
            correlation_identity,
            *create_request_digest,
        ),
        CommandBody::RecordPiSessionReady {
            native_child_id,
            pi_session_identity,
        } => record_pi_session_ready(
            transaction,
            command_row_id,
            request.expected_generation,
            *native_child_id,
            pi_session_identity,
        ),
        CommandBody::RecordPiAbortControlDelivery {
            native_child_id,
            cancellation_propagation_id,
            correlation_identity,
            abort_command_digest,
            outcome,
        } => record_pi_abort_control_delivery(
            transaction,
            command_row_id,
            request.expected_generation,
            PiAbortControlDeliveryInput {
                child_id: *native_child_id,
                propagation_id: *cancellation_propagation_id,
                correlation: correlation_identity,
                abort_digest: *abort_command_digest,
                outcome: *outcome,
            },
        ),
        CommandBody::RecordChildStreamSeal {
            native_child_id,
            stream_kind,
            full_observed_digest,
            retained_content_object_id,
            completeness,
        } => record_child_stream_seal(
            transaction,
            command_row_id,
            request.expected_generation,
            ChildStreamSealInput {
                child_id: *native_child_id,
                stream: *stream_kind,
                full_digest: *full_observed_digest,
                retained: *retained_content_object_id,
                completeness: *completeness,
            },
        ),
        CommandBody::RecordChildProcessLiveness {
            native_child_id,
            liveness,
        } => record_child_process_liveness(
            transaction,
            command_row_id,
            request.expected_generation,
            *native_child_id,
            *liveness,
        ),
        CommandBody::RecordProcessSignalReceipt {
            native_child_id,
            action,
            delivery,
            observed_liveness,
            cause,
        } => record_process_signal_receipt(
            transaction,
            command_row_id,
            request.expected_generation,
            ProcessSignalReceiptInput {
                child_id: *native_child_id,
                action: *action,
                delivery: *delivery,
                liveness: *observed_liveness,
                cause: *cause,
            },
        ),
        CommandBody::RecordDirectChildReap {
            native_child_id,
            wait_status,
            group_liveness_before_cleanup,
            group_liveness_after_cleanup,
        } => record_direct_child_reap(
            transaction,
            command_row_id,
            request.expected_generation,
            *native_child_id,
            *wait_status,
            *group_liveness_before_cleanup,
            *group_liveness_after_cleanup,
        ),
        CommandBody::RecordChildRecovery {
            native_child_id,
            observation,
            group_liveness_after_restart,
        } => record_child_recovery(
            transaction,
            command_row_id,
            request.expected_generation,
            *native_child_id,
            *observation,
            *group_liveness_after_restart,
        ),
        CommandBody::FinalizeChildProcess { native_child_id } => finalize_child_process(
            transaction,
            command_row_id,
            request.expected_generation,
            *native_child_id,
        ),
        CommandBody::BeginCancellationPropagation {
            cancellation_request_id,
        } => begin_cancellation_propagation(
            transaction,
            command_row_id,
            request.expected_generation,
            *cancellation_request_id,
        ),
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id,
        } => reconcile_cancellation_propagation(
            transaction,
            command_row_id,
            request.expected_generation,
            *cancellation_propagation_id,
        ),
        CommandBody::RecordNativeChildNotSpawned {
            native_child_spawn_admission_id,
            reason,
        } => record_native_child_not_spawned(
            transaction,
            command_row_id,
            request.expected_generation,
            *native_child_spawn_admission_id,
            *reason,
        ),
        CommandBody::AuthorizePiOfficeTurnPrompt {
            office_turn_id,
            correlation_identity,
            prompt_content_object_id,
            prompt_digest,
            frontier_event_id,
        } => authorize_pi_office_turn_prompt(
            transaction,
            command_row_id,
            PiOfficeTurnPromptAuthorizationInput {
                expected_generation: request.expected_generation,
                office_turn_id: *office_turn_id,
                correlation_identity,
                prompt_content_object_id: *prompt_content_object_id,
                prompt_digest: *prompt_digest,
                frontier_event_id: *frontier_event_id,
            },
        ),
        CommandBody::RecordPiOfficeTurnPromptDelivery {
            office_turn_id,
            correlation_identity,
            prompt_digest,
        } => record_pi_office_turn_prompt_delivery(
            transaction,
            command_row_id,
            *office_turn_id,
            correlation_identity,
            *prompt_digest,
        ),
        CommandBody::RecordPiOfficeTurnPromptAccepted {
            office_turn_id,
            correlation_identity,
            command_result_sequence,
        } => record_pi_office_turn_prompt_accepted(
            transaction,
            command_row_id,
            *office_turn_id,
            correlation_identity,
            *command_result_sequence,
        ),
        CommandBody::RecordPiOfficeTurnUsage {
            office_turn_id,
            correlation_identity,
            protocol_sequence,
            usage,
        } => record_pi_office_turn_usage(
            transaction,
            command_row_id,
            *office_turn_id,
            correlation_identity,
            *protocol_sequence,
            *usage,
        ),
        CommandBody::RecordPiOfficeTurnUsageFailure {
            office_turn_id,
            correlation_identity,
            protocol_sequence,
            failure,
        } => record_pi_office_turn_usage_failure(
            transaction,
            command_row_id,
            *office_turn_id,
            correlation_identity,
            *protocol_sequence,
            *failure,
        ),
        CommandBody::RecordPiOfficeTurnTerminal {
            office_turn_id,
            correlation_identity,
            terminal_evidence,
            settled_sequence,
            disposition,
            assistant_outcome,
            transcript_disposition,
        } => record_pi_office_turn_terminal(
            transaction,
            command_row_id,
            PiOfficeTurnTerminalInput {
                office_turn_id: *office_turn_id,
                correlation_identity,
                terminal_evidence: *terminal_evidence,
                settled_sequence: *settled_sequence,
                disposition: *disposition,
                assistant_outcome: *assistant_outcome,
                transcript_disposition: *transcript_disposition,
            },
        ),
        CommandBody::RecordPiOfficeSessionDisposeDelivery {
            session_id,
            correlation_identity,
        } => record_pi_office_session_dispose_delivery(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
            correlation_identity,
        ),
        CommandBody::AuthorizePiOfficeSessionDispose {
            session_id,
            correlation_identity,
        } => authorize_pi_office_session_dispose(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
            correlation_identity,
        ),
        CommandBody::RecordPiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity,
            command_result_sequence,
        } => record_pi_office_session_dispose_accepted(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
            correlation_identity,
            *command_result_sequence,
        ),
        CommandBody::RecordPiOfficeSessionDisposeUsage {
            session_id,
            correlation_identity,
            protocol_sequence,
            usage,
        } => record_pi_office_session_dispose_usage(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
            correlation_identity,
            *protocol_sequence,
            *usage,
        ),
        CommandBody::RecordPiOfficeSessionDisposeUsageFailure {
            session_id,
            correlation_identity,
            protocol_sequence,
            failure,
        } => record_pi_office_session_dispose_usage_failure(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
            correlation_identity,
            *protocol_sequence,
            *failure,
        ),
        CommandBody::RecordPiOfficeSessionDisposed {
            session_id,
            correlation_identity,
            disposed_sequence,
            transcript_receipt,
        } => record_pi_office_session_disposed(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
            correlation_identity,
            *disposed_sequence,
            transcript_receipt,
        ),
    };

    if result.is_ok()
        && request.principal_id == PrincipalId::BOOTSTRAP
        && request.capability.requires_consumption()
    {
        transaction.execute(
            "UPDATE capability_grants SET grant_state = 2, consumed_by_command_id = ?1
             WHERE capability_grant_id = ?2 AND grant_state = 1",
            params![command_row_id, grant_id],
        )?;
    }
    Ok(result)
}

fn create_society(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    name: &SocietyName,
) -> Result<EventBody, Rejection> {
    if exists(transaction, "SELECT 1 FROM societies LIMIT 1")? {
        return Err(Rejection::FoundingInvariant);
    }
    transaction
        .execute(
            "INSERT INTO societies(name, lifecycle_state) VALUES (?1, 1)",
            [name.as_str()],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    let society_id = id_from_last_insert::<SocietyId>(transaction)?;
    let _ = command_row_id;
    Ok(EventBody::SocietyIdentityCreated { society_id })
}

fn install_root_authority_office(
    transaction: &Transaction<'_>,
    command_row_id: i64,
) -> Result<EventBody, Rejection> {
    if !exists(transaction, "SELECT 1 FROM societies LIMIT 1")?
        || exists(
            transaction,
            "SELECT 1 FROM office_contracts WHERE office_kind = 1",
        )?
    {
        return Err(Rejection::FoundingInvariant);
    }
    transaction
        .execute(
            "INSERT INTO office_contracts(office_kind, installed_by_command_id) VALUES (?1, ?2)",
            params![OfficeKind::RootAuthorityOffice as i64, command_row_id],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    Ok(EventBody::RootAuthorityOfficeInstalled {
        office_id: id_from_last_insert::<OfficeId>(transaction)?,
    })
}

fn install_founding_mission(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    mission: &ApplicationMissionInput,
) -> Result<EventBody, Rejection> {
    let society_id = only_society_id(transaction)?;
    if exists(
        transaction,
        "SELECT 1 FROM founding_missions WHERE society_id = (SELECT society_id FROM societies LIMIT 1)",
    )? {
        return Err(Rejection::FoundingInvariant);
    }
    let source_content_object_id =
        mission_source_content_object_id(transaction, mission.source_rendering_digest)?;
    transaction
        .execute(
            "INSERT INTO applications(application_identity, application_name, created_by_command_id)
             VALUES (?1, ?2, ?3)",
            params![
                mission.application_identity.as_str(),
                mission.application_name.as_str(),
                command_row_id
            ],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    let application_id = id_from_last_insert::<ApplicationId>(transaction)?;
    transaction
        .execute(
            "INSERT INTO application_revisions(
                 application_id, revision_ordinal, mission_statement,
                 source_rendering_digest, source_content_object_id, installed_by_command_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                application_id.value(),
                mission.revision_ordinal.value(),
                mission.statement.as_str(),
                mission.source_rendering_digest.as_bytes().as_slice(),
                source_content_object_id.value(),
                command_row_id,
            ],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    let application_revision_id = id_from_last_insert::<ApplicationRevisionId>(transaction)?;
    for (index, principle) in mission.principles.as_slice().iter().enumerate() {
        let ordinal = i64::try_from(index + 1).map_err(|_| Rejection::FoundingInvariant)?;
        transaction
            .execute(
                "INSERT INTO application_revision_principles(
                     application_revision_id, principle_ordinal,
                     principle_kind, principle_text
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    application_revision_id.value(),
                    ordinal,
                    principle.kind as i64,
                    principle.text.as_str(),
                ],
            )
            .map_err(|_| Rejection::FoundingInvariant)?;
    }
    let questions = &mission.north_star_questions;
    transaction
        .execute(
            "INSERT INTO application_revision_north_star_questions(
                 application_revision_id, change_question,
                 improvement_evidence_question, boundary_commitment_question,
                 revisit_question
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                application_revision_id.value(),
                questions.change.as_str(),
                questions.improvement_evidence.as_str(),
                questions.boundary_commitment.as_str(),
                questions.revisit.as_str(),
            ],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    transaction
        .execute(
            "INSERT INTO founding_missions(
                 society_id, application_revision_id, revision,
                 active, installed_by_command_id
             ) VALUES (?1, ?2, ?3, 1, ?4)",
            params![
                society_id.value(),
                application_revision_id.value(),
                mission.revision_ordinal.value(),
                command_row_id
            ],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    Ok(EventBody::FoundingMissionInstalled {
        mission_id: id_from_last_insert::<FoundingMissionId>(transaction)?,
        application_revision_id,
    })
}

fn appoint_initial_root_authority(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    actor_display_name: &str,
) -> Result<EventBody, Rejection> {
    let office_id = root_authority_office_id(transaction)?;
    if exists(
        transaction,
        "SELECT 1 FROM office_occupancies WHERE office_id = (SELECT office_id FROM office_contracts WHERE office_kind = 1) AND active = 1",
    )? {
        return Err(Rejection::FoundingInvariant);
    }
    transaction
        .execute(
            "INSERT INTO principals(principal_kind, display_name, active) VALUES (?1, ?2, 1)",
            params![PrincipalKind::Actor as i64, actor_display_name],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    let actor_principal = id_from_last_insert::<PrincipalId>(transaction)?;
    transaction
        .execute(
            "INSERT INTO office_occupancies(office_id, principal_id, active, appointed_by_command_id)
             VALUES (?1, ?2, 1, ?3)",
            params![office_id.value(), actor_principal.value(), command_row_id],
        )
        .map_err(|_| Rejection::ActiveOfficeOccupancyAlreadyExists)?;
    let occupancy_id = id_from_last_insert::<OfficeOccupancyId>(transaction)?;
    for capability in Capability::ROOT_AUTHORITY {
        transaction
            .execute(
                "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id,
                                                grant_state, grant_origin, granted_by_command_id, consumed_by_command_id)
                 VALUES (?1, ?2, ?3, 1, 2, ?4, NULL)",
                params![actor_principal.value(), capability as i64, occupancy_id.value(), command_row_id],
            )
            .map_err(|_| Rejection::FoundingInvariant)?;
    }
    Ok(EventBody::RootAuthorityAppointed {
        occupancy_id,
        principal_id: actor_principal,
    })
}

fn set_r0_hard_ceiling(
    transaction: &Transaction<'_>,
    _command_row_id: i64,
    ceiling: UsdMicros,
) -> Result<EventBody, Rejection> {
    let society_id = only_society_id(transaction)?;
    if exists(transaction, "SELECT 1 FROM society_bootstraps LIMIT 1")? {
        return Err(Rejection::FoundingInvariant);
    }
    if ceiling == UsdMicros::ZERO {
        return Err(Rejection::BudgetPolicyViolation);
    }
    Ok(EventBody::R0HardCeilingSet {
        society_id,
        ceiling,
    })
}

fn bootstrap_society(
    transaction: &Transaction<'_>,
    command_row_id: i64,
) -> Result<EventBody, Rejection> {
    let society_id = only_society_id(transaction)?;
    if exists(transaction, "SELECT 1 FROM society_bootstraps LIMIT 1")? {
        return Err(Rejection::FoundingInvariant);
    }
    let mission_id = active_founding_mission_id(transaction, society_id)?;
    let office_id = root_authority_office_id(transaction)?;
    let occupancy_id = active_root_authority_occupancy_id(transaction)?;
    let ceiling = hard_ceiling_from_event_body(transaction)?;
    transaction
        .execute(
            "INSERT INTO society_bootstraps(society_id, founding_mission_id, office_id, office_occupancy_id,
                                             hard_ceiling_micros, bootstrapped_by_command_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![society_id.value(), mission_id.value(), office_id.value(), occupancy_id.value(), ceiling.value(), command_row_id],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    transaction
        .execute(
            "UPDATE societies SET lifecycle_state = 2 WHERE society_id = ?1",
            [society_id.value()],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    let budget_envelope_id = create_budget_envelope(transaction, command_row_id, ceiling)?;
    transaction.execute(
        "INSERT INTO budget_envelope_constraints(budget_envelope_id, society_id, operating_cycle_id)
         VALUES (?1, ?2, NULL)",
        params![budget_envelope_id.value(), society_id.value()],
    ).map_err(|_| Rejection::FoundingInvariant)?;
    Ok(EventBody::SocietyBootstrapped { society_id })
}

fn propose_operating_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    treatment: OperatingCycleTreatment,
    budget_ceiling: UsdMicros,
) -> Result<EventBody, Rejection> {
    let (society_id, mission_id, occupancy_id, society_hard_ceiling) =
        bootstrapped_constitution(transaction)?;
    if budget_ceiling == UsdMicros::ZERO || budget_ceiling > society_hard_ceiling {
        return Err(Rejection::BudgetPolicyViolation);
    }
    if exists(
        transaction,
        "SELECT 1 FROM operating_cycles WHERE lifecycle_state NOT IN (7, 10, 11)",
    )? {
        return Err(Rejection::ActiveCycleAlreadyExists);
    }
    transaction.execute(
        "INSERT INTO operating_cycles(society_id, founding_mission_id, office_occupancy_id, treatment,
                                      budget_ceiling_micros, lifecycle_state, admission_generation,
                                      proposed_by_command_id, last_transition_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?7)",
        params![society_id.value(), mission_id.value(), occupancy_id.value(), treatment as i64,
                budget_ceiling.value(), OperatingCycleState::Proposed as i64, command_row_id],
    ).map_err(|_| Rejection::ActiveCycleAlreadyExists)?;
    let cycle_id = id_from_last_insert::<OperatingCycleId>(transaction)?;
    let budget_envelope_id = create_budget_envelope(transaction, command_row_id, budget_ceiling)?;
    transaction.execute(
        "INSERT INTO budget_envelope_constraints(budget_envelope_id, society_id, operating_cycle_id)
         VALUES (?1, NULL, ?2)",
        params![budget_envelope_id.value(), cycle_id.value()],
    ).map_err(|_| Rejection::FoundingInvariant)?;
    Ok(EventBody::OperatingCycleProposed {
        cycle_id,
        generation: AdmissionGeneration::INITIAL,
        treatment,
        budget_ceiling,
    })
}

fn admit_operating_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Proposed {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE operating_cycles SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE operating_cycle_id = ?3",
        params![OperatingCycleState::Admitted as i64, command_row_id, cycle_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "INSERT INTO operating_cycle_admissions(operating_cycle_id, admitted_by_command_id, started_by_command_id)
         VALUES (?1, ?2, NULL)",
        params![cycle_id.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: OperatingCycleState::Admitted,
        generation: cycle.generation,
    })
}

fn start_office_session(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Admitted {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE operating_cycles SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE operating_cycle_id = ?3",
        params![OperatingCycleState::Running as i64, command_row_id, cycle_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "UPDATE operating_cycle_admissions SET started_by_command_id = ?1 WHERE operating_cycle_id = ?2",
        params![command_row_id, cycle_id.value()],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction.execute(
        "INSERT INTO root_authority_office_sessions(operating_cycle_id, office_occupancy_id, lifecycle_state,
                                                      started_by_command_id, last_transition_command_id)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![cycle_id.value(), cycle.occupancy_id.value(), OfficeSessionState::Reserved as i64, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::RootAuthorityOfficeSessionStarted {
        session_id: id_from_last_insert::<RootAuthorityOfficeSessionId>(transaction)?,
        cycle_id,
    })
}

fn record_office_session_ready(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: RootAuthorityOfficeSessionId,
) -> Result<EventBody, Rejection> {
    let (state, cycle_id) = session_row(transaction, session_id)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if state != OfficeSessionState::Reserved || cycle.state != OperatingCycleState::Running {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    // An Office session is an operational authority, not a free-standing
    // kernel-service assertion. M5 therefore requires the exact supervised
    // Pi admission and the correlated SessionReady protocol fact before a
    // session can authorize ordinary Office turns.
    let supervised_admission: Option<i64> = transaction
        .query_row(
            "SELECT native_child_spawn_admission_id FROM native_child_spawn_admissions
          WHERE root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?;
    let Some(admission_id) = supervised_admission else {
        return Err(Rejection::ChildLifecycleReceiptMissing);
    };
    let session_ready: bool = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM native_children c
               JOIN pi_child_session_protocols p ON p.native_child_id = c.native_child_id
              WHERE c.native_child_spawn_admission_id = ?1
                AND c.lifecycle_state = ?2
                AND p.lifecycle_state = ?3)",
            params![
                admission_id,
                ChildProcessState::Running as i64,
                PiChildSessionState::SessionReady as i64,
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?
        != 0;
    if !session_ready {
        return Err(Rejection::ChildLifecycleReceiptMissing);
    }
    transaction.execute(
        "UPDATE root_authority_office_sessions SET lifecycle_state = ?1, last_transition_command_id = ?2
         WHERE root_authority_office_session_id = ?3",
        params![OfficeSessionState::Ready as i64, command_row_id, session_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    Ok(EventBody::RootAuthorityOfficeSessionStateChanged {
        session_id,
        state: OfficeSessionState::Ready,
    })
}

/// The kernel records the observed terminal classification after its supervisor
/// has collected process/session evidence. `Closed` is a reconciliation fact;
/// cancellation and failure are separate durable classifications rather than a
/// convenient way to make an unsafe session look normally closed.
fn record_office_session_terminal(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: RootAuthorityOfficeSessionId,
    terminal_state: OfficeSessionTerminalState,
) -> Result<EventBody, Rejection> {
    let (state, cycle_id) = session_row(transaction, session_id)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if session_has_active_turn(transaction, session_id)? {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    // M5 has only physical child receipts. It must not let the older atomic
    // Office terminal fact manufacture a semantic Pi/Office settlement for a
    // supervised session; a later normalized receipt owns that transition.
    let has_pi_child: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM native_child_spawn_admissions WHERE root_authority_office_session_id = ?1)",
        [session_id.value()], |row| row.get::<_, i64>(0),
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)? != 0;
    if has_pi_child {
        return Err(Rejection::SupervisedTerminalReceiptRequired);
    }
    let next_state = match terminal_state {
        OfficeSessionTerminalState::Closed
            if state == OfficeSessionState::Ready
                && cycle.state == OperatingCycleState::Reconciling =>
        {
            OfficeSessionState::Closed
        }
        OfficeSessionTerminalState::Cancelled
            if matches!(
                state,
                OfficeSessionState::Reserved | OfficeSessionState::Ready
            ) && cycle.state == OperatingCycleState::Cancelling =>
        {
            OfficeSessionState::Cancelled
        }
        OfficeSessionTerminalState::Failed
            if !matches!(
                state,
                OfficeSessionState::Closed
                    | OfficeSessionState::Cancelled
                    | OfficeSessionState::Failed
            ) && cycle.state.is_nonterminal() =>
        {
            OfficeSessionState::Failed
        }
        _ => return Err(Rejection::InvalidLifecycleTransition),
    };
    transaction
        .execute(
            "UPDATE root_authority_office_sessions SET lifecycle_state = ?1, last_transition_command_id = ?2
             WHERE root_authority_office_session_id = ?3",
            params![
                next_state as i64,
                command_row_id,
                session_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    Ok(EventBody::RootAuthorityOfficeSessionStateChanged {
        session_id,
        state: next_state,
    })
}

fn open_office_turn(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: RootAuthorityOfficeSessionId,
    purpose: OfficeTurnPurpose,
) -> Result<EventBody, Rejection> {
    let (state, cycle_id) = session_row(transaction, session_id)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    // Ready is a point-in-time protocol fact, not a perpetual Office
    // authority. Every turn rechecks the exact supervised child: after a
    // reap, recovery parentage loss, or containment failure, M5 has no
    // normalized semantic Office settlement and therefore admits no new work.
    let supervised_admission: Option<i64> = transaction
        .query_row(
            "SELECT native_child_spawn_admission_id FROM native_child_spawn_admissions
              WHERE root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    let Some(admission_id) = supervised_admission else {
        return Err(Rejection::ChildLifecycleReceiptMissing);
    };
    let child_is_operational: bool = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM native_children c
               JOIN pi_child_session_protocols p ON p.native_child_id = c.native_child_id
              WHERE c.native_child_spawn_admission_id = ?1
                AND c.lifecycle_state = ?2
                AND p.lifecycle_state = ?3)",
            params![
                admission_id,
                ChildProcessState::Running as i64,
                PiChildSessionState::SessionReady as i64,
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?
        != 0;
    if !child_is_operational {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let purpose_is_admitted = match purpose {
        OfficeTurnPurpose::OrdinaryWork => cycle.state.admits_task_work(),
        OfficeTurnPurpose::Recovery
        | OfficeTurnPurpose::Cancellation
        | OfficeTurnPurpose::Closure => matches!(
            cycle.state,
            OperatingCycleState::Quiescing
                | OperatingCycleState::Drained
                | OperatingCycleState::Reconciling
        ),
    };
    if state != OfficeSessionState::Ready || !purpose_is_admitted {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE root_authority_office_sessions SET lifecycle_state = ?1, last_transition_command_id = ?2
         WHERE root_authority_office_session_id = ?3",
        params![OfficeSessionState::TurnActive as i64, command_row_id, session_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "INSERT INTO office_turns(root_authority_office_session_id, lifecycle_state, purpose, opened_by_command_id, settled_by_command_id)
         VALUES (?1, ?2, ?3, ?4, NULL)",
        params![session_id.value(), OfficeTurnState::Active as i64, purpose as i64, command_row_id],
    ).map_err(|_| Rejection::SessionTurnAlreadyActive)?;
    Ok(EventBody::OfficeTurnOpened {
        turn_id: id_from_last_insert::<OfficeTurnId>(transaction)?,
        session_id,
        purpose,
    })
}

/// M6 Prompt authority is deliberately narrower than an Office turn: it is a
/// kernel-only, one-shot authorization for exact already-sealed bytes at the
/// current ledger frontier. The checkpoint names the existing Office-session
/// reservation and records no second reservation or routine per-turn quota.
fn authorize_pi_office_turn_prompt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    input: PiOfficeTurnPromptAuthorizationInput<'_>,
) -> Result<EventBody, Rejection> {
    let PiOfficeTurnPromptAuthorizationInput {
        expected_generation,
        office_turn_id,
        correlation_identity,
        prompt_content_object_id,
        prompt_digest,
        frontier_event_id,
    } = input;
    let row: (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) = transaction
        .query_row(
            "SELECT t.lifecycle_state, t.purpose, t.root_authority_office_session_id,
                    s.lifecycle_state, s.operating_cycle_id, c.treatment,
                    a.budget_reservation_id, sidecar.pi_session_id, a.execution_profile_id,
                    p.native_child_id, p.lifecycle_state, proto.lifecycle_state
             FROM office_turns t
             JOIN root_authority_office_sessions s
               ON s.root_authority_office_session_id = t.root_authority_office_session_id
             JOIN operating_cycles c ON c.operating_cycle_id = s.operating_cycle_id
             JOIN native_child_spawn_admissions a
               ON a.root_authority_office_session_id = s.root_authority_office_session_id
             JOIN pi_child_spawn_sidecars sidecar
               ON sidecar.native_child_spawn_admission_id = a.native_child_spawn_admission_id
             JOIN native_children p ON p.native_child_spawn_admission_id = a.native_child_spawn_admission_id
             JOIN pi_child_session_protocols proto ON proto.native_child_id = p.native_child_id
             WHERE t.office_turn_id = ?1",
            [office_turn_id.value()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?, r.get(10)?, r.get(11)?)),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeTurnAuthorityMissing)?
        .ok_or(Rejection::PiOfficeTurnAuthorityMissing)?;
    let cycle_id =
        OperatingCycleId::try_from(row.4).map_err(|_| Rejection::PiOfficeTurnAuthorityMissing)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if row.0 != OfficeTurnState::Active as i64
        || row.1 != OfficeTurnPurpose::OrdinaryWork as i64
        || row.3 != OfficeSessionState::TurnActive as i64
        || cycle.state != OperatingCycleState::Running
        || row.5 != OperatingCycleTreatment::DeterministicPiHostFixtureV1 as i64
        || row.10 != ChildProcessState::Running as i64
        || row.11 != PiChildSessionState::SessionReady as i64
        || active_cancellation_for_cycle(transaction, cycle_id)?.is_some()
    {
        return Err(Rejection::PiOfficeTurnTreatmentIneligible);
    }
    let profile: (i64, i64) = transaction
        .query_row(
            "SELECT profile_kind, readiness FROM execution_profiles WHERE execution_profile_id = ?1",
            [row.8],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::ExecutionProfileIneligible)?
        .ok_or(Rejection::ExecutionProfileIneligible)?;
    if profile.0 != ExecutionProfileKind::DeterministicPiHostProcessDoubleV1 as i64
        || profile.1 != ExecutionProfileReadiness::DeterministicFixtureOnly as i64
    {
        return Err(Rejection::ExecutionProfileIneligible);
    }
    let reservation_id = BudgetReservationId::try_from(row.6)
        .map_err(|_| Rejection::PiOfficeTurnAuthorityMissing)?;
    let session_id = RootAuthorityOfficeSessionId::try_from(row.2)
        .map_err(|_| Rejection::PiOfficeTurnAuthorityMissing)?;
    let (mapped_reservation, reservation_state, charged, amount): (i64, i64, i64, i64) = transaction
        .query_row(
            "SELECT o.budget_reservation_id, r.reservation_state, r.charged_micros, r.amount_micros
             FROM office_session_budget_reservations o
             JOIN budget_reservations r ON r.budget_reservation_id = o.budget_reservation_id
             WHERE o.root_authority_office_session_id = ?1",
            [session_id.value()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeTurnAuthorityMissing)?
        .ok_or(Rejection::PiOfficeTurnAuthorityMissing)?;
    if mapped_reservation != reservation_id.value()
        || reservation_state != BudgetReservationState::Reserved as i64
        || charged < 0
        || amount < charged
    {
        return Err(Rejection::ReservationNotActive);
    }
    let latest_usage: i64 = transaction
        .query_row(
            "SELECT COALESCE(MAX(cumulative_ceiling_micros), 0)
             FROM pi_office_turn_usage_receipts WHERE pi_session_id = ?1",
            [row.7],
            |r| r.get(0),
        )
        .map_err(|_| Rejection::PiOfficeTurnUsageNotMonotonic)?;
    if latest_usage != charged {
        return Err(Rejection::PiOfficeTurnUsageNotMonotonic);
    }
    let current_frontier: Option<i64> = transaction
        .query_row(
            "SELECT event_id FROM events ORDER BY event_sequence DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeTurnAuthorityMissing)?;
    if current_frontier != Some(frontier_event_id.value()) {
        return Err(Rejection::StaleAdmissionGeneration);
    }
    let content_digest: Option<Vec<u8>> = transaction
        .query_row(
            "SELECT seal.digest FROM content_objects object
             JOIN content_seal_receipts seal ON seal.content_seal_receipt_id = object.content_seal_receipt_id
             WHERE object.content_object_id = ?1",
            [prompt_content_object_id.value()],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeTurnPromptBindingMismatch)?;
    if content_digest.as_deref() != Some(prompt_digest.as_bytes().as_slice()) {
        return Err(Rejection::PiOfficeTurnPromptBindingMismatch);
    }
    transaction.execute(
        "INSERT INTO office_turn_budget_checkpoints(office_turn_id, root_authority_office_session_id, budget_reservation_id, baseline_cumulative_micros, authorized_by_command_id, settled_cumulative_micros, settled_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL)",
        params![office_turn_id.value(), session_id.value(), reservation_id.value(), charged, command_row_id],
    ).map_err(|_| Rejection::PiOfficeTurnAuthorityMissing)?;
    transaction.execute(
        "INSERT INTO pi_office_turn_prompt_authorizations(office_turn_id, native_child_id, pi_session_id, budget_reservation_id, correlation_identity, prompt_content_object_id, prompt_digest, frontier_event_id, admission_generation, office_turn_purpose, authorized_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![office_turn_id.value(), row.9, row.7, reservation_id.value(), correlation_identity.as_str(), prompt_content_object_id.value(), prompt_digest.as_bytes().as_slice(), frontier_event_id.value(), cycle.generation.value(), row.1, command_row_id],
    ).map_err(|_| Rejection::PiOfficeTurnAuthorityMissing)?;
    Ok(EventBody::PiOfficeTurnPromptAuthorized {
        pi_office_turn_prompt_authorization_id: id_from_last_insert(transaction)?,
        office_turn_id,
        native_child_id: NativeChildId::try_from(row.9)
            .map_err(|_| Rejection::PiOfficeTurnAuthorityMissing)?,
        correlation_identity: correlation_identity.clone(),
        budget_reservation_id: reservation_id,
    })
}

fn record_pi_office_turn_prompt_delivery(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    office_turn_id: OfficeTurnId,
    correlation_identity: &PiCorrelationIdentity,
    prompt_digest: Blake3Digest,
) -> Result<EventBody, Rejection> {
    let authorization: Option<i64> = transaction.query_row(
        "SELECT pi_office_turn_prompt_authorization_id FROM pi_office_turn_prompt_authorizations
         WHERE office_turn_id = ?1 AND correlation_identity = ?2 AND prompt_digest = ?3",
        params![office_turn_id.value(), correlation_identity.as_str(), prompt_digest.as_bytes().as_slice()],
        |r| r.get(0),
    ).optional().map_err(|_| Rejection::PiOfficeTurnPromptBindingMismatch)?;
    let authorization = PiOfficeTurnPromptAuthorizationId::try_from(
        authorization.ok_or(Rejection::PiOfficeTurnPromptBindingMismatch)?,
    )
    .map_err(|_| Rejection::PiOfficeTurnPromptBindingMismatch)?;
    transaction.execute(
        "INSERT INTO pi_office_turn_prompt_deliveries(office_turn_id, pi_office_turn_prompt_authorization_id, correlation_identity, prompt_digest, delivered_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![office_turn_id.value(), authorization.value(), correlation_identity.as_str(), prompt_digest.as_bytes().as_slice(), command_row_id],
    ).map_err(|_| Rejection::PiOfficeTurnPromptBindingMismatch)?;
    Ok(EventBody::PiOfficeTurnPromptDelivered {
        office_turn_id,
        correlation_identity: correlation_identity.clone(),
    })
}

fn record_pi_office_turn_prompt_accepted(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    office_turn_id: OfficeTurnId,
    correlation_identity: &PiCorrelationIdentity,
    command_result_sequence: PiProtocolSequence,
) -> Result<EventBody, Rejection> {
    let authorization: Option<(i64, i64)> = transaction
        .query_row(
            "SELECT d.pi_office_turn_prompt_authorization_id, a.pi_session_id
         FROM pi_office_turn_prompt_deliveries d
         JOIN pi_office_turn_prompt_authorizations a
           ON a.pi_office_turn_prompt_authorization_id = d.pi_office_turn_prompt_authorization_id
         WHERE d.office_turn_id = ?1 AND a.correlation_identity = ?2",
            params![office_turn_id.value(), correlation_identity.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeTurnPromptBindingMismatch)?;
    let (authorization, pi_session_id) =
        authorization.ok_or(Rejection::PiOfficeTurnPromptBindingMismatch)?;
    let authorization = PiOfficeTurnPromptAuthorizationId::try_from(authorization)
        .map_err(|_| Rejection::PiOfficeTurnPromptBindingMismatch)?;
    if pi_office_session_max_sequence(transaction, pi_session_id)?
        .is_some_and(|previous| command_result_sequence.value() <= previous)
    {
        return Err(Rejection::PiOfficeTurnUsageNotMonotonic);
    }
    transaction.execute(
        "INSERT INTO pi_office_turn_prompt_acceptances(office_turn_id, pi_office_turn_prompt_authorization_id, command_result_sequence, accepted_by_command_id)
         VALUES (?1, ?2, ?3, ?4)",
        params![office_turn_id.value(), authorization.value(), command_result_sequence.value(), command_row_id],
    ).map_err(|_| Rejection::PiOfficeTurnPromptBindingMismatch)?;
    Ok(EventBody::PiOfficeTurnPromptAccepted {
        office_turn_id,
        correlation_identity: correlation_identity.clone(),
        command_result_sequence,
    })
}

fn record_pi_office_turn_usage(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    office_turn_id: OfficeTurnId,
    correlation_identity: &PiCorrelationIdentity,
    protocol_sequence: PiProtocolSequence,
    usage: PiCumulativeUsage,
) -> Result<EventBody, Rejection> {
    if pi_office_turn_has_terminal_receipt(transaction, office_turn_id)? {
        return Err(Rejection::PiOfficeTurnTerminalAlreadyRecorded);
    }
    if !usage.is_internally_consistent() {
        return Err(Rejection::PiOfficeTurnUsageNotMonotonic);
    }
    let authorization: Option<(i64, i64, i64)> = transaction.query_row(
        "SELECT a.pi_office_turn_prompt_authorization_id, a.pi_session_id,
                accepted.command_result_sequence
         FROM pi_office_turn_prompt_authorizations a
         JOIN pi_office_turn_prompt_acceptances accepted
           ON accepted.pi_office_turn_prompt_authorization_id = a.pi_office_turn_prompt_authorization_id
         WHERE a.office_turn_id = ?1 AND a.correlation_identity = ?2",
        params![office_turn_id.value(), correlation_identity.as_str()],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).optional().map_err(|_| Rejection::PiOfficeTurnPromptBindingMismatch)?;
    let (authorization, session_id, accepted_sequence) =
        authorization.ok_or(Rejection::PiOfficeTurnPromptBindingMismatch)?;
    if protocol_sequence.value() <= accepted_sequence {
        return Err(Rejection::PiOfficeTurnUsageNotMonotonic);
    }
    if pi_office_session_max_sequence(transaction, session_id)?
        .is_some_and(|previous| protocol_sequence.value() <= previous)
    {
        return Err(Rejection::PiOfficeTurnUsageNotMonotonic);
    }
    let session_already_frozen: i64 = transaction
        .query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM pi_office_turn_usage_failures WHERE pi_session_id = ?1
             )",
            [session_id],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::PiOfficeTurnUsageNotMonotonic)?;
    if session_already_frozen != 0 {
        return Err(Rejection::PiOfficeTurnUsageAlreadyFrozen);
    }
    let previous: Option<PiOfficeTurnUsageSqlRow> = transaction.query_row(
        "SELECT input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, total_tokens,
                provider_cost_binary64, cumulative_ceiling_micros, protocol_sequence
         FROM pi_office_turn_usage_receipts WHERE pi_session_id = ?1
         ORDER BY protocol_sequence DESC LIMIT 1",
        [session_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
    ).optional().map_err(|_| Rejection::PiOfficeTurnUsageNotMonotonic)?;
    if let Some(previous) = previous {
        let previous_usage = pi_cumulative_usage_from_sql(
            previous.0,
            previous.1,
            previous.2,
            previous.3,
            previous.4,
            &previous.5,
            previous.6,
        )
        .map_err(|_| Rejection::PiOfficeTurnUsageNotMonotonic)?;
        if protocol_sequence.value() <= previous.7
            || usage.input_tokens < previous_usage.input_tokens
            || usage.output_tokens < previous_usage.output_tokens
            || usage.cache_read_tokens < previous_usage.cache_read_tokens
            || usage.cache_write_tokens < previous_usage.cache_write_tokens
            || usage.total_tokens < previous_usage.total_tokens
            || usage.provider_cost < previous_usage.provider_cost
            || usage.ceiling_micro_usd < previous_usage.ceiling_micro_usd
        {
            return Err(Rejection::PiOfficeTurnUsageNotMonotonic);
        }
    }
    transaction.execute(
        "INSERT INTO pi_office_turn_usage_receipts(office_turn_id, pi_office_turn_prompt_authorization_id, pi_session_id, correlation_identity, protocol_sequence, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, total_tokens, provider_cost_binary64, cumulative_ceiling_micros, recorded_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![office_turn_id.value(), authorization, session_id, correlation_identity.as_str(), protocol_sequence.value(), usage.input_tokens.value(), usage.output_tokens.value(), usage.cache_read_tokens.value(), usage.cache_write_tokens.value(), usage.total_tokens.value(), usage.provider_cost.as_big_endian_bytes().as_slice(), usage.ceiling_micro_usd.value(), command_row_id],
    ).map_err(|_| Rejection::PiOfficeTurnUsageNotMonotonic)?;
    Ok(EventBody::PiOfficeTurnUsageRecorded {
        pi_office_turn_usage_receipt_id: id_from_last_insert(transaction)?,
        office_turn_id,
        protocol_sequence,
        cumulative_micro_usd: usage.ceiling_micro_usd,
    })
}

fn record_pi_office_turn_usage_failure(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    office_turn_id: OfficeTurnId,
    correlation_identity: &PiCorrelationIdentity,
    protocol_sequence: PiProtocolSequence,
    failure: PiOfficeTurnUsageFailure,
) -> Result<EventBody, Rejection> {
    if pi_office_turn_has_terminal_receipt(transaction, office_turn_id)? {
        return Err(Rejection::PiOfficeTurnTerminalAlreadyRecorded);
    }
    let row: Option<(i64, i64, i64, i64, i64)> = transaction.query_row(
        "SELECT a.pi_office_turn_prompt_authorization_id, a.budget_reservation_id, s.operating_cycle_id,
                a.pi_session_id, accepted.command_result_sequence
         FROM pi_office_turn_prompt_authorizations a
         JOIN office_turns t ON t.office_turn_id = a.office_turn_id
         JOIN root_authority_office_sessions s ON s.root_authority_office_session_id = t.root_authority_office_session_id
         JOIN pi_office_turn_prompt_acceptances accepted ON accepted.pi_office_turn_prompt_authorization_id = a.pi_office_turn_prompt_authorization_id
         WHERE a.office_turn_id = ?1 AND a.correlation_identity = ?2",
        params![office_turn_id.value(), correlation_identity.as_str()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    ).optional().map_err(|_| Rejection::PiOfficeTurnPromptBindingMismatch)?;
    let (authorization, reservation, cycle, session, accepted_sequence) =
        row.ok_or(Rejection::PiOfficeTurnPromptBindingMismatch)?;
    let previous_sequence = pi_office_session_max_sequence(transaction, session)?;
    if protocol_sequence.value() <= accepted_sequence
        || previous_sequence.is_some_and(|previous| protocol_sequence.value() <= previous)
    {
        return Err(Rejection::PiOfficeTurnUsageNotMonotonic);
    }
    let reservation_id =
        BudgetReservationId::try_from(reservation).map_err(|_| Rejection::ReservationNotActive)?;
    let cycle_id = OperatingCycleId::try_from(cycle).map_err(|_| Rejection::SubjectNotFound)?;
    let (amount, charged, state): (i64, i64, i64) = transaction.query_row(
        "SELECT amount_micros, charged_micros, reservation_state FROM budget_reservations WHERE budget_reservation_id = ?1",
        [reservation], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).map_err(|_| Rejection::ReservationNotActive)?;
    if state != BudgetReservationState::Reserved as i64 || amount < charged {
        return Err(Rejection::ReservationNotActive);
    }
    let freeze_reason = match failure {
        PiOfficeTurnUsageFailure::Unknown(
            PiOfficeTurnUsageUnknownReason::MissingFinalUsageSnapshot,
        ) => BudgetFreezeReason::Unknown(CostUnknownReason::ProviderDidNotReport),
        PiOfficeTurnUsageFailure::Unknown(
            PiOfficeTurnUsageUnknownReason::BoundaryStreamInterrupted,
        ) => BudgetFreezeReason::Unknown(CostUnknownReason::AdapterStreamInterrupted),
        PiOfficeTurnUsageFailure::Unknown(
            PiOfficeTurnUsageUnknownReason::TerminalEvidenceMissing,
        ) => BudgetFreezeReason::Unknown(CostUnknownReason::ReconciliationMismatch),
        PiOfficeTurnUsageFailure::Unavailable(_) => {
            BudgetFreezeReason::Unavailable(CostUnavailableReason::AdapterAccountingUnavailable)
        }
    };
    let frozen = freeze_budget_admission(
        transaction,
        command_row_id,
        reservation_id,
        cycle_id,
        UsdMicros::try_from(amount - charged).map_err(|_| Rejection::ReservationNotActive)?,
        freeze_reason,
    )?;
    let (cancellation_request_id, postmortem_id) = match frozen {
        EventBody::BudgetAdmissionFrozen {
            cancellation_request_id,
            postmortem_id,
            ..
        } => (cancellation_request_id, postmortem_id),
        _ => return Err(Rejection::PiOfficeTurnAuthorityMissing),
    };
    let (kind, unknown, unavailable) = sql_pi_office_turn_usage_failure(failure);
    transaction.execute(
        "INSERT INTO pi_office_turn_usage_failures(office_turn_id, pi_office_turn_prompt_authorization_id, pi_session_id, correlation_identity, protocol_sequence, failure_kind, unknown_reason, unavailable_reason, budget_reservation_id, cancellation_request_id, cost_postmortem_id, recorded_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![office_turn_id.value(), authorization, session, correlation_identity.as_str(), protocol_sequence.value(), kind, unknown, unavailable, reservation, cancellation_request_id.value(), postmortem_id.value(), command_row_id],
    ).map_err(|_| Rejection::PiOfficeTurnAuthorityMissing)?;
    Ok(EventBody::PiOfficeTurnUsageFrozen {
        office_turn_id,
        budget_reservation_id: reservation_id,
        cancellation_request_id,
        postmortem_id,
        failure,
    })
}

/// Resolves the one supervised Pi child and parent reservation which are
/// structurally bound to an Office session. Dispose is intentionally not a
/// generic child terminal shortcut: it can begin only after the session is
/// idle, quiesced, and still owns a live, peer-ready child.
fn pi_office_session_dispose_binding(
    transaction: &Transaction<'_>,
    session_id: RootAuthorityOfficeSessionId,
) -> Result<PiOfficeSessionDisposeBindingSqlRow, Rejection> {
    transaction
        .query_row(
            "SELECT s.lifecycle_state, s.operating_cycle_id, parent.budget_reservation_id,
                    child.native_child_id, sidecar.pi_session_id,
                    child.lifecycle_state, protocol.lifecycle_state
             FROM root_authority_office_sessions s
             JOIN office_session_budget_reservations parent
               ON parent.root_authority_office_session_id = s.root_authority_office_session_id
             JOIN native_child_spawn_admissions admission
               ON admission.root_authority_office_session_id = s.root_authority_office_session_id
             JOIN pi_child_spawn_sidecars sidecar
               ON sidecar.native_child_spawn_admission_id = admission.native_child_spawn_admission_id
             JOIN native_children child
               ON child.native_child_spawn_admission_id = admission.native_child_spawn_admission_id
             JOIN pi_child_session_protocols protocol
               ON protocol.native_child_id = child.native_child_id
             WHERE s.root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?
        .ok_or(Rejection::PiOfficeSessionDisposeBindingMismatch)
}

fn pi_office_session_has_active_turn(
    transaction: &Transaction<'_>,
    session_id: RootAuthorityOfficeSessionId,
) -> Result<bool, Rejection> {
    transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM office_turns
             WHERE root_authority_office_session_id = ?1 AND lifecycle_state = ?2)",
            params![session_id.value(), OfficeTurnState::Active as i64],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)
}

fn require_pi_office_session_dispose_authorization_generation(
    transaction: &Transaction<'_>,
    expected_generation: ExpectedGeneration,
    session_id: RootAuthorityOfficeSessionId,
) -> Result<(), Rejection> {
    let authorized_generation: i64 = transaction
        .query_row(
            "SELECT authorized_generation FROM pi_office_session_dispose_authorizations
             WHERE root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?
        .ok_or(Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    match expected_generation {
        ExpectedGeneration::Exact(generation) if generation.value() == authorized_generation => {
            Ok(())
        }
        ExpectedGeneration::Exact(_) => Err(Rejection::StaleAdmissionGeneration),
        ExpectedGeneration::NotApplicable => Err(Rejection::InvalidExpectedGeneration),
    }
}

fn authorize_pi_office_session_dispose(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: RootAuthorityOfficeSessionId,
    correlation_identity: &PiCorrelationIdentity,
) -> Result<EventBody, Rejection> {
    let (session_state, cycle, _reservation, child, pi_session, child_state, protocol_state) =
        pi_office_session_dispose_binding(transaction, session_id)?;
    let cycle_id = OperatingCycleId::try_from(cycle)
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if session_state != OfficeSessionState::Ready as i64
        || cycle.state != OperatingCycleState::Quiescing
        || child_state != ChildProcessState::Running as i64
        || protocol_state != PiChildSessionState::SessionReady as i64
        || pi_office_session_has_active_turn(transaction, session_id)?
    {
        return Err(Rejection::PiOfficeSessionDisposeBindingMismatch);
    }
    let correlation_already_used: i64 = transaction
        .query_row(
            "SELECT EXISTS(
             SELECT 1 FROM pi_office_turn_prompt_authorizations
             WHERE pi_session_id = ?1 AND correlation_identity = ?2
             UNION ALL
             SELECT 1 FROM pi_office_session_dispose_authorizations
             WHERE pi_session_id = ?1 AND correlation_identity = ?2
         )",
            params![pi_session, correlation_identity.as_str()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    transaction
        .execute(
            "UPDATE root_authority_office_sessions
         SET lifecycle_state = ?1, last_transition_command_id = ?2
         WHERE root_authority_office_session_id = ?3",
            params![
                OfficeSessionState::Quiescing as i64,
                command_row_id,
                session_id.value()
            ],
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    if correlation_already_used != 0 {
        return Err(Rejection::PiOfficeSessionDisposeBindingMismatch);
    }
    let (reservation_state, charged): (i64, i64) = transaction
        .query_row(
            "SELECT reservation.reservation_state, reservation.charged_micros
         FROM office_session_budget_reservations parent
         JOIN budget_reservations reservation
           ON reservation.budget_reservation_id = parent.budget_reservation_id
         WHERE parent.root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    let latest_known: Option<i64> = transaction
        .query_row(
            "SELECT cumulative_ceiling_micros FROM pi_office_turn_usage_receipts
         WHERE pi_session_id = ?1 ORDER BY protocol_sequence DESC LIMIT 1",
            [pi_session],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    let has_usage_failure: i64 = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pi_office_turn_usage_failures WHERE pi_session_id = ?1)",
            [pi_session],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    if reservation_state != BudgetReservationState::Reserved as i64
        || latest_known.unwrap_or(0) != charged
        || has_usage_failure != 0
    {
        return Err(Rejection::PiOfficeSessionDisposeBindingMismatch);
    }
    transaction
        .execute(
            "INSERT INTO pi_office_session_dispose_authorizations(
             root_authority_office_session_id, native_child_id, pi_session_id,
             correlation_identity, authorized_generation, authorized_by_command_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session_id.value(),
                child,
                pi_session,
                correlation_identity.as_str(),
                cycle.generation.value(),
                command_row_id
            ],
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    Ok(EventBody::PiOfficeSessionDisposeAuthorized {
        session_id,
        native_child_id: NativeChildId::try_from(child)
            .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?,
        correlation_identity: correlation_identity.clone(),
        authorized_generation: cycle.generation,
    })
}

fn record_pi_office_session_dispose_delivery(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: RootAuthorityOfficeSessionId,
    correlation_identity: &PiCorrelationIdentity,
) -> Result<EventBody, Rejection> {
    require_pi_office_session_dispose_authorization_generation(
        transaction,
        expected_generation,
        session_id,
    )?;
    let authorization: Option<(i64, i64, String, i64)> = transaction.query_row(
        "SELECT authorization.native_child_id, authorization.pi_session_id,
                authorization.correlation_identity, session.lifecycle_state
         FROM pi_office_session_dispose_authorizations authorization
         JOIN root_authority_office_sessions session
           ON session.root_authority_office_session_id = authorization.root_authority_office_session_id
         WHERE authorization.root_authority_office_session_id = ?1",
        [session_id.value()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).optional().map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    let (child, pi_session, stored_correlation, session_state) =
        authorization.ok_or(Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    if stored_correlation != correlation_identity.as_str()
        || session_state != OfficeSessionState::Quiescing as i64
        || pi_office_session_has_active_turn(transaction, session_id)?
    {
        return Err(Rejection::PiOfficeSessionDisposeBindingMismatch);
    }
    transaction
        .execute(
            "INSERT INTO pi_office_session_dispose_deliveries(
             root_authority_office_session_id, native_child_id, pi_session_id,
             correlation_identity, delivered_by_command_id
         ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session_id.value(),
                child,
                pi_session,
                correlation_identity.as_str(),
                command_row_id
            ],
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    Ok(EventBody::PiOfficeSessionDisposeDelivered {
        session_id,
        native_child_id: NativeChildId::try_from(child)
            .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?,
        correlation_identity: correlation_identity.clone(),
    })
}

fn record_pi_office_session_dispose_accepted(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: RootAuthorityOfficeSessionId,
    correlation_identity: &PiCorrelationIdentity,
    command_result_sequence: PiProtocolSequence,
) -> Result<EventBody, Rejection> {
    require_pi_office_session_dispose_authorization_generation(
        transaction,
        expected_generation,
        session_id,
    )?;
    let row: Option<(i64, String, i64)> = transaction
        .query_row(
            "SELECT d.pi_session_id, d.correlation_identity, s.lifecycle_state
         FROM pi_office_session_dispose_deliveries d
         JOIN root_authority_office_sessions s
           ON s.root_authority_office_session_id = d.root_authority_office_session_id
         WHERE d.root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    let (pi_session, stored_correlation, session_state) =
        row.ok_or(Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    if stored_correlation != correlation_identity.as_str()
        || session_state != OfficeSessionState::Quiescing as i64
    {
        return Err(Rejection::PiOfficeSessionDisposeBindingMismatch);
    }
    if pi_office_session_max_sequence(transaction, pi_session)?
        .is_some_and(|previous| command_result_sequence.value() <= previous)
    {
        return Err(Rejection::PiOfficeSessionDisposeUsageNotMonotonic);
    }
    transaction
        .execute(
            "INSERT INTO pi_office_session_dispose_acceptances(
             root_authority_office_session_id, command_result_sequence, accepted_by_command_id
         ) VALUES (?1, ?2, ?3)",
            params![
                session_id.value(),
                command_result_sequence.value(),
                command_row_id
            ],
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    Ok(EventBody::PiOfficeSessionDisposeAccepted {
        session_id,
        correlation_identity: correlation_identity.clone(),
        command_result_sequence,
    })
}

fn record_pi_office_session_dispose_usage(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: RootAuthorityOfficeSessionId,
    correlation_identity: &PiCorrelationIdentity,
    protocol_sequence: PiProtocolSequence,
    usage: PiCumulativeUsage,
) -> Result<EventBody, Rejection> {
    require_pi_office_session_dispose_authorization_generation(
        transaction,
        expected_generation,
        session_id,
    )?;
    if !usage.is_internally_consistent() {
        return Err(Rejection::PiOfficeSessionDisposeUsageNotMonotonic);
    }
    let row: Option<(i64, String, i64, i64)> = transaction
        .query_row(
            "SELECT d.pi_session_id, d.correlation_identity, a.command_result_sequence,
                s.lifecycle_state
         FROM pi_office_session_dispose_deliveries d
         JOIN pi_office_session_dispose_acceptances a
           ON a.root_authority_office_session_id = d.root_authority_office_session_id
         JOIN root_authority_office_sessions s
           ON s.root_authority_office_session_id = d.root_authority_office_session_id
         WHERE d.root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    let (pi_session, stored_correlation, accepted_sequence, session_state) =
        row.ok_or(Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    if stored_correlation != correlation_identity.as_str()
        || session_state != OfficeSessionState::Quiescing as i64
    {
        return Err(Rejection::PiOfficeSessionDisposeBindingMismatch);
    }
    if accepted_sequence.checked_add(1) != Some(protocol_sequence.value())
        || pi_office_session_max_sequence(transaction, pi_session)?
            .is_some_and(|previous| protocol_sequence.value() <= previous)
    {
        return Err(Rejection::PiOfficeSessionDisposeUsageNotMonotonic);
    }
    let prior: Option<PiOfficeTurnUsageSqlRow> = transaction.query_row(
        "SELECT input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, total_tokens,
                provider_cost_binary64, cumulative_ceiling_micros, protocol_sequence
         FROM (
             SELECT input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, total_tokens,
                    provider_cost_binary64, cumulative_ceiling_micros, protocol_sequence
             FROM pi_office_turn_usage_receipts WHERE pi_session_id = ?1
             UNION ALL
             SELECT input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, total_tokens,
                    provider_cost_binary64, cumulative_ceiling_micros, protocol_sequence
             FROM pi_office_session_dispose_usage_receipts WHERE pi_session_id = ?1
         ) ORDER BY protocol_sequence DESC LIMIT 1",
        [pi_session],
        |row| {
            Ok((
                row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?,
                row.get(6)?, row.get(7)?,
            ))
        },
    ).optional().map_err(|_| Rejection::PiOfficeSessionDisposeUsageNotMonotonic)?;
    if let Some(prior) = prior {
        let prior_usage = pi_cumulative_usage_from_sql(
            prior.0, prior.1, prior.2, prior.3, prior.4, &prior.5, prior.6,
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeUsageNotMonotonic)?;
        if protocol_sequence.value() <= prior.7
            || usage.input_tokens < prior_usage.input_tokens
            || usage.output_tokens < prior_usage.output_tokens
            || usage.cache_read_tokens < prior_usage.cache_read_tokens
            || usage.cache_write_tokens < prior_usage.cache_write_tokens
            || usage.total_tokens < prior_usage.total_tokens
            || usage.provider_cost < prior_usage.provider_cost
            || usage.ceiling_micro_usd < prior_usage.ceiling_micro_usd
        {
            return Err(Rejection::PiOfficeSessionDisposeUsageNotMonotonic);
        }
    }
    let charged: i64 = transaction
        .query_row(
            "SELECT reservation.charged_micros
         FROM office_session_budget_reservations parent
         JOIN budget_reservations reservation
           ON reservation.budget_reservation_id = parent.budget_reservation_id
         WHERE parent.root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    if usage.ceiling_micro_usd.value() < charged {
        return Err(Rejection::PiOfficeSessionDisposeUsageNotMonotonic);
    }
    transaction
        .execute(
            "INSERT INTO pi_office_session_dispose_usage_receipts(
             root_authority_office_session_id, pi_session_id, correlation_identity,
             protocol_sequence, input_tokens, output_tokens, cache_read_tokens,
             cache_write_tokens, total_tokens, provider_cost_binary64,
             cumulative_ceiling_micros, recorded_by_command_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                session_id.value(),
                pi_session,
                correlation_identity.as_str(),
                protocol_sequence.value(),
                usage.input_tokens.value(),
                usage.output_tokens.value(),
                usage.cache_read_tokens.value(),
                usage.cache_write_tokens.value(),
                usage.total_tokens.value(),
                usage.provider_cost.as_big_endian_bytes().as_slice(),
                usage.ceiling_micro_usd.value(),
                command_row_id,
            ],
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeUsageNotMonotonic)?;
    Ok(EventBody::PiOfficeSessionDisposeUsageRecorded {
        session_id,
        protocol_sequence,
        cumulative_micro_usd: usage.ceiling_micro_usd,
    })
}

fn record_pi_office_session_dispose_usage_failure(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: RootAuthorityOfficeSessionId,
    correlation_identity: &PiCorrelationIdentity,
    protocol_sequence: PiProtocolSequence,
    failure: PiOfficeTurnUsageFailure,
) -> Result<EventBody, Rejection> {
    require_pi_office_session_dispose_authorization_generation(
        transaction,
        expected_generation,
        session_id,
    )?;
    let row: Option<(i64, String, i64, i64, i64, i64)> = transaction
        .query_row(
            "SELECT d.pi_session_id, d.correlation_identity, a.command_result_sequence,
                parent.budget_reservation_id, s.operating_cycle_id, s.lifecycle_state
         FROM pi_office_session_dispose_deliveries d
         JOIN pi_office_session_dispose_acceptances a
           ON a.root_authority_office_session_id = d.root_authority_office_session_id
         JOIN root_authority_office_sessions s
           ON s.root_authority_office_session_id = d.root_authority_office_session_id
         JOIN office_session_budget_reservations parent
           ON parent.root_authority_office_session_id = s.root_authority_office_session_id
         WHERE d.root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    let (pi_session, stored_correlation, accepted_sequence, reservation, cycle, session_state) =
        row.ok_or(Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    if stored_correlation != correlation_identity.as_str()
        || session_state != OfficeSessionState::Quiescing as i64
    {
        return Err(Rejection::PiOfficeSessionDisposeBindingMismatch);
    }
    if accepted_sequence.checked_add(1) != Some(protocol_sequence.value())
        || pi_office_session_max_sequence(transaction, pi_session)?
            .is_some_and(|previous| protocol_sequence.value() <= previous)
    {
        return Err(Rejection::PiOfficeSessionDisposeUsageNotMonotonic);
    }
    let reservation_id =
        BudgetReservationId::try_from(reservation).map_err(|_| Rejection::ReservationNotActive)?;
    let cycle_id = OperatingCycleId::try_from(cycle).map_err(|_| Rejection::SubjectNotFound)?;
    let (amount, charged, state): (i64, i64, i64) = transaction
        .query_row(
            "SELECT amount_micros, charged_micros, reservation_state
         FROM budget_reservations WHERE budget_reservation_id = ?1",
            [reservation],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| Rejection::ReservationNotActive)?;
    if state != BudgetReservationState::Reserved as i64 || amount < charged {
        return Err(Rejection::ReservationNotActive);
    }
    let freeze_reason = pi_office_turn_usage_failure_freeze_reason(failure);
    let frozen = freeze_budget_admission(
        transaction,
        command_row_id,
        reservation_id,
        cycle_id,
        UsdMicros::try_from(amount - charged).map_err(|_| Rejection::ReservationNotActive)?,
        freeze_reason,
    )?;
    let (cancellation_request_id, postmortem_id) = match frozen {
        EventBody::BudgetAdmissionFrozen {
            cancellation_request_id,
            postmortem_id,
            ..
        } => (cancellation_request_id, postmortem_id),
        _ => return Err(Rejection::PiOfficeSessionDisposeBindingMismatch),
    };
    let (kind, unknown, unavailable) = sql_pi_office_turn_usage_failure(failure);
    transaction
        .execute(
            "INSERT INTO pi_office_session_dispose_usage_failures(
             root_authority_office_session_id, pi_session_id, correlation_identity,
             protocol_sequence, failure_kind, unknown_reason, unavailable_reason,
             budget_reservation_id, cancellation_request_id, cost_postmortem_id,
             recorded_by_command_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                session_id.value(),
                pi_session,
                correlation_identity.as_str(),
                protocol_sequence.value(),
                kind,
                unknown,
                unavailable,
                reservation,
                cancellation_request_id.value(),
                postmortem_id.value(),
                command_row_id,
            ],
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeBindingMismatch)?;
    Ok(EventBody::PiOfficeSessionDisposeUsageFrozen {
        session_id,
        budget_reservation_id: reservation_id,
        cancellation_request_id,
        postmortem_id,
        failure,
    })
}

fn pi_office_turn_usage_failure_freeze_reason(
    failure: PiOfficeTurnUsageFailure,
) -> BudgetFreezeReason {
    match failure {
        PiOfficeTurnUsageFailure::Unknown(
            PiOfficeTurnUsageUnknownReason::MissingFinalUsageSnapshot,
        ) => BudgetFreezeReason::Unknown(CostUnknownReason::ProviderDidNotReport),
        PiOfficeTurnUsageFailure::Unknown(
            PiOfficeTurnUsageUnknownReason::BoundaryStreamInterrupted,
        ) => BudgetFreezeReason::Unknown(CostUnknownReason::AdapterStreamInterrupted),
        PiOfficeTurnUsageFailure::Unknown(
            PiOfficeTurnUsageUnknownReason::TerminalEvidenceMissing,
        ) => BudgetFreezeReason::Unknown(CostUnknownReason::ReconciliationMismatch),
        PiOfficeTurnUsageFailure::Unavailable(_) => {
            BudgetFreezeReason::Unavailable(CostUnavailableReason::AdapterAccountingUnavailable)
        }
    }
}

fn record_pi_office_session_disposed(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: RootAuthorityOfficeSessionId,
    correlation_identity: &PiCorrelationIdentity,
    disposed_sequence: PiProtocolSequence,
    transcript_receipt: &PiOfficeSessionTranscriptReceipt,
) -> Result<EventBody, Rejection> {
    require_pi_office_session_dispose_authorization_generation(
        transaction,
        expected_generation,
        session_id,
    )?;
    let row: Option<PiOfficeSessionDisposeTerminalSqlRow> = transaction
        .query_row(
            "SELECT d.native_child_id, d.pi_session_id, d.correlation_identity,
                accepted.command_result_sequence, usage.protocol_sequence,
                usage.input_tokens, usage.output_tokens, usage.cache_read_tokens,
                usage.cache_write_tokens, usage.total_tokens, usage.provider_cost_binary64,
                usage.cumulative_ceiling_micros, parent.budget_reservation_id,
                s.operating_cycle_id, s.lifecycle_state
         FROM pi_office_session_dispose_deliveries d
         JOIN pi_office_session_dispose_acceptances accepted
           ON accepted.root_authority_office_session_id = d.root_authority_office_session_id
         JOIN pi_office_session_dispose_usage_receipts usage
           ON usage.root_authority_office_session_id = d.root_authority_office_session_id
         JOIN root_authority_office_sessions s
           ON s.root_authority_office_session_id = d.root_authority_office_session_id
         JOIN office_session_budget_reservations parent
           ON parent.root_authority_office_session_id = s.root_authority_office_session_id
         WHERE d.root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                    row.get(13)?,
                    row.get(14)?,
                ))
            },
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeSessionDisposeReceiptMissing)?;
    let (
        child,
        pi_session,
        stored_correlation,
        accepted_sequence,
        usage_sequence,
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
        total_tokens,
        provider_cost,
        cumulative_ceiling,
        reservation,
        cycle,
        session_state,
    ) = row.ok_or(Rejection::PiOfficeSessionDisposeReceiptMissing)?;
    if stored_correlation != correlation_identity.as_str()
        || session_state != OfficeSessionState::Quiescing as i64
    {
        return Err(Rejection::PiOfficeSessionDisposeBindingMismatch);
    }
    if accepted_sequence.checked_add(1) != Some(usage_sequence)
        || usage_sequence.checked_add(1) != Some(disposed_sequence.value())
        || pi_office_session_max_sequence(transaction, pi_session)?
            .is_some_and(|previous| disposed_sequence.value() <= previous)
        || pi_office_session_has_active_turn(transaction, session_id)?
    {
        return Err(Rejection::PiOfficeSessionDisposeUsageNotMonotonic);
    }
    let usage = pi_cumulative_usage_from_sql(
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
        total_tokens,
        &provider_cost,
        cumulative_ceiling,
    )
    .map_err(|_| Rejection::PiOfficeSessionDisposeUsageNotMonotonic)?;
    let (
        transcript_kind,
        session_file,
        session_file_digest,
        transcript_content_object_id,
        first_user_prompt_kind,
        first_user_prompt_digest,
    ) = transcript_receipt_sql_values(transcript_receipt);
    validate_pi_office_session_transcript_receipt(
        transaction,
        pi_session,
        session_file_digest.as_deref(),
        transcript_content_object_id,
        first_user_prompt_kind,
        first_user_prompt_digest.as_deref(),
        transcript_kind,
    )?;
    let reservation_id =
        BudgetReservationId::try_from(reservation).map_err(|_| Rejection::ReservationNotActive)?;
    let cycle_id = OperatingCycleId::try_from(cycle).map_err(|_| Rejection::SubjectNotFound)?;
    let (amount, charged, reservation_state): (i64, i64, i64) = transaction
        .query_row(
            "SELECT amount_micros, charged_micros, reservation_state
         FROM budget_reservations WHERE budget_reservation_id = ?1",
            [reservation],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| Rejection::ReservationNotActive)?;
    if reservation_state != BudgetReservationState::Reserved as i64
        || amount < charged
        || usage.ceiling_micro_usd.value() < charged
    {
        return Err(Rejection::PiOfficeSessionDisposeUsageNotMonotonic);
    }
    let (budget_disposition, next_session_state) = if usage.ceiling_micro_usd.value() > amount {
        let frozen = freeze_budget_admission(
            transaction,
            command_row_id,
            reservation_id,
            cycle_id,
            UsdMicros::try_from(amount - charged).map_err(|_| Rejection::ReservationNotActive)?,
            BudgetFreezeReason::KnownOverrun {
                observed: usage.ceiling_micro_usd,
                reserved: UsdMicros::try_from(amount - charged)
                    .map_err(|_| Rejection::ReservationNotActive)?,
            },
        )?;
        let (cancellation_request_id, postmortem_id) = match frozen {
            EventBody::BudgetAdmissionFrozen {
                cancellation_request_id,
                postmortem_id,
                ..
            } => (cancellation_request_id, postmortem_id),
            _ => return Err(Rejection::PiOfficeSessionDisposeReceiptMissing),
        };
        (
            PiOfficeSessionDisposeBudgetDisposition::Frozen {
                cancellation_request_id,
                postmortem_id,
            },
            OfficeSessionState::Cancelled,
        )
    } else {
        let delta = usage.ceiling_micro_usd.value() - charged;
        let mut charge_statement = transaction
            .prepare(
                "SELECT budget_envelope_id, amount_micros FROM budget_reservation_charges
                 WHERE budget_reservation_id = ?1 ORDER BY budget_envelope_id",
            )
            .map_err(|_| Rejection::ReservationNotActive)?;
        let charges = charge_statement
            .query_map([reservation_id.value()], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(|_| Rejection::ReservationNotActive)?;
        for charge in charges {
            let (budget_id, reserved_charge) =
                charge.map_err(|_| Rejection::ReservationNotActive)?;
            if reserved_charge < delta {
                return Err(Rejection::PiOfficeSessionDisposeUsageNotMonotonic);
            }
            transaction
                .execute(
                    "UPDATE budget_envelopes
                 SET reserved_micros = reserved_micros - ?1,
                     spent_micros = spent_micros + ?2
                 WHERE budget_envelope_id = ?3",
                    params![reserved_charge, delta, budget_id],
                )
                .map_err(|_| Rejection::BudgetCeilingExceeded)?;
            transaction
                .execute(
                    "UPDATE budget_reservation_charges SET amount_micros = 0
                 WHERE budget_reservation_id = ?1 AND budget_envelope_id = ?2",
                    params![reservation_id.value(), budget_id],
                )
                .map_err(|_| Rejection::ReservationNotActive)?;
        }
        transaction
            .execute(
                "UPDATE budget_reservations
             SET reservation_state = ?1, charged_micros = ?2, reconciled_by_command_id = ?3
             WHERE budget_reservation_id = ?4",
                params![
                    BudgetReservationState::Reconciled as i64,
                    usage.ceiling_micro_usd.value(),
                    command_row_id,
                    reservation_id.value(),
                ],
            )
            .map_err(|_| Rejection::ReservationNotActive)?;
        (
            PiOfficeSessionDisposeBudgetDisposition::Reconciled {
                observed_cumulative_micro_usd: usage.ceiling_micro_usd,
            },
            OfficeSessionState::Closed,
        )
    };
    let (budget_disposition_kind, cancellation_request_id, postmortem_id) =
        sql_pi_office_session_dispose_budget_disposition(budget_disposition);
    transaction
        .execute(
            "INSERT INTO pi_office_session_dispose_receipts(
             root_authority_office_session_id, native_child_id, pi_session_id,
             correlation_identity, disposed_sequence, transcript_kind, session_file,
             session_file_digest, transcript_content_object_id, first_user_prompt_kind,
             first_user_prompt_digest, budget_disposition_kind, cancellation_request_id,
             cost_postmortem_id, recorded_by_command_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                session_id.value(),
                child,
                pi_session,
                correlation_identity.as_str(),
                disposed_sequence.value(),
                transcript_kind,
                session_file,
                session_file_digest,
                transcript_content_object_id,
                first_user_prompt_kind,
                first_user_prompt_digest,
                budget_disposition_kind,
                cancellation_request_id,
                postmortem_id,
                command_row_id,
            ],
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeReceiptMissing)?;
    let receipt_id = id_from_last_insert::<PiOfficeSessionDisposeReceiptId>(transaction)?;
    transaction
        .execute(
            "UPDATE root_authority_office_sessions
         SET lifecycle_state = ?1, last_transition_command_id = ?2
         WHERE root_authority_office_session_id = ?3",
            params![
                next_session_state as i64,
                command_row_id,
                session_id.value()
            ],
        )
        .map_err(|_| Rejection::PiOfficeSessionDisposeReceiptMissing)?;
    Ok(EventBody::PiOfficeSessionDisposed {
        pi_office_session_dispose_receipt_id: receipt_id,
        session_id,
        budget_reservation_id: reservation_id,
        observed_cumulative_micro_usd: usage.ceiling_micro_usd,
        budget_disposition,
    })
}

fn transcript_receipt_sql_values(
    receipt: &PiOfficeSessionTranscriptReceipt,
) -> PiOfficeSessionTranscriptReceiptSqlValues {
    match receipt {
        PiOfficeSessionTranscriptReceipt::Materialized {
            session_file,
            session_file_digest,
            transcript_content_object_id,
            first_user_prompt,
        } => {
            let (first_user_prompt_kind, first_user_prompt_digest) = match first_user_prompt {
                PiOfficeSessionFirstUserPromptReceipt::Absent => (Some(1), None),
                PiOfficeSessionFirstUserPromptReceipt::Verified { digest } => {
                    (Some(2), Some(digest.as_bytes().to_vec()))
                }
            };
            (
                1,
                session_file.as_str().to_owned(),
                Some(session_file_digest.as_bytes().to_vec()),
                Some(transcript_content_object_id.value()),
                first_user_prompt_kind,
                first_user_prompt_digest,
            )
        }
        PiOfficeSessionTranscriptReceipt::UnmaterializedNoPrompt { session_file } => {
            (2, session_file.as_str().to_owned(), None, None, None, None)
        }
    }
}

fn validate_pi_office_session_transcript_receipt(
    transaction: &Transaction<'_>,
    pi_session_id: i64,
    session_file_digest: Option<&[u8]>,
    transcript_content_object_id: Option<i64>,
    first_user_prompt_kind: Option<i64>,
    first_user_prompt_digest: Option<&[u8]>,
    transcript_kind: i64,
) -> Result<(), Rejection> {
    let first_prompt: Option<Vec<u8>> = transaction
        .query_row(
            "SELECT prompt_digest FROM pi_office_turn_prompt_authorizations
         WHERE pi_session_id = ?1
         ORDER BY pi_office_turn_prompt_authorization_id ASC LIMIT 1",
            [pi_session_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeSessionDisposeReceiptMissing)?;
    match transcript_kind {
        1 => {
            let content_object_id = transcript_content_object_id
                .ok_or(Rejection::PiOfficeSessionDisposeReceiptMissing)?;
            let content_digest: Option<Vec<u8>> = transaction
                .query_row(
                    "SELECT seal.digest FROM content_objects object
                 JOIN content_seal_receipts seal
                   ON seal.content_seal_receipt_id = object.content_seal_receipt_id
                 WHERE object.content_object_id = ?1",
                    [content_object_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| Rejection::PiOfficeSessionDisposeReceiptMissing)?;
            if content_digest.as_deref() != session_file_digest {
                return Err(Rejection::PiOfficeSessionDisposeReceiptMissing);
            }
            match (
                first_prompt.as_deref(),
                first_user_prompt_kind,
                first_user_prompt_digest,
            ) {
                (None, Some(1), None) | (Some(_), Some(2), Some(_))
                    if first_prompt.as_deref() == first_user_prompt_digest =>
                {
                    Ok(())
                }
                _ => Err(Rejection::PiOfficeSessionDisposeReceiptMissing),
            }
        }
        2 if first_prompt.is_none() => Ok(()),
        _ => Err(Rejection::PiOfficeSessionDisposeReceiptMissing),
    }
}

fn sql_pi_office_session_dispose_budget_disposition(
    disposition: PiOfficeSessionDisposeBudgetDisposition,
) -> (i64, Option<i64>, Option<i64>) {
    match disposition {
        PiOfficeSessionDisposeBudgetDisposition::Reconciled { .. } => (1, None, None),
        PiOfficeSessionDisposeBudgetDisposition::Frozen {
            cancellation_request_id,
            postmortem_id,
        } => (
            2,
            Some(cancellation_request_id.value()),
            Some(postmortem_id.value()),
        ),
    }
}

/// The peer's `Settled` boundary is final for this exact Prompt correlation.
/// Buffered evidence that arrived before it remains recordable before the
/// terminal receipt, but no later usage observation may silently outrun the
/// cumulative checkpoint that allowed Office authority to return to Ready.
fn pi_office_turn_has_terminal_receipt(
    transaction: &Transaction<'_>,
    office_turn_id: OfficeTurnId,
) -> Result<bool, Rejection> {
    transaction
        .query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM pi_office_turn_terminal_receipts
                 WHERE office_turn_id = ?1
             )",
            [office_turn_id.value()],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(|_| Rejection::PiOfficeTurnTerminalEvidenceMissing)
}

/// Returns the greatest durable Pi protocol sequence for one boundary
/// session. This is a closed union over named Pi facts, not a generic event
/// stream: Prompt CommandResults, usage/failure observations, and terminal
/// peer boundaries share exactly one session-scoped sequence namespace.
fn pi_office_session_max_sequence(
    transaction: &Transaction<'_>,
    pi_session_id: i64,
) -> Result<Option<i64>, Rejection> {
    transaction
        .query_row(
            "SELECT MAX(protocol_sequence) FROM (
                 SELECT accepted.command_result_sequence AS protocol_sequence
                 FROM pi_office_turn_prompt_acceptances accepted
                 JOIN pi_office_turn_prompt_authorizations authorization
                   ON authorization.pi_office_turn_prompt_authorization_id = accepted.pi_office_turn_prompt_authorization_id
                 WHERE authorization.pi_session_id = ?1
                 UNION ALL
                 SELECT protocol_sequence FROM pi_office_turn_usage_receipts
                 WHERE pi_session_id = ?1
                 UNION ALL
                 SELECT protocol_sequence FROM pi_office_turn_usage_failures
                 WHERE pi_session_id = ?1
                 UNION ALL
                 SELECT agent_settled_sequence FROM pi_office_turn_terminal_receipts
                 WHERE pi_session_id = ?1
                 UNION ALL
                 SELECT final_accounting_sequence FROM pi_office_turn_terminal_receipts
                 WHERE pi_session_id = ?1
                 UNION ALL
                 SELECT settled_sequence FROM pi_office_turn_terminal_receipts
                 WHERE pi_session_id = ?1
                 UNION ALL
                 SELECT accepted.command_result_sequence AS protocol_sequence
                 FROM pi_office_session_dispose_acceptances accepted
                 JOIN pi_office_session_dispose_deliveries delivery
                   ON delivery.root_authority_office_session_id = accepted.root_authority_office_session_id
                 WHERE delivery.pi_session_id = ?1
                 UNION ALL
                 SELECT protocol_sequence FROM pi_office_session_dispose_usage_receipts
                 WHERE pi_session_id = ?1
                 UNION ALL
                 SELECT protocol_sequence FROM pi_office_session_dispose_usage_failures
                 WHERE pi_session_id = ?1
                 UNION ALL
                 SELECT disposed_sequence AS protocol_sequence FROM pi_office_session_dispose_receipts
                 WHERE pi_session_id = ?1
             )",
            [pi_session_id],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::PiOfficeTurnUsageNotMonotonic)
}

fn record_pi_office_turn_terminal(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    input: PiOfficeTurnTerminalInput<'_>,
) -> Result<EventBody, Rejection> {
    let PiOfficeTurnTerminalInput {
        office_turn_id,
        correlation_identity,
        terminal_evidence,
        settled_sequence,
        disposition,
        assistant_outcome,
        transcript_disposition,
    } = input;
    let final_accounting_sequence = terminal_evidence.final_accounting_sequence();
    if !disposition.accepts(assistant_outcome)
        || !terminal_evidence.accepts(assistant_outcome)
        || final_accounting_sequence.value().checked_add(1) != Some(settled_sequence.value())
    {
        return Err(Rejection::PiOfficeTurnTerminalEvidenceMissing);
    }
    let row: Option<(i64, i64, i64, i64, i64)> = transaction.query_row(
        "SELECT a.pi_office_turn_prompt_authorization_id, a.native_child_id, a.pi_session_id,
                s.operating_cycle_id, accepted.command_result_sequence
         FROM pi_office_turn_prompt_authorizations a
         JOIN office_turns t ON t.office_turn_id = a.office_turn_id
         JOIN root_authority_office_sessions s ON s.root_authority_office_session_id = t.root_authority_office_session_id
         JOIN pi_office_turn_prompt_acceptances accepted ON accepted.pi_office_turn_prompt_authorization_id = a.pi_office_turn_prompt_authorization_id
         WHERE a.office_turn_id = ?1 AND a.correlation_identity = ?2",
        params![office_turn_id.value(), correlation_identity.as_str()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    ).optional().map_err(|_| Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
    let (authorization, child, session, cycle, accepted_sequence) =
        row.ok_or(Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
    let prior_session_settled: Option<i64> = transaction
        .query_row(
            "SELECT MAX(settled_sequence) FROM pi_office_turn_terminal_receipts
             WHERE pi_session_id = ?1 AND office_turn_id != ?2",
            params![session, office_turn_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
    let first_terminal_sequence = terminal_evidence
        .agent_settled_sequence()
        .unwrap_or(final_accounting_sequence);
    if accepted_sequence >= first_terminal_sequence.value()
        || prior_session_settled.is_some_and(|previous| first_terminal_sequence.value() <= previous)
    {
        return Err(Rejection::PiOfficeTurnTerminalEvidenceMissing);
    }
    let usage_id: Option<i64> = transaction
        .query_row(
            "SELECT pi_office_turn_usage_receipt_id FROM pi_office_turn_usage_receipts
         WHERE office_turn_id = ?1 AND pi_office_turn_prompt_authorization_id = ?2
           AND correlation_identity = ?3 AND protocol_sequence = ?4",
            params![
                office_turn_id.value(),
                authorization,
                correlation_identity.as_str(),
                final_accounting_sequence.value()
            ],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
    let usage_id = usage_id
        .map(PiOfficeTurnUsageReceiptId::try_from)
        .transpose()
        .map_err(|_| Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
    let failure_id: Option<i64> = transaction
        .query_row(
            "SELECT pi_office_turn_usage_failure_id FROM pi_office_turn_usage_failures
         WHERE office_turn_id = ?1 AND pi_office_turn_prompt_authorization_id = ?2
           AND correlation_identity = ?3 AND protocol_sequence = ?4",
            params![
                office_turn_id.value(),
                authorization,
                correlation_identity.as_str(),
                final_accounting_sequence.value()
            ],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
    let failure_id = failure_id
        .map(PiOfficeTurnUsageFailureId::try_from)
        .transpose()
        .map_err(|_| Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
    if usage_id.is_some() == failure_id.is_some() {
        return Err(Rejection::PiOfficeTurnTerminalEvidenceMissing);
    }
    if matches!(
        terminal_evidence,
        PiOfficeTurnTerminalEvidence::UnavailableAssistant { .. }
    ) && usage_id.is_none()
    {
        // The host emits an unavailable assistant terminal only after its
        // forced final Known snapshot succeeds. An unavailable accounting
        // observation fences before Settled and cannot certify this shape.
        return Err(Rejection::PiOfficeTurnTerminalEvidenceMissing);
    }
    // SDK/assistant outcome and cost knowledge are independent peer facts.
    // A peer-valid Error may still carry its final Known cumulative usage;
    // inventing an Unknown/Unavailable receipt would erase that truth. Any
    // non-ready terminal remains an active Office-turn/closure blocker until a
    // later typed cancellation or recovery settlement owns it.
    if disposition == PiOfficeTurnDisposition::Aborted {
        let cycle_id = OperatingCycleId::try_from(cycle)
            .map_err(|_| Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
        if cycle_row(transaction, cycle_id)?.state == OperatingCycleState::Running {
            return Err(Rejection::PiOfficeTurnTerminalEvidenceMissing);
        }
    }
    let (evidence_kind, agent_settled_sequence) =
        sql_pi_office_turn_terminal_evidence(terminal_evidence);
    transaction.execute(
        "INSERT INTO pi_office_turn_terminal_receipts(office_turn_id, pi_office_turn_prompt_authorization_id, native_child_id, pi_session_id, correlation_identity, terminal_evidence_kind, agent_settled_sequence, final_accounting_sequence, settled_sequence, final_usage_receipt_id, final_usage_failure_id, disposition, assistant_outcome, transcript_disposition, recorded_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![office_turn_id.value(), authorization, child, session, correlation_identity.as_str(), evidence_kind, agent_settled_sequence, final_accounting_sequence.value(), settled_sequence.value(), usage_id.map(PiOfficeTurnUsageReceiptId::value), failure_id.map(PiOfficeTurnUsageFailureId::value), disposition as i64, assistant_outcome as i64, transcript_disposition as i64, command_row_id],
    ).map_err(|_| Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
    Ok(EventBody::PiOfficeTurnTerminalRecorded {
        pi_office_turn_terminal_receipt_id: id_from_last_insert(transaction)?,
        office_turn_id,
        disposition,
        assistant_outcome,
    })
}

fn settle_office_turn(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    turn_id: OfficeTurnId,
    terminal_receipt_id: PiOfficeTurnTerminalReceiptId,
) -> Result<EventBody, Rejection> {
    let row: Option<PiOfficeTurnSettlementSqlRow> = transaction.query_row(
        "SELECT t.lifecycle_state, t.root_authority_office_session_id,
                terminal.disposition, terminal.assistant_outcome, terminal.native_child_id,
                terminal.pi_session_id, terminal.final_usage_receipt_id,
                checkpoint.budget_reservation_id, usage.cumulative_ceiling_micros,
                checkpoint.baseline_cumulative_micros
         FROM office_turns t
         JOIN pi_office_turn_terminal_receipts terminal ON terminal.office_turn_id = t.office_turn_id
         JOIN office_turn_budget_checkpoints checkpoint ON checkpoint.office_turn_id = t.office_turn_id
         JOIN pi_office_turn_usage_receipts usage ON usage.pi_office_turn_usage_receipt_id = terminal.final_usage_receipt_id
         WHERE t.office_turn_id = ?1 AND terminal.pi_office_turn_terminal_receipt_id = ?2",
        params![turn_id.value(), terminal_receipt_id.value()],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?)),
    ).optional().map_err(|_| Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
    let row = row.ok_or(Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
    if row.0 != OfficeTurnState::Active as i64
        || row.2 != PiOfficeTurnDisposition::Completed as i64
        || row.3 != PiOfficeTurnAssistantOutcome::ObservedStop as i64
    {
        return Err(Rejection::PiOfficeTurnNotReconciled);
    }
    let session_id = RootAuthorityOfficeSessionId::try_from(row.1)
        .map_err(|_| Rejection::PiOfficeTurnTerminalEvidenceMissing)?;
    let guard: Option<(i64, i64, i64, i64, i64)> = transaction.query_row(
        "SELECT s.lifecycle_state, s.operating_cycle_id, c.lifecycle_state, c.admission_generation,
                a.admission_generation
         FROM root_authority_office_sessions s
         JOIN operating_cycles c ON c.operating_cycle_id = s.operating_cycle_id
         JOIN pi_office_turn_prompt_authorizations a ON a.office_turn_id = ?1
         WHERE s.root_authority_office_session_id = ?2",
        params![turn_id.value(), session_id.value()],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
    ).optional().map_err(|_| Rejection::PiOfficeTurnNotReconciled)?;
    let (session_state, cycle, cycle_state, current_generation, authorized_generation) =
        guard.ok_or(Rejection::PiOfficeTurnNotReconciled)?;
    let cycle_id =
        OperatingCycleId::try_from(cycle).map_err(|_| Rejection::PiOfficeTurnNotReconciled)?;
    if session_state != OfficeSessionState::TurnActive as i64
        || cycle_state != OperatingCycleState::Running as i64
        || current_generation != authorized_generation
        || active_cancellation_count(transaction, cycle_id)? != 0
    {
        return Err(Rejection::PiOfficeTurnNotReconciled);
    }
    let child_is_still_live: i64 = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM native_children child
          JOIN pi_child_session_protocols protocol ON protocol.native_child_id = child.native_child_id
          WHERE child.native_child_id = ?1 AND child.lifecycle_state = ?2 AND protocol.lifecycle_state = ?3)",
        params![row.4, ChildProcessState::Running as i64, PiChildSessionState::SessionReady as i64],
        |r| r.get(0),
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    if child_is_still_live == 0 {
        return Err(Rejection::ChildLifecycleReceiptMissing);
    }
    let reservation_id =
        BudgetReservationId::try_from(row.7).map_err(|_| Rejection::ReservationNotActive)?;
    let (amount, charged, state): (i64, i64, i64) = transaction.query_row(
        "SELECT amount_micros, charged_micros, reservation_state FROM budget_reservations WHERE budget_reservation_id = ?1",
        [reservation_id.value()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).map_err(|_| Rejection::ReservationNotActive)?;
    if state != BudgetReservationState::Reserved as i64
        || amount < charged
        || row.8 < charged
        || row.9 != charged
    {
        return Err(Rejection::PiOfficeTurnUsageNotMonotonic);
    }
    if row.8 > amount {
        return freeze_budget_admission(
            transaction,
            command_row_id,
            reservation_id,
            transaction.query_row("SELECT operating_cycle_id FROM budget_reservations WHERE budget_reservation_id = ?1", [reservation_id.value()], |r| r.get::<_, i64>(0))
                .map_err(|_| Rejection::ReservationNotActive)
                .and_then(|id| OperatingCycleId::try_from(id).map_err(|_| Rejection::ReservationNotActive))?,
            UsdMicros::try_from(amount - charged).map_err(|_| Rejection::ReservationNotActive)?,
            BudgetFreezeReason::KnownOverrun {
                observed: UsdMicros::try_from(row.8).map_err(|_| Rejection::ReservationNotActive)?,
                reserved: UsdMicros::try_from(amount - charged).map_err(|_| Rejection::ReservationNotActive)?,
            },
        );
    }
    let delta = row.8 - charged;
    let mut charge_statement = transaction
        .prepare(
            "SELECT budget_envelope_id, amount_micros FROM budget_reservation_charges
         WHERE budget_reservation_id = ?1 ORDER BY budget_envelope_id",
        )
        .map_err(|_| Rejection::ReservationNotActive)?;
    let charges = charge_statement
        .query_map([reservation_id.value()], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
        })
        .map_err(|_| Rejection::ReservationNotActive)?;
    for charge in charges {
        let (envelope_id, remaining) = charge.map_err(|_| Rejection::ReservationNotActive)?;
        if remaining < delta {
            return Err(Rejection::PiOfficeTurnUsageNotMonotonic);
        }
        transaction.execute(
            "UPDATE budget_envelopes SET reserved_micros = reserved_micros - ?1, spent_micros = spent_micros + ?1 WHERE budget_envelope_id = ?2",
            params![delta, envelope_id],
        ).map_err(|_| Rejection::BudgetCeilingExceeded)?;
        transaction.execute(
            "UPDATE budget_reservation_charges SET amount_micros = amount_micros - ?1 WHERE budget_reservation_id = ?2 AND budget_envelope_id = ?3",
            params![delta, reservation_id.value(), envelope_id],
        ).map_err(|_| Rejection::ReservationNotActive)?;
    }
    transaction.execute(
        "UPDATE budget_reservations SET charged_micros = charged_micros + ?1 WHERE budget_reservation_id = ?2",
        params![delta, reservation_id.value()],
    ).map_err(|_| Rejection::ReservationNotActive)?;
    transaction.execute(
        "UPDATE office_turn_budget_checkpoints SET settled_cumulative_micros = ?1, settled_by_command_id = ?2 WHERE office_turn_id = ?3 AND settled_cumulative_micros IS NULL",
        params![row.8, command_row_id, turn_id.value()],
    ).map_err(|_| Rejection::PiOfficeTurnNotReconciled)?;
    transaction.execute(
        "UPDATE office_turns SET lifecycle_state = ?1, settled_by_command_id = ?2 WHERE office_turn_id = ?3",
        params![OfficeTurnState::Settled as i64, command_row_id, turn_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "UPDATE root_authority_office_sessions SET lifecycle_state = ?1, last_transition_command_id = ?2
         WHERE root_authority_office_session_id = ?3",
        params![OfficeSessionState::Ready as i64, command_row_id, session_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    Ok(EventBody::OfficeTurnSettled {
        turn_id,
        session_id,
        charged_delta: UsdMicros::try_from(delta)
            .map_err(|_| Rejection::PiOfficeTurnUsageNotMonotonic)?,
    })
}

fn quiesce_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if !matches!(
        cycle.state,
        OperatingCycleState::Admitted | OperatingCycleState::Running
    ) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let new_generation = cycle
        .generation
        .increment()
        .map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        OperatingCycleState::Quiescing,
        new_generation,
    )?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: OperatingCycleState::Quiescing,
        generation: new_generation,
    })
}

fn record_cycle_drained(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_row(transaction, cycle_id)?;
    if cycle.state != OperatingCycleState::Quiescing
        || active_office_turn_count(transaction, cycle_id)? != 0
        || live_actor_attempt_count(transaction, cycle_id)? != 0
        || active_work_lease_count(transaction, cycle_id)? != 0
        || live_native_child_count(transaction, cycle_id)? != 0
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    // A Quiesce-mode cancellation has drained its children but remains a
    // cancellation duty. Keep the cycle fenced as Cancelling until its
    // explicit reconciliation; ordinary quiescence may become Drained.
    let next_state = if active_cancellation_count(transaction, cycle_id)? != 0 {
        OperatingCycleState::Cancelling
    } else {
        OperatingCycleState::Drained
    };
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        next_state,
        cycle.generation,
    )?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: next_state,
        generation: cycle.generation,
    })
}

fn resume_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Drained {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    // Drained only says that owned execution has stopped. It does not make an
    // unresolved cost admissible again: Frozen reservations deliberately hold
    // their full authorization until a future typed resolution/Postmortem.
    // Likewise, a cancellation duty must reach a durable terminal receipt
    // before the same cycle may reopen admission.
    if unreconciled_reservation_count(transaction, cycle_id)? != 0
        || active_cancellation_count(transaction, cycle_id)? != 0
        || live_actor_attempt_count(transaction, cycle_id)? != 0
        || active_work_lease_count(transaction, cycle_id)? != 0
        || live_native_child_count(transaction, cycle_id)? != 0
        // M5 has no durable workspace-disposal receipt. A physically exited
        // child therefore does not make a cycle close-eligible yet; a later
        // disposal tranche must release this intentionally conservative fence.
        || undisposed_pi_workspace_count(transaction, cycle_id)? != 0
    {
        return Err(Rejection::IncompleteCycleReconciliation);
    }
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        OperatingCycleState::Running,
        cycle.generation,
    )?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: OperatingCycleState::Running,
        generation: cycle.generation,
    })
}

fn begin_reconciliation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Drained {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        OperatingCycleState::Reconciling,
        cycle.generation,
    )?;
    transaction.execute(
        "INSERT INTO operating_cycle_reconciliations(operating_cycle_id, reconciliation_started_by_command_id, closed_by_command_id)
         VALUES (?1, ?2, NULL)",
        params![cycle_id.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: OperatingCycleState::Reconciling,
        generation: cycle.generation,
    })
}

fn close_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Reconciling
        || active_office_turn_count(transaction, cycle_id)? != 0
        || live_office_session_count(transaction, cycle_id)? != 0
        || unreconciled_reservation_count(transaction, cycle_id)? != 0
        || active_cancellation_count(transaction, cycle_id)? != 0
        || live_actor_attempt_count(transaction, cycle_id)? != 0
        || active_work_lease_count(transaction, cycle_id)? != 0
        || live_native_child_count(transaction, cycle_id)? != 0
    {
        return Err(Rejection::IncompleteCycleReconciliation);
    }
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        OperatingCycleState::Closed,
        cycle.generation,
    )?;
    transaction.execute(
        "UPDATE operating_cycle_reconciliations SET closed_by_command_id = ?1 WHERE operating_cycle_id = ?2",
        params![command_row_id, cycle_id.value()],
    ).map_err(|_| Rejection::IncompleteCycleReconciliation)?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: OperatingCycleState::Closed,
        generation: cycle.generation,
    })
}

fn reserve_budget(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
    amount: UsdMicros,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if amount == UsdMicros::ZERO {
        return Err(Rejection::BudgetCeilingExceeded);
    }
    if !cycle.state.admits_task_work() {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let (society_budget, cycle_budget) =
        budget_envelopes_for_cycle(transaction, cycle.society_id, cycle_id)?;
    for budget_id in [society_budget, cycle_budget] {
        let (ceiling, reserved, spent) = budget_amounts(transaction, budget_id)?;
        let Some(next_reserved) = reserved.checked_add(amount) else {
            return Err(Rejection::BudgetCeilingExceeded);
        };
        if next_reserved
            .checked_add(spent)
            .is_none_or(|value| value > ceiling)
        {
            return Err(Rejection::BudgetCeilingExceeded);
        }
    }
    transaction
        .execute(
            "INSERT INTO budget_reservations(operating_cycle_id, amount_micros, reservation_state,
                                         reserved_by_command_id, reconciled_by_command_id)
         VALUES (?1, ?2, ?3, ?4, NULL)",
            params![
                cycle_id.value(),
                amount.value(),
                BudgetReservationState::Reserved as i64,
                command_row_id
            ],
        )
        .map_err(|_| Rejection::BudgetCeilingExceeded)?;
    let reservation_id = id_from_last_insert::<BudgetReservationId>(transaction)?;
    for budget_id in [society_budget, cycle_budget] {
        transaction.execute(
            "UPDATE budget_envelopes SET reserved_micros = reserved_micros + ?1 WHERE budget_envelope_id = ?2",
            params![amount.value(), budget_id.value()],
        ).map_err(|_| Rejection::BudgetCeilingExceeded)?;
        transaction.execute(
            "INSERT INTO budget_reservation_charges(budget_reservation_id, budget_envelope_id, amount_micros)
             VALUES (?1, ?2, ?3)",
            params![reservation_id.value(), budget_id.value(), amount.value()],
        ).map_err(|_| Rejection::BudgetCeilingExceeded)?;
    }
    Ok(EventBody::BudgetReserved {
        reservation_id,
        cycle_id,
        amount,
    })
}

fn reconcile_budget(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    reservation_id: BudgetReservationId,
    observation: CostObservation,
) -> Result<EventBody, Rejection> {
    let (cycle_id, reserved_amount, charged_amount, state) = transaction.query_row(
        "SELECT operating_cycle_id, amount_micros, charged_micros, reservation_state FROM budget_reservations WHERE budget_reservation_id = ?1",
        [reservation_id.value()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    if state != BudgetReservationState::Reserved as i64 {
        return Err(Rejection::ReservationNotActive);
    }
    // An Office-session reservation is incrementally debited by typed Pi
    // turn settlements. Its final remainder belongs to the later typed
    // Dispose reconciliation, not this generic final-only command.
    let office_parent: i64 = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM office_session_budget_reservations WHERE budget_reservation_id = ?1)",
            [reservation_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ReservationNotActive)?;
    if office_parent != 0 {
        return Err(Rejection::OfficeSessionBudgetRequiresDispose);
    }
    let cycle_id = OperatingCycleId::try_from(cycle_id).map_err(|_| Rejection::SubjectNotFound)?;
    let reserved_amount =
        UsdMicros::try_from(reserved_amount).map_err(|_| Rejection::SubjectNotFound)?;
    let charged_amount =
        UsdMicros::try_from(charged_amount).map_err(|_| Rejection::SubjectNotFound)?;
    if charged_amount > reserved_amount {
        return Err(Rejection::ReservationNotActive);
    }
    let remaining_amount = reserved_amount
        .checked_sub(charged_amount)
        .ok_or(Rejection::ReservationNotActive)?;
    match observation {
        CostObservation::Known(observed) => {
            if observed < charged_amount {
                return Err(Rejection::PiOfficeTurnUsageNotMonotonic);
            }
            if observed > reserved_amount {
                return freeze_budget_admission(
                    transaction,
                    command_row_id,
                    reservation_id,
                    cycle_id,
                    remaining_amount,
                    BudgetFreezeReason::KnownOverrun {
                        observed,
                        reserved: remaining_amount,
                    },
                );
            }
            let mut charge_statement = transaction
                .prepare(
                    "SELECT budget_envelope_id, amount_micros FROM budget_reservation_charges
                 WHERE budget_reservation_id = ?1 ORDER BY budget_envelope_id",
                )
                .map_err(|_| Rejection::SubjectNotFound)?;
            let charges = charge_statement
                .query_map([reservation_id.value()], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(|_| Rejection::SubjectNotFound)?;
            let charge_delta = observed
                .checked_sub(charged_amount)
                .ok_or(Rejection::PiOfficeTurnUsageNotMonotonic)?;
            for charge in charges {
                let (budget_id, charge_amount) = charge.map_err(|_| Rejection::SubjectNotFound)?;
                transaction
                    .execute(
                        "UPDATE budget_envelopes
                     SET reserved_micros = reserved_micros - ?1, spent_micros = spent_micros + ?2
                     WHERE budget_envelope_id = ?3",
                        params![charge_amount, charge_delta.value(), budget_id],
                    )
                    .map_err(|_| Rejection::BudgetCeilingExceeded)?;
            }
            transaction.execute(
                "UPDATE budget_reservations SET reservation_state = ?1, charged_micros = ?2, reconciled_by_command_id = ?3
                 WHERE budget_reservation_id = ?4",
                params![BudgetReservationState::Reconciled as i64, observed.value(), command_row_id, reservation_id.value()],
            ).map_err(|_| Rejection::SubjectNotFound)?;
            Ok(EventBody::BudgetReconciled {
                reservation_id,
                observed,
            })
        }
        CostObservation::Unknown(reason) => freeze_budget_admission(
            transaction,
            command_row_id,
            reservation_id,
            cycle_id,
            remaining_amount,
            BudgetFreezeReason::Unknown(reason),
        ),
        CostObservation::Unavailable(reason) => freeze_budget_admission(
            transaction,
            command_row_id,
            reservation_id,
            cycle_id,
            remaining_amount,
            BudgetFreezeReason::Unavailable(reason),
        ),
    }
}

/// Holds the full reservation, records why the cost cannot be reconciled, and
/// atomically fences the owning cycle before creating its cancellation duty.
/// This same path is used for unknown cost, unavailable accounting, and a
/// known provider overrun: none is permitted to become a rejected fact or zero.
fn freeze_budget_admission(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    reservation_id: BudgetReservationId,
    cycle_id: OperatingCycleId,
    reserved_amount: UsdMicros,
    reason: BudgetFreezeReason,
) -> Result<EventBody, Rejection> {
    transaction
        .execute(
            "UPDATE budget_reservations SET reservation_state = ?1
             WHERE budget_reservation_id = ?2",
            params![
                BudgetReservationState::Frozen as i64,
                reservation_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let cycle = cycle_row(transaction, cycle_id)?;
    let new_generation = cycle
        .generation
        .increment()
        .map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction
        .execute(
            "UPDATE operating_cycles SET lifecycle_state = ?1, admission_generation = ?2,
                                     last_transition_command_id = ?3 WHERE operating_cycle_id = ?4",
            params![
                OperatingCycleState::Cancelling as i64,
                new_generation.value(),
                command_row_id,
                cycle_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let cancellation_request_id = match active_cancellation_for_cycle(transaction, cycle_id)? {
        Some(existing) => existing,
        None => {
            transaction.execute(
                "INSERT INTO cancellation_requests(operating_cycle_id, cancellation_mode, lifecycle_state,
                                                   observed_admission_generation, requested_by_command_id, reconciled_by_command_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
                params![
                    cycle_id.value(),
                    CancellationMode::GracefulCancel as i64,
                    CancellationState::Accepted as i64,
                    cycle.generation.value(),
                    command_row_id
                ],
            ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
            id_from_last_insert::<CancellationRequestId>(transaction)?
        }
    };
    let (cause, observed, unknown, unavailable) = match reason {
        BudgetFreezeReason::KnownOverrun { observed, .. } => (
            CostPostmortemCause::KnownOverrun,
            Some(observed.value()),
            None,
            None,
        ),
        BudgetFreezeReason::Unknown(reason) => (
            CostPostmortemCause::UnknownCost,
            None,
            Some(reason as i64),
            None,
        ),
        BudgetFreezeReason::Unavailable(reason) => (
            CostPostmortemCause::UnavailableCost,
            None,
            None,
            Some(reason as i64),
        ),
    };
    transaction.execute(
        "INSERT INTO cost_postmortems(budget_reservation_id, operating_cycle_id, cancellation_request_id,
                                      cause_kind, observed_micros, reserved_micros, unknown_reason,
                                      unavailable_reason, lifecycle_state, opened_by_command_id, closed_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL)",
        params![
            reservation_id.value(),
            cycle_id.value(),
            cancellation_request_id.value(),
            cause as i64,
            observed,
            reserved_amount.value(),
            unknown,
            unavailable,
            CostPostmortemState::Open as i64,
            command_row_id,
        ],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::BudgetAdmissionFrozen {
        reservation_id,
        cycle_id,
        cancellation_request_id,
        postmortem_id: id_from_last_insert::<CostPostmortemId>(transaction)?,
        reason,
    })
}

fn request_cancellation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
    mode: CancellationMode,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if !cycle.state.is_nonterminal()
        || matches!(
            cycle.state,
            OperatingCycleState::Reconciling
                | OperatingCycleState::Cancelling
                | OperatingCycleState::Reaping
        )
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let new_generation = cycle
        .generation
        .increment()
        .map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let cycle_state = match mode {
        CancellationMode::Quiesce => OperatingCycleState::Quiescing,
        CancellationMode::GracefulCancel | CancellationMode::EmergencyStop => {
            OperatingCycleState::Cancelling
        }
    };
    transaction
        .execute(
            "UPDATE operating_cycles SET lifecycle_state = ?1, admission_generation = ?2,
                                     last_transition_command_id = ?3 WHERE operating_cycle_id = ?4",
            params![
                cycle_state as i64,
                new_generation.value(),
                command_row_id,
                cycle_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "INSERT INTO cancellation_requests(operating_cycle_id, cancellation_mode, lifecycle_state,
                                           observed_admission_generation, requested_by_command_id, reconciled_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
        params![cycle_id.value(), mode as i64, CancellationState::Accepted as i64, cycle.generation.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::CancellationRequested {
        cancellation_request_id: id_from_last_insert::<CancellationRequestId>(transaction)?,
        cycle_id,
        mode,
        generation: new_generation,
    })
}

fn reconcile_cancellation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    cancellation_request_id: CancellationRequestId,
) -> Result<EventBody, Rejection> {
    // This foundation accepts one atomic terminal fact only from the compiled
    // kernel-service grant and only after its currently modeled Office work is
    // gone. It is not process-liveness evidence. Milestone 4 must refine this
    // seam with typed propagation, signal, wait/reap, evidence-sealing, and
    // containment receipts before a supervised child can reach this command.
    let (cycle_id, state) = transaction.query_row(
        "SELECT operating_cycle_id, lifecycle_state FROM cancellation_requests WHERE cancellation_request_id = ?1",
        [cancellation_request_id.value()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    if state == CancellationState::Completed as i64
        || state == CancellationState::ContainmentFailed as i64
    {
        return Err(Rejection::CancellationAlreadyTerminal);
    }
    let cycle_id = OperatingCycleId::try_from(cycle_id).map_err(|_| Rejection::SubjectNotFound)?;
    let cycle = cycle_row(transaction, cycle_id)?;
    if cycle.state != OperatingCycleState::Cancelling {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    if active_office_turn_count(transaction, cycle_id)? != 0
        || live_office_session_count(transaction, cycle_id)? != 0
    {
        return Err(Rejection::IncompleteCycleReconciliation);
    }
    let has_pi_child: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM native_children p JOIN native_child_spawn_admissions a ON a.native_child_spawn_admission_id = p.native_child_spawn_admission_id WHERE a.operating_cycle_id = ?1)",
        [cycle_id.value()], |row| row.get::<_, i64>(0),
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)? != 0;
    if has_pi_child {
        let propagation_reconciled: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM cancellation_propagations WHERE cancellation_request_id = ?1 AND lifecycle_state = ?2)",
            params![cancellation_request_id.value(), CancellationPropagationState::Reconciled as i64],
            |row| row.get::<_, i64>(0),
        ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)? != 0;
        if !propagation_reconciled {
            return Err(Rejection::CancellationPropagationIncomplete);
        }
    }
    transaction
        .execute(
            "UPDATE cancellation_requests SET lifecycle_state = ?1, reconciled_by_command_id = ?2
         WHERE cancellation_request_id = ?3",
            params![
                CancellationState::Completed as i64,
                command_row_id,
                cancellation_request_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        OperatingCycleState::Drained,
        cycle.generation,
    )?;
    Ok(EventBody::CancellationReconciled {
        cancellation_request_id,
        cycle_id,
    })
}

/// Closes one automatically opened cost Postmortem and performs the only
/// terminal accounting transition permitted for its Frozen reservation. The
/// resolution is deliberately closed over its cause: uncertain accounting is
/// charged conservatively, while a known overrun records the observed amount.
fn close_cost_postmortem(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    postmortem_id: CostPostmortemId,
    resolution: CostPostmortemResolution,
) -> Result<EventBody, Rejection> {
    let (reservation_id, cycle_id, cause, observed, reserved, state) = transaction
        .query_row(
            "SELECT budget_reservation_id, operating_cycle_id, cause_kind, observed_micros,
                    reserved_micros, lifecycle_state
             FROM cost_postmortems WHERE postmortem_id = ?1",
            [postmortem_id.value()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    if state != CostPostmortemState::Open as i64 {
        return Err(Rejection::CostPostmortemNotOpen);
    }
    let reservation_id =
        BudgetReservationId::try_from(reservation_id).map_err(|_| Rejection::SubjectNotFound)?;
    let cycle_id = OperatingCycleId::try_from(cycle_id).map_err(|_| Rejection::SubjectNotFound)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if !matches!(
        cycle.state,
        OperatingCycleState::Drained | OperatingCycleState::Reconciling
    ) || active_cancellation_count(transaction, cycle_id)? != 0
    {
        return Err(Rejection::IncompleteCycleReconciliation);
    }
    let cause = cost_postmortem_cause_from_i64(cause).map_err(|_| Rejection::SubjectNotFound)?;
    let reserved = UsdMicros::try_from(reserved).map_err(|_| Rejection::SubjectNotFound)?;
    let (reservation_state, reservation_amount, reservation_charged): (i64, i64, i64) = transaction
        .query_row(
            "SELECT reservation_state, amount_micros, charged_micros
             FROM budget_reservations WHERE budget_reservation_id = ?1",
            [reservation_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    if reservation_state != BudgetReservationState::Frozen as i64 {
        return Err(Rejection::ReservationNotActive);
    }
    let reservation_charged =
        UsdMicros::try_from(reservation_charged).map_err(|_| Rejection::ReservationNotActive)?;
    let reservation_amount =
        UsdMicros::try_from(reservation_amount).map_err(|_| Rejection::ReservationNotActive)?;
    let remaining = reserved;
    if reservation_amount
        .checked_sub(reservation_charged)
        .ok_or(Rejection::ReservationNotActive)?
        != remaining
    {
        return Err(Rejection::ReservationNotActive);
    }
    let charged = match (cause, resolution, observed) {
        (
            CostPostmortemCause::KnownOverrun,
            CostPostmortemResolution::ChargeObservedOverrun,
            Some(observed),
        ) => UsdMicros::try_from(observed)
            .map_err(|_| Rejection::SubjectNotFound)?
            .checked_sub(reservation_charged)
            .ok_or(Rejection::InvalidCostPostmortemResolution)?,
        (
            CostPostmortemCause::UnknownCost | CostPostmortemCause::UnavailableCost,
            CostPostmortemResolution::ConservativeFullReservation,
            None,
        ) => remaining,
        _ => return Err(Rejection::InvalidCostPostmortemResolution),
    };
    if charged < remaining && cause == CostPostmortemCause::KnownOverrun {
        return Err(Rejection::ReservationNotActive);
    }
    let mut charges = transaction
        .prepare(
            "SELECT budget_envelope_id, amount_micros FROM budget_reservation_charges
             WHERE budget_reservation_id = ?1 ORDER BY budget_envelope_id",
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let charges = charges
        .query_map([reservation_id.value()], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|_| Rejection::SubjectNotFound)?;
    for charge in charges {
        let (envelope_id, reserved_charge) = charge.map_err(|_| Rejection::SubjectNotFound)?;
        transaction
            .execute(
                "UPDATE budget_envelopes
                 SET reserved_micros = reserved_micros - ?1, spent_micros = spent_micros + ?2
                 WHERE budget_envelope_id = ?3",
                params![reserved_charge, charged.value(), envelope_id],
            )
            .map_err(|_| Rejection::BudgetCeilingExceeded)?;
    }
    transaction
        .execute(
            "UPDATE budget_reservations
             SET reservation_state = ?1, charged_micros = ?2, reconciled_by_command_id = ?3
             WHERE budget_reservation_id = ?4",
            params![
                BudgetReservationState::Reconciled as i64,
                reservation_charged
                    .checked_add(charged)
                    .ok_or(Rejection::BudgetCeilingExceeded)?
                    .value(),
                command_row_id,
                reservation_id.value(),
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    transaction
        .execute(
            "UPDATE cost_postmortems SET lifecycle_state = ?1, closed_by_command_id = ?2
             WHERE postmortem_id = ?3",
            params![
                CostPostmortemState::Closed as i64,
                command_row_id,
                postmortem_id.value(),
            ],
        )
        .map_err(|_| Rejection::CostPostmortemNotOpen)?;
    transaction
        .execute(
            "INSERT INTO cost_postmortem_resolutions(postmortem_id, resolution_kind, charged_micros,
                                                      resolved_by_command_id)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                postmortem_id.value(),
                resolution as i64,
                charged.value(),
                command_row_id,
            ],
        )
        .map_err(|_| Rejection::CostPostmortemNotOpen)?;
    Ok(EventBody::CostPostmortemClosed {
        postmortem_id,
        reservation_id,
        cycle_id,
        resolution,
        charged,
    })
}

/// Every coordination command is attributed to the exact Operating Cycle in
/// which it acted. Projects and causal Episodes intentionally retain only
/// their founding-mission/project identity, so a successor cycle does not rewrite their
/// historical scope into a false single-cycle ownership claim.
fn coordination_cycle(
    transaction: &Transaction<'_>,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
) -> Result<CycleRow, Rejection> {
    let cycle = cycle_for_generation(transaction, operating_cycle_id, expected_generation)?;
    if !cycle.state.admits_task_work() {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    Ok(cycle)
}

fn record_coordination_provenance(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    cycle: CycleRow,
    operating_cycle_id: OperatingCycleId,
    project_id: Option<ProjectId>,
) -> Result<(), Rejection> {
    transaction.execute(
        "INSERT INTO coordination_command_provenance(command_row_id, founding_mission_id, operating_cycle_id, project_id)
         VALUES (?1, ?2, ?3, ?4)",
        params![command_row_id, cycle.mission_id.value(), operating_cycle_id.value(), project_id.map(ProjectId::value)],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(())
}

fn project_row(
    transaction: &Transaction<'_>,
    project_id: ProjectId,
) -> Result<(ProjectState, FoundingMissionId), Rejection> {
    let row = transaction
        .query_row(
            "SELECT lifecycle_state, founding_mission_id FROM projects WHERE project_id = ?1",
            [project_id.value()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        project_state_from_i64(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        FoundingMissionId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn create_project(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_name: &str,
    north_star_alignment: &ProjectNorthStarAlignment,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, operating_cycle_id, expected_generation)?;
    let founding_mission_application_revision_id: i64 = transaction
        .query_row(
            "SELECT application_revision_id FROM founding_missions WHERE founding_mission_id = ?1",
            [cycle.mission_id.value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    if founding_mission_application_revision_id
        != north_star_alignment.application_revision_id.value()
    {
        return Err(Rejection::ProjectNorthStarAlignmentMismatch);
    }
    transaction.execute(
        "INSERT INTO projects(project_name, founding_mission_id, lifecycle_state, created_by_command_id, last_transition_command_id)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![project_name, cycle.mission_id.value(), ProjectState::Proposed as i64, command_row_id],
    ).map_err(|_| Rejection::FoundingInvariant)?;
    let project_id = id_from_last_insert::<ProjectId>(transaction)?;
    transaction
        .execute(
            "INSERT INTO project_north_star_alignments(
                 project_id, application_revision_id, change_answer,
                 improvement_evidence_answer, boundary_commitment_answer,
                 revisit_answer, aligned_by_command_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                project_id.value(),
                north_star_alignment.application_revision_id.value(),
                north_star_alignment.change_answer.as_str(),
                north_star_alignment.improvement_evidence_answer.as_str(),
                north_star_alignment.boundary_commitment_answer.as_str(),
                north_star_alignment.revisit_answer.as_str(),
                command_row_id,
            ],
        )
        .map_err(|_| Rejection::ProjectNorthStarAlignmentMismatch)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ProjectCreated {
        project_id,
        application_revision_id: north_star_alignment.application_revision_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn charter_project(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    objective: &str,
    milestone: &str,
    stop_condition: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (state, _) = project_row(transaction, project_id)?;
    if state != ProjectState::Challenged {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE projects SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE project_id = ?3",
        params![ProjectState::Chartered as i64, command_row_id, project_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "INSERT INTO project_objectives(project_id, objective_text, chartered_by_command_id) VALUES (?1, ?2, ?3)",
        params![project_id.value(), objective, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction.execute(
        "INSERT INTO project_milestones(project_id, milestone_name, lifecycle_state, chartered_by_command_id, completed_by_command_id)
         VALUES (?1, ?2, 1, ?3, NULL)",
        params![project_id.value(), milestone, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction.execute(
        "INSERT INTO project_stop_conditions(project_id, stop_condition_text, chartered_by_command_id) VALUES (?1, ?2, ?3)",
        params![project_id.value(), stop_condition, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ProjectChartered { project_id })
}

fn project_close_blocked(
    transaction: &Transaction<'_>,
    project_id: ProjectId,
) -> Result<bool, Rejection> {
    let incomplete_milestones: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM project_milestones WHERE project_id = ?1 AND lifecycle_state != 2",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    let incomplete_tickets: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM tickets WHERE project_id = ?1 AND lifecycle_state != ?2",
            params![project_id.value(), TicketState::Completed as i64],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let open_reviews: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM adversarial_reviews WHERE project_id = ?1 AND lifecycle_state NOT IN (6, 7, 8)",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    let open_postmortems: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM postmortems WHERE project_id = ?1 AND lifecycle_state != 3",
            [project_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let live_attempts: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM attempts a JOIN attempt_budget_reservations r ON r.actor_attempt_id = a.actor_attempt_id
         WHERE r.project_id = ?1 AND a.lifecycle_state IN (1, 2)",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    let active_leases: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM leases l JOIN work_items w ON w.work_item_id = l.work_item_id
         JOIN tickets t ON t.ticket_id = w.ticket_id WHERE t.project_id = ?1 AND l.lifecycle_state = 1",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    let unreconciled_attempt_reservations: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM attempt_budget_reservations a JOIN budget_reservations b ON b.budget_reservation_id = a.budget_reservation_id
         WHERE a.project_id = ?1 AND b.reservation_state != ?2",
        params![project_id.value(), BudgetReservationState::Reconciled as i64], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    let open_outcomes: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM outcome_obligations WHERE project_id = ?1 AND lifecycle_state = 1",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    // An admitted observation is not yet a closed Experiment. The Project
    // cannot discard an evidence-producing experiment while its explicit
    // lifecycle remains open.
    let open_deterministic_experiments: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM deterministic_experiments WHERE project_id = ?1 AND lifecycle_state != 3",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    let live_pi_children: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM native_children p
         JOIN native_child_spawn_admissions s ON s.native_child_spawn_admission_id = p.native_child_spawn_admission_id
         JOIN attempt_budget_reservations r ON r.actor_attempt_id = s.actor_attempt_id
         WHERE r.project_id = ?1 AND p.lifecycle_state != ?2",
        params![project_id.value(), ChildProcessState::Finalized as i64], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    // M5 records workspace registration but deliberately does not claim a
    // secure removal/disposal action. Keep the Project open even after a child
    // reaps so a later receipt cannot be silently skipped.
    let undisposed_pi_workspaces: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM native_child_spawn_admissions s
         JOIN attempt_budget_reservations r ON r.actor_attempt_id = s.actor_attempt_id
         WHERE r.project_id = ?1",
            [project_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    Ok(incomplete_milestones != 0
        || incomplete_tickets != 0
        || open_reviews != 0
        || open_postmortems != 0
        || live_attempts != 0
        || active_leases != 0
        || unreconciled_attempt_reservations != 0
        || open_outcomes != 0
        || open_deterministic_experiments != 0
        || live_pi_children != 0
        || undisposed_pi_workspaces != 0)
}

fn project_transition_allowed(from: ProjectState, to: ProjectState) -> bool {
    matches!(
        (from, to),
        (ProjectState::Proposed, ProjectState::Challenged)
            | (ProjectState::Chartered, ProjectState::Active)
            | (
                ProjectState::Active,
                ProjectState::Paused | ProjectState::Observing | ProjectState::Terminated
            )
            | (
                ProjectState::Paused,
                ProjectState::Active | ProjectState::Terminated
            )
            | (
                ProjectState::Observing,
                ProjectState::Closed | ProjectState::Terminated
            )
            | (ProjectState::Chartered, ProjectState::Terminated)
            | (ProjectState::Reopened, ProjectState::Active)
    )
}

/// Project transitions remain narrow charter/closure control. M3's specific
/// Actor, WorkItem, and Attempt commands own execution state; this generic
/// Project transition never bypasses their live lease, reservation, outcome,
/// or independent-review close blockers.
fn transition_project(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    target: ProjectState,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (state, _) = project_row(transaction, project_id)?;
    if !project_transition_allowed(state, target) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    if target == ProjectState::Closed && project_close_blocked(transaction, project_id)? {
        return Err(Rejection::ProjectCloseBlocked);
    }
    transaction.execute("UPDATE projects SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE project_id = ?3", params![target as i64, command_row_id, project_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ProjectStateChanged {
        project_id,
        state: target,
    })
}

fn complete_project_milestone(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_milestone_id: ProjectMilestoneId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project, state) = transaction.query_row(
        "SELECT project_id, lifecycle_state FROM project_milestones WHERE project_milestone_id = ?1",
        [project_milestone_id.value()], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    let project_id = ProjectId::try_from(project).map_err(|_| Rejection::SubjectNotFound)?;
    if state != ProjectMilestoneState::Pending as i64 {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("UPDATE project_milestones SET lifecycle_state = 2, completed_by_command_id = ?1 WHERE project_milestone_id = ?2", params![command_row_id, project_milestone_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ProjectMilestoneCompleted {
        project_milestone_id,
    })
}

fn reopen_project(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (state, _) = project_row(transaction, project_id)?;
    if !matches!(state, ProjectState::Closed | ProjectState::Terminated) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("UPDATE projects SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE project_id = ?3", params![ProjectState::Reopened as i64, command_row_id, project_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ProjectStateChanged {
        project_id,
        state: ProjectState::Reopened,
    })
}

fn ticket_row(
    transaction: &Transaction<'_>,
    ticket_id: TicketId,
) -> Result<(ProjectId, TicketState), Rejection> {
    let row = transaction
        .query_row(
            "SELECT project_id, lifecycle_state FROM tickets WHERE ticket_id = ?1",
            [ticket_id.value()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        ProjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        ticket_state_from_i64(row.1).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn project_is_active(
    transaction: &Transaction<'_>,
    project_id: ProjectId,
) -> Result<(), Rejection> {
    let (state, _) = project_row(transaction, project_id)?;
    if state != ProjectState::Active {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    Ok(())
}

fn ticket_prerequisites_complete(
    transaction: &Transaction<'_>,
    ticket_id: TicketId,
) -> Result<bool, Rejection> {
    let incomplete: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM ticket_prerequisites p JOIN tickets t ON t.ticket_id = p.prerequisite_ticket_id
         WHERE p.ticket_id = ?1 AND t.lifecycle_state != ?2",
        params![ticket_id.value(), TicketState::Completed as i64], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    Ok(incomplete == 0)
}

#[allow(clippy::too_many_arguments)]
fn create_ticket(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    ticket_title: &str,
    acceptance_condition: &str,
    prerequisite_ticket_id: Option<TicketId>,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    if let Some(prerequisite_ticket_id) = prerequisite_ticket_id {
        let (prerequisite_project, _) = ticket_row(transaction, prerequisite_ticket_id)?;
        if prerequisite_project != project_id {
            return Err(Rejection::SubjectNotFound);
        }
    }
    transaction.execute(
        "INSERT INTO tickets(project_id, ticket_title, lifecycle_state, created_by_command_id, last_transition_command_id)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![project_id.value(), ticket_title, TicketState::Draft as i64, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let ticket_id = id_from_last_insert::<TicketId>(transaction)?;
    transaction.execute(
        "INSERT INTO ticket_acceptance_conditions(ticket_id, condition_text, lifecycle_state, created_by_command_id, satisfied_by_command_id)
         VALUES (?1, ?2, 1, ?3, NULL)",
        params![ticket_id.value(), acceptance_condition, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    if let Some(prerequisite_ticket_id) = prerequisite_ticket_id {
        transaction.execute(
            "INSERT INTO ticket_prerequisites(ticket_id, prerequisite_ticket_id, created_by_command_id) VALUES (?1, ?2, ?3)",
            params![ticket_id.value(), prerequisite_ticket_id.value(), command_row_id],
        ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    }
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::TicketCreated {
        ticket_id,
        project_id,
    })
}

fn transition_ticket(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    ticket_id: TicketId,
    target: TicketState,
) -> Result<EventBody, Rejection> {
    // M3 deliberately retires this broad M2 transition surface. Admission,
    // readiness, claiming, terminal settlement, validation, retry, and
    // completion each require their own Actor/WorkItem/Lease/Attempt command.
    let _ = (
        transaction,
        command_row_id,
        expected_generation,
        operating_cycle_id,
        ticket_id,
        target,
    );
    Err(Rejection::InvalidLifecycleTransition)
}

fn graph_revision_row(
    transaction: &Transaction<'_>,
    graph_revision_id: GraphRevisionId,
) -> Result<
    (
        GraphObjectId,
        ProjectId,
        GraphObjectKind,
        GraphRevisionState,
    ),
    Rejection,
> {
    let row = transaction
        .query_row(
            "SELECT r.graph_object_id, o.project_id, o.object_kind, r.lifecycle_state
         FROM object_revisions r JOIN objects o ON o.graph_object_id = r.graph_object_id
         WHERE r.graph_revision_id = ?1",
            [graph_revision_id.value()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        GraphObjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        ProjectId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        graph_object_kind_from_i64(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        graph_revision_state_from_i64(row.3).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn add_graph_object_revision(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    causal_episode_id: Option<CausalEpisodeId>,
    existing_graph_object_id: Option<GraphObjectId>,
    body: &GraphRevisionBody,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let object_kind = body.object_kind();
    project_is_active(transaction, project_id)?;
    if let Some(episode_id) = causal_episode_id {
        let episode_project: i64 = transaction
            .query_row(
                "SELECT project_id FROM episodes WHERE causal_episode_id = ?1",
                [episode_id.value()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| Rejection::SubjectNotFound)?
            .ok_or(Rejection::SubjectNotFound)?;
        if episode_project != project_id.value() {
            return Err(Rejection::SubjectNotFound);
        }
    }
    let graph_object_id = match existing_graph_object_id {
        Some(graph_object_id) => {
            let (object_project, stored_kind): (i64, i64) = transaction
                .query_row(
                    "SELECT project_id, object_kind FROM objects WHERE graph_object_id = ?1",
                    [graph_object_id.value()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|_| Rejection::SubjectNotFound)?
                .ok_or(Rejection::SubjectNotFound)?;
            if object_project != project_id.value() || stored_kind != object_kind as i64 {
                return Err(Rejection::SubjectNotFound);
            }
            graph_object_id
        }
        None => {
            transaction.execute(
                "INSERT INTO objects(project_id, causal_episode_id, founding_mission_id, object_kind, created_by_command_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![project_id.value(), causal_episode_id.map(CausalEpisodeId::value), cycle.mission_id.value(), object_kind as i64, command_row_id],
            ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
            id_from_last_insert::<GraphObjectId>(transaction)?
        }
    };
    let next_ordinal: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(revision_ordinal), 0) + 1 FROM object_revisions WHERE graph_object_id = ?1",
        [graph_object_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction.execute(
        "INSERT INTO object_revisions(graph_object_id, revision_ordinal, lifecycle_state, created_by_command_id, committed_by_command_id)
         VALUES (?1, ?2, 1, ?3, NULL)",
        params![graph_object_id.value(), next_ordinal, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let graph_revision_id = id_from_last_insert::<GraphRevisionId>(transaction)?;
    match body {
        GraphRevisionBody::Observation { observation } => {
            transaction.execute(
                "INSERT INTO observation_revisions(graph_revision_id, observation_text) VALUES (?1, ?2)",
                params![graph_revision_id.value(), observation.as_str()],
            ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
        }
        GraphRevisionBody::Hypothesis { hypothesis } => {
            transaction.execute(
                "INSERT INTO hypothesis_revisions(graph_revision_id, hypothesis_text) VALUES (?1, ?2)",
                params![graph_revision_id.value(), hypothesis.as_str()],
            ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
        }
    }
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::GraphObjectRevisionAdded {
        graph_object_id,
        graph_revision_id,
    })
}

fn commit_graph_revision(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    graph_revision_id: GraphRevisionId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (_, project_id, _, state) = graph_revision_row(transaction, graph_revision_id)?;
    project_is_active(transaction, project_id)?;
    if state != GraphRevisionState::Draft {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE object_revisions SET lifecycle_state = 2, committed_by_command_id = ?1 WHERE graph_revision_id = ?2",
        params![command_row_id, graph_revision_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::GraphRevisionCommitted { graph_revision_id })
}

#[allow(clippy::too_many_arguments)]
fn add_graph_edge(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    from_graph_revision_id: GraphRevisionId,
    to_graph_revision_id: GraphRevisionId,
    edge_kind: GraphEdgeKind,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    let (_, from_project, from_kind, from_state) =
        graph_revision_row(transaction, from_graph_revision_id)?;
    let (_, to_project, to_kind, to_state) = graph_revision_row(transaction, to_graph_revision_id)?;
    if from_project != project_id || to_project != project_id {
        return Err(Rejection::SubjectNotFound);
    }
    if from_state != GraphRevisionState::Committed || to_state != GraphRevisionState::Committed {
        return Err(Rejection::GraphRevisionNotCommitted);
    }
    if !edge_kind.allows(from_kind, to_kind) {
        return Err(Rejection::IllegalGraphEdgeEndpoint);
    }
    transaction.execute(
        "INSERT INTO edges(project_id, from_graph_revision_id, to_graph_revision_id, edge_kind, created_by_command_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![project_id.value(), from_graph_revision_id.value(), to_graph_revision_id.value(), edge_kind as i64, command_row_id],
    ).map_err(|_| Rejection::IllegalGraphEdgeEndpoint)?;
    let graph_edge_id = id_from_last_insert::<GraphEdgeId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::GraphEdgeAdded { graph_edge_id })
}

fn episode_row(
    transaction: &Transaction<'_>,
    episode_id: CausalEpisodeId,
) -> Result<(ProjectId, EpisodeState), Rejection> {
    let row: (i64, i64) = transaction
        .query_row(
            "SELECT project_id, lifecycle_state FROM episodes WHERE causal_episode_id = ?1",
            [episode_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        ProjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        episode_state_from_i64(row.1).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn episode_transition_allowed(from: EpisodeState, to: EpisodeState) -> bool {
    matches!(
        (from, to),
        (
            EpisodeState::Framed,
            EpisodeState::Admitted | EpisodeState::Abandoned
        ) | (
            EpisodeState::Admitted,
            EpisodeState::Investigating | EpisodeState::Abandoned
        ) | (
            EpisodeState::Investigating,
            EpisodeState::PrototypeDeliberating
                | EpisodeState::ClosedNoAction
                | EpisodeState::Abandoned
        ) | (
            EpisodeState::PrototypeDeliberating,
            EpisodeState::Prototyping | EpisodeState::ClosedNoAction | EpisodeState::Abandoned
        ) | (
            EpisodeState::Prototyping,
            EpisodeState::CandidateValidating | EpisodeState::Reverted | EpisodeState::Abandoned
        ) | (
            EpisodeState::CandidateValidating,
            EpisodeState::DeliveryDeliberating | EpisodeState::Reverted | EpisodeState::Abandoned
        ) | (
            EpisodeState::DeliveryDeliberating,
            EpisodeState::DeliveryAuthorized
                | EpisodeState::ClosedNoDelivery
                | EpisodeState::Abandoned
        ) | (
            EpisodeState::DeliveryAuthorized,
            EpisodeState::Materializing | EpisodeState::Abandoned
        ) | (
            EpisodeState::Materializing,
            EpisodeState::Observing | EpisodeState::Abandoned
        ) | (
            EpisodeState::Observing,
            EpisodeState::Learning | EpisodeState::Closed | EpisodeState::Abandoned
        ) | (
            EpisodeState::Learning,
            EpisodeState::Closed | EpisodeState::Abandoned
        ) | (
            EpisodeState::Reopened,
            EpisodeState::Investigating | EpisodeState::Abandoned
        )
    )
}

fn create_episode(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    transaction.execute(
        "INSERT INTO episodes(project_id, founding_mission_id, lifecycle_state, created_by_command_id, last_transition_command_id) VALUES (?1, ?2, 1, ?3, ?3)",
        params![project_id.value(), cycle.mission_id.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let causal_episode_id = id_from_last_insert::<CausalEpisodeId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::EpisodeCreated {
        causal_episode_id,
        project_id,
    })
}

fn transition_episode(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    causal_episode_id: CausalEpisodeId,
    target: EpisodeState,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = episode_row(transaction, causal_episode_id)?;
    if !episode_transition_allowed(state, target) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("UPDATE episodes SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE causal_episode_id = ?3", params![target as i64, command_row_id, causal_episode_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::EpisodeStateChanged {
        causal_episode_id,
        state: target,
    })
}

fn reopen_episode(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    causal_episode_id: CausalEpisodeId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = episode_row(transaction, causal_episode_id)?;
    if !matches!(
        state,
        EpisodeState::Closed
            | EpisodeState::ClosedNoAction
            | EpisodeState::ClosedNoDelivery
            | EpisodeState::Reverted
    ) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("UPDATE episodes SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE causal_episode_id = ?3", params![EpisodeState::Reopened as i64, command_row_id, causal_episode_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::EpisodeStateChanged {
        causal_episode_id,
        state: EpisodeState::Reopened,
    })
}

fn review_row(
    transaction: &Transaction<'_>,
    review_id: AdversarialReviewId,
) -> Result<(ProjectId, GraphRevisionId, AdversarialReviewState), Rejection> {
    let row: (i64, i64, i64) = transaction.query_row(
        "SELECT project_id, target_graph_revision_id, lifecycle_state FROM adversarial_reviews WHERE adversarial_review_id = ?1",
        [review_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        ProjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        GraphRevisionId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        adversarial_review_state_from_i64(row.2).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn request_adversarial_review(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    target_graph_revision_id: GraphRevisionId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    let (_, revision_project, _, revision_state) =
        graph_revision_row(transaction, target_graph_revision_id)?;
    if revision_project != project_id {
        return Err(Rejection::SubjectNotFound);
    }
    if revision_state != GraphRevisionState::Committed {
        return Err(Rejection::GraphRevisionNotCommitted);
    }
    transaction.execute(
        "INSERT INTO adversarial_reviews(project_id, target_graph_revision_id, lifecycle_state, requested_by_command_id, assigned_reviewer_principal_id, resolved_by_command_id) VALUES (?1, ?2, 1, ?3, NULL, NULL)",
        params![project_id.value(), target_graph_revision_id.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let adversarial_review_id = id_from_last_insert::<AdversarialReviewId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::AdversarialReviewRequested {
        adversarial_review_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn assign_adversarial_reviewer(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    adversarial_review_id: AdversarialReviewId,
    reviewer_principal_id: PrincipalId,
    reviewer_actor_instance_id: ActorInstanceId,
    reviewer_actor_attempt_id: ActorAttemptId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, _, review_state) = review_row(transaction, adversarial_review_id)?;
    if review_state != AdversarialReviewState::Requested {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let cycle_root_authority: i64 = transaction
        .query_row(
            "SELECT principal_id FROM office_occupancies WHERE office_occupancy_id = ?1",
            [cycle.occupancy_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    if reviewer_principal_id.value() == cycle_root_authority {
        return Err(Rejection::ReviewAssignmentNotIndependent);
    }
    let (actor_principal, _, _, actor_cycle, actor_state) =
        actor_instance_row(transaction, reviewer_actor_instance_id)?;
    if actor_principal != reviewer_principal_id
        || actor_cycle != operating_cycle_id
        || actor_state != ActorInstanceState::Active
    {
        return Err(Rejection::ReviewAssignmentEvidenceMissing);
    }
    let (attempt_cycle, ticket_id, work_item_id, _, attempt_actor, attempt_state) =
        actor_attempt_row(transaction, reviewer_actor_attempt_id)?;
    let (ticket_project, _) = ticket_row(transaction, ticket_id)?;
    let (_, _, context_pack_id, work_kind, bound_review_id, _, _) =
        work_item_row(transaction, work_item_id)?;
    let context_purpose: i64 = transaction
        .query_row(
            "SELECT purpose FROM context_packs WHERE context_pack_id = ?1",
            [context_pack_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ReviewAssignmentEvidenceMissing)?;
    if attempt_cycle != operating_cycle_id
        || attempt_actor != reviewer_actor_instance_id
        || ticket_project != project_id
        || work_kind != WorkItemKind::IndependentReview
        || bound_review_id != Some(adversarial_review_id)
        || context_pack_purpose_from_i64(context_purpose)
            .map_err(|_| Rejection::ReviewAssignmentEvidenceMissing)?
            != ContextPackPurpose::IndependentReview
        || !matches!(
            attempt_state,
            ActorAttemptState::Succeeded | ActorAttemptState::Validated
        )
    {
        return Err(Rejection::ReviewAssignmentEvidenceMissing);
    }
    // The service assigns a named, active Actor. The author check at finding
    // submission compares against this durable assignment; Principal kind is
    // only a prerequisite, never reviewer jurisdiction by itself.
    let eligible: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM principals WHERE principal_id = ?1 AND principal_kind = ?2 AND active = 1)",
            params![reviewer_principal_id.value(), PrincipalKind::Actor as i64],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?
        != 0;
    if !eligible {
        return Err(Rejection::SubjectNotFound);
    }
    transaction
        .execute(
            "UPDATE adversarial_reviews SET lifecycle_state = ?1, assigned_reviewer_principal_id = ?2, assigned_reviewer_actor_instance_id = ?3, reviewer_actor_attempt_id = ?4 WHERE adversarial_review_id = ?5",
            params![
                AdversarialReviewState::Assigned as i64,
                reviewer_principal_id.value(),
                reviewer_actor_instance_id.value(),
                reviewer_actor_attempt_id.value(),
                adversarial_review_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::AdversarialReviewerAssigned {
        adversarial_review_id,
        reviewer_principal_id,
        reviewer_actor_instance_id,
        reviewer_actor_attempt_id,
    })
}

/// A Review finding is submitted by the kernel service on behalf of the exact
/// independently provisioned Actor named by assignment evidence. M3 provides
/// the minimum WorkItem/Attempt foundation for resolution; Pi/process evidence
/// remains outside the trusted claim until the supervisor receipt tranche.
#[allow(clippy::too_many_arguments)]
fn submit_review_challenge(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    author_principal_id: PrincipalId,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    adversarial_review_id: AdversarialReviewId,
    target_graph_revision_id: GraphRevisionId,
    severity: ReviewChallengeSeverity,
    failure_hypothesis: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, review_target, review_state) = review_row(transaction, adversarial_review_id)?;
    let assigned_reviewer: (Option<i64>, Option<i64>) = transaction
        .query_row(
            "SELECT assigned_reviewer_principal_id, assigned_reviewer_actor_instance_id FROM adversarial_reviews WHERE adversarial_review_id = ?1",
            [adversarial_review_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    if assigned_reviewer.0 != Some(author_principal_id.value()) {
        return Err(Rejection::CapabilityNotGranted);
    }
    let Some(assigned_actor_instance_id) = assigned_reviewer.1 else {
        return Err(Rejection::ReviewAssignmentEvidenceMissing);
    };
    let (assigned_principal, _, _, assigned_cycle, assigned_state) = actor_instance_row(
        transaction,
        ActorInstanceId::try_from(assigned_actor_instance_id)
            .map_err(|_| Rejection::ReviewAssignmentEvidenceMissing)?,
    )?;
    if assigned_principal != author_principal_id
        || assigned_cycle != operating_cycle_id
        || assigned_state != ActorInstanceState::Active
    {
        return Err(Rejection::ReviewAssignmentEvidenceMissing);
    }
    if review_target != target_graph_revision_id
        || !matches!(
            review_state,
            AdversarialReviewState::Assigned
                | AdversarialReviewState::Active
                | AdversarialReviewState::FindingsSubmitted
                | AdversarialReviewState::ResponsesDue
        )
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let (_, revision_project, _, revision_state) =
        graph_revision_row(transaction, target_graph_revision_id)?;
    if revision_project != project_id {
        return Err(Rejection::SubjectNotFound);
    }
    if revision_state != GraphRevisionState::Committed {
        return Err(Rejection::GraphRevisionNotCommitted);
    }
    transaction.execute(
        "INSERT INTO review_challenges(adversarial_review_id, target_graph_revision_id, author_principal_id, severity, failure_hypothesis, response_state, submitted_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
        params![adversarial_review_id.value(), target_graph_revision_id.value(), author_principal_id.value(), severity as i64, failure_hypothesis, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let review_challenge_id = id_from_last_insert::<ReviewChallengeId>(transaction)?;
    transaction
        .execute(
            "UPDATE adversarial_reviews SET lifecycle_state = 5 WHERE adversarial_review_id = ?1",
            [adversarial_review_id.value()],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ReviewChallengeSubmitted {
        review_challenge_id,
        author_principal_id,
    })
}

fn review_challenge_row(
    transaction: &Transaction<'_>,
    challenge_id: ReviewChallengeId,
) -> Result<
    (
        AdversarialReviewId,
        ProjectId,
        PrincipalId,
        ReviewChallengeResponseState,
    ),
    Rejection,
> {
    let row: (i64, i64, i64, i64) = transaction.query_row(
        "SELECT c.adversarial_review_id, r.project_id, c.author_principal_id, c.response_state
         FROM review_challenges c JOIN adversarial_reviews r ON r.adversarial_review_id = c.adversarial_review_id
         WHERE c.review_challenge_id = ?1",
        [challenge_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        AdversarialReviewId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        ProjectId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        PrincipalId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        review_challenge_response_state_from_i64(row.3).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn respond_to_review_challenge(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    review_challenge_id: ReviewChallengeId,
    response: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (_, project_id, _, response_state) =
        review_challenge_row(transaction, review_challenge_id)?;
    if response_state != ReviewChallengeResponseState::Pending {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("INSERT INTO review_challenge_responses(review_challenge_id, response_text, responded_by_command_id) VALUES (?1, ?2, ?3)", params![review_challenge_id.value(), response, command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction
        .execute(
            "UPDATE review_challenges SET response_state = 2 WHERE review_challenge_id = ?1",
            [review_challenge_id.value()],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ReviewChallengeResponded {
        review_challenge_id,
    })
}

fn disposition_review_challenge(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    principal_id: PrincipalId,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    review_challenge_id: ReviewChallengeId,
    disposition: ReviewDispositionKind,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (_, project_id, author, response_state) =
        review_challenge_row(transaction, review_challenge_id)?;
    if principal_id == author {
        return Err(Rejection::ReviewSelfDispositionDenied);
    }
    if response_state != ReviewChallengeResponseState::Responded {
        return Err(Rejection::ReviewDispositionIncomplete);
    }
    transaction.execute("INSERT INTO review_dispositions(review_challenge_id, disposition_kind, disposed_by_principal_id, disposed_by_command_id) VALUES (?1, ?2, ?3, ?4)", params![review_challenge_id.value(), disposition as i64, principal_id.value(), command_row_id]).map_err(|_| Rejection::ReviewDispositionIncomplete)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ReviewChallengeDispositioned {
        review_challenge_id,
        disposition,
    })
}

fn resolve_adversarial_review(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    adversarial_review_id: AdversarialReviewId,
    resolution: ReviewResolutionKind,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, _, state) = review_row(transaction, adversarial_review_id)?;
    if state != AdversarialReviewState::ResponsesDue {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let missing: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM review_challenges c LEFT JOIN review_dispositions d ON d.review_challenge_id = c.review_challenge_id
         WHERE c.adversarial_review_id = ?1 AND (c.response_state != 2 OR d.review_disposition_id IS NULL)",
        [adversarial_review_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    if missing != 0 {
        return Err(Rejection::ReviewDispositionIncomplete);
    }
    let target_state = match resolution {
        ReviewResolutionKind::Resolved => AdversarialReviewState::Resolved,
        ReviewResolutionKind::AcceptedRisk => AdversarialReviewState::AcceptedRisk,
    };
    transaction.execute("UPDATE adversarial_reviews SET lifecycle_state = ?1, resolved_by_command_id = ?2 WHERE adversarial_review_id = ?3", params![target_state as i64, command_row_id, adversarial_review_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::AdversarialReviewResolved {
        adversarial_review_id,
        state: target_state,
    })
}

fn postmortem_row(
    transaction: &Transaction<'_>,
    postmortem_id: PostmortemId,
) -> Result<(ProjectId, PostmortemState), Rejection> {
    let row: (i64, i64) = transaction
        .query_row(
            "SELECT project_id, lifecycle_state FROM postmortems WHERE postmortem_id = ?1",
            [postmortem_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        ProjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        postmortem_state_from_i64(row.1).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn trigger_postmortem(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    causal_episode_id: Option<CausalEpisodeId>,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    if let Some(episode) = causal_episode_id
        && episode_row(transaction, episode)?.0 != project_id
    {
        return Err(Rejection::SubjectNotFound);
    }
    transaction.execute("INSERT INTO postmortems(project_id, causal_episode_id, founding_mission_id, lifecycle_state, triggered_by_command_id, closed_by_command_id) VALUES (?1, ?2, ?3, 1, ?4, NULL)", params![project_id.value(), causal_episode_id.map(CausalEpisodeId::value), cycle.mission_id.value(), command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let postmortem_id = id_from_last_insert::<PostmortemId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::PostmortemTriggered { postmortem_id })
}

fn record_postmortem_causal_claim(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    postmortem_id: PostmortemId,
    claim_kind: PostmortemCausalClaimKind,
    claim: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = postmortem_row(transaction, postmortem_id)?;
    if state == PostmortemState::Closed {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("INSERT INTO postmortem_causal_claims(postmortem_id, claim_kind, claim_text, recorded_by_command_id) VALUES (?1, ?2, ?3, ?4)", params![postmortem_id.value(), claim_kind as i64, claim, command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let postmortem_causal_claim_id = id_from_last_insert::<PostmortemCausalClaimId>(transaction)?;
    transaction
        .execute(
            "UPDATE postmortems SET lifecycle_state = 2 WHERE postmortem_id = ?1",
            [postmortem_id.value()],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::PostmortemCausalClaimRecorded {
        postmortem_causal_claim_id,
    })
}

fn propose_postmortem_action(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    postmortem_id: PostmortemId,
    action_kind: PostmortemActionKind,
    action: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = postmortem_row(transaction, postmortem_id)?;
    if state == PostmortemState::Closed {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("INSERT INTO postmortem_action_proposals(postmortem_id, action_kind, action_text, proposed_by_command_id) VALUES (?1, ?2, ?3, ?4)", params![postmortem_id.value(), action_kind as i64, action, command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let postmortem_action_proposal_id =
        id_from_last_insert::<PostmortemActionProposalId>(transaction)?;
    transaction
        .execute(
            "UPDATE postmortems SET lifecycle_state = 2 WHERE postmortem_id = ?1",
            [postmortem_id.value()],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::PostmortemActionProposed {
        postmortem_action_proposal_id,
    })
}

fn close_postmortem(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    postmortem_id: PostmortemId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = postmortem_row(transaction, postmortem_id)?;
    if state == PostmortemState::Closed {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let counts: (i64, i64) = transaction.query_row(
        "SELECT (SELECT COUNT(*) FROM postmortem_causal_claims WHERE postmortem_id = ?1), (SELECT COUNT(*) FROM postmortem_action_proposals WHERE postmortem_id = ?1)",
        [postmortem_id.value()], |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    if counts.0 == 0 || counts.1 == 0 {
        return Err(Rejection::PostmortemCloseBlocked);
    }
    transaction.execute("UPDATE postmortems SET lifecycle_state = 3, closed_by_command_id = ?1 WHERE postmortem_id = ?2", params![command_row_id, postmortem_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::PostmortemClosed { postmortem_id })
}

/// M3 is the first executable, but deliberately receipt-free, task boundary.
/// Its kernel-service terminal attestations are atomic trusted facts only; the
/// later supervisor tranche must bind them to Pi/process/evidence receipts.
fn register_actor_configuration(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    configuration_name: &str,
    model_policy: ActorModelPolicy,
    primary_attractor: DevelopmentalAttractor,
) -> Result<EventBody, Rejection> {
    transaction.execute(
        "INSERT INTO actor_configurations(configuration_name, lifecycle_state, created_by_command_id) VALUES (?1, 1, ?2)",
        params![configuration_name, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let actor_configuration_id = id_from_last_insert::<ActorConfigurationId>(transaction)?;
    transaction.execute(
        "INSERT INTO actor_configuration_revisions(actor_configuration_id, revision_ordinal, model_policy, primary_attractor, created_by_command_id) VALUES (?1, 1, ?2, ?3, ?4)",
        params![actor_configuration_id.value(), model_policy as i64, primary_attractor as i64, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::ActorConfigurationRegistered {
        actor_configuration_id,
        actor_configuration_revision_id: id_from_last_insert::<ActorConfigurationRevisionId>(
            transaction,
        )?,
    })
}

fn register_context_pack(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    purpose: ContextPackPurpose,
    rendering_digest: Blake3Digest,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    transaction.execute(
        "INSERT INTO context_packs(founding_mission_id, purpose, rendering_digest, created_by_command_id) VALUES (?1, ?2, ?3, ?4)",
        params![cycle.mission_id.value(), purpose as i64, rendering_digest.as_bytes(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let context_pack_id = id_from_last_insert::<ContextPackId>(transaction)?;
    record_coordination_provenance(transaction, command_row_id, cycle, operating_cycle_id, None)?;
    Ok(EventBody::ContextPackRegistered { context_pack_id })
}

fn admit_actor_instance(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    actor_configuration_revision_id: ActorConfigurationRevisionId,
    execution_profile_id: ExecutionProfileId,
    actor_display_name: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let configuration_active: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM actor_configuration_revisions r JOIN actor_configurations c ON c.actor_configuration_id = r.actor_configuration_id WHERE r.actor_configuration_revision_id = ?1 AND c.lifecycle_state = 1)",
        [actor_configuration_revision_id.value()], |row| row.get::<_, i64>(0),
    ).map_err(|_| Rejection::SubjectNotFound)? != 0;
    let profile: Option<(i64, i64)> = transaction
        .query_row(
            "SELECT profile_kind, readiness FROM execution_profiles WHERE execution_profile_id = ?1",
            [execution_profile_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?;
    let profile_is_admissible = match profile {
        Some((kind, readiness)) => matches!(
            (
                cycle._treatment,
                execution_profile_kind_from_i64(kind),
                execution_profile_readiness_from_i64(readiness),
            ),
            (
                OperatingCycleTreatment::DeterministicPiHostFixtureV1,
                Ok(ExecutionProfileKind::DeterministicPiHostProcessDoubleV1),
                Ok(ExecutionProfileReadiness::DeterministicFixtureOnly),
            ) | (
                OperatingCycleTreatment::PinnedPiSdkLiveV1,
                Ok(ExecutionProfileKind::NativePinnedPiSdkV1),
                Ok(ExecutionProfileReadiness::QualifiedForLiveUse),
            )
        ),
        None => false,
    };
    if !configuration_active {
        return Err(Rejection::SubjectNotFound);
    }
    if !profile_is_admissible {
        return Err(Rejection::ExecutionProfileIneligible);
    }
    transaction
        .execute(
            "INSERT INTO principals(principal_kind, display_name, active) VALUES (?1, ?2, 1)",
            params![PrincipalKind::Actor as i64, actor_display_name],
        )
        .map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let principal_id = id_from_last_insert::<PrincipalId>(transaction)?;
    transaction.execute(
        "INSERT INTO actor_instances(principal_id, actor_configuration_revision_id, execution_profile_id, operating_cycle_id, lifecycle_state, admitted_by_command_id) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
        params![principal_id.value(), actor_configuration_revision_id.value(), execution_profile_id.value(), operating_cycle_id.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let actor_instance_id = id_from_last_insert::<ActorInstanceId>(transaction)?;
    transaction.execute(
        "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id, actor_instance_id, grant_state, grant_origin, granted_by_command_id, consumed_by_command_id) VALUES (?1, ?2, NULL, ?3, 1, 2, ?4, NULL)",
        params![principal_id.value(), Capability::ClaimWorkItem as i64, actor_instance_id.value(), command_row_id],
    ).map_err(|_| Rejection::ActorJurisdictionDenied)?;
    record_coordination_provenance(transaction, command_row_id, cycle, operating_cycle_id, None)?;
    Ok(EventBody::ActorInstanceAdmitted {
        actor_instance_id,
        principal_id,
    })
}

/// The Pi child bridge repeats the M3 admission matrix at its final Create
/// gate. `PiSdkQualificationV1` has no M5 Office/Actor owner constructor yet:
/// bootstrap-native qualification remains a later explicit authority path, not
/// a disguised Root Authority session. The deterministic double is confined
/// to its provider-free fixture treatment and cannot cross into paid/native
/// qualification or live work.
fn pi_child_profile_allowed(
    treatment: OperatingCycleTreatment,
    kind: ExecutionProfileKind,
    readiness: ExecutionProfileReadiness,
) -> bool {
    matches!(
        (treatment, kind, readiness),
        (
            OperatingCycleTreatment::DeterministicPiHostFixtureV1,
            ExecutionProfileKind::DeterministicPiHostProcessDoubleV1,
            ExecutionProfileReadiness::DeterministicFixtureOnly,
        ) | (
            OperatingCycleTreatment::PiSdkQualificationV1,
            ExecutionProfileKind::NativePinnedPiSdkV1,
            ExecutionProfileReadiness::Unqualified | ExecutionProfileReadiness::QualifiedForLiveUse,
        ) | (
            OperatingCycleTreatment::PinnedPiSdkLiveV1,
            ExecutionProfileKind::NativePinnedPiSdkV1,
            ExecutionProfileReadiness::QualifiedForLiveUse,
        )
    )
}

fn admit_ticket(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    ticket_id: TicketId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = ticket_row(transaction, ticket_id)?;
    project_is_active(transaction, project_id)?;
    if state != TicketState::Draft {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3",
        params![TicketState::Admitted as i64, command_row_id, ticket_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::TicketAdmitted { ticket_id })
}

fn actor_instance_row(
    transaction: &Transaction<'_>,
    actor_instance_id: ActorInstanceId,
) -> Result<
    (
        PrincipalId,
        ActorConfigurationRevisionId,
        ExecutionProfileId,
        OperatingCycleId,
        ActorInstanceState,
    ),
    Rejection,
> {
    let row: (i64, i64, i64, i64, i64) = transaction.query_row(
        "SELECT principal_id, actor_configuration_revision_id, execution_profile_id, operating_cycle_id, lifecycle_state FROM actor_instances WHERE actor_instance_id = ?1",
        [actor_instance_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        PrincipalId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        ActorConfigurationRevisionId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        ExecutionProfileId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        OperatingCycleId::try_from(row.3).map_err(|_| Rejection::SubjectNotFound)?,
        actor_instance_state_from_i64(row.4).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn register_work_item(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    ticket_id: TicketId,
    actor_instance_id: ActorInstanceId,
    context_pack_id: ContextPackId,
    work_kind: WorkItemKind,
    adversarial_review_id: Option<AdversarialReviewId>,
    assignment: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, ticket_state) = ticket_row(transaction, ticket_id)?;
    project_is_active(transaction, project_id)?;
    if ticket_state != TicketState::Admitted
        || !ticket_prerequisites_complete(transaction, ticket_id)?
    {
        return Err(Rejection::TicketPrerequisiteIncomplete);
    }
    let (_, _, _, actor_cycle, actor_state) = actor_instance_row(transaction, actor_instance_id)?;
    if actor_cycle != operating_cycle_id || actor_state != ActorInstanceState::Active {
        return Err(Rejection::ActorJurisdictionDenied);
    }
    let context: (i64, i64) = transaction
        .query_row(
            "SELECT founding_mission_id, purpose FROM context_packs WHERE context_pack_id = ?1",
            [context_pack_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    if context.0 != cycle.mission_id.value()
        || context_pack_purpose_from_i64(context.1).map_err(|_| Rejection::SubjectNotFound)?
            != work_kind.required_context_purpose()
    {
        return Err(Rejection::ActorJurisdictionDenied);
    }
    match (work_kind, adversarial_review_id) {
        (WorkItemKind::TicketExecution, None) => {}
        (WorkItemKind::IndependentReview, Some(review_id)) => {
            let (review_project_id, target_revision_id, review_state) =
                review_row(transaction, review_id)?;
            let (_, revision_project_id, _, revision_state) =
                graph_revision_row(transaction, target_revision_id)?;
            if review_project_id != project_id
                || review_state != AdversarialReviewState::Requested
                || revision_project_id != project_id
                || revision_state != GraphRevisionState::Committed
            {
                return Err(Rejection::ReviewAssignmentEvidenceMissing);
            }
        }
        _ => return Err(Rejection::ReviewAssignmentEvidenceMissing),
    }
    transaction.execute(
        "INSERT INTO work_items(ticket_id, actor_instance_id, context_pack_id, work_kind, adversarial_review_id, assignment_text, lifecycle_state, retry_of_actor_attempt_id, created_by_command_id, last_transition_command_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, NULL, ?7, ?7)",
        params![ticket_id.value(), actor_instance_id.value(), context_pack_id.value(), work_kind as i64, adversarial_review_id.map(AdversarialReviewId::value), assignment, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let work_item_id = id_from_last_insert::<WorkItemId>(transaction)?;
    transaction.execute(
        "UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3",
        params![TicketState::Ready as i64, command_row_id, ticket_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::WorkItemRegistered {
        work_item_id,
        ticket_id,
        adversarial_review_id,
    })
}

fn work_item_row(
    transaction: &Transaction<'_>,
    work_item_id: WorkItemId,
) -> Result<WorkItemRow, Rejection> {
    let row: (i64, i64, i64, i64, Option<i64>, i64, Option<i64>) = transaction.query_row(
        "SELECT ticket_id, actor_instance_id, context_pack_id, work_kind, adversarial_review_id, lifecycle_state, retry_of_actor_attempt_id FROM work_items WHERE work_item_id = ?1",
        [work_item_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        TicketId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        ActorInstanceId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        ContextPackId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        work_item_kind_from_i64(row.3).map_err(|_| Rejection::SubjectNotFound)?,
        row.4
            .map(AdversarialReviewId::try_from)
            .transpose()
            .map_err(|_| Rejection::SubjectNotFound)?,
        work_item_state_from_i64(row.5).map_err(|_| Rejection::SubjectNotFound)?,
        row.6
            .map(ActorAttemptId::try_from)
            .transpose()
            .map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn claim_work_item(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    principal_id: PrincipalId,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    work_item_id: WorkItemId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (ticket_id, actor_instance_id, _, _, _, state, _) =
        work_item_row(transaction, work_item_id)?;
    let (actor_principal, _, _, actor_cycle, actor_state) =
        actor_instance_row(transaction, actor_instance_id)?;
    let (project_id, ticket_state) = ticket_row(transaction, ticket_id)?;
    project_is_active(transaction, project_id)?;
    if principal_id != actor_principal
        || actor_cycle != operating_cycle_id
        || actor_state != ActorInstanceState::Active
    {
        return Err(Rejection::ActorJurisdictionDenied);
    }
    if state != WorkItemState::Ready || ticket_state != TicketState::Ready {
        return Err(Rejection::WorkLeaseUnavailable);
    }
    transaction.execute(
        "INSERT INTO leases(work_item_id, actor_instance_id, lifecycle_state, claimed_by_command_id, terminal_by_command_id) VALUES (?1, ?2, 1, ?3, NULL)",
        params![work_item_id.value(), actor_instance_id.value(), command_row_id],
    ).map_err(|_| Rejection::WorkLeaseUnavailable)?;
    let work_lease_id = id_from_last_insert::<WorkLeaseId>(transaction)?;
    transaction.execute("UPDATE work_items SET lifecycle_state = 2, last_transition_command_id = ?1 WHERE work_item_id = ?2", params![command_row_id, work_item_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![TicketState::Claimed as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::WorkItemClaimed {
        work_item_id,
        work_lease_id,
        actor_instance_id,
    })
}

fn reserve_attempt_budget(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    cycle: CycleRow,
    cycle_id: OperatingCycleId,
    amount: UsdMicros,
) -> Result<BudgetReservationId, Rejection> {
    // M3 reserves only the governing society and Operating Cycle envelopes.
    // It is a provisional execution-boundary reservation, not a claim that
    // every future pinned execution accounting dimension has been modeled.
    if amount == UsdMicros::ZERO {
        return Err(Rejection::BudgetCeilingExceeded);
    }
    let (society_budget, cycle_budget) =
        budget_envelopes_for_cycle(transaction, cycle.society_id, cycle_id)?;
    for budget_id in [society_budget, cycle_budget] {
        let (ceiling, reserved, spent) = budget_amounts(transaction, budget_id)?;
        let Some(next_reserved) = reserved.checked_add(amount) else {
            return Err(Rejection::BudgetCeilingExceeded);
        };
        if next_reserved
            .checked_add(spent)
            .is_none_or(|value| value > ceiling)
        {
            return Err(Rejection::BudgetCeilingExceeded);
        }
    }
    transaction.execute("INSERT INTO budget_reservations(operating_cycle_id, amount_micros, reservation_state, reserved_by_command_id, reconciled_by_command_id) VALUES (?1, ?2, 1, ?3, NULL)", params![cycle_id.value(), amount.value(), command_row_id]).map_err(|_| Rejection::BudgetCeilingExceeded)?;
    let reservation_id = id_from_last_insert::<BudgetReservationId>(transaction)?;
    for budget_id in [society_budget, cycle_budget] {
        transaction.execute("UPDATE budget_envelopes SET reserved_micros = reserved_micros + ?1 WHERE budget_envelope_id = ?2", params![amount.value(), budget_id.value()]).map_err(|_| Rejection::BudgetCeilingExceeded)?;
        transaction.execute("INSERT INTO budget_reservation_charges(budget_reservation_id, budget_envelope_id, amount_micros) VALUES (?1, ?2, ?3)", params![reservation_id.value(), budget_id.value(), amount.value()]).map_err(|_| Rejection::BudgetCeilingExceeded)?;
    }
    Ok(reservation_id)
}

fn start_actor_attempt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    work_item_id: WorkItemId,
    reservation_amount: UsdMicros,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (
        ticket_id,
        actor_instance_id,
        context_pack_id,
        _,
        _,
        work_state,
        retry_of_actor_attempt_id,
    ) = work_item_row(transaction, work_item_id)?;
    let (_, configuration_revision_id, execution_profile_id, actor_cycle, actor_state) =
        actor_instance_row(transaction, actor_instance_id)?;
    let (project_id, ticket_state) = ticket_row(transaction, ticket_id)?;
    project_is_active(transaction, project_id)?;
    if work_state != WorkItemState::Claimed
        || ticket_state != TicketState::Claimed
        || actor_cycle != operating_cycle_id
        || actor_state != ActorInstanceState::Active
    {
        return Err(Rejection::WorkLeaseUnavailable);
    }
    let lease_id: i64 = transaction.query_row("SELECT work_lease_id FROM leases WHERE work_item_id = ?1 AND actor_instance_id = ?2 AND lifecycle_state = 1", params![work_item_id.value(), actor_instance_id.value()], |row| row.get(0)).optional().map_err(|_| Rejection::WorkLeaseUnavailable)?.ok_or(Rejection::WorkLeaseUnavailable)?;
    let work_lease_id =
        WorkLeaseId::try_from(lease_id).map_err(|_| Rejection::WorkLeaseUnavailable)?;
    let reservation_id = reserve_attempt_budget(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        reservation_amount,
    )?;
    transaction.execute("INSERT INTO attempts(operating_cycle_id, ticket_id, work_item_id, work_lease_id, actor_instance_id, actor_configuration_revision_id, execution_profile_id, context_pack_id, retry_of_actor_attempt_id, lifecycle_state, started_by_command_id, terminal_by_command_id, validated_by_command_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, NULL, NULL)", params![operating_cycle_id.value(), ticket_id.value(), work_item_id.value(), work_lease_id.value(), actor_instance_id.value(), configuration_revision_id.value(), execution_profile_id.value(), context_pack_id.value(), retry_of_actor_attempt_id.map(ActorAttemptId::value), command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let actor_attempt_id = id_from_last_insert::<ActorAttemptId>(transaction)?;
    transaction.execute("INSERT INTO attempt_budget_reservations(actor_attempt_id, budget_reservation_id, project_id, ticket_id) VALUES (?1, ?2, ?3, ?4)", params![actor_attempt_id.value(), reservation_id.value(), project_id.value(), ticket_id.value()]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction.execute("UPDATE work_items SET lifecycle_state = 3, last_transition_command_id = ?1 WHERE work_item_id = ?2", params![command_row_id, work_item_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ActorAttemptStarted {
        actor_attempt_id,
        work_item_id,
        budget_reservation_id: reservation_id,
    })
}

fn actor_attempt_row(
    transaction: &Transaction<'_>,
    actor_attempt_id: ActorAttemptId,
) -> Result<
    (
        OperatingCycleId,
        TicketId,
        WorkItemId,
        WorkLeaseId,
        ActorInstanceId,
        ActorAttemptState,
    ),
    Rejection,
> {
    let row: (i64, i64, i64, i64, i64, i64) = transaction.query_row("SELECT operating_cycle_id, ticket_id, work_item_id, work_lease_id, actor_instance_id, lifecycle_state FROM attempts WHERE actor_attempt_id = ?1", [actor_attempt_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        OperatingCycleId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        TicketId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        WorkItemId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        WorkLeaseId::try_from(row.3).map_err(|_| Rejection::SubjectNotFound)?,
        ActorInstanceId::try_from(row.4).map_err(|_| Rejection::SubjectNotFound)?,
        actor_attempt_state_from_i64(row.5).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn attest_actor_attempt_terminal(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    actor_attempt_id: ActorAttemptId,
    terminal_kind: ActorAttemptTerminalKind,
) -> Result<EventBody, Rejection> {
    // M3's receipt-free fixture attestation is deliberately confined to the
    // provider-free no-child seam. Once a native child has been admitted,
    // semantic Attempt settlement remains unavailable until later normalized
    // Pi/submission/validation receipts; physical M5 finalization never
    // invents a model outcome.
    let has_supervised_child: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM native_child_spawn_admissions WHERE actor_attempt_id = ?1)",
            [actor_attempt_id.value()],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?
        != 0;
    if has_supervised_child {
        return Err(Rejection::SupervisedTerminalReceiptRequired);
    }
    let (operating_cycle_id, ticket_id, work_item_id, work_lease_id, _, state) =
        actor_attempt_row(transaction, actor_attempt_id)?;
    if !terminal_kind.allowed_from(state) {
        return Err(Rejection::ActorAttemptNotTerminal);
    }
    transaction.execute("INSERT INTO actor_attempt_terminal_facts(actor_attempt_id, terminal_kind, attested_by_command_id) VALUES (?1, ?2, ?3)", params![actor_attempt_id.value(), terminal_kind as i64, command_row_id]).map_err(|_| Rejection::ActorAttemptNotTerminal)?;
    transaction.execute("UPDATE attempts SET lifecycle_state = ?1, terminal_by_command_id = ?2 WHERE actor_attempt_id = ?3", params![terminal_kind.state() as i64, command_row_id, actor_attempt_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    let lease_state = if terminal_kind == ActorAttemptTerminalKind::Cancelled {
        WorkLeaseState::Cancelled
    } else {
        WorkLeaseState::Released
    };
    transaction.execute("UPDATE leases SET lifecycle_state = ?1, terminal_by_command_id = ?2 WHERE work_lease_id = ?3 AND lifecycle_state = 1", params![lease_state as i64, command_row_id, work_lease_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE work_items SET lifecycle_state = 4, last_transition_command_id = ?1 WHERE work_item_id = ?2", params![command_row_id, work_item_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    let ticket_state = match terminal_kind {
        ActorAttemptTerminalKind::Succeeded => TicketState::Submitted,
        ActorAttemptTerminalKind::Cancelled => TicketState::Cancelled,
        ActorAttemptTerminalKind::Expired => TicketState::Ready,
        ActorAttemptTerminalKind::Failed
        | ActorAttemptTerminalKind::ProtocolFailed
        | ActorAttemptTerminalKind::SupervisorFailed => TicketState::Failed,
    };
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![ticket_state as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    let (project_id, _) = ticket_row(transaction, ticket_id)?;
    let cycle = cycle_row(transaction, operating_cycle_id)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ActorAttemptTerminalAttested {
        actor_attempt_id,
        terminal_kind,
    })
}

fn validate_ticket_attempt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    actor_attempt_id: ActorAttemptId,
) -> Result<EventBody, Rejection> {
    // This M3 kernel-service command is a receipt-free atomic fixture
    // attestation: it records that this exact Ticket acceptance condition was
    // satisfied. It is not VS evidence validation; a later evidence receipt
    // must refine this boundary rather than letting the Root Authority
    // self-attest acceptance.
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (attempt_cycle, ticket_id, _, _, _, state) =
        actor_attempt_row(transaction, actor_attempt_id)?;
    let (project_id, ticket_state) = ticket_row(transaction, ticket_id)?;
    if attempt_cycle != operating_cycle_id
        || state != ActorAttemptState::Succeeded
        || ticket_state != TicketState::Submitted
    {
        return Err(Rejection::ActorAttemptNotValidatable);
    }
    let evidence_pending: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM deterministic_experiments WHERE ticket_id = ?1 AND lifecycle_state = 1",
        [ticket_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    if evidence_pending != 0 {
        return Err(Rejection::EvidenceAdmissionRequired);
    }
    transaction.execute("UPDATE attempts SET lifecycle_state = 9, validated_by_command_id = ?1 WHERE actor_attempt_id = ?2", params![command_row_id, actor_attempt_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE ticket_acceptance_conditions SET lifecycle_state = 2, satisfied_by_command_id = ?1 WHERE ticket_id = ?2 AND lifecycle_state = 1", params![command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![TicketState::Verified as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::TicketAttemptValidated {
        actor_attempt_id,
        ticket_id,
    })
}

fn retry_actor_attempt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    actor_attempt_id: ActorAttemptId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (attempt_cycle, ticket_id, work_item_id, _, _, state) =
        actor_attempt_row(transaction, actor_attempt_id)?;
    let (project_id, _) = ticket_row(transaction, ticket_id)?;
    project_is_active(transaction, project_id)?;
    if attempt_cycle != operating_cycle_id
        || !matches!(
            state,
            ActorAttemptState::Failed
                | ActorAttemptState::Cancelled
                | ActorAttemptState::Expired
                | ActorAttemptState::ProtocolFailed
                | ActorAttemptState::SupervisorFailed
        )
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let (_, _, _, _, _, work_state, _) = work_item_row(transaction, work_item_id)?;
    if work_state != WorkItemState::Settled {
        return Err(Rejection::WorkLeaseUnavailable);
    }
    transaction.execute("UPDATE work_items SET lifecycle_state = 1, retry_of_actor_attempt_id = ?1, last_transition_command_id = ?2 WHERE work_item_id = ?3", params![actor_attempt_id.value(), command_row_id, work_item_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![TicketState::Ready as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ActorAttemptRetryPrepared {
        actor_attempt_id,
        work_item_id,
        ticket_id,
    })
}

fn complete_ticket(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    actor_attempt_id: ActorAttemptId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (attempt_cycle, ticket_id, _, _, _, state) =
        actor_attempt_row(transaction, actor_attempt_id)?;
    let (project_id, ticket_state) = ticket_row(transaction, ticket_id)?;
    if attempt_cycle != operating_cycle_id
        || state != ActorAttemptState::Validated
        || ticket_state != TicketState::Verified
    {
        return Err(Rejection::ActorAttemptNotValidatable);
    }
    let unsatisfied_condition_count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM ticket_acceptance_conditions
         WHERE ticket_id = ?1 AND lifecycle_state = 1",
            [ticket_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    if unsatisfied_condition_count != 0 {
        return Err(Rejection::TicketAcceptanceConditionUnsatisfied);
    }
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![TicketState::Completed as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::TicketCompleted {
        ticket_id,
        actor_attempt_id,
    })
}

fn expire_work_lease(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    work_lease_id: WorkLeaseId,
) -> Result<EventBody, Rejection> {
    let row: (i64, i64, i64, i64) = transaction.query_row("SELECT l.work_item_id, l.lifecycle_state, w.ticket_id, a.operating_cycle_id FROM leases l JOIN work_items w ON w.work_item_id = l.work_item_id JOIN actor_instances a ON a.actor_instance_id = l.actor_instance_id WHERE l.work_lease_id = ?1", [work_lease_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    if row.1 != WorkLeaseState::Active as i64 {
        return Err(Rejection::WorkLeaseUnavailable);
    }
    let attempt_count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM attempts WHERE work_lease_id = ?1",
            [work_lease_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    if attempt_count != 0 {
        return Err(Rejection::WorkLeaseUnavailable);
    }
    let work_item_id = WorkItemId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?;
    let ticket_id = TicketId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?;
    let operating_cycle_id =
        OperatingCycleId::try_from(row.3).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE leases SET lifecycle_state = 3, terminal_by_command_id = ?1 WHERE work_lease_id = ?2", params![command_row_id, work_lease_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE work_items SET lifecycle_state = 1, last_transition_command_id = ?1 WHERE work_item_id = ?2", params![command_row_id, work_item_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![TicketState::Ready as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    let (project_id, _) = ticket_row(transaction, ticket_id)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle_row(transaction, operating_cycle_id)?,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::WorkLeaseExpired {
        work_lease_id,
        work_item_id,
    })
}

fn cancel_actor_attempt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    actor_attempt_id: ActorAttemptId,
    reason: ActorAttemptCancellationReason,
) -> Result<EventBody, Rejection> {
    let (operating_cycle_id, ticket_id, _, _, _, state) =
        actor_attempt_row(transaction, actor_attempt_id)?;
    if state != ActorAttemptState::Running {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction
        .execute(
            "UPDATE attempts SET lifecycle_state = 2 WHERE actor_attempt_id = ?1",
            [actor_attempt_id.value()],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let (project_id, _) = ticket_row(transaction, ticket_id)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle_row(transaction, operating_cycle_id)?,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ActorAttemptCancellationRequested {
        actor_attempt_id,
        reason,
    })
}

fn register_outcome_obligation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    obligation: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    transaction.execute("INSERT INTO outcome_obligations(project_id, obligation_text, lifecycle_state, scheduled_by_command_id, resolved_by_command_id) VALUES (?1, ?2, 1, ?3, NULL)", params![project_id.value(), obligation, command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let outcome_obligation_id = id_from_last_insert::<OutcomeObligationId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::OutcomeObligationRegistered {
        outcome_obligation_id,
        project_id,
    })
}

fn resolve_outcome_obligation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    outcome_obligation_id: OutcomeObligationId,
    disposition: OutcomeObligationDisposition,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let row: (i64, i64) = transaction.query_row("SELECT project_id, lifecycle_state FROM outcome_obligations WHERE outcome_obligation_id = ?1", [outcome_obligation_id.value()], |row| Ok((row.get(0)?, row.get(1)?))).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    if row.1 != OutcomeObligationState::Scheduled as i64 {
        return Err(Rejection::OutcomeObligationOpen);
    }
    let project_id = ProjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?;
    let state = disposition.state();
    transaction.execute("UPDATE outcome_obligations SET lifecycle_state = ?1, resolved_by_command_id = ?2 WHERE outcome_obligation_id = ?3", params![state as i64, command_row_id, outcome_obligation_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::OutcomeObligationResolved {
        outcome_obligation_id,
        state,
    })
}

/// Stores a narrow receipt from the later `society-content` boundary. The
/// kernel has no byte stream here and therefore cannot honestly call this a
/// physical seal operation. This is byte identity only: a later forensic
/// manifest records each specific production/capture occurrence.
fn record_content_seal_receipt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    digest: Blake3Digest,
) -> Result<EventBody, Rejection> {
    let duplicate: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM content_seal_receipts WHERE digest = ?1",
            [digest.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    if duplicate != 0 {
        return Err(Rejection::ContentObjectNotSealed);
    }
    transaction
        .execute(
            "INSERT INTO content_seal_receipts(digest, attested_by_command_id)
             VALUES (?1, ?2)",
            params![digest.as_bytes().as_slice(), command_row_id],
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    let content_seal_receipt_id = id_from_last_insert::<ContentSealReceiptId>(transaction)?;
    Ok(EventBody::ContentSealReceiptRecorded {
        content_seal_receipt_id,
        digest,
    })
}

/// Resolves the one globally registered object which attests the exact bytes
/// named by a founding mission rendering. The digest is never a substitute for
/// this physical-seal admission: both the receipt and its one `ContentObject`
/// must exist, and their joined digest must agree exactly.
fn mission_source_content_object_id(
    transaction: &Transaction<'_>,
    digest: Blake3Digest,
) -> Result<ContentObjectId, Rejection> {
    let object = transaction
        .query_row(
            "SELECT object.content_object_id
               FROM content_seal_receipts AS receipt
               JOIN content_objects AS object
                 ON object.content_seal_receipt_id = receipt.content_seal_receipt_id
              WHERE receipt.digest = ?1",
            [digest.as_bytes().as_slice()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::MissionSourceContentNotSealed)?
        .ok_or(Rejection::MissionSourceContentNotSealed)?;
    ContentObjectId::try_from(object).map_err(|_| Rejection::MissionSourceContentNotSealed)
}

fn register_content_object(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    content_seal_receipt_id: ContentSealReceiptId,
) -> Result<EventBody, Rejection> {
    let present: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM content_seal_receipts WHERE content_seal_receipt_id = ?1)",
            [content_seal_receipt_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ContentSealReceiptMissing)?;
    if !present {
        return Err(Rejection::ContentSealReceiptMissing);
    }
    let registered: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM content_objects WHERE content_seal_receipt_id = ?1)",
            [content_seal_receipt_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    if registered {
        return Err(Rejection::ContentObjectNotSealed);
    }
    transaction
        .execute(
            "INSERT INTO content_objects(content_seal_receipt_id, registered_by_command_id)
             VALUES (?1, ?2)",
            params![content_seal_receipt_id.value(), command_row_id],
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    let content_object_id = id_from_last_insert::<ContentObjectId>(transaction)?;
    Ok(EventBody::ContentObjectRegistered {
        content_object_id,
        content_seal_receipt_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn register_forensic_manifest(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    producing_deterministic_experiment_id: DeterministicExperimentId,
    capture_policy: ForensicManifestCapturePolicy,
    retention_access_class: RetentionAccessClass,
    evaluator_output_content_object_id: ContentObjectId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let experiment: Option<(i64, i64)> = transaction.query_row(
        "SELECT project_id, operating_cycle_id FROM deterministic_experiments WHERE deterministic_experiment_id = ?1 AND lifecycle_state = 1",
        [producing_deterministic_experiment_id.value()], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().map_err(|_| Rejection::ForensicManifestBindingMismatch)?;
    let (project, experiment_cycle) =
        experiment.ok_or(Rejection::ForensicManifestBindingMismatch)?;
    if experiment_cycle != operating_cycle_id.value() {
        return Err(Rejection::ForensicManifestBindingMismatch);
    }
    let scheduler_admission_exists: bool = transaction
        .query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM native_child_spawn_admissions
                  WHERE deterministic_experiment_id = ?1
             )",
            [producing_deterministic_experiment_id.value()],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| Rejection::ForensicManifestBindingMismatch)?
        != 0;
    if scheduler_admission_exists {
        // Once the resident scheduler has claimed an experiment, its output
        // may enter a manifest only through the derived stdout-seal command
        // below. The generic occurrence command cannot recombine that child
        // with another sealed object.
        return Err(Rejection::ForensicManifestBindingMismatch);
    }
    let project_id = ProjectId::try_from(project).map_err(|_| Rejection::SubjectNotFound)?;
    let object_exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM content_objects WHERE content_object_id = ?1)",
            [evaluator_output_content_object_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    if !object_exists {
        return Err(Rejection::ForensicManifestBindingMismatch);
    }
    transaction
        .execute(
            "INSERT INTO forensic_manifests(producing_deterministic_experiment_id, capture_policy, retention_access_class, registered_by_command_id)
             VALUES (?1, ?2, ?3, ?4)",
            params![producing_deterministic_experiment_id.value(), capture_policy as i64, retention_access_class as i64, command_row_id],
        )
        .map_err(|_| Rejection::ForensicManifestBindingMismatch)?;
    let forensic_manifest_id = id_from_last_insert::<ForensicManifestId>(transaction)?;
    transaction
        .execute(
            "INSERT INTO forensic_manifest_objects(forensic_manifest_id, member_ordinal, object_role, media_schema_contract, content_object_id)
             VALUES (?1, 1, 1, ?2, ?3)",
            params![
                forensic_manifest_id.value(),
                ContentMediaSchemaContract::DeterministicEvaluatorOutputV1 as i64,
                evaluator_output_content_object_id.value()
            ],
        )
        .map_err(|_| Rejection::ForensicManifestBindingMismatch)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ForensicManifestRegistered {
        forensic_manifest_id,
        producing_deterministic_experiment_id,
        evaluator_output_content_object_id,
    })
}

fn register_deterministic_evaluator_forensic_manifest(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    // The accepted child must have reached the full physical terminal: both
    // output streams are complete and it has been finalized after direct reap.
    // We derive only stdout as the evaluator result; stderr remains a required
    // custody receipt, never a silent optional diagnostic channel.
    let row: Option<(i64, i64, i64, i64, i64, i64, i64)> = transaction
        .query_row(
            "SELECT experiment.project_id,
                    experiment.deterministic_experiment_id,
                    admission.evaluator_revision_id,
                    admission.input_manifest_id,
                    child.native_child_id,
                    stdout.native_child_stream_seal_id,
                    stdout.retained_content_object_id
               FROM native_child_spawn_admissions admission
               JOIN deterministic_experiments experiment
                 ON experiment.deterministic_experiment_id = admission.deterministic_experiment_id
                AND experiment.operating_cycle_id = admission.operating_cycle_id
                AND experiment.evaluator_revision_id = admission.evaluator_revision_id
                AND experiment.input_manifest_id = admission.input_manifest_id
               JOIN native_children child
                 ON child.native_child_spawn_admission_id = admission.native_child_spawn_admission_id
               JOIN native_child_stream_seals stdout
                 ON stdout.native_child_id = child.native_child_id
                AND stdout.stream_kind = ?1
                AND stdout.completeness = ?2
               JOIN native_child_stream_seals stderr
                 ON stderr.native_child_id = child.native_child_id
                AND stderr.stream_kind = ?3
                AND stderr.completeness = ?2
              WHERE admission.native_child_spawn_admission_id = ?4
                AND admission.operating_cycle_id = ?5
                AND admission.actor_attempt_id IS NULL
                AND admission.root_authority_office_session_id IS NULL
                AND admission.deterministic_experiment_id IS NOT NULL
                AND admission.evaluator_revision_id IS NOT NULL
                AND admission.input_manifest_id IS NOT NULL
                AND admission.budget_reservation_id IS NULL
                AND admission.lifecycle_state = ?6
                AND experiment.lifecycle_state = ?7
                AND child.lifecycle_state = ?8
                AND NOT EXISTS(
                    SELECT 1 FROM pi_child_spawn_sidecars sidecar
                     WHERE sidecar.native_child_spawn_admission_id = admission.native_child_spawn_admission_id
                )",
            params![
                ChildStreamKind::Stdout as i64,
                ChildStreamSealCompleteness::Complete as i64,
                ChildStreamKind::Stderr as i64,
                native_child_spawn_admission_id.value(),
                operating_cycle_id.value(),
                NativeChildSpawnAdmissionState::Spawned as i64,
                DeterministicExperimentState::Registered as i64,
                ChildProcessState::Finalized as i64,
            ],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .optional()
        .map_err(|_| Rejection::ForensicManifestBindingMismatch)?;
    let Some((project, experiment, evaluator, input, child, stdout_seal, output)) = row else {
        return Err(Rejection::ForensicManifestBindingMismatch);
    };
    transaction
        .execute(
            "INSERT INTO forensic_manifests(
             producing_deterministic_experiment_id, capture_policy,
             retention_access_class, registered_by_command_id
         ) VALUES (?1, ?2, ?3, ?4)",
            params![
                experiment,
                ForensicManifestCapturePolicy::DeterministicExperimentEvaluatorV1 as i64,
                RetentionAccessClass::ForensicRestricted as i64,
                command_row_id,
            ],
        )
        .map_err(|_| Rejection::ForensicManifestBindingMismatch)?;
    let forensic_manifest_id = id_from_last_insert::<ForensicManifestId>(transaction)?;
    transaction
        .execute(
            "INSERT INTO forensic_manifest_objects(
             forensic_manifest_id, member_ordinal, object_role,
             media_schema_contract, content_object_id
         ) VALUES (?1, 1, 1, ?2, ?3)",
            params![
                forensic_manifest_id.value(),
                ContentMediaSchemaContract::DeterministicEvaluatorOutputV1 as i64,
                output,
            ],
        )
        .map_err(|_| Rejection::ForensicManifestBindingMismatch)?;
    transaction
        .execute(
            "INSERT INTO deterministic_evaluator_forensic_manifest_bindings(
             forensic_manifest_id, deterministic_experiment_id,
             native_child_spawn_admission_id, native_child_id,
             evaluator_revision_id, input_manifest_id,
             native_child_stream_seal_id, evaluator_output_content_object_id,
             registered_by_command_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                forensic_manifest_id.value(),
                experiment,
                native_child_spawn_admission_id.value(),
                child,
                evaluator,
                input,
                stdout_seal,
                output,
                command_row_id,
            ],
        )
        .map_err(|_| Rejection::ForensicManifestBindingMismatch)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(ProjectId::try_from(project).map_err(|_| Rejection::SubjectNotFound)?),
    )?;
    Ok(
        EventBody::DeterministicEvaluatorForensicManifestRegistered {
            forensic_manifest_id,
            deterministic_experiment_id: DeterministicExperimentId::try_from(experiment)
                .map_err(|_| Rejection::ForensicManifestBindingMismatch)?,
            native_child_spawn_admission_id,
            native_child_stream_seal_id: NativeChildStreamSealId::try_from(stdout_seal)
                .map_err(|_| Rejection::ForensicManifestBindingMismatch)?,
            evaluator_output_content_object_id: ContentObjectId::try_from(output)
                .map_err(|_| Rejection::ForensicManifestBindingMismatch)?,
        },
    )
}

fn content_object_exists(
    transaction: &Transaction<'_>,
    content_object_id: ContentObjectId,
) -> Result<bool, Rejection> {
    transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM content_objects WHERE content_object_id = ?1)",
            [content_object_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)
}

fn evaluator_revision_for_content(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    content_object_id: ContentObjectId,
) -> Result<EvaluatorRevisionId, Rejection> {
    let existing: Option<i64> = transaction
        .query_row(
            "SELECT evaluator_revision_id FROM evaluator_revisions WHERE content_object_id = ?1",
            [content_object_id.value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    if let Some(existing) = existing {
        return EvaluatorRevisionId::try_from(existing).map_err(|_| Rejection::SubjectNotFound);
    }
    transaction
        .execute(
            "INSERT INTO evaluator_revisions(content_object_id, media_schema_contract, registered_by_command_id) VALUES (?1, ?2, ?3)",
            params![
                content_object_id.value(),
                ContentMediaSchemaContract::DeterministicEvaluatorV1 as i64,
                command_row_id
            ],
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    id_from_last_insert::<EvaluatorRevisionId>(transaction)
}

fn input_manifest_for_content(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    content_object_id: ContentObjectId,
) -> Result<InputManifestId, Rejection> {
    let existing: Option<i64> = transaction
        .query_row(
            "SELECT input_manifest_id FROM input_manifests WHERE content_object_id = ?1",
            [content_object_id.value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    if let Some(existing) = existing {
        return InputManifestId::try_from(existing).map_err(|_| Rejection::SubjectNotFound);
    }
    transaction
        .execute(
            "INSERT INTO input_manifests(content_object_id, media_schema_contract, registered_by_command_id) VALUES (?1, ?2, ?3)",
            params![
                content_object_id.value(),
                ContentMediaSchemaContract::DeterministicInputManifestV1 as i64,
                command_row_id
            ],
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    id_from_last_insert::<InputManifestId>(transaction)
}

#[allow(clippy::too_many_arguments)]
fn register_deterministic_experiment(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    ticket_id: TicketId,
    target_graph_revision_id: GraphRevisionId,
    evaluator_content_object_id: ContentObjectId,
    input_manifest_content_object_id: ContentObjectId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    let (_, revision_project_id, _, revision_state) =
        graph_revision_row(transaction, target_graph_revision_id)?;
    if ticket_row(transaction, ticket_id)?.0 != project_id
        || revision_project_id != project_id
        || revision_state != GraphRevisionState::Committed
        || !content_object_exists(transaction, evaluator_content_object_id)?
        || !content_object_exists(transaction, input_manifest_content_object_id)?
    {
        return Err(Rejection::DeterministicExperimentBindingMismatch);
    }
    let evaluator_revision_id =
        evaluator_revision_for_content(transaction, command_row_id, evaluator_content_object_id)?;
    let input_manifest_id = input_manifest_for_content(
        transaction,
        command_row_id,
        input_manifest_content_object_id,
    )?;
    transaction
        .execute(
            "INSERT INTO deterministic_experiments(operating_cycle_id, project_id, ticket_id, target_graph_revision_id, evaluator_revision_id, input_manifest_id, lifecycle_state, registered_by_command_id, last_transition_command_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![operating_cycle_id.value(), project_id.value(), ticket_id.value(), target_graph_revision_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), DeterministicExperimentState::Registered as i64, command_row_id],
        )
        .map_err(|_| Rejection::DeterministicExperimentBindingMismatch)?;
    let deterministic_experiment_id =
        id_from_last_insert::<DeterministicExperimentId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::DeterministicExperimentRegistered {
        deterministic_experiment_id,
        evaluator_revision_id,
        input_manifest_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn record_deterministic_evaluation_receipt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    deterministic_experiment_id: DeterministicExperimentId,
    evaluator_revision_id: EvaluatorRevisionId,
    input_manifest_id: InputManifestId,
    forensic_manifest_id: ForensicManifestId,
    evaluator_output_content_object_id: ContentObjectId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let row: Option<(i64, i64, i64, i64)> = transaction.query_row(
        "SELECT project_id, ticket_id, evaluator_revision_id, input_manifest_id
         FROM deterministic_experiments WHERE deterministic_experiment_id = ?1 AND operating_cycle_id = ?2 AND lifecycle_state = 1",
        params![deterministic_experiment_id.value(), operating_cycle_id.value()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).optional().map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    let (project, _ticket, evaluator, input) =
        row.ok_or(Rejection::DeterministicEvaluationBindingMismatch)?;
    if evaluator != evaluator_revision_id.value() || input != input_manifest_id.value() {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    let manifest_experiment: Option<i64> = transaction.query_row(
        "SELECT producing_deterministic_experiment_id FROM forensic_manifests WHERE forensic_manifest_id = ?1",
        [forensic_manifest_id.value()], |row| row.get(0),
    ).optional().map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    if manifest_experiment != Some(deterministic_experiment_id.value()) {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    let output_in_manifest: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM forensic_manifest_objects WHERE forensic_manifest_id = ?1 AND content_object_id = ?2 AND object_role = 1 AND media_schema_contract = 3)",
        params![forensic_manifest_id.value(), evaluator_output_content_object_id.value()],
        |row| row.get(0),
    ).map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    if !output_in_manifest {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    // A generic forensic occurrence predating scheduling remains auditable,
    // but it cannot become the result of a claimed evaluator. Once an exact
    // evaluator admission exists, its receipt must name the manifest/output
    // occurrence derived from that admission's finalized stdout custody.
    let scheduler_claimed: bool = transaction
        .query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM native_child_spawn_admissions
                  WHERE deterministic_experiment_id = ?1
             )",
            [deterministic_experiment_id.value()],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?
        != 0;
    if scheduler_claimed {
        let exact_derived_occurrence: bool = transaction
            .query_row(
                "SELECT EXISTS(
                     SELECT 1
                       FROM deterministic_evaluator_forensic_manifest_bindings binding
                       JOIN native_child_spawn_admissions admission
                         ON admission.native_child_spawn_admission_id = binding.native_child_spawn_admission_id
                        AND admission.deterministic_experiment_id = binding.deterministic_experiment_id
                        AND admission.evaluator_revision_id = binding.evaluator_revision_id
                        AND admission.input_manifest_id = binding.input_manifest_id
                       JOIN native_children child
                         ON child.native_child_id = binding.native_child_id
                        AND child.native_child_spawn_admission_id = binding.native_child_spawn_admission_id
                        AND child.lifecycle_state = ?6
                       JOIN native_child_stream_seals stdout
                         ON stdout.native_child_stream_seal_id = binding.native_child_stream_seal_id
                        AND stdout.native_child_id = binding.native_child_id
                        AND stdout.stream_kind = ?7
                        AND stdout.completeness = ?8
                        AND stdout.retained_content_object_id = binding.evaluator_output_content_object_id
                       JOIN native_child_stream_seals stderr
                         ON stderr.native_child_id = binding.native_child_id
                        AND stderr.stream_kind = ?9
                        AND stderr.completeness = ?8
                      WHERE binding.forensic_manifest_id = ?1
                        AND binding.deterministic_experiment_id = ?2
                        AND binding.evaluator_revision_id = ?3
                        AND binding.input_manifest_id = ?4
                        AND binding.evaluator_output_content_object_id = ?5
                 )",
                params![
                    forensic_manifest_id.value(),
                    deterministic_experiment_id.value(),
                    evaluator_revision_id.value(),
                    input_manifest_id.value(),
                    evaluator_output_content_object_id.value(),
                    ChildProcessState::Finalized as i64,
                    ChildStreamKind::Stdout as i64,
                    ChildStreamSealCompleteness::Complete as i64,
                    ChildStreamKind::Stderr as i64,
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?
            != 0;
        if !exact_derived_occurrence {
            return Err(Rejection::DeterministicEvaluationBindingMismatch);
        }
    }
    transaction.execute(
        "INSERT INTO deterministic_evaluation_receipts(deterministic_experiment_id, evaluator_revision_id, input_manifest_id, forensic_manifest_id, evaluator_output_content_object_id, attested_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![deterministic_experiment_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), forensic_manifest_id.value(), evaluator_output_content_object_id.value(), command_row_id],
    ).map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    let deterministic_evaluation_receipt_id =
        id_from_last_insert::<DeterministicEvaluationReceiptId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(ProjectId::try_from(project).map_err(|_| Rejection::SubjectNotFound)?),
    )?;
    Ok(EventBody::DeterministicEvaluationReceiptRecorded {
        deterministic_evaluation_receipt_id,
        deterministic_experiment_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn admit_deterministic_evidence(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId,
    deterministic_experiment_id: DeterministicExperimentId,
    evaluator_revision_id: EvaluatorRevisionId,
    input_manifest_id: InputManifestId,
    evaluator_output_content_object_id: ContentObjectId,
    related_graph_revision_id: GraphRevisionId,
    semantic_role: EvidenceSemanticRole,
    applicability: crate::EvidenceApplicability,
    limitation: &EvidenceLimitationText,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let row: Option<(i64, i64, i64, i64, i64)> = transaction.query_row(
        "SELECT e.project_id, e.target_graph_revision_id, r.deterministic_experiment_id, r.evaluator_revision_id, r.input_manifest_id
         FROM deterministic_experiments e JOIN deterministic_evaluation_receipts r ON r.deterministic_experiment_id = e.deterministic_experiment_id
         WHERE r.deterministic_evaluation_receipt_id = ?1 AND e.operating_cycle_id = ?2 AND e.lifecycle_state = 1",
        params![deterministic_evaluation_receipt_id.value(), operating_cycle_id.value()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).optional().map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    let (project, target, experiment, evaluator, input) =
        row.ok_or(Rejection::DeterministicEvaluationBindingMismatch)?;
    if experiment != deterministic_experiment_id.value()
        || evaluator != evaluator_revision_id.value()
        || input != input_manifest_id.value()
        || target != related_graph_revision_id.value()
    {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    let (_, graph_project, graph_kind, graph_state) =
        graph_revision_row(transaction, related_graph_revision_id)?;
    if graph_project.value() != project
        || graph_kind != GraphObjectKind::Hypothesis
        || graph_state != GraphRevisionState::Committed
        || semantic_role != EvidenceSemanticRole::DeterministicObservation
        || applicability != crate::EvidenceApplicability::TestsTargetHypothesis
    {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    let receipt_output: i64 = transaction.query_row("SELECT evaluator_output_content_object_id FROM deterministic_evaluation_receipts WHERE deterministic_evaluation_receipt_id = ?1", [deterministic_evaluation_receipt_id.value()], |row| row.get(0)).map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    if receipt_output != evaluator_output_content_object_id.value() {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    transaction.execute(
        "INSERT INTO evidence_admissions(deterministic_evaluation_receipt_id, deterministic_experiment_id, evaluator_revision_id, input_manifest_id, evaluator_output_content_object_id, related_graph_revision_id, semantic_role, applicability, limitation_text, admitted_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![deterministic_evaluation_receipt_id.value(), deterministic_experiment_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), evaluator_output_content_object_id.value(), related_graph_revision_id.value(), semantic_role as i64, applicability as i64, limitation.as_str(), command_row_id],
    ).map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    let evidence_admission_id = id_from_last_insert::<EvidenceAdmissionId>(transaction)?;
    transaction.execute("UPDATE deterministic_experiments SET lifecycle_state = 2, last_transition_command_id = ?1 WHERE deterministic_experiment_id = ?2", params![command_row_id, deterministic_experiment_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(ProjectId::try_from(project).map_err(|_| Rejection::SubjectNotFound)?),
    )?;
    Ok(EventBody::DeterministicEvidenceAdmitted {
        evidence_admission_id,
        deterministic_evaluation_receipt_id,
        semantic_role,
        applicability,
    })
}

fn finalize_deterministic_experiment(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    deterministic_experiment_id: DeterministicExperimentId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, operating_cycle_id, expected_generation)?;
    let row: Option<(i64, i64)> = transaction.query_row("SELECT project_id, lifecycle_state FROM deterministic_experiments WHERE deterministic_experiment_id = ?1 AND operating_cycle_id = ?2", params![deterministic_experiment_id.value(), operating_cycle_id.value()], |row| Ok((row.get(0)?, row.get(1)?))).optional().map_err(|_| Rejection::SubjectNotFound)?;
    let (project, state) = row.ok_or(Rejection::SubjectNotFound)?;
    let terminal_state = if state == DeterministicExperimentState::EvidenceAdmitted as i64 {
        DeterministicExperimentState::Closed
    } else if state == DeterministicExperimentState::Registered as i64 {
        let finalized: bool = transaction.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM native_children child
                 JOIN native_child_spawn_admissions admission
                   ON admission.native_child_spawn_admission_id = child.native_child_spawn_admission_id
                 WHERE admission.deterministic_experiment_id = ?1
                   AND child.lifecycle_state = ?2
             )",
            params![deterministic_experiment_id.value(), ChildProcessState::Finalized as i64],
            |row| row.get::<_, i64>(0),
        ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)? != 0;
        let invalidated: bool = transaction.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM native_child_spawn_admissions admission
                 JOIN native_child_spawn_invalidations invalidation
                   ON invalidation.native_child_spawn_admission_id = admission.native_child_spawn_admission_id
                 WHERE admission.deterministic_experiment_id = ?1
                   AND admission.lifecycle_state = ?2
             )",
            params![
                deterministic_experiment_id.value(),
                NativeChildSpawnAdmissionState::Invalidated as i64,
            ],
            |row| row.get::<_, i64>(0),
        ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)? != 0;
        if !finalized && !invalidated {
            return Err(Rejection::EvidenceAdmissionRequired);
        }
        let cancelled: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM cancellation_propagation_targets
              WHERE deterministic_experiment_id = ?1 AND target_disposition IN (4, 5))
                 OR EXISTS(
                     SELECT 1 FROM native_child_spawn_admissions admission
                     JOIN native_child_spawn_invalidations invalidation
                       ON invalidation.native_child_spawn_admission_id = admission.native_child_spawn_admission_id
                     WHERE admission.deterministic_experiment_id = ?1
                       AND invalidation.reason = 1
                 )",
                [deterministic_experiment_id.value()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?
            != 0;
        if cancelled {
            DeterministicExperimentState::Cancelled
        } else {
            DeterministicExperimentState::Failed
        }
    } else {
        return Err(Rejection::EvidenceAdmissionRequired);
    };
    let terminal_cycle_allowed = match terminal_state {
        DeterministicExperimentState::Closed => cycle.state == OperatingCycleState::Running,
        DeterministicExperimentState::Failed => matches!(
            cycle.state,
            OperatingCycleState::Running
                | OperatingCycleState::Cancelling
                | OperatingCycleState::Reaping
        ),
        DeterministicExperimentState::Cancelled => {
            matches!(
                cycle.state,
                OperatingCycleState::Cancelling | OperatingCycleState::Reaping
            )
        }
        DeterministicExperimentState::Registered
        | DeterministicExperimentState::EvidenceAdmitted => false,
    };
    if !terminal_cycle_allowed {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("UPDATE deterministic_experiments SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE deterministic_experiment_id = ?3", params![terminal_state as i64, command_row_id, deterministic_experiment_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(ProjectId::try_from(project).map_err(|_| Rejection::SubjectNotFound)?),
    )?;
    Ok(EventBody::DeterministicExperimentFinalized {
        deterministic_experiment_id,
        terminal_state,
    })
}

#[allow(clippy::too_many_arguments)] // mirrors the closed command body exactly at the trusted boundary.
fn admit_deterministic_evaluator_native_child(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    deterministic_experiment_id: DeterministicExperimentId,
    evaluator_revision_id: EvaluatorRevisionId,
    input_manifest_id: InputManifestId,
    execution_profile_id: ExecutionProfileId,
    native_workspace_id: &NativeWorkspaceId,
    canonical_workspace_path: &CanonicalWorkspacePath,
    supervisor_epoch_id: SupervisorEpochId,
    supervisor_epoch_identity: &SupervisorEpochIdentity,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, operating_cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Running
        || cycle._treatment != OperatingCycleTreatment::DeterministicEvaluatorFixtureV1
    {
        return Err(Rejection::ExecutionProfileIneligible);
    }
    let binding: Option<(i64, i64, i64)> = transaction.query_row(
        "SELECT evaluator_revision_id, input_manifest_id, lifecycle_state FROM deterministic_experiments
         WHERE deterministic_experiment_id = ?1 AND operating_cycle_id = ?2",
        params![deterministic_experiment_id.value(), operating_cycle_id.value()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().map_err(|_| Rejection::DeterministicExperimentBindingMismatch)?;
    if binding
        != Some((
            evaluator_revision_id.value(),
            input_manifest_id.value(),
            DeterministicExperimentState::Registered as i64,
        ))
    {
        return Err(Rejection::DeterministicExperimentBindingMismatch);
    }
    let profile: Option<(i64, i64)> = transaction.query_row(
        "SELECT profile_kind, readiness FROM execution_profiles WHERE execution_profile_id = ?1",
        [execution_profile_id.value()], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().map_err(|_| Rejection::ExecutionProfileIneligible)?;
    if profile
        != Some((
            ExecutionProfileKind::DeterministicEvaluatorProcessFixtureV1 as i64,
            ExecutionProfileReadiness::DeterministicFixtureOnly as i64,
        ))
    {
        return Err(Rejection::ExecutionProfileIneligible);
    }
    let epoch_matches: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM supervisor_epochs WHERE supervisor_epoch_id = ?1 AND supervisor_epoch_identity = ?2)",
        params![supervisor_epoch_id.value(), supervisor_epoch_identity.as_str()], |row| row.get::<_, i64>(0),
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)? != 0;
    if !epoch_matches {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    transaction.execute(
        "INSERT OR IGNORE INTO workspaces(native_workspace_id, canonical_workspace_path, registered_by_command_id) VALUES (?1, ?2, ?3)",
        params![native_workspace_id.as_str(), canonical_workspace_path.as_str(), command_row_id],
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    let workspace_id: i64 = transaction.query_row(
        "SELECT workspace_id FROM workspaces WHERE native_workspace_id = ?1 AND canonical_workspace_path = ?2",
        params![native_workspace_id.as_str(), canonical_workspace_path.as_str()], |row| row.get(0),
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    transaction.execute(
        "INSERT INTO native_child_spawn_admissions(operating_cycle_id, actor_attempt_id, root_authority_office_session_id, deterministic_experiment_id, evaluator_revision_id, input_manifest_id, budget_reservation_id, execution_profile_id, workspace_id, supervisor_epoch_id, admission_generation, lifecycle_state, admitted_by_command_id, spawned_by_command_id)
         VALUES (?1, NULL, NULL, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, 1, ?9, NULL)",
        params![operating_cycle_id.value(), deterministic_experiment_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), execution_profile_id.value(), workspace_id, supervisor_epoch_id.value(), cycle.generation.value(), command_row_id],
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    let admission_id = id_from_last_insert::<NativeChildSpawnAdmissionId>(transaction)?;
    Ok(EventBody::DeterministicEvaluatorNativeChildAdmitted {
        native_child_spawn_admission_id: admission_id,
        owner: NativeChildOwner::DeterministicEvaluator {
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
        },
    })
}

fn record_deterministic_evaluator_native_child_spawn(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    _expected_generation: ExpectedGeneration,
    admission_id: NativeChildSpawnAdmissionId,
    child_identity: &SupervisedChildIdentity,
    direct_child_pid: NativeChildPid,
    process_group_id: OwnedProcessGroupId,
) -> Result<EventBody, Rejection> {
    // Spawn is a post-exec custody receipt. As with an inert Pi child, it
    // remains recordable after a generation advance so a raced child cannot
    // be orphaned; later evaluator work remains generation-fenced.
    if direct_child_pid.value() != process_group_id.value() {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    let row: Option<DeterministicEvaluatorSpawnAdmissionSqlRow> = transaction
        .query_row(
            "SELECT admission.operating_cycle_id, admission.lifecycle_state,
                admission.deterministic_experiment_id, admission.evaluator_revision_id,
                admission.input_manifest_id, admission.budget_reservation_id,
                sidecar.pi_session_id
           FROM native_child_spawn_admissions admission
      LEFT JOIN pi_child_spawn_sidecars sidecar
             ON sidecar.native_child_spawn_admission_id = admission.native_child_spawn_admission_id
          WHERE admission.native_child_spawn_admission_id = ?1",
            [admission_id.value()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .optional()
        .map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    let Some((cycle, state, Some(experiment), Some(evaluator), Some(input), None, None)) = row
    else {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    };
    let _ = cycle_row(
        transaction,
        OperatingCycleId::try_from(cycle).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?,
    )?;
    if state != NativeChildSpawnAdmissionState::Admitted as i64 {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let binding_matches: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM deterministic_experiments
          WHERE deterministic_experiment_id = ?1 AND operating_cycle_id = ?2
            AND evaluator_revision_id = ?3 AND input_manifest_id = ?4 AND lifecycle_state = 1)",
            params![experiment, cycle, evaluator, input],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?
        != 0;
    if !binding_matches {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    transaction.execute(
        "INSERT INTO native_children(native_child_spawn_admission_id, child_identity, direct_child_pid, process_group_id, lifecycle_state, terminal_disposition, spawned_by_command_id, last_transition_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?6)",
        params![admission_id.value(), child_identity.as_str(), direct_child_pid.value(), process_group_id.value(), ChildProcessState::Spawned as i64, command_row_id],
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    let native_child_id = id_from_last_insert::<NativeChildId>(transaction)?;
    transaction.execute(
        "UPDATE native_child_spawn_admissions SET lifecycle_state = 2, spawned_by_command_id = ?1 WHERE native_child_spawn_admission_id = ?2",
        params![command_row_id, admission_id.value()],
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    // A cancellation snapshot may precede the physical spawn. Attach this
    // raced evaluator child to the exact frozen experiment target rather than
    // pretending it belonged to an Actor or Office.
    transaction.execute(
        "UPDATE cancellation_propagation_targets
            SET native_child_id = ?1, target_disposition = ?2
          WHERE native_child_id IS NULL
            AND deterministic_experiment_id = (SELECT deterministic_experiment_id FROM native_child_spawn_admissions WHERE native_child_spawn_admission_id = ?3)
            AND cancellation_propagation_id IN (SELECT cancellation_propagation_id FROM cancellation_propagations WHERE lifecycle_state = 1)",
        params![native_child_id.value(), CancellationPropagationTargetDisposition::AwaitingChildReceipt as i64, admission_id.value()],
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    transaction.execute(
        "INSERT OR IGNORE INTO cancellation_propagation_children(cancellation_propagation_id, native_child_id)
         SELECT cancellation_propagation_id, native_child_id FROM cancellation_propagation_targets
          WHERE native_child_id = ?1 AND target_disposition = ?2",
        params![native_child_id.value(), CancellationPropagationTargetDisposition::AwaitingChildReceipt as i64],
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    Ok(EventBody::DeterministicEvaluatorNativeChildSpawnRecorded {
        native_child_id,
        native_child_spawn_admission_id: admission_id,
    })
}

// M5 deliberately persists the native-child bridge in small transitions.  A
// database admission is not a spawn receipt; adapter readiness is not Create
// authorization; and an absence after a supervisor restart is not wait(2).
fn open_supervisor_epoch(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    supervisor_epoch_id: SupervisorEpochId,
    supervisor_epoch_identity: &SupervisorEpochIdentity,
) -> Result<EventBody, Rejection> {
    let epoch_already_open: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM supervisor_epochs)",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?
        != 0;
    if epoch_already_open {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    transaction.execute(
        "INSERT INTO supervisor_epochs(supervisor_epoch_id, supervisor_epoch_identity, opened_by_command_id)
         VALUES (?1, ?2, ?3)",
        params![
            supervisor_epoch_id.value(),
            supervisor_epoch_identity.as_str(),
            command_row_id,
        ],
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    Ok(EventBody::SupervisorEpochOpened {
        supervisor_epoch_id,
    })
}

fn admit_pi_child_spawn(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    input: PiChildSpawnAdmissionInput<'_>,
) -> Result<EventBody, Rejection> {
    let PiChildSpawnAdmissionInput {
        operating_cycle_id,
        owner,
        budget_reservation_id,
        execution_profile_id,
        native_workspace_id,
        canonical_workspace_path,
        supervisor_epoch_id,
        supervisor_epoch_identity,
        pi_session_identity,
        spawn_nonce,
    } = input;
    let cycle = cycle_for_generation(transaction, operating_cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Running {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let reservation: Option<(i64, i64)> = transaction.query_row(
        "SELECT operating_cycle_id, reservation_state FROM budget_reservations WHERE budget_reservation_id = ?1",
        [budget_reservation_id.value()], |r| Ok((r.get(0)?, r.get(1)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?;
    let Some((reservation_cycle, reservation_state)) = reservation else {
        return Err(Rejection::ReservationNotActive);
    };
    if reservation_cycle != operating_cycle_id.value()
        || reservation_state != BudgetReservationState::Reserved as i64
    {
        return Err(Rejection::ReservationNotActive);
    }
    let (attempt, office_session) = match owner {
        PiChildOwner::ActorAttempt(actor_attempt_id) => {
            let row: Option<(i64, i64, i64)> = transaction.query_row(
                "SELECT a.operating_cycle_id, a.execution_profile_id, r.budget_reservation_id
                   FROM attempts a JOIN attempt_budget_reservations r ON r.actor_attempt_id = a.actor_attempt_id
                  WHERE a.actor_attempt_id = ?1 AND a.lifecycle_state = ?2",
                params![actor_attempt_id.value(), ActorAttemptState::Running as i64],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ).optional().map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
            let Some((attempt_cycle, profile, reserved)) = row else {
                return Err(Rejection::ChildSpawnAdmissionInvalid);
            };
            if attempt_cycle != operating_cycle_id.value()
                || profile != execution_profile_id.value()
                || reserved != budget_reservation_id.value()
            {
                return Err(Rejection::ChildSpawnAdmissionInvalid);
            }
            (Some(actor_attempt_id), None)
        }
        PiChildOwner::RootAuthorityOfficeSession(session_id) => {
            let row: Option<i64> = transaction
                .query_row(
                    "SELECT operating_cycle_id FROM root_authority_office_sessions
                  WHERE root_authority_office_session_id = ?1 AND lifecycle_state IN (1, 2, 3, 4)",
                    [session_id.value()],
                    |r| r.get(0),
                )
                .optional()
                .map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
            if row != Some(operating_cycle_id.value()) {
                return Err(Rejection::ChildSpawnAdmissionInvalid);
            }
            let existing: Option<i64> = transaction.query_row(
                "SELECT budget_reservation_id FROM office_session_budget_reservations WHERE root_authority_office_session_id = ?1",
                [session_id.value()], |r| r.get(0),
            ).optional().map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
            match existing {
                Some(bound) if bound == budget_reservation_id.value() => {}
                Some(_) => return Err(Rejection::ChildSpawnAdmissionInvalid),
                None => {
                    transaction.execute(
                        "INSERT INTO office_session_budget_reservations(root_authority_office_session_id, budget_reservation_id, bound_by_command_id) VALUES (?1, ?2, ?3)",
                        params![session_id.value(), budget_reservation_id.value(), command_row_id],
                    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
                }
            }
            (None, Some(session_id))
        }
    };
    let profile: Option<(i64, i64)> = transaction.query_row(
        "SELECT profile_kind, readiness FROM execution_profiles WHERE execution_profile_id = ?1",
        [execution_profile_id.value()], |r| Ok((r.get(0)?, r.get(1)?)),
    ).optional().map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    let profile_is_allowed = profile.is_some_and(|(kind, readiness)| {
        matches!(
            (execution_profile_kind_from_i64(kind), execution_profile_readiness_from_i64(readiness)),
            (Ok(kind), Ok(readiness)) if pi_child_profile_allowed(cycle._treatment, kind, readiness)
        )
    });
    // Admission and Create authorization use the same closed treatment matrix.
    // The latter rechecks it so an out-of-band profile mutation cannot turn a
    // formerly lawful pre-spawn receipt into a work authorization.
    if !profile_is_allowed {
        return Err(Rejection::ExecutionProfileIneligible);
    }
    let epoch_matches: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM supervisor_epochs WHERE supervisor_epoch_id = ?1 AND supervisor_epoch_identity = ?2)",
        params![supervisor_epoch_id.value(), supervisor_epoch_identity.as_str()],
        |row| row.get::<_, i64>(0),
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)? != 0;
    if !epoch_matches {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    transaction.execute("INSERT OR IGNORE INTO workspaces(native_workspace_id, canonical_workspace_path, registered_by_command_id) VALUES (?1, ?2, ?3)", params![native_workspace_id.as_str(), canonical_workspace_path.as_str(), command_row_id]).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    let workspace_id: i64 = transaction
        .query_row(
            "SELECT workspace_id FROM workspaces WHERE native_workspace_id = ?1 AND canonical_workspace_path = ?2",
            params![native_workspace_id.as_str(), canonical_workspace_path.as_str()],
            |r| r.get(0),
        )
        .map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    transaction.execute("INSERT INTO pi_child_sessions(pi_session_identity, spawn_nonce, created_by_command_id, ready_by_command_id) VALUES (?1, ?2, ?3, NULL)", params![pi_session_identity.as_str(), spawn_nonce.as_str(), command_row_id]).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    let pi_session_id = id_from_last_insert::<PiSessionId>(transaction)?;
    transaction.execute(
        "INSERT INTO native_child_spawn_admissions(operating_cycle_id, actor_attempt_id, root_authority_office_session_id, deterministic_experiment_id, evaluator_revision_id, input_manifest_id, budget_reservation_id, execution_profile_id, workspace_id, supervisor_epoch_id, admission_generation, lifecycle_state, admitted_by_command_id, spawned_by_command_id)
         VALUES (?1, ?2, ?3, NULL, NULL, NULL, ?4, ?5, ?6, ?7, ?8, 1, ?9, NULL)",
        params![operating_cycle_id.value(), attempt.map(ActorAttemptId::value), office_session.map(RootAuthorityOfficeSessionId::value), budget_reservation_id.value(), execution_profile_id.value(), workspace_id, supervisor_epoch_id.value(), cycle.generation.value(), command_row_id],
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    let admission_id = id_from_last_insert::<NativeChildSpawnAdmissionId>(transaction)?;
    transaction.execute(
        "INSERT INTO pi_child_spawn_sidecars(native_child_spawn_admission_id, pi_session_id) VALUES (?1, ?2)",
        params![admission_id.value(), pi_session_id.value()],
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    Ok(EventBody::PiChildSpawnAdmitted {
        native_child_spawn_admission_id: admission_id,
        owner,
        budget_reservation_id,
    })
}

fn admission_cycle_for_generation(
    transaction: &Transaction<'_>,
    admission_id: NativeChildSpawnAdmissionId,
    _expected: ExpectedGeneration,
) -> Result<(OperatingCycleId, NativeChildSpawnAdmissionState), Rejection> {
    let row: Option<(i64, i64)> = transaction.query_row("SELECT operating_cycle_id, lifecycle_state FROM native_child_spawn_admissions WHERE native_child_spawn_admission_id = ?1", [admission_id.value()], |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(|_| Rejection::SubjectNotFound)?;
    let Some((cycle, state)) = row else {
        return Err(Rejection::SubjectNotFound);
    };
    let cycle_id = OperatingCycleId::try_from(cycle).map_err(|_| Rejection::SubjectNotFound)?;
    // Spawn/AdapterReady are recovery receipts. They stay recordable after a
    // quiesce changes generation so an already-raced inert child is never
    // orphaned. Create authorization performs the strict generation fence.
    let _ = cycle_row(transaction, cycle_id)?;
    let state = match state {
        1 => NativeChildSpawnAdmissionState::Admitted,
        2 => NativeChildSpawnAdmissionState::Spawned,
        3 => NativeChildSpawnAdmissionState::Invalidated,
        _ => return Err(Rejection::SubjectNotFound),
    };
    Ok((cycle_id, state))
}

fn native_child_cycle_for_generation(
    transaction: &Transaction<'_>,
    native_child_id: NativeChildId,
    _expected: ExpectedGeneration,
) -> Result<(OperatingCycleId, ChildProcessState), Rejection> {
    let row: Option<(i64, i64)> = transaction
        .query_row(
            "SELECT admission.operating_cycle_id, child.lifecycle_state
         FROM native_children child
         JOIN native_child_spawn_admissions admission
           ON admission.native_child_spawn_admission_id = child.native_child_spawn_admission_id
         WHERE child.native_child_id = ?1",
            [native_child_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?;
    let Some((cycle, state)) = row else {
        return Err(Rejection::SubjectNotFound);
    };
    let cycle_id = OperatingCycleId::try_from(cycle).map_err(|_| Rejection::SubjectNotFound)?;
    let _ = cycle_row(transaction, cycle_id)?;
    Ok((
        cycle_id,
        child_process_state_from_i64(state).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn child_cycle_for_generation(
    transaction: &Transaction<'_>,
    native_child_id: NativeChildId,
    _expected: ExpectedGeneration,
) -> Result<(OperatingCycleId, ChildProcessState, PiSessionId), Rejection> {
    let row: Option<(i64, i64, i64)> = transaction
        .query_row(
            "SELECT a.operating_cycle_id, c.lifecycle_state, sidecar.pi_session_id
         FROM native_children c
         JOIN native_child_spawn_admissions a
           ON a.native_child_spawn_admission_id = c.native_child_spawn_admission_id
         JOIN pi_child_spawn_sidecars sidecar
           ON sidecar.native_child_spawn_admission_id = a.native_child_spawn_admission_id
         WHERE c.native_child_id = ?1",
            [native_child_id.value()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?;
    let Some((cycle, state, pi_session)) = row else {
        return Err(Rejection::SubjectNotFound);
    };
    let cycle_id = OperatingCycleId::try_from(cycle).map_err(|_| Rejection::SubjectNotFound)?;
    let _ = cycle_row(transaction, cycle_id)?;
    let state = child_process_state_from_i64(state).map_err(|_| Rejection::SubjectNotFound)?;
    Ok((
        cycle_id,
        state,
        PiSessionId::try_from(pi_session).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn record_inert_child_spawn(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    admission_id: NativeChildSpawnAdmissionId,
    child_identity: &SupervisedChildIdentity,
    direct_child_pid: NativeChildPid,
    process_group_id: OwnedProcessGroupId,
) -> Result<EventBody, Rejection> {
    let (_, state) = admission_cycle_for_generation(transaction, admission_id, expected)?;
    if state != NativeChildSpawnAdmissionState::Admitted {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let pi_owner: bool = transaction.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM native_child_spawn_admissions admission
             JOIN pi_child_spawn_sidecars sidecar
               ON sidecar.native_child_spawn_admission_id = admission.native_child_spawn_admission_id
             WHERE admission.native_child_spawn_admission_id = ?1
               AND admission.deterministic_experiment_id IS NULL
               AND admission.budget_reservation_id IS NOT NULL
               AND ((admission.actor_attempt_id IS NOT NULL AND admission.root_authority_office_session_id IS NULL)
                 OR (admission.actor_attempt_id IS NULL AND admission.root_authority_office_session_id IS NOT NULL))
         )",
        [admission_id.value()], |row| row.get::<_, i64>(0),
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)? != 0;
    if !pi_owner {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    if direct_child_pid.value() != process_group_id.value() {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    let identity_in_use: bool = transaction
        .query_row(
            "SELECT EXISTS(
           SELECT 1 FROM native_children
            WHERE lifecycle_state != ?1
              AND (direct_child_pid = ?2 OR process_group_id = ?3))",
            params![
                ChildProcessState::Finalized as i64,
                direct_child_pid.value(),
                process_group_id.value(),
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?
        != 0;
    if identity_in_use {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    transaction.execute("INSERT INTO native_children(native_child_spawn_admission_id, child_identity, direct_child_pid, process_group_id, lifecycle_state, terminal_disposition, spawned_by_command_id, last_transition_command_id) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?6)", params![admission_id.value(), child_identity.as_str(), direct_child_pid.value(), process_group_id.value(), ChildProcessState::Spawned as i64, command_row_id]).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    let native_child_id = id_from_last_insert::<NativeChildId>(transaction)?;
    transaction.execute("INSERT INTO pi_child_session_protocols(native_child_id, lifecycle_state, create_correlation_identity, create_request_digest) VALUES (?1, 1, NULL, NULL)", [native_child_id.value()]).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    transaction.execute("UPDATE native_child_spawn_admissions SET lifecycle_state = 2, spawned_by_command_id = ?1 WHERE native_child_spawn_admission_id = ?2", params![command_row_id, admission_id.value()]).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    // A cancellation snapshot may have observed this owner before the OS
    // child raced into existence. Attach that raced child to the already
    // frozen owner target rather than omitting it from propagation.
    transaction.execute(
        "UPDATE cancellation_propagation_targets
            SET native_child_id = ?1, target_disposition = ?2
          WHERE native_child_id IS NULL
            AND cancellation_propagation_id IN (
                SELECT p.cancellation_propagation_id
                  FROM cancellation_propagations p
                  JOIN native_child_spawn_admissions a
                    ON a.native_child_spawn_admission_id = ?3
                 WHERE p.operating_cycle_id = a.operating_cycle_id
                   AND p.lifecycle_state = 1)
            AND (actor_attempt_id = (SELECT actor_attempt_id FROM native_child_spawn_admissions WHERE native_child_spawn_admission_id = ?3)
              OR root_authority_office_session_id = (SELECT root_authority_office_session_id FROM native_child_spawn_admissions WHERE native_child_spawn_admission_id = ?3))",
        params![
            native_child_id.value(),
            CancellationPropagationTargetDisposition::AwaitingChildReceipt as i64,
            admission_id.value(),
        ],
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    transaction.execute(
        "INSERT OR IGNORE INTO cancellation_propagation_children(cancellation_propagation_id, native_child_id)
         SELECT cancellation_propagation_id, ?1
           FROM cancellation_propagation_targets
          WHERE native_child_id = ?1
            AND target_disposition = ?2",
        params![
            native_child_id.value(),
            CancellationPropagationTargetDisposition::AwaitingChildReceipt as i64,
        ],
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    Ok(EventBody::InertPiChildSpawnRecorded {
        native_child_id,
        native_child_spawn_admission_id: admission_id,
    })
}

fn record_native_child_not_spawned(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    admission_id: NativeChildSpawnAdmissionId,
    reason: NativeChildNotSpawnedReason,
) -> Result<EventBody, Rejection> {
    let native_owner: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM native_child_spawn_admissions admission
          WHERE admission.native_child_spawn_admission_id = ?1
            AND ((admission.deterministic_experiment_id IS NULL
                  AND admission.evaluator_revision_id IS NULL
                  AND admission.input_manifest_id IS NULL
                  AND admission.budget_reservation_id IS NOT NULL
                  AND EXISTS(SELECT 1 FROM pi_child_spawn_sidecars sidecar
                               WHERE sidecar.native_child_spawn_admission_id = admission.native_child_spawn_admission_id))
              OR (admission.deterministic_experiment_id IS NOT NULL
                  AND admission.evaluator_revision_id IS NOT NULL
                  AND admission.input_manifest_id IS NOT NULL
                  AND admission.budget_reservation_id IS NULL
                  AND NOT EXISTS(SELECT 1 FROM pi_child_spawn_sidecars sidecar
                                  WHERE sidecar.native_child_spawn_admission_id = admission.native_child_spawn_admission_id))))",
        [admission_id.value()], |row| row.get::<_, i64>(0),
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)? != 0;
    if !native_owner {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    let (cycle_id, state) = admission_cycle_for_generation(transaction, admission_id, expected)?;
    let _ = cycle_for_generation(transaction, cycle_id, expected)?;
    if state != NativeChildSpawnAdmissionState::Admitted {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    // A pre-spawn failure is meaningful even before cancellation. If an
    // active cancellation snapshot did name this owner, resolve exactly that
    // one outstanding target; more than one is an invariant breach. A zero
    // count is the ordinary native-workspace/artifact/spawn failure path.
    let targets = transaction.execute(
        "UPDATE cancellation_propagation_targets
            SET target_disposition = ?1
          WHERE cancellation_propagation_id IN (
                SELECT cancellation_propagation_id FROM cancellation_propagations
                 WHERE operating_cycle_id = ?2 AND lifecycle_state = 1)
            AND native_child_id IS NULL
            AND target_disposition = ?3
            AND (actor_attempt_id = (SELECT actor_attempt_id FROM native_child_spawn_admissions WHERE native_child_spawn_admission_id = ?4)
              OR root_authority_office_session_id = (SELECT root_authority_office_session_id FROM native_child_spawn_admissions WHERE native_child_spawn_admission_id = ?4)
              OR deterministic_experiment_id = (SELECT deterministic_experiment_id FROM native_child_spawn_admissions WHERE native_child_spawn_admission_id = ?4))",
        params![
            CancellationPropagationTargetDisposition::NotRunning as i64,
            cycle_id.value(),
            CancellationPropagationTargetDisposition::AwaitingChildReceipt as i64,
            admission_id.value(),
        ],
    ).map_err(|_| Rejection::CancellationPropagationIncomplete)?;
    if targets > 1 {
        return Err(Rejection::CancellationPropagationIncomplete);
    }
    if reason == NativeChildNotSpawnedReason::CancelledBeforeSpawn && targets != 1 {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction
        .execute(
            "UPDATE native_child_spawn_admissions
            SET lifecycle_state = ?1
          WHERE native_child_spawn_admission_id = ?2",
            params![
                NativeChildSpawnAdmissionState::Invalidated as i64,
                admission_id.value()
            ],
        )
        .map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    transaction.execute(
        "INSERT INTO native_child_spawn_invalidations(native_child_spawn_admission_id, reason, invalidated_by_command_id)
         VALUES (?1, ?2, ?3)",
        params![admission_id.value(), reason as i64, command_row_id],
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    Ok(EventBody::NativeChildSpawnInvalidated {
        native_child_spawn_admission_id: admission_id,
        reason,
    })
}

fn record_pi_adapter_ready(
    transaction: &Transaction<'_>,
    _command_row_id: i64,
    expected: ExpectedGeneration,
    child_id: NativeChildId,
    identity: &PiBoundarySessionIdentity,
    nonce: &SpawnNonce,
) -> Result<EventBody, Rejection> {
    let (_, state, pi_session_id) = child_cycle_for_generation(transaction, child_id, expected)?;
    if !matches!(
        state,
        ChildProcessState::Spawned | ChildProcessState::CancellationRequested
    ) || pi_protocol_state(transaction, child_id)? != PiChildSessionState::InertSpawned
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let matches: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM pi_child_sessions WHERE pi_session_id = ?1 AND pi_session_identity = ?2 AND spawn_nonce = ?3)", params![pi_session_id.value(), identity.as_str(), nonce.as_str()], |r| r.get::<_, i64>(0)).map_err(|_| Rejection::ChildLifecycleReceiptMissing)? != 0;
    if !matches {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    set_pi_protocol_state(transaction, child_id, PiChildSessionState::AdapterReady)?;
    Ok(EventBody::PiAdapterReadyRecorded {
        native_child_id: child_id,
        pi_session_id,
    })
}

fn authorize_pi_create_session(
    transaction: &Transaction<'_>,
    _command_row_id: i64,
    expected: ExpectedGeneration,
    child_id: NativeChildId,
    correlation: &PiCorrelationIdentity,
    create_request_digest: Blake3Digest,
) -> Result<EventBody, Rejection> {
    let (cycle_id, state, _) = child_cycle_for_generation(transaction, child_id, expected)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected)?;
    let admission: (i64, i64, i64, Option<i64>, Option<i64>, i64, i64) = transaction.query_row(
        "SELECT a.admission_generation, a.budget_reservation_id, a.execution_profile_id, a.actor_attempt_id, a.root_authority_office_session_id, p.profile_kind, p.readiness
           FROM native_children c JOIN native_child_spawn_admissions a ON a.native_child_spawn_admission_id = c.native_child_spawn_admission_id
           JOIN execution_profiles p ON p.execution_profile_id = a.execution_profile_id
          WHERE c.native_child_id = ?1",
        [child_id.value()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?)),
    ).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)?;
    if state != ChildProcessState::Spawned
        || pi_protocol_state(transaction, child_id)? != PiChildSessionState::AdapterReady
        || cycle.state != OperatingCycleState::Running
        || admission.0 != cycle.generation.value()
    {
        return Err(Rejection::StaleAdmissionGeneration);
    }
    let reservation_active: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM budget_reservations WHERE budget_reservation_id = ?1 AND operating_cycle_id = ?2 AND reservation_state = ?3)", params![admission.1, cycle_id.value(), BudgetReservationState::Reserved as i64], |r| r.get::<_,i64>(0)).map_err(|_| Rejection::ReservationNotActive)? != 0;
    if !reservation_active || active_cancellation_count(transaction, cycle_id)? != 0 {
        return Err(Rejection::ReservationNotActive);
    }
    let owner_active = match (admission.3, admission.4) {
        (Some(attempt), None) => transaction.query_row("SELECT EXISTS(SELECT 1 FROM attempts WHERE actor_attempt_id = ?1 AND lifecycle_state = 1)", [attempt], |r| r.get::<_,i64>(0)).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)? != 0,
        (None, Some(session)) => transaction.query_row("SELECT EXISTS(SELECT 1 FROM root_authority_office_sessions s JOIN office_session_budget_reservations b ON b.root_authority_office_session_id = s.root_authority_office_session_id WHERE s.root_authority_office_session_id = ?1 AND b.budget_reservation_id = ?2 AND s.lifecycle_state IN (1,2,3,4))", params![session, admission.1], |r| r.get::<_,i64>(0)).map_err(|_| Rejection::ChildSpawnAdmissionInvalid)? != 0,
        _ => false,
    };
    if !owner_active {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    let profile_allowed = match (
        execution_profile_kind_from_i64(admission.5),
        execution_profile_readiness_from_i64(admission.6),
    ) {
        (Ok(kind), Ok(readiness)) => pi_child_profile_allowed(cycle._treatment, kind, readiness),
        _ => false,
    };
    if !profile_allowed {
        return Err(Rejection::ExecutionProfileIneligible);
    }
    transaction.execute("UPDATE pi_child_session_protocols SET lifecycle_state = ?1, create_correlation_identity = ?2, create_request_digest = ?3 WHERE native_child_id = ?4", params![PiChildSessionState::CreateAuthorized as i64, correlation.as_str(), create_request_digest.as_bytes().as_slice(), child_id.value()]).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    Ok(EventBody::PiCreateSessionAuthorized {
        native_child_id: child_id,
    })
}

fn record_pi_create_session_delivery(
    transaction: &Transaction<'_>,
    _command_row_id: i64,
    expected: ExpectedGeneration,
    child_id: NativeChildId,
    correlation: &PiCorrelationIdentity,
    create_request_digest: Blake3Digest,
) -> Result<EventBody, Rejection> {
    let (_, state, _) = child_cycle_for_generation(transaction, child_id, expected)?;
    let matches: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM pi_child_session_protocols WHERE native_child_id = ?1 AND lifecycle_state = ?2 AND create_correlation_identity = ?3 AND create_request_digest = ?4)", params![child_id.value(), PiChildSessionState::CreateAuthorized as i64, correlation.as_str(), create_request_digest.as_bytes().as_slice()], |r| r.get::<_, i64>(0)).map_err(|_| Rejection::ChildLifecycleReceiptMissing)? != 0;
    if !matches!(
        state,
        ChildProcessState::Spawned | ChildProcessState::CancellationRequested
    ) || !matches
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    set_pi_protocol_state(transaction, child_id, PiChildSessionState::CreateDelivered)?;
    Ok(EventBody::PiCreateSessionDeliveryRecorded {
        native_child_id: child_id,
    })
}

fn record_pi_session_ready(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    child_id: NativeChildId,
    identity: &PiBoundarySessionIdentity,
) -> Result<EventBody, Rejection> {
    let (_, state, pi_session_id) = child_cycle_for_generation(transaction, child_id, expected)?;
    if !matches!(
        state,
        ChildProcessState::Spawned | ChildProcessState::CancellationRequested
    ) || pi_protocol_state(transaction, child_id)? != PiChildSessionState::CreateDelivered
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let matches: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM pi_child_sessions WHERE pi_session_id = ?1 AND pi_session_identity = ?2 AND ready_by_command_id IS NULL)", params![pi_session_id.value(), identity.as_str()], |r| r.get::<_, i64>(0)).map_err(|_| Rejection::ChildLifecycleReceiptMissing)? != 0;
    if !matches {
        return Err(Rejection::ChildSpawnAdmissionInvalid);
    }
    transaction
        .execute(
            "UPDATE pi_child_sessions SET ready_by_command_id = ?1 WHERE pi_session_id = ?2",
            params![command_row_id, pi_session_id.value()],
        )
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    set_pi_protocol_state(transaction, child_id, PiChildSessionState::SessionReady)?;
    if state == ChildProcessState::Spawned {
        set_child_state(
            transaction,
            child_id,
            ChildProcessState::Running,
            command_row_id,
        )?;
    }
    Ok(EventBody::PiSessionReadyRecorded {
        native_child_id: child_id,
        pi_session_id,
    })
}

fn record_pi_abort_control_delivery(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    input: PiAbortControlDeliveryInput<'_>,
) -> Result<EventBody, Rejection> {
    let PiAbortControlDeliveryInput {
        child_id,
        propagation_id,
        correlation,
        abort_digest,
        outcome,
    } = input;
    let (cycle_id, state, _) = child_cycle_for_generation(transaction, child_id, expected)?;
    let _ = cycle_for_generation(transaction, cycle_id, expected)?;
    if !matches!(
        state,
        ChildProcessState::Running | ChildProcessState::CancellationRequested
    ) || pi_protocol_state(transaction, child_id)? != PiChildSessionState::SessionReady
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let target_exists: bool = transaction.query_row(
        "SELECT EXISTS(
           SELECT 1 FROM cancellation_propagation_targets t
           JOIN cancellation_propagations p ON p.cancellation_propagation_id = t.cancellation_propagation_id
          WHERE t.cancellation_propagation_id = ?1
            AND t.native_child_id = ?2
            AND t.target_disposition = ?3
            AND p.lifecycle_state = 1)",
        params![propagation_id.value(), child_id.value(), CancellationPropagationTargetDisposition::AwaitingChildReceipt as i64],
        |row| row.get::<_, i64>(0),
    ).map_err(|_| Rejection::CancellationPropagationIncomplete)? != 0;
    if !target_exists {
        return Err(Rejection::CancellationPropagationIncomplete);
    }
    transaction.execute(
        "INSERT INTO pi_abort_control_receipts(native_child_id, cancellation_propagation_id, correlation_identity, abort_command_digest, physical_write_outcome, recorded_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![child_id.value(), propagation_id.value(), correlation.as_str(), abort_digest.as_bytes().as_slice(), outcome as i64, command_row_id],
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    Ok(EventBody::PiAbortControlDeliveryRecorded {
        pi_abort_control_receipt_id: id_from_last_insert::<PiAbortControlReceiptId>(transaction)?,
        native_child_id: child_id,
        cancellation_propagation_id: propagation_id,
        correlation_identity: correlation.clone(),
        abort_command_digest: abort_digest,
        outcome,
    })
}

fn record_child_stream_seal(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    input: ChildStreamSealInput,
) -> Result<EventBody, Rejection> {
    let ChildStreamSealInput {
        child_id,
        stream,
        full_digest,
        retained,
        completeness,
    } = input;
    let (_, state) = native_child_cycle_for_generation(transaction, child_id, expected)?;
    if matches!(
        state,
        ChildProcessState::Finalized
            | ChildProcessState::RecoveryContainmentRequired
            | ChildProcessState::LostParentage
    ) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let evaluator_owned: bool = transaction.query_row(
        "SELECT EXISTS(
             SELECT 1 FROM native_children child
             JOIN native_child_spawn_admissions admission
               ON admission.native_child_spawn_admission_id = child.native_child_spawn_admission_id
             WHERE child.native_child_id = ?1
               AND admission.deterministic_experiment_id IS NOT NULL
               AND admission.evaluator_revision_id IS NOT NULL
               AND admission.input_manifest_id IS NOT NULL
               AND admission.budget_reservation_id IS NULL
               AND NOT EXISTS(SELECT 1 FROM pi_child_spawn_sidecars sidecar
                              WHERE sidecar.native_child_spawn_admission_id = admission.native_child_spawn_admission_id)
         )",
        [child_id.value()],
        |row| row.get::<_, i64>(0),
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)? != 0;
    if evaluator_owned
        && (state != ChildProcessState::DirectChildReaped
            || !matches!(stream, ChildStreamKind::Stdout | ChildStreamKind::Stderr))
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let retained_digest: Option<Vec<u8>> = transaction.query_row("SELECT r.digest FROM content_objects o JOIN content_seal_receipts r ON r.content_seal_receipt_id = o.content_seal_receipt_id WHERE o.content_object_id = ?1", [retained.value()], |r| r.get(0)).optional().map_err(|_| Rejection::ContentObjectNotSealed)?;
    let Some(retained_digest) = retained_digest else {
        return Err(Rejection::ContentObjectNotSealed);
    };
    if completeness == ChildStreamSealCompleteness::Complete
        && retained_digest.as_slice() != full_digest.as_bytes()
    {
        return Err(Rejection::ChildStreamSealBindingMismatch);
    }
    transaction.execute("INSERT INTO native_child_stream_seals(native_child_id, stream_kind, full_observed_digest, retained_content_object_id, completeness, sealed_by_command_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![child_id.value(), stream as i64, full_digest.as_bytes().as_slice(), retained.value(), completeness as i64, command_row_id]).map_err(|_| Rejection::ChildStreamSealBindingMismatch)?;
    let seal_id = id_from_last_insert::<NativeChildStreamSealId>(transaction)?;
    if completeness == ChildStreamSealCompleteness::CountOverflow {
        mark_child_containment_failed(transaction, child_id, command_row_id)?;
    }
    Ok(EventBody::ChildStreamSealed {
        native_child_stream_seal_id: seal_id,
        native_child_id: child_id,
        stream_kind: stream,
        completeness,
    })
}

fn record_child_process_liveness(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    child_id: NativeChildId,
    liveness: ProcessGroupLiveness,
) -> Result<EventBody, Rejection> {
    let (_, state) = native_child_cycle_for_generation(transaction, child_id, expected)?;
    if state.is_terminal() {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    if state == ChildProcessState::RecoveryContainmentRequired
        && liveness == ProcessGroupLiveness::Present
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let liveness_regressed = liveness_is_reuse_conflict(transaction, child_id, liveness)?;
    transaction.execute("INSERT INTO native_child_liveness_observations(native_child_id, liveness, observed_by_command_id) VALUES (?1, ?2, ?3)", params![child_id.value(), liveness as i64, command_row_id]).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    // Preserve the receipt identity before a containment transition performs
    // any material-row update. The event must name the physical observation,
    // never an incidental later SQLite row identity.
    let observation_id = id_from_last_insert::<NativeChildLivenessObservationId>(transaction)?;
    if liveness == ProcessGroupLiveness::Inaccessible || liveness_regressed {
        mark_child_containment_failed(transaction, child_id, command_row_id)?;
    } else if state == ChildProcessState::RecoveryContainmentRequired
        && liveness == ProcessGroupLiveness::Absent
    {
        transaction
            .execute(
                "UPDATE native_children
                SET lifecycle_state = ?1, terminal_disposition = ?2,
                    last_transition_command_id = ?3
              WHERE native_child_id = ?4 AND lifecycle_state = ?5",
                params![
                    ChildProcessState::LostParentage as i64,
                    ChildTerminalDisposition::SupervisionLost as i64,
                    command_row_id,
                    child_id.value(),
                    ChildProcessState::RecoveryContainmentRequired as i64,
                ],
            )
            .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    }
    Ok(EventBody::ChildProcessLivenessObserved {
        native_child_liveness_observation_id: observation_id,
        native_child_id: child_id,
        liveness,
    })
}

fn record_process_signal_receipt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    input: ProcessSignalReceiptInput,
) -> Result<EventBody, Rejection> {
    let ProcessSignalReceiptInput {
        child_id,
        action,
        delivery,
        liveness,
        cause,
    } = input;
    if !matches!(
        (delivery, liveness),
        (ProcessSignalDelivery::Delivered, _)
            | (
                ProcessSignalDelivery::AbsentBeforeSignal,
                ProcessGroupLiveness::Absent
            )
            | (
                ProcessSignalDelivery::AbsentDuringSignal,
                ProcessGroupLiveness::Absent
            )
            | (
                ProcessSignalDelivery::Inaccessible,
                ProcessGroupLiveness::Inaccessible
            )
    ) {
        return Err(Rejection::ChildLifecycleReceiptMissing);
    }
    let (_, state) = native_child_cycle_for_generation(transaction, child_id, expected)?;
    if state.is_terminal() {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    // Native custody is shared.  Only a Pi-sidecar child owes an Abort write
    // before TERM; a deterministic evaluator has no fabricated Pi identity.
    let protocol_state = pi_protocol_state(transaction, child_id).ok();
    let (cause_kind, propagation_id) = match cause {
        ProcessSignalCause::CancellationPropagation(propagation_id) => {
            // A cancellation signal is legal only after the durable target
            // snapshot exists and names this exact child as still owing a
            // terminal receipt. This prevents a later supervisor action from
            // being retroactively attributed to cancellation.
            let target_exists: bool = transaction
                .query_row(
                    "SELECT EXISTS(
                   SELECT 1 FROM cancellation_propagation_targets t
                   JOIN cancellation_propagations p
                     ON p.cancellation_propagation_id = t.cancellation_propagation_id
                  WHERE t.cancellation_propagation_id = ?1
                    AND t.native_child_id = ?2
                    AND t.target_disposition = ?3
                    AND p.lifecycle_state = 1)",
                    params![
                        propagation_id.value(),
                        child_id.value(),
                        CancellationPropagationTargetDisposition::AwaitingChildReceipt as i64,
                    ],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|_| Rejection::CancellationPropagationIncomplete)?
                != 0;
            if !target_exists {
                return Err(Rejection::CancellationPropagationIncomplete);
            }
            match action {
                ProcessSignalAction::Terminate => {
                    // Before SessionReady a cancellation may contain an inert
                    // child directly. Once a live Pi session exists, a TERM
                    // must follow the exact propagation's durable Pi Abort
                    // attempt, never an OS-signal lookalike. The nonblocking
                    // host may have discarded a partial/failed write; that
                    // still authorizes containment escalation, not success.
                    if protocol_state == Some(PiChildSessionState::SessionReady)
                        && !prior_pi_abort_control_attempt(transaction, child_id, propagation_id)?
                    {
                        return Err(Rejection::CancellationPropagationIncomplete);
                    }
                }
                ProcessSignalAction::Kill => {
                    if !prior_signal_attempt(
                        transaction,
                        child_id,
                        ProcessSignalAction::Terminate,
                        1,
                        Some(propagation_id),
                    )? {
                        return Err(Rejection::CancellationPropagationIncomplete);
                    }
                }
                ProcessSignalAction::LingeringGroupKill => {
                    if !lingering_group_cleanup_is_due(transaction, child_id)? {
                        return Err(Rejection::CancellationPropagationIncomplete);
                    }
                }
            }
            (1, Some(propagation_id.value()))
        }
        ProcessSignalCause::AutomaticBoundaryContainment => {
            // Automatic protocol containment is intentionally narrower than
            // cancellation propagation. The transient supervisor's emergency
            // boundary containment uses TERM then KILL (and may later use a
            // lingering-group kill). Pi Abort is a separate control receipt.
            if !matches!(
                action,
                ProcessSignalAction::Terminate
                    | ProcessSignalAction::Kill
                    | ProcessSignalAction::LingeringGroupKill
            ) {
                return Err(Rejection::CancellationPropagationIncomplete);
            }
            if action == ProcessSignalAction::Kill
                && !prior_signal_attempt(
                    transaction,
                    child_id,
                    ProcessSignalAction::Terminate,
                    2,
                    None,
                )?
            {
                return Err(Rejection::CancellationPropagationIncomplete);
            }
            if action == ProcessSignalAction::LingeringGroupKill
                && !lingering_group_cleanup_is_due(transaction, child_id)?
            {
                return Err(Rejection::CancellationPropagationIncomplete);
            }
            (2, None)
        }
    };
    let liveness_regressed = liveness_is_reuse_conflict(transaction, child_id, liveness)?;
    transaction.execute("INSERT INTO process_signal_receipts(native_child_id, signal_action, delivery, observed_liveness, cause_kind, cancellation_propagation_id, recorded_by_command_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![child_id.value(), action as i64, delivery as i64, liveness as i64, cause_kind, propagation_id, command_row_id]).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    if delivery == ProcessSignalDelivery::Delivered
        && matches!(
            state,
            ChildProcessState::Spawned | ChildProcessState::Running
        )
    {
        set_child_state(
            transaction,
            child_id,
            ChildProcessState::CancellationRequested,
            command_row_id,
        )?;
    }
    if delivery == ProcessSignalDelivery::Inaccessible
        || liveness == ProcessGroupLiveness::Inaccessible
        || liveness_regressed
    {
        mark_child_containment_failed(transaction, child_id, command_row_id)?;
    } else if state == ChildProcessState::RecoveryContainmentRequired
        && liveness == ProcessGroupLiveness::Absent
    {
        // A post-restart Absent signal observation is equivalent to the
        // liveness probe: it is the first exact evidence that the unknown
        // parentage no longer leaves a live process group to contain.
        transaction
            .execute(
                "UPDATE native_children
                    SET lifecycle_state = ?1, terminal_disposition = ?2,
                        last_transition_command_id = ?3
                  WHERE native_child_id = ?4 AND lifecycle_state = ?5",
                params![
                    ChildProcessState::LostParentage as i64,
                    ChildTerminalDisposition::SupervisionLost as i64,
                    command_row_id,
                    child_id.value(),
                    ChildProcessState::RecoveryContainmentRequired as i64,
                ],
            )
            .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    }
    Ok(EventBody::ProcessSignalReceiptRecorded {
        process_signal_receipt_id: id_from_last_insert::<ProcessSignalReceiptId>(transaction)?,
        native_child_id: child_id,
        action,
        delivery,
        observed_liveness: liveness,
        cause,
    })
}

fn prior_signal_attempt(
    transaction: &Transaction<'_>,
    child_id: NativeChildId,
    action: ProcessSignalAction,
    cause_kind: i64,
    propagation_id: Option<CancellationPropagationId>,
) -> Result<bool, Rejection> {
    transaction
        .query_row(
            "SELECT EXISTS(
           SELECT 1 FROM process_signal_receipts
            WHERE native_child_id = ?1
              AND signal_action = ?2
              AND cause_kind = ?3
              AND cancellation_propagation_id IS ?4)",
            params![
                child_id.value(),
                action as i64,
                cause_kind,
                propagation_id.map(CancellationPropagationId::value),
            ],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)
}

fn prior_pi_abort_control_attempt(
    transaction: &Transaction<'_>,
    child_id: NativeChildId,
    propagation_id: CancellationPropagationId,
) -> Result<bool, Rejection> {
    transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM pi_abort_control_receipts
                WHERE native_child_id = ?1
                  AND cancellation_propagation_id = ?2
                 )",
            params![child_id.value(), propagation_id.value(),],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)
}

/// The PID/PGID pair is one owned physical identity. Seeing it absent and
/// later seeing it present/inaccessible is a durable reuse/containment fact,
/// not an invalid caller request that may simply disappear from the ledger.
fn liveness_is_reuse_conflict(
    transaction: &Transaction<'_>,
    child_id: NativeChildId,
    liveness: ProcessGroupLiveness,
) -> Result<bool, Rejection> {
    if liveness == ProcessGroupLiveness::Absent {
        return Ok(false);
    }
    transaction.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM native_child_liveness_observations WHERE native_child_id = ?1 AND liveness = 2
            UNION ALL SELECT 1 FROM process_signal_receipts WHERE native_child_id = ?1 AND observed_liveness = 2
            UNION ALL SELECT 1 FROM native_child_reap_receipts WHERE native_child_id = ?1 AND (group_liveness_before_cleanup = 2 OR group_liveness_after_cleanup = 2)
        )",
        [child_id.value()],
        |row| row.get::<_, i64>(0),
    ).map(|value| value != 0).map_err(|_| Rejection::ChildLifecycleReceiptMissing)
}

fn lingering_group_cleanup_is_due(
    transaction: &Transaction<'_>,
    child_id: NativeChildId,
) -> Result<bool, Rejection> {
    transaction
        .query_row(
            "SELECT EXISTS(
           SELECT 1 FROM native_children c
           JOIN native_child_reap_receipts r ON r.native_child_id = c.native_child_id
          WHERE c.native_child_id = ?1
            AND c.lifecycle_state = ?2
            AND r.group_liveness_after_cleanup = ?3)",
            params![
                child_id.value(),
                ChildProcessState::DirectChildReaped as i64,
                ProcessGroupLiveness::Present as i64,
            ],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)
}

fn record_direct_child_reap(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    child_id: NativeChildId,
    status: DirectChildWaitStatus,
    before: ProcessGroupLiveness,
    after: ProcessGroupLiveness,
) -> Result<EventBody, Rejection> {
    let (_, state) = native_child_cycle_for_generation(transaction, child_id, expected)?;
    if matches!(
        state,
        ChildProcessState::Finalized
            | ChildProcessState::RecoveryContainmentRequired
            | ChildProcessState::LostParentage
            | ChildProcessState::DirectChildReaped
    ) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let liveness_regressed = liveness_is_reuse_conflict(transaction, child_id, before)?
        || liveness_is_reuse_conflict(transaction, child_id, after)?;
    let (kind, value) = match status {
        DirectChildWaitStatus::Exited { exit_code } => (1, Some(i64::from(exit_code.value()))),
        DirectChildWaitStatus::Signaled { signal_number } => {
            (2, Some(i64::from(signal_number.value())))
        }
        DirectChildWaitStatus::Unknown => (3, None),
    };
    transaction.execute("INSERT INTO native_child_reap_receipts(native_child_id, wait_status_kind, status_value, group_liveness_before_cleanup, group_liveness_after_cleanup, reaped_by_command_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![child_id.value(), kind, value, before as i64, after as i64, command_row_id]).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    // A wait(2) only proves the direct child. Descendants which remain present
    // or inaccessible after cleanup are a durable containment failure, not a
    // close-eligible finalization. We preserve the wait receipt for audit.
    if after == ProcessGroupLiveness::Inaccessible || liveness_regressed {
        mark_child_containment_failed(transaction, child_id, command_row_id)?;
    } else if state != ChildProcessState::ContainmentFailed {
        set_child_state(
            transaction,
            child_id,
            ChildProcessState::DirectChildReaped,
            command_row_id,
        )?;
    }
    Ok(EventBody::DirectChildReaped {
        native_child_reap_receipt_id: id_from_last_insert::<NativeChildReapReceiptId>(transaction)?,
        native_child_id: child_id,
        wait_status: status,
        group_liveness_before_cleanup: before,
        group_liveness_after_cleanup: after,
    })
}

fn record_child_recovery(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    child_id: NativeChildId,
    observation: ChildRecoveryObservation,
    group_liveness_after_restart: ProcessGroupLiveness,
) -> Result<EventBody, Rejection> {
    let (_, state) = native_child_cycle_for_generation(transaction, child_id, expected)?;
    // Containment failure is an immutable close blocker in M5. A later
    // restart observation cannot down-classify a known descendant/liveness
    // failure into merely lost parentage.
    if matches!(
        state,
        ChildProcessState::Finalized
            | ChildProcessState::DirectChildReaped
            | ChildProcessState::RecoveryContainmentRequired
            | ChildProcessState::LostParentage
            | ChildProcessState::ContainmentFailed
    ) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let liveness_regressed =
        liveness_is_reuse_conflict(transaction, child_id, group_liveness_after_restart)?;
    transaction.execute("INSERT INTO native_child_recovery_receipts(native_child_id, observation, group_liveness_after_restart, recorded_by_command_id) VALUES (?1, ?2, ?3, ?4)", params![child_id.value(), observation as i64, group_liveness_after_restart as i64, command_row_id]).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    match group_liveness_after_restart {
        ProcessGroupLiveness::Absent => {
            transaction
                .execute(
                    "UPDATE native_children
            SET lifecycle_state = ?1, terminal_disposition = ?2,
                last_transition_command_id = ?3
          WHERE native_child_id = ?4",
                    params![
                        ChildProcessState::LostParentage as i64,
                        ChildTerminalDisposition::SupervisionLost as i64,
                        command_row_id,
                        child_id.value(),
                    ],
                )
                .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
        }
        ProcessGroupLiveness::Inaccessible => {
            mark_child_containment_failed(transaction, child_id, command_row_id)?;
        }
        // Parentage loss plus a still-present group is deliberately not a
        // terminal recovery fact. It enters the nonterminal
        // RecoveryContainmentRequired
        // containment state: no Pi protocol/new-work or wait(2) receipt may
        // reopen it. The one-shot recovery receipt is now consumed; later
        // containment/liveness observations use their dedicated receipt.
        ProcessGroupLiveness::Present if liveness_regressed => {
            mark_child_containment_failed(transaction, child_id, command_row_id)?;
        }
        ProcessGroupLiveness::Present => {
            transaction
                .execute(
                    "UPDATE native_children
                    SET lifecycle_state = ?1, terminal_disposition = NULL,
                        last_transition_command_id = ?2
                  WHERE native_child_id = ?3",
                    params![
                        ChildProcessState::RecoveryContainmentRequired as i64,
                        command_row_id,
                        child_id.value(),
                    ],
                )
                .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
        }
    };
    Ok(EventBody::ChildRecoveryObserved {
        native_child_recovery_receipt_id: id_from_last_insert::<NativeChildRecoveryReceiptId>(
            transaction,
        )?,
        native_child_id: child_id,
        observation,
        group_liveness_after_restart,
    })
}

fn finalize_child_process(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    child_id: NativeChildId,
) -> Result<EventBody, Rejection> {
    let (_, state) = native_child_cycle_for_generation(transaction, child_id, expected)?;
    let disposition = if state == ChildProcessState::DirectChildReaped {
        let (kind, value, after, reap_command): (i64, Option<i64>, i64, i64) = transaction.query_row("SELECT wait_status_kind, status_value, group_liveness_after_cleanup, reaped_by_command_id FROM native_child_reap_receipts WHERE native_child_id = ?1", [child_id.value()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
        let group_absent_after_reap = after == ProcessGroupLiveness::Absent as i64 || transaction.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM native_child_liveness_observations
                 WHERE native_child_id = ?1 AND liveness = 2 AND observed_by_command_id > ?2
                UNION ALL
                SELECT 1 FROM process_signal_receipts
                 WHERE native_child_id = ?1 AND observed_liveness = 2 AND recorded_by_command_id > ?2
            )",
            params![child_id.value(), reap_command],
            |row| row.get::<_, i64>(0),
        ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)? != 0;
        if !group_absent_after_reap {
            return Err(Rejection::ProcessContainmentFailed);
        }
        let required_streams: i64 = transaction.query_row(
            "SELECT CASE WHEN EXISTS(
                 SELECT 1 FROM native_children child
                 JOIN native_child_spawn_admissions admission
                   ON admission.native_child_spawn_admission_id = child.native_child_spawn_admission_id
                 JOIN pi_child_spawn_sidecars sidecar
                   ON sidecar.native_child_spawn_admission_id = admission.native_child_spawn_admission_id
                 WHERE child.native_child_id = ?1
             ) THEN 4 ELSE 2 END",
            [child_id.value()], |r| r.get(0),
        ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
        let retained_seals: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM native_child_stream_seals
              WHERE native_child_id = ?1 AND completeness IN (1, 2)",
                [child_id.value()],
                |r| r.get(0),
            )
            .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
        let required_evaluator_output: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM native_child_stream_seals
              WHERE native_child_id = ?1 AND stream_kind IN (3, 4) AND completeness IN (1, 2)",
                [child_id.value()],
                |r| r.get(0),
            )
            .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
        if retained_seals != required_streams
            || (required_streams == 2 && required_evaluator_output != 2)
        {
            return Err(Rejection::ChildLifecycleReceiptMissing);
        }
        match (kind, value) {
            (1, Some(_)) => ChildTerminalDisposition::Exited,
            (2, Some(signal_number)) => signal_terminal_disposition(
                transaction,
                child_id,
                ProcessSignalNumber::try_from(
                    i32::try_from(signal_number)
                        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?,
                )
                .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?,
            )?,
            // Unknown wait status is a distinct loss-of-parentage fact. M5
            // never converts it into a close-eligible child finalization.
            (3, None) => return Err(Rejection::ChildLifecycleReceiptMissing),
            _ => return Err(Rejection::ChildLifecycleReceiptMissing),
        }
    } else if state == ChildProcessState::ContainmentFailed {
        return Err(Rejection::ProcessContainmentFailed);
    } else {
        return Err(Rejection::ChildLifecycleReceiptMissing);
    };
    transaction.execute("UPDATE native_children SET lifecycle_state = ?1, terminal_disposition = ?2, last_transition_command_id = ?3 WHERE native_child_id = ?4", params![ChildProcessState::Finalized as i64, disposition as i64, command_row_id, child_id.value()]).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    // A physical exit is not a Pi/model outcome, submission, validation, or
    // budget reconciliation. M5 therefore ends only the child receipt chain;
    // later normalized supervisor/Pi receipts own semantic Attempt or Office
    // settlement. This intentionally keeps those higher-level close fences.
    Ok(EventBody::ChildProcessFinalized {
        native_child_id: child_id,
        disposition,
    })
}

fn begin_cancellation_propagation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    cancellation_request_id: CancellationRequestId,
) -> Result<EventBody, Rejection> {
    let row: Option<(i64, i64)> = transaction.query_row("SELECT operating_cycle_id, lifecycle_state FROM cancellation_requests WHERE cancellation_request_id = ?1", [cancellation_request_id.value()], |r| Ok((r.get(0)?, r.get(1)?))).optional().map_err(|_| Rejection::SubjectNotFound)?;
    let Some((cycle, request_state)) = row else {
        return Err(Rejection::SubjectNotFound);
    };
    let cycle_id = OperatingCycleId::try_from(cycle).map_err(|_| Rejection::SubjectNotFound)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected)?;
    if cycle.state != OperatingCycleState::Cancelling
        || request_state == CancellationState::Completed as i64
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("INSERT INTO cancellation_propagations(cancellation_request_id, operating_cycle_id, observed_generation, lifecycle_state, begun_by_command_id, reconciled_by_command_id) VALUES (?1, ?2, ?3, 1, ?4, NULL)", params![cancellation_request_id.value(), cycle_id.value(), cycle.generation.value(), command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let id = id_from_last_insert::<CancellationPropagationId>(transaction)?;
    // Snapshot owners before children. An admitted-but-unspawned owner stays
    // AwaitingChildReceipt until a typed invalidation or raced spawn resolves
    // it; a target with no admission is the explicit `not_running` fact.
    transaction.execute(
        "INSERT INTO cancellation_propagation_targets(cancellation_propagation_id, actor_attempt_id, root_authority_office_session_id, native_child_id, target_disposition)
         SELECT ?1, a.actor_attempt_id, NULL, p.native_child_id,
                CASE
                  WHEN p.native_child_id IS NULL AND s.native_child_spawn_admission_id IS NOT NULL AND s.lifecycle_state = 1 THEN 2
                  WHEN p.native_child_id IS NULL THEN 1
                  WHEN p.lifecycle_state = 8 AND p.terminal_disposition = 1 THEN 3
                  WHEN p.lifecycle_state = 8 AND p.terminal_disposition = 4 THEN 4
                  WHEN p.lifecycle_state = 8 AND p.terminal_disposition = 5 THEN 5
                  WHEN p.lifecycle_state = 7 THEN 6
                  WHEN p.lifecycle_state = 6 AND p.terminal_disposition = 6 THEN 7
                  WHEN p.lifecycle_state = 5 THEN 2
                  ELSE 2
                END
           FROM attempts a
      LEFT JOIN native_child_spawn_admissions s ON s.actor_attempt_id = a.actor_attempt_id
      LEFT JOIN native_children p ON p.native_child_spawn_admission_id = s.native_child_spawn_admission_id
          WHERE a.operating_cycle_id = ?2 AND a.lifecycle_state IN (1, 2)",
        params![id.value(), cycle_id.value()],
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    transaction.execute(
        "INSERT INTO cancellation_propagation_targets(cancellation_propagation_id, actor_attempt_id, root_authority_office_session_id, deterministic_experiment_id, native_child_id, target_disposition)
         SELECT ?1, NULL, NULL, experiment.deterministic_experiment_id, child.native_child_id,
                CASE
                  WHEN child.native_child_id IS NULL AND admission.native_child_spawn_admission_id IS NOT NULL AND admission.lifecycle_state = 1 THEN 2
                  WHEN child.native_child_id IS NULL THEN 1
                  WHEN child.lifecycle_state = 8 AND child.terminal_disposition = 1 THEN 3
                  WHEN child.lifecycle_state = 8 AND child.terminal_disposition = 4 THEN 4
                  WHEN child.lifecycle_state = 8 AND child.terminal_disposition = 5 THEN 5
                  WHEN child.lifecycle_state = 7 THEN 6
                  WHEN child.lifecycle_state = 6 AND child.terminal_disposition = 6 THEN 7
                  ELSE 2
                END
           FROM deterministic_experiments experiment
      LEFT JOIN native_child_spawn_admissions admission
             ON admission.deterministic_experiment_id = experiment.deterministic_experiment_id
      LEFT JOIN native_children child
             ON child.native_child_spawn_admission_id = admission.native_child_spawn_admission_id
          WHERE experiment.operating_cycle_id = ?2 AND experiment.lifecycle_state = 1",
        params![id.value(), cycle_id.value()],
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    transaction.execute(
        "INSERT INTO cancellation_propagation_targets(cancellation_propagation_id, actor_attempt_id, root_authority_office_session_id, native_child_id, target_disposition)
         SELECT ?1, NULL, o.root_authority_office_session_id, p.native_child_id,
                CASE
                  WHEN p.native_child_id IS NULL AND s.native_child_spawn_admission_id IS NOT NULL AND s.lifecycle_state = 1 THEN 2
                  WHEN p.native_child_id IS NULL THEN 1
                  WHEN p.lifecycle_state = 8 AND p.terminal_disposition = 1 THEN 3
                  WHEN p.lifecycle_state = 8 AND p.terminal_disposition = 4 THEN 4
                  WHEN p.lifecycle_state = 8 AND p.terminal_disposition = 5 THEN 5
                  WHEN p.lifecycle_state = 7 THEN 6
                  WHEN p.lifecycle_state = 6 AND p.terminal_disposition = 6 THEN 7
                  WHEN p.lifecycle_state = 5 THEN 2
                  ELSE 2
                END
           FROM root_authority_office_sessions o
      LEFT JOIN native_child_spawn_admissions s ON s.root_authority_office_session_id = o.root_authority_office_session_id
      LEFT JOIN native_children p ON p.native_child_spawn_admission_id = s.native_child_spawn_admission_id
          WHERE o.operating_cycle_id = ?2 AND o.lifecycle_state NOT IN (8, 10, 11)",
        params![id.value(), cycle_id.value()],
    ).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    transaction.execute("INSERT INTO cancellation_propagation_children(cancellation_propagation_id, native_child_id) SELECT ?1, native_child_id FROM cancellation_propagation_targets WHERE cancellation_propagation_id = ?1 AND native_child_id IS NOT NULL AND target_disposition = 2", [id.value()]).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    transaction.execute("UPDATE cancellation_requests SET lifecycle_state = ?1 WHERE cancellation_request_id = ?2", params![CancellationState::Propagating as i64, cancellation_request_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    Ok(EventBody::CancellationPropagationBegun {
        cancellation_propagation_id: id,
        cancellation_request_id,
    })
}

fn reconcile_cancellation_propagation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected: ExpectedGeneration,
    propagation_id: CancellationPropagationId,
) -> Result<EventBody, Rejection> {
    let row: Option<(i64, i64, i64)> = transaction.query_row("SELECT operating_cycle_id, lifecycle_state, cancellation_request_id FROM cancellation_propagations WHERE cancellation_propagation_id = ?1", [propagation_id.value()], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional().map_err(|_| Rejection::SubjectNotFound)?;
    let Some((cycle, state, _)) = row else {
        return Err(Rejection::SubjectNotFound);
    };
    let cycle_id = OperatingCycleId::try_from(cycle).map_err(|_| Rejection::SubjectNotFound)?;
    let _ = cycle_for_generation(transaction, cycle_id, expected)?;
    if state != CancellationPropagationState::Propagating as i64 {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction
        .execute(
            "UPDATE cancellation_propagation_targets
            SET target_disposition = CASE
                 WHEN p.lifecycle_state = 8 AND p.terminal_disposition = 1 THEN 3
                 WHEN p.lifecycle_state = 8 AND p.terminal_disposition = 4 THEN 4
                 WHEN p.lifecycle_state = 8 AND p.terminal_disposition = 5 THEN 5
                 WHEN p.lifecycle_state = 7 THEN 6
                 WHEN p.lifecycle_state = 6 AND p.terminal_disposition = 6 THEN 7
                 ELSE target_disposition
               END
           FROM native_children p
          WHERE cancellation_propagation_targets.cancellation_propagation_id = ?1
            AND cancellation_propagation_targets.native_child_id = p.native_child_id",
            [propagation_id.value()],
        )
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    let containment: i64 = transaction.query_row("SELECT COUNT(*) FROM cancellation_propagation_targets WHERE cancellation_propagation_id = ?1 AND target_disposition = 6", [propagation_id.value()], |r| r.get(0)).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    if containment != 0 {
        transaction.execute("UPDATE cancellation_propagations SET lifecycle_state = 3 WHERE cancellation_propagation_id = ?1", [propagation_id.value()]).map_err(|_| Rejection::ProcessContainmentFailed)?;
        return Ok(EventBody::CancellationPropagationContainmentFailed {
            cancellation_propagation_id: propagation_id,
        });
    }
    let unfinished: i64 = transaction.query_row("SELECT COUNT(*) FROM cancellation_propagation_targets WHERE cancellation_propagation_id = ?1 AND target_disposition = 2", [propagation_id.value()], |r| r.get(0)).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    if unfinished != 0 {
        return Err(Rejection::CancellationPropagationIncomplete);
    }
    transaction.execute("UPDATE cancellation_propagations SET lifecycle_state = 2, reconciled_by_command_id = ?1 WHERE cancellation_propagation_id = ?2", params![command_row_id, propagation_id.value()]).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    Ok(EventBody::CancellationPropagationReconciled {
        cancellation_propagation_id: propagation_id,
    })
}

fn set_child_state(
    transaction: &Transaction<'_>,
    child_id: NativeChildId,
    state: ChildProcessState,
    command_row_id: i64,
) -> Result<(), Rejection> {
    transaction.execute("UPDATE native_children SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE native_child_id = ?3", params![state as i64, command_row_id, child_id.value()]).map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    Ok(())
}

fn mark_child_containment_failed(
    transaction: &Transaction<'_>,
    child_id: NativeChildId,
    command_row_id: i64,
) -> Result<(), Rejection> {
    transaction
        .execute(
            "UPDATE native_children
            SET lifecycle_state = ?1, terminal_disposition = ?2,
                last_transition_command_id = ?3
          WHERE native_child_id = ?4",
            params![
                ChildProcessState::ContainmentFailed as i64,
                ChildTerminalDisposition::ContainmentFailed as i64,
                command_row_id,
                child_id.value(),
            ],
        )
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    Ok(())
}

fn pi_protocol_state(
    transaction: &Transaction<'_>,
    child_id: NativeChildId,
) -> Result<PiChildSessionState, Rejection> {
    let value: i64 = transaction
        .query_row(
            "SELECT lifecycle_state FROM pi_child_session_protocols WHERE native_child_id = ?1",
            [child_id.value()],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?
        .ok_or(Rejection::ChildLifecycleReceiptMissing)?;
    match value {
        1 => Ok(PiChildSessionState::InertSpawned),
        2 => Ok(PiChildSessionState::AdapterReady),
        3 => Ok(PiChildSessionState::CreateAuthorized),
        4 => Ok(PiChildSessionState::CreateDelivered),
        5 => Ok(PiChildSessionState::SessionReady),
        _ => Err(Rejection::ChildLifecycleReceiptMissing),
    }
}

fn set_pi_protocol_state(
    transaction: &Transaction<'_>,
    child_id: NativeChildId,
    state: PiChildSessionState,
) -> Result<(), Rejection> {
    transaction
        .execute(
            "UPDATE pi_child_session_protocols SET lifecycle_state = ?1 WHERE native_child_id = ?2",
            params![state as i64, child_id.value()],
        )
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?;
    Ok(())
}

fn signal_terminal_disposition(
    transaction: &Transaction<'_>,
    child_id: NativeChildId,
    signal_number: ProcessSignalNumber,
) -> Result<ChildTerminalDisposition, Rejection> {
    // A delivered supervisory action is not itself proof of the process's
    // terminal cause. The waited signal must agree with an exact delivered
    // action. AdapterAbort is intentionally absent: without a typed Pi
    // terminal receipt it cannot turn an arbitrary crash into "cooperative".
    let expected_action = match signal_number.value() {
        15 => ProcessSignalAction::Terminate,
        9 => ProcessSignalAction::Kill,
        _ => return Err(Rejection::ChildLifecycleReceiptMissing),
    };
    let delivered: bool = transaction
        .query_row(
            "SELECT EXISTS(
           SELECT 1 FROM process_signal_receipts
            WHERE native_child_id = ?1
              AND signal_action IN (?2, ?3)
              AND delivery = ?4)",
            params![
                child_id.value(),
                expected_action as i64,
                if expected_action == ProcessSignalAction::Kill {
                    ProcessSignalAction::LingeringGroupKill as i64
                } else {
                    -1_i64
                },
                ProcessSignalDelivery::Delivered as i64,
            ],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| Rejection::ChildLifecycleReceiptMissing)?
        != 0;
    if !delivered {
        return Err(Rejection::ChildLifecycleReceiptMissing);
    }
    Ok(match expected_action {
        ProcessSignalAction::Terminate => ChildTerminalDisposition::Terminated,
        ProcessSignalAction::Kill => ChildTerminalDisposition::Killed,
        _ => return Err(Rejection::ChildLifecycleReceiptMissing),
    })
}

fn capability_grant(
    transaction: &Transaction<'_>,
    principal_id: PrincipalId,
    capability: Capability,
    capability_grant_id: crate::CapabilityGrantId,
) -> Result<Option<CapabilityGrantLookup>, StoreError> {
    let grant = transaction
        .query_row(
            "SELECT grant_state, office_occupancy_id, actor_instance_id FROM capability_grants
             WHERE capability_grant_id = ?1 AND principal_id = ?2 AND capability_kind = ?3",
            params![
                capability_grant_id.value(),
                principal_id.value(),
                capability as i64
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .optional()?;
    match grant {
        Some((1, office_occupancy_id, actor_instance_id)) => {
            Ok(Some(CapabilityGrantLookup::Active {
                grant_id: capability_grant_id.value(),
                office_occupancy_id: office_occupancy_id
                    .map(OfficeOccupancyId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_instance_id: actor_instance_id
                    .map(ActorInstanceId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }))
        }
        Some(_) => Ok(Some(CapabilityGrantLookup::Inactive)),
        None => Ok(None),
    }
}

fn grant_has_active_occupancy(
    transaction: &Transaction<'_>,
    grant_id: i64,
) -> Result<bool, StoreError> {
    transaction
        .query_row(
            "SELECT EXISTS(
             SELECT 1 FROM capability_grants g
             JOIN office_occupancies o ON o.office_occupancy_id = g.office_occupancy_id
             WHERE g.capability_grant_id = ?1
               AND o.active = 1
               AND o.principal_id = g.principal_id
         )",
            [grant_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(StoreError::from)
}

fn grant_has_active_actor_instance(
    transaction: &Transaction<'_>,
    grant_id: i64,
) -> Result<bool, StoreError> {
    transaction
        .query_row(
            "SELECT EXISTS(
             SELECT 1 FROM capability_grants g
             JOIN actor_instances a ON a.actor_instance_id = g.actor_instance_id
             JOIN principals p ON p.principal_id = a.principal_id
             WHERE g.capability_grant_id = ?1
               AND a.lifecycle_state = 1
               AND p.active = 1
               AND p.principal_id = g.principal_id
         )",
            [grant_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(StoreError::from)
}

/// Every actor-side command that governs a cycle, session, or cost incident is
/// bound to that object's pinned Office occupancy. A merely active Root
/// Authority grant is not interchangeable with the grant that governed the
/// scoped object when succession becomes possible.
fn command_target_occupancy(
    transaction: &Transaction<'_>,
    body: &CommandBody,
) -> Result<Option<OfficeOccupancyId>, Rejection> {
    match body {
        CommandBody::ProposeOperatingCycle { .. } => {
            Ok(Some(bootstrapped_constitution(transaction)?.2))
        }
        CommandBody::AdmitOperatingCycle { cycle_id }
        | CommandBody::StartRootAuthorityOfficeSession { cycle_id }
        | CommandBody::QuiesceOperatingCycle { cycle_id }
        | CommandBody::ResumeOperatingCycle { cycle_id }
        | CommandBody::ReconcileOperatingCycle { cycle_id }
        | CommandBody::CloseOperatingCycle { cycle_id }
        | CommandBody::ReserveBudget { cycle_id, .. }
        | CommandBody::RequestCancellation { cycle_id, .. } => {
            Ok(Some(cycle_row(transaction, *cycle_id)?.occupancy_id))
        }
        CommandBody::OpenOfficeTurn { session_id, .. } => {
            session_occupancy_id(transaction, *session_id).map(Some)
        }
        CommandBody::CloseCostPostmortem { postmortem_id, .. } => {
            let cycle_id = transaction
                .query_row(
                    "SELECT operating_cycle_id FROM cost_postmortems WHERE postmortem_id = ?1",
                    [postmortem_id.value()],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(|_| Rejection::SubjectNotFound)?
                .ok_or(Rejection::SubjectNotFound)?;
            Ok(Some(
                cycle_row(
                    transaction,
                    OperatingCycleId::try_from(cycle_id).map_err(|_| Rejection::SubjectNotFound)?,
                )?
                .occupancy_id,
            ))
        }
        CommandBody::CreateProject {
            operating_cycle_id, ..
        }
        | CommandBody::CharterProject {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionProject {
            operating_cycle_id, ..
        }
        | CommandBody::CompleteProjectMilestone {
            operating_cycle_id, ..
        }
        | CommandBody::ReopenProject {
            operating_cycle_id, ..
        }
        | CommandBody::CreateTicket {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionTicket {
            operating_cycle_id, ..
        }
        | CommandBody::AddGraphObjectRevision {
            operating_cycle_id, ..
        }
        | CommandBody::CommitGraphRevision {
            operating_cycle_id, ..
        }
        | CommandBody::AddGraphEdge {
            operating_cycle_id, ..
        }
        | CommandBody::CreateEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::ReopenEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::RequestAdversarialReview {
            operating_cycle_id, ..
        }
        | CommandBody::AssignAdversarialReviewer {
            operating_cycle_id, ..
        }
        | CommandBody::SubmitReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::RespondToReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::DispositionReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::ResolveAdversarialReview {
            operating_cycle_id, ..
        }
        | CommandBody::TriggerPostmortem {
            operating_cycle_id, ..
        }
        | CommandBody::RecordPostmortemCausalClaim {
            operating_cycle_id, ..
        }
        | CommandBody::ProposePostmortemAction {
            operating_cycle_id, ..
        }
        | CommandBody::ClosePostmortem {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterContextPack {
            operating_cycle_id, ..
        }
        | CommandBody::AdmitActorInstance {
            operating_cycle_id, ..
        }
        | CommandBody::AdmitTicket {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterWorkItem {
            operating_cycle_id, ..
        }
        | CommandBody::StartActorAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::ValidateTicketAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::RetryActorAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::CompleteTicket {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterOutcomeObligation {
            operating_cycle_id, ..
        }
        | CommandBody::ResolveOutcomeObligation {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id, ..
        }
        | CommandBody::FinalizeDeterministicExperiment {
            operating_cycle_id, ..
        } => Ok(Some(
            cycle_row(transaction, *operating_cycle_id)?.occupancy_id,
        )),
        _ => Ok(None),
    }
}

fn only_society_id(transaction: &Transaction<'_>) -> Result<SocietyId, Rejection> {
    let value = transaction
        .query_row("SELECT society_id FROM societies LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    SocietyId::try_from(value).map_err(|_| Rejection::SubjectNotFound)
}

fn root_authority_office_id(transaction: &Transaction<'_>) -> Result<OfficeId, Rejection> {
    let value = transaction
        .query_row(
            "SELECT office_id FROM office_contracts WHERE office_kind = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    OfficeId::try_from(value).map_err(|_| Rejection::SubjectNotFound)
}

fn active_founding_mission_id(
    transaction: &Transaction<'_>,
    society_id: SocietyId,
) -> Result<FoundingMissionId, Rejection> {
    let value = transaction
        .query_row(
            "SELECT founding_mission_id FROM founding_missions WHERE society_id = ?1 AND active = 1",
            [society_id.value()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    FoundingMissionId::try_from(value).map_err(|_| Rejection::SubjectNotFound)
}

fn active_root_authority_occupancy_id(
    transaction: &Transaction<'_>,
) -> Result<OfficeOccupancyId, Rejection> {
    let value = transaction
        .query_row(
            "SELECT o.office_occupancy_id FROM office_occupancies o
         JOIN office_contracts c ON c.office_id = o.office_id
         WHERE c.office_kind = 1 AND o.active = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    OfficeOccupancyId::try_from(value).map_err(|_| Rejection::SubjectNotFound)
}

fn hard_ceiling_from_event_body(transaction: &Transaction<'_>) -> Result<UsdMicros, Rejection> {
    let value = transaction
        .query_row(
            "SELECT ceiling_micros FROM event_r0_hard_ceiling_set ORDER BY event_id DESC LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::FoundingInvariant)?;
    UsdMicros::try_from(value).map_err(|_| Rejection::FoundingInvariant)
}

fn bootstrapped_constitution(
    transaction: &Transaction<'_>,
) -> Result<(SocietyId, FoundingMissionId, OfficeOccupancyId, UsdMicros), Rejection> {
    let row = transaction
        .query_row(
            "SELECT society_id, founding_mission_id, office_occupancy_id, hard_ceiling_micros
         FROM society_bootstraps LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::FoundingInvariant)?;
    Ok((
        SocietyId::try_from(row.0).map_err(|_| Rejection::FoundingInvariant)?,
        FoundingMissionId::try_from(row.1).map_err(|_| Rejection::FoundingInvariant)?,
        OfficeOccupancyId::try_from(row.2).map_err(|_| Rejection::FoundingInvariant)?,
        UsdMicros::try_from(row.3).map_err(|_| Rejection::FoundingInvariant)?,
    ))
}

fn cycle_row(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<CycleRow, Rejection> {
    let row = transaction.query_row(
        "SELECT society_id, founding_mission_id, office_occupancy_id, treatment, lifecycle_state, admission_generation
         FROM operating_cycles WHERE operating_cycle_id = ?1",
        [cycle_id.value()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?, row.get::<_, i64>(5)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok(CycleRow {
        society_id: SocietyId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        mission_id: FoundingMissionId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        occupancy_id: OfficeOccupancyId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        _treatment: operating_cycle_treatment_from_i64(row.3)
            .map_err(|_| Rejection::SubjectNotFound)?,
        state: operating_cycle_state_from_i64(row.4).map_err(|_| Rejection::SubjectNotFound)?,
        generation: AdmissionGeneration::try_from(row.5).map_err(|_| Rejection::SubjectNotFound)?,
    })
}

fn cycle_for_generation(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
    expected_generation: ExpectedGeneration,
) -> Result<CycleRow, Rejection> {
    let cycle = cycle_row(transaction, cycle_id)?;
    match expected_generation {
        ExpectedGeneration::NotApplicable => Err(Rejection::InvalidExpectedGeneration),
        ExpectedGeneration::Exact(generation) if generation != cycle.generation => {
            Err(Rejection::StaleAdmissionGeneration)
        }
        ExpectedGeneration::Exact(_) => Ok(cycle),
    }
}

fn session_row(
    transaction: &Transaction<'_>,
    session_id: RootAuthorityOfficeSessionId,
) -> Result<(OfficeSessionState, OperatingCycleId), Rejection> {
    let row = transaction
        .query_row(
            "SELECT lifecycle_state, operating_cycle_id FROM root_authority_office_sessions
         WHERE root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        office_session_state_from_i64(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        OperatingCycleId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn session_occupancy_id(
    transaction: &Transaction<'_>,
    session_id: RootAuthorityOfficeSessionId,
) -> Result<OfficeOccupancyId, Rejection> {
    let value = transaction
        .query_row(
            "SELECT office_occupancy_id FROM root_authority_office_sessions
             WHERE root_authority_office_session_id = ?1",
            [session_id.value()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    OfficeOccupancyId::try_from(value).map_err(|_| Rejection::SubjectNotFound)
}

fn transition_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    cycle_id: OperatingCycleId,
    state: OperatingCycleState,
    generation: AdmissionGeneration,
) -> Result<(), Rejection> {
    transaction
        .execute(
            "UPDATE operating_cycles SET lifecycle_state = ?1, admission_generation = ?2,
                                     last_transition_command_id = ?3 WHERE operating_cycle_id = ?4",
            params![
                state as i64,
                generation.value(),
                command_row_id,
                cycle_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    Ok(())
}

fn create_budget_envelope(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    ceiling: UsdMicros,
) -> Result<BudgetEnvelopeId, Rejection> {
    transaction.execute(
        "INSERT INTO budget_envelopes(ceiling_micros, reserved_micros, spent_micros, created_by_command_id)
         VALUES (?1, 0, 0, ?2)",
        params![ceiling.value(), command_row_id],
    ).map_err(|_| Rejection::BudgetCeilingExceeded)?;
    id_from_last_insert::<BudgetEnvelopeId>(transaction)
}

fn budget_envelopes_for_cycle(
    transaction: &Transaction<'_>,
    society_id: SocietyId,
    cycle_id: OperatingCycleId,
) -> Result<(BudgetEnvelopeId, BudgetEnvelopeId), Rejection> {
    let society_budget = transaction
        .query_row(
            "SELECT budget_envelope_id FROM budget_envelope_constraints WHERE society_id = ?1",
            [society_id.value()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    let cycle_budget = transaction.query_row(
        "SELECT budget_envelope_id FROM budget_envelope_constraints WHERE operating_cycle_id = ?1",
        [cycle_id.value()], |row| row.get::<_, i64>(0),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        BudgetEnvelopeId::try_from(society_budget).map_err(|_| Rejection::SubjectNotFound)?,
        BudgetEnvelopeId::try_from(cycle_budget).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn budget_amounts(
    transaction: &Transaction<'_>,
    budget_id: BudgetEnvelopeId,
) -> Result<(UsdMicros, UsdMicros, UsdMicros), Rejection> {
    let row = transaction.query_row(
        "SELECT ceiling_micros, reserved_micros, spent_micros FROM budget_envelopes WHERE budget_envelope_id = ?1",
        [budget_id.value()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    Ok((
        UsdMicros::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        UsdMicros::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        UsdMicros::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn active_office_turn_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction.query_row(
        "SELECT COUNT(*) FROM office_turns t
         JOIN root_authority_office_sessions s ON s.root_authority_office_session_id = t.root_authority_office_session_id
         WHERE s.operating_cycle_id = ?1 AND t.lifecycle_state = ?2",
        params![cycle_id.value(), OfficeTurnState::Active as i64],
        |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)
}

fn session_has_active_turn(
    transaction: &Transaction<'_>,
    session_id: RootAuthorityOfficeSessionId,
) -> Result<bool, Rejection> {
    transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM office_turns
             WHERE root_authority_office_session_id = ?1 AND lifecycle_state = ?2)",
            params![session_id.value(), OfficeTurnState::Active as i64],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(|_| Rejection::SubjectNotFound)
}

fn live_office_session_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM root_authority_office_sessions
             WHERE operating_cycle_id = ?1 AND lifecycle_state NOT IN (?2, ?3, ?4)",
            params![
                cycle_id.value(),
                OfficeSessionState::Closed as i64,
                OfficeSessionState::Cancelled as i64,
                OfficeSessionState::Failed as i64
            ],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)
}

fn unreconciled_reservation_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction.query_row(
        "SELECT COUNT(*) FROM budget_reservations WHERE operating_cycle_id = ?1 AND reservation_state != ?2",
        params![cycle_id.value(), BudgetReservationState::Reconciled as i64],
        |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)
}

fn active_cancellation_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM cancellation_requests WHERE operating_cycle_id = ?1
         AND lifecycle_state NOT IN (?2, ?3)",
            params![
                cycle_id.value(),
                CancellationState::Completed as i64,
                CancellationState::ContainmentFailed as i64
            ],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)
}

/// Work execution is not represented by an Office turn. These independent
/// actor-owned children must therefore drain before a cycle can be resumed or
/// closed; a lease with no budget reservation is still a material obligation.
fn active_work_lease_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM leases l
             JOIN actor_instances a ON a.actor_instance_id = l.actor_instance_id
             WHERE a.operating_cycle_id = ?1 AND l.lifecycle_state = ?2",
            params![cycle_id.value(), WorkLeaseState::Active as i64],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)
}

fn live_actor_attempt_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM attempts WHERE operating_cycle_id = ?1 AND lifecycle_state IN (?2, ?3)",
            params![
                cycle_id.value(),
                ActorAttemptState::Running as i64,
                ActorAttemptState::CancellationRequested as i64,
            ],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)
}

/// Physical native children are an independent close fence. Their terminal
/// disposition is intentionally not an Attempt/Office semantic settlement,
/// but a cycle cannot disappear while any process remains live or indeterminate.
fn live_native_child_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction.query_row(
        "SELECT COUNT(*) FROM native_children p
         JOIN native_child_spawn_admissions a ON a.native_child_spawn_admission_id = p.native_child_spawn_admission_id
         WHERE a.operating_cycle_id = ?1 AND p.lifecycle_state != ?2",
        params![cycle_id.value(), ChildProcessState::Finalized as i64],
        |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)
}

fn undisposed_pi_workspace_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM native_child_spawn_admissions admission
             JOIN pi_child_spawn_sidecars sidecar
               ON sidecar.native_child_spawn_admission_id = admission.native_child_spawn_admission_id
             WHERE admission.operating_cycle_id = ?1",
            [cycle_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)
}

fn active_cancellation_for_cycle(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<Option<CancellationRequestId>, Rejection> {
    transaction
        .query_row(
            "SELECT cancellation_request_id FROM cancellation_requests
             WHERE operating_cycle_id = ?1 AND lifecycle_state NOT IN (?2, ?3)
             ORDER BY cancellation_request_id ASC LIMIT 1",
            params![
                cycle_id.value(),
                CancellationState::Completed as i64,
                CancellationState::ContainmentFailed as i64,
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .map(CancellationRequestId::try_from)
        .transpose()
        .map_err(|_| Rejection::SubjectNotFound)
}

fn exists(transaction: &Transaction<'_>, query: &str) -> Result<bool, Rejection> {
    transaction
        .query_row(query, [], |row| row.get::<_, i64>(0))
        .optional()
        .map(|value| value.is_some())
        .map_err(|_| Rejection::SubjectNotFound)
}

fn id_from_last_insert<T>(transaction: &Transaction<'_>) -> Result<T, Rejection>
where
    T: TryFrom<i64>,
{
    T::try_from(transaction.last_insert_rowid()).map_err(|_| Rejection::SubjectNotFound)
}

fn expected_generation_to_sql(value: ExpectedGeneration) -> Option<i64> {
    match value {
        ExpectedGeneration::NotApplicable => None,
        ExpectedGeneration::Exact(generation) => Some(generation.value()),
    }
}

fn request_fingerprint(request: &CommandRequest) -> Blake3Digest {
    let mut bytes = Vec::with_capacity(128);
    put_bytes(&mut bytes, request.command_id.as_str().as_bytes());
    put_i64(&mut bytes, request.principal_id.value());
    put_i64(&mut bytes, request.capability_grant_id.value());
    put_i64(&mut bytes, request.capability as i64);
    match request.expected_generation {
        ExpectedGeneration::NotApplicable => put_i64(&mut bytes, -1),
        ExpectedGeneration::Exact(generation) => put_i64(&mut bytes, generation.value()),
    }
    put_i64(&mut bytes, request.body.kind() as i64);
    match &request.body {
        CommandBody::CreateSocietyIdentity { name } => {
            put_bytes(&mut bytes, name.as_str().as_bytes())
        }
        CommandBody::InstallRootAuthorityOffice | CommandBody::BootstrapSociety => {}
        CommandBody::InstallFoundingMission { mission } => {
            put_bytes(&mut bytes, mission.application_identity.as_str().as_bytes());
            put_bytes(&mut bytes, mission.application_name.as_str().as_bytes());
            put_i64(&mut bytes, mission.revision_ordinal.value());
            put_bytes(&mut bytes, mission.statement.as_str().as_bytes());
            put_i64(&mut bytes, mission.principles.as_slice().len() as i64);
            for principle in mission.principles.as_slice() {
                put_i64(&mut bytes, principle.kind as i64);
                put_bytes(&mut bytes, principle.text.as_str().as_bytes());
            }
            put_bytes(
                &mut bytes,
                mission.north_star_questions.change.as_str().as_bytes(),
            );
            put_bytes(
                &mut bytes,
                mission
                    .north_star_questions
                    .improvement_evidence
                    .as_str()
                    .as_bytes(),
            );
            put_bytes(
                &mut bytes,
                mission
                    .north_star_questions
                    .boundary_commitment
                    .as_str()
                    .as_bytes(),
            );
            put_bytes(
                &mut bytes,
                mission.north_star_questions.revisit.as_str().as_bytes(),
            );
            put_bytes(&mut bytes, &mission.source_rendering_digest.as_bytes());
        }
        CommandBody::AppointInitialRootAuthority { actor_display_name } => {
            put_bytes(&mut bytes, actor_display_name.as_str().as_bytes())
        }
        CommandBody::SetR0HardCeiling { ceiling } => put_i64(&mut bytes, ceiling.value()),
        CommandBody::ProposeOperatingCycle {
            treatment,
            budget_ceiling,
        } => {
            put_i64(&mut bytes, *treatment as i64);
            put_i64(&mut bytes, budget_ceiling.value());
        }
        CommandBody::AdmitOperatingCycle { cycle_id }
        | CommandBody::StartRootAuthorityOfficeSession { cycle_id }
        | CommandBody::QuiesceOperatingCycle { cycle_id }
        | CommandBody::RecordCycleDrained { cycle_id }
        | CommandBody::ResumeOperatingCycle { cycle_id }
        | CommandBody::ReconcileOperatingCycle { cycle_id }
        | CommandBody::CloseOperatingCycle { cycle_id } => put_i64(&mut bytes, cycle_id.value()),
        CommandBody::RecordOfficeSessionReady { session_id } => {
            put_i64(&mut bytes, session_id.value())
        }
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, *terminal_state as i64);
        }
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, *purpose as i64);
        }
        CommandBody::SettleOfficeTurn {
            turn_id,
            terminal_receipt_id,
        } => {
            put_i64(&mut bytes, turn_id.value());
            put_i64(&mut bytes, terminal_receipt_id.value());
        }
        CommandBody::ReserveBudget { cycle_id, amount } => {
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, amount.value());
        }
        CommandBody::ReconcileBudget {
            reservation_id,
            observation,
        } => {
            put_i64(&mut bytes, reservation_id.value());
            match observation {
                CostObservation::Known(amount) => {
                    put_i64(&mut bytes, 1);
                    put_i64(&mut bytes, amount.value());
                }
                CostObservation::Unknown(reason) => {
                    put_i64(&mut bytes, 2);
                    put_i64(&mut bytes, *reason as i64);
                }
                CostObservation::Unavailable(reason) => {
                    put_i64(&mut bytes, 3);
                    put_i64(&mut bytes, *reason as i64);
                }
            }
        }
        CommandBody::RequestCancellation { cycle_id, mode } => {
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, *mode as i64);
        }
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        } => put_i64(&mut bytes, cancellation_request_id.value()),
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution,
        } => {
            put_i64(&mut bytes, postmortem_id.value());
            put_i64(&mut bytes, *resolution as i64);
        }
        CommandBody::CreateProject {
            operating_cycle_id,
            project_name,
            north_star_alignment,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_bytes(&mut bytes, project_name.as_str().as_bytes());
            put_i64(
                &mut bytes,
                north_star_alignment.application_revision_id.value(),
            );
            put_bytes(
                &mut bytes,
                north_star_alignment.change_answer.as_str().as_bytes(),
            );
            put_bytes(
                &mut bytes,
                north_star_alignment
                    .improvement_evidence_answer
                    .as_str()
                    .as_bytes(),
            );
            put_bytes(
                &mut bytes,
                north_star_alignment
                    .boundary_commitment_answer
                    .as_str()
                    .as_bytes(),
            );
            put_bytes(
                &mut bytes,
                north_star_alignment.revisit_answer.as_str().as_bytes(),
            );
        }
        CommandBody::CharterProject {
            operating_cycle_id,
            project_id,
            objective,
            initial_milestone,
            stop_condition,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_bytes(&mut bytes, objective.as_str().as_bytes());
            put_bytes(&mut bytes, initial_milestone.as_str().as_bytes());
            put_bytes(&mut bytes, stop_condition.as_str().as_bytes());
        }
        CommandBody::TransitionProject {
            operating_cycle_id,
            project_id,
            target,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_i64(&mut bytes, *target as i64);
        }
        CommandBody::CompleteProjectMilestone {
            operating_cycle_id,
            project_milestone_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_milestone_id.value());
        }
        CommandBody::ReopenProject {
            operating_cycle_id,
            project_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
        }
        CommandBody::CreateTicket {
            operating_cycle_id,
            project_id,
            ticket_title,
            acceptance_condition,
            prerequisite_ticket_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_bytes(&mut bytes, ticket_title.as_str().as_bytes());
            put_bytes(&mut bytes, acceptance_condition.as_str().as_bytes());
            put_optional_i64(&mut bytes, prerequisite_ticket_id.map(TicketId::value));
        }
        CommandBody::TransitionTicket {
            operating_cycle_id,
            ticket_id,
            target,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, ticket_id.value());
            put_i64(&mut bytes, *target as i64);
        }
        CommandBody::AddGraphObjectRevision {
            operating_cycle_id,
            project_id,
            causal_episode_id,
            graph_object_id,
            body,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_optional_i64(&mut bytes, causal_episode_id.map(CausalEpisodeId::value));
            put_optional_i64(&mut bytes, graph_object_id.map(GraphObjectId::value));
            match body {
                GraphRevisionBody::Observation { observation } => {
                    put_i64(&mut bytes, GraphObjectKind::Observation as i64);
                    put_bytes(&mut bytes, observation.as_str().as_bytes());
                }
                GraphRevisionBody::Hypothesis { hypothesis } => {
                    put_i64(&mut bytes, GraphObjectKind::Hypothesis as i64);
                    put_bytes(&mut bytes, hypothesis.as_str().as_bytes());
                }
            }
        }
        CommandBody::CommitGraphRevision {
            operating_cycle_id,
            graph_revision_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, graph_revision_id.value());
        }
        CommandBody::AddGraphEdge {
            operating_cycle_id,
            project_id,
            from_graph_revision_id,
            to_graph_revision_id,
            edge_kind,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_i64(&mut bytes, from_graph_revision_id.value());
            put_i64(&mut bytes, to_graph_revision_id.value());
            put_i64(&mut bytes, *edge_kind as i64);
        }
        CommandBody::CreateEpisode {
            operating_cycle_id,
            project_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
        }
        CommandBody::TransitionEpisode {
            operating_cycle_id,
            causal_episode_id,
            target,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, causal_episode_id.value());
            put_i64(&mut bytes, *target as i64);
        }
        CommandBody::ReopenEpisode {
            operating_cycle_id,
            causal_episode_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, causal_episode_id.value());
        }
        CommandBody::RequestAdversarialReview {
            operating_cycle_id,
            project_id,
            target_graph_revision_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_i64(&mut bytes, target_graph_revision_id.value());
        }
        CommandBody::AssignAdversarialReviewer {
            operating_cycle_id,
            adversarial_review_id,
            reviewer_principal_id,
            reviewer_actor_instance_id,
            reviewer_actor_attempt_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, adversarial_review_id.value());
            put_i64(&mut bytes, reviewer_principal_id.value());
            put_i64(&mut bytes, reviewer_actor_instance_id.value());
            put_i64(&mut bytes, reviewer_actor_attempt_id.value());
        }
        CommandBody::SubmitReviewChallenge {
            operating_cycle_id,
            adversarial_review_id,
            target_graph_revision_id,
            author_principal_id,
            severity,
            failure_hypothesis,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, adversarial_review_id.value());
            put_i64(&mut bytes, target_graph_revision_id.value());
            put_i64(&mut bytes, author_principal_id.value());
            put_i64(&mut bytes, *severity as i64);
            put_bytes(&mut bytes, failure_hypothesis.as_str().as_bytes());
        }
        CommandBody::RespondToReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            response,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, review_challenge_id.value());
            put_bytes(&mut bytes, response.as_str().as_bytes());
        }
        CommandBody::DispositionReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            disposition,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, review_challenge_id.value());
            put_i64(&mut bytes, *disposition as i64);
        }
        CommandBody::RecordContentSealReceipt { digest } => {
            put_bytes(&mut bytes, &digest.as_bytes());
        }
        CommandBody::RegisterContentObject {
            content_seal_receipt_id,
        } => {
            put_i64(&mut bytes, content_seal_receipt_id.value());
        }
        CommandBody::RegisterForensicManifest {
            operating_cycle_id,
            producing_deterministic_experiment_id,
            capture_policy,
            retention_access_class,
            evaluator_output_content_object_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, producing_deterministic_experiment_id.value());
            put_i64(&mut bytes, *capture_policy as i64);
            put_i64(&mut bytes, *retention_access_class as i64);
            put_i64(&mut bytes, evaluator_output_content_object_id.value());
        }
        CommandBody::RegisterDeterministicEvaluatorForensicManifest {
            operating_cycle_id,
            native_child_spawn_admission_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, native_child_spawn_admission_id.value());
        }
        CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id,
            project_id,
            ticket_id,
            target_graph_revision_id,
            evaluator_content_object_id,
            input_manifest_content_object_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_i64(&mut bytes, ticket_id.value());
            put_i64(&mut bytes, target_graph_revision_id.value());
            put_i64(&mut bytes, evaluator_content_object_id.value());
            put_i64(&mut bytes, input_manifest_content_object_id.value());
        }
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            forensic_manifest_id,
            evaluator_output_content_object_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, deterministic_experiment_id.value());
            put_i64(&mut bytes, evaluator_revision_id.value());
            put_i64(&mut bytes, input_manifest_id.value());
            put_i64(&mut bytes, forensic_manifest_id.value());
            put_i64(&mut bytes, evaluator_output_content_object_id.value());
        }
        CommandBody::AdmitDeterministicEvidence {
            operating_cycle_id,
            deterministic_evaluation_receipt_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            evaluator_output_content_object_id,
            related_graph_revision_id,
            semantic_role,
            applicability,
            limitation,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, deterministic_evaluation_receipt_id.value());
            put_i64(&mut bytes, deterministic_experiment_id.value());
            put_i64(&mut bytes, evaluator_revision_id.value());
            put_i64(&mut bytes, input_manifest_id.value());
            put_i64(&mut bytes, evaluator_output_content_object_id.value());
            put_i64(&mut bytes, related_graph_revision_id.value());
            put_i64(&mut bytes, *semantic_role as i64);
            put_i64(&mut bytes, *applicability as i64);
            put_bytes(&mut bytes, limitation.as_str().as_bytes());
        }
        CommandBody::FinalizeDeterministicExperiment {
            operating_cycle_id,
            deterministic_experiment_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, deterministic_experiment_id.value());
        }
        CommandBody::AdmitDeterministicEvaluatorNativeChild {
            operating_cycle_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            execution_profile_id,
            native_workspace_id,
            canonical_workspace_path,
            supervisor_epoch_id,
            supervisor_epoch_identity,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, deterministic_experiment_id.value());
            put_i64(&mut bytes, evaluator_revision_id.value());
            put_i64(&mut bytes, input_manifest_id.value());
            put_i64(&mut bytes, execution_profile_id.value());
            put_bytes(&mut bytes, native_workspace_id.as_str().as_bytes());
            put_bytes(&mut bytes, canonical_workspace_path.as_str().as_bytes());
            put_i64(&mut bytes, supervisor_epoch_id.value());
            put_bytes(&mut bytes, supervisor_epoch_identity.as_str().as_bytes());
        }
        CommandBody::RecordDeterministicEvaluatorNativeChildSpawn {
            native_child_spawn_admission_id,
            child_identity,
            direct_child_pid,
            process_group_id,
        } => {
            put_i64(&mut bytes, native_child_spawn_admission_id.value());
            put_bytes(&mut bytes, child_identity.as_str().as_bytes());
            put_i64(&mut bytes, i64::from(direct_child_pid.value()));
            put_i64(&mut bytes, i64::from(process_group_id.value()));
        }
        CommandBody::ResolveAdversarialReview {
            operating_cycle_id,
            adversarial_review_id,
            resolution,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, adversarial_review_id.value());
            put_i64(&mut bytes, *resolution as i64);
        }
        CommandBody::TriggerPostmortem {
            operating_cycle_id,
            project_id,
            causal_episode_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_optional_i64(&mut bytes, causal_episode_id.map(CausalEpisodeId::value));
        }
        CommandBody::RecordPostmortemCausalClaim {
            operating_cycle_id,
            postmortem_id,
            claim_kind,
            claim,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, postmortem_id.value());
            put_i64(&mut bytes, *claim_kind as i64);
            put_bytes(&mut bytes, claim.as_str().as_bytes());
        }
        CommandBody::ProposePostmortemAction {
            operating_cycle_id,
            postmortem_id,
            action_kind,
            action,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, postmortem_id.value());
            put_i64(&mut bytes, *action_kind as i64);
            put_bytes(&mut bytes, action.as_str().as_bytes());
        }
        CommandBody::ClosePostmortem {
            operating_cycle_id,
            postmortem_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, postmortem_id.value());
        }
        CommandBody::RegisterActorConfiguration {
            configuration_name,
            model_policy,
            primary_attractor,
        } => {
            put_bytes(&mut bytes, configuration_name.as_str().as_bytes());
            put_i64(&mut bytes, *model_policy as i64);
            put_i64(&mut bytes, *primary_attractor as i64);
        }
        CommandBody::RegisterContextPack {
            operating_cycle_id,
            purpose,
            rendering_digest,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, *purpose as i64);
            put_bytes(&mut bytes, &rendering_digest.as_bytes());
        }
        CommandBody::AdmitActorInstance {
            operating_cycle_id,
            actor_configuration_revision_id,
            execution_profile_id,
            actor_display_name,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, actor_configuration_revision_id.value());
            put_i64(&mut bytes, execution_profile_id.value());
            put_bytes(&mut bytes, actor_display_name.as_str().as_bytes());
        }
        CommandBody::AdmitTicket {
            operating_cycle_id,
            ticket_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, ticket_id.value());
        }
        CommandBody::RegisterWorkItem {
            operating_cycle_id,
            ticket_id,
            actor_instance_id,
            context_pack_id,
            work_kind,
            adversarial_review_id,
            assignment,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, ticket_id.value());
            put_i64(&mut bytes, actor_instance_id.value());
            put_i64(&mut bytes, context_pack_id.value());
            put_i64(&mut bytes, *work_kind as i64);
            put_optional_i64(
                &mut bytes,
                adversarial_review_id.map(AdversarialReviewId::value),
            );
            put_bytes(&mut bytes, assignment.as_str().as_bytes());
        }
        CommandBody::ClaimWorkItem {
            operating_cycle_id,
            work_item_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, work_item_id.value());
        }
        CommandBody::StartActorAttempt {
            operating_cycle_id,
            work_item_id,
            reservation_amount,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, work_item_id.value());
            put_i64(&mut bytes, reservation_amount.value());
        }
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id,
            terminal_kind,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, *terminal_kind as i64);
        }
        CommandBody::ValidateTicketAttempt {
            operating_cycle_id,
            actor_attempt_id,
        }
        | CommandBody::RetryActorAttempt {
            operating_cycle_id,
            actor_attempt_id,
        }
        | CommandBody::CompleteTicket {
            operating_cycle_id,
            actor_attempt_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, actor_attempt_id.value());
        }
        CommandBody::ExpireWorkLease { work_lease_id } => {
            put_i64(&mut bytes, work_lease_id.value())
        }
        CommandBody::CancelActorAttempt {
            actor_attempt_id,
            reason,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, *reason as i64);
        }
        CommandBody::RegisterOutcomeObligation {
            operating_cycle_id,
            project_id,
            obligation,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_bytes(&mut bytes, obligation.as_str().as_bytes());
        }
        CommandBody::ResolveOutcomeObligation {
            operating_cycle_id,
            outcome_obligation_id,
            disposition,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, outcome_obligation_id.value());
            put_i64(&mut bytes, *disposition as i64);
        }
        CommandBody::OpenSupervisorEpoch {
            supervisor_epoch_id,
            supervisor_epoch_identity,
        } => {
            put_i64(&mut bytes, supervisor_epoch_id.value());
            put_bytes(&mut bytes, supervisor_epoch_identity.as_str().as_bytes());
        }
        CommandBody::AdmitPiChildSpawn {
            operating_cycle_id,
            owner,
            budget_reservation_id,
            execution_profile_id,
            native_workspace_id,
            canonical_workspace_path,
            supervisor_epoch_id,
            supervisor_epoch_identity,
            pi_session_identity,
            spawn_nonce,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            match owner {
                PiChildOwner::ActorAttempt(id) => {
                    put_i64(&mut bytes, 1);
                    put_i64(&mut bytes, id.value());
                }
                PiChildOwner::RootAuthorityOfficeSession(id) => {
                    put_i64(&mut bytes, 2);
                    put_i64(&mut bytes, id.value());
                }
            }
            put_i64(&mut bytes, budget_reservation_id.value());
            put_i64(&mut bytes, execution_profile_id.value());
            put_bytes(&mut bytes, native_workspace_id.as_str().as_bytes());
            put_bytes(&mut bytes, canonical_workspace_path.as_str().as_bytes());
            put_i64(&mut bytes, supervisor_epoch_id.value());
            put_bytes(&mut bytes, supervisor_epoch_identity.as_str().as_bytes());
            put_bytes(&mut bytes, pi_session_identity.as_str().as_bytes());
            put_bytes(&mut bytes, spawn_nonce.as_str().as_bytes());
        }
        CommandBody::RecordInertChildSpawn {
            native_child_spawn_admission_id,
            child_identity,
            direct_child_pid,
            process_group_id,
        } => {
            put_i64(&mut bytes, native_child_spawn_admission_id.value());
            put_bytes(&mut bytes, child_identity.as_str().as_bytes());
            put_i64(&mut bytes, i64::from(direct_child_pid.value()));
            put_i64(&mut bytes, i64::from(process_group_id.value()));
        }
        CommandBody::RecordPiAdapterReady {
            native_child_id,
            pi_session_identity,
            spawn_nonce,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_bytes(&mut bytes, pi_session_identity.as_str().as_bytes());
            put_bytes(&mut bytes, spawn_nonce.as_str().as_bytes());
        }
        CommandBody::AuthorizePiCreateSession {
            native_child_id,
            correlation_identity,
            create_request_digest,
        }
        | CommandBody::RecordPiCreateSessionDelivery {
            native_child_id,
            correlation_identity,
            create_request_digest,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_bytes(&mut bytes, &create_request_digest.as_bytes());
        }
        CommandBody::RecordPiSessionReady {
            native_child_id,
            pi_session_identity,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_bytes(&mut bytes, pi_session_identity.as_str().as_bytes());
        }
        CommandBody::RecordPiAbortControlDelivery {
            native_child_id,
            cancellation_propagation_id,
            correlation_identity,
            abort_command_digest,
            outcome,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, cancellation_propagation_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_bytes(&mut bytes, &abort_command_digest.as_bytes());
            put_i64(&mut bytes, *outcome as i64);
        }
        CommandBody::RecordChildStreamSeal {
            native_child_id,
            stream_kind,
            full_observed_digest,
            retained_content_object_id,
            completeness,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, *stream_kind as i64);
            put_bytes(&mut bytes, &full_observed_digest.as_bytes());
            put_i64(&mut bytes, retained_content_object_id.value());
            put_i64(&mut bytes, *completeness as i64);
        }
        CommandBody::RecordChildProcessLiveness {
            native_child_id,
            liveness,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, *liveness as i64);
        }
        CommandBody::RecordProcessSignalReceipt {
            native_child_id,
            action,
            delivery,
            observed_liveness,
            cause,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, *action as i64);
            put_i64(&mut bytes, *delivery as i64);
            put_i64(&mut bytes, *observed_liveness as i64);
            put_process_signal_cause(&mut bytes, *cause);
        }
        CommandBody::RecordDirectChildReap {
            native_child_id,
            wait_status,
            group_liveness_before_cleanup,
            group_liveness_after_cleanup,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_direct_wait_status(&mut bytes, *wait_status);
            put_i64(&mut bytes, *group_liveness_before_cleanup as i64);
            put_i64(&mut bytes, *group_liveness_after_cleanup as i64);
        }
        CommandBody::RecordChildRecovery {
            native_child_id,
            observation,
            group_liveness_after_restart,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, *observation as i64);
            put_i64(&mut bytes, *group_liveness_after_restart as i64);
        }
        CommandBody::FinalizeChildProcess { native_child_id } => {
            put_i64(&mut bytes, native_child_id.value())
        }
        CommandBody::BeginCancellationPropagation {
            cancellation_request_id,
        } => put_i64(&mut bytes, cancellation_request_id.value()),
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id,
        } => put_i64(&mut bytes, cancellation_propagation_id.value()),
        CommandBody::RecordNativeChildNotSpawned {
            native_child_spawn_admission_id,
            reason,
        } => {
            put_i64(&mut bytes, native_child_spawn_admission_id.value());
            put_i64(&mut bytes, *reason as i64);
        }
        CommandBody::AuthorizePiOfficeTurnPrompt {
            office_turn_id,
            correlation_identity,
            prompt_content_object_id,
            prompt_digest,
            frontier_event_id,
        } => {
            put_i64(&mut bytes, office_turn_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, prompt_content_object_id.value());
            put_bytes(&mut bytes, &prompt_digest.as_bytes());
            put_i64(&mut bytes, frontier_event_id.value());
        }
        CommandBody::RecordPiOfficeTurnPromptDelivery {
            office_turn_id,
            correlation_identity,
            prompt_digest,
        } => {
            put_i64(&mut bytes, office_turn_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_bytes(&mut bytes, &prompt_digest.as_bytes());
        }
        CommandBody::RecordPiOfficeTurnPromptAccepted {
            office_turn_id,
            correlation_identity,
            command_result_sequence,
        } => {
            put_i64(&mut bytes, office_turn_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, command_result_sequence.value());
        }
        CommandBody::RecordPiOfficeTurnUsage {
            office_turn_id,
            correlation_identity,
            protocol_sequence,
            usage,
        } => {
            put_i64(&mut bytes, office_turn_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, protocol_sequence.value());
            put_pi_cumulative_usage(&mut bytes, *usage);
        }
        CommandBody::RecordPiOfficeTurnUsageFailure {
            office_turn_id,
            correlation_identity,
            protocol_sequence,
            failure,
        } => {
            put_i64(&mut bytes, office_turn_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, protocol_sequence.value());
            put_pi_office_turn_usage_failure(&mut bytes, *failure);
        }
        CommandBody::RecordPiOfficeTurnTerminal {
            office_turn_id,
            correlation_identity,
            terminal_evidence,
            settled_sequence,
            disposition,
            assistant_outcome,
            transcript_disposition,
        } => {
            put_i64(&mut bytes, office_turn_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_pi_office_turn_terminal_evidence(&mut bytes, *terminal_evidence);
            put_i64(&mut bytes, settled_sequence.value());
            put_i64(&mut bytes, *disposition as i64);
            put_i64(&mut bytes, *assistant_outcome as i64);
            put_i64(&mut bytes, *transcript_disposition as i64);
        }
        CommandBody::AuthorizePiOfficeSessionDispose {
            session_id,
            correlation_identity,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
        }
        CommandBody::RecordPiOfficeSessionDisposeDelivery {
            session_id,
            correlation_identity,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
        }
        CommandBody::RecordPiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity,
            command_result_sequence,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, command_result_sequence.value());
        }
        CommandBody::RecordPiOfficeSessionDisposeUsage {
            session_id,
            correlation_identity,
            protocol_sequence,
            usage,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, protocol_sequence.value());
            put_pi_cumulative_usage(&mut bytes, *usage);
        }
        CommandBody::RecordPiOfficeSessionDisposeUsageFailure {
            session_id,
            correlation_identity,
            protocol_sequence,
            failure,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, protocol_sequence.value());
            put_pi_office_turn_usage_failure(&mut bytes, *failure);
        }
        CommandBody::RecordPiOfficeSessionDisposed {
            session_id,
            correlation_identity,
            disposed_sequence,
            transcript_receipt,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, disposed_sequence.value());
            put_pi_office_session_transcript_receipt(&mut bytes, transcript_receipt);
        }
    }
    Blake3Digest::of_bytes(&bytes)
}

/// A compact integrity commitment to an exact ledger event. Event identity and
/// its command identity are committed before the closed body so relinking an
/// otherwise valid event to a different command is detectable. This is not a
/// hash chain: events remain independently inspectable.
fn event_fingerprint(event_id: EventId, command_id: &CommandId, body: &EventBody) -> Blake3Digest {
    let mut bytes = Vec::with_capacity(96);
    put_i64(&mut bytes, event_id.value());
    put_bytes(&mut bytes, command_id.as_str().as_bytes());
    put_i64(&mut bytes, body.kind() as i64);
    match body {
        EventBody::SocietyIdentityCreated { society_id }
        | EventBody::SocietyBootstrapped { society_id } => {
            put_i64(&mut bytes, society_id.value());
        }
        EventBody::RootAuthorityOfficeInstalled { office_id } => {
            put_i64(&mut bytes, office_id.value());
        }
        EventBody::FoundingMissionInstalled {
            mission_id,
            application_revision_id,
        } => {
            put_i64(&mut bytes, mission_id.value());
            put_i64(&mut bytes, application_revision_id.value());
        }
        EventBody::RootAuthorityAppointed {
            occupancy_id,
            principal_id,
        } => {
            put_i64(&mut bytes, occupancy_id.value());
            put_i64(&mut bytes, principal_id.value());
        }
        EventBody::R0HardCeilingSet {
            society_id,
            ceiling,
        } => {
            put_i64(&mut bytes, society_id.value());
            put_i64(&mut bytes, ceiling.value());
        }
        EventBody::OperatingCycleProposed {
            cycle_id,
            generation,
            treatment,
            budget_ceiling,
        } => {
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, generation.value());
            put_i64(&mut bytes, *treatment as i64);
            put_i64(&mut bytes, budget_ceiling.value());
        }
        EventBody::OperatingCycleStateChanged {
            cycle_id,
            state,
            generation,
        } => {
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, *state as i64);
            put_i64(&mut bytes, generation.value());
        }
        EventBody::RootAuthorityOfficeSessionStarted {
            session_id,
            cycle_id,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, cycle_id.value());
        }
        EventBody::RootAuthorityOfficeSessionStateChanged { session_id, state } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::OfficeTurnOpened {
            turn_id,
            session_id,
            purpose,
        } => {
            put_i64(&mut bytes, turn_id.value());
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, *purpose as i64);
        }
        EventBody::OfficeTurnSettled {
            turn_id,
            session_id,
            charged_delta,
        } => {
            put_i64(&mut bytes, turn_id.value());
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, charged_delta.value());
        }
        EventBody::BudgetReserved {
            reservation_id,
            cycle_id,
            amount,
        } => {
            put_i64(&mut bytes, reservation_id.value());
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, amount.value());
        }
        EventBody::BudgetReconciled {
            reservation_id,
            observed,
        } => {
            put_i64(&mut bytes, reservation_id.value());
            put_i64(&mut bytes, observed.value());
        }
        EventBody::BudgetAdmissionFrozen {
            reservation_id,
            cycle_id,
            cancellation_request_id,
            postmortem_id,
            reason,
        } => {
            put_i64(&mut bytes, reservation_id.value());
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, cancellation_request_id.value());
            put_i64(&mut bytes, postmortem_id.value());
            match reason {
                BudgetFreezeReason::KnownOverrun { observed, reserved } => {
                    put_i64(&mut bytes, 1);
                    put_i64(&mut bytes, observed.value());
                    put_i64(&mut bytes, reserved.value());
                }
                BudgetFreezeReason::Unknown(reason) => {
                    put_i64(&mut bytes, 2);
                    put_i64(&mut bytes, *reason as i64);
                }
                BudgetFreezeReason::Unavailable(reason) => {
                    put_i64(&mut bytes, 3);
                    put_i64(&mut bytes, *reason as i64);
                }
            }
        }
        EventBody::CancellationRequested {
            cancellation_request_id,
            cycle_id,
            mode,
            generation,
        } => {
            put_i64(&mut bytes, cancellation_request_id.value());
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, *mode as i64);
            put_i64(&mut bytes, generation.value());
        }
        EventBody::CancellationReconciled {
            cancellation_request_id,
            cycle_id,
        } => {
            put_i64(&mut bytes, cancellation_request_id.value());
            put_i64(&mut bytes, cycle_id.value());
        }
        EventBody::CostPostmortemClosed {
            postmortem_id,
            reservation_id,
            cycle_id,
            resolution,
            charged,
        } => {
            put_i64(&mut bytes, postmortem_id.value());
            put_i64(&mut bytes, reservation_id.value());
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, *resolution as i64);
            put_i64(&mut bytes, charged.value());
        }
        EventBody::ProjectCreated {
            project_id,
            application_revision_id,
        } => {
            put_i64(&mut bytes, project_id.value());
            put_i64(&mut bytes, application_revision_id.value());
        }
        EventBody::ProjectChartered { project_id } => put_i64(&mut bytes, project_id.value()),
        EventBody::ProjectStateChanged { project_id, state } => {
            put_i64(&mut bytes, project_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::ProjectMilestoneCompleted {
            project_milestone_id,
        } => put_i64(&mut bytes, project_milestone_id.value()),
        EventBody::TicketCreated {
            ticket_id,
            project_id,
        } => {
            put_i64(&mut bytes, ticket_id.value());
            put_i64(&mut bytes, project_id.value());
        }
        EventBody::TicketStateChanged { ticket_id, state } => {
            put_i64(&mut bytes, ticket_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::GraphObjectRevisionAdded {
            graph_object_id,
            graph_revision_id,
        } => {
            put_i64(&mut bytes, graph_object_id.value());
            put_i64(&mut bytes, graph_revision_id.value());
        }
        EventBody::GraphRevisionCommitted { graph_revision_id } => {
            put_i64(&mut bytes, graph_revision_id.value())
        }
        EventBody::GraphEdgeAdded { graph_edge_id } => put_i64(&mut bytes, graph_edge_id.value()),
        EventBody::EpisodeCreated {
            causal_episode_id,
            project_id,
        } => {
            put_i64(&mut bytes, causal_episode_id.value());
            put_i64(&mut bytes, project_id.value());
        }
        EventBody::EpisodeStateChanged {
            causal_episode_id,
            state,
        } => {
            put_i64(&mut bytes, causal_episode_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::AdversarialReviewRequested {
            adversarial_review_id,
        } => put_i64(&mut bytes, adversarial_review_id.value()),
        EventBody::AdversarialReviewerAssigned {
            adversarial_review_id,
            reviewer_principal_id,
            reviewer_actor_instance_id,
            reviewer_actor_attempt_id,
        } => {
            put_i64(&mut bytes, adversarial_review_id.value());
            put_i64(&mut bytes, reviewer_principal_id.value());
            put_i64(&mut bytes, reviewer_actor_instance_id.value());
            put_i64(&mut bytes, reviewer_actor_attempt_id.value());
        }
        EventBody::AdversarialReviewResolved {
            adversarial_review_id,
            state,
        } => {
            put_i64(&mut bytes, adversarial_review_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::ReviewChallengeSubmitted {
            review_challenge_id,
            author_principal_id,
        } => {
            put_i64(&mut bytes, review_challenge_id.value());
            put_i64(&mut bytes, author_principal_id.value());
        }
        EventBody::ReviewChallengeResponded {
            review_challenge_id,
        } => put_i64(&mut bytes, review_challenge_id.value()),
        EventBody::ReviewChallengeDispositioned {
            review_challenge_id,
            disposition,
        } => {
            put_i64(&mut bytes, review_challenge_id.value());
            put_i64(&mut bytes, *disposition as i64);
        }
        EventBody::PostmortemTriggered { postmortem_id }
        | EventBody::PostmortemClosed { postmortem_id } => {
            put_i64(&mut bytes, postmortem_id.value())
        }
        EventBody::PostmortemCausalClaimRecorded {
            postmortem_causal_claim_id,
        } => put_i64(&mut bytes, postmortem_causal_claim_id.value()),
        EventBody::PostmortemActionProposed {
            postmortem_action_proposal_id,
        } => put_i64(&mut bytes, postmortem_action_proposal_id.value()),
        EventBody::ActorConfigurationRegistered {
            actor_configuration_id,
            actor_configuration_revision_id,
        } => {
            put_i64(&mut bytes, actor_configuration_id.value());
            put_i64(&mut bytes, actor_configuration_revision_id.value());
        }
        EventBody::ContextPackRegistered { context_pack_id } => {
            put_i64(&mut bytes, context_pack_id.value())
        }
        EventBody::ActorInstanceAdmitted {
            actor_instance_id,
            principal_id,
        } => {
            put_i64(&mut bytes, actor_instance_id.value());
            put_i64(&mut bytes, principal_id.value());
        }
        EventBody::TicketAdmitted { ticket_id } => put_i64(&mut bytes, ticket_id.value()),
        EventBody::WorkItemRegistered {
            work_item_id,
            ticket_id,
            adversarial_review_id,
        } => {
            put_i64(&mut bytes, work_item_id.value());
            put_i64(&mut bytes, ticket_id.value());
            put_optional_i64(
                &mut bytes,
                adversarial_review_id.map(AdversarialReviewId::value),
            );
        }
        EventBody::WorkItemClaimed {
            work_item_id,
            work_lease_id,
            actor_instance_id,
        } => {
            put_i64(&mut bytes, work_item_id.value());
            put_i64(&mut bytes, work_lease_id.value());
            put_i64(&mut bytes, actor_instance_id.value());
        }
        EventBody::ActorAttemptStarted {
            actor_attempt_id,
            work_item_id,
            budget_reservation_id,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, work_item_id.value());
            put_i64(&mut bytes, budget_reservation_id.value());
        }
        EventBody::ActorAttemptTerminalAttested {
            actor_attempt_id,
            terminal_kind,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, *terminal_kind as i64);
        }
        EventBody::TicketAttemptValidated {
            actor_attempt_id,
            ticket_id,
        }
        | EventBody::TicketCompleted {
            actor_attempt_id,
            ticket_id,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, ticket_id.value());
        }
        EventBody::ActorAttemptRetryPrepared {
            actor_attempt_id,
            work_item_id,
            ticket_id,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, work_item_id.value());
            put_i64(&mut bytes, ticket_id.value());
        }
        EventBody::WorkLeaseExpired {
            work_lease_id,
            work_item_id,
        } => {
            put_i64(&mut bytes, work_lease_id.value());
            put_i64(&mut bytes, work_item_id.value());
        }
        EventBody::ActorAttemptCancellationRequested {
            actor_attempt_id,
            reason,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, *reason as i64);
        }
        EventBody::OutcomeObligationRegistered {
            outcome_obligation_id,
            project_id,
        } => {
            put_i64(&mut bytes, outcome_obligation_id.value());
            put_i64(&mut bytes, project_id.value());
        }
        EventBody::OutcomeObligationResolved {
            outcome_obligation_id,
            state,
        } => {
            put_i64(&mut bytes, outcome_obligation_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::ContentSealReceiptRecorded {
            content_seal_receipt_id,
            digest,
        } => {
            put_i64(&mut bytes, content_seal_receipt_id.value());
            put_bytes(&mut bytes, &digest.as_bytes());
        }
        EventBody::ContentObjectRegistered {
            content_object_id,
            content_seal_receipt_id,
        } => {
            put_i64(&mut bytes, content_object_id.value());
            put_i64(&mut bytes, content_seal_receipt_id.value());
        }
        EventBody::ForensicManifestRegistered {
            forensic_manifest_id,
            producing_deterministic_experiment_id,
            evaluator_output_content_object_id,
        } => {
            put_i64(&mut bytes, forensic_manifest_id.value());
            put_i64(&mut bytes, producing_deterministic_experiment_id.value());
            put_i64(&mut bytes, evaluator_output_content_object_id.value());
        }
        EventBody::DeterministicEvaluatorForensicManifestRegistered {
            forensic_manifest_id,
            deterministic_experiment_id,
            native_child_spawn_admission_id,
            native_child_stream_seal_id,
            evaluator_output_content_object_id,
        } => {
            put_i64(&mut bytes, forensic_manifest_id.value());
            put_i64(&mut bytes, deterministic_experiment_id.value());
            put_i64(&mut bytes, native_child_spawn_admission_id.value());
            put_i64(&mut bytes, native_child_stream_seal_id.value());
            put_i64(&mut bytes, evaluator_output_content_object_id.value());
        }
        EventBody::DeterministicExperimentRegistered {
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
        } => {
            put_i64(&mut bytes, deterministic_experiment_id.value());
            put_i64(&mut bytes, evaluator_revision_id.value());
            put_i64(&mut bytes, input_manifest_id.value());
        }
        EventBody::DeterministicEvaluationReceiptRecorded {
            deterministic_evaluation_receipt_id,
            deterministic_experiment_id,
        } => {
            put_i64(&mut bytes, deterministic_evaluation_receipt_id.value());
            put_i64(&mut bytes, deterministic_experiment_id.value());
        }
        EventBody::DeterministicEvidenceAdmitted {
            evidence_admission_id,
            deterministic_evaluation_receipt_id,
            semantic_role,
            applicability,
        } => {
            put_i64(&mut bytes, evidence_admission_id.value());
            put_i64(&mut bytes, deterministic_evaluation_receipt_id.value());
            put_i64(&mut bytes, *semantic_role as i64);
            put_i64(&mut bytes, *applicability as i64);
        }
        EventBody::DeterministicExperimentFinalized {
            deterministic_experiment_id,
            terminal_state,
        } => {
            put_i64(&mut bytes, deterministic_experiment_id.value());
            put_i64(&mut bytes, *terminal_state as i64);
        }
        EventBody::DeterministicEvaluatorNativeChildAdmitted {
            native_child_spawn_admission_id,
            owner,
        } => {
            put_i64(&mut bytes, native_child_spawn_admission_id.value());
            match owner {
                NativeChildOwner::DeterministicEvaluator {
                    deterministic_experiment_id,
                    evaluator_revision_id,
                    input_manifest_id,
                } => {
                    put_i64(&mut bytes, 2);
                    put_i64(&mut bytes, deterministic_experiment_id.value());
                    put_i64(&mut bytes, evaluator_revision_id.value());
                    put_i64(&mut bytes, input_manifest_id.value());
                }
                NativeChildOwner::Pi(_) => {
                    unreachable!("evaluator admission cannot carry Pi owner")
                }
            }
        }
        EventBody::DeterministicEvaluatorNativeChildSpawnRecorded {
            native_child_id,
            native_child_spawn_admission_id,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, native_child_spawn_admission_id.value());
        }
        EventBody::PiChildSpawnAdmitted {
            native_child_spawn_admission_id,
            owner,
            budget_reservation_id,
        } => {
            put_i64(&mut bytes, native_child_spawn_admission_id.value());
            match owner {
                PiChildOwner::ActorAttempt(id) => {
                    put_i64(&mut bytes, 1);
                    put_i64(&mut bytes, id.value());
                }
                PiChildOwner::RootAuthorityOfficeSession(id) => {
                    put_i64(&mut bytes, 2);
                    put_i64(&mut bytes, id.value());
                }
            }
            put_i64(&mut bytes, budget_reservation_id.value());
        }
        EventBody::InertPiChildSpawnRecorded {
            native_child_id,
            native_child_spawn_admission_id,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, native_child_spawn_admission_id.value());
        }
        EventBody::PiAdapterReadyRecorded {
            native_child_id,
            pi_session_id,
        }
        | EventBody::PiSessionReadyRecorded {
            native_child_id,
            pi_session_id,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, pi_session_id.value());
        }
        EventBody::PiAbortControlDeliveryRecorded {
            pi_abort_control_receipt_id,
            native_child_id,
            cancellation_propagation_id,
            correlation_identity,
            abort_command_digest,
            outcome,
        } => {
            put_i64(&mut bytes, pi_abort_control_receipt_id.value());
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, cancellation_propagation_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_bytes(&mut bytes, &abort_command_digest.as_bytes());
            put_i64(&mut bytes, *outcome as i64);
        }
        EventBody::PiCreateSessionAuthorized { native_child_id }
        | EventBody::PiCreateSessionDeliveryRecorded { native_child_id } => {
            put_i64(&mut bytes, native_child_id.value())
        }
        EventBody::ChildStreamSealed {
            native_child_stream_seal_id,
            native_child_id,
            stream_kind,
            completeness,
        } => {
            put_i64(&mut bytes, native_child_stream_seal_id.value());
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, *stream_kind as i64);
            put_i64(&mut bytes, *completeness as i64);
        }
        EventBody::ChildProcessLivenessObserved {
            native_child_liveness_observation_id,
            native_child_id,
            liveness,
        } => {
            put_i64(&mut bytes, native_child_liveness_observation_id.value());
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, *liveness as i64);
        }
        EventBody::ProcessSignalReceiptRecorded {
            process_signal_receipt_id,
            native_child_id,
            action,
            delivery,
            observed_liveness,
            cause,
        } => {
            put_i64(&mut bytes, process_signal_receipt_id.value());
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, *action as i64);
            put_i64(&mut bytes, *delivery as i64);
            put_i64(&mut bytes, *observed_liveness as i64);
            put_process_signal_cause(&mut bytes, *cause);
        }
        EventBody::DirectChildReaped {
            native_child_reap_receipt_id,
            native_child_id,
            wait_status,
            group_liveness_before_cleanup,
            group_liveness_after_cleanup,
        } => {
            put_i64(&mut bytes, native_child_reap_receipt_id.value());
            put_i64(&mut bytes, native_child_id.value());
            put_direct_wait_status(&mut bytes, *wait_status);
            put_i64(&mut bytes, *group_liveness_before_cleanup as i64);
            put_i64(&mut bytes, *group_liveness_after_cleanup as i64);
        }
        EventBody::ChildRecoveryObserved {
            native_child_recovery_receipt_id,
            native_child_id,
            observation,
            group_liveness_after_restart,
        } => {
            put_i64(&mut bytes, native_child_recovery_receipt_id.value());
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, *observation as i64);
            put_i64(&mut bytes, *group_liveness_after_restart as i64);
        }
        EventBody::ChildProcessFinalized {
            native_child_id,
            disposition,
        } => {
            put_i64(&mut bytes, native_child_id.value());
            put_i64(&mut bytes, *disposition as i64);
        }
        EventBody::CancellationPropagationBegun {
            cancellation_propagation_id,
            cancellation_request_id,
        } => {
            put_i64(&mut bytes, cancellation_propagation_id.value());
            put_i64(&mut bytes, cancellation_request_id.value());
        }
        EventBody::CancellationPropagationReconciled {
            cancellation_propagation_id,
        } => put_i64(&mut bytes, cancellation_propagation_id.value()),
        EventBody::SupervisorEpochOpened {
            supervisor_epoch_id,
        } => put_i64(&mut bytes, supervisor_epoch_id.value()),
        EventBody::CancellationPropagationContainmentFailed {
            cancellation_propagation_id,
        } => put_i64(&mut bytes, cancellation_propagation_id.value()),
        EventBody::NativeChildSpawnInvalidated {
            native_child_spawn_admission_id,
            reason,
        } => {
            put_i64(&mut bytes, native_child_spawn_admission_id.value());
            put_i64(&mut bytes, *reason as i64);
        }
        EventBody::PiOfficeTurnPromptAuthorized {
            pi_office_turn_prompt_authorization_id,
            office_turn_id,
            native_child_id,
            correlation_identity,
            budget_reservation_id,
        } => {
            put_i64(&mut bytes, pi_office_turn_prompt_authorization_id.value());
            put_i64(&mut bytes, office_turn_id.value());
            put_i64(&mut bytes, native_child_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, budget_reservation_id.value());
        }
        EventBody::PiOfficeTurnPromptDelivered {
            office_turn_id,
            correlation_identity,
        } => {
            put_i64(&mut bytes, office_turn_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
        }
        EventBody::PiOfficeTurnPromptAccepted {
            office_turn_id,
            correlation_identity,
            command_result_sequence,
        } => {
            put_i64(&mut bytes, office_turn_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, command_result_sequence.value());
        }
        EventBody::PiOfficeTurnUsageRecorded {
            pi_office_turn_usage_receipt_id,
            office_turn_id,
            protocol_sequence,
            cumulative_micro_usd,
        } => {
            put_i64(&mut bytes, pi_office_turn_usage_receipt_id.value());
            put_i64(&mut bytes, office_turn_id.value());
            put_i64(&mut bytes, protocol_sequence.value());
            put_i64(&mut bytes, cumulative_micro_usd.value());
        }
        EventBody::PiOfficeTurnUsageFrozen {
            office_turn_id,
            budget_reservation_id,
            cancellation_request_id,
            postmortem_id,
            failure,
        } => {
            put_i64(&mut bytes, office_turn_id.value());
            put_i64(&mut bytes, budget_reservation_id.value());
            put_i64(&mut bytes, cancellation_request_id.value());
            put_i64(&mut bytes, postmortem_id.value());
            put_pi_office_turn_usage_failure(&mut bytes, *failure);
        }
        EventBody::PiOfficeTurnTerminalRecorded {
            pi_office_turn_terminal_receipt_id,
            office_turn_id,
            disposition,
            assistant_outcome,
        } => {
            put_i64(&mut bytes, pi_office_turn_terminal_receipt_id.value());
            put_i64(&mut bytes, office_turn_id.value());
            put_i64(&mut bytes, *disposition as i64);
            put_i64(&mut bytes, *assistant_outcome as i64);
        }
        EventBody::PiOfficeSessionDisposeAuthorized {
            session_id,
            native_child_id,
            correlation_identity,
            authorized_generation,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, native_child_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, authorized_generation.value());
        }
        EventBody::PiOfficeSessionDisposeDelivered {
            session_id,
            native_child_id,
            correlation_identity,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, native_child_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
        }
        EventBody::PiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity,
            command_result_sequence,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_bytes(&mut bytes, correlation_identity.as_str().as_bytes());
            put_i64(&mut bytes, command_result_sequence.value());
        }
        EventBody::PiOfficeSessionDisposeUsageRecorded {
            session_id,
            protocol_sequence,
            cumulative_micro_usd,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, protocol_sequence.value());
            put_i64(&mut bytes, cumulative_micro_usd.value());
        }
        EventBody::PiOfficeSessionDisposeUsageFrozen {
            session_id,
            budget_reservation_id,
            cancellation_request_id,
            postmortem_id,
            failure,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, budget_reservation_id.value());
            put_i64(&mut bytes, cancellation_request_id.value());
            put_i64(&mut bytes, postmortem_id.value());
            put_pi_office_turn_usage_failure(&mut bytes, *failure);
        }
        EventBody::PiOfficeSessionDisposed {
            pi_office_session_dispose_receipt_id,
            session_id,
            budget_reservation_id,
            observed_cumulative_micro_usd,
            budget_disposition,
        } => {
            put_i64(&mut bytes, pi_office_session_dispose_receipt_id.value());
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, budget_reservation_id.value());
            put_i64(&mut bytes, observed_cumulative_micro_usd.value());
            put_pi_office_session_dispose_budget_disposition(&mut bytes, *budget_disposition);
        }
    }
    Blake3Digest::of_bytes(&bytes)
}

fn put_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    put_i64(bytes, value.len() as i64);
    bytes.extend_from_slice(value);
}

fn put_i64(bytes: &mut Vec<u8>, value: i64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn put_optional_i64(bytes: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            put_i64(bytes, 1);
            put_i64(bytes, value);
        }
        None => put_i64(bytes, 0),
    }
}

fn put_pi_cumulative_usage(bytes: &mut Vec<u8>, usage: PiCumulativeUsage) {
    put_i64(bytes, usage.input_tokens.value());
    put_i64(bytes, usage.output_tokens.value());
    put_i64(bytes, usage.cache_read_tokens.value());
    put_i64(bytes, usage.cache_write_tokens.value());
    put_i64(bytes, usage.total_tokens.value());
    put_bytes(bytes, &usage.provider_cost.as_big_endian_bytes());
    put_i64(bytes, usage.ceiling_micro_usd.value());
}

fn put_pi_office_turn_usage_failure(bytes: &mut Vec<u8>, failure: PiOfficeTurnUsageFailure) {
    match failure {
        PiOfficeTurnUsageFailure::Unknown(reason) => {
            put_i64(bytes, 1);
            put_i64(bytes, reason as i64);
        }
        PiOfficeTurnUsageFailure::Unavailable(reason) => {
            put_i64(bytes, 2);
            put_i64(bytes, reason as i64);
        }
    }
}

fn put_pi_office_session_transcript_receipt(
    bytes: &mut Vec<u8>,
    receipt: &PiOfficeSessionTranscriptReceipt,
) {
    match receipt {
        PiOfficeSessionTranscriptReceipt::Materialized {
            session_file,
            session_file_digest,
            transcript_content_object_id,
            first_user_prompt,
        } => {
            put_i64(bytes, 1);
            put_bytes(bytes, session_file.as_str().as_bytes());
            put_bytes(bytes, session_file_digest.as_bytes().as_slice());
            put_i64(bytes, transcript_content_object_id.value());
            match first_user_prompt {
                PiOfficeSessionFirstUserPromptReceipt::Absent => put_i64(bytes, 1),
                PiOfficeSessionFirstUserPromptReceipt::Verified { digest } => {
                    put_i64(bytes, 2);
                    put_bytes(bytes, digest.as_bytes().as_slice());
                }
            }
        }
        PiOfficeSessionTranscriptReceipt::UnmaterializedNoPrompt { session_file } => {
            put_i64(bytes, 2);
            put_bytes(bytes, session_file.as_str().as_bytes());
        }
    }
}

fn put_pi_office_session_dispose_budget_disposition(
    bytes: &mut Vec<u8>,
    disposition: PiOfficeSessionDisposeBudgetDisposition,
) {
    match disposition {
        PiOfficeSessionDisposeBudgetDisposition::Reconciled {
            observed_cumulative_micro_usd,
        } => {
            put_i64(bytes, 1);
            put_i64(bytes, observed_cumulative_micro_usd.value());
        }
        PiOfficeSessionDisposeBudgetDisposition::Frozen {
            cancellation_request_id,
            postmortem_id,
        } => {
            put_i64(bytes, 2);
            put_i64(bytes, cancellation_request_id.value());
            put_i64(bytes, postmortem_id.value());
        }
    }
}

fn sql_pi_office_turn_usage_failure(
    failure: PiOfficeTurnUsageFailure,
) -> (i64, Option<i64>, Option<i64>) {
    match failure {
        PiOfficeTurnUsageFailure::Unknown(reason) => (1, Some(reason as i64), None),
        PiOfficeTurnUsageFailure::Unavailable(reason) => (2, None, Some(reason as i64)),
    }
}

fn sql_pi_office_turn_terminal_evidence(
    evidence: PiOfficeTurnTerminalEvidence,
) -> (i64, Option<i64>) {
    match evidence {
        PiOfficeTurnTerminalEvidence::ObservedAssistant {
            agent_settled_sequence,
            ..
        } => (1, Some(agent_settled_sequence.value())),
        PiOfficeTurnTerminalEvidence::UnavailableAssistant { .. } => (2, None),
    }
}

fn put_pi_office_turn_terminal_evidence(
    bytes: &mut Vec<u8>,
    evidence: PiOfficeTurnTerminalEvidence,
) {
    match evidence {
        PiOfficeTurnTerminalEvidence::ObservedAssistant {
            agent_settled_sequence,
            final_accounting_sequence,
        } => {
            put_i64(bytes, 1);
            put_i64(bytes, agent_settled_sequence.value());
            put_i64(bytes, final_accounting_sequence.value());
        }
        PiOfficeTurnTerminalEvidence::UnavailableAssistant {
            final_known_usage_sequence,
        } => {
            put_i64(bytes, 2);
            put_i64(bytes, final_known_usage_sequence.value());
        }
    }
}

fn put_direct_wait_status(bytes: &mut Vec<u8>, status: DirectChildWaitStatus) {
    match status {
        DirectChildWaitStatus::Exited { exit_code } => {
            put_i64(bytes, 1);
            put_i64(bytes, i64::from(exit_code.value()));
        }
        DirectChildWaitStatus::Signaled { signal_number } => {
            put_i64(bytes, 2);
            put_i64(bytes, i64::from(signal_number.value()));
        }
        DirectChildWaitStatus::Unknown => put_i64(bytes, 3),
    }
}

fn put_process_signal_cause(bytes: &mut Vec<u8>, cause: ProcessSignalCause) {
    match cause {
        ProcessSignalCause::CancellationPropagation(propagation_id) => {
            put_i64(bytes, 1);
            put_i64(bytes, propagation_id.value());
        }
        ProcessSignalCause::AutomaticBoundaryContainment => {
            put_i64(bytes, 2);
            put_i64(bytes, 0);
        }
    }
}

fn insert_command_body(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    body: &CommandBody,
    source_content_object_id: Option<ContentObjectId>,
) -> Result<(), StoreError> {
    match body {
        CommandBody::CreateSocietyIdentity { name } => {
            transaction.execute(
                "INSERT INTO command_create_society_identity(command_row_id, name) VALUES (?1, ?2)",
                params![command_row_id, name.as_str()],
            )?;
        }
        CommandBody::RecordPiAbortControlDelivery {
            native_child_id,
            cancellation_propagation_id,
            correlation_identity,
            abort_command_digest,
            outcome,
        } => {
            transaction.execute(
                "INSERT INTO command_record_pi_abort_control_delivery VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![command_row_id, native_child_id.value(), cancellation_propagation_id.value(), correlation_identity.as_str(), abort_command_digest.as_bytes().as_slice(), *outcome as i64],
            )?;
        }
        CommandBody::InstallRootAuthorityOffice => {
            transaction.execute(
                "INSERT INTO command_install_root_authority_office(command_row_id) VALUES (?1)",
                [command_row_id],
            )?;
        }
        CommandBody::InstallFoundingMission { mission } => {
            transaction.execute(
                "INSERT INTO command_install_founding_mission(
                     command_row_id, application_identity, application_name,
                     revision_ordinal, mission_statement, source_rendering_digest,
                     source_content_object_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    command_row_id,
                    mission.application_identity.as_str(),
                    mission.application_name.as_str(),
                    mission.revision_ordinal.value(),
                    mission.statement.as_str(),
                    mission.source_rendering_digest.as_bytes().as_slice(),
                    source_content_object_id.map(ContentObjectId::value),
                ],
            )?;
            for (index, principle) in mission.principles.as_slice().iter().enumerate() {
                transaction.execute(
                    "INSERT INTO command_install_founding_mission_principles(
                         command_row_id, principle_ordinal, principle_kind, principle_text
                     ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        command_row_id,
                        i64::try_from(index + 1).map_err(|_| StoreError::InvalidStoredValue)?,
                        principle.kind as i64,
                        principle.text.as_str(),
                    ],
                )?;
            }
            let questions = &mission.north_star_questions;
            transaction.execute(
                "INSERT INTO command_install_founding_mission_north_star_questions(
                     command_row_id, change_question, improvement_evidence_question,
                     boundary_commitment_question, revisit_question
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    command_row_id,
                    questions.change.as_str(),
                    questions.improvement_evidence.as_str(),
                    questions.boundary_commitment.as_str(),
                    questions.revisit.as_str(),
                ],
            )?;
        }
        CommandBody::AppointInitialRootAuthority { actor_display_name } => {
            transaction.execute("INSERT INTO command_appoint_initial_root_authority(command_row_id, actor_display_name) VALUES (?1, ?2)", params![command_row_id, actor_display_name.as_str()])?;
        }
        CommandBody::SetR0HardCeiling { ceiling } => {
            transaction.execute("INSERT INTO command_set_r0_hard_ceiling(command_row_id, ceiling_micros) VALUES (?1, ?2)", params![command_row_id, ceiling.value()])?;
        }
        CommandBody::BootstrapSociety => {
            transaction.execute(
                "INSERT INTO command_bootstrap_society(command_row_id) VALUES (?1)",
                [command_row_id],
            )?;
        }
        CommandBody::ProposeOperatingCycle {
            treatment,
            budget_ceiling,
        } => {
            transaction.execute("INSERT INTO command_propose_operating_cycle(command_row_id, treatment, budget_ceiling_micros) VALUES (?1, ?2, ?3)", params![command_row_id, *treatment as i64, budget_ceiling.value()])?;
        }
        CommandBody::AdmitOperatingCycle { cycle_id } => {
            transaction.execute("INSERT INTO command_admit_operating_cycle(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::StartRootAuthorityOfficeSession { cycle_id } => {
            transaction.execute("INSERT INTO command_start_root_authority_office_session(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::RecordOfficeSessionReady { session_id } => {
            transaction.execute("INSERT INTO command_record_office_session_ready(command_row_id, root_authority_office_session_id) VALUES (?1, ?2)", params![command_row_id, session_id.value()])?;
        }
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state,
        } => {
            transaction.execute("INSERT INTO command_record_office_session_terminal(command_row_id, root_authority_office_session_id, terminal_state) VALUES (?1, ?2, ?3)", params![command_row_id, session_id.value(), *terminal_state as i64])?;
        }
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose,
        } => {
            transaction.execute("INSERT INTO command_open_office_turn(command_row_id, root_authority_office_session_id, purpose) VALUES (?1, ?2, ?3)", params![command_row_id, session_id.value(), *purpose as i64])?;
        }
        CommandBody::SettleOfficeTurn {
            turn_id,
            terminal_receipt_id,
        } => {
            transaction.execute("INSERT INTO command_settle_office_turn(command_row_id, office_turn_id, pi_office_turn_terminal_receipt_id) VALUES (?1, ?2, ?3)", params![command_row_id, turn_id.value(), terminal_receipt_id.value()])?;
        }
        CommandBody::QuiesceOperatingCycle { cycle_id } => {
            transaction.execute("INSERT INTO command_quiesce_operating_cycle(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::RecordCycleDrained { cycle_id } => {
            transaction.execute("INSERT INTO command_record_cycle_drained(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::ResumeOperatingCycle { cycle_id } => {
            transaction.execute("INSERT INTO command_resume_operating_cycle(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::ReconcileOperatingCycle { cycle_id } => {
            transaction.execute("INSERT INTO command_reconcile_operating_cycle(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::CloseOperatingCycle { cycle_id } => {
            transaction.execute("INSERT INTO command_close_operating_cycle(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::ReserveBudget { cycle_id, amount } => {
            transaction.execute("INSERT INTO command_reserve_budget(command_row_id, operating_cycle_id, amount_micros) VALUES (?1, ?2, ?3)", params![command_row_id, cycle_id.value(), amount.value()])?;
        }
        CommandBody::ReconcileBudget {
            reservation_id,
            observation,
        } => {
            let (kind, known, unknown, unavailable): (i64, Option<i64>, Option<i64>, Option<i64>) =
                match observation {
                    CostObservation::Known(amount) => (1, Some(amount.value()), None, None),
                    CostObservation::Unknown(reason) => (2, None, Some(*reason as i64), None),
                    CostObservation::Unavailable(reason) => (3, None, None, Some(*reason as i64)),
                };
            transaction.execute("INSERT INTO command_reconcile_budget(command_row_id, budget_reservation_id, observation_kind, known_micros, unknown_reason, unavailable_reason) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![command_row_id, reservation_id.value(), kind, known, unknown, unavailable])?;
        }
        CommandBody::RequestCancellation { cycle_id, mode } => {
            transaction.execute("INSERT INTO command_request_cancellation(command_row_id, operating_cycle_id, cancellation_mode) VALUES (?1, ?2, ?3)", params![command_row_id, cycle_id.value(), *mode as i64])?;
        }
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        } => {
            transaction.execute("INSERT INTO command_reconcile_cancellation(command_row_id, cancellation_request_id) VALUES (?1, ?2)", params![command_row_id, cancellation_request_id.value()])?;
        }
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution,
        } => {
            transaction.execute("INSERT INTO command_close_cost_postmortem(command_row_id, postmortem_id, resolution_kind) VALUES (?1, ?2, ?3)", params![command_row_id, postmortem_id.value(), *resolution as i64])?;
        }
        CommandBody::CreateProject {
            operating_cycle_id,
            project_name,
            north_star_alignment,
        } => {
            transaction.execute(
                "INSERT INTO command_create_project VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_name.as_str(),
                    north_star_alignment.application_revision_id.value(),
                    north_star_alignment.change_answer.as_str(),
                    north_star_alignment.improvement_evidence_answer.as_str(),
                    north_star_alignment.boundary_commitment_answer.as_str(),
                    north_star_alignment.revisit_answer.as_str(),
                ],
            )?;
        }
        CommandBody::AuthorizePiOfficeTurnPrompt {
            office_turn_id,
            correlation_identity,
            prompt_content_object_id,
            prompt_digest,
            frontier_event_id,
        } => {
            transaction.execute(
                "INSERT INTO command_authorize_pi_office_turn_prompt VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![command_row_id, office_turn_id.value(), correlation_identity.as_str(), prompt_content_object_id.value(), prompt_digest.as_bytes().as_slice(), frontier_event_id.value()],
            )?;
        }
        CommandBody::RecordPiOfficeTurnPromptDelivery {
            office_turn_id,
            correlation_identity,
            prompt_digest,
        } => {
            transaction.execute(
                "INSERT INTO command_record_pi_office_turn_prompt_delivery VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    office_turn_id.value(),
                    correlation_identity.as_str(),
                    prompt_digest.as_bytes().as_slice()
                ],
            )?;
        }
        CommandBody::RecordPiOfficeTurnPromptAccepted {
            office_turn_id,
            correlation_identity,
            command_result_sequence,
        } => {
            transaction.execute(
                "INSERT INTO command_record_pi_office_turn_prompt_accepted VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    office_turn_id.value(),
                    correlation_identity.as_str(),
                    command_result_sequence.value()
                ],
            )?;
        }
        CommandBody::RecordPiOfficeTurnUsage {
            office_turn_id,
            correlation_identity,
            protocol_sequence,
            usage,
        } => {
            transaction.execute(
                "INSERT INTO command_record_pi_office_turn_usage VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![command_row_id, office_turn_id.value(), correlation_identity.as_str(), protocol_sequence.value(), usage.input_tokens.value(), usage.output_tokens.value(), usage.cache_read_tokens.value(), usage.cache_write_tokens.value(), usage.total_tokens.value(), usage.provider_cost.as_big_endian_bytes().as_slice(), usage.ceiling_micro_usd.value()],
            )?;
        }
        CommandBody::RecordPiOfficeTurnUsageFailure {
            office_turn_id,
            correlation_identity,
            protocol_sequence,
            failure,
        } => {
            let (kind, unknown, unavailable) = sql_pi_office_turn_usage_failure(*failure);
            transaction.execute(
                "INSERT INTO command_record_pi_office_turn_usage_failure VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![command_row_id, office_turn_id.value(), correlation_identity.as_str(), protocol_sequence.value(), kind, unknown, unavailable],
            )?;
        }
        CommandBody::RecordPiOfficeTurnTerminal {
            office_turn_id,
            correlation_identity,
            terminal_evidence,
            settled_sequence,
            disposition,
            assistant_outcome,
            transcript_disposition,
        } => {
            let (evidence_kind, agent_settled_sequence) =
                sql_pi_office_turn_terminal_evidence(*terminal_evidence);
            transaction.execute(
                "INSERT INTO command_record_pi_office_turn_terminal VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![command_row_id, office_turn_id.value(), correlation_identity.as_str(), evidence_kind, agent_settled_sequence, terminal_evidence.final_accounting_sequence().value(), settled_sequence.value(), *disposition as i64, *assistant_outcome as i64, *transcript_disposition as i64],
            )?;
        }
        CommandBody::AuthorizePiOfficeSessionDispose {
            session_id,
            correlation_identity,
        } => {
            transaction.execute(
                "INSERT INTO command_authorize_pi_office_session_dispose VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    session_id.value(),
                    correlation_identity.as_str()
                ],
            )?;
        }
        CommandBody::RecordPiOfficeSessionDisposeDelivery {
            session_id,
            correlation_identity,
        } => {
            transaction.execute(
                "INSERT INTO command_record_pi_office_session_dispose_delivery VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    session_id.value(),
                    correlation_identity.as_str()
                ],
            )?;
        }
        CommandBody::RecordPiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity,
            command_result_sequence,
        } => {
            transaction.execute(
                "INSERT INTO command_record_pi_office_session_dispose_accepted VALUES (?1, ?2, ?3, ?4)",
                params![command_row_id, session_id.value(), correlation_identity.as_str(), command_result_sequence.value()],
            )?;
        }
        CommandBody::RecordPiOfficeSessionDisposeUsage {
            session_id,
            correlation_identity,
            protocol_sequence,
            usage,
        } => {
            transaction.execute(
                "INSERT INTO command_record_pi_office_session_dispose_usage VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![command_row_id, session_id.value(), correlation_identity.as_str(), protocol_sequence.value(), usage.input_tokens.value(), usage.output_tokens.value(), usage.cache_read_tokens.value(), usage.cache_write_tokens.value(), usage.total_tokens.value(), usage.provider_cost.as_big_endian_bytes().as_slice(), usage.ceiling_micro_usd.value()],
            )?;
        }
        CommandBody::RecordPiOfficeSessionDisposeUsageFailure {
            session_id,
            correlation_identity,
            protocol_sequence,
            failure,
        } => {
            let (kind, unknown, unavailable) = sql_pi_office_turn_usage_failure(*failure);
            transaction.execute(
                "INSERT INTO command_record_pi_office_session_dispose_usage_failure VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![command_row_id, session_id.value(), correlation_identity.as_str(), protocol_sequence.value(), kind, unknown, unavailable],
            )?;
        }
        CommandBody::RecordPiOfficeSessionDisposed {
            session_id,
            correlation_identity,
            disposed_sequence,
            transcript_receipt,
        } => {
            let (kind, session_file, digest, content, first_kind, first_digest) =
                transcript_receipt_sql_values(transcript_receipt);
            transaction.execute(
                "INSERT INTO command_record_pi_office_session_disposed VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![command_row_id, session_id.value(), correlation_identity.as_str(), disposed_sequence.value(), kind, session_file, digest, content, first_kind, first_digest],
            )?;
        }
        CommandBody::RecordContentSealReceipt { digest } => {
            transaction.execute(
                "INSERT INTO command_record_content_seal_receipt VALUES (?1, ?2)",
                params![command_row_id, digest.as_bytes().as_slice()],
            )?;
        }
        CommandBody::RegisterContentObject {
            content_seal_receipt_id,
        } => {
            transaction.execute(
                "INSERT INTO command_register_content_object VALUES (?1, ?2)",
                params![command_row_id, content_seal_receipt_id.value()],
            )?;
        }
        CommandBody::RegisterForensicManifest {
            operating_cycle_id,
            producing_deterministic_experiment_id,
            capture_policy,
            retention_access_class,
            evaluator_output_content_object_id,
        } => {
            transaction.execute(
                "INSERT INTO command_register_forensic_manifest VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    producing_deterministic_experiment_id.value(),
                    *capture_policy as i64,
                    *retention_access_class as i64,
                    evaluator_output_content_object_id.value()
                ],
            )?;
        }
        CommandBody::RegisterDeterministicEvaluatorForensicManifest {
            operating_cycle_id,
            native_child_spawn_admission_id,
        } => {
            transaction.execute(
                "INSERT INTO command_register_deterministic_evaluator_forensic_manifest VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    native_child_spawn_admission_id.value(),
                ],
            )?;
        }
        CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id,
            project_id,
            ticket_id,
            target_graph_revision_id,
            evaluator_content_object_id,
            input_manifest_content_object_id,
        } => {
            transaction.execute("INSERT INTO command_register_deterministic_experiment VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![command_row_id, operating_cycle_id.value(), project_id.value(), ticket_id.value(), target_graph_revision_id.value(), evaluator_content_object_id.value(), input_manifest_content_object_id.value()])?;
        }
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            forensic_manifest_id,
            evaluator_output_content_object_id,
        } => {
            transaction.execute("INSERT INTO command_record_deterministic_evaluation_receipt VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![command_row_id, operating_cycle_id.value(), deterministic_experiment_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), forensic_manifest_id.value(), evaluator_output_content_object_id.value()])?;
        }
        CommandBody::AdmitDeterministicEvidence {
            operating_cycle_id,
            deterministic_evaluation_receipt_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            evaluator_output_content_object_id,
            related_graph_revision_id,
            semantic_role,
            applicability,
            limitation,
        } => {
            transaction.execute("INSERT INTO command_admit_deterministic_evidence VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", params![command_row_id, operating_cycle_id.value(), deterministic_evaluation_receipt_id.value(), deterministic_experiment_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), evaluator_output_content_object_id.value(), related_graph_revision_id.value(), *semantic_role as i64, *applicability as i64, limitation.as_str()])?;
        }
        CommandBody::FinalizeDeterministicExperiment {
            operating_cycle_id,
            deterministic_experiment_id,
        } => {
            transaction.execute(
                "INSERT INTO command_finalize_deterministic_experiment VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    deterministic_experiment_id.value()
                ],
            )?;
        }
        CommandBody::AdmitDeterministicEvaluatorNativeChild {
            operating_cycle_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            execution_profile_id,
            native_workspace_id,
            canonical_workspace_path,
            supervisor_epoch_id,
            supervisor_epoch_identity,
        } => {
            transaction.execute(
                "INSERT INTO command_admit_deterministic_evaluator_native_child VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![command_row_id, operating_cycle_id.value(), deterministic_experiment_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), execution_profile_id.value(), native_workspace_id.as_str(), canonical_workspace_path.as_str(), supervisor_epoch_id.value(), supervisor_epoch_identity.as_str()],
            )?;
        }
        CommandBody::RecordDeterministicEvaluatorNativeChildSpawn {
            native_child_spawn_admission_id,
            child_identity,
            direct_child_pid,
            process_group_id,
        } => {
            transaction.execute(
                "INSERT INTO command_record_deterministic_evaluator_native_child_spawn VALUES (?1, ?2, ?3, ?4, ?5)",
                params![command_row_id, native_child_spawn_admission_id.value(), child_identity.as_str(), direct_child_pid.value(), process_group_id.value()],
            )?;
        }
        CommandBody::RegisterActorConfiguration {
            configuration_name,
            model_policy,
            primary_attractor,
        } => {
            transaction.execute(
                "INSERT INTO command_register_actor_configuration VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    configuration_name.as_str(),
                    *model_policy as i64,
                    *primary_attractor as i64
                ],
            )?;
        }
        CommandBody::RegisterContextPack {
            operating_cycle_id,
            purpose,
            rendering_digest,
        } => {
            transaction.execute(
                "INSERT INTO command_register_context_pack VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    *purpose as i64,
                    rendering_digest.as_bytes().as_slice()
                ],
            )?;
        }
        CommandBody::AdmitActorInstance {
            operating_cycle_id,
            actor_configuration_revision_id,
            execution_profile_id,
            actor_display_name,
        } => {
            transaction.execute(
                "INSERT INTO command_admit_actor_instance VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    actor_configuration_revision_id.value(),
                    execution_profile_id.value(),
                    actor_display_name.as_str()
                ],
            )?;
        }
        CommandBody::AdmitTicket {
            operating_cycle_id,
            ticket_id,
        } => {
            transaction.execute(
                "INSERT INTO command_admit_ticket VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    ticket_id.value()
                ],
            )?;
        }
        CommandBody::RegisterWorkItem {
            operating_cycle_id,
            ticket_id,
            actor_instance_id,
            context_pack_id,
            work_kind,
            adversarial_review_id,
            assignment,
        } => {
            transaction.execute(
                "INSERT INTO command_register_work_item VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    ticket_id.value(),
                    actor_instance_id.value(),
                    context_pack_id.value(),
                    *work_kind as i64,
                    adversarial_review_id.map(AdversarialReviewId::value),
                    assignment.as_str()
                ],
            )?;
        }
        CommandBody::ClaimWorkItem {
            operating_cycle_id,
            work_item_id,
        } => {
            transaction.execute(
                "INSERT INTO command_claim_work_item VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    work_item_id.value()
                ],
            )?;
        }
        CommandBody::StartActorAttempt {
            operating_cycle_id,
            work_item_id,
            reservation_amount,
        } => {
            transaction.execute(
                "INSERT INTO command_start_actor_attempt VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    work_item_id.value(),
                    reservation_amount.value()
                ],
            )?;
        }
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id,
            terminal_kind,
        } => {
            transaction.execute(
                "INSERT INTO command_attest_actor_attempt_terminal VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    actor_attempt_id.value(),
                    *terminal_kind as i64
                ],
            )?;
        }
        CommandBody::ValidateTicketAttempt {
            operating_cycle_id,
            actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO command_validate_ticket_attempt VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    actor_attempt_id.value()
                ],
            )?;
        }
        CommandBody::RetryActorAttempt {
            operating_cycle_id,
            actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO command_retry_actor_attempt VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    actor_attempt_id.value()
                ],
            )?;
        }
        CommandBody::CompleteTicket {
            operating_cycle_id,
            actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO command_complete_ticket VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    actor_attempt_id.value()
                ],
            )?;
        }
        CommandBody::ExpireWorkLease { work_lease_id } => {
            transaction.execute(
                "INSERT INTO command_expire_work_lease VALUES (?1, ?2)",
                params![command_row_id, work_lease_id.value()],
            )?;
        }
        CommandBody::CancelActorAttempt {
            actor_attempt_id,
            reason,
        } => {
            transaction.execute(
                "INSERT INTO command_cancel_actor_attempt VALUES (?1, ?2, ?3)",
                params![command_row_id, actor_attempt_id.value(), *reason as i64],
            )?;
        }
        CommandBody::RegisterOutcomeObligation {
            operating_cycle_id,
            project_id,
            obligation,
        } => {
            transaction.execute(
                "INSERT INTO command_register_outcome_obligation VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    obligation.as_str()
                ],
            )?;
        }
        CommandBody::ResolveOutcomeObligation {
            operating_cycle_id,
            outcome_obligation_id,
            disposition,
        } => {
            transaction.execute(
                "INSERT INTO command_resolve_outcome_obligation VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    outcome_obligation_id.value(),
                    *disposition as i64
                ],
            )?;
        }
        CommandBody::CharterProject {
            operating_cycle_id,
            project_id,
            objective,
            initial_milestone,
            stop_condition,
        } => {
            transaction.execute(
                "INSERT INTO command_charter_project VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    objective.as_str(),
                    initial_milestone.as_str(),
                    stop_condition.as_str()
                ],
            )?;
        }
        CommandBody::TransitionProject {
            operating_cycle_id,
            project_id,
            target,
        } => {
            transaction.execute(
                "INSERT INTO command_transition_project VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    *target as i64
                ],
            )?;
        }
        CommandBody::CompleteProjectMilestone {
            operating_cycle_id,
            project_milestone_id,
        } => {
            transaction.execute(
                "INSERT INTO command_complete_project_milestone VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_milestone_id.value()
                ],
            )?;
        }
        CommandBody::ReopenProject {
            operating_cycle_id,
            project_id,
        } => {
            transaction.execute(
                "INSERT INTO command_reopen_project VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value()
                ],
            )?;
        }
        CommandBody::CreateTicket {
            operating_cycle_id,
            project_id,
            ticket_title,
            acceptance_condition,
            prerequisite_ticket_id,
        } => {
            transaction.execute(
                "INSERT INTO command_create_ticket VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    ticket_title.as_str(),
                    acceptance_condition.as_str(),
                    prerequisite_ticket_id.map(TicketId::value)
                ],
            )?;
        }
        CommandBody::TransitionTicket {
            operating_cycle_id,
            ticket_id,
            target,
        } => {
            transaction.execute(
                "INSERT INTO command_transition_ticket VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    ticket_id.value(),
                    *target as i64
                ],
            )?;
        }
        CommandBody::AddGraphObjectRevision {
            operating_cycle_id,
            project_id,
            causal_episode_id,
            graph_object_id,
            body,
        } => {
            transaction.execute(
                "INSERT INTO command_add_graph_object_revision VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    causal_episode_id.map(CausalEpisodeId::value),
                    graph_object_id.map(GraphObjectId::value)
                ],
            )?;
            match body {
                GraphRevisionBody::Observation { observation } => {
                    transaction.execute(
                        "INSERT INTO command_add_observation_revision VALUES (?1, ?2)",
                        params![command_row_id, observation.as_str()],
                    )?;
                }
                GraphRevisionBody::Hypothesis { hypothesis } => {
                    transaction.execute(
                        "INSERT INTO command_add_hypothesis_revision VALUES (?1, ?2)",
                        params![command_row_id, hypothesis.as_str()],
                    )?;
                }
            }
        }
        CommandBody::CommitGraphRevision {
            operating_cycle_id,
            graph_revision_id,
        } => {
            transaction.execute(
                "INSERT INTO command_commit_graph_revision VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    graph_revision_id.value()
                ],
            )?;
        }
        CommandBody::AddGraphEdge {
            operating_cycle_id,
            project_id,
            from_graph_revision_id,
            to_graph_revision_id,
            edge_kind,
        } => {
            transaction.execute(
                "INSERT INTO command_add_graph_edge VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    from_graph_revision_id.value(),
                    to_graph_revision_id.value(),
                    *edge_kind as i64
                ],
            )?;
        }
        CommandBody::CreateEpisode {
            operating_cycle_id,
            project_id,
        } => {
            transaction.execute(
                "INSERT INTO command_create_episode VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value()
                ],
            )?;
        }
        CommandBody::TransitionEpisode {
            operating_cycle_id,
            causal_episode_id,
            target,
        } => {
            transaction.execute(
                "INSERT INTO command_transition_episode VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    causal_episode_id.value(),
                    *target as i64
                ],
            )?;
        }
        CommandBody::ReopenEpisode {
            operating_cycle_id,
            causal_episode_id,
        } => {
            transaction.execute(
                "INSERT INTO command_reopen_episode VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    causal_episode_id.value()
                ],
            )?;
        }
        CommandBody::RequestAdversarialReview {
            operating_cycle_id,
            project_id,
            target_graph_revision_id,
        } => {
            transaction.execute(
                "INSERT INTO command_request_adversarial_review VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    target_graph_revision_id.value()
                ],
            )?;
        }
        CommandBody::AssignAdversarialReviewer {
            operating_cycle_id,
            adversarial_review_id,
            reviewer_principal_id,
            reviewer_actor_instance_id,
            reviewer_actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO command_assign_adversarial_reviewer VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    adversarial_review_id.value(),
                    reviewer_principal_id.value(),
                    reviewer_actor_instance_id.value(),
                    reviewer_actor_attempt_id.value()
                ],
            )?;
        }
        CommandBody::SubmitReviewChallenge {
            operating_cycle_id,
            adversarial_review_id,
            target_graph_revision_id,
            author_principal_id,
            severity,
            failure_hypothesis,
        } => {
            transaction.execute(
                "INSERT INTO command_submit_review_challenge VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    adversarial_review_id.value(),
                    target_graph_revision_id.value(),
                    author_principal_id.value(),
                    *severity as i64,
                    failure_hypothesis.as_str()
                ],
            )?;
        }
        CommandBody::RespondToReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            response,
        } => {
            transaction.execute(
                "INSERT INTO command_respond_to_review_challenge VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    review_challenge_id.value(),
                    response.as_str()
                ],
            )?;
        }
        CommandBody::DispositionReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            disposition,
        } => {
            transaction.execute(
                "INSERT INTO command_disposition_review_challenge VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    review_challenge_id.value(),
                    *disposition as i64
                ],
            )?;
        }
        CommandBody::ResolveAdversarialReview {
            operating_cycle_id,
            adversarial_review_id,
            resolution,
        } => {
            transaction.execute(
                "INSERT INTO command_resolve_adversarial_review VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    adversarial_review_id.value(),
                    *resolution as i64
                ],
            )?;
        }
        CommandBody::TriggerPostmortem {
            operating_cycle_id,
            project_id,
            causal_episode_id,
        } => {
            transaction.execute(
                "INSERT INTO command_trigger_postmortem VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    causal_episode_id.map(CausalEpisodeId::value)
                ],
            )?;
        }
        CommandBody::RecordPostmortemCausalClaim {
            operating_cycle_id,
            postmortem_id,
            claim_kind,
            claim,
        } => {
            transaction.execute(
                "INSERT INTO command_record_postmortem_causal_claim VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    postmortem_id.value(),
                    *claim_kind as i64,
                    claim.as_str()
                ],
            )?;
        }
        CommandBody::ProposePostmortemAction {
            operating_cycle_id,
            postmortem_id,
            action_kind,
            action,
        } => {
            transaction.execute(
                "INSERT INTO command_propose_postmortem_action VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    postmortem_id.value(),
                    *action_kind as i64,
                    action.as_str()
                ],
            )?;
        }
        CommandBody::ClosePostmortem {
            operating_cycle_id,
            postmortem_id,
        } => {
            transaction.execute(
                "INSERT INTO command_close_postmortem VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    postmortem_id.value()
                ],
            )?;
        }
        CommandBody::OpenSupervisorEpoch {
            supervisor_epoch_id,
            supervisor_epoch_identity,
        } => {
            transaction.execute(
                "INSERT INTO command_open_supervisor_epoch VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    supervisor_epoch_id.value(),
                    supervisor_epoch_identity.as_str()
                ],
            )?;
        }
        CommandBody::AdmitPiChildSpawn {
            operating_cycle_id,
            owner,
            budget_reservation_id,
            execution_profile_id,
            native_workspace_id,
            canonical_workspace_path,
            supervisor_epoch_id,
            supervisor_epoch_identity,
            pi_session_identity,
            spawn_nonce,
        } => {
            let (attempt, office) = match owner {
                PiChildOwner::ActorAttempt(id) => (Some(id.value()), None),
                PiChildOwner::RootAuthorityOfficeSession(id) => (None, Some(id.value())),
            };
            transaction.execute("INSERT INTO command_admit_pi_child_spawn VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)", params![command_row_id, operating_cycle_id.value(), attempt, office, budget_reservation_id.value(), execution_profile_id.value(), native_workspace_id.as_str(), canonical_workspace_path.as_str(), supervisor_epoch_id.value(), supervisor_epoch_identity.as_str(), pi_session_identity.as_str(), spawn_nonce.as_str()])?;
        }
        CommandBody::RecordInertChildSpawn {
            native_child_spawn_admission_id,
            child_identity,
            direct_child_pid,
            process_group_id,
        } => {
            transaction.execute(
                "INSERT INTO command_record_inert_pi_child_spawn VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    command_row_id,
                    native_child_spawn_admission_id.value(),
                    child_identity.as_str(),
                    direct_child_pid.value(),
                    process_group_id.value()
                ],
            )?;
        }
        CommandBody::RecordPiAdapterReady {
            native_child_id,
            pi_session_identity,
            spawn_nonce,
        } => {
            transaction.execute(
                "INSERT INTO command_record_pi_adapter_ready VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    native_child_id.value(),
                    pi_session_identity.as_str(),
                    spawn_nonce.as_str()
                ],
            )?;
        }
        CommandBody::AuthorizePiCreateSession {
            native_child_id,
            correlation_identity,
            create_request_digest,
        } => {
            transaction.execute(
                "INSERT INTO command_authorize_pi_create_session VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    native_child_id.value(),
                    correlation_identity.as_str(),
                    create_request_digest.as_bytes().as_slice()
                ],
            )?;
        }
        CommandBody::RecordPiCreateSessionDelivery {
            native_child_id,
            correlation_identity,
            create_request_digest,
        } => {
            transaction.execute(
                "INSERT INTO command_record_pi_create_session_delivery VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    native_child_id.value(),
                    correlation_identity.as_str(),
                    create_request_digest.as_bytes().as_slice()
                ],
            )?;
        }
        CommandBody::RecordPiSessionReady {
            native_child_id,
            pi_session_identity,
        } => {
            transaction.execute(
                "INSERT INTO command_record_pi_session_ready VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    native_child_id.value(),
                    pi_session_identity.as_str()
                ],
            )?;
        }
        CommandBody::RecordChildStreamSeal {
            native_child_id,
            stream_kind,
            full_observed_digest,
            retained_content_object_id,
            completeness,
        } => {
            transaction.execute(
                "INSERT INTO command_record_child_stream_seal VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    native_child_id.value(),
                    *stream_kind as i64,
                    full_observed_digest.as_bytes().as_slice(),
                    retained_content_object_id.value(),
                    *completeness as i64
                ],
            )?;
        }
        CommandBody::RecordChildProcessLiveness {
            native_child_id,
            liveness,
        } => {
            transaction.execute(
                "INSERT INTO command_record_child_process_liveness VALUES (?1, ?2, ?3)",
                params![command_row_id, native_child_id.value(), *liveness as i64],
            )?;
        }
        CommandBody::RecordProcessSignalReceipt {
            native_child_id,
            action,
            delivery,
            observed_liveness,
            cause,
        } => {
            let (cause_kind, propagation_id) = signal_cause_parts(*cause);
            transaction.execute("INSERT INTO command_record_process_signal_receipt VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![command_row_id, native_child_id.value(), *action as i64, *delivery as i64, *observed_liveness as i64, cause_kind, propagation_id])?;
        }
        CommandBody::RecordDirectChildReap {
            native_child_id,
            wait_status,
            group_liveness_before_cleanup,
            group_liveness_after_cleanup,
        } => {
            let (kind, value): (i64, Option<i64>) = match wait_status {
                DirectChildWaitStatus::Exited { exit_code } => {
                    (1, Some(i64::from(exit_code.value())))
                }
                DirectChildWaitStatus::Signaled { signal_number } => {
                    (2, Some(i64::from(signal_number.value())))
                }
                DirectChildWaitStatus::Unknown => (3, None),
            };
            transaction.execute(
                "INSERT INTO command_record_direct_child_reap VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    native_child_id.value(),
                    kind,
                    value,
                    *group_liveness_before_cleanup as i64,
                    *group_liveness_after_cleanup as i64
                ],
            )?;
        }
        CommandBody::RecordChildRecovery {
            native_child_id,
            observation,
            group_liveness_after_restart,
        } => {
            transaction.execute(
                "INSERT INTO command_record_child_recovery VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    native_child_id.value(),
                    *observation as i64,
                    *group_liveness_after_restart as i64
                ],
            )?;
        }
        CommandBody::FinalizeChildProcess { native_child_id } => {
            transaction.execute(
                "INSERT INTO command_finalize_child_process VALUES (?1, ?2)",
                params![command_row_id, native_child_id.value()],
            )?;
        }
        CommandBody::BeginCancellationPropagation {
            cancellation_request_id,
        } => {
            transaction.execute(
                "INSERT INTO command_begin_cancellation_propagation VALUES (?1, ?2)",
                params![command_row_id, cancellation_request_id.value()],
            )?;
        }
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id,
        } => {
            transaction.execute(
                "INSERT INTO command_reconcile_cancellation_propagation VALUES (?1, ?2)",
                params![command_row_id, cancellation_propagation_id.value()],
            )?;
        }
        CommandBody::RecordNativeChildNotSpawned {
            native_child_spawn_admission_id,
            reason,
        } => {
            transaction.execute(
                "INSERT INTO command_record_native_child_not_spawned VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    native_child_spawn_admission_id.value(),
                    *reason as i64
                ],
            )?;
        }
    }
    Ok(())
}

fn insert_event(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    command_id: &CommandId,
    body: &EventBody,
) -> Result<EventId, StoreError> {
    let sequence: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(event_sequence), 0) + 1 FROM events",
        [],
        |row| row.get(0),
    )?;
    let event_id = EventId::try_from(transaction.query_row(
        "SELECT COALESCE(MAX(event_id), 0) + 1 FROM events",
        [],
        |row| row.get::<_, i64>(0),
    )?)
    .map_err(|_| StoreError::InvalidStoredValue)?;
    transaction.execute(
        "INSERT INTO events(event_id, command_row_id, event_kind, event_sequence, event_fingerprint)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            event_id.value(),
            command_row_id,
            body.kind() as i64,
            sequence,
            event_fingerprint(event_id, command_id, body)
                .as_bytes()
                .as_slice()
        ],
    )?;
    insert_event_body(transaction, event_id, body)?;
    Ok(event_id)
}

fn insert_event_body(
    transaction: &Transaction<'_>,
    event_id: EventId,
    body: &EventBody,
) -> Result<(), StoreError> {
    match body {
        EventBody::PiOfficeSessionDisposeAuthorized {
            session_id,
            native_child_id,
            correlation_identity,
            authorized_generation,
        } => {
            transaction.execute(
                "INSERT INTO event_pi_office_session_dispose_authorized VALUES (?1, ?2, ?3, ?4, ?5)",
                params![event_id.value(), session_id.value(), native_child_id.value(), correlation_identity.as_str(), authorized_generation.value()],
            )?;
        }
        EventBody::SocietyIdentityCreated { society_id } => {
            transaction.execute(
                "INSERT INTO event_society_identity_created(event_id, society_id) VALUES (?1, ?2)",
                params![event_id.value(), society_id.value()],
            )?;
        }
        EventBody::RootAuthorityOfficeInstalled { office_id } => {
            transaction.execute("INSERT INTO event_root_authority_office_installed(event_id, office_id) VALUES (?1, ?2)", params![event_id.value(), office_id.value()])?;
        }
        EventBody::FoundingMissionInstalled {
            mission_id,
            application_revision_id,
        } => {
            transaction.execute(
                "INSERT INTO event_founding_mission_installed(
                     event_id, founding_mission_id, application_revision_id
                 ) VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    mission_id.value(),
                    application_revision_id.value()
                ],
            )?;
        }
        EventBody::RootAuthorityAppointed {
            occupancy_id,
            principal_id,
        } => {
            transaction.execute("INSERT INTO event_root_authority_appointed(event_id, office_occupancy_id, principal_id) VALUES (?1, ?2, ?3)", params![event_id.value(), occupancy_id.value(), principal_id.value()])?;
        }
        EventBody::R0HardCeilingSet {
            society_id,
            ceiling,
        } => {
            transaction.execute("INSERT INTO event_r0_hard_ceiling_set(event_id, society_id, ceiling_micros) VALUES (?1, ?2, ?3)", params![event_id.value(), society_id.value(), ceiling.value()])?;
        }
        EventBody::SocietyBootstrapped { society_id } => {
            transaction.execute(
                "INSERT INTO event_society_bootstrapped(event_id, society_id) VALUES (?1, ?2)",
                params![event_id.value(), society_id.value()],
            )?;
        }
        EventBody::OperatingCycleProposed {
            cycle_id,
            generation,
            treatment,
            budget_ceiling,
        } => {
            transaction.execute("INSERT INTO event_operating_cycle_proposed(event_id, operating_cycle_id, admission_generation, treatment, budget_ceiling_micros) VALUES (?1, ?2, ?3, ?4, ?5)", params![event_id.value(), cycle_id.value(), generation.value(), *treatment as i64, budget_ceiling.value()])?;
        }
        EventBody::OperatingCycleStateChanged {
            cycle_id,
            state,
            generation,
        } => {
            transaction.execute("INSERT INTO event_operating_cycle_state_changed(event_id, operating_cycle_id, lifecycle_state, admission_generation) VALUES (?1, ?2, ?3, ?4)", params![event_id.value(), cycle_id.value(), *state as i64, generation.value()])?;
        }
        EventBody::RootAuthorityOfficeSessionStarted {
            session_id,
            cycle_id,
        } => {
            transaction.execute("INSERT INTO event_root_authority_office_session_started(event_id, root_authority_office_session_id, operating_cycle_id) VALUES (?1, ?2, ?3)", params![event_id.value(), session_id.value(), cycle_id.value()])?;
        }
        EventBody::RootAuthorityOfficeSessionStateChanged { session_id, state } => {
            transaction.execute("INSERT INTO event_root_authority_office_session_state_changed(event_id, root_authority_office_session_id, lifecycle_state) VALUES (?1, ?2, ?3)", params![event_id.value(), session_id.value(), *state as i64])?;
        }
        EventBody::OfficeTurnOpened {
            turn_id,
            session_id,
            purpose,
        } => {
            transaction.execute("INSERT INTO event_office_turn_opened(event_id, office_turn_id, root_authority_office_session_id, purpose) VALUES (?1, ?2, ?3, ?4)", params![event_id.value(), turn_id.value(), session_id.value(), *purpose as i64])?;
        }
        EventBody::OfficeTurnSettled {
            turn_id,
            session_id,
            charged_delta,
        } => {
            transaction.execute("INSERT INTO event_office_turn_settled(event_id, office_turn_id, root_authority_office_session_id, charged_delta_micros) VALUES (?1, ?2, ?3, ?4)", params![event_id.value(), turn_id.value(), session_id.value(), charged_delta.value()])?;
        }
        EventBody::BudgetReserved {
            reservation_id,
            cycle_id,
            amount,
        } => {
            transaction.execute("INSERT INTO event_budget_reserved(event_id, budget_reservation_id, operating_cycle_id, amount_micros) VALUES (?1, ?2, ?3, ?4)", params![event_id.value(), reservation_id.value(), cycle_id.value(), amount.value()])?;
        }
        EventBody::BudgetReconciled {
            reservation_id,
            observed,
        } => {
            transaction.execute("INSERT INTO event_budget_reconciled(event_id, budget_reservation_id, observed_micros) VALUES (?1, ?2, ?3)", params![event_id.value(), reservation_id.value(), observed.value()])?;
        }
        EventBody::BudgetAdmissionFrozen {
            reservation_id,
            cycle_id,
            cancellation_request_id,
            postmortem_id,
            reason,
        } => {
            let (reason_kind, observed, reserved, unknown, unavailable) =
                budget_freeze_reason_to_sql(*reason);
            transaction.execute("INSERT INTO event_budget_admission_frozen(event_id, budget_reservation_id, operating_cycle_id, cancellation_request_id, postmortem_id, freeze_reason_kind, observed_micros, reserved_micros, unknown_reason, unavailable_reason) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)", params![event_id.value(), reservation_id.value(), cycle_id.value(), cancellation_request_id.value(), postmortem_id.value(), reason_kind, observed, reserved, unknown, unavailable])?;
        }
        EventBody::CancellationRequested {
            cancellation_request_id,
            cycle_id,
            mode,
            generation,
        } => {
            transaction.execute("INSERT INTO event_cancellation_requested(event_id, cancellation_request_id, operating_cycle_id, cancellation_mode, admission_generation) VALUES (?1, ?2, ?3, ?4, ?5)", params![event_id.value(), cancellation_request_id.value(), cycle_id.value(), *mode as i64, generation.value()])?;
        }
        EventBody::CancellationReconciled {
            cancellation_request_id,
            cycle_id,
        } => {
            transaction.execute("INSERT INTO event_cancellation_reconciled(event_id, cancellation_request_id, operating_cycle_id) VALUES (?1, ?2, ?3)", params![event_id.value(), cancellation_request_id.value(), cycle_id.value()])?;
        }
        EventBody::CostPostmortemClosed {
            postmortem_id,
            reservation_id,
            cycle_id,
            resolution,
            charged,
        } => {
            transaction.execute("INSERT INTO event_cost_postmortem_closed(event_id, postmortem_id, budget_reservation_id, operating_cycle_id, resolution_kind, charged_micros) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![event_id.value(), postmortem_id.value(), reservation_id.value(), cycle_id.value(), *resolution as i64, charged.value()])?;
        }
        EventBody::ProjectCreated {
            project_id,
            application_revision_id,
        } => {
            transaction.execute(
                "INSERT INTO event_project_created VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    project_id.value(),
                    application_revision_id.value()
                ],
            )?;
        }
        EventBody::ProjectChartered { project_id } => {
            transaction.execute(
                "INSERT INTO event_project_chartered VALUES (?1, ?2)",
                params![event_id.value(), project_id.value()],
            )?;
        }
        EventBody::ProjectStateChanged { project_id, state } => {
            transaction.execute(
                "INSERT INTO event_project_state_changed VALUES (?1, ?2, ?3)",
                params![event_id.value(), project_id.value(), *state as i64],
            )?;
        }
        EventBody::ProjectMilestoneCompleted {
            project_milestone_id,
        } => {
            transaction.execute(
                "INSERT INTO event_project_milestone_completed VALUES (?1, ?2)",
                params![event_id.value(), project_milestone_id.value()],
            )?;
        }
        EventBody::TicketCreated {
            ticket_id,
            project_id,
        } => {
            transaction.execute(
                "INSERT INTO event_ticket_created VALUES (?1, ?2, ?3)",
                params![event_id.value(), ticket_id.value(), project_id.value()],
            )?;
        }
        EventBody::TicketStateChanged { ticket_id, state } => {
            transaction.execute(
                "INSERT INTO event_ticket_state_changed VALUES (?1, ?2, ?3)",
                params![event_id.value(), ticket_id.value(), *state as i64],
            )?;
        }
        EventBody::GraphObjectRevisionAdded {
            graph_object_id,
            graph_revision_id,
        } => {
            transaction.execute(
                "INSERT INTO event_graph_object_revision_added VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    graph_object_id.value(),
                    graph_revision_id.value()
                ],
            )?;
        }
        EventBody::GraphRevisionCommitted { graph_revision_id } => {
            transaction.execute(
                "INSERT INTO event_graph_revision_committed VALUES (?1, ?2)",
                params![event_id.value(), graph_revision_id.value()],
            )?;
        }
        EventBody::GraphEdgeAdded { graph_edge_id } => {
            transaction.execute(
                "INSERT INTO event_graph_edge_added VALUES (?1, ?2)",
                params![event_id.value(), graph_edge_id.value()],
            )?;
        }
        EventBody::EpisodeCreated {
            causal_episode_id,
            project_id,
        } => {
            transaction.execute(
                "INSERT INTO event_episode_created VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    causal_episode_id.value(),
                    project_id.value()
                ],
            )?;
        }
        EventBody::EpisodeStateChanged {
            causal_episode_id,
            state,
        } => {
            transaction.execute(
                "INSERT INTO event_episode_state_changed VALUES (?1, ?2, ?3)",
                params![event_id.value(), causal_episode_id.value(), *state as i64],
            )?;
        }
        EventBody::AdversarialReviewRequested {
            adversarial_review_id,
        } => {
            transaction.execute(
                "INSERT INTO event_adversarial_review_requested VALUES (?1, ?2)",
                params![event_id.value(), adversarial_review_id.value()],
            )?;
        }
        EventBody::AdversarialReviewerAssigned {
            adversarial_review_id,
            reviewer_principal_id,
            reviewer_actor_instance_id,
            reviewer_actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO event_adversarial_reviewer_assigned VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event_id.value(),
                    adversarial_review_id.value(),
                    reviewer_principal_id.value(),
                    reviewer_actor_instance_id.value(),
                    reviewer_actor_attempt_id.value()
                ],
            )?;
        }
        EventBody::ReviewChallengeSubmitted {
            review_challenge_id,
            author_principal_id,
        } => {
            transaction.execute(
                "INSERT INTO event_review_challenge_submitted VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    review_challenge_id.value(),
                    author_principal_id.value()
                ],
            )?;
        }
        EventBody::ReviewChallengeResponded {
            review_challenge_id,
        } => {
            transaction.execute(
                "INSERT INTO event_review_challenge_responded VALUES (?1, ?2)",
                params![event_id.value(), review_challenge_id.value()],
            )?;
        }
        EventBody::ReviewChallengeDispositioned {
            review_challenge_id,
            disposition,
        } => {
            transaction.execute(
                "INSERT INTO event_review_challenge_dispositioned VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    review_challenge_id.value(),
                    *disposition as i64
                ],
            )?;
        }
        EventBody::AdversarialReviewResolved {
            adversarial_review_id,
            state,
        } => {
            transaction.execute(
                "INSERT INTO event_adversarial_review_resolved VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    adversarial_review_id.value(),
                    *state as i64
                ],
            )?;
        }
        EventBody::PostmortemTriggered { postmortem_id } => {
            transaction.execute(
                "INSERT INTO event_postmortem_triggered VALUES (?1, ?2)",
                params![event_id.value(), postmortem_id.value()],
            )?;
        }
        EventBody::PostmortemCausalClaimRecorded {
            postmortem_causal_claim_id,
        } => {
            transaction.execute(
                "INSERT INTO event_postmortem_causal_claim_recorded VALUES (?1, ?2)",
                params![event_id.value(), postmortem_causal_claim_id.value()],
            )?;
        }
        EventBody::PostmortemActionProposed {
            postmortem_action_proposal_id,
        } => {
            transaction.execute(
                "INSERT INTO event_postmortem_action_proposed VALUES (?1, ?2)",
                params![event_id.value(), postmortem_action_proposal_id.value()],
            )?;
        }
        EventBody::PostmortemClosed { postmortem_id } => {
            transaction.execute(
                "INSERT INTO event_postmortem_closed VALUES (?1, ?2)",
                params![event_id.value(), postmortem_id.value()],
            )?;
        }
        EventBody::ActorConfigurationRegistered {
            actor_configuration_id,
            actor_configuration_revision_id,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_configuration_registered VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    actor_configuration_id.value(),
                    actor_configuration_revision_id.value()
                ],
            )?;
        }
        EventBody::ContextPackRegistered { context_pack_id } => {
            transaction.execute(
                "INSERT INTO event_context_pack_registered VALUES (?1, ?2)",
                params![event_id.value(), context_pack_id.value()],
            )?;
        }
        EventBody::ActorInstanceAdmitted {
            actor_instance_id,
            principal_id,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_instance_admitted VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    actor_instance_id.value(),
                    principal_id.value()
                ],
            )?;
        }
        EventBody::TicketAdmitted { ticket_id } => {
            transaction.execute(
                "INSERT INTO event_ticket_admitted VALUES (?1, ?2)",
                params![event_id.value(), ticket_id.value()],
            )?;
        }
        EventBody::WorkItemRegistered {
            work_item_id,
            ticket_id,
            adversarial_review_id,
        } => {
            transaction.execute(
                "INSERT INTO event_work_item_registered VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    work_item_id.value(),
                    ticket_id.value(),
                    adversarial_review_id.map(AdversarialReviewId::value)
                ],
            )?;
        }
        EventBody::WorkItemClaimed {
            work_item_id,
            work_lease_id,
            actor_instance_id,
        } => {
            transaction.execute(
                "INSERT INTO event_work_item_claimed VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    work_item_id.value(),
                    work_lease_id.value(),
                    actor_instance_id.value()
                ],
            )?;
        }
        EventBody::ActorAttemptStarted {
            actor_attempt_id,
            work_item_id,
            budget_reservation_id,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_attempt_started VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    actor_attempt_id.value(),
                    work_item_id.value(),
                    budget_reservation_id.value()
                ],
            )?;
        }
        EventBody::ActorAttemptTerminalAttested {
            actor_attempt_id,
            terminal_kind,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_attempt_terminal_attested VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    actor_attempt_id.value(),
                    *terminal_kind as i64
                ],
            )?;
        }
        EventBody::TicketAttemptValidated {
            actor_attempt_id,
            ticket_id,
        } => {
            transaction.execute(
                "INSERT INTO event_ticket_attempt_validated VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    actor_attempt_id.value(),
                    ticket_id.value()
                ],
            )?;
        }
        EventBody::ActorAttemptRetryPrepared {
            actor_attempt_id,
            work_item_id,
            ticket_id,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_attempt_retry_prepared VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    actor_attempt_id.value(),
                    work_item_id.value(),
                    ticket_id.value()
                ],
            )?;
        }
        EventBody::TicketCompleted {
            ticket_id,
            actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO event_ticket_completed VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    ticket_id.value(),
                    actor_attempt_id.value()
                ],
            )?;
        }
        EventBody::WorkLeaseExpired {
            work_lease_id,
            work_item_id,
        } => {
            transaction.execute(
                "INSERT INTO event_work_lease_expired VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    work_lease_id.value(),
                    work_item_id.value()
                ],
            )?;
        }
        EventBody::ActorAttemptCancellationRequested {
            actor_attempt_id,
            reason,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_attempt_cancellation_requested VALUES (?1, ?2, ?3)",
                params![event_id.value(), actor_attempt_id.value(), *reason as i64],
            )?;
        }
        EventBody::OutcomeObligationRegistered {
            outcome_obligation_id,
            project_id,
        } => {
            transaction.execute(
                "INSERT INTO event_outcome_obligation_registered VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    outcome_obligation_id.value(),
                    project_id.value()
                ],
            )?;
        }
        EventBody::OutcomeObligationResolved {
            outcome_obligation_id,
            state,
        } => {
            transaction.execute(
                "INSERT INTO event_outcome_obligation_resolved VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    outcome_obligation_id.value(),
                    *state as i64
                ],
            )?;
        }
        EventBody::ContentSealReceiptRecorded {
            content_seal_receipt_id,
            digest,
        } => {
            transaction.execute(
                "INSERT INTO event_content_seal_receipt_recorded VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    content_seal_receipt_id.value(),
                    digest.as_bytes().as_slice()
                ],
            )?;
        }
        EventBody::ContentObjectRegistered {
            content_object_id,
            content_seal_receipt_id,
        } => {
            transaction.execute(
                "INSERT INTO event_content_object_registered VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    content_object_id.value(),
                    content_seal_receipt_id.value()
                ],
            )?;
        }
        EventBody::ForensicManifestRegistered {
            forensic_manifest_id,
            producing_deterministic_experiment_id,
            evaluator_output_content_object_id,
        } => {
            transaction.execute(
                "INSERT INTO event_forensic_manifest_registered VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    forensic_manifest_id.value(),
                    producing_deterministic_experiment_id.value(),
                    evaluator_output_content_object_id.value()
                ],
            )?;
        }
        EventBody::DeterministicEvaluatorForensicManifestRegistered {
            forensic_manifest_id,
            deterministic_experiment_id,
            native_child_spawn_admission_id,
            native_child_stream_seal_id,
            evaluator_output_content_object_id,
        } => {
            transaction.execute(
                "INSERT INTO event_deterministic_evaluator_forensic_manifest_registered
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    event_id.value(),
                    forensic_manifest_id.value(),
                    deterministic_experiment_id.value(),
                    native_child_spawn_admission_id.value(),
                    native_child_stream_seal_id.value(),
                    evaluator_output_content_object_id.value(),
                ],
            )?;
        }
        EventBody::DeterministicExperimentRegistered {
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
        } => {
            transaction.execute(
                "INSERT INTO event_deterministic_experiment_registered VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    deterministic_experiment_id.value(),
                    evaluator_revision_id.value(),
                    input_manifest_id.value()
                ],
            )?;
        }
        EventBody::DeterministicEvaluationReceiptRecorded {
            deterministic_evaluation_receipt_id,
            deterministic_experiment_id,
        } => {
            transaction.execute(
                "INSERT INTO event_deterministic_evaluation_receipt_recorded VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    deterministic_evaluation_receipt_id.value(),
                    deterministic_experiment_id.value()
                ],
            )?;
        }
        EventBody::DeterministicEvidenceAdmitted {
            evidence_admission_id,
            deterministic_evaluation_receipt_id,
            semantic_role,
            applicability,
        } => {
            transaction.execute(
                "INSERT INTO event_deterministic_evidence_admitted VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event_id.value(),
                    evidence_admission_id.value(),
                    deterministic_evaluation_receipt_id.value(),
                    *semantic_role as i64,
                    *applicability as i64
                ],
            )?;
        }
        EventBody::DeterministicExperimentFinalized {
            deterministic_experiment_id,
            terminal_state,
        } => {
            transaction.execute(
                "INSERT INTO event_deterministic_experiment_finalized VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    deterministic_experiment_id.value(),
                    *terminal_state as i64
                ],
            )?;
        }
        EventBody::DeterministicEvaluatorNativeChildAdmitted {
            native_child_spawn_admission_id,
            owner:
                NativeChildOwner::DeterministicEvaluator {
                    deterministic_experiment_id,
                    evaluator_revision_id,
                    input_manifest_id,
                },
        } => {
            transaction.execute(
                "INSERT INTO event_deterministic_evaluator_native_child_admitted VALUES (?1, ?2, ?3, ?4, ?5)",
                params![event_id.value(), native_child_spawn_admission_id.value(), deterministic_experiment_id.value(), evaluator_revision_id.value(), input_manifest_id.value()],
            )?;
        }
        EventBody::DeterministicEvaluatorNativeChildAdmitted {
            owner: NativeChildOwner::Pi(_),
            ..
        } => {
            return Err(StoreError::LedgerCorruption(
                "evaluator-native child event has Pi owner",
            ));
        }
        EventBody::DeterministicEvaluatorNativeChildSpawnRecorded {
            native_child_id,
            native_child_spawn_admission_id,
        } => {
            transaction.execute(
                "INSERT INTO event_deterministic_evaluator_native_child_spawn_recorded VALUES (?1, ?2, ?3)",
                params![event_id.value(), native_child_id.value(), native_child_spawn_admission_id.value()],
            )?;
        }
        EventBody::PiChildSpawnAdmitted {
            native_child_spawn_admission_id,
            owner,
            budget_reservation_id,
        } => {
            let (attempt, office) = match owner {
                PiChildOwner::ActorAttempt(id) => (Some(id.value()), None),
                PiChildOwner::RootAuthorityOfficeSession(id) => (None, Some(id.value())),
            };
            transaction.execute(
                "INSERT INTO event_pi_child_spawn_admitted VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event_id.value(),
                    native_child_spawn_admission_id.value(),
                    attempt,
                    office,
                    budget_reservation_id.value()
                ],
            )?;
        }
        EventBody::InertPiChildSpawnRecorded {
            native_child_id,
            native_child_spawn_admission_id,
        } => {
            transaction.execute(
                "INSERT INTO event_inert_pi_child_spawn_recorded VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    native_child_id.value(),
                    native_child_spawn_admission_id.value()
                ],
            )?;
        }
        EventBody::PiAdapterReadyRecorded {
            native_child_id,
            pi_session_id,
        } => {
            transaction.execute(
                "INSERT INTO event_pi_adapter_ready_recorded VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    native_child_id.value(),
                    pi_session_id.value()
                ],
            )?;
        }
        EventBody::PiCreateSessionAuthorized { native_child_id } => {
            transaction.execute(
                "INSERT INTO event_pi_create_session_authorized VALUES (?1, ?2)",
                params![event_id.value(), native_child_id.value()],
            )?;
        }
        EventBody::PiCreateSessionDeliveryRecorded { native_child_id } => {
            transaction.execute(
                "INSERT INTO event_pi_create_session_delivery_recorded VALUES (?1, ?2)",
                params![event_id.value(), native_child_id.value()],
            )?;
        }
        EventBody::PiSessionReadyRecorded {
            native_child_id,
            pi_session_id,
        } => {
            transaction.execute(
                "INSERT INTO event_pi_session_ready_recorded VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    native_child_id.value(),
                    pi_session_id.value()
                ],
            )?;
        }
        EventBody::PiAbortControlDeliveryRecorded {
            pi_abort_control_receipt_id,
            native_child_id,
            cancellation_propagation_id,
            correlation_identity,
            abort_command_digest,
            outcome,
        } => {
            transaction.execute(
                "INSERT INTO event_pi_abort_control_delivery_recorded VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![event_id.value(), pi_abort_control_receipt_id.value(), native_child_id.value(), cancellation_propagation_id.value(), correlation_identity.as_str(), abort_command_digest.as_bytes().as_slice(), *outcome as i64],
            )?;
        }
        EventBody::ChildStreamSealed {
            native_child_stream_seal_id,
            native_child_id,
            stream_kind,
            completeness,
        } => {
            transaction.execute(
                "INSERT INTO event_child_stream_sealed VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event_id.value(),
                    native_child_stream_seal_id.value(),
                    native_child_id.value(),
                    *stream_kind as i64,
                    *completeness as i64
                ],
            )?;
        }
        EventBody::ChildProcessLivenessObserved {
            native_child_liveness_observation_id,
            native_child_id,
            liveness,
        } => {
            transaction.execute(
                "INSERT INTO event_child_process_liveness_observed VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    native_child_liveness_observation_id.value(),
                    native_child_id.value(),
                    *liveness as i64
                ],
            )?;
        }
        EventBody::ProcessSignalReceiptRecorded {
            process_signal_receipt_id,
            native_child_id,
            action,
            delivery,
            observed_liveness,
            cause,
        } => {
            let (cause_kind, propagation_id) = signal_cause_parts(*cause);
            transaction.execute("INSERT INTO event_process_signal_receipt_recorded VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", params![event_id.value(), process_signal_receipt_id.value(), native_child_id.value(), *action as i64, *delivery as i64, *observed_liveness as i64, cause_kind, propagation_id])?;
        }
        EventBody::DirectChildReaped {
            native_child_reap_receipt_id,
            native_child_id,
            wait_status,
            group_liveness_before_cleanup,
            group_liveness_after_cleanup,
        } => {
            let (kind, value): (i64, Option<i64>) = match wait_status {
                DirectChildWaitStatus::Exited { exit_code } => {
                    (1, Some(i64::from(exit_code.value())))
                }
                DirectChildWaitStatus::Signaled { signal_number } => {
                    (2, Some(i64::from(signal_number.value())))
                }
                DirectChildWaitStatus::Unknown => (3, None),
            };
            transaction.execute(
                "INSERT INTO event_direct_child_reaped VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    event_id.value(),
                    native_child_reap_receipt_id.value(),
                    native_child_id.value(),
                    kind,
                    value,
                    *group_liveness_before_cleanup as i64,
                    *group_liveness_after_cleanup as i64
                ],
            )?;
        }
        EventBody::ChildRecoveryObserved {
            native_child_recovery_receipt_id,
            native_child_id,
            observation,
            group_liveness_after_restart,
        } => {
            transaction.execute(
                "INSERT INTO event_child_recovery_observed VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event_id.value(),
                    native_child_recovery_receipt_id.value(),
                    native_child_id.value(),
                    *observation as i64,
                    *group_liveness_after_restart as i64
                ],
            )?;
        }
        EventBody::ChildProcessFinalized {
            native_child_id,
            disposition,
        } => {
            transaction.execute(
                "INSERT INTO event_child_process_finalized VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    native_child_id.value(),
                    *disposition as i64
                ],
            )?;
        }
        EventBody::CancellationPropagationBegun {
            cancellation_propagation_id,
            cancellation_request_id,
        } => {
            transaction.execute(
                "INSERT INTO event_cancellation_propagation_begun VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    cancellation_propagation_id.value(),
                    cancellation_request_id.value()
                ],
            )?;
        }
        EventBody::CancellationPropagationReconciled {
            cancellation_propagation_id,
        } => {
            transaction.execute(
                "INSERT INTO event_cancellation_propagation_reconciled VALUES (?1, ?2)",
                params![event_id.value(), cancellation_propagation_id.value()],
            )?;
        }
        EventBody::SupervisorEpochOpened {
            supervisor_epoch_id,
        } => {
            transaction.execute(
                "INSERT INTO event_supervisor_epoch_opened VALUES (?1, ?2)",
                params![event_id.value(), supervisor_epoch_id.value()],
            )?;
        }
        EventBody::CancellationPropagationContainmentFailed {
            cancellation_propagation_id,
        } => {
            transaction.execute(
                "INSERT INTO event_cancellation_propagation_containment_failed VALUES (?1, ?2)",
                params![event_id.value(), cancellation_propagation_id.value()],
            )?;
        }
        EventBody::NativeChildSpawnInvalidated {
            native_child_spawn_admission_id,
            reason,
        } => {
            transaction.execute(
                "INSERT INTO event_native_child_spawn_invalidated VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    native_child_spawn_admission_id.value(),
                    *reason as i64
                ],
            )?;
        }
        EventBody::PiOfficeTurnPromptAuthorized {
            pi_office_turn_prompt_authorization_id,
            office_turn_id,
            native_child_id,
            correlation_identity,
            budget_reservation_id,
        } => {
            transaction.execute("INSERT INTO event_pi_office_turn_prompt_authorized VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![event_id.value(), pi_office_turn_prompt_authorization_id.value(), office_turn_id.value(), native_child_id.value(), correlation_identity.as_str(), budget_reservation_id.value()])?;
        }
        EventBody::PiOfficeTurnPromptDelivered {
            office_turn_id,
            correlation_identity,
        } => {
            transaction.execute(
                "INSERT INTO event_pi_office_turn_prompt_delivered VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    office_turn_id.value(),
                    correlation_identity.as_str()
                ],
            )?;
        }
        EventBody::PiOfficeTurnPromptAccepted {
            office_turn_id,
            correlation_identity,
            command_result_sequence,
        } => {
            transaction.execute(
                "INSERT INTO event_pi_office_turn_prompt_accepted VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    office_turn_id.value(),
                    correlation_identity.as_str(),
                    command_result_sequence.value()
                ],
            )?;
        }
        EventBody::PiOfficeTurnUsageRecorded {
            pi_office_turn_usage_receipt_id,
            office_turn_id,
            protocol_sequence,
            cumulative_micro_usd,
        } => {
            transaction.execute(
                "INSERT INTO event_pi_office_turn_usage_recorded VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event_id.value(),
                    pi_office_turn_usage_receipt_id.value(),
                    office_turn_id.value(),
                    protocol_sequence.value(),
                    cumulative_micro_usd.value()
                ],
            )?;
        }
        EventBody::PiOfficeTurnUsageFrozen {
            office_turn_id,
            budget_reservation_id,
            cancellation_request_id,
            postmortem_id,
            failure,
        } => {
            let (kind, unknown, unavailable) = sql_pi_office_turn_usage_failure(*failure);
            transaction.execute("INSERT INTO event_pi_office_turn_usage_frozen VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", params![event_id.value(), office_turn_id.value(), budget_reservation_id.value(), cancellation_request_id.value(), postmortem_id.value(), kind, unknown, unavailable])?;
        }
        EventBody::PiOfficeTurnTerminalRecorded {
            pi_office_turn_terminal_receipt_id,
            office_turn_id,
            disposition,
            assistant_outcome,
        } => {
            transaction.execute(
                "INSERT INTO event_pi_office_turn_terminal_recorded VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event_id.value(),
                    pi_office_turn_terminal_receipt_id.value(),
                    office_turn_id.value(),
                    *disposition as i64,
                    *assistant_outcome as i64
                ],
            )?;
        }
        EventBody::PiOfficeSessionDisposeDelivered {
            session_id,
            native_child_id,
            correlation_identity,
        } => {
            transaction.execute(
                "INSERT INTO event_pi_office_session_dispose_delivered VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    session_id.value(),
                    native_child_id.value(),
                    correlation_identity.as_str()
                ],
            )?;
        }
        EventBody::PiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity,
            command_result_sequence,
        } => {
            transaction.execute(
                "INSERT INTO event_pi_office_session_dispose_accepted VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    session_id.value(),
                    correlation_identity.as_str(),
                    command_result_sequence.value()
                ],
            )?;
        }
        EventBody::PiOfficeSessionDisposeUsageRecorded {
            session_id,
            protocol_sequence,
            cumulative_micro_usd,
        } => {
            transaction.execute(
                "INSERT INTO event_pi_office_session_dispose_usage_recorded VALUES (?1, ?2, ?3, ?4)",
                params![event_id.value(), session_id.value(), protocol_sequence.value(), cumulative_micro_usd.value()],
            )?;
        }
        EventBody::PiOfficeSessionDisposeUsageFrozen {
            session_id,
            budget_reservation_id,
            cancellation_request_id,
            postmortem_id,
            failure,
        } => {
            let (kind, unknown, unavailable) = sql_pi_office_turn_usage_failure(*failure);
            transaction.execute(
                "INSERT INTO event_pi_office_session_dispose_usage_frozen VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![event_id.value(), session_id.value(), budget_reservation_id.value(), cancellation_request_id.value(), postmortem_id.value(), kind, unknown, unavailable],
            )?;
        }
        EventBody::PiOfficeSessionDisposed {
            pi_office_session_dispose_receipt_id,
            session_id,
            budget_reservation_id,
            observed_cumulative_micro_usd,
            budget_disposition,
        } => {
            let (kind, cancellation_request_id, postmortem_id) =
                sql_pi_office_session_dispose_budget_disposition(*budget_disposition);
            transaction.execute(
                "INSERT INTO event_pi_office_session_disposed VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![event_id.value(), pi_office_session_dispose_receipt_id.value(), session_id.value(), budget_reservation_id.value(), observed_cumulative_micro_usd.value(), kind, cancellation_request_id, postmortem_id],
            )?;
        }
    }
    Ok(())
}

fn decode_event_body(
    connection: &Connection,
    event_id: i64,
    kind: i64,
    command_id: &CommandId,
) -> Result<EventBody, StoreError> {
    let event_id_typed = EventId::try_from(event_id).map_err(|_| StoreError::InvalidStoredValue)?;
    let kind = event_kind_from_i64(kind)?;
    verify_exact_event_body(connection, event_id_typed, kind)?;
    let body = match kind {
        EventKind::SocietyIdentityCreated => EventBody::SocietyIdentityCreated {
            society_id: query_event_id(
                connection,
                "event_society_identity_created",
                "society_id",
                event_id_typed,
            )?,
        },
        EventKind::RootAuthorityOfficeInstalled => EventBody::RootAuthorityOfficeInstalled {
            office_id: query_event_id(
                connection,
                "event_root_authority_office_installed",
                "office_id",
                event_id_typed,
            )?,
        },
        EventKind::FoundingMissionInstalled => {
            let (mission_id, application_revision_id) = connection
                .query_row(
                    "SELECT founding_mission_id, application_revision_id
                     FROM event_founding_mission_installed WHERE event_id = ?1",
                    [event_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing founding mission event body",
                ))?;
            EventBody::FoundingMissionInstalled {
                mission_id: FoundingMissionId::try_from(mission_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                application_revision_id: ApplicationRevisionId::try_from(application_revision_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::RootAuthorityAppointed => {
            let (occupancy_id, principal_id) = connection
                .query_row(
                    "SELECT office_occupancy_id, principal_id
                     FROM event_root_authority_appointed WHERE event_id = ?1",
                    [event_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing root authority appointment event body",
                ))?;
            EventBody::RootAuthorityAppointed {
                occupancy_id: OfficeOccupancyId::try_from(occupancy_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                principal_id: PrincipalId::try_from(principal_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::R0HardCeilingSet => {
            let (society, ceiling) = connection.query_row("SELECT society_id, ceiling_micros FROM event_r0_hard_ceiling_set WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing r0 ceiling event body"))?;
            EventBody::R0HardCeilingSet {
                society_id: SocietyId::try_from(society)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ceiling: UsdMicros::try_from(ceiling)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::SocietyBootstrapped => EventBody::SocietyBootstrapped {
            society_id: query_event_id(
                connection,
                "event_society_bootstrapped",
                "society_id",
                event_id_typed,
            )?,
        },
        EventKind::OperatingCycleProposed => {
            let (cycle, generation, treatment, budget_ceiling) = connection.query_row("SELECT operating_cycle_id, admission_generation, treatment, budget_ceiling_micros FROM event_operating_cycle_proposed WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing cycle proposal event body"))?;
            EventBody::OperatingCycleProposed {
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                generation: AdmissionGeneration::try_from(generation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                treatment: operating_cycle_treatment_from_i64(treatment)?,
                budget_ceiling: UsdMicros::try_from(budget_ceiling)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::OperatingCycleStateChanged => {
            let (cycle, state, generation) = connection.query_row("SELECT operating_cycle_id, lifecycle_state, admission_generation FROM event_operating_cycle_state_changed WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing cycle transition event body"))?;
            EventBody::OperatingCycleStateChanged {
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                state: operating_cycle_state_from_i64(state)?,
                generation: AdmissionGeneration::try_from(generation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::RootAuthorityOfficeSessionStarted => {
            let (session, cycle) = connection.query_row("SELECT root_authority_office_session_id, operating_cycle_id FROM event_root_authority_office_session_started WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing office session event body"))?;
            EventBody::RootAuthorityOfficeSessionStarted {
                session_id: RootAuthorityOfficeSessionId::try_from(session)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::RootAuthorityOfficeSessionStateChanged => {
            let (session, state) = connection.query_row("SELECT root_authority_office_session_id, lifecycle_state FROM event_root_authority_office_session_state_changed WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing office session state event body"))?;
            EventBody::RootAuthorityOfficeSessionStateChanged {
                session_id: RootAuthorityOfficeSessionId::try_from(session)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                state: office_session_state_from_i64(state)?,
            }
        }
        EventKind::OfficeTurnOpened => decode_office_turn_opened_event(connection, event_id_typed)?,
        EventKind::OfficeTurnSettled => {
            decode_office_turn_settled_event(connection, event_id_typed)?
        }
        EventKind::BudgetReserved => {
            let (reservation, cycle, amount) = connection.query_row("SELECT budget_reservation_id, operating_cycle_id, amount_micros FROM event_budget_reserved WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing budget reserve event body"))?;
            EventBody::BudgetReserved {
                reservation_id: BudgetReservationId::try_from(reservation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                amount: UsdMicros::try_from(amount).map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::BudgetReconciled => {
            let (reservation, amount) = connection.query_row("SELECT budget_reservation_id, observed_micros FROM event_budget_reconciled WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing budget reconciliation event body"))?;
            EventBody::BudgetReconciled {
                reservation_id: BudgetReservationId::try_from(reservation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                observed: UsdMicros::try_from(amount)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::BudgetAdmissionFrozen => {
            let (reservation, cycle, cancellation_request, postmortem, reason_kind, observed, reserved, unknown, unavailable) = connection.query_row("SELECT budget_reservation_id, operating_cycle_id, cancellation_request_id, postmortem_id, freeze_reason_kind, observed_micros, reserved_micros, unknown_reason, unavailable_reason FROM event_budget_admission_frozen WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?, row.get::<_, Option<i64>>(5)?, row.get::<_, Option<i64>>(6)?, row.get::<_, Option<i64>>(7)?, row.get::<_, Option<i64>>(8)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing budget frozen event body"))?;
            EventBody::BudgetAdmissionFrozen {
                reservation_id: BudgetReservationId::try_from(reservation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cancellation_request_id: CancellationRequestId::try_from(cancellation_request)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                postmortem_id: CostPostmortemId::try_from(postmortem)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reason: budget_freeze_reason_from_sql(
                    reason_kind,
                    observed,
                    reserved,
                    unknown,
                    unavailable,
                )?,
            }
        }
        EventKind::CancellationRequested => {
            let (request, cycle, mode, generation) = connection.query_row("SELECT cancellation_request_id, operating_cycle_id, cancellation_mode, admission_generation FROM event_cancellation_requested WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing cancellation request event body"))?;
            EventBody::CancellationRequested {
                cancellation_request_id: CancellationRequestId::try_from(request)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                mode: cancellation_mode_from_i64(mode)?,
                generation: AdmissionGeneration::try_from(generation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::CancellationReconciled => {
            let (request, cycle) = connection.query_row("SELECT cancellation_request_id, operating_cycle_id FROM event_cancellation_reconciled WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing cancellation reconciliation event body"))?;
            EventBody::CancellationReconciled {
                cancellation_request_id: CancellationRequestId::try_from(request)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::CostPostmortemClosed => {
            let (postmortem, reservation, cycle, resolution, charged) = connection.query_row("SELECT postmortem_id, budget_reservation_id, operating_cycle_id, resolution_kind, charged_micros FROM event_cost_postmortem_closed WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing cost postmortem closed event body"))?;
            EventBody::CostPostmortemClosed {
                postmortem_id: CostPostmortemId::try_from(postmortem)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reservation_id: BudgetReservationId::try_from(reservation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                resolution: cost_postmortem_resolution_from_i64(resolution)?,
                charged: UsdMicros::try_from(charged)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ProjectCreated => {
            let (project_id, application_revision_id) = connection
                .query_row(
                    "SELECT project_id, application_revision_id FROM event_project_created
                     WHERE event_id = ?1",
                    [event_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing project creation event body",
                ))?;
            EventBody::ProjectCreated {
                project_id: ProjectId::try_from(project_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                application_revision_id: ApplicationRevisionId::try_from(application_revision_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ProjectChartered => EventBody::ProjectChartered {
            project_id: query_event_id(
                connection,
                "event_project_chartered",
                "project_id",
                event_id_typed,
            )?,
        },
        EventKind::ProjectStateChanged => {
            let (id, state) =
                query_event_pair(connection, "event_project_state_changed", event_id)?;
            EventBody::ProjectStateChanged {
                project_id: ProjectId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
                state: project_state_from_i64(state)?,
            }
        }
        EventKind::ProjectMilestoneCompleted => EventBody::ProjectMilestoneCompleted {
            project_milestone_id: query_event_id(
                connection,
                "event_project_milestone_completed",
                "project_milestone_id",
                event_id_typed,
            )?,
        },
        EventKind::TicketCreated => {
            let (id, project) = query_event_pair(connection, "event_ticket_created", event_id)?;
            EventBody::TicketCreated {
                ticket_id: TicketId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::TicketStateChanged => {
            let (id, state) = query_event_pair(connection, "event_ticket_state_changed", event_id)?;
            EventBody::TicketStateChanged {
                ticket_id: TicketId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
                state: ticket_state_from_i64(state)?,
            }
        }
        EventKind::GraphObjectRevisionAdded => {
            let (object, revision) =
                query_event_pair(connection, "event_graph_object_revision_added", event_id)?;
            EventBody::GraphObjectRevisionAdded {
                graph_object_id: GraphObjectId::try_from(object)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                graph_revision_id: GraphRevisionId::try_from(revision)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::GraphRevisionCommitted => EventBody::GraphRevisionCommitted {
            graph_revision_id: query_event_id(
                connection,
                "event_graph_revision_committed",
                "graph_revision_id",
                event_id_typed,
            )?,
        },
        EventKind::GraphEdgeAdded => EventBody::GraphEdgeAdded {
            graph_edge_id: query_event_id(
                connection,
                "event_graph_edge_added",
                "graph_edge_id",
                event_id_typed,
            )?,
        },
        EventKind::EpisodeCreated => {
            let (episode, project) =
                query_event_pair(connection, "event_episode_created", event_id)?;
            EventBody::EpisodeCreated {
                causal_episode_id: CausalEpisodeId::try_from(episode)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::EpisodeStateChanged => {
            let (episode, state) =
                query_event_pair(connection, "event_episode_state_changed", event_id)?;
            EventBody::EpisodeStateChanged {
                causal_episode_id: CausalEpisodeId::try_from(episode)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                state: episode_state_from_i64(state)?,
            }
        }
        EventKind::AdversarialReviewRequested => EventBody::AdversarialReviewRequested {
            adversarial_review_id: query_event_id(
                connection,
                "event_adversarial_review_requested",
                "adversarial_review_id",
                event_id_typed,
            )?,
        },
        EventKind::AdversarialReviewerAssigned => {
            let (review, reviewer, actor, attempt): (i64, i64, i64, i64) = connection.query_row(
                "SELECT adversarial_review_id, reviewer_principal_id, reviewer_actor_instance_id, reviewer_actor_attempt_id FROM event_adversarial_reviewer_assigned WHERE event_id = ?1",
                [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing reviewer assignment event body"))?;
            EventBody::AdversarialReviewerAssigned {
                adversarial_review_id: AdversarialReviewId::try_from(review)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_principal_id: PrincipalId::try_from(reviewer)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_actor_instance_id: ActorInstanceId::try_from(actor)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ReviewChallengeSubmitted => {
            let (challenge, author) =
                query_event_pair(connection, "event_review_challenge_submitted", event_id)?;
            EventBody::ReviewChallengeSubmitted {
                review_challenge_id: ReviewChallengeId::try_from(challenge)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                author_principal_id: PrincipalId::try_from(author)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ReviewChallengeResponded => EventBody::ReviewChallengeResponded {
            review_challenge_id: query_event_id(
                connection,
                "event_review_challenge_responded",
                "review_challenge_id",
                event_id_typed,
            )?,
        },
        EventKind::ReviewChallengeDispositioned => {
            let (id, disposition) =
                query_event_pair(connection, "event_review_challenge_dispositioned", event_id)?;
            EventBody::ReviewChallengeDispositioned {
                review_challenge_id: ReviewChallengeId::try_from(id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                disposition: review_disposition_kind_from_i64(disposition)?,
            }
        }
        EventKind::AdversarialReviewResolved => {
            let (id, state) =
                query_event_pair(connection, "event_adversarial_review_resolved", event_id)?;
            EventBody::AdversarialReviewResolved {
                adversarial_review_id: AdversarialReviewId::try_from(id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                state: adversarial_review_state_from_i64(state)?,
            }
        }
        EventKind::PostmortemTriggered => EventBody::PostmortemTriggered {
            postmortem_id: query_event_id(
                connection,
                "event_postmortem_triggered",
                "postmortem_id",
                event_id_typed,
            )?,
        },
        EventKind::PostmortemCausalClaimRecorded => EventBody::PostmortemCausalClaimRecorded {
            postmortem_causal_claim_id: query_event_id(
                connection,
                "event_postmortem_causal_claim_recorded",
                "postmortem_causal_claim_id",
                event_id_typed,
            )?,
        },
        EventKind::PostmortemActionProposed => EventBody::PostmortemActionProposed {
            postmortem_action_proposal_id: query_event_id(
                connection,
                "event_postmortem_action_proposed",
                "postmortem_action_proposal_id",
                event_id_typed,
            )?,
        },
        EventKind::PostmortemClosed => EventBody::PostmortemClosed {
            postmortem_id: query_event_id(
                connection,
                "event_postmortem_closed",
                "postmortem_id",
                event_id_typed,
            )?,
        },
        EventKind::ActorConfigurationRegistered => {
            let (configuration, revision) =
                query_event_pair(connection, "event_actor_configuration_registered", event_id)?;
            EventBody::ActorConfigurationRegistered {
                actor_configuration_id: ActorConfigurationId::try_from(configuration)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_configuration_revision_id: ActorConfigurationRevisionId::try_from(revision)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ContextPackRegistered => EventBody::ContextPackRegistered {
            context_pack_id: query_event_id(
                connection,
                "event_context_pack_registered",
                "context_pack_id",
                event_id_typed,
            )?,
        },
        EventKind::ActorInstanceAdmitted => {
            let (actor, principal) =
                query_event_pair(connection, "event_actor_instance_admitted", event_id)?;
            EventBody::ActorInstanceAdmitted {
                actor_instance_id: ActorInstanceId::try_from(actor)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                principal_id: PrincipalId::try_from(principal)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::TicketAdmitted => EventBody::TicketAdmitted {
            ticket_id: query_event_id(
                connection,
                "event_ticket_admitted",
                "ticket_id",
                event_id_typed,
            )?,
        },
        EventKind::WorkItemRegistered => {
            let (work, ticket, review): (i64, i64, Option<i64>) = connection.query_row(
                "SELECT work_item_id, ticket_id, adversarial_review_id FROM event_work_item_registered WHERE event_id = ?1",
                [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing work item event body"))?;
            EventBody::WorkItemRegistered {
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                adversarial_review_id: review
                    .map(AdversarialReviewId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::WorkItemClaimed => {
            let (work, lease, actor): (i64, i64, i64) = connection.query_row("SELECT work_item_id, work_lease_id, actor_instance_id FROM event_work_item_claimed WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing work item claim event body"))?;
            EventBody::WorkItemClaimed {
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_lease_id: WorkLeaseId::try_from(lease)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_instance_id: ActorInstanceId::try_from(actor)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ActorAttemptStarted => {
            let (attempt, work, reservation): (i64, i64, i64) = connection.query_row("SELECT actor_attempt_id, work_item_id, budget_reservation_id FROM event_actor_attempt_started WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing actor attempt started event body"))?;
            EventBody::ActorAttemptStarted {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                budget_reservation_id: BudgetReservationId::try_from(reservation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ActorAttemptTerminalAttested => {
            let (attempt, terminal) = query_event_pair(
                connection,
                "event_actor_attempt_terminal_attested",
                event_id,
            )?;
            EventBody::ActorAttemptTerminalAttested {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                terminal_kind: actor_attempt_terminal_kind_from_i64(terminal)?,
            }
        }
        EventKind::TicketAttemptValidated => {
            let (attempt, ticket) =
                query_event_pair(connection, "event_ticket_attempt_validated", event_id)?;
            EventBody::TicketAttemptValidated {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ActorAttemptRetryPrepared => {
            let (attempt, work, ticket): (i64, i64, i64) = connection.query_row("SELECT actor_attempt_id, work_item_id, ticket_id FROM event_actor_attempt_retry_prepared WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing retry event body"))?;
            EventBody::ActorAttemptRetryPrepared {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::TicketCompleted => {
            let (ticket, attempt) =
                query_event_pair(connection, "event_ticket_completed", event_id)?;
            EventBody::TicketCompleted {
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::WorkLeaseExpired => {
            let (lease, work) = query_event_pair(connection, "event_work_lease_expired", event_id)?;
            EventBody::WorkLeaseExpired {
                work_lease_id: WorkLeaseId::try_from(lease)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ActorAttemptCancellationRequested => {
            let (attempt, reason) = query_event_pair(
                connection,
                "event_actor_attempt_cancellation_requested",
                event_id,
            )?;
            EventBody::ActorAttemptCancellationRequested {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reason: actor_attempt_cancellation_reason_from_i64(reason)?,
            }
        }
        EventKind::OutcomeObligationRegistered => {
            let (obligation, project) =
                query_event_pair(connection, "event_outcome_obligation_registered", event_id)?;
            EventBody::OutcomeObligationRegistered {
                outcome_obligation_id: OutcomeObligationId::try_from(obligation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::OutcomeObligationResolved => {
            let (obligation, state) =
                query_event_pair(connection, "event_outcome_obligation_resolved", event_id)?;
            EventBody::OutcomeObligationResolved {
                outcome_obligation_id: OutcomeObligationId::try_from(obligation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                state: outcome_obligation_state_from_i64(state)?,
            }
        }
        EventKind::ContentSealReceiptRecorded => {
            let (receipt, digest): (i64, Vec<u8>) = connection.query_row("SELECT content_seal_receipt_id, digest FROM event_content_seal_receipt_recorded WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?, row.get(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing content seal receipt event body"))?;
            EventBody::ContentSealReceiptRecorded {
                content_seal_receipt_id: ContentSealReceiptId::try_from(receipt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                digest: digest_from_stored_bytes(&digest)?,
            }
        }
        EventKind::ContentObjectRegistered => {
            let (object, receipt) =
                query_event_pair(connection, "event_content_object_registered", event_id)?;
            EventBody::ContentObjectRegistered {
                content_object_id: ContentObjectId::try_from(object)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                content_seal_receipt_id: ContentSealReceiptId::try_from(receipt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ForensicManifestRegistered => {
            let (manifest, experiment, output): (i64, i64, i64) = connection.query_row("SELECT forensic_manifest_id, producing_deterministic_experiment_id, evaluator_output_content_object_id FROM event_forensic_manifest_registered WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing forensic manifest event body"))?;
            EventBody::ForensicManifestRegistered {
                forensic_manifest_id: ForensicManifestId::try_from(manifest)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                producing_deterministic_experiment_id: DeterministicExperimentId::try_from(
                    experiment,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_output_content_object_id: ContentObjectId::try_from(output)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::DeterministicEvaluatorForensicManifestRegistered => {
            let row: (i64, i64, i64, i64, i64) = connection
                .query_row(
                    "SELECT forensic_manifest_id, deterministic_experiment_id,
                            native_child_spawn_admission_id,
                            native_child_stream_seal_id,
                            evaluator_output_content_object_id
                       FROM event_deterministic_evaluator_forensic_manifest_registered
                      WHERE event_id = ?1",
                    [event_id],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing deterministic evaluator forensic manifest event body",
                ))?;
            EventBody::DeterministicEvaluatorForensicManifestRegistered {
                forensic_manifest_id: ForensicManifestId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_experiment_id: DeterministicExperimentId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_spawn_admission_id: NativeChildSpawnAdmissionId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_stream_seal_id: NativeChildStreamSealId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_output_content_object_id: ContentObjectId::try_from(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::DeterministicExperimentRegistered => {
            let (experiment, evaluator, input): (i64,i64,i64) = connection.query_row("SELECT deterministic_experiment_id, evaluator_revision_id, input_manifest_id FROM event_deterministic_experiment_registered WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing deterministic experiment event body"))?;
            EventBody::DeterministicExperimentRegistered {
                deterministic_experiment_id: DeterministicExperimentId::try_from(experiment)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_revision_id: EvaluatorRevisionId::try_from(evaluator)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                input_manifest_id: InputManifestId::try_from(input)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::DeterministicEvaluationReceiptRecorded => {
            let (receipt, experiment) = query_event_pair(
                connection,
                "event_deterministic_evaluation_receipt_recorded",
                event_id,
            )?;
            EventBody::DeterministicEvaluationReceiptRecorded {
                deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId::try_from(
                    receipt,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_experiment_id: DeterministicExperimentId::try_from(experiment)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::DeterministicEvidenceAdmitted => {
            let (admission, receipt, role, applicability): (i64,i64,i64,i64) = connection.query_row("SELECT evidence_admission_id, deterministic_evaluation_receipt_id, semantic_role, applicability FROM event_deterministic_evidence_admitted WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing deterministic evidence event body"))?;
            EventBody::DeterministicEvidenceAdmitted {
                evidence_admission_id: EvidenceAdmissionId::try_from(admission)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId::try_from(
                    receipt,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                semantic_role: evidence_semantic_role_from_i64(role)?,
                applicability: evidence_applicability_from_i64(applicability)?,
            }
        }
        EventKind::DeterministicExperimentFinalized => {
            let (experiment, terminal_state): (i64, i64) = connection.query_row(
                "SELECT deterministic_experiment_id, terminal_state FROM event_deterministic_experiment_finalized WHERE event_id = ?1",
                [event_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing deterministic experiment finalized event body"))?;
            EventBody::DeterministicExperimentFinalized {
                deterministic_experiment_id: DeterministicExperimentId::try_from(experiment)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                terminal_state: deterministic_experiment_state_from_i64(terminal_state)?,
            }
        }
        EventKind::DeterministicEvaluatorNativeChildAdmitted => {
            let row: (i64,i64,i64,i64) = connection.query_row(
                "SELECT native_child_spawn_admission_id, deterministic_experiment_id, evaluator_revision_id, input_manifest_id FROM event_deterministic_evaluator_native_child_admitted WHERE event_id = ?1",
                [event_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing evaluator native-child admission event body"))?;
            EventBody::DeterministicEvaluatorNativeChildAdmitted {
                native_child_spawn_admission_id: NativeChildSpawnAdmissionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                owner: NativeChildOwner::DeterministicEvaluator {
                    deterministic_experiment_id: DeterministicExperimentId::try_from(row.1)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                    evaluator_revision_id: EvaluatorRevisionId::try_from(row.2)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                    input_manifest_id: InputManifestId::try_from(row.3)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                },
            }
        }
        EventKind::DeterministicEvaluatorNativeChildSpawnRecorded => {
            let row: (i64,i64) = connection.query_row(
                "SELECT native_child_id, native_child_spawn_admission_id FROM event_deterministic_evaluator_native_child_spawn_recorded WHERE event_id = ?1",
                [event_id], |r| Ok((r.get(0)?,r.get(1)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing evaluator native-child spawn event body"))?;
            EventBody::DeterministicEvaluatorNativeChildSpawnRecorded {
                native_child_id: NativeChildId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_spawn_admission_id: NativeChildSpawnAdmissionId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiChildSpawnAdmitted => {
            let row: (i64, Option<i64>, Option<i64>, i64) = connection.query_row("SELECT native_child_spawn_admission_id, actor_attempt_id, root_authority_office_session_id, budget_reservation_id FROM event_pi_child_spawn_admitted WHERE event_id = ?1", [event_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi child admission event body"))?;
            let owner = decode_child_owner(row.1, row.2)?;
            EventBody::PiChildSpawnAdmitted {
                native_child_spawn_admission_id: NativeChildSpawnAdmissionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                owner,
                budget_reservation_id: BudgetReservationId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::InertPiChildSpawnRecorded => {
            let (child, admission) =
                query_event_pair(connection, "event_inert_pi_child_spawn_recorded", event_id)?;
            EventBody::InertPiChildSpawnRecorded {
                native_child_id: NativeChildId::try_from(child)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_spawn_admission_id: NativeChildSpawnAdmissionId::try_from(admission)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiAdapterReadyRecorded => {
            let (child, session) =
                query_event_pair(connection, "event_pi_adapter_ready_recorded", event_id)?;
            EventBody::PiAdapterReadyRecorded {
                native_child_id: NativeChildId::try_from(child)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                pi_session_id: PiSessionId::try_from(session)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiCreateSessionAuthorized => EventBody::PiCreateSessionAuthorized {
            native_child_id: query_event_id(
                connection,
                "event_pi_create_session_authorized",
                "native_child_id",
                event_id_typed,
            )?,
        },
        EventKind::PiCreateSessionDeliveryRecorded => EventBody::PiCreateSessionDeliveryRecorded {
            native_child_id: query_event_id(
                connection,
                "event_pi_create_session_delivery_recorded",
                "native_child_id",
                event_id_typed,
            )?,
        },
        EventKind::PiSessionReadyRecorded => {
            let (child, session) =
                query_event_pair(connection, "event_pi_session_ready_recorded", event_id)?;
            EventBody::PiSessionReadyRecorded {
                native_child_id: NativeChildId::try_from(child)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                pi_session_id: PiSessionId::try_from(session)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiAbortControlDeliveryRecorded => {
            let row: (i64, i64, i64, String, Vec<u8>, i64) = connection.query_row(
                "SELECT pi_abort_control_receipt_id, native_child_id, cancellation_propagation_id, correlation_identity, abort_command_digest, physical_write_outcome FROM event_pi_abort_control_delivery_recorded WHERE event_id = ?1",
                [event_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Abort event body"))?;
            EventBody::PiAbortControlDeliveryRecorded {
                pi_abort_control_receipt_id: PiAbortControlReceiptId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_id: NativeChildId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cancellation_propagation_id: CancellationPropagationId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                abort_command_digest: digest_from_stored_bytes(&row.4)?,
                outcome: pi_abort_control_write_outcome_from_i64(row.5)?,
            }
        }
        EventKind::ChildStreamSealed => {
            let row: (i64,i64,i64,i64) = connection.query_row("SELECT native_child_stream_seal_id, native_child_id, stream_kind, completeness FROM event_child_stream_sealed WHERE event_id = ?1", [event_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing child stream event body"))?;
            EventBody::ChildStreamSealed {
                native_child_stream_seal_id: NativeChildStreamSealId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_id: NativeChildId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                stream_kind: child_stream_kind_from_i64(row.2)?,
                completeness: child_stream_completeness_from_i64(row.3)?,
            }
        }
        EventKind::ChildProcessLivenessObserved => {
            let row:(i64,i64,i64)=connection.query_row("SELECT native_child_liveness_observation_id, native_child_id, liveness FROM event_child_process_liveness_observed WHERE event_id = ?1",[event_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing liveness event body"))?;
            EventBody::ChildProcessLivenessObserved {
                native_child_liveness_observation_id: NativeChildLivenessObservationId::try_from(
                    row.0,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_id: NativeChildId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                liveness: process_group_liveness_from_i64(row.2)?,
            }
        }
        EventKind::ProcessSignalReceiptRecorded => {
            let row:(i64,i64,i64,i64,i64,i64,Option<i64>)=connection.query_row("SELECT process_signal_receipt_id, native_child_id, signal_action, delivery, observed_liveness, cause_kind, cancellation_propagation_id FROM event_process_signal_receipt_recorded WHERE event_id = ?1",[event_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing signal event body"))?;
            EventBody::ProcessSignalReceiptRecorded {
                process_signal_receipt_id: ProcessSignalReceiptId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_id: NativeChildId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                action: process_signal_action_from_i64(row.2)?,
                delivery: process_signal_delivery_from_i64(row.3)?,
                observed_liveness: process_group_liveness_from_i64(row.4)?,
                cause: process_signal_cause_from_sql(row.5, row.6)?,
            }
        }
        EventKind::DirectChildReaped => {
            let row:(i64,i64,i64,Option<i64>,i64,i64)=connection.query_row("SELECT native_child_reap_receipt_id, native_child_id, wait_status_kind, status_value, group_liveness_before_cleanup, group_liveness_after_cleanup FROM event_direct_child_reaped WHERE event_id = ?1",[event_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing reap event body"))?;
            EventBody::DirectChildReaped {
                native_child_reap_receipt_id: NativeChildReapReceiptId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_id: NativeChildId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                wait_status: direct_wait_status_from_sql(row.2, row.3)?,
                group_liveness_before_cleanup: process_group_liveness_from_i64(row.4)?,
                group_liveness_after_cleanup: process_group_liveness_from_i64(row.5)?,
            }
        }
        EventKind::ChildRecoveryObserved => {
            let row:(i64,i64,i64,i64)=connection.query_row("SELECT native_child_recovery_receipt_id, native_child_id, observation, group_liveness_after_restart FROM event_child_recovery_observed WHERE event_id = ?1",[event_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing recovery event body"))?;
            EventBody::ChildRecoveryObserved {
                native_child_recovery_receipt_id: NativeChildRecoveryReceiptId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_id: NativeChildId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                observation: child_recovery_observation_from_i64(row.2)?,
                group_liveness_after_restart: process_group_liveness_from_i64(row.3)?,
            }
        }
        EventKind::ChildProcessFinalized => {
            let (child, disposition) =
                query_event_pair(connection, "event_child_process_finalized", event_id)?;
            EventBody::ChildProcessFinalized {
                native_child_id: NativeChildId::try_from(child)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                disposition: child_terminal_disposition_from_i64(disposition)?,
            }
        }
        EventKind::CancellationPropagationBegun => {
            let (propagation, request) =
                query_event_pair(connection, "event_cancellation_propagation_begun", event_id)?;
            EventBody::CancellationPropagationBegun {
                cancellation_propagation_id: CancellationPropagationId::try_from(propagation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cancellation_request_id: CancellationRequestId::try_from(request)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::CancellationPropagationReconciled => {
            EventBody::CancellationPropagationReconciled {
                cancellation_propagation_id: query_event_id(
                    connection,
                    "event_cancellation_propagation_reconciled",
                    "cancellation_propagation_id",
                    event_id_typed,
                )?,
            }
        }
        EventKind::SupervisorEpochOpened => EventBody::SupervisorEpochOpened {
            supervisor_epoch_id: query_event_id(
                connection,
                "event_supervisor_epoch_opened",
                "supervisor_epoch_id",
                event_id_typed,
            )?,
        },
        EventKind::CancellationPropagationContainmentFailed => {
            EventBody::CancellationPropagationContainmentFailed {
                cancellation_propagation_id: query_event_id(
                    connection,
                    "event_cancellation_propagation_containment_failed",
                    "cancellation_propagation_id",
                    event_id_typed,
                )?,
            }
        }
        EventKind::NativeChildSpawnInvalidated => {
            let (admission, reason): (i64, i64) = connection.query_row(
                "SELECT native_child_spawn_admission_id, reason FROM event_native_child_spawn_invalidated WHERE event_id = ?1",
                [event_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing native-child invalidation event body"))?;
            EventBody::NativeChildSpawnInvalidated {
                native_child_spawn_admission_id: NativeChildSpawnAdmissionId::try_from(admission)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reason: native_child_not_spawned_reason_from_i64(reason)?,
            }
        }
        EventKind::PiOfficeTurnPromptAuthorized => {
            let row: (i64, i64, i64, String, i64) = connection.query_row(
                "SELECT pi_office_turn_prompt_authorization_id, office_turn_id, native_child_id, correlation_identity, budget_reservation_id FROM event_pi_office_turn_prompt_authorized WHERE event_id = ?1",
                [event_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Prompt authorization event body"))?;
            EventBody::PiOfficeTurnPromptAuthorized {
                pi_office_turn_prompt_authorization_id:
                    PiOfficeTurnPromptAuthorizationId::try_from(row.0)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                office_turn_id: OfficeTurnId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_id: NativeChildId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                budget_reservation_id: BudgetReservationId::try_from(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiOfficeTurnPromptDelivered => {
            let row: (i64, String) = connection.query_row(
                "SELECT office_turn_id, correlation_identity FROM event_pi_office_turn_prompt_delivered WHERE event_id = ?1",
                [event_id], |r| Ok((r.get(0)?, r.get(1)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Prompt delivery event body"))?;
            EventBody::PiOfficeTurnPromptDelivered {
                office_turn_id: OfficeTurnId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiOfficeTurnPromptAccepted => {
            let row: (i64, String, i64) = connection.query_row(
                "SELECT office_turn_id, correlation_identity, command_result_sequence FROM event_pi_office_turn_prompt_accepted WHERE event_id = ?1",
                [event_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Prompt acceptance event body"))?;
            EventBody::PiOfficeTurnPromptAccepted {
                office_turn_id: OfficeTurnId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                command_result_sequence: PiProtocolSequence::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiOfficeTurnUsageRecorded => {
            let row: (i64, i64, i64, i64) = connection.query_row(
                "SELECT pi_office_turn_usage_receipt_id, office_turn_id, protocol_sequence, cumulative_ceiling_micros FROM event_pi_office_turn_usage_recorded WHERE event_id = ?1",
                [event_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office usage event body"))?;
            EventBody::PiOfficeTurnUsageRecorded {
                pi_office_turn_usage_receipt_id: PiOfficeTurnUsageReceiptId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                office_turn_id: OfficeTurnId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                protocol_sequence: PiProtocolSequence::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cumulative_micro_usd: UsdMicros::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiOfficeTurnUsageFrozen => {
            let row: (i64, i64, i64, i64, i64, Option<i64>, Option<i64>) = connection.query_row(
                "SELECT office_turn_id, budget_reservation_id, cancellation_request_id, cost_postmortem_id, failure_kind, unknown_reason, unavailable_reason FROM event_pi_office_turn_usage_frozen WHERE event_id = ?1",
                [event_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office usage freeze event body"))?;
            EventBody::PiOfficeTurnUsageFrozen {
                office_turn_id: OfficeTurnId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                budget_reservation_id: BudgetReservationId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cancellation_request_id: CancellationRequestId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                postmortem_id: CostPostmortemId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                failure: pi_office_turn_usage_failure_from_sql(row.4, row.5, row.6)?,
            }
        }
        EventKind::PiOfficeTurnTerminalRecorded => {
            let row: (i64, i64, i64, i64) = connection.query_row(
                "SELECT pi_office_turn_terminal_receipt_id, office_turn_id, disposition, assistant_outcome FROM event_pi_office_turn_terminal_recorded WHERE event_id = ?1",
                [event_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office terminal event body"))?;
            EventBody::PiOfficeTurnTerminalRecorded {
                pi_office_turn_terminal_receipt_id: PiOfficeTurnTerminalReceiptId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                office_turn_id: OfficeTurnId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                disposition: pi_office_turn_disposition_from_i64(row.2)?,
                assistant_outcome: pi_office_turn_assistant_outcome_from_i64(row.3)?,
            }
        }
        EventKind::PiOfficeSessionDisposeAuthorized => {
            let row: (i64, i64, String, i64) = connection
                .query_row(
                    "SELECT root_authority_office_session_id, native_child_id,
                        correlation_identity, authorized_generation
                 FROM event_pi_office_session_dispose_authorized WHERE event_id = ?1",
                    [event_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing Pi Office Dispose authorization event body",
                ))?;
            EventBody::PiOfficeSessionDisposeAuthorized {
                session_id: RootAuthorityOfficeSessionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_id: NativeChildId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                authorized_generation: AdmissionGeneration::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiOfficeSessionDisposeDelivered => {
            let row: (i64, i64, String) = connection
                .query_row(
                    "SELECT root_authority_office_session_id, native_child_id, correlation_identity
                 FROM event_pi_office_session_dispose_delivered WHERE event_id = ?1",
                    [event_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing Pi Office Dispose delivery event body",
                ))?;
            EventBody::PiOfficeSessionDisposeDelivered {
                session_id: RootAuthorityOfficeSessionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_id: NativeChildId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiOfficeSessionDisposeAccepted => {
            let row: (i64, String, i64) = connection.query_row(
                "SELECT root_authority_office_session_id, correlation_identity, command_result_sequence
                 FROM event_pi_office_session_dispose_accepted WHERE event_id = ?1",
                [event_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Dispose acceptance event body"))?;
            EventBody::PiOfficeSessionDisposeAccepted {
                session_id: RootAuthorityOfficeSessionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                command_result_sequence: PiProtocolSequence::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiOfficeSessionDisposeUsageRecorded => {
            let row: (i64, i64, i64) = connection.query_row(
                "SELECT root_authority_office_session_id, protocol_sequence, cumulative_ceiling_micros
                 FROM event_pi_office_session_dispose_usage_recorded WHERE event_id = ?1",
                [event_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Dispose usage event body"))?;
            EventBody::PiOfficeSessionDisposeUsageRecorded {
                session_id: RootAuthorityOfficeSessionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                protocol_sequence: PiProtocolSequence::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cumulative_micro_usd: UsdMicros::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::PiOfficeSessionDisposeUsageFrozen => {
            let row: (i64, i64, i64, i64, i64, Option<i64>, Option<i64>) = connection
                .query_row(
                    "SELECT root_authority_office_session_id, budget_reservation_id,
                        cancellation_request_id, cost_postmortem_id, failure_kind,
                        unknown_reason, unavailable_reason
                 FROM event_pi_office_session_dispose_usage_frozen WHERE event_id = ?1",
                    [event_id],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing Pi Office Dispose usage freeze event body",
                ))?;
            EventBody::PiOfficeSessionDisposeUsageFrozen {
                session_id: RootAuthorityOfficeSessionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                budget_reservation_id: BudgetReservationId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cancellation_request_id: CancellationRequestId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                postmortem_id: CostPostmortemId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                failure: pi_office_turn_usage_failure_from_sql(row.4, row.5, row.6)?,
            }
        }
        EventKind::PiOfficeSessionDisposed => {
            let row: (i64, i64, i64, i64, i64, Option<i64>, Option<i64>) = connection
                .query_row(
                    "SELECT pi_office_session_dispose_receipt_id,
                        root_authority_office_session_id, budget_reservation_id,
                        observed_cumulative_micros, budget_disposition_kind,
                        cancellation_request_id, cost_postmortem_id
                 FROM event_pi_office_session_disposed WHERE event_id = ?1",
                    [event_id],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing Pi Office Disposed event body",
                ))?;
            EventBody::PiOfficeSessionDisposed {
                pi_office_session_dispose_receipt_id: PiOfficeSessionDisposeReceiptId::try_from(
                    row.0,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                session_id: RootAuthorityOfficeSessionId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                budget_reservation_id: BudgetReservationId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                observed_cumulative_micro_usd: UsdMicros::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                budget_disposition: pi_office_session_dispose_budget_disposition_from_sql(
                    row.4, row.3, row.5, row.6,
                )?,
            }
        }
    };
    let stored_fingerprint: Vec<u8> = connection.query_row(
        "SELECT event_fingerprint FROM events WHERE event_id = ?1",
        [event_id],
        |row| row.get(0),
    )?;
    if stored_fingerprint.as_slice()
        != event_fingerprint(event_id_typed, command_id, &body).as_bytes()
    {
        return Err(StoreError::LedgerCorruption(
            "event body fingerprint mismatch",
        ));
    }
    Ok(body)
}

/// Rebuilds every persisted command request from its named body and proves its
/// original request commitment still matches. This includes rejections: a
/// rejected command is durable operational history, not an untyped error
/// record that may escape integrity checks.
fn verify_command_bodies(connection: &Connection) -> Result<(), StoreError> {
    let mut statement =
        connection.prepare("SELECT command_row_id FROM commands ORDER BY command_row_id ASC")?;
    let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
    for row in rows {
        verify_command_body(connection, row?)?;
    }
    Ok(())
}

/// Validates the one durable typed request and receipt relation that owns a
/// selected event. `ledger_event` uses this rather than treating an event row
/// as authoritative on its own.
fn verify_command_body(connection: &Connection, command_row_id: i64) -> Result<(), StoreError> {
    let (
        command_id,
        principal_id,
        capability_grant_id,
        capability_kind,
        expected_generation,
        command_kind,
        stored_fingerprint,
        status,
        accepted_event_id,
    ) = connection
        .query_row(
            "SELECT command_id, principal_id, capability_grant_id,
                    capability_kind, expected_generation, command_kind,
                    request_fingerprint, command_status, accepted_event_id
             FROM commands WHERE command_row_id = ?1",
            [command_row_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Vec<u8>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, Option<i64>>(8)?,
                ))
            },
        )
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "event references a missing command",
        ))?;
    let kind = command_kind_from_i64(command_kind)?;
    let expected_table = command_body_table(kind)?;
    verify_exact_named_body(
        connection,
        command_row_id,
        expected_table,
        &COMMAND_BODY_TABLES,
    )?;
    let request = CommandRequest {
        command_id: CommandId::parse(command_id).map_err(|_| StoreError::InvalidStoredValue)?,
        principal_id: PrincipalId::try_from(principal_id)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        capability_grant_id: crate::CapabilityGrantId::try_from(capability_grant_id)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        capability: capability_from_i64(capability_kind)?,
        expected_generation: match expected_generation {
            Some(generation) => ExpectedGeneration::Exact(
                AdmissionGeneration::try_from(generation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            ),
            None => ExpectedGeneration::NotApplicable,
        },
        body: decode_command_body(connection, command_row_id, kind)?,
    };
    if kind == CommandKind::InstallFoundingMission && status == 1 {
        let (digest, source_content_object_id): (Vec<u8>, Option<i64>) = connection.query_row(
            "SELECT source_rendering_digest, source_content_object_id
                   FROM command_install_founding_mission WHERE command_row_id = ?1",
            [command_row_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let source_content_object_id =
            source_content_object_id.ok_or(StoreError::LedgerCorruption(
                "accepted founding mission command has no source content object",
            ))?;
        verify_mission_source_binding(
            connection,
            &digest,
            ContentObjectId::try_from(source_content_object_id)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        )?;
    }
    if request.body.kind() != kind {
        return Err(StoreError::LedgerCorruption(
            "command body does not match command kind",
        ));
    }
    if stored_fingerprint.as_slice() != request_fingerprint(&request).as_bytes() {
        return Err(StoreError::LedgerCorruption(
            "command request fingerprint mismatch",
        ));
    }
    let event_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM events WHERE command_row_id = ?1",
        [command_row_id],
        |row| row.get(0),
    )?;
    match (status, accepted_event_id, event_count) {
        (1, Some(event_id), 1) => {
            let linked: i64 = connection.query_row(
                "SELECT COUNT(*) FROM events WHERE event_id = ?1 AND command_row_id = ?2",
                params![event_id, command_row_id],
                |row| row.get(0),
            )?;
            if linked != 1 {
                return Err(StoreError::LedgerCorruption(
                    "accepted command does not name its event",
                ));
            }
        }
        (2, None, 0) => {}
        _ => {
            return Err(StoreError::LedgerCorruption(
                "command receipt and event relation disagree",
            ));
        }
    }
    Ok(())
}

/// Proves that an object identifier still joins through its one receipt to the
/// exact stored rendering digest. Foreign keys alone cannot express this
/// cross-table byte-identity invariant.
fn verify_mission_source_binding(
    connection: &Connection,
    digest: &[u8],
    source_content_object_id: ContentObjectId,
) -> Result<(), StoreError> {
    let matches: bool = connection.query_row(
        "SELECT EXISTS(
             SELECT 1
               FROM content_objects AS object
               JOIN content_seal_receipts AS receipt
                 ON receipt.content_seal_receipt_id = object.content_seal_receipt_id
              WHERE object.content_object_id = ?1 AND receipt.digest = ?2
         )",
        params![source_content_object_id.value(), digest],
        |row| row.get(0),
    )?;
    if matches {
        Ok(())
    } else {
        Err(StoreError::LedgerCorruption(
            "mission source content object does not match rendering digest",
        ))
    }
}

fn verify_application_mission_source_bindings(connection: &Connection) -> Result<(), StoreError> {
    let mut statement = connection.prepare(
        "SELECT source_rendering_digest, source_content_object_id
           FROM application_revisions ORDER BY application_revision_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (digest, source_content_object_id) = row?;
        verify_mission_source_binding(
            connection,
            &digest,
            ContentObjectId::try_from(source_content_object_id)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        )?;
    }
    Ok(())
}

fn replay_command_requests(
    connection: &Connection,
) -> Result<Vec<(CommandRequest, CommandDisposition)>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT command_row_id, command_id, principal_id, capability_grant_id,
                capability_kind, expected_generation, command_kind, command_status,
                accepted_event_id, rejection_code
         FROM commands ORDER BY command_row_id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, Option<i64>>(8)?,
            row.get::<_, Option<i64>>(9)?,
        ))
    })?;
    let mut commands = Vec::new();
    for row in rows {
        let (
            command_row_id,
            command_id,
            principal_id,
            capability_grant_id,
            capability_kind,
            expected_generation,
            command_kind,
            status,
            accepted_event_id,
            rejection_code,
        ) = row?;
        let kind = command_kind_from_i64(command_kind)?;
        let request = CommandRequest {
            command_id: CommandId::parse(command_id).map_err(|_| StoreError::InvalidStoredValue)?,
            principal_id: PrincipalId::try_from(principal_id)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            capability_grant_id: crate::CapabilityGrantId::try_from(capability_grant_id)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            capability: capability_from_i64(capability_kind)?,
            expected_generation: match expected_generation {
                Some(generation) => ExpectedGeneration::Exact(
                    AdmissionGeneration::try_from(generation)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                ),
                None => ExpectedGeneration::NotApplicable,
            },
            body: decode_command_body(connection, command_row_id, kind)?,
        };
        let disposition = match status {
            1 => CommandDisposition::Accepted(
                EventId::try_from(accepted_event_id.ok_or(StoreError::LedgerCorruption(
                    "accepted command has no event",
                ))?)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            ),
            2 => CommandDisposition::Rejected(rejection_from_i64(rejection_code.ok_or(
                StoreError::LedgerCorruption("rejected command has no rejection code"),
            )?)?),
            _ => return Err(StoreError::LedgerCorruption("unknown command status")),
        };
        commands.push((request, disposition));
    }
    Ok(commands)
}

const MATERIALIZED_TABLES: [&str; 97] = [
    "principals",
    "societies",
    "office_contracts",
    "applications",
    "application_revisions",
    "application_revision_principles",
    "application_revision_north_star_questions",
    "founding_missions",
    "office_occupancies",
    "capability_grants",
    "society_bootstraps",
    "operating_cycles",
    "operating_cycle_admissions",
    "operating_cycle_reconciliations",
    "root_authority_office_sessions",
    "office_turns",
    "budget_envelopes",
    "budget_envelope_constraints",
    "budget_reservations",
    "budget_reservation_charges",
    "cancellation_requests",
    "cost_postmortems",
    "cost_postmortem_resolutions",
    "projects",
    "project_north_star_alignments",
    "project_objectives",
    "project_milestones",
    "project_stop_conditions",
    "tickets",
    "ticket_acceptance_conditions",
    "ticket_prerequisites",
    "objects",
    "object_revisions",
    "observation_revisions",
    "hypothesis_revisions",
    "edges",
    "episodes",
    "adversarial_reviews",
    "review_challenges",
    "review_challenge_responses",
    "review_dispositions",
    "postmortems",
    "postmortem_causal_claims",
    "postmortem_action_proposals",
    "coordination_command_provenance",
    "execution_profiles",
    "actor_configurations",
    "actor_configuration_revisions",
    "context_packs",
    "actor_instances",
    "work_items",
    "leases",
    "attempts",
    "attempt_budget_reservations",
    "actor_attempt_terminal_facts",
    "outcome_obligations",
    "content_seal_receipts",
    "content_objects",
    "forensic_manifests",
    "forensic_manifest_objects",
    "deterministic_evaluator_forensic_manifest_bindings",
    "evaluator_revisions",
    "input_manifests",
    "deterministic_experiments",
    "deterministic_evaluation_receipts",
    "evidence_admissions",
    "supervisor_epochs",
    "workspaces",
    "pi_child_sessions",
    "native_child_spawn_admissions",
    "pi_child_spawn_sidecars",
    "native_child_spawn_invalidations",
    "office_session_budget_reservations",
    "native_children",
    "pi_child_session_protocols",
    "native_child_liveness_observations",
    "process_signal_receipts",
    "native_child_reap_receipts",
    "native_child_recovery_receipts",
    "pi_abort_control_receipts",
    "native_child_stream_seals",
    "cancellation_propagations",
    "cancellation_propagation_children",
    "cancellation_propagation_targets",
    "office_turn_budget_checkpoints",
    "pi_office_turn_prompt_authorizations",
    "pi_office_turn_prompt_deliveries",
    "pi_office_turn_prompt_acceptances",
    "pi_office_turn_usage_receipts",
    "pi_office_turn_usage_failures",
    "pi_office_turn_terminal_receipts",
    "pi_office_session_dispose_authorizations",
    "pi_office_session_dispose_deliveries",
    "pi_office_session_dispose_acceptances",
    "pi_office_session_dispose_usage_receipts",
    "pi_office_session_dispose_usage_failures",
    "pi_office_session_dispose_receipts",
];

fn materialized_state_digest(connection: &Connection) -> Result<Blake3Digest, StoreError> {
    let mut bytes = Vec::with_capacity(4_096);
    for table in MATERIALIZED_TABLES {
        put_bytes(&mut bytes, table.as_bytes());
        let mut statement = connection.prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))?;
        let column_count = statement.column_count();
        put_i64(&mut bytes, column_count as i64);
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            put_i64(&mut bytes, 1);
            for index in 0..column_count {
                match row.get_ref(index)? {
                    ValueRef::Null => put_i64(&mut bytes, 0),
                    ValueRef::Integer(value) => {
                        put_i64(&mut bytes, 1);
                        put_i64(&mut bytes, value);
                    }
                    ValueRef::Real(value) => {
                        put_i64(&mut bytes, 2);
                        bytes.extend_from_slice(&value.to_bits().to_be_bytes());
                    }
                    ValueRef::Text(value) => {
                        put_i64(&mut bytes, 3);
                        put_bytes(&mut bytes, value);
                    }
                    ValueRef::Blob(value) => {
                        put_i64(&mut bytes, 4);
                        put_bytes(&mut bytes, value);
                    }
                }
            }
        }
        put_i64(&mut bytes, 0);
    }
    Ok(Blake3Digest::of_bytes(&bytes))
}

fn decode_command_body(
    connection: &Connection,
    command_row_id: i64,
    kind: CommandKind,
) -> Result<CommandBody, StoreError> {
    let body = match kind {
        CommandKind::CreateSocietyIdentity => {
            let name: String = query_command_value(
                connection,
                "command_create_society_identity",
                "name",
                command_row_id,
            )?;
            CommandBody::CreateSocietyIdentity {
                name: SocietyName::parse(name).map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::InstallRootAuthorityOffice => CommandBody::InstallRootAuthorityOffice,
        CommandKind::InstallFoundingMission => CommandBody::InstallFoundingMission {
            mission: decode_application_mission_input(connection, command_row_id)?,
        },
        CommandKind::AppointInitialRootAuthority => {
            let display_name: String = query_command_value(
                connection,
                "command_appoint_initial_root_authority",
                "actor_display_name",
                command_row_id,
            )?;
            CommandBody::AppointInitialRootAuthority {
                actor_display_name: crate::PrincipalDisplayName::parse(display_name)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::SetR0HardCeiling => CommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::try_from(query_command_value::<i64>(
                connection,
                "command_set_r0_hard_ceiling",
                "ceiling_micros",
                command_row_id,
            )?)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        },
        CommandKind::BootstrapSociety => CommandBody::BootstrapSociety,
        CommandKind::ProposeOperatingCycle => CommandBody::ProposeOperatingCycle {
            treatment: operating_cycle_treatment_from_i64(query_command_value::<i64>(
                connection,
                "command_propose_operating_cycle",
                "treatment",
                command_row_id,
            )?)?,
            budget_ceiling: UsdMicros::try_from(query_command_value::<i64>(
                connection,
                "command_propose_operating_cycle",
                "budget_ceiling_micros",
                command_row_id,
            )?)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        },
        CommandKind::AdmitOperatingCycle => CommandBody::AdmitOperatingCycle {
            cycle_id: query_command_id(
                connection,
                "command_admit_operating_cycle",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::StartRootAuthorityOfficeSession => {
            CommandBody::StartRootAuthorityOfficeSession {
                cycle_id: query_command_id(
                    connection,
                    "command_start_root_authority_office_session",
                    "operating_cycle_id",
                    command_row_id,
                )?,
            }
        }
        CommandKind::RecordOfficeSessionReady => CommandBody::RecordOfficeSessionReady {
            session_id: query_command_id(
                connection,
                "command_record_office_session_ready",
                "root_authority_office_session_id",
                command_row_id,
            )?,
        },
        CommandKind::OpenOfficeTurn => {
            let (session_id, purpose) = connection
                .query_row(
                    "SELECT root_authority_office_session_id, purpose
                     FROM command_open_office_turn WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing office turn command body",
                ))?;
            CommandBody::OpenOfficeTurn {
                session_id: RootAuthorityOfficeSessionId::try_from(session_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                purpose: office_turn_purpose_from_i64(purpose)?,
            }
        }
        CommandKind::SettleOfficeTurn => {
            let (turn, terminal): (i64, i64) = connection
                .query_row(
                    "SELECT office_turn_id, pi_office_turn_terminal_receipt_id
                     FROM command_settle_office_turn WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing office turn settlement command body",
                ))?;
            CommandBody::SettleOfficeTurn {
                turn_id: OfficeTurnId::try_from(turn)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                terminal_receipt_id: PiOfficeTurnTerminalReceiptId::try_from(terminal)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::QuiesceOperatingCycle => CommandBody::QuiesceOperatingCycle {
            cycle_id: query_command_id(
                connection,
                "command_quiesce_operating_cycle",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::RecordCycleDrained => CommandBody::RecordCycleDrained {
            cycle_id: query_command_id(
                connection,
                "command_record_cycle_drained",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::ResumeOperatingCycle => CommandBody::ResumeOperatingCycle {
            cycle_id: query_command_id(
                connection,
                "command_resume_operating_cycle",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::ReconcileOperatingCycle => CommandBody::ReconcileOperatingCycle {
            cycle_id: query_command_id(
                connection,
                "command_reconcile_operating_cycle",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::CloseOperatingCycle => CommandBody::CloseOperatingCycle {
            cycle_id: query_command_id(
                connection,
                "command_close_operating_cycle",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::ReserveBudget => {
            let (cycle_id, amount) = connection
                .query_row(
                    "SELECT operating_cycle_id, amount_micros FROM command_reserve_budget
                     WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing budget reserve command body",
                ))?;
            CommandBody::ReserveBudget {
                cycle_id: OperatingCycleId::try_from(cycle_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                amount: UsdMicros::try_from(amount).map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ReconcileBudget => {
            let (reservation_id, observation_kind, known, unknown, unavailable) = connection
                .query_row(
                    "SELECT budget_reservation_id, observation_kind, known_micros,
                            unknown_reason, unavailable_reason
                     FROM command_reconcile_budget WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, Option<i64>>(2)?,
                            row.get::<_, Option<i64>>(3)?,
                            row.get::<_, Option<i64>>(4)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing budget reconciliation command body",
                ))?;
            CommandBody::ReconcileBudget {
                reservation_id: BudgetReservationId::try_from(reservation_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                observation: cost_observation_from_sql(
                    observation_kind,
                    known,
                    unknown,
                    unavailable,
                )?,
            }
        }
        CommandKind::RequestCancellation => {
            let (cycle_id, mode) = connection
                .query_row(
                    "SELECT operating_cycle_id, cancellation_mode
                     FROM command_request_cancellation WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing cancellation request command body",
                ))?;
            CommandBody::RequestCancellation {
                cycle_id: OperatingCycleId::try_from(cycle_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                mode: cancellation_mode_from_i64(mode)?,
            }
        }
        CommandKind::ReconcileCancellation => CommandBody::ReconcileCancellation {
            cancellation_request_id: query_command_id(
                connection,
                "command_reconcile_cancellation",
                "cancellation_request_id",
                command_row_id,
            )?,
        },
        CommandKind::RecordOfficeSessionTerminal => {
            let (session_id, terminal_state) = connection
                .query_row(
                    "SELECT root_authority_office_session_id, terminal_state
                     FROM command_record_office_session_terminal WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing office session terminal command body",
                ))?;
            CommandBody::RecordOfficeSessionTerminal {
                session_id: RootAuthorityOfficeSessionId::try_from(session_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                terminal_state: office_session_terminal_state_from_i64(terminal_state)?,
            }
        }
        CommandKind::CloseCostPostmortem => {
            let (postmortem_id, resolution) = connection
                .query_row(
                    "SELECT postmortem_id, resolution_kind
                     FROM command_close_cost_postmortem WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing cost postmortem close command body",
                ))?;
            CommandBody::CloseCostPostmortem {
                postmortem_id: CostPostmortemId::try_from(postmortem_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                resolution: cost_postmortem_resolution_from_i64(resolution)?,
            }
        }
        CommandKind::CreateProject => {
            let (cycle, name, revision, change, evidence, boundary, revisit) = connection
                .query_row(
                    "SELECT operating_cycle_id, project_name, application_revision_id,
                            change_answer, improvement_evidence_answer,
                            boundary_commitment_answer, revisit_answer
                     FROM command_create_project WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, String>(5)?,
                            row.get::<_, String>(6)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing project creation command body",
                ))?;
            CommandBody::CreateProject {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_name: crate::ProjectName::parse(name)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                north_star_alignment: ProjectNorthStarAlignment {
                    application_revision_id: ApplicationRevisionId::try_from(revision)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                    change_answer: ProjectNorthStarChangeAnswer::parse(change)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                    improvement_evidence_answer: ProjectNorthStarImprovementEvidenceAnswer::parse(
                        evidence,
                    )
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                    boundary_commitment_answer: ProjectNorthStarBoundaryCommitmentAnswer::parse(
                        boundary,
                    )
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                    revisit_answer: ProjectNorthStarRevisitAnswer::parse(revisit)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                },
            }
        }
        CommandKind::CharterProject => {
            let row = query_command_six(connection, "command_charter_project", command_row_id)?;
            CommandBody::CharterProject {
                operating_cycle_id: OperatingCycleId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                objective: crate::ProjectObjectiveText::parse(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                initial_milestone: crate::ProjectMilestoneName::parse(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                stop_condition: crate::ProjectStopConditionText::parse(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::TransitionProject => {
            let (cycle, id, state) =
                query_command_three(connection, "command_transition_project", command_row_id)?;
            CommandBody::TransitionProject {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
                target: project_state_from_i64(state)?,
            }
        }
        CommandKind::CompleteProjectMilestone => {
            let (cycle, id) = query_command_pair(
                connection,
                "command_complete_project_milestone",
                command_row_id,
            )?;
            CommandBody::CompleteProjectMilestone {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_milestone_id: ProjectMilestoneId::try_from(id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ReopenProject => {
            let (cycle, id) =
                query_command_pair(connection, "command_reopen_project", command_row_id)?;
            CommandBody::ReopenProject {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::CreateTicket => {
            let (cycle, project, title, condition, prerequisite) =
                query_create_ticket(connection, command_row_id)?;
            CommandBody::CreateTicket {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_title: crate::TicketTitle::parse(title)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                acceptance_condition: crate::TicketAcceptanceConditionText::parse(condition)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                prerequisite_ticket_id: prerequisite
                    .map(TicketId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::TransitionTicket => {
            let (cycle, id, state) =
                query_command_three(connection, "command_transition_ticket", command_row_id)?;
            CommandBody::TransitionTicket {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
                target: ticket_state_from_i64(state)?,
            }
        }
        CommandKind::AddGraphObjectRevision => {
            let (cycle, project, episode, object) =
                query_graph_revision_command(connection, command_row_id)?;
            CommandBody::AddGraphObjectRevision {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                causal_episode_id: episode
                    .map(CausalEpisodeId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                graph_object_id: object
                    .map(GraphObjectId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                body: query_graph_revision_command_body(connection, command_row_id)?,
            }
        }
        CommandKind::CommitGraphRevision => {
            let (cycle, id) =
                query_command_pair(connection, "command_commit_graph_revision", command_row_id)?;
            CommandBody::CommitGraphRevision {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                graph_revision_id: GraphRevisionId::try_from(id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AddGraphEdge => {
            let (cycle, project, from, to, kind) = query_edge_command(connection, command_row_id)?;
            CommandBody::AddGraphEdge {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                from_graph_revision_id: GraphRevisionId::try_from(from)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                to_graph_revision_id: GraphRevisionId::try_from(to)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                edge_kind: graph_edge_kind_from_i64(kind)?,
            }
        }
        CommandKind::CreateEpisode => {
            let (cycle, project) =
                query_command_pair(connection, "command_create_episode", command_row_id)?;
            CommandBody::CreateEpisode {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::TransitionEpisode => {
            let (cycle, episode, state) =
                query_command_three(connection, "command_transition_episode", command_row_id)?;
            CommandBody::TransitionEpisode {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                causal_episode_id: CausalEpisodeId::try_from(episode)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                target: episode_state_from_i64(state)?,
            }
        }
        CommandKind::ReopenEpisode => {
            let (cycle, episode) =
                query_command_pair(connection, "command_reopen_episode", command_row_id)?;
            CommandBody::ReopenEpisode {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                causal_episode_id: CausalEpisodeId::try_from(episode)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RequestAdversarialReview => {
            let (cycle, project, revision) = query_command_three(
                connection,
                "command_request_adversarial_review",
                command_row_id,
            )?;
            CommandBody::RequestAdversarialReview {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                target_graph_revision_id: GraphRevisionId::try_from(revision)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AssignAdversarialReviewer => {
            let (cycle, review, reviewer, actor, attempt): (i64, i64, i64, i64, i64) = connection.query_row(
                "SELECT operating_cycle_id, adversarial_review_id, reviewer_principal_id, reviewer_actor_instance_id, reviewer_actor_attempt_id FROM command_assign_adversarial_reviewer WHERE command_row_id = ?1",
                [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing reviewer assignment command body"))?;
            CommandBody::AssignAdversarialReviewer {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                adversarial_review_id: AdversarialReviewId::try_from(review)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_principal_id: PrincipalId::try_from(reviewer)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_actor_instance_id: ActorInstanceId::try_from(actor)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::SubmitReviewChallenge => {
            let (cycle, review, revision, author, severity, hypothesis) =
                query_review_submit(connection, command_row_id)?;
            CommandBody::SubmitReviewChallenge {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                adversarial_review_id: AdversarialReviewId::try_from(review)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                target_graph_revision_id: GraphRevisionId::try_from(revision)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                author_principal_id: PrincipalId::try_from(author)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                severity: review_challenge_severity_from_i64(severity)?,
                failure_hypothesis: crate::ReviewFailureHypothesis::parse(hypothesis)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RespondToReviewChallenge => {
            let (cycle, challenge, response): (i64, i64, String) = connection.query_row("SELECT operating_cycle_id, review_challenge_id, response_text FROM command_respond_to_review_challenge WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing review response command body"))?;
            CommandBody::RespondToReviewChallenge {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                review_challenge_id: ReviewChallengeId::try_from(challenge)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                response: crate::ReviewResponseText::parse(response)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::DispositionReviewChallenge => {
            let (cycle, challenge, disposition) = query_command_three(
                connection,
                "command_disposition_review_challenge",
                command_row_id,
            )?;
            CommandBody::DispositionReviewChallenge {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                review_challenge_id: ReviewChallengeId::try_from(challenge)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                disposition: review_disposition_kind_from_i64(disposition)?,
            }
        }
        CommandKind::ResolveAdversarialReview => {
            let (cycle, review, resolution) = query_command_three(
                connection,
                "command_resolve_adversarial_review",
                command_row_id,
            )?;
            CommandBody::ResolveAdversarialReview {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                adversarial_review_id: AdversarialReviewId::try_from(review)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                resolution: review_resolution_kind_from_i64(resolution)?,
            }
        }
        CommandKind::TriggerPostmortem => {
            let (cycle, project, episode) = query_postmortem_trigger(connection, command_row_id)?;
            CommandBody::TriggerPostmortem {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                causal_episode_id: episode
                    .map(CausalEpisodeId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordPostmortemCausalClaim => {
            let (cycle, postmortem, kind, text) = query_postmortem_text_command(
                connection,
                "command_record_postmortem_causal_claim",
                command_row_id,
            )?;
            CommandBody::RecordPostmortemCausalClaim {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                postmortem_id: PostmortemId::try_from(postmortem)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                claim_kind: postmortem_causal_claim_kind_from_i64(kind)?,
                claim: crate::PostmortemCausalClaimText::parse(text)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ProposePostmortemAction => {
            let (cycle, postmortem, kind, text) = query_postmortem_text_command(
                connection,
                "command_propose_postmortem_action",
                command_row_id,
            )?;
            CommandBody::ProposePostmortemAction {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                postmortem_id: PostmortemId::try_from(postmortem)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                action_kind: postmortem_action_kind_from_i64(kind)?,
                action: crate::PostmortemActionProposalText::parse(text)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ClosePostmortem => {
            let (cycle, postmortem) =
                query_command_pair(connection, "command_close_postmortem", command_row_id)?;
            CommandBody::ClosePostmortem {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                postmortem_id: PostmortemId::try_from(postmortem)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RegisterActorConfiguration => {
            let (name, model, attractor): (String, i64, i64) = connection.query_row("SELECT configuration_name, model_policy, primary_attractor FROM command_register_actor_configuration WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing actor configuration command body"))?;
            CommandBody::RegisterActorConfiguration {
                configuration_name: crate::ActorConfigurationName::parse(name)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                model_policy: actor_model_policy_from_i64(model)?,
                primary_attractor: developmental_attractor_from_i64(attractor)?,
            }
        }
        CommandKind::RegisterContextPack => {
            let (cycle, purpose, digest): (i64, i64, Vec<u8>) = connection.query_row("SELECT operating_cycle_id, purpose, rendering_digest FROM command_register_context_pack WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing context pack command body"))?;
            CommandBody::RegisterContextPack {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                purpose: context_pack_purpose_from_i64(purpose)?,
                rendering_digest: digest_from_stored_bytes(&digest)?,
            }
        }
        CommandKind::AdmitActorInstance => {
            let (cycle, revision, profile, display): (i64, i64, i64, String) = connection.query_row("SELECT operating_cycle_id, actor_configuration_revision_id, execution_profile_id, actor_display_name FROM command_admit_actor_instance WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing actor admission command body"))?;
            CommandBody::AdmitActorInstance {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_configuration_revision_id: ActorConfigurationRevisionId::try_from(revision)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                execution_profile_id: ExecutionProfileId::try_from(profile)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_display_name: crate::PrincipalDisplayName::parse(display)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AdmitTicket => {
            let (cycle, ticket) =
                query_command_pair(connection, "command_admit_ticket", command_row_id)?;
            CommandBody::AdmitTicket {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RegisterWorkItem => {
            let (cycle, ticket, actor, context, kind, review, assignment): (i64, i64, i64, i64, i64, Option<i64>, String) = connection.query_row("SELECT operating_cycle_id, ticket_id, actor_instance_id, context_pack_id, work_kind, adversarial_review_id, assignment_text FROM command_register_work_item WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing work item command body"))?;
            CommandBody::RegisterWorkItem {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_instance_id: ActorInstanceId::try_from(actor)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                context_pack_id: ContextPackId::try_from(context)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_kind: work_item_kind_from_i64(kind)?,
                adversarial_review_id: review
                    .map(AdversarialReviewId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                assignment: crate::WorkAssignmentText::parse(assignment)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ClaimWorkItem => {
            let (cycle, work) =
                query_command_pair(connection, "command_claim_work_item", command_row_id)?;
            CommandBody::ClaimWorkItem {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::StartActorAttempt => {
            let (cycle, work, amount): (i64, i64, i64) = connection.query_row("SELECT operating_cycle_id, work_item_id, reservation_micros FROM command_start_actor_attempt WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing actor attempt command body"))?;
            CommandBody::StartActorAttempt {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reservation_amount: UsdMicros::try_from(amount)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AttestActorAttemptTerminal => {
            let (attempt, terminal) = query_command_pair(
                connection,
                "command_attest_actor_attempt_terminal",
                command_row_id,
            )?;
            CommandBody::AttestActorAttemptTerminal {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                terminal_kind: actor_attempt_terminal_kind_from_i64(terminal)?,
            }
        }
        CommandKind::ValidateTicketAttempt => {
            let (cycle, attempt) = query_command_pair(
                connection,
                "command_validate_ticket_attempt",
                command_row_id,
            )?;
            CommandBody::ValidateTicketAttempt {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RetryActorAttempt => {
            let (cycle, attempt) =
                query_command_pair(connection, "command_retry_actor_attempt", command_row_id)?;
            CommandBody::RetryActorAttempt {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::CompleteTicket => {
            let (cycle, attempt) =
                query_command_pair(connection, "command_complete_ticket", command_row_id)?;
            CommandBody::CompleteTicket {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ExpireWorkLease => CommandBody::ExpireWorkLease {
            work_lease_id: query_command_id(
                connection,
                "command_expire_work_lease",
                "work_lease_id",
                command_row_id,
            )?,
        },
        CommandKind::CancelActorAttempt => {
            let (attempt, reason) =
                query_command_pair(connection, "command_cancel_actor_attempt", command_row_id)?;
            CommandBody::CancelActorAttempt {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reason: actor_attempt_cancellation_reason_from_i64(reason)?,
            }
        }
        CommandKind::RegisterOutcomeObligation => {
            let (cycle, project, obligation): (i64, i64, String) = connection.query_row("SELECT operating_cycle_id, project_id, obligation_text FROM command_register_outcome_obligation WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing outcome obligation command body"))?;
            CommandBody::RegisterOutcomeObligation {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                obligation: crate::OutcomeObligationText::parse(obligation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ResolveOutcomeObligation => {
            let (cycle, obligation, disposition) = query_command_three(
                connection,
                "command_resolve_outcome_obligation",
                command_row_id,
            )?;
            CommandBody::ResolveOutcomeObligation {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                outcome_obligation_id: OutcomeObligationId::try_from(obligation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                disposition: outcome_obligation_disposition_from_i64(disposition)?,
            }
        }
        CommandKind::RecordContentSealReceipt => {
            let digest: Vec<u8> = connection.query_row("SELECT digest FROM command_record_content_seal_receipt WHERE command_row_id = ?1", [command_row_id], |row| row.get(0)).optional()?.ok_or(StoreError::LedgerCorruption("missing content seal command body"))?;
            CommandBody::RecordContentSealReceipt {
                digest: digest_from_stored_bytes(&digest)?,
            }
        }
        CommandKind::RegisterContentObject => {
            let receipt: i64 = connection.query_row("SELECT content_seal_receipt_id FROM command_register_content_object WHERE command_row_id = ?1", [command_row_id], |row| row.get(0)).optional()?.ok_or(StoreError::LedgerCorruption("missing content object command body"))?;
            CommandBody::RegisterContentObject {
                content_seal_receipt_id: ContentSealReceiptId::try_from(receipt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RegisterForensicManifest => {
            let (cycle, experiment, policy, retention, output): (i64, i64, i64, i64, i64) = connection.query_row("SELECT operating_cycle_id, producing_deterministic_experiment_id, capture_policy, retention_access_class, evaluator_output_content_object_id FROM command_register_forensic_manifest WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing forensic manifest command body"))?;
            CommandBody::RegisterForensicManifest {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                producing_deterministic_experiment_id: DeterministicExperimentId::try_from(
                    experiment,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                capture_policy: forensic_manifest_capture_policy_from_i64(policy)?,
                retention_access_class: retention_access_class_from_i64(retention)?,
                evaluator_output_content_object_id: ContentObjectId::try_from(output)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RegisterDeterministicEvaluatorForensicManifest => {
            let (cycle, admission): (i64, i64) = connection
                .query_row(
                    "SELECT operating_cycle_id, native_child_spawn_admission_id
                       FROM command_register_deterministic_evaluator_forensic_manifest
                      WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing deterministic evaluator forensic manifest command body",
                ))?;
            CommandBody::RegisterDeterministicEvaluatorForensicManifest {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_child_spawn_admission_id: NativeChildSpawnAdmissionId::try_from(admission)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RegisterDeterministicExperiment => {
            let row: (i64,i64,i64,i64,i64,i64) = connection.query_row("SELECT operating_cycle_id, project_id, ticket_id, target_graph_revision_id, evaluator_content_object_id, input_manifest_content_object_id FROM command_register_deterministic_experiment WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing deterministic experiment command body"))?;
            CommandBody::RegisterDeterministicExperiment {
                operating_cycle_id: OperatingCycleId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(row.2).map_err(|_| StoreError::InvalidStoredValue)?,
                target_graph_revision_id: GraphRevisionId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_content_object_id: ContentObjectId::try_from(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                input_manifest_content_object_id: ContentObjectId::try_from(row.5)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordDeterministicEvaluationReceipt => {
            let row: (i64,i64,i64,i64,i64,i64) = connection.query_row("SELECT operating_cycle_id, deterministic_experiment_id, evaluator_revision_id, input_manifest_id, forensic_manifest_id, evaluator_output_content_object_id FROM command_record_deterministic_evaluation_receipt WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing deterministic evaluation receipt command body"))?;
            CommandBody::RecordDeterministicEvaluationReceipt {
                operating_cycle_id: OperatingCycleId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_experiment_id: DeterministicExperimentId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_revision_id: EvaluatorRevisionId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                input_manifest_id: InputManifestId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                forensic_manifest_id: ForensicManifestId::try_from(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_output_content_object_id: ContentObjectId::try_from(row.5)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AdmitDeterministicEvidence => {
            let row: (i64,i64,i64,i64,i64,i64,i64,i64,i64,String) = connection.query_row("SELECT operating_cycle_id, deterministic_evaluation_receipt_id, deterministic_experiment_id, evaluator_revision_id, input_manifest_id, evaluator_output_content_object_id, related_graph_revision_id, semantic_role, applicability, limitation_text FROM command_admit_deterministic_evidence WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?,row.get(9)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing deterministic evidence command body"))?;
            CommandBody::AdmitDeterministicEvidence {
                operating_cycle_id: OperatingCycleId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId::try_from(
                    row.1,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_experiment_id: DeterministicExperimentId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_revision_id: EvaluatorRevisionId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                input_manifest_id: InputManifestId::try_from(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_output_content_object_id: ContentObjectId::try_from(row.5)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                related_graph_revision_id: GraphRevisionId::try_from(row.6)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                semantic_role: evidence_semantic_role_from_i64(row.7)?,
                applicability: evidence_applicability_from_i64(row.8)?,
                limitation: EvidenceLimitationText::parse(row.9)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::FinalizeDeterministicExperiment => {
            let (cycle, experiment) = query_command_pair(
                connection,
                "command_finalize_deterministic_experiment",
                command_row_id,
            )?;
            CommandBody::FinalizeDeterministicExperiment {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_experiment_id: DeterministicExperimentId::try_from(experiment)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AdmitDeterministicEvaluatorNativeChild => {
            let row: (i64,i64,i64,i64,i64,String,String,i64,String) = connection.query_row(
                "SELECT operating_cycle_id, deterministic_experiment_id, evaluator_revision_id, input_manifest_id, execution_profile_id, native_workspace_id, canonical_workspace_path, supervisor_epoch_id, supervisor_epoch_identity FROM command_admit_deterministic_evaluator_native_child WHERE command_row_id = ?1",
                [command_row_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing evaluator native-child admission command body"))?;
            CommandBody::AdmitDeterministicEvaluatorNativeChild {
                operating_cycle_id: OperatingCycleId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_experiment_id: DeterministicExperimentId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_revision_id: EvaluatorRevisionId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                input_manifest_id: InputManifestId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                execution_profile_id: ExecutionProfileId::try_from(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_workspace_id: NativeWorkspaceId::parse(row.5)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                canonical_workspace_path: CanonicalWorkspacePath::parse(row.6)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                supervisor_epoch_id: SupervisorEpochId::try_from(row.7)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                supervisor_epoch_identity: SupervisorEpochIdentity::parse(row.8)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordDeterministicEvaluatorNativeChildSpawn => {
            let row: (i64,String,i32,i32) = connection.query_row(
                "SELECT native_child_spawn_admission_id, child_identity, direct_child_pid, process_group_id FROM command_record_deterministic_evaluator_native_child_spawn WHERE command_row_id = ?1",
                [command_row_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing evaluator native-child spawn command body"))?;
            CommandBody::RecordDeterministicEvaluatorNativeChildSpawn {
                native_child_spawn_admission_id: NativeChildSpawnAdmissionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                child_identity: SupervisedChildIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                direct_child_pid: NativeChildPid::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                process_group_id: OwnedProcessGroupId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::OpenSupervisorEpoch => {
            let row:(i64,String)=connection.query_row("SELECT supervisor_epoch_id, supervisor_epoch_identity FROM command_open_supervisor_epoch WHERE command_row_id = ?1",[command_row_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing supervisor epoch command body"))?;
            CommandBody::OpenSupervisorEpoch {
                supervisor_epoch_id: SupervisorEpochId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                supervisor_epoch_identity: SupervisorEpochIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AdmitPiChildSpawn => {
            let row: StoredPiChildAdmissionCommand = connection.query_row("SELECT operating_cycle_id, actor_attempt_id, root_authority_office_session_id, budget_reservation_id, execution_profile_id, native_workspace_id, canonical_workspace_path, supervisor_epoch_id, supervisor_epoch_identity, pi_session_identity, spawn_nonce FROM command_admit_pi_child_spawn WHERE command_row_id = ?1",[command_row_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi child admission command body"))?;
            CommandBody::AdmitPiChildSpawn {
                operating_cycle_id: OperatingCycleId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                owner: decode_child_owner(row.1, row.2)?,
                budget_reservation_id: BudgetReservationId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                execution_profile_id: ExecutionProfileId::try_from(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                native_workspace_id: NativeWorkspaceId::parse(row.5)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                canonical_workspace_path: CanonicalWorkspacePath::parse(row.6)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                supervisor_epoch_id: SupervisorEpochId::try_from(row.7)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                supervisor_epoch_identity: SupervisorEpochIdentity::parse(row.8)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                pi_session_identity: PiBoundarySessionIdentity::parse(row.9)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                spawn_nonce: SpawnNonce::parse(row.10)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordInertChildSpawn => {
            let row:(i64,String,i64,i64)=connection.query_row("SELECT native_child_spawn_admission_id, child_identity, direct_child_pid, process_group_id FROM command_record_inert_pi_child_spawn WHERE command_row_id = ?1",[command_row_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing inert Pi child command body"))?;
            CommandBody::RecordInertChildSpawn {
                native_child_spawn_admission_id: NativeChildSpawnAdmissionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                child_identity: SupervisedChildIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                direct_child_pid: NativeChildPid::try_from(
                    i32::try_from(row.2).map_err(|_| StoreError::InvalidStoredValue)?,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                process_group_id: OwnedProcessGroupId::try_from(
                    i32::try_from(row.3).map_err(|_| StoreError::InvalidStoredValue)?,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordPiAdapterReady => {
            let row:(i64,String,String)=connection.query_row("SELECT native_child_id, pi_session_identity, spawn_nonce FROM command_record_pi_adapter_ready WHERE command_row_id = ?1",[command_row_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing adapter ready command body"))?;
            CommandBody::RecordPiAdapterReady {
                native_child_id: NativeChildId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                pi_session_identity: PiBoundarySessionIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                spawn_nonce: SpawnNonce::parse(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AuthorizePiCreateSession | CommandKind::RecordPiCreateSessionDelivery => {
            let row:(i64,String,Vec<u8>)=connection.query_row(if kind == CommandKind::AuthorizePiCreateSession { "SELECT native_child_id, correlation_identity, create_request_digest FROM command_authorize_pi_create_session WHERE command_row_id = ?1" } else { "SELECT native_child_id, correlation_identity, create_request_digest FROM command_record_pi_create_session_delivery WHERE command_row_id = ?1" },[command_row_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Create command body"))?;
            let native_child_id =
                NativeChildId::try_from(row.0).map_err(|_| StoreError::InvalidStoredValue)?;
            let correlation_identity =
                PiCorrelationIdentity::parse(row.1).map_err(|_| StoreError::InvalidStoredValue)?;
            let create_request_digest = digest_from_stored_bytes(&row.2)?;
            if kind == CommandKind::AuthorizePiCreateSession {
                CommandBody::AuthorizePiCreateSession {
                    native_child_id,
                    correlation_identity,
                    create_request_digest,
                }
            } else {
                CommandBody::RecordPiCreateSessionDelivery {
                    native_child_id,
                    correlation_identity,
                    create_request_digest,
                }
            }
        }
        CommandKind::RecordPiSessionReady => {
            let row:(i64,String)=connection.query_row("SELECT native_child_id, pi_session_identity FROM command_record_pi_session_ready WHERE command_row_id = ?1",[command_row_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi ready command body"))?;
            CommandBody::RecordPiSessionReady {
                native_child_id: NativeChildId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                pi_session_identity: PiBoundarySessionIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordPiAbortControlDelivery => {
            let row: (i64, i64, String, Vec<u8>, i64) = connection.query_row(
                "SELECT native_child_id, cancellation_propagation_id, correlation_identity, abort_command_digest, physical_write_outcome FROM command_record_pi_abort_control_delivery WHERE command_row_id = ?1",
                [command_row_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Abort command body"))?;
            CommandBody::RecordPiAbortControlDelivery {
                native_child_id: NativeChildId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cancellation_propagation_id: CancellationPropagationId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                abort_command_digest: digest_from_stored_bytes(&row.3)?,
                outcome: pi_abort_control_write_outcome_from_i64(row.4)?,
            }
        }
        CommandKind::RecordChildStreamSeal => {
            let row:(i64,i64,Vec<u8>,i64,i64)=connection.query_row("SELECT native_child_id, stream_kind, full_observed_digest, retained_content_object_id, completeness FROM command_record_child_stream_seal WHERE command_row_id = ?1",[command_row_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing stream seal command body"))?;
            CommandBody::RecordChildStreamSeal {
                native_child_id: NativeChildId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                stream_kind: child_stream_kind_from_i64(row.1)?,
                full_observed_digest: digest_from_stored_bytes(&row.2)?,
                retained_content_object_id: ContentObjectId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                completeness: child_stream_completeness_from_i64(row.4)?,
            }
        }
        CommandKind::RecordChildProcessLiveness => {
            let (child, liveness) = query_command_pair(
                connection,
                "command_record_child_process_liveness",
                command_row_id,
            )?;
            CommandBody::RecordChildProcessLiveness {
                native_child_id: NativeChildId::try_from(child)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                liveness: process_group_liveness_from_i64(liveness)?,
            }
        }
        CommandKind::RecordProcessSignalReceipt => {
            let row:(i64,i64,i64,i64,i64,Option<i64>)=connection.query_row("SELECT native_child_id, signal_action, delivery, observed_liveness, cause_kind, cancellation_propagation_id FROM command_record_process_signal_receipt WHERE command_row_id = ?1",[command_row_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing signal command body"))?;
            CommandBody::RecordProcessSignalReceipt {
                native_child_id: NativeChildId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                action: process_signal_action_from_i64(row.1)?,
                delivery: process_signal_delivery_from_i64(row.2)?,
                observed_liveness: process_group_liveness_from_i64(row.3)?,
                cause: process_signal_cause_from_sql(row.4, row.5)?,
            }
        }
        CommandKind::RecordDirectChildReap => {
            let row:(i64,i64,Option<i64>,i64,i64)=connection.query_row("SELECT native_child_id, wait_status_kind, status_value, group_liveness_before_cleanup, group_liveness_after_cleanup FROM command_record_direct_child_reap WHERE command_row_id = ?1",[command_row_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing reap command body"))?;
            CommandBody::RecordDirectChildReap {
                native_child_id: NativeChildId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                wait_status: direct_wait_status_from_sql(row.1, row.2)?,
                group_liveness_before_cleanup: process_group_liveness_from_i64(row.3)?,
                group_liveness_after_cleanup: process_group_liveness_from_i64(row.4)?,
            }
        }
        CommandKind::RecordChildRecovery => {
            let (child, obs, liveness):(i64,i64,i64) = connection.query_row("SELECT native_child_id, observation, group_liveness_after_restart FROM command_record_child_recovery WHERE command_row_id = ?1", [command_row_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing recovery command body"))?;
            CommandBody::RecordChildRecovery {
                native_child_id: NativeChildId::try_from(child)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                observation: child_recovery_observation_from_i64(obs)?,
                group_liveness_after_restart: process_group_liveness_from_i64(liveness)?,
            }
        }
        CommandKind::FinalizeChildProcess => CommandBody::FinalizeChildProcess {
            native_child_id: query_command_id(
                connection,
                "command_finalize_child_process",
                "native_child_id",
                command_row_id,
            )?,
        },
        CommandKind::BeginCancellationPropagation => CommandBody::BeginCancellationPropagation {
            cancellation_request_id: query_command_id(
                connection,
                "command_begin_cancellation_propagation",
                "cancellation_request_id",
                command_row_id,
            )?,
        },
        CommandKind::ReconcileCancellationPropagation => {
            CommandBody::ReconcileCancellationPropagation {
                cancellation_propagation_id: query_command_id(
                    connection,
                    "command_reconcile_cancellation_propagation",
                    "cancellation_propagation_id",
                    command_row_id,
                )?,
            }
        }
        CommandKind::RecordNativeChildNotSpawned => {
            let (admission, reason): (i64, i64) = connection.query_row(
                "SELECT native_child_spawn_admission_id, reason FROM command_record_native_child_not_spawned WHERE command_row_id = ?1",
                [command_row_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing native-child invalidation command body"))?;
            CommandBody::RecordNativeChildNotSpawned {
                native_child_spawn_admission_id: NativeChildSpawnAdmissionId::try_from(admission)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reason: native_child_not_spawned_reason_from_i64(reason)?,
            }
        }
        CommandKind::AuthorizePiOfficeTurnPrompt => {
            let row: (i64, String, i64, Vec<u8>, i64) = connection.query_row(
                "SELECT office_turn_id, correlation_identity, prompt_content_object_id, prompt_digest, frontier_event_id FROM command_authorize_pi_office_turn_prompt WHERE command_row_id = ?1",
                [command_row_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Prompt authorization command body"))?;
            CommandBody::AuthorizePiOfficeTurnPrompt {
                office_turn_id: OfficeTurnId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                prompt_content_object_id: ContentObjectId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                prompt_digest: digest_from_stored_bytes(&row.3)?,
                frontier_event_id: EventId::try_from(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordPiOfficeTurnPromptDelivery => {
            let row: (i64, String, Vec<u8>) = connection.query_row(
                "SELECT office_turn_id, correlation_identity, prompt_digest FROM command_record_pi_office_turn_prompt_delivery WHERE command_row_id = ?1",
                [command_row_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Prompt delivery command body"))?;
            CommandBody::RecordPiOfficeTurnPromptDelivery {
                office_turn_id: OfficeTurnId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                prompt_digest: digest_from_stored_bytes(&row.2)?,
            }
        }
        CommandKind::RecordPiOfficeTurnPromptAccepted => {
            let row: (i64, String, i64) = connection.query_row(
                "SELECT office_turn_id, correlation_identity, command_result_sequence FROM command_record_pi_office_turn_prompt_accepted WHERE command_row_id = ?1",
                [command_row_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Prompt acceptance command body"))?;
            CommandBody::RecordPiOfficeTurnPromptAccepted {
                office_turn_id: OfficeTurnId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                command_result_sequence: PiProtocolSequence::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordPiOfficeTurnUsage => {
            let row: (i64, String, i64, i64, i64, i64, i64, i64, Vec<u8>, i64) = connection.query_row(
                "SELECT office_turn_id, correlation_identity, protocol_sequence, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, total_tokens, provider_cost_binary64, cumulative_ceiling_micros FROM command_record_pi_office_turn_usage WHERE command_row_id = ?1",
                [command_row_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office usage command body"))?;
            CommandBody::RecordPiOfficeTurnUsage {
                office_turn_id: OfficeTurnId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                protocol_sequence: PiProtocolSequence::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                usage: pi_cumulative_usage_from_sql(
                    row.3, row.4, row.5, row.6, row.7, &row.8, row.9,
                )?,
            }
        }
        CommandKind::RecordPiOfficeTurnUsageFailure => {
            let row: (i64, String, i64, i64, Option<i64>, Option<i64>) = connection.query_row(
                "SELECT office_turn_id, correlation_identity, protocol_sequence, failure_kind, unknown_reason, unavailable_reason FROM command_record_pi_office_turn_usage_failure WHERE command_row_id = ?1",
                [command_row_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office usage failure command body"))?;
            CommandBody::RecordPiOfficeTurnUsageFailure {
                office_turn_id: OfficeTurnId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                protocol_sequence: PiProtocolSequence::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                failure: pi_office_turn_usage_failure_from_sql(row.3, row.4, row.5)?,
            }
        }
        CommandKind::RecordPiOfficeTurnTerminal => {
            let row: (i64, String, i64, Option<i64>, i64, i64, i64, i64, i64) = connection.query_row(
                "SELECT office_turn_id, correlation_identity, terminal_evidence_kind, agent_settled_sequence, final_accounting_sequence, settled_sequence, disposition, assistant_outcome, transcript_disposition FROM command_record_pi_office_turn_terminal WHERE command_row_id = ?1",
                [command_row_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office terminal command body"))?;
            CommandBody::RecordPiOfficeTurnTerminal {
                office_turn_id: OfficeTurnId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                terminal_evidence: pi_office_turn_terminal_evidence_from_sql(row.2, row.3, row.4)?,
                settled_sequence: PiProtocolSequence::try_from(row.5)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                disposition: pi_office_turn_disposition_from_i64(row.6)?,
                assistant_outcome: pi_office_turn_assistant_outcome_from_i64(row.7)?,
                transcript_disposition: pi_office_turn_transcript_disposition_from_i64(row.8)?,
            }
        }
        CommandKind::AuthorizePiOfficeSessionDispose => {
            let row: (i64, String) = connection
                .query_row(
                    "SELECT root_authority_office_session_id, correlation_identity
                 FROM command_authorize_pi_office_session_dispose WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing Pi Office Dispose authorization command body",
                ))?;
            CommandBody::AuthorizePiOfficeSessionDispose {
                session_id: RootAuthorityOfficeSessionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordPiOfficeSessionDisposeDelivery => {
            let row: (i64, String) = connection
                .query_row(
                    "SELECT root_authority_office_session_id, correlation_identity
                 FROM command_record_pi_office_session_dispose_delivery WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing Pi Office Dispose delivery command body",
                ))?;
            CommandBody::RecordPiOfficeSessionDisposeDelivery {
                session_id: RootAuthorityOfficeSessionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordPiOfficeSessionDisposeAccepted => {
            let row: (i64, String, i64) = connection.query_row(
                "SELECT root_authority_office_session_id, correlation_identity, command_result_sequence
                 FROM command_record_pi_office_session_dispose_accepted WHERE command_row_id = ?1",
                [command_row_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Dispose acceptance command body"))?;
            CommandBody::RecordPiOfficeSessionDisposeAccepted {
                session_id: RootAuthorityOfficeSessionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                command_result_sequence: PiProtocolSequence::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordPiOfficeSessionDisposeUsage => {
            let row: (i64, String, i64, i64, i64, i64, i64, i64, Vec<u8>, i64) = connection.query_row(
                "SELECT root_authority_office_session_id, correlation_identity, protocol_sequence,
                        input_tokens, output_tokens, cache_read_tokens, cache_write_tokens,
                        total_tokens, provider_cost_binary64, cumulative_ceiling_micros
                 FROM command_record_pi_office_session_dispose_usage WHERE command_row_id = ?1",
                [command_row_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Dispose usage command body"))?;
            CommandBody::RecordPiOfficeSessionDisposeUsage {
                session_id: RootAuthorityOfficeSessionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                protocol_sequence: PiProtocolSequence::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                usage: pi_cumulative_usage_from_sql(
                    row.3, row.4, row.5, row.6, row.7, &row.8, row.9,
                )?,
            }
        }
        CommandKind::RecordPiOfficeSessionDisposeUsageFailure => {
            let row: (i64, String, i64, i64, Option<i64>, Option<i64>) = connection.query_row(
                "SELECT root_authority_office_session_id, correlation_identity, protocol_sequence,
                        failure_kind, unknown_reason, unavailable_reason
                 FROM command_record_pi_office_session_dispose_usage_failure WHERE command_row_id = ?1",
                [command_row_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Dispose usage failure command body"))?;
            CommandBody::RecordPiOfficeSessionDisposeUsageFailure {
                session_id: RootAuthorityOfficeSessionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                protocol_sequence: PiProtocolSequence::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                failure: pi_office_turn_usage_failure_from_sql(row.3, row.4, row.5)?,
            }
        }
        CommandKind::RecordPiOfficeSessionDisposed => {
            let row: PiOfficeSessionDisposedCommandSqlRow = connection.query_row(
                "SELECT root_authority_office_session_id, correlation_identity, disposed_sequence,
                        transcript_kind, session_file, session_file_digest,
                        transcript_content_object_id, first_user_prompt_kind,
                        first_user_prompt_digest
                 FROM command_record_pi_office_session_disposed WHERE command_row_id = ?1",
                [command_row_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing Pi Office Disposed command body"))?;
            CommandBody::RecordPiOfficeSessionDisposed {
                session_id: RootAuthorityOfficeSessionId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                correlation_identity: PiCorrelationIdentity::parse(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                disposed_sequence: PiProtocolSequence::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                transcript_receipt: pi_office_session_transcript_receipt_from_sql(
                    row.3, row.4, row.5, row.6, row.7, row.8,
                )?,
            }
        }
    };
    Ok(body)
}

fn query_command_value<T>(
    connection: &Connection,
    table: &str,
    column: &str,
    command_row_id: i64,
) -> Result<T, StoreError>
where
    T: FromSql,
{
    let query = format!("SELECT {column} FROM {table} WHERE command_row_id = ?1");
    connection
        .query_row(&query, [command_row_id], |row| row.get(0))
        .optional()?
        .ok_or(StoreError::LedgerCorruption("missing simple command body"))
}

fn query_event_pair(
    connection: &Connection,
    table: &str,
    event_id: i64,
) -> Result<(i64, i64), StoreError> {
    let query = format!("SELECT * FROM {table} WHERE event_id = ?1");
    connection
        .query_row(&query, [event_id], |row| Ok((row.get(1)?, row.get(2)?)))
        .optional()?
        .ok_or(StoreError::LedgerCorruption("missing two-field event body"))
}

fn query_command_pair(
    connection: &Connection,
    table: &str,
    id: i64,
) -> Result<(i64, i64), StoreError> {
    let query = format!("SELECT * FROM {table} WHERE command_row_id = ?1");
    connection
        .query_row(&query, [id], |r| Ok((r.get(1)?, r.get(2)?)))
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing two-field command body",
        ))
}
fn query_command_three(
    connection: &Connection,
    table: &str,
    id: i64,
) -> Result<(i64, i64, i64), StoreError> {
    let query = format!("SELECT * FROM {table} WHERE command_row_id = ?1");
    connection
        .query_row(&query, [id], |r| Ok((r.get(1)?, r.get(2)?, r.get(3)?)))
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing three-field command body",
        ))
}
fn query_command_six(
    connection: &Connection,
    table: &str,
    id: i64,
) -> Result<(i64, i64, String, String, String), StoreError> {
    let query = format!("SELECT * FROM {table} WHERE command_row_id = ?1");
    connection
        .query_row(&query, [id], |r| {
            Ok((r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
        })
        .optional()?
        .ok_or(StoreError::LedgerCorruption("missing charter command body"))
}
fn query_create_ticket(
    connection: &Connection,
    id: i64,
) -> Result<(i64, i64, String, String, Option<i64>), StoreError> {
    connection.query_row("SELECT operating_cycle_id, project_id, ticket_title, acceptance_condition_text, prerequisite_ticket_id FROM command_create_ticket WHERE command_row_id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing ticket command body"))
}
type GraphRevisionCommandRow = (i64, i64, Option<i64>, Option<i64>);

fn query_graph_revision_command(
    connection: &Connection,
    id: i64,
) -> Result<GraphRevisionCommandRow, StoreError> {
    connection.query_row("SELECT operating_cycle_id, project_id, causal_episode_id, graph_object_id FROM command_add_graph_object_revision WHERE command_row_id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing revision command body"))
}

fn query_graph_revision_command_body(
    connection: &Connection,
    command_row_id: i64,
) -> Result<GraphRevisionBody, StoreError> {
    let observation: Option<String> = connection
        .query_row(
            "SELECT observation_text FROM command_add_observation_revision WHERE command_row_id = ?1",
            [command_row_id],
            |row| row.get(0),
        )
        .optional()?;
    let hypothesis: Option<String> = connection
        .query_row(
            "SELECT hypothesis_text FROM command_add_hypothesis_revision WHERE command_row_id = ?1",
            [command_row_id],
            |row| row.get(0),
        )
        .optional()?;
    match (observation, hypothesis) {
        (Some(observation), None) => Ok(GraphRevisionBody::Observation {
            observation: ObservationRevisionText::parse(observation)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        }),
        (None, Some(hypothesis)) => Ok(GraphRevisionBody::Hypothesis {
            hypothesis: HypothesisRevisionText::parse(hypothesis)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        }),
        _ => Err(StoreError::LedgerCorruption(
            "graph revision command has missing or ambiguous typed body",
        )),
    }
}
fn query_edge_command(
    connection: &Connection,
    id: i64,
) -> Result<(i64, i64, i64, i64, i64), StoreError> {
    connection.query_row("SELECT operating_cycle_id, project_id, from_graph_revision_id, to_graph_revision_id, edge_kind FROM command_add_graph_edge WHERE command_row_id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing edge command body"))
}
fn query_review_submit(
    connection: &Connection,
    id: i64,
) -> Result<(i64, i64, i64, i64, i64, String), StoreError> {
    connection.query_row("SELECT operating_cycle_id, adversarial_review_id, target_graph_revision_id, author_principal_id, severity, failure_hypothesis FROM command_submit_review_challenge WHERE command_row_id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing review command body"))
}
fn query_postmortem_trigger(
    connection: &Connection,
    id: i64,
) -> Result<(i64, i64, Option<i64>), StoreError> {
    connection.query_row("SELECT operating_cycle_id, project_id, causal_episode_id FROM command_trigger_postmortem WHERE command_row_id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing postmortem trigger command body"))
}
fn query_postmortem_text_command(
    connection: &Connection,
    table: &str,
    id: i64,
) -> Result<(i64, i64, i64, String), StoreError> {
    let query = format!("SELECT * FROM {table} WHERE command_row_id = ?1");
    connection
        .query_row(&query, [id], |r| {
            Ok((r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing postmortem text command body",
        ))
}

fn query_command_id<T>(
    connection: &Connection,
    table: &str,
    column: &str,
    command_row_id: i64,
) -> Result<T, StoreError>
where
    T: TryFrom<i64>,
{
    T::try_from(query_command_value::<i64>(
        connection,
        table,
        column,
        command_row_id,
    )?)
    .map_err(|_| StoreError::InvalidStoredValue)
}

fn digest_from_stored_bytes(bytes: &[u8]) -> Result<Blake3Digest, StoreError> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| StoreError::InvalidStoredValue)?;
    Ok(Blake3Digest::from_bytes(bytes))
}

/// Decodes the complete normalized founding mission body. The parent body is
/// still subject to ordinary one-to-one command-cardinality checks; this
/// helper additionally makes its ordered principle relation and exact four
/// question fields part of that body rather than treating either as ambient
/// configuration.
fn decode_application_mission_input(
    connection: &Connection,
    command_row_id: i64,
) -> Result<ApplicationMissionInput, StoreError> {
    let (identity, name, ordinal, statement, rendering_digest, source_object_id) = connection
        .query_row(
            "SELECT application_identity, application_name, revision_ordinal,
                    mission_statement, source_rendering_digest, source_content_object_id
             FROM command_install_founding_mission WHERE command_row_id = ?1",
            [command_row_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                ))
            },
        )
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing founding mission command body",
        ))?;
    if let Some(source_object_id) = source_object_id {
        verify_mission_source_binding(
            connection,
            &rendering_digest,
            ContentObjectId::try_from(source_object_id)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        )?;
    }
    let mut statement_rows = connection.prepare(
        "SELECT principle_ordinal, principle_kind, principle_text
         FROM command_install_founding_mission_principles
         WHERE command_row_id = ?1 ORDER BY principle_ordinal",
    )?;
    let rows = statement_rows.query_map([command_row_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut principles = Vec::new();
    for (index, row) in rows.enumerate() {
        let (stored_ordinal, kind, text) = row?;
        let expected_ordinal =
            i64::try_from(index + 1).map_err(|_| StoreError::InvalidStoredValue)?;
        if stored_ordinal != expected_ordinal {
            return Err(StoreError::LedgerCorruption(
                "mission principle ordinals are not contiguous",
            ));
        }
        principles.push(MissionPrinciple {
            kind: mission_principle_kind_from_i64(kind)?,
            text: MissionPrincipleText::parse(text).map_err(|_| StoreError::InvalidStoredValue)?,
        });
    }
    let questions = connection
        .query_row(
            "SELECT change_question, improvement_evidence_question,
                    boundary_commitment_question, revisit_question
             FROM command_install_founding_mission_north_star_questions
             WHERE command_row_id = ?1",
            [command_row_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing founding North Star questions command body",
        ))?;
    Ok(ApplicationMissionInput {
        application_identity: ApplicationIdentity::parse(identity)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        application_name: ApplicationName::parse(name)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        revision_ordinal: ApplicationRevisionOrdinal::try_from(ordinal)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        statement: MissionStatement::parse(statement)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        principles: MissionPrinciples::new(principles)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        north_star_questions: NorthStarQuestionSet {
            change: NorthStarChangeQuestion::parse(questions.0)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            improvement_evidence: NorthStarImprovementEvidenceQuestion::parse(questions.1)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            boundary_commitment: NorthStarBoundaryCommitmentQuestion::parse(questions.2)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            revisit: NorthStarRevisitQuestion::parse(questions.3)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        },
        source_rendering_digest: digest_from_stored_bytes(&rendering_digest)?,
    })
}

fn verify_exact_event_body(
    connection: &Connection,
    event_id: EventId,
    kind: EventKind,
) -> Result<(), StoreError> {
    let expected_table = EVENT_BODY_TABLES[(kind as usize) - 1];
    verify_exact_named_body(
        connection,
        event_id.value(),
        expected_table,
        &EVENT_BODY_TABLES,
    )
}

/// Graph revision semantics are deliberately outside the shared revision row.
/// Replay therefore proves the selected object kind owns exactly one matching
/// named body and that the stored semantic field still decodes as its closed
/// Rust type. This catches missing, duplicate, and cross-kind body tampering.
fn verify_graph_revision_bodies(connection: &Connection) -> Result<(), StoreError> {
    let mut statement = connection.prepare(
        "SELECT r.graph_revision_id, o.object_kind
         FROM object_revisions r
         JOIN objects o ON o.graph_object_id = r.graph_object_id
         ORDER BY r.graph_revision_id",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
    for row in rows {
        let (graph_revision_id, object_kind) = row?;
        let object_kind = graph_object_kind_from_i64(object_kind)?;
        let expected_table = match object_kind {
            GraphObjectKind::Observation => "observation_revisions",
            GraphObjectKind::Hypothesis => "hypothesis_revisions",
        };
        let mut body_count = 0_i64;
        let mut expected_present = false;
        for table in GRAPH_REVISION_BODY_TABLES {
            let query = format!("SELECT COUNT(*) FROM {table} WHERE graph_revision_id = ?1");
            let count: i64 = connection.query_row(&query, [graph_revision_id], |row| row.get(0))?;
            body_count += count;
            if table == expected_table {
                expected_present = count == 1;
            }
        }
        if body_count != 1 || !expected_present {
            return Err(StoreError::LedgerCorruption(
                "graph revision typed body is missing, duplicated, or mismatched",
            ));
        }
        match object_kind {
            GraphObjectKind::Observation => {
                let text: String = connection.query_row(
                    "SELECT observation_text FROM observation_revisions WHERE graph_revision_id = ?1",
                    [graph_revision_id],
                    |row| row.get(0),
                )?;
                ObservationRevisionText::parse(text).map_err(|_| StoreError::InvalidStoredValue)?;
            }
            GraphObjectKind::Hypothesis => {
                let text: String = connection.query_row(
                    "SELECT hypothesis_text FROM hypothesis_revisions WHERE graph_revision_id = ?1",
                    [graph_revision_id],
                    |row| row.get(0),
                )?;
                HypothesisRevisionText::parse(text).map_err(|_| StoreError::InvalidStoredValue)?;
            }
        }
    }
    Ok(())
}

/// The table names are compiled constants, never protocol input. Counting all
/// closed body tables makes an inserted second body as corrupt as a missing or
/// mismatched body instead of silently trusting the discriminant.
fn verify_exact_named_body(
    connection: &Connection,
    row_id: i64,
    expected_table: &str,
    tables: &[&str],
) -> Result<(), StoreError> {
    let mut body_count = 0_i64;
    let mut expected_present = false;
    for table in tables {
        let query = format!(
            "SELECT COUNT(*) FROM {table} WHERE {} = ?1",
            body_key_column(table)
        );
        let count: i64 = connection.query_row(&query, [row_id], |row| row.get(0))?;
        body_count += count;
        if *table == expected_table {
            expected_present = count == 1;
        }
    }
    if body_count != 1 || !expected_present {
        return Err(StoreError::LedgerCorruption(
            "closed body is missing, duplicated, or mismatched",
        ));
    }
    Ok(())
}

fn body_key_column(table: &str) -> &'static str {
    if table.starts_with("command_") {
        "command_row_id"
    } else {
        "event_id"
    }
}

fn command_body_table(kind: CommandKind) -> Result<&'static str, StoreError> {
    COMMAND_BODY_TABLES
        .get((kind as usize) - 1)
        .copied()
        .ok_or(StoreError::InvalidStoredValue)
}

fn command_kind_from_i64(value: i64) -> Result<CommandKind, StoreError> {
    match value {
        1 => Ok(CommandKind::CreateSocietyIdentity),
        2 => Ok(CommandKind::InstallRootAuthorityOffice),
        3 => Ok(CommandKind::InstallFoundingMission),
        4 => Ok(CommandKind::AppointInitialRootAuthority),
        5 => Ok(CommandKind::SetR0HardCeiling),
        6 => Ok(CommandKind::BootstrapSociety),
        7 => Ok(CommandKind::ProposeOperatingCycle),
        8 => Ok(CommandKind::AdmitOperatingCycle),
        9 => Ok(CommandKind::StartRootAuthorityOfficeSession),
        10 => Ok(CommandKind::RecordOfficeSessionReady),
        11 => Ok(CommandKind::OpenOfficeTurn),
        12 => Ok(CommandKind::SettleOfficeTurn),
        13 => Ok(CommandKind::QuiesceOperatingCycle),
        14 => Ok(CommandKind::RecordCycleDrained),
        15 => Ok(CommandKind::ResumeOperatingCycle),
        16 => Ok(CommandKind::ReconcileOperatingCycle),
        17 => Ok(CommandKind::CloseOperatingCycle),
        18 => Ok(CommandKind::ReserveBudget),
        19 => Ok(CommandKind::ReconcileBudget),
        20 => Ok(CommandKind::RequestCancellation),
        21 => Ok(CommandKind::ReconcileCancellation),
        22 => Ok(CommandKind::RecordOfficeSessionTerminal),
        23 => Ok(CommandKind::CloseCostPostmortem),
        24 => Ok(CommandKind::CreateProject),
        25 => Ok(CommandKind::CharterProject),
        26 => Ok(CommandKind::TransitionProject),
        27 => Ok(CommandKind::CompleteProjectMilestone),
        28 => Ok(CommandKind::ReopenProject),
        29 => Ok(CommandKind::CreateTicket),
        30 => Ok(CommandKind::TransitionTicket),
        31 => Ok(CommandKind::AddGraphObjectRevision),
        32 => Ok(CommandKind::CommitGraphRevision),
        33 => Ok(CommandKind::AddGraphEdge),
        34 => Ok(CommandKind::CreateEpisode),
        35 => Ok(CommandKind::TransitionEpisode),
        36 => Ok(CommandKind::ReopenEpisode),
        37 => Ok(CommandKind::RequestAdversarialReview),
        38 => Ok(CommandKind::SubmitReviewChallenge),
        39 => Ok(CommandKind::RespondToReviewChallenge),
        40 => Ok(CommandKind::DispositionReviewChallenge),
        41 => Ok(CommandKind::ResolveAdversarialReview),
        42 => Ok(CommandKind::TriggerPostmortem),
        43 => Ok(CommandKind::RecordPostmortemCausalClaim),
        44 => Ok(CommandKind::ProposePostmortemAction),
        45 => Ok(CommandKind::ClosePostmortem),
        46 => Ok(CommandKind::AssignAdversarialReviewer),
        47 => Ok(CommandKind::RegisterActorConfiguration),
        48 => Ok(CommandKind::RegisterContextPack),
        49 => Ok(CommandKind::AdmitActorInstance),
        50 => Ok(CommandKind::AdmitTicket),
        51 => Ok(CommandKind::RegisterWorkItem),
        52 => Ok(CommandKind::ClaimWorkItem),
        53 => Ok(CommandKind::StartActorAttempt),
        54 => Ok(CommandKind::AttestActorAttemptTerminal),
        55 => Ok(CommandKind::ValidateTicketAttempt),
        56 => Ok(CommandKind::RetryActorAttempt),
        57 => Ok(CommandKind::CompleteTicket),
        58 => Ok(CommandKind::ExpireWorkLease),
        59 => Ok(CommandKind::CancelActorAttempt),
        60 => Ok(CommandKind::RegisterOutcomeObligation),
        61 => Ok(CommandKind::ResolveOutcomeObligation),
        62 => Ok(CommandKind::RecordContentSealReceipt),
        63 => Ok(CommandKind::RegisterContentObject),
        64 => Ok(CommandKind::RegisterForensicManifest),
        65 => Ok(CommandKind::RegisterDeterministicExperiment),
        66 => Ok(CommandKind::RecordDeterministicEvaluationReceipt),
        67 => Ok(CommandKind::AdmitDeterministicEvidence),
        68 => Ok(CommandKind::FinalizeDeterministicExperiment),
        69 => Ok(CommandKind::AdmitPiChildSpawn),
        70 => Ok(CommandKind::RecordInertChildSpawn),
        71 => Ok(CommandKind::RecordPiAdapterReady),
        72 => Ok(CommandKind::AuthorizePiCreateSession),
        73 => Ok(CommandKind::RecordPiCreateSessionDelivery),
        74 => Ok(CommandKind::RecordPiSessionReady),
        75 => Ok(CommandKind::RecordChildStreamSeal),
        76 => Ok(CommandKind::RecordChildProcessLiveness),
        77 => Ok(CommandKind::RecordProcessSignalReceipt),
        78 => Ok(CommandKind::RecordDirectChildReap),
        79 => Ok(CommandKind::RecordChildRecovery),
        80 => Ok(CommandKind::FinalizeChildProcess),
        81 => Ok(CommandKind::BeginCancellationPropagation),
        82 => Ok(CommandKind::ReconcileCancellationPropagation),
        83 => Ok(CommandKind::OpenSupervisorEpoch),
        84 => Ok(CommandKind::RecordPiAbortControlDelivery),
        85 => Ok(CommandKind::RecordNativeChildNotSpawned),
        86 => Ok(CommandKind::AuthorizePiOfficeTurnPrompt),
        87 => Ok(CommandKind::RecordPiOfficeTurnPromptDelivery),
        88 => Ok(CommandKind::RecordPiOfficeTurnPromptAccepted),
        89 => Ok(CommandKind::RecordPiOfficeTurnUsage),
        90 => Ok(CommandKind::RecordPiOfficeTurnUsageFailure),
        91 => Ok(CommandKind::RecordPiOfficeTurnTerminal),
        92 => Ok(CommandKind::AuthorizePiOfficeSessionDispose),
        93 => Ok(CommandKind::RecordPiOfficeSessionDisposeDelivery),
        94 => Ok(CommandKind::RecordPiOfficeSessionDisposeAccepted),
        95 => Ok(CommandKind::RecordPiOfficeSessionDisposeUsage),
        96 => Ok(CommandKind::RecordPiOfficeSessionDisposeUsageFailure),
        97 => Ok(CommandKind::RecordPiOfficeSessionDisposed),
        98 => Ok(CommandKind::AdmitDeterministicEvaluatorNativeChild),
        99 => Ok(CommandKind::RecordDeterministicEvaluatorNativeChildSpawn),
        100 => Ok(CommandKind::RegisterDeterministicEvaluatorForensicManifest),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn capability_from_i64(value: i64) -> Result<Capability, StoreError> {
    match value {
        1 => Ok(Capability::CreateSocietyIdentity),
        2 => Ok(Capability::InstallRootAuthorityOffice),
        3 => Ok(Capability::InstallFoundingMission),
        4 => Ok(Capability::AppointInitialRootAuthority),
        5 => Ok(Capability::SetR0HardCeiling),
        6 => Ok(Capability::BootstrapSociety),
        7 => Ok(Capability::ProposeOperatingCycle),
        8 => Ok(Capability::AdmitOperatingCycle),
        9 => Ok(Capability::QuiesceOperatingCycle),
        10 => Ok(Capability::ResumeOperatingCycle),
        11 => Ok(Capability::ReconcileOperatingCycle),
        12 => Ok(Capability::CloseOperatingCycle),
        13 => Ok(Capability::StartRootAuthorityOfficeSession),
        14 => Ok(Capability::OpenOfficeTurn),
        15 => Ok(Capability::RequestCancellation),
        16 => Ok(Capability::ReserveBudget),
        17 => Ok(Capability::ReconcileBudget),
        18 => Ok(Capability::RecordCycleDrained),
        19 => Ok(Capability::RecordOfficeSessionReady),
        20 => Ok(Capability::SettleOfficeTurn),
        21 => Ok(Capability::ReconcileCancellation),
        22 => Ok(Capability::RecordOfficeSessionTerminal),
        23 => Ok(Capability::CloseCostPostmortem),
        24 => Ok(Capability::CreateProject),
        25 => Ok(Capability::CharterProject),
        26 => Ok(Capability::TransitionProject),
        27 => Ok(Capability::CompleteProjectMilestone),
        28 => Ok(Capability::ReopenProject),
        29 => Ok(Capability::CreateTicket),
        30 => Ok(Capability::TransitionTicket),
        31 => Ok(Capability::AddGraphObjectRevision),
        32 => Ok(Capability::CommitGraphRevision),
        33 => Ok(Capability::AddGraphEdge),
        34 => Ok(Capability::CreateEpisode),
        35 => Ok(Capability::TransitionEpisode),
        36 => Ok(Capability::ReopenEpisode),
        37 => Ok(Capability::RequestAdversarialReview),
        38 => Ok(Capability::SubmitReviewChallenge),
        39 => Ok(Capability::RespondToReviewChallenge),
        40 => Ok(Capability::DispositionReviewChallenge),
        41 => Ok(Capability::ResolveAdversarialReview),
        42 => Ok(Capability::TriggerPostmortem),
        43 => Ok(Capability::RecordPostmortemCausalClaim),
        44 => Ok(Capability::ProposePostmortemAction),
        45 => Ok(Capability::ClosePostmortem),
        46 => Ok(Capability::AssignAdversarialReviewer),
        47 => Ok(Capability::RegisterActorConfiguration),
        48 => Ok(Capability::RegisterContextPack),
        49 => Ok(Capability::AdmitActorInstance),
        50 => Ok(Capability::AdmitTicket),
        51 => Ok(Capability::RegisterWorkItem),
        52 => Ok(Capability::ClaimWorkItem),
        53 => Ok(Capability::StartActorAttempt),
        54 => Ok(Capability::AttestActorAttemptTerminal),
        55 => Ok(Capability::ValidateTicketAttempt),
        56 => Ok(Capability::RetryActorAttempt),
        57 => Ok(Capability::CompleteTicket),
        58 => Ok(Capability::ExpireWorkLease),
        59 => Ok(Capability::CancelActorAttempt),
        60 => Ok(Capability::RegisterOutcomeObligation),
        61 => Ok(Capability::ResolveOutcomeObligation),
        62 => Ok(Capability::RecordContentSealReceipt),
        63 => Ok(Capability::RegisterContentObject),
        64 => Ok(Capability::RegisterForensicManifest),
        65 => Ok(Capability::RegisterDeterministicExperiment),
        66 => Ok(Capability::RecordDeterministicEvaluationReceipt),
        67 => Ok(Capability::AdmitDeterministicEvidence),
        68 => Ok(Capability::FinalizeDeterministicExperiment),
        69 => Ok(Capability::AdmitPiChildSpawn),
        70 => Ok(Capability::RecordInertChildSpawn),
        71 => Ok(Capability::RecordPiAdapterReady),
        72 => Ok(Capability::AuthorizePiCreateSession),
        73 => Ok(Capability::RecordPiCreateSessionDelivery),
        74 => Ok(Capability::RecordPiSessionReady),
        75 => Ok(Capability::RecordChildStreamSeal),
        76 => Ok(Capability::RecordChildProcessLiveness),
        77 => Ok(Capability::RecordProcessSignalReceipt),
        78 => Ok(Capability::RecordDirectChildReap),
        79 => Ok(Capability::RecordChildRecovery),
        80 => Ok(Capability::FinalizeChildProcess),
        81 => Ok(Capability::BeginCancellationPropagation),
        82 => Ok(Capability::ReconcileCancellationPropagation),
        83 => Ok(Capability::OpenSupervisorEpoch),
        84 => Ok(Capability::RecordPiAbortControlDelivery),
        85 => Ok(Capability::RecordNativeChildNotSpawned),
        86 => Ok(Capability::AuthorizePiOfficeTurnPrompt),
        87 => Ok(Capability::RecordPiOfficeTurnPromptDelivery),
        88 => Ok(Capability::RecordPiOfficeTurnPromptAccepted),
        89 => Ok(Capability::RecordPiOfficeTurnUsage),
        90 => Ok(Capability::RecordPiOfficeTurnUsageFailure),
        91 => Ok(Capability::RecordPiOfficeTurnTerminal),
        92 => Ok(Capability::AuthorizePiOfficeSessionDispose),
        93 => Ok(Capability::RecordPiOfficeSessionDisposeDelivery),
        94 => Ok(Capability::RecordPiOfficeSessionDisposeAccepted),
        95 => Ok(Capability::RecordPiOfficeSessionDisposeUsage),
        96 => Ok(Capability::RecordPiOfficeSessionDisposeUsageFailure),
        97 => Ok(Capability::RecordPiOfficeSessionDisposed),
        98 => Ok(Capability::AdmitDeterministicEvaluatorNativeChild),
        99 => Ok(Capability::RecordDeterministicEvaluatorNativeChildSpawn),
        100 => Ok(Capability::RegisterDeterministicEvaluatorForensicManifest),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn query_event_id<T>(
    connection: &Connection,
    table: &str,
    column: &str,
    event_id: EventId,
) -> Result<T, StoreError>
where
    T: TryFrom<i64>,
{
    let query = format!("SELECT {column} FROM {table} WHERE event_id = ?1");
    let value = connection
        .query_row(&query, [event_id.value()], |row| row.get::<_, i64>(0))
        .optional()?
        .ok_or(StoreError::LedgerCorruption("missing simple event body"))?;
    T::try_from(value).map_err(|_| StoreError::InvalidStoredValue)
}

fn decode_office_turn_opened_event(
    connection: &Connection,
    event_id: EventId,
) -> Result<EventBody, StoreError> {
    let (turn, session, purpose) = connection
        .query_row(
            "SELECT office_turn_id, root_authority_office_session_id, purpose
             FROM event_office_turn_opened WHERE event_id = ?1",
            [event_id.value()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing office turn event body",
        ))?;
    let turn_id = OfficeTurnId::try_from(turn).map_err(|_| StoreError::InvalidStoredValue)?;
    let session_id = RootAuthorityOfficeSessionId::try_from(session)
        .map_err(|_| StoreError::InvalidStoredValue)?;
    Ok(EventBody::OfficeTurnOpened {
        turn_id,
        session_id,
        purpose: office_turn_purpose_from_i64(purpose)?,
    })
}

fn decode_office_turn_settled_event(
    connection: &Connection,
    event_id: EventId,
) -> Result<EventBody, StoreError> {
    let (turn, session, charged_delta) = connection
        .query_row(
            "SELECT office_turn_id, root_authority_office_session_id, charged_delta_micros
             FROM event_office_turn_settled WHERE event_id = ?1",
            [event_id.value()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing office turn event body",
        ))?;
    Ok(EventBody::OfficeTurnSettled {
        turn_id: OfficeTurnId::try_from(turn).map_err(|_| StoreError::InvalidStoredValue)?,
        session_id: RootAuthorityOfficeSessionId::try_from(session)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        charged_delta: UsdMicros::try_from(charged_delta)
            .map_err(|_| StoreError::InvalidStoredValue)?,
    })
}

fn rejection_from_i64(value: i64) -> Result<Rejection, StoreError> {
    Rejection::try_from(value).map_err(|_| StoreError::InvalidStoredValue)
}

fn pi_office_turn_disposition_from_i64(value: i64) -> Result<PiOfficeTurnDisposition, StoreError> {
    match value {
        1 => Ok(PiOfficeTurnDisposition::Completed),
        2 => Ok(PiOfficeTurnDisposition::Length),
        3 => Ok(PiOfficeTurnDisposition::Error),
        4 => Ok(PiOfficeTurnDisposition::Aborted),
        5 => Ok(PiOfficeTurnDisposition::Failed),
        6 => Ok(PiOfficeTurnDisposition::ProtocolFailed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn pi_office_turn_assistant_outcome_from_i64(
    value: i64,
) -> Result<PiOfficeTurnAssistantOutcome, StoreError> {
    match value {
        1 => Ok(PiOfficeTurnAssistantOutcome::ObservedStop),
        2 => Ok(PiOfficeTurnAssistantOutcome::ObservedLength),
        3 => Ok(PiOfficeTurnAssistantOutcome::ObservedError),
        4 => Ok(PiOfficeTurnAssistantOutcome::ObservedAborted),
        5 => Ok(PiOfficeTurnAssistantOutcome::SdkPromiseRejected),
        6 => Ok(PiOfficeTurnAssistantOutcome::MissingFinalAssistantOutcome),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn pi_office_turn_terminal_evidence_from_sql(
    kind: i64,
    agent_settled_sequence: Option<i64>,
    final_accounting_sequence: i64,
) -> Result<PiOfficeTurnTerminalEvidence, StoreError> {
    let final_accounting_sequence = PiProtocolSequence::try_from(final_accounting_sequence)
        .map_err(|_| StoreError::InvalidStoredValue)?;
    match (kind, agent_settled_sequence) {
        (1, Some(agent_settled_sequence)) => Ok(PiOfficeTurnTerminalEvidence::ObservedAssistant {
            agent_settled_sequence: PiProtocolSequence::try_from(agent_settled_sequence)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            final_accounting_sequence,
        }),
        (2, None) => Ok(PiOfficeTurnTerminalEvidence::UnavailableAssistant {
            final_known_usage_sequence: final_accounting_sequence,
        }),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn pi_office_turn_transcript_disposition_from_i64(
    value: i64,
) -> Result<PiOfficeTurnTranscriptDisposition, StoreError> {
    match value {
        1 => Ok(PiOfficeTurnTranscriptDisposition::DeferredUntilOfficeSessionDispose),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn pi_office_turn_usage_failure_from_sql(
    kind: i64,
    unknown: Option<i64>,
    unavailable: Option<i64>,
) -> Result<PiOfficeTurnUsageFailure, StoreError> {
    match (kind, unknown, unavailable) {
        (1, Some(1), None) => Ok(PiOfficeTurnUsageFailure::Unknown(
            PiOfficeTurnUsageUnknownReason::MissingFinalUsageSnapshot,
        )),
        (1, Some(2), None) => Ok(PiOfficeTurnUsageFailure::Unknown(
            PiOfficeTurnUsageUnknownReason::BoundaryStreamInterrupted,
        )),
        (1, Some(3), None) => Ok(PiOfficeTurnUsageFailure::Unknown(
            PiOfficeTurnUsageUnknownReason::TerminalEvidenceMissing,
        )),
        (2, None, Some(1)) => Ok(PiOfficeTurnUsageFailure::Unavailable(
            PiOfficeTurnUsageUnavailableReason::InvalidSdkUsage,
        )),
        (2, None, Some(2)) => Ok(PiOfficeTurnUsageFailure::Unavailable(
            PiOfficeTurnUsageUnavailableReason::UsageRegressed,
        )),
        (2, None, Some(3)) => Ok(PiOfficeTurnUsageFailure::Unavailable(
            PiOfficeTurnUsageUnavailableReason::UsageInconsistent,
        )),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn pi_office_session_transcript_receipt_from_sql(
    transcript_kind: i64,
    session_file: String,
    session_file_digest: Option<Vec<u8>>,
    transcript_content_object_id: Option<i64>,
    first_user_prompt_kind: Option<i64>,
    first_user_prompt_digest: Option<Vec<u8>>,
) -> Result<PiOfficeSessionTranscriptReceipt, StoreError> {
    let session_file = CanonicalPiSessionTranscriptPath::parse(session_file)
        .map_err(|_| StoreError::InvalidStoredValue)?;
    match (
        transcript_kind,
        session_file_digest,
        transcript_content_object_id,
        first_user_prompt_kind,
        first_user_prompt_digest,
    ) {
        (1, Some(session_file_digest), Some(content_object_id), Some(1), None) => {
            Ok(PiOfficeSessionTranscriptReceipt::Materialized {
                session_file,
                session_file_digest: digest_from_stored_bytes(&session_file_digest)?,
                transcript_content_object_id: ContentObjectId::try_from(content_object_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                first_user_prompt: PiOfficeSessionFirstUserPromptReceipt::Absent,
            })
        }
        (1, Some(session_file_digest), Some(content_object_id), Some(2), Some(prompt_digest)) => {
            Ok(PiOfficeSessionTranscriptReceipt::Materialized {
                session_file,
                session_file_digest: digest_from_stored_bytes(&session_file_digest)?,
                transcript_content_object_id: ContentObjectId::try_from(content_object_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                first_user_prompt: PiOfficeSessionFirstUserPromptReceipt::Verified {
                    digest: digest_from_stored_bytes(&prompt_digest)?,
                },
            })
        }
        (2, None, None, None, None) => {
            Ok(PiOfficeSessionTranscriptReceipt::UnmaterializedNoPrompt { session_file })
        }
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn pi_office_session_dispose_budget_disposition_from_sql(
    kind: i64,
    observed: i64,
    cancellation_request_id: Option<i64>,
    postmortem_id: Option<i64>,
) -> Result<PiOfficeSessionDisposeBudgetDisposition, StoreError> {
    match (kind, cancellation_request_id, postmortem_id) {
        (1, None, None) => Ok(PiOfficeSessionDisposeBudgetDisposition::Reconciled {
            observed_cumulative_micro_usd: UsdMicros::try_from(observed)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        }),
        (2, Some(cancellation_request_id), Some(postmortem_id)) => {
            Ok(PiOfficeSessionDisposeBudgetDisposition::Frozen {
                cancellation_request_id: CancellationRequestId::try_from(cancellation_request_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                postmortem_id: CostPostmortemId::try_from(postmortem_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            })
        }
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn pi_cumulative_usage_from_sql(
    input: i64,
    output: i64,
    cache_read: i64,
    cache_write: i64,
    total: i64,
    provider_cost: &[u8],
    ceiling: i64,
) -> Result<PiCumulativeUsage, StoreError> {
    let bytes: [u8; 8] = provider_cost
        .try_into()
        .map_err(|_| StoreError::InvalidStoredValue)?;
    let usage = PiCumulativeUsage {
        input_tokens: PiTokenCount::try_from(input).map_err(|_| StoreError::InvalidStoredValue)?,
        output_tokens: PiTokenCount::try_from(output)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        cache_read_tokens: PiTokenCount::try_from(cache_read)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        cache_write_tokens: PiTokenCount::try_from(cache_write)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        total_tokens: PiTokenCount::try_from(total).map_err(|_| StoreError::InvalidStoredValue)?,
        provider_cost: ProviderCostBinary64::from_big_endian(bytes)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        ceiling_micro_usd: UsdMicros::try_from(ceiling)
            .map_err(|_| StoreError::InvalidStoredValue)?,
    };
    if !usage.is_internally_consistent() {
        return Err(StoreError::InvalidStoredValue);
    }
    Ok(usage)
}

fn event_kind_from_i64(value: i64) -> Result<EventKind, StoreError> {
    match value {
        1 => Ok(EventKind::SocietyIdentityCreated),
        2 => Ok(EventKind::RootAuthorityOfficeInstalled),
        3 => Ok(EventKind::FoundingMissionInstalled),
        4 => Ok(EventKind::RootAuthorityAppointed),
        5 => Ok(EventKind::R0HardCeilingSet),
        6 => Ok(EventKind::SocietyBootstrapped),
        7 => Ok(EventKind::OperatingCycleProposed),
        8 => Ok(EventKind::OperatingCycleStateChanged),
        9 => Ok(EventKind::RootAuthorityOfficeSessionStarted),
        10 => Ok(EventKind::RootAuthorityOfficeSessionStateChanged),
        11 => Ok(EventKind::OfficeTurnOpened),
        12 => Ok(EventKind::OfficeTurnSettled),
        13 => Ok(EventKind::BudgetReserved),
        14 => Ok(EventKind::BudgetReconciled),
        15 => Ok(EventKind::BudgetAdmissionFrozen),
        16 => Ok(EventKind::CancellationRequested),
        17 => Ok(EventKind::CancellationReconciled),
        18 => Ok(EventKind::CostPostmortemClosed),
        19 => Ok(EventKind::ProjectCreated),
        20 => Ok(EventKind::ProjectChartered),
        21 => Ok(EventKind::ProjectStateChanged),
        22 => Ok(EventKind::ProjectMilestoneCompleted),
        23 => Ok(EventKind::TicketCreated),
        24 => Ok(EventKind::TicketStateChanged),
        25 => Ok(EventKind::GraphObjectRevisionAdded),
        26 => Ok(EventKind::GraphRevisionCommitted),
        27 => Ok(EventKind::GraphEdgeAdded),
        28 => Ok(EventKind::EpisodeCreated),
        29 => Ok(EventKind::EpisodeStateChanged),
        30 => Ok(EventKind::AdversarialReviewRequested),
        31 => Ok(EventKind::ReviewChallengeSubmitted),
        32 => Ok(EventKind::ReviewChallengeResponded),
        33 => Ok(EventKind::ReviewChallengeDispositioned),
        34 => Ok(EventKind::AdversarialReviewResolved),
        35 => Ok(EventKind::PostmortemTriggered),
        36 => Ok(EventKind::PostmortemCausalClaimRecorded),
        37 => Ok(EventKind::PostmortemActionProposed),
        38 => Ok(EventKind::PostmortemClosed),
        39 => Ok(EventKind::AdversarialReviewerAssigned),
        40 => Ok(EventKind::ActorConfigurationRegistered),
        41 => Ok(EventKind::ContextPackRegistered),
        42 => Ok(EventKind::ActorInstanceAdmitted),
        43 => Ok(EventKind::TicketAdmitted),
        44 => Ok(EventKind::WorkItemRegistered),
        45 => Ok(EventKind::WorkItemClaimed),
        46 => Ok(EventKind::ActorAttemptStarted),
        47 => Ok(EventKind::ActorAttemptTerminalAttested),
        48 => Ok(EventKind::TicketAttemptValidated),
        49 => Ok(EventKind::ActorAttemptRetryPrepared),
        50 => Ok(EventKind::TicketCompleted),
        51 => Ok(EventKind::WorkLeaseExpired),
        52 => Ok(EventKind::ActorAttemptCancellationRequested),
        53 => Ok(EventKind::OutcomeObligationRegistered),
        54 => Ok(EventKind::OutcomeObligationResolved),
        55 => Ok(EventKind::ContentSealReceiptRecorded),
        56 => Ok(EventKind::ContentObjectRegistered),
        57 => Ok(EventKind::ForensicManifestRegistered),
        58 => Ok(EventKind::DeterministicExperimentRegistered),
        59 => Ok(EventKind::DeterministicEvaluationReceiptRecorded),
        60 => Ok(EventKind::DeterministicEvidenceAdmitted),
        61 => Ok(EventKind::DeterministicExperimentFinalized),
        62 => Ok(EventKind::PiChildSpawnAdmitted),
        63 => Ok(EventKind::InertPiChildSpawnRecorded),
        64 => Ok(EventKind::PiAdapterReadyRecorded),
        65 => Ok(EventKind::PiCreateSessionAuthorized),
        66 => Ok(EventKind::PiCreateSessionDeliveryRecorded),
        67 => Ok(EventKind::PiSessionReadyRecorded),
        68 => Ok(EventKind::ChildStreamSealed),
        69 => Ok(EventKind::ChildProcessLivenessObserved),
        70 => Ok(EventKind::ProcessSignalReceiptRecorded),
        71 => Ok(EventKind::DirectChildReaped),
        72 => Ok(EventKind::ChildRecoveryObserved),
        73 => Ok(EventKind::ChildProcessFinalized),
        74 => Ok(EventKind::CancellationPropagationBegun),
        75 => Ok(EventKind::CancellationPropagationReconciled),
        76 => Ok(EventKind::SupervisorEpochOpened),
        77 => Ok(EventKind::CancellationPropagationContainmentFailed),
        78 => Ok(EventKind::PiAbortControlDeliveryRecorded),
        79 => Ok(EventKind::NativeChildSpawnInvalidated),
        80 => Ok(EventKind::PiOfficeTurnPromptAuthorized),
        81 => Ok(EventKind::PiOfficeTurnPromptDelivered),
        82 => Ok(EventKind::PiOfficeTurnPromptAccepted),
        83 => Ok(EventKind::PiOfficeTurnUsageRecorded),
        84 => Ok(EventKind::PiOfficeTurnUsageFrozen),
        85 => Ok(EventKind::PiOfficeTurnTerminalRecorded),
        86 => Ok(EventKind::PiOfficeSessionDisposeAuthorized),
        87 => Ok(EventKind::PiOfficeSessionDisposeDelivered),
        88 => Ok(EventKind::PiOfficeSessionDisposeAccepted),
        89 => Ok(EventKind::PiOfficeSessionDisposeUsageRecorded),
        90 => Ok(EventKind::PiOfficeSessionDisposeUsageFrozen),
        91 => Ok(EventKind::PiOfficeSessionDisposed),
        92 => Ok(EventKind::DeterministicEvaluatorNativeChildAdmitted),
        93 => Ok(EventKind::DeterministicEvaluatorNativeChildSpawnRecorded),
        94 => Ok(EventKind::DeterministicEvaluatorForensicManifestRegistered),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn operating_cycle_state_from_i64(value: i64) -> Result<OperatingCycleState, StoreError> {
    match value {
        1 => Ok(OperatingCycleState::Proposed),
        2 => Ok(OperatingCycleState::Admitted),
        3 => Ok(OperatingCycleState::Running),
        4 => Ok(OperatingCycleState::Quiescing),
        5 => Ok(OperatingCycleState::Drained),
        6 => Ok(OperatingCycleState::Reconciling),
        7 => Ok(OperatingCycleState::Closed),
        8 => Ok(OperatingCycleState::Cancelling),
        9 => Ok(OperatingCycleState::Reaping),
        10 => Ok(OperatingCycleState::Cancelled),
        11 => Ok(OperatingCycleState::Failed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn child_process_state_from_i64(value: i64) -> Result<ChildProcessState, StoreError> {
    match value {
        1 => Ok(ChildProcessState::Spawned),
        2 => Ok(ChildProcessState::Running),
        3 => Ok(ChildProcessState::CancellationRequested),
        4 => Ok(ChildProcessState::DirectChildReaped),
        5 => Ok(ChildProcessState::RecoveryContainmentRequired),
        6 => Ok(ChildProcessState::LostParentage),
        7 => Ok(ChildProcessState::ContainmentFailed),
        8 => Ok(ChildProcessState::Finalized),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn decode_child_owner(
    attempt: Option<i64>,
    office: Option<i64>,
) -> Result<PiChildOwner, StoreError> {
    match (attempt, office) {
        (Some(attempt), None) => Ok(PiChildOwner::ActorAttempt(
            ActorAttemptId::try_from(attempt).map_err(|_| StoreError::InvalidStoredValue)?,
        )),
        (None, Some(office)) => Ok(PiChildOwner::RootAuthorityOfficeSession(
            RootAuthorityOfficeSessionId::try_from(office)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        )),
        _ => Err(StoreError::LedgerCorruption("invalid Pi child owner union")),
    }
}

fn child_stream_kind_from_i64(value: i64) -> Result<ChildStreamKind, StoreError> {
    match value {
        1 => Ok(ChildStreamKind::AdmittedControl),
        2 => Ok(ChildStreamKind::PhysicalStdin),
        3 => Ok(ChildStreamKind::Stdout),
        4 => Ok(ChildStreamKind::Stderr),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn child_stream_completeness_from_i64(
    value: i64,
) -> Result<ChildStreamSealCompleteness, StoreError> {
    match value {
        1 => Ok(ChildStreamSealCompleteness::Complete),
        2 => Ok(ChildStreamSealCompleteness::PrefixBounded),
        3 => Ok(ChildStreamSealCompleteness::CountOverflow),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn process_group_liveness_from_i64(value: i64) -> Result<ProcessGroupLiveness, StoreError> {
    match value {
        1 => Ok(ProcessGroupLiveness::Present),
        2 => Ok(ProcessGroupLiveness::Absent),
        3 => Ok(ProcessGroupLiveness::Inaccessible),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn process_signal_action_from_i64(value: i64) -> Result<ProcessSignalAction, StoreError> {
    match value {
        1 => Ok(ProcessSignalAction::Terminate),
        2 => Ok(ProcessSignalAction::Kill),
        3 => Ok(ProcessSignalAction::LingeringGroupKill),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn process_signal_delivery_from_i64(value: i64) -> Result<ProcessSignalDelivery, StoreError> {
    match value {
        1 => Ok(ProcessSignalDelivery::Delivered),
        2 => Ok(ProcessSignalDelivery::AbsentBeforeSignal),
        3 => Ok(ProcessSignalDelivery::AbsentDuringSignal),
        4 => Ok(ProcessSignalDelivery::Inaccessible),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn pi_abort_control_write_outcome_from_i64(
    value: i64,
) -> Result<PiAbortControlWriteOutcome, StoreError> {
    match value {
        1 => Ok(PiAbortControlWriteOutcome::FullyWritten),
        2 => Ok(PiAbortControlWriteOutcome::PipeClosedBeforeWrite),
        3 => Ok(PiAbortControlWriteOutcome::WriteFailed),
        4 => Ok(PiAbortControlWriteOutcome::PartialWriteDiscarded),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn native_child_not_spawned_reason_from_i64(
    value: i64,
) -> Result<NativeChildNotSpawnedReason, StoreError> {
    match value {
        1 => Ok(NativeChildNotSpawnedReason::CancelledBeforeSpawn),
        2 => Ok(NativeChildNotSpawnedReason::WorkspacePreparationFailed),
        3 => Ok(NativeChildNotSpawnedReason::ArtifactQualificationFailed),
        4 => Ok(NativeChildNotSpawnedReason::NativeSpawnFailed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn deterministic_experiment_state_from_i64(
    value: i64,
) -> Result<DeterministicExperimentState, StoreError> {
    match value {
        1 => Ok(DeterministicExperimentState::Registered),
        2 => Ok(DeterministicExperimentState::EvidenceAdmitted),
        3 => Ok(DeterministicExperimentState::Closed),
        4 => Ok(DeterministicExperimentState::Failed),
        5 => Ok(DeterministicExperimentState::Cancelled),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn child_recovery_observation_from_i64(value: i64) -> Result<ChildRecoveryObservation, StoreError> {
    match value {
        1 => Ok(ChildRecoveryObservation::ParentageLost),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn child_terminal_disposition_from_i64(value: i64) -> Result<ChildTerminalDisposition, StoreError> {
    match value {
        1 => Ok(ChildTerminalDisposition::Exited),
        4 => Ok(ChildTerminalDisposition::Terminated),
        5 => Ok(ChildTerminalDisposition::Killed),
        6 => Ok(ChildTerminalDisposition::SupervisionLost),
        7 => Ok(ChildTerminalDisposition::ContainmentFailed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn direct_wait_status_from_sql(
    kind: i64,
    value: Option<i64>,
) -> Result<DirectChildWaitStatus, StoreError> {
    match (kind, value) {
        (1, Some(exit_code)) => Ok(DirectChildWaitStatus::Exited {
            exit_code: ProcessExitCode::try_from(
                i32::try_from(exit_code).map_err(|_| StoreError::InvalidStoredValue)?,
            )
            .map_err(|_| StoreError::InvalidStoredValue)?,
        }),
        (2, Some(signal_number)) => Ok(DirectChildWaitStatus::Signaled {
            signal_number: ProcessSignalNumber::try_from(
                i32::try_from(signal_number).map_err(|_| StoreError::InvalidStoredValue)?,
            )
            .map_err(|_| StoreError::InvalidStoredValue)?,
        }),
        (3, None) => Ok(DirectChildWaitStatus::Unknown),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn signal_cause_parts(cause: ProcessSignalCause) -> (i64, Option<i64>) {
    match cause {
        ProcessSignalCause::CancellationPropagation(propagation_id) => {
            (1, Some(propagation_id.value()))
        }
        ProcessSignalCause::AutomaticBoundaryContainment => (2, None),
    }
}

fn process_signal_cause_from_sql(
    kind: i64,
    propagation_id: Option<i64>,
) -> Result<ProcessSignalCause, StoreError> {
    match (kind, propagation_id) {
        (1, Some(propagation_id)) => Ok(ProcessSignalCause::CancellationPropagation(
            CancellationPropagationId::try_from(propagation_id)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        )),
        (2, None) => Ok(ProcessSignalCause::AutomaticBoundaryContainment),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn operating_cycle_treatment_from_i64(value: i64) -> Result<OperatingCycleTreatment, StoreError> {
    match value {
        1 => Ok(OperatingCycleTreatment::PiSdkQualificationV1),
        2 => Ok(OperatingCycleTreatment::PinnedPiSdkLiveV1),
        3 => Ok(OperatingCycleTreatment::DeterministicPiHostFixtureV1),
        4 => Ok(OperatingCycleTreatment::DeterministicEvaluatorFixtureV1),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn retention_access_class_from_i64(value: i64) -> Result<RetentionAccessClass, StoreError> {
    match value {
        1 => Ok(RetentionAccessClass::ForensicRestricted),
        2 => Ok(RetentionAccessClass::ProjectScoped),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn forensic_manifest_capture_policy_from_i64(
    value: i64,
) -> Result<ForensicManifestCapturePolicy, StoreError> {
    match value {
        1 => Ok(ForensicManifestCapturePolicy::DeterministicExperimentEvaluatorV1),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn evidence_semantic_role_from_i64(value: i64) -> Result<EvidenceSemanticRole, StoreError> {
    match value {
        1 => Ok(EvidenceSemanticRole::DeterministicObservation),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn evidence_applicability_from_i64(value: i64) -> Result<crate::EvidenceApplicability, StoreError> {
    match value {
        1 => Ok(crate::EvidenceApplicability::TestsTargetHypothesis),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn office_session_state_from_i64(value: i64) -> Result<OfficeSessionState, StoreError> {
    match value {
        1 => Ok(OfficeSessionState::Reserved),
        2 => Ok(OfficeSessionState::Starting),
        3 => Ok(OfficeSessionState::Ready),
        4 => Ok(OfficeSessionState::TurnActive),
        5 => Ok(OfficeSessionState::Quiescing),
        6 => Ok(OfficeSessionState::ProcessEnded),
        7 => Ok(OfficeSessionState::EvidenceSealing),
        8 => Ok(OfficeSessionState::Closed),
        9 => Ok(OfficeSessionState::Cancelling),
        10 => Ok(OfficeSessionState::Cancelled),
        11 => Ok(OfficeSessionState::Failed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn office_session_terminal_state_from_i64(
    value: i64,
) -> Result<OfficeSessionTerminalState, StoreError> {
    match value {
        1 => Ok(OfficeSessionTerminalState::Closed),
        2 => Ok(OfficeSessionTerminalState::Cancelled),
        3 => Ok(OfficeSessionTerminalState::Failed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn office_turn_purpose_from_i64(value: i64) -> Result<OfficeTurnPurpose, StoreError> {
    match value {
        1 => Ok(OfficeTurnPurpose::OrdinaryWork),
        2 => Ok(OfficeTurnPurpose::Recovery),
        3 => Ok(OfficeTurnPurpose::Cancellation),
        4 => Ok(OfficeTurnPurpose::Closure),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn cancellation_mode_from_i64(value: i64) -> Result<CancellationMode, StoreError> {
    match value {
        1 => Ok(CancellationMode::Quiesce),
        2 => Ok(CancellationMode::GracefulCancel),
        3 => Ok(CancellationMode::EmergencyStop),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn budget_freeze_reason_to_sql(
    reason: BudgetFreezeReason,
) -> (i64, Option<i64>, Option<i64>, Option<i64>, Option<i64>) {
    match reason {
        BudgetFreezeReason::KnownOverrun { observed, reserved } => (
            1,
            Some(observed.value()),
            Some(reserved.value()),
            None,
            None,
        ),
        BudgetFreezeReason::Unknown(reason) => (2, None, None, Some(reason as i64), None),
        BudgetFreezeReason::Unavailable(reason) => (3, None, None, None, Some(reason as i64)),
    }
}

fn budget_freeze_reason_from_sql(
    kind: i64,
    observed: Option<i64>,
    reserved: Option<i64>,
    unknown: Option<i64>,
    unavailable: Option<i64>,
) -> Result<BudgetFreezeReason, StoreError> {
    match (kind, observed, reserved, unknown, unavailable) {
        (1, Some(observed), Some(reserved), None, None) => Ok(BudgetFreezeReason::KnownOverrun {
            observed: UsdMicros::try_from(observed).map_err(|_| StoreError::InvalidStoredValue)?,
            reserved: UsdMicros::try_from(reserved).map_err(|_| StoreError::InvalidStoredValue)?,
        }),
        (2, None, None, Some(reason), None) => Ok(BudgetFreezeReason::Unknown(
            cost_unknown_reason_from_i64(reason)?,
        )),
        (3, None, None, None, Some(reason)) => Ok(BudgetFreezeReason::Unavailable(
            cost_unavailable_reason_from_i64(reason)?,
        )),
        _ => Err(StoreError::LedgerCorruption(
            "invalid budget freeze reason body",
        )),
    }
}

fn cost_observation_from_sql(
    kind: i64,
    known: Option<i64>,
    unknown: Option<i64>,
    unavailable: Option<i64>,
) -> Result<CostObservation, StoreError> {
    match (kind, known, unknown, unavailable) {
        (1, Some(amount), None, None) => Ok(CostObservation::Known(
            UsdMicros::try_from(amount).map_err(|_| StoreError::InvalidStoredValue)?,
        )),
        (2, None, Some(reason), None) => Ok(CostObservation::Unknown(
            cost_unknown_reason_from_i64(reason)?,
        )),
        (3, None, None, Some(reason)) => Ok(CostObservation::Unavailable(
            cost_unavailable_reason_from_i64(reason)?,
        )),
        _ => Err(StoreError::LedgerCorruption(
            "invalid cost observation body",
        )),
    }
}

fn cost_postmortem_cause_from_i64(value: i64) -> Result<CostPostmortemCause, StoreError> {
    match value {
        1 => Ok(CostPostmortemCause::KnownOverrun),
        2 => Ok(CostPostmortemCause::UnknownCost),
        3 => Ok(CostPostmortemCause::UnavailableCost),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn cost_postmortem_resolution_from_i64(value: i64) -> Result<CostPostmortemResolution, StoreError> {
    match value {
        1 => Ok(CostPostmortemResolution::ConservativeFullReservation),
        2 => Ok(CostPostmortemResolution::ChargeObservedOverrun),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn cost_unknown_reason_from_i64(value: i64) -> Result<CostUnknownReason, StoreError> {
    match value {
        1 => Ok(CostUnknownReason::ProviderDidNotReport),
        2 => Ok(CostUnknownReason::AdapterStreamInterrupted),
        3 => Ok(CostUnknownReason::ReconciliationMismatch),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn cost_unavailable_reason_from_i64(value: i64) -> Result<CostUnavailableReason, StoreError> {
    match value {
        1 => Ok(CostUnavailableReason::ProviderUnavailable),
        2 => Ok(CostUnavailableReason::CredentialUnavailable),
        3 => Ok(CostUnavailableReason::QualificationRejected),
        4 => Ok(CostUnavailableReason::AdapterAccountingUnavailable),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn mission_principle_kind_from_i64(value: i64) -> Result<MissionPrincipleKind, StoreError> {
    match value {
        1 => Ok(MissionPrincipleKind::Purpose),
        2 => Ok(MissionPrincipleKind::Evidence),
        3 => Ok(MissionPrincipleKind::Boundary),
        4 => Ok(MissionPrincipleKind::Revision),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn project_state_from_i64(value: i64) -> Result<ProjectState, StoreError> {
    match value {
        1 => Ok(ProjectState::Proposed),
        2 => Ok(ProjectState::Challenged),
        3 => Ok(ProjectState::Chartered),
        4 => Ok(ProjectState::Active),
        5 => Ok(ProjectState::Paused),
        6 => Ok(ProjectState::Observing),
        7 => Ok(ProjectState::Closed),
        8 => Ok(ProjectState::Terminated),
        9 => Ok(ProjectState::Reopened),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn ticket_state_from_i64(value: i64) -> Result<TicketState, StoreError> {
    match value {
        1 => Ok(TicketState::Draft),
        2 => Ok(TicketState::Admitted),
        3 => Ok(TicketState::Ready),
        4 => Ok(TicketState::Claimed),
        5 => Ok(TicketState::Submitted),
        6 => Ok(TicketState::Verified),
        7 => Ok(TicketState::Completed),
        8 => Ok(TicketState::ChangesRequested),
        9 => Ok(TicketState::Expired),
        10 => Ok(TicketState::Cancelled),
        11 => Ok(TicketState::Failed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn actor_model_policy_from_i64(value: i64) -> Result<ActorModelPolicy, StoreError> {
    match value {
        1 => Ok(ActorModelPolicy::PinnedDeepseekV4FlashHigh),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn developmental_attractor_from_i64(value: i64) -> Result<DevelopmentalAttractor, StoreError> {
    match value {
        1 => Ok(DevelopmentalAttractor::Explore),
        2 => Ok(DevelopmentalAttractor::Build),
        3 => Ok(DevelopmentalAttractor::Measure),
        4 => Ok(DevelopmentalAttractor::Challenge),
        5 => Ok(DevelopmentalAttractor::Synthesize),
        6 => Ok(DevelopmentalAttractor::Integrate),
        7 => Ok(DevelopmentalAttractor::Remember),
        8 => Ok(DevelopmentalAttractor::Coordinate),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn execution_profile_kind_from_i64(value: i64) -> Result<ExecutionProfileKind, StoreError> {
    match value {
        1 => Ok(ExecutionProfileKind::DeterministicPiHostProcessDoubleV1),
        2 => Ok(ExecutionProfileKind::NativePinnedPiSdkV1),
        3 => Ok(ExecutionProfileKind::DeterministicEvaluatorProcessFixtureV1),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn execution_profile_readiness_from_i64(
    value: i64,
) -> Result<ExecutionProfileReadiness, StoreError> {
    match value {
        1 => Ok(ExecutionProfileReadiness::DeterministicFixtureOnly),
        2 => Ok(ExecutionProfileReadiness::Unqualified),
        3 => Ok(ExecutionProfileReadiness::QualifiedForLiveUse),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn actor_instance_state_from_i64(value: i64) -> Result<ActorInstanceState, StoreError> {
    match value {
        1 => Ok(ActorInstanceState::Active),
        2 => Ok(ActorInstanceState::Retired),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn context_pack_purpose_from_i64(value: i64) -> Result<ContextPackPurpose, StoreError> {
    match value {
        1 => Ok(ContextPackPurpose::TicketExecution),
        2 => Ok(ContextPackPurpose::IndependentReview),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn work_item_kind_from_i64(value: i64) -> Result<WorkItemKind, StoreError> {
    match value {
        1 => Ok(WorkItemKind::TicketExecution),
        2 => Ok(WorkItemKind::IndependentReview),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn work_item_state_from_i64(value: i64) -> Result<WorkItemState, StoreError> {
    match value {
        1 => Ok(WorkItemState::Ready),
        2 => Ok(WorkItemState::Claimed),
        3 => Ok(WorkItemState::Running),
        4 => Ok(WorkItemState::Settled),
        5 => Ok(WorkItemState::Cancelled),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn actor_attempt_state_from_i64(value: i64) -> Result<ActorAttemptState, StoreError> {
    match value {
        1 => Ok(ActorAttemptState::Running),
        2 => Ok(ActorAttemptState::CancellationRequested),
        3 => Ok(ActorAttemptState::Succeeded),
        4 => Ok(ActorAttemptState::Failed),
        5 => Ok(ActorAttemptState::Cancelled),
        6 => Ok(ActorAttemptState::Expired),
        7 => Ok(ActorAttemptState::ProtocolFailed),
        8 => Ok(ActorAttemptState::SupervisorFailed),
        9 => Ok(ActorAttemptState::Validated),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn actor_attempt_terminal_kind_from_i64(
    value: i64,
) -> Result<ActorAttemptTerminalKind, StoreError> {
    match value {
        1 => Ok(ActorAttemptTerminalKind::Succeeded),
        2 => Ok(ActorAttemptTerminalKind::Failed),
        3 => Ok(ActorAttemptTerminalKind::Cancelled),
        4 => Ok(ActorAttemptTerminalKind::Expired),
        5 => Ok(ActorAttemptTerminalKind::ProtocolFailed),
        6 => Ok(ActorAttemptTerminalKind::SupervisorFailed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn actor_attempt_cancellation_reason_from_i64(
    value: i64,
) -> Result<ActorAttemptCancellationReason, StoreError> {
    match value {
        1 => Ok(ActorAttemptCancellationReason::RootAuthorityRequested),
        2 => Ok(ActorAttemptCancellationReason::CycleCancellation),
        3 => Ok(ActorAttemptCancellationReason::LeaseContainment),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn outcome_obligation_state_from_i64(value: i64) -> Result<OutcomeObligationState, StoreError> {
    match value {
        1 => Ok(OutcomeObligationState::Scheduled),
        2 => Ok(OutcomeObligationState::Satisfied),
        3 => Ok(OutcomeObligationState::Waived),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn outcome_obligation_disposition_from_i64(
    value: i64,
) -> Result<OutcomeObligationDisposition, StoreError> {
    match value {
        1 => Ok(OutcomeObligationDisposition::Satisfied),
        2 => Ok(OutcomeObligationDisposition::Waived),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn graph_object_kind_from_i64(value: i64) -> Result<GraphObjectKind, StoreError> {
    match value {
        1 => Ok(GraphObjectKind::Observation),
        2 => Ok(GraphObjectKind::Hypothesis),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn graph_revision_state_from_i64(value: i64) -> Result<GraphRevisionState, StoreError> {
    match value {
        1 => Ok(GraphRevisionState::Draft),
        2 => Ok(GraphRevisionState::Committed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn graph_edge_kind_from_i64(value: i64) -> Result<GraphEdgeKind, StoreError> {
    match value {
        1 => Ok(GraphEdgeKind::Supports),
        2 => Ok(GraphEdgeKind::Challenges),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn episode_state_from_i64(value: i64) -> Result<EpisodeState, StoreError> {
    match value {
        1 => Ok(EpisodeState::Framed),
        2 => Ok(EpisodeState::Admitted),
        3 => Ok(EpisodeState::Investigating),
        4 => Ok(EpisodeState::PrototypeDeliberating),
        5 => Ok(EpisodeState::Prototyping),
        6 => Ok(EpisodeState::CandidateValidating),
        7 => Ok(EpisodeState::DeliveryDeliberating),
        8 => Ok(EpisodeState::DeliveryAuthorized),
        9 => Ok(EpisodeState::Materializing),
        10 => Ok(EpisodeState::Observing),
        11 => Ok(EpisodeState::Learning),
        12 => Ok(EpisodeState::Closed),
        13 => Ok(EpisodeState::ClosedNoAction),
        14 => Ok(EpisodeState::ClosedNoDelivery),
        15 => Ok(EpisodeState::Abandoned),
        16 => Ok(EpisodeState::Reverted),
        17 => Ok(EpisodeState::Reopened),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn adversarial_review_state_from_i64(value: i64) -> Result<AdversarialReviewState, StoreError> {
    match value {
        1 => Ok(AdversarialReviewState::Requested),
        2 => Ok(AdversarialReviewState::Assigned),
        3 => Ok(AdversarialReviewState::Active),
        4 => Ok(AdversarialReviewState::FindingsSubmitted),
        5 => Ok(AdversarialReviewState::ResponsesDue),
        6 => Ok(AdversarialReviewState::Resolved),
        7 => Ok(AdversarialReviewState::AcceptedRisk),
        8 => Ok(AdversarialReviewState::Superseded),
        9 => Ok(AdversarialReviewState::Escalated),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn review_challenge_severity_from_i64(value: i64) -> Result<ReviewChallengeSeverity, StoreError> {
    match value {
        1 => Ok(ReviewChallengeSeverity::Low),
        2 => Ok(ReviewChallengeSeverity::Moderate),
        3 => Ok(ReviewChallengeSeverity::High),
        4 => Ok(ReviewChallengeSeverity::Critical),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn review_challenge_response_state_from_i64(
    value: i64,
) -> Result<ReviewChallengeResponseState, StoreError> {
    match value {
        1 => Ok(ReviewChallengeResponseState::Pending),
        2 => Ok(ReviewChallengeResponseState::Responded),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn review_disposition_kind_from_i64(value: i64) -> Result<ReviewDispositionKind, StoreError> {
    match value {
        1 => Ok(ReviewDispositionKind::Addressed),
        2 => Ok(ReviewDispositionKind::RejectedWithDissentPreserved),
        3 => Ok(ReviewDispositionKind::AcceptedRisk),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn review_resolution_kind_from_i64(value: i64) -> Result<ReviewResolutionKind, StoreError> {
    match value {
        1 => Ok(ReviewResolutionKind::Resolved),
        2 => Ok(ReviewResolutionKind::AcceptedRisk),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn postmortem_state_from_i64(value: i64) -> Result<PostmortemState, StoreError> {
    match value {
        1 => Ok(PostmortemState::Triggered),
        2 => Ok(PostmortemState::Investigating),
        3 => Ok(PostmortemState::Closed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn postmortem_causal_claim_kind_from_i64(
    value: i64,
) -> Result<PostmortemCausalClaimKind, StoreError> {
    match value {
        1 => Ok(PostmortemCausalClaimKind::ContributingCondition),
        2 => Ok(PostmortemCausalClaimKind::Counterfactual),
        3 => Ok(PostmortemCausalClaimKind::Containment),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn postmortem_action_kind_from_i64(value: i64) -> Result<PostmortemActionKind, StoreError> {
    match value {
        1 => Ok(PostmortemActionKind::CreateFollowUpTicket),
        2 => Ok(PostmortemActionKind::ChangePolicyProposal),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
