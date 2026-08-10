use std::fmt;

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
identifier!(FoundingMissionId);
identifier!(ApplicationId);
identifier!(ApplicationRevisionId);
identifier!(OfficeId);
identifier!(OfficeOccupancyId);
identifier!(OperatingCycleId);
identifier!(PrincipalId);
identifier!(CapabilityGrantId);
identifier!(BudgetEnvelopeId);
identifier!(BudgetReservationId);
identifier!(CancellationRequestId);
identifier!(CostPostmortemId);
identifier!(RootAuthorityOfficeSessionId);
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
// Identity of one OS child under resident custody. Pi protocol identity is a
// sidecar and is absent for deterministic evaluator children.
identifier!(NativeChildId);
identifier!(NativeChildSpawnAdmissionId);
identifier!(NativeChildLivenessObservationId);
identifier!(NativeChildReapReceiptId);
identifier!(NativeChildRecoveryReceiptId);
identifier!(NativeChildStreamSealId);
identifier!(ProcessSignalReceiptId);
identifier!(PiAbortControlReceiptId);
identifier!(PiOfficeTurnPromptAuthorizationId);
identifier!(PiOfficeTurnUsageReceiptId);
identifier!(PiOfficeTurnUsageFailureId);
identifier!(PiOfficeTurnTerminalReceiptId);
identifier!(PiOfficeSessionDisposeReceiptId);
identifier!(PiProtocolSequence);
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

/// The supervisor-attested, syntactically canonical absolute transcript path
/// reported by the peer-sealed Pi Dispose boundary. It is nominally distinct
/// from a workspace custody path; physical no-follow/private-root custody
/// remains daemon responsibility while the kernel preserves this exact
/// durable receipt without admitting a caller-selected filesystem namespace.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CanonicalPiSessionTranscriptPath(String);

impl CanonicalPiSessionTranscriptPath {
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
                type_name: "CanonicalPiSessionTranscriptPath",
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
    /// Provider-free native evaluator fixture profile. It has no Pi protocol
    /// authority and is intentionally distinct from paid/live treatments.
    pub const DETERMINISTIC_EVALUATOR_PROCESS_FIXTURE_V1: Self = Self(3);
}

impl PrincipalId {
    /// The compiled, local founding authority. The current-schema bootstrap
    /// installs it; it is never selected through an environment variable or
    /// user-supplied string.
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

/// A stable, application-owned identity. It intentionally uses the same
/// portable grammar as other durable boundary identifiers, while remaining a
/// distinct domain type: a mission cannot be accidentally attached to an
/// Office, workspace, or Pi session identity.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ApplicationIdentity(String);

impl ApplicationIdentity {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainValueError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
            || !value
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            || !value
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric)
        {
            return Err(DomainValueError::InvalidApplicationIdentity);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ApplicationName(String);

impl ApplicationName {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainValueError> {
        let value = value.into();
        if value.trim().is_empty() || value.len() > 160 || value.contains('\0') {
            return Err(DomainValueError::InvalidApplicationName);
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

macro_rules! mission_text {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, PartialEq)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, DomainValueError> {
                let value = value.into();
                if value.trim().is_empty() || value.len() > 4_096 || value.contains('\0') {
                    return Err(DomainValueError::InvalidMissionText {
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

mission_text!(MissionStatement);
mission_text!(MissionPrincipleText);
mission_text!(NorthStarChangeQuestion);
mission_text!(NorthStarImprovementEvidenceQuestion);
mission_text!(NorthStarBoundaryCommitmentQuestion);
mission_text!(NorthStarRevisitQuestion);
mission_text!(ProjectNorthStarChangeAnswer);
mission_text!(ProjectNorthStarImprovementEvidenceAnswer);
mission_text!(ProjectNorthStarBoundaryCommitmentAnswer);
mission_text!(ProjectNorthStarRevisitAnswer);

/// Application revisions are immutable ordered statements. The ordinal is
/// supplied by the application boundary rather than inferred from insertion
/// order, so a later importer cannot silently rewrite its declared lineage.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ApplicationRevisionOrdinal(i64);

impl ApplicationRevisionOrdinal {
    pub const fn new(value: i64) -> Option<Self> {
        if value > 0 { Some(Self(value)) } else { None }
    }

    pub const fn value(self) -> i64 {
        self.0
    }
}

impl TryFrom<i64> for ApplicationRevisionOrdinal {
    type Error = DomainValueError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(DomainValueError::NonPositiveApplicationRevisionOrdinal(
            value,
        ))
    }
}

/// A generic closed category for a mission principle. The actual commitment is
/// in `MissionPrincipleText`; application-specific category strings never
/// become a hidden discriminator in the kernel schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum MissionPrincipleKind {
    Purpose = 1,
    Evidence = 2,
    Boundary = 3,
    Revision = 4,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionPrinciple {
    pub kind: MissionPrincipleKind,
    pub text: MissionPrincipleText,
}

/// The founding boundary admits a compact, ordered constitution rather than
/// an unbounded narrative list. PostgreSQL stores its position explicitly and the
/// request fingerprint preserves the caller's ordering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionPrinciples(Vec<MissionPrinciple>);

impl MissionPrinciples {
    pub const MAX_COUNT: usize = 16;

    pub fn new(principles: Vec<MissionPrinciple>) -> Result<Self, DomainValueError> {
        if principles.is_empty() || principles.len() > Self::MAX_COUNT {
            return Err(DomainValueError::InvalidMissionPrincipleCount {
                count: principles.len(),
            });
        }
        Ok(Self(principles))
    }

    pub fn as_slice(&self) -> &[MissionPrinciple] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NorthStarQuestionSet {
    pub change: NorthStarChangeQuestion,
    pub improvement_evidence: NorthStarImprovementEvidenceQuestion,
    pub boundary_commitment: NorthStarBoundaryCommitmentQuestion,
    pub revisit: NorthStarRevisitQuestion,
}

/// The complete first-class mission boundary for one application revision.
/// Its source rendering retains a BLAKE3 byte identity. `InstallFoundingMission`
/// separately binds that identity to one already admitted `ContentObject`; the
/// kernel uses the normalized fields below for durable reasoning and alignment
/// rather than treating the rendering as an opaque prompt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationMissionInput {
    pub application_identity: ApplicationIdentity,
    pub application_name: ApplicationName,
    pub revision_ordinal: ApplicationRevisionOrdinal,
    pub statement: MissionStatement,
    pub principles: MissionPrinciples,
    pub north_star_questions: NorthStarQuestionSet,
    pub source_rendering_digest: Blake3Digest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectNorthStarAlignment {
    pub application_revision_id: ApplicationRevisionId,
    pub change_answer: ProjectNorthStarChangeAnswer,
    pub improvement_evidence_answer: ProjectNorthStarImprovementEvidenceAnswer,
    pub boundary_commitment_answer: ProjectNorthStarBoundaryCommitmentAnswer,
    pub revisit_answer: ProjectNorthStarRevisitAnswer,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Blake3Digest([u8; 32]);

impl Blake3Digest {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for Blake3Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Bounded canonical bytes whose BLAKE3 digest is proposed as a founding
/// mission's source rendering. These bytes are supervisor-composition input;
/// this value keeps that framed boundary bounded and nonempty.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionSourceRendering(Vec<u8>);

impl MissionSourceRendering {
    pub const MAX_BYTES: usize = 16_384;

    pub fn parse(bytes: Vec<u8>) -> Result<Self, DomainValueError> {
        if bytes.is_empty() || bytes.len() > Self::MAX_BYTES {
            return Err(DomainValueError::InvalidMissionSourceRendering);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn digest(&self) -> Blake3Digest {
        Blake3Digest::of_bytes(self.as_bytes())
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UsdMicros(i64);

impl UsdMicros {
    pub const ZERO: Self = Self(0);

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

/// One nonnegative token total normalized from the Pi SDK's cumulative usage
/// snapshot. The trusted kernel keeps these five dimensions separate so a
/// session-wide provider total cannot be recombined across Office turns.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PiTokenCount(i64);

impl PiTokenCount {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: i64) -> Option<Self> {
        if value >= 0 { Some(Self(value)) } else { None }
    }

    pub const fn value(self) -> i64 {
        self.0
    }
}

impl TryFrom<i64> for PiTokenCount {
    type Error = DomainValueError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(DomainValueError::NegativePiTokenCount(value))
    }
}

/// Exact raw IEEE-754 binary64 provider-cost evidence from the Pi boundary.
///
/// PostgreSQL stores these eight big-endian bytes alongside the independently
/// verified integer micro-USD ceiling. No Rust `f64` round trip may rewrite
/// the observed provider value before replay or accounting compares it.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProviderCostBinary64([u8; 8]);

impl ProviderCostBinary64 {
    pub fn from_big_endian(bytes: [u8; 8]) -> Result<Self, DomainValueError> {
        let bits = u64::from_be_bytes(bytes);
        if bits >> 63 != 0 || (bits >> 52) & 0x7ff == 0x7ff {
            return Err(DomainValueError::InvalidProviderCostBinary64);
        }
        Ok(Self(bytes))
    }

    pub const fn as_big_endian_bytes(self) -> [u8; 8] {
        self.0
    }

    /// `ceil(binary64 * 1_000_000)` using integer arithmetic, matching the
    /// Pi boundary's normalization rule exactly.
    pub fn ceil_micro_usd(self) -> Result<UsdMicros, DomainValueError> {
        let bits = u64::from_be_bytes(self.0);
        let exponent = ((bits >> 52) & 0x7ff) as i32;
        let fraction = bits & ((1_u64 << 52) - 1);
        if exponent == 0 && fraction == 0 {
            return Ok(UsdMicros::ZERO);
        }
        let (significand, binary_exponent) = if exponent == 0 {
            (u128::from(fraction), -1074_i32)
        } else {
            (u128::from((1_u64 << 52) | fraction), exponent - 1023 - 52)
        };
        let numerator = significand
            .checked_mul(1_000_000)
            .ok_or(DomainValueError::ProviderCostMicroUsdOverflow)?;
        let micro_usd = if binary_exponent >= 0 {
            numerator
                .checked_shl(
                    u32::try_from(binary_exponent)
                        .map_err(|_| DomainValueError::ProviderCostMicroUsdOverflow)?,
                )
                .ok_or(DomainValueError::ProviderCostMicroUsdOverflow)?
        } else {
            let denominator_shift = u32::try_from(-binary_exponent)
                .map_err(|_| DomainValueError::ProviderCostMicroUsdOverflow)?;
            if denominator_shift >= 128 {
                1
            } else {
                let denominator = 1_u128 << denominator_shift;
                let quotient = numerator / denominator;
                quotient + u128::from(numerator % denominator != 0)
            }
        };
        i64::try_from(micro_usd)
            .ok()
            .and_then(UsdMicros::new)
            .ok_or(DomainValueError::ProviderCostMicroUsdOverflow)
    }
}

/// One complete session-cumulative Pi usage observation. The token sum and
/// raw-cost ceiling are independently rechecked before it crosses into the
/// durable Office-turn ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PiCumulativeUsage {
    pub input_tokens: PiTokenCount,
    pub output_tokens: PiTokenCount,
    pub cache_read_tokens: PiTokenCount,
    pub cache_write_tokens: PiTokenCount,
    pub total_tokens: PiTokenCount,
    pub provider_cost: ProviderCostBinary64,
    pub ceiling_micro_usd: UsdMicros,
}

impl PiCumulativeUsage {
    pub fn is_internally_consistent(self) -> bool {
        self.input_tokens
            .value()
            .checked_add(self.output_tokens.value())
            .and_then(|value| value.checked_add(self.cache_read_tokens.value()))
            .and_then(|value| value.checked_add(self.cache_write_tokens.value()))
            == Some(self.total_tokens.value())
            && self.provider_cost.ceil_micro_usd().ok() == Some(self.ceiling_micro_usd)
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
    InstallRootAuthorityOffice = 2,
    InstallFoundingMission = 3,
    AppointInitialRootAuthority = 4,
    SetR0HardCeiling = 5,
    BootstrapSociety = 6,
    ProposeOperatingCycle = 7,
    AdmitOperatingCycle = 8,
    QuiesceOperatingCycle = 9,
    ResumeOperatingCycle = 10,
    ReconcileOperatingCycle = 11,
    CloseOperatingCycle = 12,
    StartRootAuthorityOfficeSession = 13,
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
    FinalizeDeterministicExperiment = 68,
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
    RecordNativeChildNotSpawned = 85,
    AuthorizePiOfficeTurnPrompt = 86,
    RecordPiOfficeTurnPromptDelivery = 87,
    RecordPiOfficeTurnPromptAccepted = 88,
    RecordPiOfficeTurnUsage = 89,
    RecordPiOfficeTurnUsageFailure = 90,
    RecordPiOfficeTurnTerminal = 91,
    /// Authorizes one exact Pi `Dispose` correlation before the daemon may
    /// touch the host pipe. It binds the current cycle generation and idle
    /// peer-ready child, preventing a post-write delivery receipt from being
    /// the first authority decision.
    AuthorizePiOfficeSessionDispose = 92,
    /// Records the complete physical handoff of the one closing Dispose
    /// command. Host acceptance and final accounting are separate facts.
    RecordPiOfficeSessionDisposeDelivery = 93,
    RecordPiOfficeSessionDisposeAccepted = 94,
    RecordPiOfficeSessionDisposeUsage = 95,
    RecordPiOfficeSessionDisposeUsageFailure = 96,
    RecordPiOfficeSessionDisposed = 97,
    /// Provider-free admission for one deterministic evaluator native child.
    /// The input binds the exact registered evaluator experiment; it has no Pi
    /// identity and no monetary-reservation field because this fixture profile
    /// is explicitly non-monetized.
    AdmitDeterministicEvaluatorNativeChild = 98,
    RecordDeterministicEvaluatorNativeChildSpawn = 99,
    /// Binds one reaped deterministic evaluator child's complete stdout to a
    /// fresh forensic occurrence. The output object is derived, never chosen.
    RegisterDeterministicEvaluatorForensicManifest = 100,
    /// Executes one closed, generic experimental-control transition.  The
    /// nested study command has its own normalized named body and cannot carry
    /// application JSON or a generic metadata payload.
    RunStudyTransition = 101,
}

impl Capability {
    pub const FOUNDING: [Self; 8] = [
        Self::CreateSocietyIdentity,
        Self::InstallRootAuthorityOffice,
        Self::InstallFoundingMission,
        Self::AppointInitialRootAuthority,
        Self::SetR0HardCeiling,
        Self::BootstrapSociety,
        Self::ProposeOperatingCycle,
        Self::AdmitOperatingCycle,
    ];

    pub const ROOT_AUTHORITY: [Self; 44] = [
        Self::ProposeOperatingCycle,
        Self::AdmitOperatingCycle,
        Self::QuiesceOperatingCycle,
        Self::ResumeOperatingCycle,
        Self::ReconcileOperatingCycle,
        Self::CloseOperatingCycle,
        Self::StartRootAuthorityOfficeSession,
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
        Self::FinalizeDeterministicExperiment,
    ];

    pub const KERNEL_SERVICE: [Self; 50] = [
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
        Self::RecordNativeChildNotSpawned,
        Self::AuthorizePiOfficeTurnPrompt,
        Self::RecordPiOfficeTurnPromptDelivery,
        Self::RecordPiOfficeTurnPromptAccepted,
        Self::RecordPiOfficeTurnUsage,
        Self::RecordPiOfficeTurnUsageFailure,
        Self::RecordPiOfficeTurnTerminal,
        Self::AuthorizePiOfficeSessionDispose,
        Self::RecordPiOfficeSessionDisposeDelivery,
        Self::RecordPiOfficeSessionDisposeAccepted,
        Self::RecordPiOfficeSessionDisposeUsage,
        Self::RecordPiOfficeSessionDisposeUsageFailure,
        Self::RecordPiOfficeSessionDisposed,
        Self::AdmitDeterministicEvaluatorNativeChild,
        Self::RecordDeterministicEvaluatorNativeChildSpawn,
        Self::RegisterDeterministicEvaluatorForensicManifest,
        Self::RunStudyTransition,
    ];

    pub const fn requires_consumption(self) -> bool {
        matches!(
            self,
            Self::CreateSocietyIdentity
                | Self::InstallRootAuthorityOffice
                | Self::InstallFoundingMission
                | Self::AppointInitialRootAuthority
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
    RootAuthorityOffice = 1,
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

/// The finite operating treatments admitted by the current prototype. A
/// treatment carries its constitutional budget exactly; callers never select
/// an arbitrary ceiling for a cycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum OperatingCycleTreatment {
    PiSdkQualificationV1 = 1,
    PinnedPiSdkLiveV1 = 2,
    /// Provider-free process-double treatment for trusted-kernel/supervisor
    /// fixtures. It carries the pinned Pi SDK cycle envelope but denies
    /// provider access and cannot stand in for the paid native qualification
    /// run.
    DeterministicPiHostFixtureV1 = 3,
    /// Provider-free fixture treatment for deterministic evaluator processes.
    /// It is intentionally distinct from the Pi host double: evaluator work
    /// must not manufacture Pi identity or become paid/live work.
    DeterministicEvaluatorFixtureV1 = 4,
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

/// The M3 execution foundation permits only the current pinned Pi SDK model
/// policy. A future policy mutation needs its own versioned, qualified path;
/// a provider/model string may not quietly change an Actor's identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum ActorModelPolicy {
    PinnedDeepseekV4FlashHigh = 1,
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
    DeterministicEvaluatorProcessFixtureV1 = 3,
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
/// to the Root Authority's Office session that initiated it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PiChildOwner {
    ActorAttempt(ActorAttemptId),
    RootAuthorityOfficeSession(RootAuthorityOfficeSessionId),
}

/// The closed owner union for a native child.  Only the Pi arm has a Pi
/// session/protocol sidecar.  The evaluator arm pins the exact experiment
/// revision and input manifest that the child may execute; it is not an
/// application-defined discriminator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeChildOwner {
    Pi(PiChildOwner),
    DeterministicEvaluator {
        deterministic_experiment_id: DeterministicExperimentId,
        evaluator_revision_id: EvaluatorRevisionId,
        input_manifest_id: InputManifestId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum NativeChildSpawnAdmissionState {
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

/// Closed normalization of `society-pi::TurnReceipt::disposition`. It is a
/// protocol result, not an assertion that an Office command succeeded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PiOfficeTurnDisposition {
    Completed = 1,
    Length = 2,
    Error = 3,
    Aborted = 4,
    Failed = 5,
    ProtocolFailed = 6,
}

/// Closed normalization of the final assistant arm paired with a Pi turn
/// disposition. Keeping the pair explicit prevents a zero process exit or a
/// generic "success" label from settling an Office turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PiOfficeTurnAssistantOutcome {
    ObservedStop = 1,
    ObservedLength = 2,
    ObservedError = 3,
    ObservedAborted = 4,
    SdkPromiseRejected = 5,
    MissingFinalAssistantOutcome = 6,
}

/// The two peer-valid sequence topologies for a terminal Office Prompt.
/// Observed assistant outcomes require the exact `agent_settled` boundary;
/// an SDK-level failure may have no agent lifecycle at all and is instead
/// closed by its final Prompt-correlated Known usage fact immediately before
/// `Settled`. The variants prevent callers from inventing an optional agent
/// sequence for the unavailable-assistant path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PiOfficeTurnTerminalEvidence {
    ObservedAssistant {
        agent_settled_sequence: PiProtocolSequence,
        final_accounting_sequence: PiProtocolSequence,
    },
    UnavailableAssistant {
        final_known_usage_sequence: PiProtocolSequence,
    },
}

impl PiOfficeTurnTerminalEvidence {
    pub const fn final_accounting_sequence(self) -> PiProtocolSequence {
        match self {
            Self::ObservedAssistant {
                final_accounting_sequence,
                ..
            }
            | Self::UnavailableAssistant {
                final_known_usage_sequence: final_accounting_sequence,
            } => final_accounting_sequence,
        }
    }

    pub const fn agent_settled_sequence(self) -> Option<PiProtocolSequence> {
        match self {
            Self::ObservedAssistant {
                agent_settled_sequence,
                ..
            } => Some(agent_settled_sequence),
            Self::UnavailableAssistant { .. } => None,
        }
    }

    pub const fn accepts(self, outcome: PiOfficeTurnAssistantOutcome) -> bool {
        matches!(
            (self, outcome),
            (
                Self::ObservedAssistant { .. },
                PiOfficeTurnAssistantOutcome::ObservedStop
                    | PiOfficeTurnAssistantOutcome::ObservedLength
                    | PiOfficeTurnAssistantOutcome::ObservedError
                    | PiOfficeTurnAssistantOutcome::ObservedAborted
            ) | (
                Self::UnavailableAssistant { .. },
                PiOfficeTurnAssistantOutcome::SdkPromiseRejected
                    | PiOfficeTurnAssistantOutcome::MissingFinalAssistantOutcome
            )
        )
    }
}

impl PiOfficeTurnDisposition {
    pub const fn accepts(self, outcome: PiOfficeTurnAssistantOutcome) -> bool {
        matches!(
            (self, outcome),
            (Self::Completed, PiOfficeTurnAssistantOutcome::ObservedStop)
                | (Self::Length, PiOfficeTurnAssistantOutcome::ObservedLength)
                | (Self::Error, PiOfficeTurnAssistantOutcome::ObservedError)
                | (Self::Aborted, PiOfficeTurnAssistantOutcome::ObservedAborted)
                | (
                    Self::Failed,
                    PiOfficeTurnAssistantOutcome::SdkPromiseRejected
                )
                | (
                    Self::ProtocolFailed,
                    PiOfficeTurnAssistantOutcome::MissingFinalAssistantOutcome
                )
        )
    }

    pub const fn may_return_office_to_ready(self) -> bool {
        matches!(self, Self::Completed)
    }
}

/// The per-turn durable disposition of the session transcript. Pi emits the
/// actual SessionManager flush receipt only on `Dispose`, so M6 records this
/// precise deferral rather than falsely attaching one session transcript to
/// every Prompt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PiOfficeTurnTranscriptDisposition {
    DeferredUntilOfficeSessionDispose = 1,
}

/// The peer's Dispose receipt says whether a first user Prompt was present in
/// the flushed SessionManager transcript. The kernel independently binds a
/// verified digest to its prior Prompt authorization; this closed receipt
/// never accepts an SDK narrative or arbitrary transcript metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PiOfficeSessionFirstUserPromptReceipt {
    Absent,
    Verified { digest: Blake3Digest },
}

/// The final peer-sealed transcript materialization for one Pi Office
/// session. A materialized file is admitted only through the normal sealed
/// content-object boundary and the exact host digest; an empty prompt history
/// may instead produce the closed unmaterialized receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PiOfficeSessionTranscriptReceipt {
    Materialized {
        session_file: CanonicalPiSessionTranscriptPath,
        session_file_digest: Blake3Digest,
        transcript_content_object_id: ContentObjectId,
        first_user_prompt: PiOfficeSessionFirstUserPromptReceipt,
    },
    UnmaterializedNoPrompt {
        session_file: CanonicalPiSessionTranscriptPath,
    },
}

/// The final reservation disposition recorded with a peer-valid Dispose
/// terminal carrying Known usage. A usage failure cannot produce this
/// terminal at all: the peer protocol fatals after that boundary and leaves
/// the frozen parent/session for a later recovery tranche. A known overrun is
/// still a durable physical Dispose fact but remains a frozen duty.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PiOfficeSessionDisposeBudgetDisposition {
    Reconciled {
        observed_cumulative_micro_usd: UsdMicros,
    },
    Frozen {
        cancellation_request_id: CancellationRequestId,
        postmortem_id: CostPostmortemId,
    },
}

/// Why the trusted boundary could not produce a usable cumulative provider
/// accounting observation. These exact Pi reasons remain durable even though
/// the cross-cutting CostPostmortem uses its coarser conservative cause.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PiOfficeTurnUsageUnknownReason {
    MissingFinalUsageSnapshot = 1,
    BoundaryStreamInterrupted = 2,
    TerminalEvidenceMissing = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum PiOfficeTurnUsageUnavailableReason {
    InvalidSdkUsage = 1,
    UsageRegressed = 2,
    UsageInconsistent = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PiOfficeTurnUsageFailure {
    Unknown(PiOfficeTurnUsageUnknownReason),
    Unavailable(PiOfficeTurnUsageUnavailableReason),
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
pub enum NativeChildNotSpawnedReason {
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

/// The one direct process grammar the generic evaluator custody bridge may
/// execute. Its application-owned program receives only the fixed
/// `--input-manifest` file in a daemon-owned workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum EvaluatorExecutionContract {
    DirectExecutableFixedInputManifestV1 = 1,
}

/// The generic success boundary for a registered application evaluator. The
/// kernel never parses its stdout grammar; exit zero only attests that the
/// exact sealed application contract produced its canonical observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum EvaluatorOutputContract {
    ExitZeroCanonicalObservationV1 = 1,
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
    Failed = 4,
    Cancelled = 5,
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

/// Every currently admitted evaluator observation remains limited by the
/// fact that its application-owned grammar is opaque to generic physics.
/// This records the boundary without admitting application vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum EvidenceLimitationKind {
    ApplicationSemanticsUninterpreted = 1,
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
    RootAuthorityRequested = 1,
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
    /// The SDK boundary supplied an explicit unusable accounting observation.
    /// This is deliberately distinct from a provider or credential outage: the
    /// exact Pi reason remains on its named Office-turn receipt.
    AdapterAccountingUnavailable = 4,
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
    InstallRootAuthorityOffice,
    InstallFoundingMission {
        mission: ApplicationMissionInput,
    },
    AppointInitialRootAuthority {
        actor_display_name: PrincipalDisplayName,
    },
    SetR0HardCeiling {
        ceiling: UsdMicros,
    },
    BootstrapSociety,
    ProposeOperatingCycle {
        treatment: OperatingCycleTreatment,
        budget_ceiling: UsdMicros,
    },
    AdmitOperatingCycle {
        cycle_id: OperatingCycleId,
    },
    StartRootAuthorityOfficeSession {
        cycle_id: OperatingCycleId,
    },
    RecordOfficeSessionReady {
        session_id: RootAuthorityOfficeSessionId,
    },
    RecordOfficeSessionTerminal {
        session_id: RootAuthorityOfficeSessionId,
        terminal_state: OfficeSessionTerminalState,
    },
    OpenOfficeTurn {
        session_id: RootAuthorityOfficeSessionId,
        purpose: OfficeTurnPurpose,
    },
    SettleOfficeTurn {
        turn_id: OfficeTurnId,
        terminal_receipt_id: PiOfficeTurnTerminalReceiptId,
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
        north_star_alignment: ProjectNorthStarAlignment,
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
        rendering_digest: Blake3Digest,
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
        digest: Blake3Digest,
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
    /// The resident scheduler may only turn the complete stdout seal of its
    /// exact evaluator child into a forensic occurrence. Experiment, revision,
    /// input manifest, stream seal, and output object are all derived from the
    /// durable admission; no application or daemon-selected output joins here.
    RegisterDeterministicEvaluatorForensicManifest {
        operating_cycle_id: OperatingCycleId,
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
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
    },
    FinalizeDeterministicExperiment {
        operating_cycle_id: OperatingCycleId,
        deterministic_experiment_id: DeterministicExperimentId,
    },
    AdmitDeterministicEvaluatorNativeChild {
        operating_cycle_id: OperatingCycleId,
        deterministic_experiment_id: DeterministicExperimentId,
        evaluator_revision_id: EvaluatorRevisionId,
        input_manifest_id: InputManifestId,
        execution_profile_id: ExecutionProfileId,
        native_workspace_id: NativeWorkspaceId,
        canonical_workspace_path: CanonicalWorkspacePath,
        supervisor_epoch_id: SupervisorEpochId,
        supervisor_epoch_identity: SupervisorEpochIdentity,
    },
    RecordDeterministicEvaluatorNativeChildSpawn {
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
        child_identity: SupervisedChildIdentity,
        direct_child_pid: NativeChildPid,
        process_group_id: OwnedProcessGroupId,
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
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
        child_identity: SupervisedChildIdentity,
        direct_child_pid: NativeChildPid,
        process_group_id: OwnedProcessGroupId,
    },
    RecordPiAdapterReady {
        native_child_id: NativeChildId,
        pi_session_identity: PiBoundarySessionIdentity,
        spawn_nonce: SpawnNonce,
    },
    AuthorizePiCreateSession {
        native_child_id: NativeChildId,
        correlation_identity: PiCorrelationIdentity,
        create_request_digest: Blake3Digest,
    },
    RecordPiCreateSessionDelivery {
        native_child_id: NativeChildId,
        correlation_identity: PiCorrelationIdentity,
        create_request_digest: Blake3Digest,
    },
    RecordPiSessionReady {
        native_child_id: NativeChildId,
        pi_session_identity: PiBoundarySessionIdentity,
    },
    RecordPiAbortControlDelivery {
        native_child_id: NativeChildId,
        cancellation_propagation_id: CancellationPropagationId,
        correlation_identity: PiCorrelationIdentity,
        abort_command_digest: Blake3Digest,
        outcome: PiAbortControlWriteOutcome,
    },
    RecordChildStreamSeal {
        native_child_id: NativeChildId,
        stream_kind: ChildStreamKind,
        full_observed_digest: Blake3Digest,
        retained_content_object_id: ContentObjectId,
        completeness: ChildStreamSealCompleteness,
    },
    RecordChildProcessLiveness {
        native_child_id: NativeChildId,
        liveness: ProcessGroupLiveness,
    },
    RecordProcessSignalReceipt {
        native_child_id: NativeChildId,
        action: ProcessSignalAction,
        delivery: ProcessSignalDelivery,
        observed_liveness: ProcessGroupLiveness,
        cause: ProcessSignalCause,
    },
    RecordDirectChildReap {
        native_child_id: NativeChildId,
        wait_status: DirectChildWaitStatus,
        group_liveness_before_cleanup: ProcessGroupLiveness,
        group_liveness_after_cleanup: ProcessGroupLiveness,
    },
    RecordChildRecovery {
        native_child_id: NativeChildId,
        observation: ChildRecoveryObservation,
        group_liveness_after_restart: ProcessGroupLiveness,
    },
    /// A terminal classification is derived only from preceding receipt rows;
    /// this command does not accept a caller-selected result.
    FinalizeChildProcess {
        native_child_id: NativeChildId,
    },
    BeginCancellationPropagation {
        cancellation_request_id: CancellationRequestId,
    },
    ReconcileCancellationPropagation {
        cancellation_propagation_id: CancellationPropagationId,
    },
    RecordNativeChildNotSpawned {
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
        reason: NativeChildNotSpawnedReason,
    },
    /// Final, provider-free kernel authority for one exact Office Prompt.
    /// The sealed bytes and current ledger frontier are named before anything
    /// may reach the physical stdin pipe.
    AuthorizePiOfficeTurnPrompt {
        office_turn_id: OfficeTurnId,
        correlation_identity: PiCorrelationIdentity,
        prompt_content_object_id: ContentObjectId,
        prompt_digest: Blake3Digest,
        frontier_event_id: EventId,
    },
    /// Attests only a complete physical Prompt write for an already authorized
    /// correlation. It is deliberately distinct from host acceptance.
    RecordPiOfficeTurnPromptDelivery {
        office_turn_id: OfficeTurnId,
        correlation_identity: PiCorrelationIdentity,
        prompt_digest: Blake3Digest,
    },
    /// Attests the Pi-host's accepted Prompt command result at one exact
    /// outbound protocol sequence after the complete physical delivery.
    RecordPiOfficeTurnPromptAccepted {
        office_turn_id: OfficeTurnId,
        correlation_identity: PiCorrelationIdentity,
        command_result_sequence: PiProtocolSequence,
    },
    /// One exact known cumulative SDK usage snapshot. Its values are
    /// session-scoped and must monotonically extend the prior checkpoint.
    RecordPiOfficeTurnUsage {
        office_turn_id: OfficeTurnId,
        correlation_identity: PiCorrelationIdentity,
        protocol_sequence: PiProtocolSequence,
        usage: PiCumulativeUsage,
    },
    /// Preserves a typed inability to account for a Prompt and atomically
    /// freezes the parent Office-session reservation rather than treating it
    /// as zero spend.
    RecordPiOfficeTurnUsageFailure {
        office_turn_id: OfficeTurnId,
        correlation_identity: PiCorrelationIdentity,
        protocol_sequence: PiProtocolSequence,
        failure: PiOfficeTurnUsageFailure,
    },
    /// The closed normalized peer terminal chain. Observed assistant outcomes
    /// preserve `agent_settled -> final accounting fact -> Settled`; an
    /// unavailable assistant outcome preserves `final Known usage fact ->
    /// Settled` because SDK command failure can precede any agent lifecycle.
    /// No generic SDK event or narrative output can substitute for either.
    RecordPiOfficeTurnTerminal {
        office_turn_id: OfficeTurnId,
        correlation_identity: PiCorrelationIdentity,
        terminal_evidence: PiOfficeTurnTerminalEvidence,
        settled_sequence: PiProtocolSequence,
        disposition: PiOfficeTurnDisposition,
        assistant_outcome: PiOfficeTurnAssistantOutcome,
        transcript_disposition: PiOfficeTurnTranscriptDisposition,
    },
    /// Freezes one exact Dispose correlation under the current cycle
    /// generation before the daemon performs the physical pipe write.
    AuthorizePiOfficeSessionDispose {
        session_id: RootAuthorityOfficeSessionId,
        correlation_identity: PiCorrelationIdentity,
    },
    /// Attests the complete physical Pi `Dispose` write for an idle Office
    /// session. It deliberately precedes the host CommandResult and terminal
    /// receipt, so a crash cannot turn a partial closing observation into a
    /// synthetic completed disposal.
    RecordPiOfficeSessionDisposeDelivery {
        session_id: RootAuthorityOfficeSessionId,
        correlation_identity: PiCorrelationIdentity,
    },
    /// Attests the accepted Pi `Dispose` CommandResult at one exact outbound
    /// protocol sequence after delivery.
    RecordPiOfficeSessionDisposeAccepted {
        session_id: RootAuthorityOfficeSessionId,
        correlation_identity: PiCorrelationIdentity,
        command_result_sequence: PiProtocolSequence,
    },
    /// The exact final known SDK cumulative usage emitted by a peer after it
    /// accepted Dispose. This does not finalize the reservation until the
    /// peer's terminal transcript receipt arrives.
    RecordPiOfficeSessionDisposeUsage {
        session_id: RootAuthorityOfficeSessionId,
        correlation_identity: PiCorrelationIdentity,
        protocol_sequence: PiProtocolSequence,
        usage: PiCumulativeUsage,
    },
    /// Preserves a final typed accounting failure and freezes the parent
    /// reservation before terminal Dispose evidence may close the session.
    RecordPiOfficeSessionDisposeUsageFailure {
        session_id: RootAuthorityOfficeSessionId,
        correlation_identity: PiCorrelationIdentity,
        protocol_sequence: PiProtocolSequence,
        failure: PiOfficeTurnUsageFailure,
    },
    /// The peer's final `Disposed` boundary. It proves the same-correlation
    /// transcript flush and atomically reconciles a known parent remainder,
    /// or durably records the frozen cancellation/postmortem outcome.
    RecordPiOfficeSessionDisposed {
        session_id: RootAuthorityOfficeSessionId,
        correlation_identity: PiCorrelationIdentity,
        disposed_sequence: PiProtocolSequence,
        transcript_receipt: PiOfficeSessionTranscriptReceipt,
    },
    StudyTransition {
        command: crate::StudyCommand,
    },
}

impl CommandBody {
    pub const fn kind(&self) -> CommandKind {
        match self {
            Self::CreateSocietyIdentity { .. } => CommandKind::CreateSocietyIdentity,
            Self::InstallRootAuthorityOffice => CommandKind::InstallRootAuthorityOffice,
            Self::InstallFoundingMission { .. } => CommandKind::InstallFoundingMission,
            Self::AppointInitialRootAuthority { .. } => CommandKind::AppointInitialRootAuthority,
            Self::SetR0HardCeiling { .. } => CommandKind::SetR0HardCeiling,
            Self::BootstrapSociety => CommandKind::BootstrapSociety,
            Self::ProposeOperatingCycle { .. } => CommandKind::ProposeOperatingCycle,
            Self::AdmitOperatingCycle { .. } => CommandKind::AdmitOperatingCycle,
            Self::StartRootAuthorityOfficeSession { .. } => {
                CommandKind::StartRootAuthorityOfficeSession
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
            Self::RegisterDeterministicEvaluatorForensicManifest { .. } => {
                CommandKind::RegisterDeterministicEvaluatorForensicManifest
            }
            Self::RegisterDeterministicExperiment { .. } => {
                CommandKind::RegisterDeterministicExperiment
            }
            Self::RecordDeterministicEvaluationReceipt { .. } => {
                CommandKind::RecordDeterministicEvaluationReceipt
            }
            Self::AdmitDeterministicEvidence { .. } => CommandKind::AdmitDeterministicEvidence,
            Self::FinalizeDeterministicExperiment { .. } => {
                CommandKind::FinalizeDeterministicExperiment
            }
            Self::AdmitDeterministicEvaluatorNativeChild { .. } => {
                CommandKind::AdmitDeterministicEvaluatorNativeChild
            }
            Self::RecordDeterministicEvaluatorNativeChildSpawn { .. } => {
                CommandKind::RecordDeterministicEvaluatorNativeChildSpawn
            }
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
            Self::RecordNativeChildNotSpawned { .. } => CommandKind::RecordNativeChildNotSpawned,
            Self::AuthorizePiOfficeTurnPrompt { .. } => CommandKind::AuthorizePiOfficeTurnPrompt,
            Self::RecordPiOfficeTurnPromptDelivery { .. } => {
                CommandKind::RecordPiOfficeTurnPromptDelivery
            }
            Self::RecordPiOfficeTurnPromptAccepted { .. } => {
                CommandKind::RecordPiOfficeTurnPromptAccepted
            }
            Self::RecordPiOfficeTurnUsage { .. } => CommandKind::RecordPiOfficeTurnUsage,
            Self::RecordPiOfficeTurnUsageFailure { .. } => {
                CommandKind::RecordPiOfficeTurnUsageFailure
            }
            Self::RecordPiOfficeTurnTerminal { .. } => CommandKind::RecordPiOfficeTurnTerminal,
            Self::AuthorizePiOfficeSessionDispose { .. } => {
                CommandKind::AuthorizePiOfficeSessionDispose
            }
            Self::RecordPiOfficeSessionDisposeDelivery { .. } => {
                CommandKind::RecordPiOfficeSessionDisposeDelivery
            }
            Self::RecordPiOfficeSessionDisposeAccepted { .. } => {
                CommandKind::RecordPiOfficeSessionDisposeAccepted
            }
            Self::RecordPiOfficeSessionDisposeUsage { .. } => {
                CommandKind::RecordPiOfficeSessionDisposeUsage
            }
            Self::RecordPiOfficeSessionDisposeUsageFailure { .. } => {
                CommandKind::RecordPiOfficeSessionDisposeUsageFailure
            }
            Self::RecordPiOfficeSessionDisposed { .. } => {
                CommandKind::RecordPiOfficeSessionDisposed
            }
            Self::StudyTransition { .. } => CommandKind::StudyTransition,
        }
    }

    pub const fn required_capability(&self) -> Capability {
        match self {
            Self::CreateSocietyIdentity { .. } => Capability::CreateSocietyIdentity,
            Self::InstallRootAuthorityOffice => Capability::InstallRootAuthorityOffice,
            Self::InstallFoundingMission { .. } => Capability::InstallFoundingMission,
            Self::AppointInitialRootAuthority { .. } => Capability::AppointInitialRootAuthority,
            Self::SetR0HardCeiling { .. } => Capability::SetR0HardCeiling,
            Self::BootstrapSociety => Capability::BootstrapSociety,
            Self::ProposeOperatingCycle { .. } => Capability::ProposeOperatingCycle,
            Self::AdmitOperatingCycle { .. } => Capability::AdmitOperatingCycle,
            Self::StartRootAuthorityOfficeSession { .. } => {
                Capability::StartRootAuthorityOfficeSession
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
            Self::RegisterDeterministicEvaluatorForensicManifest { .. } => {
                Capability::RegisterDeterministicEvaluatorForensicManifest
            }
            Self::RegisterDeterministicExperiment { .. } => {
                Capability::RegisterDeterministicExperiment
            }
            Self::RecordDeterministicEvaluationReceipt { .. } => {
                Capability::RecordDeterministicEvaluationReceipt
            }
            Self::AdmitDeterministicEvidence { .. } => Capability::AdmitDeterministicEvidence,
            Self::FinalizeDeterministicExperiment { .. } => {
                Capability::FinalizeDeterministicExperiment
            }
            Self::AdmitDeterministicEvaluatorNativeChild { .. } => {
                Capability::AdmitDeterministicEvaluatorNativeChild
            }
            Self::RecordDeterministicEvaluatorNativeChildSpawn { .. } => {
                Capability::RecordDeterministicEvaluatorNativeChildSpawn
            }
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
            Self::RecordNativeChildNotSpawned { .. } => Capability::RecordNativeChildNotSpawned,
            Self::AuthorizePiOfficeTurnPrompt { .. } => Capability::AuthorizePiOfficeTurnPrompt,
            Self::RecordPiOfficeTurnPromptDelivery { .. } => {
                Capability::RecordPiOfficeTurnPromptDelivery
            }
            Self::RecordPiOfficeTurnPromptAccepted { .. } => {
                Capability::RecordPiOfficeTurnPromptAccepted
            }
            Self::RecordPiOfficeTurnUsage { .. } => Capability::RecordPiOfficeTurnUsage,
            Self::RecordPiOfficeTurnUsageFailure { .. } => {
                Capability::RecordPiOfficeTurnUsageFailure
            }
            Self::RecordPiOfficeTurnTerminal { .. } => Capability::RecordPiOfficeTurnTerminal,
            Self::AuthorizePiOfficeSessionDispose { .. } => {
                Capability::AuthorizePiOfficeSessionDispose
            }
            Self::RecordPiOfficeSessionDisposeDelivery { .. } => {
                Capability::RecordPiOfficeSessionDisposeDelivery
            }
            Self::RecordPiOfficeSessionDisposeAccepted { .. } => {
                Capability::RecordPiOfficeSessionDisposeAccepted
            }
            Self::RecordPiOfficeSessionDisposeUsage { .. } => {
                Capability::RecordPiOfficeSessionDisposeUsage
            }
            Self::RecordPiOfficeSessionDisposeUsageFailure { .. } => {
                Capability::RecordPiOfficeSessionDisposeUsageFailure
            }
            Self::RecordPiOfficeSessionDisposed { .. } => Capability::RecordPiOfficeSessionDisposed,
            Self::StudyTransition { .. } => Capability::RunStudyTransition,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum CommandKind {
    CreateSocietyIdentity = 1,
    InstallRootAuthorityOffice = 2,
    InstallFoundingMission = 3,
    AppointInitialRootAuthority = 4,
    SetR0HardCeiling = 5,
    BootstrapSociety = 6,
    ProposeOperatingCycle = 7,
    AdmitOperatingCycle = 8,
    StartRootAuthorityOfficeSession = 9,
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
    FinalizeDeterministicExperiment = 68,
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
    RecordNativeChildNotSpawned = 85,
    AuthorizePiOfficeTurnPrompt = 86,
    RecordPiOfficeTurnPromptDelivery = 87,
    RecordPiOfficeTurnPromptAccepted = 88,
    RecordPiOfficeTurnUsage = 89,
    RecordPiOfficeTurnUsageFailure = 90,
    RecordPiOfficeTurnTerminal = 91,
    AuthorizePiOfficeSessionDispose = 92,
    RecordPiOfficeSessionDisposeDelivery = 93,
    RecordPiOfficeSessionDisposeAccepted = 94,
    RecordPiOfficeSessionDisposeUsage = 95,
    RecordPiOfficeSessionDisposeUsageFailure = 96,
    RecordPiOfficeSessionDisposed = 97,
    AdmitDeterministicEvaluatorNativeChild = 98,
    RecordDeterministicEvaluatorNativeChildSpawn = 99,
    RegisterDeterministicEvaluatorForensicManifest = 100,
    StudyTransition = 101,
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
/// handwritten numeric match.  PostgreSQL stores the `i64` value; the local
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
    PiOfficeTurnAuthorityMissing = 49,
    PiOfficeTurnPromptBindingMismatch = 50,
    PiOfficeTurnUsageNotMonotonic = 51,
    PiOfficeTurnTerminalEvidenceMissing = 52,
    PiOfficeTurnNotReconciled = 53,
    PiOfficeTurnTreatmentIneligible = 54,
    OfficeSessionBudgetRequiresDispose = 55,
    PiOfficeTurnTerminalAlreadyRecorded = 56,
    PiOfficeTurnUsageAlreadyFrozen = 57,
    ProjectNorthStarAlignmentMismatch = 58,
    PiOfficeSessionDisposeBindingMismatch = 59,
    PiOfficeSessionDisposeUsageNotMonotonic = 60,
    PiOfficeSessionDisposeReceiptMissing = 61,
    MissionSourceContentNotSealed = 62,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventBody {
    SocietyIdentityCreated {
        society_id: SocietyId,
    },
    RootAuthorityOfficeInstalled {
        office_id: OfficeId,
    },
    FoundingMissionInstalled {
        mission_id: FoundingMissionId,
        application_revision_id: ApplicationRevisionId,
    },
    RootAuthorityAppointed {
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
        budget_ceiling: UsdMicros,
    },
    OperatingCycleStateChanged {
        cycle_id: OperatingCycleId,
        state: OperatingCycleState,
        generation: AdmissionGeneration,
    },
    RootAuthorityOfficeSessionStarted {
        session_id: RootAuthorityOfficeSessionId,
        cycle_id: OperatingCycleId,
    },
    RootAuthorityOfficeSessionStateChanged {
        session_id: RootAuthorityOfficeSessionId,
        state: OfficeSessionState,
    },
    OfficeTurnOpened {
        turn_id: OfficeTurnId,
        session_id: RootAuthorityOfficeSessionId,
        purpose: OfficeTurnPurpose,
    },
    OfficeTurnSettled {
        turn_id: OfficeTurnId,
        session_id: RootAuthorityOfficeSessionId,
        charged_delta: UsdMicros,
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
        application_revision_id: ApplicationRevisionId,
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
        digest: Blake3Digest,
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
    DeterministicEvaluatorForensicManifestRegistered {
        forensic_manifest_id: ForensicManifestId,
        deterministic_experiment_id: DeterministicExperimentId,
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
        native_child_stream_seal_id: NativeChildStreamSealId,
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
        limitation_kind: EvidenceLimitationKind,
    },
    DeterministicExperimentFinalized {
        deterministic_experiment_id: DeterministicExperimentId,
        terminal_state: DeterministicExperimentState,
    },
    DeterministicEvaluatorNativeChildAdmitted {
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
        owner: NativeChildOwner,
    },
    DeterministicEvaluatorNativeChildSpawnRecorded {
        native_child_id: NativeChildId,
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    },
    PiChildSpawnAdmitted {
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
        owner: PiChildOwner,
        budget_reservation_id: BudgetReservationId,
    },
    InertPiChildSpawnRecorded {
        native_child_id: NativeChildId,
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
    },
    PiAdapterReadyRecorded {
        native_child_id: NativeChildId,
        pi_session_id: PiSessionId,
    },
    PiCreateSessionAuthorized {
        native_child_id: NativeChildId,
    },
    PiCreateSessionDeliveryRecorded {
        native_child_id: NativeChildId,
    },
    PiSessionReadyRecorded {
        native_child_id: NativeChildId,
        pi_session_id: PiSessionId,
    },
    PiAbortControlDeliveryRecorded {
        pi_abort_control_receipt_id: PiAbortControlReceiptId,
        native_child_id: NativeChildId,
        cancellation_propagation_id: CancellationPropagationId,
        correlation_identity: PiCorrelationIdentity,
        abort_command_digest: Blake3Digest,
        outcome: PiAbortControlWriteOutcome,
    },
    ChildStreamSealed {
        native_child_stream_seal_id: NativeChildStreamSealId,
        native_child_id: NativeChildId,
        stream_kind: ChildStreamKind,
        completeness: ChildStreamSealCompleteness,
    },
    ChildProcessLivenessObserved {
        native_child_liveness_observation_id: NativeChildLivenessObservationId,
        native_child_id: NativeChildId,
        liveness: ProcessGroupLiveness,
    },
    ProcessSignalReceiptRecorded {
        process_signal_receipt_id: ProcessSignalReceiptId,
        native_child_id: NativeChildId,
        action: ProcessSignalAction,
        delivery: ProcessSignalDelivery,
        observed_liveness: ProcessGroupLiveness,
        cause: ProcessSignalCause,
    },
    DirectChildReaped {
        native_child_reap_receipt_id: NativeChildReapReceiptId,
        native_child_id: NativeChildId,
        wait_status: DirectChildWaitStatus,
        group_liveness_before_cleanup: ProcessGroupLiveness,
        group_liveness_after_cleanup: ProcessGroupLiveness,
    },
    ChildRecoveryObserved {
        native_child_recovery_receipt_id: NativeChildRecoveryReceiptId,
        native_child_id: NativeChildId,
        observation: ChildRecoveryObservation,
        group_liveness_after_restart: ProcessGroupLiveness,
    },
    ChildProcessFinalized {
        native_child_id: NativeChildId,
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
    NativeChildSpawnInvalidated {
        native_child_spawn_admission_id: NativeChildSpawnAdmissionId,
        reason: NativeChildNotSpawnedReason,
    },
    SupervisorEpochOpened {
        supervisor_epoch_id: SupervisorEpochId,
    },
    PiOfficeTurnPromptAuthorized {
        pi_office_turn_prompt_authorization_id: PiOfficeTurnPromptAuthorizationId,
        office_turn_id: OfficeTurnId,
        native_child_id: NativeChildId,
        correlation_identity: PiCorrelationIdentity,
        budget_reservation_id: BudgetReservationId,
    },
    PiOfficeTurnPromptDelivered {
        office_turn_id: OfficeTurnId,
        correlation_identity: PiCorrelationIdentity,
    },
    PiOfficeTurnPromptAccepted {
        office_turn_id: OfficeTurnId,
        correlation_identity: PiCorrelationIdentity,
        command_result_sequence: PiProtocolSequence,
    },
    PiOfficeTurnUsageRecorded {
        pi_office_turn_usage_receipt_id: PiOfficeTurnUsageReceiptId,
        office_turn_id: OfficeTurnId,
        protocol_sequence: PiProtocolSequence,
        cumulative_micro_usd: UsdMicros,
    },
    PiOfficeTurnUsageFrozen {
        office_turn_id: OfficeTurnId,
        budget_reservation_id: BudgetReservationId,
        cancellation_request_id: CancellationRequestId,
        postmortem_id: CostPostmortemId,
        failure: PiOfficeTurnUsageFailure,
    },
    PiOfficeTurnTerminalRecorded {
        pi_office_turn_terminal_receipt_id: PiOfficeTurnTerminalReceiptId,
        office_turn_id: OfficeTurnId,
        disposition: PiOfficeTurnDisposition,
        assistant_outcome: PiOfficeTurnAssistantOutcome,
    },
    PiOfficeSessionDisposeAuthorized {
        session_id: RootAuthorityOfficeSessionId,
        native_child_id: NativeChildId,
        correlation_identity: PiCorrelationIdentity,
        authorized_generation: AdmissionGeneration,
    },
    PiOfficeSessionDisposeDelivered {
        session_id: RootAuthorityOfficeSessionId,
        native_child_id: NativeChildId,
        correlation_identity: PiCorrelationIdentity,
    },
    PiOfficeSessionDisposeAccepted {
        session_id: RootAuthorityOfficeSessionId,
        correlation_identity: PiCorrelationIdentity,
        command_result_sequence: PiProtocolSequence,
    },
    PiOfficeSessionDisposeUsageRecorded {
        session_id: RootAuthorityOfficeSessionId,
        protocol_sequence: PiProtocolSequence,
        cumulative_micro_usd: UsdMicros,
    },
    PiOfficeSessionDisposeUsageFrozen {
        session_id: RootAuthorityOfficeSessionId,
        budget_reservation_id: BudgetReservationId,
        cancellation_request_id: CancellationRequestId,
        postmortem_id: CostPostmortemId,
        failure: PiOfficeTurnUsageFailure,
    },
    PiOfficeSessionDisposed {
        pi_office_session_dispose_receipt_id: PiOfficeSessionDisposeReceiptId,
        session_id: RootAuthorityOfficeSessionId,
        budget_reservation_id: BudgetReservationId,
        observed_cumulative_micro_usd: UsdMicros,
        budget_disposition: PiOfficeSessionDisposeBudgetDisposition,
    },
    StudyTransition {
        event: crate::StudyEvent,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i64)]
pub enum EventKind {
    SocietyIdentityCreated = 1,
    RootAuthorityOfficeInstalled = 2,
    FoundingMissionInstalled = 3,
    RootAuthorityAppointed = 4,
    R0HardCeilingSet = 5,
    SocietyBootstrapped = 6,
    OperatingCycleProposed = 7,
    OperatingCycleStateChanged = 8,
    RootAuthorityOfficeSessionStarted = 9,
    RootAuthorityOfficeSessionStateChanged = 10,
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
    DeterministicExperimentFinalized = 61,
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
    NativeChildSpawnInvalidated = 79,
    PiOfficeTurnPromptAuthorized = 80,
    PiOfficeTurnPromptDelivered = 81,
    PiOfficeTurnPromptAccepted = 82,
    PiOfficeTurnUsageRecorded = 83,
    PiOfficeTurnUsageFrozen = 84,
    PiOfficeTurnTerminalRecorded = 85,
    PiOfficeSessionDisposeAuthorized = 86,
    PiOfficeSessionDisposeDelivered = 87,
    PiOfficeSessionDisposeAccepted = 88,
    PiOfficeSessionDisposeUsageRecorded = 89,
    PiOfficeSessionDisposeUsageFrozen = 90,
    PiOfficeSessionDisposed = 91,
    DeterministicEvaluatorNativeChildAdmitted = 92,
    DeterministicEvaluatorNativeChildSpawnRecorded = 93,
    DeterministicEvaluatorForensicManifestRegistered = 94,
    StudyTransition = 95,
}

impl EventBody {
    pub const fn kind(&self) -> EventKind {
        match self {
            Self::SocietyIdentityCreated { .. } => EventKind::SocietyIdentityCreated,
            Self::RootAuthorityOfficeInstalled { .. } => EventKind::RootAuthorityOfficeInstalled,
            Self::FoundingMissionInstalled { .. } => EventKind::FoundingMissionInstalled,
            Self::RootAuthorityAppointed { .. } => EventKind::RootAuthorityAppointed,
            Self::R0HardCeilingSet { .. } => EventKind::R0HardCeilingSet,
            Self::SocietyBootstrapped { .. } => EventKind::SocietyBootstrapped,
            Self::OperatingCycleProposed { .. } => EventKind::OperatingCycleProposed,
            Self::OperatingCycleStateChanged { .. } => EventKind::OperatingCycleStateChanged,
            Self::RootAuthorityOfficeSessionStarted { .. } => {
                EventKind::RootAuthorityOfficeSessionStarted
            }
            Self::RootAuthorityOfficeSessionStateChanged { .. } => {
                EventKind::RootAuthorityOfficeSessionStateChanged
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
            Self::DeterministicEvaluatorForensicManifestRegistered { .. } => {
                EventKind::DeterministicEvaluatorForensicManifestRegistered
            }
            Self::DeterministicExperimentRegistered { .. } => {
                EventKind::DeterministicExperimentRegistered
            }
            Self::DeterministicEvaluationReceiptRecorded { .. } => {
                EventKind::DeterministicEvaluationReceiptRecorded
            }
            Self::DeterministicEvidenceAdmitted { .. } => EventKind::DeterministicEvidenceAdmitted,
            Self::DeterministicExperimentFinalized { .. } => {
                EventKind::DeterministicExperimentFinalized
            }
            Self::DeterministicEvaluatorNativeChildAdmitted { .. } => {
                EventKind::DeterministicEvaluatorNativeChildAdmitted
            }
            Self::DeterministicEvaluatorNativeChildSpawnRecorded { .. } => {
                EventKind::DeterministicEvaluatorNativeChildSpawnRecorded
            }
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
            Self::NativeChildSpawnInvalidated { .. } => EventKind::NativeChildSpawnInvalidated,
            Self::PiOfficeTurnPromptAuthorized { .. } => EventKind::PiOfficeTurnPromptAuthorized,
            Self::PiOfficeTurnPromptDelivered { .. } => EventKind::PiOfficeTurnPromptDelivered,
            Self::PiOfficeTurnPromptAccepted { .. } => EventKind::PiOfficeTurnPromptAccepted,
            Self::PiOfficeTurnUsageRecorded { .. } => EventKind::PiOfficeTurnUsageRecorded,
            Self::PiOfficeTurnUsageFrozen { .. } => EventKind::PiOfficeTurnUsageFrozen,
            Self::PiOfficeTurnTerminalRecorded { .. } => EventKind::PiOfficeTurnTerminalRecorded,
            Self::PiOfficeSessionDisposeAuthorized { .. } => {
                EventKind::PiOfficeSessionDisposeAuthorized
            }
            Self::PiOfficeSessionDisposeDelivered { .. } => {
                EventKind::PiOfficeSessionDisposeDelivered
            }
            Self::PiOfficeSessionDisposeAccepted { .. } => {
                EventKind::PiOfficeSessionDisposeAccepted
            }
            Self::PiOfficeSessionDisposeUsageRecorded { .. } => {
                EventKind::PiOfficeSessionDisposeUsageRecorded
            }
            Self::PiOfficeSessionDisposeUsageFrozen { .. } => {
                EventKind::PiOfficeSessionDisposeUsageFrozen
            }
            Self::PiOfficeSessionDisposed { .. } => EventKind::PiOfficeSessionDisposed,
            Self::StudyTransition { .. } => EventKind::StudyTransition,
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
    #[error("application identity must use canonical ASCII application identity grammar")]
    InvalidApplicationIdentity,
    #[error("application name must be nonblank, shorter than 161 bytes, and contain no NUL")]
    InvalidApplicationName,
    #[error("principal display name must be nonblank, shorter than 161 bytes, and contain no NUL")]
    InvalidPrincipalDisplayName,
    #[error("{type_name} must be nonblank, shorter than 1025 bytes, and contain no NUL")]
    InvalidCoordinationText { type_name: &'static str },
    #[error("{type_name} must be nonblank, shorter than 4097 bytes, and contain no NUL")]
    InvalidMissionText { type_name: &'static str },
    #[error("application revision ordinal must be positive: {0}")]
    NonPositiveApplicationRevisionOrdinal(i64),
    #[error("mission principle relation must contain 1 through 16 ordered principles, got {count}")]
    InvalidMissionPrincipleCount { count: usize },
    #[error("mission source rendering must be nonempty and at most 16384 bytes")]
    InvalidMissionSourceRendering,
    #[error("{type_name} must use canonical boundary identity grammar")]
    InvalidOperationalIdentity { type_name: &'static str },
    #[error("micro-US-dollars cannot be negative: {0}")]
    NegativeUsdMicros(i64),
    #[error("Pi token count cannot be negative: {0}")]
    NegativePiTokenCount(i64),
    #[error("provider cost must be finite and nonnegative IEEE-754 binary64")]
    InvalidProviderCostBinary64,
    #[error("provider cost cannot be represented as integer micro-US-dollars")]
    ProviderCostMicroUsdOverflow,
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
