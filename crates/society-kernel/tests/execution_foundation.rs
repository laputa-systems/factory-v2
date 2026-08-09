// These fixtures exercise the observable execution boundary. They use the
// kernel-service terminal fact deliberately: supervisor/Pi receipt binding is
// a later trusted boundary, not something a test fixture may pretend exists.
#![allow(clippy::unwrap_used)]

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;
use society_kernel::{
    ActorAttemptCancellationReason, ActorAttemptId, ActorAttemptTerminalKind,
    ActorConfigurationName, ActorConfigurationRevisionId, ActorInstanceId, ActorModelPolicy,
    AdmissionGeneration, AdversarialReviewId, Capability, CommandBody, CommandDisposition,
    CommandId, CommandReceipt, CommandRequest, ContextPackPurpose, DevelopmentalAttractor,
    EventBody, ExecutionProfileId, ExpectedGeneration, GraphRevisionBody, GraphRevisionId,
    HypothesisRevisionText, KernelStore, OperatingCycleId, OperatingCycleTreatment,
    OutcomeObligationDisposition, OutcomeObligationId, OutcomeObligationText, PrincipalDisplayName,
    PrincipalId, ProjectId, ProjectMilestoneId, ProjectMilestoneName, ProjectName,
    ProjectObjectiveText, ProjectState, ProjectStopConditionText, Rejection, ReviewChallengeId,
    ReviewChallengeSeverity, ReviewDispositionKind, ReviewFailureHypothesis, ReviewResolutionKind,
    ReviewResponseText, Sha256Digest, SocietyName, TicketAcceptanceConditionText, TicketId,
    TicketTitle, UsdMicros, WorkAssignmentText, WorkItemId, WorkItemKind, WorkLeaseId,
};

fn request(
    store: &mut KernelStore,
    command_id: &str,
    principal_id: PrincipalId,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> CommandRequest {
    CommandRequest {
        command_id: CommandId::parse(command_id).unwrap(),
        principal_id,
        capability_grant_id: store
            .active_capability_grant(principal_id, capability)
            .unwrap()
            .unwrap(),
        capability,
        expected_generation,
        body,
    }
}

fn accepted(
    store: &mut KernelStore,
    command_id: &str,
    principal_id: PrincipalId,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> CommandReceipt {
    let request = request(
        store,
        command_id,
        principal_id,
        capability,
        expected_generation,
        body,
    );
    let receipt = store.execute(request).unwrap();
    assert!(
        matches!(receipt.disposition, CommandDisposition::Accepted(_)),
        "{command_id}: {receipt:?}"
    );
    receipt
}

fn rejected(
    store: &mut KernelStore,
    command_id: &str,
    principal_id: PrincipalId,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
    expected: Rejection,
) {
    let request = request(
        store,
        command_id,
        principal_id,
        capability,
        expected_generation,
        body,
    );
    let receipt = store.execute(request).unwrap();
    assert_eq!(receipt.disposition, CommandDisposition::Rejected(expected));
}

fn founded_cycle(
    store: &mut KernelStore,
    treatment: OperatingCycleTreatment,
) -> (PrincipalId, OperatingCycleId) {
    let bootstrap = PrincipalId::BOOTSTRAP;
    accepted(
        store,
        "m3-found-society",
        bootstrap,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        CommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("Execution foundation society").unwrap(),
        },
    );
    accepted(
        store,
        "m3-found-seed",
        bootstrap,
        Capability::InstallFoundingUniverseSeed,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallFoundingUniverseSeed {
            rendering_digest: Sha256Digest::of_bytes(b"execution-foundation-seed"),
        },
    );
    accepted(
        store,
        "m3-found-office",
        bootstrap,
        Capability::InstallGrandArchitectOffice,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallGrandArchitectOffice,
    );
    accepted(
        store,
        "m3-found-architect",
        bootstrap,
        Capability::AppointInitialGrandArchitect,
        ExpectedGeneration::NotApplicable,
        CommandBody::AppointInitialGrandArchitect {
            actor_display_name: PrincipalDisplayName::parse("Grand Architect").unwrap(),
        },
    );
    accepted(
        store,
        "m3-found-ceiling",
        bootstrap,
        Capability::SetR0HardCeiling,
        ExpectedGeneration::NotApplicable,
        CommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::VS001_SOCIETY_HARD_CEILING,
        },
    );
    accepted(
        store,
        "m3-found-bootstrap",
        bootstrap,
        Capability::BootstrapSociety,
        ExpectedGeneration::NotApplicable,
        CommandBody::BootstrapSociety,
    );
    accepted(
        store,
        "m3-found-propose",
        bootstrap,
        Capability::ProposeOperatingCycle,
        ExpectedGeneration::NotApplicable,
        CommandBody::ProposeOperatingCycle { treatment },
    );
    let cycle_id = OperatingCycleId::new(1).unwrap();
    accepted(
        store,
        "m3-found-admit",
        bootstrap,
        Capability::AdmitOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::AdmitOperatingCycle { cycle_id },
    );
    (PrincipalId::new(3).unwrap(), cycle_id)
}

fn active_project(
    store: &mut KernelStore,
    architect: PrincipalId,
    cycle: OperatingCycleId,
) -> ProjectId {
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    accepted(
        store,
        "m3-project-office-session",
        architect,
        Capability::StartGrandArchitectOfficeSession,
        generation,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id: cycle },
    );
    accepted(
        store,
        "m3-project-create",
        architect,
        Capability::CreateProject,
        generation,
        CommandBody::CreateProject {
            operating_cycle_id: cycle,
            project_name: ProjectName::parse("Independent execution proof").unwrap(),
        },
    );
    let project_id = ProjectId::new(1).unwrap();
    rejected(
        store,
        "m3-project-charter-too-early",
        architect,
        Capability::CharterProject,
        generation,
        CommandBody::CharterProject {
            operating_cycle_id: cycle,
            project_id,
            objective: ProjectObjectiveText::parse("A durable independent review.").unwrap(),
            initial_milestone: ProjectMilestoneName::parse("Finish verified ticket").unwrap(),
            stop_condition: ProjectStopConditionText::parse("No remaining safe path.").unwrap(),
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        store,
        "m3-project-challenge",
        architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle,
            project_id,
            target: ProjectState::Challenged,
        },
    );
    accepted(
        store,
        "m3-project-charter",
        architect,
        Capability::CharterProject,
        generation,
        CommandBody::CharterProject {
            operating_cycle_id: cycle,
            project_id,
            objective: ProjectObjectiveText::parse("A durable independent review.").unwrap(),
            initial_milestone: ProjectMilestoneName::parse("Finish verified ticket").unwrap(),
            stop_condition: ProjectStopConditionText::parse("No remaining safe path.").unwrap(),
        },
    );
    accepted(
        store,
        "m3-project-activate",
        architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle,
            project_id,
            target: ProjectState::Active,
        },
    );
    project_id
}

#[test]
fn typed_attempt_retry_review_resolution_and_close_are_replayable() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xsh-execution-foundation-{nonce}.sqlite"));
    let mut store = KernelStore::open(&path).unwrap();
    let (architect, cycle) =
        founded_cycle(&mut store, OperatingCycleTreatment::Vs001DeterministicV1);
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let project = active_project(&mut store, architect, cycle);

    accepted(
        &mut store,
        "m3-ticket-create",
        architect,
        Capability::CreateTicket,
        generation,
        CommandBody::CreateTicket {
            operating_cycle_id: cycle,
            project_id: project,
            ticket_title: TicketTitle::parse("Challenge the claimed result").unwrap(),
            acceptance_condition: TicketAcceptanceConditionText::parse(
                "A validated independent challenge exists.",
            )
            .unwrap(),
            prerequisite_ticket_id: None,
        },
    );
    let ticket = TicketId::new(1).unwrap();
    accepted(
        &mut store,
        "m3-config-register",
        architect,
        Capability::RegisterActorConfiguration,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterActorConfiguration {
            configuration_name: ActorConfigurationName::parse("independent critic v1").unwrap(),
            model_policy: ActorModelPolicy::Vs001DeepseekV4FlashHigh,
            primary_attractor: DevelopmentalAttractor::Challenge,
        },
    );
    let configuration_revision = ActorConfigurationRevisionId::new(1).unwrap();
    accepted(
        &mut store,
        "m3-context-register",
        architect,
        Capability::RegisterContextPack,
        generation,
        CommandBody::RegisterContextPack {
            operating_cycle_id: cycle,
            purpose: ContextPackPurpose::IndependentReview,
            rendering_digest: Sha256Digest::of_bytes(b"reviewer-context-v1"),
        },
    );
    let context = society_kernel::ContextPackId::new(1).unwrap();
    accepted(
        &mut store,
        "m3-actor-admit",
        architect,
        Capability::AdmitActorInstance,
        generation,
        CommandBody::AdmitActorInstance {
            operating_cycle_id: cycle,
            actor_configuration_revision_id: configuration_revision,
            execution_profile_id: ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            actor_display_name: PrincipalDisplayName::parse("independent reviewer").unwrap(),
        },
    );
    let actor = ActorInstanceId::new(1).unwrap();
    let reviewer = PrincipalId::new(4).unwrap();
    accepted(
        &mut store,
        "m3-graph-add",
        architect,
        Capability::AddGraphObjectRevision,
        generation,
        CommandBody::AddGraphObjectRevision {
            operating_cycle_id: cycle,
            project_id: project,
            causal_episode_id: None,
            graph_object_id: None,
            body: GraphRevisionBody::Hypothesis {
                hypothesis: HypothesisRevisionText::parse(
                    "The observation is sufficient to justify delivery.",
                )
                .unwrap(),
            },
        },
    );
    let target = GraphRevisionId::new(1).unwrap();
    accepted(
        &mut store,
        "m3-graph-commit",
        architect,
        Capability::CommitGraphRevision,
        generation,
        CommandBody::CommitGraphRevision {
            operating_cycle_id: cycle,
            graph_revision_id: target,
        },
    );
    accepted(
        &mut store,
        "m3-review-request",
        architect,
        Capability::RequestAdversarialReview,
        generation,
        CommandBody::RequestAdversarialReview {
            operating_cycle_id: cycle,
            project_id: project,
            target_graph_revision_id: target,
        },
    );
    let review = AdversarialReviewId::new(1).unwrap();
    accepted(
        &mut store,
        "m3-ticket-admit",
        architect,
        Capability::AdmitTicket,
        generation,
        CommandBody::AdmitTicket {
            operating_cycle_id: cycle,
            ticket_id: ticket,
        },
    );
    accepted(
        &mut store,
        "m3-work-register",
        architect,
        Capability::RegisterWorkItem,
        generation,
        CommandBody::RegisterWorkItem {
            operating_cycle_id: cycle,
            ticket_id: ticket,
            actor_instance_id: actor,
            context_pack_id: context,
            work_kind: WorkItemKind::IndependentReview,
            adversarial_review_id: Some(review),
            assignment: WorkAssignmentText::parse("Seek a falsifying case for the graph claim.")
                .unwrap(),
        },
    );
    let work = WorkItemId::new(1).unwrap();

    accepted(
        &mut store,
        "m3-work-claim-first",
        reviewer,
        Capability::ClaimWorkItem,
        generation,
        CommandBody::ClaimWorkItem {
            operating_cycle_id: cycle,
            work_item_id: work,
        },
    );
    accepted(
        &mut store,
        "m3-attempt-start-first",
        architect,
        Capability::StartActorAttempt,
        generation,
        CommandBody::StartActorAttempt {
            operating_cycle_id: cycle,
            work_item_id: work,
            reservation_amount: UsdMicros::try_from(5_000).unwrap(),
        },
    );
    let first_attempt = ActorAttemptId::new(1).unwrap();
    // Cancellation is a durable control request, then a separate trusted
    // terminal fact. A cancellation request cannot silently become success,
    // nor may a terminal fact manufacture cancellation from a running Attempt.
    rejected(
        &mut store,
        "m3-attempt-cancel-direct",
        PrincipalId::KERNEL,
        Capability::AttestActorAttemptTerminal,
        ExpectedGeneration::NotApplicable,
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id: first_attempt,
            terminal_kind: ActorAttemptTerminalKind::Cancelled,
        },
        Rejection::ActorAttemptNotTerminal,
    );
    accepted(
        &mut store,
        "m3-attempt-cancel-first",
        PrincipalId::KERNEL,
        Capability::CancelActorAttempt,
        ExpectedGeneration::NotApplicable,
        CommandBody::CancelActorAttempt {
            actor_attempt_id: first_attempt,
            reason: ActorAttemptCancellationReason::GrandArchitectRequested,
        },
    );
    rejected(
        &mut store,
        "m3-attempt-success-after-cancellation",
        PrincipalId::KERNEL,
        Capability::AttestActorAttemptTerminal,
        ExpectedGeneration::NotApplicable,
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id: first_attempt,
            terminal_kind: ActorAttemptTerminalKind::Succeeded,
        },
        Rejection::ActorAttemptNotTerminal,
    );
    accepted(
        &mut store,
        "m3-attempt-terminal-first",
        PrincipalId::KERNEL,
        Capability::AttestActorAttemptTerminal,
        ExpectedGeneration::NotApplicable,
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id: first_attempt,
            terminal_kind: ActorAttemptTerminalKind::Cancelled,
        },
    );
    accepted(
        &mut store,
        "m3-attempt-reconcile-first",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: society_kernel::BudgetReservationId::new(1).unwrap(),
            observation: society_kernel::CostObservation::Known(UsdMicros::ZERO),
        },
    );
    accepted(
        &mut store,
        "m3-project-pause-before-retry",
        architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle,
            project_id: project,
            target: ProjectState::Paused,
        },
    );
    rejected(
        &mut store,
        "m3-attempt-retry-paused-project",
        architect,
        Capability::RetryActorAttempt,
        generation,
        CommandBody::RetryActorAttempt {
            operating_cycle_id: cycle,
            actor_attempt_id: first_attempt,
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "m3-project-resume-before-retry",
        architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle,
            project_id: project,
            target: ProjectState::Active,
        },
    );
    accepted(
        &mut store,
        "m3-attempt-retry",
        architect,
        Capability::RetryActorAttempt,
        generation,
        CommandBody::RetryActorAttempt {
            operating_cycle_id: cycle,
            actor_attempt_id: first_attempt,
        },
    );

    accepted(
        &mut store,
        "m3-work-claim-retry",
        reviewer,
        Capability::ClaimWorkItem,
        generation,
        CommandBody::ClaimWorkItem {
            operating_cycle_id: cycle,
            work_item_id: work,
        },
    );
    let start_retry_body = CommandBody::StartActorAttempt {
        operating_cycle_id: cycle,
        work_item_id: work,
        reservation_amount: UsdMicros::try_from(5_000).unwrap(),
    };
    let first_start = accepted(
        &mut store,
        "m3-attempt-start-retry",
        architect,
        Capability::StartActorAttempt,
        generation,
        start_retry_body.clone(),
    );
    let repeated_request = request(
        &mut store,
        "m3-attempt-start-retry",
        architect,
        Capability::StartActorAttempt,
        generation,
        start_retry_body,
    );
    let repeated_start = store.execute(repeated_request).unwrap();
    assert!(repeated_start.idempotent);
    assert_eq!(repeated_start.disposition, first_start.disposition);
    let second_attempt = ActorAttemptId::new(2).unwrap();
    accepted(
        &mut store,
        "m3-attempt-terminal-retry",
        PrincipalId::KERNEL,
        Capability::AttestActorAttemptTerminal,
        ExpectedGeneration::NotApplicable,
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id: second_attempt,
            terminal_kind: ActorAttemptTerminalKind::Succeeded,
        },
    );

    rejected(
        &mut store,
        "m3-review-self-assign",
        PrincipalId::KERNEL,
        Capability::AssignAdversarialReviewer,
        generation,
        CommandBody::AssignAdversarialReviewer {
            operating_cycle_id: cycle,
            adversarial_review_id: review,
            reviewer_principal_id: architect,
            reviewer_actor_instance_id: actor,
            reviewer_actor_attempt_id: second_attempt,
        },
        Rejection::ReviewAssignmentNotIndependent,
    );
    accepted(
        &mut store,
        "m3-review-assign",
        PrincipalId::KERNEL,
        Capability::AssignAdversarialReviewer,
        generation,
        CommandBody::AssignAdversarialReviewer {
            operating_cycle_id: cycle,
            adversarial_review_id: review,
            reviewer_principal_id: reviewer,
            reviewer_actor_instance_id: actor,
            reviewer_actor_attempt_id: second_attempt,
        },
    );
    accepted(
        &mut store,
        "m3-review-submit",
        PrincipalId::KERNEL,
        Capability::SubmitReviewChallenge,
        generation,
        CommandBody::SubmitReviewChallenge {
            operating_cycle_id: cycle,
            adversarial_review_id: review,
            target_graph_revision_id: target,
            author_principal_id: reviewer,
            severity: ReviewChallengeSeverity::High,
            failure_hypothesis: ReviewFailureHypothesis::parse(
                "The observation omits an adversarial platform condition.",
            )
            .unwrap(),
        },
    );
    let challenge = ReviewChallengeId::new(1).unwrap();
    accepted(
        &mut store,
        "m3-review-response",
        architect,
        Capability::RespondToReviewChallenge,
        generation,
        CommandBody::RespondToReviewChallenge {
            operating_cycle_id: cycle,
            review_challenge_id: challenge,
            response: ReviewResponseText::parse(
                "The proposal now includes the missing platform condition.",
            )
            .unwrap(),
        },
    );
    accepted(
        &mut store,
        "m3-review-disposition",
        architect,
        Capability::DispositionReviewChallenge,
        generation,
        CommandBody::DispositionReviewChallenge {
            operating_cycle_id: cycle,
            review_challenge_id: challenge,
            disposition: ReviewDispositionKind::RejectedWithDissentPreserved,
        },
    );
    accepted(
        &mut store,
        "m3-review-resolve",
        architect,
        Capability::ResolveAdversarialReview,
        generation,
        CommandBody::ResolveAdversarialReview {
            operating_cycle_id: cycle,
            adversarial_review_id: review,
            resolution: ReviewResolutionKind::Resolved,
        },
    );

    let self_validation = CommandRequest {
        command_id: CommandId::parse("m3-attempt-validate-ga-self-attest").unwrap(),
        principal_id: architect,
        capability_grant_id: store
            .active_capability_grant(architect, Capability::CompleteTicket)
            .unwrap()
            .unwrap(),
        capability: Capability::ValidateTicketAttempt,
        expected_generation: generation,
        body: CommandBody::ValidateTicketAttempt {
            operating_cycle_id: cycle,
            actor_attempt_id: second_attempt,
        },
    };
    assert_eq!(
        store.execute(self_validation).unwrap().disposition,
        CommandDisposition::Rejected(Rejection::CapabilityNotGranted)
    );
    accepted(
        &mut store,
        "m3-attempt-validate",
        PrincipalId::KERNEL,
        Capability::ValidateTicketAttempt,
        generation,
        CommandBody::ValidateTicketAttempt {
            operating_cycle_id: cycle,
            actor_attempt_id: second_attempt,
        },
    );
    accepted(
        &mut store,
        "m3-ticket-complete",
        architect,
        Capability::CompleteTicket,
        generation,
        CommandBody::CompleteTicket {
            operating_cycle_id: cycle,
            actor_attempt_id: second_attempt,
        },
    );
    accepted(
        &mut store,
        "m3-attempt-reconcile-second",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: society_kernel::BudgetReservationId::new(2).unwrap(),
            observation: society_kernel::CostObservation::Known(UsdMicros::ZERO),
        },
    );
    accepted(
        &mut store,
        "m3-outcome-register",
        architect,
        Capability::RegisterOutcomeObligation,
        generation,
        CommandBody::RegisterOutcomeObligation {
            operating_cycle_id: cycle,
            project_id: project,
            obligation: OutcomeObligationText::parse(
                "Observe the resolved dissent in the next cycle.",
            )
            .unwrap(),
        },
    );
    accepted(
        &mut store,
        "m3-milestone-complete",
        architect,
        Capability::CompleteProjectMilestone,
        generation,
        CommandBody::CompleteProjectMilestone {
            operating_cycle_id: cycle,
            project_milestone_id: ProjectMilestoneId::new(1).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m3-project-observe",
        architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle,
            project_id: project,
            target: ProjectState::Observing,
        },
    );
    rejected(
        &mut store,
        "m3-project-close-open-outcome",
        architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle,
            project_id: project,
            target: ProjectState::Closed,
        },
        Rejection::ProjectCloseBlocked,
    );
    accepted(
        &mut store,
        "m3-outcome-resolve",
        architect,
        Capability::ResolveOutcomeObligation,
        generation,
        CommandBody::ResolveOutcomeObligation {
            operating_cycle_id: cycle,
            outcome_obligation_id: OutcomeObligationId::new(1).unwrap(),
            disposition: OutcomeObligationDisposition::Satisfied,
        },
    );
    accepted(
        &mut store,
        "m3-project-close",
        architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle,
            project_id: project,
            target: ProjectState::Closed,
        },
    );

    assert!(store.validate_replayed_materialized_state().is_ok());
    assert!(
        store
            .replay_ledger()
            .unwrap()
            .iter()
            .any(|event| matches!(event.body, EventBody::AdversarialReviewResolved { .. }))
    );
    drop(store);

    let inspect = Connection::open(&path).unwrap();
    let retry_link: Option<i64> = inspect
        .query_row(
            "SELECT retry_of_actor_attempt_id FROM attempts WHERE actor_attempt_id = 2",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(retry_link, Some(1));
    let (author, disposition): (i64, i64) = inspect.query_row("SELECT c.author_principal_id, d.disposition_kind FROM review_challenges c JOIN review_dispositions d ON d.review_challenge_id = c.review_challenge_id", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    assert_eq!(
        (author, disposition),
        (
            reviewer.value(),
            ReviewDispositionKind::RejectedWithDissentPreserved as i64
        )
    );
    let acceptance_satisfied_by: String = inspect
        .query_row(
            "SELECT c.command_id FROM ticket_acceptance_conditions a
         JOIN commands c ON c.command_row_id = a.satisfied_by_command_id
         WHERE a.ticket_id = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(acceptance_satisfied_by, "m3-attempt-validate");
    inspect.execute("UPDATE command_start_actor_attempt SET reservation_micros = 4999 WHERE command_row_id = (SELECT command_row_id FROM commands WHERE command_id = 'm3-attempt-start-retry')", []).unwrap();
    drop(inspect);
    assert!(KernelStore::open(&path).unwrap().replay_ledger().is_err());

    let repair = Connection::open(&path).unwrap();
    repair.execute("UPDATE command_start_actor_attempt SET reservation_micros = 5000 WHERE command_row_id = (SELECT command_row_id FROM commands WHERE command_id = 'm3-attempt-start-retry')", []).unwrap();
    repair
        .execute(
            "UPDATE attempts SET lifecycle_state = 4 WHERE actor_attempt_id = 2",
            [],
        )
        .unwrap();
    drop(repair);
    assert!(
        KernelStore::open(&path)
            .unwrap()
            .validate_replayed_materialized_state()
            .is_err()
    );
    fs::remove_file(path).unwrap();
}

#[test]
fn lease_expiry_requires_a_work_item_without_an_attempt() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (architect, cycle) =
        founded_cycle(&mut store, OperatingCycleTreatment::Vs001DeterministicV1);
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let project = active_project(&mut store, architect, cycle);
    accepted(
        &mut store,
        "expiry-ticket-create",
        architect,
        Capability::CreateTicket,
        generation,
        CommandBody::CreateTicket {
            operating_cycle_id: cycle,
            project_id: project,
            ticket_title: TicketTitle::parse("Lease expiry fixture").unwrap(),
            acceptance_condition: TicketAcceptanceConditionText::parse(
                "Lease expiration is represented.",
            )
            .unwrap(),
            prerequisite_ticket_id: None,
        },
    );
    let ticket = TicketId::new(1).unwrap();
    accepted(
        &mut store,
        "expiry-config",
        architect,
        Capability::RegisterActorConfiguration,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterActorConfiguration {
            configuration_name: ActorConfigurationName::parse("worker configuration").unwrap(),
            model_policy: ActorModelPolicy::Vs001DeepseekV4FlashHigh,
            primary_attractor: DevelopmentalAttractor::Build,
        },
    );
    accepted(
        &mut store,
        "expiry-context",
        architect,
        Capability::RegisterContextPack,
        generation,
        CommandBody::RegisterContextPack {
            operating_cycle_id: cycle,
            purpose: ContextPackPurpose::TicketExecution,
            rendering_digest: Sha256Digest::of_bytes(b"ticket context"),
        },
    );
    accepted(
        &mut store,
        "expiry-actor",
        architect,
        Capability::AdmitActorInstance,
        generation,
        CommandBody::AdmitActorInstance {
            operating_cycle_id: cycle,
            actor_configuration_revision_id: ActorConfigurationRevisionId::new(1).unwrap(),
            execution_profile_id: ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            actor_display_name: PrincipalDisplayName::parse("lease worker").unwrap(),
        },
    );
    accepted(
        &mut store,
        "expiry-admit-ticket",
        architect,
        Capability::AdmitTicket,
        generation,
        CommandBody::AdmitTicket {
            operating_cycle_id: cycle,
            ticket_id: ticket,
        },
    );
    accepted(
        &mut store,
        "expiry-register-work",
        architect,
        Capability::RegisterWorkItem,
        generation,
        CommandBody::RegisterWorkItem {
            operating_cycle_id: cycle,
            ticket_id: ticket,
            actor_instance_id: ActorInstanceId::new(1).unwrap(),
            context_pack_id: society_kernel::ContextPackId::new(1).unwrap(),
            work_kind: WorkItemKind::TicketExecution,
            adversarial_review_id: None,
            assignment: WorkAssignmentText::parse("Do bounded ticket work.").unwrap(),
        },
    );
    accepted(
        &mut store,
        "expiry-pause-before-claim",
        architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle,
            project_id: project,
            target: ProjectState::Paused,
        },
    );
    rejected(
        &mut store,
        "expiry-claim-paused-project",
        PrincipalId::new(4).unwrap(),
        Capability::ClaimWorkItem,
        generation,
        CommandBody::ClaimWorkItem {
            operating_cycle_id: cycle,
            work_item_id: WorkItemId::new(1).unwrap(),
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "expiry-resume-before-claim",
        architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle,
            project_id: project,
            target: ProjectState::Active,
        },
    );
    accepted(
        &mut store,
        "expiry-claim",
        PrincipalId::new(4).unwrap(),
        Capability::ClaimWorkItem,
        generation,
        CommandBody::ClaimWorkItem {
            operating_cycle_id: cycle,
            work_item_id: WorkItemId::new(1).unwrap(),
        },
    );
    accepted(
        &mut store,
        "expiry-pause-before-start",
        architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle,
            project_id: project,
            target: ProjectState::Paused,
        },
    );
    rejected(
        &mut store,
        "expiry-start-paused-project",
        architect,
        Capability::StartActorAttempt,
        generation,
        CommandBody::StartActorAttempt {
            operating_cycle_id: cycle,
            work_item_id: WorkItemId::new(1).unwrap(),
            reservation_amount: UsdMicros::try_from(1_000).unwrap(),
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "expiry-expire",
        PrincipalId::KERNEL,
        Capability::ExpireWorkLease,
        ExpectedGeneration::NotApplicable,
        CommandBody::ExpireWorkLease {
            work_lease_id: WorkLeaseId::new(1).unwrap(),
        },
    );
    rejected(
        &mut store,
        "expiry-repeat",
        PrincipalId::KERNEL,
        Capability::ExpireWorkLease,
        ExpectedGeneration::NotApplicable,
        CommandBody::ExpireWorkLease {
            work_lease_id: WorkLeaseId::new(1).unwrap(),
        },
        Rejection::WorkLeaseUnavailable,
    );
    assert!(store.validate_replayed_materialized_state().is_ok());
}

/// The deterministic process double is a provider-free fixture profile. It
/// cannot stand in for either the paid native qualification treatment or a
/// live VS-001 run; the unqualified native identity cannot admit an actor in
/// any treatment before the later typed qualification receipt exists.
#[test]
fn execution_profile_admission_is_closed_by_treatment_and_readiness() {
    let deterministic_cases = [
        OperatingCycleTreatment::PiSdkQualificationV1,
        OperatingCycleTreatment::Vs001LiveV1,
    ];
    for (index, treatment) in deterministic_cases.into_iter().enumerate() {
        let mut store = KernelStore::open_in_memory().unwrap();
        let (architect, cycle) = founded_cycle(&mut store, treatment);
        let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
        if treatment != OperatingCycleTreatment::PiSdkQualificationV1 {
            let _project = active_project(&mut store, architect, cycle);
        }
        if treatment != OperatingCycleTreatment::PiSdkQualificationV1 {
            accepted(
                &mut store,
                "profile-double-config",
                architect,
                Capability::RegisterActorConfiguration,
                ExpectedGeneration::NotApplicable,
                CommandBody::RegisterActorConfiguration {
                    configuration_name: ActorConfigurationName::parse("profile gate configuration")
                        .unwrap(),
                    model_policy: ActorModelPolicy::Vs001DeepseekV4FlashHigh,
                    primary_attractor: DevelopmentalAttractor::Build,
                },
            );
        }
        rejected(
            &mut store,
            &format!("profile-double-rejected-{index}"),
            architect,
            Capability::AdmitActorInstance,
            generation,
            CommandBody::AdmitActorInstance {
                operating_cycle_id: cycle,
                actor_configuration_revision_id: ActorConfigurationRevisionId::new(1).unwrap(),
                execution_profile_id: ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
                actor_display_name: PrincipalDisplayName::parse("disallowed double actor").unwrap(),
            },
            if treatment == OperatingCycleTreatment::PiSdkQualificationV1 {
                Rejection::QualificationTreatmentRestricted
            } else {
                Rejection::ExecutionProfileIneligible
            },
        );
    }

    let native_cases = [
        OperatingCycleTreatment::PiSdkQualificationV1,
        OperatingCycleTreatment::Vs001LiveV1,
        OperatingCycleTreatment::Vs001DeterministicV1,
    ];
    for (index, treatment) in native_cases.into_iter().enumerate() {
        let mut store = KernelStore::open_in_memory().unwrap();
        let (architect, cycle) = founded_cycle(&mut store, treatment);
        let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
        if treatment != OperatingCycleTreatment::PiSdkQualificationV1 {
            let _project = active_project(&mut store, architect, cycle);
        }
        if treatment != OperatingCycleTreatment::PiSdkQualificationV1 {
            accepted(
                &mut store,
                "profile-native-config",
                architect,
                Capability::RegisterActorConfiguration,
                ExpectedGeneration::NotApplicable,
                CommandBody::RegisterActorConfiguration {
                    configuration_name: ActorConfigurationName::parse("native gate configuration")
                        .unwrap(),
                    model_policy: ActorModelPolicy::Vs001DeepseekV4FlashHigh,
                    primary_attractor: DevelopmentalAttractor::Build,
                },
            );
        }
        rejected(
            &mut store,
            &format!("profile-native-rejected-{index}"),
            architect,
            Capability::AdmitActorInstance,
            generation,
            CommandBody::AdmitActorInstance {
                operating_cycle_id: cycle,
                actor_configuration_revision_id: ActorConfigurationRevisionId::new(1).unwrap(),
                execution_profile_id: ExecutionProfileId::NATIVE_PINNED_PI_SDK_V1,
                actor_display_name: PrincipalDisplayName::parse("unqualified native actor")
                    .unwrap(),
            },
            if treatment == OperatingCycleTreatment::PiSdkQualificationV1 {
                Rejection::QualificationTreatmentRestricted
            } else {
                Rejection::ExecutionProfileIneligible
            },
        );
    }
}

#[test]
fn paid_qualification_treatment_has_no_grand_architect_work_surface() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (architect, cycle) =
        founded_cycle(&mut store, OperatingCycleTreatment::PiSdkQualificationV1);
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    rejected(
        &mut store,
        "qualification-no-office-session",
        architect,
        Capability::StartGrandArchitectOfficeSession,
        generation,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id: cycle },
        Rejection::QualificationTreatmentRestricted,
    );
    rejected(
        &mut store,
        "qualification-no-project",
        architect,
        Capability::CreateProject,
        generation,
        CommandBody::CreateProject {
            operating_cycle_id: cycle,
            project_name: ProjectName::parse("Forbidden qualification project").unwrap(),
        },
        Rejection::QualificationTreatmentRestricted,
    );
    rejected(
        &mut store,
        "qualification-no-actor-admission",
        architect,
        Capability::AdmitActorInstance,
        generation,
        CommandBody::AdmitActorInstance {
            operating_cycle_id: cycle,
            actor_configuration_revision_id: ActorConfigurationRevisionId::new(1).unwrap(),
            execution_profile_id: ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            actor_display_name: PrincipalDisplayName::parse("forbidden qualification actor")
                .unwrap(),
        },
        Rejection::QualificationTreatmentRestricted,
    );
    assert!(store.validate_replayed_materialized_state().is_ok());
}

#[test]
fn reviewer_attempt_cannot_be_rebound_to_a_different_requested_review() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (architect, cycle) =
        founded_cycle(&mut store, OperatingCycleTreatment::Vs001DeterministicV1);
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let project = active_project(&mut store, architect, cycle);
    accepted(
        &mut store,
        "cross-review-ticket",
        architect,
        Capability::CreateTicket,
        generation,
        CommandBody::CreateTicket {
            operating_cycle_id: cycle,
            project_id: project,
            ticket_title: TicketTitle::parse("Bound review evidence").unwrap(),
            acceptance_condition: TicketAcceptanceConditionText::parse(
                "The exact requested review has independent evidence.",
            )
            .unwrap(),
            prerequisite_ticket_id: None,
        },
    );
    for (index, text) in ["First target", "Second target"].into_iter().enumerate() {
        accepted(
            &mut store,
            &format!("cross-review-graph-add-{index}"),
            architect,
            Capability::AddGraphObjectRevision,
            generation,
            CommandBody::AddGraphObjectRevision {
                operating_cycle_id: cycle,
                project_id: project,
                causal_episode_id: None,
                graph_object_id: None,
                body: GraphRevisionBody::Hypothesis {
                    hypothesis: HypothesisRevisionText::parse(text).unwrap(),
                },
            },
        );
        accepted(
            &mut store,
            &format!("cross-review-graph-commit-{index}"),
            architect,
            Capability::CommitGraphRevision,
            generation,
            CommandBody::CommitGraphRevision {
                operating_cycle_id: cycle,
                graph_revision_id: GraphRevisionId::new((index + 1) as i64).unwrap(),
            },
        );
        accepted(
            &mut store,
            &format!("cross-review-request-{index}"),
            architect,
            Capability::RequestAdversarialReview,
            generation,
            CommandBody::RequestAdversarialReview {
                operating_cycle_id: cycle,
                project_id: project,
                target_graph_revision_id: GraphRevisionId::new((index + 1) as i64).unwrap(),
            },
        );
    }
    let first_review = AdversarialReviewId::new(1).unwrap();
    let second_review = AdversarialReviewId::new(2).unwrap();
    accepted(
        &mut store,
        "cross-review-config",
        architect,
        Capability::RegisterActorConfiguration,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterActorConfiguration {
            configuration_name: ActorConfigurationName::parse("cross review critic").unwrap(),
            model_policy: ActorModelPolicy::Vs001DeepseekV4FlashHigh,
            primary_attractor: DevelopmentalAttractor::Challenge,
        },
    );
    accepted(
        &mut store,
        "cross-review-context",
        architect,
        Capability::RegisterContextPack,
        generation,
        CommandBody::RegisterContextPack {
            operating_cycle_id: cycle,
            purpose: ContextPackPurpose::IndependentReview,
            rendering_digest: Sha256Digest::of_bytes(b"cross-review-context"),
        },
    );
    accepted(
        &mut store,
        "cross-review-actor",
        architect,
        Capability::AdmitActorInstance,
        generation,
        CommandBody::AdmitActorInstance {
            operating_cycle_id: cycle,
            actor_configuration_revision_id: ActorConfigurationRevisionId::new(1).unwrap(),
            execution_profile_id: ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            actor_display_name: PrincipalDisplayName::parse("cross review actor").unwrap(),
        },
    );
    accepted(
        &mut store,
        "cross-review-ticket-admit",
        architect,
        Capability::AdmitTicket,
        generation,
        CommandBody::AdmitTicket {
            operating_cycle_id: cycle,
            ticket_id: TicketId::new(1).unwrap(),
        },
    );
    rejected(
        &mut store,
        "cross-review-work-missing-binding",
        architect,
        Capability::RegisterWorkItem,
        generation,
        CommandBody::RegisterWorkItem {
            operating_cycle_id: cycle,
            ticket_id: TicketId::new(1).unwrap(),
            actor_instance_id: ActorInstanceId::new(1).unwrap(),
            context_pack_id: society_kernel::ContextPackId::new(1).unwrap(),
            work_kind: WorkItemKind::IndependentReview,
            adversarial_review_id: None,
            assignment: WorkAssignmentText::parse("Missing exact review binding.").unwrap(),
        },
        Rejection::ReviewAssignmentEvidenceMissing,
    );
    accepted(
        &mut store,
        "cross-review-work",
        architect,
        Capability::RegisterWorkItem,
        generation,
        CommandBody::RegisterWorkItem {
            operating_cycle_id: cycle,
            ticket_id: TicketId::new(1).unwrap(),
            actor_instance_id: ActorInstanceId::new(1).unwrap(),
            context_pack_id: society_kernel::ContextPackId::new(1).unwrap(),
            work_kind: WorkItemKind::IndependentReview,
            adversarial_review_id: Some(first_review),
            assignment: WorkAssignmentText::parse("Challenge only the first target.").unwrap(),
        },
    );
    accepted(
        &mut store,
        "cross-review-claim",
        PrincipalId::new(4).unwrap(),
        Capability::ClaimWorkItem,
        generation,
        CommandBody::ClaimWorkItem {
            operating_cycle_id: cycle,
            work_item_id: WorkItemId::new(1).unwrap(),
        },
    );
    accepted(
        &mut store,
        "cross-review-start",
        architect,
        Capability::StartActorAttempt,
        generation,
        CommandBody::StartActorAttempt {
            operating_cycle_id: cycle,
            work_item_id: WorkItemId::new(1).unwrap(),
            reservation_amount: UsdMicros::try_from(1_000).unwrap(),
        },
    );
    accepted(
        &mut store,
        "cross-review-terminal",
        PrincipalId::KERNEL,
        Capability::AttestActorAttemptTerminal,
        ExpectedGeneration::NotApplicable,
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id: ActorAttemptId::new(1).unwrap(),
            terminal_kind: ActorAttemptTerminalKind::Succeeded,
        },
    );
    rejected(
        &mut store,
        "cross-review-wrong-assignment",
        PrincipalId::KERNEL,
        Capability::AssignAdversarialReviewer,
        generation,
        CommandBody::AssignAdversarialReviewer {
            operating_cycle_id: cycle,
            adversarial_review_id: second_review,
            reviewer_principal_id: PrincipalId::new(4).unwrap(),
            reviewer_actor_instance_id: ActorInstanceId::new(1).unwrap(),
            reviewer_actor_attempt_id: ActorAttemptId::new(1).unwrap(),
        },
        Rejection::ReviewAssignmentEvidenceMissing,
    );
    assert!(store.validate_replayed_materialized_state().is_ok());
}

#[test]
fn compiled_capability_grants_have_closed_origin_and_exact_service_set() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xsh-capability-origin-{nonce}.sqlite"));
    let mut store = KernelStore::open(&path).unwrap();
    let (architect, _) = founded_cycle(&mut store, OperatingCycleTreatment::Vs001DeterministicV1);
    assert!(store.validate_replayed_materialized_state().is_ok());
    drop(store);

    let inspect = Connection::open(&path).unwrap();
    let bootstrap_origins: Vec<(i64, Option<i64>)> = inspect
        .prepare(
            "SELECT DISTINCT grant_origin, granted_by_command_id
             FROM capability_grants WHERE principal_id = 1 ORDER BY grant_origin",
        )
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(bootstrap_origins, vec![(1, None)]);

    let service_capabilities: Vec<i64> = inspect
        .prepare(
            "SELECT capability_kind FROM capability_grants
             WHERE principal_id = 2 AND grant_origin = 3
             ORDER BY capability_kind",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut expected_service_capabilities: Vec<i64> = Capability::KERNEL_SERVICE
        .into_iter()
        .map(|capability| capability as i64)
        .collect();
    expected_service_capabilities.sort_unstable();
    assert_eq!(service_capabilities, expected_service_capabilities);
    let architect_ledger_grants: i64 = inspect
        .query_row(
            "SELECT COUNT(*) FROM capability_grants
             WHERE principal_id = ?1 AND grant_origin = 2
               AND granted_by_command_id IS NOT NULL",
            [architect.value()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        architect_ledger_grants,
        Capability::GRAND_ARCHITECT.len() as i64
    );
    assert!(
        inspect
            .execute(
                "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id,
                                             actor_instance_id, grant_state, grant_origin,
                                             granted_by_command_id, consumed_by_command_id)
             VALUES (?1, 54, NULL, NULL, 1, 3, NULL, NULL)",
                [architect.value()],
            )
            .is_err()
    );
    drop(inspect);
    assert!(
        KernelStore::open(&path)
            .unwrap()
            .validate_replayed_materialized_state()
            .is_ok()
    );
    fs::remove_file(path).unwrap();
}
