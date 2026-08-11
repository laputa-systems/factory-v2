//! Resident-backed execution of CL-001's canonical two-pair pilot.
//!
//! [`CanonicalPilotRunner`](crate::CanonicalPilotRunner) owns the experiment's
//! schedule. This adapter is its one concrete implementation boundary: it
//! translates that already-fixed schedule into closed generic study commands,
//! root-owned M3 seat provisioning, and the resident TaskAttempt lifecycle.
//! It deliberately does not select a model, build a Pi spawn request, expose
//! a child handle, or let application code name an M3 work item.
//!
//! Constructing this type has no provider side effect. The only method which
//! can reach a provider is `execute_actor_lifetime`, after a sealed run,
//! durable root allocation, and claim-gated task launch have all succeeded.

use std::fmt;

use correction_latency_world::WorldFixture;
use society_kernel::{
    ActorConfigurationName, ActorModelPolicy, ApplicationRevisionId, Blake3Digest,
    DevelopmentalAttractor,
    EpisodeForumId, ForumMessageBody, ForumMessageKind, ForumPublicationState, ForumThreadId,
    ForumThreadTitle, OperatingCycleId, PrincipalDisplayName, PrincipalId,
    ProjectMilestoneName, ProjectName, ProjectNorthStarAlignment,
    ProjectNorthStarBoundaryCommitmentAnswer, ProjectNorthStarChangeAnswer,
    ProjectNorthStarImprovementEvidenceAnswer, ProjectNorthStarRevisitAnswer,
    ProjectObjectiveText, ProjectStopConditionText, StudyActorObligationId, StudyBudgetUnits,
    StudyCommand, StudyEpisodeId, StudyEvent, StudyGroundTruthReveal,
    StudyMeasurementSlot, StudyMeasurementStatus, StudyPairId, StudyPopulationPhase,
    StudyPopulationSnapshotId, StudyProtocolRevisionId, StudyRoleOrdinal, StudyRunId,
    StudyRunPairOrdinal, StudyTreatment, StudyTransitionDisposition, SupervisorEpochId,
    SupervisorEpochIdentity, TicketAcceptanceConditionText, TicketTitle, UsdMicros,
    WorkAssignmentText,
};
use societyd::{
    Daemon, RootStudyM3Project, RootStudyM3ProvisioningError,
    RootStudyM3ProvisioningOperationId, RootStudyM3ProvisioningPlan, RootStudyM3Seat,
    StudyAdmissionContentSlot, StudyAdmissionError, StudyAdmissionOperationId,
    StudyPlanLifetimeKey,
};

use crate::{
    terminal_public_forum_decision, ActorLifetime, FeasibilityPilotPlan,
    ForumExposure, LiveRunDescriptor, LiveRunPlan, NativeTaskAttemptDriver, NativeTaskDriveError,
    PairObservation, PairSeed, PilotExecutionBackend, PopulationPhase, PublicForumDecision,
    PublicForumDecisionError, SealedLiveRunAdmission, TreatmentArm,
};

/// Revision of the concrete CL-001 resident composition contract.
pub const RESIDENT_PILOT_BACKEND_REVISION: &str = "cl-001-resident-pilot-backend-v1";

const ROOT_PROJECT_NAME: &str = "Finite sealed study work";
const ROOT_PROJECT_OBJECTIVE: &str = "Execute the admitted finite study seats.";
const ROOT_PROJECT_MILESTONE: &str = "Execute sealed study seats";
const ROOT_PROJECT_STOP: &str = "Every allocated sealed study seat is terminal.";
const ROOT_CONFIGURATION_NAME: &str = "Pinned study actor configuration";
const ROOT_TICKET_ACCEPTANCE: &str = "The assigned sealed study task is terminal.";
const ROOT_ASSIGNMENT: &str = "Execute exactly one sealed bounded study task.";
const FORUM_THREAD_TITLE: &str = "CL-001 chronological discussion";

/// Operator facts required to compose the Root Authority's M3 work plan.
///
/// These are deliberately supplied separately from the CL-001 plan. The
/// application binds exact seats to the already sealed study; the operator
/// supplies the authority, operating cycle, generation, and supervisor epoch
/// under which those seats may exist. There is no fallback principal, cycle,
/// epoch, or spend value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentPilotM3Authority {
    root_authority: PrincipalId,
    operating_cycle_id: OperatingCycleId,
    expected_generation: society_kernel::AdmissionGeneration,
    supervisor_epoch_id: SupervisorEpochId,
    supervisor_epoch_identity: SupervisorEpochIdentity,
    application_revision_id: ApplicationRevisionId,
    provisioning_operation: RootStudyM3ProvisioningOperationId,
}

impl ResidentPilotM3Authority {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        root_authority: PrincipalId,
        operating_cycle_id: OperatingCycleId,
        expected_generation: society_kernel::AdmissionGeneration,
        supervisor_epoch_id: SupervisorEpochId,
        supervisor_epoch_identity: SupervisorEpochIdentity,
        application_revision_id: ApplicationRevisionId,
        provisioning_operation: RootStudyM3ProvisioningOperationId,
    ) -> Self {
        Self {
            root_authority,
            operating_cycle_id,
            expected_generation,
            supervisor_epoch_id,
            supervisor_epoch_identity,
            application_revision_id,
            provisioning_operation,
        }
    }
}

/// Closed failures at the application/daemon composition seam.
#[derive(Debug)]
pub enum ResidentPilotBackendError {
    Admission(StudyAdmissionError),
    RootProvisioning(RootStudyM3ProvisioningError),
    NativeTask(NativeTaskDriveError),
    RuntimeProfile(societyd::PinnedPiProfileError),
    Decision(PublicForumDecisionError),
    World,
    InvalidLifecycle,
    InvalidPairOrdinal,
    InvalidLifetime,
    InvalidEvent(&'static str),
    Rejected,
    InvalidStaticValue,
    M3SeatCount,
    ExposureMismatch,
    Analysis(crate::AnalysisInputError),
}

impl fmt::Display for ResidentPilotBackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admission(error) => write!(formatter, "study admission failed: {error}"),
            Self::RootProvisioning(error) => write!(formatter, "root M3 provisioning failed: {error}"),
            Self::NativeTask(error) => write!(formatter, "native study task failed: {error}"),
            Self::RuntimeProfile(error) => write!(formatter, "pinned study runtime profile rejected: {error}"),
            Self::Decision(error) => write!(formatter, "public decision record failed: {error}"),
            Self::World => formatter.write_str("the canonical world could not evaluate a decision"),
            Self::InvalidLifecycle => formatter.write_str("canonical pilot lifecycle is not ready"),
            Self::InvalidPairOrdinal => formatter.write_str("canonical pilot pair ordinal is invalid"),
            Self::InvalidLifetime => formatter.write_str("actor lifetime does not match the admitted canonical seat"),
            Self::InvalidEvent(name) => write!(formatter, "expected accepted study event {name}"),
            Self::Rejected => formatter.write_str("the generic study transition was rejected"),
            Self::InvalidStaticValue => formatter.write_str("a fixed CL-001 value was rejected"),
            Self::M3SeatCount => formatter.write_str("root M3 provisioning did not admit every canonical seat"),
            Self::ExposureMismatch => formatter.write_str("successor Forum exposure differs from its frozen treatment frontier"),
            Self::Analysis(error) => write!(formatter, "closed pilot analysis projection failed: {error}"),
        }
    }
}

impl std::error::Error for ResidentPilotBackendError {}

impl From<StudyAdmissionError> for ResidentPilotBackendError {
    fn from(value: StudyAdmissionError) -> Self {
        Self::Admission(value)
    }
}

impl From<RootStudyM3ProvisioningError> for ResidentPilotBackendError {
    fn from(value: RootStudyM3ProvisioningError) -> Self {
        Self::RootProvisioning(value)
    }
}

impl From<NativeTaskDriveError> for ResidentPilotBackendError {
    fn from(value: NativeTaskDriveError) -> Self {
        Self::NativeTask(value)
    }
}

impl From<societyd::PinnedPiProfileError> for ResidentPilotBackendError {
    fn from(value: societyd::PinnedPiProfileError) -> Self {
        Self::RuntimeProfile(value)
    }
}

impl From<PublicForumDecisionError> for ResidentPilotBackendError {
    fn from(value: PublicForumDecisionError) -> Self {
        Self::Decision(value)
    }
}

impl From<crate::AnalysisInputError> for ResidentPilotBackendError {
    fn from(value: crate::AnalysisInputError) -> Self {
        Self::Analysis(value)
    }
}

#[derive(Clone, Debug)]
struct ArmState {
    episode_id: StudyEpisodeId,
    forum_id: EpisodeForumId,
    thread_id: ForumThreadId,
    source_obligations: Vec<StudyActorObligationId>,
    successor_obligations: Vec<StudyActorObligationId>,
    frozen_head: Option<i64>,
}

#[derive(Clone, Debug)]
struct PairState {
    application_pair: PairSeed,
    generic_pair_id: StudyPairId,
    retained: ArmState,
    reset: ArmState,
}

/// The concrete backend for one finite, sealed CL-001 pilot.
///
/// The borrow makes the daemon the single writer. This coordinator carries
/// only generic returned IDs and the sealed application plan used to validate
/// them; it never retains a PostgreSQL handle or private native material.
pub struct ResidentPilotExecutionBackend<'daemon> {
    daemon: &'daemon mut Daemon,
    admission_operation: StudyAdmissionOperationId,
    m3_authority: ResidentPilotM3Authority,
    next_sequence: u32,
    admission: Option<SealedLiveRunAdmission>,
    plan: Option<LiveRunPlan>,
    protocol_revision_id: Option<StudyProtocolRevisionId>,
    study_run_id: Option<StudyRunId>,
    actor_model_policy: Option<ActorModelPolicy>,
    seat_reservation: Option<UsdMicros>,
    pairs: Vec<PairState>,
    m3_provisioned: bool,
    task_driver: NativeTaskAttemptDriver,
}

impl<'daemon> ResidentPilotExecutionBackend<'daemon> {
    /// Create an inert backend. This is not a provider call and does not
    /// create any generic work. `CanonicalPilotRunner::start` performs the
    /// first study transition only after its plan was physically sealed.
    pub fn new(
        daemon: &'daemon mut Daemon,
        admission_operation: StudyAdmissionOperationId,
        m3_authority: ResidentPilotM3Authority,
    ) -> Self {
        Self {
            daemon,
            admission_operation,
            m3_authority,
            next_sequence: 1,
            admission: None,
            plan: None,
            protocol_revision_id: None,
            study_run_id: None,
            actor_model_policy: None,
            seat_reservation: None,
            pairs: Vec::new(),
            m3_provisioned: false,
            task_driver: NativeTaskAttemptDriver::canonical(),
        }
    }

    fn transition(&mut self, command: StudyCommand) -> Result<StudyEvent, ResidentPilotBackendError> {
        let sequence = self.next_sequence;
        let receipt = {
            let mut admission = self.daemon.open_study_admission(self.admission_operation.clone())?;
            admission.transition(sequence, command)?
        };
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(ResidentPilotBackendError::InvalidLifecycle)?;
        match receipt.disposition {
            StudyTransitionDisposition::Accepted(event) => Ok(event),
            StudyTransitionDisposition::Rejected(_) => Err(ResidentPilotBackendError::Rejected),
        }
    }

    fn seal_prompt(
        &mut self,
        schedule_index: usize,
        bytes: &[u8],
    ) -> Result<societyd::SealedStudyContent, ResidentPilotBackendError> {
        let slot = StudyAdmissionContentSlot::parse(format!("task-{schedule_index:02}"))
            .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?;
        let mut admission = self.daemon.open_study_admission(self.admission_operation.clone())?;
        admission.seal_content(slot, bytes).map_err(Into::into)
    }

    fn plan(&self) -> Result<&LiveRunPlan, ResidentPilotBackendError> {
        self.plan.as_ref().ok_or(ResidentPilotBackendError::InvalidLifecycle)
    }

    fn descriptor(&self) -> Result<&LiveRunDescriptor, ResidentPilotBackendError> {
        Ok(self.plan()?.descriptor())
    }

    fn run_id(&self) -> Result<StudyRunId, ResidentPilotBackendError> {
        self.study_run_id.ok_or(ResidentPilotBackendError::InvalidLifecycle)
    }

    fn generic_pair_ordinal(pair_ordinal: usize) -> Result<StudyRunPairOrdinal, ResidentPilotBackendError> {
        let value = pair_ordinal
            .checked_add(1)
            .and_then(|value| u16::try_from(value).ok())
            .and_then(StudyRunPairOrdinal::new)
            .ok_or(ResidentPilotBackendError::InvalidPairOrdinal)?;
        Ok(value)
    }

    fn pair(&self, pair_ordinal: usize) -> Result<&PairState, ResidentPilotBackendError> {
        self.pairs.get(pair_ordinal).ok_or(ResidentPilotBackendError::InvalidPairOrdinal)
    }

    fn arm(&self, pair_ordinal: usize, treatment: TreatmentArm) -> Result<&ArmState, ResidentPilotBackendError> {
        let pair = self.pair(pair_ordinal)?;
        Ok(match treatment {
            TreatmentArm::Retained => &pair.retained,
            TreatmentArm::Reset => &pair.reset,
        })
    }

    fn arm_mut(
        &mut self,
        pair_ordinal: usize,
        treatment: TreatmentArm,
    ) -> Result<&mut ArmState, ResidentPilotBackendError> {
        let pair = self
            .pairs
            .get_mut(pair_ordinal)
            .ok_or(ResidentPilotBackendError::InvalidPairOrdinal)?;
        Ok(match treatment {
            TreatmentArm::Retained => &mut pair.retained,
            TreatmentArm::Reset => &mut pair.reset,
        })
    }

    fn study_treatment(treatment: TreatmentArm) -> StudyTreatment {
        match treatment {
            TreatmentArm::Retained => StudyTreatment::Retained,
            TreatmentArm::Reset => StudyTreatment::Reset,
        }
    }

    fn study_phase(phase: PopulationPhase) -> StudyPopulationPhase {
        match phase {
            PopulationPhase::Source => StudyPopulationPhase::Source,
            PopulationPhase::Successor => StudyPopulationPhase::Successor,
        }
    }

    fn role_obligation(
        &self,
        lifetime: &ActorLifetime,
    ) -> Result<StudyActorObligationId, ResidentPilotBackendError> {
        let obligations = match lifetime.phase() {
            PopulationPhase::Source => &self.arm(lifetime.pair_ordinal(), lifetime.treatment())?.source_obligations,
            PopulationPhase::Successor => &self.arm(lifetime.pair_ordinal(), lifetime.treatment())?.successor_obligations,
        };
        let position = usize::from(lifetime.role_ordinal())
            .checked_sub(1)
            .ok_or(ResidentPilotBackendError::InvalidLifetime)?;
        obligations
            .get(position)
            .copied()
            .ok_or(ResidentPilotBackendError::InvalidLifecycle)
    }

    fn obligation_state(
        &mut self,
        episode_id: StudyEpisodeId,
        obligation_id: StudyActorObligationId,
    ) -> Result<society_kernel::StudyActorObligationState, ResidentPilotBackendError> {
        let admission = self
            .daemon
            .open_study_admission(self.admission_operation.clone())?;
        admission
            .study_actor_obligation_observations(episode_id)?
            .into_iter()
            .find(|obligation| obligation.obligation_id == obligation_id)
            .map(|obligation| obligation.lifecycle_state)
            .ok_or(ResidentPilotBackendError::InvalidLifetime)
    }

    fn admit_roles(
        &mut self,
        episode_id: StudyEpisodeId,
        phase: PopulationPhase,
        roles: &[crate::ActorSeatContract],
    ) -> Result<Vec<StudyActorObligationId>, ResidentPilotBackendError> {
        let mut obligations = Vec::with_capacity(roles.len());
        for seat in roles {
            let budget = StudyBudgetUnits::new(
                i64::try_from(seat.budget().actor_budget_units())
                    .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
            )
            .ok_or(ResidentPilotBackendError::InvalidStaticValue)?;
            let read_budget = society_kernel::ForumReadBudget::new(
                i64::try_from(seat.budget().forum_read_budget())
                    .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
            )
                .ok_or(ResidentPilotBackendError::InvalidStaticValue)?;
            let post_budget = society_kernel::ForumPostBudget::new(
                i64::try_from(seat.budget().forum_post_budget())
                    .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
            )
                .ok_or(ResidentPilotBackendError::InvalidStaticValue)?;
            let event = self.transition(StudyCommand::AdmitActorObligation {
                episode_id,
                phase: Self::study_phase(phase),
                role: StudyRoleOrdinal::new(seat.ordinal().value())
                    .ok_or(ResidentPilotBackendError::InvalidStaticValue)?,
                private_view_digest: seat.private_view_digest(),
                prompt_digest: seat.forum_prompt_digest(),
                tool_digest: seat.forum_tool_digest(),
                budget,
                read_budget,
                post_budget,
            })?;
            match event {
                StudyEvent::ActorObligationAdmitted {
                    obligation_id,
                    episode_id: actual_episode_id,
                    phase: actual_phase,
                    ..
                } if actual_episode_id == episode_id && actual_phase == Self::study_phase(phase) => {
                    obligations.push(obligation_id)
                }
                _ => return Err(ResidentPilotBackendError::InvalidEvent("ActorObligationAdmitted")),
            }
        }
        Ok(obligations)
    }

    fn admit_exposure(
        &mut self,
        forum_id: EpisodeForumId,
        obligations: &[StudyActorObligationId],
        visible_from_message_ordinal: i64,
    ) -> Result<(), ResidentPilotBackendError> {
        for obligation_id in obligations {
            let event = self.transition(StudyCommand::AdmitForumExposure {
                obligation_id: *obligation_id,
                forum_id,
                visible_from_message_ordinal,
            })?;
            if !matches!(event, StudyEvent::ForumExposureAdmitted { obligation_id: actual, visible_from_message_ordinal: actual_visible, .. } if actual == *obligation_id && actual_visible == visible_from_message_ordinal) {
                return Err(ResidentPilotBackendError::InvalidEvent("ForumExposureAdmitted"));
            }
        }
        Ok(())
    }

    fn provision_m3(&mut self) -> Result<(), ResidentPilotBackendError> {
        if self.m3_provisioned {
            return Ok(());
        }
        let run_id = self.run_id()?;
        let plan = self.plan()?.clone();
        let reservation = self
            .seat_reservation
            .ok_or(ResidentPilotBackendError::InvalidLifecycle)?;
        let actor_model_policy = self
            .actor_model_policy
            .ok_or(ResidentPilotBackendError::InvalidLifecycle)?;
        let seats = canonical_m3_seats(&plan, reservation)?;
        let project = RootStudyM3Project::new(
            ProjectName::parse(ROOT_PROJECT_NAME).map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
            ProjectNorthStarAlignment {
                application_revision_id: self.m3_authority.application_revision_id,
                change_answer: ProjectNorthStarChangeAnswer::parse("Execute the declared finite treatment.")
                    .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
                improvement_evidence_answer: ProjectNorthStarImprovementEvidenceAnswer::parse("Retain exact closed study observations.")
                    .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
                boundary_commitment_answer: ProjectNorthStarBoundaryCommitmentAnswer::parse("No study actor receives root authority.")
                    .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
                revisit_answer: ProjectNorthStarRevisitAnswer::parse("Revisit after the preregistered pilot closes.")
                    .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
            },
            ProjectObjectiveText::parse(ROOT_PROJECT_OBJECTIVE)
                .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
            ProjectMilestoneName::parse(ROOT_PROJECT_MILESTONE)
                .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
            ProjectStopConditionText::parse(ROOT_PROJECT_STOP)
                .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
        );
        let provision = RootStudyM3ProvisioningPlan::new(
            run_id,
            self.m3_authority.operating_cycle_id,
            self.m3_authority.expected_generation,
            self.m3_authority.supervisor_epoch_id,
            self.m3_authority.supervisor_epoch_identity.clone(),
            project,
            ActorConfigurationName::parse(ROOT_CONFIGURATION_NAME)
                .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
            actor_model_policy,
            DevelopmentalAttractor::Measure,
            seats,
        )?;
        let expected_count = provision.seats().len();
        let receipt = self
            .daemon
            .open_root_study_m3_provisioning(
                self.m3_authority.root_authority,
                self.m3_authority.provisioning_operation.clone(),
            )?
            .provision(&provision)?;
        if receipt.admitted_seat_count() != expected_count {
            return Err(ResidentPilotBackendError::M3SeatCount);
        }
        self.m3_provisioned = true;
        Ok(())
    }

    fn record_terminal_decision(
        &mut self,
        arm: &ArmState,
        phase: PopulationPhase,
    ) -> Result<
        (
            Option<PublicForumDecision>,
            society_kernel::StudyPostActorPublicForumObservation,
        ),
        ResidentPilotBackendError,
    > {
        let forum = {
            let admission = self.daemon.open_study_admission(self.admission_operation.clone())?;
            admission.post_actor_public_forum(arm.episode_id)?
        };
        let decision = match terminal_public_forum_decision(&forum, phase) {
            Ok(decision) => decision,
            // The strict application grammar is an analysis-input contract,
            // not a license to fabricate a decision. Close with explicit
            // unavailable measurement slots when the public declaration is
            // missing, duplicate, or malformed.
            Err(
                PublicForumDecisionError::Missing
                | PublicForumDecisionError::Duplicate
                | PublicForumDecisionError::Malformed,
            ) => return Ok((None, forum)),
        };
        let event = self.transition(StudyCommand::RecordDecision {
            obligation_id: decision.obligation_id(),
            decision: decision.decision().clone(),
            cited_message_id: Some(decision.cited_message_id()),
        })?;
        if !matches!(event, StudyEvent::DecisionRecorded { obligation_id } if obligation_id == decision.obligation_id()) {
            return Err(ResidentPilotBackendError::InvalidEvent("DecisionRecorded"));
        }
        Ok((Some(decision), forum))
    }

    fn record_measurements(
        &mut self,
        arm: &ArmState,
        successor: Option<&PublicForumDecision>,
        forum: &society_kernel::StudyPostActorPublicForumObservation,
    ) -> Result<(), ResidentPilotBackendError> {
        if let Some(successor) = successor {
            let correction_ordinal = forum
            .messages
            .iter()
            .find(|message| {
                message.author == society_kernel::StudyForumPublicAuthor::SocietyService
                    && message.kind == ForumMessageKind::Correction
                    && message.publication_state == ForumPublicationState::Published
                    && message.body_digest == WorldFixture::canonical().correction_package().digest()
            })
            .map(|message| message.thread_message_ordinal)
                .ok_or(ResidentPilotBackendError::InvalidEvent("published correction"))?;
            let decision_ordinal = forum
            .messages
            .iter()
            .find(|message| message.message_id == successor.cited_message_id())
            .map(|message| message.thread_message_ordinal)
                .ok_or(ResidentPilotBackendError::InvalidEvent("published successor decision"))?;
            let latency = decision_ordinal
            .checked_sub(correction_ordinal)
                .ok_or(ResidentPilotBackendError::InvalidEvent("correction precedes decision"))?;
            let fixture = WorldFixture::canonical();
            let correct = fixture
            .analysis_evaluator()
            .evaluate_decision(fixture.evidence(), successor.outcome())
            .map_err(|_| ResidentPilotBackendError::World)?
                .decision_correct();
            self.record_observed_measurement(arm.episode_id, 1, latency, "correction-to-final-decision-ordinal-distance")?;
            self.record_observed_measurement(
                arm.episode_id,
                2,
                i64::from(correct),
                "post-reveal-final-decision-correctness",
            )?;
        } else {
            self.record_unavailable_measurement(
                arm.episode_id,
                1,
                "correction-to-final-decision-missing-strict-public-decision",
            )?;
            self.record_unavailable_measurement(
                arm.episode_id,
                2,
                "final-decision-correctness-missing-strict-public-decision",
            )?;
        }
        for (slot, name) in [
            (3, "false-claim-persistence"),
            (4, "correction-visibility"),
            (5, "dissent-survival"),
            (6, "forum-history-utilization"),
            (7, "forum-attention-bytes"),
            (8, "forum-attention-turns"),
            (9, "forum-attention-runtime"),
            (10, "operational-cost"),
            (11, "amortized-institutional-cost"),
        ] {
            self.record_unavailable_measurement(arm.episode_id, slot, name)?;
        }
        Ok(())
    }

    fn record_observed_measurement(
        &mut self,
        episode_id: StudyEpisodeId,
        slot: u8,
        value: i64,
        derivation: &str,
    ) -> Result<(), ResidentPilotBackendError> {
        let measurement_slot = StudyMeasurementSlot::new(slot)
            .ok_or(ResidentPilotBackendError::InvalidStaticValue)?;
        let value_digest = Blake3Digest::of_bytes(
            format!(
                "{RESIDENT_PILOT_BACKEND_REVISION}|measurement|{derivation}|episode={}|slot={slot}|value={value}",
                episode_id.value(),
            )
            .as_bytes(),
        );
        let event = self.transition(StudyCommand::RecordMeasurementResult {
            episode_id,
            measurement_slot,
            status: StudyMeasurementStatus::Observed,
            value: Some(value),
            value_digest: Some(value_digest),
            reason_digest: None,
        })?;
        if !matches!(event, StudyEvent::MeasurementResultRecorded { episode_id: actual, status: StudyMeasurementStatus::Observed, .. } if actual == episode_id) {
            return Err(ResidentPilotBackendError::InvalidEvent("MeasurementResultRecorded"));
        }
        Ok(())
    }

    fn record_unavailable_measurement(
        &mut self,
        episode_id: StudyEpisodeId,
        slot: u8,
        metric: &str,
    ) -> Result<(), ResidentPilotBackendError> {
        let measurement_slot = StudyMeasurementSlot::new(slot)
            .ok_or(ResidentPilotBackendError::InvalidStaticValue)?;
        let reason_digest = Blake3Digest::of_bytes(
            format!(
                "{RESIDENT_PILOT_BACKEND_REVISION}|measurement-unavailable|{metric}|episode={}|slot={slot}|no-preregistered-public-observability-contract",
                episode_id.value(),
            )
            .as_bytes(),
        );
        let event = self.transition(StudyCommand::RecordMeasurementResult {
            episode_id,
            measurement_slot,
            status: StudyMeasurementStatus::Unavailable,
            value: None,
            value_digest: None,
            reason_digest: Some(reason_digest),
        })?;
        if !matches!(event, StudyEvent::MeasurementResultRecorded { episode_id: actual, status: StudyMeasurementStatus::Unavailable, .. } if actual == episode_id) {
            return Err(ResidentPilotBackendError::InvalidEvent("MeasurementResultRecorded"));
        }
        Ok(())
    }
}

impl PilotExecutionBackend for ResidentPilotExecutionBackend<'_> {
    type Error = ResidentPilotBackendError;

    fn admit_study_run(
        &mut self,
        admission: SealedLiveRunAdmission,
        pilot: &FeasibilityPilotPlan,
    ) -> Result<(), Self::Error> {
        if self.plan.is_some() || self.study_run_id.is_some() || self.admission.is_some() {
            return Err(ResidentPilotBackendError::InvalidLifecycle);
        }
        self.daemon
            .assert_pinned_study_runtime_profile(pilot.runtime().sealed_digest())?;
        let descriptor = pilot.live_plan().descriptor();
        let episode_budget = StudyBudgetUnits::new(
            i64::try_from(descriptor.budget().episode_budget_units())
                .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
        )
        .ok_or(ResidentPilotBackendError::InvalidStaticValue)?;
        let protocol_revision_id = match self.transition(StudyCommand::AdmitProtocolRevision {
            application_revision_id: self.m3_authority.application_revision_id,
            protocol_digest: descriptor.protocol_digest(),
            actor_policy_digest: descriptor.actor_policy().digest(),
            forum_prompt_digest: descriptor.forum_prompt_digest(),
            forum_tool_digest: descriptor.forum_tool_digest(),
            evidence_digest: descriptor.evidence_digest(),
            ground_truth_commitment_digest: descriptor.ground_truth_commitment_digest(),
            correction_digest: descriptor.correction_digest(),
            topology_digest: descriptor.role_topology_digest(),
            episode_budget,
        })? {
            StudyEvent::ProtocolRevisionAdmitted { protocol_revision_id } => protocol_revision_id,
            _ => return Err(ResidentPilotBackendError::InvalidEvent("ProtocolRevisionAdmitted")),
        };
        let world_revision_id = match self.transition(StudyCommand::AdmitWorldRevision {
            protocol_revision_id,
            world_digest: descriptor.world_digest(),
        })? {
            StudyEvent::WorldRevisionAdmitted { world_revision_id } => world_revision_id,
            _ => return Err(ResidentPilotBackendError::InvalidEvent("WorldRevisionAdmitted")),
        };
        let measurement_revision_id = match self.transition(StudyCommand::AdmitMeasurementRevision {
            protocol_revision_id,
            analysis_digest: descriptor.analysis_digest(),
            measurement_slot_count: society_kernel::StudyMeasurementSlotCount::new(descriptor.measurement_slot_count())
                .ok_or(ResidentPilotBackendError::InvalidStaticValue)?,
        })? {
            StudyEvent::MeasurementRevisionAdmitted { measurement_revision_id } => measurement_revision_id,
            _ => return Err(ResidentPilotBackendError::InvalidEvent("MeasurementRevisionAdmitted")),
        };
        let institution_revision_id = match self.transition(StudyCommand::AdmitInstitutionRevision {
            protocol_revision_id,
            institution_digest: descriptor.institution_digest(),
        })? {
            StudyEvent::InstitutionRevisionAdmitted { institution_revision_id } => institution_revision_id,
            _ => return Err(ResidentPilotBackendError::InvalidEvent("InstitutionRevisionAdmitted")),
        };
        let source_population_snapshot_id = match self.transition(StudyCommand::AdmitPopulationSnapshot {
            protocol_revision_id,
            population_digest: descriptor.population_digest(),
            population_size: i64::from(descriptor.population_size()),
        })? {
            StudyEvent::PopulationSnapshotAdmitted { population_snapshot_id } => population_snapshot_id,
            _ => return Err(ResidentPilotBackendError::InvalidEvent("PopulationSnapshotAdmitted")),
        };

        let mut pairs = Vec::with_capacity(pilot.live_plan().pairs().len());
        for pair in pilot.live_plan().pairs() {
            let retained_episode_id = self.admit_episode(
                protocol_revision_id,
                world_revision_id,
                measurement_revision_id,
                institution_revision_id,
                source_population_snapshot_id,
                pair.seed_digest(),
                StudyTreatment::Retained,
            )?;
            let reset_episode_id = self.admit_episode(
                protocol_revision_id,
                world_revision_id,
                measurement_revision_id,
                institution_revision_id,
                source_population_snapshot_id,
                pair.seed_digest(),
                StudyTreatment::Reset,
            )?;
            let generic_pair_id = match self.transition(StudyCommand::AdmitMatchedPair {
                retained_episode_id,
                reset_episode_id,
            })? {
                StudyEvent::MatchedPairAdmitted { pair_id } => pair_id,
                _ => return Err(ResidentPilotBackendError::InvalidEvent("MatchedPairAdmitted")),
            };
            let retained = self.create_source_arm(retained_episode_id, descriptor)?;
            let reset = self.create_source_arm(reset_episode_id, descriptor)?;
            pairs.push(PairState {
                application_pair: pair.clone(),
                generic_pair_id,
                retained,
                reset,
            });
        }

        let study_run_id = match self.transition(StudyCommand::AdmitStudyRun {
            protocol_revision_id,
            plan_content_object_id: admission.plan_content().content_object_id(),
            plan_digest: admission.plan_digest(),
            pair_count: admission.pair_count(),
        })? {
            StudyEvent::StudyRunAdmitted { study_run_id, .. } => study_run_id,
            _ => return Err(ResidentPilotBackendError::InvalidEvent("StudyRunAdmitted")),
        };
        self.admission = Some(admission);
        self.plan = Some(pilot.live_plan().clone());
        self.protocol_revision_id = Some(protocol_revision_id);
        self.study_run_id = Some(study_run_id);
        self.actor_model_policy = Some(pilot.runtime().generic_actor_model_policy());
        self.seat_reservation = Some(pilot.budget().actor_lifetime_cap());
        self.pairs = pairs;
        Ok(())
    }

    fn register_pair(&mut self, pair_ordinal: usize, pair: &PairSeed) -> Result<(), Self::Error> {
        let state = self.pair(pair_ordinal)?.clone();
        if &state.application_pair != pair {
            return Err(ResidentPilotBackendError::InvalidLifetime);
        }
        let event = self.transition(StudyCommand::RegisterStudyRunPair {
            study_run_id: self.run_id()?,
            pair_ordinal: Self::generic_pair_ordinal(pair_ordinal)?,
            pair_id: state.generic_pair_id,
            randomization_digest: pair.seed_digest(),
        })?;
        if !matches!(event, StudyEvent::StudyRunPairRegistered { pair_ordinal: actual, pair_id, randomization_digest, .. } if actual == Self::generic_pair_ordinal(pair_ordinal)? && pair_id == state.generic_pair_id && randomization_digest == pair.seed_digest()) {
            return Err(ResidentPilotBackendError::InvalidEvent("StudyRunPairRegistered"));
        }
        Ok(())
    }

    fn start_study_run(&mut self) -> Result<(), Self::Error> {
        self.provision_m3()?;
        let run_id = self.run_id()?;
        let event = self.transition(StudyCommand::StartStudyRun { study_run_id: run_id })?;
        if !matches!(event, StudyEvent::StudyRunStarted { study_run_id } if study_run_id == run_id) {
            return Err(ResidentPilotBackendError::InvalidEvent("StudyRunStarted"));
        }
        Ok(())
    }

    fn execute_actor_lifetime(&mut self, lifetime: &ActorLifetime) -> Result<(), Self::Error> {
        let plan = self.plan()?.clone();
        let admission = self.admission.ok_or(ResidentPilotBackendError::InvalidLifecycle)?;
        if lifetime.pair_id() != plan.pairs().get(lifetime.pair_ordinal()).ok_or(ResidentPilotBackendError::InvalidPairOrdinal)?.pair_id() {
            return Err(ResidentPilotBackendError::InvalidLifetime);
        }
        let obligation_id = self.role_obligation(lifetime)?;
        let role = StudyRoleOrdinal::new(lifetime.role_ordinal())
            .ok_or(ResidentPilotBackendError::InvalidStaticValue)?;
        let lifetime_key = StudyPlanLifetimeKey::new(
            self.run_id()?,
            admission.plan_content().content_object_id(),
            admission.plan_digest(),
            admission.pair_count(),
            Self::generic_pair_ordinal(lifetime.pair_ordinal())?,
            Self::study_treatment(lifetime.treatment()),
            Self::study_phase(lifetime.phase()),
            role,
        );
        let expected_prompt = plan
            .descriptor()
            .actor_prompt(lifetime.phase(), lifetime.prompt().ordinal(), match lifetime.phase() {
                PopulationPhase::Source => ForumExposure::source(),
                PopulationPhase::Successor => self.expected_successor_exposure(lifetime.pair_ordinal(), lifetime.treatment())?,
            })
            .map_err(|_| ResidentPilotBackendError::InvalidLifetime)?;
        if expected_prompt != *lifetime.prompt() || obligation_id.value() <= 0 {
            return Err(ResidentPilotBackendError::InvalidLifetime);
        }
        let episode_id = self.arm(lifetime.pair_ordinal(), lifetime.treatment())?.episode_id;
        match self.obligation_state(episode_id, obligation_id)? {
            society_kernel::StudyActorObligationState::Completed => return Ok(()),
            society_kernel::StudyActorObligationState::Failed => {
                return Err(ResidentPilotBackendError::InvalidLifecycle)
            }
            society_kernel::StudyActorObligationState::Active => {}
        }
        let prompt_content = self.seal_prompt(lifetime.schedule_index(), lifetime.prompt().task_prompt())?;
        let charged_budget = StudyBudgetUnits::new(
            i64::try_from(plan.descriptor().budget().actor_budget_units())
                .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
        )
        .ok_or(ResidentPilotBackendError::InvalidStaticValue)?;
        self.task_driver.execute(
            self.daemon,
            &format!("cl001-pilot-task-{:02}", lifetime.schedule_index()),
            lifetime_key,
            prompt_content,
            lifetime.prompt().task_prompt(),
            charged_budget,
        )?;
        Ok(())
    }

    fn freeze_source(&mut self, pair_ordinal: usize, treatment: TreatmentArm) -> Result<i64, Self::Error> {
        let arm = self.arm(pair_ordinal, treatment)?.clone();
        let _ = self.record_terminal_decision(&arm, PopulationPhase::Source)?;
        let event = self.transition(StudyCommand::FreezeForumHead {
            episode_id: arm.episode_id,
            thread_id: arm.thread_id,
        })?;
        let head = match event {
            StudyEvent::ForumHeadFrozen { episode_id, thread_id, head_message_ordinal }
                if episode_id == arm.episode_id && thread_id == arm.thread_id => head_message_ordinal,
            _ => return Err(ResidentPilotBackendError::InvalidEvent("ForumHeadFrozen")),
        };
        self.arm_mut(pair_ordinal, treatment)?.frozen_head = Some(head);
        Ok(head)
    }

    fn expose_successor(
        &mut self,
        pair_ordinal: usize,
        treatment: TreatmentArm,
        exposure: ForumExposure,
    ) -> Result<(), Self::Error> {
        if exposure != self.expected_successor_exposure(pair_ordinal, treatment)? {
            return Err(ResidentPilotBackendError::ExposureMismatch);
        }
        let protocol_revision_id = self
            .protocol_revision_id
            .ok_or(ResidentPilotBackendError::InvalidLifecycle)?;
        let descriptor = self.descriptor()?.clone();
        let arm = self.arm(pair_ordinal, treatment)?.clone();
        if !arm.successor_obligations.is_empty() {
            return Err(ResidentPilotBackendError::InvalidLifecycle);
        }
        let successor_population_snapshot_id = match self.transition(StudyCommand::AdmitPopulationSnapshot {
            protocol_revision_id,
            population_digest: descriptor.population_digest(),
            population_size: i64::from(descriptor.population_size()),
        })? {
            StudyEvent::PopulationSnapshotAdmitted { population_snapshot_id } => population_snapshot_id,
            _ => return Err(ResidentPilotBackendError::InvalidEvent("PopulationSnapshotAdmitted")),
        };
        let event = self.transition(StudyCommand::ReplacePopulation {
            episode_id: arm.episode_id,
            successor_population_snapshot_id,
        })?;
        if !matches!(event, StudyEvent::PopulationReplaced { episode_id, successor_population_snapshot_id: actual } if episode_id == arm.episode_id && actual == successor_population_snapshot_id) {
            return Err(ResidentPilotBackendError::InvalidEvent("PopulationReplaced"));
        }
        let obligations = self.admit_roles(arm.episode_id, PopulationPhase::Successor, descriptor.successor_roles())?;
        self.admit_exposure(arm.forum_id, &obligations, exposure.visible_from_message_ordinal())?;
        self.arm_mut(pair_ordinal, treatment)?.successor_obligations = obligations;
        Ok(())
    }

    fn release_matched_correction(&mut self, pair_ordinal: usize) -> Result<(), Self::Error> {
        let pair = self.pair(pair_ordinal)?.clone();
        let correction = ForumMessageBody::parse(
            std::str::from_utf8(WorldFixture::canonical().correction_package().bytes())
                .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
        )
        .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?;
        let event = self.transition(StudyCommand::ReleaseMatchedCorrection {
            pair_id: pair.generic_pair_id,
            retained_thread_id: pair.retained.thread_id,
            reset_thread_id: pair.reset.thread_id,
            correction: correction.clone(),
        })?;
        if !matches!(event, StudyEvent::MatchedCorrectionReleased { pair_id, body_digest, .. } if pair_id == pair.generic_pair_id && body_digest == correction.digest()) {
            return Err(ResidentPilotBackendError::InvalidEvent("MatchedCorrectionReleased"));
        }
        Ok(())
    }

    fn close_successor(&mut self, pair_ordinal: usize, treatment: TreatmentArm) -> Result<(), Self::Error> {
        let arm = self.arm(pair_ordinal, treatment)?.clone();
        let (decision, forum) = self.record_terminal_decision(&arm, PopulationPhase::Successor)?;
        let reveal = StudyGroundTruthReveal::parse(
            std::str::from_utf8(WorldFixture::canonical().analysis_ground_truth_reveal().bytes())
                .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
        )
        .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?;
        let event = self.transition(StudyCommand::RevealGroundTruth {
            episode_id: arm.episode_id,
            reveal: reveal.clone(),
        })?;
        if !matches!(event, StudyEvent::GroundTruthRevealed { episode_id, reveal_digest } if episode_id == arm.episode_id && reveal_digest == reveal.digest()) {
            return Err(ResidentPilotBackendError::InvalidEvent("GroundTruthRevealed"));
        }
        self.record_measurements(&arm, decision.as_ref(), &forum)?;
        let event = self.transition(StudyCommand::CloseEpisode { episode_id: arm.episode_id })?;
        if !matches!(event, StudyEvent::EpisodeClosed { episode_id } if episode_id == arm.episode_id) {
            return Err(ResidentPilotBackendError::InvalidEvent("EpisodeClosed"));
        }
        Ok(())
    }

    fn complete_study_run(&mut self) -> Result<(), Self::Error> {
        let run_id = self.run_id()?;
        let event = self.transition(StudyCommand::CompleteStudyRun { study_run_id: run_id })?;
        if !matches!(event, StudyEvent::StudyRunCompleted { study_run_id } if study_run_id == run_id) {
            return Err(ResidentPilotBackendError::InvalidEvent("StudyRunCompleted"));
        }
        Ok(())
    }

    fn closed_pair_observations(&mut self) -> Result<Vec<PairObservation>, Self::Error> {
        let run_id = self.run_id()?;
        let (run, pairs) = {
            let admission = self.daemon.open_study_admission(self.admission_operation.clone())?;
            let run = admission.study_run_observation(run_id)?;
            let pairs = self
                .pairs
                .iter()
                .map(|state| admission.study_pair_observation(state.generic_pair_id))
                .collect::<Result<Vec<_>, _>>()?;
            (run, pairs)
        };
        let artifact = self.plan()?.analysis_artifact_from_study_run(&run, pairs)?;
        Ok(artifact.pairs)
    }
}

impl ResidentPilotExecutionBackend<'_> {
    #[allow(clippy::too_many_arguments)]
    fn admit_episode(
        &mut self,
        protocol_revision_id: StudyProtocolRevisionId,
        world_revision_id: society_kernel::StudyWorldRevisionId,
        measurement_revision_id: society_kernel::StudyMeasurementRevisionId,
        institution_revision_id: society_kernel::StudyInstitutionRevisionId,
        population_snapshot_id: StudyPopulationSnapshotId,
        randomization_digest: Blake3Digest,
        treatment: StudyTreatment,
    ) -> Result<StudyEpisodeId, ResidentPilotBackendError> {
        let episode_id = match self.transition(StudyCommand::AdmitEpisode {
            protocol_revision_id,
            world_revision_id,
            measurement_revision_id,
            institution_revision_id,
            population_snapshot_id,
            randomization_digest,
        })? {
            StudyEvent::EpisodeAdmitted { episode_id } => episode_id,
            _ => return Err(ResidentPilotBackendError::InvalidEvent("EpisodeAdmitted")),
        };
        let event = self.transition(StudyCommand::AssignTreatment { episode_id, treatment })?;
        if !matches!(event, StudyEvent::TreatmentAssigned { episode_id: actual, treatment: actual_treatment, .. } if actual == episode_id && actual_treatment == treatment) {
            return Err(ResidentPilotBackendError::InvalidEvent("TreatmentAssigned"));
        }
        Ok(episode_id)
    }

    fn create_source_arm(
        &mut self,
        episode_id: StudyEpisodeId,
        descriptor: &LiveRunDescriptor,
    ) -> Result<ArmState, ResidentPilotBackendError> {
        let forum_id = match self.transition(StudyCommand::CreateEpisodeForum {
            episode_id,
            charter_digest: descriptor.forum_charter_digest(),
        })? {
            StudyEvent::EpisodeForumCreated { forum_id, episode_id: actual } if actual == episode_id => forum_id,
            _ => return Err(ResidentPilotBackendError::InvalidEvent("EpisodeForumCreated")),
        };
        let thread_id = match self.transition(StudyCommand::OpenForumThread {
            forum_id,
            title: ForumThreadTitle::parse(FORUM_THREAD_TITLE)
                .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
        })? {
            StudyEvent::ForumThreadOpened { thread_id, forum_id: actual } if actual == forum_id => thread_id,
            _ => return Err(ResidentPilotBackendError::InvalidEvent("ForumThreadOpened")),
        };
        let source_obligations = self.admit_roles(episode_id, PopulationPhase::Source, descriptor.source_roles())?;
        self.admit_exposure(forum_id, &source_obligations, 1)?;
        Ok(ArmState {
            episode_id,
            forum_id,
            thread_id,
            source_obligations,
            successor_obligations: Vec::new(),
            frozen_head: None,
        })
    }

    fn expected_successor_exposure(
        &self,
        pair_ordinal: usize,
        treatment: TreatmentArm,
    ) -> Result<ForumExposure, ResidentPilotBackendError> {
        let head = self
            .arm(pair_ordinal, treatment)?
            .frozen_head
            .ok_or(ResidentPilotBackendError::InvalidLifecycle)?;
        match treatment {
            TreatmentArm::Retained => Ok(ForumExposure::retained_successor()),
            TreatmentArm::Reset => ForumExposure::reset_successor(head)
                .map_err(|_| ResidentPilotBackendError::ExposureMismatch),
        }
    }
}

fn context_digest(
    plan_digest: Blake3Digest,
    pair_ordinal: StudyRunPairOrdinal,
    treatment: StudyTreatment,
    phase: StudyPopulationPhase,
    role: u8,
) -> Blake3Digest {
    Blake3Digest::of_bytes(
        format!(
            "{RESIDENT_PILOT_BACKEND_REVISION}|m3-context|{:?}|{}|{}|{}|{role}",
            plan_digest,
            pair_ordinal.value(),
            treatment as i64,
            phase as i64,
        )
        .as_bytes(),
    )
}

/// Expand the sealed pilot topology into the only permitted root M3 seats.
/// This is deliberately a pure translation so the exact allocation count and
/// unique pair/arm/phase/role topology are testable without starting a daemon
/// or a provider-backed Pi host.
fn canonical_m3_seats(
    plan: &LiveRunPlan,
    reservation: UsdMicros,
) -> Result<Vec<RootStudyM3Seat>, ResidentPilotBackendError> {
    let mut seats = Vec::with_capacity(
        plan.pairs().len()
            * plan.descriptor().treatment_arms().len()
            * 2
            * plan.descriptor().source_roles().len(),
    );
    for (pair_index, _) in plan.pairs().iter().enumerate() {
        let pair_ordinal = pair_index
            .checked_add(1)
            .and_then(|value| u16::try_from(value).ok())
            .and_then(StudyRunPairOrdinal::new)
            .ok_or(ResidentPilotBackendError::InvalidPairOrdinal)?;
        for treatment in [StudyTreatment::Retained, StudyTreatment::Reset] {
            for (phase, roles) in [
                (
                    StudyPopulationPhase::Source,
                    plan.descriptor().source_roles().as_slice(),
                ),
                (
                    StudyPopulationPhase::Successor,
                    plan.descriptor().successor_roles().as_slice(),
                ),
            ] {
                for seat in roles {
                    seats.push(RootStudyM3Seat::new(
                        pair_ordinal,
                        treatment,
                        phase,
                        StudyRoleOrdinal::new(seat.ordinal().value())
                            .ok_or(ResidentPilotBackendError::InvalidStaticValue)?,
                        context_digest(
                            plan.sealed_digest(),
                            pair_ordinal,
                            treatment,
                            phase,
                            seat.ordinal().value(),
                        ),
                        TicketTitle::parse(format!(
                            "sealed study seat {} {} {} {}",
                            pair_ordinal.value(),
                            treatment as i64,
                            phase as i64,
                            seat.ordinal().value()
                        ))
                        .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
                        TicketAcceptanceConditionText::parse(ROOT_TICKET_ACCEPTANCE)
                            .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
                        PrincipalDisplayName::parse(format!(
                            "study actor {} {} {} {}",
                            pair_ordinal.value(),
                            treatment as i64,
                            phase as i64,
                            seat.ordinal().value()
                        ))
                        .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
                        WorkAssignmentText::parse(ROOT_ASSIGNMENT)
                            .map_err(|_| ResidentPilotBackendError::InvalidStaticValue)?,
                        reservation,
                    )?);
                }
            }
        }
    }
    Ok(seats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m3_authority_retains_only_operator_governance_facts() {
        let authority = ResidentPilotM3Authority::new(
            PrincipalId::KERNEL,
            OperatingCycleId::new(1).unwrap(),
            society_kernel::AdmissionGeneration::INITIAL,
            SupervisorEpochId::new(1).unwrap(),
            SupervisorEpochIdentity::parse("test-epoch").unwrap(),
            ApplicationRevisionId::new(1).unwrap(),
            RootStudyM3ProvisioningOperationId::parse("cl001-pilot-m3").unwrap(),
        );
        assert_eq!(authority.operating_cycle_id.value(), 1);
        assert_eq!(authority.provisioning_operation.as_str(), "cl001-pilot-m3");
    }

    #[test]
    fn context_digest_changes_with_each_generic_seat() {
        let plan = Blake3Digest::of_bytes(b"sealed plan");
        let first = context_digest(
            plan,
            StudyRunPairOrdinal::new(1).unwrap(),
            StudyTreatment::Retained,
            StudyPopulationPhase::Source,
            1,
        );
        let second = context_digest(
            plan,
            StudyRunPairOrdinal::new(1).unwrap(),
            StudyTreatment::Retained,
            StudyPopulationPhase::Source,
            2,
        );
        assert_ne!(first, second);
    }

    #[test]
    fn canonical_root_manifest_has_exactly_one_allocation_for_all_64_lifetimes() {
        let descriptor = LiveRunDescriptor::canonical(
            crate::ActorPolicyIdentity::new(
                Blake3Digest::of_bytes(b"policy"),
                Blake3Digest::of_bytes(b"runtime"),
                Blake3Digest::of_bytes(b"sampling"),
            )
            .unwrap(),
        )
        .unwrap();
        let plan = LiveRunPlan::new(
            descriptor,
            vec![
                PairSeed::new("pair-01", Blake3Digest::of_bytes(b"seed-01")).unwrap(),
                PairSeed::new("pair-02", Blake3Digest::of_bytes(b"seed-02")).unwrap(),
            ],
            [crate::PrecisionTarget::new(1).unwrap(); crate::Cl001Metric::ALL.len()],
        )
        .unwrap();
        let seats = canonical_m3_seats(&plan, UsdMicros::new(3_125).unwrap()).unwrap();
        assert_eq!(seats.len(), 64);
        let project = RootStudyM3Project::new(
            ProjectName::parse(ROOT_PROJECT_NAME).unwrap(),
            ProjectNorthStarAlignment {
                application_revision_id: ApplicationRevisionId::new(1).unwrap(),
                change_answer: ProjectNorthStarChangeAnswer::parse("Change.").unwrap(),
                improvement_evidence_answer: ProjectNorthStarImprovementEvidenceAnswer::parse("Evidence.").unwrap(),
                boundary_commitment_answer: ProjectNorthStarBoundaryCommitmentAnswer::parse("Boundary.").unwrap(),
                revisit_answer: ProjectNorthStarRevisitAnswer::parse("Revisit.").unwrap(),
            },
            ProjectObjectiveText::parse(ROOT_PROJECT_OBJECTIVE).unwrap(),
            ProjectMilestoneName::parse(ROOT_PROJECT_MILESTONE).unwrap(),
            ProjectStopConditionText::parse(ROOT_PROJECT_STOP).unwrap(),
        );
        assert!(RootStudyM3ProvisioningPlan::new(
            StudyRunId::new(1).unwrap(),
            OperatingCycleId::new(1).unwrap(),
            society_kernel::AdmissionGeneration::INITIAL,
            SupervisorEpochId::new(1).unwrap(),
            SupervisorEpochIdentity::parse("test-epoch").unwrap(),
            project,
            ActorConfigurationName::parse(ROOT_CONFIGURATION_NAME).unwrap(),
            ActorModelPolicy::PinnedOpenRouterLing26FlashOff,
            DevelopmentalAttractor::Measure,
            seats,
        )
        .is_ok());
    }
}
