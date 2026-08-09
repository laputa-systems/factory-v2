-- V2 migration 1: founding constitutional state and finite operational control.
-- This schema deliberately has no JSON column, opaque payload, metadata map,
-- or entity-attribute-value escape hatch. Every discriminant below is a
-- closed Rust enum whose integer representation is checked at the boundary.

PRAGMA foreign_keys = ON;

CREATE TABLE principals (
    principal_id INTEGER PRIMARY KEY,
    principal_kind INTEGER NOT NULL CHECK (principal_kind IN (1, 2, 3, 4)),
    display_name TEXT NOT NULL UNIQUE,
    active INTEGER NOT NULL CHECK (active IN (0, 1))
);

CREATE TABLE societies (
    society_id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2))
);

CREATE TABLE office_contracts (
    office_id INTEGER PRIMARY KEY,
    office_kind INTEGER NOT NULL UNIQUE CHECK (office_kind = 1),
    installed_by_command_id INTEGER NOT NULL
);

CREATE TABLE universe_seeds (
    universe_seed_id INTEGER PRIMARY KEY,
    society_id INTEGER NOT NULL REFERENCES societies(society_id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    rendering_digest BLOB NOT NULL CHECK (length(rendering_digest) = 32),
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    installed_by_command_id INTEGER NOT NULL,
    UNIQUE (society_id, revision)
);

CREATE UNIQUE INDEX one_active_universe_seed_per_society
    ON universe_seeds(society_id) WHERE active = 1;

CREATE TABLE office_occupancies (
    office_occupancy_id INTEGER PRIMARY KEY,
    office_id INTEGER NOT NULL REFERENCES office_contracts(office_id),
    principal_id INTEGER NOT NULL REFERENCES principals(principal_id),
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    appointed_by_command_id INTEGER NOT NULL
);

CREATE UNIQUE INDEX one_active_occupancy_per_office
    ON office_occupancies(office_id) WHERE active = 1;

CREATE TABLE capability_grants (
    capability_grant_id INTEGER PRIMARY KEY,
    principal_id INTEGER NOT NULL REFERENCES principals(principal_id),
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 23),
    office_occupancy_id INTEGER REFERENCES office_occupancies(office_occupancy_id),
    grant_state INTEGER NOT NULL CHECK (grant_state IN (1, 2)),
    granted_by_command_id INTEGER,
    consumed_by_command_id INTEGER,
    CHECK (
        (grant_state = 1 AND consumed_by_command_id IS NULL)
        OR (grant_state = 2 AND consumed_by_command_id IS NOT NULL)
    )
);

-- Office-scoped authority is tied to the office holder, not merely an
-- occupancy number. Runtime authorization repeats this join because an
-- operator with raw database access can remove schema defenses.
CREATE TRIGGER capability_grant_principal_matches_occupancy_on_insert
BEFORE INSERT ON capability_grants
WHEN NEW.office_occupancy_id IS NOT NULL
 AND NOT EXISTS (
     SELECT 1 FROM office_occupancies
     WHERE office_occupancy_id = NEW.office_occupancy_id
       AND principal_id = NEW.principal_id
 )
BEGIN
    SELECT RAISE(ABORT, 'capability grant principal must hold its occupancy');
END;

CREATE TRIGGER capability_grant_principal_matches_occupancy_on_update
BEFORE UPDATE OF principal_id, office_occupancy_id ON capability_grants
WHEN NEW.office_occupancy_id IS NOT NULL
 AND NOT EXISTS (
     SELECT 1 FROM office_occupancies
     WHERE office_occupancy_id = NEW.office_occupancy_id
       AND principal_id = NEW.principal_id
 )
BEGIN
    SELECT RAISE(ABORT, 'capability grant principal must hold its occupancy');
END;

CREATE TRIGGER occupancy_principal_matches_existing_grants
BEFORE UPDATE OF principal_id ON office_occupancies
WHEN EXISTS (
    SELECT 1 FROM capability_grants
    WHERE office_occupancy_id = NEW.office_occupancy_id
      AND principal_id != NEW.principal_id
)
BEGIN
    SELECT RAISE(ABORT, 'occupancy principal must match existing grants');
END;

CREATE UNIQUE INDEX active_capability_grant_per_principal
    ON capability_grants(principal_id, capability_kind, COALESCE(office_occupancy_id, -1))
    WHERE grant_state = 1;

CREATE TABLE society_bootstraps (
    society_id INTEGER PRIMARY KEY REFERENCES societies(society_id),
    universe_seed_id INTEGER NOT NULL REFERENCES universe_seeds(universe_seed_id),
    office_id INTEGER NOT NULL REFERENCES office_contracts(office_id),
    office_occupancy_id INTEGER NOT NULL REFERENCES office_occupancies(office_occupancy_id),
    hard_ceiling_micros INTEGER NOT NULL CHECK (hard_ceiling_micros >= 0),
    bootstrapped_by_command_id INTEGER NOT NULL
);

CREATE TABLE operating_cycles (
    operating_cycle_id INTEGER PRIMARY KEY,
    society_id INTEGER NOT NULL REFERENCES societies(society_id),
    universe_seed_id INTEGER NOT NULL REFERENCES universe_seeds(universe_seed_id),
    office_occupancy_id INTEGER NOT NULL REFERENCES office_occupancies(office_occupancy_id),
    treatment INTEGER NOT NULL CHECK (treatment IN (1, 2)),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 11),
    admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0),
    proposed_by_command_id INTEGER NOT NULL,
    last_transition_command_id INTEGER NOT NULL
);

CREATE UNIQUE INDEX one_nonterminal_operating_cycle_per_society
    ON operating_cycles(society_id)
    WHERE lifecycle_state NOT IN (7, 10, 11);

CREATE TABLE operating_cycle_admissions (
    operating_cycle_id INTEGER PRIMARY KEY REFERENCES operating_cycles(operating_cycle_id),
    admitted_by_command_id INTEGER NOT NULL,
    started_by_command_id INTEGER
);

CREATE TABLE operating_cycle_reconciliations (
    operating_cycle_id INTEGER PRIMARY KEY REFERENCES operating_cycles(operating_cycle_id),
    reconciliation_started_by_command_id INTEGER NOT NULL,
    closed_by_command_id INTEGER
);

CREATE TABLE grand_architect_office_sessions (
    grand_architect_office_session_id INTEGER PRIMARY KEY,
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    office_occupancy_id INTEGER NOT NULL REFERENCES office_occupancies(office_occupancy_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 11),
    started_by_command_id INTEGER NOT NULL,
    last_transition_command_id INTEGER NOT NULL
);

CREATE UNIQUE INDEX one_live_office_session_per_cycle
    ON grand_architect_office_sessions(operating_cycle_id)
    WHERE lifecycle_state NOT IN (8, 10, 11);

CREATE TABLE office_turns (
    office_turn_id INTEGER PRIMARY KEY,
    grand_architect_office_session_id INTEGER NOT NULL
        REFERENCES grand_architect_office_sessions(grand_architect_office_session_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 4),
    purpose INTEGER NOT NULL CHECK (purpose BETWEEN 1 AND 4),
    opened_by_command_id INTEGER NOT NULL,
    settled_by_command_id INTEGER
);

CREATE UNIQUE INDEX one_active_office_turn_per_session
    ON office_turns(grand_architect_office_session_id)
    WHERE lifecycle_state = 1;

CREATE TABLE budget_envelopes (
    budget_envelope_id INTEGER PRIMARY KEY,
    ceiling_micros INTEGER NOT NULL CHECK (ceiling_micros >= 0),
    reserved_micros INTEGER NOT NULL CHECK (reserved_micros >= 0),
    spent_micros INTEGER NOT NULL CHECK (spent_micros >= 0),
    created_by_command_id INTEGER NOT NULL,
    -- Admission enforces reserved + spent before paid work. A known provider
    -- overrun is later recorded at its actual amount, which may exceed the
    -- ceiling, so only live reservations are constrained in storage.
    CHECK (reserved_micros <= ceiling_micros)
);

CREATE TABLE budget_envelope_constraints (
    budget_envelope_constraint_id INTEGER PRIMARY KEY,
    budget_envelope_id INTEGER NOT NULL UNIQUE REFERENCES budget_envelopes(budget_envelope_id),
    society_id INTEGER REFERENCES societies(society_id),
    operating_cycle_id INTEGER REFERENCES operating_cycles(operating_cycle_id),
    CHECK (
        (society_id IS NOT NULL AND operating_cycle_id IS NULL)
        OR (society_id IS NULL AND operating_cycle_id IS NOT NULL)
    ),
    UNIQUE (society_id),
    UNIQUE (operating_cycle_id)
);

CREATE TABLE budget_reservations (
    budget_reservation_id INTEGER PRIMARY KEY,
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    amount_micros INTEGER NOT NULL CHECK (amount_micros >= 0),
    reservation_state INTEGER NOT NULL CHECK (reservation_state IN (1, 2, 3)),
    reserved_by_command_id INTEGER NOT NULL,
    reconciled_by_command_id INTEGER
);

CREATE TABLE budget_reservation_charges (
    budget_reservation_charge_id INTEGER PRIMARY KEY,
    budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id),
    budget_envelope_id INTEGER NOT NULL REFERENCES budget_envelopes(budget_envelope_id),
    amount_micros INTEGER NOT NULL CHECK (amount_micros >= 0),
    UNIQUE (budget_reservation_id, budget_envelope_id)
);

CREATE TABLE cancellation_requests (
    cancellation_request_id INTEGER PRIMARY KEY,
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    cancellation_mode INTEGER NOT NULL CHECK (cancellation_mode IN (1, 2, 3)),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 9),
    observed_admission_generation INTEGER NOT NULL CHECK (observed_admission_generation >= 0),
    requested_by_command_id INTEGER NOT NULL,
    reconciled_by_command_id INTEGER
);

CREATE UNIQUE INDEX one_active_cancellation_per_cycle
    ON cancellation_requests(operating_cycle_id)
    WHERE lifecycle_state NOT IN (8, 9);

CREATE TABLE cost_postmortems (
    postmortem_id INTEGER PRIMARY KEY,
    budget_reservation_id INTEGER NOT NULL UNIQUE REFERENCES budget_reservations(budget_reservation_id),
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id),
    cause_kind INTEGER NOT NULL CHECK (cause_kind IN (1, 2, 3)),
    observed_micros INTEGER CHECK (observed_micros >= 0),
    reserved_micros INTEGER CHECK (reserved_micros >= 0),
    unknown_reason INTEGER,
    unavailable_reason INTEGER,
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2)),
    opened_by_command_id INTEGER NOT NULL,
    closed_by_command_id INTEGER,
    CHECK (
        (cause_kind = 1 AND observed_micros IS NOT NULL AND reserved_micros IS NOT NULL AND unknown_reason IS NULL AND unavailable_reason IS NULL)
        OR (cause_kind = 2 AND observed_micros IS NULL AND reserved_micros IS NOT NULL AND unknown_reason IS NOT NULL AND unavailable_reason IS NULL)
        OR (cause_kind = 3 AND observed_micros IS NULL AND reserved_micros IS NOT NULL AND unknown_reason IS NULL AND unavailable_reason IS NOT NULL)
    ),
    CHECK (
        (lifecycle_state = 1 AND closed_by_command_id IS NULL)
        OR (lifecycle_state = 2 AND closed_by_command_id IS NOT NULL)
    )
);

CREATE TABLE cost_postmortem_resolutions (
    postmortem_id INTEGER PRIMARY KEY REFERENCES cost_postmortems(postmortem_id),
    resolution_kind INTEGER NOT NULL CHECK (resolution_kind IN (1, 2)),
    charged_micros INTEGER NOT NULL CHECK (charged_micros >= 0),
    resolved_by_command_id INTEGER NOT NULL
);

CREATE TABLE commands (
    command_row_id INTEGER PRIMARY KEY,
    command_id TEXT NOT NULL UNIQUE,
    -- Input identity is intentionally not a foreign key: an unauthenticated
    -- principal attempt must remain a durable rejected command.
    principal_id INTEGER NOT NULL,
    capability_grant_id INTEGER NOT NULL,
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 23),
    expected_generation INTEGER,
    command_kind INTEGER NOT NULL CHECK (command_kind BETWEEN 1 AND 23),
    request_fingerprint BLOB NOT NULL CHECK (length(request_fingerprint) = 32),
    command_status INTEGER NOT NULL CHECK (command_status IN (1, 2)),
    rejection_code INTEGER,
    accepted_event_id INTEGER,
    CHECK (
        (command_status = 1 AND rejection_code IS NULL AND accepted_event_id IS NOT NULL)
        OR (command_status = 2 AND rejection_code IS NOT NULL AND accepted_event_id IS NULL)
    )
);

CREATE TABLE events (
    event_id INTEGER PRIMARY KEY,
    command_row_id INTEGER NOT NULL UNIQUE REFERENCES commands(command_row_id),
    event_kind INTEGER NOT NULL CHECK (event_kind BETWEEN 1 AND 18),
    event_sequence INTEGER NOT NULL UNIQUE CHECK (event_sequence > 0),
    event_fingerprint BLOB NOT NULL CHECK (length(event_fingerprint) = 32)
);

-- One table per closed CommandBody variant. A Rust transaction inserts exactly
-- one; replay verifies that no command row has a missing or mismatched body.
CREATE TABLE command_create_society_identity (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), name TEXT NOT NULL);
CREATE TABLE command_install_grand_architect_office (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id));
CREATE TABLE command_install_founding_universe_seed (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), rendering_digest BLOB NOT NULL CHECK (length(rendering_digest) = 32));
CREATE TABLE command_appoint_initial_grand_architect (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), actor_display_name TEXT NOT NULL);
CREATE TABLE command_set_r0_hard_ceiling (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), ceiling_micros INTEGER NOT NULL CHECK (ceiling_micros >= 0));
CREATE TABLE command_bootstrap_society (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id));
CREATE TABLE command_propose_operating_cycle (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), treatment INTEGER NOT NULL CHECK (treatment IN (1, 2)));
-- Command bodies retain the caller's typed input even when it names an absent
-- subject. That is why their subject IDs are not foreign keys; authority and
-- existence are validated only by the command transition, and a rejected
-- command remains inspectable without manufacturing a subject row.
CREATE TABLE command_admit_operating_cycle (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL);
CREATE TABLE command_start_grand_architect_office_session (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL);
CREATE TABLE command_record_office_session_ready (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), grand_architect_office_session_id INTEGER NOT NULL);
CREATE TABLE command_record_office_session_terminal (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), grand_architect_office_session_id INTEGER NOT NULL, terminal_state INTEGER NOT NULL CHECK (terminal_state IN (1, 2, 3)));
CREATE TABLE command_open_office_turn (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), grand_architect_office_session_id INTEGER NOT NULL, purpose INTEGER NOT NULL CHECK (purpose BETWEEN 1 AND 4));
CREATE TABLE command_settle_office_turn (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), office_turn_id INTEGER NOT NULL);
CREATE TABLE command_quiesce_operating_cycle (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL);
CREATE TABLE command_record_cycle_drained (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL);
CREATE TABLE command_resume_operating_cycle (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL);
CREATE TABLE command_reconcile_operating_cycle (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL);
CREATE TABLE command_close_operating_cycle (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL);
CREATE TABLE command_reserve_budget (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, amount_micros INTEGER NOT NULL CHECK (amount_micros >= 0));
CREATE TABLE command_reconcile_budget (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), budget_reservation_id INTEGER NOT NULL, observation_kind INTEGER NOT NULL CHECK (observation_kind IN (1, 2, 3)), known_micros INTEGER CHECK (known_micros >= 0), unknown_reason INTEGER, unavailable_reason INTEGER, CHECK ((observation_kind = 1 AND known_micros IS NOT NULL AND unknown_reason IS NULL AND unavailable_reason IS NULL) OR (observation_kind = 2 AND known_micros IS NULL AND unknown_reason IS NOT NULL AND unavailable_reason IS NULL) OR (observation_kind = 3 AND known_micros IS NULL AND unknown_reason IS NULL AND unavailable_reason IS NOT NULL)));
CREATE TABLE command_request_cancellation (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, cancellation_mode INTEGER NOT NULL CHECK (cancellation_mode IN (1, 2, 3)));
CREATE TABLE command_reconcile_cancellation (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), cancellation_request_id INTEGER NOT NULL);
CREATE TABLE command_close_cost_postmortem (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), postmortem_id INTEGER NOT NULL, resolution_kind INTEGER NOT NULL CHECK (resolution_kind IN (1, 2)));

-- One table per closed EventBody variant. These bodies make ledger replay
-- independent of generic blobs and detect a schema/body mismatch as corruption.
CREATE TABLE event_society_identity_created (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), society_id INTEGER NOT NULL REFERENCES societies(society_id));
CREATE TABLE event_grand_architect_office_installed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), office_id INTEGER NOT NULL REFERENCES office_contracts(office_id));
CREATE TABLE event_founding_universe_seed_installed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), universe_seed_id INTEGER NOT NULL REFERENCES universe_seeds(universe_seed_id));
CREATE TABLE event_grand_architect_appointed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), office_occupancy_id INTEGER NOT NULL REFERENCES office_occupancies(office_occupancy_id), principal_id INTEGER NOT NULL REFERENCES principals(principal_id));
CREATE TABLE event_r0_hard_ceiling_set (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), society_id INTEGER NOT NULL REFERENCES societies(society_id), ceiling_micros INTEGER NOT NULL CHECK (ceiling_micros >= 0));
CREATE TABLE event_society_bootstrapped (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), society_id INTEGER NOT NULL REFERENCES societies(society_id));
CREATE TABLE event_operating_cycle_proposed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id), admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0), treatment INTEGER NOT NULL CHECK (treatment IN (1, 2)));
CREATE TABLE event_operating_cycle_state_changed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id), lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 11), admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0));
CREATE TABLE event_grand_architect_office_session_started (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), grand_architect_office_session_id INTEGER NOT NULL REFERENCES grand_architect_office_sessions(grand_architect_office_session_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id));
CREATE TABLE event_grand_architect_office_session_state_changed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), grand_architect_office_session_id INTEGER NOT NULL REFERENCES grand_architect_office_sessions(grand_architect_office_session_id), lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 11));
CREATE TABLE event_office_turn_opened (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), office_turn_id INTEGER NOT NULL REFERENCES office_turns(office_turn_id), grand_architect_office_session_id INTEGER NOT NULL REFERENCES grand_architect_office_sessions(grand_architect_office_session_id), purpose INTEGER NOT NULL CHECK (purpose BETWEEN 1 AND 4));
CREATE TABLE event_office_turn_settled (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), office_turn_id INTEGER NOT NULL REFERENCES office_turns(office_turn_id), grand_architect_office_session_id INTEGER NOT NULL REFERENCES grand_architect_office_sessions(grand_architect_office_session_id));
CREATE TABLE event_budget_reserved (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id), amount_micros INTEGER NOT NULL CHECK (amount_micros >= 0));
CREATE TABLE event_budget_reconciled (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id), observed_micros INTEGER NOT NULL CHECK (observed_micros >= 0));
CREATE TABLE event_budget_admission_frozen (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id), cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id), postmortem_id INTEGER NOT NULL REFERENCES cost_postmortems(postmortem_id), freeze_reason_kind INTEGER NOT NULL CHECK (freeze_reason_kind IN (1, 2, 3)), observed_micros INTEGER CHECK (observed_micros >= 0), reserved_micros INTEGER CHECK (reserved_micros >= 0), unknown_reason INTEGER, unavailable_reason INTEGER, CHECK ((freeze_reason_kind = 1 AND observed_micros IS NOT NULL AND reserved_micros IS NOT NULL AND unknown_reason IS NULL AND unavailable_reason IS NULL) OR (freeze_reason_kind = 2 AND observed_micros IS NULL AND reserved_micros IS NULL AND unknown_reason IS NOT NULL AND unavailable_reason IS NULL) OR (freeze_reason_kind = 3 AND observed_micros IS NULL AND reserved_micros IS NULL AND unknown_reason IS NULL AND unavailable_reason IS NOT NULL)));
CREATE TABLE event_cancellation_requested (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id), cancellation_mode INTEGER NOT NULL CHECK (cancellation_mode IN (1, 2, 3)), admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0));
CREATE TABLE event_cancellation_reconciled (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id));
CREATE TABLE event_cost_postmortem_closed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), postmortem_id INTEGER NOT NULL REFERENCES cost_postmortems(postmortem_id), budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id), resolution_kind INTEGER NOT NULL CHECK (resolution_kind IN (1, 2)), charged_micros INTEGER NOT NULL CHECK (charged_micros >= 0));

-- The founding root is compiled into the local store. It is a one-time,
-- consumable authority rather than an ambient process setting.
INSERT INTO principals(principal_id, principal_kind, display_name, active)
VALUES (1, 1, 'bootstrap_principal', 1), (2, 2, 'kernel_service', 1);

INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id, grant_state, granted_by_command_id, consumed_by_command_id)
VALUES
    (1, 1, NULL, 1, NULL, NULL), (1, 2, NULL, 1, NULL, NULL),
    (1, 3, NULL, 1, NULL, NULL), (1, 4, NULL, 1, NULL, NULL),
    (1, 5, NULL, 1, NULL, NULL), (1, 6, NULL, 1, NULL, NULL),
    (1, 7, NULL, 1, NULL, NULL), (1, 8, NULL, 1, NULL, NULL),
    (2, 17, NULL, 1, NULL, NULL), (2, 18, NULL, 1, NULL, NULL),
    (2, 19, NULL, 1, NULL, NULL), (2, 20, NULL, 1, NULL, NULL),
    (2, 21, NULL, 1, NULL, NULL), (2, 22, NULL, 1, NULL, NULL);

PRAGMA user_version = 1;
