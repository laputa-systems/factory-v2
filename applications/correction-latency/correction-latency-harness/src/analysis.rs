//! Application-owned CL-001 analysis input and paired estimators.
//!
//! This module is deliberately outside the generic kernel.  The kernel stores
//! typed measurement results and their derivation digests; this module gives
//! the CL-001 world a closed metric vocabulary and a deterministic artifact
//! which can be inspected without teaching generic Rust or PostgreSQL about
//! this experiment.

use std::fmt;

use society_kernel::{
    Blake3Digest, StudyEpisodeId, StudyEpisodeObservation, StudyEpisodeState,
    StudyInstitutionRevisionId, StudyMeasurementRevisionId, StudyMeasurementSlotCount,
    StudyMeasurementStatus, StudyPairObservation as PersistedStudyPairObservation,
    StudyPopulationSnapshotId, StudyProtocolRevisionId, StudyRunLifecycleState,
    StudyRunObservation, StudyWorldRevisionId,
};

use crate::{ArmReport, MeasurementOutcome, PairedReport};

/// Revision of the application-owned analysis artifact and estimator.
pub const ANALYSIS_REVISION: &str = "cl-001-analysis-v1";

/// The estimand declared by the first CL-001 analysis plan. A different
/// contrast is a different analysis contract and must not be silently mixed
/// into this artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalysisEstimand {
    RetainedMinusReset,
}

/// Missingness is retained in the raw artifact and excluded independently
/// for each metric's paired estimate. This is deliberately a closed policy;
/// a live study cannot change it after outcomes are visible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalysisExclusionPolicy {
    MetricwiseCompleteCase,
}

/// The pre-registered maximum half-width of a metric's two-sided 95% interval,
/// in that metric's integer units. It is a target recorded with the plan, not
/// a post-hoc claim that the observed interval achieved it.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PrecisionTarget(u64);

impl PrecisionTarget {
    pub const fn new(max_abs_ci95_half_width: u64) -> Option<Self> {
        if max_abs_ci95_half_width > 0 {
            Some(Self(max_abs_ci95_half_width))
        } else {
            None
        }
    }

    pub const fn max_abs_ci95_half_width(self) -> u64 {
        self.0
    }
}

/// A declared live-study analysis plan. The plan is application-owned and is
/// intentionally separate from generic study admission: it specifies which
/// closed pairs may enter the estimator and what precision was targeted
/// before their outcomes were inspected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreregisteredAnalysisPlan {
    pub pair_ids: Vec<AnalysisPairId>,
    pub world_seed_digests: Vec<Blake3Digest>,
    pub estimand: AnalysisEstimand,
    pub exclusion_policy: AnalysisExclusionPolicy,
    pub precision_targets: [PrecisionTarget; 11],
}

impl PreregisteredAnalysisPlan {
    pub fn new(
        pair_ids: Vec<AnalysisPairId>,
        world_seed_digests: Vec<Blake3Digest>,
        estimand: AnalysisEstimand,
        exclusion_policy: AnalysisExclusionPolicy,
        precision_targets: [PrecisionTarget; 11],
    ) -> Result<Self, AnalysisInputError> {
        if pair_ids.is_empty() || pair_ids.len() != world_seed_digests.len() {
            return Err(AnalysisInputError::InvalidAnalysisPlan);
        }
        for (index, pair_id) in pair_ids.iter().enumerate() {
            if pair_ids[..index].iter().any(|prior| prior == pair_id) {
                return Err(AnalysisInputError::DuplicatePairId);
            }
            if world_seed_digests[..index]
                .iter()
                .any(|prior| prior == &world_seed_digests[index])
            {
                return Err(AnalysisInputError::DuplicateWorldSeed);
            }
        }
        Ok(Self {
            pair_ids,
            world_seed_digests,
            estimand,
            exclusion_policy,
            precision_targets,
        })
    }
}

/// The closed set of measurements currently defined by CL-001.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Cl001Metric {
    CorrectionAdoptionLatency,
    FinalDecisionCorrect,
    FalseClaimPersistence,
    CorrectionVisibilityBasisPoints,
    DissentSurvival,
    ForumHistoryUtilizationBasisPoints,
    ForumAttentionBytes,
    ForumAttentionTurns,
    ForumAttentionRuntimeMicros,
    OperationalCostMicroUsd,
    AmortizedInstitutionalCostMicroUsd,
}

impl Cl001Metric {
    /// All metrics in stable artifact order.
    pub const ALL: [Self; 11] = [
        Self::CorrectionAdoptionLatency,
        Self::FinalDecisionCorrect,
        Self::FalseClaimPersistence,
        Self::CorrectionVisibilityBasisPoints,
        Self::DissentSurvival,
        Self::ForumHistoryUtilizationBasisPoints,
        Self::ForumAttentionBytes,
        Self::ForumAttentionTurns,
        Self::ForumAttentionRuntimeMicros,
        Self::OperationalCostMicroUsd,
        Self::AmortizedInstitutionalCostMicroUsd,
    ];

    /// Stable machine-readable metric name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::CorrectionAdoptionLatency => "correction_adoption_latency",
            Self::FinalDecisionCorrect => "final_decision_correct",
            Self::FalseClaimPersistence => "false_claim_persistence",
            Self::CorrectionVisibilityBasisPoints => "correction_visibility_bps",
            Self::DissentSurvival => "dissent_survival",
            Self::ForumHistoryUtilizationBasisPoints => "forum_history_utilization_bps",
            Self::ForumAttentionBytes => "forum_attention_bytes",
            Self::ForumAttentionTurns => "forum_attention_turns",
            Self::ForumAttentionRuntimeMicros => "forum_attention_runtime_micros",
            Self::OperationalCostMicroUsd => "operational_cost_microusd",
            Self::AmortizedInstitutionalCostMicroUsd => "amortized_institutional_cost_microusd",
        }
    }
}

/// A single arm's raw CL-001 values in stable metric order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArmAnalysisObservation {
    values: [MeasurementOutcome; Cl001Metric::ALL.len()],
}

impl ArmAnalysisObservation {
    /// Build an observation from a provider-free or live arm report.
    pub fn from_report(report: &ArmReport) -> Self {
        Self {
            values: [
                report.correction_adoption_latency,
                report.final_decision_correct,
                report.false_claim_persistence,
                report.correction_visibility,
                report.dissent_survival,
                report.forum_history_utilization,
                report.forum_attention_bytes,
                report.forum_attention_turns,
                report.forum_attention_runtime_micros,
                report.operational_cost_microusd,
                report.amortized_institutional_cost_microusd,
            ],
        }
    }

    /// Construct a live observation from the complete closed metric set.
    pub const fn from_values(values: [MeasurementOutcome; Cl001Metric::ALL.len()]) -> Self {
        Self { values }
    }

    /// Return the raw observation for one metric.
    pub fn value(&self, metric: Cl001Metric) -> MeasurementOutcome {
        self.values[metric.index()]
    }
}

impl Cl001Metric {
    const fn index(self) -> usize {
        match self {
            Self::CorrectionAdoptionLatency => 0,
            Self::FinalDecisionCorrect => 1,
            Self::FalseClaimPersistence => 2,
            Self::CorrectionVisibilityBasisPoints => 3,
            Self::DissentSurvival => 4,
            Self::ForumHistoryUtilizationBasisPoints => 5,
            Self::ForumAttentionBytes => 6,
            Self::ForumAttentionTurns => 7,
            Self::ForumAttentionRuntimeMicros => 8,
            Self::OperationalCostMicroUsd => 9,
            Self::AmortizedInstitutionalCostMicroUsd => 10,
        }
    }
}

/// A validated identity for one matched retained/reset pair in an analysis
/// artifact.  This is application data, not a generic ledger identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisPairId(String);

impl AnalysisPairId {
    /// Parse a stable, shell- and TSV-safe pair identity.
    pub fn parse(value: &str) -> Result<Self, AnalysisInputError> {
        if value.is_empty() || value.len() > 128 {
            return Err(AnalysisInputError::InvalidPairId);
        }
        if !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(AnalysisInputError::InvalidPairId);
        }
        Ok(Self(value.to_owned()))
    }

    /// Return the stable pair identity.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One retained/reset pair's raw observations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairObservation {
    pub pair_id: AnalysisPairId,
    pub retained: ArmAnalysisObservation,
    pub reset: ArmAnalysisObservation,
    /// Durable identity and sealed revision facts from the kernel query. This
    /// is present for persisted study pairs and absent for an application
    /// caller that has supplied only already-validated arm values.
    pub provenance: Option<PairProvenance>,
}

/// The minimum persisted identity needed to audit a CL-001 estimate back to
/// its two episode rows and matched protocol contract. The metric values are
/// still application-owned; these fields prevent a TSV from becoming a bag of
/// untraceable numbers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairProvenance {
    pub retained_episode_id: StudyEpisodeId,
    pub reset_episode_id: StudyEpisodeId,
    pub protocol_revision_id: StudyProtocolRevisionId,
    pub world_revision_id: StudyWorldRevisionId,
    pub measurement_revision_id: StudyMeasurementRevisionId,
    pub measurement_slot_count: StudyMeasurementSlotCount,
    pub institution_revision_id: StudyInstitutionRevisionId,
    pub retained_source_population_snapshot_id: StudyPopulationSnapshotId,
    pub reset_source_population_snapshot_id: StudyPopulationSnapshotId,
    pub retained_successor_population_snapshot_id: StudyPopulationSnapshotId,
    pub reset_successor_population_snapshot_id: StudyPopulationSnapshotId,
    pub randomization_digest: Blake3Digest,
    pub retained_frozen_forum_head: i64,
    pub reset_frozen_forum_head: i64,
    pub retained_ground_truth_reveal_digest: Blake3Digest,
    pub reset_ground_truth_reveal_digest: Blake3Digest,
}

impl PairObservation {
    /// Build a pair from an existing CL-001 report.
    pub fn from_report(pair_id: &str, report: &PairedReport) -> Result<Self, AnalysisInputError> {
        Ok(Self {
            pair_id: AnalysisPairId::parse(pair_id)?,
            retained: ArmAnalysisObservation::from_report(&report.retained),
            reset: ArmAnalysisObservation::from_report(&report.reset),
            provenance: None,
        })
    }

    /// Build a pair from a live runner's application-owned arm values.
    pub fn new(
        pair_id: &str,
        retained: ArmAnalysisObservation,
        reset: ArmAnalysisObservation,
    ) -> Result<Self, AnalysisInputError> {
        Ok(Self {
            pair_id: AnalysisPairId::parse(pair_id)?,
            retained,
            reset,
            provenance: None,
        })
    }

    /// Build one application pair from the kernel's read-only persisted-pair
    /// boundary. A missing application metric is an invalidated observation,
    /// never an implicit zero; the reason digest identifies the exact closed
    /// episode/slot whose record is absent.
    pub fn from_persisted_study_pair(
        pair: &PersistedStudyPairObservation,
    ) -> Result<Self, AnalysisInputError> {
        validate_persisted_pair(pair)?;
        let retained_frozen_forum_head = pair
            .retained
            .frozen_forum_head
            .ok_or(AnalysisInputError::IncompletePersistedPair)?;
        let reset_frozen_forum_head = pair
            .reset
            .frozen_forum_head
            .ok_or(AnalysisInputError::IncompletePersistedPair)?;
        let retained_successor_population_snapshot_id = pair
            .retained
            .successor_population_snapshot_id
            .ok_or(AnalysisInputError::IncompletePersistedPair)?;
        let reset_successor_population_snapshot_id = pair
            .reset
            .successor_population_snapshot_id
            .ok_or(AnalysisInputError::IncompletePersistedPair)?;
        let retained_ground_truth_reveal_digest = pair
            .retained
            .ground_truth_reveal_digest
            .ok_or(AnalysisInputError::IncompletePersistedPair)?;
        let reset_ground_truth_reveal_digest = pair
            .reset
            .ground_truth_reveal_digest
            .ok_or(AnalysisInputError::IncompletePersistedPair)?;
        Ok(Self {
            pair_id: AnalysisPairId::parse(&format!("study-pair-{}", pair.pair_id.value()))?,
            retained: ArmAnalysisObservation::from_persisted_episode(&pair.retained),
            reset: ArmAnalysisObservation::from_persisted_episode(&pair.reset),
            provenance: Some(PairProvenance {
                retained_episode_id: pair.retained.episode_id,
                reset_episode_id: pair.reset.episode_id,
                protocol_revision_id: pair.retained.protocol_revision_id,
                world_revision_id: pair.retained.world_revision_id,
                measurement_revision_id: pair.retained.measurement_revision_id,
                measurement_slot_count: pair.retained.measurement_slot_count,
                institution_revision_id: pair.retained.institution_revision_id,
                retained_source_population_snapshot_id: pair.retained.source_population_snapshot_id,
                reset_source_population_snapshot_id: pair.reset.source_population_snapshot_id,
                retained_successor_population_snapshot_id,
                reset_successor_population_snapshot_id,
                randomization_digest: pair.retained.randomization_digest,
                retained_frozen_forum_head,
                reset_frozen_forum_head,
                retained_ground_truth_reveal_digest,
                reset_ground_truth_reveal_digest,
            }),
        })
    }
}

fn validate_persisted_pair(pair: &PersistedStudyPairObservation) -> Result<(), AnalysisInputError> {
    let retained = &pair.retained;
    let reset = &pair.reset;
    if retained.lifecycle_state != StudyEpisodeState::Closed
        || reset.lifecycle_state != StudyEpisodeState::Closed
        || retained.source_actor_obligations != retained.source_terminal_actor_obligations
        || retained.successor_actor_obligations != retained.successor_terminal_actor_obligations
        || reset.source_actor_obligations != reset.source_terminal_actor_obligations
        || reset.successor_actor_obligations != reset.successor_terminal_actor_obligations
        || retained.failed_actor_obligations != 0
        || reset.failed_actor_obligations != 0
        || retained.runtime_bindings != retained.reconciled_runtime_bindings
        || reset.runtime_bindings != reset.reconciled_runtime_bindings
        || retained.successor_population_snapshot_id.is_none()
        || reset.successor_population_snapshot_id.is_none()
        || retained.frozen_forum_head.is_none()
        || reset.frozen_forum_head.is_none()
        || retained.ground_truth_reveal_digest.is_none()
        || reset.ground_truth_reveal_digest.is_none()
        || retained.decisions != 2
        || reset.decisions != 2
    {
        return Err(AnalysisInputError::IncompletePersistedPair);
    }
    if retained.treatment != society_kernel::StudyTreatment::Retained
        || reset.treatment != society_kernel::StudyTreatment::Reset
        || retained.protocol_revision_id != reset.protocol_revision_id
        || retained.world_revision_id != reset.world_revision_id
        || retained.measurement_revision_id != reset.measurement_revision_id
        || retained.measurement_slot_count != reset.measurement_slot_count
        || retained.institution_revision_id != reset.institution_revision_id
        || retained.randomization_digest != reset.randomization_digest
        || retained.measurement_slot_count.value() as usize != Cl001Metric::ALL.len()
        || reset.measurement_slot_count.value() as usize != Cl001Metric::ALL.len()
        || !has_exact_measurement_slots(retained)
        || !has_exact_measurement_slots(reset)
    {
        return Err(AnalysisInputError::MismatchedPersistedPair);
    }
    Ok(())
}

fn has_exact_measurement_slots(episode: &StudyEpisodeObservation) -> bool {
    episode.measurements.len() == Cl001Metric::ALL.len()
        && (1..=Cl001Metric::ALL.len()).all(|slot| {
            episode
                .measurements
                .iter()
                .filter(|measurement| measurement.measurement_slot.value() as usize == slot)
                .count()
                == 1
        })
}

impl ArmAnalysisObservation {
    fn from_persisted_episode(episode: &StudyEpisodeObservation) -> Self {
        let values = std::array::from_fn(|index| {
            let slot = u8::try_from(index + 1).expect("CL-001 metric slots fit in u8");
            let outcome = episode
                .measurements
                .iter()
                .find(|measurement| measurement.measurement_slot.value() == slot);
            match outcome {
                Some(measurement) => match measurement.status {
                    StudyMeasurementStatus::Observed => {
                        match (measurement.value, measurement.value_digest) {
                            (Some(value), Some(value_digest)) => MeasurementOutcome::Observed {
                                value,
                                value_digest,
                            },
                            _ => missing_measurement(episode, slot),
                        }
                    }
                    StudyMeasurementStatus::Unavailable => match measurement.reason_digest {
                        Some(reason_digest) => MeasurementOutcome::Unavailable { reason_digest },
                        None => missing_measurement(episode, slot),
                    },
                    StudyMeasurementStatus::Invalidated => match measurement.reason_digest {
                        Some(reason_digest) => MeasurementOutcome::Invalidated { reason_digest },
                        None => missing_measurement(episode, slot),
                    },
                },
                None => missing_measurement(episode, slot),
            }
        });
        Self::from_values(values)
    }
}

fn missing_measurement(episode: &StudyEpisodeObservation, slot: u8) -> MeasurementOutcome {
    MeasurementOutcome::Invalidated {
        reason_digest: Blake3Digest::of_bytes(
            format!(
                "cl-001|analysis-v1|missing-or-malformed-measurement|episode={}|slot={slot}",
                episode.episode_id.value(),
            )
            .as_bytes(),
        ),
    }
}

/// Input validation failure for the analysis artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalysisInputError {
    InvalidPairId,
    DuplicatePairId,
    DuplicateWorldSeed,
    EmptyInput,
    IncompletePersistedPair,
    MismatchedPersistedPair,
    InvalidAnalysisPlan,
    AnalysisPlanPairMismatch,
    AnalysisPlanProvenanceMissing,
    AnalysisPlanSeedMismatch,
    StudyRunNotReady,
    StudyRunPlanDigestMismatch,
    StudyRunPairRegistrationMismatch,
}

impl fmt::Display for AnalysisInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPairId => "pair id must be 1..=128 ASCII alphanumeric, '.', '_' or '-'",
            Self::DuplicatePairId => "analysis input contains a duplicate pair id",
            Self::DuplicateWorldSeed => "analysis plan contains a duplicate world seed",
            Self::EmptyInput => "analysis input must contain at least one matched pair",
            Self::IncompletePersistedPair => {
                "persisted pair is not closed with every admitted actor obligation terminal"
            }
            Self::MismatchedPersistedPair => {
                "persisted pair does not match the CL-001 protocol or measurement contract"
            }
            Self::InvalidAnalysisPlan => {
                "analysis plan must contain the same non-empty number of pair ids and world seeds"
            }
            Self::AnalysisPlanPairMismatch => {
                "analysis pairs do not match the pre-registered pair identity list"
            }
            Self::AnalysisPlanProvenanceMissing => {
                "a pre-registered analysis requires persisted pair provenance"
            }
            Self::AnalysisPlanSeedMismatch => {
                "persisted pair randomization does not match its pre-registered world seed"
            }
            Self::StudyRunNotReady => {
                "the admitted study run is not terminally completed with its full matched-pair set"
            }
            Self::StudyRunPlanDigestMismatch => {
                "the admitted study run does not retain this sealed CL-001 plan"
            }
            Self::StudyRunPairRegistrationMismatch => {
                "the persisted pair observations do not match the admitted study-run registration"
            }
        })
    }
}

impl std::error::Error for AnalysisInputError {}

/// A metric's paired estimate and explicit missingness counts.
#[derive(Clone, Debug, PartialEq)]
pub struct MetricSummary {
    pub metric: Cl001Metric,
    pub retained_observed: usize,
    pub reset_observed: usize,
    pub paired_observed: usize,
    pub retained_unavailable: usize,
    pub retained_invalidated: usize,
    pub reset_unavailable: usize,
    pub reset_invalidated: usize,
    pub delta_unavailable: usize,
    pub delta_invalidated: usize,
    pub retained_mean: Option<f64>,
    pub reset_mean: Option<f64>,
    pub paired_delta_mean: Option<f64>,
    pub paired_delta_sample_sd: Option<f64>,
    pub paired_delta_ci95_low: Option<f64>,
    pub paired_delta_ci95_high: Option<f64>,
}

/// Complete application-owned analysis artifact.
#[derive(Clone, Debug, PartialEq)]
pub struct AnalysisArtifact {
    pub revision: &'static str,
    pub plan: Option<PreregisteredAnalysisPlan>,
    pub pairs: Vec<PairObservation>,
    pub summaries: Vec<MetricSummary>,
}

impl AnalysisArtifact {
    /// Validate and analyze matched pairs.  Missing or invalid values are
    /// retained in the artifact but excluded from the paired estimand.
    pub fn from_pairs(pairs: Vec<PairObservation>) -> Result<Self, AnalysisInputError> {
        if pairs.is_empty() {
            return Err(AnalysisInputError::EmptyInput);
        }
        for (index, pair) in pairs.iter().enumerate() {
            if pairs[..index]
                .iter()
                .any(|prior| prior.pair_id == pair.pair_id)
            {
                return Err(AnalysisInputError::DuplicatePairId);
            }
        }
        let summaries = Cl001Metric::ALL
            .into_iter()
            .map(|metric| summarize(metric, &pairs))
            .collect();
        Ok(Self {
            revision: ANALYSIS_REVISION,
            plan: None,
            pairs,
            summaries,
        })
    }

    /// Build an inferential artifact only after validating a declared plan
    /// against the exact pair identities and persisted world-seed digests.
    /// A planned estimate requires kernel-derived provenance; otherwise a
    /// caller could pair arbitrary application values with an unverifiable
    /// seed label. The ordinary `from_pairs` path intentionally has no plan
    /// and is suitable only for deterministic acceptance output.
    pub fn from_preregistered_plan(
        plan: PreregisteredAnalysisPlan,
        pairs: Vec<PairObservation>,
    ) -> Result<Self, AnalysisInputError> {
        if pairs.len() != plan.pair_ids.len() {
            return Err(AnalysisInputError::AnalysisPlanPairMismatch);
        }
        for ((pair, expected_id), expected_seed) in pairs
            .iter()
            .zip(plan.pair_ids.iter())
            .zip(plan.world_seed_digests.iter())
        {
            if &pair.pair_id != expected_id {
                return Err(AnalysisInputError::AnalysisPlanPairMismatch);
            }
            let provenance = pair
                .provenance
                .as_ref()
                .ok_or(AnalysisInputError::AnalysisPlanProvenanceMissing)?;
            if provenance.randomization_digest != *expected_seed {
                return Err(AnalysisInputError::AnalysisPlanSeedMismatch);
            }
        }
        let mut artifact = Self::from_pairs(pairs)?;
        artifact.plan = Some(plan);
        Ok(artifact)
    }

    /// Convert fixed PostgreSQL study-pair observations through the
    /// application-owned CL-001 metric map, then calculate the paired
    /// artifact. Callers never need to reconstruct generic table joins or
    /// assign a favorable value to a missing result.
    pub fn from_persisted_study_pairs(
        pairs: Vec<PersistedStudyPairObservation>,
    ) -> Result<Self, AnalysisInputError> {
        pairs
            .iter()
            .map(PairObservation::from_persisted_study_pair)
            .collect::<Result<Vec<_>, _>>()
            .and_then(Self::from_pairs)
    }

    /// Build a planned artifact from the daemon's read-only study-run and
    /// pair observations. The sealed CL-001 plan owns human-stable pair IDs;
    /// the generic ledger assigns separate numeric pair IDs. This explicit,
    /// ordinal-preserving join proves that each application pair label and
    /// pre-registered world seed refers to the exact generic pair registered
    /// in the admitted run rather than relying on a post-hoc name convention.
    pub fn from_preregistered_study_run(
        plan: PreregisteredAnalysisPlan,
        study_run: &StudyRunObservation,
        persisted_pairs: Vec<PersistedStudyPairObservation>,
    ) -> Result<Self, AnalysisInputError> {
        let planned_pair_count = plan.pair_ids.len();
        // `Ready` means that pair registrations are complete and `Running`
        // means execution may still be in progress. A generic terminal
        // transition accepts only after both episodes in every registered
        // pair have individually passed their closure checks. Requiring it
        // prevents analysis of an admitted, started, or partially closed run.
        if study_run.lifecycle_state != StudyRunLifecycleState::Completed
            || usize::from(study_run.pair_count.value()) != planned_pair_count
            || usize::from(study_run.registered_pair_count.value()) != planned_pair_count
            || study_run.pairs.len() != planned_pair_count
        {
            return Err(AnalysisInputError::StudyRunNotReady);
        }
        if persisted_pairs.len() != planned_pair_count {
            return Err(AnalysisInputError::StudyRunPairRegistrationMismatch);
        }
        let mut pairs = Vec::with_capacity(planned_pair_count);
        for (index, (((registration, persisted_pair), plan_pair_id), plan_seed)) in study_run
            .pairs
            .iter()
            .zip(persisted_pairs.iter())
            .zip(plan.pair_ids.iter())
            .zip(plan.world_seed_digests.iter())
            .enumerate()
        {
            if usize::from(registration.pair_ordinal.value()) != index + 1
                || registration.pair_id != persisted_pair.pair_id
                || registration.randomization_digest != *plan_seed
                || persisted_pair.retained.randomization_digest != *plan_seed
                || persisted_pair.reset.randomization_digest != *plan_seed
            {
                return Err(AnalysisInputError::StudyRunPairRegistrationMismatch);
            }
            let mut pair = PairObservation::from_persisted_study_pair(persisted_pair)?;
            pair.pair_id = plan_pair_id.clone();
            pairs.push(pair);
        }
        Self::from_preregistered_plan(plan, pairs)
    }

    /// Render a deterministic TSV artifact suitable for a spreadsheet, R,
    /// or a small shell/awk analysis.  Raw values precede summaries so that
    /// every estimate can be audited back to its pair and arm.
    pub fn render_tsv(&self) -> String {
        let mut output = String::new();
        output.push_str("artifact\t");
        output.push_str(self.revision);
        output.push('\n');
        if let Some(plan) = &self.plan {
            output.push_str("plan\testimand\texclusion_policy\tpair_count\n");
            output.push_str("plan\tretained_minus_reset\tmetricwise_complete_case\t");
            output.push_str(&plan.pair_ids.len().to_string());
            output.push('\n');
            output.push_str("plan_pair\tpair_id\tworld_seed_digest\n");
            for (pair_id, seed_digest) in plan.pair_ids.iter().zip(&plan.world_seed_digests) {
                output.push_str("plan_pair\t");
                output.push_str(pair_id.as_str());
                output.push('\t');
                output.push_str(&hex_digest(*seed_digest));
                output.push('\n');
            }
            output.push_str("plan_precision\tmetric\tmax_abs_ci95_half_width\n");
            for (metric, target) in Cl001Metric::ALL.into_iter().zip(plan.precision_targets) {
                output.push_str("plan_precision\t");
                output.push_str(metric.name());
                output.push('\t');
                output.push_str(&target.max_abs_ci95_half_width().to_string());
                output.push('\n');
            }
        }
        output.push_str("provenance\tpair_id\tretained_episode_id\treset_episode_id\tprotocol_revision_id\tworld_revision_id\tmeasurement_revision_id\tmeasurement_slot_count\tinstitution_revision_id\tretained_source_population_snapshot_id\treset_source_population_snapshot_id\tretained_successor_population_snapshot_id\treset_successor_population_snapshot_id\trandomization_digest\tretained_frozen_forum_head\treset_frozen_forum_head\tretained_ground_truth_reveal_digest\treset_ground_truth_reveal_digest\n");
        for pair in &self.pairs {
            if let Some(provenance) = &pair.provenance {
                output.push_str("provenance\t");
                output.push_str(pair.pair_id.as_str());
                for value in [
                    provenance.retained_episode_id.value(),
                    provenance.reset_episode_id.value(),
                    provenance.protocol_revision_id.value(),
                    provenance.world_revision_id.value(),
                    provenance.measurement_revision_id.value(),
                    i64::from(provenance.measurement_slot_count.value()),
                    provenance.institution_revision_id.value(),
                    provenance.retained_source_population_snapshot_id.value(),
                    provenance.reset_source_population_snapshot_id.value(),
                    provenance.retained_successor_population_snapshot_id.value(),
                    provenance.reset_successor_population_snapshot_id.value(),
                ] {
                    output.push('\t');
                    output.push_str(&value.to_string());
                }
                output.push('\t');
                output.push_str(&hex_digest(provenance.randomization_digest));
                output.push('\t');
                output.push_str(&provenance.retained_frozen_forum_head.to_string());
                output.push('\t');
                output.push_str(&provenance.reset_frozen_forum_head.to_string());
                output.push('\t');
                output.push_str(&hex_digest(provenance.retained_ground_truth_reveal_digest));
                output.push('\t');
                output.push_str(&hex_digest(provenance.reset_ground_truth_reveal_digest));
                output.push('\n');
            }
        }
        output.push_str("row\tpair_id\tmetric\tretained_status\tretained_value\tretained_digest\tretained_reason_digest\treset_status\treset_value\treset_digest\treset_reason_digest\tdelta_status\tdelta_value\n");
        for pair in &self.pairs {
            for metric in Cl001Metric::ALL {
                let retained = pair.retained.value(metric);
                let reset = pair.reset.value(metric);
                let (delta_status, delta_value) = delta_cell(retained, reset);
                output.push_str("raw\t");
                output.push_str(pair.pair_id.as_str());
                output.push('\t');
                output.push_str(metric.name());
                output.push('\t');
                append_outcome(&mut output, retained);
                output.push('\t');
                append_outcome(&mut output, reset);
                output.push('\t');
                output.push_str(delta_status);
                output.push('\t');
                append_optional_i64(&mut output, delta_value);
                output.push('\n');
            }
        }
        output.push_str("row\tmetric\tretained_observed\treset_observed\tpaired_observed\tretained_unavailable\tretained_invalidated\treset_unavailable\treset_invalidated\tdelta_unavailable\tdelta_invalidated\tretained_mean\treset_mean\tpaired_delta_mean\tpaired_delta_sample_sd\tpaired_delta_ci95_low\tpaired_delta_ci95_high\n");
        for summary in &self.summaries {
            output.push_str("summary\t");
            output.push_str(summary.metric.name());
            for value in [
                summary.retained_observed,
                summary.reset_observed,
                summary.paired_observed,
                summary.retained_unavailable,
                summary.retained_invalidated,
                summary.reset_unavailable,
                summary.reset_invalidated,
                summary.delta_unavailable,
                summary.delta_invalidated,
            ] {
                output.push('\t');
                output.push_str(&value.to_string());
            }
            for value in [
                summary.retained_mean,
                summary.reset_mean,
                summary.paired_delta_mean,
                summary.paired_delta_sample_sd,
                summary.paired_delta_ci95_low,
                summary.paired_delta_ci95_high,
            ] {
                output.push('\t');
                append_optional_f64(&mut output, value);
            }
            output.push('\n');
        }
        output
    }
}

fn summarize(metric: Cl001Metric, pairs: &[PairObservation]) -> MetricSummary {
    let mut retained = Vec::new();
    let mut reset = Vec::new();
    let mut deltas = Vec::new();
    let mut retained_unavailable = 0;
    let mut retained_invalidated = 0;
    let mut reset_unavailable = 0;
    let mut reset_invalidated = 0;
    let mut delta_unavailable = 0;
    let mut delta_invalidated = 0;
    for pair in pairs {
        let retained_value = pair.retained.value(metric);
        let reset_value = pair.reset.value(metric);
        classify(
            retained_value,
            &mut retained,
            &mut retained_unavailable,
            &mut retained_invalidated,
        );
        classify(
            reset_value,
            &mut reset,
            &mut reset_unavailable,
            &mut reset_invalidated,
        );
        match delta_cell(retained_value, reset_value) {
            ("observed", Some(value)) => deltas.push(value as f64),
            ("invalidated", _) => delta_invalidated += 1,
            ("unavailable", _) => delta_unavailable += 1,
            _ => unreachable!("delta_cell returns a closed status"),
        }
    }
    let paired_delta_sample_sd = sample_sd(&deltas);
    let paired_delta_mean = mean_f64(&deltas);
    let (paired_delta_ci95_low, paired_delta_ci95_high) = confidence_interval_95(&deltas);
    MetricSummary {
        metric,
        retained_observed: retained.len(),
        reset_observed: reset.len(),
        paired_observed: deltas.len(),
        retained_unavailable,
        retained_invalidated,
        reset_unavailable,
        reset_invalidated,
        delta_unavailable,
        delta_invalidated,
        retained_mean: mean(&retained),
        reset_mean: mean(&reset),
        paired_delta_mean,
        paired_delta_sample_sd,
        paired_delta_ci95_low,
        paired_delta_ci95_high,
    }
}

fn classify(
    outcome: MeasurementOutcome,
    observed: &mut Vec<i64>,
    unavailable: &mut usize,
    invalidated: &mut usize,
) {
    match outcome {
        MeasurementOutcome::Observed { value, .. } => observed.push(value),
        MeasurementOutcome::Unavailable { .. } => *unavailable += 1,
        MeasurementOutcome::Invalidated { .. } => *invalidated += 1,
    }
}

fn delta_cell(left: MeasurementOutcome, right: MeasurementOutcome) -> (&'static str, Option<i64>) {
    match (left, right) {
        (
            MeasurementOutcome::Observed { value: left, .. },
            MeasurementOutcome::Observed { value: right, .. },
        ) => match left.checked_sub(right) {
            Some(value) => ("observed", Some(value)),
            None => ("invalidated", None),
        },
        (MeasurementOutcome::Invalidated { .. }, _)
        | (_, MeasurementOutcome::Invalidated { .. }) => ("invalidated", None),
        _ => ("unavailable", None),
    }
}

fn mean(values: &[i64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().map(|value| *value as f64).sum::<f64>() / values.len() as f64)
}

fn mean_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

fn sample_sd(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let average = values.iter().sum::<f64>() / values.len() as f64;
    Some(
        (values
            .iter()
            .map(|value| (value - average).powi(2))
            .sum::<f64>()
            / (values.len() - 1) as f64)
            .sqrt(),
    )
}

fn confidence_interval_95(values: &[f64]) -> (Option<f64>, Option<f64>) {
    if values.len() < 2 {
        return (None, None);
    }
    let average = values.iter().sum::<f64>() / values.len() as f64;
    let standard_error =
        sample_sd(values).expect("sample size is at least two") / (values.len() as f64).sqrt();
    let margin = student_t_975(values.len() - 1) * standard_error;
    (Some(average - margin), Some(average + margin))
}

// Two-sided 95% Student-t critical values for df=1..=30. For larger samples,
// use the Cornish-Fisher expansion around the exact normal quantile rather
// than silently switching to a normal interval. The expansion is more than
// adequate above this table's range and keeps the application free of a
// statistics dependency while retaining its stated Student-t interval.
fn student_t_975(degrees_of_freedom: usize) -> f64 {
    const VALUES: [f64; 30] = [
        12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179, 2.160,
        2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086, 2.080, 2.074, 2.069, 2.064, 2.060, 2.056,
        2.052, 2.048, 2.045, 2.042,
    ];
    if let Some(value) = VALUES.get(degrees_of_freedom.saturating_sub(1)) {
        return *value;
    }
    let z = 1.959_963_984_540_054_f64;
    let z2 = z * z;
    let z3 = z2 * z;
    let z5 = z3 * z2;
    let z7 = z5 * z2;
    let z9 = z7 * z2;
    let degrees = degrees_of_freedom as f64;
    z + (z3 + z) / (4.0 * degrees)
        + (5.0 * z5 + 16.0 * z3 + 3.0 * z) / (96.0 * degrees.powi(2))
        + (3.0 * z7 + 19.0 * z5 + 17.0 * z3 - 15.0 * z) / (384.0 * degrees.powi(3))
        + (79.0 * z9 + 776.0 * z7 + 1482.0 * z5 - 1920.0 * z3 - 945.0 * z)
            / (92_160.0 * degrees.powi(4))
}

fn append_outcome(output: &mut String, outcome: MeasurementOutcome) {
    match outcome {
        MeasurementOutcome::Observed {
            value,
            value_digest,
        } => {
            output.push_str("observed\t");
            output.push_str(&value.to_string());
            output.push('\t');
            output.push_str(&hex_digest(value_digest));
            output.push('\t');
            output.push_str("-");
        }
        MeasurementOutcome::Unavailable { reason_digest } => {
            output.push_str("unavailable\t-\t-\t");
            output.push_str(&hex_digest(reason_digest));
        }
        MeasurementOutcome::Invalidated { reason_digest } => {
            output.push_str("invalidated\t-\t-\t");
            output.push_str(&hex_digest(reason_digest));
        }
    }
}

fn append_optional_i64(output: &mut String, value: Option<i64>) {
    match value {
        Some(value) => output.push_str(&value.to_string()),
        None => output.push('-'),
    }
}

fn append_optional_f64(output: &mut String, value: Option<f64>) {
    match value {
        Some(value) => output.push_str(&format!("{value:.6}")),
        None => output.push('-'),
    }
}

fn hex_digest(digest: Blake3Digest) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest.as_bytes() {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed(value: i64) -> MeasurementOutcome {
        MeasurementOutcome::Observed {
            value,
            value_digest: Blake3Digest::of_bytes(&value.to_be_bytes()),
        }
    }

    fn unavailable() -> MeasurementOutcome {
        MeasurementOutcome::Unavailable {
            reason_digest: Blake3Digest::of_bytes(b"unavailable"),
        }
    }

    fn invalidated() -> MeasurementOutcome {
        MeasurementOutcome::Invalidated {
            reason_digest: Blake3Digest::of_bytes(b"invalidated"),
        }
    }

    fn arm(latency: MeasurementOutcome, cost: MeasurementOutcome) -> ArmAnalysisObservation {
        ArmAnalysisObservation::from_values([
            latency,
            observed(1),
            observed(0),
            observed(10_000),
            observed(1),
            observed(2_500),
            observed(100),
            observed(8),
            observed(20),
            cost,
            cost,
        ])
    }

    fn complete_persisted_pair(
        pair_id: i64,
        randomization_digest: Blake3Digest,
    ) -> PersistedStudyPairObservation {
        use society_kernel::{
            StudyInstitutionRevisionId, StudyMeasurementObservation, StudyMeasurementRevisionId,
            StudyMeasurementSlot, StudyMeasurementSlotCount, StudyPairId,
            StudyPopulationSnapshotId, StudyProtocolRevisionId, StudyWorldRevisionId,
        };

        let episode = |episode_id, treatment| StudyEpisodeObservation {
            episode_id: StudyEpisodeId::new(episode_id).unwrap(),
            protocol_revision_id: StudyProtocolRevisionId::new(1).unwrap(),
            world_revision_id: StudyWorldRevisionId::new(2).unwrap(),
            measurement_revision_id: StudyMeasurementRevisionId::new(3).unwrap(),
            measurement_slot_count: StudyMeasurementSlotCount::new(11).unwrap(),
            institution_revision_id: StudyInstitutionRevisionId::new(4).unwrap(),
            source_population_snapshot_id: StudyPopulationSnapshotId::new(5).unwrap(),
            successor_population_snapshot_id: Some(StudyPopulationSnapshotId::new(6).unwrap()),
            randomization_digest,
            treatment,
            lifecycle_state: StudyEpisodeState::Closed,
            source_actor_obligations: 8,
            source_terminal_actor_obligations: 8,
            successor_actor_obligations: 8,
            successor_terminal_actor_obligations: 8,
            failed_actor_obligations: 0,
            runtime_bindings: 16,
            reconciled_runtime_bindings: 16,
            frozen_forum_head: Some(8),
            forum_messages: 17,
            forum_reads: 16,
            forum_returned_bytes: 2_048,
            decisions: 2,
            ground_truth_reveal_digest: Some(Blake3Digest::of_bytes(b"truth")),
            measurements: (1..=11)
                .map(|slot| {
                    let value = i64::from(slot);
                    StudyMeasurementObservation {
                        measurement_slot: StudyMeasurementSlot::new(slot).unwrap(),
                        status: StudyMeasurementStatus::Observed,
                        value: Some(value),
                        value_digest: Some(Blake3Digest::of_bytes(&value.to_be_bytes())),
                        reason_digest: None,
                    }
                })
                .collect(),
        };
        PersistedStudyPairObservation {
            pair_id: StudyPairId::new(pair_id).unwrap(),
            retained: episode(11, society_kernel::StudyTreatment::Retained),
            reset: episode(12, society_kernel::StudyTreatment::Reset),
        }
    }

    #[test]
    fn paired_estimate_preserves_raw_values_missingness_and_ci() {
        let pairs = vec![
            PairObservation::new(
                "seed-01",
                arm(observed(3), observed(1_000)),
                arm(observed(1), observed(800)),
            )
            .unwrap(),
            PairObservation::new(
                "seed-02",
                arm(observed(5), observed(1_200)),
                arm(observed(2), observed(900)),
            )
            .unwrap(),
            PairObservation::new(
                "seed-03",
                arm(unavailable(), unavailable()),
                arm(observed(4), observed(700)),
            )
            .unwrap(),
            PairObservation::new(
                "seed-04",
                arm(invalidated(), invalidated()),
                arm(invalidated(), unavailable()),
            )
            .unwrap(),
        ];
        let artifact = AnalysisArtifact::from_pairs(pairs).unwrap();
        let latency = &artifact.summaries[0];
        assert_eq!(latency.retained_observed, 2);
        assert_eq!(latency.reset_observed, 3);
        assert_eq!(latency.paired_observed, 2);
        assert_eq!(latency.retained_unavailable, 1);
        assert_eq!(latency.retained_invalidated, 1);
        assert_eq!(latency.delta_unavailable, 1);
        assert_eq!(latency.delta_invalidated, 1);
        assert_eq!(latency.paired_delta_mean, Some(2.5));
        assert!(latency.paired_delta_ci95_low.is_some());
        assert!(latency.paired_delta_ci95_high.is_some());

        let rendered = artifact.render_tsv();
        assert!(rendered.starts_with("artifact\tcl-001-analysis-v1\n"));
        assert!(rendered.contains("raw\tseed-01\tcorrection_adoption_latency\tobserved\t3\t"));
        assert!(rendered.contains("summary\toperational_cost_microusd\t2\t3\t2\t1\t1"));
        assert!(rendered.contains("\tpaired_delta_ci95_low\tpaired_delta_ci95_high\n"));
    }

    #[test]
    fn pair_ids_are_unique_and_tsv_safe() {
        assert_eq!(
            AnalysisPairId::parse("seed/01"),
            Err(AnalysisInputError::InvalidPairId)
        );
        let one = PairObservation::new(
            "seed-01",
            arm(observed(1), unavailable()),
            arm(observed(1), unavailable()),
        )
        .unwrap();
        let two = PairObservation::new(
            "seed-01",
            arm(observed(2), unavailable()),
            arm(observed(2), unavailable()),
        )
        .unwrap();
        assert_eq!(
            AnalysisArtifact::from_pairs(vec![one, two]),
            Err(AnalysisInputError::DuplicatePairId)
        );
    }

    #[test]
    fn preregistered_plan_rejects_unprovenanced_application_values() {
        let pair = PairObservation::new(
            "seed-01",
            arm(observed(3), unavailable()),
            arm(observed(1), unavailable()),
        )
        .unwrap();
        let plan = PreregisteredAnalysisPlan::new(
            vec![AnalysisPairId::parse("seed-01").unwrap()],
            vec![Blake3Digest::of_bytes(b"world-seed-01")],
            AnalysisEstimand::RetainedMinusReset,
            AnalysisExclusionPolicy::MetricwiseCompleteCase,
            [PrecisionTarget::new(250).unwrap(); Cl001Metric::ALL.len()],
        )
        .unwrap();
        assert_eq!(
            AnalysisArtifact::from_preregistered_plan(plan, vec![pair]),
            Err(AnalysisInputError::AnalysisPlanProvenanceMissing)
        );
    }

    #[test]
    fn preregistered_plan_is_required_for_planned_artifact_and_is_rendered() {
        let seed = Blake3Digest::of_bytes(b"world-seed-01");
        let mut pair = PairObservation::new(
            "seed-01",
            arm(observed(3), unavailable()),
            arm(observed(1), unavailable()),
        )
        .unwrap();
        pair.provenance = Some(PairProvenance {
            retained_episode_id: StudyEpisodeId::new(1).unwrap(),
            reset_episode_id: StudyEpisodeId::new(2).unwrap(),
            protocol_revision_id: StudyProtocolRevisionId::new(1).unwrap(),
            world_revision_id: StudyWorldRevisionId::new(1).unwrap(),
            measurement_revision_id: StudyMeasurementRevisionId::new(1).unwrap(),
            measurement_slot_count: StudyMeasurementSlotCount::new(11).unwrap(),
            institution_revision_id: StudyInstitutionRevisionId::new(1).unwrap(),
            retained_source_population_snapshot_id: StudyPopulationSnapshotId::new(1).unwrap(),
            reset_source_population_snapshot_id: StudyPopulationSnapshotId::new(2).unwrap(),
            retained_successor_population_snapshot_id: StudyPopulationSnapshotId::new(3).unwrap(),
            reset_successor_population_snapshot_id: StudyPopulationSnapshotId::new(4).unwrap(),
            randomization_digest: seed,
            retained_frozen_forum_head: 8,
            reset_frozen_forum_head: 8,
            retained_ground_truth_reveal_digest: Blake3Digest::of_bytes(b"truth"),
            reset_ground_truth_reveal_digest: Blake3Digest::of_bytes(b"truth"),
        });
        let plan = PreregisteredAnalysisPlan::new(
            vec![AnalysisPairId::parse("seed-01").unwrap()],
            vec![seed],
            AnalysisEstimand::RetainedMinusReset,
            AnalysisExclusionPolicy::MetricwiseCompleteCase,
            [PrecisionTarget::new(250).unwrap(); Cl001Metric::ALL.len()],
        )
        .unwrap();
        let artifact = AnalysisArtifact::from_preregistered_plan(plan, vec![pair]).unwrap();
        assert!(artifact.plan.is_some());
        let rendered = artifact.render_tsv();
        assert!(rendered.contains("plan\tretained_minus_reset\tmetricwise_complete_case\t1\n"));
        assert!(rendered.contains("plan_precision\tcorrection_adoption_latency\t250\n"));
    }

    #[test]
    fn preregistered_study_run_requires_exact_registered_pair_mapping() {
        use society_kernel::{
            ContentObjectId, StudyPairId, StudyProtocolRevisionId, StudyRunId,
            StudyRunLifecycleState, StudyRunObservation, StudyRunPairCount, StudyRunPairOrdinal,
            StudyRunPairRegistrationObservation, StudyRunRegisteredPairCount,
        };

        let seed = Blake3Digest::of_bytes(b"study-run-seed-01");
        let persisted_pair = complete_persisted_pair(17, seed);
        let plan = PreregisteredAnalysisPlan::new(
            vec![AnalysisPairId::parse("pre-registered-01").unwrap()],
            vec![seed],
            AnalysisEstimand::RetainedMinusReset,
            AnalysisExclusionPolicy::MetricwiseCompleteCase,
            [PrecisionTarget::new(250).unwrap(); Cl001Metric::ALL.len()],
        )
        .unwrap();
        let study_run = StudyRunObservation {
            study_run_id: StudyRunId::new(7).unwrap(),
            protocol_revision_id: StudyProtocolRevisionId::new(1).unwrap(),
            plan_content_object_id: ContentObjectId::new(9).unwrap(),
            plan_digest: Blake3Digest::of_bytes(b"sealed-live-plan"),
            pair_count: StudyRunPairCount::new(1).unwrap(),
            registered_pair_count: StudyRunRegisteredPairCount::new(1).unwrap(),
            lifecycle_state: StudyRunLifecycleState::Completed,
            pairs: vec![StudyRunPairRegistrationObservation {
                pair_ordinal: StudyRunPairOrdinal::new(1).unwrap(),
                pair_id: StudyPairId::new(17).unwrap(),
                randomization_digest: seed,
            }],
        };
        let artifact = AnalysisArtifact::from_preregistered_study_run(
            plan.clone(),
            &study_run,
            vec![persisted_pair.clone()],
        )
        .unwrap();
        assert_eq!(artifact.pairs[0].pair_id.as_str(), "pre-registered-01");
        assert!(artifact.plan.is_some());

        let mut noncanonical_ordinal_run = study_run.clone();
        noncanonical_ordinal_run.pairs[0].pair_ordinal = StudyRunPairOrdinal::new(2).unwrap();
        assert_eq!(
            AnalysisArtifact::from_preregistered_study_run(
                plan.clone(),
                &noncanonical_ordinal_run,
                vec![persisted_pair.clone()],
            ),
            Err(AnalysisInputError::StudyRunPairRegistrationMismatch)
        );

        let mut mismatched_run = study_run;
        mismatched_run.pairs[0].pair_id = StudyPairId::new(18).unwrap();
        assert_eq!(
            AnalysisArtifact::from_preregistered_study_run(
                plan,
                &mismatched_run,
                vec![persisted_pair],
            ),
            Err(AnalysisInputError::StudyRunPairRegistrationMismatch)
        );
    }

    #[test]
    fn preregistered_plan_rejects_duplicate_world_seeds() {
        let id_one = AnalysisPairId::parse("seed-01").unwrap();
        let id_two = AnalysisPairId::parse("seed-02").unwrap();
        let seed = Blake3Digest::of_bytes(b"same-seed");
        assert_eq!(
            PreregisteredAnalysisPlan::new(
                vec![id_one, id_two],
                vec![seed, seed],
                AnalysisEstimand::RetainedMinusReset,
                AnalysisExclusionPolicy::MetricwiseCompleteCase,
                [PrecisionTarget::new(1).unwrap(); Cl001Metric::ALL.len()],
            ),
            Err(AnalysisInputError::DuplicateWorldSeed)
        );
    }

    #[test]
    fn large_sample_interval_uses_student_t_not_the_normal_limit() {
        assert!((student_t_975(31) - 2.040).abs() < 0.001);
        assert!((student_t_975(100) - 1.984).abs() < 0.001);
        assert!(student_t_975(31) > 1.960);
    }

    #[test]
    fn persisted_pair_conversion_preserves_malformed_rows_as_invalidated() {
        use society_kernel::{
            StudyEpisodeId, StudyInstitutionRevisionId, StudyMeasurementObservation,
            StudyMeasurementRevisionId, StudyMeasurementSlot, StudyMeasurementSlotCount,
            StudyPairId, StudyPopulationSnapshotId, StudyProtocolRevisionId, StudyWorldRevisionId,
        };

        let episode = |episode_id, treatment| StudyEpisodeObservation {
            episode_id: StudyEpisodeId::new(episode_id).unwrap(),
            protocol_revision_id: StudyProtocolRevisionId::new(1).unwrap(),
            world_revision_id: StudyWorldRevisionId::new(1).unwrap(),
            measurement_revision_id: StudyMeasurementRevisionId::new(1).unwrap(),
            measurement_slot_count: StudyMeasurementSlotCount::new(11).unwrap(),
            institution_revision_id: StudyInstitutionRevisionId::new(1).unwrap(),
            source_population_snapshot_id: StudyPopulationSnapshotId::new(1).unwrap(),
            successor_population_snapshot_id: Some(StudyPopulationSnapshotId::new(2).unwrap()),
            randomization_digest: Blake3Digest::of_bytes(b"seed"),
            treatment,
            lifecycle_state: StudyEpisodeState::Closed,
            source_actor_obligations: 8,
            source_terminal_actor_obligations: 8,
            successor_actor_obligations: 8,
            successor_terminal_actor_obligations: 8,
            failed_actor_obligations: 0,
            runtime_bindings: 16,
            reconciled_runtime_bindings: 16,
            frozen_forum_head: Some(8),
            forum_messages: 17,
            forum_reads: 16,
            forum_returned_bytes: 2_048,
            decisions: 2,
            ground_truth_reveal_digest: Some(Blake3Digest::of_bytes(b"truth")),
            measurements: (1..=11)
                .map(|slot| {
                    let measurement_slot = StudyMeasurementSlot::new(slot).unwrap();
                    match slot {
                        1 => StudyMeasurementObservation {
                            measurement_slot,
                            status: StudyMeasurementStatus::Observed,
                            value: Some(4),
                            value_digest: Some(Blake3Digest::of_bytes(b"latency")),
                            reason_digest: None,
                        },
                        // A row is present but violates the observed shape;
                        // conversion must retain it as invalidated rather than
                        // manufacturing a value for the estimator.
                        2 => StudyMeasurementObservation {
                            measurement_slot,
                            status: StudyMeasurementStatus::Observed,
                            value: None,
                            value_digest: None,
                            reason_digest: None,
                        },
                        _ => StudyMeasurementObservation {
                            measurement_slot,
                            status: StudyMeasurementStatus::Unavailable,
                            value: None,
                            value_digest: None,
                            reason_digest: Some(Blake3Digest::of_bytes(b"not-applicable")),
                        },
                    }
                })
                .collect(),
        };
        let persisted = PersistedStudyPairObservation {
            pair_id: StudyPairId::new(7).unwrap(),
            retained: episode(11, society_kernel::StudyTreatment::Retained),
            reset: episode(12, society_kernel::StudyTreatment::Reset),
        };
        let pair = PairObservation::from_persisted_study_pair(&persisted).unwrap();
        assert_eq!(pair.pair_id.as_str(), "study-pair-7");
        assert!(matches!(
            pair.retained.value(Cl001Metric::CorrectionAdoptionLatency),
            MeasurementOutcome::Observed { value: 4, .. }
        ));
        assert!(matches!(
            pair.reset.value(Cl001Metric::FinalDecisionCorrect),
            MeasurementOutcome::Invalidated { .. }
        ));
        let artifact = AnalysisArtifact::from_persisted_study_pairs(vec![persisted]).unwrap();
        assert_eq!(artifact.pairs.len(), 1);
        assert_eq!(artifact.summaries[0].paired_observed, 1);
    }
}
