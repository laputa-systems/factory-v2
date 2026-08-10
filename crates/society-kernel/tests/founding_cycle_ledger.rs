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
    AdmissionGeneration, AdversarialReviewId, ApplicationIdentity, ApplicationMissionInput,
    ApplicationName, ApplicationRevisionId, ApplicationRevisionOrdinal, Blake3Digest,
    BudgetFreezeReason, BudgetReservationId, CancellationRequestId,
    CanonicalPiSessionTranscriptPath, CanonicalWorkspacePath, Capability, CausalEpisodeId,
    CommandBody, CommandDisposition, CommandId, CommandReceipt, CommandRequest, CostObservation,
    CostPostmortemResolution, CostUnavailableReason, CostUnknownReason, EpisodeState, EventBody,
    ExpectedGeneration, GraphEdgeKind, GraphRevisionBody, GraphRevisionId, HypothesisRevisionText,
    InstallFoundingMissionPreflight, KernelStore, MissionPrinciple, MissionPrincipleKind,
    MissionPrincipleText, MissionPrinciples, MissionSourceRendering, MissionStatement,
    NativeChildId, NativeChildPid, NativeChildSpawnAdmissionId, NativeWorkspaceId,
    NorthStarBoundaryCommitmentQuestion, NorthStarChangeQuestion,
    NorthStarImprovementEvidenceQuestion, NorthStarQuestionSet, NorthStarRevisitQuestion,
    ObservationRevisionText, OfficeSessionTerminalState, OfficeTurnId, OfficeTurnPurpose,
    OperatingCycleId, OperatingCycleState, OperatingCycleTreatment, OwnedProcessGroupId,
    PiBoundarySessionIdentity, PiChildOwner, PiCorrelationIdentity, PiCumulativeUsage,
    PiOfficeSessionTranscriptReceipt, PiOfficeTurnAssistantOutcome, PiOfficeTurnDisposition,
    PiOfficeTurnTerminalEvidence, PiOfficeTurnTerminalReceiptId, PiOfficeTurnTranscriptDisposition,
    PiOfficeTurnUsageFailure, PiOfficeTurnUsageUnavailableReason, PiProtocolSequence, PiTokenCount,
    PostmortemActionKind, PostmortemActionProposalText, PostmortemCausalClaimKind,
    PostmortemCausalClaimText, PostmortemId, PrincipalDisplayName, PrincipalId, ProjectId,
    ProjectMilestoneName, ProjectName, ProjectNorthStarAlignment,
    ProjectNorthStarBoundaryCommitmentAnswer, ProjectNorthStarChangeAnswer,
    ProjectNorthStarImprovementEvidenceAnswer, ProjectNorthStarRevisitAnswer, ProjectObjectiveText,
    ProjectState, ProjectStopConditionText, ProviderCostBinary64, Rejection,
    ReviewChallengeSeverity, ReviewFailureHypothesis, RootAuthorityOfficeSessionId, SocietyName,
    SpawnNonce, StoreError, SupervisedChildIdentity, SupervisorEpochId, SupervisorEpochIdentity,
    UsdMicros,
};

fn example_application_mission() -> ApplicationMissionInput {
    ApplicationMissionInput {
        application_identity: ApplicationIdentity::parse("example-application").unwrap(),
        application_name: ApplicationName::parse("Example Application").unwrap(),
        revision_ordinal: ApplicationRevisionOrdinal::new(1).unwrap(),
        statement: MissionStatement::parse("Improve a bounded example system responsibly.")
            .unwrap(),
        principles: MissionPrinciples::new(vec![
            MissionPrinciple {
                kind: MissionPrincipleKind::Purpose,
                text: MissionPrincipleText::parse("Pursue a clear public purpose.").unwrap(),
            },
            MissionPrinciple {
                kind: MissionPrincipleKind::Evidence,
                text: MissionPrincipleText::parse("Prefer reproducible improvement evidence.")
                    .unwrap(),
            },
            MissionPrinciple {
                kind: MissionPrincipleKind::Boundary,
                text: MissionPrincipleText::parse("Preserve explicit safety boundaries.").unwrap(),
            },
        ])
        .unwrap(),
        north_star_questions: NorthStarQuestionSet {
            change: NorthStarChangeQuestion::parse("What change should this application make?")
                .unwrap(),
            improvement_evidence: NorthStarImprovementEvidenceQuestion::parse(
                "What evidence would demonstrate improvement?",
            )
            .unwrap(),
            boundary_commitment: NorthStarBoundaryCommitmentQuestion::parse(
                "Which boundary must remain intact?",
            )
            .unwrap(),
            revisit: NorthStarRevisitQuestion::parse("When should this direction be revisited?")
                .unwrap(),
        },
        source_rendering_digest: Blake3Digest::of_bytes(b"example application mission revision 1"),
    }
}

fn example_project_north_star_alignment() -> ProjectNorthStarAlignment {
    ProjectNorthStarAlignment {
        application_revision_id: ApplicationRevisionId::new(1).unwrap(),
        change_answer: ProjectNorthStarChangeAnswer::parse("Deliver one bounded improvement.")
            .unwrap(),
        improvement_evidence_answer: ProjectNorthStarImprovementEvidenceAnswer::parse(
            "Use a reproducible integration judge.",
        )
        .unwrap(),
        boundary_commitment_answer: ProjectNorthStarBoundaryCommitmentAnswer::parse(
            "Do not expand authority outside the fixture.",
        )
        .unwrap(),
        revisit_answer: ProjectNorthStarRevisitAnswer::parse(
            "Revisit after the next durable evidence review.",
        )
        .unwrap(),
    }
}

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

fn seal_and_register_mission_source(
    store: &mut KernelStore,
    command_prefix: &str,
    mission: &ApplicationMissionInput,
) {
    let kernel = PrincipalId::KERNEL;
    accepted(
        store,
        &format!("{command_prefix}-seal-mission-source"),
        kernel,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: mission.source_rendering_digest,
        },
    );
    accepted(
        store,
        &format!("{command_prefix}-register-mission-source"),
        kernel,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(1).unwrap(),
        },
    );
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
    let mission = example_application_mission();
    seal_and_register_mission_source(store, "found", &mission);
    accepted(
        store,
        "found-install-founding-mission",
        bootstrap,
        Capability::InstallFoundingMission,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallFoundingMission { mission },
    );
    accepted(
        store,
        "found-install-office",
        bootstrap,
        Capability::InstallRootAuthorityOffice,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallRootAuthorityOffice,
    );
    accepted(
        store,
        "found-appoint-root-authority",
        bootstrap,
        Capability::AppointInitialRootAuthority,
        ExpectedGeneration::NotApplicable,
        CommandBody::AppointInitialRootAuthority {
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
            ceiling: UsdMicros::new(1_030_000).unwrap(),
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
            budget_ceiling: UsdMicros::new(1_000_000).unwrap(),
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
fn founding_mission_requires_a_registered_exact_source_object_and_preflight_is_side_effect_free() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let bootstrap = PrincipalId::BOOTSTRAP;
    accepted(
        &mut store,
        "source-create-society",
        bootstrap,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        CommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("Source-bound Society").unwrap(),
        },
    );
    let mission = example_application_mission();
    let request = CommandRequest {
        command_id: CommandId::parse("source-install-mission").unwrap(),
        principal_id: bootstrap,
        capability_grant_id: store
            .active_capability_grant(bootstrap, Capability::InstallFoundingMission)
            .unwrap()
            .unwrap(),
        capability: Capability::InstallFoundingMission,
        expected_generation: ExpectedGeneration::NotApplicable,
        body: CommandBody::InstallFoundingMission {
            mission: mission.clone(),
        },
    };
    assert_eq!(
        store.preflight_install_founding_mission(&request).unwrap(),
        InstallFoundingMissionPreflight::Ready
    );
    assert_eq!(store.command_count().unwrap(), 1);
    assert!(
        store
            .active_capability_grant(bootstrap, Capability::InstallFoundingMission)
            .unwrap()
            .is_some()
    );

    let unsealed = store.execute(request.clone()).unwrap();
    assert_eq!(
        unsealed.disposition,
        CommandDisposition::Rejected(Rejection::MissionSourceContentNotSealed)
    );
    assert!(
        store
            .active_capability_grant(bootstrap, Capability::InstallFoundingMission)
            .unwrap()
            .is_some()
    );

    accepted(
        &mut store,
        "source-record-receipt",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: mission.source_rendering_digest,
        },
    );
    let receipt_only = CommandRequest {
        command_id: CommandId::parse("source-receipt-only-install").unwrap(),
        ..request.clone()
    };
    assert_eq!(
        store.execute(receipt_only).unwrap().disposition,
        CommandDisposition::Rejected(Rejection::MissionSourceContentNotSealed)
    );
    accepted(
        &mut store,
        "source-register-object",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(1).unwrap(),
        },
    );
    let accepted_request = CommandRequest {
        command_id: CommandId::parse("source-bound-install").unwrap(),
        ..request
    };
    let accepted_receipt = store.execute(accepted_request.clone()).unwrap();
    assert!(matches!(
        accepted_receipt.disposition,
        CommandDisposition::Accepted(_)
    ));
    assert_eq!(
        store
            .preflight_install_founding_mission(&accepted_request)
            .unwrap(),
        InstallFoundingMissionPreflight::ExistingReceipt(CommandReceipt {
            disposition: accepted_receipt.disposition,
            idempotent: true,
        })
    );
    let mut conflicting_mission = mission;
    conflicting_mission.statement =
        MissionStatement::parse("A conflicting source mission.").unwrap();
    let conflicting_request = CommandRequest {
        body: CommandBody::InstallFoundingMission {
            mission: conflicting_mission,
        },
        ..accepted_request
    };
    assert!(matches!(
        store.preflight_install_founding_mission(&conflicting_request),
        Err(StoreError::IdempotencyConflict)
    ));
}

#[test]
fn mission_source_rendering_is_bounded_and_hashes_its_exact_bytes() {
    let rendering =
        MissionSourceRendering::parse(vec![7; MissionSourceRendering::MAX_BYTES]).unwrap();
    assert_eq!(
        rendering.digest(),
        Blake3Digest::of_bytes(rendering.as_bytes())
    );
    assert!(MissionSourceRendering::parse(Vec::new()).is_err());
    assert!(MissionSourceRendering::parse(vec![0; MissionSourceRendering::MAX_BYTES + 1]).is_err());
}

#[test]
fn replay_rejects_recombined_persisted_mission_source_objects() {
    let path = std::env::temp_dir().join(format!(
        "society-mission-source-tamper-{}-{}.sqlite",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut store = KernelStore::open(&path).unwrap();
    let bootstrap = PrincipalId::BOOTSTRAP;
    accepted(
        &mut store,
        "tamper-source-create-society",
        bootstrap,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        CommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("Tamper source Society").unwrap(),
        },
    );
    let mission = example_application_mission();
    accepted(
        &mut store,
        "tamper-source-seal-mission",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: mission.source_rendering_digest,
        },
    );
    accepted(
        &mut store,
        "tamper-source-register-mission",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(1).unwrap(),
        },
    );
    accepted(
        &mut store,
        "tamper-source-seal-other",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: Blake3Digest::of_bytes(b"different sealed source"),
        },
    );
    accepted(
        &mut store,
        "tamper-source-register-other",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(2).unwrap(),
        },
    );
    accepted(
        &mut store,
        "tamper-source-install",
        bootstrap,
        Capability::InstallFoundingMission,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallFoundingMission { mission },
    );
    drop(store);

    let tamper = Connection::open(&path).unwrap();
    tamper
        .execute(
            "UPDATE application_revisions SET source_content_object_id = 2",
            [],
        )
        .unwrap();
    drop(tamper);
    assert!(matches!(
        KernelStore::open(&path).unwrap().replay_ledger(),
        Err(StoreError::LedgerCorruption(_))
    ));

    let repair = Connection::open(&path).unwrap();
    repair
        .execute(
            "UPDATE application_revisions SET source_content_object_id = 1",
            [],
        )
        .unwrap();
    repair
        .execute(
            "UPDATE command_install_founding_mission SET source_content_object_id = 2",
            [],
        )
        .unwrap();
    drop(repair);
    assert!(matches!(
        KernelStore::open(&path).unwrap().replay_ledger(),
        Err(StoreError::LedgerCorruption(_))
    ));
    fs::remove_file(path).unwrap();
}

#[test]
fn current_operating_cycle_generation_is_typed_and_distinguishes_absence_from_corruption() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-cycle-generation-read-{nonce}.sqlite"));
    let mut store = KernelStore::open(&path).unwrap();
    let (root_authority, cycle_id) = found_cycle(&mut store);
    assert_eq!(
        store
            .current_operating_cycle_admission_generation(cycle_id)
            .unwrap(),
        AdmissionGeneration::INITIAL
    );

    accepted(
        &mut store,
        "cycle-generation-read-cancel",
        root_authority,
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
    root_authority: PrincipalId,
    cycle_id: OperatingCycleId,
    session_id: RootAuthorityOfficeSessionId,
    label: &str,
    reservation_amount: UsdMicros,
) {
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    accepted(
        store,
        &format!("{label}-reserve"),
        root_authority,
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
            owner: PiChildOwner::RootAuthorityOfficeSession(session_id),
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
    let child = NativeChildId::new(1).unwrap();
    accepted(
        store,
        &format!("{label}-spawn"),
        PrincipalId::KERNEL,
        Capability::RecordInertChildSpawn,
        generation,
        CommandBody::RecordInertChildSpawn {
            native_child_spawn_admission_id: NativeChildSpawnAdmissionId::new(1).unwrap(),
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
            native_child_id: child,
            pi_session_identity: session_identity.clone(),
            spawn_nonce,
        },
    );
    let correlation = PiCorrelationIdentity::parse(format!("create-{label}")).unwrap();
    let create_digest = Blake3Digest::of_bytes(format!("create-{label}").as_bytes());
    accepted(
        store,
        &format!("{label}-create-authorized"),
        PrincipalId::KERNEL,
        Capability::AuthorizePiCreateSession,
        generation,
        CommandBody::AuthorizePiCreateSession {
            native_child_id: child,
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
            native_child_id: child,
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
            native_child_id: child,
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

/// Builds the idle, quiesced parent boundary which a physical Dispose write
/// must authorize before it touches the peer. The returned generation is the
/// one frozen into that authorization: late delivery evidence may use it even
/// if a later cancellation advances the cycle generation.
fn authorized_dispose_session(
    store: &mut KernelStore,
    label: &str,
    reservation_amount: UsdMicros,
) -> (
    PrincipalId,
    OperatingCycleId,
    RootAuthorityOfficeSessionId,
    ExpectedGeneration,
    PiCorrelationIdentity,
) {
    let (root_authority, cycle_id) = found_cycle(store);
    let initial = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let session_id = RootAuthorityOfficeSessionId::new(1).unwrap();
    accepted(
        store,
        &format!("{label}-start-office-session"),
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        initial,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        store,
        root_authority,
        cycle_id,
        session_id,
        label,
        reservation_amount,
    );
    accepted(
        store,
        &format!("{label}-quiesce-before-dispose"),
        root_authority,
        Capability::QuiesceOperatingCycle,
        initial,
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    let authorized_generation =
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let correlation = PiCorrelationIdentity::parse(format!("{label}-dispose-correlation")).unwrap();
    accepted(
        store,
        &format!("{label}-authorize-dispose"),
        PrincipalId::KERNEL,
        Capability::AuthorizePiOfficeSessionDispose,
        authorized_generation,
        CommandBody::AuthorizePiOfficeSessionDispose {
            session_id,
            correlation_identity: correlation.clone(),
        },
    );
    (
        root_authority,
        cycle_id,
        session_id,
        authorized_generation,
        correlation,
    )
}

#[test]
fn founding_cycle_is_idempotent_fenced_and_replayable() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (root_authority, cycle_id) = found_cycle(&mut store);

    let first = submit(
        &mut store,
        "root-authority-quiesce-generation-zero",
        root_authority,
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    let repeat = submit(
        &mut store,
        "root-authority-quiesce-generation-zero",
        root_authority,
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    assert_eq!(repeat.disposition, first.disposition);
    assert!(repeat.idempotent);

    rejected(
        &mut store,
        "root-authority-resume-stale-generation-zero",
        root_authority,
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
        "root-authority-resume-generation-one",
        root_authority,
        Capability::ResumeOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap()),
        CommandBody::ResumeOperatingCycle { cycle_id },
    );
    accepted(
        &mut store,
        "root-authority-quiesce-generation-one",
        root_authority,
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
        "root-authority-reconcile-cycle",
        root_authority,
        Capability::ReconcileOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(2).unwrap()),
        CommandBody::ReconcileOperatingCycle { cycle_id },
    );
    accepted(
        &mut store,
        "root-authority-close-cycle",
        root_authority,
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
    assert_eq!(store.command_count().unwrap(), 18);
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    accepted(
        &mut store,
        "coord-start-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        generation,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "coord-create-project",
        root_authority,
        Capability::CreateProject,
        generation,
        CommandBody::CreateProject {
            operating_cycle_id: cycle_id,
            project_name: ProjectName::parse("coordination spine").unwrap(),
            north_star_alignment: example_project_north_star_alignment(),
        },
    );
    let project_id = ProjectId::new(1).unwrap();
    rejected(
        &mut store,
        "coord-charter-proposed-rejected",
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
            root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
            author_principal_id: root_authority,
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
            reviewer_principal_id: root_authority,
            reviewer_actor_instance_id: society_kernel::ActorInstanceId::new(1).unwrap(),
            reviewer_actor_attempt_id: society_kernel::ActorAttemptId::new(1).unwrap(),
        },
        Rejection::ReviewAssignmentNotIndependent,
    );
    accepted(
        &mut store,
        "coord-observe-project",
        root_authority,
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
        root_authority,
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
        root_authority,
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
        root_authority,
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
fn application_mission_alignment_is_founding_mission_bound_and_replay_verified() {
    let path = std::env::temp_dir().join(format!(
        "society-application-mission-alignment-{}-{}.sqlite",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut store = KernelStore::open(&path).unwrap();
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let generation = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    accepted(
        &mut store,
        "mission-start-office-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        generation,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );

    let mut stale_alignment = example_project_north_star_alignment();
    stale_alignment.application_revision_id = ApplicationRevisionId::new(2).unwrap();
    rejected(
        &mut store,
        "mission-project-stale-revision",
        root_authority,
        Capability::CreateProject,
        generation,
        CommandBody::CreateProject {
            operating_cycle_id: cycle_id,
            project_name: ProjectName::parse("stale alignment project").unwrap(),
            north_star_alignment: stale_alignment,
        },
        Rejection::ProjectNorthStarAlignmentMismatch,
    );
    accepted(
        &mut store,
        "mission-project-exact-alignment",
        root_authority,
        Capability::CreateProject,
        generation,
        CommandBody::CreateProject {
            operating_cycle_id: cycle_id,
            project_name: ProjectName::parse("exact alignment project").unwrap(),
            north_star_alignment: example_project_north_star_alignment(),
        },
    );
    drop(store);
    let inspection = Connection::open(&path).unwrap();
    let application_row: (String, String, i64, String, i64) = inspection
        .query_row(
            "SELECT a.application_identity, a.application_name, r.revision_ordinal,
                    r.mission_statement, COUNT(p.principle_ordinal)
             FROM applications a
             JOIN application_revisions r ON r.application_id = a.application_id
             JOIN application_revision_principles p
               ON p.application_revision_id = r.application_revision_id
             GROUP BY a.application_id, r.application_revision_id",
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
    assert_eq!(application_row.0, "example-application");
    assert_eq!(application_row.1, "Example Application");
    assert_eq!(application_row.2, 1);
    assert_eq!(
        application_row.3,
        "Improve a bounded example system responsibly."
    );
    assert_eq!(application_row.4, 3);
    let alignment: (i64, String, String, String, String) = inspection
        .query_row(
            "SELECT application_revision_id, change_answer, improvement_evidence_answer,
                    boundary_commitment_answer, revisit_answer
             FROM project_north_star_alignments WHERE project_id = 1",
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
    assert_eq!(alignment.0, 1);
    assert_eq!(alignment.1, "Deliver one bounded improvement.");
    assert_eq!(alignment.2, "Use a reproducible integration judge.");
    assert_eq!(alignment.3, "Do not expand authority outside the fixture.");
    assert_eq!(
        alignment.4,
        "Revisit after the next durable evidence review."
    );
    drop(inspection);

    assert!(KernelStore::open(&path).unwrap().replay_ledger().is_ok());

    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE command_install_founding_mission_principles
             SET principle_text = 'tampered mission principle'
             WHERE command_row_id = (
                 SELECT command_row_id FROM commands WHERE command_id = 'found-install-founding-mission'
             ) AND principle_ordinal = 1",
            [],
        )
        .unwrap();
    drop(connection);
    assert!(KernelStore::open(&path).unwrap().replay_ledger().is_err());

    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE command_install_founding_mission_principles
             SET principle_text = 'Pursue a clear public purpose.'
             WHERE command_row_id = (
                 SELECT command_row_id FROM commands WHERE command_id = 'found-install-founding-mission'
             ) AND principle_ordinal = 1",
            [],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE project_north_star_alignments
             SET change_answer = 'forged material alignment' WHERE project_id = 1",
            [],
        )
        .unwrap();
    drop(connection);
    let tampered = KernelStore::open(&path).unwrap();
    assert!(tampered.validate_replayed_materialized_state().is_err());
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
        27
    );
    drop(connection);
    drop(KernelStore::open(&path).unwrap());
    let reopened = Connection::open(&path).unwrap();
    assert_eq!(
        reopened
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        27
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
fn historical_schema_current_minus_one_is_rejected_without_current_schema_mutation() {
    let path = std::env::temp_dir().join(format!(
        "society-historical-schema-twelve-{}-{}.sqlite",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let historical = Connection::open(&path).unwrap();
    // Schema twelve was the immediately preceding fresh-only identity. Schema
    // thirteen must not mistake its durable Dispose shape for current data.
    historical
        .execute_batch(
            "CREATE TABLE previous_v12_ledger_marker (entry_id INTEGER PRIMARY KEY);
             INSERT INTO previous_v12_ledger_marker VALUES (1);
             PRAGMA user_version = 12;",
        )
        .unwrap();
    drop(historical);

    assert!(matches!(
        KernelStore::open(&path),
        Err(StoreError::UnsupportedSchemaVersion(12))
    ));
    let inspection = Connection::open(&path).unwrap();
    assert_eq!(
        inspection
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        12
    );
    assert_eq!(
        inspection
            .query_row(
                "SELECT COUNT(*) FROM previous_v12_ledger_marker",
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
fn founding_budget_policy_is_explicit_per_closed_treatment() {
    let society_hard_ceiling = UsdMicros::new(42_000).unwrap();
    let cases = [
        (
            "qualification",
            OperatingCycleTreatment::PiSdkQualificationV1,
            UsdMicros::new(7_000).unwrap(),
        ),
        (
            "live",
            OperatingCycleTreatment::PinnedPiSdkLiveV1,
            UsdMicros::new(19_000).unwrap(),
        ),
        (
            "deterministic",
            OperatingCycleTreatment::DeterministicPiHostFixtureV1,
            UsdMicros::new(31_000).unwrap(),
        ),
    ];

    for (case, treatment, budget_ceiling) in cases {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "society-generic-budget-policy-{case}-{unique}.sqlite3"
        ));
        let mut store = KernelStore::open(&path).unwrap();
        let bootstrap = PrincipalId::BOOTSTRAP;
        accepted(
            &mut store,
            "policy-create-society",
            bootstrap,
            Capability::CreateSocietyIdentity,
            ExpectedGeneration::NotApplicable,
            CommandBody::CreateSocietyIdentity {
                name: SocietyName::parse("Synthetic budget society").unwrap(),
            },
        );
        let mission = example_application_mission();
        seal_and_register_mission_source(&mut store, "policy", &mission);
        accepted(
            &mut store,
            "policy-install-founding-mission",
            bootstrap,
            Capability::InstallFoundingMission,
            ExpectedGeneration::NotApplicable,
            CommandBody::InstallFoundingMission { mission },
        );
        accepted(
            &mut store,
            "policy-install-office",
            bootstrap,
            Capability::InstallRootAuthorityOffice,
            ExpectedGeneration::NotApplicable,
            CommandBody::InstallRootAuthorityOffice,
        );
        accepted(
            &mut store,
            "policy-appoint-root-authority",
            bootstrap,
            Capability::AppointInitialRootAuthority,
            ExpectedGeneration::NotApplicable,
            CommandBody::AppointInitialRootAuthority {
                actor_display_name: PrincipalDisplayName::parse("synthetic root_authority")
                    .unwrap(),
            },
        );
        rejected(
            &mut store,
            "policy-reject-zero-r0-ceiling",
            bootstrap,
            Capability::SetR0HardCeiling,
            ExpectedGeneration::NotApplicable,
            CommandBody::SetR0HardCeiling {
                ceiling: UsdMicros::ZERO,
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
                ceiling: society_hard_ceiling,
            },
        );
        assert_eq!(
            store
                .active_capability_grant(bootstrap, Capability::SetR0HardCeiling)
                .unwrap(),
            None,
            "the accepted founding ceiling consumes its one-shot capability"
        );
        accepted(
            &mut store,
            "policy-bootstrap",
            bootstrap,
            Capability::BootstrapSociety,
            ExpectedGeneration::NotApplicable,
            CommandBody::BootstrapSociety,
        );
        rejected(
            &mut store,
            "policy-reject-over-society-cycle-ceiling",
            bootstrap,
            Capability::ProposeOperatingCycle,
            ExpectedGeneration::NotApplicable,
            CommandBody::ProposeOperatingCycle {
                treatment,
                budget_ceiling: UsdMicros::new(42_001).unwrap(),
            },
            Rejection::BudgetPolicyViolation,
        );
        accepted(
            &mut store,
            "policy-propose-explicit-cycle-ceiling",
            bootstrap,
            Capability::ProposeOperatingCycle,
            ExpectedGeneration::NotApplicable,
            CommandBody::ProposeOperatingCycle {
                treatment,
                budget_ceiling,
            },
        );

        let cycle_id = OperatingCycleId::new(1).unwrap();
        assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
            event.body,
            EventBody::OperatingCycleProposed {
                cycle_id: event_cycle_id,
                treatment: event_treatment,
                budget_ceiling: event_budget_ceiling,
                ..
            } if event_cycle_id == cycle_id
                && event_treatment == treatment
                && event_budget_ceiling == budget_ceiling
        )));
        assert!(store.validate_replayed_materialized_state().is_ok());
        drop(store);

        let inspection = Connection::open(&path).unwrap();
        let material: (i64, i64, i64) = inspection
            .query_row(
                "SELECT c.treatment, c.budget_ceiling_micros, e.ceiling_micros
                 FROM operating_cycles c
                 JOIN budget_envelope_constraints b
                   ON b.operating_cycle_id = c.operating_cycle_id
                 JOIN budget_envelopes e ON e.budget_envelope_id = b.budget_envelope_id
                 WHERE c.operating_cycle_id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let command: (i64, i64) = inspection
            .query_row(
                "SELECT proposal.treatment, proposal.budget_ceiling_micros
                 FROM command_propose_operating_cycle proposal
                 JOIN commands command
                   ON command.command_row_id = proposal.command_row_id
                 WHERE command.command_id = 'policy-propose-explicit-cycle-ceiling'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let event: (i64, i64) = inspection
            .query_row(
                "SELECT treatment, budget_ceiling_micros
                 FROM event_operating_cycle_proposed",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let expected = (treatment as i64, budget_ceiling.value());
        assert_eq!(material, (expected.0, expected.1, expected.1));
        assert_eq!(command, expected);
        assert_eq!(event, expected);

        inspection
            .execute(
                "UPDATE command_propose_operating_cycle
                 SET budget_ceiling_micros = ?1
                 WHERE command_row_id = (
                    SELECT command_row_id FROM commands
                    WHERE command_id = 'policy-propose-explicit-cycle-ceiling'
                 )",
                [budget_ceiling.value() + 1],
            )
            .unwrap();
        drop(inspection);
        assert!(matches!(
            KernelStore::open(&path).unwrap().replay_ledger(),
            Err(StoreError::LedgerCorruption(_))
        ));

        let inspection = Connection::open(&path).unwrap();
        inspection
            .execute(
                "UPDATE command_propose_operating_cycle
                 SET budget_ceiling_micros = ?1
                 WHERE command_row_id = (
                    SELECT command_row_id FROM commands
                    WHERE command_id = 'policy-propose-explicit-cycle-ceiling'
                 )",
                [budget_ceiling.value()],
            )
            .unwrap();
        inspection
            .execute(
                "UPDATE event_operating_cycle_proposed
                 SET budget_ceiling_micros = ?1",
                [budget_ceiling.value() + 1],
            )
            .unwrap();
        drop(inspection);
        assert!(matches!(
            KernelStore::open(&path).unwrap().replay_ledger(),
            Err(StoreError::LedgerCorruption(_))
        ));

        let inspection = Connection::open(&path).unwrap();
        inspection
            .execute(
                "UPDATE event_operating_cycle_proposed
                 SET budget_ceiling_micros = ?1",
                [budget_ceiling.value()],
            )
            .unwrap();
        inspection
            .execute(
                "UPDATE operating_cycles SET budget_ceiling_micros = ?1",
                [budget_ceiling.value() + 1],
            )
            .unwrap();
        drop(inspection);
        assert!(matches!(
            KernelStore::open(&path)
                .unwrap()
                .validate_replayed_materialized_state(),
            Err(StoreError::LedgerCorruption(_))
        ));
        fs::remove_file(path).unwrap();
    }
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let session_id = RootAuthorityOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "root-authority-start-office-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        zero,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        &mut store,
        root_authority,
        cycle_id,
        session_id,
        "office-turn",
        UsdMicros::new(1).unwrap(),
    );
    accepted(
        &mut store,
        "root-authority-open-office-turn",
        root_authority,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
    );
    rejected(
        &mut store,
        "root-authority-open-concurrent-office-turn",
        root_authority,
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
        "root-authority-reject-zero-budget-reservation",
        root_authority,
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
        "root-authority-reserve-six-hundred-thousand",
        root_authority,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(600_000).unwrap(),
        },
    );
    rejected(
        &mut store,
        "root-authority-reject-cycle-cap-overrun",
        root_authority,
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
        "root-authority-reserve-remaining-seven-hundred-thousand",
        root_authority,
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
        "root-authority-reserve-after-unknown-cost",
        root_authority,
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
            .command_receipt(
                &CommandId::parse("root-authority-reserve-after-unknown-cost").unwrap()
            )
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);

    accepted(
        &mut store,
        "root-authority-start-unavailable-cost-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        zero,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "root-authority-reserve-unavailable-cost",
        root_authority,
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());

    accepted(
        &mut store,
        "root-authority-request-quiesce-cancellation",
        root_authority,
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
        "root-authority-cannot-resume-unreconciled-cancellation",
        root_authority,
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
        "root-authority-resume-reconciled-cancellation",
        root_authority,
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = RootAuthorityOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "root-authority-start-purpose-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        zero,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        &mut store,
        root_authority,
        cycle_id,
        session_id,
        "office-purpose",
        UsdMicros::new(1).unwrap(),
    );
    accepted(
        &mut store,
        "root-authority-quiesce-purpose-cycle",
        root_authority,
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
        "root-authority-ordinary-turn-rejected-while-quiescing",
        root_authority,
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
        "root-authority-recovery-turn-while-quiescing",
        root_authority,
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = RootAuthorityOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "root-authority-start-terminal-fence-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        zero,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        &mut store,
        root_authority,
        cycle_id,
        session_id,
        "office-terminal",
        UsdMicros::new(1).unwrap(),
    );
    accepted(
        &mut store,
        "root-authority-open-terminal-fence-turn",
        root_authority,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
    );
    accepted(
        &mut store,
        "root-authority-cancel-terminal-fence-cycle",
        root_authority,
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = RootAuthorityOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "root-authority-start-known-overrun-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        zero,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "root-authority-reserve-known-overrun",
        root_authority,
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
        "root-authority-admission-fenced-by-known-overrun",
        root_authority,
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
        "root-authority-cannot-resume-frozen-known-overrun",
        root_authority,
        Capability::ResumeOperatingCycle,
        one,
        CommandBody::ResumeOperatingCycle { cycle_id },
        Rejection::IncompleteCycleReconciliation,
    );
    accepted(
        &mut store,
        "root-authority-begin-known-overrun-reconciliation",
        root_authority,
        Capability::ReconcileOperatingCycle,
        one,
        CommandBody::ReconcileOperatingCycle { cycle_id },
    );
    rejected(
        &mut store,
        "root-authority-cannot-close-frozen-known-overrun",
        root_authority,
        Capability::CloseOperatingCycle,
        one,
        CommandBody::CloseOperatingCycle { cycle_id },
        Rejection::IncompleteCycleReconciliation,
    );
}

#[test]
fn cost_postmortem_is_the_only_conservative_frozen_cost_resolution() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = RootAuthorityOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "root-authority-start-postmortem-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        zero,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "root-authority-reserve-postmortem-cost",
        root_authority,
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
        "root-authority-cannot-resolve-before-postmortem-is-closable",
        root_authority,
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
        "root-authority-reject-overrun-resolution-for-unknown-cost",
        root_authority,
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
        "root-authority-close-unknown-cost-postmortem",
        root_authority,
        Capability::CloseCostPostmortem,
        one,
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution: CostPostmortemResolution::ConservativeFullReservation,
        },
    );
    accepted(
        &mut store,
        "root-authority-resume-after-conservative-postmortem-resolution",
        root_authority,
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = RootAuthorityOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "root-authority-start-actual-overrun-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        zero,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "root-authority-reserve-actual-overrun",
        root_authority,
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
        "root-authority-close-known-overrun-postmortem",
        root_authority,
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let command_id = CommandId::parse("same-command-id-different-body").unwrap();
    let quiesce_grant = store
        .active_capability_grant(root_authority, Capability::QuiesceOperatingCycle)
        .unwrap()
        .unwrap();
    let first = store
        .execute(CommandRequest {
            command_id: command_id.clone(),
            principal_id: root_authority,
            capability_grant_id: quiesce_grant,
            capability: Capability::QuiesceOperatingCycle,
            expected_generation: ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
            body: CommandBody::QuiesceOperatingCycle { cycle_id },
        })
        .unwrap();
    assert!(matches!(first.disposition, CommandDisposition::Accepted(_)));
    let count_after_first = store.command_count().unwrap();

    let resume_grant = store
        .active_capability_grant(root_authority, Capability::ResumeOperatingCycle)
        .unwrap()
        .unwrap();
    assert!(matches!(
        store.execute(CommandRequest {
            command_id,
            principal_id: root_authority,
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
fn pi_office_session_dispose_orders_peer_facts_and_reconciles_its_parent_budget() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-m7-dispose-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let initial = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let session_id = RootAuthorityOfficeSessionId::new(1).unwrap();
    accepted(
        &mut store,
        "m7-start-office-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        initial,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        &mut store,
        root_authority,
        cycle_id,
        session_id,
        "m7-dispose",
        UsdMicros::new(100).unwrap(),
    );
    let first_prompt_digest = Blake3Digest::of_bytes(b"m7 first durable prompt");
    accepted(
        &mut store,
        "m7-seal-first-prompt",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: first_prompt_digest,
        },
    );
    accepted(
        &mut store,
        "m7-register-first-prompt",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(2).unwrap(),
        },
    );
    let opened = accepted(
        &mut store,
        "m7-open-first-turn",
        root_authority,
        Capability::OpenOfficeTurn,
        initial,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
    );
    let frontier_event_id = match opened.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        other => panic!("unexpected first turn receipt: {other:?}"),
    };
    let first_turn = OfficeTurnId::new(1).unwrap();
    let first_correlation = PiCorrelationIdentity::parse("m7-first-prompt").unwrap();
    accepted(
        &mut store,
        "m7-authorize-first-prompt",
        PrincipalId::KERNEL,
        Capability::AuthorizePiOfficeTurnPrompt,
        initial,
        CommandBody::AuthorizePiOfficeTurnPrompt {
            office_turn_id: first_turn,
            correlation_identity: first_correlation.clone(),
            prompt_content_object_id: society_kernel::ContentObjectId::new(2).unwrap(),
            prompt_digest: first_prompt_digest,
            frontier_event_id,
        },
    );
    accepted(
        &mut store,
        "m7-deliver-first-prompt",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnPromptDelivery,
        initial,
        CommandBody::RecordPiOfficeTurnPromptDelivery {
            office_turn_id: first_turn,
            correlation_identity: first_correlation.clone(),
            prompt_digest: first_prompt_digest,
        },
    );
    accepted(
        &mut store,
        "m7-accept-first-prompt",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnPromptAccepted,
        initial,
        CommandBody::RecordPiOfficeTurnPromptAccepted {
            office_turn_id: first_turn,
            correlation_identity: first_correlation.clone(),
            command_result_sequence: PiProtocolSequence::try_from(10).unwrap(),
        },
    );
    let first_usage = PiCumulativeUsage {
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
        "m7-record-first-usage",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnUsage,
        initial,
        CommandBody::RecordPiOfficeTurnUsage {
            office_turn_id: first_turn,
            correlation_identity: first_correlation.clone(),
            protocol_sequence: PiProtocolSequence::try_from(14).unwrap(),
            usage: first_usage,
        },
    );
    accepted(
        &mut store,
        "m7-record-first-terminal",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnTerminal,
        initial,
        CommandBody::RecordPiOfficeTurnTerminal {
            office_turn_id: first_turn,
            correlation_identity: first_correlation,
            terminal_evidence: PiOfficeTurnTerminalEvidence::ObservedAssistant {
                agent_settled_sequence: PiProtocolSequence::try_from(13).unwrap(),
                final_accounting_sequence: PiProtocolSequence::try_from(14).unwrap(),
            },
            settled_sequence: PiProtocolSequence::try_from(15).unwrap(),
            disposition: PiOfficeTurnDisposition::Completed,
            assistant_outcome: PiOfficeTurnAssistantOutcome::ObservedStop,
            transcript_disposition:
                PiOfficeTurnTranscriptDisposition::DeferredUntilOfficeSessionDispose,
        },
    );
    accepted(
        &mut store,
        "m7-settle-first-turn",
        PrincipalId::KERNEL,
        Capability::SettleOfficeTurn,
        ExpectedGeneration::NotApplicable,
        CommandBody::SettleOfficeTurn {
            turn_id: first_turn,
            terminal_receipt_id: PiOfficeTurnTerminalReceiptId::new(1).unwrap(),
        },
    );
    let transcript_digest = Blake3Digest::of_bytes(b"m7 verified transcript bytes");
    accepted(
        &mut store,
        "m7-seal-transcript",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: transcript_digest,
        },
    );
    accepted(
        &mut store,
        "m7-register-transcript",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(3).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m7-quiesce-before-dispose",
        root_authority,
        Capability::QuiesceOperatingCycle,
        initial,
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    let quiesced = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());

    let correlation = PiCorrelationIdentity::parse("m7-dispose-correlation").unwrap();
    rejected(
        &mut store,
        "m7-reject-stale-dispose-authorization",
        PrincipalId::KERNEL,
        Capability::AuthorizePiOfficeSessionDispose,
        initial,
        CommandBody::AuthorizePiOfficeSessionDispose {
            session_id,
            correlation_identity: PiCorrelationIdentity::parse("m7-stale-dispose").unwrap(),
        },
        Rejection::StaleAdmissionGeneration,
    );
    accepted(
        &mut store,
        "m7-authorize-dispose",
        PrincipalId::KERNEL,
        Capability::AuthorizePiOfficeSessionDispose,
        quiesced,
        CommandBody::AuthorizePiOfficeSessionDispose {
            session_id,
            correlation_identity: correlation.clone(),
        },
    );
    rejected(
        &mut store,
        "m7-authorized-dispose-fences-new-turns",
        root_authority,
        Capability::OpenOfficeTurn,
        quiesced,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
        Rejection::InvalidLifecycleTransition,
    );
    rejected(
        &mut store,
        "m7-reject-accepted-before-delivery",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeAccepted,
        quiesced,
        CommandBody::RecordPiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity: correlation.clone(),
            command_result_sequence: PiProtocolSequence::try_from(20).unwrap(),
        },
        Rejection::PiOfficeSessionDisposeBindingMismatch,
    );
    accepted(
        &mut store,
        "m7-record-dispose-delivery",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeDelivery,
        quiesced,
        CommandBody::RecordPiOfficeSessionDisposeDelivery {
            session_id,
            correlation_identity: correlation.clone(),
        },
    );
    rejected(
        &mut store,
        "m7-reject-wrong-dispose-correlation",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeAccepted,
        quiesced,
        CommandBody::RecordPiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity: PiCorrelationIdentity::parse("m7-wrong-dispose-correlation")
                .unwrap(),
            command_result_sequence: PiProtocolSequence::try_from(20).unwrap(),
        },
        Rejection::PiOfficeSessionDisposeBindingMismatch,
    );
    accepted(
        &mut store,
        "m7-record-dispose-accepted",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeAccepted,
        quiesced,
        CommandBody::RecordPiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity: correlation.clone(),
            command_result_sequence: PiProtocolSequence::try_from(20).unwrap(),
        },
    );
    let final_usage = PiCumulativeUsage {
        input_tokens: PiTokenCount::try_from(1).unwrap(),
        output_tokens: PiTokenCount::try_from(1).unwrap(),
        cache_read_tokens: PiTokenCount::try_from(1).unwrap(),
        cache_write_tokens: PiTokenCount::try_from(1).unwrap(),
        total_tokens: PiTokenCount::try_from(4).unwrap(),
        provider_cost: ProviderCostBinary64::from_big_endian(0.0000085_f64.to_bits().to_be_bytes())
            .unwrap(),
        ceiling_micro_usd: UsdMicros::new(9).unwrap(),
    };
    rejected(
        &mut store,
        "m7-reject-final-usage-below-charged-checkpoint",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeUsage,
        quiesced,
        CommandBody::RecordPiOfficeSessionDisposeUsage {
            session_id,
            correlation_identity: correlation.clone(),
            protocol_sequence: PiProtocolSequence::try_from(21).unwrap(),
            usage: PiCumulativeUsage {
                input_tokens: PiTokenCount::try_from(1).unwrap(),
                output_tokens: PiTokenCount::try_from(1).unwrap(),
                cache_read_tokens: PiTokenCount::try_from(1).unwrap(),
                cache_write_tokens: PiTokenCount::try_from(1).unwrap(),
                total_tokens: PiTokenCount::try_from(4).unwrap(),
                provider_cost: ProviderCostBinary64::from_big_endian(
                    0.0000025_f64.to_bits().to_be_bytes(),
                )
                .unwrap(),
                ceiling_micro_usd: UsdMicros::new(3).unwrap(),
            },
        },
        Rejection::PiOfficeSessionDisposeUsageNotMonotonic,
    );
    accepted(
        &mut store,
        "m7-record-final-known-usage",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeUsage,
        quiesced,
        CommandBody::RecordPiOfficeSessionDisposeUsage {
            session_id,
            correlation_identity: correlation.clone(),
            protocol_sequence: PiProtocolSequence::try_from(21).unwrap(),
            usage: final_usage,
        },
    );
    rejected(
        &mut store,
        "m7-reject-nonmonotonic-dispose-sequence",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposed,
        quiesced,
        CommandBody::RecordPiOfficeSessionDisposed {
            session_id,
            correlation_identity: correlation.clone(),
            disposed_sequence: PiProtocolSequence::try_from(21).unwrap(),
            transcript_receipt: PiOfficeSessionTranscriptReceipt::Materialized {
                session_file: CanonicalPiSessionTranscriptPath::parse(
                    "/tmp/m7-dispose-session.jsonl",
                )
                .unwrap(),
                session_file_digest: transcript_digest,
                transcript_content_object_id: society_kernel::ContentObjectId::new(3).unwrap(),
                first_user_prompt:
                    society_kernel::PiOfficeSessionFirstUserPromptReceipt::Verified {
                        digest: first_prompt_digest,
                    },
            },
        },
        Rejection::PiOfficeSessionDisposeUsageNotMonotonic,
    );
    rejected(
        &mut store,
        "m7-reject-unmaterialized-dispose-after-prompt",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposed,
        quiesced,
        CommandBody::RecordPiOfficeSessionDisposed {
            session_id,
            correlation_identity: correlation.clone(),
            disposed_sequence: PiProtocolSequence::try_from(22).unwrap(),
            transcript_receipt: PiOfficeSessionTranscriptReceipt::UnmaterializedNoPrompt {
                session_file: CanonicalPiSessionTranscriptPath::parse(
                    "/tmp/m7-dispose-session.jsonl",
                )
                .unwrap(),
            },
        },
        Rejection::PiOfficeSessionDisposeReceiptMissing,
    );
    let disposed_body = CommandBody::RecordPiOfficeSessionDisposed {
        session_id,
        correlation_identity: correlation.clone(),
        disposed_sequence: PiProtocolSequence::try_from(22).unwrap(),
        transcript_receipt: PiOfficeSessionTranscriptReceipt::Materialized {
            session_file: CanonicalPiSessionTranscriptPath::parse("/tmp/m7-dispose-session.jsonl")
                .unwrap(),
            session_file_digest: transcript_digest,
            transcript_content_object_id: society_kernel::ContentObjectId::new(3).unwrap(),
            first_user_prompt: society_kernel::PiOfficeSessionFirstUserPromptReceipt::Verified {
                digest: first_prompt_digest,
            },
        },
    };
    let terminal = accepted(
        &mut store,
        "m7-record-disposed",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposed,
        quiesced,
        disposed_body.clone(),
    );
    let duplicate = submit(
        &mut store,
        "m7-record-disposed",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposed,
        quiesced,
        disposed_body,
    );
    assert_eq!(duplicate.disposition, terminal.disposition);
    assert!(duplicate.idempotent);
    let terminal_event = match terminal.disposition {
        CommandDisposition::Accepted(event_id) => store.ledger_event(event_id).unwrap(),
        other => panic!("unexpected Dispose terminal receipt: {other:?}"),
    };
    assert!(matches!(
        terminal_event.body,
        EventBody::PiOfficeSessionDisposed {
            session_id: observed_session,
            observed_cumulative_micro_usd,
            ..
        } if observed_session == session_id && observed_cumulative_micro_usd == UsdMicros::new(9).unwrap()
    ));
    rejected(
        &mut store,
        "m7-dispose-terminal-does-not-pretend-child-reaped-or-workspace-disposed",
        PrincipalId::KERNEL,
        Capability::RecordCycleDrained,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordCycleDrained { cycle_id },
        Rejection::InvalidLifecycleTransition,
    );
    drop(store);

    let connection = Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT reservation_state, charged_micros FROM budget_reservations WHERE budget_reservation_id = 1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .unwrap(),
        (2, 9),
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT lifecycle_state FROM root_authority_office_sessions WHERE root_authority_office_session_id = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        8,
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT envelope.reserved_micros, envelope.spent_micros
                 FROM budget_envelopes envelope
                 JOIN budget_envelope_constraints budget_constraint
                   ON budget_constraint.budget_envelope_id = envelope.budget_envelope_id
                 WHERE budget_constraint.operating_cycle_id = ?1",
                [cycle_id.value()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .unwrap(),
        (0, 9),
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COALESCE(SUM(amount_micros), 0)
                 FROM budget_reservation_charges WHERE budget_reservation_id = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        0,
        "the parent reservation released its remaining 91 micros"
    );
    let collision = connection
        .execute(
            "INSERT INTO pi_office_turn_usage_receipts(
                 office_turn_id, pi_office_turn_prompt_authorization_id,
                 pi_session_id, correlation_identity, protocol_sequence,
                 input_tokens, output_tokens, cache_read_tokens,
                 cache_write_tokens, total_tokens, provider_cost_binary64,
                 cumulative_ceiling_micros, recorded_by_command_id
             ) VALUES (
                 1, 1, 1, 'm7-first-prompt', 21,
                 0, 0, 0, 0, 0, X'0000000000000000', 0,
                 (SELECT command_row_id FROM commands
                  WHERE command_id = 'm7-record-final-known-usage')
             )",
            [],
        )
        .unwrap_err();
    assert!(
        collision
            .to_string()
            .contains("Pi usage/failure sequence collision"),
        "raw SQL may not insert a turn Known fact at the Dispose Known sequence: {collision}"
    );
    drop(connection);

    let replayed = KernelStore::open(&path).unwrap();
    replayed.replay_ledger().unwrap();
    drop(replayed);
    let tampering = Connection::open(&path).unwrap();
    tampering
        .execute(
            "UPDATE command_record_pi_office_session_dispose_accepted
             SET command_result_sequence = 19
             WHERE command_row_id = (
                SELECT command_row_id FROM commands
                WHERE command_id = 'm7-record-dispose-accepted'
             )",
            [],
        )
        .unwrap();
    drop(tampering);
    let tampered = KernelStore::open(&path).unwrap();
    assert!(matches!(
        tampered.replay_ledger(),
        Err(StoreError::LedgerCorruption(
            "command request fingerprint mismatch"
        ))
    ));
    drop(tampered);
    fs::remove_file(path).unwrap();
}

#[test]
fn pi_office_session_dispose_usage_failure_freezes_without_fabricating_terminal_closure() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (_root_authority, _cycle_id, session_id, authorized_generation, correlation) =
        authorized_dispose_session(
            &mut store,
            "m7-dispose-failure",
            UsdMicros::new(100).unwrap(),
        );
    accepted(
        &mut store,
        "m7-dispose-failure-delivery",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeDelivery,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeDelivery {
            session_id,
            correlation_identity: correlation.clone(),
        },
    );
    accepted(
        &mut store,
        "m7-dispose-failure-accepted",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeAccepted,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity: correlation.clone(),
            command_result_sequence: PiProtocolSequence::try_from(4).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m7-dispose-failure-unavailable",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeUsageFailure,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeUsageFailure {
            session_id,
            correlation_identity: correlation.clone(),
            protocol_sequence: PiProtocolSequence::try_from(5).unwrap(),
            failure: PiOfficeTurnUsageFailure::Unavailable(
                PiOfficeTurnUsageUnavailableReason::InvalidSdkUsage,
            ),
        },
    );
    rejected(
        &mut store,
        "m7-dispose-failure-reject-terminal-after-fatal-peer",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposed,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposed {
            session_id,
            correlation_identity: correlation,
            disposed_sequence: PiProtocolSequence::try_from(6).unwrap(),
            transcript_receipt: PiOfficeSessionTranscriptReceipt::UnmaterializedNoPrompt {
                session_file: CanonicalPiSessionTranscriptPath::parse(
                    "/tmp/m7-dispose-failure.jsonl",
                )
                .unwrap(),
            },
        },
        Rejection::PiOfficeSessionDisposeReceiptMissing,
    );
    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::PiOfficeSessionDisposeUsageFrozen {
            session_id: observed_session,
            failure: PiOfficeTurnUsageFailure::Unavailable(
                PiOfficeTurnUsageUnavailableReason::InvalidSdkUsage,
            ),
            ..
        } if observed_session == session_id
    )));
}

#[test]
fn pi_office_session_dispose_accepts_a_materialized_no_prompt_transcript_with_absent_receipt() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "society-m7-materialized-no-prompt-{unique}.sqlite3"
    ));
    let mut store = KernelStore::open(&path).unwrap();
    let (_root_authority, _cycle_id, session_id, authorized_generation, correlation) =
        authorized_dispose_session(
            &mut store,
            "m7-dispose-materialized-no-prompt",
            UsdMicros::new(100).unwrap(),
        );
    let transcript_digest = Blake3Digest::of_bytes(b"m7 materialized no-prompt transcript");
    accepted(
        &mut store,
        "m7-dispose-materialized-no-prompt-seal",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordContentSealReceipt {
            digest: transcript_digest,
        },
    );
    accepted(
        &mut store,
        "m7-dispose-materialized-no-prompt-register",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        ExpectedGeneration::NotApplicable,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(2).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m7-dispose-materialized-no-prompt-delivery",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeDelivery,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeDelivery {
            session_id,
            correlation_identity: correlation.clone(),
        },
    );
    accepted(
        &mut store,
        "m7-dispose-materialized-no-prompt-accepted",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeAccepted,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity: correlation.clone(),
            command_result_sequence: PiProtocolSequence::try_from(4).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m7-dispose-materialized-no-prompt-known-usage",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeUsage,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeUsage {
            session_id,
            correlation_identity: correlation.clone(),
            protocol_sequence: PiProtocolSequence::try_from(5).unwrap(),
            usage: PiCumulativeUsage {
                input_tokens: PiTokenCount::try_from(0).unwrap(),
                output_tokens: PiTokenCount::try_from(0).unwrap(),
                cache_read_tokens: PiTokenCount::try_from(0).unwrap(),
                cache_write_tokens: PiTokenCount::try_from(0).unwrap(),
                total_tokens: PiTokenCount::try_from(0).unwrap(),
                provider_cost: ProviderCostBinary64::from_big_endian(0_f64.to_bits().to_be_bytes())
                    .unwrap(),
                ceiling_micro_usd: UsdMicros::ZERO,
            },
        },
    );
    let materialized = |first_user_prompt| CommandBody::RecordPiOfficeSessionDisposed {
        session_id,
        correlation_identity: correlation.clone(),
        disposed_sequence: PiProtocolSequence::try_from(6).unwrap(),
        transcript_receipt: PiOfficeSessionTranscriptReceipt::Materialized {
            session_file: CanonicalPiSessionTranscriptPath::parse(
                "/tmp/m7-materialized-no-prompt.jsonl",
            )
            .unwrap(),
            session_file_digest: transcript_digest,
            transcript_content_object_id: society_kernel::ContentObjectId::new(2).unwrap(),
            first_user_prompt,
        },
    };
    rejected(
        &mut store,
        "m7-dispose-materialized-no-prompt-reject-verified",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposed,
        authorized_generation,
        materialized(
            society_kernel::PiOfficeSessionFirstUserPromptReceipt::Verified {
                digest: transcript_digest,
            },
        ),
        Rejection::PiOfficeSessionDisposeReceiptMissing,
    );
    accepted(
        &mut store,
        "m7-dispose-materialized-no-prompt-absent",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposed,
        authorized_generation,
        materialized(society_kernel::PiOfficeSessionFirstUserPromptReceipt::Absent),
    );
    assert!(store.replay_ledger().is_ok());
    assert!(store.validate_replayed_materialized_state().is_ok());
    drop(store);

    let tamper = Connection::open(&path).unwrap();
    tamper
        .execute(
            "UPDATE pi_office_session_dispose_receipts
             SET session_file = '/tmp/m7-forged-materialized-no-prompt.jsonl'
             WHERE root_authority_office_session_id = 1",
            [],
        )
        .unwrap();
    drop(tamper);
    let reopened = KernelStore::open(&path).unwrap();
    assert!(reopened.replay_ledger().is_ok());
    assert!(matches!(
        reopened.validate_replayed_materialized_state(),
        Err(StoreError::LedgerCorruption(_))
    ));
    drop(reopened);
    fs::remove_file(path).unwrap();
}

#[test]
fn pi_office_session_dispose_known_overrun_records_terminal_then_freezes_parent() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-m7-dispose-overrun-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (_root_authority, _cycle_id, session_id, authorized_generation, correlation) =
        authorized_dispose_session(
            &mut store,
            "m7-dispose-overrun",
            UsdMicros::new(100).unwrap(),
        );
    accepted(
        &mut store,
        "m7-dispose-overrun-delivery",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeDelivery,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeDelivery {
            session_id,
            correlation_identity: correlation.clone(),
        },
    );
    accepted(
        &mut store,
        "m7-dispose-overrun-accepted",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeAccepted,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity: correlation.clone(),
            command_result_sequence: PiProtocolSequence::try_from(4).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m7-dispose-overrun-known-usage",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeUsage,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeUsage {
            session_id,
            correlation_identity: correlation.clone(),
            protocol_sequence: PiProtocolSequence::try_from(5).unwrap(),
            usage: PiCumulativeUsage {
                input_tokens: PiTokenCount::try_from(0).unwrap(),
                output_tokens: PiTokenCount::try_from(0).unwrap(),
                cache_read_tokens: PiTokenCount::try_from(0).unwrap(),
                cache_write_tokens: PiTokenCount::try_from(0).unwrap(),
                total_tokens: PiTokenCount::try_from(0).unwrap(),
                provider_cost: ProviderCostBinary64::from_big_endian(
                    0.0001005_f64.to_bits().to_be_bytes(),
                )
                .unwrap(),
                ceiling_micro_usd: UsdMicros::new(101).unwrap(),
            },
        },
    );
    let terminal = accepted(
        &mut store,
        "m7-dispose-overrun-terminal",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposed,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposed {
            session_id,
            correlation_identity: correlation,
            disposed_sequence: PiProtocolSequence::try_from(6).unwrap(),
            transcript_receipt: PiOfficeSessionTranscriptReceipt::UnmaterializedNoPrompt {
                session_file: CanonicalPiSessionTranscriptPath::parse(
                    "/tmp/m7-dispose-overrun.jsonl",
                )
                .unwrap(),
            },
        },
    );
    let event_id = match terminal.disposition {
        CommandDisposition::Accepted(event_id) => event_id,
        other => panic!("unexpected known-overrun terminal receipt: {other:?}"),
    };
    assert!(matches!(
        store.ledger_event(event_id).unwrap().body,
        EventBody::PiOfficeSessionDisposed {
            budget_disposition: society_kernel::PiOfficeSessionDisposeBudgetDisposition::Frozen { .. },
            ..
        }
    ));
    drop(store);
    let inspection = Connection::open(&path).unwrap();
    assert_eq!(
        inspection
            .query_row(
                "SELECT reservation_state FROM budget_reservations WHERE budget_reservation_id = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        3
    );
    assert_eq!(
        inspection
            .query_row(
                "SELECT lifecycle_state FROM root_authority_office_sessions WHERE root_authority_office_session_id = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        10
    );
    drop(inspection);
    let replayed = KernelStore::open(&path).unwrap();
    assert!(replayed.replay_ledger().is_ok());
    drop(replayed);
    fs::remove_file(path).unwrap();
}

#[test]
fn pi_office_session_dispose_late_peer_evidence_uses_frozen_authorization_generation() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (root_authority, cycle_id, session_id, authorized_generation, correlation) =
        authorized_dispose_session(&mut store, "m7-dispose-late", UsdMicros::new(100).unwrap());
    accepted(
        &mut store,
        "m7-dispose-late-cancel-after-authorization",
        root_authority,
        Capability::RequestCancellation,
        authorized_generation,
        CommandBody::RequestCancellation {
            cycle_id,
            mode: society_kernel::CancellationMode::GracefulCancel,
        },
    );
    accepted(
        &mut store,
        "m7-dispose-late-delivery",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeDelivery,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeDelivery {
            session_id,
            correlation_identity: correlation.clone(),
        },
    );
    accepted(
        &mut store,
        "m7-dispose-late-accepted",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeAccepted,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeAccepted {
            session_id,
            correlation_identity: correlation.clone(),
            command_result_sequence: PiProtocolSequence::try_from(4).unwrap(),
        },
    );
    accepted(
        &mut store,
        "m7-dispose-late-known-usage",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposeUsage,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposeUsage {
            session_id,
            correlation_identity: correlation.clone(),
            protocol_sequence: PiProtocolSequence::try_from(5).unwrap(),
            usage: PiCumulativeUsage {
                input_tokens: PiTokenCount::try_from(0).unwrap(),
                output_tokens: PiTokenCount::try_from(0).unwrap(),
                cache_read_tokens: PiTokenCount::try_from(0).unwrap(),
                cache_write_tokens: PiTokenCount::try_from(0).unwrap(),
                total_tokens: PiTokenCount::try_from(0).unwrap(),
                provider_cost: ProviderCostBinary64::from_big_endian(0_f64.to_bits().to_be_bytes())
                    .unwrap(),
                ceiling_micro_usd: UsdMicros::ZERO,
            },
        },
    );
    accepted(
        &mut store,
        "m7-dispose-late-terminal",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeSessionDisposed,
        authorized_generation,
        CommandBody::RecordPiOfficeSessionDisposed {
            session_id,
            correlation_identity: correlation,
            disposed_sequence: PiProtocolSequence::try_from(6).unwrap(),
            transcript_receipt: PiOfficeSessionTranscriptReceipt::UnmaterializedNoPrompt {
                session_file: CanonicalPiSessionTranscriptPath::parse("/tmp/m7-dispose-late.jsonl")
                    .unwrap(),
            },
        },
    );
    assert!(store.replay_ledger().is_ok());
}

#[test]
fn pi_office_turn_requires_authorized_delivered_peer_terminal_chain_and_debits_only_delta() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("society-m6-office-turn-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let session_id = RootAuthorityOfficeSessionId::new(1).unwrap();
    accepted(
        &mut store,
        "m6-start-office-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        zero,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        &mut store,
        root_authority,
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
    let prompt_digest = Blake3Digest::of_bytes(b"sealed deterministic prompt");
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
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(2).unwrap(),
        },
    );

    let open = accepted(
        &mut store,
        "m6-open-turn-one",
        root_authority,
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
            prompt_content_object_id: society_kernel::ContentObjectId::new(2).unwrap(),
            prompt_digest: Blake3Digest::of_bytes(b"recombined"),
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
            prompt_content_object_id: society_kernel::ContentObjectId::new(2).unwrap(),
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
            terminal_evidence: PiOfficeTurnTerminalEvidence::ObservedAssistant {
                agent_settled_sequence: PiProtocolSequence::try_from(10).unwrap(),
                final_accounting_sequence: PiProtocolSequence::try_from(14).unwrap(),
            },
            settled_sequence: PiProtocolSequence::try_from(15).unwrap(),
            disposition: PiOfficeTurnDisposition::Completed,
            assistant_outcome: PiOfficeTurnAssistantOutcome::ObservedStop,
            transcript_disposition:
                PiOfficeTurnTranscriptDisposition::DeferredUntilOfficeSessionDispose,
        },
        Rejection::PiOfficeTurnTerminalEvidenceMissing,
    );
    rejected(
        &mut store,
        "m6-reject-unavailable-terminal-shape-for-observed-assistant",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnTerminal,
        zero,
        CommandBody::RecordPiOfficeTurnTerminal {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            terminal_evidence: PiOfficeTurnTerminalEvidence::UnavailableAssistant {
                final_known_usage_sequence: PiProtocolSequence::try_from(14).unwrap(),
            },
            settled_sequence: PiProtocolSequence::try_from(15).unwrap(),
            disposition: PiOfficeTurnDisposition::Completed,
            assistant_outcome: PiOfficeTurnAssistantOutcome::ObservedStop,
            transcript_disposition:
                PiOfficeTurnTranscriptDisposition::DeferredUntilOfficeSessionDispose,
        },
        Rejection::PiOfficeTurnTerminalEvidenceMissing,
    );
    rejected(
        &mut store,
        "m6-reject-nonordered-observed-terminal-shape",
        PrincipalId::KERNEL,
        Capability::RecordPiOfficeTurnTerminal,
        zero,
        CommandBody::RecordPiOfficeTurnTerminal {
            office_turn_id: turn_one,
            correlation_identity: correlation_one.clone(),
            terminal_evidence: PiOfficeTurnTerminalEvidence::ObservedAssistant {
                agent_settled_sequence: PiProtocolSequence::try_from(14).unwrap(),
                final_accounting_sequence: PiProtocolSequence::try_from(14).unwrap(),
            },
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
            terminal_evidence: PiOfficeTurnTerminalEvidence::ObservedAssistant {
                agent_settled_sequence: PiProtocolSequence::try_from(11).unwrap(),
                final_accounting_sequence: PiProtocolSequence::try_from(14).unwrap(),
            },
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
        root_authority,
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
            prompt_content_object_id: society_kernel::ContentObjectId::new(2).unwrap(),
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
            terminal_evidence: PiOfficeTurnTerminalEvidence::ObservedAssistant {
                agent_settled_sequence: PiProtocolSequence::try_from(21).unwrap(),
                final_accounting_sequence: PiProtocolSequence::try_from(22).unwrap(),
            },
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
        root_authority,
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
            prompt_content_object_id: society_kernel::ContentObjectId::new(2).unwrap(),
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
            terminal_evidence: PiOfficeTurnTerminalEvidence::ObservedAssistant {
                agent_settled_sequence: PiProtocolSequence::try_from(31).unwrap(),
                final_accounting_sequence: PiProtocolSequence::try_from(32).unwrap(),
            },
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let session_id = RootAuthorityOfficeSessionId::new(1).unwrap();
    accepted(
        &mut store,
        "m6-race-start-office-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        zero,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    ready_supervised_office_session(
        &mut store,
        root_authority,
        cycle_id,
        session_id,
        "m6-race-office",
        UsdMicros::new(100).unwrap(),
    );
    let digest = Blake3Digest::of_bytes(b"cancellation-race-prompt");
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
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(2).unwrap(),
        },
    );
    let opened = accepted(
        &mut store,
        "m6-race-open-turn",
        root_authority,
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
            prompt_content_object_id: society_kernel::ContentObjectId::new(2).unwrap(),
            prompt_digest: digest,
            frontier_event_id: frontier,
        },
    );
    accepted(
        &mut store,
        "m6-race-cancel-between-authorize-and-delivery",
        root_authority,
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
            terminal_evidence: PiOfficeTurnTerminalEvidence::ObservedAssistant {
                agent_settled_sequence: PiProtocolSequence::try_from(2).unwrap(),
                final_accounting_sequence: PiProtocolSequence::try_from(3).unwrap(),
            },
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);

    accepted(
        &mut store,
        "root-authority-start-duplicate-cost-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        zero,
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "root-authority-reserve-duplicate-cost",
        root_authority,
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    accepted(
        &mut store,
        "root-authority-start-material-replay-session",
        root_authority,
        Capability::StartRootAuthorityOfficeSession,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::StartRootAuthorityOfficeSession { cycle_id },
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
            "UPDATE root_authority_office_sessions SET lifecycle_state = ?1
             WHERE root_authority_office_session_id = 1",
            [11_i64],
        )
        .unwrap();
    assert!(matches!(
        store.validate_replayed_materialized_state(),
        Err(society_kernel::StoreError::LedgerCorruption(_))
    ));
    tamper
        .execute(
            "UPDATE root_authority_office_sessions SET lifecycle_state = 1
             WHERE root_authority_office_session_id = 1",
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
    let (treatment, material_ceiling, envelope_ceiling): (i64, i64, i64) = tamper
        .query_row(
            "SELECT c.treatment, c.budget_ceiling_micros, e.ceiling_micros
             FROM operating_cycles c
             JOIN budget_envelope_constraints b
               ON b.operating_cycle_id = c.operating_cycle_id
             JOIN budget_envelopes e ON e.budget_envelope_id = b.budget_envelope_id
             WHERE c.operating_cycle_id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        (treatment, material_ceiling, envelope_ceiling),
        (
            OperatingCycleTreatment::DeterministicPiHostFixtureV1 as i64,
            1_000_000,
            1_000_000,
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
    let (root_authority, cycle_id) = found_cycle(&mut store);
    let invalid_grant_id = CommandId::parse("root-authority-invalid-capability-grant").unwrap();
    let invalid_grant = store
        .execute(CommandRequest {
            command_id: invalid_grant_id.clone(),
            principal_id: root_authority,
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
        "root-authority-quiesce-absent-cycle",
        root_authority,
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle {
            cycle_id: OperatingCycleId::new(999).unwrap(),
        },
        Rejection::SubjectNotFound,
    );
    assert_eq!(
        store
            .command_receipt(&CommandId::parse("root-authority-quiesce-absent-cycle").unwrap())
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
            "UPDATE event_r0_hard_ceiling_set SET ceiling_micros = ?1 WHERE event_id = 7",
            [1_i64],
        )
        .unwrap();
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    tamper
        .execute(
            "UPDATE event_r0_hard_ceiling_set SET ceiling_micros = ?1 WHERE event_id = 7",
            [1_030_000_i64],
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
            "UPDATE events SET command_row_id = 11 WHERE event_id = 1",
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
            "UPDATE events SET command_row_id = 11 WHERE event_id = 1",
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
