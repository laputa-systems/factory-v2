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
    AdmissionGeneration, AdversarialReviewId, ApplicationIdentity, ApplicationMissionInput,
    ApplicationName, ApplicationRevisionId, ApplicationRevisionOrdinal, Blake3Digest,
    BudgetReservationId, CancellationMode, CancellationPropagationId, CancellationRequestId,
    CanonicalWorkspacePath, Capability, ChildRecoveryObservation, ChildStreamKind,
    ChildStreamSealCompleteness, CommandBody, CommandDisposition, CommandId, CommandReceipt,
    CommandRequest, ContentObjectId, ContentSealReceiptId, ContextPackPurpose,
    DeterministicEvaluationReceiptId, DeterministicEvaluatorScheduleClaim,
    DeterministicEvaluatorScheduleClaimRequest, DeterministicExperimentId, DevelopmentalAttractor,
    DirectChildWaitStatus, EvaluatorRevisionId, EventBody, EventId, EvidenceApplicability,
    EvidenceLimitationText, EvidenceSemanticRole, ExecutionProfileId, ExpectedGeneration,
    ForensicManifestCapturePolicy, ForensicManifestId, GraphRevisionBody, GraphRevisionId,
    HypothesisRevisionText, InputManifestId, KernelStore, MissionPrinciple, MissionPrincipleKind,
    MissionPrincipleText, MissionPrinciples, MissionStatement, NativeChildId, NativeChildPid,
    NativeChildSpawnAdmissionId, NativeWorkspaceId, NorthStarBoundaryCommitmentQuestion,
    NorthStarChangeQuestion, NorthStarImprovementEvidenceQuestion, NorthStarQuestionSet,
    NorthStarRevisitQuestion, OfficeTurnPurpose, OperatingCycleId, OperatingCycleTreatment,
    OutcomeObligationDisposition, OutcomeObligationId, OutcomeObligationText, OwnedProcessGroupId,
    PiAbortControlWriteOutcome, PiBoundarySessionIdentity, PiChildOwner, PiCorrelationIdentity,
    PrincipalDisplayName, PrincipalId, ProcessExitCode, ProcessGroupLiveness, ProcessSignalAction,
    ProcessSignalCause, ProcessSignalDelivery, ProjectId, ProjectMilestoneId, ProjectMilestoneName,
    ProjectName, ProjectNorthStarAlignment, ProjectNorthStarBoundaryCommitmentAnswer,
    ProjectNorthStarChangeAnswer, ProjectNorthStarImprovementEvidenceAnswer,
    ProjectNorthStarRevisitAnswer, ProjectObjectiveText, ProjectState, ProjectStopConditionText,
    Rejection, RetentionAccessClass, ReviewChallengeId, ReviewChallengeSeverity,
    ReviewDispositionKind, ReviewFailureHypothesis, ReviewResolutionKind, ReviewResponseText,
    RootAuthorityOfficeSessionId, SocietyName, SpawnNonce, StoreError, SupervisedChildIdentity,
    SupervisorEpochId, SupervisorEpochIdentity, TicketAcceptanceConditionText, TicketId,
    TicketTitle, UsdMicros, WorkAssignmentText, WorkItemId, WorkItemKind, WorkLeaseId,
};

fn example_application_mission() -> ApplicationMissionInput {
    ApplicationMissionInput {
        application_identity: ApplicationIdentity::parse("example-application").unwrap(),
        application_name: ApplicationName::parse("Example Application").unwrap(),
        revision_ordinal: ApplicationRevisionOrdinal::new(1).unwrap(),
        statement: MissionStatement::parse("Improve a bounded example system responsibly.")
            .unwrap(),
        principles: MissionPrinciples::new(vec![MissionPrinciple {
            kind: MissionPrincipleKind::Purpose,
            text: MissionPrincipleText::parse("Keep work legible and bounded.").unwrap(),
        }])
        .unwrap(),
        north_star_questions: NorthStarQuestionSet {
            change: NorthStarChangeQuestion::parse("What change should be made?").unwrap(),
            improvement_evidence: NorthStarImprovementEvidenceQuestion::parse(
                "What evidence demonstrates improvement?",
            )
            .unwrap(),
            boundary_commitment: NorthStarBoundaryCommitmentQuestion::parse(
                "What boundary must remain intact?",
            )
            .unwrap(),
            revisit: NorthStarRevisitQuestion::parse("When should it be revisited?").unwrap(),
        },
        source_rendering_digest: Blake3Digest::of_bytes(b"execution-foundation-mission"),
    }
}

fn example_project_north_star_alignment() -> ProjectNorthStarAlignment {
    ProjectNorthStarAlignment {
        application_revision_id: ApplicationRevisionId::new(1).unwrap(),
        change_answer: ProjectNorthStarChangeAnswer::parse("Deliver one bounded change.").unwrap(),
        improvement_evidence_answer: ProjectNorthStarImprovementEvidenceAnswer::parse(
            "A deterministic judge must pass.",
        )
        .unwrap(),
        boundary_commitment_answer: ProjectNorthStarBoundaryCommitmentAnswer::parse(
            "No authority is widened.",
        )
        .unwrap(),
        revisit_answer: ProjectNorthStarRevisitAnswer::parse("Review after evidence arrives.")
            .unwrap(),
    }
}

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

fn seal_and_register_mission_source(store: &mut KernelStore, mission: &ApplicationMissionInput) {
    let kernel = PrincipalId::KERNEL;
    accepted(
        store,
        "m3-seal-mission-source",
        kernel,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: mission.source_rendering_digest,
        },
    );
    accepted(
        store,
        "m3-register-mission-source",
        kernel,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: ContentSealReceiptId::new(1).unwrap(),
        },
    );
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
    let mission = example_application_mission();
    seal_and_register_mission_source(store, &mission);
    accepted(
        store,
        "m3-found-founding-mission",
        bootstrap,
        Capability::InstallFoundingMission,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallFoundingMission { mission },
    );
    accepted(
        store,
        "m3-found-office",
        bootstrap,
        Capability::InstallRootAuthorityOffice,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallRootAuthorityOffice,
    );
    accepted(
        store,
        "m3-found-root_authority",
        bootstrap,
        Capability::AppointInitialRootAuthority,
        ExpectedGeneration::NotApplicable,
        CommandBody::AppointInitialRootAuthority {
            actor_display_name: PrincipalDisplayName::parse("Root Authority").unwrap(),
        },
    );
    accepted(
        store,
        "m3-found-ceiling",
        bootstrap,
        Capability::SetR0HardCeiling,
        ExpectedGeneration::NotApplicable,
        CommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::new(1_030_000).unwrap(),
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
        CommandBody::ProposeOperatingCycle {
            treatment,
            budget_ceiling: UsdMicros::new(1_000_000).unwrap(),
        },
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
    root_authority: PrincipalId,
    cycle: OperatingCycleId,
) -> ProjectId {
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    accepted(
        store,
        "m3-project-office-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        generation,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id: cycle },
    );
    accepted(
        store,
        "m3-project-create",
        root_authority,
        Capability::CreateProject,
        generation,
        CommandBody::CreateProject {
            operating_cycle_id: cycle,
            project_name: ProjectName::parse("Independent execution proof").unwrap(),
            north_star_alignment: example_project_north_star_alignment(),
        },
    );
    let project_id = ProjectId::new(1).unwrap();
    rejected(
        store,
        "m3-project-charter-too-early",
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
fn deterministic_evaluator_native_child_is_not_a_pi_child() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-evaluator-native-{nonce}.sqlite"));
    let mut store = KernelStore::open(&path).unwrap();
    let (root_authority, cycle) = founded_cycle(
        &mut store,
        OperatingCycleTreatment::DeterministicEvaluatorFixtureV1,
    );
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let project = active_project(&mut store, root_authority, cycle);
    accepted(
        &mut store,
        "evaluator-ticket",
        root_authority,
        Capability::CreateTicket,
        generation,
        CommandBody::CreateTicket {
            operating_cycle_id: cycle,
            project_id: project,
            ticket_title: TicketTitle::parse("Evaluate one committed hypothesis").unwrap(),
            acceptance_condition: TicketAcceptanceConditionText::parse(
                "Evaluator bytes are sealed.",
            )
            .unwrap(),
            prerequisite_ticket_id: None,
        },
    );
    accepted(
        &mut store,
        "evaluator-graph",
        root_authority,
        Capability::AddGraphObjectRevision,
        generation,
        CommandBody::AddGraphObjectRevision {
            operating_cycle_id: cycle,
            project_id: project,
            causal_episode_id: None,
            graph_object_id: None,
            body: GraphRevisionBody::Hypothesis {
                hypothesis: HypothesisRevisionText::parse("Evaluator result is bounded.").unwrap(),
            },
        },
    );
    let graph_revision = GraphRevisionId::new(1).unwrap();
    accepted(
        &mut store,
        "evaluator-graph-commit",
        root_authority,
        Capability::CommitGraphRevision,
        generation,
        CommandBody::CommitGraphRevision {
            operating_cycle_id: cycle,
            graph_revision_id: graph_revision,
        },
    );
    for (command, digest, receipt) in [
        (
            "evaluator-seal-program",
            Blake3Digest::of_bytes(b"evaluator-program"),
            2_i64,
        ),
        (
            "evaluator-seal-input",
            Blake3Digest::of_bytes(b"evaluator-input"),
            3_i64,
        ),
    ] {
        accepted(
            &mut store,
            command,
            PrincipalId::KERNEL,
            Capability::RecordContentSealReceipt,
            ExpectedGeneration::NotApplicable,
            CommandBody::RecordContentSealReceipt { digest },
        );
        accepted(
            &mut store,
            &format!("{command}-object"),
            PrincipalId::KERNEL,
            Capability::RegisterContentObject,
            ExpectedGeneration::NotApplicable,
            CommandBody::RegisterContentObject {
                content_seal_receipt_id: ContentSealReceiptId::new(receipt).unwrap(),
            },
        );
    }
    accepted(
        &mut store,
        "evaluator-experiment",
        root_authority,
        Capability::RegisterDeterministicExperiment,
        generation,
        CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id: cycle,
            project_id: project,
            ticket_id: TicketId::new(1).unwrap(),
            target_graph_revision_id: graph_revision,
            evaluator_content_object_id: ContentObjectId::new(2).unwrap(),
            input_manifest_content_object_id: ContentObjectId::new(3).unwrap(),
        },
    );
    // This intentionally precedes scheduling. It remains a generic forensic
    // occurrence, but it must never be usable as the output receipt once this
    // experiment has an exact evaluator-child claim.
    accepted(
        &mut store,
        "evaluator-pre-schedule-generic-manifest",
        PrincipalId::KERNEL,
        Capability::RegisterForensicManifest,
        generation,
        CommandBody::RegisterForensicManifest {
            operating_cycle_id: cycle,
            producing_deterministic_experiment_id: DeterministicExperimentId::new(1).unwrap(),
            capture_policy: ForensicManifestCapturePolicy::DeterministicExperimentEvaluatorV1,
            retention_access_class: RetentionAccessClass::ForensicRestricted,
            evaluator_output_content_object_id: ContentObjectId::new(2).unwrap(),
        },
    );
    let epoch_identity = SupervisorEpochIdentity::parse("evaluator-native-epoch").unwrap();
    accepted(
        &mut store,
        "evaluator-epoch",
        PrincipalId::KERNEL,
        Capability::OpenSupervisorEpoch,
        ExpectedGeneration::NotApplicable,
        CommandBody::OpenSupervisorEpoch {
            supervisor_epoch_id: SupervisorEpochId::new(1).unwrap(),
            supervisor_epoch_identity: epoch_identity.clone(),
        },
    );
    let schedule_request = DeterministicEvaluatorScheduleClaimRequest::new(
        CommandId::parse("evaluator-schedule-claim").unwrap(),
        NativeWorkspaceId::parse("evaluator-native-workspace").unwrap(),
        CanonicalWorkspacePath::parse("/tmp/evaluator-native-workspace").unwrap(),
        SupervisorEpochId::new(1).unwrap(),
        epoch_identity.clone(),
    );
    let claimed = store
        .claim_registered_deterministic_evaluator(schedule_request.clone())
        .unwrap()
        .unwrap();
    let DeterministicEvaluatorScheduleClaim::SpawnAuthorized(admission) = &claimed else {
        panic!("fresh scheduler claim must remain spawn-authoritative");
    };
    assert_eq!(admission.operating_cycle_id(), cycle);
    assert_eq!(
        claimed.evaluator_content_object_id(),
        Some(ContentObjectId::new(2).unwrap())
    );
    assert_eq!(
        claimed.input_manifest_content_object_id(),
        Some(ContentObjectId::new(3).unwrap())
    );
    let spawned_retry_request = schedule_request.clone();
    let repeated_claim = store
        .claim_registered_deterministic_evaluator(schedule_request)
        .unwrap()
        .unwrap();
    assert_eq!(repeated_claim, claimed);
    let admission_id = NativeChildSpawnAdmissionId::new(1).unwrap();
    let admission = store
        .deterministic_evaluator_native_child_admission(admission_id)
        .unwrap()
        .unwrap();
    assert_eq!(admission.operating_cycle_id(), cycle);
    assert_eq!(
        admission.evaluator_digest(),
        Blake3Digest::of_bytes(b"evaluator-program")
    );
    assert_eq!(
        admission.input_manifest_digest(),
        Blake3Digest::of_bytes(b"evaluator-input")
    );
    assert!(
        store
            .deterministic_evaluator_native_child_admission(
                NativeChildSpawnAdmissionId::new(99).unwrap()
            )
            .unwrap()
            .is_none()
    );
    rejected(
        &mut store,
        "pi-cannot-spawn-evaluator",
        PrincipalId::KERNEL,
        Capability::RecordInertChildSpawn,
        generation,
        CommandBody::RecordInertChildSpawn {
            native_child_spawn_admission_id: admission_id,
            child_identity: SupervisedChildIdentity::parse("evaluator-native-child").unwrap(),
            direct_child_pid: NativeChildPid::try_from(9031).unwrap(),
            process_group_id: OwnedProcessGroupId::try_from(9031).unwrap(),
        },
        Rejection::ChildSpawnAdmissionInvalid,
    );
    accepted(
        &mut store,
        "evaluator-spawn",
        PrincipalId::KERNEL,
        Capability::RecordDeterministicEvaluatorNativeChildSpawn,
        generation,
        CommandBody::RecordDeterministicEvaluatorNativeChildSpawn {
            native_child_spawn_admission_id: admission_id,
            child_identity: SupervisedChildIdentity::parse("evaluator-native-child").unwrap(),
            direct_child_pid: NativeChildPid::try_from(9031).unwrap(),
            process_group_id: OwnedProcessGroupId::try_from(9031).unwrap(),
        },
    );
    assert!(matches!(
        store
            .claim_registered_deterministic_evaluator(spawned_retry_request)
            .unwrap(),
        Some(DeterministicEvaluatorScheduleClaim::AlreadyClaimed {
            native_child_spawn_admission_id,
        }) if native_child_spawn_admission_id == admission_id
    ));
    let child = NativeChildId::new(1).unwrap();
    rejected(
        &mut store,
        "pi-cannot-ready-evaluator",
        PrincipalId::KERNEL,
        Capability::RecordPiAdapterReady,
        generation,
        CommandBody::RecordPiAdapterReady {
            native_child_id: child,
            pi_session_identity: PiBoundarySessionIdentity::parse("forged-pi-session").unwrap(),
            spawn_nonce: SpawnNonce::parse("forged-pi-nonce").unwrap(),
        },
        Rejection::SubjectNotFound,
    );
    rejected(
        &mut store,
        "evaluator-stdout-before-reap",
        PrincipalId::KERNEL,
        Capability::RecordChildStreamSeal,
        generation,
        CommandBody::RecordChildStreamSeal {
            native_child_id: child,
            stream_kind: ChildStreamKind::Stdout,
            full_observed_digest: Blake3Digest::of_bytes(b"evaluator-program"),
            retained_content_object_id: ContentObjectId::new(2).unwrap(),
            completeness: ChildStreamSealCompleteness::Complete,
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "evaluator-reap",
        PrincipalId::KERNEL,
        Capability::RecordDirectChildReap,
        generation,
        CommandBody::RecordDirectChildReap {
            native_child_id: child,
            wait_status: DirectChildWaitStatus::Exited {
                exit_code: ProcessExitCode::try_from(0).unwrap(),
            },
            group_liveness_before_cleanup: ProcessGroupLiveness::Absent,
            group_liveness_after_cleanup: ProcessGroupLiveness::Absent,
        },
    );
    rejected(
        &mut store,
        "evaluator-control-stream-rejected",
        PrincipalId::KERNEL,
        Capability::RecordChildStreamSeal,
        generation,
        CommandBody::RecordChildStreamSeal {
            native_child_id: child,
            stream_kind: ChildStreamKind::AdmittedControl,
            full_observed_digest: Blake3Digest::of_bytes(b"evaluator-program"),
            retained_content_object_id: ContentObjectId::new(2).unwrap(),
            completeness: ChildStreamSealCompleteness::Complete,
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "evaluator-stdout",
        PrincipalId::KERNEL,
        Capability::RecordChildStreamSeal,
        generation,
        CommandBody::RecordChildStreamSeal {
            native_child_id: child,
            stream_kind: ChildStreamKind::Stdout,
            full_observed_digest: Blake3Digest::of_bytes(b"evaluator-program"),
            retained_content_object_id: ContentObjectId::new(2).unwrap(),
            completeness: ChildStreamSealCompleteness::Complete,
        },
    );
    rejected(
        &mut store,
        "evaluator-one-stream-cannot-finalize",
        PrincipalId::KERNEL,
        Capability::FinalizeChildProcess,
        generation,
        CommandBody::FinalizeChildProcess {
            native_child_id: child,
        },
        Rejection::ChildLifecycleReceiptMissing,
    );
    rejected(
        &mut store,
        "evaluator-manifest-requires-finalized-two-stream-child",
        PrincipalId::KERNEL,
        Capability::RegisterDeterministicEvaluatorForensicManifest,
        generation,
        CommandBody::RegisterDeterministicEvaluatorForensicManifest {
            operating_cycle_id: cycle,
            native_child_spawn_admission_id: admission_id,
        },
        Rejection::ForensicManifestBindingMismatch,
    );
    accepted(
        &mut store,
        "evaluator-stderr",
        PrincipalId::KERNEL,
        Capability::RecordChildStreamSeal,
        generation,
        CommandBody::RecordChildStreamSeal {
            native_child_id: child,
            stream_kind: ChildStreamKind::Stderr,
            full_observed_digest: Blake3Digest::of_bytes(b"evaluator-input"),
            retained_content_object_id: ContentObjectId::new(3).unwrap(),
            completeness: ChildStreamSealCompleteness::Complete,
        },
    );
    rejected(
        &mut store,
        "evaluator-manifest-requires-finalized-child-after-both-streams",
        PrincipalId::KERNEL,
        Capability::RegisterDeterministicEvaluatorForensicManifest,
        generation,
        CommandBody::RegisterDeterministicEvaluatorForensicManifest {
            operating_cycle_id: cycle,
            native_child_spawn_admission_id: admission_id,
        },
        Rejection::ForensicManifestBindingMismatch,
    );
    accepted(
        &mut store,
        "evaluator-two-stream-finalize",
        PrincipalId::KERNEL,
        Capability::FinalizeChildProcess,
        generation,
        CommandBody::FinalizeChildProcess {
            native_child_id: child,
        },
    );
    accepted(
        &mut store,
        "evaluator-seal-recombined-output",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: Blake3Digest::of_bytes(b"recombined-evaluator-output"),
        },
    );
    accepted(
        &mut store,
        "evaluator-register-recombined-output",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: ContentSealReceiptId::new(4).unwrap(),
        },
    );
    rejected(
        &mut store,
        "evaluator-generic-manifest-cannot-recombine-scheduled-output",
        PrincipalId::KERNEL,
        Capability::RegisterForensicManifest,
        generation,
        CommandBody::RegisterForensicManifest {
            operating_cycle_id: cycle,
            producing_deterministic_experiment_id: DeterministicExperimentId::new(1).unwrap(),
            capture_policy: ForensicManifestCapturePolicy::DeterministicExperimentEvaluatorV1,
            retention_access_class: RetentionAccessClass::ForensicRestricted,
            evaluator_output_content_object_id: ContentObjectId::new(4).unwrap(),
        },
        Rejection::ForensicManifestBindingMismatch,
    );
    let manifest_receipt = accepted(
        &mut store,
        "evaluator-derived-forensic-manifest",
        PrincipalId::KERNEL,
        Capability::RegisterDeterministicEvaluatorForensicManifest,
        generation,
        CommandBody::RegisterDeterministicEvaluatorForensicManifest {
            operating_cycle_id: cycle,
            native_child_spawn_admission_id: admission_id,
        },
    );
    let manifest_event_id = match manifest_receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        other => panic!("accepted helper returned {other:?}"),
    };
    assert!(matches!(
        store.ledger_event(manifest_event_id).unwrap().body,
        EventBody::DeterministicEvaluatorForensicManifestRegistered {
            forensic_manifest_id,
            deterministic_experiment_id,
            native_child_spawn_admission_id,
            native_child_stream_seal_id,
            evaluator_output_content_object_id,
        } if forensic_manifest_id == ForensicManifestId::new(2).unwrap()
            && deterministic_experiment_id == DeterministicExperimentId::new(1).unwrap()
            && native_child_spawn_admission_id == admission_id
            && native_child_stream_seal_id == society_kernel::NativeChildStreamSealId::new(1).unwrap()
            && evaluator_output_content_object_id == ContentObjectId::new(2).unwrap()
    ));
    rejected(
        &mut store,
        "evaluator-receipt-rejects-pre-schedule-unbound-manifest",
        PrincipalId::KERNEL,
        Capability::RecordDeterministicEvaluationReceipt,
        generation,
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id: cycle,
            deterministic_experiment_id: DeterministicExperimentId::new(1).unwrap(),
            evaluator_revision_id: EvaluatorRevisionId::new(1).unwrap(),
            input_manifest_id: InputManifestId::new(1).unwrap(),
            forensic_manifest_id: ForensicManifestId::new(1).unwrap(),
            evaluator_output_content_object_id: ContentObjectId::new(2).unwrap(),
        },
        Rejection::DeterministicEvaluationBindingMismatch,
    );
    rejected(
        &mut store,
        "evaluator-receipt-rejects-recombined-output",
        PrincipalId::KERNEL,
        Capability::RecordDeterministicEvaluationReceipt,
        generation,
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id: cycle,
            deterministic_experiment_id: DeterministicExperimentId::new(1).unwrap(),
            evaluator_revision_id: EvaluatorRevisionId::new(1).unwrap(),
            input_manifest_id: InputManifestId::new(1).unwrap(),
            forensic_manifest_id: ForensicManifestId::new(2).unwrap(),
            evaluator_output_content_object_id: ContentObjectId::new(4).unwrap(),
        },
        Rejection::DeterministicEvaluationBindingMismatch,
    );
    accepted(
        &mut store,
        "evaluator-receipt-records-derived-output",
        PrincipalId::KERNEL,
        Capability::RecordDeterministicEvaluationReceipt,
        generation,
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id: cycle,
            deterministic_experiment_id: DeterministicExperimentId::new(1).unwrap(),
            evaluator_revision_id: EvaluatorRevisionId::new(1).unwrap(),
            input_manifest_id: InputManifestId::new(1).unwrap(),
            forensic_manifest_id: ForensicManifestId::new(2).unwrap(),
            evaluator_output_content_object_id: ContentObjectId::new(2).unwrap(),
        },
    );
    assert!(
        store
            .deterministic_evaluator_native_child_admission(admission_id)
            .unwrap()
            .is_none()
    );
    let failed_receipt = accepted(
        &mut store,
        "evaluator-process-failure-finalizes-experiment",
        root_authority,
        Capability::FinalizeDeterministicExperiment,
        generation,
        CommandBody::FinalizeDeterministicExperiment {
            operating_cycle_id: cycle,
            deterministic_experiment_id: DeterministicExperimentId::new(1).unwrap(),
        },
    );
    let failed_event_id = match failed_receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        other => panic!("accepted helper returned {other:?}"),
    };
    assert!(matches!(
        store.ledger_event(failed_event_id).unwrap().body,
        EventBody::DeterministicExperimentFinalized {
            deterministic_experiment_id,
            terminal_state: society_kernel::DeterministicExperimentState::Failed,
        } if deterministic_experiment_id == DeterministicExperimentId::new(1).unwrap()
    ));

    // A second admitted evaluator is cancelled before exec. Its typed absence
    // resolves the frozen experiment target and must remain Cancelled rather
    // than being collapsed into an ordinary spawn failure.
    accepted(
        &mut store,
        "cancelled-evaluator-experiment",
        root_authority,
        Capability::RegisterDeterministicExperiment,
        generation,
        CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id: cycle,
            project_id: project,
            ticket_id: TicketId::new(1).unwrap(),
            target_graph_revision_id: graph_revision,
            evaluator_content_object_id: ContentObjectId::new(2).unwrap(),
            input_manifest_content_object_id: ContentObjectId::new(3).unwrap(),
        },
    );
    assert!(matches!(
        store
            .claim_registered_deterministic_evaluator(
                DeterministicEvaluatorScheduleClaimRequest::new(
                    CommandId::parse("cancelled-evaluator-schedule-claim").unwrap(),
                    NativeWorkspaceId::parse("cancelled-evaluator-workspace").unwrap(),
                    CanonicalWorkspacePath::parse("/tmp/cancelled-evaluator-workspace").unwrap(),
                    SupervisorEpochId::new(1).unwrap(),
                    epoch_identity.clone(),
                ),
            )
            .unwrap(),
        Some(DeterministicEvaluatorScheduleClaim::SpawnAuthorized(admission))
            if admission.native_child_spawn_admission_id() == NativeChildSpawnAdmissionId::new(2).unwrap()
                && admission.deterministic_experiment_id() == DeterministicExperimentId::new(2).unwrap()
    ));
    rejected(
        &mut store,
        "cancelled-evaluator-reason-requires-cancellation",
        PrincipalId::KERNEL,
        Capability::RecordNativeChildNotSpawned,
        generation,
        CommandBody::RecordNativeChildNotSpawned {
            native_child_spawn_admission_id: NativeChildSpawnAdmissionId::new(2).unwrap(),
            reason: society_kernel::NativeChildNotSpawnedReason::CancelledBeforeSpawn,
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "failed-during-cancellation-experiment",
        root_authority,
        Capability::RegisterDeterministicExperiment,
        generation,
        CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id: cycle,
            project_id: project,
            ticket_id: TicketId::new(1).unwrap(),
            target_graph_revision_id: graph_revision,
            evaluator_content_object_id: ContentObjectId::new(2).unwrap(),
            input_manifest_content_object_id: ContentObjectId::new(3).unwrap(),
        },
    );
    assert!(matches!(
        store
            .claim_registered_deterministic_evaluator(
                DeterministicEvaluatorScheduleClaimRequest::new(
                    CommandId::parse("failed-during-cancellation-schedule-claim").unwrap(),
                    NativeWorkspaceId::parse("failed-during-cancellation-workspace").unwrap(),
                    CanonicalWorkspacePath::parse(
                        "/tmp/failed-during-cancellation-workspace",
                    )
                    .unwrap(),
                    SupervisorEpochId::new(1).unwrap(),
                    epoch_identity,
                ),
            )
            .unwrap(),
        Some(DeterministicEvaluatorScheduleClaim::SpawnAuthorized(admission))
            if admission.native_child_spawn_admission_id() == NativeChildSpawnAdmissionId::new(3).unwrap()
                && admission.deterministic_experiment_id() == DeterministicExperimentId::new(3).unwrap()
    ));
    accepted(
        &mut store,
        "cancelled-evaluator-request",
        root_authority,
        Capability::RequestCancellation,
        generation,
        CommandBody::RequestCancellation {
            cycle_id: cycle,
            mode: CancellationMode::EmergencyStop,
        },
    );
    let cancelled_generation = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    accepted(
        &mut store,
        "cancelled-evaluator-snapshot",
        PrincipalId::KERNEL,
        Capability::BeginCancellationPropagation,
        cancelled_generation,
        CommandBody::BeginCancellationPropagation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
    );
    rejected(
        &mut store,
        "cancelled-evaluator-reconcile-before-absence",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellationPropagation,
        cancelled_generation,
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id: CancellationPropagationId::new(1).unwrap(),
        },
        Rejection::CancellationPropagationIncomplete,
    );
    accepted(
        &mut store,
        "cancelled-evaluator-not-spawned",
        PrincipalId::KERNEL,
        Capability::RecordNativeChildNotSpawned,
        cancelled_generation,
        CommandBody::RecordNativeChildNotSpawned {
            native_child_spawn_admission_id: NativeChildSpawnAdmissionId::new(2).unwrap(),
            reason: society_kernel::NativeChildNotSpawnedReason::CancelledBeforeSpawn,
        },
    );
    accepted(
        &mut store,
        "failed-during-cancellation-not-spawned",
        PrincipalId::KERNEL,
        Capability::RecordNativeChildNotSpawned,
        cancelled_generation,
        CommandBody::RecordNativeChildNotSpawned {
            native_child_spawn_admission_id: NativeChildSpawnAdmissionId::new(3).unwrap(),
            reason: society_kernel::NativeChildNotSpawnedReason::NativeSpawnFailed,
        },
    );
    accepted(
        &mut store,
        "cancelled-evaluator-reconcile",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellationPropagation,
        cancelled_generation,
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id: CancellationPropagationId::new(1).unwrap(),
        },
    );
    let cancelled_receipt = accepted(
        &mut store,
        "cancelled-evaluator-finalize",
        root_authority,
        Capability::FinalizeDeterministicExperiment,
        cancelled_generation,
        CommandBody::FinalizeDeterministicExperiment {
            operating_cycle_id: cycle,
            deterministic_experiment_id: DeterministicExperimentId::new(2).unwrap(),
        },
    );
    let cancelled_event_id = match cancelled_receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        other => panic!("accepted helper returned {other:?}"),
    };
    assert!(matches!(
        store.ledger_event(cancelled_event_id).unwrap().body,
        EventBody::DeterministicExperimentFinalized {
            deterministic_experiment_id,
            terminal_state: society_kernel::DeterministicExperimentState::Cancelled,
        } if deterministic_experiment_id == DeterministicExperimentId::new(2).unwrap()
    ));
    let failed_during_cancellation_receipt = accepted(
        &mut store,
        "failed-during-cancellation-finalize",
        root_authority,
        Capability::FinalizeDeterministicExperiment,
        cancelled_generation,
        CommandBody::FinalizeDeterministicExperiment {
            operating_cycle_id: cycle,
            deterministic_experiment_id: DeterministicExperimentId::new(3).unwrap(),
        },
    );
    let failed_during_cancellation_event_id = match failed_during_cancellation_receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        other => panic!("accepted helper returned {other:?}"),
    };
    assert!(matches!(
        store
            .ledger_event(failed_during_cancellation_event_id)
            .unwrap()
            .body,
        EventBody::DeterministicExperimentFinalized {
            deterministic_experiment_id,
            terminal_state: society_kernel::DeterministicExperimentState::Failed,
        } if deterministic_experiment_id == DeterministicExperimentId::new(3).unwrap()
    ));
    assert!(store.validate_replayed_materialized_state().is_ok());
    drop(store);
    let tampered = Connection::open(&path).unwrap();
    tampered
        .execute(
            "UPDATE deterministic_evaluator_forensic_manifest_bindings
                SET evaluator_output_content_object_id = 4
              WHERE forensic_manifest_id = 2",
            [],
        )
        .unwrap();
    drop(tampered);
    assert!(
        KernelStore::open(&path)
            .unwrap()
            .validate_replayed_materialized_state()
            .is_err()
    );
    fs::remove_file(path).unwrap();
}

/// A fresh deterministic M5 fixture with an exact Office owner, active
/// reservation, epoch, and Pi child admission. It stops before the OS spawn
/// so each regression can establish its own physical receipt ordering.
struct AdmittedPiOfficeFixture {
    root_authority: PrincipalId,
    cycle: OperatingCycleId,
    office_session: RootAuthorityOfficeSessionId,
    admission: NativeChildSpawnAdmissionId,
    child: NativeChildId,
    pi_session_identity: PiBoundarySessionIdentity,
    spawn_nonce: SpawnNonce,
    admission_event_id: EventId,
}

fn admitted_pi_office_fixture(store: &mut KernelStore, label: &str) -> AdmittedPiOfficeFixture {
    let (root_authority, cycle) =
        founded_cycle(store, OperatingCycleTreatment::DeterministicPiHostFixtureV1);
    let _project = active_project(store, root_authority, cycle);
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let office_session = RootAuthorityOfficeSessionId::new(1).unwrap();
    accepted(
        store,
        &format!("{label}-reserve"),
        root_authority,
        Capability::ReserveBudget,
        generation,
        CommandBody::ReserveBudget {
            cycle_id: cycle,
            amount: UsdMicros::new(10_000).unwrap(),
        },
    );
    let epoch = SupervisorEpochId::new(1).unwrap();
    let epoch_identity = SupervisorEpochIdentity::parse(format!("epoch-{label}")).unwrap();
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
    let pi_session_identity = PiBoundarySessionIdentity::parse(format!("session-{label}")).unwrap();
    let spawn_nonce = SpawnNonce::parse(format!("nonce-{label}")).unwrap();
    let admission_receipt = accepted(
        store,
        &format!("{label}-admit"),
        PrincipalId::KERNEL,
        Capability::AdmitPiChildSpawn,
        generation,
        CommandBody::AdmitPiChildSpawn {
            operating_cycle_id: cycle,
            owner: PiChildOwner::RootAuthorityOfficeSession(office_session),
            budget_reservation_id: BudgetReservationId::new(1).unwrap(),
            execution_profile_id: ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            native_workspace_id: NativeWorkspaceId::parse(format!("workspace-{label}")).unwrap(),
            canonical_workspace_path: CanonicalWorkspacePath::parse(format!("/tmp/{label}"))
                .unwrap(),
            supervisor_epoch_id: epoch,
            supervisor_epoch_identity: epoch_identity,
            pi_session_identity: pi_session_identity.clone(),
            spawn_nonce: spawn_nonce.clone(),
        },
    );
    let admission_event_id = match admission_receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        CommandDisposition::Rejected(_) => unreachable!("accepted helper returned a rejection"),
    };
    AdmittedPiOfficeFixture {
        root_authority,
        cycle,
        office_session,
        admission: NativeChildSpawnAdmissionId::new(1).unwrap(),
        child: NativeChildId::new(1).unwrap(),
        pi_session_identity,
        spawn_nonce,
        admission_event_id,
    }
}

fn record_fixture_inert_spawn(
    store: &mut KernelStore,
    fixture: &AdmittedPiOfficeFixture,
    label: &str,
    expected_generation: ExpectedGeneration,
) -> CommandReceipt {
    accepted(
        store,
        &format!("{label}-spawn"),
        PrincipalId::KERNEL,
        Capability::RecordInertChildSpawn,
        expected_generation,
        CommandBody::RecordInertChildSpawn {
            native_child_spawn_admission_id: fixture.admission,
            child_identity: SupervisedChildIdentity::parse(format!("child-{label}")).unwrap(),
            direct_child_pid: NativeChildPid::try_from(7101).unwrap(),
            process_group_id: OwnedProcessGroupId::try_from(7101).unwrap(),
        },
    )
}

fn record_fixture_session_ready(
    store: &mut KernelStore,
    fixture: &AdmittedPiOfficeFixture,
    label: &str,
    expected_generation: ExpectedGeneration,
    mark_office_ready: bool,
) {
    record_fixture_inert_spawn(store, fixture, label, expected_generation);
    accepted(
        store,
        &format!("{label}-adapter-ready"),
        PrincipalId::KERNEL,
        Capability::RecordPiAdapterReady,
        expected_generation,
        CommandBody::RecordPiAdapterReady {
            native_child_id: fixture.child,
            pi_session_identity: fixture.pi_session_identity.clone(),
            spawn_nonce: fixture.spawn_nonce.clone(),
        },
    );
    let correlation = PiCorrelationIdentity::parse(format!("create-{label}")).unwrap();
    let create_digest = Blake3Digest::of_bytes(format!("create-{label}").as_bytes());
    accepted(
        store,
        &format!("{label}-create-authorized"),
        PrincipalId::KERNEL,
        Capability::AuthorizePiCreateSession,
        expected_generation,
        CommandBody::AuthorizePiCreateSession {
            native_child_id: fixture.child,
            correlation_identity: correlation.clone(),
            create_request_digest: create_digest,
        },
    );
    accepted(
        store,
        &format!("{label}-create-delivered"),
        PrincipalId::KERNEL,
        Capability::RecordPiCreateSessionDelivery,
        expected_generation,
        CommandBody::RecordPiCreateSessionDelivery {
            native_child_id: fixture.child,
            correlation_identity: correlation,
            create_request_digest: create_digest,
        },
    );
    accepted(
        store,
        &format!("{label}-session-ready"),
        PrincipalId::KERNEL,
        Capability::RecordPiSessionReady,
        expected_generation,
        CommandBody::RecordPiSessionReady {
            native_child_id: fixture.child,
            pi_session_identity: fixture.pi_session_identity.clone(),
        },
    );
    if mark_office_ready {
        accepted(
            store,
            &format!("{label}-office-ready"),
            PrincipalId::KERNEL,
            Capability::RecordOfficeSessionReady,
            expected_generation,
            CommandBody::RecordOfficeSessionReady {
                session_id: fixture.office_session,
            },
        );
    }
}

fn finalize_fixture_child(
    store: &mut KernelStore,
    fixture: &AdmittedPiOfficeFixture,
    label: &str,
    expected_generation: ExpectedGeneration,
) {
    accepted(
        store,
        &format!("{label}-direct-reap"),
        PrincipalId::KERNEL,
        Capability::RecordDirectChildReap,
        expected_generation,
        CommandBody::RecordDirectChildReap {
            native_child_id: fixture.child,
            wait_status: DirectChildWaitStatus::Exited {
                exit_code: ProcessExitCode::try_from(0).unwrap(),
            },
            group_liveness_before_cleanup: ProcessGroupLiveness::Present,
            group_liveness_after_cleanup: ProcessGroupLiveness::Absent,
        },
    );
    for (index, stream) in [
        ChildStreamKind::AdmittedControl,
        ChildStreamKind::PhysicalStdin,
        ChildStreamKind::Stdout,
        ChildStreamKind::Stderr,
    ]
    .into_iter()
    .enumerate()
    {
        let number = i64::try_from(index + 1).unwrap();
        let digest = Blake3Digest::of_bytes(format!("{label}-stream-{number}").as_bytes());
        accepted(
            store,
            &format!("{label}-seal-{number}"),
            PrincipalId::KERNEL,
            Capability::RecordContentSealReceipt,
            ExpectedGeneration::NotApplicable,
            CommandBody::RecordContentSealReceipt { digest },
        );
        accepted(
            store,
            &format!("{label}-register-{number}"),
            PrincipalId::KERNEL,
            Capability::RegisterContentObject,
            ExpectedGeneration::NotApplicable,
            CommandBody::RegisterContentObject {
                content_seal_receipt_id: ContentSealReceiptId::new(number + 1).unwrap(),
            },
        );
        accepted(
            store,
            &format!("{label}-stream-{number}"),
            PrincipalId::KERNEL,
            Capability::RecordChildStreamSeal,
            expected_generation,
            CommandBody::RecordChildStreamSeal {
                native_child_id: fixture.child,
                stream_kind: stream,
                full_observed_digest: digest,
                retained_content_object_id: ContentObjectId::new(number + 1).unwrap(),
                completeness: ChildStreamSealCompleteness::Complete,
            },
        );
    }
    accepted(
        store,
        &format!("{label}-finalize"),
        PrincipalId::KERNEL,
        Capability::FinalizeChildProcess,
        expected_generation,
        CommandBody::FinalizeChildProcess {
            native_child_id: fixture.child,
        },
    );
}

#[test]
fn ledger_event_reads_verified_pi_child_receipts_and_rejects_tampering() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-m5-ledger-event-{nonce}.sqlite"));
    let mut store = KernelStore::open(&path).unwrap();
    let fixture = admitted_pi_office_fixture(&mut store, "m5-ledger-event");
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let inert_receipt =
        record_fixture_inert_spawn(&mut store, &fixture, "m5-ledger-event", generation);
    let inert_event_id = match inert_receipt.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        CommandDisposition::Rejected(_) => unreachable!("accepted helper returned a rejection"),
    };

    let admitted = store.ledger_event(fixture.admission_event_id).unwrap();
    assert_eq!(admitted.event_id, fixture.admission_event_id);
    assert!(matches!(
        admitted.body,
        EventBody::PiChildSpawnAdmitted {
            native_child_spawn_admission_id,
            owner: PiChildOwner::RootAuthorityOfficeSession(office_session),
            budget_reservation_id,
        } if native_child_spawn_admission_id == fixture.admission
            && office_session == fixture.office_session
            && budget_reservation_id == BudgetReservationId::new(1).unwrap()
    ));
    let inert = store.ledger_event(inert_event_id).unwrap();
    assert_eq!(inert.event_id, inert_event_id);
    assert!(matches!(
        inert.body,
        EventBody::InertPiChildSpawnRecorded {
            native_child_id,
            native_child_spawn_admission_id,
        } if native_child_id == fixture.child
            && native_child_spawn_admission_id == fixture.admission
    ));

    let unknown = EventId::new(9_999_999).unwrap();
    assert!(matches!(
        store.ledger_event(unknown),
        Err(StoreError::LedgerEventNotFound(event_id)) if event_id == unknown
    ));

    drop(store);
    let inspect = Connection::open(&path).unwrap();
    // The foreign key is valid, but this second named event body is not: a
    // trusted read must reject the one-to-one body cardinality violation.
    inspect
        .execute(
            "INSERT INTO event_pi_adapter_ready_recorded(
                 event_id, native_child_id, pi_session_id
             ) VALUES (?1, 1, 1)",
            [inert_event_id.value()],
        )
        .unwrap();
    drop(inspect);
    let tampered = KernelStore::open(&path).unwrap();
    assert!(matches!(
        tampered.ledger_event(inert_event_id),
        Err(StoreError::LedgerCorruption(_))
    ));
    drop(tampered);
    fs::remove_file(path).unwrap();
}

#[test]
fn typed_attempt_retry_review_resolution_and_close_are_replayable() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-execution-foundation-{nonce}.sqlite"));
    let mut store = KernelStore::open(&path).unwrap();
    let (root_authority, cycle) = founded_cycle(
        &mut store,
        OperatingCycleTreatment::DeterministicPiHostFixtureV1,
    );
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let project = active_project(&mut store, root_authority, cycle);

    accepted(
        &mut store,
        "m3-ticket-create",
        root_authority,
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
        root_authority,
        Capability::RegisterActorConfiguration,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterActorConfiguration {
            configuration_name: ActorConfigurationName::parse("independent critic v1").unwrap(),
            model_policy: ActorModelPolicy::PinnedDeepseekV4FlashHigh,
            primary_attractor: DevelopmentalAttractor::Challenge,
        },
    );
    let configuration_revision = ActorConfigurationRevisionId::new(1).unwrap();
    accepted(
        &mut store,
        "m3-context-register",
        root_authority,
        Capability::RegisterContextPack,
        generation,
        CommandBody::RegisterContextPack {
            operating_cycle_id: cycle,
            purpose: ContextPackPurpose::IndependentReview,
            rendering_digest: Blake3Digest::of_bytes(b"reviewer-context-v1"),
        },
    );
    let context = society_kernel::ContextPackId::new(1).unwrap();
    accepted(
        &mut store,
        "m3-actor-admit",
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
            reason: ActorAttemptCancellationReason::RootAuthorityRequested,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
        Capability::StartActorAttempt,
        generation,
        start_retry_body.clone(),
    );
    let repeated_request = request(
        &mut store,
        "m3-attempt-start-retry",
        root_authority,
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
            reviewer_principal_id: root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
        Capability::ResolveAdversarialReview,
        generation,
        CommandBody::ResolveAdversarialReview {
            operating_cycle_id: cycle,
            adversarial_review_id: review,
            resolution: ReviewResolutionKind::Resolved,
        },
    );

    // M4 separates a content-store receipt, durable ContentObject identity,
    // deterministic evaluator binding, forensic manifest, and semantic
    // admission. None of these commands executes Pi or an evaluator; the
    // kernel-service facts are narrow receipt seams for later integration.
    let evaluator_digest = Blake3Digest::of_bytes(b"m4-evaluator-revision");
    accepted(
        &mut store,
        "m4-seal-evaluator",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: evaluator_digest,
        },
    );
    let changed_content_request = request(
        &mut store,
        "m4-seal-evaluator",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: Blake3Digest::of_bytes(b"m4-substituted-evaluator"),
        },
    );
    assert!(matches!(
        store.execute(changed_content_request),
        Err(society_kernel::StoreError::IdempotencyConflict)
    ));
    accepted(
        &mut store,
        "m4-register-evaluator",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: ContentSealReceiptId::new(2).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m4-seal-input",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: Blake3Digest::of_bytes(b"m4-input-manifest"),
        },
    );
    accepted(
        &mut store,
        "m4-register-input",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: ContentSealReceiptId::new(3).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m4-experiment-register-first",
        root_authority,
        Capability::RegisterDeterministicExperiment,
        generation,
        CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id: cycle,
            project_id: project,
            ticket_id: ticket,
            target_graph_revision_id: target,
            evaluator_content_object_id: ContentObjectId::new(2).unwrap(),
            input_manifest_content_object_id: ContentObjectId::new(3).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m4-experiment-register-second",
        root_authority,
        Capability::RegisterDeterministicExperiment,
        generation,
        CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id: cycle,
            project_id: project,
            ticket_id: ticket,
            target_graph_revision_id: target,
            evaluator_content_object_id: ContentObjectId::new(2).unwrap(),
            input_manifest_content_object_id: ContentObjectId::new(3).unwrap(),
        },
    );
    let first_experiment = DeterministicExperimentId::new(1).unwrap();
    let second_experiment = DeterministicExperimentId::new(2).unwrap();
    accepted(
        &mut store,
        "m4-seal-shared-output",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: Blake3Digest::of_bytes(b"m4-identical-evaluator-output"),
        },
    );
    accepted(
        &mut store,
        "m4-register-shared-output",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: ContentSealReceiptId::new(4).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m4-seal-recombined-output",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: Blake3Digest::of_bytes(b"m4-recombined-output"),
        },
    );
    accepted(
        &mut store,
        "m4-register-recombined-output",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: ContentSealReceiptId::new(5).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m4-manifest-register-first",
        PrincipalId::KERNEL,
        Capability::RegisterForensicManifest,
        generation,
        CommandBody::RegisterForensicManifest {
            operating_cycle_id: cycle,
            producing_deterministic_experiment_id: first_experiment,
            capture_policy: ForensicManifestCapturePolicy::DeterministicExperimentEvaluatorV1,
            retention_access_class: RetentionAccessClass::ProjectScoped,
            evaluator_output_content_object_id: ContentObjectId::new(4).unwrap(),
        },
    );
    // Byte identity is global. Two exact deterministic runs may emit the same
    // digest, but each manifest remains a separate producer occurrence.
    accepted(
        &mut store,
        "m4-manifest-register-second-shared-output",
        PrincipalId::KERNEL,
        Capability::RegisterForensicManifest,
        generation,
        CommandBody::RegisterForensicManifest {
            operating_cycle_id: cycle,
            producing_deterministic_experiment_id: second_experiment,
            capture_policy: ForensicManifestCapturePolicy::DeterministicExperimentEvaluatorV1,
            retention_access_class: RetentionAccessClass::ForensicRestricted,
            evaluator_output_content_object_id: ContentObjectId::new(4).unwrap(),
        },
    );
    rejected(
        &mut store,
        "m4-evaluation-wrong-evaluator",
        PrincipalId::KERNEL,
        Capability::RecordDeterministicEvaluationReceipt,
        generation,
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id: cycle,
            deterministic_experiment_id: first_experiment,
            evaluator_revision_id: EvaluatorRevisionId::new(2).unwrap(),
            input_manifest_id: InputManifestId::new(1).unwrap(),
            forensic_manifest_id: ForensicManifestId::new(1).unwrap(),
            evaluator_output_content_object_id: ContentObjectId::new(4).unwrap(),
        },
        Rejection::DeterministicEvaluationBindingMismatch,
    );
    rejected(
        &mut store,
        "m4-evaluation-recombined-digest",
        PrincipalId::KERNEL,
        Capability::RecordDeterministicEvaluationReceipt,
        generation,
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id: cycle,
            deterministic_experiment_id: first_experiment,
            evaluator_revision_id: EvaluatorRevisionId::new(1).unwrap(),
            input_manifest_id: InputManifestId::new(1).unwrap(),
            forensic_manifest_id: ForensicManifestId::new(1).unwrap(),
            evaluator_output_content_object_id: ContentObjectId::new(5).unwrap(),
        },
        Rejection::DeterministicEvaluationBindingMismatch,
    );
    rejected(
        &mut store,
        "m4-evaluation-wrong-run-manifest",
        PrincipalId::KERNEL,
        Capability::RecordDeterministicEvaluationReceipt,
        generation,
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id: cycle,
            deterministic_experiment_id: second_experiment,
            evaluator_revision_id: EvaluatorRevisionId::new(1).unwrap(),
            input_manifest_id: InputManifestId::new(1).unwrap(),
            forensic_manifest_id: ForensicManifestId::new(1).unwrap(),
            evaluator_output_content_object_id: ContentObjectId::new(4).unwrap(),
        },
        Rejection::DeterministicEvaluationBindingMismatch,
    );
    rejected(
        &mut store,
        "m4-validate-before-admission",
        PrincipalId::KERNEL,
        Capability::ValidateTicketAttempt,
        generation,
        CommandBody::ValidateTicketAttempt {
            operating_cycle_id: cycle,
            actor_attempt_id: second_attempt,
        },
        Rejection::EvidenceAdmissionRequired,
    );
    accepted(
        &mut store,
        "m4-evaluation-record",
        PrincipalId::KERNEL,
        Capability::RecordDeterministicEvaluationReceipt,
        generation,
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id: cycle,
            deterministic_experiment_id: first_experiment,
            evaluator_revision_id: EvaluatorRevisionId::new(1).unwrap(),
            input_manifest_id: InputManifestId::new(1).unwrap(),
            forensic_manifest_id: ForensicManifestId::new(1).unwrap(),
            evaluator_output_content_object_id: ContentObjectId::new(4).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m4-evidence-admit",
        PrincipalId::KERNEL,
        Capability::AdmitDeterministicEvidence,
        generation,
        CommandBody::AdmitDeterministicEvidence {
            operating_cycle_id: cycle,
            deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId::new(1).unwrap(),
            deterministic_experiment_id: first_experiment,
            evaluator_revision_id: EvaluatorRevisionId::new(1).unwrap(),
            input_manifest_id: InputManifestId::new(1).unwrap(),
            evaluator_output_content_object_id: ContentObjectId::new(4).unwrap(),
            related_graph_revision_id: target,
            semantic_role: EvidenceSemanticRole::DeterministicObservation,
            applicability: EvidenceApplicability::TestsTargetHypothesis,
            limitation: EvidenceLimitationText::parse(
                "Receipt binds a deterministic evaluator identity but does not assert truth or curation.",
            )
            .unwrap(),
        },
    );
    accepted(
        &mut store,
        "m4-evaluation-record-second",
        PrincipalId::KERNEL,
        Capability::RecordDeterministicEvaluationReceipt,
        generation,
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id: cycle,
            deterministic_experiment_id: second_experiment,
            evaluator_revision_id: EvaluatorRevisionId::new(1).unwrap(),
            input_manifest_id: InputManifestId::new(1).unwrap(),
            forensic_manifest_id: ForensicManifestId::new(2).unwrap(),
            evaluator_output_content_object_id: ContentObjectId::new(4).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m4-evidence-admit-second",
        PrincipalId::KERNEL,
        Capability::AdmitDeterministicEvidence,
        generation,
        CommandBody::AdmitDeterministicEvidence {
            operating_cycle_id: cycle,
            deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId::new(2).unwrap(),
            deterministic_experiment_id: second_experiment,
            evaluator_revision_id: EvaluatorRevisionId::new(1).unwrap(),
            input_manifest_id: InputManifestId::new(1).unwrap(),
            evaluator_output_content_object_id: ContentObjectId::new(4).unwrap(),
            related_graph_revision_id: target,
            semantic_role: EvidenceSemanticRole::DeterministicObservation,
            applicability: EvidenceApplicability::TestsTargetHypothesis,
            limitation: EvidenceLimitationText::parse(
                "The same bytes are independently bound to this second deterministic run.",
            )
            .unwrap(),
        },
    );
    let self_validation = CommandRequest {
        command_id: CommandId::parse("m3-attempt-validate-root-authority-self-attest").unwrap(),
        principal_id: root_authority,
        capability_grant_id: store
            .active_capability_grant(root_authority, Capability::CompleteTicket)
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
        Capability::ResolveOutcomeObligation,
        generation,
        CommandBody::ResolveOutcomeObligation {
            operating_cycle_id: cycle,
            outcome_obligation_id: OutcomeObligationId::new(1).unwrap(),
            disposition: OutcomeObligationDisposition::Satisfied,
        },
    );
    rejected(
        &mut store,
        "m4-project-close-open-experiment",
        root_authority,
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
        "m4-experiment-close",
        root_authority,
        Capability::FinalizeDeterministicExperiment,
        generation,
        CommandBody::FinalizeDeterministicExperiment {
            operating_cycle_id: cycle,
            deterministic_experiment_id: first_experiment,
        },
    );
    rejected(
        &mut store,
        "m4-project-close-second-experiment-still-open",
        root_authority,
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
        "m4-experiment-close-second",
        root_authority,
        Capability::FinalizeDeterministicExperiment,
        generation,
        CommandBody::FinalizeDeterministicExperiment {
            operating_cycle_id: cycle,
            deterministic_experiment_id: second_experiment,
        },
    );
    accepted(
        &mut store,
        "m3-project-close",
        root_authority,
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
    assert!(store.replay_ledger().unwrap().iter().any(|event| {
        matches!(
            event.body,
            EventBody::DeterministicEvidenceAdmitted {
                semantic_role: EvidenceSemanticRole::DeterministicObservation,
                applicability: EvidenceApplicability::TestsTargetHypothesis,
                ..
            }
        )
    }));
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
    let shared_output_occurrences: i64 = inspect
        .query_row(
            "SELECT COUNT(*) FROM forensic_manifest_objects
             WHERE content_object_id = 4 AND object_role = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(shared_output_occurrences, 2);
    let distinct_producing_runs: i64 = inspect
        .query_row(
            "SELECT COUNT(DISTINCT manifest.producing_deterministic_experiment_id)
             FROM forensic_manifests manifest
             JOIN forensic_manifest_objects object
               ON object.forensic_manifest_id = manifest.forensic_manifest_id
             WHERE object.content_object_id = 4",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(distinct_producing_runs, 2);
    let manifest_retention_classes: Vec<i64> = inspect
        .prepare(
            "SELECT retention_access_class FROM forensic_manifests
             WHERE forensic_manifest_id IN (1, 2)
             ORDER BY forensic_manifest_id",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(manifest_retention_classes, vec![2, 1]);
    let occurrence_policy_columns_on_content: i64 = inspect
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('content_objects')
             WHERE name IN ('media_schema_contract', 'retention_access_class')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(occurrence_policy_columns_on_content, 0);
    assert!(
        inspect
            .execute(
                "UPDATE forensic_manifest_objects
             SET media_schema_contract = 1
             WHERE forensic_manifest_id = 1",
                [],
            )
            .is_err()
    );
    inspect
        .execute(
            "UPDATE evidence_admissions SET limitation_text = 'tampered M4 limitation'",
            [],
        )
        .unwrap();
    drop(inspect);
    assert!(
        KernelStore::open(&path)
            .unwrap()
            .validate_replayed_materialized_state()
            .is_err()
    );
    let repair_content = Connection::open(&path).unwrap();
    repair_content
        .execute(
            "UPDATE evidence_admissions SET limitation_text = ?1",
            ["Receipt binds a deterministic evaluator identity but does not assert truth or curation."],
        )
        .unwrap();
    drop(repair_content);
    let inspect = Connection::open(&path).unwrap();
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
fn pi_child_receipts_bind_epoch_treatment_cancellation_and_containment() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-m5-child-replay-{nonce}.sqlite"));
    let mut store = KernelStore::open(&path).unwrap();
    let (root_authority, cycle) = founded_cycle(
        &mut store,
        OperatingCycleTreatment::DeterministicPiHostFixtureV1,
    );
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let _project = active_project(&mut store, root_authority, cycle);
    let office_session = RootAuthorityOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "m5-reserve-office-child-budget",
        root_authority,
        Capability::ReserveBudget,
        generation,
        CommandBody::ReserveBudget {
            cycle_id: cycle,
            amount: UsdMicros::new(10_000).unwrap(),
        },
    );
    let reservation = society_kernel::BudgetReservationId::new(1).unwrap();
    let epoch = SupervisorEpochId::new(41).unwrap();
    let epoch_identity = SupervisorEpochIdentity::parse("resident-supervisor-41").unwrap();
    accepted(
        &mut store,
        "m5-open-supervisor-epoch",
        PrincipalId::KERNEL,
        Capability::OpenSupervisorEpoch,
        ExpectedGeneration::NotApplicable,
        CommandBody::OpenSupervisorEpoch {
            supervisor_epoch_id: epoch,
            supervisor_epoch_identity: epoch_identity.clone(),
        },
    );
    rejected(
        &mut store,
        "m5-reject-duplicate-supervisor-epoch",
        PrincipalId::KERNEL,
        Capability::OpenSupervisorEpoch,
        ExpectedGeneration::NotApplicable,
        CommandBody::OpenSupervisorEpoch {
            supervisor_epoch_id: epoch,
            supervisor_epoch_identity: epoch_identity.clone(),
        },
        Rejection::ChildSpawnAdmissionInvalid,
    );
    rejected(
        &mut store,
        "m5-reject-second-supervisor-epoch-lifetime",
        PrincipalId::KERNEL,
        Capability::OpenSupervisorEpoch,
        ExpectedGeneration::NotApplicable,
        CommandBody::OpenSupervisorEpoch {
            supervisor_epoch_id: SupervisorEpochId::new(42).unwrap(),
            supervisor_epoch_identity: SupervisorEpochIdentity::parse("resident-supervisor-42")
                .unwrap(),
        },
        Rejection::ChildSpawnAdmissionInvalid,
    );

    let workspace = NativeWorkspaceId::parse("society-m5-proof").unwrap();
    let workspace_path = CanonicalWorkspacePath::parse("/tmp/society-m5-proof").unwrap();
    assert!(NativeWorkspaceId::parse("society-m5-proof-").is_err());
    assert!(PiBoundarySessionIdentity::parse("π-session").is_err());
    assert!(CanonicalWorkspacePath::parse("/tmp//society-m5-proof").is_err());
    let pi_session = PiBoundarySessionIdentity::parse("pi-session-m5-proof").unwrap();
    let nonce = SpawnNonce::parse("spawn-nonce-m5-proof").unwrap();
    rejected(
        &mut store,
        "m5-reject-native-profile-in-deterministic-cycle",
        PrincipalId::KERNEL,
        Capability::AdmitPiChildSpawn,
        generation,
        CommandBody::AdmitPiChildSpawn {
            operating_cycle_id: cycle,
            owner: PiChildOwner::RootAuthorityOfficeSession(office_session),
            budget_reservation_id: reservation,
            execution_profile_id: ExecutionProfileId::NATIVE_PINNED_PI_SDK_V1,
            native_workspace_id: workspace.clone(),
            canonical_workspace_path: workspace_path.clone(),
            supervisor_epoch_id: epoch,
            supervisor_epoch_identity: epoch_identity.clone(),
            pi_session_identity: pi_session.clone(),
            spawn_nonce: nonce.clone(),
        },
        Rejection::ExecutionProfileIneligible,
    );
    accepted(
        &mut store,
        "m5-admit-double-child",
        PrincipalId::KERNEL,
        Capability::AdmitPiChildSpawn,
        generation,
        CommandBody::AdmitPiChildSpawn {
            operating_cycle_id: cycle,
            owner: PiChildOwner::RootAuthorityOfficeSession(office_session),
            budget_reservation_id: reservation,
            execution_profile_id: ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            native_workspace_id: workspace,
            canonical_workspace_path: workspace_path,
            supervisor_epoch_id: epoch,
            supervisor_epoch_identity: epoch_identity.clone(),
            pi_session_identity: pi_session.clone(),
            spawn_nonce: nonce.clone(),
        },
    );
    let admission = NativeChildSpawnAdmissionId::new(1).unwrap();
    let child = NativeChildId::new(1).unwrap();
    assert!(NativeChildPid::try_from(0).is_err());
    assert!(OwnedProcessGroupId::try_from(-1).is_err());
    assert!(ProcessExitCode::try_from(-1).is_err());
    assert!(ProcessExitCode::try_from(256).is_err());
    rejected(
        &mut store,
        "m5-reject-nonleader-process-group",
        PrincipalId::KERNEL,
        Capability::RecordInertChildSpawn,
        generation,
        CommandBody::RecordInertChildSpawn {
            native_child_spawn_admission_id: admission,
            child_identity: SupervisedChildIdentity::parse("native-child-m5-proof").unwrap(),
            direct_child_pid: NativeChildPid::try_from(4182).unwrap(),
            process_group_id: OwnedProcessGroupId::try_from(4183).unwrap(),
        },
        Rejection::ChildSpawnAdmissionInvalid,
    );
    accepted(
        &mut store,
        "m5-record-inert-child",
        PrincipalId::KERNEL,
        Capability::RecordInertChildSpawn,
        generation,
        CommandBody::RecordInertChildSpawn {
            native_child_spawn_admission_id: admission,
            child_identity: SupervisedChildIdentity::parse("native-child-m5-proof").unwrap(),
            direct_child_pid: NativeChildPid::try_from(4182).unwrap(),
            process_group_id: OwnedProcessGroupId::try_from(4182).unwrap(),
        },
    );
    // An Office authority fact cannot bypass an admitted child whose Pi
    // session has not reached the exact durable SessionReady phase.
    rejected(
        &mut store,
        "m5-office-ready-requires-supervised-pi-ready",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        generation,
        CommandBody::RecordOfficeSessionReady {
            session_id: office_session,
        },
        Rejection::ChildLifecycleReceiptMissing,
    );
    accepted(
        &mut store,
        "m5-adapter-ready",
        PrincipalId::KERNEL,
        Capability::RecordPiAdapterReady,
        generation,
        CommandBody::RecordPiAdapterReady {
            native_child_id: child,
            pi_session_identity: pi_session.clone(),
            spawn_nonce: nonce.clone(),
        },
    );
    let correlation = PiCorrelationIdentity::parse("create-correlation-m5-proof").unwrap();
    let create_digest = Blake3Digest::of_bytes(b"canonical-create-request");
    accepted(
        &mut store,
        "m5-authorize-create",
        PrincipalId::KERNEL,
        Capability::AuthorizePiCreateSession,
        generation,
        CommandBody::AuthorizePiCreateSession {
            native_child_id: child,
            correlation_identity: correlation.clone(),
            create_request_digest: create_digest,
        },
    );
    rejected(
        &mut store,
        "m5-reject-recombined-create-digest",
        PrincipalId::KERNEL,
        Capability::RecordPiCreateSessionDelivery,
        generation,
        CommandBody::RecordPiCreateSessionDelivery {
            native_child_id: child,
            correlation_identity: correlation.clone(),
            create_request_digest: Blake3Digest::of_bytes(b"altered-create-request"),
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "m5-record-create-delivery",
        PrincipalId::KERNEL,
        Capability::RecordPiCreateSessionDelivery,
        generation,
        CommandBody::RecordPiCreateSessionDelivery {
            native_child_id: child,
            correlation_identity: correlation,
            create_request_digest: create_digest,
        },
    );
    accepted(
        &mut store,
        "m5-record-session-ready",
        PrincipalId::KERNEL,
        Capability::RecordPiSessionReady,
        generation,
        CommandBody::RecordPiSessionReady {
            native_child_id: child,
            pi_session_identity: pi_session,
        },
    );
    accepted(
        &mut store,
        "m5-office-ready-after-supervised-pi-ready",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        generation,
        CommandBody::RecordOfficeSessionReady {
            session_id: office_session,
        },
    );
    rejected(
        &mut store,
        "m5-reject-automatic-kill-before-term",
        PrincipalId::KERNEL,
        Capability::RecordProcessSignalReceipt,
        generation,
        CommandBody::RecordProcessSignalReceipt {
            native_child_id: child,
            action: ProcessSignalAction::Kill,
            delivery: ProcessSignalDelivery::Delivered,
            observed_liveness: ProcessGroupLiveness::Present,
            cause: ProcessSignalCause::AutomaticBoundaryContainment,
        },
        Rejection::CancellationPropagationIncomplete,
    );
    accepted(
        &mut store,
        "m5-automatic-containment-term",
        PrincipalId::KERNEL,
        Capability::RecordProcessSignalReceipt,
        generation,
        CommandBody::RecordProcessSignalReceipt {
            native_child_id: child,
            action: ProcessSignalAction::Terminate,
            delivery: ProcessSignalDelivery::Delivered,
            observed_liveness: ProcessGroupLiveness::Present,
            cause: ProcessSignalCause::AutomaticBoundaryContainment,
        },
    );
    rejected(
        &mut store,
        "m5-reject-contradictory-absent-signal-receipt",
        PrincipalId::KERNEL,
        Capability::RecordProcessSignalReceipt,
        generation,
        CommandBody::RecordProcessSignalReceipt {
            native_child_id: child,
            action: ProcessSignalAction::Terminate,
            delivery: ProcessSignalDelivery::AbsentBeforeSignal,
            observed_liveness: ProcessGroupLiveness::Present,
            cause: ProcessSignalCause::AutomaticBoundaryContainment,
        },
        Rejection::ChildLifecycleReceiptMissing,
    );
    rejected(
        &mut store,
        "m5-reject-unsnapshotted-cancellation-abort-control",
        PrincipalId::KERNEL,
        Capability::RecordPiAbortControlDelivery,
        generation,
        CommandBody::RecordPiAbortControlDelivery {
            native_child_id: child,
            cancellation_propagation_id: CancellationPropagationId::new(1).unwrap(),
            correlation_identity: PiCorrelationIdentity::parse("abort-unsnapshotted-m5").unwrap(),
            abort_command_digest: Blake3Digest::of_bytes(b"canonical-abort-unsnapshotted"),
            outcome: PiAbortControlWriteOutcome::FullyWritten,
        },
        Rejection::CancellationPropagationIncomplete,
    );

    accepted(
        &mut store,
        "m5-request-cancellation",
        root_authority,
        Capability::RequestCancellation,
        generation,
        CommandBody::RequestCancellation {
            cycle_id: cycle,
            mode: CancellationMode::EmergencyStop,
        },
    );
    let post_cancel_generation =
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let cancellation = CancellationRequestId::new(1).unwrap();
    let propagation = CancellationPropagationId::new(1).unwrap();
    accepted(
        &mut store,
        "m5-snapshot-cancellation-targets",
        PrincipalId::KERNEL,
        Capability::BeginCancellationPropagation,
        post_cancel_generation,
        CommandBody::BeginCancellationPropagation {
            cancellation_request_id: cancellation,
        },
    );
    rejected(
        &mut store,
        "m5-reject-cancellation-term-before-adapter-abort",
        PrincipalId::KERNEL,
        Capability::RecordProcessSignalReceipt,
        post_cancel_generation,
        CommandBody::RecordProcessSignalReceipt {
            native_child_id: child,
            action: ProcessSignalAction::Terminate,
            delivery: ProcessSignalDelivery::Delivered,
            observed_liveness: ProcessGroupLiveness::Present,
            cause: ProcessSignalCause::CancellationPropagation(propagation),
        },
        Rejection::CancellationPropagationIncomplete,
    );
    accepted(
        &mut store,
        "m5-cancellation-abort-control",
        PrincipalId::KERNEL,
        Capability::RecordPiAbortControlDelivery,
        post_cancel_generation,
        CommandBody::RecordPiAbortControlDelivery {
            native_child_id: child,
            cancellation_propagation_id: propagation,
            correlation_identity: PiCorrelationIdentity::parse("abort-cancellation-m5").unwrap(),
            abort_command_digest: Blake3Digest::of_bytes(b"canonical-abort-cancellation"),
            outcome: PiAbortControlWriteOutcome::FullyWritten,
        },
    );
    accepted(
        &mut store,
        "m5-cancellation-term-after-adapter-abort",
        PrincipalId::KERNEL,
        Capability::RecordProcessSignalReceipt,
        post_cancel_generation,
        CommandBody::RecordProcessSignalReceipt {
            native_child_id: child,
            action: ProcessSignalAction::Terminate,
            delivery: ProcessSignalDelivery::Delivered,
            observed_liveness: ProcessGroupLiveness::Present,
            cause: ProcessSignalCause::CancellationPropagation(propagation),
        },
    );
    accepted(
        &mut store,
        "m5-direct-reap-with-inaccessible-group",
        PrincipalId::KERNEL,
        Capability::RecordDirectChildReap,
        post_cancel_generation,
        CommandBody::RecordDirectChildReap {
            native_child_id: child,
            wait_status: DirectChildWaitStatus::Exited {
                exit_code: ProcessExitCode::try_from(0).unwrap(),
            },
            group_liveness_before_cleanup: ProcessGroupLiveness::Present,
            group_liveness_after_cleanup: ProcessGroupLiveness::Inaccessible,
        },
    );
    rejected(
        &mut store,
        "m5-containment-cannot-finalize",
        PrincipalId::KERNEL,
        Capability::FinalizeChildProcess,
        post_cancel_generation,
        CommandBody::FinalizeChildProcess {
            native_child_id: child,
        },
        Rejection::ProcessContainmentFailed,
    );
    rejected(
        &mut store,
        "m5-recovery-cannot-downclassify-containment",
        PrincipalId::KERNEL,
        Capability::RecordChildRecovery,
        post_cancel_generation,
        CommandBody::RecordChildRecovery {
            native_child_id: child,
            observation: society_kernel::ChildRecoveryObservation::ParentageLost,
            group_liveness_after_restart: ProcessGroupLiveness::Absent,
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "m5-record-containment-propagation-failure",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellationPropagation,
        post_cancel_generation,
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id: propagation,
        },
    );
    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::CancellationPropagationContainmentFailed { cancellation_propagation_id }
            if cancellation_propagation_id == propagation
    )));
    assert!(store.validate_replayed_materialized_state().is_ok());

    // The same pre-spawn fence applies before live Create authorization: M5
    // must not turn the schema-seeded, still-unqualified native profile into
    // an admissible live child merely because an Office has a reservation.
    let mut live = KernelStore::open_in_memory().unwrap();
    let (live_root_authority, live_cycle) =
        founded_cycle(&mut live, OperatingCycleTreatment::PinnedPiSdkLiveV1);
    let live_generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let _live_project = active_project(&mut live, live_root_authority, live_cycle);
    accepted(
        &mut live,
        "m5-live-reserve",
        live_root_authority,
        Capability::ReserveBudget,
        live_generation,
        CommandBody::ReserveBudget {
            cycle_id: live_cycle,
            amount: UsdMicros::new(10_000).unwrap(),
        },
    );
    let live_epoch = SupervisorEpochId::new(77).unwrap();
    let live_epoch_identity =
        SupervisorEpochIdentity::parse("resident-supervisor-live-77").unwrap();
    accepted(
        &mut live,
        "m5-live-open-epoch",
        PrincipalId::KERNEL,
        Capability::OpenSupervisorEpoch,
        ExpectedGeneration::NotApplicable,
        CommandBody::OpenSupervisorEpoch {
            supervisor_epoch_id: live_epoch,
            supervisor_epoch_identity: live_epoch_identity.clone(),
        },
    );
    rejected(
        &mut live,
        "m5-live-reject-unqualified-native-admission",
        PrincipalId::KERNEL,
        Capability::AdmitPiChildSpawn,
        live_generation,
        CommandBody::AdmitPiChildSpawn {
            operating_cycle_id: live_cycle,
            owner: PiChildOwner::RootAuthorityOfficeSession(
                RootAuthorityOfficeSessionId::new(1).unwrap(),
            ),
            budget_reservation_id: society_kernel::BudgetReservationId::new(1).unwrap(),
            execution_profile_id: ExecutionProfileId::NATIVE_PINNED_PI_SDK_V1,
            native_workspace_id: NativeWorkspaceId::parse("society-m5-live").unwrap(),
            canonical_workspace_path: CanonicalWorkspacePath::parse("/tmp/society-m5-live")
                .unwrap(),
            supervisor_epoch_id: live_epoch,
            supervisor_epoch_identity: live_epoch_identity,
            pi_session_identity: PiBoundarySessionIdentity::parse("pi-session-m5-live").unwrap(),
            spawn_nonce: SpawnNonce::parse("spawn-nonce-m5-live").unwrap(),
        },
        Rejection::ExecutionProfileIneligible,
    );
    drop(live);
    drop(store);
    let inspect = Connection::open(&path).unwrap();
    // SQLite independently rejects a contradictory accepted signal receipt;
    // rejected typed commands above remain ledgered because their command body
    // table deliberately does not encode this transition predicate.
    assert!(inspect
        .execute(
            "INSERT INTO process_signal_receipts(native_child_id, signal_action, delivery, observed_liveness, cause_kind, cancellation_propagation_id, recorded_by_command_id)
             VALUES (1, 1, 2, 1, 2, NULL, 1)",
            [],
        )
        .is_err());
    inspect
        .execute(
            "UPDATE native_children SET child_identity = 'tampered-child-m5' WHERE native_child_id = 1",
            [],
        )
        .unwrap();
    drop(inspect);
    assert!(
        KernelStore::open(&path)
            .unwrap()
            .validate_replayed_materialized_state()
            .is_err()
    );
    fs::remove_file(path).unwrap();
}

#[test]
fn lingering_group_cleanup_requires_later_absence_before_finalization() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (root_authority, cycle) = founded_cycle(
        &mut store,
        OperatingCycleTreatment::DeterministicPiHostFixtureV1,
    );
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let _project = active_project(&mut store, root_authority, cycle);
    accepted(
        &mut store,
        "m5-linger-reserve",
        root_authority,
        Capability::ReserveBudget,
        generation,
        CommandBody::ReserveBudget {
            cycle_id: cycle,
            amount: UsdMicros::new(10_000).unwrap(),
        },
    );
    let epoch = SupervisorEpochId::new(88).unwrap();
    let epoch_identity = SupervisorEpochIdentity::parse("resident-supervisor-linger-88").unwrap();
    accepted(
        &mut store,
        "m5-linger-open-epoch",
        PrincipalId::KERNEL,
        Capability::OpenSupervisorEpoch,
        ExpectedGeneration::NotApplicable,
        CommandBody::OpenSupervisorEpoch {
            supervisor_epoch_id: epoch,
            supervisor_epoch_identity: epoch_identity.clone(),
        },
    );
    let pi_session = PiBoundarySessionIdentity::parse("pi-session-m5-linger").unwrap();
    let nonce = SpawnNonce::parse("spawn-nonce-m5-linger").unwrap();
    accepted(
        &mut store,
        "m5-linger-admit",
        PrincipalId::KERNEL,
        Capability::AdmitPiChildSpawn,
        generation,
        CommandBody::AdmitPiChildSpawn {
            operating_cycle_id: cycle,
            owner: PiChildOwner::RootAuthorityOfficeSession(
                RootAuthorityOfficeSessionId::new(1).unwrap(),
            ),
            budget_reservation_id: society_kernel::BudgetReservationId::new(1).unwrap(),
            execution_profile_id: ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            native_workspace_id: NativeWorkspaceId::parse("society-m5-linger").unwrap(),
            canonical_workspace_path: CanonicalWorkspacePath::parse("/tmp/society-m5-linger")
                .unwrap(),
            supervisor_epoch_id: epoch,
            supervisor_epoch_identity: epoch_identity,
            pi_session_identity: pi_session.clone(),
            spawn_nonce: nonce.clone(),
        },
    );
    let child = NativeChildId::new(1).unwrap();
    accepted(
        &mut store,
        "m5-linger-spawn",
        PrincipalId::KERNEL,
        Capability::RecordInertChildSpawn,
        generation,
        CommandBody::RecordInertChildSpawn {
            native_child_spawn_admission_id: NativeChildSpawnAdmissionId::new(1).unwrap(),
            child_identity: SupervisedChildIdentity::parse("native-child-m5-linger").unwrap(),
            direct_child_pid: NativeChildPid::try_from(5182).unwrap(),
            process_group_id: OwnedProcessGroupId::try_from(5182).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m5-linger-adapter",
        PrincipalId::KERNEL,
        Capability::RecordPiAdapterReady,
        generation,
        CommandBody::RecordPiAdapterReady {
            native_child_id: child,
            pi_session_identity: pi_session.clone(),
            spawn_nonce: nonce,
        },
    );
    let digest = Blake3Digest::of_bytes(b"linger-create");
    let correlation = PiCorrelationIdentity::parse("linger-correlation").unwrap();
    accepted(
        &mut store,
        "m5-linger-authorize",
        PrincipalId::KERNEL,
        Capability::AuthorizePiCreateSession,
        generation,
        CommandBody::AuthorizePiCreateSession {
            native_child_id: child,
            correlation_identity: correlation.clone(),
            create_request_digest: digest,
        },
    );
    accepted(
        &mut store,
        "m5-linger-deliver",
        PrincipalId::KERNEL,
        Capability::RecordPiCreateSessionDelivery,
        generation,
        CommandBody::RecordPiCreateSessionDelivery {
            native_child_id: child,
            correlation_identity: correlation,
            create_request_digest: digest,
        },
    );
    accepted(
        &mut store,
        "m5-linger-ready",
        PrincipalId::KERNEL,
        Capability::RecordPiSessionReady,
        generation,
        CommandBody::RecordPiSessionReady {
            native_child_id: child,
            pi_session_identity: pi_session,
        },
    );
    accepted(
        &mut store,
        "m5-linger-direct-reap",
        PrincipalId::KERNEL,
        Capability::RecordDirectChildReap,
        generation,
        CommandBody::RecordDirectChildReap {
            native_child_id: child,
            wait_status: DirectChildWaitStatus::Exited {
                exit_code: ProcessExitCode::try_from(0).unwrap(),
            },
            group_liveness_before_cleanup: ProcessGroupLiveness::Present,
            group_liveness_after_cleanup: ProcessGroupLiveness::Present,
        },
    );
    accepted(
        &mut store,
        "m5-linger-kill",
        PrincipalId::KERNEL,
        Capability::RecordProcessSignalReceipt,
        generation,
        CommandBody::RecordProcessSignalReceipt {
            native_child_id: child,
            action: ProcessSignalAction::LingeringGroupKill,
            delivery: ProcessSignalDelivery::Delivered,
            observed_liveness: ProcessGroupLiveness::Present,
            cause: ProcessSignalCause::AutomaticBoundaryContainment,
        },
    );
    rejected(
        &mut store,
        "m5-linger-cannot-finalize-while-group-present",
        PrincipalId::KERNEL,
        Capability::FinalizeChildProcess,
        generation,
        CommandBody::FinalizeChildProcess {
            native_child_id: child,
        },
        Rejection::ProcessContainmentFailed,
    );
    accepted(
        &mut store,
        "m5-linger-post-kill-group-absent",
        PrincipalId::KERNEL,
        Capability::RecordChildProcessLiveness,
        generation,
        CommandBody::RecordChildProcessLiveness {
            native_child_id: child,
            liveness: ProcessGroupLiveness::Absent,
        },
    );
    for (index, stream_kind) in [
        society_kernel::ChildStreamKind::AdmittedControl,
        society_kernel::ChildStreamKind::PhysicalStdin,
        society_kernel::ChildStreamKind::Stdout,
        society_kernel::ChildStreamKind::Stderr,
    ]
    .into_iter()
    .enumerate()
    {
        let digest = Blake3Digest::of_bytes(format!("m5-linger-stream-{index}").as_bytes());
        accepted(
            &mut store,
            &format!("m5-linger-seal-{index}"),
            PrincipalId::KERNEL,
            Capability::RecordContentSealReceipt,
            ExpectedGeneration::NotApplicable,
            CommandBody::RecordContentSealReceipt { digest },
        );
        accepted(
            &mut store,
            &format!("m5-linger-register-{index}"),
            PrincipalId::KERNEL,
            Capability::RegisterContentObject,
            ExpectedGeneration::NotApplicable,
            CommandBody::RegisterContentObject {
                content_seal_receipt_id: ContentSealReceiptId::new((index + 2) as i64).unwrap(),
            },
        );
        accepted(
            &mut store,
            &format!("m5-linger-stream-seal-{index}"),
            PrincipalId::KERNEL,
            Capability::RecordChildStreamSeal,
            generation,
            CommandBody::RecordChildStreamSeal {
                native_child_id: child,
                stream_kind,
                full_observed_digest: digest,
                retained_content_object_id: ContentObjectId::new((index + 2) as i64).unwrap(),
                completeness: society_kernel::ChildStreamSealCompleteness::Complete,
            },
        );
    }
    accepted(
        &mut store,
        "m5-linger-finalize-after-group-absence",
        PrincipalId::KERNEL,
        Capability::FinalizeChildProcess,
        generation,
        CommandBody::FinalizeChildProcess {
            native_child_id: child,
        },
    );
}

#[test]
fn cancellation_freezes_an_admitted_unspawned_child_until_a_typed_not_spawned_fact() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (root_authority, cycle) = founded_cycle(
        &mut store,
        OperatingCycleTreatment::DeterministicPiHostFixtureV1,
    );
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let _project = active_project(&mut store, root_authority, cycle);
    accepted(
        &mut store,
        "m5-unspawned-reserve",
        root_authority,
        Capability::ReserveBudget,
        generation,
        CommandBody::ReserveBudget {
            cycle_id: cycle,
            amount: UsdMicros::new(10_000).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m5-unspawned-epoch",
        PrincipalId::KERNEL,
        Capability::OpenSupervisorEpoch,
        ExpectedGeneration::NotApplicable,
        CommandBody::OpenSupervisorEpoch {
            supervisor_epoch_id: SupervisorEpochId::new(91).unwrap(),
            supervisor_epoch_identity: SupervisorEpochIdentity::parse("resident-supervisor-91")
                .unwrap(),
        },
    );
    accepted(
        &mut store,
        "m5-unspawned-admission",
        PrincipalId::KERNEL,
        Capability::AdmitPiChildSpawn,
        generation,
        CommandBody::AdmitPiChildSpawn {
            operating_cycle_id: cycle,
            owner: PiChildOwner::RootAuthorityOfficeSession(
                RootAuthorityOfficeSessionId::new(1).unwrap(),
            ),
            budget_reservation_id: society_kernel::BudgetReservationId::new(1).unwrap(),
            execution_profile_id: ExecutionProfileId::DETERMINISTIC_PI_HOST_DOUBLE_V1,
            native_workspace_id: NativeWorkspaceId::parse("society-m5-unspawned").unwrap(),
            canonical_workspace_path: CanonicalWorkspacePath::parse("/tmp/society-m5-unspawned")
                .unwrap(),
            supervisor_epoch_id: SupervisorEpochId::new(91).unwrap(),
            supervisor_epoch_identity: SupervisorEpochIdentity::parse("resident-supervisor-91")
                .unwrap(),
            pi_session_identity: PiBoundarySessionIdentity::parse("pi-session-m5-unspawned")
                .unwrap(),
            spawn_nonce: SpawnNonce::parse("spawn-nonce-m5-unspawned").unwrap(),
        },
    );
    accepted(
        &mut store,
        "m5-unspawned-cancel",
        root_authority,
        Capability::RequestCancellation,
        generation,
        CommandBody::RequestCancellation {
            cycle_id: cycle,
            mode: CancellationMode::EmergencyStop,
        },
    );
    let cancelled_generation = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    accepted(
        &mut store,
        "m5-unspawned-snapshot",
        PrincipalId::KERNEL,
        Capability::BeginCancellationPropagation,
        cancelled_generation,
        CommandBody::BeginCancellationPropagation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
    );
    rejected(
        &mut store,
        "m5-unspawned-reconcile-too-early",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellationPropagation,
        cancelled_generation,
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id: CancellationPropagationId::new(1).unwrap(),
        },
        Rejection::CancellationPropagationIncomplete,
    );
    accepted(
        &mut store,
        "m5-unspawned-invalidate",
        PrincipalId::KERNEL,
        Capability::RecordNativeChildNotSpawned,
        cancelled_generation,
        CommandBody::RecordNativeChildNotSpawned {
            native_child_spawn_admission_id: NativeChildSpawnAdmissionId::new(1).unwrap(),
            reason: society_kernel::NativeChildNotSpawnedReason::CancelledBeforeSpawn,
        },
    );
    rejected(
        &mut store,
        "m5-unspawned-raced-spawn-after-invalidation",
        PrincipalId::KERNEL,
        Capability::RecordInertChildSpawn,
        cancelled_generation,
        CommandBody::RecordInertChildSpawn {
            native_child_spawn_admission_id: NativeChildSpawnAdmissionId::new(1).unwrap(),
            child_identity: SupervisedChildIdentity::parse("native-child-m5-unspawned").unwrap(),
            direct_child_pid: NativeChildPid::try_from(9191).unwrap(),
            process_group_id: OwnedProcessGroupId::try_from(9191).unwrap(),
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "m5-unspawned-reconcile-after-invalidation",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellationPropagation,
        cancelled_generation,
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id: CancellationPropagationId::new(1).unwrap(),
        },
    );
    assert!(store.validate_replayed_materialized_state().is_ok());
}

#[test]
fn office_ready_requires_an_exact_supervised_pi_session_ready_fact() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let fixture = admitted_pi_office_fixture(&mut store, "m5-office-authority");
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);

    rejected(
        &mut store,
        "m5-office-ready-without-child",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        generation,
        CommandBody::RecordOfficeSessionReady {
            session_id: fixture.office_session,
        },
        Rejection::ChildLifecycleReceiptMissing,
    );
    record_fixture_inert_spawn(&mut store, &fixture, "m5-office-authority", generation);
    rejected(
        &mut store,
        "m5-office-ready-before-pi-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        generation,
        CommandBody::RecordOfficeSessionReady {
            session_id: fixture.office_session,
        },
        Rejection::ChildLifecycleReceiptMissing,
    );
    assert!(store.validate_replayed_materialized_state().is_ok());
}

#[test]
fn supervised_office_turns_recheck_the_exact_live_pi_child() {
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let mut store = KernelStore::open_in_memory().unwrap();
    let fixture = admitted_pi_office_fixture(&mut store, "m5-turn-reap-finalize");
    record_fixture_session_ready(&mut store, &fixture, "m5-turn-reap-finalize", zero, true);
    accepted(
        &mut store,
        "m5-turn-direct-reap",
        PrincipalId::KERNEL,
        Capability::RecordDirectChildReap,
        zero,
        CommandBody::RecordDirectChildReap {
            native_child_id: fixture.child,
            wait_status: DirectChildWaitStatus::Exited {
                exit_code: ProcessExitCode::try_from(0).unwrap(),
            },
            group_liveness_before_cleanup: ProcessGroupLiveness::Present,
            group_liveness_after_cleanup: ProcessGroupLiveness::Absent,
        },
    );
    rejected(
        &mut store,
        "m5-turn-reject-after-direct-reap",
        fixture.root_authority,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id: fixture.office_session,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
        Rejection::InvalidLifecycleTransition,
    );
    for (index, stream) in [
        ChildStreamKind::AdmittedControl,
        ChildStreamKind::PhysicalStdin,
        ChildStreamKind::Stdout,
        ChildStreamKind::Stderr,
    ]
    .into_iter()
    .enumerate()
    {
        let number = i64::try_from(index + 1).unwrap();
        let digest = Blake3Digest::of_bytes(format!("m5-turn-final-stream-{number}").as_bytes());
        accepted(
            &mut store,
            &format!("m5-turn-final-seal-{number}"),
            PrincipalId::KERNEL,
            Capability::RecordContentSealReceipt,
            ExpectedGeneration::NotApplicable,
            CommandBody::RecordContentSealReceipt { digest },
        );
        accepted(
            &mut store,
            &format!("m5-turn-final-register-{number}"),
            PrincipalId::KERNEL,
            Capability::RegisterContentObject,
            ExpectedGeneration::NotApplicable,
            CommandBody::RegisterContentObject {
                content_seal_receipt_id: ContentSealReceiptId::new(number + 1).unwrap(),
            },
        );
        accepted(
            &mut store,
            &format!("m5-turn-final-stream-{number}"),
            PrincipalId::KERNEL,
            Capability::RecordChildStreamSeal,
            zero,
            CommandBody::RecordChildStreamSeal {
                native_child_id: fixture.child,
                stream_kind: stream,
                full_observed_digest: digest,
                retained_content_object_id: ContentObjectId::new(number + 1).unwrap(),
                completeness: ChildStreamSealCompleteness::Complete,
            },
        );
    }
    accepted(
        &mut store,
        "m5-turn-finalize-child",
        PrincipalId::KERNEL,
        Capability::FinalizeChildProcess,
        zero,
        CommandBody::FinalizeChildProcess {
            native_child_id: fixture.child,
        },
    );
    rejected(
        &mut store,
        "m5-turn-reject-after-finalize",
        fixture.root_authority,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id: fixture.office_session,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
        Rejection::InvalidLifecycleTransition,
    );
    assert!(store.validate_replayed_materialized_state().is_ok());

    let mut finalized_before_ready = KernelStore::open_in_memory().unwrap();
    let before_ready_fixture = admitted_pi_office_fixture(
        &mut finalized_before_ready,
        "m5-turn-finalized-before-ready",
    );
    record_fixture_session_ready(
        &mut finalized_before_ready,
        &before_ready_fixture,
        "m5-turn-finalized-before-ready",
        zero,
        false,
    );
    finalize_fixture_child(
        &mut finalized_before_ready,
        &before_ready_fixture,
        "m5-turn-finalized-before-ready",
        zero,
    );
    rejected(
        &mut finalized_before_ready,
        "m5-turn-ready-rejects-finalized-child",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        zero,
        CommandBody::RecordOfficeSessionReady {
            session_id: before_ready_fixture.office_session,
        },
        Rejection::ChildLifecycleReceiptMissing,
    );
    assert!(
        finalized_before_ready
            .validate_replayed_materialized_state()
            .is_ok()
    );

    for (label, liveness) in [
        (
            "m5-turn-recovery-containment",
            ProcessGroupLiveness::Present,
        ),
        ("m5-turn-lost-parentage", ProcessGroupLiveness::Absent),
        (
            "m5-turn-containment-failed",
            ProcessGroupLiveness::Inaccessible,
        ),
    ] {
        let mut secondary = KernelStore::open_in_memory().unwrap();
        let secondary_fixture = admitted_pi_office_fixture(&mut secondary, label);
        record_fixture_session_ready(&mut secondary, &secondary_fixture, label, zero, true);
        accepted(
            &mut secondary,
            &format!("{label}-recovery"),
            PrincipalId::KERNEL,
            Capability::RecordChildRecovery,
            zero,
            CommandBody::RecordChildRecovery {
                native_child_id: secondary_fixture.child,
                observation: ChildRecoveryObservation::ParentageLost,
                group_liveness_after_restart: liveness,
            },
        );
        rejected(
            &mut secondary,
            &format!("{label}-reject-turn"),
            secondary_fixture.root_authority,
            Capability::OpenOfficeTurn,
            zero,
            CommandBody::OpenOfficeTurn {
                session_id: secondary_fixture.office_session,
                purpose: OfficeTurnPurpose::Recovery,
            },
            Rejection::InvalidLifecycleTransition,
        );
        assert!(secondary.validate_replayed_materialized_state().is_ok());
    }
}

#[test]
fn buffered_pi_receipts_after_cancellation_are_attributed_without_reopening_work() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let fixture = admitted_pi_office_fixture(&mut store, "m5-buffered-receipts");
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    record_fixture_inert_spawn(&mut store, &fixture, "m5-buffered-receipts", zero);
    accepted(
        &mut store,
        "m5-buffered-adapter-before-cancel",
        PrincipalId::KERNEL,
        Capability::RecordPiAdapterReady,
        zero,
        CommandBody::RecordPiAdapterReady {
            native_child_id: fixture.child,
            pi_session_identity: fixture.pi_session_identity.clone(),
            spawn_nonce: fixture.spawn_nonce.clone(),
        },
    );
    let correlation = PiCorrelationIdentity::parse("buffered-create-correlation").unwrap();
    let create_digest = Blake3Digest::of_bytes(b"buffered-create-request");
    accepted(
        &mut store,
        "m5-buffered-authorize-before-cancel",
        PrincipalId::KERNEL,
        Capability::AuthorizePiCreateSession,
        zero,
        CommandBody::AuthorizePiCreateSession {
            native_child_id: fixture.child,
            correlation_identity: correlation.clone(),
            create_request_digest: create_digest,
        },
    );
    accepted(
        &mut store,
        "m5-buffered-request-cancel",
        fixture.root_authority,
        Capability::RequestCancellation,
        zero,
        CommandBody::RequestCancellation {
            cycle_id: fixture.cycle,
            mode: CancellationMode::EmergencyStop,
        },
    );
    accepted(
        &mut store,
        "m5-buffered-begin-propagation",
        PrincipalId::KERNEL,
        Capability::BeginCancellationPropagation,
        one,
        CommandBody::BeginCancellationPropagation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
    );
    let propagation = CancellationPropagationId::new(1).unwrap();
    accepted(
        &mut store,
        "m5-buffered-term-before-delivery-receipt",
        PrincipalId::KERNEL,
        Capability::RecordProcessSignalReceipt,
        one,
        CommandBody::RecordProcessSignalReceipt {
            native_child_id: fixture.child,
            action: ProcessSignalAction::Terminate,
            delivery: ProcessSignalDelivery::Delivered,
            observed_liveness: ProcessGroupLiveness::Present,
            cause: ProcessSignalCause::CancellationPropagation(propagation),
        },
    );
    rejected(
        &mut store,
        "m5-buffered-reject-altered-create-delivery",
        PrincipalId::KERNEL,
        Capability::RecordPiCreateSessionDelivery,
        one,
        CommandBody::RecordPiCreateSessionDelivery {
            native_child_id: fixture.child,
            correlation_identity: correlation.clone(),
            create_request_digest: Blake3Digest::of_bytes(b"altered-buffered-create"),
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "m5-buffered-record-create-delivery-after-cancel",
        PrincipalId::KERNEL,
        Capability::RecordPiCreateSessionDelivery,
        one,
        CommandBody::RecordPiCreateSessionDelivery {
            native_child_id: fixture.child,
            correlation_identity: correlation.clone(),
            create_request_digest: create_digest,
        },
    );
    accepted(
        &mut store,
        "m5-buffered-record-session-ready-after-cancel",
        PrincipalId::KERNEL,
        Capability::RecordPiSessionReady,
        one,
        CommandBody::RecordPiSessionReady {
            native_child_id: fixture.child,
            pi_session_identity: fixture.pi_session_identity.clone(),
        },
    );
    rejected(
        &mut store,
        "m5-buffered-create-remains-fenced-after-cancel",
        PrincipalId::KERNEL,
        Capability::AuthorizePiCreateSession,
        one,
        CommandBody::AuthorizePiCreateSession {
            native_child_id: fixture.child,
            correlation_identity: PiCorrelationIdentity::parse("second-buffered-create").unwrap(),
            create_request_digest: Blake3Digest::of_bytes(b"second-buffered-create"),
        },
        Rejection::StaleAdmissionGeneration,
    );
    assert!(store.validate_replayed_materialized_state().is_ok());
}

#[test]
fn adapter_ready_race_after_cancellation_preserves_receipt_but_rejects_create() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let fixture = admitted_pi_office_fixture(&mut store, "m5-adapter-race");
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    record_fixture_inert_spawn(&mut store, &fixture, "m5-adapter-race", zero);
    accepted(
        &mut store,
        "m5-adapter-race-request-cancel",
        fixture.root_authority,
        Capability::RequestCancellation,
        zero,
        CommandBody::RequestCancellation {
            cycle_id: fixture.cycle,
            mode: CancellationMode::EmergencyStop,
        },
    );
    accepted(
        &mut store,
        "m5-adapter-race-begin-propagation",
        PrincipalId::KERNEL,
        Capability::BeginCancellationPropagation,
        one,
        CommandBody::BeginCancellationPropagation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m5-adapter-race-term",
        PrincipalId::KERNEL,
        Capability::RecordProcessSignalReceipt,
        one,
        CommandBody::RecordProcessSignalReceipt {
            native_child_id: fixture.child,
            action: ProcessSignalAction::Terminate,
            delivery: ProcessSignalDelivery::Delivered,
            observed_liveness: ProcessGroupLiveness::Present,
            cause: ProcessSignalCause::CancellationPropagation(
                CancellationPropagationId::new(1).unwrap(),
            ),
        },
    );
    accepted(
        &mut store,
        "m5-adapter-race-record-ready-after-cancel",
        PrincipalId::KERNEL,
        Capability::RecordPiAdapterReady,
        one,
        CommandBody::RecordPiAdapterReady {
            native_child_id: fixture.child,
            pi_session_identity: fixture.pi_session_identity.clone(),
            spawn_nonce: fixture.spawn_nonce.clone(),
        },
    );
    for (command_id, expected) in [
        ("m5-adapter-race-old-generation-create", zero),
        ("m5-adapter-race-current-generation-create", one),
    ] {
        rejected(
            &mut store,
            command_id,
            PrincipalId::KERNEL,
            Capability::AuthorizePiCreateSession,
            expected,
            CommandBody::AuthorizePiCreateSession {
                native_child_id: fixture.child,
                correlation_identity: PiCorrelationIdentity::parse(format!(
                    "{command_id}-correlation"
                ))
                .unwrap(),
                create_request_digest: Blake3Digest::of_bytes(command_id.as_bytes()),
            },
            Rejection::StaleAdmissionGeneration,
        );
    }
    assert!(store.validate_replayed_materialized_state().is_ok());
}

#[test]
fn partial_abort_is_a_durable_attempt_and_allows_cancellation_escalation() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let fixture = admitted_pi_office_fixture(&mut store, "m5-partial-abort");
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    record_fixture_inert_spawn(&mut store, &fixture, "m5-partial-abort", zero);
    accepted(
        &mut store,
        "m5-partial-abort-adapter-ready",
        PrincipalId::KERNEL,
        Capability::RecordPiAdapterReady,
        zero,
        CommandBody::RecordPiAdapterReady {
            native_child_id: fixture.child,
            pi_session_identity: fixture.pi_session_identity.clone(),
            spawn_nonce: fixture.spawn_nonce.clone(),
        },
    );
    let create = PiCorrelationIdentity::parse("partial-abort-create").unwrap();
    let create_digest = Blake3Digest::of_bytes(b"partial-abort-create");
    accepted(
        &mut store,
        "m5-partial-abort-authorize-create",
        PrincipalId::KERNEL,
        Capability::AuthorizePiCreateSession,
        zero,
        CommandBody::AuthorizePiCreateSession {
            native_child_id: fixture.child,
            correlation_identity: create.clone(),
            create_request_digest: create_digest,
        },
    );
    accepted(
        &mut store,
        "m5-partial-abort-deliver-create",
        PrincipalId::KERNEL,
        Capability::RecordPiCreateSessionDelivery,
        zero,
        CommandBody::RecordPiCreateSessionDelivery {
            native_child_id: fixture.child,
            correlation_identity: create,
            create_request_digest: create_digest,
        },
    );
    accepted(
        &mut store,
        "m5-partial-abort-session-ready",
        PrincipalId::KERNEL,
        Capability::RecordPiSessionReady,
        zero,
        CommandBody::RecordPiSessionReady {
            native_child_id: fixture.child,
            pi_session_identity: fixture.pi_session_identity.clone(),
        },
    );
    accepted(
        &mut store,
        "m5-partial-abort-request-cancel",
        fixture.root_authority,
        Capability::RequestCancellation,
        zero,
        CommandBody::RequestCancellation {
            cycle_id: fixture.cycle,
            mode: CancellationMode::EmergencyStop,
        },
    );
    accepted(
        &mut store,
        "m5-partial-abort-begin-propagation",
        PrincipalId::KERNEL,
        Capability::BeginCancellationPropagation,
        one,
        CommandBody::BeginCancellationPropagation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
    );
    let propagation = CancellationPropagationId::new(1).unwrap();
    rejected(
        &mut store,
        "m5-partial-abort-term-without-attempt",
        PrincipalId::KERNEL,
        Capability::RecordProcessSignalReceipt,
        one,
        CommandBody::RecordProcessSignalReceipt {
            native_child_id: fixture.child,
            action: ProcessSignalAction::Terminate,
            delivery: ProcessSignalDelivery::Delivered,
            observed_liveness: ProcessGroupLiveness::Present,
            cause: ProcessSignalCause::CancellationPropagation(propagation),
        },
        Rejection::CancellationPropagationIncomplete,
    );
    accepted(
        &mut store,
        "m5-partial-abort-record-partial-write",
        PrincipalId::KERNEL,
        Capability::RecordPiAbortControlDelivery,
        one,
        CommandBody::RecordPiAbortControlDelivery {
            native_child_id: fixture.child,
            cancellation_propagation_id: propagation,
            correlation_identity: PiCorrelationIdentity::parse("partial-abort-attempt").unwrap(),
            abort_command_digest: Blake3Digest::of_bytes(b"partial-abort-command"),
            outcome: PiAbortControlWriteOutcome::PartialWriteDiscarded,
        },
    );
    accepted(
        &mut store,
        "m5-partial-abort-term-after-attempt",
        PrincipalId::KERNEL,
        Capability::RecordProcessSignalReceipt,
        one,
        CommandBody::RecordProcessSignalReceipt {
            native_child_id: fixture.child,
            action: ProcessSignalAction::Terminate,
            delivery: ProcessSignalDelivery::Delivered,
            observed_liveness: ProcessGroupLiveness::Present,
            cause: ProcessSignalCause::CancellationPropagation(propagation),
        },
    );
    assert!(store.validate_replayed_materialized_state().is_ok());
}

#[test]
fn recovery_containment_and_liveness_reuse_remain_durable_close_blockers() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let fixture = admitted_pi_office_fixture(&mut store, "m5-recovery-present");
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    record_fixture_inert_spawn(&mut store, &fixture, "m5-recovery-present", zero);
    accepted(
        &mut store,
        "m5-recovery-parentage-lost-group-present",
        PrincipalId::KERNEL,
        Capability::RecordChildRecovery,
        zero,
        CommandBody::RecordChildRecovery {
            native_child_id: fixture.child,
            observation: ChildRecoveryObservation::ParentageLost,
            group_liveness_after_restart: ProcessGroupLiveness::Present,
        },
    );
    rejected(
        &mut store,
        "m5-recovery-cannot-record-adapter-ready",
        PrincipalId::KERNEL,
        Capability::RecordPiAdapterReady,
        zero,
        CommandBody::RecordPiAdapterReady {
            native_child_id: fixture.child,
            pi_session_identity: fixture.pi_session_identity.clone(),
            spawn_nonce: fixture.spawn_nonce.clone(),
        },
        Rejection::InvalidLifecycleTransition,
    );
    rejected(
        &mut store,
        "m5-recovery-cannot-direct-reap-with-lost-parentage",
        PrincipalId::KERNEL,
        Capability::RecordDirectChildReap,
        zero,
        CommandBody::RecordDirectChildReap {
            native_child_id: fixture.child,
            wait_status: DirectChildWaitStatus::Exited {
                exit_code: ProcessExitCode::try_from(0).unwrap(),
            },
            group_liveness_before_cleanup: ProcessGroupLiveness::Present,
            group_liveness_after_cleanup: ProcessGroupLiveness::Present,
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "m5-recovery-request-cancel",
        fixture.root_authority,
        Capability::RequestCancellation,
        zero,
        CommandBody::RequestCancellation {
            cycle_id: fixture.cycle,
            mode: CancellationMode::EmergencyStop,
        },
    );
    accepted(
        &mut store,
        "m5-recovery-begin-propagation",
        PrincipalId::KERNEL,
        Capability::BeginCancellationPropagation,
        one,
        CommandBody::BeginCancellationPropagation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
    );
    rejected(
        &mut store,
        "m5-recovery-cannot-reconcile-present-group",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellationPropagation,
        one,
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id: CancellationPropagationId::new(1).unwrap(),
        },
        Rejection::CancellationPropagationIncomplete,
    );
    accepted(
        &mut store,
        "m5-recovery-liveness-absent",
        PrincipalId::KERNEL,
        Capability::RecordChildProcessLiveness,
        one,
        CommandBody::RecordChildProcessLiveness {
            native_child_id: fixture.child,
            liveness: ProcessGroupLiveness::Absent,
        },
    );
    accepted(
        &mut store,
        "m5-recovery-reconcile-absent-group",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellationPropagation,
        one,
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id: CancellationPropagationId::new(1).unwrap(),
        },
    );

    let mut inaccessible = KernelStore::open_in_memory().unwrap();
    let inaccessible_fixture =
        admitted_pi_office_fixture(&mut inaccessible, "m5-recovery-inaccessible");
    record_fixture_inert_spawn(
        &mut inaccessible,
        &inaccessible_fixture,
        "m5-recovery-inaccessible",
        zero,
    );
    accepted(
        &mut inaccessible,
        "m5-recovery-parentage-lost-group-inaccessible",
        PrincipalId::KERNEL,
        Capability::RecordChildRecovery,
        zero,
        CommandBody::RecordChildRecovery {
            native_child_id: inaccessible_fixture.child,
            observation: ChildRecoveryObservation::ParentageLost,
            group_liveness_after_restart: ProcessGroupLiveness::Inaccessible,
        },
    );
    rejected(
        &mut inaccessible,
        "m5-recovery-inaccessible-cannot-finalize",
        PrincipalId::KERNEL,
        Capability::FinalizeChildProcess,
        zero,
        CommandBody::FinalizeChildProcess {
            native_child_id: inaccessible_fixture.child,
        },
        Rejection::ProcessContainmentFailed,
    );
    assert!(inaccessible.validate_replayed_materialized_state().is_ok());

    let mut reappearance = KernelStore::open_in_memory().unwrap();
    let reappearance_fixture = admitted_pi_office_fixture(&mut reappearance, "m5-reappearance");
    record_fixture_inert_spawn(
        &mut reappearance,
        &reappearance_fixture,
        "m5-reappearance",
        zero,
    );
    accepted(
        &mut reappearance,
        "m5-reappearance-observe-absent",
        PrincipalId::KERNEL,
        Capability::RecordChildProcessLiveness,
        zero,
        CommandBody::RecordChildProcessLiveness {
            native_child_id: reappearance_fixture.child,
            liveness: ProcessGroupLiveness::Absent,
        },
    );
    accepted(
        &mut reappearance,
        "m5-reappearance-observe-present-conflict",
        PrincipalId::KERNEL,
        Capability::RecordChildProcessLiveness,
        zero,
        CommandBody::RecordChildProcessLiveness {
            native_child_id: reappearance_fixture.child,
            liveness: ProcessGroupLiveness::Present,
        },
    );
    rejected(
        &mut reappearance,
        "m5-reappearance-containment-cannot-finalize",
        PrincipalId::KERNEL,
        Capability::FinalizeChildProcess,
        zero,
        CommandBody::FinalizeChildProcess {
            native_child_id: reappearance_fixture.child,
        },
        Rejection::ProcessContainmentFailed,
    );
    assert!(store.validate_replayed_materialized_state().is_ok());
    assert!(reappearance.validate_replayed_materialized_state().is_ok());
}

#[test]
fn pre_spawn_failure_and_raced_spawn_are_accounted_before_cancellation_reconciliation() {
    let mut ordinary = KernelStore::open_in_memory().unwrap();
    let ordinary_fixture = admitted_pi_office_fixture(&mut ordinary, "m5-ordinary-no-spawn");
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let body = CommandBody::RecordNativeChildNotSpawned {
        native_child_spawn_admission_id: ordinary_fixture.admission,
        reason: society_kernel::NativeChildNotSpawnedReason::NativeSpawnFailed,
    };
    let first = accepted(
        &mut ordinary,
        "m5-ordinary-no-spawn",
        PrincipalId::KERNEL,
        Capability::RecordNativeChildNotSpawned,
        zero,
        body.clone(),
    );
    let repeat_request = request(
        &mut ordinary,
        "m5-ordinary-no-spawn",
        PrincipalId::KERNEL,
        Capability::RecordNativeChildNotSpawned,
        zero,
        body,
    );
    let repeated = ordinary.execute(repeat_request).unwrap();
    assert!(repeated.idempotent);
    assert_eq!(repeated.disposition, first.disposition);
    assert!(ordinary.validate_replayed_materialized_state().is_ok());

    let mut raced = KernelStore::open_in_memory().unwrap();
    let raced_fixture = admitted_pi_office_fixture(&mut raced, "m5-raced-spawn");
    accepted(
        &mut raced,
        "m5-raced-spawn-request-cancel",
        raced_fixture.root_authority,
        Capability::RequestCancellation,
        zero,
        CommandBody::RequestCancellation {
            cycle_id: raced_fixture.cycle,
            mode: CancellationMode::EmergencyStop,
        },
    );
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    accepted(
        &mut raced,
        "m5-raced-spawn-snapshot",
        PrincipalId::KERNEL,
        Capability::BeginCancellationPropagation,
        one,
        CommandBody::BeginCancellationPropagation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
    );
    record_fixture_inert_spawn(&mut raced, &raced_fixture, "m5-raced-spawn", one);
    rejected(
        &mut raced,
        "m5-raced-spawn-cannot-reconcile-live-child",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellationPropagation,
        one,
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id: CancellationPropagationId::new(1).unwrap(),
        },
        Rejection::CancellationPropagationIncomplete,
    );
    accepted(
        &mut raced,
        "m5-raced-spawn-recovery-inaccessible",
        PrincipalId::KERNEL,
        Capability::RecordChildRecovery,
        one,
        CommandBody::RecordChildRecovery {
            native_child_id: raced_fixture.child,
            observation: ChildRecoveryObservation::ParentageLost,
            group_liveness_after_restart: ProcessGroupLiveness::Inaccessible,
        },
    );
    accepted(
        &mut raced,
        "m5-raced-spawn-reconcile-containment",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellationPropagation,
        one,
        CommandBody::ReconcileCancellationPropagation {
            cancellation_propagation_id: CancellationPropagationId::new(1).unwrap(),
        },
    );
    assert!(raced.validate_replayed_materialized_state().is_ok());
}

#[test]
fn lease_expiry_requires_a_work_item_without_an_attempt() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (root_authority, cycle) = founded_cycle(
        &mut store,
        OperatingCycleTreatment::DeterministicPiHostFixtureV1,
    );
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let project = active_project(&mut store, root_authority, cycle);
    accepted(
        &mut store,
        "expiry-ticket-create",
        root_authority,
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
        root_authority,
        Capability::RegisterActorConfiguration,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterActorConfiguration {
            configuration_name: ActorConfigurationName::parse("worker configuration").unwrap(),
            model_policy: ActorModelPolicy::PinnedDeepseekV4FlashHigh,
            primary_attractor: DevelopmentalAttractor::Build,
        },
    );
    accepted(
        &mut store,
        "expiry-context",
        root_authority,
        Capability::RegisterContextPack,
        generation,
        CommandBody::RegisterContextPack {
            operating_cycle_id: cycle,
            purpose: ContextPackPurpose::TicketExecution,
            rendering_digest: Blake3Digest::of_bytes(b"ticket context"),
        },
    );
    accepted(
        &mut store,
        "expiry-actor",
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
/// live pinned Pi SDK run; the unqualified native identity cannot admit an actor in
/// any treatment before the later typed qualification receipt exists.
#[test]
fn execution_profile_admission_is_closed_by_treatment_and_readiness() {
    let deterministic_cases = [
        OperatingCycleTreatment::PiSdkQualificationV1,
        OperatingCycleTreatment::PinnedPiSdkLiveV1,
    ];
    for (index, treatment) in deterministic_cases.into_iter().enumerate() {
        let mut store = KernelStore::open_in_memory().unwrap();
        let (root_authority, cycle) = founded_cycle(&mut store, treatment);
        let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
        if treatment != OperatingCycleTreatment::PiSdkQualificationV1 {
            let _project = active_project(&mut store, root_authority, cycle);
        }
        if treatment != OperatingCycleTreatment::PiSdkQualificationV1 {
            accepted(
                &mut store,
                "profile-double-config",
                root_authority,
                Capability::RegisterActorConfiguration,
                ExpectedGeneration::NotApplicable,
                CommandBody::RegisterActorConfiguration {
                    configuration_name: ActorConfigurationName::parse("profile gate configuration")
                        .unwrap(),
                    model_policy: ActorModelPolicy::PinnedDeepseekV4FlashHigh,
                    primary_attractor: DevelopmentalAttractor::Build,
                },
            );
        }
        rejected(
            &mut store,
            &format!("profile-double-rejected-{index}"),
            root_authority,
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
        OperatingCycleTreatment::PinnedPiSdkLiveV1,
        OperatingCycleTreatment::DeterministicPiHostFixtureV1,
    ];
    for (index, treatment) in native_cases.into_iter().enumerate() {
        let mut store = KernelStore::open_in_memory().unwrap();
        let (root_authority, cycle) = founded_cycle(&mut store, treatment);
        let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
        if treatment != OperatingCycleTreatment::PiSdkQualificationV1 {
            let _project = active_project(&mut store, root_authority, cycle);
        }
        if treatment != OperatingCycleTreatment::PiSdkQualificationV1 {
            accepted(
                &mut store,
                "profile-native-config",
                root_authority,
                Capability::RegisterActorConfiguration,
                ExpectedGeneration::NotApplicable,
                CommandBody::RegisterActorConfiguration {
                    configuration_name: ActorConfigurationName::parse("native gate configuration")
                        .unwrap(),
                    model_policy: ActorModelPolicy::PinnedDeepseekV4FlashHigh,
                    primary_attractor: DevelopmentalAttractor::Build,
                },
            );
        }
        rejected(
            &mut store,
            &format!("profile-native-rejected-{index}"),
            root_authority,
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
fn paid_qualification_treatment_has_no_root_authority_work_surface() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (root_authority, cycle) =
        founded_cycle(&mut store, OperatingCycleTreatment::PiSdkQualificationV1);
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    rejected(
        &mut store,
        "qualification-no-office-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        generation,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id: cycle },
        Rejection::QualificationTreatmentRestricted,
    );
    rejected(
        &mut store,
        "qualification-no-project",
        root_authority,
        Capability::CreateProject,
        generation,
        CommandBody::CreateProject {
            operating_cycle_id: cycle,
            project_name: ProjectName::parse("Forbidden qualification project").unwrap(),
            north_star_alignment: example_project_north_star_alignment(),
        },
        Rejection::QualificationTreatmentRestricted,
    );
    rejected(
        &mut store,
        "qualification-no-actor-admission",
        root_authority,
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
    let (root_authority, cycle) = founded_cycle(
        &mut store,
        OperatingCycleTreatment::DeterministicPiHostFixtureV1,
    );
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let project = active_project(&mut store, root_authority, cycle);
    accepted(
        &mut store,
        "cross-review-ticket",
        root_authority,
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
            root_authority,
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
            root_authority,
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
            root_authority,
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
        root_authority,
        Capability::RegisterActorConfiguration,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterActorConfiguration {
            configuration_name: ActorConfigurationName::parse("cross review critic").unwrap(),
            model_policy: ActorModelPolicy::PinnedDeepseekV4FlashHigh,
            primary_attractor: DevelopmentalAttractor::Challenge,
        },
    );
    accepted(
        &mut store,
        "cross-review-context",
        root_authority,
        Capability::RegisterContextPack,
        generation,
        CommandBody::RegisterContextPack {
            operating_cycle_id: cycle,
            purpose: ContextPackPurpose::IndependentReview,
            rendering_digest: Blake3Digest::of_bytes(b"cross-review-context"),
        },
    );
    accepted(
        &mut store,
        "cross-review-actor",
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
    let path = std::env::temp_dir().join(format!("society-capability-origin-{nonce}.sqlite"));
    let mut store = KernelStore::open(&path).unwrap();
    let (root_authority, _) = founded_cycle(
        &mut store,
        OperatingCycleTreatment::DeterministicPiHostFixtureV1,
    );
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
    let root_authority_ledger_grants: i64 = inspect
        .query_row(
            "SELECT COUNT(*) FROM capability_grants
             WHERE principal_id = ?1 AND grant_origin = 2
               AND granted_by_command_id IS NOT NULL",
            [root_authority.value()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        root_authority_ledger_grants,
        Capability::ROOT_AUTHORITY.len() as i64
    );
    assert!(
        inspect
            .execute(
                "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id,
                                             actor_instance_id, grant_state, grant_origin,
                                             granted_by_command_id, consumed_by_command_id)
             VALUES (?1, 54, NULL, NULL, 1, 3, NULL, NULL)",
                [root_authority.value()],
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
