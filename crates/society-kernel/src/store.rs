use std::path::Path;

use rusqlite::{
    Connection, OptionalExtension, Transaction, TransactionBehavior, params,
    types::{FromSql, ValueRef},
};
use thiserror::Error;

use crate::{
    AdmissionGeneration, BudgetEnvelopeId, BudgetFreezeReason, BudgetReservationId,
    BudgetReservationState, CancellationMode, CancellationRequestId, CancellationState, Capability,
    CommandBody, CommandDisposition, CommandId, CommandKind, CommandReceipt, CommandRequest,
    CostObservation, CostPostmortemCause, CostPostmortemResolution, CostUnavailableReason,
    CostUnknownReason, EventBody, EventId, EventKind, ExpectedGeneration,
    GrandArchitectOfficeSessionId, LedgerEvent, OfficeId, OfficeKind, OfficeOccupancyId,
    OfficeSessionState, OfficeSessionTerminalState, OfficeTurnId, OfficeTurnPurpose,
    OfficeTurnState, OperatingCycleId, OperatingCycleState, OperatingCycleTreatment, PostmortemId,
    PostmortemState, PrincipalId, PrincipalKind, Rejection, Sha256Digest, SocietyId, SocietyName,
    UniverseSeedId, UsdMicros,
};

const MIGRATION_1: &str = include_str!("../../../migrations/0001_kernel.sql");

const COMMAND_BODY_TABLES: [&str; 23] = [
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
];

const EVENT_BODY_TABLES: [&str; 18] = [
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
];

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
    _seed_id: UniverseSeedId,
    occupancy_id: OfficeOccupancyId,
    _treatment: OperatingCycleTreatment,
    state: OperatingCycleState,
    generation: AdmissionGeneration,
}

enum CapabilityGrantLookup {
    Active {
        grant_id: i64,
        office_occupancy_id: Option<OfficeOccupancyId>,
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
            0 => connection.execute_batch(MIGRATION_1)?,
            1 => {}
            other => return Err(StoreError::UnsupportedSchemaVersion(other)),
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
    ) != matches!(request.expected_generation, ExpectedGeneration::Exact(_))
    {
        return Ok(Err(Rejection::InvalidExpectedGeneration));
    }
    let (grant_id, office_occupancy_id) = match capability_grant(
        transaction,
        request.principal_id,
        request.capability,
        request.capability_grant_id,
    )? {
        Some(CapabilityGrantLookup::Active {
            grant_id,
            office_occupancy_id,
        }) => (grant_id, office_occupancy_id),
        Some(CapabilityGrantLookup::Inactive) => {
            return Ok(Err(Rejection::CapabilityNoLongerActive));
        }
        None => return Ok(Err(Rejection::CapabilityNotGranted)),
    };
    if request.principal_id != PrincipalId::BOOTSTRAP
        && request.principal_id != PrincipalId::KERNEL
        && !grant_has_active_occupancy(transaction, grant_id)?
    {
        return Ok(Err(Rejection::CapabilityNoLongerActive));
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
                                                grant_state, granted_by_command_id, consumed_by_command_id)
                 VALUES (?1, ?2, ?3, 1, ?4, NULL)",
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
            PostmortemState::Open as i64,
            command_row_id,
        ],
    ).map_err(|_| Rejection::InvalidLifecycleTransition)?;
    Ok(EventBody::BudgetAdmissionFrozen {
        reservation_id,
        cycle_id,
        cancellation_request_id,
        postmortem_id: id_from_last_insert::<PostmortemId>(transaction)?,
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
    postmortem_id: PostmortemId,
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
    if state != PostmortemState::Open as i64 {
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
                PostmortemState::Closed as i64,
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

fn capability_grant(
    transaction: &Transaction<'_>,
    principal_id: PrincipalId,
    capability: Capability,
    capability_grant_id: crate::CapabilityGrantId,
) -> Result<Option<CapabilityGrantLookup>, StoreError> {
    let grant = transaction
        .query_row(
            "SELECT grant_state, office_occupancy_id FROM capability_grants
             WHERE capability_grant_id = ?1 AND principal_id = ?2 AND capability_kind = ?3",
            params![
                capability_grant_id.value(),
                principal_id.value(),
                capability as i64
            ],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
        )
        .optional()?;
    match grant {
        Some((1, office_occupancy_id)) => Ok(Some(CapabilityGrantLookup::Active {
            grant_id: capability_grant_id.value(),
            office_occupancy_id: office_occupancy_id
                .map(OfficeOccupancyId::try_from)
                .transpose()
                .map_err(|_| StoreError::InvalidStoredValue)?,
        })),
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
        // The pinned seed is queried with the cycle so malformed storage is
        // detected before a lifecycle transition, even though this first
        // kernel tranche does not otherwise need to return it to the caller.
        _seed_id: UniverseSeedId::try_from(row.1).map_err(|_| Rejection::SubjectNotFound)?,
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
    }
    Sha256Digest::of_bytes(&bytes)
}

fn put_i64(bytes: &mut Vec<u8>, value: i64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn put_bytes(bytes: &mut Vec<u8>, value: &[u8]) {
    put_i64(bytes, value.len() as i64);
    bytes.extend_from_slice(value);
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
                postmortem_id: PostmortemId::try_from(postmortem)
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
                postmortem_id: PostmortemId::try_from(postmortem)
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
         FROM commands
         ORDER BY command_row_id ASC",
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

const MATERIALIZED_TABLES: [&str; 19] = [
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
                postmortem_id: PostmortemId::try_from(postmortem_id)
                    .map_err(|_| StoreError::InvalidStoredValue)?,
                resolution: cost_postmortem_resolution_from_i64(resolution)?,
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
