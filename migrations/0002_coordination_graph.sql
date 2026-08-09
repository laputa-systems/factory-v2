-- V2 migration 2: finite coordination, review, postmortem, and graph state.
-- Migration 1 remains the historical bootstrap schema. SQLite cannot widen a
-- CHECK constraint in place, so the common ledger tables are rebuilt while
-- foreign-key rewriting is disabled; their child tables continue to name the
-- recreated canonical table names. This is one atomic version-2 step, not an
-- atomic 0 -> 2 install: the migration runner commits version 1 separately,
-- and a failed M2 rolls back to that complete version for a later retry.

PRAGMA foreign_keys = OFF;
PRAGMA legacy_alter_table = ON;
BEGIN IMMEDIATE;

ALTER TABLE capability_grants RENAME TO capability_grants_m1;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_occupancy_on_insert;
DROP TRIGGER IF EXISTS capability_grant_principal_matches_occupancy_on_update;
DROP TRIGGER IF EXISTS occupancy_principal_matches_existing_grants;
DROP INDEX IF EXISTS active_capability_grant_per_principal;
CREATE TABLE capability_grants (
    capability_grant_id INTEGER PRIMARY KEY,
    principal_id INTEGER NOT NULL REFERENCES principals(principal_id),
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 46),
    office_occupancy_id INTEGER REFERENCES office_occupancies(office_occupancy_id),
    grant_state INTEGER NOT NULL CHECK (grant_state IN (1, 2)),
    granted_by_command_id INTEGER,
    consumed_by_command_id INTEGER,
    CHECK ((grant_state = 1 AND consumed_by_command_id IS NULL)
        OR (grant_state = 2 AND consumed_by_command_id IS NOT NULL))
);
INSERT INTO capability_grants SELECT * FROM capability_grants_m1;
DROP TABLE capability_grants_m1;
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
CREATE UNIQUE INDEX active_capability_grant_per_principal
 ON capability_grants(principal_id, capability_kind, COALESCE(office_occupancy_id, -1)) WHERE grant_state = 1;

ALTER TABLE commands RENAME TO commands_m1;
CREATE TABLE commands (
    command_row_id INTEGER PRIMARY KEY,
    command_id TEXT NOT NULL UNIQUE,
    principal_id INTEGER NOT NULL,
    capability_grant_id INTEGER NOT NULL,
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 46),
    expected_generation INTEGER,
    command_kind INTEGER NOT NULL CHECK (command_kind BETWEEN 1 AND 46),
    request_fingerprint BLOB NOT NULL CHECK (length(request_fingerprint) = 32),
    command_status INTEGER NOT NULL CHECK (command_status IN (1, 2)),
    rejection_code INTEGER,
    accepted_event_id INTEGER,
    CHECK (
        (command_status = 1 AND rejection_code IS NULL AND accepted_event_id IS NOT NULL)
        OR (command_status = 2 AND rejection_code IS NOT NULL AND accepted_event_id IS NULL)
    )
);
INSERT INTO commands SELECT * FROM commands_m1;
DROP TABLE commands_m1;

ALTER TABLE events RENAME TO events_m1;
CREATE TABLE events (
    event_id INTEGER PRIMARY KEY,
    command_row_id INTEGER NOT NULL UNIQUE REFERENCES commands(command_row_id),
    event_kind INTEGER NOT NULL CHECK (event_kind BETWEEN 1 AND 39),
    event_sequence INTEGER NOT NULL UNIQUE CHECK (event_sequence > 0),
    event_fingerprint BLOB NOT NULL CHECK (length(event_fingerprint) = 32)
);
INSERT INTO events SELECT * FROM events_m1;
DROP TABLE events_m1;

CREATE TABLE projects (
    project_id INTEGER PRIMARY KEY,
    project_name TEXT NOT NULL UNIQUE,
    universe_seed_id INTEGER NOT NULL REFERENCES universe_seeds(universe_seed_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 9),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    last_transition_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE project_objectives (
    project_objective_id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL UNIQUE REFERENCES projects(project_id),
    objective_text TEXT NOT NULL,
    chartered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE project_milestones (
    project_milestone_id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(project_id),
    milestone_name TEXT NOT NULL,
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2)),
    chartered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    completed_by_command_id INTEGER REFERENCES commands(command_row_id),
    UNIQUE(project_id, milestone_name),
    CHECK ((lifecycle_state = 1 AND completed_by_command_id IS NULL)
        OR (lifecycle_state = 2 AND completed_by_command_id IS NOT NULL))
);

CREATE TABLE project_stop_conditions (
    project_stop_condition_id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL UNIQUE REFERENCES projects(project_id),
    stop_condition_text TEXT NOT NULL,
    chartered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE tickets (
    ticket_id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(project_id),
    ticket_title TEXT NOT NULL,
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 11),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    last_transition_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE(project_id, ticket_title)
);

CREATE TABLE ticket_acceptance_conditions (
    ticket_acceptance_condition_id INTEGER PRIMARY KEY,
    ticket_id INTEGER NOT NULL UNIQUE REFERENCES tickets(ticket_id),
    condition_text TEXT NOT NULL,
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2)),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    satisfied_by_command_id INTEGER REFERENCES commands(command_row_id),
    CHECK ((lifecycle_state = 1 AND satisfied_by_command_id IS NULL)
        OR (lifecycle_state = 2 AND satisfied_by_command_id IS NOT NULL))
);

CREATE TABLE ticket_prerequisites (
    ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id),
    prerequisite_ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    PRIMARY KEY(ticket_id, prerequisite_ticket_id),
    CHECK(ticket_id != prerequisite_ticket_id)
);

CREATE TABLE objects (
    graph_object_id INTEGER PRIMARY KEY,
    object_kind INTEGER NOT NULL CHECK (object_kind BETWEEN 1 AND 2),
    project_id INTEGER REFERENCES projects(project_id),
    causal_episode_id INTEGER REFERENCES episodes(causal_episode_id),
    universe_seed_id INTEGER NOT NULL REFERENCES universe_seeds(universe_seed_id),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE object_revisions (
    graph_revision_id INTEGER PRIMARY KEY,
    graph_object_id INTEGER NOT NULL REFERENCES objects(graph_object_id),
    revision_ordinal INTEGER NOT NULL CHECK (revision_ordinal > 0),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2)),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    committed_by_command_id INTEGER REFERENCES commands(command_row_id),
    UNIQUE(graph_object_id, revision_ordinal),
    CHECK ((lifecycle_state = 1 AND committed_by_command_id IS NULL)
        OR (lifecycle_state = 2 AND committed_by_command_id IS NOT NULL))
);
CREATE TRIGGER graph_revision_identity_is_immutable
BEFORE UPDATE OF graph_object_id, revision_ordinal, created_by_command_id
ON object_revisions
BEGIN
    SELECT RAISE(ABORT, 'graph revision content is immutable');
END;

-- The shared row has no semantic text. Exactly one kind-specific body is
-- required by the kernel and replay verifier; these tables retain the
-- searchable typed content for the only M2 epistemic kinds.
CREATE TABLE observation_revisions (
    graph_revision_id INTEGER PRIMARY KEY REFERENCES object_revisions(graph_revision_id),
    observation_text TEXT NOT NULL
);
CREATE TABLE hypothesis_revisions (
    graph_revision_id INTEGER PRIMARY KEY REFERENCES object_revisions(graph_revision_id),
    hypothesis_text TEXT NOT NULL
);
CREATE TRIGGER observation_revision_matches_object_kind
BEFORE INSERT ON observation_revisions
WHEN NOT EXISTS (
    SELECT 1 FROM object_revisions r JOIN objects o ON o.graph_object_id = r.graph_object_id
    WHERE r.graph_revision_id = NEW.graph_revision_id AND o.object_kind = 1
)
BEGIN SELECT RAISE(ABORT, 'observation body must match observation object'); END;
CREATE TRIGGER hypothesis_revision_matches_object_kind
BEFORE INSERT ON hypothesis_revisions
WHEN NOT EXISTS (
    SELECT 1 FROM object_revisions r JOIN objects o ON o.graph_object_id = r.graph_object_id
    WHERE r.graph_revision_id = NEW.graph_revision_id AND o.object_kind = 2
)
BEGIN SELECT RAISE(ABORT, 'hypothesis body must match hypothesis object'); END;
CREATE TRIGGER observation_revision_cannot_update
BEFORE UPDATE ON observation_revisions
BEGIN SELECT RAISE(ABORT, 'observation revision body is immutable'); END;
CREATE TRIGGER observation_revision_cannot_delete
BEFORE DELETE ON observation_revisions
BEGIN SELECT RAISE(ABORT, 'observation revision body is immutable'); END;
CREATE TRIGGER hypothesis_revision_cannot_update
BEFORE UPDATE ON hypothesis_revisions
BEGIN SELECT RAISE(ABORT, 'hypothesis revision body is immutable'); END;
CREATE TRIGGER hypothesis_revision_cannot_delete
BEFORE DELETE ON hypothesis_revisions
BEGIN SELECT RAISE(ABORT, 'hypothesis revision body is immutable'); END;
CREATE TRIGGER graph_revision_cannot_be_deleted
BEFORE DELETE ON object_revisions
BEGIN
    SELECT RAISE(ABORT, 'graph revision cannot be deleted');
END;

CREATE TABLE edges (
    graph_edge_id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(project_id),
    from_graph_revision_id INTEGER NOT NULL REFERENCES object_revisions(graph_revision_id),
    to_graph_revision_id INTEGER NOT NULL REFERENCES object_revisions(graph_revision_id),
    edge_kind INTEGER NOT NULL CHECK (edge_kind BETWEEN 1 AND 2),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE(from_graph_revision_id, to_graph_revision_id, edge_kind),
    CHECK(from_graph_revision_id != to_graph_revision_id)
);

CREATE TABLE episodes (
    causal_episode_id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(project_id),
    universe_seed_id INTEGER NOT NULL REFERENCES universe_seeds(universe_seed_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 17),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    last_transition_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE adversarial_reviews (
    adversarial_review_id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(project_id),
    target_graph_revision_id INTEGER NOT NULL REFERENCES object_revisions(graph_revision_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 9),
    requested_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    assigned_reviewer_principal_id INTEGER REFERENCES principals(principal_id),
    resolved_by_command_id INTEGER REFERENCES commands(command_row_id)
);

CREATE TABLE review_challenges (
    review_challenge_id INTEGER PRIMARY KEY,
    adversarial_review_id INTEGER NOT NULL REFERENCES adversarial_reviews(adversarial_review_id),
    target_graph_revision_id INTEGER NOT NULL REFERENCES object_revisions(graph_revision_id),
    author_principal_id INTEGER NOT NULL REFERENCES principals(principal_id),
    severity INTEGER NOT NULL CHECK (severity BETWEEN 1 AND 4),
    failure_hypothesis TEXT NOT NULL,
    response_state INTEGER NOT NULL CHECK (response_state IN (1, 2)),
    submitted_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE review_challenge_responses (
    review_challenge_response_id INTEGER PRIMARY KEY,
    review_challenge_id INTEGER NOT NULL UNIQUE REFERENCES review_challenges(review_challenge_id),
    response_text TEXT NOT NULL,
    responded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE review_dispositions (
    review_disposition_id INTEGER PRIMARY KEY,
    review_challenge_id INTEGER NOT NULL UNIQUE REFERENCES review_challenges(review_challenge_id),
    disposition_kind INTEGER NOT NULL CHECK (disposition_kind IN (1, 2, 3)),
    disposed_by_principal_id INTEGER NOT NULL REFERENCES principals(principal_id),
    disposed_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE postmortems (
    postmortem_id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(project_id),
    causal_episode_id INTEGER REFERENCES episodes(causal_episode_id),
    universe_seed_id INTEGER NOT NULL REFERENCES universe_seeds(universe_seed_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2, 3)),
    triggered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    closed_by_command_id INTEGER REFERENCES commands(command_row_id),
    CHECK ((lifecycle_state IN (1, 2) AND closed_by_command_id IS NULL)
        OR (lifecycle_state = 3 AND closed_by_command_id IS NOT NULL))
);

CREATE TABLE postmortem_causal_claims (
    postmortem_causal_claim_id INTEGER PRIMARY KEY,
    postmortem_id INTEGER NOT NULL REFERENCES postmortems(postmortem_id),
    claim_kind INTEGER NOT NULL CHECK (claim_kind BETWEEN 1 AND 3),
    claim_text TEXT NOT NULL,
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

CREATE TABLE postmortem_action_proposals (
    postmortem_action_proposal_id INTEGER PRIMARY KEY,
    postmortem_id INTEGER NOT NULL REFERENCES postmortems(postmortem_id),
    action_kind INTEGER NOT NULL CHECK (action_kind BETWEEN 1 AND 2),
    action_text TEXT NOT NULL,
    proposed_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);

-- Coordination provenance belongs to the command that acted in a cycle. The
-- cross-cycle Project/Episode identities deliberately have no cycle column.
CREATE TABLE coordination_command_provenance (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    universe_seed_id INTEGER NOT NULL REFERENCES universe_seeds(universe_seed_id),
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    project_id INTEGER REFERENCES projects(project_id)
);

-- One named command body table per new closed command variant.
CREATE TABLE command_create_project (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, project_name TEXT NOT NULL);
CREATE TABLE command_charter_project (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, project_id INTEGER NOT NULL, objective_text TEXT NOT NULL, milestone_name TEXT NOT NULL, stop_condition_text TEXT NOT NULL);
CREATE TABLE command_transition_project (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, project_id INTEGER NOT NULL, target_state INTEGER NOT NULL CHECK (target_state BETWEEN 1 AND 9));
CREATE TABLE command_complete_project_milestone (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, project_milestone_id INTEGER NOT NULL);
CREATE TABLE command_reopen_project (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, project_id INTEGER NOT NULL);
CREATE TABLE command_create_ticket (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, project_id INTEGER NOT NULL, ticket_title TEXT NOT NULL, acceptance_condition_text TEXT NOT NULL, prerequisite_ticket_id INTEGER);
CREATE TABLE command_transition_ticket (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, ticket_id INTEGER NOT NULL, target_state INTEGER NOT NULL CHECK (target_state BETWEEN 1 AND 11));
CREATE TABLE command_add_graph_object_revision (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, project_id INTEGER NOT NULL, causal_episode_id INTEGER, graph_object_id INTEGER);
CREATE TABLE command_add_observation_revision (command_row_id INTEGER PRIMARY KEY REFERENCES command_add_graph_object_revision(command_row_id), observation_text TEXT NOT NULL);
CREATE TABLE command_add_hypothesis_revision (command_row_id INTEGER PRIMARY KEY REFERENCES command_add_graph_object_revision(command_row_id), hypothesis_text TEXT NOT NULL);
CREATE TABLE command_commit_graph_revision (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, graph_revision_id INTEGER NOT NULL);
CREATE TABLE command_add_graph_edge (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, project_id INTEGER NOT NULL, from_graph_revision_id INTEGER NOT NULL, to_graph_revision_id INTEGER NOT NULL, edge_kind INTEGER NOT NULL CHECK (edge_kind BETWEEN 1 AND 2));
CREATE TABLE command_create_episode (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, project_id INTEGER NOT NULL);
CREATE TABLE command_transition_episode (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, causal_episode_id INTEGER NOT NULL, target_state INTEGER NOT NULL CHECK (target_state BETWEEN 1 AND 17));
CREATE TABLE command_reopen_episode (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, causal_episode_id INTEGER NOT NULL);
CREATE TABLE command_request_adversarial_review (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, project_id INTEGER NOT NULL, target_graph_revision_id INTEGER NOT NULL);
CREATE TABLE command_submit_review_challenge (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, adversarial_review_id INTEGER NOT NULL, target_graph_revision_id INTEGER NOT NULL, author_principal_id INTEGER NOT NULL, severity INTEGER NOT NULL CHECK (severity BETWEEN 1 AND 4), failure_hypothesis TEXT NOT NULL);
CREATE TABLE command_respond_to_review_challenge (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, review_challenge_id INTEGER NOT NULL, response_text TEXT NOT NULL);
CREATE TABLE command_disposition_review_challenge (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, review_challenge_id INTEGER NOT NULL, disposition_kind INTEGER NOT NULL CHECK (disposition_kind BETWEEN 1 AND 3));
CREATE TABLE command_resolve_adversarial_review (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, adversarial_review_id INTEGER NOT NULL, resolution_kind INTEGER NOT NULL CHECK (resolution_kind IN (1, 2)));
CREATE TABLE command_trigger_postmortem (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, project_id INTEGER NOT NULL, causal_episode_id INTEGER);
CREATE TABLE command_record_postmortem_causal_claim (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, postmortem_id INTEGER NOT NULL, claim_kind INTEGER NOT NULL CHECK (claim_kind BETWEEN 1 AND 3), claim_text TEXT NOT NULL);
CREATE TABLE command_propose_postmortem_action (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, postmortem_id INTEGER NOT NULL, action_kind INTEGER NOT NULL CHECK (action_kind IN (1, 2)), action_text TEXT NOT NULL);
CREATE TABLE command_close_postmortem (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, postmortem_id INTEGER NOT NULL);
CREATE TABLE command_assign_adversarial_reviewer (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL, adversarial_review_id INTEGER NOT NULL, reviewer_principal_id INTEGER NOT NULL);

-- One named event body table per new closed event variant.
CREATE TABLE event_project_created (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), project_id INTEGER NOT NULL REFERENCES projects(project_id));
CREATE TABLE event_project_chartered (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), project_id INTEGER NOT NULL REFERENCES projects(project_id));
CREATE TABLE event_project_state_changed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), project_id INTEGER NOT NULL REFERENCES projects(project_id), lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 9));
CREATE TABLE event_project_milestone_completed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), project_milestone_id INTEGER NOT NULL REFERENCES project_milestones(project_milestone_id));
CREATE TABLE event_ticket_created (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id), project_id INTEGER NOT NULL REFERENCES projects(project_id));
CREATE TABLE event_ticket_state_changed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), ticket_id INTEGER NOT NULL REFERENCES tickets(ticket_id), lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 11));
CREATE TABLE event_graph_object_revision_added (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), graph_object_id INTEGER NOT NULL REFERENCES objects(graph_object_id), graph_revision_id INTEGER NOT NULL REFERENCES object_revisions(graph_revision_id));
CREATE TABLE event_graph_revision_committed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), graph_revision_id INTEGER NOT NULL REFERENCES object_revisions(graph_revision_id));
CREATE TABLE event_graph_edge_added (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), graph_edge_id INTEGER NOT NULL REFERENCES edges(graph_edge_id));
CREATE TABLE event_episode_created (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), causal_episode_id INTEGER NOT NULL REFERENCES episodes(causal_episode_id), project_id INTEGER NOT NULL REFERENCES projects(project_id));
CREATE TABLE event_episode_state_changed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), causal_episode_id INTEGER NOT NULL REFERENCES episodes(causal_episode_id), lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 17));
CREATE TABLE event_adversarial_review_requested (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), adversarial_review_id INTEGER NOT NULL REFERENCES adversarial_reviews(adversarial_review_id));
CREATE TABLE event_review_challenge_submitted (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), review_challenge_id INTEGER NOT NULL REFERENCES review_challenges(review_challenge_id), author_principal_id INTEGER NOT NULL REFERENCES principals(principal_id));
CREATE TABLE event_review_challenge_responded (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), review_challenge_id INTEGER NOT NULL REFERENCES review_challenges(review_challenge_id));
CREATE TABLE event_review_challenge_dispositioned (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), review_challenge_id INTEGER NOT NULL REFERENCES review_challenges(review_challenge_id), disposition_kind INTEGER NOT NULL CHECK (disposition_kind BETWEEN 1 AND 3));
CREATE TABLE event_adversarial_review_resolved (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), adversarial_review_id INTEGER NOT NULL REFERENCES adversarial_reviews(adversarial_review_id), lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (6, 7)));
CREATE TABLE event_postmortem_triggered (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), postmortem_id INTEGER NOT NULL REFERENCES postmortems(postmortem_id));
CREATE TABLE event_postmortem_causal_claim_recorded (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), postmortem_causal_claim_id INTEGER NOT NULL REFERENCES postmortem_causal_claims(postmortem_causal_claim_id));
CREATE TABLE event_postmortem_action_proposed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), postmortem_action_proposal_id INTEGER NOT NULL REFERENCES postmortem_action_proposals(postmortem_action_proposal_id));
CREATE TABLE event_postmortem_closed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), postmortem_id INTEGER NOT NULL REFERENCES postmortems(postmortem_id));
CREATE TABLE event_adversarial_reviewer_assigned (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), adversarial_review_id INTEGER NOT NULL REFERENCES adversarial_reviews(adversarial_review_id), reviewer_principal_id INTEGER NOT NULL REFERENCES principals(principal_id));

INSERT OR IGNORE INTO capability_grants(principal_id, capability_kind, office_occupancy_id, grant_state, granted_by_command_id, consumed_by_command_id)
VALUES (2, 38, NULL, 1, NULL, NULL);
INSERT OR IGNORE INTO capability_grants(principal_id, capability_kind, office_occupancy_id, grant_state, granted_by_command_id, consumed_by_command_id)
VALUES (2, 46, NULL, 1, NULL, NULL);
INSERT OR IGNORE INTO capability_grants(principal_id, capability_kind, office_occupancy_id, grant_state, granted_by_command_id, consumed_by_command_id)
SELECT o.principal_id, c.capability_kind, o.office_occupancy_id, 1, NULL, NULL
FROM office_occupancies o CROSS JOIN (
 SELECT 24 AS capability_kind UNION ALL SELECT 25 UNION ALL SELECT 26 UNION ALL SELECT 27 UNION ALL SELECT 28
 UNION ALL SELECT 29 UNION ALL SELECT 30 UNION ALL SELECT 31 UNION ALL SELECT 32 UNION ALL SELECT 33
 UNION ALL SELECT 34 UNION ALL SELECT 35 UNION ALL SELECT 36 UNION ALL SELECT 37 UNION ALL SELECT 39
 UNION ALL SELECT 40 UNION ALL SELECT 41 UNION ALL SELECT 42 UNION ALL SELECT 43
 UNION ALL SELECT 44 UNION ALL SELECT 45
) c
JOIN office_contracts office ON office.office_id = o.office_id AND office.office_kind = 1
WHERE o.active = 1;
PRAGMA user_version = 2;
COMMIT;
PRAGMA foreign_keys = ON;
