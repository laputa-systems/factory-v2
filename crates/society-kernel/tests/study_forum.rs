#![allow(clippy::unwrap_used)]

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use society_kernel::postgres_db::Connection;
use society_kernel::{
    ApplicationIdentity, ApplicationMissionInput, ApplicationName, ApplicationRevisionId,
    ApplicationRevisionOrdinal, Blake3Digest, Capability, CommandBody, CommandId, CommandRequest,
    ContentIdentityState, ContentObjectId, ExpectedGeneration, ForumMessageBody, ForumMessageKind,
    ForumPostBudget, ForumReadBudget, ForumThreadTitle, KernelStore, MissionPrinciple,
    MissionPrincipleKind, MissionPrincipleText, MissionPrinciples, MissionStatement,
    NorthStarBoundaryCommitmentQuestion, NorthStarChangeQuestion,
    NorthStarImprovementEvidenceQuestion, NorthStarQuestionSet, NorthStarRevisitQuestion,
    PrincipalId, Rejection, StoreError, StudyBudgetUnits, StudyCommand, StudyDecisionBody,
    StudyEpisodeId, StudyEvent, StudyGroundTruthReveal, StudyMeasurementSlot,
    StudyMeasurementSlotCount, StudyMeasurementStatus, StudyPopulationPhase, StudyRoleOrdinal,
    StudyRunLifecycleState, StudyRunPairCount, StudyRunPairOrdinal, StudyRunRegisteredPairCount,
    StudyTransitionDisposition, StudyTreatment, forum_f0_awareness_digest,
    forum_f0_tool_contract_digest,
};

fn application_mission() -> ApplicationMissionInput {
    ApplicationMissionInput {
        application_identity: ApplicationIdentity::parse("study-forum-test").unwrap(),
        application_name: ApplicationName::parse("Study Forum Test").unwrap(),
        revision_ordinal: ApplicationRevisionOrdinal::new(1).unwrap(),
        statement: MissionStatement::parse("Exercise one bounded generic study control plane.")
            .unwrap(),
        principles: MissionPrinciples::new(vec![
            MissionPrinciple {
                kind: MissionPrincipleKind::Purpose,
                text: MissionPrincipleText::parse("Keep the fixture bounded.").unwrap(),
            },
            MissionPrinciple {
                kind: MissionPrincipleKind::Evidence,
                text: MissionPrincipleText::parse("Preserve exact durable evidence.").unwrap(),
            },
            MissionPrinciple {
                kind: MissionPrincipleKind::Boundary,
                text: MissionPrincipleText::parse("Do not grant peer messages authority.").unwrap(),
            },
        ])
        .unwrap(),
        north_star_questions: NorthStarQuestionSet {
            change: NorthStarChangeQuestion::parse("What bounded transition is being tested?")
                .unwrap(),
            improvement_evidence: NorthStarImprovementEvidenceQuestion::parse(
                "What durable comparison would count as evidence?",
            )
            .unwrap(),
            boundary_commitment: NorthStarBoundaryCommitmentQuestion::parse(
                "Which authority boundary remains closed?",
            )
            .unwrap(),
            revisit: NorthStarRevisitQuestion::parse("When should the fixture be revisited?")
                .unwrap(),
        },
        source_rendering_digest: Blake3Digest::of_bytes(b"study-forum-test-mission-v1"),
    }
}

fn execute(
    store: &mut KernelStore,
    command_id: &str,
    principal_id: PrincipalId,
    capability: Capability,
    body: CommandBody,
) {
    let capability_grant_id = store
        .active_capability_grant(principal_id, capability)
        .unwrap()
        .expect("current schema grants this fixture operation");
    let receipt = store
        .execute(CommandRequest {
            command_id: CommandId::parse(command_id).unwrap(),
            principal_id,
            capability_grant_id,
            capability,
            expected_generation: ExpectedGeneration::NotApplicable,
            body,
        })
        .unwrap();
    assert!(matches!(
        receipt.disposition,
        society_kernel::CommandDisposition::Accepted(_)
    ));
}

fn install_application_revision(store: &mut KernelStore) {
    let mission = application_mission();
    execute(
        store,
        "study-create-society",
        PrincipalId::BOOTSTRAP,
        Capability::CreateSocietyIdentity,
        CommandBody::CreateSocietyIdentity {
            name: society_kernel::SocietyName::parse("Study Forum Society").unwrap(),
        },
    );
    execute(
        store,
        "study-seal-mission",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        CommandBody::RecordContentSealReceipt {
            digest: mission.source_rendering_digest,
        },
    );
    execute(
        store,
        "study-register-mission",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(1).unwrap(),
        },
    );
    execute(
        store,
        "study-install-mission",
        PrincipalId::BOOTSTRAP,
        Capability::InstallFoundingMission,
        CommandBody::InstallFoundingMission { mission },
    );
}

fn submit_study(store: &mut KernelStore, ordinal: &mut u16, command: StudyCommand) -> StudyEvent {
    let command_id = CommandId::parse(format!("study-transition-{ordinal}")).unwrap();
    *ordinal += 1;
    let receipt = store.execute_study_transition(command_id, command).unwrap();
    assert!(!receipt.idempotent);
    match receipt.disposition {
        StudyTransitionDisposition::Accepted(event) => event,
        StudyTransitionDisposition::Rejected(rejection) => {
            panic!("study transition unexpectedly rejected: {rejection:?}")
        }
    }
}

fn register_forum_rendering(
    store: &mut KernelStore,
    label: &str,
    rendering: &[u8],
) -> ContentObjectId {
    let digest = Blake3Digest::of_bytes(rendering);
    execute(
        store,
        &format!("{label}-seal"),
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        CommandBody::RecordContentSealReceipt { digest },
    );
    let receipt_id = match store.content_identity_state(digest).unwrap() {
        ContentIdentityState::SealReceiptOnly {
            content_seal_receipt_id,
        } => content_seal_receipt_id,
        state => panic!("unexpected content identity after seal: {state:?}"),
    };
    execute(
        store,
        &format!("{label}-object"),
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: receipt_id,
        },
    );
    match store.content_identity_state(digest).unwrap() {
        ContentIdentityState::Registered {
            content_object_id, ..
        } => content_object_id,
        state => panic!("unexpected content identity after registration: {state:?}"),
    }
}

fn rejected_study(store: &mut KernelStore, ordinal: &mut u16, command: StudyCommand) -> Rejection {
    let command_id = CommandId::parse(format!("study-transition-{ordinal}")).unwrap();
    *ordinal += 1;
    match store
        .execute_study_transition(command_id, command)
        .unwrap()
        .disposition
    {
        StudyTransitionDisposition::Accepted(event) => {
            panic!("study transition unexpectedly accepted: {event:?}")
        }
        StudyTransitionDisposition::Rejected(rejection) => rejection,
    }
}

fn event_id(event: StudyEvent) -> i64 {
    match event {
        StudyEvent::ProtocolRevisionAdmitted {
            protocol_revision_id,
        } => protocol_revision_id.value(),
        StudyEvent::WorldRevisionAdmitted { world_revision_id } => world_revision_id.value(),
        StudyEvent::MeasurementRevisionAdmitted {
            measurement_revision_id,
        } => measurement_revision_id.value(),
        StudyEvent::InstitutionRevisionAdmitted {
            institution_revision_id,
        } => institution_revision_id.value(),
        StudyEvent::PopulationSnapshotAdmitted {
            population_snapshot_id,
        } => population_snapshot_id.value(),
        StudyEvent::EpisodeAdmitted { episode_id } => episode_id.value(),
        StudyEvent::StudyRunStarted { study_run_id } => study_run_id.value(),
        unexpected => panic!("unexpected study admission event: {unexpected:?}"),
    }
}

fn temporary_database_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "society-study-forum-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn forum_body_limits_are_exact_and_no_nul_body_is_admissible() {
    assert!(ForumMessageBody::parse("x".repeat(8_191)).is_ok());
    assert!(ForumMessageBody::parse("x".repeat(8_192)).is_ok());
    assert!(ForumMessageBody::parse("x".repeat(8_193)).is_err());
    assert!(ForumMessageBody::parse("visible\0hidden").is_err());
}

#[test]
fn provider_free_pair_preserves_reset_boundary_and_replays_after_restart() {
    let path = temporary_database_path();
    let mut store = KernelStore::connect_test_path(&path).unwrap();
    install_application_revision(&mut store);
    let mut ordinal = 1_u16;

    let prompt = forum_f0_awareness_digest();
    let tools = forum_f0_tool_contract_digest();
    let ground_truth_reveal = StudyGroundTruthReveal::parse("study-forum-ground-truth-v1").unwrap();
    let protocol = society_kernel::StudyProtocolRevisionId::try_from(event_id(submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitProtocolRevision {
            application_revision_id: ApplicationRevisionId::new(1).unwrap(),
            protocol_digest: Blake3Digest::of_bytes(b"protocol-v1"),
            actor_policy_digest: Blake3Digest::of_bytes(b"actor-policy-v1"),
            forum_prompt_digest: prompt,
            forum_tool_digest: tools,
            evidence_digest: Blake3Digest::of_bytes(b"evidence-v1"),
            ground_truth_commitment_digest: ground_truth_reveal.digest(),
            correction_digest: Blake3Digest::of_bytes(b"correction: proposition one"),
            topology_digest: Blake3Digest::of_bytes(b"topology-v1"),
            episode_budget: StudyBudgetUnits::new(10).unwrap(),
        },
    )))
    .unwrap();
    let world = society_kernel::StudyWorldRevisionId::try_from(event_id(submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitWorldRevision {
            protocol_revision_id: protocol,
            world_digest: Blake3Digest::of_bytes(b"world-v1"),
        },
    )))
    .unwrap();
    let measurement = society_kernel::StudyMeasurementRevisionId::try_from(event_id(submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitMeasurementRevision {
            protocol_revision_id: protocol,
            analysis_digest: Blake3Digest::of_bytes(b"analysis-v1"),
            measurement_slot_count: StudyMeasurementSlotCount::new(3).unwrap(),
        },
    )))
    .unwrap();
    let institution = society_kernel::StudyInstitutionRevisionId::try_from(event_id(submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitInstitutionRevision {
            protocol_revision_id: protocol,
            institution_digest: Blake3Digest::of_bytes(b"institution-v1"),
        },
    )))
    .unwrap();
    let population = society_kernel::StudyPopulationSnapshotId::try_from(event_id(submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitPopulationSnapshot {
            protocol_revision_id: protocol,
            population_digest: Blake3Digest::of_bytes(b"population-v1"),
            population_size: 1,
        },
    )))
    .unwrap();
    let retained_successor_population =
        society_kernel::StudyPopulationSnapshotId::try_from(event_id(submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::AdmitPopulationSnapshot {
                protocol_revision_id: protocol,
                population_digest: Blake3Digest::of_bytes(b"population-v1"),
                population_size: 1,
            },
        )))
        .unwrap();
    let reset_successor_population =
        society_kernel::StudyPopulationSnapshotId::try_from(event_id(submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::AdmitPopulationSnapshot {
                protocol_revision_id: protocol,
                population_digest: Blake3Digest::of_bytes(b"population-v1"),
                population_size: 1,
            },
        )))
        .unwrap();
    let mismatched_successor_population =
        society_kernel::StudyPopulationSnapshotId::try_from(event_id(submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::AdmitPopulationSnapshot {
                protocol_revision_id: protocol,
                population_digest: Blake3Digest::of_bytes(b"population-v2"),
                population_size: 1,
            },
        )))
        .unwrap();
    assert_ne!(population, retained_successor_population);
    assert_ne!(population, reset_successor_population);
    assert_ne!(retained_successor_population, reset_successor_population);
    let randomization = Blake3Digest::of_bytes(b"matched-seed-v1");
    let retained = StudyEpisodeId::try_from(event_id(submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitEpisode {
            protocol_revision_id: protocol,
            world_revision_id: world,
            measurement_revision_id: measurement,
            institution_revision_id: institution,
            population_snapshot_id: population,
            randomization_digest: randomization,
        },
    )))
    .unwrap();
    let reset = StudyEpisodeId::try_from(event_id(submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitEpisode {
            protocol_revision_id: protocol,
            world_revision_id: world,
            measurement_revision_id: measurement,
            institution_revision_id: institution,
            population_snapshot_id: population,
            randomization_digest: randomization,
        },
    )))
    .unwrap();
    let unassigned = StudyEpisodeId::try_from(event_id(submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitEpisode {
            protocol_revision_id: protocol,
            world_revision_id: world,
            measurement_revision_id: measurement,
            institution_revision_id: institution,
            population_snapshot_id: population,
            randomization_digest: randomization,
        },
    )))
    .unwrap();
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::AdmitActorObligation {
                episode_id: unassigned,
                phase: StudyPopulationPhase::Source,
                role: StudyRoleOrdinal::new(1).unwrap(),
                private_view_digest: Blake3Digest::of_bytes(b"unassigned-private-view-v1"),
                prompt_digest: prompt,
                tool_digest: tools,
                budget: StudyBudgetUnits::new(3).unwrap(),
                read_budget: ForumReadBudget::new(3).unwrap(),
                post_budget: ForumPostBudget::new(1).unwrap(),
            },
        ),
        Rejection::InvalidLifecycleTransition
    );
    submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AssignTreatment {
            episode_id: retained,
            treatment: StudyTreatment::Retained,
        },
    );
    submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AssignTreatment {
            episode_id: reset,
            treatment: StudyTreatment::Reset,
        },
    );
    let mismatched_reset = StudyEpisodeId::try_from(event_id(submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitEpisode {
            protocol_revision_id: protocol,
            world_revision_id: world,
            measurement_revision_id: measurement,
            institution_revision_id: institution,
            population_snapshot_id: population,
            randomization_digest: Blake3Digest::of_bytes(b"different-seed-v1"),
        },
    )))
    .unwrap();
    submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AssignTreatment {
            episode_id: mismatched_reset,
            treatment: StudyTreatment::Reset,
        },
    );
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::AdmitMatchedPair {
                retained_episode_id: retained,
                reset_episode_id: mismatched_reset,
            },
        ),
        Rejection::InvalidLifecycleTransition
    );
    let pair = match submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitMatchedPair {
            retained_episode_id: retained,
            reset_episode_id: reset,
        },
    ) {
        StudyEvent::MatchedPairAdmitted { pair_id } => pair_id,
        unexpected => panic!("unexpected event: {unexpected:?}"),
    };

    // A run retains only the sealed plan identity, digest, and finite paired
    // execution set. Its contents remain application-owned immutable bytes.
    let plan_bytes = b"opaque-correction-latency-run-plan-v1";
    let plan_content_object_id = register_forum_rendering(&mut store, "study-run-plan", plan_bytes);
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::AdmitStudyRun {
                protocol_revision_id: protocol,
                plan_content_object_id,
                plan_digest: Blake3Digest::of_bytes(b"wrong-run-plan-digest"),
                pair_count: StudyRunPairCount::new(1).unwrap(),
            },
        ),
        Rejection::InvalidLifecycleTransition
    );
    let study_run_id = match submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitStudyRun {
            protocol_revision_id: protocol,
            plan_content_object_id,
            plan_digest: Blake3Digest::of_bytes(plan_bytes),
            pair_count: StudyRunPairCount::new(1).unwrap(),
        },
    ) {
        StudyEvent::StudyRunAdmitted {
            study_run_id,
            protocol_revision_id,
            plan_content_object_id: observed_plan_content_object_id,
            plan_digest,
            pair_count,
        } => {
            assert_eq!(protocol_revision_id, protocol);
            assert_eq!(observed_plan_content_object_id, plan_content_object_id);
            assert_eq!(plan_digest, Blake3Digest::of_bytes(plan_bytes));
            assert_eq!(pair_count, StudyRunPairCount::new(1).unwrap());
            study_run_id
        }
        unexpected => panic!("unexpected event: {unexpected:?}"),
    };
    let pairing_run = store.study_run_observation(study_run_id).unwrap();
    assert_eq!(pairing_run.lifecycle_state, StudyRunLifecycleState::Pairing);
    assert_eq!(pairing_run.pairs.len(), 0);
    assert_eq!(
        pairing_run.registered_pair_count,
        StudyRunRegisteredPairCount::new(0).unwrap()
    );
    assert_eq!(
        store
            .study_run_pair_registration(study_run_id, StudyRunPairOrdinal::new(1).unwrap())
            .unwrap(),
        None
    );
    let registered = submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::RegisterStudyRunPair {
            study_run_id,
            pair_ordinal: StudyRunPairOrdinal::new(1).unwrap(),
            pair_id: pair,
            randomization_digest: randomization,
        },
    );
    assert!(matches!(
        registered,
        StudyEvent::StudyRunPairRegistered {
            lifecycle_state: StudyRunLifecycleState::Ready,
            ..
        }
    ));
    let run = store.study_run_observation(study_run_id).unwrap();
    assert_eq!(run.protocol_revision_id, protocol);
    assert_eq!(run.plan_content_object_id, plan_content_object_id);
    assert_eq!(run.plan_digest, Blake3Digest::of_bytes(plan_bytes));
    assert_eq!(run.pair_count, StudyRunPairCount::new(1).unwrap());
    assert_eq!(
        run.registered_pair_count,
        StudyRunRegisteredPairCount::new(1).unwrap()
    );
    assert_eq!(run.lifecycle_state, StudyRunLifecycleState::Ready);
    assert_eq!(run.pairs.len(), 1);
    assert_eq!(
        run.pairs[0].pair_ordinal,
        StudyRunPairOrdinal::new(1).unwrap()
    );
    assert_eq!(run.pairs[0].pair_id, pair);
    assert_eq!(run.pairs[0].randomization_digest, randomization);
    let registration = store
        .study_run_pair_registration(study_run_id, StudyRunPairOrdinal::new(1).unwrap())
        .unwrap()
        .expect("the registered ordinal must be queryable without loading the run");
    assert_eq!(registration.pair_id, pair);
    assert_eq!(registration.randomization_digest, randomization);
    let start_command_id = CommandId::parse("study-run-start-once").unwrap();
    let started = store
        .execute_study_transition(
            start_command_id.clone(),
            StudyCommand::StartStudyRun { study_run_id },
        )
        .unwrap();
    assert!(!started.idempotent);
    assert_eq!(
        started.disposition,
        StudyTransitionDisposition::Accepted(StudyEvent::StudyRunStarted { study_run_id })
    );
    assert_eq!(
        store
            .study_run_observation(study_run_id)
            .unwrap()
            .lifecycle_state,
        StudyRunLifecycleState::Running
    );
    let retried = store
        .execute_study_transition(
            start_command_id,
            StudyCommand::StartStudyRun { study_run_id },
        )
        .unwrap();
    assert!(retried.idempotent);
    assert_eq!(retried.disposition, started.disposition);
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::StartStudyRun { study_run_id },
        ),
        Rejection::InvalidLifecycleTransition
    );
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::RegisterStudyRunPair {
                study_run_id,
                pair_ordinal: StudyRunPairOrdinal::new(1).unwrap(),
                pair_id: pair,
                randomization_digest: randomization,
            },
        ),
        Rejection::InvalidLifecycleTransition
    );
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::CompleteStudyRun { study_run_id },
        ),
        Rejection::InvalidLifecycleTransition
    );

    let retained_forum = match submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::CreateEpisodeForum {
            episode_id: retained,
            charter_digest: Blake3Digest::of_bytes(b"charter-v1"),
        },
    ) {
        StudyEvent::EpisodeForumCreated { forum_id, .. } => forum_id,
        unexpected => panic!("unexpected event: {unexpected:?}"),
    };
    let reset_forum = match submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::CreateEpisodeForum {
            episode_id: reset,
            charter_digest: Blake3Digest::of_bytes(b"charter-v1"),
        },
    ) {
        StudyEvent::EpisodeForumCreated { forum_id, .. } => forum_id,
        unexpected => panic!("unexpected event: {unexpected:?}"),
    };
    let retained_thread = match submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::OpenForumThread {
            forum_id: retained_forum,
            title: ForumThreadTitle::parse("one chronological thread").unwrap(),
        },
    ) {
        StudyEvent::ForumThreadOpened { thread_id, .. } => thread_id,
        unexpected => panic!("unexpected event: {unexpected:?}"),
    };
    let reset_thread = match submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::OpenForumThread {
            forum_id: reset_forum,
            title: ForumThreadTitle::parse("one chronological thread").unwrap(),
        },
    ) {
        StudyEvent::ForumThreadOpened { thread_id, .. } => thread_id,
        unexpected => panic!("unexpected event: {unexpected:?}"),
    };
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::OpenForumThread {
                forum_id: retained_forum,
                title: ForumThreadTitle::parse("second thread must be rejected").unwrap(),
            },
        ),
        Rejection::InvalidLifecycleTransition
    );

    let private_view = Blake3Digest::of_bytes(b"same-private-view-v1");
    let admit = |store: &mut KernelStore,
                 ordinal: &mut u16,
                 episode_id,
                 phase,
                 role,
                 expected_population_snapshot_id|
     -> society_kernel::StudyActorObligationId {
        match submit_study(
            store,
            ordinal,
            StudyCommand::AdmitActorObligation {
                episode_id,
                phase,
                role,
                private_view_digest: private_view,
                prompt_digest: prompt,
                tool_digest: tools,
                budget: StudyBudgetUnits::new(3).unwrap(),
                read_budget: ForumReadBudget::new(3).unwrap(),
                post_budget: ForumPostBudget::new(1).unwrap(),
            },
        ) {
            StudyEvent::ActorObligationAdmitted {
                obligation_id,
                population_snapshot_id,
                ..
            } => {
                assert_eq!(population_snapshot_id, expected_population_snapshot_id);
                obligation_id
            }
            unexpected => panic!("unexpected event: {unexpected:?}"),
        }
    };
    let source_role = StudyRoleOrdinal::new(1).unwrap();
    let retained_source = admit(
        &mut store,
        &mut ordinal,
        retained,
        StudyPopulationPhase::Source,
        source_role,
        population,
    );
    let reset_source = admit(
        &mut store,
        &mut ordinal,
        reset,
        StudyPopulationPhase::Source,
        source_role,
        population,
    );
    for (obligation, forum) in [
        (retained_source, retained_forum),
        (reset_source, reset_forum),
    ] {
        submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::AdmitForumExposure {
                obligation_id: obligation,
                forum_id: forum,
                visible_from_message_ordinal: 1,
            },
        );
    }
    let false_claim = ForumMessageBody::parse("false claim: proposition zero").unwrap();
    let retained_false = match submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::PublishForumMessage {
            obligation_id: retained_source,
            kind: ForumMessageKind::Finding,
            body: false_claim.clone(),
            in_reply_to_message_id: None,
            supersedes_message_id: None,
        },
    ) {
        StudyEvent::ForumMessagePublished {
            message_id,
            author_occurrence_id,
            ..
        } => {
            assert_eq!(author_occurrence_id.value(), retained_source.value());
            message_id
        }
        unexpected => panic!("unexpected event: {unexpected:?}"),
    };
    let reset_false = match submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::PublishForumMessage {
            obligation_id: reset_source,
            kind: ForumMessageKind::Finding,
            body: false_claim.clone(),
            in_reply_to_message_id: None,
            supersedes_message_id: None,
        },
    ) {
        StudyEvent::ForumMessagePublished { message_id, .. } => message_id,
        unexpected => panic!("unexpected event: {unexpected:?}"),
    };
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::PublishForumMessage {
                obligation_id: retained_source,
                kind: ForumMessageKind::Finding,
                body: ForumMessageBody::parse("second source post must exceed the quota").unwrap(),
                in_reply_to_message_id: Some(retained_false),
                supersedes_message_id: None,
            },
        ),
        Rejection::BudgetPolicyViolation
    );
    for (obligation_id, message_id) in [
        (retained_source, retained_false),
        (reset_source, reset_false),
    ] {
        assert!(matches!(
            submit_study(
                &mut store,
                &mut ordinal,
                StudyCommand::RetractForumMessage {
                    obligation_id,
                    message_id,
                },
            ),
            StudyEvent::ForumMessageRetracted { .. }
        ));
    }

    submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::CompleteActorObligation {
            obligation_id: retained_source,
            charged_budget: StudyBudgetUnits::new(2).unwrap(),
        },
    );
    let source_failure = Blake3Digest::of_bytes(b"source actor deterministic failure");
    assert_eq!(
        submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::FailActorObligation {
                obligation_id: reset_source,
                reason_digest: source_failure,
            },
        ),
        StudyEvent::ActorObligationFailed {
            obligation_id: reset_source,
            reason_digest: source_failure,
        }
    );
    for (episode, thread, successor_population_snapshot_id) in [
        (retained, retained_thread, retained_successor_population),
        (reset, reset_thread, reset_successor_population),
    ] {
        submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::FreezeForumHead {
                episode_id: episode,
                thread_id: thread,
            },
        );
        assert_eq!(
            rejected_study(
                &mut store,
                &mut ordinal,
                StudyCommand::AdmitActorObligation {
                    episode_id: episode,
                    phase: StudyPopulationPhase::Source,
                    role: StudyRoleOrdinal::new(2).unwrap(),
                    private_view_digest: private_view,
                    prompt_digest: prompt,
                    tool_digest: tools,
                    budget: StudyBudgetUnits::new(3).unwrap(),
                    read_budget: ForumReadBudget::new(3).unwrap(),
                    post_budget: ForumPostBudget::new(3).unwrap(),
                },
            ),
            Rejection::InvalidLifecycleTransition
        );
        assert_eq!(
            rejected_study(
                &mut store,
                &mut ordinal,
                StudyCommand::ReplacePopulation {
                    episode_id: episode,
                    successor_population_snapshot_id: population,
                },
            ),
            Rejection::InvalidLifecycleTransition
        );
        assert_eq!(
            rejected_study(
                &mut store,
                &mut ordinal,
                StudyCommand::ReplacePopulation {
                    episode_id: episode,
                    successor_population_snapshot_id: mismatched_successor_population,
                },
            ),
            Rejection::InvalidLifecycleTransition
        );
        assert_eq!(
            rejected_study(
                &mut store,
                &mut ordinal,
                StudyCommand::ReplacePopulation {
                    episode_id: episode,
                    successor_population_snapshot_id:
                        society_kernel::StudyPopulationSnapshotId::new(999).unwrap(),
                },
            ),
            Rejection::SubjectNotFound
        );
        match submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::ReplacePopulation {
                episode_id: episode,
                successor_population_snapshot_id,
            },
        ) {
            StudyEvent::PopulationReplaced {
                successor_population_snapshot_id: emitted,
                ..
            } => assert_eq!(emitted, successor_population_snapshot_id),
            unexpected => panic!("unexpected event: {unexpected:?}"),
        }
    }
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::PublishForumMessage {
                obligation_id: retained_source,
                kind: ForumMessageKind::Challenge,
                body: ForumMessageBody::parse("source must not survive replacement").unwrap(),
                in_reply_to_message_id: Some(retained_false),
                supersedes_message_id: None,
            },
        ),
        Rejection::CapabilityNoLongerActive
    );

    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::AdmitActorObligation {
                episode_id: retained,
                phase: StudyPopulationPhase::Successor,
                role: source_role,
                private_view_digest: private_view,
                prompt_digest: Blake3Digest::of_bytes(b"wrong-prompt"),
                tool_digest: tools,
                budget: StudyBudgetUnits::new(3).unwrap(),
                read_budget: ForumReadBudget::new(3).unwrap(),
                post_budget: ForumPostBudget::new(3).unwrap(),
            },
        ),
        Rejection::InvalidLifecycleTransition
    );
    let retained_successor = admit(
        &mut store,
        &mut ordinal,
        retained,
        StudyPopulationPhase::Successor,
        source_role,
        retained_successor_population,
    );
    let reset_successor = admit(
        &mut store,
        &mut ordinal,
        reset,
        StudyPopulationPhase::Successor,
        source_role,
        reset_successor_population,
    );
    let retained_obligations = store.study_actor_obligation_observations(retained).unwrap();
    assert_eq!(retained_obligations.len(), 2);
    assert_eq!(
        retained_obligations
            .iter()
            .map(|obligation| (obligation.phase, obligation.role.value()))
            .collect::<Vec<_>>(),
        vec![
            (StudyPopulationPhase::Source, 1),
            (StudyPopulationPhase::Successor, 1),
        ]
    );
    assert_eq!(retained_obligations[0].obligation_id, retained_source);
    assert_eq!(retained_obligations[1].obligation_id, retained_successor);
    assert_eq!(retained_obligations[0].population_snapshot_id, population);
    assert_eq!(
        retained_obligations[1].population_snapshot_id,
        retained_successor_population
    );
    assert_eq!(
        retained_obligations[0].lifecycle_state,
        society_kernel::StudyActorObligationState::Completed
    );
    assert_eq!(retained_obligations[0].prompt_digest, prompt);
    assert_eq!(retained_obligations[0].tool_digest, tools);
    assert_eq!(retained_obligations[0].budget.value(), 3);
    assert_eq!(retained_obligations[0].charged_budget.value(), 2);
    assert_eq!(retained_obligations[0].reads_used, 0);
    assert_eq!(retained_obligations[0].posts_used, 1);
    assert!(matches!(
        store.study_actor_obligation_observations(StudyEpisodeId::new(999).unwrap()),
        Err(StoreError::StudyEpisodeNotFound(episode)) if episode == StudyEpisodeId::new(999).unwrap()
    ));
    submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitForumExposure {
            obligation_id: retained_successor,
            forum_id: retained_forum,
            visible_from_message_ordinal: 1,
        },
    );
    submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::AdmitForumExposure {
            obligation_id: reset_successor,
            forum_id: reset_forum,
            visible_from_message_ordinal: 2,
        },
    );
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::ReadForum {
                obligation_id: retained_successor,
                first_message_ordinal: 1,
                through_message_ordinal: 1,
                rendered_content_object_id: ContentObjectId::new(1).unwrap(),
            },
        ),
        Rejection::InvalidLifecycleTransition
    );
    let correction = ForumMessageBody::parse("correction: proposition one").unwrap();
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::ReleaseMatchedCorrection {
                pair_id: pair,
                retained_thread_id: retained_thread,
                reset_thread_id: reset_thread,
                correction: ForumMessageBody::parse("substituted correction").unwrap(),
            },
        ),
        Rejection::InvalidLifecycleTransition,
        "a matched release cannot substitute bytes after protocol admission"
    );
    let (retained_correction, reset_correction) = match submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::ReleaseMatchedCorrection {
            pair_id: pair,
            retained_thread_id: retained_thread,
            reset_thread_id: reset_thread,
            correction: correction.clone(),
        },
    ) {
        StudyEvent::MatchedCorrectionReleased {
            retained_message_id,
            reset_message_id,
            body_digest,
            ..
        } => {
            assert_eq!(body_digest, correction.digest());
            (retained_message_id, reset_message_id)
        }
        unexpected => panic!("unexpected event: {unexpected:?}"),
    };

    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::ReadForum {
                obligation_id: reset_successor,
                first_message_ordinal: 1,
                through_message_ordinal: 1,
                rendered_content_object_id: ContentObjectId::new(1).unwrap(),
            },
        ),
        Rejection::SubjectNotFound
    );
    let retained_rendering = store
        .prepare_study_forum_read(retained_successor, 1, 2)
        .unwrap();
    let retained_content_object_id =
        register_forum_rendering(&mut store, "study-retained-read", &retained_rendering);
    let _retained_receipt = match submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::ReadForum {
            obligation_id: retained_successor,
            first_message_ordinal: 1,
            through_message_ordinal: 2,
            rendered_content_object_id: retained_content_object_id,
        },
    ) {
        StudyEvent::ForumMessagesRead { receipt_id, .. } => receipt_id,
        unexpected => panic!("unexpected event: {unexpected:?}"),
    };
    let reset_rendering = store
        .prepare_study_forum_read(reset_successor, 2, 2)
        .unwrap();
    let reset_content_object_id =
        register_forum_rendering(&mut store, "study-reset-read", &reset_rendering);
    let _reset_receipt = match submit_study(
        &mut store,
        &mut ordinal,
        StudyCommand::ReadForum {
            obligation_id: reset_successor,
            first_message_ordinal: 2,
            through_message_ordinal: 2,
            rendered_content_object_id: reset_content_object_id,
        },
    ) {
        StudyEvent::ForumMessagesRead { receipt_id, .. } => receipt_id,
        unexpected => panic!("unexpected event: {unexpected:?}"),
    };
    assert!(
        retained_rendering
            .windows(false_claim.as_str().len())
            .any(|window| { window == false_claim.as_str().as_bytes() })
    );
    assert!(
        !reset_rendering
            .windows(false_claim.as_str().len())
            .any(|window| window == false_claim.as_str().as_bytes())
    );
    assert!(
        !reset_rendering
            .windows(b"message id=".len())
            .any(|window| window == b"message id="),
        "a reset actor receives thread ordinals, never global message IDs whose gaps leak another arm"
    );
    assert!(
        store
            .prepare_study_forum_read(reset_successor, 1, 2)
            .is_err()
    );
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::PublishForumMessage {
                obligation_id: reset_successor,
                kind: ForumMessageKind::Challenge,
                body: ForumMessageBody::parse("hidden source target must be rejected").unwrap(),
                in_reply_to_message_id: Some(reset_false),
                supersedes_message_id: None,
            },
        ),
        Rejection::SubjectNotFound
    );

    for (obligation, correction_message) in [
        (retained_successor, retained_correction),
        (reset_successor, reset_correction),
    ] {
        submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::RecordDecision {
                obligation_id: obligation,
                decision: StudyDecisionBody::parse("corrected decision v1").unwrap(),
                cited_message_id: Some(correction_message),
            },
        );
        submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::CompleteActorObligation {
                obligation_id: obligation,
                charged_budget: StudyBudgetUnits::new(2).unwrap(),
            },
        );
    }
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::RecordMeasurementResult {
                episode_id: retained,
                measurement_slot: StudyMeasurementSlot::new(1).unwrap(),
                status: StudyMeasurementStatus::Observed,
                value: Some(1),
                value_digest: Some(Blake3Digest::of_bytes(b"latency-value-v1")),
                reason_digest: None,
            },
        ),
        Rejection::InvalidLifecycleTransition
    );
    assert_eq!(
        rejected_study(
            &mut store,
            &mut ordinal,
            StudyCommand::RevealGroundTruth {
                episode_id: retained,
                reveal: StudyGroundTruthReveal::parse("wrong-ground-truth-v1").unwrap(),
            },
        ),
        Rejection::InvalidLifecycleTransition
    );
    for episode in [retained, reset] {
        submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::RevealGroundTruth {
                episode_id: episode,
                reveal: ground_truth_reveal.clone(),
            },
        );
    }
    for episode in [retained, reset] {
        submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::RecordMeasurementResult {
                episode_id: episode,
                measurement_slot: StudyMeasurementSlot::new(1).unwrap(),
                status: StudyMeasurementStatus::Observed,
                value: Some(1),
                value_digest: Some(Blake3Digest::of_bytes(b"latency-value-v1")),
                reason_digest: None,
            },
        );
        submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::RecordMeasurementResult {
                episode_id: episode,
                measurement_slot: StudyMeasurementSlot::new(2).unwrap(),
                status: StudyMeasurementStatus::Unavailable,
                value: None,
                value_digest: None,
                reason_digest: Some(Blake3Digest::of_bytes(b"runtime-cost-unavailable")),
            },
        );
        assert_eq!(
            rejected_study(
                &mut store,
                &mut ordinal,
                StudyCommand::CloseEpisode {
                    episode_id: episode
                },
            ),
            Rejection::InvalidLifecycleTransition
        );
        submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::RecordMeasurementResult {
                episode_id: episode,
                measurement_slot: StudyMeasurementSlot::new(3).unwrap(),
                status: StudyMeasurementStatus::Invalidated,
                value: None,
                value_digest: None,
                reason_digest: Some(Blake3Digest::of_bytes(b"invalidated-control")),
            },
        );
        assert_eq!(
            rejected_study(
                &mut store,
                &mut ordinal,
                StudyCommand::RecordMeasurementResult {
                    episode_id: episode,
                    measurement_slot: StudyMeasurementSlot::new(4).unwrap(),
                    status: StudyMeasurementStatus::Invalidated,
                    value: None,
                    value_digest: None,
                    reason_digest: Some(Blake3Digest::of_bytes(b"outside-plan")),
                },
            ),
            Rejection::InvalidLifecycleTransition
        );
        submit_study(
            &mut store,
            &mut ordinal,
            StudyCommand::CloseEpisode {
                episode_id: episode,
            },
        );
    }

    let completion_command_id = CommandId::parse("study-run-complete-once").unwrap();
    let completed = store
        .execute_study_transition(
            completion_command_id.clone(),
            StudyCommand::CompleteStudyRun { study_run_id },
        )
        .unwrap();
    assert_eq!(
        completed.disposition,
        StudyTransitionDisposition::Accepted(StudyEvent::StudyRunCompleted { study_run_id })
    );
    assert_eq!(
        store
            .study_run_observation(study_run_id)
            .unwrap()
            .lifecycle_state,
        StudyRunLifecycleState::Completed
    );
    let completion_retry = store
        .execute_study_transition(
            completion_command_id,
            StudyCommand::CompleteStudyRun { study_run_id },
        )
        .unwrap();
    assert!(completion_retry.idempotent);
    assert_eq!(completion_retry.disposition, completed.disposition);

    let observation = store.study_pair_observation(pair).unwrap();
    assert_eq!(observation.pair_id, pair);
    assert_eq!(observation.retained.episode_id, retained);
    assert_eq!(observation.reset.episode_id, reset);
    assert_eq!(observation.retained.treatment, StudyTreatment::Retained);
    assert_eq!(observation.reset.treatment, StudyTreatment::Reset);
    assert_eq!(
        observation.retained.lifecycle_state,
        society_kernel::StudyEpisodeState::Closed
    );
    assert_eq!(
        observation.reset.lifecycle_state,
        society_kernel::StudyEpisodeState::Closed
    );
    assert_eq!(observation.retained.source_actor_obligations, 1);
    assert_eq!(observation.retained.source_terminal_actor_obligations, 1);
    assert_eq!(observation.retained.successor_actor_obligations, 1);
    assert_eq!(observation.retained.successor_terminal_actor_obligations, 1);
    assert_eq!(observation.retained.failed_actor_obligations, 0);
    assert_eq!(observation.retained.forum_reads, 1);
    assert_eq!(observation.retained.decisions, 1);
    assert_eq!(observation.retained.measurements.len(), 3);
    assert!(matches!(
        observation.retained.measurements[0].status,
        StudyMeasurementStatus::Observed
    ));
    assert!(matches!(
        observation.retained.measurements[1].status,
        StudyMeasurementStatus::Unavailable
    ));
    assert!(matches!(
        observation.retained.measurements[2].status,
        StudyMeasurementStatus::Invalidated
    ));

    let idempotent = store
        .execute_study_transition(
            CommandId::parse("study-transition-1").unwrap(),
            StudyCommand::AdmitProtocolRevision {
                application_revision_id: ApplicationRevisionId::new(1).unwrap(),
                protocol_digest: Blake3Digest::of_bytes(b"protocol-v1"),
                actor_policy_digest: Blake3Digest::of_bytes(b"actor-policy-v1"),
                forum_prompt_digest: prompt,
                forum_tool_digest: tools,
                evidence_digest: Blake3Digest::of_bytes(b"evidence-v1"),
                ground_truth_commitment_digest: ground_truth_reveal.digest(),
                correction_digest: Blake3Digest::of_bytes(b"correction: proposition one"),
                topology_digest: Blake3Digest::of_bytes(b"topology-v1"),
                episode_budget: StudyBudgetUnits::new(10).unwrap(),
            },
        )
        .unwrap();
    assert!(idempotent.idempotent);
    assert!(matches!(
        store.execute_study_transition(
            CommandId::parse("study-transition-1").unwrap(),
            StudyCommand::AdmitProtocolRevision {
                application_revision_id: ApplicationRevisionId::new(1).unwrap(),
                protocol_digest: Blake3Digest::of_bytes(b"changed-protocol-v1"),
                actor_policy_digest: Blake3Digest::of_bytes(b"actor-policy-v1"),
                forum_prompt_digest: prompt,
                forum_tool_digest: tools,
                evidence_digest: Blake3Digest::of_bytes(b"evidence-v1"),
                ground_truth_commitment_digest: ground_truth_reveal.digest(),
                correction_digest: Blake3Digest::of_bytes(b"correction: proposition one"),
                topology_digest: Blake3Digest::of_bytes(b"topology-v1"),
                episode_budget: StudyBudgetUnits::new(10).unwrap(),
            },
        ),
        Err(StoreError::IdempotencyConflict)
    ));

    store.replay_ledger().unwrap();
    store.validate_replayed_materialized_state().unwrap();
    drop(store);
    let connection = Connection::connect_test_path(&path).unwrap();
    connection
        .execute(
            "UPDATE study_forum_exposures SET visible_from_message_ordinal = 1
             WHERE study_actor_obligation_id = $1",
            [reset_successor.value()],
        )
        .unwrap();
    drop(connection);
    assert!(
        KernelStore::connect_test_path(&path)
            .unwrap()
            .validate_replayed_materialized_state()
            .is_err()
    );
    let _ = fs::remove_file(path);
}
