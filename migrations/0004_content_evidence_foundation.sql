-- V2 migration 4: forensic content identity and deterministic evidence
-- admission. The runner deliberately accepts this rebuild only from an empty
-- M3 ledger. It does not rewrite historical command/event commitments.
--
-- This step records content-store/evaluator adapter attestations. It neither
-- stores physical bytes nor claims evaluator execution, curation, graph
-- meaning, epistemic truth, or influence.

PRAGMA foreign_keys = OFF;
PRAGMA legacy_alter_table = ON;
BEGIN IMMEDIATE;

ALTER TABLE capability_grants RENAME TO capability_grants_m3;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_occupancy_on_insert;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_occupancy_on_update;
DROP TRIGGER IF EXISTS occupancy_principal_matches_existing_grants;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_actor_instance_on_insert;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_actor_instance_on_update;
DROP INDEX IF EXISTS active_capability_grant_per_principal;
CREATE TABLE capability_grants (
    capability_grant_id INTEGER PRIMARY KEY,
    principal_id INTEGER NOT NULL REFERENCES principals(principal_id),
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 68),
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
INSERT INTO capability_grants SELECT * FROM capability_grants_m3;
DROP TABLE capability_grants_m3;
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

ALTER TABLE commands RENAME TO commands_m3;
CREATE TABLE commands (
    command_row_id INTEGER PRIMARY KEY,
    command_id TEXT NOT NULL UNIQUE,
    principal_id INTEGER NOT NULL,
    capability_grant_id INTEGER NOT NULL,
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 68),
    expected_generation INTEGER,
    command_kind INTEGER NOT NULL CHECK (command_kind BETWEEN 1 AND 68),
    request_fingerprint BLOB NOT NULL CHECK (length(request_fingerprint) = 32),
    command_status INTEGER NOT NULL CHECK (command_status IN (1, 2)),
    rejection_code INTEGER,
    accepted_event_id INTEGER,
    CHECK ((command_status = 1 AND rejection_code IS NULL AND accepted_event_id IS NOT NULL)
        OR (command_status = 2 AND rejection_code IS NOT NULL AND accepted_event_id IS NULL))
);
INSERT INTO commands SELECT * FROM commands_m3;
DROP TABLE commands_m3;

ALTER TABLE events RENAME TO events_m3;
CREATE TABLE events (
    event_id INTEGER PRIMARY KEY,
    command_row_id INTEGER NOT NULL UNIQUE REFERENCES commands(command_row_id),
    event_kind INTEGER NOT NULL CHECK (event_kind BETWEEN 1 AND 61),
    event_sequence INTEGER NOT NULL UNIQUE CHECK (event_sequence > 0),
    event_fingerprint BLOB NOT NULL CHECK (length(event_fingerprint) = 32)
);
INSERT INTO events SELECT * FROM events_m3;
DROP TABLE events_m3;

CREATE TABLE content_seal_receipts (
    content_seal_receipt_id INTEGER PRIMARY KEY,
    digest BLOB NOT NULL UNIQUE CHECK (length(digest) = 32),
    attested_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE content_objects (
    content_object_id INTEGER PRIMARY KEY,
    content_seal_receipt_id INTEGER NOT NULL UNIQUE REFERENCES content_seal_receipts(content_seal_receipt_id),
    registered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE forensic_manifests (
    forensic_manifest_id INTEGER PRIMARY KEY,
    producing_deterministic_experiment_id INTEGER NOT NULL REFERENCES deterministic_experiments(deterministic_experiment_id),
    capture_policy INTEGER NOT NULL CHECK (capture_policy = 1),
    retention_access_class INTEGER NOT NULL CHECK (retention_access_class IN (1, 2)),
    registered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE forensic_manifest_objects (
    forensic_manifest_id INTEGER NOT NULL REFERENCES forensic_manifests(forensic_manifest_id),
    member_ordinal INTEGER NOT NULL CHECK (member_ordinal > 0),
    object_role INTEGER NOT NULL CHECK (object_role = 1),
    media_schema_contract INTEGER NOT NULL CHECK (media_schema_contract = 3),
    content_object_id INTEGER NOT NULL REFERENCES content_objects(content_object_id),
    PRIMARY KEY (forensic_manifest_id, member_ordinal),
    UNIQUE (forensic_manifest_id, content_object_id),
    UNIQUE (forensic_manifest_id, object_role)
);

CREATE TABLE evaluator_revisions (
    evaluator_revision_id INTEGER PRIMARY KEY,
    content_object_id INTEGER NOT NULL UNIQUE REFERENCES content_objects(content_object_id),
    media_schema_contract INTEGER NOT NULL CHECK (media_schema_contract = 1),
    registered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE input_manifests (
    input_manifest_id INTEGER PRIMARY KEY,
    content_object_id INTEGER NOT NULL UNIQUE REFERENCES content_objects(content_object_id),
    media_schema_contract INTEGER NOT NULL CHECK (media_schema_contract = 2),
    registered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE deterministic_experiments (
    deterministic_experiment_id INTEGER PRIMARY KEY,
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    project_id INTEGER NOT NULL REFERENCES projects(project_id),
    ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id),
    target_graph_revision_id INTEGER NOT NULL REFERENCES object_revisions(graph_revision_id),
    evaluator_revision_id INTEGER NOT NULL REFERENCES evaluator_revisions(evaluator_revision_id),
    input_manifest_id INTEGER NOT NULL REFERENCES input_manifests(input_manifest_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2, 3)),
    registered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    last_transition_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE deterministic_evaluation_receipts (
    deterministic_evaluation_receipt_id INTEGER PRIMARY KEY,
    deterministic_experiment_id INTEGER NOT NULL UNIQUE REFERENCES deterministic_experiments(deterministic_experiment_id),
    evaluator_revision_id INTEGER NOT NULL REFERENCES evaluator_revisions(evaluator_revision_id),
    input_manifest_id INTEGER NOT NULL REFERENCES input_manifests(input_manifest_id),
    forensic_manifest_id INTEGER NOT NULL REFERENCES forensic_manifests(forensic_manifest_id),
    evaluator_output_content_object_id INTEGER NOT NULL REFERENCES content_objects(content_object_id),
    attested_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE evidence_admissions (
    evidence_admission_id INTEGER PRIMARY KEY,
    deterministic_evaluation_receipt_id INTEGER NOT NULL UNIQUE REFERENCES deterministic_evaluation_receipts(deterministic_evaluation_receipt_id),
    deterministic_experiment_id INTEGER NOT NULL REFERENCES deterministic_experiments(deterministic_experiment_id),
    evaluator_revision_id INTEGER NOT NULL REFERENCES evaluator_revisions(evaluator_revision_id),
    input_manifest_id INTEGER NOT NULL REFERENCES input_manifests(input_manifest_id),
    evaluator_output_content_object_id INTEGER NOT NULL REFERENCES content_objects(content_object_id),
    related_graph_revision_id INTEGER NOT NULL REFERENCES object_revisions(graph_revision_id),
    semantic_role INTEGER NOT NULL CHECK (semantic_role = 1),
    applicability INTEGER NOT NULL CHECK (applicability = 1),
    limitation_text TEXT NOT NULL,
    admitted_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE command_record_content_seal_receipt (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    digest BLOB NOT NULL CHECK (length(digest) = 32)
);
CREATE TABLE command_register_content_object (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    content_seal_receipt_id INTEGER NOT NULL
);
CREATE TABLE command_register_forensic_manifest (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    producing_deterministic_experiment_id INTEGER NOT NULL,
    capture_policy INTEGER NOT NULL CHECK (capture_policy = 1),
    retention_access_class INTEGER NOT NULL CHECK (retention_access_class IN (1, 2)),
    evaluator_output_content_object_id INTEGER NOT NULL
);
CREATE TABLE command_register_deterministic_experiment (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    project_id INTEGER NOT NULL,
    ticket_id INTEGER NOT NULL,
    target_graph_revision_id INTEGER NOT NULL,
    evaluator_content_object_id INTEGER NOT NULL,
    input_manifest_content_object_id INTEGER NOT NULL
);
CREATE TABLE command_record_deterministic_evaluation_receipt (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    deterministic_experiment_id INTEGER NOT NULL,
    evaluator_revision_id INTEGER NOT NULL,
    input_manifest_id INTEGER NOT NULL,
    forensic_manifest_id INTEGER NOT NULL,
    evaluator_output_content_object_id INTEGER NOT NULL
);
CREATE TABLE command_admit_deterministic_evidence (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    deterministic_evaluation_receipt_id INTEGER NOT NULL,
    deterministic_experiment_id INTEGER NOT NULL,
    evaluator_revision_id INTEGER NOT NULL,
    input_manifest_id INTEGER NOT NULL,
    evaluator_output_content_object_id INTEGER NOT NULL,
    related_graph_revision_id INTEGER NOT NULL,
    semantic_role INTEGER NOT NULL CHECK (semantic_role = 1),
    applicability INTEGER NOT NULL CHECK (applicability = 1),
    limitation_text TEXT NOT NULL
);
CREATE TABLE command_close_deterministic_experiment (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    deterministic_experiment_id INTEGER NOT NULL
);

CREATE TABLE event_content_seal_receipt_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    content_seal_receipt_id INTEGER NOT NULL REFERENCES content_seal_receipts(content_seal_receipt_id),
    digest BLOB NOT NULL CHECK (length(digest) = 32)
);
CREATE TABLE event_content_object_registered (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    content_object_id INTEGER NOT NULL REFERENCES content_objects(content_object_id),
    content_seal_receipt_id INTEGER NOT NULL REFERENCES content_seal_receipts(content_seal_receipt_id)
);
CREATE TABLE event_forensic_manifest_registered (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    forensic_manifest_id INTEGER NOT NULL REFERENCES forensic_manifests(forensic_manifest_id),
    producing_deterministic_experiment_id INTEGER NOT NULL REFERENCES deterministic_experiments(deterministic_experiment_id),
    evaluator_output_content_object_id INTEGER NOT NULL REFERENCES content_objects(content_object_id)
);
CREATE TABLE event_deterministic_experiment_registered (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    deterministic_experiment_id INTEGER NOT NULL REFERENCES deterministic_experiments(deterministic_experiment_id),
    evaluator_revision_id INTEGER NOT NULL REFERENCES evaluator_revisions(evaluator_revision_id),
    input_manifest_id INTEGER NOT NULL REFERENCES input_manifests(input_manifest_id)
);
CREATE TABLE event_deterministic_evaluation_receipt_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    deterministic_evaluation_receipt_id INTEGER NOT NULL REFERENCES deterministic_evaluation_receipts(deterministic_evaluation_receipt_id),
    deterministic_experiment_id INTEGER NOT NULL REFERENCES deterministic_experiments(deterministic_experiment_id)
);
CREATE TABLE event_deterministic_evidence_admitted (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    evidence_admission_id INTEGER NOT NULL REFERENCES evidence_admissions(evidence_admission_id),
    deterministic_evaluation_receipt_id INTEGER NOT NULL REFERENCES deterministic_evaluation_receipts(deterministic_evaluation_receipt_id),
    semantic_role INTEGER NOT NULL CHECK (semantic_role = 1),
    applicability INTEGER NOT NULL CHECK (applicability = 1)
);
CREATE TABLE event_deterministic_experiment_closed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    deterministic_experiment_id INTEGER NOT NULL REFERENCES deterministic_experiments(deterministic_experiment_id)
);

INSERT OR IGNORE INTO capability_grants(principal_id, capability_kind, office_occupancy_id, actor_instance_id, grant_state, grant_origin, granted_by_command_id, consumed_by_command_id)
VALUES (2, 62, NULL, NULL, 1, 3, NULL, NULL),
       (2, 63, NULL, NULL, 1, 3, NULL, NULL),
       (2, 64, NULL, NULL, 1, 3, NULL, NULL),
       (2, 66, NULL, NULL, 1, 3, NULL, NULL),
       (2, 67, NULL, NULL, 1, 3, NULL, NULL);

PRAGMA user_version = 4;
COMMIT;
PRAGMA foreign_keys = ON;
