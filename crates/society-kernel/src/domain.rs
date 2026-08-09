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

    pub const GRAND_ARCHITECT: [Self; 32] = [
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
    ];

    pub const KERNEL_SERVICE: [Self; 8] = [
        Self::RecordCycleDrained,
        Self::RecordOfficeSessionReady,
        Self::SettleOfficeTurn,
        Self::ReconcileBudget,
        Self::ReconcileCancellation,
        Self::RecordOfficeSessionTerminal,
        Self::SubmitReviewChallenge,
        Self::AssignAdversarialReviewer,
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
            Self::Vs001LiveV1 => UsdMicros::VS001_CYCLE_CEILING,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum Rejection {
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
    #[error("micro-US-dollars cannot be negative: {0}")]
    NegativeUsdMicros(i64),
    #[error("admission generation cannot be negative: {0}")]
    NegativeAdmissionGeneration(i64),
    #[error("admission generation overflow")]
    GenerationOverflow,
}
