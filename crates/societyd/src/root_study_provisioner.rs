//! Root-owned M3 provisioning for one finite sealed study.
//!
//! This is deliberately neither a daemon scheduler nor an application command
//! API. A Root Authority explicitly supplies a closed work plan, and this
//! authority can create only the project, ticket, actor, context, work, and
//! immutable study-seat allocation required by that plan. The daemon never
//! selects a work item; it merely executes root-authorized commands and later
//! consumes the resulting allocation projection.

use std::collections::BTreeSet;

use society_kernel::{
    ActorConfigurationName, ActorConfigurationRevisionId, ActorModelPolicy, AdmissionGeneration,
    Blake3Digest, Capability, CommandBody, CommandDisposition, CommandId, ContextPackPurpose,
    DevelopmentalAttractor, EventBody, ExecutionProfileId, ExpectedGeneration,
    PrincipalDisplayName, PrincipalId, ProjectId, ProjectMilestoneName, ProjectName,
    ProjectNorthStarAlignment, ProjectObjectiveText, ProjectState, ProjectStopConditionText,
    Rejection, StudyPopulationPhase, StudyRoleOrdinal, StudyRunId, StudyRunPairOrdinal,
    StudyTreatment, SupervisorEpochId, SupervisorEpochIdentity, TicketAcceptanceConditionText,
    TicketTitle, UsdMicros, WorkAssignmentText, WorkItemKind,
};
use thiserror::Error;

use crate::{Daemon, StartupMode};

const COMMAND_PREFIX: &str = "root-study-m3-v1";
const MAX_OPERATION_BYTES: usize = 36;

/// Stable root-selected identity for one complete M3 provisioning plan.
/// Retries use the same identity, so every command below resolves through the
/// append-only ledger rather than creating a second actor or work item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootStudyM3ProvisioningOperationId(String);

impl RootStudyM3ProvisioningOperationId {
    pub fn parse(value: impl Into<String>) -> Result<Self, RootStudyM3ProvisioningError> {
        let value = value.into();
        if !valid_label(&value) {
            return Err(RootStudyM3ProvisioningError::InvalidOperationIdentity);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The Root Authority's M3 project charter. These are root-authored generic
/// governance facts; no experimental-world vocabulary enters the daemon.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootStudyM3Project {
    project_name: ProjectName,
    north_star_alignment: ProjectNorthStarAlignment,
    objective: ProjectObjectiveText,
    initial_milestone: ProjectMilestoneName,
    stop_condition: ProjectStopConditionText,
}

impl RootStudyM3Project {
    pub fn new(
        project_name: ProjectName,
        north_star_alignment: ProjectNorthStarAlignment,
        objective: ProjectObjectiveText,
        initial_milestone: ProjectMilestoneName,
        stop_condition: ProjectStopConditionText,
    ) -> Self {
        Self {
            project_name,
            north_star_alignment,
            objective,
            initial_milestone,
            stop_condition,
        }
    }
}

/// One exact generic study seat and the M3 work Root Authority chose for it.
/// The constructor intentionally omits every native identity: profile,
/// workspace, child, attempt, reservation, and launch claim are all derived
/// below the allocation boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootStudyM3Seat {
    pair_ordinal: StudyRunPairOrdinal,
    treatment: StudyTreatment,
    phase: StudyPopulationPhase,
    role: StudyRoleOrdinal,
    context_rendering_digest: Blake3Digest,
    ticket_title: TicketTitle,
    acceptance_condition: TicketAcceptanceConditionText,
    actor_display_name: PrincipalDisplayName,
    assignment: WorkAssignmentText,
    reservation_amount: UsdMicros,
}

impl RootStudyM3Seat {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pair_ordinal: StudyRunPairOrdinal,
        treatment: StudyTreatment,
        phase: StudyPopulationPhase,
        role: StudyRoleOrdinal,
        context_rendering_digest: Blake3Digest,
        ticket_title: TicketTitle,
        acceptance_condition: TicketAcceptanceConditionText,
        actor_display_name: PrincipalDisplayName,
        assignment: WorkAssignmentText,
        reservation_amount: UsdMicros,
    ) -> Result<Self, RootStudyM3ProvisioningError> {
        if reservation_amount == UsdMicros::ZERO {
            return Err(RootStudyM3ProvisioningError::ZeroSeatReservation);
        }
        Ok(Self {
            pair_ordinal,
            treatment,
            phase,
            role,
            context_rendering_digest,
            ticket_title,
            acceptance_condition,
            actor_display_name,
            assignment,
            reservation_amount,
        })
    }

    fn key(&self) -> (u16, i64, i64, u8) {
        (
            self.pair_ordinal.value(),
            self.treatment as i64,
            self.phase as i64,
            self.role.value(),
        )
    }
}

/// Complete root-approved M3 work material for one already sealed study run.
/// A plan is immutable by convention: changing any field under the same
/// operation identity encounters the command ledger's idempotency fence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootStudyM3ProvisioningPlan {
    study_run_id: StudyRunId,
    operating_cycle_id: society_kernel::OperatingCycleId,
    expected_generation: AdmissionGeneration,
    supervisor_epoch_id: SupervisorEpochId,
    supervisor_epoch_identity: SupervisorEpochIdentity,
    project: RootStudyM3Project,
    configuration_name: ActorConfigurationName,
    model_policy: ActorModelPolicy,
    primary_attractor: DevelopmentalAttractor,
    seats: Vec<RootStudyM3Seat>,
}

impl RootStudyM3ProvisioningPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        study_run_id: StudyRunId,
        operating_cycle_id: society_kernel::OperatingCycleId,
        expected_generation: AdmissionGeneration,
        supervisor_epoch_id: SupervisorEpochId,
        supervisor_epoch_identity: SupervisorEpochIdentity,
        project: RootStudyM3Project,
        configuration_name: ActorConfigurationName,
        model_policy: ActorModelPolicy,
        primary_attractor: DevelopmentalAttractor,
        seats: Vec<RootStudyM3Seat>,
    ) -> Result<Self, RootStudyM3ProvisioningError> {
        if seats.is_empty() {
            return Err(RootStudyM3ProvisioningError::EmptySeatSet);
        }
        let mut seen = BTreeSet::new();
        if seats.iter().any(|seat| !seen.insert(seat.key())) {
            return Err(RootStudyM3ProvisioningError::DuplicateStudySeat);
        }
        Ok(Self {
            study_run_id,
            operating_cycle_id,
            expected_generation,
            supervisor_epoch_id,
            supervisor_epoch_identity,
            project,
            configuration_name,
            model_policy,
            primary_attractor,
            seats,
        })
    }

    pub const fn study_run_id(&self) -> StudyRunId {
        self.study_run_id
    }

    pub fn seats(&self) -> &[RootStudyM3Seat] {
        &self.seats
    }
}

/// The only completion fact exposed by root M3 provisioning. The individual
/// project, actor, ticket, context, and work IDs remain resident-private; a
/// later application uses only its sealed lifetime selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootStudyM3ProvisioningReceipt {
    admitted_seat_count: usize,
}

impl RootStudyM3ProvisioningReceipt {
    pub const fn admitted_seat_count(self) -> usize {
        self.admitted_seat_count
    }
}

/// Errors at the root-owned finite-work provisioning boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Error)]
pub enum RootStudyM3ProvisioningError {
    #[error("root study provisioning operation identity is not canonical")]
    InvalidOperationIdentity,
    #[error("a root study provisioning plan must contain at least one seat")]
    EmptySeatSet,
    #[error("a root study seat reservation must be positive")]
    ZeroSeatReservation,
    #[error("a root study provisioning plan names one study seat more than once")]
    DuplicateStudySeat,
    #[error("the daemon is recovery-fenced")]
    RecoveryFenced,
    #[error("the root capability grant is absent or the kernel could not accept the command")]
    Kernel,
    #[error("the root provisioning command was rejected: {0:?}")]
    Rejected(Rejection),
    #[error("the kernel returned a different event than the root provisioner requested")]
    UnexpectedEvent,
    #[error("the root provisioner could not derive a stable command identity")]
    CommandIdentity,
}

/// Borrowed root-authority composition for exactly one closed M3 study-work
/// plan. Opening this authority is an operator action: applications retain
/// their narrow study-admission authority and never obtain a raw kernel grant.
pub struct RootStudyM3ProvisioningAuthority<'daemon> {
    daemon: &'daemon mut Daemon,
    root_authority: PrincipalId,
    operation: RootStudyM3ProvisioningOperationId,
}

impl<'daemon> RootStudyM3ProvisioningAuthority<'daemon> {
    pub(crate) fn new(
        daemon: &'daemon mut Daemon,
        root_authority: PrincipalId,
        operation: RootStudyM3ProvisioningOperationId,
    ) -> Result<Self, RootStudyM3ProvisioningError> {
        if daemon.startup_mode() == StartupMode::RecoveryFenced {
            return Err(RootStudyM3ProvisioningError::RecoveryFenced);
        }
        Ok(Self {
            daemon,
            root_authority,
            operation,
        })
    }

    /// Materializes every finite seat. Repeating this call with the same
    /// plan/operation is safe: stable command identities return their original
    /// events, whereas a changed plan is rejected by command idempotency.
    pub fn provision(
        &mut self,
        plan: &RootStudyM3ProvisioningPlan,
    ) -> Result<RootStudyM3ProvisioningReceipt, RootStudyM3ProvisioningError> {
        let generation = ExpectedGeneration::Exact(plan.expected_generation);
        let project_id = match self.execute(
            "project-create",
            Capability::CreateProject,
            generation,
            CommandBody::CreateProject {
                operating_cycle_id: plan.operating_cycle_id,
                project_name: plan.project.project_name.clone(),
                north_star_alignment: plan.project.north_star_alignment.clone(),
            },
        )? {
            EventBody::ProjectCreated { project_id, .. } => project_id,
            _ => return Err(RootStudyM3ProvisioningError::UnexpectedEvent),
        };
        self.expect_project_event(
            "project-challenge",
            generation,
            plan.operating_cycle_id,
            project_id,
            ProjectState::Challenged,
        )?;
        self.execute_expect(
            "project-charter",
            Capability::CharterProject,
            generation,
            CommandBody::CharterProject {
                operating_cycle_id: plan.operating_cycle_id,
                project_id,
                objective: plan.project.objective.clone(),
                initial_milestone: plan.project.initial_milestone.clone(),
                stop_condition: plan.project.stop_condition.clone(),
            },
            |event| matches!(event, EventBody::ProjectChartered { project_id: actual } if *actual == project_id),
        )?;
        self.expect_project_event(
            "project-activate",
            generation,
            plan.operating_cycle_id,
            project_id,
            ProjectState::Active,
        )?;
        let configuration_revision_id = match self.execute(
            "configuration-register",
            Capability::RegisterActorConfiguration,
            ExpectedGeneration::NotApplicable,
            CommandBody::RegisterActorConfiguration {
                configuration_name: plan.configuration_name.clone(),
                model_policy: plan.model_policy,
                primary_attractor: plan.primary_attractor,
            },
        )? {
            EventBody::ActorConfigurationRegistered {
                actor_configuration_revision_id,
                ..
            } => actor_configuration_revision_id,
            _ => return Err(RootStudyM3ProvisioningError::UnexpectedEvent),
        };

        for seat in &plan.seats {
            self.provision_seat(
                plan,
                generation,
                project_id,
                configuration_revision_id,
                seat,
            )?;
        }
        Ok(RootStudyM3ProvisioningReceipt {
            admitted_seat_count: plan.seats.len(),
        })
    }

    fn provision_seat(
        &mut self,
        plan: &RootStudyM3ProvisioningPlan,
        generation: ExpectedGeneration,
        project_id: ProjectId,
        configuration_revision_id: ActorConfigurationRevisionId,
        seat: &RootStudyM3Seat,
    ) -> Result<(), RootStudyM3ProvisioningError> {
        let ticket_id = match self.execute(
            &seat_command_suffix(seat, "ticket-create"),
            Capability::CreateTicket,
            generation,
            CommandBody::CreateTicket {
                operating_cycle_id: plan.operating_cycle_id,
                project_id,
                ticket_title: seat.ticket_title.clone(),
                acceptance_condition: seat.acceptance_condition.clone(),
                prerequisite_ticket_id: None,
            },
        )? {
            EventBody::TicketCreated { ticket_id, .. } => ticket_id,
            _ => return Err(RootStudyM3ProvisioningError::UnexpectedEvent),
        };
        let context_pack_id = match self.execute(
            &seat_command_suffix(seat, "context-register"),
            Capability::RegisterContextPack,
            generation,
            CommandBody::RegisterContextPack {
                operating_cycle_id: plan.operating_cycle_id,
                purpose: ContextPackPurpose::TicketExecution,
                rendering_digest: seat.context_rendering_digest,
            },
        )? {
            EventBody::ContextPackRegistered { context_pack_id } => context_pack_id,
            _ => return Err(RootStudyM3ProvisioningError::UnexpectedEvent),
        };
        let actor_instance_id = match self.execute(
            &seat_command_suffix(seat, "actor-admit"),
            Capability::AdmitActorInstance,
            generation,
            CommandBody::AdmitActorInstance {
                operating_cycle_id: plan.operating_cycle_id,
                actor_configuration_revision_id: configuration_revision_id,
                execution_profile_id: ExecutionProfileId::NATIVE_PINNED_PI_SDK_V1,
                actor_display_name: seat.actor_display_name.clone(),
            },
        )? {
            EventBody::ActorInstanceAdmitted {
                actor_instance_id, ..
            } => actor_instance_id,
            _ => return Err(RootStudyM3ProvisioningError::UnexpectedEvent),
        };
        self.execute_expect(
            &seat_command_suffix(seat, "ticket-admit"),
            Capability::AdmitTicket,
            generation,
            CommandBody::AdmitTicket {
                operating_cycle_id: plan.operating_cycle_id,
                ticket_id,
            },
            |event| matches!(event, EventBody::TicketAdmitted { ticket_id: actual } if *actual == ticket_id),
        )?;
        let work_item_id = match self.execute(
            &seat_command_suffix(seat, "work-register"),
            Capability::RegisterWorkItem,
            generation,
            CommandBody::RegisterWorkItem {
                operating_cycle_id: plan.operating_cycle_id,
                ticket_id,
                actor_instance_id,
                context_pack_id,
                work_kind: WorkItemKind::TicketExecution,
                adversarial_review_id: None,
                assignment: seat.assignment.clone(),
            },
        )? {
            EventBody::WorkItemRegistered { work_item_id, .. } => work_item_id,
            _ => return Err(RootStudyM3ProvisioningError::UnexpectedEvent),
        };
        self.execute_expect(
            &seat_command_suffix(seat, "allocate"),
            Capability::AllocateStudyActorWork,
            generation,
            CommandBody::AllocateStudyActorWork {
                study_run_id: plan.study_run_id,
                pair_ordinal: seat.pair_ordinal,
                treatment: seat.treatment,
                phase: seat.phase,
                role: seat.role,
                operating_cycle_id: plan.operating_cycle_id,
                work_item_id,
                reservation_amount: seat.reservation_amount,
                supervisor_epoch_id: plan.supervisor_epoch_id,
                supervisor_epoch_identity: plan.supervisor_epoch_identity.clone(),
            },
            |event| {
                matches!(event, EventBody::StudyActorWorkAllocated {
                    study_run_id,
                    pair_ordinal,
                    treatment,
                    phase,
                    role,
                    work_item_id: actual_work_item_id,
                    ..
                } if *study_run_id == plan.study_run_id
                    && *pair_ordinal == seat.pair_ordinal
                    && *treatment == seat.treatment
                    && *phase == seat.phase
                    && *role == seat.role
                    && *actual_work_item_id == work_item_id)
            },
        )?;
        Ok(())
    }

    fn expect_project_event(
        &mut self,
        suffix: &str,
        generation: ExpectedGeneration,
        operating_cycle_id: society_kernel::OperatingCycleId,
        project_id: ProjectId,
        target: ProjectState,
    ) -> Result<(), RootStudyM3ProvisioningError> {
        self.execute_expect(
            suffix,
            Capability::TransitionProject,
            generation,
            CommandBody::TransitionProject {
                operating_cycle_id,
                project_id,
                target,
            },
            |event| matches!(event, EventBody::ProjectStateChanged { project_id: actual, state } if *actual == project_id && *state == target),
        )
    }

    fn execute_expect(
        &mut self,
        suffix: &str,
        capability: Capability,
        expected_generation: ExpectedGeneration,
        body: CommandBody,
        expected: impl FnOnce(&EventBody) -> bool,
    ) -> Result<(), RootStudyM3ProvisioningError> {
        let event = self.execute(suffix, capability, expected_generation, body)?;
        if expected(&event) {
            Ok(())
        } else {
            Err(RootStudyM3ProvisioningError::UnexpectedEvent)
        }
    }

    fn execute(
        &mut self,
        suffix: &str,
        capability: Capability,
        expected_generation: ExpectedGeneration,
        body: CommandBody,
    ) -> Result<EventBody, RootStudyM3ProvisioningError> {
        let command_id = self.command_id(suffix)?;
        match self
            .daemon
            .execute_root_study_provisioning_command(
                command_id,
                self.root_authority,
                capability,
                expected_generation,
                body,
            )
            .map_err(|_| RootStudyM3ProvisioningError::Kernel)?
        {
            CommandDisposition::Accepted(event_id) => self
                .daemon
                .root_study_provisioning_event(event_id)
                .map_err(|_| RootStudyM3ProvisioningError::Kernel),
            CommandDisposition::Rejected(rejection) => {
                Err(RootStudyM3ProvisioningError::Rejected(rejection))
            }
        }
    }

    fn command_id(&self, suffix: &str) -> Result<CommandId, RootStudyM3ProvisioningError> {
        CommandId::parse(format!(
            "{COMMAND_PREFIX}/{}/{suffix}",
            self.operation.as_str()
        ))
        .map_err(|_| RootStudyM3ProvisioningError::CommandIdentity)
    }
}

impl Daemon {
    /// Opens the operator-authorized finite M3 provisioner. This is distinct
    /// from [`Daemon::open_study_admission`]: the latter gives an application
    /// only generic study transitions, while this one acts as the named Root
    /// Authority after the operator has supplied its principal identity.
    pub fn open_root_study_m3_provisioning(
        &mut self,
        root_authority: PrincipalId,
        operation: RootStudyM3ProvisioningOperationId,
    ) -> Result<RootStudyM3ProvisioningAuthority<'_>, RootStudyM3ProvisioningError> {
        RootStudyM3ProvisioningAuthority::new(self, root_authority, operation)
    }
}

fn seat_command_suffix(seat: &RootStudyM3Seat, action: &str) -> String {
    format!(
        "seat-{}-{}-{}-{}/{}",
        seat.pair_ordinal.value(),
        seat.treatment as i64,
        seat.phase as i64,
        seat.role.value(),
        action
    )
}

fn valid_label(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= MAX_OPERATION_BYTES
        && bytes[0].is_ascii_alphanumeric()
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn seat(role: u8) -> RootStudyM3Seat {
        RootStudyM3Seat::new(
            StudyRunPairOrdinal::new(1).unwrap(),
            StudyTreatment::Retained,
            StudyPopulationPhase::Source,
            StudyRoleOrdinal::new(role).unwrap(),
            Blake3Digest::of_bytes(b"context"),
            TicketTitle::parse(format!("task {role}")).unwrap(),
            TicketAcceptanceConditionText::parse("The assigned task is terminal.").unwrap(),
            PrincipalDisplayName::parse(format!("actor {role}")).unwrap(),
            WorkAssignmentText::parse("Execute exactly one bounded task.").unwrap(),
            UsdMicros::new(1).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn plan_rejects_duplicate_seat_before_any_root_command() {
        let project = RootStudyM3Project::new(
            ProjectName::parse("finite study work").unwrap(),
            ProjectNorthStarAlignment {
                application_revision_id: society_kernel::ApplicationRevisionId::new(1).unwrap(),
                change_answer: society_kernel::ProjectNorthStarChangeAnswer::parse("Change.")
                    .unwrap(),
                improvement_evidence_answer:
                    society_kernel::ProjectNorthStarImprovementEvidenceAnswer::parse("Evidence.")
                        .unwrap(),
                boundary_commitment_answer:
                    society_kernel::ProjectNorthStarBoundaryCommitmentAnswer::parse("Boundary.")
                        .unwrap(),
                revisit_answer: society_kernel::ProjectNorthStarRevisitAnswer::parse("Revisit.")
                    .unwrap(),
            },
            ProjectObjectiveText::parse("Provision finite study work.").unwrap(),
            ProjectMilestoneName::parse("Provisioned").unwrap(),
            ProjectStopConditionText::parse("No approved work remains.").unwrap(),
        );
        assert!(matches!(
            RootStudyM3ProvisioningPlan::new(
                StudyRunId::new(1).unwrap(),
                society_kernel::OperatingCycleId::new(1).unwrap(),
                AdmissionGeneration::INITIAL,
                SupervisorEpochId::new(1).unwrap(),
                SupervisorEpochIdentity::parse("test-epoch").unwrap(),
                project,
                ActorConfigurationName::parse("finite study actor").unwrap(),
                ActorModelPolicy::PinnedOpenRouterLing26FlashOff,
                DevelopmentalAttractor::Measure,
                vec![seat(1), seat(1)],
            ),
            Err(RootStudyM3ProvisioningError::DuplicateStudySeat)
        ));
    }

    #[test]
    fn operation_identity_is_bounded_and_canonical() {
        assert!(RootStudyM3ProvisioningOperationId::parse("cl001-pilot-m3").is_ok());
        assert!(matches!(
            RootStudyM3ProvisioningOperationId::parse("CL001-pilot-m3"),
            Err(RootStudyM3ProvisioningError::InvalidOperationIdentity)
        ));
    }
}
