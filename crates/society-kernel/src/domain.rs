use std::fmt;

use sha2::{Digest, Sha256};
use thiserror::Error;

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(i64);

        impl $name {
            pub const fn new(value: i64) -> Option<Self> {
                if value > 0 { Some(Self(value)) } else { None }
            }

            pub const fn value(self) -> i64 {
                self.0
            }
        }

        impl TryFrom<i64> for $name {
            type Error = DomainValueError;

            fn try_from(value: i64) -> Result<Self, Self::Error> {
                Self::new(value).ok_or(DomainValueError::NonPositiveIdentifier {
                    type_name: stringify!($name),
                    value,
                })
            }
        }
    };
}

identifier!(SocietyId);
identifier!(UniverseSeedId);
identifier!(OfficeId);
identifier!(OfficeOccupancyId);
identifier!(OperatingCycleId);
identifier!(PrincipalId);
identifier!(CapabilityGrantId);
identifier!(BudgetEnvelopeId);
identifier!(BudgetReservationId);
identifier!(CancellationRequestId);
identifier!(CostPostmortemId);
identifier!(GrandArchitectOfficeSessionId);
identifier!(OfficeTurnId);
identifier!(EventId);
identifier!(ProjectId);
identifier!(ProjectMilestoneId);
identifier!(TicketId);
identifier!(GraphObjectId);
identifier!(GraphRevisionId);
identifier!(GraphEdgeId);
identifier!(CausalEpisodeId);
identifier!(AdversarialReviewId);
identifier!(ReviewChallengeId);
identifier!(PostmortemId);
identifier!(PostmortemCausalClaimId);
identifier!(PostmortemActionProposalId);
identifier!(ActorConfigurationId);
identifier!(ActorConfigurationRevisionId);
identifier!(ActorInstanceId);
identifier!(ExecutionProfileId);
identifier!(ContextPackId);
identifier!(WorkItemId);
identifier!(WorkLeaseId);
identifier!(ActorAttemptId);
identifier!(OutcomeObligationId);
identifier!(ContentSealReceiptId);
identifier!(ContentObjectId);
identifier!(ForensicManifestId);
identifier!(DeterministicExperimentId);
identifier!(EvaluatorRevisionId);
identifier!(InputManifestId);
identifier!(DeterministicEvaluationReceiptId);
identifier!(EvidenceAdmissionId);
identifier!(SupervisorEpochId);
identifier!(WorkspaceId);
identifier!(PiSessionId);
identifier!(PiChildSpawnAdmissionId);
identifier!(ChildProcessId);
identifier!(ChildProcessLivenessObservationId);
identifier!(ProcessSignalReceiptId);
identifier!(PiAbortControlReceiptId);
identifier!(ChildProcessReapReceiptId);
identifier!(ChildProcessRecoveryReceiptId);
identifier!(ChildStreamSealId);
identifier!(CancellationPropagationId);
identifier!(CancellationPropagationTargetId);

macro_rules! native_process_value {
    ($name:ident, $predicate:expr, $message:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub struct $name(i32);

        impl $name {
            pub fn new(value: i32) -> Option<Self> {
                if $predicate(value) {
                    Some(Self(value))
                } else {
                    None
                }
            }
            pub const fn value(self) -> i32 {
                self.0
            }
        }

        impl TryFrom<i32> for $name {
            type Error = DomainValueError;
            fn try_from(value: i32) -> Result<Self, Self::Error> {
                Self::new(value).ok_or(DomainValueError::InvalidNativeProcessValue {
                    type_name: stringify!($name),
                    value,
                    rule: $message,
                })
            }
        }
    };
}

native_process_value!(NativeChildPid, |value: i32| value > 0, "must be positive");
native_process_value!(
    OwnedProcessGroupId,
    |value: i32| value > 0,
    "must be positive"
);
native_process_value!(
    ProcessExitCode,
    |value: i32| (0..=255).contains(&value),
    "must be in the POSIX direct-child exit range 0..=255"
);
native_process_value!(
    ProcessSignalNumber,
    |value: i32| value > 0,
    "must be positive"
);

macro_rules! boundary_identity {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, PartialEq)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, DomainValueError> {
                let value = value.into();
                if value.is_empty()
                    || value.len() > 128
                    || !value.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                    })
                    || !value
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_alphanumeric)
                    || !value
                        .as_bytes()
                        .last()
                        .is_some_and(u8::is_ascii_alphanumeric)
                {
                    return Err(DomainValueError::InvalidOperationalIdentity {
                        type_name: stringify!($name),
                    });
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

// These opaque boundary identities use exactly the same portable ASCII grammar
// as `society-pi` and `PiSupervisor`: an ASCII alphanumeric byte at both
// ends, with only ASCII alphanumerics, `.`, `_`, and `-` in between. They are
// intentionally distinct so a workspace, SDK session, child correlation, and
// spawn nonce cannot be accidentally recombined at the trusted boundary.
boundary_identity!(NativeWorkspaceId);
boundary_identity!(PiBoundarySessionIdentity);
boundary_identity!(PiCorrelationIdentity);
boundary_identity!(SupervisedChildIdentity);
boundary_identity!(SpawnNonce);
boundary_identity!(SupervisorEpochIdentity);

/// A resolved POSIX custody path is not a workspace identity. The resident
/// supervisor supplies this only after its own canonicalization/private-root
/// checks; kernel persists it separately so a logical workspace name cannot
/// be mistaken for an executable filesystem location.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CanonicalWorkspacePath(String);

impl CanonicalWorkspacePath {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainValueError> {
        let value = value.into();
        if value.len() < 2
            || value.len() > 4096
            || !value.starts_with('/')
            || value.contains('\0')
            || value.bytes().any(|byte| byte.is_ascii_control())
            || value
                .split('/')
                .skip(1)
                .any(|part| part.is_empty() || part == "." || part == "..")
            || value.ends_with('/')
        {
            return Err(DomainValueError::InvalidOperationalIdentity {
                type_name: "CanonicalWorkspacePath",
            });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ExecutionProfileId {
    /// A provider-free process-double profile for deterministic fixture work.
    /// It is neither native Pi SDK qualification nor live actor authority.
    pub const DETERMINISTIC_PI_HOST_DOUBLE_V1: Self = Self(1);
    /// The pinned native profile identity. M3 records it as Unqualified;
    /// only the later PiSdkQualification path may make it live-admissible.
    pub const NATIVE_PINNED_PI_SDK_V1: Self = Self(2);
}

impl PrincipalId {
    /// The compiled, local founding authority. It is installed by migration,
    /// not selected through an environment variable or user-supplied string.
    pub const BOOTSTRAP: Self = Self(1);
    /// A narrow kernel service principal. It cannot be represented at the
    /// actor/Pi boundary and is used only for lifecycle facts the kernel owns.
    pub const KERNEL: Self = Self(2);
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CommandId(String);

impl CommandId {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainValueError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| byte.is_ascii_graphic())
        {
            return Err(DomainValueError::InvalidCommandId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SocietyName(String);

impl SocietyName {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainValueError> {
        let value = value.into();
        if value.trim().is_empty() || value.len() > 160 || value.contains('\0') {
            return Err(DomainValueError::InvalidSocietyName);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A display value for a durable Principal. It is descriptive only; the
/// generated identifier and exact capability grant remain the authority.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PrincipalDisplayName(String);

impl PrincipalDisplayName {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainValueError> {
        let value = value.into();
        if value.trim().is_empty() || value.len() > 160 || value.contains('\0') {
            return Err(DomainValueError::InvalidPrincipalDisplayName);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

macro_rules! coordination_text {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, PartialEq)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, DomainValueError> {
                let value = value.into();
                if value.trim().is_empty() || value.len() > 1_024 || value.contains('\0') {
                    return Err(DomainValueError::InvalidCoordinationText {
                        type_name: stringify!($name),
                    });
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

coordination_text!(ProjectName);
coordination_text!(ProjectObjectiveText);
coordination_text!(ProjectMilestoneName);
coordination_text!(ProjectStopConditionText);
coordination_text!(TicketTitle);
coordination_text!(TicketAcceptanceConditionText);
coordination_text!(ObservationRevisionText);
coordination_text!(HypothesisRevisionText);
coordination_text!(ReviewFailureHypothesis);
coordination_text!(ReviewResponseText);
coordination_text!(PostmortemCausalClaimText);
coordination_text!(PostmortemActionProposalText);
coordination_text!(ActorConfigurationName);
coordination_text!(WorkAssignmentText);
coordination_text!(OutcomeObligationText);
coordination_text!(EvidenceLimitationText);

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for Sha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UsdMicros(i64);

impl UsdMicros {
    pub const ZERO: Self = Self(0);
    pub const VS001_SOCIETY_HARD_CEILING: Self = Self(1_030_000);
    pub const VS001_QUALIFICATION_CEILING: Self = Self(30_000);
    pub const VS001_CYCLE_CEILING: Self = Self(1_000_000);

    pub const fn new(value: i64) -> Option<Self> {
        if value >= 0 { Some(Self(value)) } else { None }
    }

    pub const fn value(self) -> i64 {
        self.0
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).and_then(Self::new)
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).and_then(Self::new)
    }
}

impl TryFrom<i64> for UsdMicros {
    type Error = DomainValueError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(DomainValueError::NegativeUsdMicros(value))
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AdmissionGeneration(i64);

impl AdmissionGeneration {
    pub const INITIAL: Self = Self(0);

    pub const fn value(self) -> i64 {
        self.0
    }

    pub fn increment(self) -> Result<Self, DomainValueError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(DomainValueError::GenerationOverflow)
    }
}

impl TryFrom<i64> for AdmissionGeneration {
    type Error = DomainValueError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value < 0 {
            return Err(DomainValueError::NegativeAdmissionGeneration(value));
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PrincipalKind {
    Bootstrap = 1,
    KernelService = 2,
    Actor = 3,
    User = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum Capability {
    CreateSocietyIdentity = 1,
    InstallGrandArchitectOffice = 2,
    InstallFoundingUniverseSeed = 3,
    AppointInitialGrandArchitect = 4,
    SetR0HardCeiling = 5,
    BootstrapSociety = 6,
    ProposeOperatingCycle = 7,
    AdmitOperatingCycle = 8,
    QuiesceOperatingCycle = 9,
    ResumeOperatingCycle = 10,
    ReconcileOperatingCycle = 11,
    CloseOperatingCycle = 12,
    StartGrandArchitectOfficeSession = 13,
    OpenOfficeTurn = 14,
    RequestCancellation = 15,
    ReserveBudget = 16,
    ReconcileBudget = 17,
    RecordCycleDrained = 18,
    RecordOfficeSessionReady = 19,
    SettleOfficeTurn = 20,
    ReconcileCancellation = 21,
    RecordOfficeSessionTerminal = 22,
    CloseCostPostmortem = 23,
    CreateProject = 24,
    CharterProject = 25,
    TransitionProject = 26,
    CompleteProjectMilestone = 27,
    ReopenProject = 28,
    CreateTicket = 29,
    TransitionTicket = 30,
    AddGraphObjectRevision = 31,
    CommitGraphRevision = 32,
    AddGraphEdge = 33,
    CreateEpisode = 34,
    TransitionEpisode = 35,
    ReopenEpisode = 36,
    RequestAdversarialReview = 37,
    SubmitReviewChallenge = 38,
    RespondToReviewChallenge = 39,
    DispositionReviewChallenge = 40,
    ResolveAdversarialReview = 41,
    TriggerPostmortem = 42,
    RecordPostmortemCausalClaim = 43,
    ProposePostmortemAction = 44,
    ClosePostmortem = 45,
    /// Kernel-owned assignment makes an adversarial finding attributable to
    /// the exact reviewer authorized for its Review, rather than to any actor.
    AssignAdversarialReviewer = 46,
    RegisterActorConfiguration = 47,
    RegisterContextPack = 48,
    AdmitActorInstance = 49,
    AdmitTicket = 50,
    RegisterWorkItem = 51,
    ClaimWorkItem = 52,
    StartActorAttempt = 53,
    AttestActorAttemptTerminal = 54,
    ValidateTicketAttempt = 55,
    RetryActorAttempt = 56,
    CompleteTicket = 57,
    ExpireWorkLease = 58,
    CancelActorAttempt = 59,
    RegisterOutcomeObligation = 60,
    ResolveOutcomeObligation = 61,
    /// Records a content-store receipt only. It never asserts that bytes were
    /// read, evaluated, admitted, curated, or placed in the graph.
    RecordContentSealReceipt = 62,
    RegisterContentObject = 63,
    RegisterForensicManifest = 64,
    RegisterDeterministicExperiment = 65,
    RecordDeterministicEvaluationReceipt = 66,
    AdmitDeterministicEvidence = 67,
    CloseDeterministicExperiment = 68,
    AdmitPiChildSpawn = 69,
    RecordInertChildSpawn = 70,
    RecordPiAdapterReady = 71,
    AuthorizePiCreateSession = 72,
    RecordPiCreateSessionDelivery = 73,
    RecordPiSessionReady = 74,
    RecordChildStreamSeal = 75,
    RecordChildProcessLiveness = 76,
    RecordProcessSignalReceipt = 77,
    RecordDirectChildReap = 78,
    RecordChildRecovery = 79,
    FinalizeChildProcess = 80,
    BeginCancellationPropagation = 81,
    ReconcileCancellationPropagation = 82,
    /// Opens one exact resident-supervisor epoch before it can admit children.
    /// A caller cannot silently reuse a prior epoch as a new daemon lifetime.
    OpenSupervisorEpoch = 83,
    /// Records a Pi SDK Abort control separately from TERM/KILL process
    /// signals, binding exact canonical bytes to its correlation identity.
    RecordPiAbortControlDelivery = 84,
    /// Resolves an admitted-but-never-spawned child after cancellation froze
    /// its owner target. A later inert spawn is then impossible.
    RecordPiChildNotSpawned = 85,
}

impl Capability {
    pub const FOUNDING: [Self; 8] = [
        Self::CreateSocietyIdentity,
        Self::InstallGrandArchitectOffice,
        Self::InstallFoundingUniverseSeed,
        Self::AppointInitialGrandArchitect,
        Self::SetR0HardCeiling,
        Self::BootstrapSociety,
        Self::ProposeOperatingCycle,
        Self::AdmitOperatingCycle,
    ];

    pub const GRAND_ARCHITECT: [Self; 44] = [
        Self::ProposeOperatingCycle,
        Self::AdmitOperatingCycle,
        Self::QuiesceOperatingCycle,
        Self::ResumeOperatingCycle,
        Self::ReconcileOperatingCycle,
        Self::CloseOperatingCycle,
        Self::StartGrandArchitectOfficeSession,
        Self::OpenOfficeTurn,
        Self::RequestCancellation,
        Self::ReserveBudget,
        Self::CloseCostPostmortem,
        Self::CreateProject,
        Self::CharterProject,
        Self::TransitionProject,
        Self::CompleteProjectMilestone,
        Self::ReopenProject,
        Self::CreateTicket,
        Self::TransitionTicket,
        Self::AddGraphObjectRevision,
        Self::CommitGraphRevision,
        Self::AddGraphEdge,
        Self::CreateEpisode,
        Self::TransitionEpisode,
        Self::ReopenEpisode,
        Self::RequestAdversarialReview,
        Self::RespondToReviewChallenge,
        Self::DispositionReviewChallenge,
        Self::ResolveAdversarialReview,
        Self::TriggerPostmortem,
        Self::RecordPostmortemCausalClaim,
        Self::ProposePostmortemAction,
        Self::ClosePostmortem,
        Self::RegisterActorConfiguration,
        Self::RegisterContextPack,
        Self::AdmitActorInstance,
        Self::AdmitTicket,
        Self::RegisterWorkItem,
        Self::StartActorAttempt,
        Self::RetryActorAttempt,
        Self::CompleteTicket,
        Self::RegisterOutcomeObligation,
        Self::ResolveOutcomeObligation,
        Self::RegisterDeterministicExperiment,
        Self::CloseDeterministicExperiment,
    ];

    pub const KERNEL_SERVICE: [Self; 34] = [
        Self::RecordCycleDrained,
        Self::RecordOfficeSessionReady,
        Self::SettleOfficeTurn,
        Self::ReconcileBudget,
        Self::ReconcileCancellation,
        Self::RecordOfficeSessionTerminal,
        Self::SubmitReviewChallenge,
        Self::AssignAdversarialReviewer,
        Self::AttestActorAttemptTerminal,
        Self::ValidateTicketAttempt,
        Self::ExpireWorkLease,
        Self::CancelActorAttempt,
        Self::RecordContentSealReceipt,
        Self::RegisterContentObject,
        Self::RegisterForensicManifest,
        Self::RecordDeterministicEvaluationReceipt,
        Self::AdmitDeterministicEvidence,
        Self::AdmitPiChildSpawn,
        Self::RecordInertChildSpawn,
        Self::RecordPiAdapterReady,
        Self::AuthorizePiCreateSession,
        Self::RecordPiCreateSessionDelivery,
        Self::RecordPiSessionReady,
        Self::RecordChildStreamSeal,
        Self::RecordChildProcessLiveness,
        Self::RecordProcessSignalReceipt,
        Self::RecordDirectChildReap,
        Self::RecordChildRecovery,
        Self::FinalizeChildProcess,
        Self::BeginCancellationPropagation,
        Self::ReconcileCancellationPropagation,
        Self::OpenSupervisorEpoch,
        Self::RecordPiAbortControlDelivery,
        Self::RecordPiChildNotSpawned,
    ];

    pub const fn requires_consumption(self) -> bool {
        matches!(
            self,
            Self::CreateSocietyIdentity
                | Self::InstallGrandArchitectOffice
                | Self::InstallFoundingUniverseSeed
                | Self::AppointInitialGrandArchitect
                | Self::SetR0HardCeiling
                | Self::BootstrapSociety
                | Self::ProposeOperatingCycle
                | Self::AdmitOperatingCycle
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum OfficeKind {
    TheGrandArchitect = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum OperatingCycleState {
    Proposed = 1,
    Admitted = 2,
    Running = 3,
    Quiescing = 4,
    Drained = 5,
    Reconciling = 6,
    Closed = 7,
    Cancelling = 8,
    Reaping = 9,
    Cancelled = 10,
    Failed = 11,
}

impl OperatingCycleState {
    pub const fn is_nonterminal(self) -> bool {
        !matches!(self, Self::Closed | Self::Cancelled | Self::Failed)
    }

    pub const fn admits_task_work(self) -> bool {
        matches!(self, Self::Running)
    }
}

/// The finite operating treatments permitted by VS-001. A treatment carries
/// its constitutional budget exactly; callers never select an arbitrary
/// ceiling for a cycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum OperatingCycleTreatment {
    PiSdkQualificationV1 = 1,
    Vs001LiveV1 = 2,
    /// Provider-free process-double treatment for trusted-kernel/supervisor
    /// fixtures. It carries VS-001's logical envelope but denies provider
    /// access and cannot stand in for the paid native qualification run.
    Vs001DeterministicV1 = 3,
}

/// M2 contains the planning and closure-blocker portion of the Project
/// lifecycle. Product execution and delivery remain at later typed Actor,
/// WorkItem, and Attempt boundaries; these states do not claim completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ProjectState {
    Proposed = 1,
    Challenged = 2,
    Chartered = 3,
    Active = 4,
    Paused = 5,
    Observing = 6,
    Closed = 7,
    Terminated = 8,
    Reopened = 9,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ProjectMilestoneState {
    Pending = 1,
    Completed = 2,
}

/// The execution labels are retained as closed domain vocabulary, but M2
/// permits only preparation/cancellation transitions. Claim, work, validation,
/// delivery, and retry need later typed Actor, WorkItem, and Attempt commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum TicketState {
    Draft = 1,
    Admitted = 2,
    Ready = 3,
    Claimed = 4,
    Submitted = 5,
    Verified = 6,
    Completed = 7,
    ChangesRequested = 8,
    Expired = 9,
    Cancelled = 10,
    Failed = 11,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum GraphObjectKind {
    Observation = 1,
    Hypothesis = 2,
}

/// The M2 graph intentionally supports only the two epistemic bodies its
/// coordination proof can faithfully create. The common revision row carries
/// identity and lifecycle only; this closed body chooses the named, semantic
/// one-to-one revision table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphRevisionBody {
    Observation {
        observation: ObservationRevisionText,
    },
    Hypothesis {
        hypothesis: HypothesisRevisionText,
    },
}

impl GraphRevisionBody {
    pub const fn object_kind(&self) -> GraphObjectKind {
        match self {
            Self::Observation { .. } => GraphObjectKind::Observation,
            Self::Hypothesis { .. } => GraphObjectKind::Hypothesis,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum GraphRevisionState {
    Draft = 1,
    Committed = 2,
}

/// The graph is deliberately not a universal relation bucket. Each edge kind
/// has a finite endpoint matrix enforced by the kernel before it is stored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum GraphEdgeKind {
    Supports = 1,
    Challenges = 2,
}

impl GraphEdgeKind {
    pub const fn allows(self, from: GraphObjectKind, to: GraphObjectKind) -> bool {
        match self {
            Self::Supports => matches!(
                (from, to),
                (
                    GraphObjectKind::Observation | GraphObjectKind::Hypothesis,
                    GraphObjectKind::Hypothesis
                )
            ),
            Self::Challenges => matches!(
                (from, to),
                (GraphObjectKind::Observation, GraphObjectKind::Hypothesis)
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum EpisodeState {
    Framed = 1,
    Admitted = 2,
    Investigating = 3,
    PrototypeDeliberating = 4,
    Prototyping = 5,
    CandidateValidating = 6,
    DeliveryDeliberating = 7,
    DeliveryAuthorized = 8,
    Materializing = 9,
    Observing = 10,
    Learning = 11,
    Closed = 12,
    ClosedNoAction = 13,
    ClosedNoDelivery = 14,
    Abandoned = 15,
    Reverted = 16,
    Reopened = 17,
}

/// M2 keeps Reviews as durable adverse findings and Project-close blockers.
/// Complete Review resolution awaits a later typed independent Actor,
/// WorkItem, and Attempt path; this tranche does not fabricate one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum AdversarialReviewState {
    Requested = 1,
    Assigned = 2,
    Active = 3,
    FindingsSubmitted = 4,
    ResponsesDue = 5,
    Resolved = 6,
    AcceptedRisk = 7,
    Superseded = 8,
    Escalated = 9,
}

/// The M3 execution foundation permits only the one pinned VS-001 model
/// policy. A future policy mutation needs its own versioned, qualified path;
/// a provider/model string may not quietly change an Actor's identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ActorModelPolicy {
    Vs001DeepseekV4FlashHigh = 1,
}

/// Developmental attractors are explicit treatment biases, never authority
/// titles. M3 records one primary bias per configuration revision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum DevelopmentalAttractor {
    Explore = 1,
    Build = 2,
    Measure = 3,
    Challenge = 4,
    Synthesize = 5,
    Integrate = 6,
    Remember = 7,
    Coordinate = 8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ActorConfigurationState {
    Active = 1,
    Retired = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ActorInstanceState {
    Active = 1,
    Retired = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ExecutionProfileKind {
    DeterministicPiHostProcessDoubleV1 = 1,
    NativePinnedPiSdkV1 = 2,
}

/// This is deliberately not a boolean "qualified" flag. The process-double
/// has a narrow provider-free deterministic-fixture role, while the native Pi
/// SDK identity remains unqualified until a future typed M8 fact exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ExecutionProfileReadiness {
    DeterministicFixtureOnly = 1,
    Unqualified = 2,
    QualifiedForLiveUse = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CapabilityGrantOrigin {
    CompiledBootstrap = 1,
    LedgerCommand = 2,
    CompiledKernelService = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ContextPackPurpose {
    TicketExecution = 1,
    IndependentReview = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum WorkItemKind {
    TicketExecution = 1,
    IndependentReview = 2,
}

impl WorkItemKind {
    pub const fn required_context_purpose(self) -> ContextPackPurpose {
        match self {
            Self::TicketExecution => ContextPackPurpose::TicketExecution,
            Self::IndependentReview => ContextPackPurpose::IndependentReview,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum WorkItemState {
    Ready = 1,
    Claimed = 2,
    Running = 3,
    Settled = 4,
    Cancelled = 5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum WorkLeaseState {
    Active = 1,
    Released = 2,
    Expired = 3,
    Cancelled = 4,
}

/// The only durable owners of a native child.  An owner is an exact union,
/// never a polymorphic string: a child belongs either to one ActorAttempt or
/// to the Grand Architect's Office session that initiated it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PiChildOwner {
    ActorAttempt(ActorAttemptId),
    GrandArchitectOfficeSession(GrandArchitectOfficeSessionId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PiChildSpawnAdmissionState {
    Admitted = 1,
    Spawned = 2,
    Invalidated = 3,
}

/// Generic OS-child lifecycle. Pi protocol phases are deliberately in the
/// one-to-one Pi session sidecar, so a deterministic evaluator can later use
/// the same process, signal, reaping, and containment physics without a fake
/// adapter-ready/Create/nonce state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ChildProcessState {
    Spawned = 1,
    Running = 2,
    CancellationRequested = 3,
    DirectChildReaped = 4,
    /// The parent process is gone but the owned group was still present after
    /// recovery. Only containment/liveness receipts may advance this state;
    /// no Pi protocol or wait(2) fact may recreate supervision authority.
    RecoveryContainmentRequired = 5,
    /// Exact post-restart absence proves the supervisor lost parentage but
    /// has no remaining owned group to contain. It remains a close blocker.
    LostParentage = 6,
    ContainmentFailed = 7,
    Finalized = 8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PiChildSessionState {
    InertSpawned = 1,
    AdapterReady = 2,
    CreateAuthorized = 3,
    CreateDelivered = 4,
    SessionReady = 5,
}

impl ChildProcessState {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Finalized | Self::LostParentage | Self::ContainmentFailed
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ChildStreamKind {
    AdmittedControl = 1,
    PhysicalStdin = 2,
    Stdout = 3,
    Stderr = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ChildStreamSealCompleteness {
    Complete = 1,
    PrefixBounded = 2,
    CountOverflow = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ProcessGroupLiveness {
    Present = 1,
    Absent = 2,
    Inaccessible = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ProcessSignalAction {
    Terminate = 1,
    Kill = 2,
    LingeringGroupKill = 3,
}

/// Physical result of one canonical Pi `Abort` control write. This is not an
/// OS signal receipt: the abort correlation and digest name the exact JSONL
/// command whose bytes were (or were not) handed to the host pipe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PiAbortControlWriteOutcome {
    FullyWritten = 1,
    PipeClosedBeforeWrite = 2,
    WriteFailed = 3,
    /// A nonblocking write staged a prefix which cancellation/deadline then
    /// discarded. The physical stdin seal, not this receipt, owns that prefix.
    PartialWriteDiscarded = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ProcessSignalDelivery {
    Delivered = 1,
    AbsentBeforeSignal = 2,
    AbsentDuringSignal = 3,
    Inaccessible = 4,
}

/// A signal receipt is either an intentional part of one durably snapshotted
/// cancellation propagation or a narrow automatic containment action.  It is
/// not a free-form supervisor annotation: cancellation signals must be
/// attributable to the exact target set that was frozen before escalation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessSignalCause {
    CancellationPropagation(CancellationPropagationId),
    AutomaticBoundaryContainment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectChildWaitStatus {
    Exited { exit_code: ProcessExitCode },
    Signaled { signal_number: ProcessSignalNumber },
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ChildRecoveryObservation {
    /// The resident supervisor no longer has a parentage-preserving handle.
    /// The separately recorded group observation determines whether this is
    /// terminal absence, live containment work, or inaccessible containment.
    ParentageLost = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PiChildNotSpawnedReason {
    CancelledBeforeSpawn = 1,
    WorkspacePreparationFailed = 2,
    ArtifactQualificationFailed = 3,
    NativeSpawnFailed = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ChildTerminalDisposition {
    /// Exact exit status remains on `DirectChildWaitStatus`; M5 does not call
    /// a nonzero process exit "normal" or infer a model outcome from it.
    Exited = 1,
    Terminated = 4,
    Killed = 5,
    SupervisionLost = 6,
    ContainmentFailed = 7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CancellationPropagationState {
    Propagating = 1,
    Reconciled = 2,
    ContainmentFailed = 3,
}

/// The frozen cancellation target set retains more than a boolean "done".
/// It records whether an owner never had a child, still owes a child receipt,
/// or which physical outcome blocks/allows the propagation to advance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CancellationPropagationTargetDisposition {
    NotRunning = 1,
    AwaitingChildReceipt = 2,
    Exited = 3,
    Terminated = 4,
    Killed = 5,
    ContainmentFailed = 6,
    SupervisionLost = 7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ActorAttemptState {
    Running = 1,
    CancellationRequested = 2,
    Succeeded = 3,
    Failed = 4,
    Cancelled = 5,
    Expired = 6,
    ProtocolFailed = 7,
    SupervisorFailed = 8,
    Validated = 9,
}

impl ActorAttemptState {
    pub const fn is_live(self) -> bool {
        matches!(self, Self::Running | Self::CancellationRequested)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
/// A validated use contract on an evaluator revision, input-manifest binding,
/// or manifest member. It never classifies the global `ContentObject` bytes.
pub enum ContentMediaSchemaContract {
    DeterministicEvaluatorV1 = 1,
    DeterministicInputManifestV1 = 2,
    DeterministicEvaluatorOutputV1 = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum RetentionAccessClass {
    ForensicRestricted = 1,
    ProjectScoped = 2,
}

/// A manifest is a forensic inventory with a bounded capture policy. It is
/// not a curation account and cannot make an object decision-relevant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ForensicManifestCapturePolicy {
    DeterministicExperimentEvaluatorV1 = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum DeterministicExperimentState {
    Registered = 1,
    EvidenceAdmitted = 2,
    Closed = 3,
}

/// The only M4 semantic admission role. Graph nodes, curation, influence,
/// epistemic truth, and decision relevance remain intentionally separate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum EvidenceSemanticRole {
    DeterministicObservation = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum EvidenceApplicability {
    TestsTargetHypothesis = 1,
}

/// This is a kernel-service terminal attestation, not a claim that Pi or a
/// process receipt already exists. The supervisor/evidence tranche must later
/// bind it to those independently normalized receipts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ActorAttemptTerminalKind {
    Succeeded = 1,
    Failed = 2,
    Cancelled = 3,
    Expired = 4,
    ProtocolFailed = 5,
    SupervisorFailed = 6,
}

impl ActorAttemptTerminalKind {
    pub const fn state(self) -> ActorAttemptState {
        match self {
            Self::Succeeded => ActorAttemptState::Succeeded,
            Self::Failed => ActorAttemptState::Failed,
            Self::Cancelled => ActorAttemptState::Cancelled,
            Self::Expired => ActorAttemptState::Expired,
            Self::ProtocolFailed => ActorAttemptState::ProtocolFailed,
            Self::SupervisorFailed => ActorAttemptState::SupervisorFailed,
        }
    }

    pub const fn permits_validation(self) -> bool {
        matches!(self, Self::Succeeded)
    }

    /// Cancellation is a control protocol, not a terminal label an attestor
    /// may attach retrospectively. Ordinary terminal facts come directly from
    /// `Running`; only a cancellation-requested Attempt may become
    /// `Cancelled`.
    pub const fn allowed_from(self, state: ActorAttemptState) -> bool {
        matches!(
            (state, self),
            (
                ActorAttemptState::Running,
                Self::Succeeded
                    | Self::Failed
                    | Self::Expired
                    | Self::ProtocolFailed
                    | Self::SupervisorFailed
            ) | (ActorAttemptState::CancellationRequested, Self::Cancelled)
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ActorAttemptCancellationReason {
    GrandArchitectRequested = 1,
    CycleCancellation = 2,
    LeaseContainment = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum OutcomeObligationState {
    Scheduled = 1,
    Satisfied = 2,
    Waived = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum OutcomeObligationDisposition {
    Satisfied = 1,
    Waived = 2,
}

impl OutcomeObligationDisposition {
    pub const fn state(self) -> OutcomeObligationState {
        match self {
            Self::Satisfied => OutcomeObligationState::Satisfied,
            Self::Waived => OutcomeObligationState::Waived,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ReviewChallengeSeverity {
    Low = 1,
    Moderate = 2,
    High = 3,
    Critical = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ReviewChallengeResponseState {
    Pending = 1,
    Responded = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ReviewDispositionKind {
    Addressed = 1,
    RejectedWithDissentPreserved = 2,
    AcceptedRisk = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ReviewResolutionKind {
    Resolved = 1,
    AcceptedRisk = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PostmortemState {
    Triggered = 1,
    Investigating = 2,
    Closed = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PostmortemCausalClaimKind {
    ContributingCondition = 1,
    Counterfactual = 2,
    Containment = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PostmortemActionKind {
    CreateFollowUpTicket = 1,
    ChangePolicyProposal = 2,
}

impl OperatingCycleTreatment {
    pub const fn budget_ceiling(self) -> UsdMicros {
        match self {
            Self::PiSdkQualificationV1 => UsdMicros::VS001_QUALIFICATION_CEILING,
            Self::Vs001LiveV1 | Self::Vs001DeterministicV1 => UsdMicros::VS001_CYCLE_CEILING,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum OfficeSessionState {
    Reserved = 1,
    Starting = 2,
    Ready = 3,
    TurnActive = 4,
    Quiescing = 5,
    ProcessEnded = 6,
    EvidenceSealing = 7,
    Closed = 8,
    Cancelling = 9,
    Cancelled = 10,
    Failed = 11,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum OfficeTurnState {
    Active = 1,
    Settled = 2,
    Cancelled = 3,
    Failed = 4,
}

/// An Office turn's purpose is an admission boundary. Ordinary work is allowed
/// only while the cycle runs; control purposes are bounded recovery work after
/// normal work has been fenced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum OfficeTurnPurpose {
    OrdinaryWork = 1,
    Recovery = 2,
    Cancellation = 3,
    Closure = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum OfficeSessionTerminalState {
    Closed = 1,
    Cancelled = 2,
    Failed = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CancellationMode {
    Quiesce = 1,
    GracefulCancel = 2,
    EmergencyStop = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CancellationState {
    Requested = 1,
    Accepted = 2,
    Propagating = 3,
    AwaitingGrace = 4,
    Terminating = 5,
    Killing = 6,
    Reconciling = 7,
    Completed = 8,
    ContainmentFailed = 9,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum BudgetReservationState {
    Reserved = 1,
    Reconciled = 2,
    Frozen = 3,
}

/// A bounded causal class for the automatically opened cost Postmortem.
/// Detailed accounting values stay in normalized columns; an acknowledgement
/// string cannot be used to discharge an uncertain or over-cap charge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CostPostmortemCause {
    KnownOverrun = 1,
    UnknownCost = 2,
    UnavailableCost = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CostPostmortemResolution {
    ConservativeFullReservation = 1,
    ChargeObservedOverrun = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CostPostmortemState {
    Open = 1,
    Closed = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CostUnknownReason {
    ProviderDidNotReport = 1,
    AdapterStreamInterrupted = 2,
    ReconciliationMismatch = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CostUnavailableReason {
    ProviderUnavailable = 1,
    CredentialUnavailable = 2,
    QualificationRejected = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CostObservation {
    Known(UsdMicros),
    Unknown(CostUnknownReason),
    Unavailable(CostUnavailableReason),
}

/// A closed, replayable explanation for an admission fence caused by cost.
/// Frozen reservations are deliberately nonterminal: they retain their charge
/// until a future typed resolution or Postmortem command accounts for it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetFreezeReason {
    KnownOverrun {
        observed: UsdMicros,
        reserved: UsdMicros,
    },
    Unknown(CostUnknownReason),
    Unavailable(CostUnavailableReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpectedGeneration {
    NotApplicable,
    Exact(AdmissionGeneration),
}

impl ExpectedGeneration {
    pub const fn exact(value: AdmissionGeneration) -> Self {
        Self::Exact(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandBody {
    CreateSocietyIdentity {
        name: SocietyName,
    },
    InstallGrandArchitectOffice,
    InstallFoundingUniverseSeed {
        rendering_digest: Sha256Digest,
    },
    AppointInitialGrandArchitect {
        actor_display_name: PrincipalDisplayName,
    },
    SetR0HardCeiling {
        ceiling: UsdMicros,
    },
    BootstrapSociety,
    ProposeOperatingCycle {
        treatment: OperatingCycleTreatment,
    },
    AdmitOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    StartGrandArchitectOfficeSession {
        cycle_id: OperatingCycleId,
    },
    RecordOfficeSessionReady {
        session_id: GrandArchitectOfficeSessionId,
    },
    RecordOfficeSessionTerminal {
        session_id: GrandArchitectOfficeSessionId,
        terminal_state: OfficeSessionTerminalState,
    },
    OpenOfficeTurn {
        session_id: GrandArchitectOfficeSessionId,
        purpose: OfficeTurnPurpose,
    },
    SettleOfficeTurn {
        turn_id: OfficeTurnId,
    },
    QuiesceOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    RecordCycleDrained {
        cycle_id: OperatingCycleId,
    },
    ResumeOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    ReconcileOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    CloseOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    ReserveBudget {
        cycle_id: OperatingCycleId,
        amount: UsdMicros,
    },
    ReconcileBudget {
        reservation_id: BudgetReservationId,
        observation: CostObservation,
    },
    RequestCancellation {
        cycle_id: OperatingCycleId,
        mode: CancellationMode,
    },
    ReconcileCancellation {
        cancellation_request_id: CancellationRequestId,
    },
    CloseCostPostmortem {
        postmortem_id: CostPostmortemId,
        resolution: CostPostmortemResolution,
    },
    CreateProject {
        operating_cycle_id: OperatingCycleId,
        project_name: ProjectName,
    },
    CharterProject {
        operating_cycle_id: OperatingCycleId,
        project_id: ProjectId,
        objective: ProjectObjectiveText,
        initial_milestone: ProjectMilestoneName,
        stop_condition: ProjectStopConditionText,
    },
    TransitionProject {
        operating_cycle_id: OperatingCycleId,
        project_id: ProjectId,
        target: ProjectState,
    },
    CompleteProjectMilestone {
        operating_cycle_id: OperatingCycleId,
        project_milestone_id: ProjectMilestoneId,
    },
    ReopenProject {
        operating_cycle_id: OperatingCycleId,
        project_id: ProjectId,
    },
    CreateTicket {
        operating_cycle_id: OperatingCycleId,
        project_id: ProjectId,
        ticket_title: TicketTitle,
        acceptance_condition: TicketAcceptanceConditionText,
        prerequisite_ticket_id: Option<TicketId>,
    },
    TransitionTicket {
        operating_cycle_id: OperatingCycleId,
        ticket_id: TicketId,
        target: TicketState,
    },
    AddGraphObjectRevision {
        operating_cycle_id: OperatingCycleId,
        project_id: ProjectId,
        causal_episode_id: Option<CausalEpisodeId>,
        graph_object_id: Option<GraphObjectId>,
        body: GraphRevisionBody,
    },
    CommitGraphRevision {
        operating_cycle_id: OperatingCycleId,
        graph_revision_id: GraphRevisionId,
    },
    AddGraphEdge {
        operating_cycle_id: OperatingCycleId,
        project_id: ProjectId,
        from_graph_revision_id: GraphRevisionId,
        to_graph_revision_id: GraphRevisionId,
        edge_kind: GraphEdgeKind,
    },
    CreateEpisode {
        operating_cycle_id: OperatingCycleId,
        project_id: ProjectId,
    },
    TransitionEpisode {
        operating_cycle_id: OperatingCycleId,
        causal_episode_id: CausalEpisodeId,
        target: EpisodeState,
    },
    ReopenEpisode {
        operating_cycle_id: OperatingCycleId,
        causal_episode_id: CausalEpisodeId,
    },
    RequestAdversarialReview {
        operating_cycle_id: OperatingCycleId,
        project_id: ProjectId,
        target_graph_revision_id: GraphRevisionId,
    },
    AssignAdversarialReviewer {
        operating_cycle_id: OperatingCycleId,
        adversarial_review_id: AdversarialReviewId,
        reviewer_principal_id: PrincipalId,
        reviewer_actor_instance_id: ActorInstanceId,
        reviewer_actor_attempt_id: ActorAttemptId,
    },
    SubmitReviewChallenge {
        operating_cycle_id: OperatingCycleId,
        adversarial_review_id: AdversarialReviewId,
        target_graph_revision_id: GraphRevisionId,
        author_principal_id: PrincipalId,
        severity: ReviewChallengeSeverity,
        failure_hypothesis: ReviewFailureHypothesis,
    },
    RespondToReviewChallenge {
        operating_cycle_id: OperatingCycleId,
        review_challenge_id: ReviewChallengeId,
        response: ReviewResponseText,
    },
    DispositionReviewChallenge {
        operating_cycle_id: OperatingCycleId,
        review_challenge_id: ReviewChallengeId,
        disposition: ReviewDispositionKind,
    },
    ResolveAdversarialReview {
        operating_cycle_id: OperatingCycleId,
        adversarial_review_id: AdversarialReviewId,
        resolution: ReviewResolutionKind,
    },
    TriggerPostmortem {
        operating_cycle_id: OperatingCycleId,
        project_id: ProjectId,
        causal_episode_id: Option<CausalEpisodeId>,
    },
    RecordPostmortemCausalClaim {
        operating_cycle_id: OperatingCycleId,
        postmortem_id: PostmortemId,
        claim_kind: PostmortemCausalClaimKind,
        claim: PostmortemCausalClaimText,
    },
    ProposePostmortemAction {
        operating_cycle_id: OperatingCycleId,
        postmortem_id: PostmortemId,
        action_kind: PostmortemActionKind,
        action: PostmortemActionProposalText,
    },
    ClosePostmortem {
        operating_cycle_id: OperatingCycleId,
        postmortem_id: PostmortemId,
    },
    RegisterActorConfiguration {
        configuration_name: ActorConfigurationName,
        model_policy: ActorModelPolicy,
        primary_attractor: DevelopmentalAttractor,
    },
    RegisterContextPack {
        operating_cycle_id: OperatingCycleId,
        purpose: ContextPackPurpose,
        rendering_digest: Sha256Digest,
    },
    AdmitActorInstance {
        operating_cycle_id: OperatingCycleId,
        actor_configuration_revision_id: ActorConfigurationRevisionId,
        execution_profile_id: ExecutionProfileId,
        actor_display_name: PrincipalDisplayName,
    },
    AdmitTicket {
        operating_cycle_id: OperatingCycleId,
        ticket_id: TicketId,
    },
    RegisterWorkItem {
        operating_cycle_id: OperatingCycleId,
        ticket_id: TicketId,
        actor_instance_id: ActorInstanceId,
        context_pack_id: ContextPackId,
        work_kind: WorkItemKind,
        adversarial_review_id: Option<AdversarialReviewId>,
        assignment: WorkAssignmentText,
    },
    ClaimWorkItem {
        operating_cycle_id: OperatingCycleId,
        work_item_id: WorkItemId,
    },
    StartActorAttempt {
        operating_cycle_id: OperatingCycleId,
        work_item_id: WorkItemId,
        reservation_amount: UsdMicros,
    },
    AttestActorAttemptTerminal {
        actor_attempt_id: ActorAttemptId,
        terminal_kind: ActorAttemptTerminalKind,
    },
    ValidateTicketAttempt {
        operating_cycle_id: OperatingCycleId,
        actor_attempt_id: ActorAttemptId,
    },
    RetryActorAttempt {
        operating_cycle_id: OperatingCycleId,
        actor_attempt_id: ActorAttemptId,
    },
    CompleteTicket {
        operating_cycle_id: OperatingCycleId,
        actor_attempt_id: ActorAttemptId,
    },
    ExpireWorkLease {
        work_lease_id: WorkLeaseId,
    },
    CancelActorAttempt {
        actor_attempt_id: ActorAttemptId,
        reason: ActorAttemptCancellationReason,
    },
    RegisterOutcomeObligation {
        operating_cycle_id: OperatingCycleId,
        project_id: ProjectId,
        obligation: OutcomeObligationText,
    },
    ResolveOutcomeObligation {
        operating_cycle_id: OperatingCycleId,
        outcome_obligation_id: OutcomeObligationId,
        disposition: OutcomeObligationDisposition,
    },
    /// A later content-store integration attests an already-sealed digest.
    /// This command intentionally does not receive bytes or claim physical
    /// sealing/evaluator execution happened inside the kernel. A digest is a
    /// global byte identity, so producer/capture occurrence belongs on a
    /// `ForensicManifest`, never on this receipt.
    RecordContentSealReceipt {
        digest: Sha256Digest,
    },
    /// Turns a verified digest receipt into a global content identity. This is
    /// still forensic storage identity, not an occurrence-specific schema,
    /// retention policy, evidence admission, or graph node.
    RegisterContentObject {
        content_seal_receipt_id: ContentSealReceiptId,
    },
    RegisterForensicManifest {
        operating_cycle_id: OperatingCycleId,
        producing_deterministic_experiment_id: DeterministicExperimentId,
        capture_policy: ForensicManifestCapturePolicy,
        retention_access_class: RetentionAccessClass,
        evaluator_output_content_object_id: ContentObjectId,
    },
    RegisterDeterministicExperiment {
        operating_cycle_id: OperatingCycleId,
        project_id: ProjectId,
        ticket_id: TicketId,
        target_graph_revision_id: GraphRevisionId,
        evaluator_content_object_id: ContentObjectId,
        input_manifest_content_object_id: ContentObjectId,
    },
    /// A kernel-service binding fact supplied by a later evaluator adapter.
    /// It does not execute an evaluator or establish the observation's truth.
    RecordDeterministicEvaluationReceipt {
        operating_cycle_id: OperatingCycleId,
        deterministic_experiment_id: DeterministicExperimentId,
        evaluator_revision_id: EvaluatorRevisionId,
        input_manifest_id: InputManifestId,
        forensic_manifest_id: ForensicManifestId,
        evaluator_output_content_object_id: ContentObjectId,
    },
    AdmitDeterministicEvidence {
        operating_cycle_id: OperatingCycleId,
        deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId,
        deterministic_experiment_id: DeterministicExperimentId,
        evaluator_revision_id: EvaluatorRevisionId,
        input_manifest_id: InputManifestId,
        evaluator_output_content_object_id: ContentObjectId,
        related_graph_revision_id: GraphRevisionId,
        semantic_role: EvidenceSemanticRole,
        applicability: EvidenceApplicability,
        limitation: EvidenceLimitationText,
    },
    CloseDeterministicExperiment {
        operating_cycle_id: OperatingCycleId,
        deterministic_experiment_id: DeterministicExperimentId,
    },
    /// The sole M5 opening fact for a resident supervisor lifetime. The epoch
    /// has event-sequence ordering only; it does not claim a durable clock.
    OpenSupervisorEpoch {
        supervisor_epoch_id: SupervisorEpochId,
        supervisor_epoch_identity: SupervisorEpochIdentity,
    },
    /// Pre-spawn authority is durable before native process creation.  The
    /// caller cannot create a child by merely presenting an owner ID: this
    /// binds the exact active reservation, profile, generation, workspace,
    /// supervisor epoch, and Pi boundary session/nonce.
    AdmitPiChildSpawn {
        operating_cycle_id: OperatingCycleId,
        owner: PiChildOwner,
        budget_reservation_id: BudgetReservationId,
        execution_profile_id: ExecutionProfileId,
        native_workspace_id: NativeWorkspaceId,
        canonical_workspace_path: CanonicalWorkspacePath,
        supervisor_epoch_id: SupervisorEpochId,
        supervisor_epoch_identity: SupervisorEpochIdentity,
        pi_session_identity: PiBoundarySessionIdentity,
        spawn_nonce: SpawnNonce,
    },
    RecordInertChildSpawn {
        pi_child_spawn_admission_id: PiChildSpawnAdmissionId,
        child_identity: SupervisedChildIdentity,
        direct_child_pid: NativeChildPid,
        process_group_id: OwnedProcessGroupId,
    },
    RecordPiAdapterReady {
        child_process_id: ChildProcessId,
        pi_session_identity: PiBoundarySessionIdentity,
        spawn_nonce: SpawnNonce,
    },
    AuthorizePiCreateSession {
        child_process_id: ChildProcessId,
        correlation_identity: PiCorrelationIdentity,
        create_request_digest: Sha256Digest,
    },
    RecordPiCreateSessionDelivery {
        child_process_id: ChildProcessId,
        correlation_identity: PiCorrelationIdentity,
        create_request_digest: Sha256Digest,
    },
    RecordPiSessionReady {
        child_process_id: ChildProcessId,
        pi_session_identity: PiBoundarySessionIdentity,
    },
    RecordPiAbortControlDelivery {
        child_process_id: ChildProcessId,
        cancellation_propagation_id: CancellationPropagationId,
        correlation_identity: PiCorrelationIdentity,
        abort_command_digest: Sha256Digest,
        outcome: PiAbortControlWriteOutcome,
    },
    RecordChildStreamSeal {
        child_process_id: ChildProcessId,
        stream_kind: ChildStreamKind,
        full_observed_digest: Sha256Digest,
        retained_content_object_id: ContentObjectId,
        completeness: ChildStreamSealCompleteness,
    },
    RecordChildProcessLiveness {
        child_process_id: ChildProcessId,
        liveness: ProcessGroupLiveness,
    },
    RecordProcessSignalReceipt {
        child_process_id: ChildProcessId,
        action: ProcessSignalAction,
        delivery: ProcessSignalDelivery,
        observed_liveness: ProcessGroupLiveness,
        cause: ProcessSignalCause,
    },
    RecordDirectChildReap {
        child_process_id: ChildProcessId,
        wait_status: DirectChildWaitStatus,
        group_liveness_before_cleanup: ProcessGroupLiveness,
        group_liveness_after_cleanup: ProcessGroupLiveness,
    },
    RecordChildRecovery {
        child_process_id: ChildProcessId,
        observation: ChildRecoveryObservation,
        group_liveness_after_restart: ProcessGroupLiveness,
    },
    /// A terminal classification is derived only from preceding receipt rows;
    /// this command does not accept a caller-selected result.
    FinalizeChildProcess {
        child_process_id: ChildProcessId,
    },
    BeginCancellationPropagation {
        cancellation_request_id: CancellationRequestId,
    },
    ReconcileCancellationPropagation {
        cancellation_propagation_id: CancellationPropagationId,
    },
    RecordPiChildNotSpawned {
        pi_child_spawn_admission_id: PiChildSpawnAdmissionId,
        reason: PiChildNotSpawnedReason,
    },
}

impl CommandBody {
    pub const fn kind(&self) -> CommandKind {
        match self {
            Self::CreateSocietyIdentity { .. } => CommandKind::CreateSocietyIdentity,
            Self::InstallGrandArchitectOffice => CommandKind::InstallGrandArchitectOffice,
            Self::InstallFoundingUniverseSeed { .. } => CommandKind::InstallFoundingUniverseSeed,
            Self::AppointInitialGrandArchitect { .. } => CommandKind::AppointInitialGrandArchitect,
            Self::SetR0HardCeiling { .. } => CommandKind::SetR0HardCeiling,
            Self::BootstrapSociety => CommandKind::BootstrapSociety,
            Self::ProposeOperatingCycle { .. } => CommandKind::ProposeOperatingCycle,
            Self::AdmitOperatingCycle { .. } => CommandKind::AdmitOperatingCycle,
            Self::StartGrandArchitectOfficeSession { .. } => {
                CommandKind::StartGrandArchitectOfficeSession
            }
            Self::RecordOfficeSessionReady { .. } => CommandKind::RecordOfficeSessionReady,
            Self::RecordOfficeSessionTerminal { .. } => CommandKind::RecordOfficeSessionTerminal,
            Self::OpenOfficeTurn { .. } => CommandKind::OpenOfficeTurn,
            Self::SettleOfficeTurn { .. } => CommandKind::SettleOfficeTurn,
            Self::QuiesceOperatingCycle { .. } => CommandKind::QuiesceOperatingCycle,
            Self::RecordCycleDrained { .. } => CommandKind::RecordCycleDrained,
            Self::ResumeOperatingCycle { .. } => CommandKind::ResumeOperatingCycle,
            Self::ReconcileOperatingCycle { .. } => CommandKind::ReconcileOperatingCycle,
            Self::CloseOperatingCycle { .. } => CommandKind::CloseOperatingCycle,
            Self::ReserveBudget { .. } => CommandKind::ReserveBudget,
            Self::ReconcileBudget { .. } => CommandKind::ReconcileBudget,
            Self::RequestCancellation { .. } => CommandKind::RequestCancellation,
            Self::ReconcileCancellation { .. } => CommandKind::ReconcileCancellation,
            Self::CloseCostPostmortem { .. } => CommandKind::CloseCostPostmortem,
            Self::CreateProject { .. } => CommandKind::CreateProject,
            Self::CharterProject { .. } => CommandKind::CharterProject,
            Self::TransitionProject { .. } => CommandKind::TransitionProject,
            Self::CompleteProjectMilestone { .. } => CommandKind::CompleteProjectMilestone,
            Self::ReopenProject { .. } => CommandKind::ReopenProject,
            Self::CreateTicket { .. } => CommandKind::CreateTicket,
            Self::TransitionTicket { .. } => CommandKind::TransitionTicket,
            Self::AddGraphObjectRevision { .. } => CommandKind::AddGraphObjectRevision,
            Self::CommitGraphRevision { .. } => CommandKind::CommitGraphRevision,
            Self::AddGraphEdge { .. } => CommandKind::AddGraphEdge,
            Self::CreateEpisode { .. } => CommandKind::CreateEpisode,
            Self::TransitionEpisode { .. } => CommandKind::TransitionEpisode,
            Self::ReopenEpisode { .. } => CommandKind::ReopenEpisode,
            Self::RequestAdversarialReview { .. } => CommandKind::RequestAdversarialReview,
            Self::AssignAdversarialReviewer { .. } => CommandKind::AssignAdversarialReviewer,
            Self::SubmitReviewChallenge { .. } => CommandKind::SubmitReviewChallenge,
            Self::RespondToReviewChallenge { .. } => CommandKind::RespondToReviewChallenge,
            Self::DispositionReviewChallenge { .. } => CommandKind::DispositionReviewChallenge,
            Self::ResolveAdversarialReview { .. } => CommandKind::ResolveAdversarialReview,
            Self::TriggerPostmortem { .. } => CommandKind::TriggerPostmortem,
            Self::RecordPostmortemCausalClaim { .. } => CommandKind::RecordPostmortemCausalClaim,
            Self::ProposePostmortemAction { .. } => CommandKind::ProposePostmortemAction,
            Self::ClosePostmortem { .. } => CommandKind::ClosePostmortem,
            Self::RegisterActorConfiguration { .. } => CommandKind::RegisterActorConfiguration,
            Self::RegisterContextPack { .. } => CommandKind::RegisterContextPack,
            Self::AdmitActorInstance { .. } => CommandKind::AdmitActorInstance,
            Self::AdmitTicket { .. } => CommandKind::AdmitTicket,
            Self::RegisterWorkItem { .. } => CommandKind::RegisterWorkItem,
            Self::ClaimWorkItem { .. } => CommandKind::ClaimWorkItem,
            Self::StartActorAttempt { .. } => CommandKind::StartActorAttempt,
            Self::AttestActorAttemptTerminal { .. } => CommandKind::AttestActorAttemptTerminal,
            Self::ValidateTicketAttempt { .. } => CommandKind::ValidateTicketAttempt,
            Self::RetryActorAttempt { .. } => CommandKind::RetryActorAttempt,
            Self::CompleteTicket { .. } => CommandKind::CompleteTicket,
            Self::ExpireWorkLease { .. } => CommandKind::ExpireWorkLease,
            Self::CancelActorAttempt { .. } => CommandKind::CancelActorAttempt,
            Self::RegisterOutcomeObligation { .. } => CommandKind::RegisterOutcomeObligation,
            Self::ResolveOutcomeObligation { .. } => CommandKind::ResolveOutcomeObligation,
            Self::RecordContentSealReceipt { .. } => CommandKind::RecordContentSealReceipt,
            Self::RegisterContentObject { .. } => CommandKind::RegisterContentObject,
            Self::RegisterForensicManifest { .. } => CommandKind::RegisterForensicManifest,
            Self::RegisterDeterministicExperiment { .. } => {
                CommandKind::RegisterDeterministicExperiment
            }
            Self::RecordDeterministicEvaluationReceipt { .. } => {
                CommandKind::RecordDeterministicEvaluationReceipt
            }
            Self::AdmitDeterministicEvidence { .. } => CommandKind::AdmitDeterministicEvidence,
            Self::CloseDeterministicExperiment { .. } => CommandKind::CloseDeterministicExperiment,
            Self::OpenSupervisorEpoch { .. } => CommandKind::OpenSupervisorEpoch,
            Self::AdmitPiChildSpawn { .. } => CommandKind::AdmitPiChildSpawn,
            Self::RecordInertChildSpawn { .. } => CommandKind::RecordInertChildSpawn,
            Self::RecordPiAdapterReady { .. } => CommandKind::RecordPiAdapterReady,
            Self::AuthorizePiCreateSession { .. } => CommandKind::AuthorizePiCreateSession,
            Self::RecordPiCreateSessionDelivery { .. } => {
                CommandKind::RecordPiCreateSessionDelivery
            }
            Self::RecordPiSessionReady { .. } => CommandKind::RecordPiSessionReady,
            Self::RecordPiAbortControlDelivery { .. } => CommandKind::RecordPiAbortControlDelivery,
            Self::RecordChildStreamSeal { .. } => CommandKind::RecordChildStreamSeal,
            Self::RecordChildProcessLiveness { .. } => CommandKind::RecordChildProcessLiveness,
            Self::RecordProcessSignalReceipt { .. } => CommandKind::RecordProcessSignalReceipt,
            Self::RecordDirectChildReap { .. } => CommandKind::RecordDirectChildReap,
            Self::RecordChildRecovery { .. } => CommandKind::RecordChildRecovery,
            Self::FinalizeChildProcess { .. } => CommandKind::FinalizeChildProcess,
            Self::BeginCancellationPropagation { .. } => CommandKind::BeginCancellationPropagation,
            Self::ReconcileCancellationPropagation { .. } => {
                CommandKind::ReconcileCancellationPropagation
            }
            Self::RecordPiChildNotSpawned { .. } => CommandKind::RecordPiChildNotSpawned,
        }
    }

    pub const fn required_capability(&self) -> Capability {
        match self {
            Self::CreateSocietyIdentity { .. } => Capability::CreateSocietyIdentity,
            Self::InstallGrandArchitectOffice => Capability::InstallGrandArchitectOffice,
            Self::InstallFoundingUniverseSeed { .. } => Capability::InstallFoundingUniverseSeed,
            Self::AppointInitialGrandArchitect { .. } => Capability::AppointInitialGrandArchitect,
            Self::SetR0HardCeiling { .. } => Capability::SetR0HardCeiling,
            Self::BootstrapSociety => Capability::BootstrapSociety,
            Self::ProposeOperatingCycle { .. } => Capability::ProposeOperatingCycle,
            Self::AdmitOperatingCycle { .. } => Capability::AdmitOperatingCycle,
            Self::StartGrandArchitectOfficeSession { .. } => {
                Capability::StartGrandArchitectOfficeSession
            }
            Self::RecordOfficeSessionReady { .. } => Capability::RecordOfficeSessionReady,
            Self::RecordOfficeSessionTerminal { .. } => Capability::RecordOfficeSessionTerminal,
            Self::OpenOfficeTurn { .. } => Capability::OpenOfficeTurn,
            Self::SettleOfficeTurn { .. } => Capability::SettleOfficeTurn,
            Self::QuiesceOperatingCycle { .. } => Capability::QuiesceOperatingCycle,
            Self::RecordCycleDrained { .. } => Capability::RecordCycleDrained,
            Self::ResumeOperatingCycle { .. } => Capability::ResumeOperatingCycle,
            Self::ReconcileOperatingCycle { .. } => Capability::ReconcileOperatingCycle,
            Self::CloseOperatingCycle { .. } => Capability::CloseOperatingCycle,
            Self::ReserveBudget { .. } => Capability::ReserveBudget,
            Self::ReconcileBudget { .. } => Capability::ReconcileBudget,
            Self::RequestCancellation { .. } => Capability::RequestCancellation,
            Self::ReconcileCancellation { .. } => Capability::ReconcileCancellation,
            Self::CloseCostPostmortem { .. } => Capability::CloseCostPostmortem,
            Self::CreateProject { .. } => Capability::CreateProject,
            Self::CharterProject { .. } => Capability::CharterProject,
            Self::TransitionProject { .. } => Capability::TransitionProject,
            Self::CompleteProjectMilestone { .. } => Capability::CompleteProjectMilestone,
            Self::ReopenProject { .. } => Capability::ReopenProject,
            Self::CreateTicket { .. } => Capability::CreateTicket,
            Self::TransitionTicket { .. } => Capability::TransitionTicket,
            Self::AddGraphObjectRevision { .. } => Capability::AddGraphObjectRevision,
            Self::CommitGraphRevision { .. } => Capability::CommitGraphRevision,
            Self::AddGraphEdge { .. } => Capability::AddGraphEdge,
            Self::CreateEpisode { .. } => Capability::CreateEpisode,
            Self::TransitionEpisode { .. } => Capability::TransitionEpisode,
            Self::ReopenEpisode { .. } => Capability::ReopenEpisode,
            Self::RequestAdversarialReview { .. } => Capability::RequestAdversarialReview,
            Self::AssignAdversarialReviewer { .. } => Capability::AssignAdversarialReviewer,
            Self::SubmitReviewChallenge { .. } => Capability::SubmitReviewChallenge,
            Self::RespondToReviewChallenge { .. } => Capability::RespondToReviewChallenge,
            Self::DispositionReviewChallenge { .. } => Capability::DispositionReviewChallenge,
            Self::ResolveAdversarialReview { .. } => Capability::ResolveAdversarialReview,
            Self::TriggerPostmortem { .. } => Capability::TriggerPostmortem,
            Self::RecordPostmortemCausalClaim { .. } => Capability::RecordPostmortemCausalClaim,
            Self::ProposePostmortemAction { .. } => Capability::ProposePostmortemAction,
            Self::ClosePostmortem { .. } => Capability::ClosePostmortem,
            Self::RegisterActorConfiguration { .. } => Capability::RegisterActorConfiguration,
            Self::RegisterContextPack { .. } => Capability::RegisterContextPack,
            Self::AdmitActorInstance { .. } => Capability::AdmitActorInstance,
            Self::AdmitTicket { .. } => Capability::AdmitTicket,
            Self::RegisterWorkItem { .. } => Capability::RegisterWorkItem,
            Self::ClaimWorkItem { .. } => Capability::ClaimWorkItem,
            Self::StartActorAttempt { .. } => Capability::StartActorAttempt,
            Self::AttestActorAttemptTerminal { .. } => Capability::AttestActorAttemptTerminal,
            Self::ValidateTicketAttempt { .. } => Capability::ValidateTicketAttempt,
            Self::RetryActorAttempt { .. } => Capability::RetryActorAttempt,
            Self::CompleteTicket { .. } => Capability::CompleteTicket,
            Self::ExpireWorkLease { .. } => Capability::ExpireWorkLease,
            Self::CancelActorAttempt { .. } => Capability::CancelActorAttempt,
            Self::RegisterOutcomeObligation { .. } => Capability::RegisterOutcomeObligation,
            Self::ResolveOutcomeObligation { .. } => Capability::ResolveOutcomeObligation,
            Self::RecordContentSealReceipt { .. } => Capability::RecordContentSealReceipt,
            Self::RegisterContentObject { .. } => Capability::RegisterContentObject,
            Self::RegisterForensicManifest { .. } => Capability::RegisterForensicManifest,
            Self::RegisterDeterministicExperiment { .. } => {
                Capability::RegisterDeterministicExperiment
            }
            Self::RecordDeterministicEvaluationReceipt { .. } => {
                Capability::RecordDeterministicEvaluationReceipt
            }
            Self::AdmitDeterministicEvidence { .. } => Capability::AdmitDeterministicEvidence,
            Self::CloseDeterministicExperiment { .. } => Capability::CloseDeterministicExperiment,
            Self::OpenSupervisorEpoch { .. } => Capability::OpenSupervisorEpoch,
            Self::AdmitPiChildSpawn { .. } => Capability::AdmitPiChildSpawn,
            Self::RecordInertChildSpawn { .. } => Capability::RecordInertChildSpawn,
            Self::RecordPiAdapterReady { .. } => Capability::RecordPiAdapterReady,
            Self::AuthorizePiCreateSession { .. } => Capability::AuthorizePiCreateSession,
            Self::RecordPiCreateSessionDelivery { .. } => Capability::RecordPiCreateSessionDelivery,
            Self::RecordPiSessionReady { .. } => Capability::RecordPiSessionReady,
            Self::RecordPiAbortControlDelivery { .. } => Capability::RecordPiAbortControlDelivery,
            Self::RecordChildStreamSeal { .. } => Capability::RecordChildStreamSeal,
            Self::RecordChildProcessLiveness { .. } => Capability::RecordChildProcessLiveness,
            Self::RecordProcessSignalReceipt { .. } => Capability::RecordProcessSignalReceipt,
            Self::RecordDirectChildReap { .. } => Capability::RecordDirectChildReap,
            Self::RecordChildRecovery { .. } => Capability::RecordChildRecovery,
            Self::FinalizeChildProcess { .. } => Capability::FinalizeChildProcess,
            Self::BeginCancellationPropagation { .. } => Capability::BeginCancellationPropagation,
            Self::ReconcileCancellationPropagation { .. } => {
                Capability::ReconcileCancellationPropagation
            }
            Self::RecordPiChildNotSpawned { .. } => Capability::RecordPiChildNotSpawned,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CommandKind {
    CreateSocietyIdentity = 1,
    InstallGrandArchitectOffice = 2,
    InstallFoundingUniverseSeed = 3,
    AppointInitialGrandArchitect = 4,
    SetR0HardCeiling = 5,
    BootstrapSociety = 6,
    ProposeOperatingCycle = 7,
    AdmitOperatingCycle = 8,
    StartGrandArchitectOfficeSession = 9,
    RecordOfficeSessionReady = 10,
    OpenOfficeTurn = 11,
    SettleOfficeTurn = 12,
    QuiesceOperatingCycle = 13,
    RecordCycleDrained = 14,
    ResumeOperatingCycle = 15,
    ReconcileOperatingCycle = 16,
    CloseOperatingCycle = 17,
    ReserveBudget = 18,
    ReconcileBudget = 19,
    RequestCancellation = 20,
    ReconcileCancellation = 21,
    RecordOfficeSessionTerminal = 22,
    CloseCostPostmortem = 23,
    CreateProject = 24,
    CharterProject = 25,
    TransitionProject = 26,
    CompleteProjectMilestone = 27,
    ReopenProject = 28,
    CreateTicket = 29,
    TransitionTicket = 30,
    AddGraphObjectRevision = 31,
    CommitGraphRevision = 32,
    AddGraphEdge = 33,
    CreateEpisode = 34,
    TransitionEpisode = 35,
    ReopenEpisode = 36,
    RequestAdversarialReview = 37,
    SubmitReviewChallenge = 38,
    RespondToReviewChallenge = 39,
    DispositionReviewChallenge = 40,
    ResolveAdversarialReview = 41,
    TriggerPostmortem = 42,
    RecordPostmortemCausalClaim = 43,
    ProposePostmortemAction = 44,
    ClosePostmortem = 45,
    AssignAdversarialReviewer = 46,
    RegisterActorConfiguration = 47,
    RegisterContextPack = 48,
    AdmitActorInstance = 49,
    AdmitTicket = 50,
    RegisterWorkItem = 51,
    ClaimWorkItem = 52,
    StartActorAttempt = 53,
    AttestActorAttemptTerminal = 54,
    ValidateTicketAttempt = 55,
    RetryActorAttempt = 56,
    CompleteTicket = 57,
    ExpireWorkLease = 58,
    CancelActorAttempt = 59,
    RegisterOutcomeObligation = 60,
    ResolveOutcomeObligation = 61,
    RecordContentSealReceipt = 62,
    RegisterContentObject = 63,
    RegisterForensicManifest = 64,
    RegisterDeterministicExperiment = 65,
    RecordDeterministicEvaluationReceipt = 66,
    AdmitDeterministicEvidence = 67,
    CloseDeterministicExperiment = 68,
    AdmitPiChildSpawn = 69,
    RecordInertChildSpawn = 70,
    RecordPiAdapterReady = 71,
    AuthorizePiCreateSession = 72,
    RecordPiCreateSessionDelivery = 73,
    RecordPiSessionReady = 74,
    RecordChildStreamSeal = 75,
    RecordChildProcessLiveness = 76,
    RecordProcessSignalReceipt = 77,
    RecordDirectChildReap = 78,
    RecordChildRecovery = 79,
    FinalizeChildProcess = 80,
    BeginCancellationPropagation = 81,
    ReconcileCancellationPropagation = 82,
    OpenSupervisorEpoch = 83,
    RecordPiAbortControlDelivery = 84,
    RecordPiChildNotSpawned = 85,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandRequest {
    pub command_id: CommandId,
    pub principal_id: PrincipalId,
    /// The exact durable grant authorizing this request. A capability kind by
    /// itself is never an authority token because office-scoped grants may
    /// differ in jurisdiction, expiry, or occupancy.
    pub capability_grant_id: CapabilityGrantId,
    pub capability: Capability,
    pub expected_generation: ExpectedGeneration,
    pub body: CommandBody,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandDisposition {
    Accepted(EventId),
    Rejected(Rejection),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandReceipt {
    pub disposition: CommandDisposition,
    pub idempotent: bool,
}

/// The single authority for durable rejection wire values.  The daemon must
/// eventually delegate its protocol conversion here rather than mirror a
/// handwritten numeric match.  SQLite stores the `i64` value; the local
/// control protocol uses `u8`, and neither conversion accepts gaps.
macro_rules! closed_rejection_codes {
    ($($name:ident = $value:literal),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[repr(i64)]
        pub enum Rejection { $($name = $value,)+ }

        impl Rejection {
            pub const ALL: &'static [Self] = &[$(Self::$name,)+];

            pub const fn as_i64(self) -> i64 { self as i64 }

            pub const fn as_u8(self) -> u8 { self as u8 }
        }

        impl TryFrom<i64> for Rejection {
            type Error = DomainValueError;

            fn try_from(value: i64) -> Result<Self, Self::Error> {
                match value { $($value => Ok(Self::$name),)+ _ => Err(DomainValueError::InvalidRejectionCode(value)), }
            }
        }

        impl TryFrom<u8> for Rejection {
            type Error = DomainValueError;

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                Self::try_from(i64::from(value))
            }
        }
    };
}

closed_rejection_codes! {
    CapabilityMismatch = 1,
    CapabilityNotGranted = 2,
    CapabilityNoLongerActive = 3,
    InvalidExpectedGeneration = 4,
    StaleAdmissionGeneration = 5,
    InvalidLifecycleTransition = 6,
    FoundingInvariant = 7,
    ActiveCycleAlreadyExists = 8,
    ActiveOfficeOccupancyAlreadyExists = 9,
    BudgetCeilingExceeded = 10,
    ReservationNotActive = 11,
    CostExceedsReservation = 12,
    IncompleteCycleReconciliation = 13,
    SessionTurnAlreadyActive = 14,
    CancellationAlreadyTerminal = 15,
    SubjectNotFound = 16,
    BudgetPolicyViolation = 17,
    CostPostmortemNotOpen = 18,
    InvalidCostPostmortemResolution = 19,
    ProjectCloseBlocked = 20,
    TicketPrerequisiteIncomplete = 21,
    GraphRevisionNotCommitted = 22,
    IllegalGraphEdgeEndpoint = 23,
    ReviewSelfDispositionDenied = 24,
    ReviewDispositionIncomplete = 25,
    PostmortemCloseBlocked = 26,
    ReviewAssignmentNotIndependent = 27,
    ActorJurisdictionDenied = 28,
    WorkLeaseUnavailable = 29,
    ActorAttemptNotTerminal = 30,
    ActorAttemptNotValidatable = 31,
    OutcomeObligationOpen = 32,
    ReviewAssignmentEvidenceMissing = 33,
    ExecutionProfileIneligible = 34,
    TicketAcceptanceConditionUnsatisfied = 35,
    QualificationTreatmentRestricted = 36,
    ContentSealReceiptMissing = 37,
    ContentObjectNotSealed = 38,
    ForensicManifestBindingMismatch = 39,
    DeterministicExperimentBindingMismatch = 40,
    DeterministicEvaluationBindingMismatch = 41,
    EvidenceAdmissionRequired = 42,
    ChildSpawnAdmissionInvalid = 43,
    ChildLifecycleReceiptMissing = 44,
    ChildStreamSealBindingMismatch = 45,
    ProcessContainmentFailed = 46,
    CancellationPropagationIncomplete = 47,
    SupervisedTerminalReceiptRequired = 48,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventBody {
    SocietyIdentityCreated {
        society_id: SocietyId,
    },
    GrandArchitectOfficeInstalled {
        office_id: OfficeId,
    },
    FoundingUniverseSeedInstalled {
        seed_id: UniverseSeedId,
    },
    GrandArchitectAppointed {
        occupancy_id: OfficeOccupancyId,
        principal_id: PrincipalId,
    },
    R0HardCeilingSet {
        society_id: SocietyId,
        ceiling: UsdMicros,
    },
    SocietyBootstrapped {
        society_id: SocietyId,
    },
    OperatingCycleProposed {
        cycle_id: OperatingCycleId,
        generation: AdmissionGeneration,
        treatment: OperatingCycleTreatment,
    },
    OperatingCycleStateChanged {
        cycle_id: OperatingCycleId,
        state: OperatingCycleState,
        generation: AdmissionGeneration,
    },
    GrandArchitectOfficeSessionStarted {
        session_id: GrandArchitectOfficeSessionId,
        cycle_id: OperatingCycleId,
    },
    GrandArchitectOfficeSessionStateChanged {
        session_id: GrandArchitectOfficeSessionId,
        state: OfficeSessionState,
    },
    OfficeTurnOpened {
        turn_id: OfficeTurnId,
        session_id: GrandArchitectOfficeSessionId,
        purpose: OfficeTurnPurpose,
    },
    OfficeTurnSettled {
        turn_id: OfficeTurnId,
        session_id: GrandArchitectOfficeSessionId,
    },
    BudgetReserved {
        reservation_id: BudgetReservationId,
        cycle_id: OperatingCycleId,
        amount: UsdMicros,
    },
    BudgetReconciled {
        reservation_id: BudgetReservationId,
        observed: UsdMicros,
    },
    BudgetAdmissionFrozen {
        reservation_id: BudgetReservationId,
        cycle_id: OperatingCycleId,
        cancellation_request_id: CancellationRequestId,
        postmortem_id: CostPostmortemId,
        reason: BudgetFreezeReason,
    },
    CancellationRequested {
        cancellation_request_id: CancellationRequestId,
        cycle_id: OperatingCycleId,
        mode: CancellationMode,
        generation: AdmissionGeneration,
    },
    CancellationReconciled {
        cancellation_request_id: CancellationRequestId,
        cycle_id: OperatingCycleId,
    },
    CostPostmortemClosed {
        postmortem_id: CostPostmortemId,
        reservation_id: BudgetReservationId,
        cycle_id: OperatingCycleId,
        resolution: CostPostmortemResolution,
        charged: UsdMicros,
    },
    ProjectCreated {
        project_id: ProjectId,
    },
    ProjectChartered {
        project_id: ProjectId,
    },
    ProjectStateChanged {
        project_id: ProjectId,
        state: ProjectState,
    },
    ProjectMilestoneCompleted {
        project_milestone_id: ProjectMilestoneId,
    },
    TicketCreated {
        ticket_id: TicketId,
        project_id: ProjectId,
    },
    TicketStateChanged {
        ticket_id: TicketId,
        state: TicketState,
    },
    GraphObjectRevisionAdded {
        graph_object_id: GraphObjectId,
        graph_revision_id: GraphRevisionId,
    },
    GraphRevisionCommitted {
        graph_revision_id: GraphRevisionId,
    },
    GraphEdgeAdded {
        graph_edge_id: GraphEdgeId,
    },
    EpisodeCreated {
        causal_episode_id: CausalEpisodeId,
        project_id: ProjectId,
    },
    EpisodeStateChanged {
        causal_episode_id: CausalEpisodeId,
        state: EpisodeState,
    },
    AdversarialReviewRequested {
        adversarial_review_id: AdversarialReviewId,
    },
    AdversarialReviewerAssigned {
        adversarial_review_id: AdversarialReviewId,
        reviewer_principal_id: PrincipalId,
        reviewer_actor_instance_id: ActorInstanceId,
        reviewer_actor_attempt_id: ActorAttemptId,
    },
    ReviewChallengeSubmitted {
        review_challenge_id: ReviewChallengeId,
        author_principal_id: PrincipalId,
    },
    ReviewChallengeResponded {
        review_challenge_id: ReviewChallengeId,
    },
    ReviewChallengeDispositioned {
        review_challenge_id: ReviewChallengeId,
        disposition: ReviewDispositionKind,
    },
    AdversarialReviewResolved {
        adversarial_review_id: AdversarialReviewId,
        state: AdversarialReviewState,
    },
    PostmortemTriggered {
        postmortem_id: PostmortemId,
    },
    PostmortemCausalClaimRecorded {
        postmortem_causal_claim_id: PostmortemCausalClaimId,
    },
    PostmortemActionProposed {
        postmortem_action_proposal_id: PostmortemActionProposalId,
    },
    PostmortemClosed {
        postmortem_id: PostmortemId,
    },
    ActorConfigurationRegistered {
        actor_configuration_id: ActorConfigurationId,
        actor_configuration_revision_id: ActorConfigurationRevisionId,
    },
    ContextPackRegistered {
        context_pack_id: ContextPackId,
    },
    ActorInstanceAdmitted {
        actor_instance_id: ActorInstanceId,
        principal_id: PrincipalId,
    },
    TicketAdmitted {
        ticket_id: TicketId,
    },
    WorkItemRegistered {
        work_item_id: WorkItemId,
        ticket_id: TicketId,
        adversarial_review_id: Option<AdversarialReviewId>,
    },
    WorkItemClaimed {
        work_item_id: WorkItemId,
        work_lease_id: WorkLeaseId,
        actor_instance_id: ActorInstanceId,
    },
    ActorAttemptStarted {
        actor_attempt_id: ActorAttemptId,
        work_item_id: WorkItemId,
        budget_reservation_id: BudgetReservationId,
    },
    ActorAttemptTerminalAttested {
        actor_attempt_id: ActorAttemptId,
        terminal_kind: ActorAttemptTerminalKind,
    },
    TicketAttemptValidated {
        actor_attempt_id: ActorAttemptId,
        ticket_id: TicketId,
    },
    ActorAttemptRetryPrepared {
        actor_attempt_id: ActorAttemptId,
        work_item_id: WorkItemId,
        ticket_id: TicketId,
    },
    TicketCompleted {
        ticket_id: TicketId,
        actor_attempt_id: ActorAttemptId,
    },
    WorkLeaseExpired {
        work_lease_id: WorkLeaseId,
        work_item_id: WorkItemId,
    },
    ActorAttemptCancellationRequested {
        actor_attempt_id: ActorAttemptId,
        reason: ActorAttemptCancellationReason,
    },
    OutcomeObligationRegistered {
        outcome_obligation_id: OutcomeObligationId,
        project_id: ProjectId,
    },
    OutcomeObligationResolved {
        outcome_obligation_id: OutcomeObligationId,
        state: OutcomeObligationState,
    },
    ContentSealReceiptRecorded {
        content_seal_receipt_id: ContentSealReceiptId,
        digest: Sha256Digest,
    },
    ContentObjectRegistered {
        content_object_id: ContentObjectId,
        content_seal_receipt_id: ContentSealReceiptId,
    },
    ForensicManifestRegistered {
        forensic_manifest_id: ForensicManifestId,
        producing_deterministic_experiment_id: DeterministicExperimentId,
        evaluator_output_content_object_id: ContentObjectId,
    },
    DeterministicExperimentRegistered {
        deterministic_experiment_id: DeterministicExperimentId,
        evaluator_revision_id: EvaluatorRevisionId,
        input_manifest_id: InputManifestId,
    },
    DeterministicEvaluationReceiptRecorded {
        deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId,
        deterministic_experiment_id: DeterministicExperimentId,
    },
    DeterministicEvidenceAdmitted {
        evidence_admission_id: EvidenceAdmissionId,
        deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId,
        semantic_role: EvidenceSemanticRole,
        applicability: EvidenceApplicability,
    },
    DeterministicExperimentClosed {
        deterministic_experiment_id: DeterministicExperimentId,
    },
    PiChildSpawnAdmitted {
        pi_child_spawn_admission_id: PiChildSpawnAdmissionId,
        owner: PiChildOwner,
        budget_reservation_id: BudgetReservationId,
    },
    InertPiChildSpawnRecorded {
        child_process_id: ChildProcessId,
        pi_child_spawn_admission_id: PiChildSpawnAdmissionId,
    },
    PiAdapterReadyRecorded {
        child_process_id: ChildProcessId,
        pi_session_id: PiSessionId,
    },
    PiCreateSessionAuthorized {
        child_process_id: ChildProcessId,
    },
    PiCreateSessionDeliveryRecorded {
        child_process_id: ChildProcessId,
    },
    PiSessionReadyRecorded {
        child_process_id: ChildProcessId,
        pi_session_id: PiSessionId,
    },
    PiAbortControlDeliveryRecorded {
        pi_abort_control_receipt_id: PiAbortControlReceiptId,
        child_process_id: ChildProcessId,
        cancellation_propagation_id: CancellationPropagationId,
        correlation_identity: PiCorrelationIdentity,
        abort_command_digest: Sha256Digest,
        outcome: PiAbortControlWriteOutcome,
    },
    ChildStreamSealed {
        child_stream_seal_id: ChildStreamSealId,
        child_process_id: ChildProcessId,
        stream_kind: ChildStreamKind,
        completeness: ChildStreamSealCompleteness,
    },
    ChildProcessLivenessObserved {
        child_process_liveness_observation_id: ChildProcessLivenessObservationId,
        child_process_id: ChildProcessId,
        liveness: ProcessGroupLiveness,
    },
    ProcessSignalReceiptRecorded {
        process_signal_receipt_id: ProcessSignalReceiptId,
        child_process_id: ChildProcessId,
        action: ProcessSignalAction,
        delivery: ProcessSignalDelivery,
        observed_liveness: ProcessGroupLiveness,
        cause: ProcessSignalCause,
    },
    DirectChildReaped {
        child_process_reap_receipt_id: ChildProcessReapReceiptId,
        child_process_id: ChildProcessId,
        wait_status: DirectChildWaitStatus,
        group_liveness_before_cleanup: ProcessGroupLiveness,
        group_liveness_after_cleanup: ProcessGroupLiveness,
    },
    ChildRecoveryObserved {
        child_process_recovery_receipt_id: ChildProcessRecoveryReceiptId,
        child_process_id: ChildProcessId,
        observation: ChildRecoveryObservation,
        group_liveness_after_restart: ProcessGroupLiveness,
    },
    ChildProcessFinalized {
        child_process_id: ChildProcessId,
        disposition: ChildTerminalDisposition,
    },
    CancellationPropagationBegun {
        cancellation_propagation_id: CancellationPropagationId,
        cancellation_request_id: CancellationRequestId,
    },
    CancellationPropagationReconciled {
        cancellation_propagation_id: CancellationPropagationId,
    },
    CancellationPropagationContainmentFailed {
        cancellation_propagation_id: CancellationPropagationId,
    },
    PiChildSpawnInvalidated {
        pi_child_spawn_admission_id: PiChildSpawnAdmissionId,
        reason: PiChildNotSpawnedReason,
    },
    SupervisorEpochOpened {
        supervisor_epoch_id: SupervisorEpochId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum EventKind {
    SocietyIdentityCreated = 1,
    GrandArchitectOfficeInstalled = 2,
    FoundingUniverseSeedInstalled = 3,
    GrandArchitectAppointed = 4,
    R0HardCeilingSet = 5,
    SocietyBootstrapped = 6,
    OperatingCycleProposed = 7,
    OperatingCycleStateChanged = 8,
    GrandArchitectOfficeSessionStarted = 9,
    GrandArchitectOfficeSessionStateChanged = 10,
    OfficeTurnOpened = 11,
    OfficeTurnSettled = 12,
    BudgetReserved = 13,
    BudgetReconciled = 14,
    BudgetAdmissionFrozen = 15,
    CancellationRequested = 16,
    CancellationReconciled = 17,
    CostPostmortemClosed = 18,
    ProjectCreated = 19,
    ProjectChartered = 20,
    ProjectStateChanged = 21,
    ProjectMilestoneCompleted = 22,
    TicketCreated = 23,
    TicketStateChanged = 24,
    GraphObjectRevisionAdded = 25,
    GraphRevisionCommitted = 26,
    GraphEdgeAdded = 27,
    EpisodeCreated = 28,
    EpisodeStateChanged = 29,
    AdversarialReviewRequested = 30,
    ReviewChallengeSubmitted = 31,
    ReviewChallengeResponded = 32,
    ReviewChallengeDispositioned = 33,
    AdversarialReviewResolved = 34,
    PostmortemTriggered = 35,
    PostmortemCausalClaimRecorded = 36,
    PostmortemActionProposed = 37,
    PostmortemClosed = 38,
    AdversarialReviewerAssigned = 39,
    ActorConfigurationRegistered = 40,
    ContextPackRegistered = 41,
    ActorInstanceAdmitted = 42,
    TicketAdmitted = 43,
    WorkItemRegistered = 44,
    WorkItemClaimed = 45,
    ActorAttemptStarted = 46,
    ActorAttemptTerminalAttested = 47,
    TicketAttemptValidated = 48,
    ActorAttemptRetryPrepared = 49,
    TicketCompleted = 50,
    WorkLeaseExpired = 51,
    ActorAttemptCancellationRequested = 52,
    OutcomeObligationRegistered = 53,
    OutcomeObligationResolved = 54,
    ContentSealReceiptRecorded = 55,
    ContentObjectRegistered = 56,
    ForensicManifestRegistered = 57,
    DeterministicExperimentRegistered = 58,
    DeterministicEvaluationReceiptRecorded = 59,
    DeterministicEvidenceAdmitted = 60,
    DeterministicExperimentClosed = 61,
    PiChildSpawnAdmitted = 62,
    InertPiChildSpawnRecorded = 63,
    PiAdapterReadyRecorded = 64,
    PiCreateSessionAuthorized = 65,
    PiCreateSessionDeliveryRecorded = 66,
    PiSessionReadyRecorded = 67,
    ChildStreamSealed = 68,
    ChildProcessLivenessObserved = 69,
    ProcessSignalReceiptRecorded = 70,
    DirectChildReaped = 71,
    ChildRecoveryObserved = 72,
    ChildProcessFinalized = 73,
    CancellationPropagationBegun = 74,
    CancellationPropagationReconciled = 75,
    SupervisorEpochOpened = 76,
    CancellationPropagationContainmentFailed = 77,
    PiAbortControlDeliveryRecorded = 78,
    PiChildSpawnInvalidated = 79,
}

impl EventBody {
    pub const fn kind(&self) -> EventKind {
        match self {
            Self::SocietyIdentityCreated { .. } => EventKind::SocietyIdentityCreated,
            Self::GrandArchitectOfficeInstalled { .. } => EventKind::GrandArchitectOfficeInstalled,
            Self::FoundingUniverseSeedInstalled { .. } => EventKind::FoundingUniverseSeedInstalled,
            Self::GrandArchitectAppointed { .. } => EventKind::GrandArchitectAppointed,
            Self::R0HardCeilingSet { .. } => EventKind::R0HardCeilingSet,
            Self::SocietyBootstrapped { .. } => EventKind::SocietyBootstrapped,
            Self::OperatingCycleProposed { .. } => EventKind::OperatingCycleProposed,
            Self::OperatingCycleStateChanged { .. } => EventKind::OperatingCycleStateChanged,
            Self::GrandArchitectOfficeSessionStarted { .. } => {
                EventKind::GrandArchitectOfficeSessionStarted
            }
            Self::GrandArchitectOfficeSessionStateChanged { .. } => {
                EventKind::GrandArchitectOfficeSessionStateChanged
            }
            Self::OfficeTurnOpened { .. } => EventKind::OfficeTurnOpened,
            Self::OfficeTurnSettled { .. } => EventKind::OfficeTurnSettled,
            Self::BudgetReserved { .. } => EventKind::BudgetReserved,
            Self::BudgetReconciled { .. } => EventKind::BudgetReconciled,
            Self::BudgetAdmissionFrozen { .. } => EventKind::BudgetAdmissionFrozen,
            Self::CancellationRequested { .. } => EventKind::CancellationRequested,
            Self::CancellationReconciled { .. } => EventKind::CancellationReconciled,
            Self::CostPostmortemClosed { .. } => EventKind::CostPostmortemClosed,
            Self::ProjectCreated { .. } => EventKind::ProjectCreated,
            Self::ProjectChartered { .. } => EventKind::ProjectChartered,
            Self::ProjectStateChanged { .. } => EventKind::ProjectStateChanged,
            Self::ProjectMilestoneCompleted { .. } => EventKind::ProjectMilestoneCompleted,
            Self::TicketCreated { .. } => EventKind::TicketCreated,
            Self::TicketStateChanged { .. } => EventKind::TicketStateChanged,
            Self::GraphObjectRevisionAdded { .. } => EventKind::GraphObjectRevisionAdded,
            Self::GraphRevisionCommitted { .. } => EventKind::GraphRevisionCommitted,
            Self::GraphEdgeAdded { .. } => EventKind::GraphEdgeAdded,
            Self::EpisodeCreated { .. } => EventKind::EpisodeCreated,
            Self::EpisodeStateChanged { .. } => EventKind::EpisodeStateChanged,
            Self::AdversarialReviewRequested { .. } => EventKind::AdversarialReviewRequested,
            Self::AdversarialReviewerAssigned { .. } => EventKind::AdversarialReviewerAssigned,
            Self::ReviewChallengeSubmitted { .. } => EventKind::ReviewChallengeSubmitted,
            Self::ReviewChallengeResponded { .. } => EventKind::ReviewChallengeResponded,
            Self::ReviewChallengeDispositioned { .. } => EventKind::ReviewChallengeDispositioned,
            Self::AdversarialReviewResolved { .. } => EventKind::AdversarialReviewResolved,
            Self::PostmortemTriggered { .. } => EventKind::PostmortemTriggered,
            Self::PostmortemCausalClaimRecorded { .. } => EventKind::PostmortemCausalClaimRecorded,
            Self::PostmortemActionProposed { .. } => EventKind::PostmortemActionProposed,
            Self::PostmortemClosed { .. } => EventKind::PostmortemClosed,
            Self::ActorConfigurationRegistered { .. } => EventKind::ActorConfigurationRegistered,
            Self::ContextPackRegistered { .. } => EventKind::ContextPackRegistered,
            Self::ActorInstanceAdmitted { .. } => EventKind::ActorInstanceAdmitted,
            Self::TicketAdmitted { .. } => EventKind::TicketAdmitted,
            Self::WorkItemRegistered { .. } => EventKind::WorkItemRegistered,
            Self::WorkItemClaimed { .. } => EventKind::WorkItemClaimed,
            Self::ActorAttemptStarted { .. } => EventKind::ActorAttemptStarted,
            Self::ActorAttemptTerminalAttested { .. } => EventKind::ActorAttemptTerminalAttested,
            Self::TicketAttemptValidated { .. } => EventKind::TicketAttemptValidated,
            Self::ActorAttemptRetryPrepared { .. } => EventKind::ActorAttemptRetryPrepared,
            Self::TicketCompleted { .. } => EventKind::TicketCompleted,
            Self::WorkLeaseExpired { .. } => EventKind::WorkLeaseExpired,
            Self::ActorAttemptCancellationRequested { .. } => {
                EventKind::ActorAttemptCancellationRequested
            }
            Self::OutcomeObligationRegistered { .. } => EventKind::OutcomeObligationRegistered,
            Self::OutcomeObligationResolved { .. } => EventKind::OutcomeObligationResolved,
            Self::ContentSealReceiptRecorded { .. } => EventKind::ContentSealReceiptRecorded,
            Self::ContentObjectRegistered { .. } => EventKind::ContentObjectRegistered,
            Self::ForensicManifestRegistered { .. } => EventKind::ForensicManifestRegistered,
            Self::DeterministicExperimentRegistered { .. } => {
                EventKind::DeterministicExperimentRegistered
            }
            Self::DeterministicEvaluationReceiptRecorded { .. } => {
                EventKind::DeterministicEvaluationReceiptRecorded
            }
            Self::DeterministicEvidenceAdmitted { .. } => EventKind::DeterministicEvidenceAdmitted,
            Self::DeterministicExperimentClosed { .. } => EventKind::DeterministicExperimentClosed,
            Self::PiChildSpawnAdmitted { .. } => EventKind::PiChildSpawnAdmitted,
            Self::InertPiChildSpawnRecorded { .. } => EventKind::InertPiChildSpawnRecorded,
            Self::PiAdapterReadyRecorded { .. } => EventKind::PiAdapterReadyRecorded,
            Self::PiCreateSessionAuthorized { .. } => EventKind::PiCreateSessionAuthorized,
            Self::PiCreateSessionDeliveryRecorded { .. } => {
                EventKind::PiCreateSessionDeliveryRecorded
            }
            Self::PiSessionReadyRecorded { .. } => EventKind::PiSessionReadyRecorded,
            Self::PiAbortControlDeliveryRecorded { .. } => {
                EventKind::PiAbortControlDeliveryRecorded
            }
            Self::ChildStreamSealed { .. } => EventKind::ChildStreamSealed,
            Self::ChildProcessLivenessObserved { .. } => EventKind::ChildProcessLivenessObserved,
            Self::ProcessSignalReceiptRecorded { .. } => EventKind::ProcessSignalReceiptRecorded,
            Self::DirectChildReaped { .. } => EventKind::DirectChildReaped,
            Self::ChildRecoveryObserved { .. } => EventKind::ChildRecoveryObserved,
            Self::ChildProcessFinalized { .. } => EventKind::ChildProcessFinalized,
            Self::CancellationPropagationBegun { .. } => EventKind::CancellationPropagationBegun,
            Self::CancellationPropagationReconciled { .. } => {
                EventKind::CancellationPropagationReconciled
            }
            Self::SupervisorEpochOpened { .. } => EventKind::SupervisorEpochOpened,
            Self::CancellationPropagationContainmentFailed { .. } => {
                EventKind::CancellationPropagationContainmentFailed
            }
            Self::PiChildSpawnInvalidated { .. } => EventKind::PiChildSpawnInvalidated,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerEvent {
    pub event_id: EventId,
    pub command_id: CommandId,
    pub body: EventBody,
}

#[derive(Debug, Error)]
pub enum DomainValueError {
    #[error("{type_name} must be positive, got {value}")]
    NonPositiveIdentifier { type_name: &'static str, value: i64 },
    #[error("command id must be nonempty canonical printable ASCII with at most 128 bytes")]
    InvalidCommandId,
    #[error("society name must be nonblank, shorter than 161 bytes, and contain no NUL")]
    InvalidSocietyName,
    #[error("principal display name must be nonblank, shorter than 161 bytes, and contain no NUL")]
    InvalidPrincipalDisplayName,
    #[error("{type_name} must be nonblank, shorter than 1025 bytes, and contain no NUL")]
    InvalidCoordinationText { type_name: &'static str },
    #[error("{type_name} must use canonical boundary identity grammar")]
    InvalidOperationalIdentity { type_name: &'static str },
    #[error("micro-US-dollars cannot be negative: {0}")]
    NegativeUsdMicros(i64),
    #[error("admission generation cannot be negative: {0}")]
    NegativeAdmissionGeneration(i64),
    #[error("admission generation overflow")]
    GenerationOverflow,
    #[error("unknown durable rejection code: {0}")]
    InvalidRejectionCode(i64),
    #[error("{type_name} {value} {rule}")]
    InvalidNativeProcessValue {
        type_name: &'static str,
        value: i32,
        rule: &'static str,
    },
}
