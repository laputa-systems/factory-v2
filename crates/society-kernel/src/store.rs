use std::path::Path;

use rusqlite::{
    Connection, OptionalExtension, Transaction, TransactionBehavior, params,
    types::{FromSql, ValueRef},
};
use thiserror::Error;

use crate::{
    ActorAttemptCancellationReason, ActorAttemptId, ActorAttemptState, ActorAttemptTerminalKind,
    ActorConfigurationId, ActorConfigurationRevisionId, ActorInstanceId, ActorInstanceState,
    ActorModelPolicy, AdmissionGeneration, AdversarialReviewId, AdversarialReviewState,
    BudgetEnvelopeId, BudgetFreezeReason, BudgetReservationId, BudgetReservationState,
    CancellationMode, CancellationRequestId, CancellationState, Capability, CausalEpisodeId,
    CommandBody, CommandDisposition, CommandId, CommandKind, CommandReceipt, CommandRequest,
    ContentMediaSchemaContract, ContentObjectId, ContentSealReceiptId, ContextPackId,
    ContextPackPurpose, CostObservation, CostPostmortemCause, CostPostmortemId,
    CostPostmortemResolution, CostPostmortemState, CostUnavailableReason, CostUnknownReason,
    DeterministicEvaluationReceiptId, DeterministicExperimentId, DeterministicExperimentState,
    DevelopmentalAttractor, EpisodeState, EvaluatorRevisionId, EventBody, EventId, EventKind,
    EvidenceAdmissionId, EvidenceLimitationText, EvidenceSemanticRole, ExecutionProfileId,
    ExecutionProfileKind, ExecutionProfileReadiness, ExpectedGeneration,
    ForensicManifestCapturePolicy, ForensicManifestId, GrandArchitectOfficeSessionId, GraphEdgeId,
    GraphEdgeKind, GraphObjectId, GraphObjectKind, GraphRevisionBody, GraphRevisionId,
    GraphRevisionState, HypothesisRevisionText, InputManifestId, LedgerEvent,
    ObservationRevisionText, OfficeId, OfficeKind, OfficeOccupancyId, OfficeSessionState,
    OfficeSessionTerminalState, OfficeTurnId, OfficeTurnPurpose, OfficeTurnState, OperatingCycleId,
    OperatingCycleState, OperatingCycleTreatment, OutcomeObligationDisposition,
    OutcomeObligationId, OutcomeObligationState, PostmortemActionKind, PostmortemActionProposalId,
    PostmortemCausalClaimId, PostmortemCausalClaimKind, PostmortemId, PostmortemState, PrincipalId,
    PrincipalKind, ProjectId, ProjectMilestoneId, ProjectMilestoneState, ProjectState, Rejection,
    RetentionAccessClass, ReviewChallengeId, ReviewChallengeResponseState, ReviewChallengeSeverity,
    ReviewDispositionKind, ReviewResolutionKind, Sha256Digest, SocietyId, SocietyName, TicketId,
    TicketState, UniverseSeedId, UsdMicros, WorkItemId, WorkItemKind, WorkItemState, WorkLeaseId,
    WorkLeaseState,
};

const MIGRATION_1: &str = include_str!("../../../migrations/0001_kernel.sql");
const MIGRATION_2: &str = include_str!("../../../migrations/0002_coordination_graph.sql");
const MIGRATION_3: &str = include_str!("../../../migrations/0003_execution_foundation.sql");
const MIGRATION_4: &str = include_str!("../../../migrations/0004_content_evidence_foundation.sql");

const COMMAND_BODY_TABLES: [&str; 68] = [
    "command_create_society_identity",
    "command_install_grand_architect_office",
    "command_install_founding_universe_seed",
    "command_appoint_initial_grand_architect",
    "command_set_r0_hard_ceiling",
    "command_bootstrap_society",
    "command_propose_operating_cycle",
    "command_admit_operating_cycle",
    "command_start_grand_architect_office_session",
    "command_record_office_session_ready",
    "command_open_office_turn",
    "command_settle_office_turn",
    "command_quiesce_operating_cycle",
    "command_record_cycle_drained",
    "command_resume_operating_cycle",
    "command_reconcile_operating_cycle",
    "command_close_operating_cycle",
    "command_reserve_budget",
    "command_reconcile_budget",
    "command_request_cancellation",
    "command_reconcile_cancellation",
    "command_record_office_session_terminal",
    "command_close_cost_postmortem",
    "command_create_project",
    "command_charter_project",
    "command_transition_project",
    "command_complete_project_milestone",
    "command_reopen_project",
    "command_create_ticket",
    "command_transition_ticket",
    "command_add_graph_object_revision",
    "command_commit_graph_revision",
    "command_add_graph_edge",
    "command_create_episode",
    "command_transition_episode",
    "command_reopen_episode",
    "command_request_adversarial_review",
    "command_submit_review_challenge",
    "command_respond_to_review_challenge",
    "command_disposition_review_challenge",
    "command_resolve_adversarial_review",
    "command_trigger_postmortem",
    "command_record_postmortem_causal_claim",
    "command_propose_postmortem_action",
    "command_close_postmortem",
    "command_assign_adversarial_reviewer",
    "command_register_actor_configuration",
    "command_register_context_pack",
    "command_admit_actor_instance",
    "command_admit_ticket",
    "command_register_work_item",
    "command_claim_work_item",
    "command_start_actor_attempt",
    "command_attest_actor_attempt_terminal",
    "command_validate_ticket_attempt",
    "command_retry_actor_attempt",
    "command_complete_ticket",
    "command_expire_work_lease",
    "command_cancel_actor_attempt",
    "command_register_outcome_obligation",
    "command_resolve_outcome_obligation",
    "command_record_content_seal_receipt",
    "command_register_content_object",
    "command_register_forensic_manifest",
    "command_register_deterministic_experiment",
    "command_record_deterministic_evaluation_receipt",
    "command_admit_deterministic_evidence",
    "command_close_deterministic_experiment",
];

const EVENT_BODY_TABLES: [&str; 61] = [
    "event_society_identity_created",
    "event_grand_architect_office_installed",
    "event_founding_universe_seed_installed",
    "event_grand_architect_appointed",
    "event_r0_hard_ceiling_set",
    "event_society_bootstrapped",
    "event_operating_cycle_proposed",
    "event_operating_cycle_state_changed",
    "event_grand_architect_office_session_started",
    "event_grand_architect_office_session_state_changed",
    "event_office_turn_opened",
    "event_office_turn_settled",
    "event_budget_reserved",
    "event_budget_reconciled",
    "event_budget_admission_frozen",
    "event_cancellation_requested",
    "event_cancellation_reconciled",
    "event_cost_postmortem_closed",
    "event_project_created",
    "event_project_chartered",
    "event_project_state_changed",
    "event_project_milestone_completed",
    "event_ticket_created",
    "event_ticket_state_changed",
    "event_graph_object_revision_added",
    "event_graph_revision_committed",
    "event_graph_edge_added",
    "event_episode_created",
    "event_episode_state_changed",
    "event_adversarial_review_requested",
    "event_review_challenge_submitted",
    "event_review_challenge_responded",
    "event_review_challenge_dispositioned",
    "event_adversarial_review_resolved",
    "event_postmortem_triggered",
    "event_postmortem_causal_claim_recorded",
    "event_postmortem_action_proposed",
    "event_postmortem_closed",
    "event_adversarial_reviewer_assigned",
    "event_actor_configuration_registered",
    "event_context_pack_registered",
    "event_actor_instance_admitted",
    "event_ticket_admitted",
    "event_work_item_registered",
    "event_work_item_claimed",
    "event_actor_attempt_started",
    "event_actor_attempt_terminal_attested",
    "event_ticket_attempt_validated",
    "event_actor_attempt_retry_prepared",
    "event_ticket_completed",
    "event_work_lease_expired",
    "event_actor_attempt_cancellation_requested",
    "event_outcome_obligation_registered",
    "event_outcome_obligation_resolved",
    "event_content_seal_receipt_recorded",
    "event_content_object_registered",
    "event_forensic_manifest_registered",
    "event_deterministic_experiment_registered",
    "event_deterministic_evaluation_receipt_recorded",
    "event_deterministic_evidence_admitted",
    "event_deterministic_experiment_closed",
];

const GRAPH_REVISION_BODY_TABLES: [&str; 2] = ["observation_revisions", "hypothesis_revisions"];

/// The SQLite implementation of trusted physics. `societyd` will be its only
/// production owner; this crate deliberately accepts an already-opened local
/// connection only through its own constructors so migration and foreign-key
/// enforcement cannot be skipped accidentally.
pub struct KernelStore {
    connection: Connection,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error("database has unsupported schema version {0}")]
    UnsupportedSchemaVersion(i64),
    #[error(
        "refusing schema-v1 upgrade with a nonempty ledger ({command_count} commands, {event_count} events)"
    )]
    NonemptySchemaV1LedgerUpgradeRefused {
        command_count: i64,
        event_count: i64,
    },
    #[error(
        "refusing schema-v2 upgrade with a nonempty ledger ({command_count} commands, {event_count} events)"
    )]
    NonemptySchemaV2LedgerUpgradeRefused {
        command_count: i64,
        event_count: i64,
    },
    #[error(
        "refusing schema-v3 upgrade with a nonempty ledger ({command_count} commands, {event_count} events)"
    )]
    NonemptySchemaV3LedgerUpgradeRefused {
        command_count: i64,
        event_count: i64,
    },
    #[error("command id was already used with a different typed request")]
    IdempotencyConflict,
    #[error("ledger corruption: {0}")]
    LedgerCorruption(&'static str),
    #[error("stored integer does not represent a valid domain value")]
    InvalidStoredValue,
}

#[derive(Clone, Copy)]
struct CycleRow {
    society_id: SocietyId,
    seed_id: UniverseSeedId,
    occupancy_id: OfficeOccupancyId,
    _treatment: OperatingCycleTreatment,
    state: OperatingCycleState,
    generation: AdmissionGeneration,
}

/// The exact immutable assignment/context binding of a WorkItem. The tuple is
/// private to the store because callers must not fabricate an execution
/// context outside the typed command path.
type WorkItemRow = (
    TicketId,
    ActorInstanceId,
    ContextPackId,
    WorkItemKind,
    Option<AdversarialReviewId>,
    WorkItemState,
    Option<ActorAttemptId>,
);

enum CapabilityGrantLookup {
    Active {
        grant_id: i64,
        office_occupancy_id: Option<OfficeOccupancyId>,
        actor_instance_id: Option<ActorInstanceId>,
    },
    Inactive,
}

impl KernelStore {
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Self::from_connection(Connection::open(path)?)
    }

    fn from_connection(connection: Connection) -> Result<Self, StoreError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let schema_version: i64 =
            connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        match schema_version {
            0 => {
                // A fresh database crosses ordered versioned commit boundaries,
                // not one fictional atomic 0 -> 4 boundary. Each migration either
                // commits its own version or rolls back so reopening can retry
                // that exact version step.
                apply_migration_1(&connection)?;
                apply_migration_2(&connection)?;
                apply_migration_3(&connection)?;
                apply_migration_4(&connection)?;
            }
            1 => {
                let (command_count, event_count) = schema_v1_ledger_counts(&connection)?;
                if command_count != 0 || event_count != 0 {
                    // M2 changes the command/event representation. There is
                    // no trustworthy fingerprint rewrite for an M1 ledger, so
                    // preserve it exactly rather than implying replay parity.
                    return Err(StoreError::NonemptySchemaV1LedgerUpgradeRefused {
                        command_count,
                        event_count,
                    });
                }
                apply_migration_2(&connection)?;
                apply_migration_3(&connection)?;
                apply_migration_4(&connection)?;
            }
            2 => {
                let (command_count, event_count) = schema_v1_ledger_counts(&connection)?;
                if command_count != 0 || event_count != 0 {
                    return Err(StoreError::NonemptySchemaV2LedgerUpgradeRefused {
                        command_count,
                        event_count,
                    });
                }
                apply_migration_3(&connection)?;
                apply_migration_4(&connection)?;
            }
            3 => {
                let (command_count, event_count) = schema_v1_ledger_counts(&connection)?;
                if command_count != 0 || event_count != 0 {
                    return Err(StoreError::NonemptySchemaV3LedgerUpgradeRefused {
                        command_count,
                        event_count,
                    });
                }
                apply_migration_4(&connection)?;
            }
            4 => {}
            other => return Err(StoreError::UnsupportedSchemaVersion(other)),
        }
        let foreign_key_violations: i64 =
            connection.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })?;
        if foreign_key_violations != 0 {
            return Err(StoreError::LedgerCorruption(
                "migration left foreign-key violations",
            ));
        }
        Ok(Self { connection })
    }

    /// Resolves one active, exact capability grant for a principal. Callers
    /// must carry this identity back in `CommandRequest`; the kernel will
    /// revalidate it transactionally at command acceptance.
    pub fn active_capability_grant(
        &self,
        principal_id: PrincipalId,
        capability: Capability,
    ) -> Result<Option<crate::CapabilityGrantId>, StoreError> {
        self.connection
            .query_row(
                "SELECT capability_grant_id FROM capability_grants
                 WHERE principal_id = ?1 AND capability_kind = ?2 AND grant_state = 1",
                params![principal_id.value(), capability as i64],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .map(crate::CapabilityGrantId::try_from)
            .transpose()
            .map_err(|_| StoreError::InvalidStoredValue)
    }

    /// Accepts a closed command exactly once. An equal duplicate returns its
    /// original receipt; a changed request using the same command identity is
    /// rejected before any state transition is reconsidered.
    pub fn execute(&mut self, request: CommandRequest) -> Result<CommandReceipt, StoreError> {
        let fingerprint = request_fingerprint(&request);
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some((stored_fingerprint, status, event_id, rejection)) = transaction
            .query_row(
                "SELECT request_fingerprint, command_status, accepted_event_id, rejection_code
                 FROM commands WHERE command_id = ?1",
                [request.command_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                    ))
                },
            )
            .optional()?
        {
            if stored_fingerprint.as_slice() != fingerprint.as_bytes() {
                return Err(StoreError::IdempotencyConflict);
            }
            let receipt = match status {
                1 => CommandReceipt {
                    disposition: CommandDisposition::Accepted(
                        EventId::try_from(event_id.ok_or(StoreError::LedgerCorruption(
                            "accepted command has no event",
                        ))?)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                    ),
                    idempotent: true,
                },
                2 => CommandReceipt {
                    disposition: CommandDisposition::Rejected(rejection_from_i64(
                        rejection.ok_or(StoreError::LedgerCorruption(
                            "rejected command has no rejection code",
                        ))?,
                    )?),
                    idempotent: true,
                },
                _ => return Err(StoreError::LedgerCorruption("unknown command status")),
            };
            transaction.commit()?;
            return Ok(receipt);
        }

        // A newly received command begins in a durable rejected placeholder
        // state. The savepoint below guarantees an unsuccessful transition
        // leaves its exact typed input visible while rolling back all material
        // state changes. Successful commands overwrite this placeholder in the
        // same enclosing transaction.
        transaction.execute(
            "INSERT INTO commands(command_id, principal_id, capability_grant_id, capability_kind, expected_generation,
                                  command_kind, request_fingerprint, command_status, rejection_code,
                                  accepted_event_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 2, ?8, NULL)",
            params![
                request.command_id.as_str(),
                request.principal_id.value(),
                request.capability_grant_id.value(),
                request.capability as i64,
                expected_generation_to_sql(request.expected_generation),
                request.body.kind() as i64,
                fingerprint.as_bytes().as_slice(),
                Rejection::SubjectNotFound as i64,
            ],
        )?;
        let command_row_id = transaction.last_insert_rowid();
        insert_command_body(&transaction, command_row_id, &request.body)?;

        transaction.execute_batch("SAVEPOINT apply_command")?;
        let transition = apply_command(&transaction, command_row_id, &request);
        let receipt = match transition? {
            Ok(event_body) => {
                let event_id = insert_event(
                    &transaction,
                    command_row_id,
                    &request.command_id,
                    &event_body,
                )?;
                transaction.execute(
                    "UPDATE commands
                     SET command_status = 1, rejection_code = NULL, accepted_event_id = ?1
                     WHERE command_row_id = ?2",
                    params![event_id.value(), command_row_id],
                )?;
                transaction.execute_batch("RELEASE apply_command")?;
                CommandReceipt {
                    disposition: CommandDisposition::Accepted(event_id),
                    idempotent: false,
                }
            }
            Err(rejection) => {
                transaction.execute_batch("ROLLBACK TO apply_command; RELEASE apply_command")?;
                transaction.execute(
                    "UPDATE commands SET rejection_code = ?1 WHERE command_row_id = ?2",
                    params![rejection as i64, command_row_id],
                )?;
                CommandReceipt {
                    disposition: CommandDisposition::Rejected(rejection),
                    idempotent: false,
                }
            }
        };
        transaction.commit()?;
        Ok(receipt)
    }

    /// Validates and decodes the append-only event ledger through its named
    /// bodies and stored fingerprints. `validate_replayed_materialized_state`
    /// performs the separate fresh-state reconstruction and comparison.
    pub fn replay_ledger(&self) -> Result<Vec<LedgerEvent>, StoreError> {
        verify_command_bodies(&self.connection)?;
        verify_graph_revision_bodies(&self.connection)?;
        let mut statement = self.connection.prepare(
            "SELECT e.event_id, c.command_id, e.event_kind, e.event_sequence
             FROM events e
             JOIN commands c ON c.command_row_id = e.command_row_id
             ORDER BY e.event_sequence ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;
        let mut events = Vec::new();
        for (index, row) in rows.enumerate() {
            let (event_id, command_id, event_kind, event_sequence) = row?;
            if event_sequence != (index + 1) as i64 {
                return Err(StoreError::LedgerCorruption(
                    "event sequence is not contiguous from one",
                ));
            }
            let command_id =
                CommandId::parse(command_id).map_err(|_| StoreError::InvalidStoredValue)?;
            events.push(LedgerEvent {
                event_id: EventId::try_from(event_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                body: decode_event_body(&self.connection, event_id, event_kind, &command_id)?,
                command_id,
            });
        }
        Ok(events)
    }

    /// Reconstructs this bounded kernel's materialized state by re-executing
    /// its verified typed command ledger into a fresh SQLite store, then
    /// compares every current material table/field through a deterministic
    /// digest. This catches mutable-state tampering without pretending that
    /// body-table cardinality alone is replay.
    pub fn validate_replayed_materialized_state(&self) -> Result<Sha256Digest, StoreError> {
        let expected_events = self.replay_ledger()?;
        let commands = replay_command_requests(&self.connection)?;
        let mut reconstructed = Self::open_in_memory()?;
        for (request, expected_disposition) in commands {
            let receipt = reconstructed.execute(request)?;
            if receipt.disposition != expected_disposition {
                return Err(StoreError::LedgerCorruption(
                    "replayed command receipt differs from durable receipt",
                ));
            }
        }
        if reconstructed.replay_ledger()? != expected_events {
            return Err(StoreError::LedgerCorruption(
                "replayed events differ from durable event ledger",
            ));
        }
        let actual = materialized_state_digest(&self.connection)?;
        let rebuilt = materialized_state_digest(&reconstructed.connection)?;
        if actual != rebuilt {
            return Err(StoreError::LedgerCorruption(
                "materialized state differs from fresh replay",
            ));
        }
        Ok(actual)
    }

    pub fn command_count(&self) -> Result<i64, StoreError> {
        Ok(self
            .connection
            .query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))?)
    }

    /// Replays a previously accepted or rejected command receipt by its stable
    /// correlation identity. Rejection is durable operational history but has
    /// no transition event, so it is intentionally queried apart from events.
    pub fn command_receipt(
        &self,
        command_id: &CommandId,
    ) -> Result<Option<CommandReceipt>, StoreError> {
        let row = self
            .connection
            .query_row(
                "SELECT command_status, accepted_event_id, rejection_code
                 FROM commands WHERE command_id = ?1",
                [command_id.as_str()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                    ))
                },
            )
            .optional()?;
        row.map(|(status, event_id, rejection)| match status {
            1 => Ok(CommandReceipt {
                disposition: CommandDisposition::Accepted(
                    EventId::try_from(event_id.ok_or(StoreError::LedgerCorruption(
                        "accepted command has no event",
                    ))?)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ),
                idempotent: true,
            }),
            2 => Ok(CommandReceipt {
                disposition: CommandDisposition::Rejected(rejection_from_i64(rejection.ok_or(
                    StoreError::LedgerCorruption("rejected command has no rejection code"),
                )?)?),
                idempotent: true,
            }),
            _ => Err(StoreError::LedgerCorruption("unknown command status")),
        })
        .transpose()
    }
}

fn schema_v1_ledger_counts(connection: &Connection) -> Result<(i64, i64), StoreError> {
    Ok((
        connection.query_row("SELECT COUNT(*) FROM commands", [], |row| row.get(0))?,
        connection.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?,
    ))
}

/// `PiSdkQualificationV1` is a bootstrap-only native lab treatment. It has
/// no Grand Architect office work, discovery, or Actor execution surface: the
/// future qualification command may be added only as a kernel-owned typed
/// fact. This guard is intentionally centralized before command dispatch so
/// a newly added cycle-scoped command cannot accidentally turn the paid lab
/// into an ordinary Operating Cycle.
fn qualification_treatment_fences_request(
    transaction: &Transaction<'_>,
    principal_id: PrincipalId,
    body: &CommandBody,
) -> Result<bool, StoreError> {
    if matches!(
        body,
        CommandBody::ProposeOperatingCycle {
            treatment: OperatingCycleTreatment::PiSdkQualificationV1
        }
    ) {
        return Ok(principal_id != PrincipalId::BOOTSTRAP);
    }

    if matches!(body, CommandBody::RegisterActorConfiguration { .. }) {
        let qualification_cycle_exists: i64 = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM operating_cycles
             WHERE treatment = ?1 AND lifecycle_state NOT IN (7, 10, 11))",
            [OperatingCycleTreatment::PiSdkQualificationV1 as i64],
            |row| row.get(0),
        )?;
        return Ok(qualification_cycle_exists != 0
            && principal_id != PrincipalId::BOOTSTRAP
            && principal_id != PrincipalId::KERNEL);
    }

    let Some(cycle_id) = command_operating_cycle_for_treatment(transaction, body)? else {
        return Ok(false);
    };
    let treatment: Option<i64> = transaction
        .query_row(
            "SELECT treatment FROM operating_cycles WHERE operating_cycle_id = ?1",
            [cycle_id.value()],
            |row| row.get(0),
        )
        .optional()?;
    if treatment != Some(OperatingCycleTreatment::PiSdkQualificationV1 as i64) {
        return Ok(false);
    }

    let permitted = match principal_id {
        PrincipalId::BOOTSTRAP => matches!(body, CommandBody::AdmitOperatingCycle { .. }),
        PrincipalId::KERNEL => matches!(
            body,
            CommandBody::RecordCycleDrained { .. }
                | CommandBody::RecordOfficeSessionReady { .. }
                | CommandBody::RecordOfficeSessionTerminal { .. }
                | CommandBody::SettleOfficeTurn { .. }
                | CommandBody::ReconcileBudget { .. }
                | CommandBody::ReconcileCancellation { .. }
                | CommandBody::AttestActorAttemptTerminal { .. }
                | CommandBody::ExpireWorkLease { .. }
                | CommandBody::CancelActorAttempt { .. }
        ),
        _ => false,
    };
    Ok(!permitted)
}

fn command_operating_cycle_for_treatment(
    transaction: &Transaction<'_>,
    body: &CommandBody,
) -> Result<Option<OperatingCycleId>, StoreError> {
    let direct = match body {
        CommandBody::AdmitOperatingCycle { cycle_id }
        | CommandBody::StartGrandArchitectOfficeSession { cycle_id }
        | CommandBody::QuiesceOperatingCycle { cycle_id }
        | CommandBody::RecordCycleDrained { cycle_id }
        | CommandBody::ResumeOperatingCycle { cycle_id }
        | CommandBody::ReconcileOperatingCycle { cycle_id }
        | CommandBody::CloseOperatingCycle { cycle_id }
        | CommandBody::ReserveBudget { cycle_id, .. }
        | CommandBody::RequestCancellation { cycle_id, .. } => Some(*cycle_id),
        CommandBody::CreateProject {
            operating_cycle_id, ..
        }
        | CommandBody::CharterProject {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionProject {
            operating_cycle_id, ..
        }
        | CommandBody::CompleteProjectMilestone {
            operating_cycle_id, ..
        }
        | CommandBody::ReopenProject {
            operating_cycle_id, ..
        }
        | CommandBody::CreateTicket {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionTicket {
            operating_cycle_id, ..
        }
        | CommandBody::AddGraphObjectRevision {
            operating_cycle_id, ..
        }
        | CommandBody::CommitGraphRevision {
            operating_cycle_id, ..
        }
        | CommandBody::AddGraphEdge {
            operating_cycle_id, ..
        }
        | CommandBody::CreateEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::ReopenEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::RequestAdversarialReview {
            operating_cycle_id, ..
        }
        | CommandBody::AssignAdversarialReviewer {
            operating_cycle_id, ..
        }
        | CommandBody::SubmitReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::RespondToReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::DispositionReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::ResolveAdversarialReview {
            operating_cycle_id, ..
        }
        | CommandBody::TriggerPostmortem {
            operating_cycle_id, ..
        }
        | CommandBody::RecordPostmortemCausalClaim {
            operating_cycle_id, ..
        }
        | CommandBody::ProposePostmortemAction {
            operating_cycle_id, ..
        }
        | CommandBody::ClosePostmortem {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterContextPack {
            operating_cycle_id, ..
        }
        | CommandBody::AdmitActorInstance {
            operating_cycle_id, ..
        }
        | CommandBody::AdmitTicket {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterWorkItem {
            operating_cycle_id, ..
        }
        | CommandBody::ClaimWorkItem {
            operating_cycle_id, ..
        }
        | CommandBody::StartActorAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::ValidateTicketAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::RetryActorAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::CompleteTicket {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterOutcomeObligation {
            operating_cycle_id, ..
        }
        | CommandBody::ResolveOutcomeObligation {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterForensicManifest {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id, ..
        }
        | CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id, ..
        }
        | CommandBody::AdmitDeterministicEvidence {
            operating_cycle_id, ..
        }
        | CommandBody::CloseDeterministicExperiment {
            operating_cycle_id, ..
        } => Some(*operating_cycle_id),
        _ => None,
    };
    if direct.is_some() {
        return Ok(direct);
    }

    let cycle_id: Option<i64> = match body {
        CommandBody::RecordOfficeSessionReady { session_id }
        | CommandBody::RecordOfficeSessionTerminal { session_id, .. }
        | CommandBody::OpenOfficeTurn { session_id, .. } => transaction
            .query_row(
                "SELECT operating_cycle_id FROM grand_architect_office_sessions
                 WHERE grand_architect_office_session_id = ?1",
                [session_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::SettleOfficeTurn { turn_id } => transaction
            .query_row(
                "SELECT s.operating_cycle_id FROM office_turns t
                 JOIN grand_architect_office_sessions s
                   ON s.grand_architect_office_session_id = t.grand_architect_office_session_id
                 WHERE t.office_turn_id = ?1",
                [turn_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::ReconcileBudget {
            reservation_id, ..
        } => transaction
            .query_row(
                "SELECT operating_cycle_id FROM budget_reservations WHERE budget_reservation_id = ?1",
                [reservation_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        } => transaction
            .query_row(
                "SELECT operating_cycle_id FROM cancellation_requests
                 WHERE cancellation_request_id = ?1",
                [cancellation_request_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::CloseCostPostmortem { postmortem_id, .. } => transaction
            .query_row(
                "SELECT operating_cycle_id FROM cost_postmortems WHERE postmortem_id = ?1",
                [postmortem_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id, ..
        }
        | CommandBody::CancelActorAttempt {
            actor_attempt_id, ..
        } => transaction
            .query_row(
                "SELECT operating_cycle_id FROM attempts WHERE actor_attempt_id = ?1",
                [actor_attempt_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        CommandBody::ExpireWorkLease { work_lease_id } => transaction
            .query_row(
                "SELECT a.operating_cycle_id FROM leases l
                 JOIN actor_instances a ON a.actor_instance_id = l.actor_instance_id
                 WHERE l.work_lease_id = ?1",
                [work_lease_id.value()],
                |row| row.get(0),
            )
            .optional()?,
        _ => None,
    };
    cycle_id
        .map(OperatingCycleId::try_from)
        .transpose()
        .map_err(|_| StoreError::InvalidStoredValue)
}

fn apply_migration_1(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch("BEGIN IMMEDIATE")?;
    if let Err(error) = connection.execute_batch(MIGRATION_1) {
        let _ = connection.execute_batch("ROLLBACK");
        return Err(error.into());
    }
    connection.execute_batch("COMMIT")?;
    Ok(())
}

/// Migration 2 owns its explicit SQLite transaction because it temporarily
/// disables foreign-key rewriting while rebuilding common ledger tables. On a
/// failure, reset both the transaction and connection-local FK mode; a reopen
/// can then retry version 2 from a complete version-1 database.
fn apply_migration_2(connection: &Connection) -> Result<(), StoreError> {
    if let Err(error) = connection.execute_batch(MIGRATION_2) {
        let _ = connection.execute_batch("ROLLBACK");
        let _ = connection.pragma_update(None, "foreign_keys", "ON");
        return Err(error.into());
    }
    Ok(())
}

/// M3 is an atomic version step but intentionally has no historical ledger
/// transform: `from_connection` fences nonempty M2 ledgers before this script
/// can rebuild its command/event ranges and reviewer body shape.
fn apply_migration_3(connection: &Connection) -> Result<(), StoreError> {
    if let Err(error) = connection.execute_batch(MIGRATION_3) {
        let _ = connection.execute_batch("ROLLBACK");
        let _ = connection.pragma_update(None, "foreign_keys", "ON");
        return Err(error.into());
    }
    Ok(())
}

/// M4 widens the closed ledger ranges and installs new named body tables. It
/// is atomic per version step and is deliberately fenced from a nonempty M3
/// ledger rather than inventing a historical fingerprint transformation.
fn apply_migration_4(connection: &Connection) -> Result<(), StoreError> {
    if let Err(error) = connection.execute_batch(MIGRATION_4) {
        let _ = connection.execute_batch("ROLLBACK");
        let _ = connection.pragma_update(None, "foreign_keys", "ON");
        return Err(error.into());
    }
    Ok(())
}

fn apply_command(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    request: &CommandRequest,
) -> Result<Result<EventBody, Rejection>, StoreError> {
    if request.capability != request.body.required_capability() {
        return Ok(Err(Rejection::CapabilityMismatch));
    }
    if matches!(
        request.body,
        CommandBody::AdmitOperatingCycle { .. }
            | CommandBody::StartGrandArchitectOfficeSession { .. }
            | CommandBody::RecordOfficeSessionReady { .. }
            | CommandBody::RecordOfficeSessionTerminal { .. }
            | CommandBody::OpenOfficeTurn { .. }
            | CommandBody::QuiesceOperatingCycle { .. }
            | CommandBody::ResumeOperatingCycle { .. }
            | CommandBody::ReconcileOperatingCycle { .. }
            | CommandBody::CloseOperatingCycle { .. }
            | CommandBody::ReserveBudget { .. }
            | CommandBody::RequestCancellation { .. }
            | CommandBody::CloseCostPostmortem { .. }
            | CommandBody::CreateProject { .. }
            | CommandBody::CharterProject { .. }
            | CommandBody::TransitionProject { .. }
            | CommandBody::CompleteProjectMilestone { .. }
            | CommandBody::ReopenProject { .. }
            | CommandBody::CreateTicket { .. }
            | CommandBody::TransitionTicket { .. }
            | CommandBody::AddGraphObjectRevision { .. }
            | CommandBody::CommitGraphRevision { .. }
            | CommandBody::AddGraphEdge { .. }
            | CommandBody::CreateEpisode { .. }
            | CommandBody::TransitionEpisode { .. }
            | CommandBody::ReopenEpisode { .. }
            | CommandBody::RequestAdversarialReview { .. }
            | CommandBody::AssignAdversarialReviewer { .. }
            | CommandBody::SubmitReviewChallenge { .. }
            | CommandBody::RespondToReviewChallenge { .. }
            | CommandBody::DispositionReviewChallenge { .. }
            | CommandBody::ResolveAdversarialReview { .. }
            | CommandBody::TriggerPostmortem { .. }
            | CommandBody::RecordPostmortemCausalClaim { .. }
            | CommandBody::ProposePostmortemAction { .. }
            | CommandBody::ClosePostmortem { .. }
            | CommandBody::RegisterContextPack { .. }
            | CommandBody::AdmitActorInstance { .. }
            | CommandBody::AdmitTicket { .. }
            | CommandBody::RegisterWorkItem { .. }
            | CommandBody::ClaimWorkItem { .. }
            | CommandBody::StartActorAttempt { .. }
            | CommandBody::ValidateTicketAttempt { .. }
            | CommandBody::RetryActorAttempt { .. }
            | CommandBody::CompleteTicket { .. }
            | CommandBody::RegisterOutcomeObligation { .. }
            | CommandBody::ResolveOutcomeObligation { .. }
            | CommandBody::RegisterForensicManifest { .. }
            | CommandBody::RegisterDeterministicExperiment { .. }
            | CommandBody::RecordDeterministicEvaluationReceipt { .. }
            | CommandBody::AdmitDeterministicEvidence { .. }
            | CommandBody::CloseDeterministicExperiment { .. }
    ) != matches!(request.expected_generation, ExpectedGeneration::Exact(_))
    {
        return Ok(Err(Rejection::InvalidExpectedGeneration));
    }
    let (grant_id, office_occupancy_id, actor_instance_id) = match capability_grant(
        transaction,
        request.principal_id,
        request.capability,
        request.capability_grant_id,
    )? {
        Some(CapabilityGrantLookup::Active {
            grant_id,
            office_occupancy_id,
            actor_instance_id,
        }) => (grant_id, office_occupancy_id, actor_instance_id),
        Some(CapabilityGrantLookup::Inactive) => {
            return Ok(Err(Rejection::CapabilityNoLongerActive));
        }
        None => return Ok(Err(Rejection::CapabilityNotGranted)),
    };
    if request.principal_id != PrincipalId::BOOTSTRAP && request.principal_id != PrincipalId::KERNEL
    {
        let active = match (office_occupancy_id, actor_instance_id) {
            (Some(_), None) => grant_has_active_occupancy(transaction, grant_id)?,
            (None, Some(_)) => grant_has_active_actor_instance(transaction, grant_id)?,
            _ => false,
        };
        if !active {
            return Ok(Err(Rejection::CapabilityNoLongerActive));
        }
    }
    if request.principal_id != PrincipalId::BOOTSTRAP && request.principal_id != PrincipalId::KERNEL
    {
        let target_occupancy_id = match command_target_occupancy(transaction, &request.body) {
            Ok(target_occupancy_id) => target_occupancy_id,
            Err(rejection) => return Ok(Err(rejection)),
        };
        if let Some(target_occupancy_id) = target_occupancy_id
            && office_occupancy_id != Some(target_occupancy_id)
        {
            return Ok(Err(Rejection::CapabilityNoLongerActive));
        }
    }
    if qualification_treatment_fences_request(transaction, request.principal_id, &request.body)? {
        return Ok(Err(Rejection::QualificationTreatmentRestricted));
    }

    let result = match &request.body {
        CommandBody::CreateSocietyIdentity { name } => {
            create_society(transaction, command_row_id, name)
        }
        CommandBody::InstallGrandArchitectOffice => {
            install_grand_architect_office(transaction, command_row_id)
        }
        CommandBody::InstallFoundingUniverseSeed { rendering_digest } => {
            install_founding_universe_seed(transaction, command_row_id, *rendering_digest)
        }
        CommandBody::AppointInitialGrandArchitect { actor_display_name } => {
            appoint_initial_grand_architect(
                transaction,
                command_row_id,
                actor_display_name.as_str(),
            )
        }
        CommandBody::SetR0HardCeiling { ceiling } => {
            set_r0_hard_ceiling(transaction, command_row_id, *ceiling)
        }
        CommandBody::BootstrapSociety => bootstrap_society(transaction, command_row_id),
        CommandBody::ProposeOperatingCycle { treatment } => {
            propose_operating_cycle(transaction, command_row_id, *treatment)
        }
        CommandBody::AdmitOperatingCycle { cycle_id } => admit_operating_cycle(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::StartGrandArchitectOfficeSession { cycle_id } => start_office_session(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::RecordOfficeSessionReady { session_id } => record_office_session_ready(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
        ),
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state,
        } => record_office_session_terminal(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
            *terminal_state,
        ),
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose,
        } => open_office_turn(
            transaction,
            command_row_id,
            request.expected_generation,
            *session_id,
            *purpose,
        ),
        CommandBody::SettleOfficeTurn { turn_id } => {
            settle_office_turn(transaction, command_row_id, *turn_id)
        }
        CommandBody::QuiesceOperatingCycle { cycle_id } => quiesce_cycle(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::RecordCycleDrained { cycle_id } => {
            record_cycle_drained(transaction, command_row_id, *cycle_id)
        }
        CommandBody::ResumeOperatingCycle { cycle_id } => resume_cycle(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::ReconcileOperatingCycle { cycle_id } => begin_reconciliation(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::CloseOperatingCycle { cycle_id } => close_cycle(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
        ),
        CommandBody::ReserveBudget { cycle_id, amount } => reserve_budget(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
            *amount,
        ),
        CommandBody::ReconcileBudget {
            reservation_id,
            observation,
        } => reconcile_budget(transaction, command_row_id, *reservation_id, *observation),
        CommandBody::RequestCancellation { cycle_id, mode } => request_cancellation(
            transaction,
            command_row_id,
            request.expected_generation,
            *cycle_id,
            *mode,
        ),
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        } => reconcile_cancellation(transaction, command_row_id, *cancellation_request_id),
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution,
        } => close_cost_postmortem(
            transaction,
            command_row_id,
            request.expected_generation,
            *postmortem_id,
            *resolution,
        ),
        CommandBody::CreateProject {
            operating_cycle_id,
            project_name,
        } => create_project(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            project_name.as_str(),
        ),
        CommandBody::CharterProject {
            operating_cycle_id,
            project_id,
            objective,
            initial_milestone,
            stop_condition,
        } => charter_project(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            objective.as_str(),
            initial_milestone.as_str(),
            stop_condition.as_str(),
        ),
        CommandBody::TransitionProject {
            operating_cycle_id,
            project_id,
            target,
        } => transition_project(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *target,
        ),
        CommandBody::CompleteProjectMilestone {
            operating_cycle_id,
            project_milestone_id,
        } => complete_project_milestone(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_milestone_id,
        ),
        CommandBody::ReopenProject {
            operating_cycle_id,
            project_id,
        } => reopen_project(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
        ),
        CommandBody::CreateTicket {
            operating_cycle_id,
            project_id,
            ticket_title,
            acceptance_condition,
            prerequisite_ticket_id,
        } => create_ticket(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            ticket_title.as_str(),
            acceptance_condition.as_str(),
            *prerequisite_ticket_id,
        ),
        CommandBody::TransitionTicket {
            operating_cycle_id,
            ticket_id,
            target,
        } => transition_ticket(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *ticket_id,
            *target,
        ),
        CommandBody::AddGraphObjectRevision {
            operating_cycle_id,
            project_id,
            causal_episode_id,
            graph_object_id,
            body,
        } => add_graph_object_revision(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *causal_episode_id,
            *graph_object_id,
            body,
        ),
        CommandBody::CommitGraphRevision {
            operating_cycle_id,
            graph_revision_id,
        } => commit_graph_revision(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *graph_revision_id,
        ),
        CommandBody::AddGraphEdge {
            operating_cycle_id,
            project_id,
            from_graph_revision_id,
            to_graph_revision_id,
            edge_kind,
        } => add_graph_edge(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *from_graph_revision_id,
            *to_graph_revision_id,
            *edge_kind,
        ),
        CommandBody::CreateEpisode {
            operating_cycle_id,
            project_id,
        } => create_episode(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
        ),
        CommandBody::TransitionEpisode {
            operating_cycle_id,
            causal_episode_id,
            target,
        } => transition_episode(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *causal_episode_id,
            *target,
        ),
        CommandBody::ReopenEpisode {
            operating_cycle_id,
            causal_episode_id,
        } => reopen_episode(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *causal_episode_id,
        ),
        CommandBody::RequestAdversarialReview {
            operating_cycle_id,
            project_id,
            target_graph_revision_id,
        } => request_adversarial_review(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *target_graph_revision_id,
        ),
        CommandBody::AssignAdversarialReviewer {
            operating_cycle_id,
            adversarial_review_id,
            reviewer_principal_id,
            reviewer_actor_instance_id,
            reviewer_actor_attempt_id,
        } => assign_adversarial_reviewer(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *adversarial_review_id,
            *reviewer_principal_id,
            *reviewer_actor_instance_id,
            *reviewer_actor_attempt_id,
        ),
        CommandBody::SubmitReviewChallenge {
            operating_cycle_id,
            adversarial_review_id,
            target_graph_revision_id,
            author_principal_id,
            severity,
            failure_hypothesis,
        } => submit_review_challenge(
            transaction,
            command_row_id,
            *author_principal_id,
            request.expected_generation,
            *operating_cycle_id,
            *adversarial_review_id,
            *target_graph_revision_id,
            *severity,
            failure_hypothesis.as_str(),
        ),
        CommandBody::RespondToReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            response,
        } => respond_to_review_challenge(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *review_challenge_id,
            response.as_str(),
        ),
        CommandBody::DispositionReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            disposition,
        } => disposition_review_challenge(
            transaction,
            command_row_id,
            request.principal_id,
            request.expected_generation,
            *operating_cycle_id,
            *review_challenge_id,
            *disposition,
        ),
        CommandBody::ResolveAdversarialReview {
            operating_cycle_id,
            adversarial_review_id,
            resolution,
        } => resolve_adversarial_review(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *adversarial_review_id,
            *resolution,
        ),
        CommandBody::TriggerPostmortem {
            operating_cycle_id,
            project_id,
            causal_episode_id,
        } => trigger_postmortem(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *causal_episode_id,
        ),
        CommandBody::RecordPostmortemCausalClaim {
            operating_cycle_id,
            postmortem_id,
            claim_kind,
            claim,
        } => record_postmortem_causal_claim(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *postmortem_id,
            *claim_kind,
            claim.as_str(),
        ),
        CommandBody::ProposePostmortemAction {
            operating_cycle_id,
            postmortem_id,
            action_kind,
            action,
        } => propose_postmortem_action(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *postmortem_id,
            *action_kind,
            action.as_str(),
        ),
        CommandBody::ClosePostmortem {
            operating_cycle_id,
            postmortem_id,
        } => close_postmortem(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *postmortem_id,
        ),
        CommandBody::RegisterActorConfiguration {
            configuration_name,
            model_policy,
            primary_attractor,
        } => register_actor_configuration(
            transaction,
            command_row_id,
            configuration_name.as_str(),
            *model_policy,
            *primary_attractor,
        ),
        CommandBody::RegisterContextPack {
            operating_cycle_id,
            purpose,
            rendering_digest,
        } => register_context_pack(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *purpose,
            *rendering_digest,
        ),
        CommandBody::AdmitActorInstance {
            operating_cycle_id,
            actor_configuration_revision_id,
            execution_profile_id,
            actor_display_name,
        } => admit_actor_instance(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *actor_configuration_revision_id,
            *execution_profile_id,
            actor_display_name.as_str(),
        ),
        CommandBody::AdmitTicket {
            operating_cycle_id,
            ticket_id,
        } => admit_ticket(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *ticket_id,
        ),
        CommandBody::RegisterWorkItem {
            operating_cycle_id,
            ticket_id,
            actor_instance_id,
            context_pack_id,
            work_kind,
            adversarial_review_id,
            assignment,
        } => register_work_item(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *ticket_id,
            *actor_instance_id,
            *context_pack_id,
            *work_kind,
            *adversarial_review_id,
            assignment.as_str(),
        ),
        CommandBody::ClaimWorkItem {
            operating_cycle_id,
            work_item_id,
        } => claim_work_item(
            transaction,
            command_row_id,
            request.principal_id,
            request.expected_generation,
            *operating_cycle_id,
            *work_item_id,
        ),
        CommandBody::StartActorAttempt {
            operating_cycle_id,
            work_item_id,
            reservation_amount,
        } => start_actor_attempt(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *work_item_id,
            *reservation_amount,
        ),
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id,
            terminal_kind,
        } => attest_actor_attempt_terminal(
            transaction,
            command_row_id,
            *actor_attempt_id,
            *terminal_kind,
        ),
        CommandBody::ValidateTicketAttempt {
            operating_cycle_id,
            actor_attempt_id,
        } => validate_ticket_attempt(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *actor_attempt_id,
        ),
        CommandBody::RetryActorAttempt {
            operating_cycle_id,
            actor_attempt_id,
        } => retry_actor_attempt(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *actor_attempt_id,
        ),
        CommandBody::CompleteTicket {
            operating_cycle_id,
            actor_attempt_id,
        } => complete_ticket(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *actor_attempt_id,
        ),
        CommandBody::ExpireWorkLease { work_lease_id } => {
            expire_work_lease(transaction, command_row_id, *work_lease_id)
        }
        CommandBody::CancelActorAttempt {
            actor_attempt_id,
            reason,
        } => cancel_actor_attempt(transaction, command_row_id, *actor_attempt_id, *reason),
        CommandBody::RegisterOutcomeObligation {
            operating_cycle_id,
            project_id,
            obligation,
        } => register_outcome_obligation(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            obligation.as_str(),
        ),
        CommandBody::ResolveOutcomeObligation {
            operating_cycle_id,
            outcome_obligation_id,
            disposition,
        } => resolve_outcome_obligation(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *outcome_obligation_id,
            *disposition,
        ),
        CommandBody::RecordContentSealReceipt { digest } => {
            record_content_seal_receipt(transaction, command_row_id, *digest)
        }
        CommandBody::RegisterContentObject {
            content_seal_receipt_id,
        } => register_content_object(transaction, command_row_id, *content_seal_receipt_id),
        CommandBody::RegisterForensicManifest {
            operating_cycle_id,
            producing_deterministic_experiment_id,
            capture_policy,
            retention_access_class,
            evaluator_output_content_object_id,
        } => register_forensic_manifest(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *producing_deterministic_experiment_id,
            *capture_policy,
            *retention_access_class,
            *evaluator_output_content_object_id,
        ),
        CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id,
            project_id,
            ticket_id,
            target_graph_revision_id,
            evaluator_content_object_id,
            input_manifest_content_object_id,
        } => register_deterministic_experiment(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *project_id,
            *ticket_id,
            *target_graph_revision_id,
            *evaluator_content_object_id,
            *input_manifest_content_object_id,
        ),
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            forensic_manifest_id,
            evaluator_output_content_object_id,
        } => record_deterministic_evaluation_receipt(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *deterministic_experiment_id,
            *evaluator_revision_id,
            *input_manifest_id,
            *forensic_manifest_id,
            *evaluator_output_content_object_id,
        ),
        CommandBody::AdmitDeterministicEvidence {
            operating_cycle_id,
            deterministic_evaluation_receipt_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            evaluator_output_content_object_id,
            related_graph_revision_id,
            semantic_role,
            applicability,
            limitation,
        } => admit_deterministic_evidence(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *deterministic_evaluation_receipt_id,
            *deterministic_experiment_id,
            *evaluator_revision_id,
            *input_manifest_id,
            *evaluator_output_content_object_id,
            *related_graph_revision_id,
            *semantic_role,
            *applicability,
            limitation,
        ),
        CommandBody::CloseDeterministicExperiment {
            operating_cycle_id,
            deterministic_experiment_id,
        } => close_deterministic_experiment(
            transaction,
            command_row_id,
            request.expected_generation,
            *operating_cycle_id,
            *deterministic_experiment_id,
        ),
    };

    if result.is_ok()
        && request.principal_id == PrincipalId::BOOTSTRAP
        && request.capability.requires_consumption()
    {
        transaction.execute(
            "UPDATE capability_grants SET grant_state = 2, consumed_by_command_id = ?1
             WHERE capability_grant_id = ?2 AND grant_state = 1",
            params![command_row_id, grant_id],
        )?;
    }
    Ok(result)
}

fn create_society(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    name: &SocietyName,
) -> Result<EventBody, Rejection> {
    if exists(transaction, "SELECT 1 FROM societies LIMIT 1")? {
        return Err(Rejection::FoundingInvariant);
    }
    transaction
        .execute(
            "INSERT INTO societies(name, lifecycle_state) VALUES (?1, 1)",
            [name.as_str()],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    let society_id = id_from_last_insert::<SocietyId>(transaction)?;
    let _ = command_row_id;
    Ok(EventBody::SocietyIdentityCreated { society_id })
}

fn install_grand_architect_office(
    transaction: &Transaction<'_>,
    command_row_id: i64,
) -> Result<EventBody, Rejection> {
    if !exists(transaction, "SELECT 1 FROM societies LIMIT 1")?
        || exists(
            transaction,
            "SELECT 1 FROM office_contracts WHERE office_kind = 1",
        )?
    {
        return Err(Rejection::FoundingInvariant);
    }
    transaction
        .execute(
            "INSERT INTO office_contracts(office_kind, installed_by_command_id) VALUES (?1, ?2)",
            params![OfficeKind::TheGrandArchitect as i64, command_row_id],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    Ok(EventBody::GrandArchitectOfficeInstalled {
        office_id: id_from_last_insert::<OfficeId>(transaction)?,
    })
}

fn install_founding_universe_seed(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    rendering_digest: Sha256Digest,
) -> Result<EventBody, Rejection> {
    let society_id = only_society_id(transaction)?;
    if exists(
        transaction,
        "SELECT 1 FROM universe_seeds WHERE society_id = (SELECT society_id FROM societies LIMIT 1)",
    )? {
        return Err(Rejection::FoundingInvariant);
    }
    transaction
        .execute(
            "INSERT INTO universe_seeds(society_id, revision, rendering_digest, active, installed_by_command_id)
             VALUES (?1, 1, ?2, 1, ?3)",
            params![society_id.value(), rendering_digest.as_bytes().as_slice(), command_row_id],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    Ok(EventBody::FoundingUniverseSeedInstalled {
        seed_id: id_from_last_insert::<UniverseSeedId>(transaction)?,
    })
}

fn appoint_initial_grand_architect(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    actor_display_name: &str,
) -> Result<EventBody, Rejection> {
    let office_id = grand_architect_office_id(transaction)?;
    if exists(
        transaction,
        "SELECT 1 FROM office_occupancies WHERE office_id = (SELECT office_id FROM office_contracts WHERE office_kind = 1) AND active = 1",
    )? {
        return Err(Rejection::FoundingInvariant);
    }
    transaction
        .execute(
            "INSERT INTO principals(principal_kind, display_name, active) VALUES (?1, ?2, 1)",
            params![PrincipalKind::Actor as i64, actor_display_name],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    let actor_principal = id_from_last_insert::<PrincipalId>(transaction)?;
    transaction
        .execute(
            "INSERT INTO office_occupancies(office_id, principal_id, active, appointed_by_command_id)
             VALUES (?1, ?2, 1, ?3)",
            params![office_id.value(), actor_principal.value(), command_row_id],
        )
        .map_err(|_| Rejection::ActiveOfficeOccupancyAlreadyExists)?;
    let occupancy_id = id_from_last_insert::<OfficeOccupancyId>(transaction)?;
    for capability in Capability::GRAND_ARCHITECT {
        transaction
            .execute(
                "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id,
                                                grant_state, grant_origin, granted_by_command_id, consumed_by_command_id)
                 VALUES (?1, ?2, ?3, 1, 2, ?4, NULL)",
                params![actor_principal.value(), capability as i64, occupancy_id.value(), command_row_id],
            )
            .map_err(|_| Rejection::FoundingInvariant)?;
    }
    Ok(EventBody::GrandArchitectAppointed {
        occupancy_id,
        principal_id: actor_principal,
    })
}

fn set_r0_hard_ceiling(
    transaction: &Transaction<'_>,
    _command_row_id: i64,
    ceiling: UsdMicros,
) -> Result<EventBody, Rejection> {
    let society_id = only_society_id(transaction)?;
    if exists(transaction, "SELECT 1 FROM society_bootstraps LIMIT 1")? {
        return Err(Rejection::FoundingInvariant);
    }
    if ceiling != UsdMicros::VS001_SOCIETY_HARD_CEILING {
        return Err(Rejection::BudgetPolicyViolation);
    }
    Ok(EventBody::R0HardCeilingSet {
        society_id,
        ceiling,
    })
}

fn bootstrap_society(
    transaction: &Transaction<'_>,
    command_row_id: i64,
) -> Result<EventBody, Rejection> {
    let society_id = only_society_id(transaction)?;
    if exists(transaction, "SELECT 1 FROM society_bootstraps LIMIT 1")? {
        return Err(Rejection::FoundingInvariant);
    }
    let seed_id = active_seed_id(transaction, society_id)?;
    let office_id = grand_architect_office_id(transaction)?;
    let occupancy_id = active_grand_architect_occupancy_id(transaction)?;
    let ceiling = hard_ceiling_from_event_body(transaction)?;
    transaction
        .execute(
            "INSERT INTO society_bootstraps(society_id, universe_seed_id, office_id, office_occupancy_id,
                                             hard_ceiling_micros, bootstrapped_by_command_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![society_id.value(), seed_id.value(), office_id.value(), occupancy_id.value(), ceiling.value(), command_row_id],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    transaction
        .execute(
            "UPDATE societies SET lifecycle_state = 2 WHERE society_id = ?1",
            [society_id.value()],
        )
        .map_err(|_| Rejection::FoundingInvariant)?;
    let budget_envelope_id = create_budget_envelope(transaction, command_row_id, ceiling)?;
    transaction.execute(
        "INSERT INTO budget_envelope_constraints(budget_envelope_id, society_id, operating_cycle_id)
         VALUES (?1, ?2, NULL)",
        params![budget_envelope_id.value(), society_id.value()],
    ).map_err(|_| Rejection::FoundingInvariant)?;
    Ok(EventBody::SocietyBootstrapped { society_id })
}

fn propose_operating_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    treatment: OperatingCycleTreatment,
) -> Result<EventBody, Rejection> {
    let (society_id, seed_id, occupancy_id) = bootstrapped_constitution(transaction)?;
    if exists(
        transaction,
        "SELECT 1 FROM operating_cycles WHERE lifecycle_state NOT IN (7, 10, 11)",
    )? {
        return Err(Rejection::ActiveCycleAlreadyExists);
    }
    transaction.execute(
        "INSERT INTO operating_cycles(society_id, universe_seed_id, office_occupancy_id, treatment,
                                      lifecycle_state, admission_generation, proposed_by_command_id,
                                      last_transition_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6)",
        params![society_id.value(), seed_id.value(), occupancy_id.value(), treatment as i64,
                OperatingCycleState::Proposed as i64, command_row_id],
    ).map_err(|_| Rejection::ActiveCycleAlreadyExists)?;
    let cycle_id = id_from_last_insert::<OperatingCycleId>(transaction)?;
    let budget_envelope_id =
        create_budget_envelope(transaction, command_row_id, treatment.budget_ceiling())?;
    transaction.execute(
        "INSERT INTO budget_envelope_constraints(budget_envelope_id, society_id, operating_cycle_id)
         VALUES (?1, NULL, ?2)",
        params![budget_envelope_id.value(), cycle_id.value()],
    ).map_err(|_| Rejection::FoundingInvariant)?;
    Ok(EventBody::OperatingCycleProposed {
        cycle_id,
        generation: AdmissionGeneration::INITIAL,
        treatment,
    })
}

fn admit_operating_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Proposed {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE operating_cycles SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE operating_cycle_id = ?3",
        params![OperatingCycleState::Admitted as i64, command_row_id, cycle_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "INSERT INTO operating_cycle_admissions(operating_cycle_id, admitted_by_command_id, started_by_command_id)
         VALUES (?1, ?2, NULL)",
        params![cycle_id.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: OperatingCycleState::Admitted,
        generation: cycle.generation,
    })
}

fn start_office_session(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Admitted {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE operating_cycles SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE operating_cycle_id = ?3",
        params![OperatingCycleState::Running as i64, command_row_id, cycle_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "UPDATE operating_cycle_admissions SET started_by_command_id = ?1 WHERE operating_cycle_id = ?2",
        params![command_row_id, cycle_id.value()],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction.execute(
        "INSERT INTO grand_architect_office_sessions(operating_cycle_id, office_occupancy_id, lifecycle_state,
                                                      started_by_command_id, last_transition_command_id)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![cycle_id.value(), cycle.occupancy_id.value(), OfficeSessionState::Reserved as i64, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::GrandArchitectOfficeSessionStarted {
        session_id: id_from_last_insert::<GrandArchitectOfficeSessionId>(transaction)?,
        cycle_id,
    })
}

fn record_office_session_ready(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: GrandArchitectOfficeSessionId,
) -> Result<EventBody, Rejection> {
    let (state, cycle_id) = session_row(transaction, session_id)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if state != OfficeSessionState::Reserved || cycle.state != OperatingCycleState::Running {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE grand_architect_office_sessions SET lifecycle_state = ?1, last_transition_command_id = ?2
         WHERE grand_architect_office_session_id = ?3",
        params![OfficeSessionState::Ready as i64, command_row_id, session_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    Ok(EventBody::GrandArchitectOfficeSessionStateChanged {
        session_id,
        state: OfficeSessionState::Ready,
    })
}

/// The kernel records the observed terminal classification after its supervisor
/// has collected process/session evidence. `Closed` is a reconciliation fact;
/// cancellation and failure are separate durable classifications rather than a
/// convenient way to make an unsafe session look normally closed.
fn record_office_session_terminal(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: GrandArchitectOfficeSessionId,
    terminal_state: OfficeSessionTerminalState,
) -> Result<EventBody, Rejection> {
    let (state, cycle_id) = session_row(transaction, session_id)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if session_has_active_turn(transaction, session_id)? {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let next_state = match terminal_state {
        OfficeSessionTerminalState::Closed
            if state == OfficeSessionState::Ready
                && cycle.state == OperatingCycleState::Reconciling =>
        {
            OfficeSessionState::Closed
        }
        OfficeSessionTerminalState::Cancelled
            if matches!(
                state,
                OfficeSessionState::Reserved | OfficeSessionState::Ready
            ) && cycle.state == OperatingCycleState::Cancelling =>
        {
            OfficeSessionState::Cancelled
        }
        OfficeSessionTerminalState::Failed
            if !matches!(
                state,
                OfficeSessionState::Closed
                    | OfficeSessionState::Cancelled
                    | OfficeSessionState::Failed
            ) && cycle.state.is_nonterminal() =>
        {
            OfficeSessionState::Failed
        }
        _ => return Err(Rejection::InvalidLifecycleTransition),
    };
    transaction
        .execute(
            "UPDATE grand_architect_office_sessions SET lifecycle_state = ?1, last_transition_command_id = ?2
             WHERE grand_architect_office_session_id = ?3",
            params![
                next_state as i64,
                command_row_id,
                session_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    Ok(EventBody::GrandArchitectOfficeSessionStateChanged {
        session_id,
        state: next_state,
    })
}

fn open_office_turn(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    session_id: GrandArchitectOfficeSessionId,
    purpose: OfficeTurnPurpose,
) -> Result<EventBody, Rejection> {
    let (state, cycle_id) = session_row(transaction, session_id)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    let purpose_is_admitted = match purpose {
        OfficeTurnPurpose::OrdinaryWork => cycle.state.admits_task_work(),
        OfficeTurnPurpose::Recovery
        | OfficeTurnPurpose::Cancellation
        | OfficeTurnPurpose::Closure => matches!(
            cycle.state,
            OperatingCycleState::Quiescing
                | OperatingCycleState::Drained
                | OperatingCycleState::Reconciling
        ),
    };
    if state != OfficeSessionState::Ready || !purpose_is_admitted {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE grand_architect_office_sessions SET lifecycle_state = ?1, last_transition_command_id = ?2
         WHERE grand_architect_office_session_id = ?3",
        params![OfficeSessionState::TurnActive as i64, command_row_id, session_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "INSERT INTO office_turns(grand_architect_office_session_id, lifecycle_state, purpose, opened_by_command_id, settled_by_command_id)
         VALUES (?1, ?2, ?3, ?4, NULL)",
        params![session_id.value(), OfficeTurnState::Active as i64, purpose as i64, command_row_id],
    ).map_err(|_| Rejection::SessionTurnAlreadyActive)?;
    Ok(EventBody::OfficeTurnOpened {
        turn_id: id_from_last_insert::<OfficeTurnId>(transaction)?,
        session_id,
        purpose,
    })
}

fn settle_office_turn(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    turn_id: OfficeTurnId,
) -> Result<EventBody, Rejection> {
    let (turn_state, session_id) = transaction.query_row(
        "SELECT lifecycle_state, grand_architect_office_session_id FROM office_turns WHERE office_turn_id = ?1",
        [turn_id.value()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    if turn_state != OfficeTurnState::Active as i64 {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let session_id = GrandArchitectOfficeSessionId::try_from(session_id)
        .map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "UPDATE office_turns SET lifecycle_state = ?1, settled_by_command_id = ?2 WHERE office_turn_id = ?3",
        params![OfficeTurnState::Settled as i64, command_row_id, turn_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "UPDATE grand_architect_office_sessions SET lifecycle_state = ?1, last_transition_command_id = ?2
         WHERE grand_architect_office_session_id = ?3",
        params![OfficeSessionState::Ready as i64, command_row_id, session_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    Ok(EventBody::OfficeTurnSettled {
        turn_id,
        session_id,
    })
}

fn quiesce_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if !matches!(
        cycle.state,
        OperatingCycleState::Admitted | OperatingCycleState::Running
    ) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let new_generation = cycle
        .generation
        .increment()
        .map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        OperatingCycleState::Quiescing,
        new_generation,
    )?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: OperatingCycleState::Quiescing,
        generation: new_generation,
    })
}

fn record_cycle_drained(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_row(transaction, cycle_id)?;
    if cycle.state != OperatingCycleState::Quiescing
        || active_office_turn_count(transaction, cycle_id)? != 0
        || live_actor_attempt_count(transaction, cycle_id)? != 0
        || active_work_lease_count(transaction, cycle_id)? != 0
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    // A Quiesce-mode cancellation has drained its children but remains a
    // cancellation duty. Keep the cycle fenced as Cancelling until its
    // explicit reconciliation; ordinary quiescence may become Drained.
    let next_state = if active_cancellation_count(transaction, cycle_id)? != 0 {
        OperatingCycleState::Cancelling
    } else {
        OperatingCycleState::Drained
    };
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        next_state,
        cycle.generation,
    )?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: next_state,
        generation: cycle.generation,
    })
}

fn resume_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Drained {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    // Drained only says that owned execution has stopped. It does not make an
    // unresolved cost admissible again: Frozen reservations deliberately hold
    // their full authorization until a future typed resolution/Postmortem.
    // Likewise, a cancellation duty must reach a durable terminal receipt
    // before the same cycle may reopen admission.
    if unreconciled_reservation_count(transaction, cycle_id)? != 0
        || active_cancellation_count(transaction, cycle_id)? != 0
        || live_actor_attempt_count(transaction, cycle_id)? != 0
        || active_work_lease_count(transaction, cycle_id)? != 0
    {
        return Err(Rejection::IncompleteCycleReconciliation);
    }
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        OperatingCycleState::Running,
        cycle.generation,
    )?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: OperatingCycleState::Running,
        generation: cycle.generation,
    })
}

fn begin_reconciliation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Drained {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        OperatingCycleState::Reconciling,
        cycle.generation,
    )?;
    transaction.execute(
        "INSERT INTO operating_cycle_reconciliations(operating_cycle_id, reconciliation_started_by_command_id, closed_by_command_id)
         VALUES (?1, ?2, NULL)",
        params![cycle_id.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: OperatingCycleState::Reconciling,
        generation: cycle.generation,
    })
}

fn close_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if cycle.state != OperatingCycleState::Reconciling
        || active_office_turn_count(transaction, cycle_id)? != 0
        || live_office_session_count(transaction, cycle_id)? != 0
        || unreconciled_reservation_count(transaction, cycle_id)? != 0
        || active_cancellation_count(transaction, cycle_id)? != 0
        || live_actor_attempt_count(transaction, cycle_id)? != 0
        || active_work_lease_count(transaction, cycle_id)? != 0
    {
        return Err(Rejection::IncompleteCycleReconciliation);
    }
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        OperatingCycleState::Closed,
        cycle.generation,
    )?;
    transaction.execute(
        "UPDATE operating_cycle_reconciliations SET closed_by_command_id = ?1 WHERE operating_cycle_id = ?2",
        params![command_row_id, cycle_id.value()],
    ).map_err(|_| Rejection::IncompleteCycleReconciliation)?;
    Ok(EventBody::OperatingCycleStateChanged {
        cycle_id,
        state: OperatingCycleState::Closed,
        generation: cycle.generation,
    })
}

fn reserve_budget(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
    amount: UsdMicros,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if amount == UsdMicros::ZERO {
        return Err(Rejection::BudgetCeilingExceeded);
    }
    if !cycle.state.admits_task_work() {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let (society_budget, cycle_budget) =
        budget_envelopes_for_cycle(transaction, cycle.society_id, cycle_id)?;
    for budget_id in [society_budget, cycle_budget] {
        let (ceiling, reserved, spent) = budget_amounts(transaction, budget_id)?;
        let Some(next_reserved) = reserved.checked_add(amount) else {
            return Err(Rejection::BudgetCeilingExceeded);
        };
        if next_reserved
            .checked_add(spent)
            .is_none_or(|value| value > ceiling)
        {
            return Err(Rejection::BudgetCeilingExceeded);
        }
    }
    transaction
        .execute(
            "INSERT INTO budget_reservations(operating_cycle_id, amount_micros, reservation_state,
                                         reserved_by_command_id, reconciled_by_command_id)
         VALUES (?1, ?2, ?3, ?4, NULL)",
            params![
                cycle_id.value(),
                amount.value(),
                BudgetReservationState::Reserved as i64,
                command_row_id
            ],
        )
        .map_err(|_| Rejection::BudgetCeilingExceeded)?;
    let reservation_id = id_from_last_insert::<BudgetReservationId>(transaction)?;
    for budget_id in [society_budget, cycle_budget] {
        transaction.execute(
            "UPDATE budget_envelopes SET reserved_micros = reserved_micros + ?1 WHERE budget_envelope_id = ?2",
            params![amount.value(), budget_id.value()],
        ).map_err(|_| Rejection::BudgetCeilingExceeded)?;
        transaction.execute(
            "INSERT INTO budget_reservation_charges(budget_reservation_id, budget_envelope_id, amount_micros)
             VALUES (?1, ?2, ?3)",
            params![reservation_id.value(), budget_id.value(), amount.value()],
        ).map_err(|_| Rejection::BudgetCeilingExceeded)?;
    }
    Ok(EventBody::BudgetReserved {
        reservation_id,
        cycle_id,
        amount,
    })
}

fn reconcile_budget(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    reservation_id: BudgetReservationId,
    observation: CostObservation,
) -> Result<EventBody, Rejection> {
    let (cycle_id, reserved_amount, state) = transaction.query_row(
        "SELECT operating_cycle_id, amount_micros, reservation_state FROM budget_reservations WHERE budget_reservation_id = ?1",
        [reservation_id.value()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    if state != BudgetReservationState::Reserved as i64 {
        return Err(Rejection::ReservationNotActive);
    }
    let cycle_id = OperatingCycleId::try_from(cycle_id).map_err(|_| Rejection::SubjectNotFound)?;
    let reserved_amount =
        UsdMicros::try_from(reserved_amount).map_err(|_| Rejection::SubjectNotFound)?;
    match observation {
        CostObservation::Known(observed) => {
            if observed > reserved_amount {
                return freeze_budget_admission(
                    transaction,
                    command_row_id,
                    reservation_id,
                    cycle_id,
                    reserved_amount,
                    BudgetFreezeReason::KnownOverrun {
                        observed,
                        reserved: reserved_amount,
                    },
                );
            }
            let mut charge_statement = transaction
                .prepare(
                    "SELECT budget_envelope_id, amount_micros FROM budget_reservation_charges
                 WHERE budget_reservation_id = ?1 ORDER BY budget_envelope_id",
                )
                .map_err(|_| Rejection::SubjectNotFound)?;
            let charges = charge_statement
                .query_map([reservation_id.value()], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(|_| Rejection::SubjectNotFound)?;
            for charge in charges {
                let (budget_id, charge_amount) = charge.map_err(|_| Rejection::SubjectNotFound)?;
                transaction
                    .execute(
                        "UPDATE budget_envelopes
                     SET reserved_micros = reserved_micros - ?1, spent_micros = spent_micros + ?2
                     WHERE budget_envelope_id = ?3",
                        params![charge_amount, observed.value(), budget_id],
                    )
                    .map_err(|_| Rejection::BudgetCeilingExceeded)?;
            }
            transaction.execute(
                "UPDATE budget_reservations SET reservation_state = ?1, reconciled_by_command_id = ?2
                 WHERE budget_reservation_id = ?3",
                params![BudgetReservationState::Reconciled as i64, command_row_id, reservation_id.value()],
            ).map_err(|_| Rejection::SubjectNotFound)?;
            Ok(EventBody::BudgetReconciled {
                reservation_id,
                observed,
            })
        }
        CostObservation::Unknown(reason) => freeze_budget_admission(
            transaction,
            command_row_id,
            reservation_id,
            cycle_id,
            reserved_amount,
            BudgetFreezeReason::Unknown(reason),
        ),
        CostObservation::Unavailable(reason) => freeze_budget_admission(
            transaction,
            command_row_id,
            reservation_id,
            cycle_id,
            reserved_amount,
            BudgetFreezeReason::Unavailable(reason),
        ),
    }
}

/// Holds the full reservation, records why the cost cannot be reconciled, and
/// atomically fences the owning cycle before creating its cancellation duty.
/// This same path is used for unknown cost, unavailable accounting, and a
/// known provider overrun: none is permitted to become a rejected fact or zero.
fn freeze_budget_admission(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    reservation_id: BudgetReservationId,
    cycle_id: OperatingCycleId,
    reserved_amount: UsdMicros,
    reason: BudgetFreezeReason,
) -> Result<EventBody, Rejection> {
    transaction
        .execute(
            "UPDATE budget_reservations SET reservation_state = ?1
             WHERE budget_reservation_id = ?2",
            params![
                BudgetReservationState::Frozen as i64,
                reservation_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let cycle = cycle_row(transaction, cycle_id)?;
    let new_generation = cycle
        .generation
        .increment()
        .map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction
        .execute(
            "UPDATE operating_cycles SET lifecycle_state = ?1, admission_generation = ?2,
                                     last_transition_command_id = ?3 WHERE operating_cycle_id = ?4",
            params![
                OperatingCycleState::Cancelling as i64,
                new_generation.value(),
                command_row_id,
                cycle_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let cancellation_request_id = match active_cancellation_for_cycle(transaction, cycle_id)? {
        Some(existing) => existing,
        None => {
            transaction.execute(
                "INSERT INTO cancellation_requests(operating_cycle_id, cancellation_mode, lifecycle_state,
                                                   observed_admission_generation, requested_by_command_id, reconciled_by_command_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
                params![
                    cycle_id.value(),
                    CancellationMode::GracefulCancel as i64,
                    CancellationState::Accepted as i64,
                    cycle.generation.value(),
                    command_row_id
                ],
            ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
            id_from_last_insert::<CancellationRequestId>(transaction)?
        }
    };
    let (cause, observed, unknown, unavailable) = match reason {
        BudgetFreezeReason::KnownOverrun { observed, .. } => (
            CostPostmortemCause::KnownOverrun,
            Some(observed.value()),
            None,
            None,
        ),
        BudgetFreezeReason::Unknown(reason) => (
            CostPostmortemCause::UnknownCost,
            None,
            Some(reason as i64),
            None,
        ),
        BudgetFreezeReason::Unavailable(reason) => (
            CostPostmortemCause::UnavailableCost,
            None,
            None,
            Some(reason as i64),
        ),
    };
    transaction.execute(
        "INSERT INTO cost_postmortems(budget_reservation_id, operating_cycle_id, cancellation_request_id,
                                      cause_kind, observed_micros, reserved_micros, unknown_reason,
                                      unavailable_reason, lifecycle_state, opened_by_command_id, closed_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL)",
        params![
            reservation_id.value(),
            cycle_id.value(),
            cancellation_request_id.value(),
            cause as i64,
            observed,
            reserved_amount.value(),
            unknown,
            unavailable,
            CostPostmortemState::Open as i64,
            command_row_id,
        ],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::BudgetAdmissionFrozen {
        reservation_id,
        cycle_id,
        cancellation_request_id,
        postmortem_id: id_from_last_insert::<CostPostmortemId>(transaction)?,
        reason,
    })
}

fn request_cancellation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    cycle_id: OperatingCycleId,
    mode: CancellationMode,
) -> Result<EventBody, Rejection> {
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if !cycle.state.is_nonterminal()
        || matches!(
            cycle.state,
            OperatingCycleState::Reconciling
                | OperatingCycleState::Cancelling
                | OperatingCycleState::Reaping
        )
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let new_generation = cycle
        .generation
        .increment()
        .map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let cycle_state = match mode {
        CancellationMode::Quiesce => OperatingCycleState::Quiescing,
        CancellationMode::GracefulCancel | CancellationMode::EmergencyStop => {
            OperatingCycleState::Cancelling
        }
    };
    transaction
        .execute(
            "UPDATE operating_cycles SET lifecycle_state = ?1, admission_generation = ?2,
                                     last_transition_command_id = ?3 WHERE operating_cycle_id = ?4",
            params![
                cycle_state as i64,
                new_generation.value(),
                command_row_id,
                cycle_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "INSERT INTO cancellation_requests(operating_cycle_id, cancellation_mode, lifecycle_state,
                                           observed_admission_generation, requested_by_command_id, reconciled_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
        params![cycle_id.value(), mode as i64, CancellationState::Accepted as i64, cycle.generation.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::CancellationRequested {
        cancellation_request_id: id_from_last_insert::<CancellationRequestId>(transaction)?,
        cycle_id,
        mode,
        generation: new_generation,
    })
}

fn reconcile_cancellation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    cancellation_request_id: CancellationRequestId,
) -> Result<EventBody, Rejection> {
    // This foundation accepts one atomic terminal fact only from the compiled
    // kernel-service grant and only after its currently modeled Office work is
    // gone. It is not process-liveness evidence. Milestone 4 must refine this
    // seam with typed propagation, signal, wait/reap, evidence-sealing, and
    // containment receipts before a supervised child can reach this command.
    let (cycle_id, state) = transaction.query_row(
        "SELECT operating_cycle_id, lifecycle_state FROM cancellation_requests WHERE cancellation_request_id = ?1",
        [cancellation_request_id.value()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    if state == CancellationState::Completed as i64
        || state == CancellationState::ContainmentFailed as i64
    {
        return Err(Rejection::CancellationAlreadyTerminal);
    }
    let cycle_id = OperatingCycleId::try_from(cycle_id).map_err(|_| Rejection::SubjectNotFound)?;
    let cycle = cycle_row(transaction, cycle_id)?;
    if cycle.state != OperatingCycleState::Cancelling {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    if active_office_turn_count(transaction, cycle_id)? != 0
        || live_office_session_count(transaction, cycle_id)? != 0
    {
        return Err(Rejection::IncompleteCycleReconciliation);
    }
    transaction
        .execute(
            "UPDATE cancellation_requests SET lifecycle_state = ?1, reconciled_by_command_id = ?2
         WHERE cancellation_request_id = ?3",
            params![
                CancellationState::Completed as i64,
                command_row_id,
                cancellation_request_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    transition_cycle(
        transaction,
        command_row_id,
        cycle_id,
        OperatingCycleState::Drained,
        cycle.generation,
    )?;
    Ok(EventBody::CancellationReconciled {
        cancellation_request_id,
        cycle_id,
    })
}

/// Closes one automatically opened cost Postmortem and performs the only
/// terminal accounting transition permitted for its Frozen reservation. The
/// resolution is deliberately closed over its cause: uncertain accounting is
/// charged conservatively, while a known overrun records the observed amount.
fn close_cost_postmortem(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    postmortem_id: CostPostmortemId,
    resolution: CostPostmortemResolution,
) -> Result<EventBody, Rejection> {
    let (reservation_id, cycle_id, cause, observed, reserved, state) = transaction
        .query_row(
            "SELECT budget_reservation_id, operating_cycle_id, cause_kind, observed_micros,
                    reserved_micros, lifecycle_state
             FROM cost_postmortems WHERE postmortem_id = ?1",
            [postmortem_id.value()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    if state != CostPostmortemState::Open as i64 {
        return Err(Rejection::CostPostmortemNotOpen);
    }
    let reservation_id =
        BudgetReservationId::try_from(reservation_id).map_err(|_| Rejection::SubjectNotFound)?;
    let cycle_id = OperatingCycleId::try_from(cycle_id).map_err(|_| Rejection::SubjectNotFound)?;
    let cycle = cycle_for_generation(transaction, cycle_id, expected_generation)?;
    if !matches!(
        cycle.state,
        OperatingCycleState::Drained | OperatingCycleState::Reconciling
    ) || active_cancellation_count(transaction, cycle_id)? != 0
    {
        return Err(Rejection::IncompleteCycleReconciliation);
    }
    let cause = cost_postmortem_cause_from_i64(cause).map_err(|_| Rejection::SubjectNotFound)?;
    let reserved = UsdMicros::try_from(reserved).map_err(|_| Rejection::SubjectNotFound)?;
    let charged = match (cause, resolution, observed) {
        (
            CostPostmortemCause::KnownOverrun,
            CostPostmortemResolution::ChargeObservedOverrun,
            Some(observed),
        ) => UsdMicros::try_from(observed).map_err(|_| Rejection::SubjectNotFound)?,
        (
            CostPostmortemCause::UnknownCost | CostPostmortemCause::UnavailableCost,
            CostPostmortemResolution::ConservativeFullReservation,
            None,
        ) => reserved,
        _ => return Err(Rejection::InvalidCostPostmortemResolution),
    };
    let reservation_state: i64 = transaction
        .query_row(
            "SELECT reservation_state FROM budget_reservations WHERE budget_reservation_id = ?1",
            [reservation_id.value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    if reservation_state != BudgetReservationState::Frozen as i64 {
        return Err(Rejection::ReservationNotActive);
    }
    let mut charges = transaction
        .prepare(
            "SELECT budget_envelope_id, amount_micros FROM budget_reservation_charges
             WHERE budget_reservation_id = ?1 ORDER BY budget_envelope_id",
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let charges = charges
        .query_map([reservation_id.value()], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|_| Rejection::SubjectNotFound)?;
    for charge in charges {
        let (envelope_id, reserved_charge) = charge.map_err(|_| Rejection::SubjectNotFound)?;
        transaction
            .execute(
                "UPDATE budget_envelopes
                 SET reserved_micros = reserved_micros - ?1, spent_micros = spent_micros + ?2
                 WHERE budget_envelope_id = ?3",
                params![reserved_charge, charged.value(), envelope_id],
            )
            .map_err(|_| Rejection::BudgetCeilingExceeded)?;
    }
    transaction
        .execute(
            "UPDATE budget_reservations SET reservation_state = ?1, reconciled_by_command_id = ?2
             WHERE budget_reservation_id = ?3",
            params![
                BudgetReservationState::Reconciled as i64,
                command_row_id,
                reservation_id.value(),
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    transaction
        .execute(
            "UPDATE cost_postmortems SET lifecycle_state = ?1, closed_by_command_id = ?2
             WHERE postmortem_id = ?3",
            params![
                CostPostmortemState::Closed as i64,
                command_row_id,
                postmortem_id.value(),
            ],
        )
        .map_err(|_| Rejection::CostPostmortemNotOpen)?;
    transaction
        .execute(
            "INSERT INTO cost_postmortem_resolutions(postmortem_id, resolution_kind, charged_micros,
                                                      resolved_by_command_id)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                postmortem_id.value(),
                resolution as i64,
                charged.value(),
                command_row_id,
            ],
        )
        .map_err(|_| Rejection::CostPostmortemNotOpen)?;
    Ok(EventBody::CostPostmortemClosed {
        postmortem_id,
        reservation_id,
        cycle_id,
        resolution,
        charged,
    })
}

/// Every coordination command is attributed to the exact Operating Cycle in
/// which it acted. Projects and causal Episodes intentionally retain only
/// their seed/project identity, so a successor cycle does not rewrite their
/// historical scope into a false single-cycle ownership claim.
fn coordination_cycle(
    transaction: &Transaction<'_>,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
) -> Result<CycleRow, Rejection> {
    let cycle = cycle_for_generation(transaction, operating_cycle_id, expected_generation)?;
    if !cycle.state.admits_task_work() {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    Ok(cycle)
}

fn record_coordination_provenance(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    cycle: CycleRow,
    operating_cycle_id: OperatingCycleId,
    project_id: Option<ProjectId>,
) -> Result<(), Rejection> {
    transaction.execute(
        "INSERT INTO coordination_command_provenance(command_row_id, universe_seed_id, operating_cycle_id, project_id)
         VALUES (?1, ?2, ?3, ?4)",
        params![command_row_id, cycle.seed_id.value(), operating_cycle_id.value(), project_id.map(ProjectId::value)],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(())
}

fn project_row(
    transaction: &Transaction<'_>,
    project_id: ProjectId,
) -> Result<(ProjectState, UniverseSeedId), Rejection> {
    let row = transaction
        .query_row(
            "SELECT lifecycle_state, universe_seed_id FROM projects WHERE project_id = ?1",
            [project_id.value()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        project_state_from_i64(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        UniverseSeedId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn create_project(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_name: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    transaction.execute(
        "INSERT INTO projects(project_name, universe_seed_id, lifecycle_state, created_by_command_id, last_transition_command_id)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![project_name, cycle.seed_id.value(), ProjectState::Proposed as i64, command_row_id],
    ).map_err(|_| Rejection::FoundingInvariant)?;
    let project_id = id_from_last_insert::<ProjectId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ProjectCreated { project_id })
}

#[allow(clippy::too_many_arguments)]
fn charter_project(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    objective: &str,
    milestone: &str,
    stop_condition: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (state, _) = project_row(transaction, project_id)?;
    if state != ProjectState::Challenged {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE projects SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE project_id = ?3",
        params![ProjectState::Chartered as i64, command_row_id, project_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute(
        "INSERT INTO project_objectives(project_id, objective_text, chartered_by_command_id) VALUES (?1, ?2, ?3)",
        params![project_id.value(), objective, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction.execute(
        "INSERT INTO project_milestones(project_id, milestone_name, lifecycle_state, chartered_by_command_id, completed_by_command_id)
         VALUES (?1, ?2, 1, ?3, NULL)",
        params![project_id.value(), milestone, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction.execute(
        "INSERT INTO project_stop_conditions(project_id, stop_condition_text, chartered_by_command_id) VALUES (?1, ?2, ?3)",
        params![project_id.value(), stop_condition, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ProjectChartered { project_id })
}

fn project_close_blocked(
    transaction: &Transaction<'_>,
    project_id: ProjectId,
) -> Result<bool, Rejection> {
    let incomplete_milestones: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM project_milestones WHERE project_id = ?1 AND lifecycle_state != 2",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    let incomplete_tickets: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM tickets WHERE project_id = ?1 AND lifecycle_state != ?2",
            params![project_id.value(), TicketState::Completed as i64],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let open_reviews: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM adversarial_reviews WHERE project_id = ?1 AND lifecycle_state NOT IN (6, 7, 8)",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    let open_postmortems: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM postmortems WHERE project_id = ?1 AND lifecycle_state != 3",
            [project_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let live_attempts: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM attempts a JOIN attempt_budget_reservations r ON r.actor_attempt_id = a.actor_attempt_id
         WHERE r.project_id = ?1 AND a.lifecycle_state IN (1, 2)",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    let active_leases: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM leases l JOIN work_items w ON w.work_item_id = l.work_item_id
         JOIN tickets t ON t.ticket_id = w.ticket_id WHERE t.project_id = ?1 AND l.lifecycle_state = 1",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    let unreconciled_attempt_reservations: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM attempt_budget_reservations a JOIN budget_reservations b ON b.budget_reservation_id = a.budget_reservation_id
         WHERE a.project_id = ?1 AND b.reservation_state != ?2",
        params![project_id.value(), BudgetReservationState::Reconciled as i64], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    let open_outcomes: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM outcome_obligations WHERE project_id = ?1 AND lifecycle_state = 1",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    // An admitted observation is not yet a closed Experiment. The Project
    // cannot discard an evidence-producing experiment while its explicit
    // lifecycle remains open.
    let open_deterministic_experiments: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM deterministic_experiments WHERE project_id = ?1 AND lifecycle_state != 3",
        [project_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    Ok(incomplete_milestones != 0
        || incomplete_tickets != 0
        || open_reviews != 0
        || open_postmortems != 0
        || live_attempts != 0
        || active_leases != 0
        || unreconciled_attempt_reservations != 0
        || open_outcomes != 0
        || open_deterministic_experiments != 0)
}

fn project_transition_allowed(from: ProjectState, to: ProjectState) -> bool {
    matches!(
        (from, to),
        (ProjectState::Proposed, ProjectState::Challenged)
            | (ProjectState::Chartered, ProjectState::Active)
            | (
                ProjectState::Active,
                ProjectState::Paused | ProjectState::Observing | ProjectState::Terminated
            )
            | (
                ProjectState::Paused,
                ProjectState::Active | ProjectState::Terminated
            )
            | (
                ProjectState::Observing,
                ProjectState::Closed | ProjectState::Terminated
            )
            | (ProjectState::Chartered, ProjectState::Terminated)
            | (ProjectState::Reopened, ProjectState::Active)
    )
}

/// Project transitions remain narrow charter/closure control. M3's specific
/// Actor, WorkItem, and Attempt commands own execution state; this generic
/// Project transition never bypasses their live lease, reservation, outcome,
/// or independent-review close blockers.
fn transition_project(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    target: ProjectState,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (state, _) = project_row(transaction, project_id)?;
    if !project_transition_allowed(state, target) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    if target == ProjectState::Closed && project_close_blocked(transaction, project_id)? {
        return Err(Rejection::ProjectCloseBlocked);
    }
    transaction.execute("UPDATE projects SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE project_id = ?3", params![target as i64, command_row_id, project_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ProjectStateChanged {
        project_id,
        state: target,
    })
}

fn complete_project_milestone(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_milestone_id: ProjectMilestoneId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project, state) = transaction.query_row(
        "SELECT project_id, lifecycle_state FROM project_milestones WHERE project_milestone_id = ?1",
        [project_milestone_id.value()], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    let project_id = ProjectId::try_from(project).map_err(|_| Rejection::SubjectNotFound)?;
    if state != ProjectMilestoneState::Pending as i64 {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("UPDATE project_milestones SET lifecycle_state = 2, completed_by_command_id = ?1 WHERE project_milestone_id = ?2", params![command_row_id, project_milestone_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ProjectMilestoneCompleted {
        project_milestone_id,
    })
}

fn reopen_project(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (state, _) = project_row(transaction, project_id)?;
    if !matches!(state, ProjectState::Closed | ProjectState::Terminated) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("UPDATE projects SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE project_id = ?3", params![ProjectState::Reopened as i64, command_row_id, project_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ProjectStateChanged {
        project_id,
        state: ProjectState::Reopened,
    })
}

fn ticket_row(
    transaction: &Transaction<'_>,
    ticket_id: TicketId,
) -> Result<(ProjectId, TicketState), Rejection> {
    let row = transaction
        .query_row(
            "SELECT project_id, lifecycle_state FROM tickets WHERE ticket_id = ?1",
            [ticket_id.value()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        ProjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        ticket_state_from_i64(row.1).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn project_is_active(
    transaction: &Transaction<'_>,
    project_id: ProjectId,
) -> Result<(), Rejection> {
    let (state, _) = project_row(transaction, project_id)?;
    if state != ProjectState::Active {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    Ok(())
}

fn ticket_prerequisites_complete(
    transaction: &Transaction<'_>,
    ticket_id: TicketId,
) -> Result<bool, Rejection> {
    let incomplete: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM ticket_prerequisites p JOIN tickets t ON t.ticket_id = p.prerequisite_ticket_id
         WHERE p.ticket_id = ?1 AND t.lifecycle_state != ?2",
        params![ticket_id.value(), TicketState::Completed as i64], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    Ok(incomplete == 0)
}

#[allow(clippy::too_many_arguments)]
fn create_ticket(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    ticket_title: &str,
    acceptance_condition: &str,
    prerequisite_ticket_id: Option<TicketId>,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    if let Some(prerequisite_ticket_id) = prerequisite_ticket_id {
        let (prerequisite_project, _) = ticket_row(transaction, prerequisite_ticket_id)?;
        if prerequisite_project != project_id {
            return Err(Rejection::SubjectNotFound);
        }
    }
    transaction.execute(
        "INSERT INTO tickets(project_id, ticket_title, lifecycle_state, created_by_command_id, last_transition_command_id)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![project_id.value(), ticket_title, TicketState::Draft as i64, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let ticket_id = id_from_last_insert::<TicketId>(transaction)?;
    transaction.execute(
        "INSERT INTO ticket_acceptance_conditions(ticket_id, condition_text, lifecycle_state, created_by_command_id, satisfied_by_command_id)
         VALUES (?1, ?2, 1, ?3, NULL)",
        params![ticket_id.value(), acceptance_condition, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    if let Some(prerequisite_ticket_id) = prerequisite_ticket_id {
        transaction.execute(
            "INSERT INTO ticket_prerequisites(ticket_id, prerequisite_ticket_id, created_by_command_id) VALUES (?1, ?2, ?3)",
            params![ticket_id.value(), prerequisite_ticket_id.value(), command_row_id],
        ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    }
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::TicketCreated {
        ticket_id,
        project_id,
    })
}

fn transition_ticket(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    ticket_id: TicketId,
    target: TicketState,
) -> Result<EventBody, Rejection> {
    // M3 deliberately retires this broad M2 transition surface. Admission,
    // readiness, claiming, terminal settlement, validation, retry, and
    // completion each require their own Actor/WorkItem/Lease/Attempt command.
    let _ = (
        transaction,
        command_row_id,
        expected_generation,
        operating_cycle_id,
        ticket_id,
        target,
    );
    Err(Rejection::InvalidLifecycleTransition)
}

fn graph_revision_row(
    transaction: &Transaction<'_>,
    graph_revision_id: GraphRevisionId,
) -> Result<
    (
        GraphObjectId,
        ProjectId,
        GraphObjectKind,
        GraphRevisionState,
    ),
    Rejection,
> {
    let row = transaction
        .query_row(
            "SELECT r.graph_object_id, o.project_id, o.object_kind, r.lifecycle_state
         FROM object_revisions r JOIN objects o ON o.graph_object_id = r.graph_object_id
         WHERE r.graph_revision_id = ?1",
            [graph_revision_id.value()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        GraphObjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        ProjectId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        graph_object_kind_from_i64(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        graph_revision_state_from_i64(row.3).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn add_graph_object_revision(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    causal_episode_id: Option<CausalEpisodeId>,
    existing_graph_object_id: Option<GraphObjectId>,
    body: &GraphRevisionBody,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let object_kind = body.object_kind();
    project_is_active(transaction, project_id)?;
    if let Some(episode_id) = causal_episode_id {
        let episode_project: i64 = transaction
            .query_row(
                "SELECT project_id FROM episodes WHERE causal_episode_id = ?1",
                [episode_id.value()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| Rejection::SubjectNotFound)?
            .ok_or(Rejection::SubjectNotFound)?;
        if episode_project != project_id.value() {
            return Err(Rejection::SubjectNotFound);
        }
    }
    let graph_object_id = match existing_graph_object_id {
        Some(graph_object_id) => {
            let (object_project, stored_kind): (i64, i64) = transaction
                .query_row(
                    "SELECT project_id, object_kind FROM objects WHERE graph_object_id = ?1",
                    [graph_object_id.value()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|_| Rejection::SubjectNotFound)?
                .ok_or(Rejection::SubjectNotFound)?;
            if object_project != project_id.value() || stored_kind != object_kind as i64 {
                return Err(Rejection::SubjectNotFound);
            }
            graph_object_id
        }
        None => {
            transaction.execute(
                "INSERT INTO objects(project_id, causal_episode_id, universe_seed_id, object_kind, created_by_command_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![project_id.value(), causal_episode_id.map(CausalEpisodeId::value), cycle.seed_id.value(), object_kind as i64, command_row_id],
            ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
            id_from_last_insert::<GraphObjectId>(transaction)?
        }
    };
    let next_ordinal: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(revision_ordinal), 0) + 1 FROM object_revisions WHERE graph_object_id = ?1",
        [graph_object_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction.execute(
        "INSERT INTO object_revisions(graph_object_id, revision_ordinal, lifecycle_state, created_by_command_id, committed_by_command_id)
         VALUES (?1, ?2, 1, ?3, NULL)",
        params![graph_object_id.value(), next_ordinal, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let graph_revision_id = id_from_last_insert::<GraphRevisionId>(transaction)?;
    match body {
        GraphRevisionBody::Observation { observation } => {
            transaction.execute(
                "INSERT INTO observation_revisions(graph_revision_id, observation_text) VALUES (?1, ?2)",
                params![graph_revision_id.value(), observation.as_str()],
            ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
        }
        GraphRevisionBody::Hypothesis { hypothesis } => {
            transaction.execute(
                "INSERT INTO hypothesis_revisions(graph_revision_id, hypothesis_text) VALUES (?1, ?2)",
                params![graph_revision_id.value(), hypothesis.as_str()],
            ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
        }
    }
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::GraphObjectRevisionAdded {
        graph_object_id,
        graph_revision_id,
    })
}

fn commit_graph_revision(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    graph_revision_id: GraphRevisionId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (_, project_id, _, state) = graph_revision_row(transaction, graph_revision_id)?;
    project_is_active(transaction, project_id)?;
    if state != GraphRevisionState::Draft {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE object_revisions SET lifecycle_state = 2, committed_by_command_id = ?1 WHERE graph_revision_id = ?2",
        params![command_row_id, graph_revision_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::GraphRevisionCommitted { graph_revision_id })
}

#[allow(clippy::too_many_arguments)]
fn add_graph_edge(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    from_graph_revision_id: GraphRevisionId,
    to_graph_revision_id: GraphRevisionId,
    edge_kind: GraphEdgeKind,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    let (_, from_project, from_kind, from_state) =
        graph_revision_row(transaction, from_graph_revision_id)?;
    let (_, to_project, to_kind, to_state) = graph_revision_row(transaction, to_graph_revision_id)?;
    if from_project != project_id || to_project != project_id {
        return Err(Rejection::SubjectNotFound);
    }
    if from_state != GraphRevisionState::Committed || to_state != GraphRevisionState::Committed {
        return Err(Rejection::GraphRevisionNotCommitted);
    }
    if !edge_kind.allows(from_kind, to_kind) {
        return Err(Rejection::IllegalGraphEdgeEndpoint);
    }
    transaction.execute(
        "INSERT INTO edges(project_id, from_graph_revision_id, to_graph_revision_id, edge_kind, created_by_command_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![project_id.value(), from_graph_revision_id.value(), to_graph_revision_id.value(), edge_kind as i64, command_row_id],
    ).map_err(|_| Rejection::IllegalGraphEdgeEndpoint)?;
    let graph_edge_id = id_from_last_insert::<GraphEdgeId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::GraphEdgeAdded { graph_edge_id })
}

fn episode_row(
    transaction: &Transaction<'_>,
    episode_id: CausalEpisodeId,
) -> Result<(ProjectId, EpisodeState), Rejection> {
    let row: (i64, i64) = transaction
        .query_row(
            "SELECT project_id, lifecycle_state FROM episodes WHERE causal_episode_id = ?1",
            [episode_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        ProjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        episode_state_from_i64(row.1).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn episode_transition_allowed(from: EpisodeState, to: EpisodeState) -> bool {
    matches!(
        (from, to),
        (
            EpisodeState::Framed,
            EpisodeState::Admitted | EpisodeState::Abandoned
        ) | (
            EpisodeState::Admitted,
            EpisodeState::Investigating | EpisodeState::Abandoned
        ) | (
            EpisodeState::Investigating,
            EpisodeState::PrototypeDeliberating
                | EpisodeState::ClosedNoAction
                | EpisodeState::Abandoned
        ) | (
            EpisodeState::PrototypeDeliberating,
            EpisodeState::Prototyping | EpisodeState::ClosedNoAction | EpisodeState::Abandoned
        ) | (
            EpisodeState::Prototyping,
            EpisodeState::CandidateValidating | EpisodeState::Reverted | EpisodeState::Abandoned
        ) | (
            EpisodeState::CandidateValidating,
            EpisodeState::DeliveryDeliberating | EpisodeState::Reverted | EpisodeState::Abandoned
        ) | (
            EpisodeState::DeliveryDeliberating,
            EpisodeState::DeliveryAuthorized
                | EpisodeState::ClosedNoDelivery
                | EpisodeState::Abandoned
        ) | (
            EpisodeState::DeliveryAuthorized,
            EpisodeState::Materializing | EpisodeState::Abandoned
        ) | (
            EpisodeState::Materializing,
            EpisodeState::Observing | EpisodeState::Abandoned
        ) | (
            EpisodeState::Observing,
            EpisodeState::Learning | EpisodeState::Closed | EpisodeState::Abandoned
        ) | (
            EpisodeState::Learning,
            EpisodeState::Closed | EpisodeState::Abandoned
        ) | (
            EpisodeState::Reopened,
            EpisodeState::Investigating | EpisodeState::Abandoned
        )
    )
}

fn create_episode(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    transaction.execute(
        "INSERT INTO episodes(project_id, universe_seed_id, lifecycle_state, created_by_command_id, last_transition_command_id) VALUES (?1, ?2, 1, ?3, ?3)",
        params![project_id.value(), cycle.seed_id.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let causal_episode_id = id_from_last_insert::<CausalEpisodeId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::EpisodeCreated {
        causal_episode_id,
        project_id,
    })
}

fn transition_episode(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    causal_episode_id: CausalEpisodeId,
    target: EpisodeState,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = episode_row(transaction, causal_episode_id)?;
    if !episode_transition_allowed(state, target) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("UPDATE episodes SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE causal_episode_id = ?3", params![target as i64, command_row_id, causal_episode_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::EpisodeStateChanged {
        causal_episode_id,
        state: target,
    })
}

fn reopen_episode(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    causal_episode_id: CausalEpisodeId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = episode_row(transaction, causal_episode_id)?;
    if !matches!(
        state,
        EpisodeState::Closed
            | EpisodeState::ClosedNoAction
            | EpisodeState::ClosedNoDelivery
            | EpisodeState::Reverted
    ) {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("UPDATE episodes SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE causal_episode_id = ?3", params![EpisodeState::Reopened as i64, command_row_id, causal_episode_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::EpisodeStateChanged {
        causal_episode_id,
        state: EpisodeState::Reopened,
    })
}

fn review_row(
    transaction: &Transaction<'_>,
    review_id: AdversarialReviewId,
) -> Result<(ProjectId, GraphRevisionId, AdversarialReviewState), Rejection> {
    let row: (i64, i64, i64) = transaction.query_row(
        "SELECT project_id, target_graph_revision_id, lifecycle_state FROM adversarial_reviews WHERE adversarial_review_id = ?1",
        [review_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        ProjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        GraphRevisionId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        adversarial_review_state_from_i64(row.2).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn request_adversarial_review(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    target_graph_revision_id: GraphRevisionId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    let (_, revision_project, _, revision_state) =
        graph_revision_row(transaction, target_graph_revision_id)?;
    if revision_project != project_id {
        return Err(Rejection::SubjectNotFound);
    }
    if revision_state != GraphRevisionState::Committed {
        return Err(Rejection::GraphRevisionNotCommitted);
    }
    transaction.execute(
        "INSERT INTO adversarial_reviews(project_id, target_graph_revision_id, lifecycle_state, requested_by_command_id, assigned_reviewer_principal_id, resolved_by_command_id) VALUES (?1, ?2, 1, ?3, NULL, NULL)",
        params![project_id.value(), target_graph_revision_id.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let adversarial_review_id = id_from_last_insert::<AdversarialReviewId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::AdversarialReviewRequested {
        adversarial_review_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn assign_adversarial_reviewer(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    adversarial_review_id: AdversarialReviewId,
    reviewer_principal_id: PrincipalId,
    reviewer_actor_instance_id: ActorInstanceId,
    reviewer_actor_attempt_id: ActorAttemptId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, _, review_state) = review_row(transaction, adversarial_review_id)?;
    if review_state != AdversarialReviewState::Requested {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let cycle_grand_architect: i64 = transaction
        .query_row(
            "SELECT principal_id FROM office_occupancies WHERE office_occupancy_id = ?1",
            [cycle.occupancy_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    if reviewer_principal_id.value() == cycle_grand_architect {
        return Err(Rejection::ReviewAssignmentNotIndependent);
    }
    let (actor_principal, _, _, actor_cycle, actor_state) =
        actor_instance_row(transaction, reviewer_actor_instance_id)?;
    if actor_principal != reviewer_principal_id
        || actor_cycle != operating_cycle_id
        || actor_state != ActorInstanceState::Active
    {
        return Err(Rejection::ReviewAssignmentEvidenceMissing);
    }
    let (attempt_cycle, ticket_id, work_item_id, _, attempt_actor, attempt_state) =
        actor_attempt_row(transaction, reviewer_actor_attempt_id)?;
    let (ticket_project, _) = ticket_row(transaction, ticket_id)?;
    let (_, _, context_pack_id, work_kind, bound_review_id, _, _) =
        work_item_row(transaction, work_item_id)?;
    let context_purpose: i64 = transaction
        .query_row(
            "SELECT purpose FROM context_packs WHERE context_pack_id = ?1",
            [context_pack_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ReviewAssignmentEvidenceMissing)?;
    if attempt_cycle != operating_cycle_id
        || attempt_actor != reviewer_actor_instance_id
        || ticket_project != project_id
        || work_kind != WorkItemKind::IndependentReview
        || bound_review_id != Some(adversarial_review_id)
        || context_pack_purpose_from_i64(context_purpose)
            .map_err(|_| Rejection::ReviewAssignmentEvidenceMissing)?
            != ContextPackPurpose::IndependentReview
        || !matches!(
            attempt_state,
            ActorAttemptState::Succeeded | ActorAttemptState::Validated
        )
    {
        return Err(Rejection::ReviewAssignmentEvidenceMissing);
    }
    // The service assigns a named, active Actor. The author check at finding
    // submission compares against this durable assignment; Principal kind is
    // only a prerequisite, never reviewer jurisdiction by itself.
    let eligible: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM principals WHERE principal_id = ?1 AND principal_kind = ?2 AND active = 1)",
            params![reviewer_principal_id.value(), PrincipalKind::Actor as i64],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?
        != 0;
    if !eligible {
        return Err(Rejection::SubjectNotFound);
    }
    transaction
        .execute(
            "UPDATE adversarial_reviews SET lifecycle_state = ?1, assigned_reviewer_principal_id = ?2, assigned_reviewer_actor_instance_id = ?3, reviewer_actor_attempt_id = ?4 WHERE adversarial_review_id = ?5",
            params![
                AdversarialReviewState::Assigned as i64,
                reviewer_principal_id.value(),
                reviewer_actor_instance_id.value(),
                reviewer_actor_attempt_id.value(),
                adversarial_review_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::AdversarialReviewerAssigned {
        adversarial_review_id,
        reviewer_principal_id,
        reviewer_actor_instance_id,
        reviewer_actor_attempt_id,
    })
}

/// A Review finding is submitted by the kernel service on behalf of the exact
/// independently provisioned Actor named by assignment evidence. M3 provides
/// the minimum WorkItem/Attempt foundation for resolution; Pi/process evidence
/// remains outside the trusted claim until the supervisor receipt tranche.
#[allow(clippy::too_many_arguments)]
fn submit_review_challenge(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    author_principal_id: PrincipalId,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    adversarial_review_id: AdversarialReviewId,
    target_graph_revision_id: GraphRevisionId,
    severity: ReviewChallengeSeverity,
    failure_hypothesis: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, review_target, review_state) = review_row(transaction, adversarial_review_id)?;
    let assigned_reviewer: (Option<i64>, Option<i64>) = transaction
        .query_row(
            "SELECT assigned_reviewer_principal_id, assigned_reviewer_actor_instance_id FROM adversarial_reviews WHERE adversarial_review_id = ?1",
            [adversarial_review_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    if assigned_reviewer.0 != Some(author_principal_id.value()) {
        return Err(Rejection::CapabilityNotGranted);
    }
    let Some(assigned_actor_instance_id) = assigned_reviewer.1 else {
        return Err(Rejection::ReviewAssignmentEvidenceMissing);
    };
    let (assigned_principal, _, _, assigned_cycle, assigned_state) = actor_instance_row(
        transaction,
        ActorInstanceId::try_from(assigned_actor_instance_id)
            .map_err(|_| Rejection::ReviewAssignmentEvidenceMissing)?,
    )?;
    if assigned_principal != author_principal_id
        || assigned_cycle != operating_cycle_id
        || assigned_state != ActorInstanceState::Active
    {
        return Err(Rejection::ReviewAssignmentEvidenceMissing);
    }
    if review_target != target_graph_revision_id
        || !matches!(
            review_state,
            AdversarialReviewState::Assigned
                | AdversarialReviewState::Active
                | AdversarialReviewState::FindingsSubmitted
                | AdversarialReviewState::ResponsesDue
        )
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let (_, revision_project, _, revision_state) =
        graph_revision_row(transaction, target_graph_revision_id)?;
    if revision_project != project_id {
        return Err(Rejection::SubjectNotFound);
    }
    if revision_state != GraphRevisionState::Committed {
        return Err(Rejection::GraphRevisionNotCommitted);
    }
    transaction.execute(
        "INSERT INTO review_challenges(adversarial_review_id, target_graph_revision_id, author_principal_id, severity, failure_hypothesis, response_state, submitted_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
        params![adversarial_review_id.value(), target_graph_revision_id.value(), author_principal_id.value(), severity as i64, failure_hypothesis, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let review_challenge_id = id_from_last_insert::<ReviewChallengeId>(transaction)?;
    transaction
        .execute(
            "UPDATE adversarial_reviews SET lifecycle_state = 5 WHERE adversarial_review_id = ?1",
            [adversarial_review_id.value()],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ReviewChallengeSubmitted {
        review_challenge_id,
        author_principal_id,
    })
}

fn review_challenge_row(
    transaction: &Transaction<'_>,
    challenge_id: ReviewChallengeId,
) -> Result<
    (
        AdversarialReviewId,
        ProjectId,
        PrincipalId,
        ReviewChallengeResponseState,
    ),
    Rejection,
> {
    let row: (i64, i64, i64, i64) = transaction.query_row(
        "SELECT c.adversarial_review_id, r.project_id, c.author_principal_id, c.response_state
         FROM review_challenges c JOIN adversarial_reviews r ON r.adversarial_review_id = c.adversarial_review_id
         WHERE c.review_challenge_id = ?1",
        [challenge_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        AdversarialReviewId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        ProjectId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        PrincipalId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        review_challenge_response_state_from_i64(row.3).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn respond_to_review_challenge(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    review_challenge_id: ReviewChallengeId,
    response: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (_, project_id, _, response_state) =
        review_challenge_row(transaction, review_challenge_id)?;
    if response_state != ReviewChallengeResponseState::Pending {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("INSERT INTO review_challenge_responses(review_challenge_id, response_text, responded_by_command_id) VALUES (?1, ?2, ?3)", params![review_challenge_id.value(), response, command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction
        .execute(
            "UPDATE review_challenges SET response_state = 2 WHERE review_challenge_id = ?1",
            [review_challenge_id.value()],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ReviewChallengeResponded {
        review_challenge_id,
    })
}

fn disposition_review_challenge(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    principal_id: PrincipalId,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    review_challenge_id: ReviewChallengeId,
    disposition: ReviewDispositionKind,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (_, project_id, author, response_state) =
        review_challenge_row(transaction, review_challenge_id)?;
    if principal_id == author {
        return Err(Rejection::ReviewSelfDispositionDenied);
    }
    if response_state != ReviewChallengeResponseState::Responded {
        return Err(Rejection::ReviewDispositionIncomplete);
    }
    transaction.execute("INSERT INTO review_dispositions(review_challenge_id, disposition_kind, disposed_by_principal_id, disposed_by_command_id) VALUES (?1, ?2, ?3, ?4)", params![review_challenge_id.value(), disposition as i64, principal_id.value(), command_row_id]).map_err(|_| Rejection::ReviewDispositionIncomplete)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ReviewChallengeDispositioned {
        review_challenge_id,
        disposition,
    })
}

fn resolve_adversarial_review(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    adversarial_review_id: AdversarialReviewId,
    resolution: ReviewResolutionKind,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, _, state) = review_row(transaction, adversarial_review_id)?;
    if state != AdversarialReviewState::ResponsesDue {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let missing: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM review_challenges c LEFT JOIN review_dispositions d ON d.review_challenge_id = c.review_challenge_id
         WHERE c.adversarial_review_id = ?1 AND (c.response_state != 2 OR d.review_disposition_id IS NULL)",
        [adversarial_review_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    if missing != 0 {
        return Err(Rejection::ReviewDispositionIncomplete);
    }
    let target_state = match resolution {
        ReviewResolutionKind::Resolved => AdversarialReviewState::Resolved,
        ReviewResolutionKind::AcceptedRisk => AdversarialReviewState::AcceptedRisk,
    };
    transaction.execute("UPDATE adversarial_reviews SET lifecycle_state = ?1, resolved_by_command_id = ?2 WHERE adversarial_review_id = ?3", params![target_state as i64, command_row_id, adversarial_review_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::AdversarialReviewResolved {
        adversarial_review_id,
        state: target_state,
    })
}

fn postmortem_row(
    transaction: &Transaction<'_>,
    postmortem_id: PostmortemId,
) -> Result<(ProjectId, PostmortemState), Rejection> {
    let row: (i64, i64) = transaction
        .query_row(
            "SELECT project_id, lifecycle_state FROM postmortems WHERE postmortem_id = ?1",
            [postmortem_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        ProjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        postmortem_state_from_i64(row.1).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn trigger_postmortem(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    causal_episode_id: Option<CausalEpisodeId>,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    if let Some(episode) = causal_episode_id
        && episode_row(transaction, episode)?.0 != project_id
    {
        return Err(Rejection::SubjectNotFound);
    }
    transaction.execute("INSERT INTO postmortems(project_id, causal_episode_id, universe_seed_id, lifecycle_state, triggered_by_command_id, closed_by_command_id) VALUES (?1, ?2, ?3, 1, ?4, NULL)", params![project_id.value(), causal_episode_id.map(CausalEpisodeId::value), cycle.seed_id.value(), command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let postmortem_id = id_from_last_insert::<PostmortemId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::PostmortemTriggered { postmortem_id })
}

fn record_postmortem_causal_claim(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    postmortem_id: PostmortemId,
    claim_kind: PostmortemCausalClaimKind,
    claim: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = postmortem_row(transaction, postmortem_id)?;
    if state == PostmortemState::Closed {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("INSERT INTO postmortem_causal_claims(postmortem_id, claim_kind, claim_text, recorded_by_command_id) VALUES (?1, ?2, ?3, ?4)", params![postmortem_id.value(), claim_kind as i64, claim, command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let postmortem_causal_claim_id = id_from_last_insert::<PostmortemCausalClaimId>(transaction)?;
    transaction
        .execute(
            "UPDATE postmortems SET lifecycle_state = 2 WHERE postmortem_id = ?1",
            [postmortem_id.value()],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::PostmortemCausalClaimRecorded {
        postmortem_causal_claim_id,
    })
}

fn propose_postmortem_action(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    postmortem_id: PostmortemId,
    action_kind: PostmortemActionKind,
    action: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = postmortem_row(transaction, postmortem_id)?;
    if state == PostmortemState::Closed {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute("INSERT INTO postmortem_action_proposals(postmortem_id, action_kind, action_text, proposed_by_command_id) VALUES (?1, ?2, ?3, ?4)", params![postmortem_id.value(), action_kind as i64, action, command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let postmortem_action_proposal_id =
        id_from_last_insert::<PostmortemActionProposalId>(transaction)?;
    transaction
        .execute(
            "UPDATE postmortems SET lifecycle_state = 2 WHERE postmortem_id = ?1",
            [postmortem_id.value()],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::PostmortemActionProposed {
        postmortem_action_proposal_id,
    })
}

fn close_postmortem(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    postmortem_id: PostmortemId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = postmortem_row(transaction, postmortem_id)?;
    if state == PostmortemState::Closed {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let counts: (i64, i64) = transaction.query_row(
        "SELECT (SELECT COUNT(*) FROM postmortem_causal_claims WHERE postmortem_id = ?1), (SELECT COUNT(*) FROM postmortem_action_proposals WHERE postmortem_id = ?1)",
        [postmortem_id.value()], |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    if counts.0 == 0 || counts.1 == 0 {
        return Err(Rejection::PostmortemCloseBlocked);
    }
    transaction.execute("UPDATE postmortems SET lifecycle_state = 3, closed_by_command_id = ?1 WHERE postmortem_id = ?2", params![command_row_id, postmortem_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::PostmortemClosed { postmortem_id })
}

/// M3 is the first executable, but deliberately receipt-free, task boundary.
/// Its kernel-service terminal attestations are atomic trusted facts only; the
/// later supervisor tranche must bind them to Pi/process/evidence receipts.
fn register_actor_configuration(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    configuration_name: &str,
    model_policy: ActorModelPolicy,
    primary_attractor: DevelopmentalAttractor,
) -> Result<EventBody, Rejection> {
    transaction.execute(
        "INSERT INTO actor_configurations(configuration_name, lifecycle_state, created_by_command_id) VALUES (?1, 1, ?2)",
        params![configuration_name, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let actor_configuration_id = id_from_last_insert::<ActorConfigurationId>(transaction)?;
    transaction.execute(
        "INSERT INTO actor_configuration_revisions(actor_configuration_id, revision_ordinal, model_policy, primary_attractor, created_by_command_id) VALUES (?1, 1, ?2, ?3, ?4)",
        params![actor_configuration_id.value(), model_policy as i64, primary_attractor as i64, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::ActorConfigurationRegistered {
        actor_configuration_id,
        actor_configuration_revision_id: id_from_last_insert::<ActorConfigurationRevisionId>(
            transaction,
        )?,
    })
}

fn register_context_pack(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    purpose: ContextPackPurpose,
    rendering_digest: Sha256Digest,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    transaction.execute(
        "INSERT INTO context_packs(universe_seed_id, purpose, rendering_digest, created_by_command_id) VALUES (?1, ?2, ?3, ?4)",
        params![cycle.seed_id.value(), purpose as i64, rendering_digest.as_bytes(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let context_pack_id = id_from_last_insert::<ContextPackId>(transaction)?;
    record_coordination_provenance(transaction, command_row_id, cycle, operating_cycle_id, None)?;
    Ok(EventBody::ContextPackRegistered { context_pack_id })
}

fn admit_actor_instance(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    actor_configuration_revision_id: ActorConfigurationRevisionId,
    execution_profile_id: ExecutionProfileId,
    actor_display_name: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let configuration_active: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM actor_configuration_revisions r JOIN actor_configurations c ON c.actor_configuration_id = r.actor_configuration_id WHERE r.actor_configuration_revision_id = ?1 AND c.lifecycle_state = 1)",
        [actor_configuration_revision_id.value()], |row| row.get::<_, i64>(0),
    ).map_err(|_| Rejection::SubjectNotFound)? != 0;
    let profile: Option<(i64, i64)> = transaction
        .query_row(
            "SELECT profile_kind, readiness FROM execution_profiles WHERE execution_profile_id = ?1",
            [execution_profile_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?;
    let profile_is_admissible = match profile {
        Some((kind, readiness)) => matches!(
            (
                cycle._treatment,
                execution_profile_kind_from_i64(kind),
                execution_profile_readiness_from_i64(readiness),
            ),
            (
                OperatingCycleTreatment::Vs001DeterministicV1,
                Ok(ExecutionProfileKind::DeterministicPiHostProcessDoubleV1),
                Ok(ExecutionProfileReadiness::DeterministicFixtureOnly),
            ) | (
                OperatingCycleTreatment::Vs001LiveV1,
                Ok(ExecutionProfileKind::NativePinnedPiSdkV1),
                Ok(ExecutionProfileReadiness::QualifiedForLiveUse),
            )
        ),
        None => false,
    };
    if !configuration_active {
        return Err(Rejection::SubjectNotFound);
    }
    if !profile_is_admissible {
        return Err(Rejection::ExecutionProfileIneligible);
    }
    transaction
        .execute(
            "INSERT INTO principals(principal_kind, display_name, active) VALUES (?1, ?2, 1)",
            params![PrincipalKind::Actor as i64, actor_display_name],
        )
        .map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let principal_id = id_from_last_insert::<PrincipalId>(transaction)?;
    transaction.execute(
        "INSERT INTO actor_instances(principal_id, actor_configuration_revision_id, execution_profile_id, operating_cycle_id, lifecycle_state, admitted_by_command_id) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
        params![principal_id.value(), actor_configuration_revision_id.value(), execution_profile_id.value(), operating_cycle_id.value(), command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let actor_instance_id = id_from_last_insert::<ActorInstanceId>(transaction)?;
    transaction.execute(
        "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id, actor_instance_id, grant_state, grant_origin, granted_by_command_id, consumed_by_command_id) VALUES (?1, ?2, NULL, ?3, 1, 2, ?4, NULL)",
        params![principal_id.value(), Capability::ClaimWorkItem as i64, actor_instance_id.value(), command_row_id],
    ).map_err(|_| Rejection::ActorJurisdictionDenied)?;
    record_coordination_provenance(transaction, command_row_id, cycle, operating_cycle_id, None)?;
    Ok(EventBody::ActorInstanceAdmitted {
        actor_instance_id,
        principal_id,
    })
}

fn admit_ticket(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    ticket_id: TicketId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, state) = ticket_row(transaction, ticket_id)?;
    project_is_active(transaction, project_id)?;
    if state != TicketState::Draft {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction.execute(
        "UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3",
        params![TicketState::Admitted as i64, command_row_id, ticket_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::TicketAdmitted { ticket_id })
}

fn actor_instance_row(
    transaction: &Transaction<'_>,
    actor_instance_id: ActorInstanceId,
) -> Result<
    (
        PrincipalId,
        ActorConfigurationRevisionId,
        ExecutionProfileId,
        OperatingCycleId,
        ActorInstanceState,
    ),
    Rejection,
> {
    let row: (i64, i64, i64, i64, i64) = transaction.query_row(
        "SELECT principal_id, actor_configuration_revision_id, execution_profile_id, operating_cycle_id, lifecycle_state FROM actor_instances WHERE actor_instance_id = ?1",
        [actor_instance_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        PrincipalId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        ActorConfigurationRevisionId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        ExecutionProfileId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        OperatingCycleId::try_from(row.3).map_err(|_| Rejection::SubjectNotFound)?,
        actor_instance_state_from_i64(row.4).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn register_work_item(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    ticket_id: TicketId,
    actor_instance_id: ActorInstanceId,
    context_pack_id: ContextPackId,
    work_kind: WorkItemKind,
    adversarial_review_id: Option<AdversarialReviewId>,
    assignment: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (project_id, ticket_state) = ticket_row(transaction, ticket_id)?;
    project_is_active(transaction, project_id)?;
    if ticket_state != TicketState::Admitted
        || !ticket_prerequisites_complete(transaction, ticket_id)?
    {
        return Err(Rejection::TicketPrerequisiteIncomplete);
    }
    let (_, _, _, actor_cycle, actor_state) = actor_instance_row(transaction, actor_instance_id)?;
    if actor_cycle != operating_cycle_id || actor_state != ActorInstanceState::Active {
        return Err(Rejection::ActorJurisdictionDenied);
    }
    let context: (i64, i64) = transaction
        .query_row(
            "SELECT universe_seed_id, purpose FROM context_packs WHERE context_pack_id = ?1",
            [context_pack_id.value()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    if context.0 != cycle.seed_id.value()
        || context_pack_purpose_from_i64(context.1).map_err(|_| Rejection::SubjectNotFound)?
            != work_kind.required_context_purpose()
    {
        return Err(Rejection::ActorJurisdictionDenied);
    }
    match (work_kind, adversarial_review_id) {
        (WorkItemKind::TicketExecution, None) => {}
        (WorkItemKind::IndependentReview, Some(review_id)) => {
            let (review_project_id, target_revision_id, review_state) =
                review_row(transaction, review_id)?;
            let (_, revision_project_id, _, revision_state) =
                graph_revision_row(transaction, target_revision_id)?;
            if review_project_id != project_id
                || review_state != AdversarialReviewState::Requested
                || revision_project_id != project_id
                || revision_state != GraphRevisionState::Committed
            {
                return Err(Rejection::ReviewAssignmentEvidenceMissing);
            }
        }
        _ => return Err(Rejection::ReviewAssignmentEvidenceMissing),
    }
    transaction.execute(
        "INSERT INTO work_items(ticket_id, actor_instance_id, context_pack_id, work_kind, adversarial_review_id, assignment_text, lifecycle_state, retry_of_actor_attempt_id, created_by_command_id, last_transition_command_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, NULL, ?7, ?7)",
        params![ticket_id.value(), actor_instance_id.value(), context_pack_id.value(), work_kind as i64, adversarial_review_id.map(AdversarialReviewId::value), assignment, command_row_id],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let work_item_id = id_from_last_insert::<WorkItemId>(transaction)?;
    transaction.execute(
        "UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3",
        params![TicketState::Ready as i64, command_row_id, ticket_id.value()],
    ).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::WorkItemRegistered {
        work_item_id,
        ticket_id,
        adversarial_review_id,
    })
}

fn work_item_row(
    transaction: &Transaction<'_>,
    work_item_id: WorkItemId,
) -> Result<WorkItemRow, Rejection> {
    let row: (i64, i64, i64, i64, Option<i64>, i64, Option<i64>) = transaction.query_row(
        "SELECT ticket_id, actor_instance_id, context_pack_id, work_kind, adversarial_review_id, lifecycle_state, retry_of_actor_attempt_id FROM work_items WHERE work_item_id = ?1",
        [work_item_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        TicketId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        ActorInstanceId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        ContextPackId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        work_item_kind_from_i64(row.3).map_err(|_| Rejection::SubjectNotFound)?,
        row.4
            .map(AdversarialReviewId::try_from)
            .transpose()
            .map_err(|_| Rejection::SubjectNotFound)?,
        work_item_state_from_i64(row.5).map_err(|_| Rejection::SubjectNotFound)?,
        row.6
            .map(ActorAttemptId::try_from)
            .transpose()
            .map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn claim_work_item(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    principal_id: PrincipalId,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    work_item_id: WorkItemId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (ticket_id, actor_instance_id, _, _, _, state, _) =
        work_item_row(transaction, work_item_id)?;
    let (actor_principal, _, _, actor_cycle, actor_state) =
        actor_instance_row(transaction, actor_instance_id)?;
    let (project_id, ticket_state) = ticket_row(transaction, ticket_id)?;
    project_is_active(transaction, project_id)?;
    if principal_id != actor_principal
        || actor_cycle != operating_cycle_id
        || actor_state != ActorInstanceState::Active
    {
        return Err(Rejection::ActorJurisdictionDenied);
    }
    if state != WorkItemState::Ready || ticket_state != TicketState::Ready {
        return Err(Rejection::WorkLeaseUnavailable);
    }
    transaction.execute(
        "INSERT INTO leases(work_item_id, actor_instance_id, lifecycle_state, claimed_by_command_id, terminal_by_command_id) VALUES (?1, ?2, 1, ?3, NULL)",
        params![work_item_id.value(), actor_instance_id.value(), command_row_id],
    ).map_err(|_| Rejection::WorkLeaseUnavailable)?;
    let work_lease_id = id_from_last_insert::<WorkLeaseId>(transaction)?;
    transaction.execute("UPDATE work_items SET lifecycle_state = 2, last_transition_command_id = ?1 WHERE work_item_id = ?2", params![command_row_id, work_item_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![TicketState::Claimed as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::WorkItemClaimed {
        work_item_id,
        work_lease_id,
        actor_instance_id,
    })
}

fn reserve_attempt_budget(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    cycle: CycleRow,
    cycle_id: OperatingCycleId,
    amount: UsdMicros,
) -> Result<BudgetReservationId, Rejection> {
    // M3 reserves only the governing society and Operating Cycle envelopes.
    // It is a provisional execution-boundary reservation, not a claim that
    // every future VS-001 accounting dimension has been modeled.
    if amount == UsdMicros::ZERO {
        return Err(Rejection::BudgetCeilingExceeded);
    }
    let (society_budget, cycle_budget) =
        budget_envelopes_for_cycle(transaction, cycle.society_id, cycle_id)?;
    for budget_id in [society_budget, cycle_budget] {
        let (ceiling, reserved, spent) = budget_amounts(transaction, budget_id)?;
        let Some(next_reserved) = reserved.checked_add(amount) else {
            return Err(Rejection::BudgetCeilingExceeded);
        };
        if next_reserved
            .checked_add(spent)
            .is_none_or(|value| value > ceiling)
        {
            return Err(Rejection::BudgetCeilingExceeded);
        }
    }
    transaction.execute("INSERT INTO budget_reservations(operating_cycle_id, amount_micros, reservation_state, reserved_by_command_id, reconciled_by_command_id) VALUES (?1, ?2, 1, ?3, NULL)", params![cycle_id.value(), amount.value(), command_row_id]).map_err(|_| Rejection::BudgetCeilingExceeded)?;
    let reservation_id = id_from_last_insert::<BudgetReservationId>(transaction)?;
    for budget_id in [society_budget, cycle_budget] {
        transaction.execute("UPDATE budget_envelopes SET reserved_micros = reserved_micros + ?1 WHERE budget_envelope_id = ?2", params![amount.value(), budget_id.value()]).map_err(|_| Rejection::BudgetCeilingExceeded)?;
        transaction.execute("INSERT INTO budget_reservation_charges(budget_reservation_id, budget_envelope_id, amount_micros) VALUES (?1, ?2, ?3)", params![reservation_id.value(), budget_id.value(), amount.value()]).map_err(|_| Rejection::BudgetCeilingExceeded)?;
    }
    Ok(reservation_id)
}

fn start_actor_attempt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    work_item_id: WorkItemId,
    reservation_amount: UsdMicros,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (
        ticket_id,
        actor_instance_id,
        context_pack_id,
        _,
        _,
        work_state,
        retry_of_actor_attempt_id,
    ) = work_item_row(transaction, work_item_id)?;
    let (_, configuration_revision_id, execution_profile_id, actor_cycle, actor_state) =
        actor_instance_row(transaction, actor_instance_id)?;
    let (project_id, ticket_state) = ticket_row(transaction, ticket_id)?;
    project_is_active(transaction, project_id)?;
    if work_state != WorkItemState::Claimed
        || ticket_state != TicketState::Claimed
        || actor_cycle != operating_cycle_id
        || actor_state != ActorInstanceState::Active
    {
        return Err(Rejection::WorkLeaseUnavailable);
    }
    let lease_id: i64 = transaction.query_row("SELECT work_lease_id FROM leases WHERE work_item_id = ?1 AND actor_instance_id = ?2 AND lifecycle_state = 1", params![work_item_id.value(), actor_instance_id.value()], |row| row.get(0)).optional().map_err(|_| Rejection::WorkLeaseUnavailable)?.ok_or(Rejection::WorkLeaseUnavailable)?;
    let work_lease_id =
        WorkLeaseId::try_from(lease_id).map_err(|_| Rejection::WorkLeaseUnavailable)?;
    let reservation_id = reserve_attempt_budget(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        reservation_amount,
    )?;
    transaction.execute("INSERT INTO attempts(operating_cycle_id, ticket_id, work_item_id, work_lease_id, actor_instance_id, actor_configuration_revision_id, execution_profile_id, context_pack_id, retry_of_actor_attempt_id, lifecycle_state, started_by_command_id, terminal_by_command_id, validated_by_command_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, NULL, NULL)", params![operating_cycle_id.value(), ticket_id.value(), work_item_id.value(), work_lease_id.value(), actor_instance_id.value(), configuration_revision_id.value(), execution_profile_id.value(), context_pack_id.value(), retry_of_actor_attempt_id.map(ActorAttemptId::value), command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let actor_attempt_id = id_from_last_insert::<ActorAttemptId>(transaction)?;
    transaction.execute("INSERT INTO attempt_budget_reservations(actor_attempt_id, budget_reservation_id, project_id, ticket_id) VALUES (?1, ?2, ?3, ?4)", params![actor_attempt_id.value(), reservation_id.value(), project_id.value(), ticket_id.value()]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    transaction.execute("UPDATE work_items SET lifecycle_state = 3, last_transition_command_id = ?1 WHERE work_item_id = ?2", params![command_row_id, work_item_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ActorAttemptStarted {
        actor_attempt_id,
        work_item_id,
        budget_reservation_id: reservation_id,
    })
}

fn actor_attempt_row(
    transaction: &Transaction<'_>,
    actor_attempt_id: ActorAttemptId,
) -> Result<
    (
        OperatingCycleId,
        TicketId,
        WorkItemId,
        WorkLeaseId,
        ActorInstanceId,
        ActorAttemptState,
    ),
    Rejection,
> {
    let row: (i64, i64, i64, i64, i64, i64) = transaction.query_row("SELECT operating_cycle_id, ticket_id, work_item_id, work_lease_id, actor_instance_id, lifecycle_state FROM attempts WHERE actor_attempt_id = ?1", [actor_attempt_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        OperatingCycleId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        TicketId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        WorkItemId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        WorkLeaseId::try_from(row.3).map_err(|_| Rejection::SubjectNotFound)?,
        ActorInstanceId::try_from(row.4).map_err(|_| Rejection::SubjectNotFound)?,
        actor_attempt_state_from_i64(row.5).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn attest_actor_attempt_terminal(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    actor_attempt_id: ActorAttemptId,
    terminal_kind: ActorAttemptTerminalKind,
) -> Result<EventBody, Rejection> {
    let (operating_cycle_id, ticket_id, work_item_id, work_lease_id, _, state) =
        actor_attempt_row(transaction, actor_attempt_id)?;
    if !terminal_kind.allowed_from(state) {
        return Err(Rejection::ActorAttemptNotTerminal);
    }
    transaction.execute("INSERT INTO actor_attempt_terminal_facts(actor_attempt_id, terminal_kind, attested_by_command_id) VALUES (?1, ?2, ?3)", params![actor_attempt_id.value(), terminal_kind as i64, command_row_id]).map_err(|_| Rejection::ActorAttemptNotTerminal)?;
    transaction.execute("UPDATE attempts SET lifecycle_state = ?1, terminal_by_command_id = ?2 WHERE actor_attempt_id = ?3", params![terminal_kind.state() as i64, command_row_id, actor_attempt_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    let lease_state = if terminal_kind == ActorAttemptTerminalKind::Cancelled {
        WorkLeaseState::Cancelled
    } else {
        WorkLeaseState::Released
    };
    transaction.execute("UPDATE leases SET lifecycle_state = ?1, terminal_by_command_id = ?2 WHERE work_lease_id = ?3 AND lifecycle_state = 1", params![lease_state as i64, command_row_id, work_lease_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE work_items SET lifecycle_state = 4, last_transition_command_id = ?1 WHERE work_item_id = ?2", params![command_row_id, work_item_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    let ticket_state = match terminal_kind {
        ActorAttemptTerminalKind::Succeeded => TicketState::Submitted,
        ActorAttemptTerminalKind::Cancelled => TicketState::Cancelled,
        ActorAttemptTerminalKind::Expired => TicketState::Ready,
        ActorAttemptTerminalKind::Failed
        | ActorAttemptTerminalKind::ProtocolFailed
        | ActorAttemptTerminalKind::SupervisorFailed => TicketState::Failed,
    };
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![ticket_state as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    let (project_id, _) = ticket_row(transaction, ticket_id)?;
    let cycle = cycle_row(transaction, operating_cycle_id)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ActorAttemptTerminalAttested {
        actor_attempt_id,
        terminal_kind,
    })
}

fn validate_ticket_attempt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    actor_attempt_id: ActorAttemptId,
) -> Result<EventBody, Rejection> {
    // This M3 kernel-service command is a receipt-free atomic fixture
    // attestation: it records that this exact Ticket acceptance condition was
    // satisfied. It is not VS evidence validation; a later evidence receipt
    // must refine this boundary rather than letting the Grand Architect
    // self-attest acceptance.
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (attempt_cycle, ticket_id, _, _, _, state) =
        actor_attempt_row(transaction, actor_attempt_id)?;
    let (project_id, ticket_state) = ticket_row(transaction, ticket_id)?;
    if attempt_cycle != operating_cycle_id
        || state != ActorAttemptState::Succeeded
        || ticket_state != TicketState::Submitted
    {
        return Err(Rejection::ActorAttemptNotValidatable);
    }
    let evidence_pending: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM deterministic_experiments WHERE ticket_id = ?1 AND lifecycle_state = 1",
        [ticket_id.value()], |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    if evidence_pending != 0 {
        return Err(Rejection::EvidenceAdmissionRequired);
    }
    transaction.execute("UPDATE attempts SET lifecycle_state = 9, validated_by_command_id = ?1 WHERE actor_attempt_id = ?2", params![command_row_id, actor_attempt_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE ticket_acceptance_conditions SET lifecycle_state = 2, satisfied_by_command_id = ?1 WHERE ticket_id = ?2 AND lifecycle_state = 1", params![command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![TicketState::Verified as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::TicketAttemptValidated {
        actor_attempt_id,
        ticket_id,
    })
}

fn retry_actor_attempt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    actor_attempt_id: ActorAttemptId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (attempt_cycle, ticket_id, work_item_id, _, _, state) =
        actor_attempt_row(transaction, actor_attempt_id)?;
    let (project_id, _) = ticket_row(transaction, ticket_id)?;
    project_is_active(transaction, project_id)?;
    if attempt_cycle != operating_cycle_id
        || !matches!(
            state,
            ActorAttemptState::Failed
                | ActorAttemptState::Cancelled
                | ActorAttemptState::Expired
                | ActorAttemptState::ProtocolFailed
                | ActorAttemptState::SupervisorFailed
        )
    {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    let (_, _, _, _, _, work_state, _) = work_item_row(transaction, work_item_id)?;
    if work_state != WorkItemState::Settled {
        return Err(Rejection::WorkLeaseUnavailable);
    }
    transaction.execute("UPDATE work_items SET lifecycle_state = 1, retry_of_actor_attempt_id = ?1, last_transition_command_id = ?2 WHERE work_item_id = ?3", params![actor_attempt_id.value(), command_row_id, work_item_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![TicketState::Ready as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ActorAttemptRetryPrepared {
        actor_attempt_id,
        work_item_id,
        ticket_id,
    })
}

fn complete_ticket(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    actor_attempt_id: ActorAttemptId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let (attempt_cycle, ticket_id, _, _, _, state) =
        actor_attempt_row(transaction, actor_attempt_id)?;
    let (project_id, ticket_state) = ticket_row(transaction, ticket_id)?;
    if attempt_cycle != operating_cycle_id
        || state != ActorAttemptState::Validated
        || ticket_state != TicketState::Verified
    {
        return Err(Rejection::ActorAttemptNotValidatable);
    }
    let unsatisfied_condition_count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM ticket_acceptance_conditions
         WHERE ticket_id = ?1 AND lifecycle_state = 1",
            [ticket_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    if unsatisfied_condition_count != 0 {
        return Err(Rejection::TicketAcceptanceConditionUnsatisfied);
    }
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![TicketState::Completed as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::TicketCompleted {
        ticket_id,
        actor_attempt_id,
    })
}

fn expire_work_lease(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    work_lease_id: WorkLeaseId,
) -> Result<EventBody, Rejection> {
    let row: (i64, i64, i64, i64) = transaction.query_row("SELECT l.work_item_id, l.lifecycle_state, w.ticket_id, a.operating_cycle_id FROM leases l JOIN work_items w ON w.work_item_id = l.work_item_id JOIN actor_instances a ON a.actor_instance_id = l.actor_instance_id WHERE l.work_lease_id = ?1", [work_lease_id.value()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    if row.1 != WorkLeaseState::Active as i64 {
        return Err(Rejection::WorkLeaseUnavailable);
    }
    let attempt_count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM attempts WHERE work_lease_id = ?1",
            [work_lease_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    if attempt_count != 0 {
        return Err(Rejection::WorkLeaseUnavailable);
    }
    let work_item_id = WorkItemId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?;
    let ticket_id = TicketId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?;
    let operating_cycle_id =
        OperatingCycleId::try_from(row.3).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE leases SET lifecycle_state = 3, terminal_by_command_id = ?1 WHERE work_lease_id = ?2", params![command_row_id, work_lease_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE work_items SET lifecycle_state = 1, last_transition_command_id = ?1 WHERE work_item_id = ?2", params![command_row_id, work_item_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    transaction.execute("UPDATE tickets SET lifecycle_state = ?1, last_transition_command_id = ?2 WHERE ticket_id = ?3", params![TicketState::Ready as i64, command_row_id, ticket_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    let (project_id, _) = ticket_row(transaction, ticket_id)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle_row(transaction, operating_cycle_id)?,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::WorkLeaseExpired {
        work_lease_id,
        work_item_id,
    })
}

fn cancel_actor_attempt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    actor_attempt_id: ActorAttemptId,
    reason: ActorAttemptCancellationReason,
) -> Result<EventBody, Rejection> {
    let (operating_cycle_id, ticket_id, _, _, _, state) =
        actor_attempt_row(transaction, actor_attempt_id)?;
    if state != ActorAttemptState::Running {
        return Err(Rejection::InvalidLifecycleTransition);
    }
    transaction
        .execute(
            "UPDATE attempts SET lifecycle_state = 2 WHERE actor_attempt_id = ?1",
            [actor_attempt_id.value()],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    let (project_id, _) = ticket_row(transaction, ticket_id)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle_row(transaction, operating_cycle_id)?,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ActorAttemptCancellationRequested {
        actor_attempt_id,
        reason,
    })
}

fn register_outcome_obligation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    obligation: &str,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    transaction.execute("INSERT INTO outcome_obligations(project_id, obligation_text, lifecycle_state, scheduled_by_command_id, resolved_by_command_id) VALUES (?1, ?2, 1, ?3, NULL)", params![project_id.value(), obligation, command_row_id]).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    let outcome_obligation_id = id_from_last_insert::<OutcomeObligationId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::OutcomeObligationRegistered {
        outcome_obligation_id,
        project_id,
    })
}

fn resolve_outcome_obligation(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    outcome_obligation_id: OutcomeObligationId,
    disposition: OutcomeObligationDisposition,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let row: (i64, i64) = transaction.query_row("SELECT project_id, lifecycle_state FROM outcome_obligations WHERE outcome_obligation_id = ?1", [outcome_obligation_id.value()], |row| Ok((row.get(0)?, row.get(1)?))).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    if row.1 != OutcomeObligationState::Scheduled as i64 {
        return Err(Rejection::OutcomeObligationOpen);
    }
    let project_id = ProjectId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?;
    let state = disposition.state();
    transaction.execute("UPDATE outcome_obligations SET lifecycle_state = ?1, resolved_by_command_id = ?2 WHERE outcome_obligation_id = ?3", params![state as i64, command_row_id, outcome_obligation_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::OutcomeObligationResolved {
        outcome_obligation_id,
        state,
    })
}

/// Stores a narrow receipt from the later `society-content` boundary. The
/// kernel has no byte stream here and therefore cannot honestly call this a
/// physical seal operation. This is byte identity only: a later forensic
/// manifest records each specific production/capture occurrence.
fn record_content_seal_receipt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    digest: Sha256Digest,
) -> Result<EventBody, Rejection> {
    let duplicate: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM content_seal_receipts WHERE digest = ?1",
            [digest.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    if duplicate != 0 {
        return Err(Rejection::ContentObjectNotSealed);
    }
    transaction
        .execute(
            "INSERT INTO content_seal_receipts(digest, attested_by_command_id)
             VALUES (?1, ?2)",
            params![digest.as_bytes().as_slice(), command_row_id],
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    let content_seal_receipt_id = id_from_last_insert::<ContentSealReceiptId>(transaction)?;
    Ok(EventBody::ContentSealReceiptRecorded {
        content_seal_receipt_id,
        digest,
    })
}

fn register_content_object(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    content_seal_receipt_id: ContentSealReceiptId,
) -> Result<EventBody, Rejection> {
    let present: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM content_seal_receipts WHERE content_seal_receipt_id = ?1)",
            [content_seal_receipt_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ContentSealReceiptMissing)?;
    if !present {
        return Err(Rejection::ContentSealReceiptMissing);
    }
    let registered: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM content_objects WHERE content_seal_receipt_id = ?1)",
            [content_seal_receipt_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    if registered {
        return Err(Rejection::ContentObjectNotSealed);
    }
    transaction
        .execute(
            "INSERT INTO content_objects(content_seal_receipt_id, registered_by_command_id)
             VALUES (?1, ?2)",
            params![content_seal_receipt_id.value(), command_row_id],
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    let content_object_id = id_from_last_insert::<ContentObjectId>(transaction)?;
    Ok(EventBody::ContentObjectRegistered {
        content_object_id,
        content_seal_receipt_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn register_forensic_manifest(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    producing_deterministic_experiment_id: DeterministicExperimentId,
    capture_policy: ForensicManifestCapturePolicy,
    retention_access_class: RetentionAccessClass,
    evaluator_output_content_object_id: ContentObjectId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let experiment: Option<(i64, i64)> = transaction.query_row(
        "SELECT project_id, operating_cycle_id FROM deterministic_experiments WHERE deterministic_experiment_id = ?1 AND lifecycle_state = 1",
        [producing_deterministic_experiment_id.value()], |row| Ok((row.get(0)?, row.get(1)?)),
    ).optional().map_err(|_| Rejection::ForensicManifestBindingMismatch)?;
    let (project, experiment_cycle) =
        experiment.ok_or(Rejection::ForensicManifestBindingMismatch)?;
    if experiment_cycle != operating_cycle_id.value() {
        return Err(Rejection::ForensicManifestBindingMismatch);
    }
    let project_id = ProjectId::try_from(project).map_err(|_| Rejection::SubjectNotFound)?;
    let object_exists: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM content_objects WHERE content_object_id = ?1)",
            [evaluator_output_content_object_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    if !object_exists {
        return Err(Rejection::ForensicManifestBindingMismatch);
    }
    transaction
        .execute(
            "INSERT INTO forensic_manifests(producing_deterministic_experiment_id, capture_policy, retention_access_class, registered_by_command_id)
             VALUES (?1, ?2, ?3, ?4)",
            params![producing_deterministic_experiment_id.value(), capture_policy as i64, retention_access_class as i64, command_row_id],
        )
        .map_err(|_| Rejection::ForensicManifestBindingMismatch)?;
    let forensic_manifest_id = id_from_last_insert::<ForensicManifestId>(transaction)?;
    transaction
        .execute(
            "INSERT INTO forensic_manifest_objects(forensic_manifest_id, member_ordinal, object_role, media_schema_contract, content_object_id)
             VALUES (?1, 1, 1, ?2, ?3)",
            params![
                forensic_manifest_id.value(),
                ContentMediaSchemaContract::DeterministicEvaluatorOutputV1 as i64,
                evaluator_output_content_object_id.value()
            ],
        )
        .map_err(|_| Rejection::ForensicManifestBindingMismatch)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::ForensicManifestRegistered {
        forensic_manifest_id,
        producing_deterministic_experiment_id,
        evaluator_output_content_object_id,
    })
}

fn content_object_exists(
    transaction: &Transaction<'_>,
    content_object_id: ContentObjectId,
) -> Result<bool, Rejection> {
    transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM content_objects WHERE content_object_id = ?1)",
            [content_object_id.value()],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)
}

fn evaluator_revision_for_content(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    content_object_id: ContentObjectId,
) -> Result<EvaluatorRevisionId, Rejection> {
    let existing: Option<i64> = transaction
        .query_row(
            "SELECT evaluator_revision_id FROM evaluator_revisions WHERE content_object_id = ?1",
            [content_object_id.value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    if let Some(existing) = existing {
        return EvaluatorRevisionId::try_from(existing).map_err(|_| Rejection::SubjectNotFound);
    }
    transaction
        .execute(
            "INSERT INTO evaluator_revisions(content_object_id, media_schema_contract, registered_by_command_id) VALUES (?1, ?2, ?3)",
            params![
                content_object_id.value(),
                ContentMediaSchemaContract::DeterministicEvaluatorV1 as i64,
                command_row_id
            ],
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    id_from_last_insert::<EvaluatorRevisionId>(transaction)
}

fn input_manifest_for_content(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    content_object_id: ContentObjectId,
) -> Result<InputManifestId, Rejection> {
    let existing: Option<i64> = transaction
        .query_row(
            "SELECT input_manifest_id FROM input_manifests WHERE content_object_id = ?1",
            [content_object_id.value()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    if let Some(existing) = existing {
        return InputManifestId::try_from(existing).map_err(|_| Rejection::SubjectNotFound);
    }
    transaction
        .execute(
            "INSERT INTO input_manifests(content_object_id, media_schema_contract, registered_by_command_id) VALUES (?1, ?2, ?3)",
            params![
                content_object_id.value(),
                ContentMediaSchemaContract::DeterministicInputManifestV1 as i64,
                command_row_id
            ],
        )
        .map_err(|_| Rejection::ContentObjectNotSealed)?;
    id_from_last_insert::<InputManifestId>(transaction)
}

#[allow(clippy::too_many_arguments)]
fn register_deterministic_experiment(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    project_id: ProjectId,
    ticket_id: TicketId,
    target_graph_revision_id: GraphRevisionId,
    evaluator_content_object_id: ContentObjectId,
    input_manifest_content_object_id: ContentObjectId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    project_is_active(transaction, project_id)?;
    let (_, revision_project_id, _, revision_state) =
        graph_revision_row(transaction, target_graph_revision_id)?;
    if ticket_row(transaction, ticket_id)?.0 != project_id
        || revision_project_id != project_id
        || revision_state != GraphRevisionState::Committed
        || !content_object_exists(transaction, evaluator_content_object_id)?
        || !content_object_exists(transaction, input_manifest_content_object_id)?
    {
        return Err(Rejection::DeterministicExperimentBindingMismatch);
    }
    let evaluator_revision_id =
        evaluator_revision_for_content(transaction, command_row_id, evaluator_content_object_id)?;
    let input_manifest_id = input_manifest_for_content(
        transaction,
        command_row_id,
        input_manifest_content_object_id,
    )?;
    transaction
        .execute(
            "INSERT INTO deterministic_experiments(operating_cycle_id, project_id, ticket_id, target_graph_revision_id, evaluator_revision_id, input_manifest_id, lifecycle_state, registered_by_command_id, last_transition_command_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![operating_cycle_id.value(), project_id.value(), ticket_id.value(), target_graph_revision_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), DeterministicExperimentState::Registered as i64, command_row_id],
        )
        .map_err(|_| Rejection::DeterministicExperimentBindingMismatch)?;
    let deterministic_experiment_id =
        id_from_last_insert::<DeterministicExperimentId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(project_id),
    )?;
    Ok(EventBody::DeterministicExperimentRegistered {
        deterministic_experiment_id,
        evaluator_revision_id,
        input_manifest_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn record_deterministic_evaluation_receipt(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    deterministic_experiment_id: DeterministicExperimentId,
    evaluator_revision_id: EvaluatorRevisionId,
    input_manifest_id: InputManifestId,
    forensic_manifest_id: ForensicManifestId,
    evaluator_output_content_object_id: ContentObjectId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let row: Option<(i64, i64, i64, i64)> = transaction.query_row(
        "SELECT project_id, ticket_id, evaluator_revision_id, input_manifest_id
         FROM deterministic_experiments WHERE deterministic_experiment_id = ?1 AND operating_cycle_id = ?2 AND lifecycle_state = 1",
        params![deterministic_experiment_id.value(), operating_cycle_id.value()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).optional().map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    let (project, _ticket, evaluator, input) =
        row.ok_or(Rejection::DeterministicEvaluationBindingMismatch)?;
    if evaluator != evaluator_revision_id.value() || input != input_manifest_id.value() {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    let manifest_experiment: Option<i64> = transaction.query_row(
        "SELECT producing_deterministic_experiment_id FROM forensic_manifests WHERE forensic_manifest_id = ?1",
        [forensic_manifest_id.value()], |row| row.get(0),
    ).optional().map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    if manifest_experiment != Some(deterministic_experiment_id.value()) {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    let output_in_manifest: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM forensic_manifest_objects WHERE forensic_manifest_id = ?1 AND content_object_id = ?2 AND object_role = 1 AND media_schema_contract = 3)",
        params![forensic_manifest_id.value(), evaluator_output_content_object_id.value()],
        |row| row.get(0),
    ).map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    if !output_in_manifest {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    transaction.execute(
        "INSERT INTO deterministic_evaluation_receipts(deterministic_experiment_id, evaluator_revision_id, input_manifest_id, forensic_manifest_id, evaluator_output_content_object_id, attested_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![deterministic_experiment_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), forensic_manifest_id.value(), evaluator_output_content_object_id.value(), command_row_id],
    ).map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    let deterministic_evaluation_receipt_id =
        id_from_last_insert::<DeterministicEvaluationReceiptId>(transaction)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(ProjectId::try_from(project).map_err(|_| Rejection::SubjectNotFound)?),
    )?;
    Ok(EventBody::DeterministicEvaluationReceiptRecorded {
        deterministic_evaluation_receipt_id,
        deterministic_experiment_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn admit_deterministic_evidence(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId,
    deterministic_experiment_id: DeterministicExperimentId,
    evaluator_revision_id: EvaluatorRevisionId,
    input_manifest_id: InputManifestId,
    evaluator_output_content_object_id: ContentObjectId,
    related_graph_revision_id: GraphRevisionId,
    semantic_role: EvidenceSemanticRole,
    applicability: crate::EvidenceApplicability,
    limitation: &EvidenceLimitationText,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let row: Option<(i64, i64, i64, i64, i64)> = transaction.query_row(
        "SELECT e.project_id, e.target_graph_revision_id, r.deterministic_experiment_id, r.evaluator_revision_id, r.input_manifest_id
         FROM deterministic_experiments e JOIN deterministic_evaluation_receipts r ON r.deterministic_experiment_id = e.deterministic_experiment_id
         WHERE r.deterministic_evaluation_receipt_id = ?1 AND e.operating_cycle_id = ?2 AND e.lifecycle_state = 1",
        params![deterministic_evaluation_receipt_id.value(), operating_cycle_id.value()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    ).optional().map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    let (project, target, experiment, evaluator, input) =
        row.ok_or(Rejection::DeterministicEvaluationBindingMismatch)?;
    if experiment != deterministic_experiment_id.value()
        || evaluator != evaluator_revision_id.value()
        || input != input_manifest_id.value()
        || target != related_graph_revision_id.value()
    {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    let (_, graph_project, graph_kind, graph_state) =
        graph_revision_row(transaction, related_graph_revision_id)?;
    if graph_project.value() != project
        || graph_kind != GraphObjectKind::Hypothesis
        || graph_state != GraphRevisionState::Committed
        || semantic_role != EvidenceSemanticRole::DeterministicObservation
        || applicability != crate::EvidenceApplicability::TestsTargetHypothesis
    {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    let receipt_output: i64 = transaction.query_row("SELECT evaluator_output_content_object_id FROM deterministic_evaluation_receipts WHERE deterministic_evaluation_receipt_id = ?1", [deterministic_evaluation_receipt_id.value()], |row| row.get(0)).map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    if receipt_output != evaluator_output_content_object_id.value() {
        return Err(Rejection::DeterministicEvaluationBindingMismatch);
    }
    transaction.execute(
        "INSERT INTO evidence_admissions(deterministic_evaluation_receipt_id, deterministic_experiment_id, evaluator_revision_id, input_manifest_id, evaluator_output_content_object_id, related_graph_revision_id, semantic_role, applicability, limitation_text, admitted_by_command_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![deterministic_evaluation_receipt_id.value(), deterministic_experiment_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), evaluator_output_content_object_id.value(), related_graph_revision_id.value(), semantic_role as i64, applicability as i64, limitation.as_str(), command_row_id],
    ).map_err(|_| Rejection::DeterministicEvaluationBindingMismatch)?;
    let evidence_admission_id = id_from_last_insert::<EvidenceAdmissionId>(transaction)?;
    transaction.execute("UPDATE deterministic_experiments SET lifecycle_state = 2, last_transition_command_id = ?1 WHERE deterministic_experiment_id = ?2", params![command_row_id, deterministic_experiment_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(ProjectId::try_from(project).map_err(|_| Rejection::SubjectNotFound)?),
    )?;
    Ok(EventBody::DeterministicEvidenceAdmitted {
        evidence_admission_id,
        deterministic_evaluation_receipt_id,
        semantic_role,
        applicability,
    })
}

fn close_deterministic_experiment(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    expected_generation: ExpectedGeneration,
    operating_cycle_id: OperatingCycleId,
    deterministic_experiment_id: DeterministicExperimentId,
) -> Result<EventBody, Rejection> {
    let cycle = coordination_cycle(transaction, expected_generation, operating_cycle_id)?;
    let row: Option<(i64, i64)> = transaction.query_row("SELECT project_id, lifecycle_state FROM deterministic_experiments WHERE deterministic_experiment_id = ?1 AND operating_cycle_id = ?2", params![deterministic_experiment_id.value(), operating_cycle_id.value()], |row| Ok((row.get(0)?, row.get(1)?))).optional().map_err(|_| Rejection::SubjectNotFound)?;
    let (project, state) = row.ok_or(Rejection::SubjectNotFound)?;
    if state != DeterministicExperimentState::EvidenceAdmitted as i64 {
        return Err(Rejection::EvidenceAdmissionRequired);
    }
    transaction.execute("UPDATE deterministic_experiments SET lifecycle_state = 3, last_transition_command_id = ?1 WHERE deterministic_experiment_id = ?2", params![command_row_id, deterministic_experiment_id.value()]).map_err(|_| Rejection::SubjectNotFound)?;
    record_coordination_provenance(
        transaction,
        command_row_id,
        cycle,
        operating_cycle_id,
        Some(ProjectId::try_from(project).map_err(|_| Rejection::SubjectNotFound)?),
    )?;
    Ok(EventBody::DeterministicExperimentClosed {
        deterministic_experiment_id,
    })
}

fn capability_grant(
    transaction: &Transaction<'_>,
    principal_id: PrincipalId,
    capability: Capability,
    capability_grant_id: crate::CapabilityGrantId,
) -> Result<Option<CapabilityGrantLookup>, StoreError> {
    let grant = transaction
        .query_row(
            "SELECT grant_state, office_occupancy_id, actor_instance_id FROM capability_grants
             WHERE capability_grant_id = ?1 AND principal_id = ?2 AND capability_kind = ?3",
            params![
                capability_grant_id.value(),
                principal_id.value(),
                capability as i64
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .optional()?;
    match grant {
        Some((1, office_occupancy_id, actor_instance_id)) => {
            Ok(Some(CapabilityGrantLookup::Active {
                grant_id: capability_grant_id.value(),
                office_occupancy_id: office_occupancy_id
                    .map(OfficeOccupancyId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_instance_id: actor_instance_id
                    .map(ActorInstanceId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }))
        }
        Some(_) => Ok(Some(CapabilityGrantLookup::Inactive)),
        None => Ok(None),
    }
}

fn grant_has_active_occupancy(
    transaction: &Transaction<'_>,
    grant_id: i64,
) -> Result<bool, StoreError> {
    transaction
        .query_row(
            "SELECT EXISTS(
             SELECT 1 FROM capability_grants g
             JOIN office_occupancies o ON o.office_occupancy_id = g.office_occupancy_id
             WHERE g.capability_grant_id = ?1
               AND o.active = 1
               AND o.principal_id = g.principal_id
         )",
            [grant_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(StoreError::from)
}

fn grant_has_active_actor_instance(
    transaction: &Transaction<'_>,
    grant_id: i64,
) -> Result<bool, StoreError> {
    transaction
        .query_row(
            "SELECT EXISTS(
             SELECT 1 FROM capability_grants g
             JOIN actor_instances a ON a.actor_instance_id = g.actor_instance_id
             JOIN principals p ON p.principal_id = a.principal_id
             WHERE g.capability_grant_id = ?1
               AND a.lifecycle_state = 1
               AND p.active = 1
               AND p.principal_id = g.principal_id
         )",
            [grant_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(StoreError::from)
}

/// Every actor-side command that governs a cycle, session, or cost incident is
/// bound to that object's pinned Office occupancy. A merely active Grand
/// Architect grant is not interchangeable with the grant that governed the
/// scoped object when succession becomes possible.
fn command_target_occupancy(
    transaction: &Transaction<'_>,
    body: &CommandBody,
) -> Result<Option<OfficeOccupancyId>, Rejection> {
    match body {
        CommandBody::ProposeOperatingCycle { .. } => {
            Ok(Some(bootstrapped_constitution(transaction)?.2))
        }
        CommandBody::AdmitOperatingCycle { cycle_id }
        | CommandBody::StartGrandArchitectOfficeSession { cycle_id }
        | CommandBody::QuiesceOperatingCycle { cycle_id }
        | CommandBody::ResumeOperatingCycle { cycle_id }
        | CommandBody::ReconcileOperatingCycle { cycle_id }
        | CommandBody::CloseOperatingCycle { cycle_id }
        | CommandBody::ReserveBudget { cycle_id, .. }
        | CommandBody::RequestCancellation { cycle_id, .. } => {
            Ok(Some(cycle_row(transaction, *cycle_id)?.occupancy_id))
        }
        CommandBody::OpenOfficeTurn { session_id, .. } => {
            session_occupancy_id(transaction, *session_id).map(Some)
        }
        CommandBody::CloseCostPostmortem { postmortem_id, .. } => {
            let cycle_id = transaction
                .query_row(
                    "SELECT operating_cycle_id FROM cost_postmortems WHERE postmortem_id = ?1",
                    [postmortem_id.value()],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(|_| Rejection::SubjectNotFound)?
                .ok_or(Rejection::SubjectNotFound)?;
            Ok(Some(
                cycle_row(
                    transaction,
                    OperatingCycleId::try_from(cycle_id).map_err(|_| Rejection::SubjectNotFound)?,
                )?
                .occupancy_id,
            ))
        }
        CommandBody::CreateProject {
            operating_cycle_id, ..
        }
        | CommandBody::CharterProject {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionProject {
            operating_cycle_id, ..
        }
        | CommandBody::CompleteProjectMilestone {
            operating_cycle_id, ..
        }
        | CommandBody::ReopenProject {
            operating_cycle_id, ..
        }
        | CommandBody::CreateTicket {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionTicket {
            operating_cycle_id, ..
        }
        | CommandBody::AddGraphObjectRevision {
            operating_cycle_id, ..
        }
        | CommandBody::CommitGraphRevision {
            operating_cycle_id, ..
        }
        | CommandBody::AddGraphEdge {
            operating_cycle_id, ..
        }
        | CommandBody::CreateEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::TransitionEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::ReopenEpisode {
            operating_cycle_id, ..
        }
        | CommandBody::RequestAdversarialReview {
            operating_cycle_id, ..
        }
        | CommandBody::AssignAdversarialReviewer {
            operating_cycle_id, ..
        }
        | CommandBody::SubmitReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::RespondToReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::DispositionReviewChallenge {
            operating_cycle_id, ..
        }
        | CommandBody::ResolveAdversarialReview {
            operating_cycle_id, ..
        }
        | CommandBody::TriggerPostmortem {
            operating_cycle_id, ..
        }
        | CommandBody::RecordPostmortemCausalClaim {
            operating_cycle_id, ..
        }
        | CommandBody::ProposePostmortemAction {
            operating_cycle_id, ..
        }
        | CommandBody::ClosePostmortem {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterContextPack {
            operating_cycle_id, ..
        }
        | CommandBody::AdmitActorInstance {
            operating_cycle_id, ..
        }
        | CommandBody::AdmitTicket {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterWorkItem {
            operating_cycle_id, ..
        }
        | CommandBody::StartActorAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::ValidateTicketAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::RetryActorAttempt {
            operating_cycle_id, ..
        }
        | CommandBody::CompleteTicket {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterOutcomeObligation {
            operating_cycle_id, ..
        }
        | CommandBody::ResolveOutcomeObligation {
            operating_cycle_id, ..
        }
        | CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id, ..
        }
        | CommandBody::CloseDeterministicExperiment {
            operating_cycle_id, ..
        } => Ok(Some(
            cycle_row(transaction, *operating_cycle_id)?.occupancy_id,
        )),
        _ => Ok(None),
    }
}

fn only_society_id(transaction: &Transaction<'_>) -> Result<SocietyId, Rejection> {
    let value = transaction
        .query_row("SELECT society_id FROM societies LIMIT 1", [], |row| {
            row.get::<_, i64>(0)
        })
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    SocietyId::try_from(value).map_err(|_| Rejection::SubjectNotFound)
}

fn grand_architect_office_id(transaction: &Transaction<'_>) -> Result<OfficeId, Rejection> {
    let value = transaction
        .query_row(
            "SELECT office_id FROM office_contracts WHERE office_kind = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    OfficeId::try_from(value).map_err(|_| Rejection::SubjectNotFound)
}

fn active_seed_id(
    transaction: &Transaction<'_>,
    society_id: SocietyId,
) -> Result<UniverseSeedId, Rejection> {
    let value = transaction
        .query_row(
            "SELECT universe_seed_id FROM universe_seeds WHERE society_id = ?1 AND active = 1",
            [society_id.value()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    UniverseSeedId::try_from(value).map_err(|_| Rejection::SubjectNotFound)
}

fn active_grand_architect_occupancy_id(
    transaction: &Transaction<'_>,
) -> Result<OfficeOccupancyId, Rejection> {
    let value = transaction
        .query_row(
            "SELECT o.office_occupancy_id FROM office_occupancies o
         JOIN office_contracts c ON c.office_id = o.office_id
         WHERE c.office_kind = 1 AND o.active = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    OfficeOccupancyId::try_from(value).map_err(|_| Rejection::SubjectNotFound)
}

fn hard_ceiling_from_event_body(transaction: &Transaction<'_>) -> Result<UsdMicros, Rejection> {
    let value = transaction
        .query_row(
            "SELECT ceiling_micros FROM event_r0_hard_ceiling_set ORDER BY event_id DESC LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::FoundingInvariant)?;
    UsdMicros::try_from(value).map_err(|_| Rejection::FoundingInvariant)
}

fn bootstrapped_constitution(
    transaction: &Transaction<'_>,
) -> Result<(SocietyId, UniverseSeedId, OfficeOccupancyId), Rejection> {
    let row = transaction.query_row(
        "SELECT society_id, universe_seed_id, office_occupancy_id FROM society_bootstraps LIMIT 1",
        [], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::FoundingInvariant)?;
    Ok((
        SocietyId::try_from(row.0).map_err(|_| Rejection::FoundingInvariant)?,
        UniverseSeedId::try_from(row.1).map_err(|_| Rejection::FoundingInvariant)?,
        OfficeOccupancyId::try_from(row.2).map_err(|_| Rejection::FoundingInvariant)?,
    ))
}

fn cycle_row(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<CycleRow, Rejection> {
    let row = transaction.query_row(
        "SELECT society_id, universe_seed_id, office_occupancy_id, treatment, lifecycle_state, admission_generation
         FROM operating_cycles WHERE operating_cycle_id = ?1",
        [cycle_id.value()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?, row.get::<_, i64>(5)?)),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok(CycleRow {
        society_id: SocietyId::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        seed_id: UniverseSeedId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        occupancy_id: OfficeOccupancyId::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
        _treatment: operating_cycle_treatment_from_i64(row.3)
            .map_err(|_| Rejection::SubjectNotFound)?,
        state: operating_cycle_state_from_i64(row.4).map_err(|_| Rejection::SubjectNotFound)?,
        generation: AdmissionGeneration::try_from(row.5).map_err(|_| Rejection::SubjectNotFound)?,
    })
}

fn cycle_for_generation(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
    expected_generation: ExpectedGeneration,
) -> Result<CycleRow, Rejection> {
    let cycle = cycle_row(transaction, cycle_id)?;
    match expected_generation {
        ExpectedGeneration::NotApplicable => Err(Rejection::InvalidExpectedGeneration),
        ExpectedGeneration::Exact(generation) if generation != cycle.generation => {
            Err(Rejection::StaleAdmissionGeneration)
        }
        ExpectedGeneration::Exact(_) => Ok(cycle),
    }
}

fn session_row(
    transaction: &Transaction<'_>,
    session_id: GrandArchitectOfficeSessionId,
) -> Result<(OfficeSessionState, OperatingCycleId), Rejection> {
    let row = transaction
        .query_row(
            "SELECT lifecycle_state, operating_cycle_id FROM grand_architect_office_sessions
         WHERE grand_architect_office_session_id = ?1",
            [session_id.value()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    Ok((
        office_session_state_from_i64(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        OperatingCycleId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn session_occupancy_id(
    transaction: &Transaction<'_>,
    session_id: GrandArchitectOfficeSessionId,
) -> Result<OfficeOccupancyId, Rejection> {
    let value = transaction
        .query_row(
            "SELECT office_occupancy_id FROM grand_architect_office_sessions
             WHERE grand_architect_office_session_id = ?1",
            [session_id.value()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    OfficeOccupancyId::try_from(value).map_err(|_| Rejection::SubjectNotFound)
}

fn transition_cycle(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    cycle_id: OperatingCycleId,
    state: OperatingCycleState,
    generation: AdmissionGeneration,
) -> Result<(), Rejection> {
    transaction
        .execute(
            "UPDATE operating_cycles SET lifecycle_state = ?1, admission_generation = ?2,
                                     last_transition_command_id = ?3 WHERE operating_cycle_id = ?4",
            params![
                state as i64,
                generation.value(),
                command_row_id,
                cycle_id.value()
            ],
        )
        .map_err(|_| Rejection::SubjectNotFound)?;
    Ok(())
}

fn create_budget_envelope(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    ceiling: UsdMicros,
) -> Result<BudgetEnvelopeId, Rejection> {
    transaction.execute(
        "INSERT INTO budget_envelopes(ceiling_micros, reserved_micros, spent_micros, created_by_command_id)
         VALUES (?1, 0, 0, ?2)",
        params![ceiling.value(), command_row_id],
    ).map_err(|_| Rejection::BudgetCeilingExceeded)?;
    id_from_last_insert::<BudgetEnvelopeId>(transaction)
}

fn budget_envelopes_for_cycle(
    transaction: &Transaction<'_>,
    society_id: SocietyId,
    cycle_id: OperatingCycleId,
) -> Result<(BudgetEnvelopeId, BudgetEnvelopeId), Rejection> {
    let society_budget = transaction
        .query_row(
            "SELECT budget_envelope_id FROM budget_envelope_constraints WHERE society_id = ?1",
            [society_id.value()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .ok_or(Rejection::SubjectNotFound)?;
    let cycle_budget = transaction.query_row(
        "SELECT budget_envelope_id FROM budget_envelope_constraints WHERE operating_cycle_id = ?1",
        [cycle_id.value()], |row| row.get::<_, i64>(0),
    ).optional().map_err(|_| Rejection::SubjectNotFound)?.ok_or(Rejection::SubjectNotFound)?;
    Ok((
        BudgetEnvelopeId::try_from(society_budget).map_err(|_| Rejection::SubjectNotFound)?,
        BudgetEnvelopeId::try_from(cycle_budget).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn budget_amounts(
    transaction: &Transaction<'_>,
    budget_id: BudgetEnvelopeId,
) -> Result<(UsdMicros, UsdMicros, UsdMicros), Rejection> {
    let row = transaction.query_row(
        "SELECT ceiling_micros, reserved_micros, spent_micros FROM budget_envelopes WHERE budget_envelope_id = ?1",
        [budget_id.value()],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
    ).map_err(|_| Rejection::SubjectNotFound)?;
    Ok((
        UsdMicros::try_from(row.0).map_err(|_| Rejection::SubjectNotFound)?,
        UsdMicros::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
        UsdMicros::try_from(row.2).map_err(|_| Rejection::SubjectNotFound)?,
    ))
}

fn active_office_turn_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction.query_row(
        "SELECT COUNT(*) FROM office_turns t
         JOIN grand_architect_office_sessions s ON s.grand_architect_office_session_id = t.grand_architect_office_session_id
         WHERE s.operating_cycle_id = ?1 AND t.lifecycle_state = ?2",
        params![cycle_id.value(), OfficeTurnState::Active as i64],
        |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)
}

fn session_has_active_turn(
    transaction: &Transaction<'_>,
    session_id: GrandArchitectOfficeSessionId,
) -> Result<bool, Rejection> {
    transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM office_turns
             WHERE grand_architect_office_session_id = ?1 AND lifecycle_state = ?2)",
            params![session_id.value(), OfficeTurnState::Active as i64],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(|_| Rejection::SubjectNotFound)
}

fn live_office_session_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM grand_architect_office_sessions
             WHERE operating_cycle_id = ?1 AND lifecycle_state NOT IN (?2, ?3, ?4)",
            params![
                cycle_id.value(),
                OfficeSessionState::Closed as i64,
                OfficeSessionState::Cancelled as i64,
                OfficeSessionState::Failed as i64
            ],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)
}

fn unreconciled_reservation_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction.query_row(
        "SELECT COUNT(*) FROM budget_reservations WHERE operating_cycle_id = ?1 AND reservation_state != ?2",
        params![cycle_id.value(), BudgetReservationState::Reconciled as i64],
        |row| row.get(0),
    ).map_err(|_| Rejection::SubjectNotFound)
}

fn active_cancellation_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM cancellation_requests WHERE operating_cycle_id = ?1
         AND lifecycle_state NOT IN (?2, ?3)",
            params![
                cycle_id.value(),
                CancellationState::Completed as i64,
                CancellationState::ContainmentFailed as i64
            ],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)
}

/// Work execution is not represented by an Office turn. These independent
/// actor-owned children must therefore drain before a cycle can be resumed or
/// closed; a lease with no budget reservation is still a material obligation.
fn active_work_lease_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM leases l
             JOIN actor_instances a ON a.actor_instance_id = l.actor_instance_id
             WHERE a.operating_cycle_id = ?1 AND l.lifecycle_state = ?2",
            params![cycle_id.value(), WorkLeaseState::Active as i64],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)
}

fn live_actor_attempt_count(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<i64, Rejection> {
    transaction
        .query_row(
            "SELECT COUNT(*) FROM attempts WHERE operating_cycle_id = ?1 AND lifecycle_state IN (?2, ?3)",
            params![
                cycle_id.value(),
                ActorAttemptState::Running as i64,
                ActorAttemptState::CancellationRequested as i64,
            ],
            |row| row.get(0),
        )
        .map_err(|_| Rejection::SubjectNotFound)
}

fn active_cancellation_for_cycle(
    transaction: &Transaction<'_>,
    cycle_id: OperatingCycleId,
) -> Result<Option<CancellationRequestId>, Rejection> {
    transaction
        .query_row(
            "SELECT cancellation_request_id FROM cancellation_requests
             WHERE operating_cycle_id = ?1 AND lifecycle_state NOT IN (?2, ?3)
             ORDER BY cancellation_request_id ASC LIMIT 1",
            params![
                cycle_id.value(),
                CancellationState::Completed as i64,
                CancellationState::ContainmentFailed as i64,
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|_| Rejection::SubjectNotFound)?
        .map(CancellationRequestId::try_from)
        .transpose()
        .map_err(|_| Rejection::SubjectNotFound)
}

fn exists(transaction: &Transaction<'_>, query: &str) -> Result<bool, Rejection> {
    transaction
        .query_row(query, [], |row| row.get::<_, i64>(0))
        .optional()
        .map(|value| value.is_some())
        .map_err(|_| Rejection::SubjectNotFound)
}

fn id_from_last_insert<T>(transaction: &Transaction<'_>) -> Result<T, Rejection>
where
    T: TryFrom<i64>,
{
    T::try_from(transaction.last_insert_rowid()).map_err(|_| Rejection::SubjectNotFound)
}

fn expected_generation_to_sql(value: ExpectedGeneration) -> Option<i64> {
    match value {
        ExpectedGeneration::NotApplicable => None,
        ExpectedGeneration::Exact(generation) => Some(generation.value()),
    }
}

fn request_fingerprint(request: &CommandRequest) -> Sha256Digest {
    let mut bytes = Vec::with_capacity(128);
    put_bytes(&mut bytes, request.command_id.as_str().as_bytes());
    put_i64(&mut bytes, request.principal_id.value());
    put_i64(&mut bytes, request.capability_grant_id.value());
    put_i64(&mut bytes, request.capability as i64);
    match request.expected_generation {
        ExpectedGeneration::NotApplicable => put_i64(&mut bytes, -1),
        ExpectedGeneration::Exact(generation) => put_i64(&mut bytes, generation.value()),
    }
    put_i64(&mut bytes, request.body.kind() as i64);
    match &request.body {
        CommandBody::CreateSocietyIdentity { name } => {
            put_bytes(&mut bytes, name.as_str().as_bytes())
        }
        CommandBody::InstallGrandArchitectOffice | CommandBody::BootstrapSociety => {}
        CommandBody::InstallFoundingUniverseSeed { rendering_digest } => {
            put_bytes(&mut bytes, &rendering_digest.as_bytes())
        }
        CommandBody::AppointInitialGrandArchitect { actor_display_name } => {
            put_bytes(&mut bytes, actor_display_name.as_str().as_bytes())
        }
        CommandBody::SetR0HardCeiling { ceiling } => put_i64(&mut bytes, ceiling.value()),
        CommandBody::ProposeOperatingCycle { treatment } => put_i64(&mut bytes, *treatment as i64),
        CommandBody::AdmitOperatingCycle { cycle_id }
        | CommandBody::StartGrandArchitectOfficeSession { cycle_id }
        | CommandBody::QuiesceOperatingCycle { cycle_id }
        | CommandBody::RecordCycleDrained { cycle_id }
        | CommandBody::ResumeOperatingCycle { cycle_id }
        | CommandBody::ReconcileOperatingCycle { cycle_id }
        | CommandBody::CloseOperatingCycle { cycle_id } => put_i64(&mut bytes, cycle_id.value()),
        CommandBody::RecordOfficeSessionReady { session_id } => {
            put_i64(&mut bytes, session_id.value())
        }
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, *terminal_state as i64);
        }
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, *purpose as i64);
        }
        CommandBody::SettleOfficeTurn { turn_id } => put_i64(&mut bytes, turn_id.value()),
        CommandBody::ReserveBudget { cycle_id, amount } => {
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, amount.value());
        }
        CommandBody::ReconcileBudget {
            reservation_id,
            observation,
        } => {
            put_i64(&mut bytes, reservation_id.value());
            match observation {
                CostObservation::Known(amount) => {
                    put_i64(&mut bytes, 1);
                    put_i64(&mut bytes, amount.value());
                }
                CostObservation::Unknown(reason) => {
                    put_i64(&mut bytes, 2);
                    put_i64(&mut bytes, *reason as i64);
                }
                CostObservation::Unavailable(reason) => {
                    put_i64(&mut bytes, 3);
                    put_i64(&mut bytes, *reason as i64);
                }
            }
        }
        CommandBody::RequestCancellation { cycle_id, mode } => {
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, *mode as i64);
        }
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        } => put_i64(&mut bytes, cancellation_request_id.value()),
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution,
        } => {
            put_i64(&mut bytes, postmortem_id.value());
            put_i64(&mut bytes, *resolution as i64);
        }
        CommandBody::CreateProject {
            operating_cycle_id,
            project_name,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_bytes(&mut bytes, project_name.as_str().as_bytes());
        }
        CommandBody::CharterProject {
            operating_cycle_id,
            project_id,
            objective,
            initial_milestone,
            stop_condition,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_bytes(&mut bytes, objective.as_str().as_bytes());
            put_bytes(&mut bytes, initial_milestone.as_str().as_bytes());
            put_bytes(&mut bytes, stop_condition.as_str().as_bytes());
        }
        CommandBody::TransitionProject {
            operating_cycle_id,
            project_id,
            target,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_i64(&mut bytes, *target as i64);
        }
        CommandBody::CompleteProjectMilestone {
            operating_cycle_id,
            project_milestone_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_milestone_id.value());
        }
        CommandBody::ReopenProject {
            operating_cycle_id,
            project_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
        }
        CommandBody::CreateTicket {
            operating_cycle_id,
            project_id,
            ticket_title,
            acceptance_condition,
            prerequisite_ticket_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_bytes(&mut bytes, ticket_title.as_str().as_bytes());
            put_bytes(&mut bytes, acceptance_condition.as_str().as_bytes());
            put_optional_i64(&mut bytes, prerequisite_ticket_id.map(TicketId::value));
        }
        CommandBody::TransitionTicket {
            operating_cycle_id,
            ticket_id,
            target,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, ticket_id.value());
            put_i64(&mut bytes, *target as i64);
        }
        CommandBody::AddGraphObjectRevision {
            operating_cycle_id,
            project_id,
            causal_episode_id,
            graph_object_id,
            body,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_optional_i64(&mut bytes, causal_episode_id.map(CausalEpisodeId::value));
            put_optional_i64(&mut bytes, graph_object_id.map(GraphObjectId::value));
            match body {
                GraphRevisionBody::Observation { observation } => {
                    put_i64(&mut bytes, GraphObjectKind::Observation as i64);
                    put_bytes(&mut bytes, observation.as_str().as_bytes());
                }
                GraphRevisionBody::Hypothesis { hypothesis } => {
                    put_i64(&mut bytes, GraphObjectKind::Hypothesis as i64);
                    put_bytes(&mut bytes, hypothesis.as_str().as_bytes());
                }
            }
        }
        CommandBody::CommitGraphRevision {
            operating_cycle_id,
            graph_revision_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, graph_revision_id.value());
        }
        CommandBody::AddGraphEdge {
            operating_cycle_id,
            project_id,
            from_graph_revision_id,
            to_graph_revision_id,
            edge_kind,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_i64(&mut bytes, from_graph_revision_id.value());
            put_i64(&mut bytes, to_graph_revision_id.value());
            put_i64(&mut bytes, *edge_kind as i64);
        }
        CommandBody::CreateEpisode {
            operating_cycle_id,
            project_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
        }
        CommandBody::TransitionEpisode {
            operating_cycle_id,
            causal_episode_id,
            target,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, causal_episode_id.value());
            put_i64(&mut bytes, *target as i64);
        }
        CommandBody::ReopenEpisode {
            operating_cycle_id,
            causal_episode_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, causal_episode_id.value());
        }
        CommandBody::RequestAdversarialReview {
            operating_cycle_id,
            project_id,
            target_graph_revision_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_i64(&mut bytes, target_graph_revision_id.value());
        }
        CommandBody::AssignAdversarialReviewer {
            operating_cycle_id,
            adversarial_review_id,
            reviewer_principal_id,
            reviewer_actor_instance_id,
            reviewer_actor_attempt_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, adversarial_review_id.value());
            put_i64(&mut bytes, reviewer_principal_id.value());
            put_i64(&mut bytes, reviewer_actor_instance_id.value());
            put_i64(&mut bytes, reviewer_actor_attempt_id.value());
        }
        CommandBody::SubmitReviewChallenge {
            operating_cycle_id,
            adversarial_review_id,
            target_graph_revision_id,
            author_principal_id,
            severity,
            failure_hypothesis,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, adversarial_review_id.value());
            put_i64(&mut bytes, target_graph_revision_id.value());
            put_i64(&mut bytes, author_principal_id.value());
            put_i64(&mut bytes, *severity as i64);
            put_bytes(&mut bytes, failure_hypothesis.as_str().as_bytes());
        }
        CommandBody::RespondToReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            response,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, review_challenge_id.value());
            put_bytes(&mut bytes, response.as_str().as_bytes());
        }
        CommandBody::DispositionReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            disposition,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, review_challenge_id.value());
            put_i64(&mut bytes, *disposition as i64);
        }
        CommandBody::RecordContentSealReceipt { digest } => {
            put_bytes(&mut bytes, &digest.as_bytes());
        }
        CommandBody::RegisterContentObject {
            content_seal_receipt_id,
        } => {
            put_i64(&mut bytes, content_seal_receipt_id.value());
        }
        CommandBody::RegisterForensicManifest {
            operating_cycle_id,
            producing_deterministic_experiment_id,
            capture_policy,
            retention_access_class,
            evaluator_output_content_object_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, producing_deterministic_experiment_id.value());
            put_i64(&mut bytes, *capture_policy as i64);
            put_i64(&mut bytes, *retention_access_class as i64);
            put_i64(&mut bytes, evaluator_output_content_object_id.value());
        }
        CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id,
            project_id,
            ticket_id,
            target_graph_revision_id,
            evaluator_content_object_id,
            input_manifest_content_object_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_i64(&mut bytes, ticket_id.value());
            put_i64(&mut bytes, target_graph_revision_id.value());
            put_i64(&mut bytes, evaluator_content_object_id.value());
            put_i64(&mut bytes, input_manifest_content_object_id.value());
        }
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            forensic_manifest_id,
            evaluator_output_content_object_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, deterministic_experiment_id.value());
            put_i64(&mut bytes, evaluator_revision_id.value());
            put_i64(&mut bytes, input_manifest_id.value());
            put_i64(&mut bytes, forensic_manifest_id.value());
            put_i64(&mut bytes, evaluator_output_content_object_id.value());
        }
        CommandBody::AdmitDeterministicEvidence {
            operating_cycle_id,
            deterministic_evaluation_receipt_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            evaluator_output_content_object_id,
            related_graph_revision_id,
            semantic_role,
            applicability,
            limitation,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, deterministic_evaluation_receipt_id.value());
            put_i64(&mut bytes, deterministic_experiment_id.value());
            put_i64(&mut bytes, evaluator_revision_id.value());
            put_i64(&mut bytes, input_manifest_id.value());
            put_i64(&mut bytes, evaluator_output_content_object_id.value());
            put_i64(&mut bytes, related_graph_revision_id.value());
            put_i64(&mut bytes, *semantic_role as i64);
            put_i64(&mut bytes, *applicability as i64);
            put_bytes(&mut bytes, limitation.as_str().as_bytes());
        }
        CommandBody::CloseDeterministicExperiment {
            operating_cycle_id,
            deterministic_experiment_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, deterministic_experiment_id.value());
        }
        CommandBody::ResolveAdversarialReview {
            operating_cycle_id,
            adversarial_review_id,
            resolution,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, adversarial_review_id.value());
            put_i64(&mut bytes, *resolution as i64);
        }
        CommandBody::TriggerPostmortem {
            operating_cycle_id,
            project_id,
            causal_episode_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_optional_i64(&mut bytes, causal_episode_id.map(CausalEpisodeId::value));
        }
        CommandBody::RecordPostmortemCausalClaim {
            operating_cycle_id,
            postmortem_id,
            claim_kind,
            claim,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, postmortem_id.value());
            put_i64(&mut bytes, *claim_kind as i64);
            put_bytes(&mut bytes, claim.as_str().as_bytes());
        }
        CommandBody::ProposePostmortemAction {
            operating_cycle_id,
            postmortem_id,
            action_kind,
            action,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, postmortem_id.value());
            put_i64(&mut bytes, *action_kind as i64);
            put_bytes(&mut bytes, action.as_str().as_bytes());
        }
        CommandBody::ClosePostmortem {
            operating_cycle_id,
            postmortem_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, postmortem_id.value());
        }
        CommandBody::RegisterActorConfiguration {
            configuration_name,
            model_policy,
            primary_attractor,
        } => {
            put_bytes(&mut bytes, configuration_name.as_str().as_bytes());
            put_i64(&mut bytes, *model_policy as i64);
            put_i64(&mut bytes, *primary_attractor as i64);
        }
        CommandBody::RegisterContextPack {
            operating_cycle_id,
            purpose,
            rendering_digest,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, *purpose as i64);
            put_bytes(&mut bytes, &rendering_digest.as_bytes());
        }
        CommandBody::AdmitActorInstance {
            operating_cycle_id,
            actor_configuration_revision_id,
            execution_profile_id,
            actor_display_name,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, actor_configuration_revision_id.value());
            put_i64(&mut bytes, execution_profile_id.value());
            put_bytes(&mut bytes, actor_display_name.as_str().as_bytes());
        }
        CommandBody::AdmitTicket {
            operating_cycle_id,
            ticket_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, ticket_id.value());
        }
        CommandBody::RegisterWorkItem {
            operating_cycle_id,
            ticket_id,
            actor_instance_id,
            context_pack_id,
            work_kind,
            adversarial_review_id,
            assignment,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, ticket_id.value());
            put_i64(&mut bytes, actor_instance_id.value());
            put_i64(&mut bytes, context_pack_id.value());
            put_i64(&mut bytes, *work_kind as i64);
            put_optional_i64(
                &mut bytes,
                adversarial_review_id.map(AdversarialReviewId::value),
            );
            put_bytes(&mut bytes, assignment.as_str().as_bytes());
        }
        CommandBody::ClaimWorkItem {
            operating_cycle_id,
            work_item_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, work_item_id.value());
        }
        CommandBody::StartActorAttempt {
            operating_cycle_id,
            work_item_id,
            reservation_amount,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, work_item_id.value());
            put_i64(&mut bytes, reservation_amount.value());
        }
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id,
            terminal_kind,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, *terminal_kind as i64);
        }
        CommandBody::ValidateTicketAttempt {
            operating_cycle_id,
            actor_attempt_id,
        }
        | CommandBody::RetryActorAttempt {
            operating_cycle_id,
            actor_attempt_id,
        }
        | CommandBody::CompleteTicket {
            operating_cycle_id,
            actor_attempt_id,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, actor_attempt_id.value());
        }
        CommandBody::ExpireWorkLease { work_lease_id } => {
            put_i64(&mut bytes, work_lease_id.value())
        }
        CommandBody::CancelActorAttempt {
            actor_attempt_id,
            reason,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, *reason as i64);
        }
        CommandBody::RegisterOutcomeObligation {
            operating_cycle_id,
            project_id,
            obligation,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, project_id.value());
            put_bytes(&mut bytes, obligation.as_str().as_bytes());
        }
        CommandBody::ResolveOutcomeObligation {
            operating_cycle_id,
            outcome_obligation_id,
            disposition,
        } => {
            put_i64(&mut bytes, operating_cycle_id.value());
            put_i64(&mut bytes, outcome_obligation_id.value());
            put_i64(&mut bytes, *disposition as i64);
        }
    }
    Sha256Digest::of_bytes(&bytes)
}

/// A compact integrity commitment to an exact ledger event. Event identity and
/// its command identity are committed before the closed body so relinking an
/// otherwise valid event to a different command is detectable. This is not a
/// hash chain: events remain independently inspectable.
fn event_fingerprint(event_id: EventId, command_id: &CommandId, body: &EventBody) -> Sha256Digest {
    let mut bytes = Vec::with_capacity(96);
    put_i64(&mut bytes, event_id.value());
    put_bytes(&mut bytes, command_id.as_str().as_bytes());
    put_i64(&mut bytes, body.kind() as i64);
    match body {
        EventBody::SocietyIdentityCreated { society_id }
        | EventBody::SocietyBootstrapped { society_id } => {
            put_i64(&mut bytes, society_id.value());
        }
        EventBody::GrandArchitectOfficeInstalled { office_id } => {
            put_i64(&mut bytes, office_id.value());
        }
        EventBody::FoundingUniverseSeedInstalled { seed_id } => {
            put_i64(&mut bytes, seed_id.value());
        }
        EventBody::GrandArchitectAppointed {
            occupancy_id,
            principal_id,
        } => {
            put_i64(&mut bytes, occupancy_id.value());
            put_i64(&mut bytes, principal_id.value());
        }
        EventBody::R0HardCeilingSet {
            society_id,
            ceiling,
        } => {
            put_i64(&mut bytes, society_id.value());
            put_i64(&mut bytes, ceiling.value());
        }
        EventBody::OperatingCycleProposed {
            cycle_id,
            generation,
            treatment,
        } => {
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, generation.value());
            put_i64(&mut bytes, *treatment as i64);
        }
        EventBody::OperatingCycleStateChanged {
            cycle_id,
            state,
            generation,
        } => {
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, *state as i64);
            put_i64(&mut bytes, generation.value());
        }
        EventBody::GrandArchitectOfficeSessionStarted {
            session_id,
            cycle_id,
        } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, cycle_id.value());
        }
        EventBody::GrandArchitectOfficeSessionStateChanged { session_id, state } => {
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::OfficeTurnOpened {
            turn_id,
            session_id,
            purpose,
        } => {
            put_i64(&mut bytes, turn_id.value());
            put_i64(&mut bytes, session_id.value());
            put_i64(&mut bytes, *purpose as i64);
        }
        EventBody::OfficeTurnSettled {
            turn_id,
            session_id,
        } => {
            put_i64(&mut bytes, turn_id.value());
            put_i64(&mut bytes, session_id.value());
        }
        EventBody::BudgetReserved {
            reservation_id,
            cycle_id,
            amount,
        } => {
            put_i64(&mut bytes, reservation_id.value());
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, amount.value());
        }
        EventBody::BudgetReconciled {
            reservation_id,
            observed,
        } => {
            put_i64(&mut bytes, reservation_id.value());
            put_i64(&mut bytes, observed.value());
        }
        EventBody::BudgetAdmissionFrozen {
            reservation_id,
            cycle_id,
            cancellation_request_id,
            postmortem_id,
            reason,
        } => {
            put_i64(&mut bytes, reservation_id.value());
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, cancellation_request_id.value());
            put_i64(&mut bytes, postmortem_id.value());
            match reason {
                BudgetFreezeReason::KnownOverrun { observed, reserved } => {
                    put_i64(&mut bytes, 1);
                    put_i64(&mut bytes, observed.value());
                    put_i64(&mut bytes, reserved.value());
                }
                BudgetFreezeReason::Unknown(reason) => {
                    put_i64(&mut bytes, 2);
                    put_i64(&mut bytes, *reason as i64);
                }
                BudgetFreezeReason::Unavailable(reason) => {
                    put_i64(&mut bytes, 3);
                    put_i64(&mut bytes, *reason as i64);
                }
            }
        }
        EventBody::CancellationRequested {
            cancellation_request_id,
            cycle_id,
            mode,
            generation,
        } => {
            put_i64(&mut bytes, cancellation_request_id.value());
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, *mode as i64);
            put_i64(&mut bytes, generation.value());
        }
        EventBody::CancellationReconciled {
            cancellation_request_id,
            cycle_id,
        } => {
            put_i64(&mut bytes, cancellation_request_id.value());
            put_i64(&mut bytes, cycle_id.value());
        }
        EventBody::CostPostmortemClosed {
            postmortem_id,
            reservation_id,
            cycle_id,
            resolution,
            charged,
        } => {
            put_i64(&mut bytes, postmortem_id.value());
            put_i64(&mut bytes, reservation_id.value());
            put_i64(&mut bytes, cycle_id.value());
            put_i64(&mut bytes, *resolution as i64);
            put_i64(&mut bytes, charged.value());
        }
        EventBody::ProjectCreated { project_id } | EventBody::ProjectChartered { project_id } => {
            put_i64(&mut bytes, project_id.value())
        }
        EventBody::ProjectStateChanged { project_id, state } => {
            put_i64(&mut bytes, project_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::ProjectMilestoneCompleted {
            project_milestone_id,
        } => put_i64(&mut bytes, project_milestone_id.value()),
        EventBody::TicketCreated {
            ticket_id,
            project_id,
        } => {
            put_i64(&mut bytes, ticket_id.value());
            put_i64(&mut bytes, project_id.value());
        }
        EventBody::TicketStateChanged { ticket_id, state } => {
            put_i64(&mut bytes, ticket_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::GraphObjectRevisionAdded {
            graph_object_id,
            graph_revision_id,
        } => {
            put_i64(&mut bytes, graph_object_id.value());
            put_i64(&mut bytes, graph_revision_id.value());
        }
        EventBody::GraphRevisionCommitted { graph_revision_id } => {
            put_i64(&mut bytes, graph_revision_id.value())
        }
        EventBody::GraphEdgeAdded { graph_edge_id } => put_i64(&mut bytes, graph_edge_id.value()),
        EventBody::EpisodeCreated {
            causal_episode_id,
            project_id,
        } => {
            put_i64(&mut bytes, causal_episode_id.value());
            put_i64(&mut bytes, project_id.value());
        }
        EventBody::EpisodeStateChanged {
            causal_episode_id,
            state,
        } => {
            put_i64(&mut bytes, causal_episode_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::AdversarialReviewRequested {
            adversarial_review_id,
        } => put_i64(&mut bytes, adversarial_review_id.value()),
        EventBody::AdversarialReviewerAssigned {
            adversarial_review_id,
            reviewer_principal_id,
            reviewer_actor_instance_id,
            reviewer_actor_attempt_id,
        } => {
            put_i64(&mut bytes, adversarial_review_id.value());
            put_i64(&mut bytes, reviewer_principal_id.value());
            put_i64(&mut bytes, reviewer_actor_instance_id.value());
            put_i64(&mut bytes, reviewer_actor_attempt_id.value());
        }
        EventBody::AdversarialReviewResolved {
            adversarial_review_id,
            state,
        } => {
            put_i64(&mut bytes, adversarial_review_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::ReviewChallengeSubmitted {
            review_challenge_id,
            author_principal_id,
        } => {
            put_i64(&mut bytes, review_challenge_id.value());
            put_i64(&mut bytes, author_principal_id.value());
        }
        EventBody::ReviewChallengeResponded {
            review_challenge_id,
        } => put_i64(&mut bytes, review_challenge_id.value()),
        EventBody::ReviewChallengeDispositioned {
            review_challenge_id,
            disposition,
        } => {
            put_i64(&mut bytes, review_challenge_id.value());
            put_i64(&mut bytes, *disposition as i64);
        }
        EventBody::PostmortemTriggered { postmortem_id }
        | EventBody::PostmortemClosed { postmortem_id } => {
            put_i64(&mut bytes, postmortem_id.value())
        }
        EventBody::PostmortemCausalClaimRecorded {
            postmortem_causal_claim_id,
        } => put_i64(&mut bytes, postmortem_causal_claim_id.value()),
        EventBody::PostmortemActionProposed {
            postmortem_action_proposal_id,
        } => put_i64(&mut bytes, postmortem_action_proposal_id.value()),
        EventBody::ActorConfigurationRegistered {
            actor_configuration_id,
            actor_configuration_revision_id,
        } => {
            put_i64(&mut bytes, actor_configuration_id.value());
            put_i64(&mut bytes, actor_configuration_revision_id.value());
        }
        EventBody::ContextPackRegistered { context_pack_id } => {
            put_i64(&mut bytes, context_pack_id.value())
        }
        EventBody::ActorInstanceAdmitted {
            actor_instance_id,
            principal_id,
        } => {
            put_i64(&mut bytes, actor_instance_id.value());
            put_i64(&mut bytes, principal_id.value());
        }
        EventBody::TicketAdmitted { ticket_id } => put_i64(&mut bytes, ticket_id.value()),
        EventBody::WorkItemRegistered {
            work_item_id,
            ticket_id,
            adversarial_review_id,
        } => {
            put_i64(&mut bytes, work_item_id.value());
            put_i64(&mut bytes, ticket_id.value());
            put_optional_i64(
                &mut bytes,
                adversarial_review_id.map(AdversarialReviewId::value),
            );
        }
        EventBody::WorkItemClaimed {
            work_item_id,
            work_lease_id,
            actor_instance_id,
        } => {
            put_i64(&mut bytes, work_item_id.value());
            put_i64(&mut bytes, work_lease_id.value());
            put_i64(&mut bytes, actor_instance_id.value());
        }
        EventBody::ActorAttemptStarted {
            actor_attempt_id,
            work_item_id,
            budget_reservation_id,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, work_item_id.value());
            put_i64(&mut bytes, budget_reservation_id.value());
        }
        EventBody::ActorAttemptTerminalAttested {
            actor_attempt_id,
            terminal_kind,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, *terminal_kind as i64);
        }
        EventBody::TicketAttemptValidated {
            actor_attempt_id,
            ticket_id,
        }
        | EventBody::TicketCompleted {
            actor_attempt_id,
            ticket_id,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, ticket_id.value());
        }
        EventBody::ActorAttemptRetryPrepared {
            actor_attempt_id,
            work_item_id,
            ticket_id,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, work_item_id.value());
            put_i64(&mut bytes, ticket_id.value());
        }
        EventBody::WorkLeaseExpired {
            work_lease_id,
            work_item_id,
        } => {
            put_i64(&mut bytes, work_lease_id.value());
            put_i64(&mut bytes, work_item_id.value());
        }
        EventBody::ActorAttemptCancellationRequested {
            actor_attempt_id,
            reason,
        } => {
            put_i64(&mut bytes, actor_attempt_id.value());
            put_i64(&mut bytes, *reason as i64);
        }
        EventBody::OutcomeObligationRegistered {
            outcome_obligation_id,
            project_id,
        } => {
            put_i64(&mut bytes, outcome_obligation_id.value());
            put_i64(&mut bytes, project_id.value());
        }
        EventBody::OutcomeObligationResolved {
            outcome_obligation_id,
            state,
        } => {
            put_i64(&mut bytes, outcome_obligation_id.value());
            put_i64(&mut bytes, *state as i64);
        }
        EventBody::ContentSealReceiptRecorded {
            content_seal_receipt_id,
            digest,
        } => {
            put_i64(&mut bytes, content_seal_receipt_id.value());
            put_bytes(&mut bytes, &digest.as_bytes());
        }
        EventBody::ContentObjectRegistered {
            content_object_id,
            content_seal_receipt_id,
        } => {
            put_i64(&mut bytes, content_object_id.value());
            put_i64(&mut bytes, content_seal_receipt_id.value());
        }
        EventBody::ForensicManifestRegistered {
            forensic_manifest_id,
            producing_deterministic_experiment_id,
            evaluator_output_content_object_id,
        } => {
            put_i64(&mut bytes, forensic_manifest_id.value());
            put_i64(&mut bytes, producing_deterministic_experiment_id.value());
            put_i64(&mut bytes, evaluator_output_content_object_id.value());
        }
        EventBody::DeterministicExperimentRegistered {
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
        } => {
            put_i64(&mut bytes, deterministic_experiment_id.value());
            put_i64(&mut bytes, evaluator_revision_id.value());
            put_i64(&mut bytes, input_manifest_id.value());
        }
        EventBody::DeterministicEvaluationReceiptRecorded {
            deterministic_evaluation_receipt_id,
            deterministic_experiment_id,
        } => {
            put_i64(&mut bytes, deterministic_evaluation_receipt_id.value());
            put_i64(&mut bytes, deterministic_experiment_id.value());
        }
        EventBody::DeterministicEvidenceAdmitted {
            evidence_admission_id,
            deterministic_evaluation_receipt_id,
            semantic_role,
            applicability,
        } => {
            put_i64(&mut bytes, evidence_admission_id.value());
            put_i64(&mut bytes, deterministic_evaluation_receipt_id.value());
            put_i64(&mut bytes, *semantic_role as i64);
            put_i64(&mut bytes, *applicability as i64);
        }
        EventBody::DeterministicExperimentClosed {
            deterministic_experiment_id,
        } => put_i64(&mut bytes, deterministic_experiment_id.value()),
    }
    Sha256Digest::of_bytes(&bytes)
}

fn put_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    put_i64(bytes, value.len() as i64);
    bytes.extend_from_slice(value);
}

fn put_i64(bytes: &mut Vec<u8>, value: i64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn put_optional_i64(bytes: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            put_i64(bytes, 1);
            put_i64(bytes, value);
        }
        None => put_i64(bytes, 0),
    }
}

fn insert_command_body(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    body: &CommandBody,
) -> Result<(), StoreError> {
    match body {
        CommandBody::CreateSocietyIdentity { name } => {
            transaction.execute(
                "INSERT INTO command_create_society_identity(command_row_id, name) VALUES (?1, ?2)",
                params![command_row_id, name.as_str()],
            )?;
        }
        CommandBody::InstallGrandArchitectOffice => {
            transaction.execute(
                "INSERT INTO command_install_grand_architect_office(command_row_id) VALUES (?1)",
                [command_row_id],
            )?;
        }
        CommandBody::InstallFoundingUniverseSeed { rendering_digest } => {
            transaction.execute("INSERT INTO command_install_founding_universe_seed(command_row_id, rendering_digest) VALUES (?1, ?2)", params![command_row_id, rendering_digest.as_bytes().as_slice()])?;
        }
        CommandBody::AppointInitialGrandArchitect { actor_display_name } => {
            transaction.execute("INSERT INTO command_appoint_initial_grand_architect(command_row_id, actor_display_name) VALUES (?1, ?2)", params![command_row_id, actor_display_name.as_str()])?;
        }
        CommandBody::SetR0HardCeiling { ceiling } => {
            transaction.execute("INSERT INTO command_set_r0_hard_ceiling(command_row_id, ceiling_micros) VALUES (?1, ?2)", params![command_row_id, ceiling.value()])?;
        }
        CommandBody::BootstrapSociety => {
            transaction.execute(
                "INSERT INTO command_bootstrap_society(command_row_id) VALUES (?1)",
                [command_row_id],
            )?;
        }
        CommandBody::ProposeOperatingCycle { treatment } => {
            transaction.execute("INSERT INTO command_propose_operating_cycle(command_row_id, treatment) VALUES (?1, ?2)", params![command_row_id, *treatment as i64])?;
        }
        CommandBody::AdmitOperatingCycle { cycle_id } => {
            transaction.execute("INSERT INTO command_admit_operating_cycle(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::StartGrandArchitectOfficeSession { cycle_id } => {
            transaction.execute("INSERT INTO command_start_grand_architect_office_session(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::RecordOfficeSessionReady { session_id } => {
            transaction.execute("INSERT INTO command_record_office_session_ready(command_row_id, grand_architect_office_session_id) VALUES (?1, ?2)", params![command_row_id, session_id.value()])?;
        }
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state,
        } => {
            transaction.execute("INSERT INTO command_record_office_session_terminal(command_row_id, grand_architect_office_session_id, terminal_state) VALUES (?1, ?2, ?3)", params![command_row_id, session_id.value(), *terminal_state as i64])?;
        }
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose,
        } => {
            transaction.execute("INSERT INTO command_open_office_turn(command_row_id, grand_architect_office_session_id, purpose) VALUES (?1, ?2, ?3)", params![command_row_id, session_id.value(), *purpose as i64])?;
        }
        CommandBody::SettleOfficeTurn { turn_id } => {
            transaction.execute("INSERT INTO command_settle_office_turn(command_row_id, office_turn_id) VALUES (?1, ?2)", params![command_row_id, turn_id.value()])?;
        }
        CommandBody::QuiesceOperatingCycle { cycle_id } => {
            transaction.execute("INSERT INTO command_quiesce_operating_cycle(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::RecordCycleDrained { cycle_id } => {
            transaction.execute("INSERT INTO command_record_cycle_drained(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::ResumeOperatingCycle { cycle_id } => {
            transaction.execute("INSERT INTO command_resume_operating_cycle(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::ReconcileOperatingCycle { cycle_id } => {
            transaction.execute("INSERT INTO command_reconcile_operating_cycle(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::CloseOperatingCycle { cycle_id } => {
            transaction.execute("INSERT INTO command_close_operating_cycle(command_row_id, operating_cycle_id) VALUES (?1, ?2)", params![command_row_id, cycle_id.value()])?;
        }
        CommandBody::ReserveBudget { cycle_id, amount } => {
            transaction.execute("INSERT INTO command_reserve_budget(command_row_id, operating_cycle_id, amount_micros) VALUES (?1, ?2, ?3)", params![command_row_id, cycle_id.value(), amount.value()])?;
        }
        CommandBody::ReconcileBudget {
            reservation_id,
            observation,
        } => {
            let (kind, known, unknown, unavailable): (i64, Option<i64>, Option<i64>, Option<i64>) =
                match observation {
                    CostObservation::Known(amount) => (1, Some(amount.value()), None, None),
                    CostObservation::Unknown(reason) => (2, None, Some(*reason as i64), None),
                    CostObservation::Unavailable(reason) => (3, None, None, Some(*reason as i64)),
                };
            transaction.execute("INSERT INTO command_reconcile_budget(command_row_id, budget_reservation_id, observation_kind, known_micros, unknown_reason, unavailable_reason) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![command_row_id, reservation_id.value(), kind, known, unknown, unavailable])?;
        }
        CommandBody::RequestCancellation { cycle_id, mode } => {
            transaction.execute("INSERT INTO command_request_cancellation(command_row_id, operating_cycle_id, cancellation_mode) VALUES (?1, ?2, ?3)", params![command_row_id, cycle_id.value(), *mode as i64])?;
        }
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        } => {
            transaction.execute("INSERT INTO command_reconcile_cancellation(command_row_id, cancellation_request_id) VALUES (?1, ?2)", params![command_row_id, cancellation_request_id.value()])?;
        }
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution,
        } => {
            transaction.execute("INSERT INTO command_close_cost_postmortem(command_row_id, postmortem_id, resolution_kind) VALUES (?1, ?2, ?3)", params![command_row_id, postmortem_id.value(), *resolution as i64])?;
        }
        CommandBody::CreateProject {
            operating_cycle_id,
            project_name,
        } => {
            transaction.execute(
                "INSERT INTO command_create_project VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_name.as_str()
                ],
            )?;
        }
        CommandBody::RecordContentSealReceipt { digest } => {
            transaction.execute(
                "INSERT INTO command_record_content_seal_receipt VALUES (?1, ?2)",
                params![command_row_id, digest.as_bytes().as_slice()],
            )?;
        }
        CommandBody::RegisterContentObject {
            content_seal_receipt_id,
        } => {
            transaction.execute(
                "INSERT INTO command_register_content_object VALUES (?1, ?2)",
                params![command_row_id, content_seal_receipt_id.value()],
            )?;
        }
        CommandBody::RegisterForensicManifest {
            operating_cycle_id,
            producing_deterministic_experiment_id,
            capture_policy,
            retention_access_class,
            evaluator_output_content_object_id,
        } => {
            transaction.execute(
                "INSERT INTO command_register_forensic_manifest VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    producing_deterministic_experiment_id.value(),
                    *capture_policy as i64,
                    *retention_access_class as i64,
                    evaluator_output_content_object_id.value()
                ],
            )?;
        }
        CommandBody::RegisterDeterministicExperiment {
            operating_cycle_id,
            project_id,
            ticket_id,
            target_graph_revision_id,
            evaluator_content_object_id,
            input_manifest_content_object_id,
        } => {
            transaction.execute("INSERT INTO command_register_deterministic_experiment VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![command_row_id, operating_cycle_id.value(), project_id.value(), ticket_id.value(), target_graph_revision_id.value(), evaluator_content_object_id.value(), input_manifest_content_object_id.value()])?;
        }
        CommandBody::RecordDeterministicEvaluationReceipt {
            operating_cycle_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            forensic_manifest_id,
            evaluator_output_content_object_id,
        } => {
            transaction.execute("INSERT INTO command_record_deterministic_evaluation_receipt VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![command_row_id, operating_cycle_id.value(), deterministic_experiment_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), forensic_manifest_id.value(), evaluator_output_content_object_id.value()])?;
        }
        CommandBody::AdmitDeterministicEvidence {
            operating_cycle_id,
            deterministic_evaluation_receipt_id,
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
            evaluator_output_content_object_id,
            related_graph_revision_id,
            semantic_role,
            applicability,
            limitation,
        } => {
            transaction.execute("INSERT INTO command_admit_deterministic_evidence VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", params![command_row_id, operating_cycle_id.value(), deterministic_evaluation_receipt_id.value(), deterministic_experiment_id.value(), evaluator_revision_id.value(), input_manifest_id.value(), evaluator_output_content_object_id.value(), related_graph_revision_id.value(), *semantic_role as i64, *applicability as i64, limitation.as_str()])?;
        }
        CommandBody::CloseDeterministicExperiment {
            operating_cycle_id,
            deterministic_experiment_id,
        } => {
            transaction.execute(
                "INSERT INTO command_close_deterministic_experiment VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    deterministic_experiment_id.value()
                ],
            )?;
        }
        CommandBody::RegisterActorConfiguration {
            configuration_name,
            model_policy,
            primary_attractor,
        } => {
            transaction.execute(
                "INSERT INTO command_register_actor_configuration VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    configuration_name.as_str(),
                    *model_policy as i64,
                    *primary_attractor as i64
                ],
            )?;
        }
        CommandBody::RegisterContextPack {
            operating_cycle_id,
            purpose,
            rendering_digest,
        } => {
            transaction.execute(
                "INSERT INTO command_register_context_pack VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    *purpose as i64,
                    rendering_digest.as_bytes().as_slice()
                ],
            )?;
        }
        CommandBody::AdmitActorInstance {
            operating_cycle_id,
            actor_configuration_revision_id,
            execution_profile_id,
            actor_display_name,
        } => {
            transaction.execute(
                "INSERT INTO command_admit_actor_instance VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    actor_configuration_revision_id.value(),
                    execution_profile_id.value(),
                    actor_display_name.as_str()
                ],
            )?;
        }
        CommandBody::AdmitTicket {
            operating_cycle_id,
            ticket_id,
        } => {
            transaction.execute(
                "INSERT INTO command_admit_ticket VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    ticket_id.value()
                ],
            )?;
        }
        CommandBody::RegisterWorkItem {
            operating_cycle_id,
            ticket_id,
            actor_instance_id,
            context_pack_id,
            work_kind,
            adversarial_review_id,
            assignment,
        } => {
            transaction.execute(
                "INSERT INTO command_register_work_item VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    ticket_id.value(),
                    actor_instance_id.value(),
                    context_pack_id.value(),
                    *work_kind as i64,
                    adversarial_review_id.map(AdversarialReviewId::value),
                    assignment.as_str()
                ],
            )?;
        }
        CommandBody::ClaimWorkItem {
            operating_cycle_id,
            work_item_id,
        } => {
            transaction.execute(
                "INSERT INTO command_claim_work_item VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    work_item_id.value()
                ],
            )?;
        }
        CommandBody::StartActorAttempt {
            operating_cycle_id,
            work_item_id,
            reservation_amount,
        } => {
            transaction.execute(
                "INSERT INTO command_start_actor_attempt VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    work_item_id.value(),
                    reservation_amount.value()
                ],
            )?;
        }
        CommandBody::AttestActorAttemptTerminal {
            actor_attempt_id,
            terminal_kind,
        } => {
            transaction.execute(
                "INSERT INTO command_attest_actor_attempt_terminal VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    actor_attempt_id.value(),
                    *terminal_kind as i64
                ],
            )?;
        }
        CommandBody::ValidateTicketAttempt {
            operating_cycle_id,
            actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO command_validate_ticket_attempt VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    actor_attempt_id.value()
                ],
            )?;
        }
        CommandBody::RetryActorAttempt {
            operating_cycle_id,
            actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO command_retry_actor_attempt VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    actor_attempt_id.value()
                ],
            )?;
        }
        CommandBody::CompleteTicket {
            operating_cycle_id,
            actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO command_complete_ticket VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    actor_attempt_id.value()
                ],
            )?;
        }
        CommandBody::ExpireWorkLease { work_lease_id } => {
            transaction.execute(
                "INSERT INTO command_expire_work_lease VALUES (?1, ?2)",
                params![command_row_id, work_lease_id.value()],
            )?;
        }
        CommandBody::CancelActorAttempt {
            actor_attempt_id,
            reason,
        } => {
            transaction.execute(
                "INSERT INTO command_cancel_actor_attempt VALUES (?1, ?2, ?3)",
                params![command_row_id, actor_attempt_id.value(), *reason as i64],
            )?;
        }
        CommandBody::RegisterOutcomeObligation {
            operating_cycle_id,
            project_id,
            obligation,
        } => {
            transaction.execute(
                "INSERT INTO command_register_outcome_obligation VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    obligation.as_str()
                ],
            )?;
        }
        CommandBody::ResolveOutcomeObligation {
            operating_cycle_id,
            outcome_obligation_id,
            disposition,
        } => {
            transaction.execute(
                "INSERT INTO command_resolve_outcome_obligation VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    outcome_obligation_id.value(),
                    *disposition as i64
                ],
            )?;
        }
        CommandBody::CharterProject {
            operating_cycle_id,
            project_id,
            objective,
            initial_milestone,
            stop_condition,
        } => {
            transaction.execute(
                "INSERT INTO command_charter_project VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    objective.as_str(),
                    initial_milestone.as_str(),
                    stop_condition.as_str()
                ],
            )?;
        }
        CommandBody::TransitionProject {
            operating_cycle_id,
            project_id,
            target,
        } => {
            transaction.execute(
                "INSERT INTO command_transition_project VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    *target as i64
                ],
            )?;
        }
        CommandBody::CompleteProjectMilestone {
            operating_cycle_id,
            project_milestone_id,
        } => {
            transaction.execute(
                "INSERT INTO command_complete_project_milestone VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_milestone_id.value()
                ],
            )?;
        }
        CommandBody::ReopenProject {
            operating_cycle_id,
            project_id,
        } => {
            transaction.execute(
                "INSERT INTO command_reopen_project VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value()
                ],
            )?;
        }
        CommandBody::CreateTicket {
            operating_cycle_id,
            project_id,
            ticket_title,
            acceptance_condition,
            prerequisite_ticket_id,
        } => {
            transaction.execute(
                "INSERT INTO command_create_ticket VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    ticket_title.as_str(),
                    acceptance_condition.as_str(),
                    prerequisite_ticket_id.map(TicketId::value)
                ],
            )?;
        }
        CommandBody::TransitionTicket {
            operating_cycle_id,
            ticket_id,
            target,
        } => {
            transaction.execute(
                "INSERT INTO command_transition_ticket VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    ticket_id.value(),
                    *target as i64
                ],
            )?;
        }
        CommandBody::AddGraphObjectRevision {
            operating_cycle_id,
            project_id,
            causal_episode_id,
            graph_object_id,
            body,
        } => {
            transaction.execute(
                "INSERT INTO command_add_graph_object_revision VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    causal_episode_id.map(CausalEpisodeId::value),
                    graph_object_id.map(GraphObjectId::value)
                ],
            )?;
            match body {
                GraphRevisionBody::Observation { observation } => {
                    transaction.execute(
                        "INSERT INTO command_add_observation_revision VALUES (?1, ?2)",
                        params![command_row_id, observation.as_str()],
                    )?;
                }
                GraphRevisionBody::Hypothesis { hypothesis } => {
                    transaction.execute(
                        "INSERT INTO command_add_hypothesis_revision VALUES (?1, ?2)",
                        params![command_row_id, hypothesis.as_str()],
                    )?;
                }
            }
        }
        CommandBody::CommitGraphRevision {
            operating_cycle_id,
            graph_revision_id,
        } => {
            transaction.execute(
                "INSERT INTO command_commit_graph_revision VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    graph_revision_id.value()
                ],
            )?;
        }
        CommandBody::AddGraphEdge {
            operating_cycle_id,
            project_id,
            from_graph_revision_id,
            to_graph_revision_id,
            edge_kind,
        } => {
            transaction.execute(
                "INSERT INTO command_add_graph_edge VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    from_graph_revision_id.value(),
                    to_graph_revision_id.value(),
                    *edge_kind as i64
                ],
            )?;
        }
        CommandBody::CreateEpisode {
            operating_cycle_id,
            project_id,
        } => {
            transaction.execute(
                "INSERT INTO command_create_episode VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value()
                ],
            )?;
        }
        CommandBody::TransitionEpisode {
            operating_cycle_id,
            causal_episode_id,
            target,
        } => {
            transaction.execute(
                "INSERT INTO command_transition_episode VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    causal_episode_id.value(),
                    *target as i64
                ],
            )?;
        }
        CommandBody::ReopenEpisode {
            operating_cycle_id,
            causal_episode_id,
        } => {
            transaction.execute(
                "INSERT INTO command_reopen_episode VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    causal_episode_id.value()
                ],
            )?;
        }
        CommandBody::RequestAdversarialReview {
            operating_cycle_id,
            project_id,
            target_graph_revision_id,
        } => {
            transaction.execute(
                "INSERT INTO command_request_adversarial_review VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    target_graph_revision_id.value()
                ],
            )?;
        }
        CommandBody::AssignAdversarialReviewer {
            operating_cycle_id,
            adversarial_review_id,
            reviewer_principal_id,
            reviewer_actor_instance_id,
            reviewer_actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO command_assign_adversarial_reviewer VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    adversarial_review_id.value(),
                    reviewer_principal_id.value(),
                    reviewer_actor_instance_id.value(),
                    reviewer_actor_attempt_id.value()
                ],
            )?;
        }
        CommandBody::SubmitReviewChallenge {
            operating_cycle_id,
            adversarial_review_id,
            target_graph_revision_id,
            author_principal_id,
            severity,
            failure_hypothesis,
        } => {
            transaction.execute(
                "INSERT INTO command_submit_review_challenge VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    adversarial_review_id.value(),
                    target_graph_revision_id.value(),
                    author_principal_id.value(),
                    *severity as i64,
                    failure_hypothesis.as_str()
                ],
            )?;
        }
        CommandBody::RespondToReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            response,
        } => {
            transaction.execute(
                "INSERT INTO command_respond_to_review_challenge VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    review_challenge_id.value(),
                    response.as_str()
                ],
            )?;
        }
        CommandBody::DispositionReviewChallenge {
            operating_cycle_id,
            review_challenge_id,
            disposition,
        } => {
            transaction.execute(
                "INSERT INTO command_disposition_review_challenge VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    review_challenge_id.value(),
                    *disposition as i64
                ],
            )?;
        }
        CommandBody::ResolveAdversarialReview {
            operating_cycle_id,
            adversarial_review_id,
            resolution,
        } => {
            transaction.execute(
                "INSERT INTO command_resolve_adversarial_review VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    adversarial_review_id.value(),
                    *resolution as i64
                ],
            )?;
        }
        CommandBody::TriggerPostmortem {
            operating_cycle_id,
            project_id,
            causal_episode_id,
        } => {
            transaction.execute(
                "INSERT INTO command_trigger_postmortem VALUES (?1, ?2, ?3, ?4)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    project_id.value(),
                    causal_episode_id.map(CausalEpisodeId::value)
                ],
            )?;
        }
        CommandBody::RecordPostmortemCausalClaim {
            operating_cycle_id,
            postmortem_id,
            claim_kind,
            claim,
        } => {
            transaction.execute(
                "INSERT INTO command_record_postmortem_causal_claim VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    postmortem_id.value(),
                    *claim_kind as i64,
                    claim.as_str()
                ],
            )?;
        }
        CommandBody::ProposePostmortemAction {
            operating_cycle_id,
            postmortem_id,
            action_kind,
            action,
        } => {
            transaction.execute(
                "INSERT INTO command_propose_postmortem_action VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    postmortem_id.value(),
                    *action_kind as i64,
                    action.as_str()
                ],
            )?;
        }
        CommandBody::ClosePostmortem {
            operating_cycle_id,
            postmortem_id,
        } => {
            transaction.execute(
                "INSERT INTO command_close_postmortem VALUES (?1, ?2, ?3)",
                params![
                    command_row_id,
                    operating_cycle_id.value(),
                    postmortem_id.value()
                ],
            )?;
        }
    }
    Ok(())
}

fn insert_event(
    transaction: &Transaction<'_>,
    command_row_id: i64,
    command_id: &CommandId,
    body: &EventBody,
) -> Result<EventId, StoreError> {
    let sequence: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(event_sequence), 0) + 1 FROM events",
        [],
        |row| row.get(0),
    )?;
    let event_id = EventId::try_from(transaction.query_row(
        "SELECT COALESCE(MAX(event_id), 0) + 1 FROM events",
        [],
        |row| row.get::<_, i64>(0),
    )?)
    .map_err(|_| StoreError::InvalidStoredValue)?;
    transaction.execute(
        "INSERT INTO events(event_id, command_row_id, event_kind, event_sequence, event_fingerprint)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            event_id.value(),
            command_row_id,
            body.kind() as i64,
            sequence,
            event_fingerprint(event_id, command_id, body)
                .as_bytes()
                .as_slice()
        ],
    )?;
    insert_event_body(transaction, event_id, body)?;
    Ok(event_id)
}

fn insert_event_body(
    transaction: &Transaction<'_>,
    event_id: EventId,
    body: &EventBody,
) -> Result<(), StoreError> {
    match body {
        EventBody::SocietyIdentityCreated { society_id } => {
            transaction.execute(
                "INSERT INTO event_society_identity_created(event_id, society_id) VALUES (?1, ?2)",
                params![event_id.value(), society_id.value()],
            )?;
        }
        EventBody::GrandArchitectOfficeInstalled { office_id } => {
            transaction.execute("INSERT INTO event_grand_architect_office_installed(event_id, office_id) VALUES (?1, ?2)", params![event_id.value(), office_id.value()])?;
        }
        EventBody::FoundingUniverseSeedInstalled { seed_id } => {
            transaction.execute("INSERT INTO event_founding_universe_seed_installed(event_id, universe_seed_id) VALUES (?1, ?2)", params![event_id.value(), seed_id.value()])?;
        }
        EventBody::GrandArchitectAppointed {
            occupancy_id,
            principal_id,
        } => {
            transaction.execute("INSERT INTO event_grand_architect_appointed(event_id, office_occupancy_id, principal_id) VALUES (?1, ?2, ?3)", params![event_id.value(), occupancy_id.value(), principal_id.value()])?;
        }
        EventBody::R0HardCeilingSet {
            society_id,
            ceiling,
        } => {
            transaction.execute("INSERT INTO event_r0_hard_ceiling_set(event_id, society_id, ceiling_micros) VALUES (?1, ?2, ?3)", params![event_id.value(), society_id.value(), ceiling.value()])?;
        }
        EventBody::SocietyBootstrapped { society_id } => {
            transaction.execute(
                "INSERT INTO event_society_bootstrapped(event_id, society_id) VALUES (?1, ?2)",
                params![event_id.value(), society_id.value()],
            )?;
        }
        EventBody::OperatingCycleProposed {
            cycle_id,
            generation,
            treatment,
        } => {
            transaction.execute("INSERT INTO event_operating_cycle_proposed(event_id, operating_cycle_id, admission_generation, treatment) VALUES (?1, ?2, ?3, ?4)", params![event_id.value(), cycle_id.value(), generation.value(), *treatment as i64])?;
        }
        EventBody::OperatingCycleStateChanged {
            cycle_id,
            state,
            generation,
        } => {
            transaction.execute("INSERT INTO event_operating_cycle_state_changed(event_id, operating_cycle_id, lifecycle_state, admission_generation) VALUES (?1, ?2, ?3, ?4)", params![event_id.value(), cycle_id.value(), *state as i64, generation.value()])?;
        }
        EventBody::GrandArchitectOfficeSessionStarted {
            session_id,
            cycle_id,
        } => {
            transaction.execute("INSERT INTO event_grand_architect_office_session_started(event_id, grand_architect_office_session_id, operating_cycle_id) VALUES (?1, ?2, ?3)", params![event_id.value(), session_id.value(), cycle_id.value()])?;
        }
        EventBody::GrandArchitectOfficeSessionStateChanged { session_id, state } => {
            transaction.execute("INSERT INTO event_grand_architect_office_session_state_changed(event_id, grand_architect_office_session_id, lifecycle_state) VALUES (?1, ?2, ?3)", params![event_id.value(), session_id.value(), *state as i64])?;
        }
        EventBody::OfficeTurnOpened {
            turn_id,
            session_id,
            purpose,
        } => {
            transaction.execute("INSERT INTO event_office_turn_opened(event_id, office_turn_id, grand_architect_office_session_id, purpose) VALUES (?1, ?2, ?3, ?4)", params![event_id.value(), turn_id.value(), session_id.value(), *purpose as i64])?;
        }
        EventBody::OfficeTurnSettled {
            turn_id,
            session_id,
        } => {
            transaction.execute("INSERT INTO event_office_turn_settled(event_id, office_turn_id, grand_architect_office_session_id) VALUES (?1, ?2, ?3)", params![event_id.value(), turn_id.value(), session_id.value()])?;
        }
        EventBody::BudgetReserved {
            reservation_id,
            cycle_id,
            amount,
        } => {
            transaction.execute("INSERT INTO event_budget_reserved(event_id, budget_reservation_id, operating_cycle_id, amount_micros) VALUES (?1, ?2, ?3, ?4)", params![event_id.value(), reservation_id.value(), cycle_id.value(), amount.value()])?;
        }
        EventBody::BudgetReconciled {
            reservation_id,
            observed,
        } => {
            transaction.execute("INSERT INTO event_budget_reconciled(event_id, budget_reservation_id, observed_micros) VALUES (?1, ?2, ?3)", params![event_id.value(), reservation_id.value(), observed.value()])?;
        }
        EventBody::BudgetAdmissionFrozen {
            reservation_id,
            cycle_id,
            cancellation_request_id,
            postmortem_id,
            reason,
        } => {
            let (reason_kind, observed, reserved, unknown, unavailable) =
                budget_freeze_reason_to_sql(*reason);
            transaction.execute("INSERT INTO event_budget_admission_frozen(event_id, budget_reservation_id, operating_cycle_id, cancellation_request_id, postmortem_id, freeze_reason_kind, observed_micros, reserved_micros, unknown_reason, unavailable_reason) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)", params![event_id.value(), reservation_id.value(), cycle_id.value(), cancellation_request_id.value(), postmortem_id.value(), reason_kind, observed, reserved, unknown, unavailable])?;
        }
        EventBody::CancellationRequested {
            cancellation_request_id,
            cycle_id,
            mode,
            generation,
        } => {
            transaction.execute("INSERT INTO event_cancellation_requested(event_id, cancellation_request_id, operating_cycle_id, cancellation_mode, admission_generation) VALUES (?1, ?2, ?3, ?4, ?5)", params![event_id.value(), cancellation_request_id.value(), cycle_id.value(), *mode as i64, generation.value()])?;
        }
        EventBody::CancellationReconciled {
            cancellation_request_id,
            cycle_id,
        } => {
            transaction.execute("INSERT INTO event_cancellation_reconciled(event_id, cancellation_request_id, operating_cycle_id) VALUES (?1, ?2, ?3)", params![event_id.value(), cancellation_request_id.value(), cycle_id.value()])?;
        }
        EventBody::CostPostmortemClosed {
            postmortem_id,
            reservation_id,
            cycle_id,
            resolution,
            charged,
        } => {
            transaction.execute("INSERT INTO event_cost_postmortem_closed(event_id, postmortem_id, budget_reservation_id, operating_cycle_id, resolution_kind, charged_micros) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![event_id.value(), postmortem_id.value(), reservation_id.value(), cycle_id.value(), *resolution as i64, charged.value()])?;
        }
        EventBody::ProjectCreated { project_id } => {
            transaction.execute(
                "INSERT INTO event_project_created VALUES (?1, ?2)",
                params![event_id.value(), project_id.value()],
            )?;
        }
        EventBody::ProjectChartered { project_id } => {
            transaction.execute(
                "INSERT INTO event_project_chartered VALUES (?1, ?2)",
                params![event_id.value(), project_id.value()],
            )?;
        }
        EventBody::ProjectStateChanged { project_id, state } => {
            transaction.execute(
                "INSERT INTO event_project_state_changed VALUES (?1, ?2, ?3)",
                params![event_id.value(), project_id.value(), *state as i64],
            )?;
        }
        EventBody::ProjectMilestoneCompleted {
            project_milestone_id,
        } => {
            transaction.execute(
                "INSERT INTO event_project_milestone_completed VALUES (?1, ?2)",
                params![event_id.value(), project_milestone_id.value()],
            )?;
        }
        EventBody::TicketCreated {
            ticket_id,
            project_id,
        } => {
            transaction.execute(
                "INSERT INTO event_ticket_created VALUES (?1, ?2, ?3)",
                params![event_id.value(), ticket_id.value(), project_id.value()],
            )?;
        }
        EventBody::TicketStateChanged { ticket_id, state } => {
            transaction.execute(
                "INSERT INTO event_ticket_state_changed VALUES (?1, ?2, ?3)",
                params![event_id.value(), ticket_id.value(), *state as i64],
            )?;
        }
        EventBody::GraphObjectRevisionAdded {
            graph_object_id,
            graph_revision_id,
        } => {
            transaction.execute(
                "INSERT INTO event_graph_object_revision_added VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    graph_object_id.value(),
                    graph_revision_id.value()
                ],
            )?;
        }
        EventBody::GraphRevisionCommitted { graph_revision_id } => {
            transaction.execute(
                "INSERT INTO event_graph_revision_committed VALUES (?1, ?2)",
                params![event_id.value(), graph_revision_id.value()],
            )?;
        }
        EventBody::GraphEdgeAdded { graph_edge_id } => {
            transaction.execute(
                "INSERT INTO event_graph_edge_added VALUES (?1, ?2)",
                params![event_id.value(), graph_edge_id.value()],
            )?;
        }
        EventBody::EpisodeCreated {
            causal_episode_id,
            project_id,
        } => {
            transaction.execute(
                "INSERT INTO event_episode_created VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    causal_episode_id.value(),
                    project_id.value()
                ],
            )?;
        }
        EventBody::EpisodeStateChanged {
            causal_episode_id,
            state,
        } => {
            transaction.execute(
                "INSERT INTO event_episode_state_changed VALUES (?1, ?2, ?3)",
                params![event_id.value(), causal_episode_id.value(), *state as i64],
            )?;
        }
        EventBody::AdversarialReviewRequested {
            adversarial_review_id,
        } => {
            transaction.execute(
                "INSERT INTO event_adversarial_review_requested VALUES (?1, ?2)",
                params![event_id.value(), adversarial_review_id.value()],
            )?;
        }
        EventBody::AdversarialReviewerAssigned {
            adversarial_review_id,
            reviewer_principal_id,
            reviewer_actor_instance_id,
            reviewer_actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO event_adversarial_reviewer_assigned VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event_id.value(),
                    adversarial_review_id.value(),
                    reviewer_principal_id.value(),
                    reviewer_actor_instance_id.value(),
                    reviewer_actor_attempt_id.value()
                ],
            )?;
        }
        EventBody::ReviewChallengeSubmitted {
            review_challenge_id,
            author_principal_id,
        } => {
            transaction.execute(
                "INSERT INTO event_review_challenge_submitted VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    review_challenge_id.value(),
                    author_principal_id.value()
                ],
            )?;
        }
        EventBody::ReviewChallengeResponded {
            review_challenge_id,
        } => {
            transaction.execute(
                "INSERT INTO event_review_challenge_responded VALUES (?1, ?2)",
                params![event_id.value(), review_challenge_id.value()],
            )?;
        }
        EventBody::ReviewChallengeDispositioned {
            review_challenge_id,
            disposition,
        } => {
            transaction.execute(
                "INSERT INTO event_review_challenge_dispositioned VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    review_challenge_id.value(),
                    *disposition as i64
                ],
            )?;
        }
        EventBody::AdversarialReviewResolved {
            adversarial_review_id,
            state,
        } => {
            transaction.execute(
                "INSERT INTO event_adversarial_review_resolved VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    adversarial_review_id.value(),
                    *state as i64
                ],
            )?;
        }
        EventBody::PostmortemTriggered { postmortem_id } => {
            transaction.execute(
                "INSERT INTO event_postmortem_triggered VALUES (?1, ?2)",
                params![event_id.value(), postmortem_id.value()],
            )?;
        }
        EventBody::PostmortemCausalClaimRecorded {
            postmortem_causal_claim_id,
        } => {
            transaction.execute(
                "INSERT INTO event_postmortem_causal_claim_recorded VALUES (?1, ?2)",
                params![event_id.value(), postmortem_causal_claim_id.value()],
            )?;
        }
        EventBody::PostmortemActionProposed {
            postmortem_action_proposal_id,
        } => {
            transaction.execute(
                "INSERT INTO event_postmortem_action_proposed VALUES (?1, ?2)",
                params![event_id.value(), postmortem_action_proposal_id.value()],
            )?;
        }
        EventBody::PostmortemClosed { postmortem_id } => {
            transaction.execute(
                "INSERT INTO event_postmortem_closed VALUES (?1, ?2)",
                params![event_id.value(), postmortem_id.value()],
            )?;
        }
        EventBody::ActorConfigurationRegistered {
            actor_configuration_id,
            actor_configuration_revision_id,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_configuration_registered VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    actor_configuration_id.value(),
                    actor_configuration_revision_id.value()
                ],
            )?;
        }
        EventBody::ContextPackRegistered { context_pack_id } => {
            transaction.execute(
                "INSERT INTO event_context_pack_registered VALUES (?1, ?2)",
                params![event_id.value(), context_pack_id.value()],
            )?;
        }
        EventBody::ActorInstanceAdmitted {
            actor_instance_id,
            principal_id,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_instance_admitted VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    actor_instance_id.value(),
                    principal_id.value()
                ],
            )?;
        }
        EventBody::TicketAdmitted { ticket_id } => {
            transaction.execute(
                "INSERT INTO event_ticket_admitted VALUES (?1, ?2)",
                params![event_id.value(), ticket_id.value()],
            )?;
        }
        EventBody::WorkItemRegistered {
            work_item_id,
            ticket_id,
            adversarial_review_id,
        } => {
            transaction.execute(
                "INSERT INTO event_work_item_registered VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    work_item_id.value(),
                    ticket_id.value(),
                    adversarial_review_id.map(AdversarialReviewId::value)
                ],
            )?;
        }
        EventBody::WorkItemClaimed {
            work_item_id,
            work_lease_id,
            actor_instance_id,
        } => {
            transaction.execute(
                "INSERT INTO event_work_item_claimed VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    work_item_id.value(),
                    work_lease_id.value(),
                    actor_instance_id.value()
                ],
            )?;
        }
        EventBody::ActorAttemptStarted {
            actor_attempt_id,
            work_item_id,
            budget_reservation_id,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_attempt_started VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    actor_attempt_id.value(),
                    work_item_id.value(),
                    budget_reservation_id.value()
                ],
            )?;
        }
        EventBody::ActorAttemptTerminalAttested {
            actor_attempt_id,
            terminal_kind,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_attempt_terminal_attested VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    actor_attempt_id.value(),
                    *terminal_kind as i64
                ],
            )?;
        }
        EventBody::TicketAttemptValidated {
            actor_attempt_id,
            ticket_id,
        } => {
            transaction.execute(
                "INSERT INTO event_ticket_attempt_validated VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    actor_attempt_id.value(),
                    ticket_id.value()
                ],
            )?;
        }
        EventBody::ActorAttemptRetryPrepared {
            actor_attempt_id,
            work_item_id,
            ticket_id,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_attempt_retry_prepared VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    actor_attempt_id.value(),
                    work_item_id.value(),
                    ticket_id.value()
                ],
            )?;
        }
        EventBody::TicketCompleted {
            ticket_id,
            actor_attempt_id,
        } => {
            transaction.execute(
                "INSERT INTO event_ticket_completed VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    ticket_id.value(),
                    actor_attempt_id.value()
                ],
            )?;
        }
        EventBody::WorkLeaseExpired {
            work_lease_id,
            work_item_id,
        } => {
            transaction.execute(
                "INSERT INTO event_work_lease_expired VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    work_lease_id.value(),
                    work_item_id.value()
                ],
            )?;
        }
        EventBody::ActorAttemptCancellationRequested {
            actor_attempt_id,
            reason,
        } => {
            transaction.execute(
                "INSERT INTO event_actor_attempt_cancellation_requested VALUES (?1, ?2, ?3)",
                params![event_id.value(), actor_attempt_id.value(), *reason as i64],
            )?;
        }
        EventBody::OutcomeObligationRegistered {
            outcome_obligation_id,
            project_id,
        } => {
            transaction.execute(
                "INSERT INTO event_outcome_obligation_registered VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    outcome_obligation_id.value(),
                    project_id.value()
                ],
            )?;
        }
        EventBody::OutcomeObligationResolved {
            outcome_obligation_id,
            state,
        } => {
            transaction.execute(
                "INSERT INTO event_outcome_obligation_resolved VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    outcome_obligation_id.value(),
                    *state as i64
                ],
            )?;
        }
        EventBody::ContentSealReceiptRecorded {
            content_seal_receipt_id,
            digest,
        } => {
            transaction.execute(
                "INSERT INTO event_content_seal_receipt_recorded VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    content_seal_receipt_id.value(),
                    digest.as_bytes().as_slice()
                ],
            )?;
        }
        EventBody::ContentObjectRegistered {
            content_object_id,
            content_seal_receipt_id,
        } => {
            transaction.execute(
                "INSERT INTO event_content_object_registered VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    content_object_id.value(),
                    content_seal_receipt_id.value()
                ],
            )?;
        }
        EventBody::ForensicManifestRegistered {
            forensic_manifest_id,
            producing_deterministic_experiment_id,
            evaluator_output_content_object_id,
        } => {
            transaction.execute(
                "INSERT INTO event_forensic_manifest_registered VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    forensic_manifest_id.value(),
                    producing_deterministic_experiment_id.value(),
                    evaluator_output_content_object_id.value()
                ],
            )?;
        }
        EventBody::DeterministicExperimentRegistered {
            deterministic_experiment_id,
            evaluator_revision_id,
            input_manifest_id,
        } => {
            transaction.execute(
                "INSERT INTO event_deterministic_experiment_registered VALUES (?1, ?2, ?3, ?4)",
                params![
                    event_id.value(),
                    deterministic_experiment_id.value(),
                    evaluator_revision_id.value(),
                    input_manifest_id.value()
                ],
            )?;
        }
        EventBody::DeterministicEvaluationReceiptRecorded {
            deterministic_evaluation_receipt_id,
            deterministic_experiment_id,
        } => {
            transaction.execute(
                "INSERT INTO event_deterministic_evaluation_receipt_recorded VALUES (?1, ?2, ?3)",
                params![
                    event_id.value(),
                    deterministic_evaluation_receipt_id.value(),
                    deterministic_experiment_id.value()
                ],
            )?;
        }
        EventBody::DeterministicEvidenceAdmitted {
            evidence_admission_id,
            deterministic_evaluation_receipt_id,
            semantic_role,
            applicability,
        } => {
            transaction.execute(
                "INSERT INTO event_deterministic_evidence_admitted VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event_id.value(),
                    evidence_admission_id.value(),
                    deterministic_evaluation_receipt_id.value(),
                    *semantic_role as i64,
                    *applicability as i64
                ],
            )?;
        }
        EventBody::DeterministicExperimentClosed {
            deterministic_experiment_id,
        } => {
            transaction.execute(
                "INSERT INTO event_deterministic_experiment_closed VALUES (?1, ?2)",
                params![event_id.value(), deterministic_experiment_id.value()],
            )?;
        }
    }
    Ok(())
}

fn decode_event_body(
    connection: &Connection,
    event_id: i64,
    kind: i64,
    command_id: &CommandId,
) -> Result<EventBody, StoreError> {
    let event_id_typed = EventId::try_from(event_id).map_err(|_| StoreError::InvalidStoredValue)?;
    let kind = event_kind_from_i64(kind)?;
    verify_exact_event_body(connection, event_id_typed, kind)?;
    let body = match kind {
        EventKind::SocietyIdentityCreated => EventBody::SocietyIdentityCreated {
            society_id: query_event_id(
                connection,
                "event_society_identity_created",
                "society_id",
                event_id_typed,
            )?,
        },
        EventKind::GrandArchitectOfficeInstalled => EventBody::GrandArchitectOfficeInstalled {
            office_id: query_event_id(
                connection,
                "event_grand_architect_office_installed",
                "office_id",
                event_id_typed,
            )?,
        },
        EventKind::FoundingUniverseSeedInstalled => EventBody::FoundingUniverseSeedInstalled {
            seed_id: query_event_id(
                connection,
                "event_founding_universe_seed_installed",
                "universe_seed_id",
                event_id_typed,
            )?,
        },
        EventKind::GrandArchitectAppointed => {
            let (occupancy_id, principal_id) = connection
                .query_row(
                    "SELECT office_occupancy_id, principal_id
                     FROM event_grand_architect_appointed WHERE event_id = ?1",
                    [event_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing grand architect appointment event body",
                ))?;
            EventBody::GrandArchitectAppointed {
                occupancy_id: OfficeOccupancyId::try_from(occupancy_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                principal_id: PrincipalId::try_from(principal_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::R0HardCeilingSet => {
            let (society, ceiling) = connection.query_row("SELECT society_id, ceiling_micros FROM event_r0_hard_ceiling_set WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing r0 ceiling event body"))?;
            EventBody::R0HardCeilingSet {
                society_id: SocietyId::try_from(society)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ceiling: UsdMicros::try_from(ceiling)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::SocietyBootstrapped => EventBody::SocietyBootstrapped {
            society_id: query_event_id(
                connection,
                "event_society_bootstrapped",
                "society_id",
                event_id_typed,
            )?,
        },
        EventKind::OperatingCycleProposed => {
            let (cycle, generation, treatment) = connection.query_row("SELECT operating_cycle_id, admission_generation, treatment FROM event_operating_cycle_proposed WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing cycle proposal event body"))?;
            EventBody::OperatingCycleProposed {
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                generation: AdmissionGeneration::try_from(generation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                treatment: operating_cycle_treatment_from_i64(treatment)?,
            }
        }
        EventKind::OperatingCycleStateChanged => {
            let (cycle, state, generation) = connection.query_row("SELECT operating_cycle_id, lifecycle_state, admission_generation FROM event_operating_cycle_state_changed WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing cycle transition event body"))?;
            EventBody::OperatingCycleStateChanged {
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                state: operating_cycle_state_from_i64(state)?,
                generation: AdmissionGeneration::try_from(generation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::GrandArchitectOfficeSessionStarted => {
            let (session, cycle) = connection.query_row("SELECT grand_architect_office_session_id, operating_cycle_id FROM event_grand_architect_office_session_started WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing office session event body"))?;
            EventBody::GrandArchitectOfficeSessionStarted {
                session_id: GrandArchitectOfficeSessionId::try_from(session)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::GrandArchitectOfficeSessionStateChanged => {
            let (session, state) = connection.query_row("SELECT grand_architect_office_session_id, lifecycle_state FROM event_grand_architect_office_session_state_changed WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing office session state event body"))?;
            EventBody::GrandArchitectOfficeSessionStateChanged {
                session_id: GrandArchitectOfficeSessionId::try_from(session)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                state: office_session_state_from_i64(state)?,
            }
        }
        EventKind::OfficeTurnOpened => decode_office_turn_opened_event(connection, event_id_typed)?,
        EventKind::OfficeTurnSettled => {
            decode_office_turn_settled_event(connection, event_id_typed)?
        }
        EventKind::BudgetReserved => {
            let (reservation, cycle, amount) = connection.query_row("SELECT budget_reservation_id, operating_cycle_id, amount_micros FROM event_budget_reserved WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing budget reserve event body"))?;
            EventBody::BudgetReserved {
                reservation_id: BudgetReservationId::try_from(reservation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                amount: UsdMicros::try_from(amount).map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::BudgetReconciled => {
            let (reservation, amount) = connection.query_row("SELECT budget_reservation_id, observed_micros FROM event_budget_reconciled WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing budget reconciliation event body"))?;
            EventBody::BudgetReconciled {
                reservation_id: BudgetReservationId::try_from(reservation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                observed: UsdMicros::try_from(amount)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::BudgetAdmissionFrozen => {
            let (reservation, cycle, cancellation_request, postmortem, reason_kind, observed, reserved, unknown, unavailable) = connection.query_row("SELECT budget_reservation_id, operating_cycle_id, cancellation_request_id, postmortem_id, freeze_reason_kind, observed_micros, reserved_micros, unknown_reason, unavailable_reason FROM event_budget_admission_frozen WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?, row.get::<_, Option<i64>>(5)?, row.get::<_, Option<i64>>(6)?, row.get::<_, Option<i64>>(7)?, row.get::<_, Option<i64>>(8)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing budget frozen event body"))?;
            EventBody::BudgetAdmissionFrozen {
                reservation_id: BudgetReservationId::try_from(reservation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cancellation_request_id: CancellationRequestId::try_from(cancellation_request)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                postmortem_id: CostPostmortemId::try_from(postmortem)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reason: budget_freeze_reason_from_sql(
                    reason_kind,
                    observed,
                    reserved,
                    unknown,
                    unavailable,
                )?,
            }
        }
        EventKind::CancellationRequested => {
            let (request, cycle, mode, generation) = connection.query_row("SELECT cancellation_request_id, operating_cycle_id, cancellation_mode, admission_generation FROM event_cancellation_requested WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing cancellation request event body"))?;
            EventBody::CancellationRequested {
                cancellation_request_id: CancellationRequestId::try_from(request)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                mode: cancellation_mode_from_i64(mode)?,
                generation: AdmissionGeneration::try_from(generation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::CancellationReconciled => {
            let (request, cycle) = connection.query_row("SELECT cancellation_request_id, operating_cycle_id FROM event_cancellation_reconciled WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing cancellation reconciliation event body"))?;
            EventBody::CancellationReconciled {
                cancellation_request_id: CancellationRequestId::try_from(request)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::CostPostmortemClosed => {
            let (postmortem, reservation, cycle, resolution, charged) = connection.query_row("SELECT postmortem_id, budget_reservation_id, operating_cycle_id, resolution_kind, charged_micros FROM event_cost_postmortem_closed WHERE event_id = ?1", [event_id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?, row.get::<_, i64>(4)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing cost postmortem closed event body"))?;
            EventBody::CostPostmortemClosed {
                postmortem_id: CostPostmortemId::try_from(postmortem)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reservation_id: BudgetReservationId::try_from(reservation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                resolution: cost_postmortem_resolution_from_i64(resolution)?,
                charged: UsdMicros::try_from(charged)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ProjectCreated => EventBody::ProjectCreated {
            project_id: query_event_id(
                connection,
                "event_project_created",
                "project_id",
                event_id_typed,
            )?,
        },
        EventKind::ProjectChartered => EventBody::ProjectChartered {
            project_id: query_event_id(
                connection,
                "event_project_chartered",
                "project_id",
                event_id_typed,
            )?,
        },
        EventKind::ProjectStateChanged => {
            let (id, state) =
                query_event_pair(connection, "event_project_state_changed", event_id)?;
            EventBody::ProjectStateChanged {
                project_id: ProjectId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
                state: project_state_from_i64(state)?,
            }
        }
        EventKind::ProjectMilestoneCompleted => EventBody::ProjectMilestoneCompleted {
            project_milestone_id: query_event_id(
                connection,
                "event_project_milestone_completed",
                "project_milestone_id",
                event_id_typed,
            )?,
        },
        EventKind::TicketCreated => {
            let (id, project) = query_event_pair(connection, "event_ticket_created", event_id)?;
            EventBody::TicketCreated {
                ticket_id: TicketId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::TicketStateChanged => {
            let (id, state) = query_event_pair(connection, "event_ticket_state_changed", event_id)?;
            EventBody::TicketStateChanged {
                ticket_id: TicketId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
                state: ticket_state_from_i64(state)?,
            }
        }
        EventKind::GraphObjectRevisionAdded => {
            let (object, revision) =
                query_event_pair(connection, "event_graph_object_revision_added", event_id)?;
            EventBody::GraphObjectRevisionAdded {
                graph_object_id: GraphObjectId::try_from(object)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                graph_revision_id: GraphRevisionId::try_from(revision)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::GraphRevisionCommitted => EventBody::GraphRevisionCommitted {
            graph_revision_id: query_event_id(
                connection,
                "event_graph_revision_committed",
                "graph_revision_id",
                event_id_typed,
            )?,
        },
        EventKind::GraphEdgeAdded => EventBody::GraphEdgeAdded {
            graph_edge_id: query_event_id(
                connection,
                "event_graph_edge_added",
                "graph_edge_id",
                event_id_typed,
            )?,
        },
        EventKind::EpisodeCreated => {
            let (episode, project) =
                query_event_pair(connection, "event_episode_created", event_id)?;
            EventBody::EpisodeCreated {
                causal_episode_id: CausalEpisodeId::try_from(episode)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::EpisodeStateChanged => {
            let (episode, state) =
                query_event_pair(connection, "event_episode_state_changed", event_id)?;
            EventBody::EpisodeStateChanged {
                causal_episode_id: CausalEpisodeId::try_from(episode)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                state: episode_state_from_i64(state)?,
            }
        }
        EventKind::AdversarialReviewRequested => EventBody::AdversarialReviewRequested {
            adversarial_review_id: query_event_id(
                connection,
                "event_adversarial_review_requested",
                "adversarial_review_id",
                event_id_typed,
            )?,
        },
        EventKind::AdversarialReviewerAssigned => {
            let (review, reviewer, actor, attempt): (i64, i64, i64, i64) = connection.query_row(
                "SELECT adversarial_review_id, reviewer_principal_id, reviewer_actor_instance_id, reviewer_actor_attempt_id FROM event_adversarial_reviewer_assigned WHERE event_id = ?1",
                [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing reviewer assignment event body"))?;
            EventBody::AdversarialReviewerAssigned {
                adversarial_review_id: AdversarialReviewId::try_from(review)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_principal_id: PrincipalId::try_from(reviewer)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_actor_instance_id: ActorInstanceId::try_from(actor)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ReviewChallengeSubmitted => {
            let (challenge, author) =
                query_event_pair(connection, "event_review_challenge_submitted", event_id)?;
            EventBody::ReviewChallengeSubmitted {
                review_challenge_id: ReviewChallengeId::try_from(challenge)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                author_principal_id: PrincipalId::try_from(author)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ReviewChallengeResponded => EventBody::ReviewChallengeResponded {
            review_challenge_id: query_event_id(
                connection,
                "event_review_challenge_responded",
                "review_challenge_id",
                event_id_typed,
            )?,
        },
        EventKind::ReviewChallengeDispositioned => {
            let (id, disposition) =
                query_event_pair(connection, "event_review_challenge_dispositioned", event_id)?;
            EventBody::ReviewChallengeDispositioned {
                review_challenge_id: ReviewChallengeId::try_from(id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                disposition: review_disposition_kind_from_i64(disposition)?,
            }
        }
        EventKind::AdversarialReviewResolved => {
            let (id, state) =
                query_event_pair(connection, "event_adversarial_review_resolved", event_id)?;
            EventBody::AdversarialReviewResolved {
                adversarial_review_id: AdversarialReviewId::try_from(id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                state: adversarial_review_state_from_i64(state)?,
            }
        }
        EventKind::PostmortemTriggered => EventBody::PostmortemTriggered {
            postmortem_id: query_event_id(
                connection,
                "event_postmortem_triggered",
                "postmortem_id",
                event_id_typed,
            )?,
        },
        EventKind::PostmortemCausalClaimRecorded => EventBody::PostmortemCausalClaimRecorded {
            postmortem_causal_claim_id: query_event_id(
                connection,
                "event_postmortem_causal_claim_recorded",
                "postmortem_causal_claim_id",
                event_id_typed,
            )?,
        },
        EventKind::PostmortemActionProposed => EventBody::PostmortemActionProposed {
            postmortem_action_proposal_id: query_event_id(
                connection,
                "event_postmortem_action_proposed",
                "postmortem_action_proposal_id",
                event_id_typed,
            )?,
        },
        EventKind::PostmortemClosed => EventBody::PostmortemClosed {
            postmortem_id: query_event_id(
                connection,
                "event_postmortem_closed",
                "postmortem_id",
                event_id_typed,
            )?,
        },
        EventKind::ActorConfigurationRegistered => {
            let (configuration, revision) =
                query_event_pair(connection, "event_actor_configuration_registered", event_id)?;
            EventBody::ActorConfigurationRegistered {
                actor_configuration_id: ActorConfigurationId::try_from(configuration)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_configuration_revision_id: ActorConfigurationRevisionId::try_from(revision)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ContextPackRegistered => EventBody::ContextPackRegistered {
            context_pack_id: query_event_id(
                connection,
                "event_context_pack_registered",
                "context_pack_id",
                event_id_typed,
            )?,
        },
        EventKind::ActorInstanceAdmitted => {
            let (actor, principal) =
                query_event_pair(connection, "event_actor_instance_admitted", event_id)?;
            EventBody::ActorInstanceAdmitted {
                actor_instance_id: ActorInstanceId::try_from(actor)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                principal_id: PrincipalId::try_from(principal)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::TicketAdmitted => EventBody::TicketAdmitted {
            ticket_id: query_event_id(
                connection,
                "event_ticket_admitted",
                "ticket_id",
                event_id_typed,
            )?,
        },
        EventKind::WorkItemRegistered => {
            let (work, ticket, review): (i64, i64, Option<i64>) = connection.query_row(
                "SELECT work_item_id, ticket_id, adversarial_review_id FROM event_work_item_registered WHERE event_id = ?1",
                [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing work item event body"))?;
            EventBody::WorkItemRegistered {
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                adversarial_review_id: review
                    .map(AdversarialReviewId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::WorkItemClaimed => {
            let (work, lease, actor): (i64, i64, i64) = connection.query_row("SELECT work_item_id, work_lease_id, actor_instance_id FROM event_work_item_claimed WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing work item claim event body"))?;
            EventBody::WorkItemClaimed {
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_lease_id: WorkLeaseId::try_from(lease)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_instance_id: ActorInstanceId::try_from(actor)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ActorAttemptStarted => {
            let (attempt, work, reservation): (i64, i64, i64) = connection.query_row("SELECT actor_attempt_id, work_item_id, budget_reservation_id FROM event_actor_attempt_started WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing actor attempt started event body"))?;
            EventBody::ActorAttemptStarted {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                budget_reservation_id: BudgetReservationId::try_from(reservation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ActorAttemptTerminalAttested => {
            let (attempt, terminal) = query_event_pair(
                connection,
                "event_actor_attempt_terminal_attested",
                event_id,
            )?;
            EventBody::ActorAttemptTerminalAttested {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                terminal_kind: actor_attempt_terminal_kind_from_i64(terminal)?,
            }
        }
        EventKind::TicketAttemptValidated => {
            let (attempt, ticket) =
                query_event_pair(connection, "event_ticket_attempt_validated", event_id)?;
            EventBody::TicketAttemptValidated {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ActorAttemptRetryPrepared => {
            let (attempt, work, ticket): (i64, i64, i64) = connection.query_row("SELECT actor_attempt_id, work_item_id, ticket_id FROM event_actor_attempt_retry_prepared WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing retry event body"))?;
            EventBody::ActorAttemptRetryPrepared {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::TicketCompleted => {
            let (ticket, attempt) =
                query_event_pair(connection, "event_ticket_completed", event_id)?;
            EventBody::TicketCompleted {
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::WorkLeaseExpired => {
            let (lease, work) = query_event_pair(connection, "event_work_lease_expired", event_id)?;
            EventBody::WorkLeaseExpired {
                work_lease_id: WorkLeaseId::try_from(lease)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ActorAttemptCancellationRequested => {
            let (attempt, reason) = query_event_pair(
                connection,
                "event_actor_attempt_cancellation_requested",
                event_id,
            )?;
            EventBody::ActorAttemptCancellationRequested {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reason: actor_attempt_cancellation_reason_from_i64(reason)?,
            }
        }
        EventKind::OutcomeObligationRegistered => {
            let (obligation, project) =
                query_event_pair(connection, "event_outcome_obligation_registered", event_id)?;
            EventBody::OutcomeObligationRegistered {
                outcome_obligation_id: OutcomeObligationId::try_from(obligation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::OutcomeObligationResolved => {
            let (obligation, state) =
                query_event_pair(connection, "event_outcome_obligation_resolved", event_id)?;
            EventBody::OutcomeObligationResolved {
                outcome_obligation_id: OutcomeObligationId::try_from(obligation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                state: outcome_obligation_state_from_i64(state)?,
            }
        }
        EventKind::ContentSealReceiptRecorded => {
            let (receipt, digest): (i64, Vec<u8>) = connection.query_row("SELECT content_seal_receipt_id, digest FROM event_content_seal_receipt_recorded WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?, row.get(1)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing content seal receipt event body"))?;
            EventBody::ContentSealReceiptRecorded {
                content_seal_receipt_id: ContentSealReceiptId::try_from(receipt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                digest: digest_from_stored_bytes(&digest)?,
            }
        }
        EventKind::ContentObjectRegistered => {
            let (object, receipt) =
                query_event_pair(connection, "event_content_object_registered", event_id)?;
            EventBody::ContentObjectRegistered {
                content_object_id: ContentObjectId::try_from(object)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                content_seal_receipt_id: ContentSealReceiptId::try_from(receipt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::ForensicManifestRegistered => {
            let (manifest, experiment, output): (i64, i64, i64) = connection.query_row("SELECT forensic_manifest_id, producing_deterministic_experiment_id, evaluator_output_content_object_id FROM event_forensic_manifest_registered WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing forensic manifest event body"))?;
            EventBody::ForensicManifestRegistered {
                forensic_manifest_id: ForensicManifestId::try_from(manifest)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                producing_deterministic_experiment_id: DeterministicExperimentId::try_from(
                    experiment,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_output_content_object_id: ContentObjectId::try_from(output)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::DeterministicExperimentRegistered => {
            let (experiment, evaluator, input): (i64,i64,i64) = connection.query_row("SELECT deterministic_experiment_id, evaluator_revision_id, input_manifest_id FROM event_deterministic_experiment_registered WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing deterministic experiment event body"))?;
            EventBody::DeterministicExperimentRegistered {
                deterministic_experiment_id: DeterministicExperimentId::try_from(experiment)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_revision_id: EvaluatorRevisionId::try_from(evaluator)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                input_manifest_id: InputManifestId::try_from(input)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::DeterministicEvaluationReceiptRecorded => {
            let (receipt, experiment) = query_event_pair(
                connection,
                "event_deterministic_evaluation_receipt_recorded",
                event_id,
            )?;
            EventBody::DeterministicEvaluationReceiptRecorded {
                deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId::try_from(
                    receipt,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_experiment_id: DeterministicExperimentId::try_from(experiment)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        EventKind::DeterministicEvidenceAdmitted => {
            let (admission, receipt, role, applicability): (i64,i64,i64,i64) = connection.query_row("SELECT evidence_admission_id, deterministic_evaluation_receipt_id, semantic_role, applicability FROM event_deterministic_evidence_admitted WHERE event_id = ?1", [event_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing deterministic evidence event body"))?;
            EventBody::DeterministicEvidenceAdmitted {
                evidence_admission_id: EvidenceAdmissionId::try_from(admission)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId::try_from(
                    receipt,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                semantic_role: evidence_semantic_role_from_i64(role)?,
                applicability: evidence_applicability_from_i64(applicability)?,
            }
        }
        EventKind::DeterministicExperimentClosed => EventBody::DeterministicExperimentClosed {
            deterministic_experiment_id: query_event_id(
                connection,
                "event_deterministic_experiment_closed",
                "deterministic_experiment_id",
                event_id_typed,
            )?,
        },
    };
    let stored_fingerprint: Vec<u8> = connection.query_row(
        "SELECT event_fingerprint FROM events WHERE event_id = ?1",
        [event_id],
        |row| row.get(0),
    )?;
    if stored_fingerprint.as_slice()
        != event_fingerprint(event_id_typed, command_id, &body).as_bytes()
    {
        return Err(StoreError::LedgerCorruption(
            "event body fingerprint mismatch",
        ));
    }
    Ok(body)
}

/// Rebuilds every persisted command request from its named body and proves its
/// original request commitment still matches. This includes rejections: a
/// rejected command is durable operational history, not an untyped error
/// record that may escape integrity checks.
fn verify_command_bodies(connection: &Connection) -> Result<(), StoreError> {
    let mut statement = connection.prepare(
        "SELECT command_row_id, command_id, principal_id, capability_grant_id,
                capability_kind, expected_generation, command_kind,
                request_fingerprint, command_status, accepted_event_id
         FROM commands ORDER BY command_row_id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, Vec<u8>>(7)?,
            row.get::<_, i64>(8)?,
            row.get::<_, Option<i64>>(9)?,
        ))
    })?;
    for row in rows {
        let (
            command_row_id,
            command_id,
            principal_id,
            capability_grant_id,
            capability_kind,
            expected_generation,
            command_kind,
            stored_fingerprint,
            status,
            accepted_event_id,
        ) = row?;
        let kind = command_kind_from_i64(command_kind)?;
        let expected_table = command_body_table(kind)?;
        verify_exact_named_body(
            connection,
            command_row_id,
            expected_table,
            &COMMAND_BODY_TABLES,
        )?;
        let request = CommandRequest {
            command_id: CommandId::parse(command_id).map_err(|_| StoreError::InvalidStoredValue)?,
            principal_id: PrincipalId::try_from(principal_id)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            capability_grant_id: crate::CapabilityGrantId::try_from(capability_grant_id)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            capability: capability_from_i64(capability_kind)?,
            expected_generation: match expected_generation {
                Some(generation) => ExpectedGeneration::Exact(
                    AdmissionGeneration::try_from(generation)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                ),
                None => ExpectedGeneration::NotApplicable,
            },
            body: decode_command_body(connection, command_row_id, kind)?,
        };
        if request.body.kind() != kind {
            return Err(StoreError::LedgerCorruption(
                "command body does not match command kind",
            ));
        }
        if stored_fingerprint.as_slice() != request_fingerprint(&request).as_bytes() {
            return Err(StoreError::LedgerCorruption(
                "command request fingerprint mismatch",
            ));
        }
        let event_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM events WHERE command_row_id = ?1",
            [command_row_id],
            |row| row.get(0),
        )?;
        match (status, accepted_event_id, event_count) {
            (1, Some(event_id), 1) => {
                let linked: i64 = connection.query_row(
                    "SELECT COUNT(*) FROM events WHERE event_id = ?1 AND command_row_id = ?2",
                    params![event_id, command_row_id],
                    |row| row.get(0),
                )?;
                if linked != 1 {
                    return Err(StoreError::LedgerCorruption(
                        "accepted command does not name its event",
                    ));
                }
            }
            (2, None, 0) => {}
            _ => {
                return Err(StoreError::LedgerCorruption(
                    "command receipt and event relation disagree",
                ));
            }
        }
    }
    Ok(())
}

fn replay_command_requests(
    connection: &Connection,
) -> Result<Vec<(CommandRequest, CommandDisposition)>, StoreError> {
    let mut statement = connection.prepare(
        "SELECT command_row_id, command_id, principal_id, capability_grant_id,
                capability_kind, expected_generation, command_kind, command_status,
                accepted_event_id, rejection_code
         FROM commands ORDER BY command_row_id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, Option<i64>>(8)?,
            row.get::<_, Option<i64>>(9)?,
        ))
    })?;
    let mut commands = Vec::new();
    for row in rows {
        let (
            command_row_id,
            command_id,
            principal_id,
            capability_grant_id,
            capability_kind,
            expected_generation,
            command_kind,
            status,
            accepted_event_id,
            rejection_code,
        ) = row?;
        let kind = command_kind_from_i64(command_kind)?;
        let request = CommandRequest {
            command_id: CommandId::parse(command_id).map_err(|_| StoreError::InvalidStoredValue)?,
            principal_id: PrincipalId::try_from(principal_id)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            capability_grant_id: crate::CapabilityGrantId::try_from(capability_grant_id)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            capability: capability_from_i64(capability_kind)?,
            expected_generation: match expected_generation {
                Some(generation) => ExpectedGeneration::Exact(
                    AdmissionGeneration::try_from(generation)
                        .map_err(|_| StoreError::InvalidStoredValue)?,
                ),
                None => ExpectedGeneration::NotApplicable,
            },
            body: decode_command_body(connection, command_row_id, kind)?,
        };
        let disposition = match status {
            1 => CommandDisposition::Accepted(
                EventId::try_from(accepted_event_id.ok_or(StoreError::LedgerCorruption(
                    "accepted command has no event",
                ))?)
                .map_err(|_| StoreError::InvalidStoredValue)?,
            ),
            2 => CommandDisposition::Rejected(rejection_from_i64(rejection_code.ok_or(
                StoreError::LedgerCorruption("rejected command has no rejection code"),
            )?)?),
            _ => return Err(StoreError::LedgerCorruption("unknown command status")),
        };
        commands.push((request, disposition));
    }
    Ok(commands)
}

const MATERIALIZED_TABLES: [&str; 60] = [
    "principals",
    "societies",
    "office_contracts",
    "universe_seeds",
    "office_occupancies",
    "capability_grants",
    "society_bootstraps",
    "operating_cycles",
    "operating_cycle_admissions",
    "operating_cycle_reconciliations",
    "grand_architect_office_sessions",
    "office_turns",
    "budget_envelopes",
    "budget_envelope_constraints",
    "budget_reservations",
    "budget_reservation_charges",
    "cancellation_requests",
    "cost_postmortems",
    "cost_postmortem_resolutions",
    "projects",
    "project_objectives",
    "project_milestones",
    "project_stop_conditions",
    "tickets",
    "ticket_acceptance_conditions",
    "ticket_prerequisites",
    "objects",
    "object_revisions",
    "observation_revisions",
    "hypothesis_revisions",
    "edges",
    "episodes",
    "adversarial_reviews",
    "review_challenges",
    "review_challenge_responses",
    "review_dispositions",
    "postmortems",
    "postmortem_causal_claims",
    "postmortem_action_proposals",
    "coordination_command_provenance",
    "execution_profiles",
    "actor_configurations",
    "actor_configuration_revisions",
    "context_packs",
    "actor_instances",
    "work_items",
    "leases",
    "attempts",
    "attempt_budget_reservations",
    "actor_attempt_terminal_facts",
    "outcome_obligations",
    "content_seal_receipts",
    "content_objects",
    "forensic_manifests",
    "forensic_manifest_objects",
    "evaluator_revisions",
    "input_manifests",
    "deterministic_experiments",
    "deterministic_evaluation_receipts",
    "evidence_admissions",
];

fn materialized_state_digest(connection: &Connection) -> Result<Sha256Digest, StoreError> {
    let mut bytes = Vec::with_capacity(4_096);
    for table in MATERIALIZED_TABLES {
        put_bytes(&mut bytes, table.as_bytes());
        let mut statement = connection.prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))?;
        let column_count = statement.column_count();
        put_i64(&mut bytes, column_count as i64);
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            put_i64(&mut bytes, 1);
            for index in 0..column_count {
                match row.get_ref(index)? {
                    ValueRef::Null => put_i64(&mut bytes, 0),
                    ValueRef::Integer(value) => {
                        put_i64(&mut bytes, 1);
                        put_i64(&mut bytes, value);
                    }
                    ValueRef::Real(value) => {
                        put_i64(&mut bytes, 2);
                        bytes.extend_from_slice(&value.to_bits().to_be_bytes());
                    }
                    ValueRef::Text(value) => {
                        put_i64(&mut bytes, 3);
                        put_bytes(&mut bytes, value);
                    }
                    ValueRef::Blob(value) => {
                        put_i64(&mut bytes, 4);
                        put_bytes(&mut bytes, value);
                    }
                }
            }
        }
        put_i64(&mut bytes, 0);
    }
    Ok(Sha256Digest::of_bytes(&bytes))
}

fn decode_command_body(
    connection: &Connection,
    command_row_id: i64,
    kind: CommandKind,
) -> Result<CommandBody, StoreError> {
    let body = match kind {
        CommandKind::CreateSocietyIdentity => {
            let name: String = query_command_value(
                connection,
                "command_create_society_identity",
                "name",
                command_row_id,
            )?;
            CommandBody::CreateSocietyIdentity {
                name: SocietyName::parse(name).map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::InstallGrandArchitectOffice => CommandBody::InstallGrandArchitectOffice,
        CommandKind::InstallFoundingUniverseSeed => {
            let digest: Vec<u8> = query_command_value(
                connection,
                "command_install_founding_universe_seed",
                "rendering_digest",
                command_row_id,
            )?;
            CommandBody::InstallFoundingUniverseSeed {
                rendering_digest: digest_from_stored_bytes(&digest)?,
            }
        }
        CommandKind::AppointInitialGrandArchitect => {
            let display_name: String = query_command_value(
                connection,
                "command_appoint_initial_grand_architect",
                "actor_display_name",
                command_row_id,
            )?;
            CommandBody::AppointInitialGrandArchitect {
                actor_display_name: crate::PrincipalDisplayName::parse(display_name)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::SetR0HardCeiling => CommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::try_from(query_command_value::<i64>(
                connection,
                "command_set_r0_hard_ceiling",
                "ceiling_micros",
                command_row_id,
            )?)
            .map_err(|_| StoreError::InvalidStoredValue)?,
        },
        CommandKind::BootstrapSociety => CommandBody::BootstrapSociety,
        CommandKind::ProposeOperatingCycle => CommandBody::ProposeOperatingCycle {
            treatment: operating_cycle_treatment_from_i64(query_command_value::<i64>(
                connection,
                "command_propose_operating_cycle",
                "treatment",
                command_row_id,
            )?)?,
        },
        CommandKind::AdmitOperatingCycle => CommandBody::AdmitOperatingCycle {
            cycle_id: query_command_id(
                connection,
                "command_admit_operating_cycle",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::StartGrandArchitectOfficeSession => {
            CommandBody::StartGrandArchitectOfficeSession {
                cycle_id: query_command_id(
                    connection,
                    "command_start_grand_architect_office_session",
                    "operating_cycle_id",
                    command_row_id,
                )?,
            }
        }
        CommandKind::RecordOfficeSessionReady => CommandBody::RecordOfficeSessionReady {
            session_id: query_command_id(
                connection,
                "command_record_office_session_ready",
                "grand_architect_office_session_id",
                command_row_id,
            )?,
        },
        CommandKind::OpenOfficeTurn => {
            let (session_id, purpose) = connection
                .query_row(
                    "SELECT grand_architect_office_session_id, purpose
                     FROM command_open_office_turn WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing office turn command body",
                ))?;
            CommandBody::OpenOfficeTurn {
                session_id: GrandArchitectOfficeSessionId::try_from(session_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                purpose: office_turn_purpose_from_i64(purpose)?,
            }
        }
        CommandKind::SettleOfficeTurn => CommandBody::SettleOfficeTurn {
            turn_id: query_command_id(
                connection,
                "command_settle_office_turn",
                "office_turn_id",
                command_row_id,
            )?,
        },
        CommandKind::QuiesceOperatingCycle => CommandBody::QuiesceOperatingCycle {
            cycle_id: query_command_id(
                connection,
                "command_quiesce_operating_cycle",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::RecordCycleDrained => CommandBody::RecordCycleDrained {
            cycle_id: query_command_id(
                connection,
                "command_record_cycle_drained",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::ResumeOperatingCycle => CommandBody::ResumeOperatingCycle {
            cycle_id: query_command_id(
                connection,
                "command_resume_operating_cycle",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::ReconcileOperatingCycle => CommandBody::ReconcileOperatingCycle {
            cycle_id: query_command_id(
                connection,
                "command_reconcile_operating_cycle",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::CloseOperatingCycle => CommandBody::CloseOperatingCycle {
            cycle_id: query_command_id(
                connection,
                "command_close_operating_cycle",
                "operating_cycle_id",
                command_row_id,
            )?,
        },
        CommandKind::ReserveBudget => {
            let (cycle_id, amount) = connection
                .query_row(
                    "SELECT operating_cycle_id, amount_micros FROM command_reserve_budget
                     WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing budget reserve command body",
                ))?;
            CommandBody::ReserveBudget {
                cycle_id: OperatingCycleId::try_from(cycle_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                amount: UsdMicros::try_from(amount).map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ReconcileBudget => {
            let (reservation_id, observation_kind, known, unknown, unavailable) = connection
                .query_row(
                    "SELECT budget_reservation_id, observation_kind, known_micros,
                            unknown_reason, unavailable_reason
                     FROM command_reconcile_budget WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, Option<i64>>(2)?,
                            row.get::<_, Option<i64>>(3)?,
                            row.get::<_, Option<i64>>(4)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing budget reconciliation command body",
                ))?;
            CommandBody::ReconcileBudget {
                reservation_id: BudgetReservationId::try_from(reservation_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                observation: cost_observation_from_sql(
                    observation_kind,
                    known,
                    unknown,
                    unavailable,
                )?,
            }
        }
        CommandKind::RequestCancellation => {
            let (cycle_id, mode) = connection
                .query_row(
                    "SELECT operating_cycle_id, cancellation_mode
                     FROM command_request_cancellation WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing cancellation request command body",
                ))?;
            CommandBody::RequestCancellation {
                cycle_id: OperatingCycleId::try_from(cycle_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                mode: cancellation_mode_from_i64(mode)?,
            }
        }
        CommandKind::ReconcileCancellation => CommandBody::ReconcileCancellation {
            cancellation_request_id: query_command_id(
                connection,
                "command_reconcile_cancellation",
                "cancellation_request_id",
                command_row_id,
            )?,
        },
        CommandKind::RecordOfficeSessionTerminal => {
            let (session_id, terminal_state) = connection
                .query_row(
                    "SELECT grand_architect_office_session_id, terminal_state
                     FROM command_record_office_session_terminal WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing office session terminal command body",
                ))?;
            CommandBody::RecordOfficeSessionTerminal {
                session_id: GrandArchitectOfficeSessionId::try_from(session_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                terminal_state: office_session_terminal_state_from_i64(terminal_state)?,
            }
        }
        CommandKind::CloseCostPostmortem => {
            let (postmortem_id, resolution) = connection
                .query_row(
                    "SELECT postmortem_id, resolution_kind
                     FROM command_close_cost_postmortem WHERE command_row_id = ?1",
                    [command_row_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?
                .ok_or(StoreError::LedgerCorruption(
                    "missing cost postmortem close command body",
                ))?;
            CommandBody::CloseCostPostmortem {
                postmortem_id: CostPostmortemId::try_from(postmortem_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                resolution: cost_postmortem_resolution_from_i64(resolution)?,
            }
        }
        CommandKind::CreateProject => {
            let (cycle, name) =
                query_command_i64_text(connection, "command_create_project", command_row_id)?;
            CommandBody::CreateProject {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_name: crate::ProjectName::parse(name)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::CharterProject => {
            let row = query_command_six(connection, "command_charter_project", command_row_id)?;
            CommandBody::CharterProject {
                operating_cycle_id: OperatingCycleId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                objective: crate::ProjectObjectiveText::parse(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                initial_milestone: crate::ProjectMilestoneName::parse(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                stop_condition: crate::ProjectStopConditionText::parse(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::TransitionProject => {
            let (cycle, id, state) =
                query_command_three(connection, "command_transition_project", command_row_id)?;
            CommandBody::TransitionProject {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
                target: project_state_from_i64(state)?,
            }
        }
        CommandKind::CompleteProjectMilestone => {
            let (cycle, id) = query_command_pair(
                connection,
                "command_complete_project_milestone",
                command_row_id,
            )?;
            CommandBody::CompleteProjectMilestone {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_milestone_id: ProjectMilestoneId::try_from(id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ReopenProject => {
            let (cycle, id) =
                query_command_pair(connection, "command_reopen_project", command_row_id)?;
            CommandBody::ReopenProject {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::CreateTicket => {
            let (cycle, project, title, condition, prerequisite) =
                query_create_ticket(connection, command_row_id)?;
            CommandBody::CreateTicket {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_title: crate::TicketTitle::parse(title)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                acceptance_condition: crate::TicketAcceptanceConditionText::parse(condition)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                prerequisite_ticket_id: prerequisite
                    .map(TicketId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::TransitionTicket => {
            let (cycle, id, state) =
                query_command_three(connection, "command_transition_ticket", command_row_id)?;
            CommandBody::TransitionTicket {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(id).map_err(|_| StoreError::InvalidStoredValue)?,
                target: ticket_state_from_i64(state)?,
            }
        }
        CommandKind::AddGraphObjectRevision => {
            let (cycle, project, episode, object) =
                query_graph_revision_command(connection, command_row_id)?;
            CommandBody::AddGraphObjectRevision {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                causal_episode_id: episode
                    .map(CausalEpisodeId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                graph_object_id: object
                    .map(GraphObjectId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                body: query_graph_revision_command_body(connection, command_row_id)?,
            }
        }
        CommandKind::CommitGraphRevision => {
            let (cycle, id) =
                query_command_pair(connection, "command_commit_graph_revision", command_row_id)?;
            CommandBody::CommitGraphRevision {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                graph_revision_id: GraphRevisionId::try_from(id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AddGraphEdge => {
            let (cycle, project, from, to, kind) = query_edge_command(connection, command_row_id)?;
            CommandBody::AddGraphEdge {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                from_graph_revision_id: GraphRevisionId::try_from(from)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                to_graph_revision_id: GraphRevisionId::try_from(to)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                edge_kind: graph_edge_kind_from_i64(kind)?,
            }
        }
        CommandKind::CreateEpisode => {
            let (cycle, project) =
                query_command_pair(connection, "command_create_episode", command_row_id)?;
            CommandBody::CreateEpisode {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::TransitionEpisode => {
            let (cycle, episode, state) =
                query_command_three(connection, "command_transition_episode", command_row_id)?;
            CommandBody::TransitionEpisode {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                causal_episode_id: CausalEpisodeId::try_from(episode)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                target: episode_state_from_i64(state)?,
            }
        }
        CommandKind::ReopenEpisode => {
            let (cycle, episode) =
                query_command_pair(connection, "command_reopen_episode", command_row_id)?;
            CommandBody::ReopenEpisode {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                causal_episode_id: CausalEpisodeId::try_from(episode)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RequestAdversarialReview => {
            let (cycle, project, revision) = query_command_three(
                connection,
                "command_request_adversarial_review",
                command_row_id,
            )?;
            CommandBody::RequestAdversarialReview {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                target_graph_revision_id: GraphRevisionId::try_from(revision)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AssignAdversarialReviewer => {
            let (cycle, review, reviewer, actor, attempt): (i64, i64, i64, i64, i64) = connection.query_row(
                "SELECT operating_cycle_id, adversarial_review_id, reviewer_principal_id, reviewer_actor_instance_id, reviewer_actor_attempt_id FROM command_assign_adversarial_reviewer WHERE command_row_id = ?1",
                [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            ).optional()?.ok_or(StoreError::LedgerCorruption("missing reviewer assignment command body"))?;
            CommandBody::AssignAdversarialReviewer {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                adversarial_review_id: AdversarialReviewId::try_from(review)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_principal_id: PrincipalId::try_from(reviewer)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_actor_instance_id: ActorInstanceId::try_from(actor)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reviewer_actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::SubmitReviewChallenge => {
            let (cycle, review, revision, author, severity, hypothesis) =
                query_review_submit(connection, command_row_id)?;
            CommandBody::SubmitReviewChallenge {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                adversarial_review_id: AdversarialReviewId::try_from(review)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                target_graph_revision_id: GraphRevisionId::try_from(revision)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                author_principal_id: PrincipalId::try_from(author)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                severity: review_challenge_severity_from_i64(severity)?,
                failure_hypothesis: crate::ReviewFailureHypothesis::parse(hypothesis)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RespondToReviewChallenge => {
            let (cycle, challenge, response): (i64, i64, String) = connection.query_row("SELECT operating_cycle_id, review_challenge_id, response_text FROM command_respond_to_review_challenge WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing review response command body"))?;
            CommandBody::RespondToReviewChallenge {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                review_challenge_id: ReviewChallengeId::try_from(challenge)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                response: crate::ReviewResponseText::parse(response)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::DispositionReviewChallenge => {
            let (cycle, challenge, disposition) = query_command_three(
                connection,
                "command_disposition_review_challenge",
                command_row_id,
            )?;
            CommandBody::DispositionReviewChallenge {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                review_challenge_id: ReviewChallengeId::try_from(challenge)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                disposition: review_disposition_kind_from_i64(disposition)?,
            }
        }
        CommandKind::ResolveAdversarialReview => {
            let (cycle, review, resolution) = query_command_three(
                connection,
                "command_resolve_adversarial_review",
                command_row_id,
            )?;
            CommandBody::ResolveAdversarialReview {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                adversarial_review_id: AdversarialReviewId::try_from(review)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                resolution: review_resolution_kind_from_i64(resolution)?,
            }
        }
        CommandKind::TriggerPostmortem => {
            let (cycle, project, episode) = query_postmortem_trigger(connection, command_row_id)?;
            CommandBody::TriggerPostmortem {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                causal_episode_id: episode
                    .map(CausalEpisodeId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordPostmortemCausalClaim => {
            let (cycle, postmortem, kind, text) = query_postmortem_text_command(
                connection,
                "command_record_postmortem_causal_claim",
                command_row_id,
            )?;
            CommandBody::RecordPostmortemCausalClaim {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                postmortem_id: PostmortemId::try_from(postmortem)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                claim_kind: postmortem_causal_claim_kind_from_i64(kind)?,
                claim: crate::PostmortemCausalClaimText::parse(text)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ProposePostmortemAction => {
            let (cycle, postmortem, kind, text) = query_postmortem_text_command(
                connection,
                "command_propose_postmortem_action",
                command_row_id,
            )?;
            CommandBody::ProposePostmortemAction {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                postmortem_id: PostmortemId::try_from(postmortem)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                action_kind: postmortem_action_kind_from_i64(kind)?,
                action: crate::PostmortemActionProposalText::parse(text)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ClosePostmortem => {
            let (cycle, postmortem) =
                query_command_pair(connection, "command_close_postmortem", command_row_id)?;
            CommandBody::ClosePostmortem {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                postmortem_id: PostmortemId::try_from(postmortem)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RegisterActorConfiguration => {
            let (name, model, attractor): (String, i64, i64) = connection.query_row("SELECT configuration_name, model_policy, primary_attractor FROM command_register_actor_configuration WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing actor configuration command body"))?;
            CommandBody::RegisterActorConfiguration {
                configuration_name: crate::ActorConfigurationName::parse(name)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                model_policy: actor_model_policy_from_i64(model)?,
                primary_attractor: developmental_attractor_from_i64(attractor)?,
            }
        }
        CommandKind::RegisterContextPack => {
            let (cycle, purpose, digest): (i64, i64, Vec<u8>) = connection.query_row("SELECT operating_cycle_id, purpose, rendering_digest FROM command_register_context_pack WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing context pack command body"))?;
            CommandBody::RegisterContextPack {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                purpose: context_pack_purpose_from_i64(purpose)?,
                rendering_digest: digest_from_stored_bytes(&digest)?,
            }
        }
        CommandKind::AdmitActorInstance => {
            let (cycle, revision, profile, display): (i64, i64, i64, String) = connection.query_row("SELECT operating_cycle_id, actor_configuration_revision_id, execution_profile_id, actor_display_name FROM command_admit_actor_instance WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing actor admission command body"))?;
            CommandBody::AdmitActorInstance {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_configuration_revision_id: ActorConfigurationRevisionId::try_from(revision)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                execution_profile_id: ExecutionProfileId::try_from(profile)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_display_name: crate::PrincipalDisplayName::parse(display)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AdmitTicket => {
            let (cycle, ticket) =
                query_command_pair(connection, "command_admit_ticket", command_row_id)?;
            CommandBody::AdmitTicket {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RegisterWorkItem => {
            let (cycle, ticket, actor, context, kind, review, assignment): (i64, i64, i64, i64, i64, Option<i64>, String) = connection.query_row("SELECT operating_cycle_id, ticket_id, actor_instance_id, context_pack_id, work_kind, adversarial_review_id, assignment_text FROM command_register_work_item WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing work item command body"))?;
            CommandBody::RegisterWorkItem {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(ticket)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_instance_id: ActorInstanceId::try_from(actor)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                context_pack_id: ContextPackId::try_from(context)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_kind: work_item_kind_from_i64(kind)?,
                adversarial_review_id: review
                    .map(AdversarialReviewId::try_from)
                    .transpose()
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                assignment: crate::WorkAssignmentText::parse(assignment)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ClaimWorkItem => {
            let (cycle, work) =
                query_command_pair(connection, "command_claim_work_item", command_row_id)?;
            CommandBody::ClaimWorkItem {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::StartActorAttempt => {
            let (cycle, work, amount): (i64, i64, i64) = connection.query_row("SELECT operating_cycle_id, work_item_id, reservation_micros FROM command_start_actor_attempt WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing actor attempt command body"))?;
            CommandBody::StartActorAttempt {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                work_item_id: WorkItemId::try_from(work)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reservation_amount: UsdMicros::try_from(amount)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AttestActorAttemptTerminal => {
            let (attempt, terminal) = query_command_pair(
                connection,
                "command_attest_actor_attempt_terminal",
                command_row_id,
            )?;
            CommandBody::AttestActorAttemptTerminal {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                terminal_kind: actor_attempt_terminal_kind_from_i64(terminal)?,
            }
        }
        CommandKind::ValidateTicketAttempt => {
            let (cycle, attempt) = query_command_pair(
                connection,
                "command_validate_ticket_attempt",
                command_row_id,
            )?;
            CommandBody::ValidateTicketAttempt {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RetryActorAttempt => {
            let (cycle, attempt) =
                query_command_pair(connection, "command_retry_actor_attempt", command_row_id)?;
            CommandBody::RetryActorAttempt {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::CompleteTicket => {
            let (cycle, attempt) =
                query_command_pair(connection, "command_complete_ticket", command_row_id)?;
            CommandBody::CompleteTicket {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ExpireWorkLease => CommandBody::ExpireWorkLease {
            work_lease_id: query_command_id(
                connection,
                "command_expire_work_lease",
                "work_lease_id",
                command_row_id,
            )?,
        },
        CommandKind::CancelActorAttempt => {
            let (attempt, reason) =
                query_command_pair(connection, "command_cancel_actor_attempt", command_row_id)?;
            CommandBody::CancelActorAttempt {
                actor_attempt_id: ActorAttemptId::try_from(attempt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                reason: actor_attempt_cancellation_reason_from_i64(reason)?,
            }
        }
        CommandKind::RegisterOutcomeObligation => {
            let (cycle, project, obligation): (i64, i64, String) = connection.query_row("SELECT operating_cycle_id, project_id, obligation_text FROM command_register_outcome_obligation WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing outcome obligation command body"))?;
            CommandBody::RegisterOutcomeObligation {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(project)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                obligation: crate::OutcomeObligationText::parse(obligation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::ResolveOutcomeObligation => {
            let (cycle, obligation, disposition) = query_command_three(
                connection,
                "command_resolve_outcome_obligation",
                command_row_id,
            )?;
            CommandBody::ResolveOutcomeObligation {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                outcome_obligation_id: OutcomeObligationId::try_from(obligation)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                disposition: outcome_obligation_disposition_from_i64(disposition)?,
            }
        }
        CommandKind::RecordContentSealReceipt => {
            let digest: Vec<u8> = connection.query_row("SELECT digest FROM command_record_content_seal_receipt WHERE command_row_id = ?1", [command_row_id], |row| row.get(0)).optional()?.ok_or(StoreError::LedgerCorruption("missing content seal command body"))?;
            CommandBody::RecordContentSealReceipt {
                digest: digest_from_stored_bytes(&digest)?,
            }
        }
        CommandKind::RegisterContentObject => {
            let receipt: i64 = connection.query_row("SELECT content_seal_receipt_id FROM command_register_content_object WHERE command_row_id = ?1", [command_row_id], |row| row.get(0)).optional()?.ok_or(StoreError::LedgerCorruption("missing content object command body"))?;
            CommandBody::RegisterContentObject {
                content_seal_receipt_id: ContentSealReceiptId::try_from(receipt)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RegisterForensicManifest => {
            let (cycle, experiment, policy, retention, output): (i64, i64, i64, i64, i64) = connection.query_row("SELECT operating_cycle_id, producing_deterministic_experiment_id, capture_policy, retention_access_class, evaluator_output_content_object_id FROM command_register_forensic_manifest WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing forensic manifest command body"))?;
            CommandBody::RegisterForensicManifest {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                producing_deterministic_experiment_id: DeterministicExperimentId::try_from(
                    experiment,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                capture_policy: forensic_manifest_capture_policy_from_i64(policy)?,
                retention_access_class: retention_access_class_from_i64(retention)?,
                evaluator_output_content_object_id: ContentObjectId::try_from(output)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RegisterDeterministicExperiment => {
            let row: (i64,i64,i64,i64,i64,i64) = connection.query_row("SELECT operating_cycle_id, project_id, ticket_id, target_graph_revision_id, evaluator_content_object_id, input_manifest_content_object_id FROM command_register_deterministic_experiment WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing deterministic experiment command body"))?;
            CommandBody::RegisterDeterministicExperiment {
                operating_cycle_id: OperatingCycleId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                project_id: ProjectId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                ticket_id: TicketId::try_from(row.2).map_err(|_| StoreError::InvalidStoredValue)?,
                target_graph_revision_id: GraphRevisionId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_content_object_id: ContentObjectId::try_from(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                input_manifest_content_object_id: ContentObjectId::try_from(row.5)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::RecordDeterministicEvaluationReceipt => {
            let row: (i64,i64,i64,i64,i64,i64) = connection.query_row("SELECT operating_cycle_id, deterministic_experiment_id, evaluator_revision_id, input_manifest_id, forensic_manifest_id, evaluator_output_content_object_id FROM command_record_deterministic_evaluation_receipt WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing deterministic evaluation receipt command body"))?;
            CommandBody::RecordDeterministicEvaluationReceipt {
                operating_cycle_id: OperatingCycleId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_experiment_id: DeterministicExperimentId::try_from(row.1)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_revision_id: EvaluatorRevisionId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                input_manifest_id: InputManifestId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                forensic_manifest_id: ForensicManifestId::try_from(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_output_content_object_id: ContentObjectId::try_from(row.5)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::AdmitDeterministicEvidence => {
            let row: (i64,i64,i64,i64,i64,i64,i64,i64,i64,String) = connection.query_row("SELECT operating_cycle_id, deterministic_evaluation_receipt_id, deterministic_experiment_id, evaluator_revision_id, input_manifest_id, evaluator_output_content_object_id, related_graph_revision_id, semantic_role, applicability, limitation_text FROM command_admit_deterministic_evidence WHERE command_row_id = ?1", [command_row_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?,row.get(7)?,row.get(8)?,row.get(9)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing deterministic evidence command body"))?;
            CommandBody::AdmitDeterministicEvidence {
                operating_cycle_id: OperatingCycleId::try_from(row.0)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_evaluation_receipt_id: DeterministicEvaluationReceiptId::try_from(
                    row.1,
                )
                .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_experiment_id: DeterministicExperimentId::try_from(row.2)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_revision_id: EvaluatorRevisionId::try_from(row.3)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                input_manifest_id: InputManifestId::try_from(row.4)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                evaluator_output_content_object_id: ContentObjectId::try_from(row.5)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                related_graph_revision_id: GraphRevisionId::try_from(row.6)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                semantic_role: evidence_semantic_role_from_i64(row.7)?,
                applicability: evidence_applicability_from_i64(row.8)?,
                limitation: EvidenceLimitationText::parse(row.9)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
        CommandKind::CloseDeterministicExperiment => {
            let (cycle, experiment) = query_command_pair(
                connection,
                "command_close_deterministic_experiment",
                command_row_id,
            )?;
            CommandBody::CloseDeterministicExperiment {
                operating_cycle_id: OperatingCycleId::try_from(cycle)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                deterministic_experiment_id: DeterministicExperimentId::try_from(experiment)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
            }
        }
    };
    Ok(body)
}

fn query_command_value<T>(
    connection: &Connection,
    table: &str,
    column: &str,
    command_row_id: i64,
) -> Result<T, StoreError>
where
    T: FromSql,
{
    let query = format!("SELECT {column} FROM {table} WHERE command_row_id = ?1");
    connection
        .query_row(&query, [command_row_id], |row| row.get(0))
        .optional()?
        .ok_or(StoreError::LedgerCorruption("missing simple command body"))
}

fn query_event_pair(
    connection: &Connection,
    table: &str,
    event_id: i64,
) -> Result<(i64, i64), StoreError> {
    let query = format!("SELECT * FROM {table} WHERE event_id = ?1");
    connection
        .query_row(&query, [event_id], |row| Ok((row.get(1)?, row.get(2)?)))
        .optional()?
        .ok_or(StoreError::LedgerCorruption("missing two-field event body"))
}

fn query_command_pair(
    connection: &Connection,
    table: &str,
    id: i64,
) -> Result<(i64, i64), StoreError> {
    let query = format!("SELECT * FROM {table} WHERE command_row_id = ?1");
    connection
        .query_row(&query, [id], |r| Ok((r.get(1)?, r.get(2)?)))
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing two-field command body",
        ))
}
fn query_command_three(
    connection: &Connection,
    table: &str,
    id: i64,
) -> Result<(i64, i64, i64), StoreError> {
    let query = format!("SELECT * FROM {table} WHERE command_row_id = ?1");
    connection
        .query_row(&query, [id], |r| Ok((r.get(1)?, r.get(2)?, r.get(3)?)))
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing three-field command body",
        ))
}
fn query_command_i64_text(
    connection: &Connection,
    table: &str,
    id: i64,
) -> Result<(i64, String), StoreError> {
    let query = format!("SELECT * FROM {table} WHERE command_row_id = ?1");
    connection
        .query_row(&query, [id], |r| Ok((r.get(1)?, r.get(2)?)))
        .optional()?
        .ok_or(StoreError::LedgerCorruption("missing text command body"))
}
fn query_command_six(
    connection: &Connection,
    table: &str,
    id: i64,
) -> Result<(i64, i64, String, String, String), StoreError> {
    let query = format!("SELECT * FROM {table} WHERE command_row_id = ?1");
    connection
        .query_row(&query, [id], |r| {
            Ok((r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
        })
        .optional()?
        .ok_or(StoreError::LedgerCorruption("missing charter command body"))
}
fn query_create_ticket(
    connection: &Connection,
    id: i64,
) -> Result<(i64, i64, String, String, Option<i64>), StoreError> {
    connection.query_row("SELECT operating_cycle_id, project_id, ticket_title, acceptance_condition_text, prerequisite_ticket_id FROM command_create_ticket WHERE command_row_id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing ticket command body"))
}
type GraphRevisionCommandRow = (i64, i64, Option<i64>, Option<i64>);

fn query_graph_revision_command(
    connection: &Connection,
    id: i64,
) -> Result<GraphRevisionCommandRow, StoreError> {
    connection.query_row("SELECT operating_cycle_id, project_id, causal_episode_id, graph_object_id FROM command_add_graph_object_revision WHERE command_row_id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing revision command body"))
}

fn query_graph_revision_command_body(
    connection: &Connection,
    command_row_id: i64,
) -> Result<GraphRevisionBody, StoreError> {
    let observation: Option<String> = connection
        .query_row(
            "SELECT observation_text FROM command_add_observation_revision WHERE command_row_id = ?1",
            [command_row_id],
            |row| row.get(0),
        )
        .optional()?;
    let hypothesis: Option<String> = connection
        .query_row(
            "SELECT hypothesis_text FROM command_add_hypothesis_revision WHERE command_row_id = ?1",
            [command_row_id],
            |row| row.get(0),
        )
        .optional()?;
    match (observation, hypothesis) {
        (Some(observation), None) => Ok(GraphRevisionBody::Observation {
            observation: ObservationRevisionText::parse(observation)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        }),
        (None, Some(hypothesis)) => Ok(GraphRevisionBody::Hypothesis {
            hypothesis: HypothesisRevisionText::parse(hypothesis)
                .map_err(|_| StoreError::InvalidStoredValue)?,
        }),
        _ => Err(StoreError::LedgerCorruption(
            "graph revision command has missing or ambiguous typed body",
        )),
    }
}
fn query_edge_command(
    connection: &Connection,
    id: i64,
) -> Result<(i64, i64, i64, i64, i64), StoreError> {
    connection.query_row("SELECT operating_cycle_id, project_id, from_graph_revision_id, to_graph_revision_id, edge_kind FROM command_add_graph_edge WHERE command_row_id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing edge command body"))
}
fn query_review_submit(
    connection: &Connection,
    id: i64,
) -> Result<(i64, i64, i64, i64, i64, String), StoreError> {
    connection.query_row("SELECT operating_cycle_id, adversarial_review_id, target_graph_revision_id, author_principal_id, severity, failure_hypothesis FROM command_submit_review_challenge WHERE command_row_id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing review command body"))
}
fn query_postmortem_trigger(
    connection: &Connection,
    id: i64,
) -> Result<(i64, i64, Option<i64>), StoreError> {
    connection.query_row("SELECT operating_cycle_id, project_id, causal_episode_id FROM command_trigger_postmortem WHERE command_row_id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional()?.ok_or(StoreError::LedgerCorruption("missing postmortem trigger command body"))
}
fn query_postmortem_text_command(
    connection: &Connection,
    table: &str,
    id: i64,
) -> Result<(i64, i64, i64, String), StoreError> {
    let query = format!("SELECT * FROM {table} WHERE command_row_id = ?1");
    connection
        .query_row(&query, [id], |r| {
            Ok((r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing postmortem text command body",
        ))
}

fn query_command_id<T>(
    connection: &Connection,
    table: &str,
    column: &str,
    command_row_id: i64,
) -> Result<T, StoreError>
where
    T: TryFrom<i64>,
{
    T::try_from(query_command_value::<i64>(
        connection,
        table,
        column,
        command_row_id,
    )?)
    .map_err(|_| StoreError::InvalidStoredValue)
}

fn digest_from_stored_bytes(bytes: &[u8]) -> Result<Sha256Digest, StoreError> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| StoreError::InvalidStoredValue)?;
    Ok(Sha256Digest::from_bytes(bytes))
}

fn verify_exact_event_body(
    connection: &Connection,
    event_id: EventId,
    kind: EventKind,
) -> Result<(), StoreError> {
    let expected_table = EVENT_BODY_TABLES[(kind as usize) - 1];
    verify_exact_named_body(
        connection,
        event_id.value(),
        expected_table,
        &EVENT_BODY_TABLES,
    )
}

/// Graph revision semantics are deliberately outside the shared revision row.
/// Replay therefore proves the selected object kind owns exactly one matching
/// named body and that the stored semantic field still decodes as its closed
/// Rust type. This catches missing, duplicate, and cross-kind body tampering.
fn verify_graph_revision_bodies(connection: &Connection) -> Result<(), StoreError> {
    let mut statement = connection.prepare(
        "SELECT r.graph_revision_id, o.object_kind
         FROM object_revisions r
         JOIN objects o ON o.graph_object_id = r.graph_object_id
         ORDER BY r.graph_revision_id",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
    for row in rows {
        let (graph_revision_id, object_kind) = row?;
        let object_kind = graph_object_kind_from_i64(object_kind)?;
        let expected_table = match object_kind {
            GraphObjectKind::Observation => "observation_revisions",
            GraphObjectKind::Hypothesis => "hypothesis_revisions",
        };
        let mut body_count = 0_i64;
        let mut expected_present = false;
        for table in GRAPH_REVISION_BODY_TABLES {
            let query = format!("SELECT COUNT(*) FROM {table} WHERE graph_revision_id = ?1");
            let count: i64 = connection.query_row(&query, [graph_revision_id], |row| row.get(0))?;
            body_count += count;
            if table == expected_table {
                expected_present = count == 1;
            }
        }
        if body_count != 1 || !expected_present {
            return Err(StoreError::LedgerCorruption(
                "graph revision typed body is missing, duplicated, or mismatched",
            ));
        }
        match object_kind {
            GraphObjectKind::Observation => {
                let text: String = connection.query_row(
                    "SELECT observation_text FROM observation_revisions WHERE graph_revision_id = ?1",
                    [graph_revision_id],
                    |row| row.get(0),
                )?;
                ObservationRevisionText::parse(text).map_err(|_| StoreError::InvalidStoredValue)?;
            }
            GraphObjectKind::Hypothesis => {
                let text: String = connection.query_row(
                    "SELECT hypothesis_text FROM hypothesis_revisions WHERE graph_revision_id = ?1",
                    [graph_revision_id],
                    |row| row.get(0),
                )?;
                HypothesisRevisionText::parse(text).map_err(|_| StoreError::InvalidStoredValue)?;
            }
        }
    }
    Ok(())
}

/// The table names are compiled constants, never protocol input. Counting all
/// closed body tables makes an inserted second body as corrupt as a missing or
/// mismatched body instead of silently trusting the discriminant.
fn verify_exact_named_body(
    connection: &Connection,
    row_id: i64,
    expected_table: &str,
    tables: &[&str],
) -> Result<(), StoreError> {
    let mut body_count = 0_i64;
    let mut expected_present = false;
    for table in tables {
        let query = format!(
            "SELECT COUNT(*) FROM {table} WHERE {} = ?1",
            body_key_column(table)
        );
        let count: i64 = connection.query_row(&query, [row_id], |row| row.get(0))?;
        body_count += count;
        if *table == expected_table {
            expected_present = count == 1;
        }
    }
    if body_count != 1 || !expected_present {
        return Err(StoreError::LedgerCorruption(
            "closed body is missing, duplicated, or mismatched",
        ));
    }
    Ok(())
}

fn body_key_column(table: &str) -> &'static str {
    if table.starts_with("command_") {
        "command_row_id"
    } else {
        "event_id"
    }
}

fn command_body_table(kind: CommandKind) -> Result<&'static str, StoreError> {
    COMMAND_BODY_TABLES
        .get((kind as usize) - 1)
        .copied()
        .ok_or(StoreError::InvalidStoredValue)
}

fn command_kind_from_i64(value: i64) -> Result<CommandKind, StoreError> {
    match value {
        1 => Ok(CommandKind::CreateSocietyIdentity),
        2 => Ok(CommandKind::InstallGrandArchitectOffice),
        3 => Ok(CommandKind::InstallFoundingUniverseSeed),
        4 => Ok(CommandKind::AppointInitialGrandArchitect),
        5 => Ok(CommandKind::SetR0HardCeiling),
        6 => Ok(CommandKind::BootstrapSociety),
        7 => Ok(CommandKind::ProposeOperatingCycle),
        8 => Ok(CommandKind::AdmitOperatingCycle),
        9 => Ok(CommandKind::StartGrandArchitectOfficeSession),
        10 => Ok(CommandKind::RecordOfficeSessionReady),
        11 => Ok(CommandKind::OpenOfficeTurn),
        12 => Ok(CommandKind::SettleOfficeTurn),
        13 => Ok(CommandKind::QuiesceOperatingCycle),
        14 => Ok(CommandKind::RecordCycleDrained),
        15 => Ok(CommandKind::ResumeOperatingCycle),
        16 => Ok(CommandKind::ReconcileOperatingCycle),
        17 => Ok(CommandKind::CloseOperatingCycle),
        18 => Ok(CommandKind::ReserveBudget),
        19 => Ok(CommandKind::ReconcileBudget),
        20 => Ok(CommandKind::RequestCancellation),
        21 => Ok(CommandKind::ReconcileCancellation),
        22 => Ok(CommandKind::RecordOfficeSessionTerminal),
        23 => Ok(CommandKind::CloseCostPostmortem),
        24 => Ok(CommandKind::CreateProject),
        25 => Ok(CommandKind::CharterProject),
        26 => Ok(CommandKind::TransitionProject),
        27 => Ok(CommandKind::CompleteProjectMilestone),
        28 => Ok(CommandKind::ReopenProject),
        29 => Ok(CommandKind::CreateTicket),
        30 => Ok(CommandKind::TransitionTicket),
        31 => Ok(CommandKind::AddGraphObjectRevision),
        32 => Ok(CommandKind::CommitGraphRevision),
        33 => Ok(CommandKind::AddGraphEdge),
        34 => Ok(CommandKind::CreateEpisode),
        35 => Ok(CommandKind::TransitionEpisode),
        36 => Ok(CommandKind::ReopenEpisode),
        37 => Ok(CommandKind::RequestAdversarialReview),
        38 => Ok(CommandKind::SubmitReviewChallenge),
        39 => Ok(CommandKind::RespondToReviewChallenge),
        40 => Ok(CommandKind::DispositionReviewChallenge),
        41 => Ok(CommandKind::ResolveAdversarialReview),
        42 => Ok(CommandKind::TriggerPostmortem),
        43 => Ok(CommandKind::RecordPostmortemCausalClaim),
        44 => Ok(CommandKind::ProposePostmortemAction),
        45 => Ok(CommandKind::ClosePostmortem),
        46 => Ok(CommandKind::AssignAdversarialReviewer),
        47 => Ok(CommandKind::RegisterActorConfiguration),
        48 => Ok(CommandKind::RegisterContextPack),
        49 => Ok(CommandKind::AdmitActorInstance),
        50 => Ok(CommandKind::AdmitTicket),
        51 => Ok(CommandKind::RegisterWorkItem),
        52 => Ok(CommandKind::ClaimWorkItem),
        53 => Ok(CommandKind::StartActorAttempt),
        54 => Ok(CommandKind::AttestActorAttemptTerminal),
        55 => Ok(CommandKind::ValidateTicketAttempt),
        56 => Ok(CommandKind::RetryActorAttempt),
        57 => Ok(CommandKind::CompleteTicket),
        58 => Ok(CommandKind::ExpireWorkLease),
        59 => Ok(CommandKind::CancelActorAttempt),
        60 => Ok(CommandKind::RegisterOutcomeObligation),
        61 => Ok(CommandKind::ResolveOutcomeObligation),
        62 => Ok(CommandKind::RecordContentSealReceipt),
        63 => Ok(CommandKind::RegisterContentObject),
        64 => Ok(CommandKind::RegisterForensicManifest),
        65 => Ok(CommandKind::RegisterDeterministicExperiment),
        66 => Ok(CommandKind::RecordDeterministicEvaluationReceipt),
        67 => Ok(CommandKind::AdmitDeterministicEvidence),
        68 => Ok(CommandKind::CloseDeterministicExperiment),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn capability_from_i64(value: i64) -> Result<Capability, StoreError> {
    match value {
        1 => Ok(Capability::CreateSocietyIdentity),
        2 => Ok(Capability::InstallGrandArchitectOffice),
        3 => Ok(Capability::InstallFoundingUniverseSeed),
        4 => Ok(Capability::AppointInitialGrandArchitect),
        5 => Ok(Capability::SetR0HardCeiling),
        6 => Ok(Capability::BootstrapSociety),
        7 => Ok(Capability::ProposeOperatingCycle),
        8 => Ok(Capability::AdmitOperatingCycle),
        9 => Ok(Capability::QuiesceOperatingCycle),
        10 => Ok(Capability::ResumeOperatingCycle),
        11 => Ok(Capability::ReconcileOperatingCycle),
        12 => Ok(Capability::CloseOperatingCycle),
        13 => Ok(Capability::StartGrandArchitectOfficeSession),
        14 => Ok(Capability::OpenOfficeTurn),
        15 => Ok(Capability::RequestCancellation),
        16 => Ok(Capability::ReserveBudget),
        17 => Ok(Capability::ReconcileBudget),
        18 => Ok(Capability::RecordCycleDrained),
        19 => Ok(Capability::RecordOfficeSessionReady),
        20 => Ok(Capability::SettleOfficeTurn),
        21 => Ok(Capability::ReconcileCancellation),
        22 => Ok(Capability::RecordOfficeSessionTerminal),
        23 => Ok(Capability::CloseCostPostmortem),
        24 => Ok(Capability::CreateProject),
        25 => Ok(Capability::CharterProject),
        26 => Ok(Capability::TransitionProject),
        27 => Ok(Capability::CompleteProjectMilestone),
        28 => Ok(Capability::ReopenProject),
        29 => Ok(Capability::CreateTicket),
        30 => Ok(Capability::TransitionTicket),
        31 => Ok(Capability::AddGraphObjectRevision),
        32 => Ok(Capability::CommitGraphRevision),
        33 => Ok(Capability::AddGraphEdge),
        34 => Ok(Capability::CreateEpisode),
        35 => Ok(Capability::TransitionEpisode),
        36 => Ok(Capability::ReopenEpisode),
        37 => Ok(Capability::RequestAdversarialReview),
        38 => Ok(Capability::SubmitReviewChallenge),
        39 => Ok(Capability::RespondToReviewChallenge),
        40 => Ok(Capability::DispositionReviewChallenge),
        41 => Ok(Capability::ResolveAdversarialReview),
        42 => Ok(Capability::TriggerPostmortem),
        43 => Ok(Capability::RecordPostmortemCausalClaim),
        44 => Ok(Capability::ProposePostmortemAction),
        45 => Ok(Capability::ClosePostmortem),
        46 => Ok(Capability::AssignAdversarialReviewer),
        47 => Ok(Capability::RegisterActorConfiguration),
        48 => Ok(Capability::RegisterContextPack),
        49 => Ok(Capability::AdmitActorInstance),
        50 => Ok(Capability::AdmitTicket),
        51 => Ok(Capability::RegisterWorkItem),
        52 => Ok(Capability::ClaimWorkItem),
        53 => Ok(Capability::StartActorAttempt),
        54 => Ok(Capability::AttestActorAttemptTerminal),
        55 => Ok(Capability::ValidateTicketAttempt),
        56 => Ok(Capability::RetryActorAttempt),
        57 => Ok(Capability::CompleteTicket),
        58 => Ok(Capability::ExpireWorkLease),
        59 => Ok(Capability::CancelActorAttempt),
        60 => Ok(Capability::RegisterOutcomeObligation),
        61 => Ok(Capability::ResolveOutcomeObligation),
        62 => Ok(Capability::RecordContentSealReceipt),
        63 => Ok(Capability::RegisterContentObject),
        64 => Ok(Capability::RegisterForensicManifest),
        65 => Ok(Capability::RegisterDeterministicExperiment),
        66 => Ok(Capability::RecordDeterministicEvaluationReceipt),
        67 => Ok(Capability::AdmitDeterministicEvidence),
        68 => Ok(Capability::CloseDeterministicExperiment),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn query_event_id<T>(
    connection: &Connection,
    table: &str,
    column: &str,
    event_id: EventId,
) -> Result<T, StoreError>
where
    T: TryFrom<i64>,
{
    let query = format!("SELECT {column} FROM {table} WHERE event_id = ?1");
    let value = connection
        .query_row(&query, [event_id.value()], |row| row.get::<_, i64>(0))
        .optional()?
        .ok_or(StoreError::LedgerCorruption("missing simple event body"))?;
    T::try_from(value).map_err(|_| StoreError::InvalidStoredValue)
}

fn decode_office_turn_opened_event(
    connection: &Connection,
    event_id: EventId,
) -> Result<EventBody, StoreError> {
    let (turn, session, purpose) = connection
        .query_row(
            "SELECT office_turn_id, grand_architect_office_session_id, purpose
             FROM event_office_turn_opened WHERE event_id = ?1",
            [event_id.value()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing office turn event body",
        ))?;
    let turn_id = OfficeTurnId::try_from(turn).map_err(|_| StoreError::InvalidStoredValue)?;
    let session_id = GrandArchitectOfficeSessionId::try_from(session)
        .map_err(|_| StoreError::InvalidStoredValue)?;
    Ok(EventBody::OfficeTurnOpened {
        turn_id,
        session_id,
        purpose: office_turn_purpose_from_i64(purpose)?,
    })
}

fn decode_office_turn_settled_event(
    connection: &Connection,
    event_id: EventId,
) -> Result<EventBody, StoreError> {
    let (turn, session) = connection
        .query_row(
            "SELECT office_turn_id, grand_architect_office_session_id
             FROM event_office_turn_settled WHERE event_id = ?1",
            [event_id.value()],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?
        .ok_or(StoreError::LedgerCorruption(
            "missing office turn event body",
        ))?;
    Ok(EventBody::OfficeTurnSettled {
        turn_id: OfficeTurnId::try_from(turn).map_err(|_| StoreError::InvalidStoredValue)?,
        session_id: GrandArchitectOfficeSessionId::try_from(session)
            .map_err(|_| StoreError::InvalidStoredValue)?,
    })
}

fn rejection_from_i64(value: i64) -> Result<Rejection, StoreError> {
    match value {
        1 => Ok(Rejection::CapabilityMismatch),
        2 => Ok(Rejection::CapabilityNotGranted),
        3 => Ok(Rejection::CapabilityNoLongerActive),
        4 => Ok(Rejection::InvalidExpectedGeneration),
        5 => Ok(Rejection::StaleAdmissionGeneration),
        6 => Ok(Rejection::InvalidLifecycleTransition),
        7 => Ok(Rejection::FoundingInvariant),
        8 => Ok(Rejection::ActiveCycleAlreadyExists),
        9 => Ok(Rejection::ActiveOfficeOccupancyAlreadyExists),
        10 => Ok(Rejection::BudgetCeilingExceeded),
        11 => Ok(Rejection::ReservationNotActive),
        12 => Ok(Rejection::CostExceedsReservation),
        13 => Ok(Rejection::IncompleteCycleReconciliation),
        14 => Ok(Rejection::SessionTurnAlreadyActive),
        15 => Ok(Rejection::CancellationAlreadyTerminal),
        16 => Ok(Rejection::SubjectNotFound),
        17 => Ok(Rejection::BudgetPolicyViolation),
        18 => Ok(Rejection::CostPostmortemNotOpen),
        19 => Ok(Rejection::InvalidCostPostmortemResolution),
        20 => Ok(Rejection::ProjectCloseBlocked),
        21 => Ok(Rejection::TicketPrerequisiteIncomplete),
        22 => Ok(Rejection::GraphRevisionNotCommitted),
        23 => Ok(Rejection::IllegalGraphEdgeEndpoint),
        24 => Ok(Rejection::ReviewSelfDispositionDenied),
        25 => Ok(Rejection::ReviewDispositionIncomplete),
        26 => Ok(Rejection::PostmortemCloseBlocked),
        27 => Ok(Rejection::ReviewAssignmentNotIndependent),
        28 => Ok(Rejection::ActorJurisdictionDenied),
        29 => Ok(Rejection::WorkLeaseUnavailable),
        30 => Ok(Rejection::ActorAttemptNotTerminal),
        31 => Ok(Rejection::ActorAttemptNotValidatable),
        32 => Ok(Rejection::OutcomeObligationOpen),
        33 => Ok(Rejection::ReviewAssignmentEvidenceMissing),
        34 => Ok(Rejection::ExecutionProfileIneligible),
        35 => Ok(Rejection::TicketAcceptanceConditionUnsatisfied),
        36 => Ok(Rejection::QualificationTreatmentRestricted),
        37 => Ok(Rejection::ContentSealReceiptMissing),
        38 => Ok(Rejection::ContentObjectNotSealed),
        39 => Ok(Rejection::ForensicManifestBindingMismatch),
        40 => Ok(Rejection::DeterministicExperimentBindingMismatch),
        41 => Ok(Rejection::DeterministicEvaluationBindingMismatch),
        42 => Ok(Rejection::EvidenceAdmissionRequired),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn event_kind_from_i64(value: i64) -> Result<EventKind, StoreError> {
    match value {
        1 => Ok(EventKind::SocietyIdentityCreated),
        2 => Ok(EventKind::GrandArchitectOfficeInstalled),
        3 => Ok(EventKind::FoundingUniverseSeedInstalled),
        4 => Ok(EventKind::GrandArchitectAppointed),
        5 => Ok(EventKind::R0HardCeilingSet),
        6 => Ok(EventKind::SocietyBootstrapped),
        7 => Ok(EventKind::OperatingCycleProposed),
        8 => Ok(EventKind::OperatingCycleStateChanged),
        9 => Ok(EventKind::GrandArchitectOfficeSessionStarted),
        10 => Ok(EventKind::GrandArchitectOfficeSessionStateChanged),
        11 => Ok(EventKind::OfficeTurnOpened),
        12 => Ok(EventKind::OfficeTurnSettled),
        13 => Ok(EventKind::BudgetReserved),
        14 => Ok(EventKind::BudgetReconciled),
        15 => Ok(EventKind::BudgetAdmissionFrozen),
        16 => Ok(EventKind::CancellationRequested),
        17 => Ok(EventKind::CancellationReconciled),
        18 => Ok(EventKind::CostPostmortemClosed),
        19 => Ok(EventKind::ProjectCreated),
        20 => Ok(EventKind::ProjectChartered),
        21 => Ok(EventKind::ProjectStateChanged),
        22 => Ok(EventKind::ProjectMilestoneCompleted),
        23 => Ok(EventKind::TicketCreated),
        24 => Ok(EventKind::TicketStateChanged),
        25 => Ok(EventKind::GraphObjectRevisionAdded),
        26 => Ok(EventKind::GraphRevisionCommitted),
        27 => Ok(EventKind::GraphEdgeAdded),
        28 => Ok(EventKind::EpisodeCreated),
        29 => Ok(EventKind::EpisodeStateChanged),
        30 => Ok(EventKind::AdversarialReviewRequested),
        31 => Ok(EventKind::ReviewChallengeSubmitted),
        32 => Ok(EventKind::ReviewChallengeResponded),
        33 => Ok(EventKind::ReviewChallengeDispositioned),
        34 => Ok(EventKind::AdversarialReviewResolved),
        35 => Ok(EventKind::PostmortemTriggered),
        36 => Ok(EventKind::PostmortemCausalClaimRecorded),
        37 => Ok(EventKind::PostmortemActionProposed),
        38 => Ok(EventKind::PostmortemClosed),
        39 => Ok(EventKind::AdversarialReviewerAssigned),
        40 => Ok(EventKind::ActorConfigurationRegistered),
        41 => Ok(EventKind::ContextPackRegistered),
        42 => Ok(EventKind::ActorInstanceAdmitted),
        43 => Ok(EventKind::TicketAdmitted),
        44 => Ok(EventKind::WorkItemRegistered),
        45 => Ok(EventKind::WorkItemClaimed),
        46 => Ok(EventKind::ActorAttemptStarted),
        47 => Ok(EventKind::ActorAttemptTerminalAttested),
        48 => Ok(EventKind::TicketAttemptValidated),
        49 => Ok(EventKind::ActorAttemptRetryPrepared),
        50 => Ok(EventKind::TicketCompleted),
        51 => Ok(EventKind::WorkLeaseExpired),
        52 => Ok(EventKind::ActorAttemptCancellationRequested),
        53 => Ok(EventKind::OutcomeObligationRegistered),
        54 => Ok(EventKind::OutcomeObligationResolved),
        55 => Ok(EventKind::ContentSealReceiptRecorded),
        56 => Ok(EventKind::ContentObjectRegistered),
        57 => Ok(EventKind::ForensicManifestRegistered),
        58 => Ok(EventKind::DeterministicExperimentRegistered),
        59 => Ok(EventKind::DeterministicEvaluationReceiptRecorded),
        60 => Ok(EventKind::DeterministicEvidenceAdmitted),
        61 => Ok(EventKind::DeterministicExperimentClosed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn operating_cycle_state_from_i64(value: i64) -> Result<OperatingCycleState, StoreError> {
    match value {
        1 => Ok(OperatingCycleState::Proposed),
        2 => Ok(OperatingCycleState::Admitted),
        3 => Ok(OperatingCycleState::Running),
        4 => Ok(OperatingCycleState::Quiescing),
        5 => Ok(OperatingCycleState::Drained),
        6 => Ok(OperatingCycleState::Reconciling),
        7 => Ok(OperatingCycleState::Closed),
        8 => Ok(OperatingCycleState::Cancelling),
        9 => Ok(OperatingCycleState::Reaping),
        10 => Ok(OperatingCycleState::Cancelled),
        11 => Ok(OperatingCycleState::Failed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn operating_cycle_treatment_from_i64(value: i64) -> Result<OperatingCycleTreatment, StoreError> {
    match value {
        1 => Ok(OperatingCycleTreatment::PiSdkQualificationV1),
        2 => Ok(OperatingCycleTreatment::Vs001LiveV1),
        3 => Ok(OperatingCycleTreatment::Vs001DeterministicV1),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn retention_access_class_from_i64(value: i64) -> Result<RetentionAccessClass, StoreError> {
    match value {
        1 => Ok(RetentionAccessClass::ForensicRestricted),
        2 => Ok(RetentionAccessClass::ProjectScoped),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn forensic_manifest_capture_policy_from_i64(
    value: i64,
) -> Result<ForensicManifestCapturePolicy, StoreError> {
    match value {
        1 => Ok(ForensicManifestCapturePolicy::DeterministicExperimentEvaluatorV1),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn evidence_semantic_role_from_i64(value: i64) -> Result<EvidenceSemanticRole, StoreError> {
    match value {
        1 => Ok(EvidenceSemanticRole::DeterministicObservation),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn evidence_applicability_from_i64(value: i64) -> Result<crate::EvidenceApplicability, StoreError> {
    match value {
        1 => Ok(crate::EvidenceApplicability::TestsTargetHypothesis),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn office_session_state_from_i64(value: i64) -> Result<OfficeSessionState, StoreError> {
    match value {
        1 => Ok(OfficeSessionState::Reserved),
        2 => Ok(OfficeSessionState::Starting),
        3 => Ok(OfficeSessionState::Ready),
        4 => Ok(OfficeSessionState::TurnActive),
        5 => Ok(OfficeSessionState::Quiescing),
        6 => Ok(OfficeSessionState::ProcessEnded),
        7 => Ok(OfficeSessionState::EvidenceSealing),
        8 => Ok(OfficeSessionState::Closed),
        9 => Ok(OfficeSessionState::Cancelling),
        10 => Ok(OfficeSessionState::Cancelled),
        11 => Ok(OfficeSessionState::Failed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn office_session_terminal_state_from_i64(
    value: i64,
) -> Result<OfficeSessionTerminalState, StoreError> {
    match value {
        1 => Ok(OfficeSessionTerminalState::Closed),
        2 => Ok(OfficeSessionTerminalState::Cancelled),
        3 => Ok(OfficeSessionTerminalState::Failed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn office_turn_purpose_from_i64(value: i64) -> Result<OfficeTurnPurpose, StoreError> {
    match value {
        1 => Ok(OfficeTurnPurpose::OrdinaryWork),
        2 => Ok(OfficeTurnPurpose::Recovery),
        3 => Ok(OfficeTurnPurpose::Cancellation),
        4 => Ok(OfficeTurnPurpose::Closure),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn cancellation_mode_from_i64(value: i64) -> Result<CancellationMode, StoreError> {
    match value {
        1 => Ok(CancellationMode::Quiesce),
        2 => Ok(CancellationMode::GracefulCancel),
        3 => Ok(CancellationMode::EmergencyStop),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn budget_freeze_reason_to_sql(
    reason: BudgetFreezeReason,
) -> (i64, Option<i64>, Option<i64>, Option<i64>, Option<i64>) {
    match reason {
        BudgetFreezeReason::KnownOverrun { observed, reserved } => (
            1,
            Some(observed.value()),
            Some(reserved.value()),
            None,
            None,
        ),
        BudgetFreezeReason::Unknown(reason) => (2, None, None, Some(reason as i64), None),
        BudgetFreezeReason::Unavailable(reason) => (3, None, None, None, Some(reason as i64)),
    }
}

fn budget_freeze_reason_from_sql(
    kind: i64,
    observed: Option<i64>,
    reserved: Option<i64>,
    unknown: Option<i64>,
    unavailable: Option<i64>,
) -> Result<BudgetFreezeReason, StoreError> {
    match (kind, observed, reserved, unknown, unavailable) {
        (1, Some(observed), Some(reserved), None, None) => Ok(BudgetFreezeReason::KnownOverrun {
            observed: UsdMicros::try_from(observed).map_err(|_| StoreError::InvalidStoredValue)?,
            reserved: UsdMicros::try_from(reserved).map_err(|_| StoreError::InvalidStoredValue)?,
        }),
        (2, None, None, Some(reason), None) => Ok(BudgetFreezeReason::Unknown(
            cost_unknown_reason_from_i64(reason)?,
        )),
        (3, None, None, None, Some(reason)) => Ok(BudgetFreezeReason::Unavailable(
            cost_unavailable_reason_from_i64(reason)?,
        )),
        _ => Err(StoreError::LedgerCorruption(
            "invalid budget freeze reason body",
        )),
    }
}

fn cost_observation_from_sql(
    kind: i64,
    known: Option<i64>,
    unknown: Option<i64>,
    unavailable: Option<i64>,
) -> Result<CostObservation, StoreError> {
    match (kind, known, unknown, unavailable) {
        (1, Some(amount), None, None) => Ok(CostObservation::Known(
            UsdMicros::try_from(amount).map_err(|_| StoreError::InvalidStoredValue)?,
        )),
        (2, None, Some(reason), None) => Ok(CostObservation::Unknown(
            cost_unknown_reason_from_i64(reason)?,
        )),
        (3, None, None, Some(reason)) => Ok(CostObservation::Unavailable(
            cost_unavailable_reason_from_i64(reason)?,
        )),
        _ => Err(StoreError::LedgerCorruption(
            "invalid cost observation body",
        )),
    }
}

fn cost_postmortem_cause_from_i64(value: i64) -> Result<CostPostmortemCause, StoreError> {
    match value {
        1 => Ok(CostPostmortemCause::KnownOverrun),
        2 => Ok(CostPostmortemCause::UnknownCost),
        3 => Ok(CostPostmortemCause::UnavailableCost),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn cost_postmortem_resolution_from_i64(value: i64) -> Result<CostPostmortemResolution, StoreError> {
    match value {
        1 => Ok(CostPostmortemResolution::ConservativeFullReservation),
        2 => Ok(CostPostmortemResolution::ChargeObservedOverrun),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn cost_unknown_reason_from_i64(value: i64) -> Result<CostUnknownReason, StoreError> {
    match value {
        1 => Ok(CostUnknownReason::ProviderDidNotReport),
        2 => Ok(CostUnknownReason::AdapterStreamInterrupted),
        3 => Ok(CostUnknownReason::ReconciliationMismatch),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn cost_unavailable_reason_from_i64(value: i64) -> Result<CostUnavailableReason, StoreError> {
    match value {
        1 => Ok(CostUnavailableReason::ProviderUnavailable),
        2 => Ok(CostUnavailableReason::CredentialUnavailable),
        3 => Ok(CostUnavailableReason::QualificationRejected),
        _ => Err(StoreError::InvalidStoredValue),
    }
}

fn project_state_from_i64(value: i64) -> Result<ProjectState, StoreError> {
    match value {
        1 => Ok(ProjectState::Proposed),
        2 => Ok(ProjectState::Challenged),
        3 => Ok(ProjectState::Chartered),
        4 => Ok(ProjectState::Active),
        5 => Ok(ProjectState::Paused),
        6 => Ok(ProjectState::Observing),
        7 => Ok(ProjectState::Closed),
        8 => Ok(ProjectState::Terminated),
        9 => Ok(ProjectState::Reopened),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn ticket_state_from_i64(value: i64) -> Result<TicketState, StoreError> {
    match value {
        1 => Ok(TicketState::Draft),
        2 => Ok(TicketState::Admitted),
        3 => Ok(TicketState::Ready),
        4 => Ok(TicketState::Claimed),
        5 => Ok(TicketState::Submitted),
        6 => Ok(TicketState::Verified),
        7 => Ok(TicketState::Completed),
        8 => Ok(TicketState::ChangesRequested),
        9 => Ok(TicketState::Expired),
        10 => Ok(TicketState::Cancelled),
        11 => Ok(TicketState::Failed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn actor_model_policy_from_i64(value: i64) -> Result<ActorModelPolicy, StoreError> {
    match value {
        1 => Ok(ActorModelPolicy::Vs001DeepseekV4FlashHigh),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn developmental_attractor_from_i64(value: i64) -> Result<DevelopmentalAttractor, StoreError> {
    match value {
        1 => Ok(DevelopmentalAttractor::Explore),
        2 => Ok(DevelopmentalAttractor::Build),
        3 => Ok(DevelopmentalAttractor::Measure),
        4 => Ok(DevelopmentalAttractor::Challenge),
        5 => Ok(DevelopmentalAttractor::Synthesize),
        6 => Ok(DevelopmentalAttractor::Integrate),
        7 => Ok(DevelopmentalAttractor::Remember),
        8 => Ok(DevelopmentalAttractor::Coordinate),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn execution_profile_kind_from_i64(value: i64) -> Result<ExecutionProfileKind, StoreError> {
    match value {
        1 => Ok(ExecutionProfileKind::DeterministicPiHostProcessDoubleV1),
        2 => Ok(ExecutionProfileKind::NativePinnedPiSdkV1),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn execution_profile_readiness_from_i64(
    value: i64,
) -> Result<ExecutionProfileReadiness, StoreError> {
    match value {
        1 => Ok(ExecutionProfileReadiness::DeterministicFixtureOnly),
        2 => Ok(ExecutionProfileReadiness::Unqualified),
        3 => Ok(ExecutionProfileReadiness::QualifiedForLiveUse),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn actor_instance_state_from_i64(value: i64) -> Result<ActorInstanceState, StoreError> {
    match value {
        1 => Ok(ActorInstanceState::Active),
        2 => Ok(ActorInstanceState::Retired),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn context_pack_purpose_from_i64(value: i64) -> Result<ContextPackPurpose, StoreError> {
    match value {
        1 => Ok(ContextPackPurpose::TicketExecution),
        2 => Ok(ContextPackPurpose::IndependentReview),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn work_item_kind_from_i64(value: i64) -> Result<WorkItemKind, StoreError> {
    match value {
        1 => Ok(WorkItemKind::TicketExecution),
        2 => Ok(WorkItemKind::IndependentReview),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn work_item_state_from_i64(value: i64) -> Result<WorkItemState, StoreError> {
    match value {
        1 => Ok(WorkItemState::Ready),
        2 => Ok(WorkItemState::Claimed),
        3 => Ok(WorkItemState::Running),
        4 => Ok(WorkItemState::Settled),
        5 => Ok(WorkItemState::Cancelled),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn actor_attempt_state_from_i64(value: i64) -> Result<ActorAttemptState, StoreError> {
    match value {
        1 => Ok(ActorAttemptState::Running),
        2 => Ok(ActorAttemptState::CancellationRequested),
        3 => Ok(ActorAttemptState::Succeeded),
        4 => Ok(ActorAttemptState::Failed),
        5 => Ok(ActorAttemptState::Cancelled),
        6 => Ok(ActorAttemptState::Expired),
        7 => Ok(ActorAttemptState::ProtocolFailed),
        8 => Ok(ActorAttemptState::SupervisorFailed),
        9 => Ok(ActorAttemptState::Validated),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn actor_attempt_terminal_kind_from_i64(
    value: i64,
) -> Result<ActorAttemptTerminalKind, StoreError> {
    match value {
        1 => Ok(ActorAttemptTerminalKind::Succeeded),
        2 => Ok(ActorAttemptTerminalKind::Failed),
        3 => Ok(ActorAttemptTerminalKind::Cancelled),
        4 => Ok(ActorAttemptTerminalKind::Expired),
        5 => Ok(ActorAttemptTerminalKind::ProtocolFailed),
        6 => Ok(ActorAttemptTerminalKind::SupervisorFailed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn actor_attempt_cancellation_reason_from_i64(
    value: i64,
) -> Result<ActorAttemptCancellationReason, StoreError> {
    match value {
        1 => Ok(ActorAttemptCancellationReason::GrandArchitectRequested),
        2 => Ok(ActorAttemptCancellationReason::CycleCancellation),
        3 => Ok(ActorAttemptCancellationReason::LeaseContainment),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn outcome_obligation_state_from_i64(value: i64) -> Result<OutcomeObligationState, StoreError> {
    match value {
        1 => Ok(OutcomeObligationState::Scheduled),
        2 => Ok(OutcomeObligationState::Satisfied),
        3 => Ok(OutcomeObligationState::Waived),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn outcome_obligation_disposition_from_i64(
    value: i64,
) -> Result<OutcomeObligationDisposition, StoreError> {
    match value {
        1 => Ok(OutcomeObligationDisposition::Satisfied),
        2 => Ok(OutcomeObligationDisposition::Waived),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn graph_object_kind_from_i64(value: i64) -> Result<GraphObjectKind, StoreError> {
    match value {
        1 => Ok(GraphObjectKind::Observation),
        2 => Ok(GraphObjectKind::Hypothesis),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn graph_revision_state_from_i64(value: i64) -> Result<GraphRevisionState, StoreError> {
    match value {
        1 => Ok(GraphRevisionState::Draft),
        2 => Ok(GraphRevisionState::Committed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn graph_edge_kind_from_i64(value: i64) -> Result<GraphEdgeKind, StoreError> {
    match value {
        1 => Ok(GraphEdgeKind::Supports),
        2 => Ok(GraphEdgeKind::Challenges),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn episode_state_from_i64(value: i64) -> Result<EpisodeState, StoreError> {
    match value {
        1 => Ok(EpisodeState::Framed),
        2 => Ok(EpisodeState::Admitted),
        3 => Ok(EpisodeState::Investigating),
        4 => Ok(EpisodeState::PrototypeDeliberating),
        5 => Ok(EpisodeState::Prototyping),
        6 => Ok(EpisodeState::CandidateValidating),
        7 => Ok(EpisodeState::DeliveryDeliberating),
        8 => Ok(EpisodeState::DeliveryAuthorized),
        9 => Ok(EpisodeState::Materializing),
        10 => Ok(EpisodeState::Observing),
        11 => Ok(EpisodeState::Learning),
        12 => Ok(EpisodeState::Closed),
        13 => Ok(EpisodeState::ClosedNoAction),
        14 => Ok(EpisodeState::ClosedNoDelivery),
        15 => Ok(EpisodeState::Abandoned),
        16 => Ok(EpisodeState::Reverted),
        17 => Ok(EpisodeState::Reopened),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn adversarial_review_state_from_i64(value: i64) -> Result<AdversarialReviewState, StoreError> {
    match value {
        1 => Ok(AdversarialReviewState::Requested),
        2 => Ok(AdversarialReviewState::Assigned),
        3 => Ok(AdversarialReviewState::Active),
        4 => Ok(AdversarialReviewState::FindingsSubmitted),
        5 => Ok(AdversarialReviewState::ResponsesDue),
        6 => Ok(AdversarialReviewState::Resolved),
        7 => Ok(AdversarialReviewState::AcceptedRisk),
        8 => Ok(AdversarialReviewState::Superseded),
        9 => Ok(AdversarialReviewState::Escalated),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn review_challenge_severity_from_i64(value: i64) -> Result<ReviewChallengeSeverity, StoreError> {
    match value {
        1 => Ok(ReviewChallengeSeverity::Low),
        2 => Ok(ReviewChallengeSeverity::Moderate),
        3 => Ok(ReviewChallengeSeverity::High),
        4 => Ok(ReviewChallengeSeverity::Critical),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn review_challenge_response_state_from_i64(
    value: i64,
) -> Result<ReviewChallengeResponseState, StoreError> {
    match value {
        1 => Ok(ReviewChallengeResponseState::Pending),
        2 => Ok(ReviewChallengeResponseState::Responded),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn review_disposition_kind_from_i64(value: i64) -> Result<ReviewDispositionKind, StoreError> {
    match value {
        1 => Ok(ReviewDispositionKind::Addressed),
        2 => Ok(ReviewDispositionKind::RejectedWithDissentPreserved),
        3 => Ok(ReviewDispositionKind::AcceptedRisk),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn review_resolution_kind_from_i64(value: i64) -> Result<ReviewResolutionKind, StoreError> {
    match value {
        1 => Ok(ReviewResolutionKind::Resolved),
        2 => Ok(ReviewResolutionKind::AcceptedRisk),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn postmortem_state_from_i64(value: i64) -> Result<PostmortemState, StoreError> {
    match value {
        1 => Ok(PostmortemState::Triggered),
        2 => Ok(PostmortemState::Investigating),
        3 => Ok(PostmortemState::Closed),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn postmortem_causal_claim_kind_from_i64(
    value: i64,
) -> Result<PostmortemCausalClaimKind, StoreError> {
    match value {
        1 => Ok(PostmortemCausalClaimKind::ContributingCondition),
        2 => Ok(PostmortemCausalClaimKind::Counterfactual),
        3 => Ok(PostmortemCausalClaimKind::Containment),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
fn postmortem_action_kind_from_i64(value: i64) -> Result<PostmortemActionKind, StoreError> {
    match value {
        1 => Ok(PostmortemActionKind::CreateFollowUpTicket),
        2 => Ok(PostmortemActionKind::ChangePolicyProposal),
        _ => Err(StoreError::InvalidStoredValue),
    }
}
