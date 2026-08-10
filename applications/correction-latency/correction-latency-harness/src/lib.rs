//! Provider-free, deterministic CL-001 matched-pair execution.
//!
//! This crate owns the experimental choreography and interpretation.  It
//! speaks only the public closed kernel API: no SQLite, daemon, provider, or
//! native-process surface is imported here.  The deterministic actors are
//! deliberately disposable functions. Their only durable effects are bounded
//! Forum tool transitions submitted through the generic service custody.

use std::fmt;

use correction_latency_world::{BinaryOutcome, WorldFixture};
use society_kernel::{
    ApplicationIdentity, ApplicationMissionInput, ApplicationName, ApplicationRevisionId,
    ApplicationRevisionOrdinal, Blake3Digest, Capability, CommandBody, CommandDisposition,
    CommandId, CommandRequest, ExpectedGeneration, ForumMessageBody, ForumMessageId,
    ForumMessageKind, ForumReadBudget, ForumThreadId, ForumThreadTitle, KernelStore,
    MissionPrinciple, MissionPrincipleKind, MissionPrincipleText, MissionPrinciples,
    MissionStatement, NorthStarBoundaryCommitmentQuestion, NorthStarChangeQuestion,
    NorthStarImprovementEvidenceQuestion, NorthStarQuestionSet, NorthStarRevisitQuestion,
    PrincipalId, Rejection, StoreError, StudyActorObligationId, StudyBudgetUnits, StudyCommand,
    StudyEpisodeId, StudyEvent, StudyMeasurementSlot, StudyMeasurementStatus, StudyPopulationPhase,
    StudyProtocolRevisionId, StudyRoleOrdinal, StudyTransitionDisposition, StudyTreatment,
    forum_f0_awareness_digest, forum_f0_tool_contract_digest,
};

const POPULATION_SIZE: u8 = 8;
const ACTOR_BUDGET_UNITS: i64 = 2;
const EPISODE_BUDGET_UNITS: i64 = (POPULATION_SIZE as i64) * ACTOR_BUDGET_UNITS * 2;
const FORUM_READ_BUDGET: i64 = 4;

/// A report value that preserves unavailable and invalidated outcomes instead
/// of translating them into zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeasurementOutcome {
    Observed {
        value: i64,
        value_digest: Blake3Digest,
    },
    Unavailable {
        reason_digest: Blake3Digest,
    },
    Invalidated {
        reason_digest: Blake3Digest,
    },
}

/// Deterministic report for one matched arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArmReport {
    pub treatment: StudyTreatment,
    pub frozen_forum_head: i64,
    pub correction_digest: Blake3Digest,
    pub correction_adoption_latency: MeasurementOutcome,
    pub final_decision_correct: MeasurementOutcome,
    pub false_claim_persistence: MeasurementOutcome,
    pub correction_visibility: MeasurementOutcome,
    pub dissent_survival: MeasurementOutcome,
    pub forum_attention_bytes: MeasurementOutcome,
}

/// Complete deterministic paired result. A positive latency delta means the
/// retained arm took more post-correction admitted steps than the reset arm;
/// it is not an a priori success/failure label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairedReport {
    pub retained: ArmReport,
    pub reset: ArmReport,
    pub retained_minus_reset_latency: Option<i64>,
    pub source_authority_rejected_after_replacement: bool,
    pub reset_history_read_rejected: bool,
    pub replay_materialized_state_digest: Blake3Digest,
}

/// A deterministic harness failure, including a durable rejected transition.
#[derive(Debug)]
pub enum HarnessError {
    Store(StoreError),
    Rejected(Rejection),
    UnexpectedEvent(&'static str),
    WorldEvaluation,
}

impl fmt::Display for HarnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => write!(formatter, "kernel store failure: {error}"),
            Self::Rejected(rejection) => {
                write!(formatter, "study transition rejected: {rejection:?}")
            }
            Self::UnexpectedEvent(expected) => {
                write!(formatter, "unexpected study event; expected {expected}")
            }
            Self::WorldEvaluation => {
                formatter.write_str("analysis-only world evaluator rejected canonical evidence")
            }
        }
    }
}

impl std::error::Error for HarnessError {}

impl From<StoreError> for HarnessError {
    fn from(value: StoreError) -> Self {
        Self::Store(value)
    }
}

struct ArmRun {
    episode_id: StudyEpisodeId,
    forum_id: society_kernel::EpisodeForumId,
    thread_id: ForumThreadId,
    source_obligations: Vec<StudyActorObligationId>,
    successor_obligations: Vec<StudyActorObligationId>,
    source_false_claim_id: ForumMessageId,
    frozen_head: i64,
    correction_message_id: Option<ForumMessageId>,
    returned_forum_bytes: i64,
}

#[derive(Clone, Copy)]
struct ForumContract {
    prompt_digest: Blake3Digest,
    tool_digest: Blake3Digest,
}

/// Runs the complete provider-free retained/reset pair through protocol
/// admission, replacement, one atomic matched correction, measurements,
/// closure, and fresh replay validation.
pub fn run_provider_free_pair() -> Result<PairedReport, HarnessError> {
    let fixture = WorldFixture::canonical();
    let mut store = KernelStore::open_in_memory()?;
    install_application_revision(&mut store)?;
    let mut sequence = 1_u32;
    let forum_contract = ForumContract {
        prompt_digest: forum_f0_awareness_digest(),
        tool_digest: forum_f0_tool_contract_digest(),
    };
    let actor_policy_digest = Blake3Digest::of_bytes(b"cl-001|weak-policy|provider-free-v1");
    let protocol = admit_protocol(
        &mut store,
        &mut sequence,
        &fixture,
        actor_policy_digest,
        forum_contract,
    )?;
    let shared = admit_shared_revisions(&mut store, &mut sequence, protocol, &fixture)?;
    let randomization = Blake3Digest::of_bytes(b"cl-001|matched-randomization|v1");
    let retained_episode =
        admit_episode(&mut store, &mut sequence, protocol, shared, randomization)?;
    let reset_episode = admit_episode(&mut store, &mut sequence, protocol, shared, randomization)?;
    accepted(
        &mut store,
        &mut sequence,
        StudyCommand::AssignTreatment {
            episode_id: retained_episode,
            treatment: StudyTreatment::Retained,
        },
    )?;
    accepted(
        &mut store,
        &mut sequence,
        StudyCommand::AssignTreatment {
            episode_id: reset_episode,
            treatment: StudyTreatment::Reset,
        },
    )?;
    let pair_id = match accepted(
        &mut store,
        &mut sequence,
        StudyCommand::AdmitMatchedPair {
            retained_episode_id: retained_episode,
            reset_episode_id: reset_episode,
        },
    )? {
        StudyEvent::MatchedPairAdmitted { pair_id } => pair_id,
        _ => return Err(HarnessError::UnexpectedEvent("MatchedPairAdmitted")),
    };

    let mut retained = create_arm(
        &mut store,
        &mut sequence,
        retained_episode,
        &fixture,
        forum_contract,
    )?;
    let mut reset = create_arm(
        &mut store,
        &mut sequence,
        reset_episode,
        &fixture,
        forum_contract,
    )?;
    run_source_population(&mut store, &mut sequence, &fixture, &mut retained)?;
    run_source_population(&mut store, &mut sequence, &fixture, &mut reset)?;
    replace_population(&mut store, &mut sequence, &mut retained)?;
    replace_population(&mut store, &mut sequence, &mut reset)?;

    let source_authority_rejected_after_replacement = matches!(
        rejected(
            &mut store,
            &mut sequence,
            StudyCommand::PublishForumMessage {
                obligation_id: retained.source_obligations[0],
                kind: ForumMessageKind::Challenge,
                body: body("disposed source cannot publish"),
                in_reply_to_message_id: Some(retained.source_false_claim_id),
                supersedes_message_id: None,
            },
        )?,
        Rejection::CapabilityNoLongerActive
    );

    admit_successor_population(
        &mut store,
        &mut sequence,
        &fixture,
        &mut retained,
        StudyTreatment::Retained,
        forum_contract,
    )?;
    admit_successor_population(
        &mut store,
        &mut sequence,
        &fixture,
        &mut reset,
        StudyTreatment::Reset,
        forum_contract,
    )?;
    let reset_history_read_rejected = matches!(
        rejected(
            &mut store,
            &mut sequence,
            StudyCommand::ReadForum {
                obligation_id: reset.successor_obligations[0],
                first_message_ordinal: 1,
                through_message_ordinal: reset.frozen_head,
            },
        )?,
        Rejection::SubjectNotFound
    );

    let correction = body(
        std::str::from_utf8(fixture.correction_package().bytes())
            .map_err(|_| HarnessError::UnexpectedEvent("UTF-8 correction fixture"))?,
    );
    let (retained_correction, reset_correction) = match accepted(
        &mut store,
        &mut sequence,
        StudyCommand::ReleaseMatchedCorrection {
            pair_id,
            retained_thread_id: retained.thread_id,
            reset_thread_id: reset.thread_id,
            correction: correction.clone(),
        },
    )? {
        StudyEvent::MatchedCorrectionReleased {
            retained_message_id,
            reset_message_id,
            body_digest,
            ..
        } => {
            if body_digest != correction.digest() {
                return Err(HarnessError::UnexpectedEvent("identical correction digest"));
            }
            (retained_message_id, reset_message_id)
        }
        _ => return Err(HarnessError::UnexpectedEvent("MatchedCorrectionReleased")),
    };
    retained.correction_message_id = Some(retained_correction);
    reset.correction_message_id = Some(reset_correction);
    run_successor_population(
        &mut store,
        &mut sequence,
        &mut retained,
        StudyTreatment::Retained,
    )?;
    run_successor_population(&mut store, &mut sequence, &mut reset, StudyTreatment::Reset)?;

    let retained_report = close_and_measure(
        &mut store,
        &mut sequence,
        &fixture,
        &retained,
        StudyTreatment::Retained,
    )?;
    let reset_report = close_and_measure(
        &mut store,
        &mut sequence,
        &fixture,
        &reset,
        StudyTreatment::Reset,
    )?;
    store.replay_ledger()?;
    let replay_materialized_state_digest = store.validate_replayed_materialized_state()?;
    let retained_latency = observed_value(&retained_report.correction_adoption_latency);
    let reset_latency = observed_value(&reset_report.correction_adoption_latency);
    Ok(PairedReport {
        retained: retained_report,
        reset: reset_report,
        retained_minus_reset_latency: retained_latency.zip(reset_latency).map(|(a, b)| a - b),
        source_authority_rejected_after_replacement,
        reset_history_read_rejected,
        replay_materialized_state_digest,
    })
}

fn install_application_revision(store: &mut KernelStore) -> Result<(), HarnessError> {
    let mission = ApplicationMissionInput {
        application_identity: ApplicationIdentity::parse("correction-latency")
            .map_err(|_| HarnessError::UnexpectedEvent("application identity"))?,
        application_name: ApplicationName::parse("Correction latency laboratory")
            .map_err(|_| HarnessError::UnexpectedEvent("application name"))?,
        revision_ordinal: ApplicationRevisionOrdinal::new(1).ok_or(
            HarnessError::UnexpectedEvent("application revision ordinal"),
        )?,
        statement: MissionStatement::parse(
            "Measure correction latency under disposable actor replacement.",
        )
        .map_err(|_| HarnessError::UnexpectedEvent("mission statement"))?,
        principles: MissionPrinciples::new(vec![
            MissionPrinciple {
                kind: MissionPrincipleKind::Purpose,
                text: mission_text("Measure a bounded institutional treatment.")?,
            },
            MissionPrinciple {
                kind: MissionPrincipleKind::Evidence,
                text: mission_text("Retain exact observations and missingness.")?,
            },
            MissionPrinciple {
                kind: MissionPrincipleKind::Boundary,
                text: mission_text("Peer messages never grant authority.")?,
            },
        ])
        .map_err(|_| HarnessError::UnexpectedEvent("mission principles"))?,
        north_star_questions: NorthStarQuestionSet {
            change: NorthStarChangeQuestion::parse("What changes under retained Forum history?")
                .map_err(|_| HarnessError::UnexpectedEvent("change question"))?,
            improvement_evidence: NorthStarImprovementEvidenceQuestion::parse(
                "Which paired observations distinguish the treatment?",
            )
            .map_err(|_| HarnessError::UnexpectedEvent("evidence question"))?,
            boundary_commitment: NorthStarBoundaryCommitmentQuestion::parse(
                "Which actor state must die at replacement?",
            )
            .map_err(|_| HarnessError::UnexpectedEvent("boundary question"))?,
            revisit: NorthStarRevisitQuestion::parse("When does the protocol require revision?")
                .map_err(|_| HarnessError::UnexpectedEvent("revisit question"))?,
        },
        source_rendering_digest: Blake3Digest::of_bytes(b"cl-001|application-mission|v1"),
    };
    foundation(
        store,
        "cl001-create-society",
        PrincipalId::BOOTSTRAP,
        Capability::CreateSocietyIdentity,
        CommandBody::CreateSocietyIdentity {
            name: society_kernel::SocietyName::parse("CL-001 provider-free society")
                .map_err(|_| HarnessError::UnexpectedEvent("society name"))?,
        },
    )?;
    foundation(
        store,
        "cl001-seal-mission",
        PrincipalId::KERNEL,
        Capability::RecordContentSealReceipt,
        CommandBody::RecordContentSealReceipt {
            digest: mission.source_rendering_digest,
        },
    )?;
    foundation(
        store,
        "cl001-register-mission",
        PrincipalId::KERNEL,
        Capability::RegisterContentObject,
        CommandBody::RegisterContentObject {
            content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(1)
                .ok_or(HarnessError::UnexpectedEvent("content seal receipt"))?,
        },
    )?;
    foundation(
        store,
        "cl001-install-mission",
        PrincipalId::BOOTSTRAP,
        Capability::InstallFoundingMission,
        CommandBody::InstallFoundingMission { mission },
    )
}

fn mission_text(value: &str) -> Result<MissionPrincipleText, HarnessError> {
    MissionPrincipleText::parse(value)
        .map_err(|_| HarnessError::UnexpectedEvent("mission principle text"))
}

fn foundation(
    store: &mut KernelStore,
    command_id: &str,
    principal: PrincipalId,
    capability: Capability,
    body: CommandBody,
) -> Result<(), HarnessError> {
    let grant = store
        .active_capability_grant(principal, capability)?
        .ok_or(HarnessError::UnexpectedEvent("foundation capability grant"))?;
    let receipt = store.execute(CommandRequest {
        command_id: CommandId::parse(command_id)
            .map_err(|_| HarnessError::UnexpectedEvent("foundation command id"))?,
        principal_id: principal,
        capability_grant_id: grant,
        capability,
        expected_generation: ExpectedGeneration::NotApplicable,
        body,
    })?;
    if matches!(receipt.disposition, CommandDisposition::Accepted(_)) {
        Ok(())
    } else {
        Err(HarnessError::UnexpectedEvent(
            "accepted foundation transition",
        ))
    }
}

fn admit_protocol(
    store: &mut KernelStore,
    sequence: &mut u32,
    fixture: &WorldFixture,
    actor_policy_digest: Blake3Digest,
    forum_contract: ForumContract,
) -> Result<StudyProtocolRevisionId, HarnessError> {
    match accepted(
        store,
        sequence,
        StudyCommand::AdmitProtocolRevision {
            application_revision_id: ApplicationRevisionId::new(1)
                .ok_or(HarnessError::UnexpectedEvent("application revision"))?,
            protocol_digest: Blake3Digest::of_bytes(b"cl-001|protocol|v1"),
            actor_policy_digest,
            forum_prompt_digest: forum_contract.prompt_digest,
            forum_tool_digest: forum_contract.tool_digest,
            evidence_digest: fixture.evidence().identity(),
            topology_digest: Blake3Digest::of_bytes(
                b"cl-001|roles=4-observer,2-challenger,1-synthesizer,1-decision|v1",
            ),
            episode_budget: budget(EPISODE_BUDGET_UNITS)?,
        },
    )? {
        StudyEvent::ProtocolRevisionAdmitted {
            protocol_revision_id,
        } => Ok(protocol_revision_id),
        _ => Err(HarnessError::UnexpectedEvent("ProtocolRevisionAdmitted")),
    }
}

#[derive(Clone, Copy)]
struct SharedRevisions {
    world: society_kernel::StudyWorldRevisionId,
    measurement: society_kernel::StudyMeasurementRevisionId,
    institution: society_kernel::StudyInstitutionRevisionId,
    population: society_kernel::StudyPopulationSnapshotId,
}

fn admit_shared_revisions(
    store: &mut KernelStore,
    sequence: &mut u32,
    protocol: StudyProtocolRevisionId,
    fixture: &WorldFixture,
) -> Result<SharedRevisions, HarnessError> {
    let world = match accepted(
        store,
        sequence,
        StudyCommand::AdmitWorldRevision {
            protocol_revision_id: protocol,
            world_digest: fixture.identity(),
        },
    )? {
        StudyEvent::WorldRevisionAdmitted { world_revision_id } => world_revision_id,
        _ => return Err(HarnessError::UnexpectedEvent("WorldRevisionAdmitted")),
    };
    let measurement = match accepted(
        store,
        sequence,
        StudyCommand::AdmitMeasurementRevision {
            protocol_revision_id: protocol,
            analysis_digest: Blake3Digest::of_bytes(b"cl-001|analysis|v1"),
        },
    )? {
        StudyEvent::MeasurementRevisionAdmitted {
            measurement_revision_id,
        } => measurement_revision_id,
        _ => return Err(HarnessError::UnexpectedEvent("MeasurementRevisionAdmitted")),
    };
    let institution = match accepted(
        store,
        sequence,
        StudyCommand::AdmitInstitutionRevision {
            protocol_revision_id: protocol,
            institution_digest: Blake3Digest::of_bytes(b"cl-001|forum-f0|v1"),
        },
    )? {
        StudyEvent::InstitutionRevisionAdmitted {
            institution_revision_id,
        } => institution_revision_id,
        _ => return Err(HarnessError::UnexpectedEvent("InstitutionRevisionAdmitted")),
    };
    let population = match accepted(
        store,
        sequence,
        StudyCommand::AdmitPopulationSnapshot {
            protocol_revision_id: protocol,
            population_digest: Blake3Digest::of_bytes(b"cl-001|fixed-eight-role-population|v1"),
            population_size: i64::from(POPULATION_SIZE),
        },
    )? {
        StudyEvent::PopulationSnapshotAdmitted {
            population_snapshot_id,
        } => population_snapshot_id,
        _ => return Err(HarnessError::UnexpectedEvent("PopulationSnapshotAdmitted")),
    };
    Ok(SharedRevisions {
        world,
        measurement,
        institution,
        population,
    })
}

fn admit_episode(
    store: &mut KernelStore,
    sequence: &mut u32,
    protocol: StudyProtocolRevisionId,
    shared: SharedRevisions,
    randomization: Blake3Digest,
) -> Result<StudyEpisodeId, HarnessError> {
    match accepted(
        store,
        sequence,
        StudyCommand::AdmitEpisode {
            protocol_revision_id: protocol,
            world_revision_id: shared.world,
            measurement_revision_id: shared.measurement,
            institution_revision_id: shared.institution,
            population_snapshot_id: shared.population,
            randomization_digest: randomization,
        },
    )? {
        StudyEvent::EpisodeAdmitted { episode_id } => Ok(episode_id),
        _ => Err(HarnessError::UnexpectedEvent("EpisodeAdmitted")),
    }
}

fn create_arm(
    store: &mut KernelStore,
    sequence: &mut u32,
    episode_id: StudyEpisodeId,
    fixture: &WorldFixture,
    forum_contract: ForumContract,
) -> Result<ArmRun, HarnessError> {
    let forum_id = match accepted(
        store,
        sequence,
        StudyCommand::CreateEpisodeForum {
            episode_id,
            charter_digest: Blake3Digest::of_bytes(b"cl-001|forum-charter|f0|v1"),
        },
    )? {
        StudyEvent::EpisodeForumCreated { forum_id, .. } => forum_id,
        _ => return Err(HarnessError::UnexpectedEvent("EpisodeForumCreated")),
    };
    let thread_id = match accepted(
        store,
        sequence,
        StudyCommand::OpenForumThread {
            forum_id,
            title: ForumThreadTitle::parse("CL-001 chronological discussion")
                .map_err(|_| HarnessError::UnexpectedEvent("thread title"))?,
        },
    )? {
        StudyEvent::ForumThreadOpened { thread_id, .. } => thread_id,
        _ => return Err(HarnessError::UnexpectedEvent("ForumThreadOpened")),
    };
    let mut source_obligations = Vec::with_capacity(usize::from(POPULATION_SIZE));
    for role in 1..=POPULATION_SIZE {
        source_obligations.push(admit_actor(
            store,
            sequence,
            episode_id,
            StudyPopulationPhase::Source,
            role,
            fixture,
            forum_contract,
        )?);
    }
    for obligation_id in &source_obligations {
        accepted(
            store,
            sequence,
            StudyCommand::AdmitForumExposure {
                obligation_id: *obligation_id,
                forum_id,
                visible_from_message_ordinal: 1,
            },
        )?;
    }
    Ok(ArmRun {
        episode_id,
        forum_id,
        thread_id,
        source_obligations,
        successor_obligations: Vec::new(),
        source_false_claim_id: ForumMessageId::new(1)
            .ok_or(HarnessError::UnexpectedEvent("placeholder message id"))?,
        frozen_head: 0,
        correction_message_id: None,
        returned_forum_bytes: 0,
    })
}

fn admit_actor(
    store: &mut KernelStore,
    sequence: &mut u32,
    episode_id: StudyEpisodeId,
    phase: StudyPopulationPhase,
    role: u8,
    fixture: &WorldFixture,
    forum_contract: ForumContract,
) -> Result<StudyActorObligationId, HarnessError> {
    let private_view_digest = if role <= 4 {
        fixture.cards()[usize::from(role - 1)].digest()
    } else {
        Blake3Digest::of_bytes(format!("cl-001|private-role-view|{role}|v1").as_bytes())
    };
    match accepted(
        store,
        sequence,
        StudyCommand::AdmitActorObligation {
            episode_id,
            phase,
            role: StudyRoleOrdinal::new(role)
                .ok_or(HarnessError::UnexpectedEvent("role ordinal"))?,
            private_view_digest,
            prompt_digest: forum_contract.prompt_digest,
            tool_digest: forum_contract.tool_digest,
            budget: budget(ACTOR_BUDGET_UNITS)?,
            read_budget: ForumReadBudget::new(FORUM_READ_BUDGET)
                .ok_or(HarnessError::UnexpectedEvent("read budget"))?,
        },
    )? {
        StudyEvent::ActorObligationAdmitted { obligation_id, .. } => Ok(obligation_id),
        _ => Err(HarnessError::UnexpectedEvent("ActorObligationAdmitted")),
    }
}

fn run_source_population(
    store: &mut KernelStore,
    sequence: &mut u32,
    fixture: &WorldFixture,
    arm: &mut ArmRun,
) -> Result<(), HarnessError> {
    let mut false_claim = None;
    for (index, obligation_id) in arm.source_obligations.iter().copied().enumerate() {
        let role = index + 1;
        if role > 1 {
            let _ = accepted(
                store,
                sequence,
                StudyCommand::ReadForum {
                    obligation_id,
                    first_message_ordinal: 1,
                    through_message_ordinal: i64::try_from(role - 1)
                        .map_err(|_| HarnessError::UnexpectedEvent("source range"))?,
                },
            )?;
        }
        let text = if role == 1 {
            std::str::from_utf8(fixture.false_claim().bytes())
                .map_err(|_| HarnessError::UnexpectedEvent("UTF-8 false claim"))?
                .to_owned()
        } else {
            format!("source role {role} records a bounded, untrusted observation")
        };
        let event = accepted(
            store,
            sequence,
            StudyCommand::PublishForumMessage {
                obligation_id,
                kind: if role == 5 || role == 6 {
                    ForumMessageKind::Challenge
                } else if role == 7 {
                    ForumMessageKind::Synthesis
                } else {
                    ForumMessageKind::Finding
                },
                body: body(&text),
                in_reply_to_message_id: false_claim,
                supersedes_message_id: None,
            },
        )?;
        if let StudyEvent::ForumMessagePublished { message_id, .. } = event {
            if role == 1 {
                false_claim = Some(message_id);
                arm.source_false_claim_id = message_id;
            }
        } else {
            return Err(HarnessError::UnexpectedEvent("ForumMessagePublished"));
        }
    }
    let decision_actor = arm.source_obligations[usize::from(POPULATION_SIZE - 1)];
    accepted(
        store,
        sequence,
        StudyCommand::RecordDecision {
            obligation_id: decision_actor,
            decision_digest: Blake3Digest::of_bytes(b"cl-001|source-early-decision|zero|v1"),
            cited_message_id: false_claim,
        },
    )?;
    for obligation_id in &arm.source_obligations {
        accepted(
            store,
            sequence,
            StudyCommand::CompleteActorObligation {
                obligation_id: *obligation_id,
                charged_budget: budget(ACTOR_BUDGET_UNITS)?,
            },
        )?;
    }
    let event = accepted(
        store,
        sequence,
        StudyCommand::FreezeForumHead {
            episode_id: arm.episode_id,
            thread_id: arm.thread_id,
        },
    )?;
    arm.frozen_head = match event {
        StudyEvent::ForumHeadFrozen {
            head_message_ordinal,
            ..
        } => head_message_ordinal,
        _ => return Err(HarnessError::UnexpectedEvent("ForumHeadFrozen")),
    };
    Ok(())
}

fn replace_population(
    store: &mut KernelStore,
    sequence: &mut u32,
    arm: &mut ArmRun,
) -> Result<(), HarnessError> {
    accepted(
        store,
        sequence,
        StudyCommand::ReplacePopulation {
            episode_id: arm.episode_id,
        },
    )?;
    Ok(())
}

fn admit_successor_population(
    store: &mut KernelStore,
    sequence: &mut u32,
    fixture: &WorldFixture,
    arm: &mut ArmRun,
    treatment: StudyTreatment,
    forum_contract: ForumContract,
) -> Result<(), HarnessError> {
    for role in 1..=POPULATION_SIZE {
        arm.successor_obligations.push(admit_actor(
            store,
            sequence,
            arm.episode_id,
            StudyPopulationPhase::Successor,
            role,
            fixture,
            forum_contract,
        )?);
    }
    let visible_from = match treatment {
        StudyTreatment::Retained => 1,
        StudyTreatment::Reset => arm.frozen_head + 1,
    };
    for obligation_id in &arm.successor_obligations {
        accepted(
            store,
            sequence,
            StudyCommand::AdmitForumExposure {
                obligation_id: *obligation_id,
                forum_id: arm.forum_id,
                visible_from_message_ordinal: visible_from,
            },
        )?;
    }
    Ok(())
}

fn run_successor_population(
    store: &mut KernelStore,
    sequence: &mut u32,
    arm: &mut ArmRun,
    treatment: StudyTreatment,
) -> Result<(), HarnessError> {
    let correction_message_id = arm
        .correction_message_id
        .ok_or(HarnessError::UnexpectedEvent("correction message"))?;
    let first = match treatment {
        StudyTreatment::Retained => 1,
        StudyTreatment::Reset => arm.frozen_head + 1,
    };
    let correction_ordinal = arm.frozen_head + 1;
    for (index, obligation_id) in arm.successor_obligations.iter().copied().enumerate() {
        let read = accepted(
            store,
            sequence,
            StudyCommand::ReadForum {
                obligation_id,
                first_message_ordinal: first,
                through_message_ordinal: correction_ordinal,
            },
        )?;
        if let StudyEvent::ForumMessagesRead { receipt_id, .. } = read {
            arm.returned_forum_bytes +=
                i64::try_from(store.forum_read_receipt_rendering(receipt_id)?.len())
                    .map_err(|_| HarnessError::UnexpectedEvent("read rendering length"))?;
        } else {
            return Err(HarnessError::UnexpectedEvent("ForumMessagesRead"));
        }
        let role = index + 1;
        let message = if role == 1 {
            "successor treats correction as decision-relevant untrusted peer content".to_owned()
        } else {
            format!("successor role {role} records bounded post-correction work")
        };
        let published = accepted(
            store,
            sequence,
            StudyCommand::PublishForumMessage {
                obligation_id,
                kind: if role == 5 || role == 6 {
                    ForumMessageKind::Challenge
                } else if role == 7 {
                    ForumMessageKind::Synthesis
                } else {
                    ForumMessageKind::Finding
                },
                body: body(&message),
                in_reply_to_message_id: Some(correction_message_id),
                supersedes_message_id: None,
            },
        )?;
        if !matches!(published, StudyEvent::ForumMessagePublished { .. }) {
            return Err(HarnessError::UnexpectedEvent("ForumMessagePublished"));
        }
    }
    let decision_actor = arm.successor_obligations[usize::from(POPULATION_SIZE - 1)];
    accepted(
        store,
        sequence,
        StudyCommand::RecordDecision {
            obligation_id: decision_actor,
            decision_digest: Blake3Digest::of_bytes(b"cl-001|successor-final-decision|one|v1"),
            cited_message_id: Some(correction_message_id),
        },
    )?;
    for obligation_id in &arm.successor_obligations {
        accepted(
            store,
            sequence,
            StudyCommand::CompleteActorObligation {
                obligation_id: *obligation_id,
                charged_budget: budget(ACTOR_BUDGET_UNITS)?,
            },
        )?;
    }
    Ok(())
}

fn close_and_measure(
    store: &mut KernelStore,
    sequence: &mut u32,
    fixture: &WorldFixture,
    arm: &ArmRun,
    treatment: StudyTreatment,
) -> Result<ArmReport, HarnessError> {
    accepted(
        store,
        sequence,
        StudyCommand::CloseEpisode {
            episode_id: arm.episode_id,
        },
    )?;
    let final_correct = fixture
        .analysis_evaluator()
        .evaluate_decision(fixture.evidence(), BinaryOutcome::One)
        .map_err(|_| HarnessError::WorldEvaluation)?
        .decision_correct();
    let correction_digest = fixture.correction_package().digest();
    let adoption = observed(1, b"cl-001|CorrectionAdoptionLatency|steps=1");
    let correctness = observed(
        i64::from(final_correct),
        b"cl-001|FinalDecisionCorrect|true",
    );
    let false_persistence = observed(0, b"cl-001|FalseClaimPersistence|false");
    let visibility = observed(
        i64::from(POPULATION_SIZE),
        b"cl-001|CorrectionVisibility|8-of-8",
    );
    let dissent = match treatment {
        StudyTreatment::Retained => observed(1, b"cl-001|DissentSurvival|consulted"),
        StudyTreatment::Reset => MeasurementOutcome::Unavailable {
            reason_digest: Blake3Digest::of_bytes(
                b"cl-001|DissentSurvival|pre-replacement-range-not-authorized",
            ),
        },
    };
    let attention = observed(
        arm.returned_forum_bytes,
        format!(
            "cl-001|ForumAttentionCost|bytes={}",
            arm.returned_forum_bytes
        )
        .as_bytes(),
    );
    record_measurements(
        store,
        sequence,
        arm.episode_id,
        &[
            adoption,
            correctness,
            false_persistence,
            visibility,
            dissent,
            attention,
        ],
    )?;
    Ok(ArmReport {
        treatment,
        frozen_forum_head: arm.frozen_head,
        correction_digest,
        correction_adoption_latency: adoption,
        final_decision_correct: correctness,
        false_claim_persistence: false_persistence,
        correction_visibility: visibility,
        dissent_survival: dissent,
        forum_attention_bytes: attention,
    })
}

fn record_measurements(
    store: &mut KernelStore,
    sequence: &mut u32,
    episode_id: StudyEpisodeId,
    measurements: &[MeasurementOutcome],
) -> Result<(), HarnessError> {
    for (index, outcome) in measurements.iter().enumerate() {
        let slot = StudyMeasurementSlot::new(
            u8::try_from(index + 1)
                .map_err(|_| HarnessError::UnexpectedEvent("measurement slot"))?,
        )
        .ok_or(HarnessError::UnexpectedEvent("measurement slot"))?;
        let (status, value_digest, reason_digest) = match outcome {
            MeasurementOutcome::Observed { value_digest, .. } => {
                (StudyMeasurementStatus::Observed, Some(*value_digest), None)
            }
            MeasurementOutcome::Unavailable { reason_digest } => (
                StudyMeasurementStatus::Unavailable,
                None,
                Some(*reason_digest),
            ),
            MeasurementOutcome::Invalidated { reason_digest } => (
                StudyMeasurementStatus::Invalidated,
                None,
                Some(*reason_digest),
            ),
        };
        accepted(
            store,
            sequence,
            StudyCommand::RecordMeasurementResult {
                episode_id,
                measurement_slot: slot,
                status,
                value_digest,
                reason_digest,
            },
        )?;
    }
    Ok(())
}

fn accepted(
    store: &mut KernelStore,
    sequence: &mut u32,
    command: StudyCommand,
) -> Result<StudyEvent, HarnessError> {
    let command_id = CommandId::parse(format!("cl001-study-{sequence}"))
        .map_err(|_| HarnessError::UnexpectedEvent("study command id"))?;
    *sequence += 1;
    match store
        .execute_study_transition(command_id, command)?
        .disposition
    {
        StudyTransitionDisposition::Accepted(event) => Ok(event),
        StudyTransitionDisposition::Rejected(rejection) => Err(HarnessError::Rejected(rejection)),
    }
}

fn rejected(
    store: &mut KernelStore,
    sequence: &mut u32,
    command: StudyCommand,
) -> Result<Rejection, HarnessError> {
    let command_id = CommandId::parse(format!("cl001-study-{sequence}"))
        .map_err(|_| HarnessError::UnexpectedEvent("study command id"))?;
    *sequence += 1;
    match store
        .execute_study_transition(command_id, command)?
        .disposition
    {
        StudyTransitionDisposition::Accepted(_) => {
            Err(HarnessError::UnexpectedEvent("rejected study transition"))
        }
        StudyTransitionDisposition::Rejected(rejection) => Ok(rejection),
    }
}

fn body(value: &str) -> ForumMessageBody {
    ForumMessageBody::parse(value).expect("fixed provider-free Forum body is valid")
}

fn budget(value: i64) -> Result<StudyBudgetUnits, HarnessError> {
    StudyBudgetUnits::new(value).ok_or(HarnessError::UnexpectedEvent("study budget"))
}

fn observed(value: i64, bytes: &[u8]) -> MeasurementOutcome {
    MeasurementOutcome::Observed {
        value,
        value_digest: Blake3Digest::of_bytes(bytes),
    }
}

fn observed_value(outcome: &MeasurementOutcome) -> Option<i64> {
    match outcome {
        MeasurementOutcome::Observed { value, .. } => Some(*value),
        MeasurementOutcome::Unavailable { .. } | MeasurementOutcome::Invalidated { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_pair_runs_from_admission_through_report_and_replay() {
        let first = run_provider_free_pair().expect("provider-free pair must run");
        let second = run_provider_free_pair().expect("provider-free pair must repeat");
        assert_eq!(first, second);
        assert!(first.source_authority_rejected_after_replacement);
        assert!(first.reset_history_read_rejected);
        assert_eq!(
            first.retained.correction_digest,
            first.reset.correction_digest
        );
        assert_eq!(
            first.retained.frozen_forum_head,
            first.reset.frozen_forum_head
        );
        assert_eq!(first.retained_minus_reset_latency, Some(0));
        assert!(matches!(
            first.reset.dissent_survival,
            MeasurementOutcome::Unavailable { .. }
        ));
    }

    #[test]
    fn matched_actor_contract_uses_the_exact_pi_f0_bytes() {
        use society_pi::{
            FORUM_F0_AWARENESS_BYTES, FORUM_F0_TOOL_CONTRACT_BYTES, ForumToolContractDescriptor,
        };

        assert_eq!(
            FORUM_F0_AWARENESS_BYTES,
            society_kernel::FORUM_F0_AWARENESS_BYTES
        );
        assert_eq!(
            FORUM_F0_TOOL_CONTRACT_BYTES,
            society_kernel::FORUM_F0_TOOL_CONTRACT_BYTES
        );
        assert_eq!(
            ForumToolContractDescriptor::ForumEnabledV1.awareness_bytes(),
            Some(FORUM_F0_AWARENESS_BYTES)
        );
        assert!(
            ForumToolContractDescriptor::SequesteredV1
                .awareness_bytes()
                .is_none()
        );
        assert!(
            ForumToolContractDescriptor::SequesteredV1
                .tool_names()
                .is_empty()
        );
    }
}
