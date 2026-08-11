//! Application-owned choreography for a canonical CL-001 live pair.
//!
//! `LiveRunPlan` is the admission contract: it identifies the world, policy,
//! role topology, and independent pair seeds.  This module is the next
//! application boundary a coordinator consumes.  It expands one sealed plan
//! into exact role-prompt material and keeps the paired-arm gates explicit,
//! without owning a daemon, a provider session, or a persistence handle.
//!
//! In particular, this module never includes treatment labels in actor prompt
//! bytes.  The retained/reset intervention is represented only by the
//! successor Forum exposure frontier.  This makes accidental prompt leakage
//! into the actor-policy treatment a construction error in the application
//! choreography rather than a convention at a caller site.

use correction_latency_world::{
    ForumReadObligation, PrivateViewKind, RoleKind, RoleOrdinal, RoleSpecification, WorldFixture,
};
use society_kernel::{
    Blake3Digest, StudyPairObservation as PersistedStudyPairObservation, forum_f0_awareness_digest,
    forum_f0_tool_contract_digest,
};

use crate::{
    AnalysisInputError, AnalysisPairId, ArmAnalysisObservation, LiveRunDescriptor, PairObservation,
    PairSeed, PopulationPhase, TreatmentArm,
};

/// Stable revision of the application-owned live choreography bytes.
pub const CHOREOGRAPHY_REVISION: &str = "cl-001-live-choreography-v1";

const TASK_ASSIGNMENT_REVISION: &[u8] = b"cl-001|task-assignment|v1";
const SOURCE_EXPOSURE_START: i64 = 1;

/// An exact Forum visibility frontier assigned to one actor occurrence.
///
/// The generic admission command records this value against the obligation;
/// this application type records why the value belongs to the source,
/// retained, or reset choreography.  `0` and negative ordinals are rejected
/// so a reset successor can never be represented as having access to a
/// pre-history range by arithmetic underflow.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ForumExposure {
    visible_from_message_ordinal: i64,
}

impl ForumExposure {
    /// The canonical source exposure.  A newly opened thread starts at 1.
    pub const fn source() -> Self {
        Self {
            visible_from_message_ordinal: SOURCE_EXPOSURE_START,
        }
    }

    /// The retained successor exposure.  It includes the frozen source
    /// history, including the correction once it is atomically released.
    pub const fn retained_successor() -> Self {
        Self::source()
    }

    /// The reset successor exposure, beginning immediately after the frozen
    /// pre-replacement head.  The correction is published at this ordinal.
    pub fn reset_successor(frozen_forum_head: i64) -> Result<Self, ChoreographyError> {
        if frozen_forum_head < 0 {
            return Err(ChoreographyError::InvalidFrozenForumHead);
        }
        let visible_from_message_ordinal = frozen_forum_head
            .checked_add(1)
            .ok_or(ChoreographyError::InvalidFrozenForumHead)?;
        Ok(Self {
            visible_from_message_ordinal,
        })
    }

    /// The exact generic Forum exposure ordinal.
    pub const fn visible_from_message_ordinal(self) -> i64 {
        self.visible_from_message_ordinal
    }
}

/// The application-owned private input carried by one task assignment.
///
/// Forum messages are deliberately absent.  A Forum role receives only its
/// typed obligation and exposure frontier; mutable peer content can enter an
/// actor only through an attributed, authorized Forum read receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrivateViewMaterial {
    EvidenceCard {
        card_ordinal: u8,
        bytes: Vec<u8>,
        digest: Blake3Digest,
    },
    Forum {
        obligation: ForumReadObligation,
        exposure: ForumExposure,
        digest: Blake3Digest,
    },
}

impl PrivateViewMaterial {
    /// The private-view digest bound to the canonical role seat.
    pub fn digest(&self) -> Blake3Digest {
        match self {
            Self::EvidenceCard { digest, .. } => *digest,
            Self::Forum { digest, .. } => *digest,
        }
    }

    /// Return the exact card bytes, if this is an observer view.
    pub fn card_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::EvidenceCard { bytes, .. } => Some(bytes),
            Self::Forum { .. } => None,
        }
    }

    /// Return the assigned Forum exposure, if this is a Forum view.
    pub const fn forum_exposure(&self) -> Option<ForumExposure> {
        match self {
            Self::EvidenceCard { .. } => None,
            Self::Forum { exposure, .. } => Some(*exposure),
        }
    }
}

/// Exact bytes a future coordinator gives to one actor's TaskAssignment
/// prompt.  The bytes are application material; the daemon remains the only
/// component allowed to seal and deliver them to a resident Pi child.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorPromptMaterial {
    phase: PopulationPhase,
    ordinal: RoleOrdinal,
    role: RoleKind,
    private_view: PrivateViewMaterial,
    system_prompt: Vec<u8>,
    task_prompt: Vec<u8>,
    system_prompt_digest: Blake3Digest,
    task_prompt_digest: Blake3Digest,
}

impl ActorPromptMaterial {
    /// Population phase of this actor assignment.
    pub const fn phase(&self) -> PopulationPhase {
        self.phase
    }

    /// Canonical role seat of this actor assignment.
    pub const fn ordinal(&self) -> RoleOrdinal {
        self.ordinal
    }

    /// Canonical role class of this actor assignment.
    pub const fn role(&self) -> RoleKind {
        self.role
    }

    /// Exact private input material.  Forum messages are not part of it.
    pub const fn private_view(&self) -> &PrivateViewMaterial {
        &self.private_view
    }

    /// Digest of the complete private view, excluding mutable Forum content.
    pub fn private_view_digest(&self) -> Blake3Digest {
        self.private_view.digest()
    }

    /// The immutable F0 awareness plus application role fragment bytes.
    pub fn system_prompt(&self) -> &[u8] {
        &self.system_prompt
    }

    /// The exact task assignment bytes, including the private view digest and
    /// Forum exposure frontier where applicable.
    pub fn task_prompt(&self) -> &[u8] {
        &self.task_prompt
    }

    pub const fn system_prompt_digest(&self) -> Blake3Digest {
        self.system_prompt_digest
    }

    pub const fn task_prompt_digest(&self) -> Blake3Digest {
        self.task_prompt_digest
    }
}

/// A pair's application-owned schedule.  Generic IDs are intentionally not
/// present: the resident coordinator obtains those IDs from accepted generic
/// transitions and keeps the mapping at its own trust boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairChoreography {
    pair_id: AnalysisPairId,
    seed_digest: Blake3Digest,
    descriptor_digest: Blake3Digest,
    retained: ArmChoreography,
    reset: ArmChoreography,
}

impl PairChoreography {
    pub(crate) fn new(descriptor: &LiveRunDescriptor, pair: &PairSeed) -> Self {
        Self {
            pair_id: pair.pair_id().clone(),
            seed_digest: pair.seed_digest(),
            descriptor_digest: descriptor.sealed_digest(),
            retained: ArmChoreography::new(TreatmentArm::Retained),
            reset: ArmChoreography::new(TreatmentArm::Reset),
        }
    }

    pub fn pair_id(&self) -> &AnalysisPairId {
        &self.pair_id
    }

    pub const fn seed_digest(&self) -> Blake3Digest {
        self.seed_digest
    }

    pub const fn descriptor_digest(&self) -> Blake3Digest {
        self.descriptor_digest
    }

    pub const fn retained(&self) -> &ArmChoreography {
        &self.retained
    }

    pub const fn reset(&self) -> &ArmChoreography {
        &self.reset
    }
}

/// One arm's fixed choreography.  Both arms have the same source exposure;
/// only their successor frontier is treatment-dependent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArmChoreography {
    treatment: TreatmentArm,
    source_exposure: ForumExposure,
}

impl ArmChoreography {
    const fn new(treatment: TreatmentArm) -> Self {
        Self {
            treatment,
            source_exposure: ForumExposure::source(),
        }
    }

    pub const fn treatment(&self) -> TreatmentArm {
        self.treatment
    }

    pub const fn source_exposure(&self) -> ForumExposure {
        self.source_exposure
    }

    /// Resolve the successor frontier after the generic control plane has
    /// frozen this arm's source Forum head.
    pub fn successor_exposure(
        &self,
        frozen_forum_head: i64,
    ) -> Result<ForumExposure, ChoreographyError> {
        match self.treatment {
            TreatmentArm::Retained => Ok(ForumExposure::retained_successor()),
            TreatmentArm::Reset => ForumExposure::reset_successor(frozen_forum_head),
        }
    }
}

/// Application-visible arm lifecycle.  This is a coordinator state record,
/// not a replacement for the durable generic episode state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArmLifecycle {
    Planned,
    SourceFrozen,
    SuccessorExposed,
    CorrectionReleased,
    SuccessorClosed,
    Invalidated,
}

/// State facts the application choreography needs to enforce its paired
/// barriers.  Generic transition receipts remain the source of truth for
/// durable recovery and analysis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArmStateRecord {
    treatment: TreatmentArm,
    lifecycle: ArmLifecycle,
    frozen_forum_head: Option<i64>,
    successor_exposure: Option<ForumExposure>,
}

impl ArmStateRecord {
    const fn planned(treatment: TreatmentArm) -> Self {
        Self {
            treatment,
            lifecycle: ArmLifecycle::Planned,
            frozen_forum_head: None,
            successor_exposure: None,
        }
    }

    pub const fn treatment(self) -> TreatmentArm {
        self.treatment
    }

    pub const fn lifecycle(self) -> ArmLifecycle {
        self.lifecycle
    }

    pub const fn frozen_forum_head(self) -> Option<i64> {
        self.frozen_forum_head
    }

    pub const fn successor_exposure(self) -> Option<ForumExposure> {
        self.successor_exposure
    }

    pub fn mark_source_frozen(&mut self, frozen_forum_head: i64) -> Result<(), ChoreographyError> {
        if self.lifecycle != ArmLifecycle::Planned || frozen_forum_head < 0 {
            return Err(ChoreographyError::InvalidArmTransition);
        }
        self.frozen_forum_head = Some(frozen_forum_head);
        self.lifecycle = ArmLifecycle::SourceFrozen;
        Ok(())
    }

    pub fn mark_successor_exposed(
        &mut self,
        exposure: ForumExposure,
    ) -> Result<(), ChoreographyError> {
        if self.lifecycle != ArmLifecycle::SourceFrozen {
            return Err(ChoreographyError::InvalidArmTransition);
        }
        if self.treatment == TreatmentArm::Reset
            && Some(exposure)
                != self
                    .frozen_forum_head
                    .and_then(|head| ForumExposure::reset_successor(head).ok())
        {
            return Err(ChoreographyError::ExposureDoesNotMatchTreatment);
        }
        if self.treatment == TreatmentArm::Retained
            && exposure != ForumExposure::retained_successor()
        {
            return Err(ChoreographyError::ExposureDoesNotMatchTreatment);
        }
        self.successor_exposure = Some(exposure);
        self.lifecycle = ArmLifecycle::SuccessorExposed;
        Ok(())
    }

    fn release_correction(&mut self) -> Result<(), ChoreographyError> {
        if self.lifecycle != ArmLifecycle::SuccessorExposed {
            return Err(ChoreographyError::CorrectionBarrierNotReady);
        }
        self.lifecycle = ArmLifecycle::CorrectionReleased;
        Ok(())
    }

    pub fn mark_successor_closed(&mut self) -> Result<(), ChoreographyError> {
        if self.lifecycle != ArmLifecycle::CorrectionReleased {
            return Err(ChoreographyError::InvalidArmTransition);
        }
        self.lifecycle = ArmLifecycle::SuccessorClosed;
        Ok(())
    }

    pub fn invalidate(&mut self) {
        self.lifecycle = ArmLifecycle::Invalidated;
    }
}

/// Paired state record with an explicit atomic correction barrier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairStateRecord {
    pair_id: AnalysisPairId,
    seed_digest: Blake3Digest,
    descriptor_digest: Blake3Digest,
    retained: ArmStateRecord,
    reset: ArmStateRecord,
    correction_released: bool,
}

impl PairStateRecord {
    pub fn planned(choreography: &PairChoreography) -> Self {
        Self {
            pair_id: choreography.pair_id.clone(),
            seed_digest: choreography.seed_digest,
            descriptor_digest: choreography.descriptor_digest,
            retained: ArmStateRecord::planned(TreatmentArm::Retained),
            reset: ArmStateRecord::planned(TreatmentArm::Reset),
            correction_released: false,
        }
    }

    pub fn pair_id(&self) -> &AnalysisPairId {
        &self.pair_id
    }

    pub const fn seed_digest(&self) -> Blake3Digest {
        self.seed_digest
    }

    pub const fn descriptor_digest(&self) -> Blake3Digest {
        self.descriptor_digest
    }

    pub const fn retained(&self) -> &ArmStateRecord {
        &self.retained
    }

    pub const fn reset(&self) -> &ArmStateRecord {
        &self.reset
    }

    pub const fn correction_released(&self) -> bool {
        self.correction_released
    }

    pub fn mark_source_frozen(
        &mut self,
        treatment: TreatmentArm,
        frozen_forum_head: i64,
    ) -> Result<(), ChoreographyError> {
        self.arm_mut(treatment)
            .mark_source_frozen(frozen_forum_head)
    }

    pub fn mark_successor_exposed(
        &mut self,
        treatment: TreatmentArm,
        exposure: ForumExposure,
    ) -> Result<(), ChoreographyError> {
        self.arm_mut(treatment).mark_successor_exposed(exposure)
    }

    /// Mark both arms as having received the one atomic correction release.
    /// This method intentionally has no per-arm variant.
    pub fn release_matched_correction(&mut self) -> Result<(), ChoreographyError> {
        if self.correction_released
            || self.retained.lifecycle != ArmLifecycle::SuccessorExposed
            || self.reset.lifecycle != ArmLifecycle::SuccessorExposed
        {
            return Err(ChoreographyError::CorrectionBarrierNotReady);
        }
        self.retained.release_correction()?;
        self.reset.release_correction()?;
        self.correction_released = true;
        Ok(())
    }

    pub fn mark_successor_closed(
        &mut self,
        treatment: TreatmentArm,
    ) -> Result<(), ChoreographyError> {
        self.arm_mut(treatment).mark_successor_closed()
    }

    pub fn invalidate(&mut self) {
        self.retained.invalidate();
        self.reset.invalidate();
    }

    fn arm_mut(&mut self, treatment: TreatmentArm) -> &mut ArmStateRecord {
        match treatment {
            TreatmentArm::Retained => &mut self.retained,
            TreatmentArm::Reset => &mut self.reset,
        }
    }
}

/// Application-owned outcome extraction for one completed generic pair.
///
/// The generic query supplies only closed, typed observations.  This wrapper
/// binds those measurements to the pre-registered human-stable pair label so
/// a coordinator cannot accidentally analyze a numeric pair under the wrong
/// world seed or arm name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairOutcome {
    pair_id: AnalysisPairId,
    retained: ArmAnalysisObservation,
    reset: ArmAnalysisObservation,
    provenance: crate::PairProvenance,
}

impl PairOutcome {
    pub fn from_persisted_pair(
        pair_id: AnalysisPairId,
        pair: &PersistedStudyPairObservation,
    ) -> Result<Self, ChoreographyError> {
        let mut observation = PairObservation::from_persisted_study_pair(pair)
            .map_err(ChoreographyError::Analysis)?;
        observation.pair_id = pair_id.clone();
        let provenance = observation
            .provenance
            .clone()
            .ok_or(ChoreographyError::Analysis(
                AnalysisInputError::IncompletePersistedPair,
            ))?;
        Ok(Self {
            pair_id,
            retained: observation.retained,
            reset: observation.reset,
            provenance,
        })
    }

    pub fn pair_id(&self) -> &AnalysisPairId {
        &self.pair_id
    }

    pub const fn retained(&self) -> &ArmAnalysisObservation {
        &self.retained
    }

    pub const fn reset(&self) -> &ArmAnalysisObservation {
        &self.reset
    }

    pub const fn provenance(&self) -> &crate::PairProvenance {
        &self.provenance
    }

    pub fn into_pair_observation(self) -> PairObservation {
        PairObservation {
            pair_id: self.pair_id,
            retained: self.retained,
            reset: self.reset,
            provenance: Some(self.provenance),
        }
    }
}

/// Failure to build or advance application choreography.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChoreographyError {
    InvalidRoleSeat,
    RoleContractMismatch,
    InvalidFrozenForumHead,
    InvalidArmTransition,
    ExposureDoesNotMatchTreatment,
    CorrectionBarrierNotReady,
    Analysis(AnalysisInputError),
}

impl std::fmt::Display for ChoreographyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRoleSeat => "role seat is not in the canonical CL-001 topology",
            Self::RoleContractMismatch => {
                "sealed role seat does not match the canonical role contract"
            }
            Self::InvalidFrozenForumHead => "frozen Forum head is invalid",
            Self::InvalidArmTransition => "CL-001 arm choreography transition is invalid",
            Self::ExposureDoesNotMatchTreatment => {
                "Forum exposure does not match the treatment arm"
            }
            Self::CorrectionBarrierNotReady => {
                "both paired successor exposures are not ready for correction release"
            }
            Self::Analysis(error) => return write!(formatter, "analysis input rejected: {error}"),
        })
    }
}

impl std::error::Error for ChoreographyError {}

impl LiveRunDescriptor {
    /// Build the exact actor prompt material for a role seat and exposure.
    /// Corresponding retained/reset prompts are byte-identical because this
    /// method receives no treatment label.
    pub fn actor_prompt(
        &self,
        phase: PopulationPhase,
        ordinal: RoleOrdinal,
        exposure: ForumExposure,
    ) -> Result<ActorPromptMaterial, ChoreographyError> {
        let seat = match phase {
            PopulationPhase::Source => self
                .source_roles()
                .iter()
                .copied()
                .find(|seat| seat.ordinal() == ordinal),
            PopulationPhase::Successor => self
                .successor_roles()
                .iter()
                .copied()
                .find(|seat| seat.ordinal() == ordinal),
        }
        .ok_or(ChoreographyError::InvalidRoleSeat)?;
        let specification =
            RoleSpecification::canonical(ordinal).ok_or(ChoreographyError::InvalidRoleSeat)?;
        if specification.kind() != seat.role()
            || specification.private_view_kind() != seat.private_view()
            || specification.prompt_fragment().digest() != seat.role_prompt_digest()
            || seat.forum_prompt_digest() != forum_f0_awareness_digest()
            || seat.forum_tool_digest() != forum_f0_tool_contract_digest()
        {
            return Err(ChoreographyError::RoleContractMismatch);
        }

        let fixture = WorldFixture::canonical();
        let private_view = match specification
            .private_view(&fixture)
            .map_err(|_| ChoreographyError::RoleContractMismatch)?
        {
            view if matches!(view.kind(), PrivateViewKind::EvidenceCard { .. }) => {
                let card = view.card().ok_or(ChoreographyError::RoleContractMismatch)?;
                PrivateViewMaterial::EvidenceCard {
                    card_ordinal: card.ordinal(),
                    bytes: card.bytes().to_vec(),
                    digest: card.digest(),
                }
            }
            view => PrivateViewMaterial::Forum {
                obligation: view
                    .forum_obligation()
                    .ok_or(ChoreographyError::RoleContractMismatch)?,
                exposure,
                digest: view.digest(),
            },
        };
        let role_prompt = specification.prompt_fragment().bytes();
        let mut system_prompt =
            Vec::with_capacity(society_pi::FORUM_F0_AWARENESS_BYTES.len() + role_prompt.len() + 2);
        system_prompt.extend_from_slice(society_pi::FORUM_F0_AWARENESS_BYTES);
        system_prompt.extend_from_slice(b"\n\n");
        system_prompt.extend_from_slice(role_prompt);

        let mut task_prompt = Vec::with_capacity(256);
        task_prompt.extend_from_slice(TASK_ASSIGNMENT_REVISION);
        task_prompt.push(0);
        put_phase(&mut task_prompt, phase);
        task_prompt.push(0);
        task_prompt.push(ordinal.value());
        task_prompt.push(0);
        put_digest(&mut task_prompt, private_view.digest());
        task_prompt.push(0);
        put_i64(&mut task_prompt, exposure.visible_from_message_ordinal());
        task_prompt.push(0);
        match &private_view {
            PrivateViewMaterial::EvidenceCard {
                card_ordinal,
                bytes,
                digest,
            } => {
                task_prompt.push(1);
                task_prompt.push(*card_ordinal);
                put_digest(&mut task_prompt, *digest);
                put_bytes(&mut task_prompt, bytes);
            }
            PrivateViewMaterial::Forum { obligation, .. } => {
                task_prompt.push(2);
                task_prompt.push(forum_obligation_tag(*obligation));
            }
        }

        Ok(ActorPromptMaterial {
            phase,
            ordinal,
            role: seat.role(),
            private_view,
            system_prompt_digest: Blake3Digest::of_bytes(&system_prompt),
            task_prompt_digest: Blake3Digest::of_bytes(&task_prompt),
            system_prompt,
            task_prompt,
        })
    }

    /// Expand one pre-registered pair into the two treatment arm schedules.
    pub fn pair_choreography(&self, pair: &PairSeed) -> PairChoreography {
        PairChoreography::new(self, pair)
    }
}

impl crate::LiveRunPlan {
    /// Expand every pre-registered pair in ordinal order for a coordinator.
    pub fn choreographies(&self) -> Vec<PairChoreography> {
        self.pairs()
            .iter()
            .map(|pair| self.descriptor().pair_choreography(pair))
            .collect()
    }
}

fn put_phase(bytes: &mut Vec<u8>, phase: PopulationPhase) {
    bytes.push(match phase {
        PopulationPhase::Source => 1,
        PopulationPhase::Successor => 2,
    });
}

fn put_digest(bytes: &mut Vec<u8>, digest: Blake3Digest) {
    bytes.extend_from_slice(&digest.as_bytes());
}

fn put_i64(bytes: &mut Vec<u8>, value: i64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn put_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}

fn forum_obligation_tag(obligation: ForumReadObligation) -> u8 {
    match obligation {
        ForumReadObligation::ChallengerOne => 1,
        ForumReadObligation::ChallengerTwo => 2,
        ForumReadObligation::Synthesis => 3,
        ForumReadObligation::Decision => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActorPolicyIdentity, Cl001Metric, LiveRunPlan, PrecisionTarget};
    use society_kernel::{
        StudyEpisodeId, StudyEpisodeObservation, StudyEpisodeState, StudyInstitutionRevisionId,
        StudyMeasurementObservation, StudyMeasurementRevisionId, StudyMeasurementSlot,
        StudyMeasurementSlotCount, StudyPairId, StudyPopulationSnapshotId, StudyProtocolRevisionId,
        StudyTreatment, StudyWorldRevisionId,
    };

    fn plan() -> LiveRunPlan {
        let policy = ActorPolicyIdentity::new(
            Blake3Digest::of_bytes(b"policy"),
            Blake3Digest::of_bytes(b"runtime"),
            Blake3Digest::of_bytes(b"sampling"),
        )
        .unwrap();
        let descriptor = LiveRunDescriptor::canonical(policy).unwrap();
        LiveRunPlan::new(
            descriptor,
            vec![
                PairSeed::new("pair-01", Blake3Digest::of_bytes(b"seed-01")).unwrap(),
                PairSeed::new("pair-02", Blake3Digest::of_bytes(b"seed-02")).unwrap(),
            ],
            [PrecisionTarget::new(100).unwrap(); Cl001Metric::ALL.len()],
        )
        .unwrap()
    }

    #[test]
    fn actor_prompt_contains_only_sealed_role_private_view_and_frontier() {
        let descriptor = plan().descriptor().clone();
        let source = descriptor
            .actor_prompt(
                PopulationPhase::Source,
                RoleOrdinal::new(1).unwrap(),
                ForumExposure::source(),
            )
            .unwrap();
        assert_eq!(source.role(), RoleKind::Observer);
        assert_eq!(source.private_view().card_bytes().unwrap().len(), 57);
        assert!(
            source
                .system_prompt()
                .starts_with(society_pi::FORUM_F0_AWARENESS_BYTES)
        );
        assert!(source.task_prompt().starts_with(TASK_ASSIGNMENT_REVISION));
        assert_eq!(
            source.system_prompt_digest(),
            Blake3Digest::of_bytes(source.system_prompt())
        );
        assert_eq!(
            source.task_prompt_digest(),
            Blake3Digest::of_bytes(source.task_prompt())
        );
        assert!(
            !source
                .task_prompt()
                .windows(b"retained".len())
                .any(|window| window == b"retained")
        );
        assert!(
            !source
                .task_prompt()
                .windows(b"reset".len())
                .any(|window| window == b"reset")
        );
    }

    #[test]
    fn corresponding_arm_prompts_are_equal_and_reset_frontier_is_exact() {
        let descriptor = plan().descriptor().clone();
        let retained = descriptor
            .actor_prompt(
                PopulationPhase::Successor,
                RoleOrdinal::new(8).unwrap(),
                ForumExposure::retained_successor(),
            )
            .unwrap();
        let reset = descriptor
            .actor_prompt(
                PopulationPhase::Successor,
                RoleOrdinal::new(8).unwrap(),
                ForumExposure::reset_successor(8).unwrap(),
            )
            .unwrap();
        assert_eq!(retained.system_prompt(), reset.system_prompt());
        assert_ne!(retained.task_prompt(), reset.task_prompt());
        assert_eq!(
            reset
                .private_view()
                .forum_exposure()
                .unwrap()
                .visible_from_message_ordinal(),
            9
        );
        assert_eq!(
            retained
                .private_view()
                .forum_exposure()
                .unwrap()
                .visible_from_message_ordinal(),
            1
        );
    }

    #[test]
    fn paired_correction_requires_both_successor_exposures() {
        let pair = plan().choreographies().remove(0);
        let mut state = PairStateRecord::planned(&pair);
        assert_eq!(
            state.release_matched_correction(),
            Err(ChoreographyError::CorrectionBarrierNotReady)
        );
        state.mark_source_frozen(TreatmentArm::Retained, 8).unwrap();
        state.mark_source_frozen(TreatmentArm::Reset, 8).unwrap();
        state
            .mark_successor_exposed(TreatmentArm::Retained, ForumExposure::retained_successor())
            .unwrap();
        assert_eq!(
            state.release_matched_correction(),
            Err(ChoreographyError::CorrectionBarrierNotReady)
        );
        state
            .mark_successor_exposed(
                TreatmentArm::Reset,
                ForumExposure::reset_successor(8).unwrap(),
            )
            .unwrap();
        state.release_matched_correction().unwrap();
        assert!(state.correction_released());
        assert_eq!(
            state.retained().lifecycle(),
            ArmLifecycle::CorrectionReleased
        );
        assert_eq!(state.reset().lifecycle(), ArmLifecycle::CorrectionReleased);
    }

    #[test]
    fn reset_cannot_receive_a_pre_history_frontier() {
        let pair = plan().choreographies().remove(0);
        let mut state = PairStateRecord::planned(&pair);
        state.mark_source_frozen(TreatmentArm::Reset, 8).unwrap();
        assert_eq!(
            state.mark_successor_exposed(TreatmentArm::Reset, ForumExposure::source()),
            Err(ChoreographyError::ExposureDoesNotMatchTreatment)
        );
    }

    #[test]
    fn outcome_extraction_relabels_only_after_persisted_pair_validation() {
        let randomization_digest = Blake3Digest::of_bytes(b"pair-seed");
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
                        status: society_kernel::StudyMeasurementStatus::Observed,
                        value: Some(value),
                        value_digest: Some(Blake3Digest::of_bytes(&value.to_be_bytes())),
                        reason_digest: None,
                    }
                })
                .collect(),
        };
        let persisted = PersistedStudyPairObservation {
            pair_id: StudyPairId::new(41).unwrap(),
            retained: episode(11, StudyTreatment::Retained),
            reset: episode(12, StudyTreatment::Reset),
        };
        let pair_id = AnalysisPairId::parse("pair-01").unwrap();
        let outcome = PairOutcome::from_persisted_pair(pair_id.clone(), &persisted).unwrap();
        assert_eq!(outcome.pair_id(), &pair_id);
        assert_eq!(outcome.provenance().retained_episode_id.value(), 11);
        assert_eq!(
            outcome
                .retained()
                .value(crate::Cl001Metric::CorrectionAdoptionLatency),
            crate::MeasurementOutcome::Observed {
                value: 1,
                value_digest: Blake3Digest::of_bytes(&1_i64.to_be_bytes()),
            }
        );
        assert_eq!(outcome.into_pair_observation().pair_id.as_str(), "pair-01");
    }
}
