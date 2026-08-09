-- V2 migration 5: durable native child/process/cancellation receipt foundation.
--
-- This is trusted process physics, not a Pi adapter implementation.  It
-- records only typed authority and observed receipts.  It never stores streams
-- themselves, claims a wait(2) after restart absence, or treats content
-- sealing as forensic/evaluator evidence.
--
-- The store refuses a nonempty v4 ledger before this migration because the
-- command/event fingerprint domains widen.  This script is therefore an
-- atomic empty-schema version step, not a historical ledger transformation.

PRAGMA foreign_keys = OFF;
PRAGMA legacy_alter_table = ON;
BEGIN IMMEDIATE;

ALTER TABLE capability_grants RENAME TO capability_grants_m4;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_occupancy_on_insert;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_occupancy_on_update;
DROP TRIGGER IF EXISTS occupancy_principal_matches_existing_grants;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_actor_instance_on_insert;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_actor_instance_on_update;
DROP INDEX IF EXISTS active_capability_grant_per_principal;
CREATE TABLE capability_grants (
    capability_grant_id INTEGER PRIMARY KEY,
    principal_id INTEGER NOT NULL REFERENCES principals(principal_id),
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 85),
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
INSERT INTO capability_grants SELECT * FROM capability_grants_m4;
DROP TABLE capability_grants_m4;
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

ALTER TABLE commands RENAME TO commands_m4;
CREATE TABLE commands (
    command_row_id INTEGER PRIMARY KEY,
    command_id TEXT NOT NULL UNIQUE,
    principal_id INTEGER NOT NULL,
    capability_grant_id INTEGER NOT NULL,
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 85),
    expected_generation INTEGER,
    command_kind INTEGER NOT NULL CHECK (command_kind BETWEEN 1 AND 85),
    request_fingerprint BLOB NOT NULL CHECK (length(request_fingerprint) = 32),
    command_status INTEGER NOT NULL CHECK (command_status IN (1, 2)),
    rejection_code INTEGER,
    accepted_event_id INTEGER,
    CHECK ((command_status = 1 AND rejection_code IS NULL AND accepted_event_id IS NOT NULL)
        OR (command_status = 2 AND rejection_code IS NOT NULL AND accepted_event_id IS NULL))
);
INSERT INTO commands SELECT * FROM commands_m4;
DROP TABLE commands_m4;

ALTER TABLE events RENAME TO events_m4;
CREATE TABLE events (
    event_id INTEGER PRIMARY KEY,
    command_row_id INTEGER NOT NULL UNIQUE REFERENCES commands(command_row_id),
    event_kind INTEGER NOT NULL CHECK (event_kind BETWEEN 1 AND 79),
    event_sequence INTEGER NOT NULL UNIQUE CHECK (event_sequence > 0),
    event_fingerprint BLOB NOT NULL CHECK (length(event_fingerprint) = 32)
);
INSERT INTO events SELECT * FROM events_m4;
DROP TABLE events_m4;

CREATE TABLE supervisor_epochs (
    supervisor_epoch_id INTEGER PRIMARY KEY,
    supervisor_epoch_identity TEXT NOT NULL UNIQUE,
    opened_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TRIGGER only_one_supervisor_epoch
BEFORE INSERT ON supervisor_epochs
WHEN EXISTS (SELECT 1 FROM supervisor_epochs)
BEGIN SELECT RAISE(ABORT, 'M5 has exactly one restart-fenced supervisor epoch'); END;
-- An epoch groups child receipts from one resident supervisor lifetime. M5
-- deliberately proves only command/event sequence ordering, never a monotonic
-- clock tick, deadline, or elapsed-time escalation claim.
CREATE TABLE workspaces (
    workspace_id INTEGER PRIMARY KEY,
    native_workspace_id TEXT NOT NULL UNIQUE,
    canonical_workspace_path TEXT NOT NULL UNIQUE,
    registered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE pi_child_sessions (
    pi_session_id INTEGER PRIMARY KEY,
    pi_session_identity TEXT NOT NULL UNIQUE,
    spawn_nonce TEXT NOT NULL UNIQUE,
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    ready_by_command_id INTEGER REFERENCES commands(command_row_id)
);
CREATE TABLE pi_child_spawn_admissions (
    pi_child_spawn_admission_id INTEGER PRIMARY KEY,
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    actor_attempt_id INTEGER REFERENCES attempts(actor_attempt_id),
    grand_architect_office_session_id INTEGER REFERENCES grand_architect_office_sessions(grand_architect_office_session_id),
    budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id),
    execution_profile_id INTEGER NOT NULL REFERENCES execution_profiles(execution_profile_id),
    workspace_id INTEGER NOT NULL REFERENCES workspaces(workspace_id),
    supervisor_epoch_id INTEGER NOT NULL REFERENCES supervisor_epochs(supervisor_epoch_id),
    pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id),
    admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2, 3)),
    admitted_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    spawned_by_command_id INTEGER REFERENCES commands(command_row_id),
    CHECK ((actor_attempt_id IS NOT NULL AND grand_architect_office_session_id IS NULL)
        OR (actor_attempt_id IS NULL AND grand_architect_office_session_id IS NOT NULL)),
    UNIQUE(actor_attempt_id),
    UNIQUE(grand_architect_office_session_id)
);
CREATE TABLE pi_child_spawn_invalidations (
    pi_child_spawn_admission_id INTEGER PRIMARY KEY REFERENCES pi_child_spawn_admissions(pi_child_spawn_admission_id),
    reason INTEGER NOT NULL CHECK (reason IN (1, 2, 3, 4)),
    invalidated_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
-- Office ownership has no pre-M5 Attempt reservation relation. This exact
-- one-to-one mapping is installed with Pi admission, not inferred from an
-- arbitrary cycle charge.
CREATE TABLE office_session_budget_reservations (
    grand_architect_office_session_id INTEGER PRIMARY KEY REFERENCES grand_architect_office_sessions(grand_architect_office_session_id),
    budget_reservation_id INTEGER NOT NULL UNIQUE REFERENCES budget_reservations(budget_reservation_id),
    bound_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
-- This bounded migration owns only Pi SDK host children. A later deterministic
-- runner must add its own admission sidecar before sharing the generic Rust
-- process vocabulary; it cannot attach to this Pi-specific row or fake a Pi
-- session/nonce.
CREATE TABLE pi_child_processes (
    child_process_id INTEGER PRIMARY KEY,
    pi_child_spawn_admission_id INTEGER NOT NULL UNIQUE REFERENCES pi_child_spawn_admissions(pi_child_spawn_admission_id),
    child_identity TEXT NOT NULL UNIQUE,
    direct_child_pid INTEGER NOT NULL CHECK (direct_child_pid > 0),
    process_group_id INTEGER NOT NULL CHECK (process_group_id > 0),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 8),
    terminal_disposition INTEGER CHECK (terminal_disposition BETWEEN 1 AND 7),
    spawned_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    last_transition_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    CHECK (direct_child_pid = process_group_id),
    CHECK ((lifecycle_state = 8 AND terminal_disposition IN (1, 4, 5))
        OR (lifecycle_state = 5 AND terminal_disposition IS NULL)
        OR (lifecycle_state = 6 AND terminal_disposition = 6)
        OR (lifecycle_state = 7 AND terminal_disposition = 7)
        OR (lifecycle_state IN (1, 2, 3, 4) AND terminal_disposition IS NULL))
);
-- Pi protocol facts are intentionally separate from the generic child row.
CREATE TABLE pi_child_session_protocols (
    child_process_id INTEGER PRIMARY KEY REFERENCES pi_child_processes(child_process_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 5),
    create_correlation_identity TEXT,
    create_request_digest BLOB CHECK (create_request_digest IS NULL OR length(create_request_digest) = 32),
    CHECK ((lifecycle_state < 3 AND create_correlation_identity IS NULL AND create_request_digest IS NULL)
        OR (lifecycle_state >= 3 AND create_correlation_identity IS NOT NULL AND create_request_digest IS NOT NULL))
);
CREATE TRIGGER live_pi_child_identity_not_reused
BEFORE INSERT ON pi_child_processes
WHEN EXISTS (
    SELECT 1 FROM pi_child_processes
    WHERE lifecycle_state != 8
       AND (direct_child_pid = NEW.direct_child_pid
            OR process_group_id = NEW.process_group_id)
)
BEGIN SELECT RAISE(ABORT, 'live or indeterminate PID/PGID may not be reused'); END;
CREATE TABLE child_process_liveness_observations (
    child_process_liveness_observation_id INTEGER PRIMARY KEY,
    child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id),
    liveness INTEGER NOT NULL CHECK (liveness IN (1, 2, 3)),
    observed_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE process_signal_receipts (
    process_signal_receipt_id INTEGER PRIMARY KEY,
    child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id),
    signal_action INTEGER NOT NULL CHECK (signal_action IN (1, 2, 3)),
    delivery INTEGER NOT NULL CHECK (delivery IN (1, 2, 3, 4)),
    observed_liveness INTEGER NOT NULL CHECK (observed_liveness IN (1, 2, 3)),
    cause_kind INTEGER NOT NULL CHECK (cause_kind IN (1, 2)),
    cancellation_propagation_id INTEGER REFERENCES cancellation_propagations(cancellation_propagation_id),
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    CHECK ((delivery IN (2, 3) AND observed_liveness = 2)
        OR (delivery = 4 AND observed_liveness = 3)
        OR delivery = 1)
    ,CHECK ((cause_kind = 1 AND cancellation_propagation_id IS NOT NULL)
        OR (cause_kind = 2 AND cancellation_propagation_id IS NULL))
);
CREATE TABLE child_process_reap_receipts (
    child_process_reap_receipt_id INTEGER PRIMARY KEY,
    child_process_id INTEGER NOT NULL UNIQUE REFERENCES pi_child_processes(child_process_id),
    wait_status_kind INTEGER NOT NULL CHECK (wait_status_kind IN (1, 2, 3)),
    status_value INTEGER,
    group_liveness_before_cleanup INTEGER NOT NULL CHECK (group_liveness_before_cleanup IN (1, 2, 3)),
    group_liveness_after_cleanup INTEGER NOT NULL CHECK (group_liveness_after_cleanup IN (1, 2, 3)),
    reaped_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    CHECK ((wait_status_kind IN (1, 2) AND status_value IS NOT NULL AND status_value >= 0)
        OR (wait_status_kind = 3 AND status_value IS NULL))
);
CREATE TABLE child_process_recovery_receipts (
    child_process_recovery_receipt_id INTEGER PRIMARY KEY,
    child_process_id INTEGER NOT NULL UNIQUE REFERENCES pi_child_processes(child_process_id),
    observation INTEGER NOT NULL CHECK (observation = 1),
    group_liveness_after_restart INTEGER NOT NULL CHECK (group_liveness_after_restart IN (1, 2, 3)),
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE pi_abort_control_receipts (
    pi_abort_control_receipt_id INTEGER PRIMARY KEY,
    child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id),
    cancellation_propagation_id INTEGER NOT NULL REFERENCES cancellation_propagations(cancellation_propagation_id),
    correlation_identity TEXT NOT NULL,
    abort_command_digest BLOB NOT NULL CHECK (length(abort_command_digest) = 32),
    physical_write_outcome INTEGER NOT NULL CHECK (physical_write_outcome IN (1, 2, 3, 4)),
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE(child_process_id, cancellation_propagation_id),
    UNIQUE(correlation_identity)
);
-- Once this exact owned PID/PGID is observed absent, a later present or
-- inaccessible observation is a possible identity reuse, never ordinary work.
CREATE TRIGGER child_liveness_reappearance_marks_containment
AFTER INSERT ON child_process_liveness_observations
WHEN NEW.liveness IN (1, 3) AND EXISTS (
    SELECT 1 FROM child_process_liveness_observations WHERE child_process_id = NEW.child_process_id AND liveness = 2
    UNION ALL SELECT 1 FROM process_signal_receipts WHERE child_process_id = NEW.child_process_id AND observed_liveness = 2
    UNION ALL SELECT 1 FROM child_process_reap_receipts WHERE child_process_id = NEW.child_process_id AND (group_liveness_before_cleanup = 2 OR group_liveness_after_cleanup = 2)
)
BEGIN
    UPDATE pi_child_processes SET lifecycle_state = 7, terminal_disposition = 7,
        last_transition_command_id = NEW.observed_by_command_id
     WHERE child_process_id = NEW.child_process_id;
END;
CREATE TRIGGER signal_liveness_reappearance_marks_containment
AFTER INSERT ON process_signal_receipts
WHEN NEW.observed_liveness IN (1, 3) AND EXISTS (
    SELECT 1 FROM child_process_liveness_observations WHERE child_process_id = NEW.child_process_id AND liveness = 2
    UNION ALL SELECT 1 FROM process_signal_receipts WHERE child_process_id = NEW.child_process_id AND observed_liveness = 2
    UNION ALL SELECT 1 FROM child_process_reap_receipts WHERE child_process_id = NEW.child_process_id AND (group_liveness_before_cleanup = 2 OR group_liveness_after_cleanup = 2)
)
BEGIN
    UPDATE pi_child_processes SET lifecycle_state = 7, terminal_disposition = 7,
        last_transition_command_id = NEW.recorded_by_command_id
     WHERE child_process_id = NEW.child_process_id;
END;
CREATE TRIGGER reap_liveness_reappearance_marks_containment
AFTER INSERT ON child_process_reap_receipts
WHEN (NEW.group_liveness_before_cleanup IN (1, 3) OR NEW.group_liveness_after_cleanup IN (1, 3)) AND EXISTS (
    SELECT 1 FROM child_process_liveness_observations WHERE child_process_id = NEW.child_process_id AND liveness = 2
    UNION ALL SELECT 1 FROM process_signal_receipts WHERE child_process_id = NEW.child_process_id AND observed_liveness = 2
)
BEGIN
    UPDATE pi_child_processes SET lifecycle_state = 7, terminal_disposition = 7,
        last_transition_command_id = NEW.reaped_by_command_id
     WHERE child_process_id = NEW.child_process_id;
END;
CREATE TABLE child_stream_seals (
    child_stream_seal_id INTEGER PRIMARY KEY,
    child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id),
    stream_kind INTEGER NOT NULL CHECK (stream_kind IN (1, 2, 3, 4)),
    full_observed_digest BLOB NOT NULL CHECK (length(full_observed_digest) = 32),
    retained_content_object_id INTEGER NOT NULL REFERENCES content_objects(content_object_id),
    completeness INTEGER NOT NULL CHECK (completeness IN (1, 2, 3)),
    sealed_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE(child_process_id, stream_kind)
);
CREATE TABLE cancellation_propagations (
    cancellation_propagation_id INTEGER PRIMARY KEY,
    cancellation_request_id INTEGER NOT NULL UNIQUE REFERENCES cancellation_requests(cancellation_request_id),
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    observed_generation INTEGER NOT NULL CHECK (observed_generation >= 0),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2, 3)),
    begun_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    reconciled_by_command_id INTEGER REFERENCES commands(command_row_id)
);
CREATE TABLE cancellation_propagation_children (
    cancellation_propagation_id INTEGER NOT NULL REFERENCES cancellation_propagations(cancellation_propagation_id),
    child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id),
    PRIMARY KEY(cancellation_propagation_id, child_process_id)
);
-- Snapshot targets are owner obligations, including owners that had no child
-- at cancellation acceptance. A child row is a later physical attachment,
-- not proof that the target set was complete.
CREATE TABLE cancellation_propagation_targets (
    cancellation_propagation_target_id INTEGER PRIMARY KEY,
    cancellation_propagation_id INTEGER NOT NULL REFERENCES cancellation_propagations(cancellation_propagation_id),
    actor_attempt_id INTEGER REFERENCES attempts(actor_attempt_id),
    grand_architect_office_session_id INTEGER REFERENCES grand_architect_office_sessions(grand_architect_office_session_id),
    child_process_id INTEGER REFERENCES pi_child_processes(child_process_id),
    target_disposition INTEGER NOT NULL CHECK (target_disposition IN (1, 2, 3, 4, 5, 6, 7)),
    CHECK ((actor_attempt_id IS NOT NULL AND grand_architect_office_session_id IS NULL)
        OR (actor_attempt_id IS NULL AND grand_architect_office_session_id IS NOT NULL)),
    UNIQUE(cancellation_propagation_id, actor_attempt_id),
    UNIQUE(cancellation_propagation_id, grand_architect_office_session_id)
);

CREATE TABLE command_admit_pi_child_spawn (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    actor_attempt_id INTEGER,
    grand_architect_office_session_id INTEGER,
    budget_reservation_id INTEGER NOT NULL,
    execution_profile_id INTEGER NOT NULL,
    native_workspace_id TEXT NOT NULL,
    canonical_workspace_path TEXT NOT NULL,
    supervisor_epoch_id INTEGER NOT NULL,
    supervisor_epoch_identity TEXT NOT NULL,
    pi_session_identity TEXT NOT NULL,
    spawn_nonce TEXT NOT NULL,
    CHECK ((actor_attempt_id IS NOT NULL AND grand_architect_office_session_id IS NULL)
        OR (actor_attempt_id IS NULL AND grand_architect_office_session_id IS NOT NULL))
);
CREATE TABLE command_record_inert_pi_child_spawn (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    pi_child_spawn_admission_id INTEGER NOT NULL,
    child_identity TEXT NOT NULL,
    direct_child_pid INTEGER NOT NULL CHECK (direct_child_pid > 0),
    process_group_id INTEGER NOT NULL CHECK (process_group_id > 0)
);
CREATE TABLE command_record_pi_adapter_ready (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    child_process_id INTEGER NOT NULL, pi_session_identity TEXT NOT NULL, spawn_nonce TEXT NOT NULL
);
CREATE TABLE command_authorize_pi_create_session (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), child_process_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL, create_request_digest BLOB NOT NULL CHECK (length(create_request_digest) = 32)
);
CREATE TABLE command_record_pi_create_session_delivery (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), child_process_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL, create_request_digest BLOB NOT NULL CHECK (length(create_request_digest) = 32)
);
CREATE TABLE command_record_pi_session_ready (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), child_process_id INTEGER NOT NULL, pi_session_identity TEXT NOT NULL
);
CREATE TABLE command_record_pi_abort_control_delivery (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    child_process_id INTEGER NOT NULL,
    cancellation_propagation_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL,
    abort_command_digest BLOB NOT NULL CHECK (length(abort_command_digest) = 32),
    physical_write_outcome INTEGER NOT NULL CHECK (physical_write_outcome IN (1, 2, 3, 4))
);
CREATE TABLE command_record_child_stream_seal (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), child_process_id INTEGER NOT NULL,
    stream_kind INTEGER NOT NULL CHECK (stream_kind IN (1, 2, 3, 4)),
    full_observed_digest BLOB NOT NULL CHECK (length(full_observed_digest) = 32),
    retained_content_object_id INTEGER NOT NULL, completeness INTEGER NOT NULL CHECK (completeness IN (1, 2, 3))
);
CREATE TABLE command_record_child_process_liveness (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), child_process_id INTEGER NOT NULL, liveness INTEGER NOT NULL CHECK (liveness IN (1, 2, 3))
);
CREATE TABLE command_record_process_signal_receipt (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), child_process_id INTEGER NOT NULL,
    signal_action INTEGER NOT NULL CHECK (signal_action IN (1, 2, 3)), delivery INTEGER NOT NULL CHECK (delivery IN (1, 2, 3, 4)),
    observed_liveness INTEGER NOT NULL CHECK (observed_liveness IN (1, 2, 3)),
    cause_kind INTEGER NOT NULL CHECK (cause_kind IN (1, 2)),
    cancellation_propagation_id INTEGER,
    CHECK ((cause_kind = 1 AND cancellation_propagation_id IS NOT NULL)
        OR (cause_kind = 2 AND cancellation_propagation_id IS NULL))
);
CREATE TABLE command_record_direct_child_reap (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), child_process_id INTEGER NOT NULL,
    wait_status_kind INTEGER NOT NULL CHECK (wait_status_kind IN (1, 2, 3)), status_value INTEGER,
    group_liveness_before_cleanup INTEGER NOT NULL CHECK (group_liveness_before_cleanup IN (1, 2, 3)),
    group_liveness_after_cleanup INTEGER NOT NULL CHECK (group_liveness_after_cleanup IN (1, 2, 3)),
    CHECK ((wait_status_kind IN (1, 2) AND status_value IS NOT NULL AND status_value >= 0)
        OR (wait_status_kind = 3 AND status_value IS NULL))
);
CREATE TABLE command_record_child_recovery (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), child_process_id INTEGER NOT NULL,
    observation INTEGER NOT NULL CHECK (observation = 1),
    group_liveness_after_restart INTEGER NOT NULL CHECK (group_liveness_after_restart IN (1, 2, 3))
);
CREATE TABLE command_finalize_child_process (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), child_process_id INTEGER NOT NULL
);
CREATE TABLE command_begin_cancellation_propagation (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), cancellation_request_id INTEGER NOT NULL
);
CREATE TABLE command_reconcile_cancellation_propagation (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), cancellation_propagation_id INTEGER NOT NULL
);
CREATE TABLE command_open_supervisor_epoch (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    supervisor_epoch_id INTEGER NOT NULL,
    supervisor_epoch_identity TEXT NOT NULL
);
CREATE TABLE command_record_pi_child_not_spawned (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    pi_child_spawn_admission_id INTEGER NOT NULL,
    reason INTEGER NOT NULL CHECK (reason IN (1, 2, 3, 4))
);

CREATE TABLE event_pi_child_spawn_admitted (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), pi_child_spawn_admission_id INTEGER NOT NULL REFERENCES pi_child_spawn_admissions(pi_child_spawn_admission_id),
    actor_attempt_id INTEGER, grand_architect_office_session_id INTEGER, budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id),
    CHECK ((actor_attempt_id IS NOT NULL AND grand_architect_office_session_id IS NULL)
        OR (actor_attempt_id IS NULL AND grand_architect_office_session_id IS NOT NULL))
);
CREATE TABLE event_inert_pi_child_spawn_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id), pi_child_spawn_admission_id INTEGER NOT NULL REFERENCES pi_child_spawn_admissions(pi_child_spawn_admission_id)
);
CREATE TABLE event_pi_adapter_ready_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id), pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id)
);
CREATE TABLE event_pi_create_session_authorized (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id)
);
CREATE TABLE event_pi_create_session_delivery_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id)
);
CREATE TABLE event_pi_session_ready_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id), pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id)
);
CREATE TABLE event_pi_abort_control_delivery_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    pi_abort_control_receipt_id INTEGER NOT NULL REFERENCES pi_abort_control_receipts(pi_abort_control_receipt_id),
    child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id),
    cancellation_propagation_id INTEGER NOT NULL REFERENCES cancellation_propagations(cancellation_propagation_id),
    correlation_identity TEXT NOT NULL,
    abort_command_digest BLOB NOT NULL CHECK (length(abort_command_digest) = 32),
    physical_write_outcome INTEGER NOT NULL CHECK (physical_write_outcome IN (1, 2, 3, 4))
);
CREATE TABLE event_child_stream_sealed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), child_stream_seal_id INTEGER NOT NULL REFERENCES child_stream_seals(child_stream_seal_id),
    child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id), stream_kind INTEGER NOT NULL CHECK (stream_kind IN (1, 2, 3, 4)), completeness INTEGER NOT NULL CHECK (completeness IN (1, 2, 3))
);
CREATE TABLE event_child_process_liveness_observed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), child_process_liveness_observation_id INTEGER NOT NULL REFERENCES child_process_liveness_observations(child_process_liveness_observation_id),
    child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id), liveness INTEGER NOT NULL CHECK (liveness IN (1, 2, 3))
);
CREATE TABLE event_process_signal_receipt_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), process_signal_receipt_id INTEGER NOT NULL REFERENCES process_signal_receipts(process_signal_receipt_id),
    child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id), signal_action INTEGER NOT NULL CHECK (signal_action IN (1, 2, 3)), delivery INTEGER NOT NULL CHECK (delivery IN (1, 2, 3, 4)),
    observed_liveness INTEGER NOT NULL CHECK (observed_liveness IN (1, 2, 3)),
    cause_kind INTEGER NOT NULL CHECK (cause_kind IN (1, 2)), cancellation_propagation_id INTEGER,
    CHECK ((cause_kind = 1 AND cancellation_propagation_id IS NOT NULL)
        OR (cause_kind = 2 AND cancellation_propagation_id IS NULL)),
    CHECK ((delivery IN (2, 3) AND observed_liveness = 2)
        OR (delivery = 4 AND observed_liveness = 3)
        OR delivery = 1)
);
CREATE TABLE event_direct_child_reaped (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), child_process_reap_receipt_id INTEGER NOT NULL REFERENCES child_process_reap_receipts(child_process_reap_receipt_id),
    child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id), wait_status_kind INTEGER NOT NULL CHECK (wait_status_kind IN (1, 2, 3)), status_value INTEGER,
    group_liveness_before_cleanup INTEGER NOT NULL CHECK (group_liveness_before_cleanup IN (1, 2, 3)),
    group_liveness_after_cleanup INTEGER NOT NULL CHECK (group_liveness_after_cleanup IN (1, 2, 3)),
    CHECK ((wait_status_kind IN (1, 2) AND status_value IS NOT NULL AND status_value >= 0)
        OR (wait_status_kind = 3 AND status_value IS NULL))
);
CREATE TABLE event_child_recovery_observed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), child_process_recovery_receipt_id INTEGER NOT NULL REFERENCES child_process_recovery_receipts(child_process_recovery_receipt_id),
    child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id), observation INTEGER NOT NULL CHECK (observation = 1),
    group_liveness_after_restart INTEGER NOT NULL CHECK (group_liveness_after_restart IN (1, 2, 3))
);
CREATE TABLE event_child_process_finalized (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), child_process_id INTEGER NOT NULL REFERENCES pi_child_processes(child_process_id), disposition INTEGER NOT NULL CHECK (disposition IN (1, 4, 5))
);
CREATE TABLE event_cancellation_propagation_begun (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), cancellation_propagation_id INTEGER NOT NULL REFERENCES cancellation_propagations(cancellation_propagation_id), cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id)
);
CREATE TABLE event_cancellation_propagation_reconciled (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), cancellation_propagation_id INTEGER NOT NULL REFERENCES cancellation_propagations(cancellation_propagation_id)
);
CREATE TABLE event_supervisor_epoch_opened (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    supervisor_epoch_id INTEGER NOT NULL UNIQUE REFERENCES supervisor_epochs(supervisor_epoch_id)
);
CREATE TABLE event_cancellation_propagation_containment_failed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    cancellation_propagation_id INTEGER NOT NULL REFERENCES cancellation_propagations(cancellation_propagation_id)
);
CREATE TABLE event_pi_child_spawn_invalidated (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    pi_child_spawn_admission_id INTEGER NOT NULL REFERENCES pi_child_spawn_admissions(pi_child_spawn_admission_id),
    reason INTEGER NOT NULL CHECK (reason IN (1, 2, 3, 4))
);

INSERT OR IGNORE INTO capability_grants(principal_id, capability_kind, office_occupancy_id, actor_instance_id, grant_state, grant_origin, granted_by_command_id, consumed_by_command_id)
VALUES (2, 69, NULL, NULL, 1, 3, NULL, NULL), (2, 70, NULL, NULL, 1, 3, NULL, NULL),
       (2, 71, NULL, NULL, 1, 3, NULL, NULL), (2, 72, NULL, NULL, 1, 3, NULL, NULL),
       (2, 73, NULL, NULL, 1, 3, NULL, NULL), (2, 74, NULL, NULL, 1, 3, NULL, NULL),
       (2, 75, NULL, NULL, 1, 3, NULL, NULL), (2, 76, NULL, NULL, 1, 3, NULL, NULL),
       (2, 77, NULL, NULL, 1, 3, NULL, NULL), (2, 78, NULL, NULL, 1, 3, NULL, NULL),
       (2, 79, NULL, NULL, 1, 3, NULL, NULL), (2, 80, NULL, NULL, 1, 3, NULL, NULL),
       (2, 81, NULL, NULL, 1, 3, NULL, NULL), (2, 82, NULL, NULL, 1, 3, NULL, NULL),
       (2, 83, NULL, NULL, 1, 3, NULL, NULL), (2, 84, NULL, NULL, 1, 3, NULL, NULL),
       (2, 85, NULL, NULL, 1, 3, NULL, NULL);

PRAGMA user_version = 5;
COMMIT;
PRAGMA foreign_keys = ON;
