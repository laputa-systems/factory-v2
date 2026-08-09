-- V2 migration 3: actor identity, bounded work execution, and resolvable
-- review/closure blockers. The runner only permits this migration from an
-- empty M2 ledger: changing M2 command/event fingerprints without a proven
-- historical transform would be a false replay claim.

PRAGMA foreign_keys = OFF;
PRAGMA legacy_alter_table = ON;
BEGIN IMMEDIATE;

-- M1 fixed two treatments. M3 adds a third, provider-free deterministic
-- treatment without rewriting M1 history. Its runner accepts only an empty
-- M2 ledger, so this faithful table rebuild never claims ledger migration.
ALTER TABLE operating_cycles RENAME TO operating_cycles_m2;
CREATE TABLE operating_cycles (
    operating_cycle_id INTEGER PRIMARY KEY,
    society_id INTEGER NOT NULL REFERENCES societies(society_id),
    universe_seed_id INTEGER NOT NULL REFERENCES universe_seeds(universe_seed_id),
    office_occupancy_id INTEGER NOT NULL REFERENCES office_occupancies(office_occupancy_id),
    treatment INTEGER NOT NULL CHECK (treatment IN (1, 2, 3)),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 11),
    admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0),
    proposed_by_command_id INTEGER NOT NULL,
    last_transition_command_id INTEGER NOT NULL
);
INSERT INTO operating_cycles SELECT * FROM operating_cycles_m2;
DROP TABLE operating_cycles_m2;

ALTER TABLE command_propose_operating_cycle RENAME TO command_propose_operating_cycle_m2;
CREATE TABLE command_propose_operating_cycle (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    treatment INTEGER NOT NULL CHECK (treatment IN (1, 2, 3))
);
INSERT INTO command_propose_operating_cycle SELECT * FROM command_propose_operating_cycle_m2;
DROP TABLE command_propose_operating_cycle_m2;

ALTER TABLE event_operating_cycle_proposed RENAME TO event_operating_cycle_proposed_m2;
CREATE TABLE event_operating_cycle_proposed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0),
    treatment INTEGER NOT NULL CHECK (treatment IN (1, 2, 3))
);
INSERT INTO event_operating_cycle_proposed SELECT * FROM event_operating_cycle_proposed_m2;
DROP TABLE event_operating_cycle_proposed_m2;

CREATE TABLE execution_profiles (
    execution_profile_id INTEGER PRIMARY KEY,
    profile_kind INTEGER NOT NULL UNIQUE CHECK (profile_kind IN (1, 2)),
    readiness INTEGER NOT NULL CHECK (readiness IN (1, 2, 3)),
    CHECK ((profile_kind = 1 AND readiness = 1)
        OR (profile_kind = 2 AND readiness IN (2, 3)))
);
-- The deterministic process double is restricted to the provider-free
-- deterministic fixture treatment. It cannot emit a PiSdkQualification
-- receipt. The native identity is deliberately unqualified:
-- only a future typed PiSdkQualification may move it to readiness 3.
INSERT INTO execution_profiles(execution_profile_id, profile_kind, readiness)
VALUES (1, 1, 1),
       (2, 2, 2);

CREATE TABLE actor_configurations (
    actor_configuration_id INTEGER PRIMARY KEY,
    configuration_name TEXT NOT NULL UNIQUE,
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2)),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE actor_configuration_revisions (
    actor_configuration_revision_id INTEGER PRIMARY KEY,
    actor_configuration_id INTEGER NOT NULL REFERENCES actor_configurations(actor_configuration_id),
    revision_ordinal INTEGER NOT NULL CHECK (revision_ordinal > 0),
    model_policy INTEGER NOT NULL CHECK (model_policy = 1),
    primary_attractor INTEGER NOT NULL CHECK (primary_attractor BETWEEN 1 AND 8),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE(actor_configuration_id, revision_ordinal)
);

CREATE TABLE context_packs (
    context_pack_id INTEGER PRIMARY KEY,
    universe_seed_id INTEGER NOT NULL REFERENCES universe_seeds(universe_seed_id),
    purpose INTEGER NOT NULL CHECK (purpose IN (1, 2)),
    rendering_digest BLOB NOT NULL CHECK (length(rendering_digest) = 32),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE actor_instances (
    actor_instance_id INTEGER PRIMARY KEY,
    principal_id INTEGER NOT NULL UNIQUE REFERENCES principals(principal_id),
    actor_configuration_revision_id INTEGER NOT NULL REFERENCES actor_configuration_revisions(actor_configuration_revision_id),
    execution_profile_id INTEGER NOT NULL REFERENCES execution_profiles(execution_profile_id),
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2)),
    admitted_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE work_items (
    work_item_id INTEGER PRIMARY KEY,
    ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id),
    actor_instance_id INTEGER NOT NULL REFERENCES actor_instances(actor_instance_id),
    context_pack_id INTEGER NOT NULL REFERENCES context_packs(context_pack_id),
    work_kind INTEGER NOT NULL CHECK (work_kind IN (1, 2)),
    adversarial_review_id INTEGER REFERENCES adversarial_reviews(adversarial_review_id),
    assignment_text TEXT NOT NULL,
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 5),
    retry_of_actor_attempt_id INTEGER REFERENCES attempts(actor_attempt_id),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    last_transition_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    CHECK ((work_kind = 1 AND adversarial_review_id IS NULL)
        OR (work_kind = 2 AND adversarial_review_id IS NOT NULL))
);

CREATE TABLE leases (
    work_lease_id INTEGER PRIMARY KEY,
    work_item_id INTEGER NOT NULL REFERENCES work_items(work_item_id),
    actor_instance_id INTEGER NOT NULL REFERENCES actor_instances(actor_instance_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 4),
    claimed_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    terminal_by_command_id INTEGER REFERENCES commands(command_row_id)
);
CREATE UNIQUE INDEX one_active_lease_per_work_item
    ON leases(work_item_id) WHERE lifecycle_state = 1;

CREATE TABLE attempts (
    actor_attempt_id INTEGER PRIMARY KEY,
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id),
    work_item_id INTEGER NOT NULL REFERENCES work_items(work_item_id),
    work_lease_id INTEGER NOT NULL REFERENCES leases(work_lease_id),
    actor_instance_id INTEGER NOT NULL REFERENCES actor_instances(actor_instance_id),
    actor_configuration_revision_id INTEGER NOT NULL REFERENCES actor_configuration_revisions(actor_configuration_revision_id),
    execution_profile_id INTEGER NOT NULL REFERENCES execution_profiles(execution_profile_id),
    context_pack_id INTEGER NOT NULL REFERENCES context_packs(context_pack_id),
    retry_of_actor_attempt_id INTEGER REFERENCES attempts(actor_attempt_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 9),
    started_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    terminal_by_command_id INTEGER REFERENCES commands(command_row_id),
    validated_by_command_id INTEGER REFERENCES commands(command_row_id)
);
CREATE UNIQUE INDEX one_live_attempt_per_work_item
    ON attempts(work_item_id) WHERE lifecycle_state IN (1, 2);

CREATE TABLE attempt_budget_reservations (
    actor_attempt_id INTEGER PRIMARY KEY REFERENCES attempts(actor_attempt_id),
    budget_reservation_id INTEGER NOT NULL UNIQUE REFERENCES budget_reservations(budget_reservation_id),
    project_id INTEGER NOT NULL REFERENCES projects(project_id),
    ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id)
);

CREATE TABLE actor_attempt_terminal_facts (
    actor_attempt_id INTEGER PRIMARY KEY REFERENCES attempts(actor_attempt_id),
    terminal_kind INTEGER NOT NULL CHECK (terminal_kind BETWEEN 1 AND 6),
    attested_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE outcome_obligations (
    outcome_obligation_id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(project_id),
    obligation_text TEXT NOT NULL,
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2, 3)),
    scheduled_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    resolved_by_command_id INTEGER REFERENCES commands(command_row_id)
);

ALTER TABLE capability_grants RENAME TO capability_grants_m2;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_occupancy_on_insert;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_occupancy_on_update;
DROP TRIGGER IF EXISTS occupancy_principal_matches_existing_grants;
DROP INDEX IF EXISTS active_capability_grant_per_principal;
CREATE TABLE capability_grants (
    capability_grant_id INTEGER PRIMARY KEY,
    principal_id INTEGER NOT NULL REFERENCES principals(principal_id),
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 61),
    office_occupancy_id INTEGER REFERENCES office_occupancies(office_occupancy_id),
    actor_instance_id INTEGER REFERENCES actor_instances(actor_instance_id),
    grant_state INTEGER NOT NULL CHECK (grant_state IN (1, 2)),
    grant_origin INTEGER NOT NULL CHECK (grant_origin IN (1, 2, 3)),
    granted_by_command_id INTEGER,
    consumed_by_command_id INTEGER,
    CHECK (NOT (office_occupancy_id IS NOT NULL AND actor_instance_id IS NOT NULL)),
    CHECK ((grant_state = 1 AND consumed_by_command_id IS NULL)
        OR (grant_state = 2 AND consumed_by_command_id IS NOT NULL)),
    CHECK ((grant_origin = 1 AND principal_id = 1
            AND office_occupancy_id IS NULL AND actor_instance_id IS NULL
            AND granted_by_command_id IS NULL)
        OR (grant_origin = 2 AND granted_by_command_id IS NOT NULL)
        OR (grant_origin = 3 AND principal_id = 2
            AND office_occupancy_id IS NULL AND actor_instance_id IS NULL
            AND granted_by_command_id IS NULL))
);
INSERT INTO capability_grants(capability_grant_id, principal_id, capability_kind, office_occupancy_id, actor_instance_id, grant_state, grant_origin, granted_by_command_id, consumed_by_command_id)
SELECT capability_grant_id, principal_id, capability_kind, office_occupancy_id, NULL, grant_state,
       CASE WHEN granted_by_command_id IS NOT NULL THEN 2
            WHEN principal_id = 1 THEN 1
            WHEN principal_id = 2 THEN 3
            ELSE 0 END,
       granted_by_command_id, consumed_by_command_id
FROM capability_grants_m2;
DROP TABLE capability_grants_m2;
CREATE TRIGGER capability_grant_principal_matches_occupancy_on_insert
BEFORE INSERT ON capability_grants
WHEN NEW.office_occupancy_id IS NOT NULL AND NOT EXISTS (
 SELECT 1 FROM office_occupancies WHERE office_occupancy_id = NEW.office_occupancy_id AND principal_id = NEW.principal_id)
BEGIN SELECT RAISE(ABORT, 'capability grant principal must hold its occupancy'); END;
CREATE TRIGGER capability_grant_principal_matches_occupancy_on_update
BEFORE UPDATE OF principal_id, office_occupancy_id ON capability_grants
WHEN NEW.office_occupancy_id IS NOT NULL AND NOT EXISTS (
 SELECT 1 FROM office_occupancies WHERE office_occupancy_id = NEW.office_occupancy_id AND principal_id = NEW.principal_id)
BEGIN SELECT RAISE(ABORT, 'capability grant principal must hold its occupancy'); END;
CREATE TRIGGER occupancy_principal_matches_existing_grants
BEFORE UPDATE OF principal_id ON office_occupancies
WHEN EXISTS (SELECT 1 FROM capability_grants WHERE office_occupancy_id = NEW.office_occupancy_id AND principal_id != NEW.principal_id)
BEGIN SELECT RAISE(ABORT, 'occupancy principal must match existing grants'); END;
CREATE TRIGGER capability_grant_principal_matches_actor_instance_on_insert
BEFORE INSERT ON capability_grants
WHEN NEW.actor_instance_id IS NOT NULL AND NOT EXISTS (
 SELECT 1 FROM actor_instances WHERE actor_instance_id = NEW.actor_instance_id AND principal_id = NEW.principal_id)
BEGIN SELECT RAISE(ABORT, 'capability grant principal must be the actor instance principal'); END;
CREATE TRIGGER capability_grant_principal_matches_actor_instance_on_update
BEFORE UPDATE OF principal_id, actor_instance_id ON capability_grants
WHEN NEW.actor_instance_id IS NOT NULL AND NOT EXISTS (
 SELECT 1 FROM actor_instances WHERE actor_instance_id = NEW.actor_instance_id AND principal_id = NEW.principal_id)
BEGIN SELECT RAISE(ABORT, 'capability grant principal must be the actor instance principal'); END;
CREATE UNIQUE INDEX active_capability_grant_per_principal
 ON capability_grants(principal_id, capability_kind, COALESCE(office_occupancy_id, -1), COALESCE(actor_instance_id, -1)) WHERE grant_state = 1;

ALTER TABLE commands RENAME TO commands_m2;
CREATE TABLE commands (
    command_row_id INTEGER PRIMARY KEY,
    command_id TEXT NOT NULL UNIQUE,
    principal_id INTEGER NOT NULL,
    capability_grant_id INTEGER NOT NULL,
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 61),
    expected_generation INTEGER,
    command_kind INTEGER NOT NULL CHECK (command_kind BETWEEN 1 AND 61),
    request_fingerprint BLOB NOT NULL CHECK (length(request_fingerprint) = 32),
    command_status INTEGER NOT NULL CHECK (command_status IN (1, 2)),
    rejection_code INTEGER,
    accepted_event_id INTEGER,
    CHECK ((command_status = 1 AND rejection_code IS NULL AND accepted_event_id IS NOT NULL)
        OR (command_status = 2 AND rejection_code IS NOT NULL AND accepted_event_id IS NULL))
);
INSERT INTO commands SELECT * FROM commands_m2;
DROP TABLE commands_m2;

ALTER TABLE events RENAME TO events_m2;
CREATE TABLE events (
    event_id INTEGER PRIMARY KEY,
    command_row_id INTEGER NOT NULL UNIQUE REFERENCES commands(command_row_id),
    event_kind INTEGER NOT NULL CHECK (event_kind BETWEEN 1 AND 54),
    event_sequence INTEGER NOT NULL UNIQUE CHECK (event_sequence > 0),
    event_fingerprint BLOB NOT NULL CHECK (length(event_fingerprint) = 32)
);
INSERT INTO events SELECT * FROM events_m2;
DROP TABLE events_m2;

ALTER TABLE adversarial_reviews ADD COLUMN assigned_reviewer_actor_instance_id INTEGER REFERENCES actor_instances(actor_instance_id);
ALTER TABLE adversarial_reviews ADD COLUMN reviewer_actor_attempt_id INTEGER REFERENCES attempts(actor_attempt_id);
DROP TABLE command_assign_adversarial_reviewer;
CREATE TABLE command_assign_adversarial_reviewer (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    adversarial_review_id INTEGER NOT NULL,
    reviewer_principal_id INTEGER NOT NULL,
    reviewer_actor_instance_id INTEGER NOT NULL,
    reviewer_actor_attempt_id INTEGER NOT NULL
);
DROP TABLE event_adversarial_reviewer_assigned;
CREATE TABLE event_adversarial_reviewer_assigned (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    adversarial_review_id INTEGER NOT NULL REFERENCES adversarial_reviews(adversarial_review_id),
    reviewer_principal_id INTEGER NOT NULL REFERENCES principals(principal_id),
    reviewer_actor_instance_id INTEGER NOT NULL REFERENCES actor_instances(actor_instance_id),
    reviewer_actor_attempt_id INTEGER NOT NULL REFERENCES attempts(actor_attempt_id)
);

CREATE TABLE command_register_actor_configuration (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    configuration_name TEXT NOT NULL,
    model_policy INTEGER NOT NULL CHECK (model_policy = 1),
    primary_attractor INTEGER NOT NULL CHECK (primary_attractor BETWEEN 1 AND 8)
);
CREATE TABLE command_register_context_pack (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    purpose INTEGER NOT NULL CHECK (purpose IN (1, 2)),
    rendering_digest BLOB NOT NULL CHECK (length(rendering_digest) = 32)
);
CREATE TABLE command_admit_actor_instance (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    actor_configuration_revision_id INTEGER NOT NULL,
    execution_profile_id INTEGER NOT NULL,
    actor_display_name TEXT NOT NULL
);
CREATE TABLE command_admit_ticket (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    ticket_id INTEGER NOT NULL
);
CREATE TABLE command_register_work_item (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    ticket_id INTEGER NOT NULL,
    actor_instance_id INTEGER NOT NULL,
    context_pack_id INTEGER NOT NULL,
    work_kind INTEGER NOT NULL CHECK (work_kind IN (1, 2)),
    adversarial_review_id INTEGER,
    assignment_text TEXT NOT NULL
);
CREATE TABLE command_claim_work_item (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    work_item_id INTEGER NOT NULL
);
CREATE TABLE command_start_actor_attempt (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    work_item_id INTEGER NOT NULL,
    reservation_micros INTEGER NOT NULL CHECK (reservation_micros > 0)
);
CREATE TABLE command_attest_actor_attempt_terminal (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    actor_attempt_id INTEGER NOT NULL,
    terminal_kind INTEGER NOT NULL CHECK (terminal_kind BETWEEN 1 AND 6)
);
CREATE TABLE command_validate_ticket_attempt (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    actor_attempt_id INTEGER NOT NULL
);
CREATE TABLE command_retry_actor_attempt (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    actor_attempt_id INTEGER NOT NULL
);
CREATE TABLE command_complete_ticket (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    actor_attempt_id INTEGER NOT NULL
);
CREATE TABLE command_expire_work_lease (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    work_lease_id INTEGER NOT NULL
);
CREATE TABLE command_cancel_actor_attempt (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    actor_attempt_id INTEGER NOT NULL,
    cancellation_reason INTEGER NOT NULL CHECK (cancellation_reason BETWEEN 1 AND 3)
);
CREATE TABLE command_register_outcome_obligation (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    project_id INTEGER NOT NULL,
    obligation_text TEXT NOT NULL
);
CREATE TABLE command_resolve_outcome_obligation (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    outcome_obligation_id INTEGER NOT NULL,
    disposition INTEGER NOT NULL CHECK (disposition IN (1, 2))
);

CREATE TABLE event_actor_configuration_registered (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    actor_configuration_id INTEGER NOT NULL REFERENCES actor_configurations(actor_configuration_id),
    actor_configuration_revision_id INTEGER NOT NULL REFERENCES actor_configuration_revisions(actor_configuration_revision_id)
);
CREATE TABLE event_context_pack_registered (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    context_pack_id INTEGER NOT NULL REFERENCES context_packs(context_pack_id)
);
CREATE TABLE event_actor_instance_admitted (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    actor_instance_id INTEGER NOT NULL REFERENCES actor_instances(actor_instance_id),
    principal_id INTEGER NOT NULL REFERENCES principals(principal_id)
);
CREATE TABLE event_ticket_admitted (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id)
);
CREATE TABLE event_work_item_registered (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    work_item_id INTEGER NOT NULL REFERENCES work_items(work_item_id),
    ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id),
    adversarial_review_id INTEGER REFERENCES adversarial_reviews(adversarial_review_id)
);
CREATE TABLE event_work_item_claimed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    work_item_id INTEGER NOT NULL REFERENCES work_items(work_item_id),
    work_lease_id INTEGER NOT NULL REFERENCES leases(work_lease_id),
    actor_instance_id INTEGER NOT NULL REFERENCES actor_instances(actor_instance_id)
);
CREATE TABLE event_actor_attempt_started (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    actor_attempt_id INTEGER NOT NULL REFERENCES attempts(actor_attempt_id),
    work_item_id INTEGER NOT NULL REFERENCES work_items(work_item_id),
    budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id)
);
CREATE TABLE event_actor_attempt_terminal_attested (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    actor_attempt_id INTEGER NOT NULL REFERENCES attempts(actor_attempt_id),
    terminal_kind INTEGER NOT NULL CHECK (terminal_kind BETWEEN 1 AND 6)
);
CREATE TABLE event_ticket_attempt_validated (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    actor_attempt_id INTEGER NOT NULL REFERENCES attempts(actor_attempt_id),
    ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id)
);
CREATE TABLE event_actor_attempt_retry_prepared (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    actor_attempt_id INTEGER NOT NULL REFERENCES attempts(actor_attempt_id),
    work_item_id INTEGER NOT NULL REFERENCES work_items(work_item_id),
    ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id)
);
CREATE TABLE event_ticket_completed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id),
    actor_attempt_id INTEGER NOT NULL REFERENCES attempts(actor_attempt_id)
);
CREATE TABLE event_work_lease_expired (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    work_lease_id INTEGER NOT NULL REFERENCES leases(work_lease_id),
    work_item_id INTEGER NOT NULL REFERENCES work_items(work_item_id)
);
CREATE TABLE event_actor_attempt_cancellation_requested (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    actor_attempt_id INTEGER NOT NULL REFERENCES attempts(actor_attempt_id),
    cancellation_reason INTEGER NOT NULL CHECK (cancellation_reason BETWEEN 1 AND 3)
);
CREATE TABLE event_outcome_obligation_registered (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    outcome_obligation_id INTEGER NOT NULL REFERENCES outcome_obligations(outcome_obligation_id),
    project_id INTEGER NOT NULL REFERENCES projects(project_id)
);
CREATE TABLE event_outcome_obligation_resolved (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    outcome_obligation_id INTEGER NOT NULL REFERENCES outcome_obligations(outcome_obligation_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (2, 3))
);

INSERT OR IGNORE INTO capability_grants(principal_id, capability_kind, office_occupancy_id, actor_instance_id, grant_state, grant_origin, granted_by_command_id, consumed_by_command_id)
VALUES (2, 54, NULL, NULL, 1, 3, NULL, NULL),
       (2, 55, NULL, NULL, 1, 3, NULL, NULL),
       (2, 58, NULL, NULL, 1, 3, NULL, NULL),
       (2, 59, NULL, NULL, 1, 3, NULL, NULL);

PRAGMA user_version = 3;
COMMIT;
PRAGMA foreign_keys = ON;
