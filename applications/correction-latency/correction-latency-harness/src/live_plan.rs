//! Sealed, application-owned admission material for a live CL-001 run.
//!
//! The daemon must not learn CL-001's world semantics.  This module therefore
//! stops at a typed, digest-bound descriptor: an application adapter seals
//! its canonical bytes through generic immutable content custody, then a
//! generic runner may retain only the resulting content identity and digest.
//! The application adapter interprets the role seats and emits the generic
//! study transitions. No database handle, daemon type, provider client, or
//! executable path crosses this boundary.

use std::fmt;

use correction_latency_world::{
    canonical_role_prompt_revision_digest, canonical_role_specifications,
    canonical_role_topology_digest, PrivateViewKind, RoleKind, RoleOrdinal, ROLE_COUNT,
};
use society_kernel::{
    forum_f0_awareness_digest, forum_f0_tool_contract_digest, Blake3Digest,
    StudyPairObservation as PersistedStudyPairObservation, StudyRunObservation,
};

use crate::{
    AnalysisArtifact, AnalysisEstimand, AnalysisExclusionPolicy, AnalysisInputError,
    AnalysisPairId, Cl001Metric, PrecisionTarget, PreregisteredAnalysisPlan,
};

/// Stable revision of the CL-001 live admission descriptor.
pub const LIVE_PLAN_REVISION: &str = "cl-001-live-plan-v1";

/// The exact canonical protocol identity admitted by the provider-free
/// harness and by the future daemon-owned live runner.
pub const PROTOCOL_BYTES: &[u8] = b"cl-001|protocol|v1";

/// The exact actor-population identity.  A fresh snapshot gets a new generic
/// ledger identity, but every snapshot must retain this application digest.
pub const POPULATION_BYTES: &[u8] = b"cl-001|fixed-eight-role-population|v1";

/// The exact F0 Forum charter identity.
pub const FORUM_CHARTER_BYTES: &[u8] = b"cl-001|forum-charter|f0|v1";

/// The exact institution identity for the F0 baseline.
pub const INSTITUTION_BYTES: &[u8] = b"cl-001|forum-f0|v1";

/// The exact analysis contract identity admitted alongside the eleven slots.
pub const ANALYSIS_CONTRACT_BYTES: &[u8] = b"cl-001|analysis|v1";

const ACTOR_POLICY_DOMAIN: &[u8] = b"cl-001|actor-policy|live-v1";
const ISOLATED_BASELINE_BYTES: &[u8] = b"cl-001|baseline|isolated-private-view|v1";
const UNSTRUCTURED_BASELINE_BYTES: &[u8] = b"cl-001|baseline|unstructured-ephemeral-exchange|v1";
const RESET_BASELINE_BYTES: &[u8] = b"cl-001|baseline|reset-forum-exposure|v1";

const SOURCE_PHASE_TAG: u8 = 1;
const SUCCESSOR_PHASE_TAG: u8 = 2;
const BASELINE_COUNT: usize = 3;
const MIN_PAIR_COUNT: usize = 2;
const MAX_PAIR_COUNT: usize = 10_000;
const MEASUREMENT_SLOT_COUNT: u8 = 11;
const EPISODE_POPULATION_SIZE: u8 = ROLE_COUNT as u8;
const ACTOR_BUDGET_UNITS: u64 = 2;
const FORUM_READ_BUDGET: u64 = 4;
const FORUM_POST_BUDGET: u64 = 1;
const EPISODE_BUDGET_UNITS: u64 = ACTOR_BUDGET_UNITS * EPISODE_POPULATION_SIZE as u64 * 2;

/// An independently pre-registered live weak-actor policy/runtime identity.
///
/// The application does not inspect or select a provider from these values.
/// It requires the caller to supply digests for the exact policy revision,
/// model/runtime profile, and sampling contract selected before outcomes are
/// visible.  Their combined digest is what belongs in the generic protocol
/// revision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActorPolicyIdentity {
    policy_revision_digest: Blake3Digest,
    model_runtime_digest: Blake3Digest,
    sampling_contract_digest: Blake3Digest,
}

impl ActorPolicyIdentity {
    /// Construct a policy identity from three independently pinned revisions.
    pub fn new(
        policy_revision_digest: Blake3Digest,
        model_runtime_digest: Blake3Digest,
        sampling_contract_digest: Blake3Digest,
    ) -> Result<Self, LivePlanError> {
        require_digest(policy_revision_digest, "actor policy revision")?;
        require_digest(model_runtime_digest, "model runtime profile")?;
        require_digest(sampling_contract_digest, "sampling contract")?;
        Ok(Self {
            policy_revision_digest,
            model_runtime_digest,
            sampling_contract_digest,
        })
    }

    /// Exact policy-revision identity.
    pub const fn policy_revision_digest(self) -> Blake3Digest {
        self.policy_revision_digest
    }

    /// Exact provider/model/runtime profile identity.
    pub const fn model_runtime_digest(self) -> Blake3Digest {
        self.model_runtime_digest
    }

    /// Exact sampling/decoding contract identity.
    pub const fn sampling_contract_digest(self) -> Blake3Digest {
        self.sampling_contract_digest
    }

    /// Combined actor-policy identity committed to the protocol revision.
    pub fn digest(self) -> Blake3Digest {
        let mut bytes = Vec::with_capacity(ACTOR_POLICY_DOMAIN.len() + 96);
        bytes.extend_from_slice(ACTOR_POLICY_DOMAIN);
        put_digest(&mut bytes, self.policy_revision_digest);
        put_digest(&mut bytes, self.model_runtime_digest);
        put_digest(&mut bytes, self.sampling_contract_digest);
        Blake3Digest::of_bytes(&bytes)
    }
}

/// Integer resource ceilings shared by all CL-001 arms and phases.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceBudgetContract {
    actor_budget_units: u64,
    episode_budget_units: u64,
    forum_read_budget: u64,
    forum_post_budget: u64,
}

impl ResourceBudgetContract {
    /// The canonical CL-001 budget: two units per actor, four reads, one post.
    pub const fn canonical() -> Self {
        Self {
            actor_budget_units: ACTOR_BUDGET_UNITS,
            episode_budget_units: EPISODE_BUDGET_UNITS,
            forum_read_budget: FORUM_READ_BUDGET,
            forum_post_budget: FORUM_POST_BUDGET,
        }
    }

    pub const fn actor_budget_units(self) -> u64 {
        self.actor_budget_units
    }

    pub const fn episode_budget_units(self) -> u64 {
        self.episode_budget_units
    }

    pub const fn forum_read_budget(self) -> u64 {
        self.forum_read_budget
    }

    pub const fn forum_post_budget(self) -> u64 {
        self.forum_post_budget
    }
}

/// Source/successor role contract carried in the sealed descriptor.
///
/// `private_view_digest` identifies the exact application-owned private view;
/// it deliberately does not carry the private card bytes into generic daemon
/// state.  The application adapter resolves the digest to the actor prompt at
/// its own trusted boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActorSeatContract {
    phase: PopulationPhase,
    ordinal: RoleOrdinal,
    role: RoleKind,
    private_view: PrivateViewKind,
    private_view_digest: Blake3Digest,
    role_prompt_digest: Blake3Digest,
    forum_prompt_digest: Blake3Digest,
    forum_tool_digest: Blake3Digest,
    budget: ResourceBudgetContract,
}

impl ActorSeatContract {
    pub const fn phase(self) -> PopulationPhase {
        self.phase
    }

    pub const fn ordinal(self) -> RoleOrdinal {
        self.ordinal
    }

    pub const fn role(self) -> RoleKind {
        self.role
    }

    pub const fn private_view(self) -> PrivateViewKind {
        self.private_view
    }

    pub const fn private_view_digest(self) -> Blake3Digest {
        self.private_view_digest
    }

    pub const fn role_prompt_digest(self) -> Blake3Digest {
        self.role_prompt_digest
    }

    pub const fn forum_prompt_digest(self) -> Blake3Digest {
        self.forum_prompt_digest
    }

    pub const fn forum_tool_digest(self) -> Blake3Digest {
        self.forum_tool_digest
    }

    pub const fn budget(self) -> ResourceBudgetContract {
        self.budget
    }
}

/// Population phase in the application-owned admission contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PopulationPhase {
    Source,
    Successor,
}

impl PopulationPhase {
    const fn tag(self) -> u8 {
        match self {
            Self::Source => SOURCE_PHASE_TAG,
            Self::Successor => SUCCESSOR_PHASE_TAG,
        }
    }
}

/// The three named CL-001 baseline identities.  Baselines are retained as
/// separate analysis strata and are never pooled with the retained/reset
/// matched estimand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BaselineKind {
    IsolatedPrivateView,
    UnstructuredEphemeralExchange,
    ResetForumExposure,
}

/// The only two CL-001 treatment arms.  The matched pair differs only in
/// successor Forum exposure: retained history versus reset frontier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TreatmentArm {
    Retained,
    Reset,
}

/// Domain-qualified name for [`TreatmentArm`].
pub type Cl001TreatmentArm = TreatmentArm;

impl TreatmentArm {
    const fn tag(self) -> u8 {
        match self {
            Self::Retained => 1,
            Self::Reset => 2,
        }
    }

    /// Stable arm name used by the application adapter and report.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Retained => "retained",
            Self::Reset => "reset",
        }
    }
}

impl BaselineKind {
    /// Stable baseline identity committed before execution.
    pub fn identity(self) -> Blake3Digest {
        Blake3Digest::of_bytes(match self {
            Self::IsolatedPrivateView => ISOLATED_BASELINE_BYTES,
            Self::UnstructuredEphemeralExchange => UNSTRUCTURED_BASELINE_BYTES,
            Self::ResetForumExposure => RESET_BASELINE_BYTES,
        })
    }

    /// Stable machine-readable baseline name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::IsolatedPrivateView => "isolated-private-view",
            Self::UnstructuredEphemeralExchange => "unstructured-ephemeral-exchange",
            Self::ResetForumExposure => "reset-forum-exposure",
        }
    }

    /// All baselines in pre-registered order.
    pub const fn all() -> [Self; BASELINE_COUNT] {
        [
            Self::IsolatedPrivateView,
            Self::UnstructuredEphemeralExchange,
            Self::ResetForumExposure,
        ]
    }
}

/// The exact fixed CL-001 contract shared by every independent matched pair.
///
/// This type has no public fields and can only be built from the canonical
/// application fixture.  A generic daemon may store its sealed bytes, digest,
/// and pair count without importing this crate; the live application adapter
/// uses the accessors to submit generic admissions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveRunDescriptor {
    actor_policy: ActorPolicyIdentity,
    protocol_digest: Blake3Digest,
    world_digest: Blake3Digest,
    evidence_digest: Blake3Digest,
    false_claim_digest: Blake3Digest,
    correction_digest: Blake3Digest,
    ground_truth_commitment_digest: Blake3Digest,
    role_topology_digest: Blake3Digest,
    role_prompt_revision_digest: Blake3Digest,
    forum_prompt_digest: Blake3Digest,
    forum_tool_digest: Blake3Digest,
    institution_digest: Blake3Digest,
    forum_charter_digest: Blake3Digest,
    population_digest: Blake3Digest,
    analysis_digest: Blake3Digest,
    measurement_slot_count: u8,
    population_size: u8,
    budget: ResourceBudgetContract,
    treatment_arms: [TreatmentArm; 2],
    source_roles: [ActorSeatContract; ROLE_COUNT],
    successor_roles: [ActorSeatContract; ROLE_COUNT],
    baseline_identities: [Blake3Digest; BASELINE_COUNT],
    sealed_digest: Blake3Digest,
}

impl LiveRunDescriptor {
    /// Build the only supported CL-001 live world: canonical world, F0
    /// Forum, eight-role topology, and fixed budget.  Provider/runtime
    /// choices remain caller-supplied digests and are sealed into the result.
    pub fn canonical(actor_policy: ActorPolicyIdentity) -> Result<Self, LivePlanError> {
        let fixture = correction_latency_world::WorldFixture::canonical();
        let budget = ResourceBudgetContract::canonical();
        let source_roles = canonical_seats(PopulationPhase::Source, &fixture, budget)?;
        let successor_roles = canonical_seats(PopulationPhase::Successor, &fixture, budget)?;
        let descriptor = Self {
            actor_policy,
            protocol_digest: Blake3Digest::of_bytes(PROTOCOL_BYTES),
            world_digest: fixture.identity(),
            evidence_digest: fixture.evidence().identity(),
            false_claim_digest: fixture.false_claim().digest(),
            correction_digest: fixture.correction_package().digest(),
            ground_truth_commitment_digest: fixture.analysis_ground_truth_reveal().digest(),
            role_topology_digest: canonical_role_topology_digest(),
            role_prompt_revision_digest: canonical_role_prompt_revision_digest(),
            forum_prompt_digest: forum_f0_awareness_digest(),
            forum_tool_digest: forum_f0_tool_contract_digest(),
            institution_digest: Blake3Digest::of_bytes(INSTITUTION_BYTES),
            forum_charter_digest: Blake3Digest::of_bytes(FORUM_CHARTER_BYTES),
            population_digest: Blake3Digest::of_bytes(POPULATION_BYTES),
            analysis_digest: Blake3Digest::of_bytes(ANALYSIS_CONTRACT_BYTES),
            measurement_slot_count: MEASUREMENT_SLOT_COUNT,
            population_size: EPISODE_POPULATION_SIZE,
            budget,
            treatment_arms: [TreatmentArm::Retained, TreatmentArm::Reset],
            source_roles,
            successor_roles,
            baseline_identities: BaselineKind::all().map(BaselineKind::identity),
            sealed_digest: Blake3Digest::of_bytes(b"cl-001|unsealed-placeholder"),
        };
        let sealed_digest = Blake3Digest::of_bytes(&descriptor.canonical_bytes());
        Ok(Self {
            sealed_digest,
            ..descriptor
        })
    }

    pub const fn actor_policy(&self) -> ActorPolicyIdentity {
        self.actor_policy
    }

    pub const fn protocol_digest(&self) -> Blake3Digest {
        self.protocol_digest
    }

    pub const fn world_digest(&self) -> Blake3Digest {
        self.world_digest
    }

    pub const fn evidence_digest(&self) -> Blake3Digest {
        self.evidence_digest
    }

    pub const fn false_claim_digest(&self) -> Blake3Digest {
        self.false_claim_digest
    }

    pub const fn correction_digest(&self) -> Blake3Digest {
        self.correction_digest
    }

    pub const fn ground_truth_commitment_digest(&self) -> Blake3Digest {
        self.ground_truth_commitment_digest
    }

    pub const fn role_topology_digest(&self) -> Blake3Digest {
        self.role_topology_digest
    }

    pub const fn role_prompt_revision_digest(&self) -> Blake3Digest {
        self.role_prompt_revision_digest
    }

    pub const fn forum_prompt_digest(&self) -> Blake3Digest {
        self.forum_prompt_digest
    }

    pub const fn forum_tool_digest(&self) -> Blake3Digest {
        self.forum_tool_digest
    }

    pub const fn institution_digest(&self) -> Blake3Digest {
        self.institution_digest
    }

    pub const fn forum_charter_digest(&self) -> Blake3Digest {
        self.forum_charter_digest
    }

    pub const fn population_digest(&self) -> Blake3Digest {
        self.population_digest
    }

    pub const fn analysis_digest(&self) -> Blake3Digest {
        self.analysis_digest
    }

    pub const fn measurement_slot_count(&self) -> u8 {
        self.measurement_slot_count
    }

    pub const fn population_size(&self) -> u8 {
        self.population_size
    }

    pub const fn budget(&self) -> ResourceBudgetContract {
        self.budget
    }

    pub const fn treatment_arms(&self) -> &[TreatmentArm; 2] {
        &self.treatment_arms
    }

    pub const fn source_roles(&self) -> &[ActorSeatContract; ROLE_COUNT] {
        &self.source_roles
    }

    pub const fn successor_roles(&self) -> &[ActorSeatContract; ROLE_COUNT] {
        &self.successor_roles
    }

    pub const fn baseline_identities(&self) -> &[Blake3Digest; BASELINE_COUNT] {
        &self.baseline_identities
    }

    /// Digest of all common contract fields and role seats.
    pub const fn sealed_digest(&self) -> Blake3Digest {
        self.sealed_digest
    }

    /// Canonical non-JSON application bytes for immutable content custody.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(2_048);
        bytes.extend_from_slice(LIVE_PLAN_REVISION.as_bytes());
        bytes.push(0);
        put_digest(&mut bytes, self.actor_policy.digest());
        for digest in [
            self.protocol_digest,
            self.world_digest,
            self.evidence_digest,
            self.false_claim_digest,
            self.correction_digest,
            self.ground_truth_commitment_digest,
            self.role_topology_digest,
            self.role_prompt_revision_digest,
            self.forum_prompt_digest,
            self.forum_tool_digest,
            self.institution_digest,
            self.forum_charter_digest,
            self.population_digest,
            self.analysis_digest,
        ] {
            put_digest(&mut bytes, digest);
        }
        bytes.push(self.measurement_slot_count);
        bytes.push(self.population_size);
        put_u64(&mut bytes, self.budget.actor_budget_units);
        put_u64(&mut bytes, self.budget.episode_budget_units);
        put_u64(&mut bytes, self.budget.forum_read_budget);
        put_u64(&mut bytes, self.budget.forum_post_budget);
        for arm in self.treatment_arms {
            bytes.push(arm.tag());
        }
        for seat in self.source_roles.iter().chain(self.successor_roles.iter()) {
            put_seat(&mut bytes, seat);
        }
        for identity in self.baseline_identities {
            put_digest(&mut bytes, identity);
        }
        bytes
    }
}

/// One pair identity and one independent randomization/world seed.  The
/// randomization digest is intentionally not generated from pair position;
/// callers must provide independently generated, pre-registered material.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairSeed {
    pair_id: AnalysisPairId,
    seed_digest: Blake3Digest,
}

impl PairSeed {
    pub fn new(pair_id: &str, seed_digest: Blake3Digest) -> Result<Self, LivePlanError> {
        let pair_id = AnalysisPairId::parse(pair_id).map_err(|_| LivePlanError::InvalidPairId)?;
        require_digest(seed_digest, "world seed")?;
        Ok(Self {
            pair_id,
            seed_digest,
        })
    }

    pub fn pair_id(&self) -> &AnalysisPairId {
        &self.pair_id
    }

    pub fn seed_digest(&self) -> Blake3Digest {
        self.seed_digest
    }
}

/// A complete, pre-registered CL-001 live run.
///
/// The plan owns at least two independent pairs so that an interval is
/// mathematically defined. Its canonical bytes are application material for
/// immutable content custody; no generic daemon or database dependency is
/// required to construct it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveRunPlan {
    descriptor: LiveRunDescriptor,
    pairs: Vec<PairSeed>,
    analysis: PreregisteredAnalysisPlan,
    sealed_digest: Blake3Digest,
}

/// Domain-qualified name for [`LiveRunDescriptor`].
pub type Cl001LiveRunDescriptor = LiveRunDescriptor;

/// Domain-qualified name for [`LiveRunPlan`].
pub type Cl001LiveRunPlan = LiveRunPlan;

/// Domain-qualified name for [`PairSeed`].
pub type Cl001PairSeed = PairSeed;

/// Domain-qualified name for [`ActorPolicyIdentity`].
pub type Cl001ActorPolicyIdentity = ActorPolicyIdentity;

impl LiveRunPlan {
    pub fn new(
        descriptor: LiveRunDescriptor,
        pairs: Vec<PairSeed>,
        precision_targets: [PrecisionTarget; Cl001Metric::ALL.len()],
    ) -> Result<Self, LivePlanError> {
        if !(MIN_PAIR_COUNT..=MAX_PAIR_COUNT).contains(&pairs.len()) {
            return Err(LivePlanError::InvalidPairCount);
        }
        for (index, pair) in pairs.iter().enumerate() {
            if pairs[..index]
                .iter()
                .any(|prior| prior.pair_id == pair.pair_id)
            {
                return Err(LivePlanError::DuplicatePairId);
            }
            if pairs[..index]
                .iter()
                .any(|prior| prior.seed_digest == pair.seed_digest)
            {
                return Err(LivePlanError::DuplicateWorldSeed);
            }
        }
        let analysis = PreregisteredAnalysisPlan::new(
            pairs.iter().map(|pair| pair.pair_id.clone()).collect(),
            pairs.iter().map(PairSeed::seed_digest).collect(),
            AnalysisEstimand::RetainedMinusReset,
            AnalysisExclusionPolicy::MetricwiseCompleteCase,
            precision_targets,
        )
        .map_err(LivePlanError::AnalysisPlan)?;
        let mut bytes = descriptor.canonical_bytes();
        bytes.extend_from_slice(b"|pairs|");
        for pair in &pairs {
            put_str(&mut bytes, pair.pair_id.as_str());
            put_digest(&mut bytes, pair.seed_digest);
        }
        bytes.extend_from_slice(b"|precision|");
        for target in precision_targets {
            put_u64(&mut bytes, target.max_abs_ci95_half_width());
        }
        let sealed_digest = Blake3Digest::of_bytes(&bytes);
        Ok(Self {
            descriptor,
            pairs,
            analysis,
            sealed_digest,
        })
    }

    pub const fn descriptor(&self) -> &LiveRunDescriptor {
        &self.descriptor
    }

    pub fn pairs(&self) -> &[PairSeed] {
        &self.pairs
    }

    pub const fn analysis_plan(&self) -> &PreregisteredAnalysisPlan {
        &self.analysis
    }

    /// Render analysis only from the exact generic run that retained this
    /// complete sealed plan. Pair ordinals and seed digests alone cannot
    /// distinguish two plans that happen to share those analysis inputs but
    /// differ in fixed actor policy, prompt, resource, or precision contract.
    ///
    /// A live adapter must use this method rather than calling
    /// [`AnalysisArtifact::from_preregistered_study_run`] with a detached
    /// analysis plan. The generic daemon continues to treat these bytes as
    /// opaque content; this is application-side provenance verification.
    pub fn analysis_artifact_from_study_run(
        &self,
        study_run: &StudyRunObservation,
        persisted_pairs: Vec<PersistedStudyPairObservation>,
    ) -> Result<AnalysisArtifact, AnalysisInputError> {
        if study_run.plan_digest != self.sealed_digest {
            return Err(AnalysisInputError::StudyRunPlanDigestMismatch);
        }
        AnalysisArtifact::from_preregistered_study_run(
            self.analysis.clone(),
            study_run,
            persisted_pairs,
        )
    }

    /// Digest of the complete pre-registered plan, including descriptor,
    /// pair identities/seeds, and precision targets.
    pub const fn sealed_digest(&self) -> Blake3Digest {
        self.sealed_digest
    }

    /// Canonical application bytes to seal through immutable content custody.
    /// A generic daemon must retain the resulting content identity and digest,
    /// never an untyped application payload.
    pub fn admission_bytes(&self) -> Vec<u8> {
        let mut bytes = self.descriptor.canonical_bytes();
        bytes.extend_from_slice(b"|pairs|");
        for pair in &self.pairs {
            put_str(&mut bytes, pair.pair_id.as_str());
            put_digest(&mut bytes, pair.seed_digest);
        }
        bytes.extend_from_slice(b"|precision|");
        for target in self.analysis.precision_targets {
            put_u64(&mut bytes, target.max_abs_ci95_half_width());
        }
        bytes
    }
}

fn canonical_seats(
    phase: PopulationPhase,
    fixture: &correction_latency_world::WorldFixture,
    budget: ResourceBudgetContract,
) -> Result<[ActorSeatContract; ROLE_COUNT], LivePlanError> {
    let specifications = canonical_role_specifications();
    let mut seats = Vec::with_capacity(ROLE_COUNT);
    for specification in specifications {
        seats.push(ActorSeatContract {
            phase,
            ordinal: specification.ordinal(),
            role: specification.kind(),
            private_view: specification.private_view_kind(),
            private_view_digest: specification
                .private_view_digest(fixture)
                .map_err(|_| LivePlanError::CanonicalRoleContract)?,
            role_prompt_digest: specification.prompt_fragment().digest(),
            forum_prompt_digest: forum_f0_awareness_digest(),
            forum_tool_digest: forum_f0_tool_contract_digest(),
            budget,
        });
    }
    seats
        .try_into()
        .map_err(|_| LivePlanError::CanonicalRoleContract)
}

fn put_seat(bytes: &mut Vec<u8>, seat: &ActorSeatContract) {
    bytes.push(seat.phase.tag());
    bytes.push(seat.ordinal.value());
    bytes.push(role_tag(seat.role));
    put_private_view(bytes, seat.private_view);
    put_digest(bytes, seat.private_view_digest);
    put_digest(bytes, seat.role_prompt_digest);
    put_digest(bytes, seat.forum_prompt_digest);
    put_digest(bytes, seat.forum_tool_digest);
    put_u64(bytes, seat.budget.actor_budget_units);
    put_u64(bytes, seat.budget.forum_read_budget);
    put_u64(bytes, seat.budget.forum_post_budget);
}

fn put_private_view(bytes: &mut Vec<u8>, view: PrivateViewKind) {
    match view {
        PrivateViewKind::EvidenceCard { card_ordinal } => {
            bytes.push(1);
            bytes.push(card_ordinal);
        }
        PrivateViewKind::Forum { obligation } => {
            bytes.push(2);
            bytes.push(match obligation {
                correction_latency_world::ForumReadObligation::ChallengerOne => 1,
                correction_latency_world::ForumReadObligation::ChallengerTwo => 2,
                correction_latency_world::ForumReadObligation::Synthesis => 3,
                correction_latency_world::ForumReadObligation::Decision => 4,
            });
        }
    }
}

fn role_tag(role: RoleKind) -> u8 {
    match role {
        RoleKind::Observer => 1,
        RoleKind::Challenger => 2,
        RoleKind::Synthesizer => 3,
        RoleKind::Decision => 4,
    }
}

fn put_digest(bytes: &mut Vec<u8>, digest: Blake3Digest) {
    bytes.extend_from_slice(&digest.as_bytes());
}

fn put_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn put_str(bytes: &mut Vec<u8>, value: &str) {
    put_u64(bytes, value.len() as u64);
    bytes.extend_from_slice(value.as_bytes());
}

fn require_digest(digest: Blake3Digest, field: &'static str) -> Result<(), LivePlanError> {
    if digest.as_bytes().iter().all(|byte| *byte == 0) {
        Err(LivePlanError::ZeroDigest(field))
    } else {
        Ok(())
    }
}

/// Failure to construct a sealed live CL-001 plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LivePlanError {
    ZeroDigest(&'static str),
    InvalidPairId,
    InvalidPairCount,
    DuplicatePairId,
    DuplicateWorldSeed,
    CanonicalRoleContract,
    AnalysisPlan(crate::AnalysisInputError),
}

impl fmt::Display for LivePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDigest(field) => write!(formatter, "{field} digest must not be all zero"),
            Self::InvalidPairId => formatter.write_str("pair id is not TSV-safe"),
            Self::InvalidPairCount => write!(
                formatter,
                "live CL-001 plans require {MIN_PAIR_COUNT}..={MAX_PAIR_COUNT} independent pairs"
            ),
            Self::DuplicatePairId => formatter.write_str("live plan contains a duplicate pair id"),
            Self::DuplicateWorldSeed => {
                formatter.write_str("live plan contains a duplicate world seed")
            }
            Self::CanonicalRoleContract => {
                formatter.write_str("canonical CL-001 role contract could not be resolved")
            }
            Self::AnalysisPlan(error) => write!(formatter, "analysis plan rejected: {error}"),
        }
    }
}

impl std::error::Error for LivePlanError {}

#[cfg(test)]
mod tests {
    use super::*;
    use correction_latency_world::canonical_world_identity;

    fn policy(suffix: &'static [u8]) -> ActorPolicyIdentity {
        ActorPolicyIdentity::new(
            Blake3Digest::of_bytes(&[b'P', suffix[0]]),
            Blake3Digest::of_bytes(&[b'M', suffix[0]]),
            Blake3Digest::of_bytes(&[b'S', suffix[0]]),
        )
        .unwrap()
    }

    fn pairs() -> Vec<PairSeed> {
        vec![
            PairSeed::new("pair-01", Blake3Digest::of_bytes(b"seed-01")).unwrap(),
            PairSeed::new("pair-02", Blake3Digest::of_bytes(b"seed-02")).unwrap(),
        ]
    }

    fn precision() -> [PrecisionTarget; Cl001Metric::ALL.len()] {
        [PrecisionTarget::new(100).unwrap(); Cl001Metric::ALL.len()]
    }

    #[test]
    fn canonical_descriptor_seals_exact_roles_views_prompts_and_identities() {
        let descriptor = LiveRunDescriptor::canonical(policy(b"a")).unwrap();
        assert_eq!(descriptor.population_size(), 8);
        assert_eq!(descriptor.measurement_slot_count(), 11);
        assert_eq!(descriptor.source_roles().len(), 8);
        assert_eq!(descriptor.successor_roles().len(), 8);
        assert_eq!(
            descriptor.source_roles()[0].phase(),
            PopulationPhase::Source
        );
        assert_eq!(
            descriptor.successor_roles()[0].phase(),
            PopulationPhase::Successor
        );
        assert_eq!(
            descriptor.source_roles()[0].private_view(),
            PrivateViewKind::EvidenceCard { card_ordinal: 1 }
        );
        assert_eq!(descriptor.source_roles()[7].role(), RoleKind::Decision);
        assert_eq!(
            descriptor.role_topology_digest(),
            canonical_role_topology_digest()
        );
        assert_eq!(descriptor.world_digest(), canonical_world_identity());
        assert_eq!(
            descriptor.correction_digest(),
            correction_latency_world::WorldFixture::canonical()
                .correction_package()
                .digest()
        );
        assert_eq!(descriptor.baseline_identities().len(), BASELINE_COUNT);
        assert_ne!(
            descriptor.sealed_digest(),
            Blake3Digest::of_bytes(b"cl-001|unsealed-placeholder")
        );
        assert_eq!(
            descriptor.sealed_digest(),
            Blake3Digest::of_bytes(&descriptor.canonical_bytes())
        );
    }

    #[test]
    fn plan_requires_independent_pairs_and_preserves_preregistration() {
        let descriptor = LiveRunDescriptor::canonical(policy(b"a")).unwrap();
        let plan = LiveRunPlan::new(descriptor, pairs(), precision()).unwrap();
        assert_eq!(plan.pairs().len(), 2);
        assert_eq!(plan.analysis_plan().pair_ids.len(), 2);
        assert_eq!(plan.analysis_plan().world_seed_digests.len(), 2);
        assert_eq!(
            plan.analysis_plan().estimand,
            AnalysisEstimand::RetainedMinusReset
        );
        assert_eq!(
            plan.analysis_plan().exclusion_policy,
            AnalysisExclusionPolicy::MetricwiseCompleteCase
        );
        assert_eq!(
            plan.sealed_digest(),
            Blake3Digest::of_bytes(&plan.admission_bytes())
        );
    }

    #[test]
    fn analysis_rejects_a_run_admitted_from_a_different_sealed_plan() {
        use society_kernel::{
            ContentObjectId, StudyRunId, StudyRunLifecycleState, StudyRunObservation,
            StudyRunPairCount, StudyRunRegisteredPairCount,
        };

        let plan = LiveRunPlan::new(
            LiveRunDescriptor::canonical(policy(b"a")).unwrap(),
            pairs(),
            precision(),
        )
        .unwrap();
        let run = StudyRunObservation {
            study_run_id: StudyRunId::new(1).unwrap(),
            protocol_revision_id: society_kernel::StudyProtocolRevisionId::new(1).unwrap(),
            plan_content_object_id: ContentObjectId::new(1).unwrap(),
            plan_digest: Blake3Digest::of_bytes(b"another-sealed-cl-001-plan"),
            pair_count: StudyRunPairCount::new(2).unwrap(),
            registered_pair_count: StudyRunRegisteredPairCount::new(2).unwrap(),
            lifecycle_state: StudyRunLifecycleState::Completed,
            pairs: Vec::new(),
        };
        assert_eq!(
            plan.analysis_artifact_from_study_run(&run, Vec::new()),
            Err(AnalysisInputError::StudyRunPlanDigestMismatch)
        );
    }

    #[test]
    fn plan_rejects_duplicate_ids_seeds_and_too_few_pairs() {
        let descriptor = LiveRunDescriptor::canonical(policy(b"a")).unwrap();
        assert_eq!(
            LiveRunPlan::new(descriptor.clone(), vec![pairs()[0].clone()], precision()),
            Err(LivePlanError::InvalidPairCount)
        );
        let duplicate_id = vec![
            PairSeed::new("same", Blake3Digest::of_bytes(b"seed-01")).unwrap(),
            PairSeed::new("same", Blake3Digest::of_bytes(b"seed-02")).unwrap(),
        ];
        assert_eq!(
            LiveRunPlan::new(descriptor.clone(), duplicate_id, precision()),
            Err(LivePlanError::DuplicatePairId)
        );
        let duplicate_seed = vec![
            PairSeed::new("pair-01", Blake3Digest::of_bytes(b"same")).unwrap(),
            PairSeed::new("pair-02", Blake3Digest::of_bytes(b"same")).unwrap(),
        ];
        assert_eq!(
            LiveRunPlan::new(descriptor, duplicate_seed, precision()),
            Err(LivePlanError::DuplicateWorldSeed)
        );
    }

    #[test]
    fn changing_policy_or_seed_changes_opaque_admission_identity() {
        let one = LiveRunPlan::new(
            LiveRunDescriptor::canonical(policy(b"a")).unwrap(),
            pairs(),
            precision(),
        )
        .unwrap();
        let policy_changed = LiveRunPlan::new(
            LiveRunDescriptor::canonical(policy(b"b")).unwrap(),
            pairs(),
            precision(),
        )
        .unwrap();
        let mut changed_pairs = pairs();
        changed_pairs[1] = PairSeed::new("pair-02", Blake3Digest::of_bytes(b"seed-03")).unwrap();
        let seed_changed = LiveRunPlan::new(
            LiveRunDescriptor::canonical(policy(b"a")).unwrap(),
            changed_pairs,
            precision(),
        )
        .unwrap();
        assert_ne!(one.sealed_digest(), policy_changed.sealed_digest());
        assert_ne!(one.sealed_digest(), seed_changed.sealed_digest());
    }

    #[test]
    fn zero_policy_digest_is_rejected_before_admission() {
        let zero = Blake3Digest::from_bytes([0; 32]);
        assert_eq!(
            ActorPolicyIdentity::new(
                zero,
                Blake3Digest::of_bytes(b"model"),
                Blake3Digest::of_bytes(b"sampling")
            ),
            Err(LivePlanError::ZeroDigest("actor policy revision"))
        );
    }
}
