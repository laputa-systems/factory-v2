//! Staged, sealed planning material for the first paid CL-001 study.
//!
//! A plan is not a provider execution and it is not native-runtime
//! qualification.  It fixes the proposed actor distribution and the only
//! authorized pre-substantive spend, then makes the later substantive plan
//! depend on an observed pilot artifact without pooling pilot pairs into the
//! confirmatory analysis.

use std::fmt;

use society_kernel::{ActorModelPolicy, Blake3Digest, UsdMicros};
use society_pi::{NodeRuntimeVersion, PINNED_PI_SDK_VERSION};

use crate::{ActorPolicyIdentity, LiveRunPlan, PairSeed};

/// Revision of the staged CL-001 study-program contract.
pub const STUDY_PROGRAM_REVISION: &str = "cl-001-study-program-v1";

/// The only model treatment proposed for the first native CL-001 pilot.
pub const CANONICAL_LIVE_PROVIDER: &str = "openrouter";
pub const CANONICAL_LIVE_MODEL: &str = "inclusionai/ling-2.6-flash";
pub const CANONICAL_LIVE_THINKING_LEVEL: &str = "off";
pub const CANONICAL_LIVE_TOOL_PROFILE: &str = "forum_isolated_v1";

const RUNTIME_PROFILE_DOMAIN: &[u8] = b"cl-001|runtime-profile|v1";
const ACTOR_POLICY_REVISION_BYTES: &[u8] = b"cl-001|actor-policy|openrouter-ling-2.6-flash|off|v1";
const SAMPLING_CONTRACT_BYTES: &[u8] = b"cl-001|sampling|v1|max-retries=2|base-delay-ms=2000|provider-timeout-ms=300000|provider-max-retries=1|provider-max-retry-delay-ms=30000|compaction=enabled|reserve-tokens=16384|keep-recent-tokens=20000|steering=one-at-a-time|follow-up=one-at-a-time|transport=sse|project-trust=never|telemetry=off|analytics=off|images=blocked";

const AUTHORIZED_TOTAL_MICRO_USD: i64 = 250_000;
const NATIVE_PROFILE_QUALIFICATION_MICRO_USD: i64 = 10_000;
const CHEAPEST_PAID_SMOKE_MICRO_USD: i64 = 40_000;
const CHEAPEST_PAID_SMOKE_ACTOR_LIFETIMES: u16 = 16;
const PILOT_MICRO_USD: i64 = 200_000;
const PILOT_PAIR_COUNT: usize = 2;
const TREATMENT_ARMS_PER_PAIR: u16 = 2;
const ACTOR_LIFETIMES_PER_ARM: u16 = 16;
const PILOT_ACTOR_LIFETIMES: u16 =
    (PILOT_PAIR_COUNT as u16) * TREATMENT_ARMS_PER_PAIR * ACTOR_LIFETIMES_PER_ARM;
const ACTOR_LIFETIME_MICRO_USD: i64 = 3_125;
const EPISODE_MICRO_USD: i64 = ACTOR_LIFETIME_MICRO_USD * ACTOR_LIFETIMES_PER_ARM as i64;
const PAIR_MICRO_USD: i64 = EPISODE_MICRO_USD * TREATMENT_ARMS_PER_PAIR as i64;

const AUTHORIZED_TOTAL: UsdMicros = match UsdMicros::new(AUTHORIZED_TOTAL_MICRO_USD) {
    Some(value) => value,
    None => panic!("authorized total must be nonnegative"),
};
const NATIVE_PROFILE_QUALIFICATION_TOTAL: UsdMicros =
    match UsdMicros::new(NATIVE_PROFILE_QUALIFICATION_MICRO_USD) {
        Some(value) => value,
        None => panic!("native profile qualification total must be nonnegative"),
    };
const CHEAPEST_PAID_SMOKE_TOTAL: UsdMicros = match UsdMicros::new(CHEAPEST_PAID_SMOKE_MICRO_USD) {
    Some(value) => value,
    None => panic!("cheapest paid smoke total must be nonnegative"),
};
const PILOT_TOTAL: UsdMicros = match UsdMicros::new(PILOT_MICRO_USD) {
    Some(value) => value,
    None => panic!("pilot total must be nonnegative"),
};
const ACTOR_LIFETIME_TOTAL: UsdMicros = match UsdMicros::new(ACTOR_LIFETIME_MICRO_USD) {
    Some(value) => value,
    None => panic!("actor lifetime total must be nonnegative"),
};
const EPISODE_TOTAL: UsdMicros = match UsdMicros::new(EPISODE_MICRO_USD) {
    Some(value) => value,
    None => panic!("episode total must be nonnegative"),
};
const PAIR_TOTAL: UsdMicros = match UsdMicros::new(PAIR_MICRO_USD) {
    Some(value) => value,
    None => panic!("pair total must be nonnegative"),
};

/// The three stages stay distinct in all planning and reporting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StudyStage {
    NativeProfileQualification,
    CheapestPaidAdapterSmoke,
    FeasibilityPilot,
    SubstantiveStudy,
}

impl StudyStage {
    /// Stable stage label for a sealed application artifact.
    pub const fn name(self) -> &'static str {
        match self {
            Self::NativeProfileQualification => "native-profile-qualification",
            Self::CheapestPaidAdapterSmoke => "cheapest-paid-adapter-smoke",
            Self::FeasibilityPilot => "feasibility-pilot",
            Self::SubstantiveStudy => "substantive-study",
        }
    }
}

/// Exact local artifacts which make a native Pi runtime reproducible enough to
/// qualify or execute.  The actual artifact bytes remain in their respective
/// custody stores; this plan retains only their BLAKE3 identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeRuntimeArtifacts {
    node_version: NodeRuntimeVersion,
    node_executable_digest: Blake3Digest,
    pi_host_lockfile_digest: Blake3Digest,
    pi_host_build_digest: Blake3Digest,
    pi_transitive_package_set_digest: Blake3Digest,
    model_catalog_digest: Blake3Digest,
}

impl NativeRuntimeArtifacts {
    /// Create a complete runtime identity for the pinned Pi boundary.
    pub fn new(
        node_version: NodeRuntimeVersion,
        node_executable_digest: Blake3Digest,
        pi_host_lockfile_digest: Blake3Digest,
        pi_host_build_digest: Blake3Digest,
        pi_transitive_package_set_digest: Blake3Digest,
        model_catalog_digest: Blake3Digest,
    ) -> Result<Self, StudyProgramError> {
        for (digest, field) in [
            (node_executable_digest, "node executable"),
            (pi_host_lockfile_digest, "Pi host lockfile"),
            (pi_host_build_digest, "Pi host build"),
            (
                pi_transitive_package_set_digest,
                "Pi transitive package set",
            ),
            (model_catalog_digest, "model catalog"),
        ] {
            require_digest(digest, field)?;
        }
        Ok(Self {
            node_version,
            node_executable_digest,
            pi_host_lockfile_digest,
            pi_host_build_digest,
            pi_transitive_package_set_digest,
            model_catalog_digest,
        })
    }

    pub fn node_version(&self) -> &NodeRuntimeVersion {
        &self.node_version
    }

    pub const fn node_executable_digest(&self) -> Blake3Digest {
        self.node_executable_digest
    }

    pub const fn pi_host_lockfile_digest(&self) -> Blake3Digest {
        self.pi_host_lockfile_digest
    }

    pub const fn pi_host_build_digest(&self) -> Blake3Digest {
        self.pi_host_build_digest
    }

    pub const fn pi_transitive_package_set_digest(&self) -> Blake3Digest {
        self.pi_transitive_package_set_digest
    }

    pub const fn model_catalog_digest(&self) -> Blake3Digest {
        self.model_catalog_digest
    }
}

/// A fully specified candidate runtime, before its native profile is allowed
/// to execute a paid study.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalLiveRuntimeProfile {
    artifacts: NativeRuntimeArtifacts,
    actor_policy: ActorPolicyIdentity,
    sealed_digest: Blake3Digest,
}

impl CanonicalLiveRuntimeProfile {
    /// Build CL-001's only proposed native runtime treatment.
    ///
    /// This is intentionally a plan constructor, not a qualification result.
    /// The generic native execution profile remains unqualified until a
    /// trusted qualification transition exists and accepts its receipts.
    pub fn canonical(artifacts: NativeRuntimeArtifacts) -> Result<Self, StudyProgramError> {
        let mut bytes = Vec::with_capacity(512);
        bytes.extend_from_slice(RUNTIME_PROFILE_DOMAIN);
        bytes.push(0);
        put_str(&mut bytes, CANONICAL_LIVE_PROVIDER);
        put_str(&mut bytes, CANONICAL_LIVE_MODEL);
        put_str(&mut bytes, CANONICAL_LIVE_THINKING_LEVEL);
        put_str(&mut bytes, CANONICAL_LIVE_TOOL_PROFILE);
        put_str(&mut bytes, PINNED_PI_SDK_VERSION);
        put_str(&mut bytes, artifacts.node_version.as_str());
        for digest in [
            artifacts.node_executable_digest,
            artifacts.pi_host_lockfile_digest,
            artifacts.pi_host_build_digest,
            artifacts.pi_transitive_package_set_digest,
            artifacts.model_catalog_digest,
        ] {
            put_digest(&mut bytes, digest);
        }
        let sealed_digest = Blake3Digest::of_bytes(&bytes);
        let sampling_contract_digest = Blake3Digest::of_bytes(SAMPLING_CONTRACT_BYTES);
        let actor_policy = ActorPolicyIdentity::new(
            Blake3Digest::of_bytes(ACTOR_POLICY_REVISION_BYTES),
            sealed_digest,
            sampling_contract_digest,
        )
        .map_err(StudyProgramError::LivePlan)?;
        Ok(Self {
            artifacts,
            actor_policy,
            sealed_digest,
        })
    }

    /// Generic durable actor policy selected by this candidate treatment.
    pub const fn generic_actor_model_policy(&self) -> ActorModelPolicy {
        ActorModelPolicy::PinnedOpenRouterLing26FlashOff
    }

    /// Exact application actor-policy identity used by `LiveRunDescriptor`.
    pub const fn actor_policy(&self) -> ActorPolicyIdentity {
        self.actor_policy
    }

    pub const fn sealed_digest(&self) -> Blake3Digest {
        self.sealed_digest
    }

    pub const fn artifacts(&self) -> &NativeRuntimeArtifacts {
        &self.artifacts
    }
}

/// The complete $0.25 authorization, allocated before any provider call.
///
/// `$0.01` is reserved for native-profile qualification, `$0.04` for an
/// optional noncanonical adapter smoke, and `$0.20` for exactly two canonical
/// pilot pairs. Nothing here authorizes a substantive study: its pair count
/// and spend must be pre-registered later using observed pilot variance and
/// the declared precision targets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorizedPilotBudget {
    total: UsdMicros,
    native_profile_qualification: UsdMicros,
    cheapest_paid_smoke: UsdMicros,
    pilot: UsdMicros,
}

impl AuthorizedPilotBudget {
    /// The fixed spend authorization supplied for the initial study path.
    pub const fn canonical() -> Self {
        Self {
            total: AUTHORIZED_TOTAL,
            native_profile_qualification: NATIVE_PROFILE_QUALIFICATION_TOTAL,
            cheapest_paid_smoke: CHEAPEST_PAID_SMOKE_TOTAL,
            pilot: PILOT_TOTAL,
        }
    }

    pub const fn total(self) -> UsdMicros {
        self.total
    }

    pub const fn native_profile_qualification(self) -> UsdMicros {
        self.native_profile_qualification
    }

    pub const fn cheapest_paid_smoke(self) -> UsdMicros {
        self.cheapest_paid_smoke
    }

    pub const fn pilot(self) -> UsdMicros {
        self.pilot
    }

    pub const fn cheapest_paid_smoke_actor_lifetimes(self) -> u16 {
        CHEAPEST_PAID_SMOKE_ACTOR_LIFETIMES
    }

    pub const fn pilot_pair_count(self) -> usize {
        PILOT_PAIR_COUNT
    }

    pub const fn pilot_actor_lifetimes(self) -> u16 {
        PILOT_ACTOR_LIFETIMES
    }

    pub const fn actor_lifetime_cap(self) -> UsdMicros {
        ACTOR_LIFETIME_TOTAL
    }

    pub const fn episode_cap(self) -> UsdMicros {
        EPISODE_TOTAL
    }

    pub const fn pair_cap(self) -> UsdMicros {
        PAIR_TOTAL
    }
}

/// The exact two-pair feasibility pilot, held separate from a later
/// substantive study. The plan cannot be created under another actor policy
/// or with a partial/more-expensive pilot topology.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeasibilityPilotPlan {
    runtime: CanonicalLiveRuntimeProfile,
    budget: AuthorizedPilotBudget,
    live_plan: LiveRunPlan,
    sealed_digest: Blake3Digest,
}

impl FeasibilityPilotPlan {
    pub fn new(
        runtime: CanonicalLiveRuntimeProfile,
        live_plan: LiveRunPlan,
    ) -> Result<Self, StudyProgramError> {
        if live_plan.pairs().len() != PILOT_PAIR_COUNT {
            return Err(StudyProgramError::PilotPairCount);
        }
        if live_plan.descriptor().actor_policy() != runtime.actor_policy() {
            return Err(StudyProgramError::PilotRuntimeMismatch);
        }
        let budget = AuthorizedPilotBudget::canonical();
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(STUDY_PROGRAM_REVISION.as_bytes());
        bytes.extend_from_slice(b"|stage|");
        bytes.extend_from_slice(StudyStage::FeasibilityPilot.name().as_bytes());
        put_digest(&mut bytes, runtime.sealed_digest());
        put_i64(&mut bytes, budget.pilot().value());
        put_digest(&mut bytes, live_plan.sealed_digest());
        let sealed_digest = Blake3Digest::of_bytes(&bytes);
        Ok(Self {
            runtime,
            budget,
            live_plan,
            sealed_digest,
        })
    }

    pub const fn runtime(&self) -> &CanonicalLiveRuntimeProfile {
        &self.runtime
    }

    pub const fn budget(&self) -> AuthorizedPilotBudget {
        self.budget
    }

    pub const fn live_plan(&self) -> &LiveRunPlan {
        &self.live_plan
    }

    pub const fn sealed_digest(&self) -> Blake3Digest {
        self.sealed_digest
    }
}

/// Application-side identity of the completed pilot analysis artifact that
/// fixes the design inputs for a later substantive study. The caller must
/// derive this from the closed pilot's trusted observations; a digest alone is
/// not a qualification receipt or a result claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PilotAnalysisArtifactReference {
    pilot_plan_digest: Blake3Digest,
    analysis_artifact_digest: Blake3Digest,
}

impl PilotAnalysisArtifactReference {
    pub fn new(
        pilot: &FeasibilityPilotPlan,
        analysis_artifact_digest: Blake3Digest,
    ) -> Result<Self, StudyProgramError> {
        require_digest(analysis_artifact_digest, "pilot analysis artifact")?;
        Ok(Self {
            pilot_plan_digest: pilot.sealed_digest(),
            analysis_artifact_digest,
        })
    }

    pub const fn pilot_plan_digest(self) -> Blake3Digest {
        self.pilot_plan_digest
    }

    pub const fn analysis_artifact_digest(self) -> Blake3Digest {
        self.analysis_artifact_digest
    }
}

/// A future confirmatory plan derived after the pilot. It deliberately has no
/// spend ceiling because the $0.25 authorization ends at the feasibility
/// pilot. The plan's finite pair count and precision targets belong in its
/// embedded `LiveRunPlan` and must be fixed after pilot analysis, before any
/// new outcomes are visible.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubstantiveStudyPlan {
    runtime: CanonicalLiveRuntimeProfile,
    pilot_reference: PilotAnalysisArtifactReference,
    live_plan: LiveRunPlan,
    sealed_digest: Blake3Digest,
}

impl SubstantiveStudyPlan {
    pub fn new(
        pilot: &FeasibilityPilotPlan,
        pilot_reference: PilotAnalysisArtifactReference,
        live_plan: LiveRunPlan,
    ) -> Result<Self, StudyProgramError> {
        if pilot_reference.pilot_plan_digest != pilot.sealed_digest() {
            return Err(StudyProgramError::PilotReferenceMismatch);
        }
        if live_plan.descriptor().actor_policy() != pilot.runtime().actor_policy() {
            return Err(StudyProgramError::SubstantiveRuntimeMismatch);
        }
        reject_pilot_pair_reuse(pilot.live_plan().pairs(), live_plan.pairs())?;
        let mut bytes = Vec::with_capacity(160);
        bytes.extend_from_slice(STUDY_PROGRAM_REVISION.as_bytes());
        bytes.extend_from_slice(b"|stage|");
        bytes.extend_from_slice(StudyStage::SubstantiveStudy.name().as_bytes());
        put_digest(&mut bytes, pilot.runtime().sealed_digest());
        put_digest(&mut bytes, pilot_reference.pilot_plan_digest());
        put_digest(&mut bytes, pilot_reference.analysis_artifact_digest());
        put_digest(&mut bytes, live_plan.sealed_digest());
        let sealed_digest = Blake3Digest::of_bytes(&bytes);
        Ok(Self {
            runtime: pilot.runtime().clone(),
            pilot_reference,
            live_plan,
            sealed_digest,
        })
    }

    pub const fn runtime(&self) -> &CanonicalLiveRuntimeProfile {
        &self.runtime
    }

    pub const fn pilot_reference(&self) -> PilotAnalysisArtifactReference {
        self.pilot_reference
    }

    pub const fn live_plan(&self) -> &LiveRunPlan {
        &self.live_plan
    }

    pub const fn sealed_digest(&self) -> Blake3Digest {
        self.sealed_digest
    }
}

fn reject_pilot_pair_reuse(
    pilot_pairs: &[PairSeed],
    substantive_pairs: &[PairSeed],
) -> Result<(), StudyProgramError> {
    for candidate in substantive_pairs {
        if pilot_pairs
            .iter()
            .any(|pilot| pilot.pair_id() == candidate.pair_id())
        {
            return Err(StudyProgramError::SubstantiveReusesPilotPairId);
        }
        if pilot_pairs
            .iter()
            .any(|pilot| pilot.seed_digest() == candidate.seed_digest())
        {
            return Err(StudyProgramError::SubstantiveReusesPilotWorldSeed);
        }
    }
    Ok(())
}

fn require_digest(digest: Blake3Digest, field: &'static str) -> Result<(), StudyProgramError> {
    if digest.as_bytes().iter().all(|byte| *byte == 0) {
        Err(StudyProgramError::ZeroDigest(field))
    } else {
        Ok(())
    }
}

fn put_digest(bytes: &mut Vec<u8>, digest: Blake3Digest) {
    bytes.extend_from_slice(&digest.as_bytes());
}

fn put_str(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn put_i64(bytes: &mut Vec<u8>, value: i64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

/// Rejection while constructing sealed staged-study material.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StudyProgramError {
    ZeroDigest(&'static str),
    LivePlan(crate::LivePlanError),
    PilotPairCount,
    PilotRuntimeMismatch,
    PilotReferenceMismatch,
    SubstantiveRuntimeMismatch,
    SubstantiveReusesPilotPairId,
    SubstantiveReusesPilotWorldSeed,
}

impl fmt::Display for StudyProgramError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDigest(field) => write!(formatter, "{field} digest must not be all zero"),
            Self::LivePlan(error) => write!(formatter, "live CL-001 plan rejected: {error}"),
            Self::PilotPairCount => {
                formatter.write_str("the feasibility pilot requires exactly two pairs")
            }
            Self::PilotRuntimeMismatch => {
                formatter.write_str("pilot plan does not use the sealed runtime actor policy")
            }
            Self::PilotReferenceMismatch => {
                formatter.write_str("pilot analysis reference belongs to another pilot plan")
            }
            Self::SubstantiveRuntimeMismatch => formatter
                .write_str("substantive plan does not use the pilot's sealed runtime actor policy"),
            Self::SubstantiveReusesPilotPairId => {
                formatter.write_str("substantive plan reuses a pilot pair identity")
            }
            Self::SubstantiveReusesPilotWorldSeed => {
                formatter.write_str("substantive plan reuses a pilot world seed")
            }
        }
    }
}

impl std::error::Error for StudyProgramError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cl001Metric, LiveRunDescriptor, PrecisionTarget};

    fn digest(label: &str) -> Blake3Digest {
        Blake3Digest::of_bytes(label.as_bytes())
    }

    fn runtime(label: &str) -> CanonicalLiveRuntimeProfile {
        let artifacts = NativeRuntimeArtifacts::new(
            NodeRuntimeVersion::parse("v26.5.0").unwrap(),
            digest(&format!("{label}-node")),
            digest(&format!("{label}-lockfile")),
            digest(&format!("{label}-build")),
            digest(&format!("{label}-package-set")),
            digest(&format!("{label}-catalog")),
        )
        .unwrap();
        CanonicalLiveRuntimeProfile::canonical(artifacts).unwrap()
    }

    fn precision() -> [PrecisionTarget; Cl001Metric::ALL.len()] {
        [PrecisionTarget::new(100).unwrap(); Cl001Metric::ALL.len()]
    }

    fn plan(runtime: &CanonicalLiveRuntimeProfile, names: &[(&str, &str)]) -> LiveRunPlan {
        LiveRunPlan::new(
            LiveRunDescriptor::canonical(runtime.actor_policy()).unwrap(),
            names
                .iter()
                .map(|(pair, seed)| PairSeed::new(pair, digest(seed)).unwrap())
                .collect(),
            precision(),
        )
        .unwrap()
    }

    #[test]
    fn authorized_budget_is_exactly_partitioned_without_rounding() {
        let budget = AuthorizedPilotBudget::canonical();
        assert_eq!(budget.total().value(), 250_000);
        assert_eq!(budget.native_profile_qualification().value(), 10_000);
        assert_eq!(budget.cheapest_paid_smoke().value(), 40_000);
        assert_eq!(budget.pilot().value(), 200_000);
        assert_eq!(
            budget
                .native_profile_qualification()
                .checked_add(budget.cheapest_paid_smoke())
                .and_then(|pre_pilot| pre_pilot.checked_add(budget.pilot())),
            Some(budget.total())
        );
        assert_eq!(budget.cheapest_paid_smoke_actor_lifetimes(), 16);
        assert_eq!(budget.pilot_pair_count(), 2);
        assert_eq!(budget.pilot_actor_lifetimes(), 64);
        assert_eq!(budget.actor_lifetime_cap().value(), 3_125);
        assert_eq!(budget.episode_cap().value(), 50_000);
        assert_eq!(budget.pair_cap().value(), 100_000);
        assert_eq!(
            budget.actor_lifetime_cap().value() * i64::from(budget.pilot_actor_lifetimes()),
            budget.pilot().value()
        );
    }

    #[test]
    fn runtime_profile_binds_every_runtime_artifact_and_the_ling_policy() {
        let one = runtime("one");
        let two = runtime("two");
        assert_eq!(
            one.generic_actor_model_policy(),
            ActorModelPolicy::PinnedOpenRouterLing26FlashOff
        );
        assert_eq!(one.artifacts().node_version().as_str(), "v26.5.0");
        assert_ne!(one.sealed_digest(), two.sealed_digest());
        assert_ne!(
            one.actor_policy().model_runtime_digest(),
            two.actor_policy().model_runtime_digest()
        );
    }

    #[test]
    fn pilot_is_exactly_two_pairs_and_uses_its_pinned_runtime() {
        let profile = runtime("pilot");
        let pilot = FeasibilityPilotPlan::new(
            profile.clone(),
            plan(
                &profile,
                &[("pilot-01", "pilot-seed-01"), ("pilot-02", "pilot-seed-02")],
            ),
        )
        .unwrap();
        assert_eq!(pilot.live_plan().pairs().len(), 2);
        assert_eq!(pilot.budget(), AuthorizedPilotBudget::canonical());

        let invalid = plan(
            &profile,
            &[("only-one", "only-one-seed"), ("two", "two-seed")],
        );
        let other_runtime = runtime("other");
        assert_eq!(
            FeasibilityPilotPlan::new(other_runtime, invalid),
            Err(StudyProgramError::PilotRuntimeMismatch)
        );
    }

    #[test]
    fn substantive_plan_requires_pilot_analysis_and_fresh_pairs() {
        let runtime = runtime("pilot");
        let pilot = FeasibilityPilotPlan::new(
            runtime.clone(),
            plan(
                &runtime,
                &[("pilot-01", "pilot-seed-01"), ("pilot-02", "pilot-seed-02")],
            ),
        )
        .unwrap();
        let reference =
            PilotAnalysisArtifactReference::new(&pilot, digest("pilot-analysis")).unwrap();
        let substantive = SubstantiveStudyPlan::new(
            &pilot,
            reference,
            plan(
                &runtime,
                &[
                    ("substantive-01", "substantive-seed-01"),
                    ("substantive-02", "substantive-seed-02"),
                ],
            ),
        )
        .unwrap();
        assert_ne!(substantive.sealed_digest(), pilot.sealed_digest());

        assert_eq!(
            SubstantiveStudyPlan::new(
                &pilot,
                reference,
                plan(
                    &runtime,
                    &[
                        ("pilot-01", "fresh-seed"),
                        ("substantive-02", "substantive-seed-02")
                    ],
                ),
            ),
            Err(StudyProgramError::SubstantiveReusesPilotPairId)
        );
        assert_eq!(
            SubstantiveStudyPlan::new(
                &pilot,
                reference,
                plan(
                    &runtime,
                    &[
                        ("substantive-01", "pilot-seed-01"),
                        ("substantive-02", "substantive-seed-02")
                    ],
                ),
            ),
            Err(StudyProgramError::SubstantiveReusesPilotWorldSeed)
        );
    }
}
