// These integration fixtures deliberately fail at the exact setup/action
// boundary, so `unwrap` keeps the behavior under test visible rather than
// forcing every assertion through unrelated error plumbing.
#![allow(clippy::unwrap_used)]

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::Connection;
use society_kernel::{
    AdmissionGeneration, AdversarialReviewId, BudgetFreezeReason, BudgetReservationId,
    CancellationRequestId, CanonicalWorkspacePath, Capability, CausalEpisodeId, ChildProcessId,
    CommandBody, CommandDisposition, CommandId, CommandReceipt, CommandRequest, CostObservation,
    CostPostmortemResolution, CostUnavailableReason, CostUnknownReason, EpisodeState, EventBody,
    ExpectedGeneration, GrandArchitectOfficeSessionId, GraphEdgeKind, GraphRevisionBody,
    GraphRevisionId, HypothesisRevisionText, KernelStore, NativeChildPid, NativeWorkspaceId,
    ObservationRevisionText, OfficeSessionTerminalState, OfficeTurnId, OfficeTurnPurpose,
    OperatingCycleId, OperatingCycleState, OperatingCycleTreatment, OwnedProcessGroupId,
    PiBoundarySessionIdentity, PiChildOwner, PiChildSpawnAdmissionId, PiCorrelationIdentity,
    PiCumulativeUsage, PiOfficeTurnAssistantOutcome, PiOfficeTurnDisposition,
    PiOfficeTurnTerminalReceiptId, PiOfficeTurnTranscriptDisposition, PiOfficeTurnUsageFailure,
    PiOfficeTurnUsageUnavailableReason, PiProtocolSequence, PiTokenCount, PostmortemActionKind,
    PostmortemActionProposalText, PostmortemCausalClaimKind, PostmortemCausalClaimText,
    PostmortemId, PrincipalDisplayName, PrincipalId, ProjectId, ProjectMilestoneName, ProjectName,
    ProjectObjectiveText, ProjectState, ProjectStopConditionText, ProviderCostBinary64, Rejection,
    ReviewChallengeSeverity, ReviewFailureHypothesis, Sha256Digest, SocietyName, SpawnNonce,
    StoreError, SupervisedChildIdentity, SupervisorEpochId, SupervisorEpochIdentity, UsdMicros,
};

fn submit(
    store: &mut KernelStore,
    command_id: &str,
    principal_id: PrincipalId,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> CommandReceipt {
    let request = CommandRequest {
        command_id: CommandId::parse(command_id).unwrap(),
        principal_id,
        capability_grant_id: store
            .active_capability_grant(principal_id, capability)
            .unwrap()
            .expect("active test grant"),
        capability,
        expected_generation,
        body,
    };
    store.execute(request).unwrap()
}

fn accepted(
    store: &mut KernelStore,
    command_id: &str,
    principal_id: PrincipalId,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> CommandReceipt {
    let receipt = submit(
        store,
        command_id,
        principal_id,
        capability,
        expected_generation,
        body,
    );
    assert!(
        matches!(receipt.disposition, CommandDisposition::Accepted(_)),
        "{command_id}: unexpected receipt: {receipt:?}"
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
) -> CommandReceipt {
    let receipt = submit(
        store,
        command_id,
        principal_id,
        capability,
        expected_generation,
        body,
    );
    assert_eq!(receipt.disposition, CommandDisposition::Rejected(expected));
    receipt
}

fn found_cycle(store: &mut KernelStore) -> (PrincipalId, OperatingCycleId) {
    let bootstrap = PrincipalId::BOOTSTRAP;
    accepted(
        store,
        "found-create-society",
        bootstrap,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        CommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("Founding Society").unwrap(),
        },
    );
    accepted(
        store,
        "found-install-seed",
        bootstrap,
        Capability::InstallFoundingUniverseSeed,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallFoundingUniverseSeed {
            rendering_digest: Sha256Digest::of_bytes(b"UniverseSeed revision 1"),
        },
    );
    accepted(
        store,
        "found-install-office",
        bootstrap,
        Capability::InstallGrandArchitectOffice,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallGrandArchitectOffice,
    );
    accepted(
        store,
        "found-appoint-grand-architect",
        bootstrap,
        Capability::AppointInitialGrandArchitect,
        ExpectedGeneration::NotApplicable,
        CommandBody::AppointInitialGrandArchitect {
            actor_display_name: PrincipalDisplayName::parse("GA1 actor").unwrap(),
        },
    );
    accepted(
        store,
        "found-set-r0-ceiling",
        bootstrap,
        Capability::SetR0HardCeiling,
        ExpectedGeneration::NotApplicable,
        CommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::FOUNDING_SOCIETY_HARD_CEILING,
        },
    );
    accepted(
        store,
        "found-bootstrap",
        bootstrap,
        Capability::BootstrapSociety,
        ExpectedGeneration::NotApplicable,
        CommandBody::BootstrapSociety,
    );
    accepted(
        store,
        "found-propose-cycle",
        bootstrap,
        Capability::ProposeOperatingCycle,
        ExpectedGeneration::NotApplicable,
        CommandBody::ProposeOperatingCycle {
            treatment: OperatingCycleTreatment::DeterministicPiHostFixtureV1,
        },
    );
    let cycle_id = OperatingCycleId::new(1).unwrap();
    accepted(
        store,
        "found-admit-cycle",
        bootstrap,
        Capability::AdmitOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::AdmitOperatingCycle { cycle_id },
    );
    (PrincipalId::new(3).unwrap(), cycle_id)
}

#[test]
fn current_operating_cycle_generation_is_typed_and_distinguishes_absence_from_corruption() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-cycle-generation-read-{nonce}.sqlite"));
    let mut store = KernelStore::open(&path).unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    assert_eq!(
        store
            .current_operating_cycle_admission_generation(cycle_id)
            .unwrap(),
        AdmissionGeneration::INITIAL
    );

    accepted(
        &mut store,
        "cycle-generation-read-cancel",
        grand_architect,
        Capability::RequestCancellation,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::RequestCancellation {
            cycle_id,
            mode: society_kernel::CancellationMode::GracefulCancel,
        },
    );
    assert_eq!(
        store
            .current_operating_cycle_admission_generation(cycle_id)
            .unwrap(),
        AdmissionGeneration::try_from(1).unwrap()
    );

    let unknown = OperatingCycleId::new(9_999_999).unwrap();
    assert!(matches!(
        store.current_operating_cycle_admission_generation(unknown),
        Err(StoreError::OperatingCycleNotFound(found)) if found == unknown
    ));

    drop(store);
    let inspect = Connection::open(&path).unwrap();
    inspect
        .pragma_update(None, "ignore_check_constraints", "ON")
        .unwrap();
    inspect
        .execute(
            "UPDATE operating_cycles SET admission_generation = -1 WHERE operating_cycle_id = ?1",
            [cycle_id.value()],
        )
        .unwrap();
    drop(inspect);
    let corrupted = KernelStore::open(&path).unwrap();
    assert!(matches!(
        corrupted.current_operating_cycle_admission_generation(cycle_id),
        Err(StoreError::LedgerCorruption(
            "operating cycle has invalid admission generation"
        ))
    ));
    drop(corrupted);
    fs::remove_file(path).unwrap();
}

/// The M5 Office Ready fact is no longer a synthetic service assertion. This
/// fixture builds the provider-free deterministic child receipt chain needed
/// before an Office can open ordinary turns. Its Office-session reservation
/// deliberately remains active: only typed Pi turn checkpoints may debit it,
/// and a later typed Dispose receipt will reconcile its unused remainder. It
/// is not evidence of a paid Pi run.
fn ready_supervised_office_session(
    store: &mut KernelStore,
    architect: PrincipalId,
    cycle_id: OperatingCycleId,
    session_id: GrandArchitectOfficeSessionId,
    label: &str,
    reservation_amount: UsdMicros,
) {
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    accepted(
        store,
        &format!("{label}-reserve"),
        architect,
        Capability::ReserveBudget,
        generation,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: reservation_amount,
        },
    );
    let epoch = SupervisorEpochId::new(1).unwrap();
    let epoch_identity = SupervisorEpochIdentity::parse("foundation-supervisor-1").unwrap();
    accepted(
        store,
        &format!("{label}-epoch"),
        PrincipalId::KERNEL,
        Capability::OpenSupervisorEpoch,
        ExpectedGeneration::NotApplicable,
        CommandBody::OpenSupervisorEpoch {
            supervisor_epoch_id: epoch,
            supervisor_epoch_identity: epoch_identity.clone(),
        },
    );
    let session_identity = PiBoundarySessionIdentity::parse(format!("pi-{label}-session")).unwrap();
    let spawn_nonce = SpawnNonce::parse(format!("pi-{label}-nonce")).unwrap();
    accepted(
        store,
        &format!("{label}-admit"),
        PrincipalId::KERNEL,
        Capability::AdmitPiChildSpawn,
        generation,
        CommandBody::AdmitPiChildSpawn {
            operating_cycle_id: cycle_id,
            owner: PiChildOwner::GrandArchitectOfficeSession(session_id),
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id:
                society_kernel::ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            native_workspace_id: NativeWorkspaceId::parse(format!("workspace-{label}")).unwrap(),
            canonical_workspace_path: CanonicalWorkspacePath::parse(format!("/tmp/{label}"))
                .unwrap(),
            supervisor_epoch_id: epoch,
            supervisor_epoch_identity: epoch_identity,
            pi_session_identity: session_identity.clone(),
            spawn_nonce: spawn_nonce.clone(),
        },
    );
    let child = ChildProcessId::new(1).unwrap();
    accepted(
        store,
        &format!("{label}-spawn"),
        PrincipalId::KERNEL,
        Capability::RecordInertChildSpawn,
        generation,
        CommandBody::RecordInertChildSpawn {
            pi_child_spawn_admission_id: PiChildSpawnAdmissionId::new(1).unwrap(),
            child_identity: SupervisedChildIdentity::parse(format!("child-{label}")).unwrap(),
            direct_child_pid: NativeChildPid::try_from(3001).unwrap(),
            process_group_id: OwnedProcessGroupId::try_from(3001).unwrap(),
        },
    );
    accepted(
        store,
        &format!("{label}-adapter-ready"),
        PrincipalId::KERNEL,
        Capability::RecordPiAdapterReady,
        generation,
        CommandBody::RecordPiAdapterReady {
            child_process_id: child,
            pi_session_identity: session_identity.clone(),
            spawn_nonce,
        },
    );
    let correlation = PiCorrelationIdentity::parse(format!("create-{label}")).unwrap();
    let create_digest = Sha256Digest::of_bytes(format!("create-{label}").as_bytes());
    accepted(
        store,
        &format!("{label}-create-authorized"),
        PrincipalId::KERNEL,
        Capability::AuthorizePiCreateSession,
        generation,
        CommandBody::AuthorizePiCreateSession {
            child_process_id: child,
            correlation_identity: correlation.clone(),
            create_request_digest: create_digest,
        },
    );
    accepted(
        store,
        &format!("{label}-create-delivered"),
        PrincipalId::KERNEL,
        Capability::RecordPiCreateSessionDelivery,
        generation,
        CommandBody::RecordPiCreateSessionDelivery {
            child_process_id: child,
            correlation_identity: correlation,
            create_request_digest: create_digest,
        },
    );
    accepted(
        store,
        &format!("{label}-session-ready"),
        PrincipalId::KERNEL,
        Capability::RecordPiSessionReady,
        generation,
        CommandBody::RecordPiSessionReady {
            child_process_id: child,
            pi_session_identity: session_identity,
        },
    );
    accepted(
        store,
        &format!("{label}-office-ready"),
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        generation,
        CommandBody::RecordOfficeSessionReady { session_id },
    );
}

#[test]
fn founding_cycle_is_idempotent_fenced_and_replayable() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);

    let first = submit(
        &mut store,
        "ga-quiesce-generation-zero",
        grand_architect,
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    let repeat = submit(
        &mut store,
        "ga-quiesce-generation-zero",
        grand_architect,
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    assert_eq!(repeat.disposition, first.disposition);
    assert!(repeat.idempotent);

    rejected(
        &mut store,
        "ga-resume-stale-generation-zero",
        grand_architect,
        Capability::ResumeOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::ResumeOperatingCycle { cycle_id },
        Rejection::StaleAdmissionGeneration,
    );
    accepted(
        &mut store,
        "kernel-cycle-drained-one",
        PrincipalId::KERNEL,
        Capability::RecordCycleDrained,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordCycleDrained { cycle_id },
    );
    accepted(
        &mut store,
        "ga-resume-generation-one",
        grand_architect,
        Capability::ResumeOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap()),
        CommandBody::ResumeOperatingCycle { cycle_id },
    );
    accepted(
        &mut store,
        "ga-quiesce-generation-one",
        grand_architect,
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap()),
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    accepted(
        &mut store,
        "kernel-cycle-drained-two",
        PrincipalId::KERNEL,
        Capability::RecordCycleDrained,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordCycleDrained { cycle_id },
    );
    accepted(
        &mut store,
        "ga-reconcile-cycle",
        grand_architect,
        Capability::ReconcileOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(2).unwrap()),
        CommandBody::ReconcileOperatingCycle { cycle_id },
    );
    accepted(
        &mut store,
        "ga-close-cycle",
        grand_architect,
        Capability::CloseOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(2).unwrap()),
        CommandBody::CloseOperatingCycle { cycle_id },
    );

    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::OperatingCycleStateChanged {
            state: OperatingCycleState::Closed,
            ..
        }
    )));
    assert!(store.validate_replayed_materialized_state().is_ok());
    assert_eq!(store.command_count().unwrap(), 16);
}

#[test]
fn project_charter_activation_and_close_blocker_are_typed_and_replayable() {
    let path = std::env::temp_dir().join(format!(
        "society-typed-graph-revisions-{}-{}.sqlite",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut store = KernelStore::open(&path).unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    accepted(
        &mut store,
        "coord-start-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        generation,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "coord-create-project",
        grand_architect,
        Capability::CreateProject,
        generation,
        CommandBody::CreateProject {
            operating_cycle_id: cycle_id,
            project_name: ProjectName::parse("coordination spine").unwrap(),
        },
    );
    let project_id = ProjectId::new(1).unwrap();
    rejected(
        &mut store,
        "coord-charter-proposed-rejected",
        grand_architect,
        Capability::CharterProject,
        generation,
        CommandBody::CharterProject {
            operating_cycle_id: cycle_id,
            project_id,
            objective: ProjectObjectiveText::parse("Exercise durable coordination.").unwrap(),
            initial_milestone: ProjectMilestoneName::parse("Charter accepted").unwrap(),
            stop_condition: ProjectStopConditionText::parse("No reliable path remains.").unwrap(),
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "coord-challenge-project",
        grand_architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle_id,
            project_id,
            target: ProjectState::Challenged,
        },
    );
    accepted(
        &mut store,
        "coord-charter-project",
        grand_architect,
        Capability::CharterProject,
        generation,
        CommandBody::CharterProject {
            operating_cycle_id: cycle_id,
            project_id,
            objective: ProjectObjectiveText::parse("Exercise durable coordination.").unwrap(),
            initial_milestone: ProjectMilestoneName::parse("Charter accepted").unwrap(),
            stop_condition: ProjectStopConditionText::parse("No reliable path remains.").unwrap(),
        },
    );
    accepted(
        &mut store,
        "coord-activate-project",
        grand_architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle_id,
            project_id,
            target: ProjectState::Active,
        },
    );
    accepted(
        &mut store,
        "coord-create-episode",
        grand_architect,
        Capability::CreateEpisode,
        generation,
        CommandBody::CreateEpisode {
            operating_cycle_id: cycle_id,
            project_id,
        },
    );
    let episode_id = CausalEpisodeId::new(1).unwrap();
    accepted(
        &mut store,
        "coord-admit-episode",
        grand_architect,
        Capability::TransitionEpisode,
        generation,
        CommandBody::TransitionEpisode {
            operating_cycle_id: cycle_id,
            causal_episode_id: episode_id,
            target: EpisodeState::Admitted,
        },
    );
    accepted(
        &mut store,
        "coord-add-observation",
        grand_architect,
        Capability::AddGraphObjectRevision,
        generation,
        CommandBody::AddGraphObjectRevision {
            operating_cycle_id: cycle_id,
            project_id,
            causal_episode_id: Some(episode_id),
            graph_object_id: None,
            body: GraphRevisionBody::Observation {
                observation: ObservationRevisionText::parse("Observed a reproducible constraint.")
                    .unwrap(),
            },
        },
    );
    accepted(
        &mut store,
        "coord-add-hypothesis",
        grand_architect,
        Capability::AddGraphObjectRevision,
        generation,
        CommandBody::AddGraphObjectRevision {
            operating_cycle_id: cycle_id,
            project_id,
            causal_episode_id: None,
            graph_object_id: None,
            body: GraphRevisionBody::Hypothesis {
                hypothesis: HypothesisRevisionText::parse("A bounded change may resolve it.")
                    .unwrap(),
            },
        },
    );
    let observation = GraphRevisionId::new(1).unwrap();
    let hypothesis = GraphRevisionId::new(2).unwrap();
    for (id, revision) in [
        ("coord-commit-observation", observation),
        ("coord-commit-hypothesis", hypothesis),
    ] {
        accepted(
            &mut store,
            id,
            grand_architect,
            Capability::CommitGraphRevision,
            generation,
            CommandBody::CommitGraphRevision {
                operating_cycle_id: cycle_id,
                graph_revision_id: revision,
            },
        );
    }
    accepted(
        &mut store,
        "coord-support-edge",
        grand_architect,
        Capability::AddGraphEdge,
        generation,
        CommandBody::AddGraphEdge {
            operating_cycle_id: cycle_id,
            project_id,
            from_graph_revision_id: observation,
            to_graph_revision_id: hypothesis,
            edge_kind: GraphEdgeKind::Supports,
        },
    );
    accepted(
        &mut store,
        "coord-request-review",
        grand_architect,
        Capability::RequestAdversarialReview,
        generation,
        CommandBody::RequestAdversarialReview {
            operating_cycle_id: cycle_id,
            project_id,
            target_graph_revision_id: hypothesis,
        },
    );
    accepted(
        &mut store,
        "coord-trigger-postmortem",
        grand_architect,
        Capability::TriggerPostmortem,
        generation,
        CommandBody::TriggerPostmortem {
            operating_cycle_id: cycle_id,
            project_id,
            causal_episode_id: None,
        },
    );
    let postmortem_id = PostmortemId::new(1).unwrap();
    accepted(
        &mut store,
        "coord-record-causal-claim",
        grand_architect,
        Capability::RecordPostmortemCausalClaim,
        generation,
        CommandBody::RecordPostmortemCausalClaim {
            operating_cycle_id: cycle_id,
            postmortem_id,
            claim_kind: PostmortemCausalClaimKind::ContributingCondition,
            claim: PostmortemCausalClaimText::parse("The review exposed a missing discriminant.")
                .unwrap(),
        },
    );
    accepted(
        &mut store,
        "coord-propose-postmortem-action",
        grand_architect,
        Capability::ProposePostmortemAction,
        generation,
        CommandBody::ProposePostmortemAction {
            operating_cycle_id: cycle_id,
            postmortem_id,
            action_kind: PostmortemActionKind::CreateFollowUpTicket,
            action: PostmortemActionProposalText::parse(
                "Create a separately admitted follow-up Ticket.",
            )
            .unwrap(),
        },
    );
    accepted(
        &mut store,
        "coord-close-postmortem",
        grand_architect,
        Capability::ClosePostmortem,
        generation,
        CommandBody::ClosePostmortem {
            operating_cycle_id: cycle_id,
            postmortem_id,
        },
    );
    let review_id = AdversarialReviewId::new(1).unwrap();
    rejected(
        &mut store,
        "coord-unassigned-reviewer-cannot-submit",
        PrincipalId::KERNEL,
        Capability::SubmitReviewChallenge,
        generation,
        CommandBody::SubmitReviewChallenge {
            operating_cycle_id: cycle_id,
            adversarial_review_id: review_id,
            target_graph_revision_id: hypothesis,
            author_principal_id: grand_architect,
            severity: ReviewChallengeSeverity::High,
            failure_hypothesis: ReviewFailureHypothesis::parse(
                "The causal direction may be inverted.",
            )
            .unwrap(),
        },
        Rejection::CapabilityNotGranted,
    );
    rejected(
        &mut store,
        "coord-reject-self-assigned-adversarial-reviewer",
        PrincipalId::KERNEL,
        Capability::AssignAdversarialReviewer,
        generation,
        CommandBody::AssignAdversarialReviewer {
            operating_cycle_id: cycle_id,
            adversarial_review_id: review_id,
            reviewer_principal_id: grand_architect,
            reviewer_actor_instance_id: society_kernel::ActorInstanceId::new(1).unwrap(),
            reviewer_actor_attempt_id: society_kernel::ActorAttemptId::new(1).unwrap(),
        },
        Rejection::ReviewAssignmentNotIndependent,
    );
    accepted(
        &mut store,
        "coord-observe-project",
        grand_architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle_id,
            project_id,
            target: ProjectState::Observing,
        },
    );
    rejected(
        &mut store,
        "coord-close-with-milestone",
        grand_architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle_id,
            project_id,
            target: ProjectState::Closed,
        },
        Rejection::ProjectCloseBlocked,
    );
    accepted(
        &mut store,
        "coord-complete-milestone",
        grand_architect,
        Capability::CompleteProjectMilestone,
        generation,
        CommandBody::CompleteProjectMilestone {
            operating_cycle_id: cycle_id,
            project_milestone_id: society_kernel::ProjectMilestoneId::new(1).unwrap(),
        },
    );
    rejected(
        &mut store,
        "coord-close-project-with-open-review",
        grand_architect,
        Capability::TransitionProject,
        generation,
        CommandBody::TransitionProject {
            operating_cycle_id: cycle_id,
            project_id,
            target: ProjectState::Closed,
        },
        Rejection::ProjectCloseBlocked,
    );
    assert!(store.validate_replayed_materialized_state().is_ok());
    drop(store);

    // A hostile direct writer can remove a schema trigger; fresh replay still
    // reconstructs the typed command body and catches a changed semantic body.
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "DROP TRIGGER observation_revision_cannot_update;
             UPDATE observation_revisions
             SET observation_text = 'tampered observation body'
             WHERE graph_revision_id = 1;",
        )
        .unwrap();
    drop(connection);
    assert!(
        KernelStore::open(&path)
            .unwrap()
            .validate_replayed_materialized_state()
            .is_err()
    );

    // The nested typed command body participates in its parent command's
    // request fingerprint, so an in-place semantic edit is replay corruption.
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "UPDATE observation_revisions
             SET observation_text = 'Observed a reproducible constraint.'
             WHERE graph_revision_id = 1;
             UPDATE command_add_observation_revision
             SET observation_text = 'tampered command body'
             WHERE command_row_id = (SELECT command_row_id FROM commands WHERE command_id = 'coord-add-observation');",
        )
        .unwrap();
    drop(connection);
    assert!(KernelStore::open(&path).unwrap().replay_ledger().is_err());

    // Cardinality is independently checked: even after bypassing the schema
    // matching trigger, a second kind body makes ledger replay corrupt.
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "UPDATE command_add_observation_revision
             SET observation_text = 'Observed a reproducible constraint.'
             WHERE command_row_id = (SELECT command_row_id FROM commands WHERE command_id = 'coord-add-observation');
             DROP TRIGGER hypothesis_revision_matches_object_kind;
             INSERT INTO hypothesis_revisions(graph_revision_id, hypothesis_text)
             VALUES (1, 'forged second body');",
        )
        .unwrap();
    drop(connection);
    assert!(KernelStore::open(&path).unwrap().replay_ledger().is_err());
    fs::remove_file(path).unwrap();
}

#[test]
fn current_schema_reopens_after_atomic_fresh_bootstrap() {
    let path = std::env::temp_dir().join(format!(
        "society-fresh-schema-{}-{}.sqlite",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(include_str!("../../../migrations/0001_kernel.sql"))
        .unwrap();
    assert_eq!(
        connection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        6
    );
    drop(connection);
    drop(KernelStore::open(&path).unwrap());
    let reopened = Connection::open(&path).unwrap();
    assert_eq!(
        reopened
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        6
    );
    assert_eq!(
        reopened
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    let objects_table: String = reopened
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'objects'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(objects_table, "objects");
    drop(reopened);
    fs::remove_file(path).unwrap();
}

#[test]
fn historical_schema_one_is_rejected_without_current_schema_mutation() {
    let path = std::env::temp_dir().join(format!(
        "society-historical-schema-one-{}-{}.sqlite",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let historical = Connection::open(&path).unwrap();
    // A schema-1 identity is sufficient to prove the fail-closed boundary:
    // current kernels must never reinterpret an old ledger as canonical just
    // because both happen to be SQLite databases.
    historical
        .execute_batch(
            "CREATE TABLE historical_v1_ledger_marker (entry_id INTEGER PRIMARY KEY);
             INSERT INTO historical_v1_ledger_marker VALUES (1);
             PRAGMA user_version = 1;",
        )
        .unwrap();
    drop(historical);

    assert!(matches!(
        KernelStore::open(&path),
        Err(StoreError::UnsupportedSchemaVersion(1))
    ));
    let inspection = Connection::open(&path).unwrap();
    assert_eq!(
        inspection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        inspection
            .query_row(
                "SELECT COUNT(*) FROM historical_v1_ledger_marker",
                [],
                |row| { row.get::<_, i64>(0) }
            )
            .unwrap(),
        1
    );
    drop(inspection);
    fs::remove_file(path).unwrap();
}

#[test]
fn rejection_wire_codes_have_one_closed_round_trip_authority() {
    for rejection in Rejection::ALL {
        assert_eq!(Rejection::try_from(rejection.as_i64()).unwrap(), *rejection);
        assert_eq!(Rejection::try_from(rejection.as_u8()).unwrap(), *rejection);
    }
    assert!(Rejection::try_from(0_i64).is_err());
    assert!(Rejection::try_from(255_u8).is_err());
}

#[test]
fn founding_policy_uses_closed_exact_cycle_treatments() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("society-qualification-treatment-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let bootstrap = PrincipalId::BOOTSTRAP;
    accepted(
        &mut store,
        "policy-create-society",
        bootstrap,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        CommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("Founding Society policy test").unwrap(),
        },
    );
    accepted(
        &mut store,
        "policy-install-seed",
        bootstrap,
        Capability::InstallFoundingUniverseSeed,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallFoundingUniverseSeed {
            rendering_digest: Sha256Digest::of_bytes(b"policy seed"),
        },
    );
    accepted(
        &mut store,
        "policy-install-office",
        bootstrap,
        Capability::InstallGrandArchitectOffice,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallGrandArchitectOffice,
    );
    accepted(
        &mut store,
        "policy-appoint-ga",
        bootstrap,
        Capability::AppointInitialGrandArchitect,
        ExpectedGeneration::NotApplicable,
        CommandBody::AppointInitialGrandArchitect {
            actor_display_name: PrincipalDisplayName::parse("policy GA").unwrap(),
        },
    );
    rejected(
        &mut store,
        "policy-reject-wrong-r0-ceiling",
        bootstrap,
        Capability::SetR0HardCeiling,
        ExpectedGeneration::NotApplicable,
        CommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::PINNED_PI_SDK_CYCLE_CEILING,
        },
        Rejection::BudgetPolicyViolation,
    );
    accepted(
        &mut store,
        "policy-set-r0-ceiling",
        bootstrap,
        Capability::SetR0HardCeiling,
        ExpectedGeneration::NotApplicable,
        CommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::FOUNDING_SOCIETY_HARD_CEILING,
        },
    );
    accepted(
        &mut store,
        "policy-bootstrap",
        bootstrap,
        Capability::BootstrapSociety,
        ExpectedGeneration::NotApplicable,
        CommandBody::BootstrapSociety,
    );
    accepted(
        &mut store,
        "policy-propose-qualification-treatment",
        bootstrap,
        Capability::ProposeOperatingCycle,
        ExpectedGeneration::NotApplicable,
        CommandBody::ProposeOperatingCycle {
            treatment: OperatingCycleTreatment::PiSdkQualificationV1,
        },
    );
    accepted(
        &mut store,
        "policy-admit-qualification-treatment",
        bootstrap,
        Capability::AdmitOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::AdmitOperatingCycle {
            cycle_id: OperatingCycleId::new(1).unwrap(),
        },
    );
    assert_eq!(
        OperatingCycleTreatment::PiSdkQualificationV1.budget_ceiling(),
        UsdMicros::PI_SDK_QUALIFICATION_CEILING
    );
    assert_eq!(
        OperatingCycleTreatment::PinnedPiSdkLiveV1.budget_ceiling(),
        UsdMicros::PINNED_PI_SDK_CYCLE_CEILING
    );
    assert_eq!(
        OperatingCycleTreatment::DeterministicPiHostFixtureV1.budget_ceiling(),
        UsdMicros::PINNED_PI_SDK_CYCLE_CEILING
    );
    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::OperatingCycleProposed {
            treatment: OperatingCycleTreatment::PiSdkQualificationV1,
            ..
        }
    )));
    let inspection = rusqlite::Connection::open(&path).unwrap();
    let (treatment, ceiling): (i64, i64) = inspection
        .query_row(
            "SELECT c.treatment, e.ceiling_micros
             FROM operating_cycles c
             JOIN budget_envelope_constraints b
               ON b.operating_cycle_id = c.operating_cycle_id
             JOIN budget_envelopes e ON e.budget_envelope_id = b.budget_envelope_id
             WHERE c.operating_cycle_id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        (treatment, ceiling),
        (
            OperatingCycleTreatment::PiSdkQualificationV1 as i64,
            UsdMicros::PI_SDK_QUALIFICATION_CEILING.value(),
        )
    );
    drop(inspection);
    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn actor_grant_must_match_the_cycle_pinned_office_occupancy() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-occupancy-scope-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (_, cycle_id) = found_cycle(&mut store);

    // Succession is not a production command in this bounded slice. This
    // direct fixture creates a second, active office occupancy and its exact
    // grant, while the already proposed cycle remains pinned to occupancy 1.
    let fixture = rusqlite::Connection::open(&path).unwrap();
    fixture
        .execute(
            "UPDATE office_occupancies SET active = 0 WHERE office_occupancy_id = 1",
            [],
        )
        .unwrap();
    fixture
        .execute(
            "INSERT INTO principals(principal_id, principal_kind, display_name, active)
             VALUES (4, 3, 'successor fixture', 1)",
            [],
        )
        .unwrap();
    fixture
        .execute(
            "INSERT INTO office_occupancies(office_id, principal_id, active, appointed_by_command_id)
             VALUES (1, 4, 1, 1)",
            [],
        )
        .unwrap();
    fixture
        .execute(
            "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id,
                                              grant_state, grant_origin, granted_by_command_id, consumed_by_command_id)
             VALUES (4, 9, 2, 1, 2, 1, NULL)",
            [],
        )
        .unwrap();
    drop(fixture);

    rejected(
        &mut store,
        "successor-cannot-quiesce-predecessor-cycle",
        PrincipalId::new(4).unwrap(),
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle { cycle_id },
        Rejection::CapabilityNoLongerActive,
    );

    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn forged_grant_principal_cannot_borrow_an_active_occupancy() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-forged-grant-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (_, cycle_id) = found_cycle(&mut store);
    let fixture = rusqlite::Connection::open(&path).unwrap();
    fixture
        .execute(
            "INSERT INTO principals(principal_id, principal_kind, display_name, active)
             VALUES (4, 3, 'forged grant fixture', 1)",
            [],
        )
        .unwrap();
    // Normal SQL cannot create a grant whose principal does not hold the
    // named occupancy.
    assert!(fixture
        .execute(
            "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id,
                                              grant_state, grant_origin, granted_by_command_id, consumed_by_command_id)
             VALUES (4, 9, 1, 1, 2, 1, NULL)",
            [],
        )
        .is_err());
    // An attacker able to remove the schema defense can still manufacture the
    // row. The runtime authorization join must reject it before transition.
    fixture
        .execute_batch(
            "DROP TRIGGER capability_grant_principal_matches_occupancy_on_insert;
             DROP TRIGGER capability_grant_principal_matches_occupancy_on_update;
             DROP TRIGGER occupancy_principal_matches_existing_grants;",
        )
        .unwrap();
    fixture
        .execute(
            "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id,
                                              grant_state, grant_origin, granted_by_command_id, consumed_by_command_id)
             VALUES (4, 9, 1, 1, 2, 1, NULL)",
            [],
        )
        .unwrap();
    drop(fixture);

    rejected(
        &mut store,
        "forged-principal-cannot-quiesce",
        PrincipalId::new(4).unwrap(),
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle { cycle_id },
        Rejection::CapabilityNoLongerActive,
    );

    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn office_turn_and_cross_cut_budget_freeze_unknown_cost() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-office-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        &mut store,
        grand_architect,
        cycle_id,
        session_id,
        "office-turn",
        UsdMicros::new(1).unwrap(),
    );
    accepted(
        &mut store,
        "ga-open-office-turn",
        grand_architect,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
    );
    rejected(
        &mut store,
        "ga-open-concurrent-office-turn",
        grand_architect,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
        Rejection::InvalidLifecycleTransition,
    );
    rejected(
        &mut store,
        "kernel-settle-office-turn",
        PrincipalId::KERNEL,
        Capability::SettleOfficeTurn,
        ExpectedGeneration::NotApplicable,
        CommandBody::SettleOfficeTurn {
            turn_id: OfficeTurnId::new(1).unwrap(),
            terminal_receipt_id: society_kernel::PiOfficeTurnTerminalReceiptId::new(1).unwrap(),
        },
        Rejection::PiOfficeTurnTerminalEvidenceMissing,
    );

    rejected(
        &mut store,
        "ga-reject-zero-budget-reservation",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::ZERO,
        },
        Rejection::BudgetCeilingExceeded,
    );
    accepted(
        &mut store,
        "ga-reserve-six-hundred-thousand",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(600_000).unwrap(),
        },
    );
    rejected(
        &mut store,
        "ga-reject-cycle-cap-overrun",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(500_000).unwrap(),
        },
        Rejection::BudgetCeilingExceeded,
    );
    accepted(
        &mut store,
        "kernel-reconcile-three-hundred-thousand",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(2).unwrap(),
            observation: CostObservation::Known(UsdMicros::new(300_000).unwrap()),
        },
    );
    accepted(
        &mut store,
        "ga-reserve-remaining-seven-hundred-thousand",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(699_999).unwrap(),
        },
    );
    accepted(
        &mut store,
        "kernel-freeze-unknown-cost",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(3).unwrap(),
            observation: CostObservation::Unknown(CostUnknownReason::AdapterStreamInterrupted),
        },
    );
    let stale = rejected(
        &mut store,
        "ga-reserve-after-unknown-cost",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(1).unwrap(),
        },
        Rejection::StaleAdmissionGeneration,
    );
    assert_eq!(
        store
            .command_receipt(&CommandId::parse("ga-reserve-after-unknown-cost").unwrap())
            .unwrap()
            .unwrap()
            .disposition,
        stale.disposition
    );
    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::BudgetAdmissionFrozen {
            reason: BudgetFreezeReason::Unknown(CostUnknownReason::AdapterStreamInterrupted),
            ..
        }
    )));
}

#[test]
fn unavailable_cost_reason_survives_event_replay() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);

    accepted(
        &mut store,
        "ga-start-unavailable-cost-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "ga-reserve-unavailable-cost",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(10_000).unwrap(),
        },
    );
    accepted(
        &mut store,
        "kernel-freeze-unavailable-cost",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Unavailable(CostUnavailableReason::ProviderUnavailable),
        },
    );

    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::BudgetAdmissionFrozen {
            reason: BudgetFreezeReason::Unavailable(CostUnavailableReason::ProviderUnavailable),
            ..
        }
    )));
}

#[test]
fn quiesce_cancellation_must_reconcile_before_resuming_admission() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());

    accepted(
        &mut store,
        "ga-request-quiesce-cancellation",
        grand_architect,
        Capability::RequestCancellation,
        zero,
        CommandBody::RequestCancellation {
            cycle_id,
            mode: society_kernel::CancellationMode::Quiesce,
        },
    );
    rejected(
        &mut store,
        "kernel-cannot-reconcile-quiescing-cancellation",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "kernel-record-quiesce-drained",
        PrincipalId::KERNEL,
        Capability::RecordCycleDrained,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordCycleDrained { cycle_id },
    );
    rejected(
        &mut store,
        "ga-cannot-resume-unreconciled-cancellation",
        grand_architect,
        Capability::ResumeOperatingCycle,
        one,
        CommandBody::ResumeOperatingCycle { cycle_id },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "kernel-reconcile-quiesce-cancellation",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
    );
    accepted(
        &mut store,
        "ga-resume-reconciled-cancellation",
        grand_architect,
        Capability::ResumeOperatingCycle,
        one,
        CommandBody::ResumeOperatingCycle { cycle_id },
    );
    rejected(
        &mut store,
        "kernel-cannot-reconcile-cancellation-after-resume",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
        Rejection::CancellationAlreadyTerminal,
    );
}

#[test]
fn office_turn_purpose_and_session_fences_follow_cycle_state() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-purpose-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        &mut store,
        grand_architect,
        cycle_id,
        session_id,
        "office-purpose",
        UsdMicros::new(1).unwrap(),
    );
    accepted(
        &mut store,
        "ga-quiesce-purpose-cycle",
        grand_architect,
        Capability::QuiesceOperatingCycle,
        zero,
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    rejected(
        &mut store,
        "kernel-stale-ready-after-quiesce",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        zero,
        CommandBody::RecordOfficeSessionReady { session_id },
        Rejection::StaleAdmissionGeneration,
    );
    rejected(
        &mut store,
        "ga-ordinary-turn-rejected-while-quiescing",
        grand_architect,
        Capability::OpenOfficeTurn,
        one,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "ga-recovery-turn-while-quiescing",
        grand_architect,
        Capability::OpenOfficeTurn,
        one,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::Recovery,
        },
    );
    rejected(
        &mut store,
        "kernel-settle-recovery-turn",
        PrincipalId::KERNEL,
        Capability::SettleOfficeTurn,
        ExpectedGeneration::NotApplicable,
        CommandBody::SettleOfficeTurn {
            turn_id: OfficeTurnId::new(1).unwrap(),
            terminal_receipt_id: society_kernel::PiOfficeTurnTerminalReceiptId::new(1).unwrap(),
        },
        Rejection::PiOfficeTurnTerminalEvidenceMissing,
    );
    // A real supervised Office may run recovery turns while the cycle is
    // quiescing, but cannot claim the cycle is drained while its native child
    // still has no terminal/containment receipts. M5 deliberately keeps the
    // former synthetic drained path unavailable.
    rejected(
        &mut store,
        "kernel-reject-purpose-cycle-drained-with-live-child",
        PrincipalId::KERNEL,
        Capability::RecordCycleDrained,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordCycleDrained { cycle_id },
        Rejection::InvalidLifecycleTransition,
    );
    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::OfficeTurnOpened {
            purpose: OfficeTurnPurpose::Recovery,
            ..
        }
    )));
}

#[test]
fn terminal_session_fact_cannot_close_an_active_turn() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-terminal-fence-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        &mut store,
        grand_architect,
        cycle_id,
        session_id,
        "office-terminal",
        UsdMicros::new(1).unwrap(),
    );
    accepted(
        &mut store,
        "ga-open-terminal-fence-turn",
        grand_architect,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
    );
    accepted(
        &mut store,
        "ga-cancel-terminal-fence-cycle",
        grand_architect,
        Capability::RequestCancellation,
        zero,
        CommandBody::RequestCancellation {
            cycle_id,
            mode: society_kernel::CancellationMode::GracefulCancel,
        },
    );
    rejected(
        &mut store,
        "kernel-cannot-terminal-active-turn",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionTerminal,
        one,
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state: OfficeSessionTerminalState::Cancelled,
        },
        Rejection::InvalidLifecycleTransition,
    );
}

#[test]
fn known_overrun_fences_admission_and_frozen_charge_blocks_cycle_close() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-known-overrun-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "ga-reserve-known-overrun",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(600_000).unwrap(),
        },
    );
    accepted(
        &mut store,
        "kernel-record-known-overrun",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Known(UsdMicros::new(700_000).unwrap()),
        },
    );
    let cancellation_request_id = store
        .replay_ledger()
        .unwrap()
        .into_iter()
        .find_map(|event| match event.body {
            EventBody::BudgetAdmissionFrozen {
                cancellation_request_id,
                reason: BudgetFreezeReason::KnownOverrun { observed, reserved },
                ..
            } if observed == UsdMicros::new(700_000).unwrap()
                && reserved == UsdMicros::new(600_000).unwrap() =>
            {
                Some(cancellation_request_id)
            }
            _ => None,
        })
        .expect("known overrun must become a typed frozen fact");
    assert_eq!(
        cancellation_request_id,
        CancellationRequestId::new(1).unwrap()
    );

    rejected(
        &mut store,
        "ga-admission-fenced-by-known-overrun",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(1).unwrap(),
        },
        Rejection::StaleAdmissionGeneration,
    );
    accepted(
        &mut store,
        "kernel-close-known-overrun-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionTerminal,
        one,
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state: OfficeSessionTerminalState::Cancelled,
        },
    );
    accepted(
        &mut store,
        "kernel-reconcile-known-overrun-cancellation",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        },
    );
    rejected(
        &mut store,
        "ga-cannot-resume-frozen-known-overrun",
        grand_architect,
        Capability::ResumeOperatingCycle,
        one,
        CommandBody::ResumeOperatingCycle { cycle_id },
        Rejection::IncompleteCycleReconciliation,
    );
    accepted(
        &mut store,
        "ga-begin-known-overrun-reconciliation",
        grand_architect,
        Capability::ReconcileOperatingCycle,
        one,
        CommandBody::ReconcileOperatingCycle { cycle_id },
    );
    rejected(
        &mut store,
        "ga-cannot-close-frozen-known-overrun",
        grand_architect,
        Capability::CloseOperatingCycle,
        one,
        CommandBody::CloseOperatingCycle { cycle_id },
        Rejection::IncompleteCycleReconciliation,
    );
}

#[test]
fn cost_postmortem_is_the_only_conservative_frozen_cost_resolution() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-postmortem-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "ga-reserve-postmortem-cost",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(10_000).unwrap(),
        },
    );
    accepted(
        &mut store,
        "kernel-freeze-postmortem-cost",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Unknown(CostUnknownReason::ProviderDidNotReport),
        },
    );
    let (postmortem_id, cancellation_request_id) = store
        .replay_ledger()
        .unwrap()
        .into_iter()
        .find_map(|event| match event.body {
            EventBody::BudgetAdmissionFrozen {
                postmortem_id,
                cancellation_request_id,
                reason: BudgetFreezeReason::Unknown(CostUnknownReason::ProviderDidNotReport),
                ..
            } => Some((postmortem_id, cancellation_request_id)),
            _ => None,
        })
        .expect("freeze opens a linked cost postmortem");

    rejected(
        &mut store,
        "ga-cannot-resolve-before-postmortem-is-closable",
        grand_architect,
        Capability::CloseCostPostmortem,
        one,
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution: CostPostmortemResolution::ConservativeFullReservation,
        },
        Rejection::IncompleteCycleReconciliation,
    );
    accepted(
        &mut store,
        "kernel-cancel-postmortem-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionTerminal,
        one,
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state: OfficeSessionTerminalState::Cancelled,
        },
    );
    accepted(
        &mut store,
        "kernel-reconcile-postmortem-cancellation",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        },
    );
    rejected(
        &mut store,
        "ga-reject-overrun-resolution-for-unknown-cost",
        grand_architect,
        Capability::CloseCostPostmortem,
        one,
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution: CostPostmortemResolution::ChargeObservedOverrun,
        },
        Rejection::InvalidCostPostmortemResolution,
    );
    accepted(
        &mut store,
        "ga-close-unknown-cost-postmortem",
        grand_architect,
        Capability::CloseCostPostmortem,
        one,
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution: CostPostmortemResolution::ConservativeFullReservation,
        },
    );
    accepted(
        &mut store,
        "ga-resume-after-conservative-postmortem-resolution",
        grand_architect,
        Capability::ResumeOperatingCycle,
        one,
        CommandBody::ResumeOperatingCycle { cycle_id },
    );
    assert!(store.validate_replayed_materialized_state().is_ok());
}

#[test]
fn known_overrun_postmortem_records_actual_spend_even_above_admission_ceiling() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-known-overrun-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-actual-overrun-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "ga-reserve-actual-overrun",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(600_000).unwrap(),
        },
    );
    accepted(
        &mut store,
        "kernel-record-actual-overrun",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Known(UsdMicros::new(1_500_000).unwrap()),
        },
    );
    let (postmortem_id, cancellation_request_id) = store
        .replay_ledger()
        .unwrap()
        .into_iter()
        .find_map(|event| match event.body {
            EventBody::BudgetAdmissionFrozen {
                postmortem_id,
                cancellation_request_id,
                reason: BudgetFreezeReason::KnownOverrun { observed, reserved },
                ..
            } if observed == UsdMicros::new(1_500_000).unwrap()
                && reserved == UsdMicros::new(600_000).unwrap() =>
            {
                Some((postmortem_id, cancellation_request_id))
            }
            _ => None,
        })
        .expect("known overrun is preserved with its linked postmortem");
    accepted(
        &mut store,
        "kernel-cancel-actual-overrun-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionTerminal,
        one,
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state: OfficeSessionTerminalState::Cancelled,
        },
    );
    accepted(
        &mut store,
        "kernel-reconcile-actual-overrun-cancellation",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        },
    );
    accepted(
        &mut store,
        "ga-close-known-overrun-postmortem",
        grand_architect,
        Capability::CloseCostPostmortem,
        one,
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution: CostPostmortemResolution::ChargeObservedOverrun,
        },
    );
    let accounting = rusqlite::Connection::open(&path).unwrap();
    let mut statement = accounting
        .prepare("SELECT reserved_micros, spent_micros FROM budget_envelopes ORDER BY budget_envelope_id")
        .unwrap();
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec![(0, 1_500_000), (0, 1_500_000)]);
    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::CostPostmortemClosed {
            resolution: CostPostmortemResolution::ChargeObservedOverrun,
            charged,
            ..
        } if charged == UsdMicros::new(1_500_000).unwrap()
    )));
    assert!(store.validate_replayed_materialized_state().is_ok());
    drop(statement);
    drop(accounting);
    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn changed_typed_body_reusing_a_command_id_is_an_idempotency_conflict() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let command_id = CommandId::parse("same-command-id-different-body").unwrap();
    let quiesce_grant = store
        .active_capability_grant(grand_architect, Capability::QuiesceOperatingCycle)
        .unwrap()
        .unwrap();
    let first = store
        .execute(CommandRequest {
            command_id: command_id.clone(),
            principal_id: grand_architect,
            capability_grant_id: quiesce_grant,
            capability: Capability::QuiesceOperatingCycle,
            expected_generation: ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
            body: CommandBody::QuiesceOperatingCycle { cycle_id },
        })
        .unwrap();
    assert!(matches!(first.disposition, CommandDisposition::Accepted(_)));
    let count_after_first = store.command_count().unwrap();

    let resume_grant = store
        .active_capability_grant(grand_architect, Capability::ResumeOperatingCycle)
        .unwrap()
        .unwrap();
    assert!(matches!(
        store.execute(CommandRequest {
            command_id,
            principal_id: grand_architect,
            capability_grant_id: resume_grant,
            capability: Capability::ResumeOperatingCycle,
            expected_generation: ExpectedGeneration::Exact(
                AdmissionGeneration::try_from(1).unwrap()
            ),
            body: CommandBody::ResumeOperatingCycle { cycle_id },
        }),
        Err(society_kernel::StoreError::IdempotencyConflict)
    ));
    assert_eq!(store.command_count().unwrap(), count_after_first);
}

#[test]
fn pi_office_turn_requires_authorized_delivered_peer_terminal_chain_and_debits_only_delta() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-m6-office-turn-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();
    accepted(
        &mut store,
        "m6-start-office-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        &mut store,
        grand_architect,
        cycle_id,
        session_id,
        "m6-office",
        UsdMicros::new(100).unwrap(),
    );
    rejected(
        &mut store,
        "m6-generic-office-parent-reconciliation-is-fenced",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Known(UsdMicros::ZERO),
        },
        Rejection::OfficeSessionBudgetRequiresDispose,
    );
    let prompt_digest = Sha256Digest::of_bytes(b"sealed deterministic prompt");
    accepted(
        &mut store,
        "m6-seal-prompt",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: prompt_digest,
        },
    );
    accepted(
        &mut store,
        "m6-register-prompt",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(1).unwrap(),
        },
    );

    let open = accepted(
        &mut store,
        "m6-open-turn-one",
        grand_architect,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
    );
    let frontier = match open.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        other => panic!("unexpected open receipt: {other:?}"),
    };
    let turn_one = OfficeTurnId::new(1).unwrap();
    let correlation_one = PiCorrelationIdentity::parse("m6-prompt-one").unwrap();
    rejected(
        &mut store,
        "m6-reject-unsealed-prompt",
        PrincipalId::KERNEL,
        Capability::AuthorizePiOfficeTurnPrompt,
        zero,
        CommandBody::AuthorizePiOfficeTurnPrompt {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            prompt_content_object_id: society_kernel::ContentObjectId::new(1).unwrap(),
            prompt_digest: Sha256Digest::of_bytes(b"recombined"),
            frontier_event_id: frontier,
        },
        Rejection::PiOfficeTurnPromptBindingMismatch,
    );
    accepted(
        &mut store,
        "m6-authorize-prompt-one",
        PrincipalId::KERNEL,
        Capability::AuthorizePiOfficeTurnPrompt,
        zero,
        CommandBody::AuthorizePiOfficeTurnPrompt {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            prompt_content_object_id: society_kernel::ContentObjectId::new(1).unwrap(),
            prompt_digest,
            frontier_event_id: frontier,
        },
    );
    accepted(
        &mut store,
        "m6-deliver-prompt-one",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnPromptDelivery,
        zero,
        CommandBody::RecordPiOfficeTurnPromptDelivery {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            prompt_digest,
        },
    );
    accepted(
        &mut store,
        "m6-accept-prompt-one",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnPromptAccepted,
        zero,
        CommandBody::RecordPiOfficeTurnPromptAccepted {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            command_result_sequence: PiProtocolSequence::try_from(10).unwrap(),
        },
    );
    let usage_one = PiCumulativeUsage {
        input_tokens: PiTokenCount::try_from(1).unwrap(),
        output_tokens: PiTokenCount::try_from(1).unwrap(),
        cache_read_tokens: PiTokenCount::try_from(1).unwrap(),
        cache_write_tokens: PiTokenCount::try_from(1).unwrap(),
        total_tokens: PiTokenCount::try_from(4).unwrap(),
        provider_cost: ProviderCostBinary64::from_big_endian(0.000004_f64.to_bits().to_be_bytes())
            .unwrap(),
        ceiling_micro_usd: UsdMicros::new(4).unwrap(),
    };
    rejected(
        &mut store,
        "m6-reject-usage-before-prompt-acceptance-sequence",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnUsage,
        zero,
        CommandBody::RecordPiOfficeTurnUsage {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            protocol_sequence: PiProtocolSequence::try_from(10).unwrap(),
            usage: usage_one,
        },
        Rejection::PiOfficeTurnUsageNotMonotonic,
    );
    accepted(
        &mut store,
        "m6-usage-one",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnUsage,
        zero,
        CommandBody::RecordPiOfficeTurnUsage {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            // Control replies may legally interleave after AgentSettled. Only
            // the final prompt usage immediately precedes Settled.
            protocol_sequence: PiProtocolSequence::try_from(14).unwrap(),
            usage: usage_one,
        },
    );
    rejected(
        &mut store,
        "m6-reject-terminal-before-accepted-prompt-result",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnTerminal,
        zero,
        CommandBody::RecordPiOfficeTurnTerminal {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            agent_settled_sequence: PiProtocolSequence::try_from(10).unwrap(),
            final_usage_sequence: PiProtocolSequence::try_from(14).unwrap(),
            settled_sequence: PiProtocolSequence::try_from(15).unwrap(),
            disposition: PiOfficeTurnDisposition::Completed,
            assistant_outcome: PiOfficeTurnAssistantOutcome::ObservedStop,
            transcript_disposition:
                PiOfficeTurnTranscriptDisposition::DeferredUntilOfficeSessionDispose,
        },
        Rejection::PiOfficeTurnTerminalEvidenceMissing,
    );
    accepted(
        &mut store,
        "m6-terminal-one-with-interleaved-control-sequences",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnTerminal,
        zero,
        CommandBody::RecordPiOfficeTurnTerminal {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            agent_settled_sequence: PiProtocolSequence::try_from(11).unwrap(),
            final_usage_sequence: PiProtocolSequence::try_from(14).unwrap(),
            settled_sequence: PiProtocolSequence::try_from(15).unwrap(),
            disposition: PiOfficeTurnDisposition::Completed,
            assistant_outcome: PiOfficeTurnAssistantOutcome::ObservedStop,
            transcript_disposition:
                PiOfficeTurnTranscriptDisposition::DeferredUntilOfficeSessionDispose,
        },
    );
    accepted(
        &mut store,
        "m6-settle-one",
        PrincipalId::KERNEL,
        Capability::SettleOfficeTurn,
        ExpectedGeneration::NotApplicable,
        CommandBody::SettleOfficeTurn {
            turn_id: turn_one,
            terminal_receipt_id: PiOfficeTurnTerminalReceiptId::new(1).unwrap(),
        },
    );
    let late_usage = PiCumulativeUsage {
        provider_cost: ProviderCostBinary64::from_big_endian(0.0000041_f64.to_bits().to_be_bytes())
            .unwrap(),
        ceiling_micro_usd: UsdMicros::new(5).unwrap(),
        ..usage_one
    };
    rejected(
        &mut store,
        "m6-reject-usage-after-terminal",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnUsage,
        zero,
        CommandBody::RecordPiOfficeTurnUsage {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            protocol_sequence: PiProtocolSequence::try_from(16).unwrap(),
            usage: late_usage,
        },
        Rejection::PiOfficeTurnTerminalAlreadyRecorded,
    );
    rejected(
        &mut store,
        "m6-reject-usage-failure-after-terminal",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnUsageFailure,
        zero,
        CommandBody::RecordPiOfficeTurnUsageFailure {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            protocol_sequence: PiProtocolSequence::try_from(16).unwrap(),
            failure: PiOfficeTurnUsageFailure::Unknown(
                society_kernel::PiOfficeTurnUsageUnknownReason::TerminalEvidenceMissing,
            ),
        },
        Rejection::PiOfficeTurnTerminalAlreadyRecorded,
    );

    let open_two = accepted(
        &mut store,
        "m6-open-turn-two",
        grand_architect,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
    );
    let frontier_two = match open_two.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        other => panic!("unexpected second open receipt: {other:?}"),
    };
    let turn_two = OfficeTurnId::new(2).unwrap();
    let correlation_two = PiCorrelationIdentity::parse("m6-prompt-two").unwrap();
    accepted(
        &mut store,
        "m6-authorize-prompt-two",
        PrincipalId::KERNEL,
        Capability::AuthorizePiOfficeTurnPrompt,
        zero,
        CommandBody::AuthorizePiOfficeTurnPrompt {
            office_turn_id: turn_two,
            correlation_identity: correlation_two.clone(),
            prompt_content_object_id: society_kernel::ContentObjectId::new(1).unwrap(),
            prompt_digest,
            frontier_event_id: frontier_two,
        },
    );
    accepted(
        &mut store,
        "m6-deliver-prompt-two",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnPromptDelivery,
        zero,
        CommandBody::RecordPiOfficeTurnPromptDelivery {
            office_turn_id: turn_two,
            correlation_identity: correlation_two.clone(),
            prompt_digest,
        },
    );
    rejected(
        &mut store,
        "m6-reject-turn-two-acceptance-reusing-prior-settlement-sequence",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnPromptAccepted,
        zero,
        CommandBody::RecordPiOfficeTurnPromptAccepted {
            office_turn_id: turn_two,
            correlation_identity: correlation_two.clone(),
            command_result_sequence: PiProtocolSequence::try_from(15).unwrap(),
        },
        Rejection::PiOfficeTurnUsageNotMonotonic,
    );
    accepted(
        &mut store,
        "m6-accept-prompt-two",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnPromptAccepted,
        zero,
        CommandBody::RecordPiOfficeTurnPromptAccepted {
            office_turn_id: turn_two,
            correlation_identity: correlation_two.clone(),
            command_result_sequence: PiProtocolSequence::try_from(20).unwrap(),
        },
    );
    let usage_two = PiCumulativeUsage {
        provider_cost: ProviderCostBinary64::from_big_endian(0.0000085_f64.to_bits().to_be_bytes())
            .unwrap(),
        ceiling_micro_usd: UsdMicros::new(9).unwrap(),
        ..usage_one
    };
    accepted(
        &mut store,
        "m6-usage-two",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnUsage,
        zero,
        CommandBody::RecordPiOfficeTurnUsage {
            office_turn_id: turn_two,
            correlation_identity: correlation_two.clone(),
            protocol_sequence: PiProtocolSequence::try_from(22).unwrap(),
            usage: usage_two,
        },
    );
    accepted(
        &mut store,
        "m6-terminal-two",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnTerminal,
        zero,
        CommandBody::RecordPiOfficeTurnTerminal {
            office_turn_id: turn_two,
            correlation_identity: correlation_two,
            agent_settled_sequence: PiProtocolSequence::try_from(21).unwrap(),
            final_usage_sequence: PiProtocolSequence::try_from(22).unwrap(),
            settled_sequence: PiProtocolSequence::try_from(23).unwrap(),
            disposition: PiOfficeTurnDisposition::Completed,
            assistant_outcome: PiOfficeTurnAssistantOutcome::ObservedStop,
            transcript_disposition:
                PiOfficeTurnTranscriptDisposition::DeferredUntilOfficeSessionDispose,
        },
    );
    accepted(
        &mut store,
        "m6-settle-two",
        PrincipalId::KERNEL,
        Capability::SettleOfficeTurn,
        ExpectedGeneration::NotApplicable,
        CommandBody::SettleOfficeTurn {
            turn_id: turn_two,
            terminal_receipt_id: PiOfficeTurnTerminalReceiptId::new(2).unwrap(),
        },
    );
    let settled_deltas: Vec<_> = store
        .replay_ledger()
        .unwrap()
        .into_iter()
        .filter_map(|event| match event.body {
            EventBody::OfficeTurnSettled { charged_delta, .. } => Some(charged_delta),
            _ => None,
        })
        .collect();
    assert_eq!(
        settled_deltas,
        [UsdMicros::new(4).unwrap(), UsdMicros::new(5).unwrap()]
    );

    let open_three = accepted(
        &mut store,
        "m6-open-turn-three",
        grand_architect,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
    );
    let frontier_three = match open_three.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        other => panic!("unexpected third open receipt: {other:?}"),
    };
    let turn_three = OfficeTurnId::new(3).unwrap();
    let correlation_three = PiCorrelationIdentity::parse("m6-prompt-three").unwrap();
    accepted(
        &mut store,
        "m6-authorize-prompt-three",
        PrincipalId::KERNEL,
        Capability::AuthorizePiOfficeTurnPrompt,
        zero,
        CommandBody::AuthorizePiOfficeTurnPrompt {
            office_turn_id: turn_three,
            correlation_identity: correlation_three.clone(),
            prompt_content_object_id: society_kernel::ContentObjectId::new(1).unwrap(),
            prompt_digest,
            frontier_event_id: frontier_three,
        },
    );
    accepted(
        &mut store,
        "m6-deliver-prompt-three",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnPromptDelivery,
        zero,
        CommandBody::RecordPiOfficeTurnPromptDelivery {
            office_turn_id: turn_three,
            correlation_identity: correlation_three.clone(),
            prompt_digest,
        },
    );
    accepted(
        &mut store,
        "m6-accept-prompt-three",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnPromptAccepted,
        zero,
        CommandBody::RecordPiOfficeTurnPromptAccepted {
            office_turn_id: turn_three,
            correlation_identity: correlation_three.clone(),
            command_result_sequence: PiProtocolSequence::try_from(30).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m6-unavailable-usage-freezes-parent",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnUsageFailure,
        zero,
        CommandBody::RecordPiOfficeTurnUsageFailure {
            office_turn_id: turn_three,
            correlation_identity: correlation_three.clone(),
            protocol_sequence: PiProtocolSequence::try_from(32).unwrap(),
            failure: PiOfficeTurnUsageFailure::Unavailable(
                PiOfficeTurnUsageUnavailableReason::InvalidSdkUsage,
            ),
        },
    );
    let contradictory_frozen_usage = PiCumulativeUsage {
        provider_cost: ProviderCostBinary64::from_big_endian(0.0000095_f64.to_bits().to_be_bytes())
            .unwrap(),
        ceiling_micro_usd: UsdMicros::new(10).unwrap(),
        ..usage_two
    };
    rejected(
        &mut store,
        "m6-reject-known-usage-sharing-failure-sequence",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnUsage,
        zero,
        CommandBody::RecordPiOfficeTurnUsage {
            office_turn_id: turn_three,
            correlation_identity: correlation_three.clone(),
            protocol_sequence: PiProtocolSequence::try_from(32).unwrap(),
            usage: contradictory_frozen_usage,
        },
        Rejection::PiOfficeTurnUsageNotMonotonic,
    );
    rejected(
        &mut store,
        "m6-reject-known-usage-after-frozen-accounting",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnUsage,
        zero,
        CommandBody::RecordPiOfficeTurnUsage {
            office_turn_id: turn_three,
            correlation_identity: correlation_three.clone(),
            protocol_sequence: PiProtocolSequence::try_from(34).unwrap(),
            usage: contradictory_frozen_usage,
        },
        Rejection::PiOfficeTurnUsageAlreadyFrozen,
    );
    accepted(
        &mut store,
        "m6-terminal-error-follows-frozen-accounting",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnTerminal,
        zero,
        CommandBody::RecordPiOfficeTurnTerminal {
            office_turn_id: turn_three,
            correlation_identity: correlation_three,
            agent_settled_sequence: PiProtocolSequence::try_from(31).unwrap(),
            final_usage_sequence: PiProtocolSequence::try_from(32).unwrap(),
            settled_sequence: PiProtocolSequence::try_from(33).unwrap(),
            disposition: PiOfficeTurnDisposition::Error,
            assistant_outcome: PiOfficeTurnAssistantOutcome::ObservedError,
            transcript_disposition:
                PiOfficeTurnTranscriptDisposition::DeferredUntilOfficeSessionDispose,
        },
    );
    let events = store.replay_ledger().unwrap();
    assert!(events.iter().any(|event| matches!(
        event.body,
        EventBody::PiOfficeTurnUsageFrozen {
            office_turn_id,
            failure: PiOfficeTurnUsageFailure::Unavailable(
                PiOfficeTurnUsageUnavailableReason::InvalidSdkUsage,
            ),
            ..
        } if office_turn_id == turn_three
    )));
    let inspection = Connection::open(&path).unwrap();
    let (coarse_unavailable_reason, frozen_remainder, charged, envelope_reserved, envelope_spent):
        (i64, i64, i64, i64, i64) = inspection
        .query_row(
            "SELECT postmortem.unavailable_reason, postmortem.reserved_micros,
                    reservation.charged_micros, envelope.reserved_micros, envelope.spent_micros
             FROM cost_postmortems postmortem
             JOIN budget_reservations reservation
               ON reservation.budget_reservation_id = postmortem.budget_reservation_id
             JOIN budget_reservation_charges charge
               ON charge.budget_reservation_id = reservation.budget_reservation_id
             JOIN budget_envelopes envelope ON envelope.budget_envelope_id = charge.budget_envelope_id
             WHERE postmortem.postmortem_id = 1
             ORDER BY envelope.budget_envelope_id LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        coarse_unavailable_reason,
        CostUnavailableReason::AdapterAccountingUnavailable as i64
    );
    // The failed third turn freezes only the parent reservation's uncharged
    // remainder. Typed session Dispose/Postmortem closure is intentionally
    // deferred until non-ready Office cancellation settlement exists; this is
    // the exact durable handoff it must consume rather than double-charge.
    assert_eq!(
        (frozen_remainder, charged, envelope_reserved, envelope_spent),
        (91, 9, 91, 9)
    );
    drop(inspection);
    let collision = Connection::open(&path).unwrap();
    assert!(
        collision
            .execute(
                "INSERT INTO pi_office_turn_usage_receipts(
                 office_turn_id, pi_office_turn_prompt_authorization_id, pi_session_id,
                 correlation_identity, protocol_sequence, input_tokens, output_tokens,
                 cache_read_tokens, cache_write_tokens, total_tokens,
                 provider_cost_binary64, cumulative_ceiling_micros, recorded_by_command_id
             ) VALUES (3, 3, 1, 'm6-prompt-three', 32, 1, 1, 1, 1, 4,
                       X'3ED26E2A23FAD2B5', 10, 1)",
                [],
            )
            .is_err()
    );
    drop(collision);
    assert!(store.validate_replayed_materialized_state().is_ok());
    let body_tamper = Connection::open(&path).unwrap();
    body_tamper
        .execute(
            "UPDATE command_record_pi_office_turn_usage
             SET provider_cost_binary64 = X'3ED132576B20E04A',
                 cumulative_ceiling_micros = 5
             WHERE command_row_id = (
                 SELECT command_row_id FROM commands WHERE command_id = 'm6-usage-one'
             )",
            [],
        )
        .unwrap();
    drop(body_tamper);
    let tampered_replay = store.replay_ledger();
    assert!(
        matches!(tampered_replay, Err(StoreError::LedgerCorruption(_))),
        "M6 usage command-body edit escaped its request commitment: {tampered_replay:?}"
    );
    let body_restore = Connection::open(&path).unwrap();
    body_restore
        .execute(
            "UPDATE command_record_pi_office_turn_usage
             SET provider_cost_binary64 = X'3ED0C6F7A0B5ED8D',
                 cumulative_ceiling_micros = 4
             WHERE command_row_id = (
                 SELECT command_row_id FROM commands WHERE command_id = 'm6-usage-one'
             )",
            [],
        )
        .unwrap();
    drop(body_restore);
    assert!(store.replay_ledger().is_ok());
    let tamper = Connection::open(&path).unwrap();
    tamper
        .execute(
            "UPDATE office_turn_budget_checkpoints
             SET baseline_cumulative_micros = 1 WHERE office_turn_id = 1",
            [],
        )
        .unwrap();
    assert!(matches!(
        store.validate_replayed_materialized_state(),
        Err(StoreError::LedgerCorruption(_))
    ));
    drop(tamper);
    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn pi_office_turn_late_receipts_after_cancellation_never_restore_office_authority() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();
    accepted(
        &mut store,
        "m6-race-start-office-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        &mut store,
        grand_architect,
        cycle_id,
        session_id,
        "m6-race-office",
        UsdMicros::new(100).unwrap(),
    );
    let digest = Sha256Digest::of_bytes(b"cancellation-race-prompt");
    accepted(
        &mut store,
        "m6-race-seal-prompt",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt { digest },
    );
    accepted(
        &mut store,
        "m6-race-register-prompt",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(1).unwrap(),
        },
    );
    let opened = accepted(
        &mut store,
        "m6-race-open-turn",
        grand_architect,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
    );
    let frontier = match opened.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        other => panic!("unexpected race turn opening: {other:?}"),
    };
    let turn_id = OfficeTurnId::new(1).unwrap();
    let correlation = PiCorrelationIdentity::parse("m6-race-prompt").unwrap();
    accepted(
        &mut store,
        "m6-race-authorize-prompt",
        PrincipalId::KERNEL,
        Capability::AuthorizePiOfficeTurnPrompt,
        zero,
        CommandBody::AuthorizePiOfficeTurnPrompt {
            office_turn_id: turn_id,
            correlation_identity: correlation.clone(),
            prompt_content_object_id: society_kernel::ContentObjectId::new(1).unwrap(),
            prompt_digest: digest,
            frontier_event_id: frontier,
        },
    );
    accepted(
        &mut store,
        "m6-race-cancel-between-authorize-and-delivery",
        grand_architect,
        Capability::RequestCancellation,
        zero,
        CommandBody::RequestCancellation {
            cycle_id,
            mode: society_kernel::CancellationMode::GracefulCancel,
        },
    );
    // These buffered physical/peer facts are still durable evidence, but the
    // cancellation fence means they cannot reopen Office authority.
    accepted(
        &mut store,
        "m6-race-late-delivery",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnPromptDelivery,
        zero,
        CommandBody::RecordPiOfficeTurnPromptDelivery {
            office_turn_id: turn_id,
            correlation_identity: correlation.clone(),
            prompt_digest: digest,
        },
    );
    accepted(
        &mut store,
        "m6-race-late-acceptance",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnPromptAccepted,
        zero,
        CommandBody::RecordPiOfficeTurnPromptAccepted {
            office_turn_id: turn_id,
            correlation_identity: correlation.clone(),
            command_result_sequence: PiProtocolSequence::try_from(1).unwrap(),
        },
    );
    let usage = PiCumulativeUsage {
        input_tokens: PiTokenCount::try_from(1).unwrap(),
        output_tokens: PiTokenCount::try_from(1).unwrap(),
        cache_read_tokens: PiTokenCount::try_from(1).unwrap(),
        cache_write_tokens: PiTokenCount::try_from(1).unwrap(),
        total_tokens: PiTokenCount::try_from(4).unwrap(),
        provider_cost: ProviderCostBinary64::from_big_endian(0.000004_f64.to_bits().to_be_bytes())
            .unwrap(),
        ceiling_micro_usd: UsdMicros::new(4).unwrap(),
    };
    accepted(
        &mut store,
        "m6-race-late-usage",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnUsage,
        zero,
        CommandBody::RecordPiOfficeTurnUsage {
            office_turn_id: turn_id,
            correlation_identity: correlation.clone(),
            protocol_sequence: PiProtocolSequence::try_from(3).unwrap(),
            usage,
        },
    );
    accepted(
        &mut store,
        "m6-race-late-known-cost-error-terminal",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnTerminal,
        zero,
        CommandBody::RecordPiOfficeTurnTerminal {
            office_turn_id: turn_id,
            correlation_identity: correlation,
            agent_settled_sequence: PiProtocolSequence::try_from(2).unwrap(),
            final_usage_sequence: PiProtocolSequence::try_from(3).unwrap(),
            settled_sequence: PiProtocolSequence::try_from(4).unwrap(),
            disposition: PiOfficeTurnDisposition::Error,
            assistant_outcome: PiOfficeTurnAssistantOutcome::ObservedError,
            transcript_disposition:
                PiOfficeTurnTranscriptDisposition::DeferredUntilOfficeSessionDispose,
        },
    );
    rejected(
        &mut store,
        "m6-race-settlement-cannot-reopen-after-cancellation",
        PrincipalId::KERNEL,
        Capability::SettleOfficeTurn,
        ExpectedGeneration::NotApplicable,
        CommandBody::SettleOfficeTurn {
            turn_id,
            terminal_receipt_id: PiOfficeTurnTerminalReceiptId::new(1).unwrap(),
        },
        Rejection::PiOfficeTurnNotReconciled,
    );
    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::PiOfficeTurnTerminalRecorded {
            office_turn_id,
            disposition: PiOfficeTurnDisposition::Error,
            assistant_outcome: PiOfficeTurnAssistantOutcome::ObservedError,
            ..
        } if office_turn_id == turn_id
    )));
    assert!(store.validate_replayed_materialized_state().is_ok());
}

#[test]
fn duplicate_cost_incident_cannot_open_another_postmortem_or_cancellation() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("society-duplicate-cost-incident-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);

    accepted(
        &mut store,
        "ga-start-duplicate-cost-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "ga-reserve-duplicate-cost",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(10_000).unwrap(),
        },
    );
    let first = accepted(
        &mut store,
        "kernel-freeze-duplicate-cost",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Unknown(CostUnknownReason::ReconciliationMismatch),
        },
    );
    let replay = submit(
        &mut store,
        "kernel-freeze-duplicate-cost",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Unknown(CostUnknownReason::ReconciliationMismatch),
        },
    );
    assert_eq!(replay.disposition, first.disposition);
    assert!(replay.idempotent);
    rejected(
        &mut store,
        "kernel-reject-second-cost-incident",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Unknown(CostUnknownReason::ReconciliationMismatch),
        },
        Rejection::ReservationNotActive,
    );

    let inspection = rusqlite::Connection::open(&path).unwrap();
    let postmortems: i64 = inspection
        .query_row("SELECT COUNT(*) FROM cost_postmortems", [], |row| {
            row.get(0)
        })
        .unwrap();
    let cancellations: i64 = inspection
        .query_row("SELECT COUNT(*) FROM cancellation_requests", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!((postmortems, cancellations), (1, 1));
    drop(inspection);
    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn fresh_replay_detects_operating_cycle_budget_and_session_row_tampering() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-materialized-replay-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    accepted(
        &mut store,
        "ga-start-material-replay-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    assert!(store.validate_replayed_materialized_state().is_ok());

    let tamper = rusqlite::Connection::open(&path).unwrap();
    tamper
        .execute(
            "UPDATE operating_cycles SET lifecycle_state = ?1 WHERE operating_cycle_id = 1",
            [OperatingCycleState::Failed as i64],
        )
        .unwrap();
    assert!(matches!(
        store.validate_replayed_materialized_state(),
        Err(society_kernel::StoreError::LedgerCorruption(_))
    ));
    tamper
        .execute(
            "UPDATE operating_cycles SET lifecycle_state = ?1 WHERE operating_cycle_id = 1",
            [OperatingCycleState::Running as i64],
        )
        .unwrap();

    tamper
        .execute(
            "UPDATE budget_envelopes SET spent_micros = 1 WHERE budget_envelope_id = 1",
            [],
        )
        .unwrap();
    assert!(matches!(
        store.validate_replayed_materialized_state(),
        Err(society_kernel::StoreError::LedgerCorruption(_))
    ));
    tamper
        .execute(
            "UPDATE budget_envelopes SET spent_micros = 0 WHERE budget_envelope_id = 1",
            [],
        )
        .unwrap();

    tamper
        .execute(
            "UPDATE grand_architect_office_sessions SET lifecycle_state = ?1
             WHERE grand_architect_office_session_id = 1",
            [11_i64],
        )
        .unwrap();
    assert!(matches!(
        store.validate_replayed_materialized_state(),
        Err(society_kernel::StoreError::LedgerCorruption(_))
    ));
    tamper
        .execute(
            "UPDATE grand_architect_office_sessions SET lifecycle_state = 1
             WHERE grand_architect_office_session_id = 1",
            [],
        )
        .unwrap();
    assert!(store.validate_replayed_materialized_state().is_ok());

    drop(tamper);
    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn on_disk_reopen_preserves_treatment_and_detects_materialized_tampering() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("society-reopen-material-replay-{unique}.sqlite3"));
    {
        let mut store = KernelStore::open(&path).unwrap();
        let (_, cycle_id) = found_cycle(&mut store);
        assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
            event.body,
            EventBody::OperatingCycleProposed {
                cycle_id: event_cycle_id,
                treatment: OperatingCycleTreatment::DeterministicPiHostFixtureV1,
                ..
            } if event_cycle_id == cycle_id
        )));
        assert!(store.validate_replayed_materialized_state().is_ok());
    }

    {
        let reopened = KernelStore::open(&path).unwrap();
        assert!(reopened.replay_ledger().is_ok());
        assert!(reopened.validate_replayed_materialized_state().is_ok());
    }

    let tamper = rusqlite::Connection::open(&path).unwrap();
    let (treatment, ceiling): (i64, i64) = tamper
        .query_row(
            "SELECT c.treatment, e.ceiling_micros
             FROM operating_cycles c
             JOIN budget_envelope_constraints b
               ON b.operating_cycle_id = c.operating_cycle_id
             JOIN budget_envelopes e ON e.budget_envelope_id = b.budget_envelope_id
             WHERE c.operating_cycle_id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        (treatment, ceiling),
        (
            OperatingCycleTreatment::DeterministicPiHostFixtureV1 as i64,
            UsdMicros::PINNED_PI_SDK_CYCLE_CEILING.value(),
        )
    );
    tamper
        .execute(
            "UPDATE budget_envelopes SET spent_micros = 1 WHERE budget_envelope_id = 1",
            [],
        )
        .unwrap();
    drop(tamper);

    let reopened_after_tamper = KernelStore::open(&path).unwrap();
    assert!(reopened_after_tamper.replay_ledger().is_ok());
    assert!(matches!(
        reopened_after_tamper.validate_replayed_materialized_state(),
        Err(society_kernel::StoreError::LedgerCorruption(_))
    ));
    drop(reopened_after_tamper);
    fs::remove_file(path).unwrap();
}

#[test]
fn rejected_missing_subject_and_tampered_extra_body_are_detected() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-kernel-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let invalid_grant_id = CommandId::parse("ga-invalid-capability-grant").unwrap();
    let invalid_grant = store
        .execute(CommandRequest {
            command_id: invalid_grant_id.clone(),
            principal_id: grand_architect,
            capability_grant_id: society_kernel::CapabilityGrantId::new(999).unwrap(),
            capability: Capability::QuiesceOperatingCycle,
            expected_generation: ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
            body: CommandBody::QuiesceOperatingCycle { cycle_id },
        })
        .unwrap();
    assert_eq!(
        invalid_grant.disposition,
        CommandDisposition::Rejected(Rejection::CapabilityNotGranted)
    );
    assert_eq!(
        store
            .command_receipt(&invalid_grant_id)
            .unwrap()
            .unwrap()
            .disposition,
        invalid_grant.disposition
    );
    let absent = rejected(
        &mut store,
        "ga-quiesce-absent-cycle",
        grand_architect,
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle {
            cycle_id: OperatingCycleId::new(999).unwrap(),
        },
        Rejection::SubjectNotFound,
    );
    assert_eq!(
        store
            .command_receipt(&CommandId::parse("ga-quiesce-absent-cycle").unwrap())
            .unwrap()
            .unwrap()
            .disposition,
        absent.disposition
    );
    assert!(store.replay_ledger().is_ok());
    assert!(CommandId::parse("contains a space").is_err());
    assert!(CommandId::parse("contains\na-newline").is_err());

    let tamper = rusqlite::Connection::open(&path).unwrap();
    let stored_grant: i64 = tamper
        .query_row(
            "SELECT capability_grant_id FROM commands WHERE command_id = ?1",
            [invalid_grant_id.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored_grant, 999);
    let opaque_or_json_tables: i64 = tamper
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema
             WHERE type = 'table' AND (upper(sql) LIKE '%JSON%' OR lower(sql) LIKE '%payload%' OR lower(sql) LIKE '%metadata%')",
            [],
            |row| row.get(0),
    )
    .unwrap();
    assert_eq!(opaque_or_json_tables, 0);
    tamper
        .execute(
            "UPDATE commands SET command_id = ?1 WHERE command_row_id = 1",
            ["mutated-but-unique-command-id"],
        )
        .unwrap();
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    tamper
        .execute(
            "UPDATE commands SET command_id = ?1 WHERE command_row_id = 1",
            ["found-create-society"],
        )
        .unwrap();
    assert!(store.replay_ledger().is_ok());
    tamper
        .execute(
            "UPDATE command_create_society_identity SET name = ?1 WHERE command_row_id = 1",
            ["altered durable command body"],
        )
        .unwrap();
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    tamper
        .execute(
            "UPDATE command_create_society_identity SET name = ?1 WHERE command_row_id = 1",
            ["Founding Society"],
        )
        .unwrap();
    assert!(store.replay_ledger().is_ok());
    tamper
        .execute(
            "UPDATE event_r0_hard_ceiling_set SET ceiling_micros = ?1 WHERE event_id = 5",
            [1_i64],
        )
        .unwrap();
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    tamper
        .execute(
            "UPDATE event_r0_hard_ceiling_set SET ceiling_micros = ?1 WHERE event_id = 5",
            [UsdMicros::FOUNDING_SOCIETY_HARD_CEILING.value()],
        )
        .unwrap();
    assert!(store.replay_ledger().is_ok());
    tamper
        .execute(
            "INSERT INTO command_bootstrap_society(command_row_id) VALUES (?1)",
            [1_i64],
        )
        .unwrap();
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    tamper
        .execute(
            "DELETE FROM command_bootstrap_society WHERE command_row_id = ?1",
            [1_i64],
        )
        .unwrap();
    assert!(store.replay_ledger().is_ok());
    // Swap two accepted event links while preserving one event per accepted
    // command and their reciprocal receipt references. The event commitments
    // still bind each body to the original command identity, so this remains
    // detectable without relying on a missing-link side effect.
    tamper
        .execute(
            "UPDATE events SET command_row_id = 9 WHERE event_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE events SET command_row_id = 1 WHERE event_id = 2",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE events SET command_row_id = 2 WHERE event_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE commands SET accepted_event_id = 2 WHERE command_row_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE commands SET accepted_event_id = 1 WHERE command_row_id = 2",
            [],
        )
        .unwrap();
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    tamper
        .execute(
            "UPDATE events SET command_row_id = 9 WHERE event_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE events SET command_row_id = 2 WHERE event_id = 2",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE events SET command_row_id = 1 WHERE event_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE commands SET accepted_event_id = 1 WHERE command_row_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE commands SET accepted_event_id = 2 WHERE command_row_id = 2",
            [],
        )
        .unwrap();
    assert!(store.replay_ledger().is_ok());
    tamper
        .execute(
            "INSERT INTO event_society_bootstrapped(event_id, society_id) VALUES (?1, ?2)",
            rusqlite::params![1_i64, 1_i64],
        )
        .unwrap();
    drop(tamper);
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    drop(store);
    fs::remove_file(path).unwrap();
}
