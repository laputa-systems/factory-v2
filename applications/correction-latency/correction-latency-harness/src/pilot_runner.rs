//! Canonical CL-001 feasibility-pilot orchestration.
//!
//! This module owns the application-side run state machine.  It never opens a
//! store, chooses a child, or interprets generic IDs.  A daemon composition
//! supplies [`PilotExecutionBackend`], whose implementation is responsible for
//! translating these typed calls into resident generic transitions and for
//! returning closed, read-only pair observations. The production adapter is
//! [`crate::ResidentPilotExecutionBackend`].
//!
//! The backend boundary is intentionally explicit. The daemon accepts only a
//! sealed study-lifetime selector and opens a TaskAttempt only from a durable
//! claim; the matching M3 allocation and native spawn material remain
//! resident-private. The backend still requires operator-supplied Root
//! Authority, cycle, generation, and epoch facts before it may provision the
//! complete 64-seat M3 manifest or start a paid pilot.

use std::{collections::BTreeSet, fmt};

use correction_latency_world::{
    parse_public_decision_record, BinaryOutcome, PublicDecisionConfidence, DECISION_ROLE_ORDINAL,
};
use society_kernel::{
    ForumMessageKind, ForumPublicationState, StudyDecisionBody, StudyForumPublicAuthor,
    StudyPopulationPhase, StudyPostActorPublicForumObservation,
};
use societyd::{Daemon, StudyAdmissionOperationId};

use crate::{
    ActorPromptMaterial, AnalysisArtifact, AnalysisInputError, ChoreographyError,
    FeasibilityPilotPlan, ForumExposure, PairObservation, PairSeed, PairStateRecord,
    PopulationPhase, PreparedLiveRunAdmission, SealedLiveRunAdmission, TreatmentArm,
};

/// Revision of the application-owned canonical pilot coordinator contract.
pub const PILOT_RUNNER_REVISION: &str = "cl-001-canonical-pilot-runner-v1";

/// One application decision accepted from a terminal actor population's
/// public Forum. The original Forum message remains cited in generic state;
/// this projection adds only CL-001's closed interpretation of its strict
/// body grammar.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicForumDecision {
    decision: StudyDecisionBody,
    obligation_id: society_kernel::StudyActorObligationId,
    cited_message_id: society_kernel::ForumMessageId,
    outcome: BinaryOutcome,
    confidence: PublicDecisionConfidence,
}

impl PublicForumDecision {
    pub fn decision(&self) -> &StudyDecisionBody {
        &self.decision
    }

    pub const fn cited_message_id(&self) -> society_kernel::ForumMessageId {
        self.cited_message_id
    }

    /// The terminal decision seat whose public declaration is cited. This
    /// generic study identity is sufficient for `RecordDecision`; it is not
    /// an M3, process, workspace, or native-child identity.
    pub const fn obligation_id(&self) -> society_kernel::StudyActorObligationId {
        self.obligation_id
    }

    pub const fn outcome(&self) -> BinaryOutcome {
        self.outcome
    }

    pub const fn confidence(&self) -> PublicDecisionConfidence {
        self.confidence
    }
}

/// A terminal population did not leave exactly one valid CL-001 public
/// decision record. This is an application analysis/input failure; it never
/// licenses a fallback to private Pi transcript interpretation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicForumDecisionError {
    Missing,
    Duplicate,
    Malformed,
}

impl fmt::Display for PublicForumDecisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("terminal public decision record is missing"),
            Self::Duplicate => formatter.write_str("terminal public decision record is duplicate"),
            Self::Malformed => formatter.write_str("terminal public decision record is malformed"),
        }
    }
}

impl std::error::Error for PublicForumDecisionError {}

/// Parse CL-001's only accepted decision declaration from a bounded
/// post-actor public Forum. The actor attribution, phase, role, publication
/// state, and message kind are checked here in addition to the world-owned
/// body grammar. Raw private prompts and Pi transcripts never enter this
/// function.
pub fn terminal_public_forum_decision(
    forum: &StudyPostActorPublicForumObservation,
    phase: PopulationPhase,
) -> Result<PublicForumDecision, PublicForumDecisionError> {
    let phase = match phase {
        PopulationPhase::Source => StudyPopulationPhase::Source,
        PopulationPhase::Successor => StudyPopulationPhase::Successor,
    };
    let mut accepted = None;
    for message in &forum.messages {
        let StudyForumPublicAuthor::Actor {
            obligation_id,
            phase: author_phase,
            role,
            ..
        } = message.author
        else {
            continue;
        };
        if author_phase != phase
            || role.value() != DECISION_ROLE_ORDINAL
            || message.kind != ForumMessageKind::Synthesis
            || message.publication_state != ForumPublicationState::Published
        {
            continue;
        }
        let record = parse_public_decision_record(message.body.as_str())
            .map_err(|_| PublicForumDecisionError::Malformed)?;
        let decision = StudyDecisionBody::parse(message.body.as_str().to_owned())
            .map_err(|_| PublicForumDecisionError::Malformed)?;
        let value = PublicForumDecision {
            decision,
            obligation_id,
            cited_message_id: message.message_id,
            outcome: record.outcome(),
            confidence: record.confidence(),
        };
        if accepted.replace(value).is_some() {
            return Err(PublicForumDecisionError::Duplicate);
        }
    }
    accepted.ok_or(PublicForumDecisionError::Missing)
}

/// Durable lifecycle expected by the application coordinator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PilotRunLifecycle {
    Prepared,
    Sealed,
    Running,
    Closed,
    Completed,
}

/// One canonical actor lifetime expanded from a sealed CL-001 role seat.
///
/// The runner derives the pair, arm, phase, role, exposure frontier, and exact
/// prompt material. A daemon backend uses those facts to select an admitted
/// obligation and native TaskAttempt; no application caller supplies an
/// attempt, child, budget, workspace, or supervisor identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorLifetime {
    pair_ordinal: usize,
    pair_id: crate::AnalysisPairId,
    treatment: TreatmentArm,
    phase: PopulationPhase,
    prompt: ActorPromptMaterial,
}

impl ActorLifetime {
    /// Stable one-based role ordinal within the canonical population.
    ///
    /// This is the only application fact a plan-scoped daemon adapter needs
    /// to join a sealed seat to its resident generic obligation.  It is not a
    /// generic actor-attempt or native-child identity.
    pub const fn role_ordinal(&self) -> u8 {
        self.prompt.ordinal().value()
    }

    /// Stable zero-based position in the complete two-pair pilot schedule.
    ///
    /// The value is derived solely from the sealed pair ordinal, treatment,
    /// phase, and canonical role ordinal.  A daemon may use it as a bounded
    /// retry/idempotency key, but must still resolve the actual obligation and
    /// TaskAttempt from its own durable plan projection.
    pub const fn schedule_index(&self) -> usize {
        let treatment_offset = match self.treatment {
            TreatmentArm::Retained => 0,
            TreatmentArm::Reset => 1,
        };
        let phase_offset = match self.phase {
            PopulationPhase::Source => 0,
            PopulationPhase::Successor => 1,
        };
        self.pair_ordinal * 32
            + treatment_offset * 16
            + phase_offset * 8
            + self.prompt.ordinal().value() as usize
            - 1
    }

    pub const fn pair_ordinal(&self) -> usize {
        self.pair_ordinal
    }

    pub fn pair_id(&self) -> &crate::AnalysisPairId {
        &self.pair_id
    }

    pub const fn treatment(&self) -> TreatmentArm {
        self.treatment
    }

    pub const fn phase(&self) -> PopulationPhase {
        self.phase
    }

    pub const fn prompt(&self) -> &ActorPromptMaterial {
        &self.prompt
    }
}

/// The narrow resident operations required by the CL-001 application runner.
///
/// Implementations must make each operation idempotent under the admission
/// operation identity.  They must not accept a treatment-specific prompt or
/// an application payload: the application passes only the pre-registered
/// pair seed, role-independent exposure frontier, and typed lifecycle fact.
pub trait PilotExecutionBackend {
    type Error: fmt::Debug;

    /// Admit the exact sealed opaque plan into the generic finite study.
    fn admit_study_run(
        &mut self,
        admission: SealedLiveRunAdmission,
        pilot: &FeasibilityPilotPlan,
    ) -> Result<(), Self::Error>;

    /// Register one plan ordinal and its pre-registered world seed.
    fn register_pair(&mut self, pair_ordinal: usize, pair: &PairSeed) -> Result<(), Self::Error>;

    /// Start the finite run after every pair registration has been accepted.
    fn start_study_run(&mut self) -> Result<(), Self::Error>;

    /// Execute one actor lifetime derived from a canonical role seat. The
    /// backend resolves the corresponding generic obligation and TaskAttempt;
    /// callers cannot supply or replace those daemon-owned identities.
    fn execute_actor_lifetime(&mut self, lifetime: &ActorLifetime) -> Result<(), Self::Error>;

    /// Freeze one arm's source Forum head and return the exact durable
    /// ordinal. The backend, not an application caller, learns that ordinal
    /// from the accepted generic `FreezeForumHead` event.
    fn freeze_source(
        &mut self,
        pair_ordinal: usize,
        treatment: TreatmentArm,
    ) -> Result<i64, Self::Error>;

    /// Admit the exact successor visibility frontier for one arm.
    fn expose_successor(
        &mut self,
        pair_ordinal: usize,
        treatment: TreatmentArm,
        exposure: ForumExposure,
    ) -> Result<(), Self::Error>;

    /// Release the one correction into both members of a matched pair.
    fn release_matched_correction(&mut self, pair_ordinal: usize) -> Result<(), Self::Error>;

    /// Close one successor arm after its correction/reveal/measurement chain.
    fn close_successor(
        &mut self,
        pair_ordinal: usize,
        treatment: TreatmentArm,
    ) -> Result<(), Self::Error>;

    /// Complete the finite generic run after every registered pair is closed.
    fn complete_study_run(&mut self) -> Result<(), Self::Error>;

    /// Return exactly one validated application observation per registered
    /// pair. The backend must read this from the closed generic boundary; it
    /// must not synthesize values from coordinator state.
    fn closed_pair_observations(&mut self) -> Result<Vec<PairObservation>, Self::Error>;
}

/// Application-owned canonical two-pair pilot coordinator.
#[derive(Debug)]
pub struct CanonicalPilotRunner {
    pilot: FeasibilityPilotPlan,
    prepared: PreparedLiveRunAdmission,
    sealed: Option<SealedLiveRunAdmission>,
    lifecycle: PilotRunLifecycle,
    pair_states: Vec<PairStateRecord>,
    executed_lifetimes: BTreeSet<(usize, u8, u8, u8)>,
    artifact: Option<AnalysisArtifact>,
}

impl CanonicalPilotRunner {
    /// Prepare the exact two-pair pilot under one daemon admission identity.
    pub fn new(
        operation: StudyAdmissionOperationId,
        pilot: FeasibilityPilotPlan,
    ) -> Result<Self, PilotRunnerError<()>> {
        let prepared = PreparedLiveRunAdmission::new(operation, pilot.live_plan())
            .map_err(PilotRunnerError::Admission)?;
        let pair_states = pilot
            .live_plan()
            .choreographies()
            .iter()
            .map(PairStateRecord::planned)
            .collect();
        Ok(Self {
            pilot,
            prepared,
            sealed: None,
            lifecycle: PilotRunLifecycle::Prepared,
            pair_states,
            executed_lifetimes: BTreeSet::new(),
            artifact: None,
        })
    }

    pub const fn lifecycle(&self) -> PilotRunLifecycle {
        self.lifecycle
    }

    pub const fn pilot(&self) -> &FeasibilityPilotPlan {
        &self.pilot
    }

    pub const fn prepared(&self) -> &PreparedLiveRunAdmission {
        &self.prepared
    }

    pub const fn sealed(&self) -> Option<SealedLiveRunAdmission> {
        self.sealed
    }

    pub fn pair_state(
        &self,
        pair_ordinal: usize,
    ) -> Result<&PairStateRecord, PilotRunnerError<()>> {
        self.pair_states
            .get(pair_ordinal)
            .ok_or(PilotRunnerError::PairOrdinal(pair_ordinal))
    }

    /// Physically seal the complete plan under resident immutable-content
    /// custody. No generic run registration is possible before this succeeds.
    pub fn seal_into(&mut self, daemon: &mut Daemon) -> Result<(), PilotRunnerError<()>> {
        if self.lifecycle != PilotRunLifecycle::Prepared {
            return Err(PilotRunnerError::InvalidLifecycle {
                expected: PilotRunLifecycle::Prepared,
                actual: self.lifecycle,
            });
        }
        let sealed = self
            .prepared
            .seal_into(daemon)
            .map_err(PilotRunnerError::Admission)?;
        if sealed.plan_digest() != self.pilot.live_plan().sealed_digest()
            || sealed.pair_count().value() as usize != self.pilot.live_plan().pairs().len()
        {
            return Err(PilotRunnerError::SealedPlanMismatch);
        }
        self.sealed = Some(sealed);
        self.lifecycle = PilotRunLifecycle::Sealed;
        Ok(())
    }

    /// Start the pilot only after the daemon has accepted the sealed plan and
    /// all two pre-registered pair seeds in ordinal order.
    pub fn start<B: PilotExecutionBackend>(
        &mut self,
        backend: &mut B,
    ) -> Result<(), PilotRunnerError<B::Error>> {
        if self.lifecycle != PilotRunLifecycle::Sealed {
            return Err(PilotRunnerError::InvalidLifecycle {
                expected: PilotRunLifecycle::Sealed,
                actual: self.lifecycle,
            });
        }
        let admission = self.sealed.ok_or(PilotRunnerError::SealedPlanMissing)?;
        backend
            .admit_study_run(admission, &self.pilot)
            .map_err(PilotRunnerError::Backend)?;
        for (ordinal, pair) in self.pilot.live_plan().pairs().iter().enumerate() {
            backend
                .register_pair(ordinal, pair)
                .map_err(PilotRunnerError::Backend)?;
        }
        backend
            .start_study_run()
            .map_err(PilotRunnerError::Backend)?;
        self.lifecycle = PilotRunLifecycle::Running;
        Ok(())
    }

    /// Execute all eight canonical actor lifetimes for one arm and phase.
    /// Source lifetimes must precede source freeze; successor lifetimes must
    /// follow the matched correction release. The method is retry-stable at
    /// role granularity if a backend reports an error partway through a phase.
    pub fn execute_population<B: PilotExecutionBackend>(
        &mut self,
        backend: &mut B,
        pair_ordinal: usize,
        treatment: TreatmentArm,
        phase: PopulationPhase,
    ) -> Result<(), PilotRunnerError<B::Error>> {
        self.require_running()?;
        let state = self.pair_state(pair_ordinal).map_err(cast_unit_error)?;
        match phase {
            PopulationPhase::Source
                if state.lifecycle(treatment) != crate::ArmLifecycle::Planned =>
            {
                return Err(PilotRunnerError::Choreography(
                    ChoreographyError::InvalidArmTransition,
                ));
            }
            PopulationPhase::Successor
                if state.lifecycle(treatment) != crate::ArmLifecycle::CorrectionReleased =>
            {
                return Err(PilotRunnerError::Choreography(
                    ChoreographyError::InvalidArmTransition,
                ));
            }
            _ => {}
        }
        for lifetime in self
            .planned_actor_lifetimes(pair_ordinal, treatment, phase)
            .map_err(cast_unit_error)?
        {
            let key = lifetime_key(pair_ordinal, treatment, phase, lifetime.role_ordinal());
            if self.executed_lifetimes.contains(&key) {
                continue;
            }
            backend
                .execute_actor_lifetime(&lifetime)
                .map_err(PilotRunnerError::Backend)?;
            self.executed_lifetimes.insert(key);
        }
        Ok(())
    }

    /// Expand one exact population into immutable application descriptors.
    ///
    /// This is the plan-scoped handoff consumed by a daemon adapter.  It
    /// contains no generic obligation, work-item, actor-attempt, workspace,
    /// native-child, budget-reservation, or supervisor identity.  A resident
    /// backend must resolve those identities from its sealed run projection;
    /// an adapter must not manufacture a 64-entry app-local substitute.
    pub fn planned_actor_lifetimes(
        &self,
        pair_ordinal: usize,
        treatment: TreatmentArm,
        phase: PopulationPhase,
    ) -> Result<Vec<ActorLifetime>, PilotRunnerError<()>> {
        let choreography = self
            .pilot
            .live_plan()
            .choreographies()
            .into_iter()
            .nth(pair_ordinal)
            .ok_or(PilotRunnerError::PairOrdinal(pair_ordinal))?;
        let state = self.pair_state(pair_ordinal).map_err(cast_unit_error)?;
        let exposure =
            match phase {
                PopulationPhase::Source => choreography.retained().source_exposure(),
                PopulationPhase::Successor => state.arm(treatment).successor_exposure().ok_or(
                    PilotRunnerError::Choreography(ChoreographyError::InvalidArmTransition),
                )?,
            };
        let descriptor = self.pilot.live_plan().descriptor();
        let pair_id = self.pilot.live_plan().pairs()[pair_ordinal]
            .pair_id()
            .clone();
        let seats = match phase {
            PopulationPhase::Source => descriptor.source_roles(),
            PopulationPhase::Successor => descriptor.successor_roles(),
        };
        seats
            .iter()
            .map(|seat| {
                let prompt = descriptor
                    .actor_prompt(phase, seat.ordinal(), exposure)
                    .map_err(PilotRunnerError::Choreography)?;
                Ok(ActorLifetime {
                    pair_ordinal,
                    pair_id: pair_id.clone(),
                    treatment,
                    phase,
                    prompt,
                })
            })
            .collect()
    }

    /// Freeze one arm's source head. The backend owns the durable freeze and
    /// the application records only the typed barrier fact.
    pub fn freeze_source<B: PilotExecutionBackend>(
        &mut self,
        backend: &mut B,
        pair_ordinal: usize,
        treatment: TreatmentArm,
    ) -> Result<i64, PilotRunnerError<B::Error>> {
        self.require_running()?;
        let state = self.pair_state(pair_ordinal).map_err(cast_unit_error)?;
        if state.lifecycle(treatment) != crate::ArmLifecycle::Planned {
            return Err(PilotRunnerError::Choreography(
                ChoreographyError::InvalidArmTransition,
            ));
        }
        if !self.population_complete(pair_ordinal, treatment, PopulationPhase::Source) {
            return Err(PilotRunnerError::Choreography(
                ChoreographyError::InvalidArmTransition,
            ));
        }
        let frozen_forum_head = backend
            .freeze_source(pair_ordinal, treatment)
            .map_err(PilotRunnerError::Backend)?;
        self.pair_state_mut(pair_ordinal)
            .map_err(cast_unit_error)?
            .mark_source_frozen(treatment, frozen_forum_head)
            .map_err(PilotRunnerError::Choreography)?;
        Ok(frozen_forum_head)
    }

    /// Expose a successor only after both arms' source heads are frozen. The
    /// exposure is derived from the treatment and the recorded frozen head;
    /// callers cannot supply a substituted frontier.
    pub fn expose_successor<B: PilotExecutionBackend>(
        &mut self,
        backend: &mut B,
        pair_ordinal: usize,
        treatment: TreatmentArm,
    ) -> Result<ForumExposure, PilotRunnerError<B::Error>> {
        self.require_running()?;
        let choreography = self
            .pilot
            .live_plan()
            .choreographies()
            .into_iter()
            .nth(pair_ordinal)
            .ok_or(PilotRunnerError::PairOrdinal(pair_ordinal))?;
        let state = self.pair_state(pair_ordinal).map_err(cast_unit_error)?;
        let other_treatment = match treatment {
            TreatmentArm::Retained => TreatmentArm::Reset,
            TreatmentArm::Reset => TreatmentArm::Retained,
        };
        if state.lifecycle(treatment) != crate::ArmLifecycle::SourceFrozen
            || !matches!(
                state.lifecycle(other_treatment),
                crate::ArmLifecycle::SourceFrozen | crate::ArmLifecycle::SuccessorExposed
            )
        {
            return Err(PilotRunnerError::Choreography(
                ChoreographyError::InvalidArmTransition,
            ));
        }
        let frozen_head =
            state
                .arm(treatment)
                .frozen_forum_head()
                .ok_or(PilotRunnerError::Choreography(
                    ChoreographyError::InvalidArmTransition,
                ))?;
        let exposure = match treatment {
            TreatmentArm::Retained => choreography.retained().successor_exposure(frozen_head),
            TreatmentArm::Reset => choreography.reset().successor_exposure(frozen_head),
        }
        .map_err(PilotRunnerError::Choreography)?;
        backend
            .expose_successor(pair_ordinal, treatment, exposure)
            .map_err(PilotRunnerError::Backend)?;
        self.pair_state_mut(pair_ordinal)
            .map_err(cast_unit_error)?
            .mark_successor_exposed(treatment, exposure)
            .map_err(PilotRunnerError::Choreography)?;
        Ok(exposure)
    }

    /// Release the one matched correction only after both successor frontiers
    /// are exposed. The backend must use one generic atomic release command.
    pub fn release_matched_correction<B: PilotExecutionBackend>(
        &mut self,
        backend: &mut B,
        pair_ordinal: usize,
    ) -> Result<(), PilotRunnerError<B::Error>> {
        self.require_running()?;
        let state = self.pair_state(pair_ordinal).map_err(cast_unit_error)?;
        if state.retained().lifecycle() != crate::ArmLifecycle::SuccessorExposed
            || state.reset().lifecycle() != crate::ArmLifecycle::SuccessorExposed
        {
            return Err(PilotRunnerError::Choreography(
                ChoreographyError::CorrectionBarrierNotReady,
            ));
        }
        backend
            .release_matched_correction(pair_ordinal)
            .map_err(PilotRunnerError::Backend)?;
        self.pair_state_mut(pair_ordinal)
            .map_err(cast_unit_error)?
            .release_matched_correction()
            .map_err(PilotRunnerError::Choreography)
    }

    /// Close one successor arm. A pair cannot be completed until both arms
    /// have passed this gate.
    pub fn close_successor<B: PilotExecutionBackend>(
        &mut self,
        backend: &mut B,
        pair_ordinal: usize,
        treatment: TreatmentArm,
    ) -> Result<(), PilotRunnerError<B::Error>> {
        self.require_running()?;
        let state = self.pair_state(pair_ordinal).map_err(cast_unit_error)?;
        if state.lifecycle(treatment) != crate::ArmLifecycle::CorrectionReleased {
            return Err(PilotRunnerError::Choreography(
                ChoreographyError::InvalidArmTransition,
            ));
        }
        if !self.population_complete(pair_ordinal, treatment, PopulationPhase::Successor) {
            return Err(PilotRunnerError::Choreography(
                ChoreographyError::InvalidArmTransition,
            ));
        }
        backend
            .close_successor(pair_ordinal, treatment)
            .map_err(PilotRunnerError::Backend)?;
        self.pair_state_mut(pair_ordinal)
            .map_err(cast_unit_error)?
            .mark_successor_closed(treatment)
            .map_err(PilotRunnerError::Choreography)
    }

    /// Complete the generic finite run and construct the preregistered
    /// analysis artifact from closed backend observations. There is no method
    /// which returns an artifact before this gate succeeds.
    pub fn complete<B: PilotExecutionBackend>(
        &mut self,
        backend: &mut B,
    ) -> Result<&AnalysisArtifact, PilotRunnerError<B::Error>> {
        self.require_running()?;
        if self.pair_states.iter().any(|state| {
            state.retained().lifecycle() != crate::ArmLifecycle::SuccessorClosed
                || state.reset().lifecycle() != crate::ArmLifecycle::SuccessorClosed
        }) {
            return Err(PilotRunnerError::Choreography(
                ChoreographyError::InvalidArmTransition,
            ));
        }
        backend
            .complete_study_run()
            .map_err(PilotRunnerError::Backend)?;
        self.lifecycle = PilotRunLifecycle::Closed;
        let observations = backend
            .closed_pair_observations()
            .map_err(PilotRunnerError::Backend)?;
        let artifact = AnalysisArtifact::from_preregistered_plan(
            self.pilot.live_plan().analysis_plan().clone(),
            observations,
        )
        .map_err(PilotRunnerError::Analysis)?;
        self.artifact = Some(artifact);
        self.lifecycle = PilotRunLifecycle::Completed;
        Ok(self.artifact.as_ref().expect("artifact just stored"))
    }

    pub fn artifact(&self) -> Option<&AnalysisArtifact> {
        self.artifact.as_ref()
    }

    #[cfg(test)]
    fn force_running_for_test(&mut self) {
        self.lifecycle = PilotRunLifecycle::Running;
    }

    fn require_running<E>(&self) -> Result<(), PilotRunnerError<E>> {
        if self.lifecycle != PilotRunLifecycle::Running {
            Err(PilotRunnerError::InvalidLifecycle {
                expected: PilotRunLifecycle::Running,
                actual: self.lifecycle,
            })
        } else {
            Ok(())
        }
    }

    fn pair_state_mut(
        &mut self,
        pair_ordinal: usize,
    ) -> Result<&mut PairStateRecord, PilotRunnerError<()>> {
        self.pair_states
            .get_mut(pair_ordinal)
            .ok_or(PilotRunnerError::PairOrdinal(pair_ordinal))
    }

    fn population_complete(
        &self,
        pair_ordinal: usize,
        treatment: TreatmentArm,
        phase: PopulationPhase,
    ) -> bool {
        let roles = match phase {
            PopulationPhase::Source => self.pilot.live_plan().descriptor().source_roles(),
            PopulationPhase::Successor => self.pilot.live_plan().descriptor().successor_roles(),
        };
        roles.iter().all(|seat| {
            self.executed_lifetimes.contains(&lifetime_key(
                pair_ordinal,
                treatment,
                phase,
                seat.ordinal().value(),
            ))
        })
    }
}

fn lifetime_key(
    pair_ordinal: usize,
    treatment: TreatmentArm,
    phase: PopulationPhase,
    role_ordinal: u8,
) -> (usize, u8, u8, u8) {
    (
        pair_ordinal,
        match treatment {
            TreatmentArm::Retained => 1,
            TreatmentArm::Reset => 2,
        },
        match phase {
            PopulationPhase::Source => 1,
            PopulationPhase::Successor => 2,
        },
        role_ordinal,
    )
}

fn cast_unit_error<E>(error: PilotRunnerError<()>) -> PilotRunnerError<E> {
    match error {
        PilotRunnerError::Admission(error) => PilotRunnerError::Admission(error),
        PilotRunnerError::Backend(()) => unreachable!("unit backend error is never cast"),
        PilotRunnerError::Choreography(error) => PilotRunnerError::Choreography(error),
        PilotRunnerError::Analysis(error) => PilotRunnerError::Analysis(error),
        PilotRunnerError::InvalidLifecycle { expected, actual } => {
            PilotRunnerError::InvalidLifecycle { expected, actual }
        }
        PilotRunnerError::PairOrdinal(ordinal) => PilotRunnerError::PairOrdinal(ordinal),
        PilotRunnerError::SealedPlanMismatch => PilotRunnerError::SealedPlanMismatch,
        PilotRunnerError::SealedPlanMissing => PilotRunnerError::SealedPlanMissing,
    }
}

/// Errors raised before a backend can claim a canonical pilot transition.
#[derive(Debug)]
pub enum PilotRunnerError<E> {
    Admission(crate::DaemonCompositionError),
    Backend(E),
    Choreography(ChoreographyError),
    Analysis(AnalysisInputError),
    InvalidLifecycle {
        expected: PilotRunLifecycle,
        actual: PilotRunLifecycle,
    },
    PairOrdinal(usize),
    SealedPlanMismatch,
    SealedPlanMissing,
}

impl<E: fmt::Debug> fmt::Display for PilotRunnerError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admission(error) => write!(formatter, "daemon admission failed: {error}"),
            Self::Backend(error) => write!(formatter, "pilot backend failed: {error:?}"),
            Self::Choreography(error) => write!(formatter, "pilot barrier failed: {error}"),
            Self::Analysis(error) => write!(formatter, "pilot analysis rejected: {error}"),
            Self::InvalidLifecycle { expected, actual } => {
                write!(
                    formatter,
                    "pilot lifecycle requires {expected:?}, found {actual:?}"
                )
            }
            Self::PairOrdinal(ordinal) => write!(formatter, "unknown pair ordinal {ordinal}"),
            Self::SealedPlanMismatch => formatter.write_str("sealed plan does not match pilot"),
            Self::SealedPlanMissing => formatter.write_str("pilot plan has not been sealed"),
        }
    }
}

impl<E: fmt::Debug + 'static> std::error::Error for PilotRunnerError<E> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ArmAnalysisObservation, Cl001Metric, LiveRunDescriptor, LiveRunPlan, MeasurementOutcome,
        PrecisionTarget,
    };
    use society_kernel::{
        Blake3Digest, ForumMessageBody, ForumMessageId, ForumThreadId, StudyEpisodeId,
        StudyForumPublicAuthor, StudyPostActorPublicForumMessage, StudyRoleOrdinal,
        StudyInstitutionRevisionId, StudyMeasurementRevisionId, StudyMeasurementSlotCount,
        StudyPopulationSnapshotId, StudyProtocolRevisionId,
    };

    #[derive(Debug)]
    struct FakeBackend {
        calls: Vec<&'static str>,
        lifetime_keys: Vec<(usize, u8, u8, u8)>,
        observations: Vec<PairObservation>,
    }

    impl PilotExecutionBackend for FakeBackend {
        type Error = &'static str;

        fn admit_study_run(
            &mut self,
            _admission: SealedLiveRunAdmission,
            _pilot: &FeasibilityPilotPlan,
        ) -> Result<(), Self::Error> {
            self.calls.push("admit");
            Ok(())
        }

        fn register_pair(&mut self, _ordinal: usize, _pair: &PairSeed) -> Result<(), Self::Error> {
            self.calls.push("register");
            Ok(())
        }

        fn start_study_run(&mut self) -> Result<(), Self::Error> {
            self.calls.push("start");
            Ok(())
        }

        fn execute_actor_lifetime(&mut self, lifetime: &ActorLifetime) -> Result<(), Self::Error> {
            self.calls.push("actor");
            self.lifetime_keys.push(lifetime_key(
                lifetime.pair_ordinal(),
                lifetime.treatment(),
                lifetime.phase(),
                lifetime.prompt().ordinal().value(),
            ));
            Ok(())
        }

        fn freeze_source(
            &mut self,
            _pair_ordinal: usize,
            _treatment: TreatmentArm,
        ) -> Result<i64, Self::Error> {
            self.calls.push("freeze");
            Ok(8)
        }

        fn expose_successor(
            &mut self,
            _pair_ordinal: usize,
            _treatment: TreatmentArm,
            _exposure: ForumExposure,
        ) -> Result<(), Self::Error> {
            self.calls.push("expose");
            Ok(())
        }

        fn release_matched_correction(&mut self, _pair_ordinal: usize) -> Result<(), Self::Error> {
            self.calls.push("release");
            Ok(())
        }

        fn close_successor(
            &mut self,
            _pair_ordinal: usize,
            _treatment: TreatmentArm,
        ) -> Result<(), Self::Error> {
            self.calls.push("close");
            Ok(())
        }

        fn complete_study_run(&mut self) -> Result<(), Self::Error> {
            self.calls.push("complete");
            Ok(())
        }

        fn closed_pair_observations(&mut self) -> Result<Vec<PairObservation>, Self::Error> {
            self.calls.push("observations");
            Ok(self.observations.clone())
        }
    }

    fn digest(label: &str) -> Blake3Digest {
        Blake3Digest::of_bytes(label.as_bytes())
    }

    fn pilot() -> FeasibilityPilotPlan {
        let runtime = crate::CanonicalLiveRuntimeProfile::canonical(
            crate::NativeRuntimeArtifacts::new(
                society_pi::NodeRuntimeVersion::parse("v26.5.0").unwrap(),
                digest("node"),
                digest("lock"),
                digest("build"),
                digest("packages"),
                digest("catalog"),
            )
            .unwrap(),
        )
        .unwrap();
        let descriptor = LiveRunDescriptor::canonical(runtime.actor_policy()).unwrap();
        let plan = LiveRunPlan::new(
            descriptor,
            vec![
                PairSeed::new("pilot-01", digest("seed-01")).unwrap(),
                PairSeed::new("pilot-02", digest("seed-02")).unwrap(),
            ],
            [PrecisionTarget::new(100).unwrap(); Cl001Metric::ALL.len()],
        )
        .unwrap();
        FeasibilityPilotPlan::new(runtime, plan).unwrap()
    }

    fn observations(pilot: &FeasibilityPilotPlan) -> Vec<PairObservation> {
        let values = [MeasurementOutcome::Unavailable {
            reason_digest: digest("missing"),
        }; Cl001Metric::ALL.len()];
        pilot
            .live_plan()
            .pairs()
            .iter()
            .map(|pair| {
                let provenance = crate::PairProvenance {
                    retained_episode_id: StudyEpisodeId::new(1).unwrap(),
                    reset_episode_id: StudyEpisodeId::new(2).unwrap(),
                    protocol_revision_id: StudyProtocolRevisionId::new(3).unwrap(),
                    world_revision_id: society_kernel::StudyWorldRevisionId::new(4).unwrap(),
                    measurement_revision_id: StudyMeasurementRevisionId::new(5).unwrap(),
                    measurement_slot_count: StudyMeasurementSlotCount::new(11).unwrap(),
                    institution_revision_id: StudyInstitutionRevisionId::new(6).unwrap(),
                    retained_source_population_snapshot_id: StudyPopulationSnapshotId::new(7)
                        .unwrap(),
                    reset_source_population_snapshot_id: StudyPopulationSnapshotId::new(8).unwrap(),
                    retained_successor_population_snapshot_id: StudyPopulationSnapshotId::new(9)
                        .unwrap(),
                    reset_successor_population_snapshot_id: StudyPopulationSnapshotId::new(10)
                        .unwrap(),
                    randomization_digest: pair.seed_digest(),
                    retained_frozen_forum_head: 8,
                    reset_frozen_forum_head: 8,
                    retained_ground_truth_reveal_digest: digest("truth-r"),
                    reset_ground_truth_reveal_digest: digest("truth-x"),
                };
                PairObservation {
                    pair_id: pair.pair_id().clone(),
                    retained: ArmAnalysisObservation::from_values(values),
                    reset: ArmAnalysisObservation::from_values(values),
                    provenance: Some(provenance),
                }
            })
            .collect()
    }

    fn public_decision_for_test(
        body: &str,
        phase: StudyPopulationPhase,
    ) -> StudyPostActorPublicForumObservation {
        let body = ForumMessageBody::parse(body).unwrap();
        StudyPostActorPublicForumObservation {
            episode_id: StudyEpisodeId::new(1).unwrap(),
            messages: vec![StudyPostActorPublicForumMessage {
                message_id: ForumMessageId::new(2).unwrap(),
                thread_id: ForumThreadId::new(3).unwrap(),
                thread_message_ordinal: 4,
                author: StudyForumPublicAuthor::Actor {
                    obligation_id: society_kernel::StudyActorObligationId::new(6).unwrap(),
                    occurrence_id: society_kernel::ActorOccurrenceId::new(5).unwrap(),
                    phase,
                    role: StudyRoleOrdinal::new(DECISION_ROLE_ORDINAL).unwrap(),
                },
                kind: ForumMessageKind::Synthesis,
                body_digest: body.digest(),
                body,
                publication_state: ForumPublicationState::Published,
            }],
        }
    }

    #[test]
    fn terminal_public_decision_requires_attributed_strict_synthesis() {
        let forum = public_decision_for_test(
            "cl-001|decision-record|v2|outcome=1|confidence=high",
            StudyPopulationPhase::Successor,
        );
        let decision = terminal_public_forum_decision(&forum, PopulationPhase::Successor)
            .expect("strict attributed Synthesis must be accepted");
        assert_eq!(decision.outcome(), BinaryOutcome::One);
        assert_eq!(decision.confidence(), PublicDecisionConfidence::High);
        assert_eq!(decision.cited_message_id(), ForumMessageId::new(2).unwrap());
        assert_eq!(
            decision.obligation_id(),
            society_kernel::StudyActorObligationId::new(6).unwrap()
        );
        assert_eq!(
            terminal_public_forum_decision(&forum, PopulationPhase::Source),
            Err(PublicForumDecisionError::Missing)
        );
        let malformed = public_decision_for_test(
            "the model seems confident that outcome=1",
            StudyPopulationPhase::Successor,
        );
        assert_eq!(
            terminal_public_forum_decision(&malformed, PopulationPhase::Successor),
            Err(PublicForumDecisionError::Malformed)
        );
    }

    #[test]
    fn runner_rejects_successor_exposure_until_both_source_heads_are_frozen() {
        let pilot = pilot();
        let mut runner = CanonicalPilotRunner::new(
            StudyAdmissionOperationId::parse("cl001-pilot-test").unwrap(),
            pilot,
        )
        .unwrap();
        runner.force_running_for_test();
        let mut backend = FakeBackend {
            calls: Vec::new(),
            lifetime_keys: Vec::new(),
            observations: Vec::new(),
        };
        runner
            .execute_population(
                &mut backend,
                0,
                TreatmentArm::Retained,
                PopulationPhase::Source,
            )
            .unwrap();
        runner
            .freeze_source(&mut backend, 0, TreatmentArm::Retained)
            .unwrap();
        assert!(matches!(
            runner.expose_successor(&mut backend, 0, TreatmentArm::Retained),
            Err(PilotRunnerError::Choreography(
                ChoreographyError::InvalidArmTransition
            ))
        ));
    }

    #[test]
    fn runner_emits_analysis_only_after_both_pairs_close() {
        let pilot = pilot();
        let expected_observations = observations(&pilot);
        let mut runner = CanonicalPilotRunner::new(
            StudyAdmissionOperationId::parse("cl001-pilot-test").unwrap(),
            pilot,
        )
        .unwrap();
        runner.force_running_for_test();
        let mut backend = FakeBackend {
            calls: Vec::new(),
            lifetime_keys: Vec::new(),
            observations: expected_observations,
        };

        for pair_ordinal in 0..2 {
            runner
                .execute_population(
                    &mut backend,
                    pair_ordinal,
                    TreatmentArm::Retained,
                    PopulationPhase::Source,
                )
                .unwrap();
            runner
                .execute_population(
                    &mut backend,
                    pair_ordinal,
                    TreatmentArm::Reset,
                    PopulationPhase::Source,
                )
                .unwrap();
            runner
                .freeze_source(&mut backend, pair_ordinal, TreatmentArm::Retained)
                .unwrap();
            runner
                .freeze_source(&mut backend, pair_ordinal, TreatmentArm::Reset)
                .unwrap();
            assert_eq!(
                runner
                    .expose_successor(&mut backend, pair_ordinal, TreatmentArm::Retained)
                    .unwrap(),
                ForumExposure::retained_successor()
            );
            assert_eq!(
                runner
                    .expose_successor(&mut backend, pair_ordinal, TreatmentArm::Reset)
                    .unwrap(),
                ForumExposure::reset_successor(8).unwrap()
            );
            runner
                .release_matched_correction(&mut backend, pair_ordinal)
                .unwrap();
            runner
                .execute_population(
                    &mut backend,
                    pair_ordinal,
                    TreatmentArm::Retained,
                    PopulationPhase::Successor,
                )
                .unwrap();
            runner
                .execute_population(
                    &mut backend,
                    pair_ordinal,
                    TreatmentArm::Reset,
                    PopulationPhase::Successor,
                )
                .unwrap();
            runner
                .close_successor(&mut backend, pair_ordinal, TreatmentArm::Retained)
                .unwrap();
            runner
                .close_successor(&mut backend, pair_ordinal, TreatmentArm::Reset)
                .unwrap();
        }

        assert!(runner.artifact().is_none());
        let pair_count = runner
            .complete(&mut backend)
            .unwrap()
            .plan
            .as_ref()
            .unwrap()
            .pair_ids
            .len();
        assert_eq!(runner.lifecycle(), PilotRunLifecycle::Completed);
        assert_eq!(pair_count, 2);
        assert_eq!(backend.calls.last(), Some(&"observations"));
        let expected_lifetimes = (0..2)
            .flat_map(|pair_ordinal| {
                [TreatmentArm::Retained, TreatmentArm::Reset]
                    .into_iter()
                    .flat_map(move |treatment| {
                        [PopulationPhase::Source, PopulationPhase::Successor]
                            .into_iter()
                            .flat_map(move |phase| {
                                (1..=8).map(move |role_ordinal| {
                                    lifetime_key(pair_ordinal, treatment, phase, role_ordinal)
                                })
                            })
                    })
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(backend.lifetime_keys.len(), 64);
        assert_eq!(
            backend
                .lifetime_keys
                .iter()
                .copied()
                .collect::<BTreeSet<_>>(),
            expected_lifetimes
        );
        let mut schedule_indexes = backend
            .lifetime_keys
            .iter()
            .map(|(pair, treatment, phase, role)| {
                let treatment = if *treatment == 1 {
                    TreatmentArm::Retained
                } else {
                    TreatmentArm::Reset
                };
                let phase = if *phase == 1 {
                    PopulationPhase::Source
                } else {
                    PopulationPhase::Successor
                };
                *pair * 32
                    + if treatment == TreatmentArm::Retained {
                        0
                    } else {
                        16
                    }
                    + if phase == PopulationPhase::Source {
                        0
                    } else {
                        8
                    }
                    + usize::from(*role)
                    - 1
            })
            .collect::<Vec<_>>();
        schedule_indexes.sort_unstable();
        assert_eq!(schedule_indexes, (0..64).collect::<Vec<_>>());
    }
}
