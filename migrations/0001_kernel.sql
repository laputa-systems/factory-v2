PRAGMA foreign_keys = OFF;
BEGIN IMMEDIATE;
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
-- The application mission is a first-class, normalized constitution. Its
-- source rendering has a BLAKE3 byte identity and an exact, already sealed
-- ContentObject binding, while the mission remains queryable.
CREATE TABLE applications (
    application_id INTEGER PRIMARY KEY,
    application_identity TEXT NOT NULL UNIQUE,
    application_name TEXT NOT NULL UNIQUE,
    created_by_command_id INTEGER NOT NULL
);
CREATE TABLE application_revisions (
    application_revision_id INTEGER PRIMARY KEY,
    application_id INTEGER NOT NULL REFERENCES applications(application_id),
    revision_ordinal INTEGER NOT NULL CHECK (revision_ordinal > 0),
    mission_statement TEXT NOT NULL,
    source_rendering_digest BLOB NOT NULL CHECK (length(source_rendering_digest) = 32),
    source_content_object_id INTEGER NOT NULL
        REFERENCES content_objects(content_object_id),
    installed_by_command_id INTEGER NOT NULL,
    UNIQUE(application_id, revision_ordinal)
);
CREATE TABLE application_revision_principles (
    application_revision_id INTEGER NOT NULL
        REFERENCES application_revisions(application_revision_id),
    principle_ordinal INTEGER NOT NULL CHECK (principle_ordinal > 0 AND principle_ordinal <= 16),
    principle_kind INTEGER NOT NULL CHECK (principle_kind BETWEEN 1 AND 4),
    principle_text TEXT NOT NULL,
    PRIMARY KEY(application_revision_id, principle_ordinal)
);
CREATE TABLE application_revision_north_star_questions (
    application_revision_id INTEGER PRIMARY KEY
        REFERENCES application_revisions(application_revision_id),
    change_question TEXT NOT NULL,
    improvement_evidence_question TEXT NOT NULL,
    boundary_commitment_question TEXT NOT NULL,
    revisit_question TEXT NOT NULL
);
CREATE TABLE founding_missions (
    founding_mission_id INTEGER PRIMARY KEY,
    society_id INTEGER NOT NULL REFERENCES societies(society_id),
    application_revision_id INTEGER NOT NULL
        REFERENCES application_revisions(application_revision_id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    installed_by_command_id INTEGER NOT NULL,
    UNIQUE (society_id, revision)
);
CREATE UNIQUE INDEX one_active_founding_mission_per_society
    ON founding_missions(society_id) WHERE active = 1;
CREATE TABLE office_occupancies (
    office_occupancy_id INTEGER PRIMARY KEY,
    office_id INTEGER NOT NULL REFERENCES office_contracts(office_id),
    principal_id INTEGER NOT NULL REFERENCES principals(principal_id),
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    appointed_by_command_id INTEGER NOT NULL
);
CREATE UNIQUE INDEX one_active_occupancy_per_office
    ON office_occupancies(office_id) WHERE active = 1;
CREATE TABLE society_bootstraps (
    society_id INTEGER PRIMARY KEY REFERENCES societies(society_id),
    founding_mission_id INTEGER NOT NULL REFERENCES founding_missions(founding_mission_id),
    office_id INTEGER NOT NULL REFERENCES office_contracts(office_id),
    office_occupancy_id INTEGER NOT NULL REFERENCES office_occupancies(office_occupancy_id),
    hard_ceiling_micros INTEGER NOT NULL CHECK (hard_ceiling_micros > 0),
    bootstrapped_by_command_id INTEGER NOT NULL
);
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
CREATE TABLE root_authority_office_sessions (
    root_authority_office_session_id INTEGER PRIMARY KEY,
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    office_occupancy_id INTEGER NOT NULL REFERENCES office_occupancies(office_occupancy_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 11),
    started_by_command_id INTEGER NOT NULL,
    last_transition_command_id INTEGER NOT NULL
);
CREATE UNIQUE INDEX one_live_office_session_per_cycle
    ON root_authority_office_sessions(operating_cycle_id)
    WHERE lifecycle_state NOT IN (8, 10, 11);
CREATE TABLE office_turns (
    office_turn_id INTEGER PRIMARY KEY,
    root_authority_office_session_id INTEGER NOT NULL
        REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 4),
    purpose INTEGER NOT NULL CHECK (purpose BETWEEN 1 AND 4),
    opened_by_command_id INTEGER NOT NULL,
    settled_by_command_id INTEGER
);
CREATE UNIQUE INDEX one_active_office_turn_per_session
    ON office_turns(root_authority_office_session_id)
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
    -- A Reserved/Frozen row cannot debit more than its authorization. A
    -- Reconciled known overrun records actual observed spend, which may exceed
    -- that authorization and its envelope ceiling after admission is fenced.
    charged_micros INTEGER NOT NULL DEFAULT 0 CHECK (charged_micros >= 0),
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
CREATE TABLE command_create_society_identity (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), name TEXT NOT NULL);
CREATE TABLE command_install_root_authority_office (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id));
CREATE TABLE command_install_founding_mission (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    application_identity TEXT NOT NULL,
    application_name TEXT NOT NULL,
    revision_ordinal INTEGER NOT NULL CHECK (revision_ordinal > 0),
    mission_statement TEXT NOT NULL,
    source_rendering_digest BLOB NOT NULL CHECK (length(source_rendering_digest) = 32),
    source_content_object_id INTEGER
        REFERENCES content_objects(content_object_id)
);
CREATE TABLE command_install_founding_mission_principles (
    command_row_id INTEGER NOT NULL REFERENCES command_install_founding_mission(command_row_id),
    principle_ordinal INTEGER NOT NULL CHECK (principle_ordinal > 0 AND principle_ordinal <= 16),
    principle_kind INTEGER NOT NULL CHECK (principle_kind BETWEEN 1 AND 4),
    principle_text TEXT NOT NULL,
    PRIMARY KEY(command_row_id, principle_ordinal)
);
CREATE TABLE command_install_founding_mission_north_star_questions (
    command_row_id INTEGER PRIMARY KEY REFERENCES command_install_founding_mission(command_row_id),
    change_question TEXT NOT NULL,
    improvement_evidence_question TEXT NOT NULL,
    boundary_commitment_question TEXT NOT NULL,
    revisit_question TEXT NOT NULL
);
CREATE TABLE command_appoint_initial_root_authority (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), actor_display_name TEXT NOT NULL);
CREATE TABLE command_set_r0_hard_ceiling (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), ceiling_micros INTEGER NOT NULL CHECK (ceiling_micros >= 0));
CREATE TABLE command_bootstrap_society (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id));
CREATE TABLE command_admit_operating_cycle (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL);
CREATE TABLE command_start_root_authority_office_session (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), operating_cycle_id INTEGER NOT NULL);
CREATE TABLE command_record_office_session_ready (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), root_authority_office_session_id INTEGER NOT NULL);
CREATE TABLE command_record_office_session_terminal (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), root_authority_office_session_id INTEGER NOT NULL, terminal_state INTEGER NOT NULL CHECK (terminal_state IN (1, 2, 3)));
CREATE TABLE command_open_office_turn (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), root_authority_office_session_id INTEGER NOT NULL, purpose INTEGER NOT NULL CHECK (purpose BETWEEN 1 AND 4));
CREATE TABLE command_settle_office_turn (command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), office_turn_id INTEGER NOT NULL, pi_office_turn_terminal_receipt_id INTEGER NOT NULL);
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
CREATE TABLE event_society_identity_created (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), society_id INTEGER NOT NULL REFERENCES societies(society_id));
CREATE TABLE event_root_authority_office_installed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), office_id INTEGER NOT NULL REFERENCES office_contracts(office_id));
CREATE TABLE event_founding_mission_installed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    founding_mission_id INTEGER NOT NULL REFERENCES founding_missions(founding_mission_id),
    application_revision_id INTEGER NOT NULL REFERENCES application_revisions(application_revision_id)
);
CREATE TABLE event_root_authority_appointed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), office_occupancy_id INTEGER NOT NULL REFERENCES office_occupancies(office_occupancy_id), principal_id INTEGER NOT NULL REFERENCES principals(principal_id));
CREATE TABLE event_r0_hard_ceiling_set (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), society_id INTEGER NOT NULL REFERENCES societies(society_id), ceiling_micros INTEGER NOT NULL CHECK (ceiling_micros > 0));
CREATE TABLE event_society_bootstrapped (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), society_id INTEGER NOT NULL REFERENCES societies(society_id));
CREATE TABLE event_operating_cycle_state_changed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id), lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 11), admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0));
CREATE TABLE event_root_authority_office_session_started (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id));
CREATE TABLE event_root_authority_office_session_state_changed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id), lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 11));
CREATE TABLE event_office_turn_opened (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), office_turn_id INTEGER NOT NULL REFERENCES office_turns(office_turn_id), root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id), purpose INTEGER NOT NULL CHECK (purpose BETWEEN 1 AND 4));
CREATE TABLE event_office_turn_settled (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), office_turn_id INTEGER NOT NULL REFERENCES office_turns(office_turn_id), root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id), charged_delta_micros INTEGER NOT NULL CHECK (charged_delta_micros >= 0));
CREATE TABLE event_budget_reserved (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id), amount_micros INTEGER NOT NULL CHECK (amount_micros >= 0));
CREATE TABLE event_budget_reconciled (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id), observed_micros INTEGER NOT NULL CHECK (observed_micros >= 0));
CREATE TABLE event_budget_admission_frozen (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id), cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id), postmortem_id INTEGER NOT NULL REFERENCES cost_postmortems(postmortem_id), freeze_reason_kind INTEGER NOT NULL CHECK (freeze_reason_kind IN (1, 2, 3)), observed_micros INTEGER CHECK (observed_micros >= 0), reserved_micros INTEGER CHECK (reserved_micros >= 0), unknown_reason INTEGER, unavailable_reason INTEGER, CHECK ((freeze_reason_kind = 1 AND observed_micros IS NOT NULL AND reserved_micros IS NOT NULL AND unknown_reason IS NULL AND unavailable_reason IS NULL) OR (freeze_reason_kind = 2 AND observed_micros IS NULL AND reserved_micros IS NULL AND unknown_reason IS NOT NULL AND unavailable_reason IS NULL) OR (freeze_reason_kind = 3 AND observed_micros IS NULL AND reserved_micros IS NULL AND unknown_reason IS NULL AND unavailable_reason IS NOT NULL)));
CREATE TABLE event_cancellation_requested (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id), cancellation_mode INTEGER NOT NULL CHECK (cancellation_mode IN (1, 2, 3)), admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0));
CREATE TABLE event_cancellation_reconciled (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id));
CREATE TABLE event_cost_postmortem_closed (event_id INTEGER PRIMARY KEY REFERENCES events(event_id), postmortem_id INTEGER NOT NULL REFERENCES cost_postmortems(postmortem_id), budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id), operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id), resolution_kind INTEGER NOT NULL CHECK (resolution_kind IN (1, 2)), charged_micros INTEGER NOT NULL CHECK (charged_micros >= 0));
CREATE TABLE projects (
    project_id INTEGER PRIMARY KEY,
    project_name TEXT NOT NULL UNIQUE,
    founding_mission_id INTEGER NOT NULL REFERENCES founding_missions(founding_mission_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 9),
    created_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    last_transition_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE project_north_star_alignments (
    project_id INTEGER PRIMARY KEY REFERENCES projects(project_id),
    application_revision_id INTEGER NOT NULL
        REFERENCES application_revisions(application_revision_id),
    change_answer TEXT NOT NULL,
    improvement_evidence_answer TEXT NOT NULL,
    boundary_commitment_answer TEXT NOT NULL,
    revisit_answer TEXT NOT NULL,
    aligned_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
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
    founding_mission_id INTEGER NOT NULL REFERENCES founding_missions(founding_mission_id),
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
    founding_mission_id INTEGER NOT NULL REFERENCES founding_missions(founding_mission_id),
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
, assigned_reviewer_actor_instance_id INTEGER REFERENCES actor_instances(actor_instance_id), reviewer_actor_attempt_id INTEGER REFERENCES attempts(actor_attempt_id));
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
    founding_mission_id INTEGER NOT NULL REFERENCES founding_missions(founding_mission_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2, 3, 4, 5)),
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
CREATE TABLE coordination_command_provenance (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    founding_mission_id INTEGER NOT NULL REFERENCES founding_missions(founding_mission_id),
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    project_id INTEGER REFERENCES projects(project_id)
);
CREATE TABLE command_create_project (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    project_name TEXT NOT NULL,
    application_revision_id INTEGER NOT NULL,
    change_answer TEXT NOT NULL,
    improvement_evidence_answer TEXT NOT NULL,
    boundary_commitment_answer TEXT NOT NULL,
    revisit_answer TEXT NOT NULL
);
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
CREATE TABLE event_project_created (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    project_id INTEGER NOT NULL REFERENCES projects(project_id),
    application_revision_id INTEGER NOT NULL REFERENCES application_revisions(application_revision_id)
);
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
CREATE TABLE operating_cycles (
    operating_cycle_id INTEGER PRIMARY KEY,
    society_id INTEGER NOT NULL REFERENCES societies(society_id),
    founding_mission_id INTEGER NOT NULL REFERENCES founding_missions(founding_mission_id),
    office_occupancy_id INTEGER NOT NULL REFERENCES office_occupancies(office_occupancy_id),
    treatment INTEGER NOT NULL CHECK (treatment IN (1, 2, 3, 4)),
    budget_ceiling_micros INTEGER NOT NULL CHECK (budget_ceiling_micros > 0),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 11),
    admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0),
    proposed_by_command_id INTEGER NOT NULL,
    last_transition_command_id INTEGER NOT NULL
);
CREATE TABLE command_propose_operating_cycle (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    treatment INTEGER NOT NULL CHECK (treatment IN (1, 2, 3, 4)),
    budget_ceiling_micros INTEGER NOT NULL CHECK (budget_ceiling_micros >= 0)
);
CREATE TABLE event_operating_cycle_proposed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0),
    treatment INTEGER NOT NULL CHECK (treatment IN (1, 2, 3, 4)),
    budget_ceiling_micros INTEGER NOT NULL CHECK (budget_ceiling_micros > 0)
);
CREATE TABLE execution_profiles (
    execution_profile_id INTEGER PRIMARY KEY,
    profile_kind INTEGER NOT NULL UNIQUE CHECK (profile_kind IN (1, 2, 3)),
    readiness INTEGER NOT NULL CHECK (readiness IN (1, 2, 3)),
    CHECK ((profile_kind = 1 AND readiness = 1)
        OR (profile_kind = 2 AND readiness IN (2, 3))
        OR (profile_kind = 3 AND readiness = 1))
);
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
    founding_mission_id INTEGER NOT NULL REFERENCES founding_missions(founding_mission_id),
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
CREATE TABLE command_assign_adversarial_reviewer (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    adversarial_review_id INTEGER NOT NULL,
    reviewer_principal_id INTEGER NOT NULL,
    reviewer_actor_instance_id INTEGER NOT NULL,
    reviewer_actor_attempt_id INTEGER NOT NULL
);
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
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2, 3, 4, 5)),
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
CREATE TABLE command_finalize_deterministic_experiment (
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
CREATE TABLE event_deterministic_experiment_finalized (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    deterministic_experiment_id INTEGER NOT NULL REFERENCES deterministic_experiments(deterministic_experiment_id),
    terminal_state INTEGER NOT NULL CHECK (terminal_state IN (3, 4, 5))
);
CREATE TABLE capability_grants (
    capability_grant_id INTEGER PRIMARY KEY,
    principal_id INTEGER NOT NULL REFERENCES principals(principal_id),
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 99),
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
CREATE TABLE commands (
    command_row_id INTEGER PRIMARY KEY,
    command_id TEXT NOT NULL UNIQUE,
    principal_id INTEGER NOT NULL,
    capability_grant_id INTEGER NOT NULL,
    capability_kind INTEGER NOT NULL CHECK (capability_kind BETWEEN 1 AND 99),
    expected_generation INTEGER,
    command_kind INTEGER NOT NULL CHECK (command_kind BETWEEN 1 AND 99),
    request_fingerprint BLOB NOT NULL CHECK (length(request_fingerprint) = 32),
    command_status INTEGER NOT NULL CHECK (command_status IN (1, 2)),
    rejection_code INTEGER,
    accepted_event_id INTEGER,
    CHECK ((command_status = 1 AND rejection_code IS NULL AND accepted_event_id IS NOT NULL)
        OR (command_status = 2 AND rejection_code IS NOT NULL AND accepted_event_id IS NULL))
);
CREATE TABLE events (
    event_id INTEGER PRIMARY KEY,
    command_row_id INTEGER NOT NULL UNIQUE REFERENCES commands(command_row_id),
    event_kind INTEGER NOT NULL CHECK (event_kind BETWEEN 1 AND 93),
    event_sequence INTEGER NOT NULL UNIQUE CHECK (event_sequence > 0),
    event_fingerprint BLOB NOT NULL CHECK (length(event_fingerprint) = 32)
);
CREATE TABLE supervisor_epochs (
    supervisor_epoch_id INTEGER PRIMARY KEY,
    supervisor_epoch_identity TEXT NOT NULL UNIQUE,
    opened_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TRIGGER only_one_supervisor_epoch
BEFORE INSERT ON supervisor_epochs
WHEN EXISTS (SELECT 1 FROM supervisor_epochs)
BEGIN SELECT RAISE(ABORT, 'M5 has exactly one restart-fenced supervisor epoch'); END;
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
CREATE TABLE native_child_spawn_admissions (
    native_child_spawn_admission_id INTEGER PRIMARY KEY,
    operating_cycle_id INTEGER NOT NULL REFERENCES operating_cycles(operating_cycle_id),
    actor_attempt_id INTEGER REFERENCES attempts(actor_attempt_id),
    root_authority_office_session_id INTEGER REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    deterministic_experiment_id INTEGER REFERENCES deterministic_experiments(deterministic_experiment_id),
    evaluator_revision_id INTEGER REFERENCES evaluator_revisions(evaluator_revision_id),
    input_manifest_id INTEGER REFERENCES input_manifests(input_manifest_id),
    budget_reservation_id INTEGER REFERENCES budget_reservations(budget_reservation_id),
    execution_profile_id INTEGER NOT NULL REFERENCES execution_profiles(execution_profile_id),
    workspace_id INTEGER NOT NULL REFERENCES workspaces(workspace_id),
    supervisor_epoch_id INTEGER NOT NULL REFERENCES supervisor_epochs(supervisor_epoch_id),
    admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state IN (1, 2, 3)),
    admitted_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    spawned_by_command_id INTEGER REFERENCES commands(command_row_id),
    CHECK ((actor_attempt_id IS NOT NULL AND root_authority_office_session_id IS NULL
            AND deterministic_experiment_id IS NULL AND evaluator_revision_id IS NULL
            AND input_manifest_id IS NULL AND budget_reservation_id IS NOT NULL)
        OR (actor_attempt_id IS NULL AND root_authority_office_session_id IS NOT NULL
            AND deterministic_experiment_id IS NULL AND evaluator_revision_id IS NULL
            AND input_manifest_id IS NULL AND budget_reservation_id IS NOT NULL)
        OR (actor_attempt_id IS NULL AND root_authority_office_session_id IS NULL
            AND deterministic_experiment_id IS NOT NULL AND evaluator_revision_id IS NOT NULL
            AND input_manifest_id IS NOT NULL AND budget_reservation_id IS NULL)),
    UNIQUE(actor_attempt_id),
    UNIQUE(root_authority_office_session_id),
    UNIQUE(deterministic_experiment_id)
);
-- Pi session identity is a strict sidecar over a generic native admission;
-- deterministic evaluator admissions have no row here and therefore cannot
-- cross the Adapter/Create/Session protocol boundary.
CREATE TABLE pi_child_spawn_sidecars (
    native_child_spawn_admission_id INTEGER PRIMARY KEY
        REFERENCES native_child_spawn_admissions(native_child_spawn_admission_id),
    pi_session_id INTEGER NOT NULL UNIQUE REFERENCES pi_child_sessions(pi_session_id)
);
CREATE TABLE native_child_spawn_invalidations (
    native_child_spawn_admission_id INTEGER PRIMARY KEY REFERENCES native_child_spawn_admissions(native_child_spawn_admission_id),
    reason INTEGER NOT NULL CHECK (reason IN (1, 2, 3, 4)),
    invalidated_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE office_session_budget_reservations (
    root_authority_office_session_id INTEGER PRIMARY KEY REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    budget_reservation_id INTEGER NOT NULL UNIQUE REFERENCES budget_reservations(budget_reservation_id),
    bound_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE native_children (
    native_child_id INTEGER PRIMARY KEY,
    native_child_spawn_admission_id INTEGER NOT NULL UNIQUE REFERENCES native_child_spawn_admissions(native_child_spawn_admission_id),
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
CREATE TABLE pi_child_session_protocols (
    native_child_id INTEGER PRIMARY KEY REFERENCES native_children(native_child_id),
    lifecycle_state INTEGER NOT NULL CHECK (lifecycle_state BETWEEN 1 AND 5),
    create_correlation_identity TEXT,
    create_request_digest BLOB CHECK (create_request_digest IS NULL OR length(create_request_digest) = 32),
    CHECK ((lifecycle_state < 3 AND create_correlation_identity IS NULL AND create_request_digest IS NULL)
        OR (lifecycle_state >= 3 AND create_correlation_identity IS NOT NULL AND create_request_digest IS NOT NULL))
);
CREATE TRIGGER pi_protocol_requires_pi_spawn_sidecar
BEFORE INSERT ON pi_child_session_protocols
WHEN NOT EXISTS (
    SELECT 1
      FROM native_children child
      JOIN pi_child_spawn_sidecars sidecar
        ON sidecar.native_child_spawn_admission_id = child.native_child_spawn_admission_id
     WHERE child.native_child_id = NEW.native_child_id
)
BEGIN SELECT RAISE(ABORT, 'Pi protocol requires Pi native-child sidecar'); END;
CREATE TRIGGER pi_protocol_update_requires_pi_spawn_sidecar
BEFORE UPDATE OF native_child_id ON pi_child_session_protocols
WHEN NOT EXISTS (
    SELECT 1
      FROM native_children child
      JOIN pi_child_spawn_sidecars sidecar
        ON sidecar.native_child_spawn_admission_id = child.native_child_spawn_admission_id
     WHERE child.native_child_id = NEW.native_child_id
)
BEGIN SELECT RAISE(ABORT, 'Pi protocol requires Pi native-child sidecar'); END;
CREATE TRIGGER live_native_child_identity_not_reused
BEFORE INSERT ON native_children
WHEN EXISTS (
    SELECT 1 FROM native_children
    WHERE lifecycle_state != 8
       AND (direct_child_pid = NEW.direct_child_pid
            OR process_group_id = NEW.process_group_id)
)
BEGIN SELECT RAISE(ABORT, 'live or indeterminate PID/PGID may not be reused'); END;
CREATE TABLE native_child_liveness_observations (
    native_child_liveness_observation_id INTEGER PRIMARY KEY,
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    liveness INTEGER NOT NULL CHECK (liveness IN (1, 2, 3)),
    observed_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE process_signal_receipts (
    process_signal_receipt_id INTEGER PRIMARY KEY,
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
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
CREATE TABLE native_child_reap_receipts (
    native_child_reap_receipt_id INTEGER PRIMARY KEY,
    native_child_id INTEGER NOT NULL UNIQUE REFERENCES native_children(native_child_id),
    wait_status_kind INTEGER NOT NULL CHECK (wait_status_kind IN (1, 2, 3)),
    status_value INTEGER,
    group_liveness_before_cleanup INTEGER NOT NULL CHECK (group_liveness_before_cleanup IN (1, 2, 3)),
    group_liveness_after_cleanup INTEGER NOT NULL CHECK (group_liveness_after_cleanup IN (1, 2, 3)),
    reaped_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    CHECK ((wait_status_kind IN (1, 2) AND status_value IS NOT NULL AND status_value >= 0)
        OR (wait_status_kind = 3 AND status_value IS NULL))
);
CREATE TABLE native_child_recovery_receipts (
    native_child_recovery_receipt_id INTEGER PRIMARY KEY,
    native_child_id INTEGER NOT NULL UNIQUE REFERENCES native_children(native_child_id),
    observation INTEGER NOT NULL CHECK (observation = 1),
    group_liveness_after_restart INTEGER NOT NULL CHECK (group_liveness_after_restart IN (1, 2, 3)),
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE pi_abort_control_receipts (
    pi_abort_control_receipt_id INTEGER PRIMARY KEY,
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    cancellation_propagation_id INTEGER NOT NULL REFERENCES cancellation_propagations(cancellation_propagation_id),
    correlation_identity TEXT NOT NULL,
    abort_command_digest BLOB NOT NULL CHECK (length(abort_command_digest) = 32),
    physical_write_outcome INTEGER NOT NULL CHECK (physical_write_outcome IN (1, 2, 3, 4)),
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE(native_child_id, cancellation_propagation_id),
    UNIQUE(correlation_identity)
);
CREATE TRIGGER child_liveness_reappearance_marks_containment
AFTER INSERT ON native_child_liveness_observations
WHEN NEW.liveness IN (1, 3) AND EXISTS (
    SELECT 1 FROM native_child_liveness_observations WHERE native_child_id = NEW.native_child_id AND liveness = 2
    UNION ALL SELECT 1 FROM process_signal_receipts WHERE native_child_id = NEW.native_child_id AND observed_liveness = 2
    UNION ALL SELECT 1 FROM native_child_reap_receipts WHERE native_child_id = NEW.native_child_id AND (group_liveness_before_cleanup = 2 OR group_liveness_after_cleanup = 2)
)
BEGIN
    UPDATE native_children SET lifecycle_state = 7, terminal_disposition = 7,
        last_transition_command_id = NEW.observed_by_command_id
     WHERE native_child_id = NEW.native_child_id;
END;
CREATE TRIGGER signal_liveness_reappearance_marks_containment
AFTER INSERT ON process_signal_receipts
WHEN NEW.observed_liveness IN (1, 3) AND EXISTS (
    SELECT 1 FROM native_child_liveness_observations WHERE native_child_id = NEW.native_child_id AND liveness = 2
    UNION ALL SELECT 1 FROM process_signal_receipts WHERE native_child_id = NEW.native_child_id AND observed_liveness = 2
    UNION ALL SELECT 1 FROM native_child_reap_receipts WHERE native_child_id = NEW.native_child_id AND (group_liveness_before_cleanup = 2 OR group_liveness_after_cleanup = 2)
)
BEGIN
    UPDATE native_children SET lifecycle_state = 7, terminal_disposition = 7,
        last_transition_command_id = NEW.recorded_by_command_id
     WHERE native_child_id = NEW.native_child_id;
END;
CREATE TRIGGER reap_liveness_reappearance_marks_containment
AFTER INSERT ON native_child_reap_receipts
WHEN (NEW.group_liveness_before_cleanup IN (1, 3) OR NEW.group_liveness_after_cleanup IN (1, 3)) AND EXISTS (
    SELECT 1 FROM native_child_liveness_observations WHERE native_child_id = NEW.native_child_id AND liveness = 2
    UNION ALL SELECT 1 FROM process_signal_receipts WHERE native_child_id = NEW.native_child_id AND observed_liveness = 2
)
BEGIN
    UPDATE native_children SET lifecycle_state = 7, terminal_disposition = 7,
        last_transition_command_id = NEW.reaped_by_command_id
     WHERE native_child_id = NEW.native_child_id;
END;
CREATE TABLE native_child_stream_seals (
    native_child_stream_seal_id INTEGER PRIMARY KEY,
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    stream_kind INTEGER NOT NULL CHECK (stream_kind IN (1, 2, 3, 4)),
    full_observed_digest BLOB NOT NULL CHECK (length(full_observed_digest) = 32),
    retained_content_object_id INTEGER NOT NULL REFERENCES content_objects(content_object_id),
    completeness INTEGER NOT NULL CHECK (completeness IN (1, 2, 3)),
    sealed_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE(native_child_id, stream_kind)
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
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    PRIMARY KEY(cancellation_propagation_id, native_child_id)
);
CREATE TABLE cancellation_propagation_targets (
    cancellation_propagation_target_id INTEGER PRIMARY KEY,
    cancellation_propagation_id INTEGER NOT NULL REFERENCES cancellation_propagations(cancellation_propagation_id),
    actor_attempt_id INTEGER REFERENCES attempts(actor_attempt_id),
    root_authority_office_session_id INTEGER REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    deterministic_experiment_id INTEGER REFERENCES deterministic_experiments(deterministic_experiment_id),
    native_child_id INTEGER REFERENCES native_children(native_child_id),
    target_disposition INTEGER NOT NULL CHECK (target_disposition IN (1, 2, 3, 4, 5, 6, 7)),
    CHECK ((actor_attempt_id IS NOT NULL AND root_authority_office_session_id IS NULL AND deterministic_experiment_id IS NULL)
        OR (actor_attempt_id IS NULL AND root_authority_office_session_id IS NOT NULL AND deterministic_experiment_id IS NULL)
        OR (actor_attempt_id IS NULL AND root_authority_office_session_id IS NULL AND deterministic_experiment_id IS NOT NULL)),
    UNIQUE(cancellation_propagation_id, actor_attempt_id),
    UNIQUE(cancellation_propagation_id, root_authority_office_session_id),
    UNIQUE(cancellation_propagation_id, deterministic_experiment_id)
);
CREATE TABLE command_admit_pi_child_spawn (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    actor_attempt_id INTEGER,
    root_authority_office_session_id INTEGER,
    budget_reservation_id INTEGER NOT NULL,
    execution_profile_id INTEGER NOT NULL,
    native_workspace_id TEXT NOT NULL,
    canonical_workspace_path TEXT NOT NULL,
    supervisor_epoch_id INTEGER NOT NULL,
    supervisor_epoch_identity TEXT NOT NULL,
    pi_session_identity TEXT NOT NULL,
    spawn_nonce TEXT NOT NULL,
    CHECK ((actor_attempt_id IS NOT NULL AND root_authority_office_session_id IS NULL)
        OR (actor_attempt_id IS NULL AND root_authority_office_session_id IS NOT NULL))
);
CREATE TABLE command_record_inert_pi_child_spawn (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    native_child_spawn_admission_id INTEGER NOT NULL,
    child_identity TEXT NOT NULL,
    direct_child_pid INTEGER NOT NULL CHECK (direct_child_pid > 0),
    process_group_id INTEGER NOT NULL CHECK (process_group_id > 0)
);
CREATE TABLE command_admit_deterministic_evaluator_native_child (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    operating_cycle_id INTEGER NOT NULL,
    deterministic_experiment_id INTEGER NOT NULL,
    evaluator_revision_id INTEGER NOT NULL,
    input_manifest_id INTEGER NOT NULL,
    execution_profile_id INTEGER NOT NULL,
    native_workspace_id TEXT NOT NULL,
    canonical_workspace_path TEXT NOT NULL,
    supervisor_epoch_id INTEGER NOT NULL,
    supervisor_epoch_identity TEXT NOT NULL
);
CREATE TABLE command_record_deterministic_evaluator_native_child_spawn (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    native_child_spawn_admission_id INTEGER NOT NULL,
    child_identity TEXT NOT NULL,
    direct_child_pid INTEGER NOT NULL CHECK (direct_child_pid > 0),
    process_group_id INTEGER NOT NULL CHECK (process_group_id > 0)
);
CREATE TABLE command_record_pi_adapter_ready (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    native_child_id INTEGER NOT NULL, pi_session_identity TEXT NOT NULL, spawn_nonce TEXT NOT NULL
);
CREATE TABLE command_authorize_pi_create_session (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), native_child_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL, create_request_digest BLOB NOT NULL CHECK (length(create_request_digest) = 32)
);
CREATE TABLE command_record_pi_create_session_delivery (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), native_child_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL, create_request_digest BLOB NOT NULL CHECK (length(create_request_digest) = 32)
);
CREATE TABLE command_record_pi_session_ready (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), native_child_id INTEGER NOT NULL, pi_session_identity TEXT NOT NULL
);
CREATE TABLE command_record_pi_abort_control_delivery (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    native_child_id INTEGER NOT NULL,
    cancellation_propagation_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL,
    abort_command_digest BLOB NOT NULL CHECK (length(abort_command_digest) = 32),
    physical_write_outcome INTEGER NOT NULL CHECK (physical_write_outcome IN (1, 2, 3, 4))
);
CREATE TABLE command_record_child_stream_seal (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), native_child_id INTEGER NOT NULL,
    stream_kind INTEGER NOT NULL CHECK (stream_kind IN (1, 2, 3, 4)),
    full_observed_digest BLOB NOT NULL CHECK (length(full_observed_digest) = 32),
    retained_content_object_id INTEGER NOT NULL, completeness INTEGER NOT NULL CHECK (completeness IN (1, 2, 3))
);
CREATE TABLE command_record_child_process_liveness (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), native_child_id INTEGER NOT NULL, liveness INTEGER NOT NULL CHECK (liveness IN (1, 2, 3))
);
CREATE TABLE command_record_process_signal_receipt (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), native_child_id INTEGER NOT NULL,
    signal_action INTEGER NOT NULL CHECK (signal_action IN (1, 2, 3)), delivery INTEGER NOT NULL CHECK (delivery IN (1, 2, 3, 4)),
    observed_liveness INTEGER NOT NULL CHECK (observed_liveness IN (1, 2, 3)),
    cause_kind INTEGER NOT NULL CHECK (cause_kind IN (1, 2)),
    cancellation_propagation_id INTEGER,
    CHECK ((cause_kind = 1 AND cancellation_propagation_id IS NOT NULL)
        OR (cause_kind = 2 AND cancellation_propagation_id IS NULL))
);
CREATE TABLE command_record_direct_child_reap (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), native_child_id INTEGER NOT NULL,
    wait_status_kind INTEGER NOT NULL CHECK (wait_status_kind IN (1, 2, 3)), status_value INTEGER,
    group_liveness_before_cleanup INTEGER NOT NULL CHECK (group_liveness_before_cleanup IN (1, 2, 3)),
    group_liveness_after_cleanup INTEGER NOT NULL CHECK (group_liveness_after_cleanup IN (1, 2, 3)),
    CHECK ((wait_status_kind IN (1, 2) AND status_value IS NOT NULL AND status_value >= 0)
        OR (wait_status_kind = 3 AND status_value IS NULL))
);
CREATE TABLE command_record_child_recovery (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), native_child_id INTEGER NOT NULL,
    observation INTEGER NOT NULL CHECK (observation = 1),
    group_liveness_after_restart INTEGER NOT NULL CHECK (group_liveness_after_restart IN (1, 2, 3))
);
CREATE TABLE command_finalize_child_process (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id), native_child_id INTEGER NOT NULL
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
CREATE TABLE command_record_native_child_not_spawned (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    native_child_spawn_admission_id INTEGER NOT NULL,
    reason INTEGER NOT NULL CHECK (reason IN (1, 2, 3, 4))
);
CREATE TABLE event_pi_child_spawn_admitted (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), native_child_spawn_admission_id INTEGER NOT NULL REFERENCES native_child_spawn_admissions(native_child_spawn_admission_id),
    actor_attempt_id INTEGER, root_authority_office_session_id INTEGER, budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id),
    CHECK ((actor_attempt_id IS NOT NULL AND root_authority_office_session_id IS NULL)
        OR (actor_attempt_id IS NULL AND root_authority_office_session_id IS NOT NULL))
);
CREATE TABLE event_inert_pi_child_spawn_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id), native_child_spawn_admission_id INTEGER NOT NULL REFERENCES native_child_spawn_admissions(native_child_spawn_admission_id)
);
CREATE TABLE event_deterministic_evaluator_native_child_admitted (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    native_child_spawn_admission_id INTEGER NOT NULL REFERENCES native_child_spawn_admissions(native_child_spawn_admission_id),
    deterministic_experiment_id INTEGER NOT NULL REFERENCES deterministic_experiments(deterministic_experiment_id),
    evaluator_revision_id INTEGER NOT NULL REFERENCES evaluator_revisions(evaluator_revision_id),
    input_manifest_id INTEGER NOT NULL REFERENCES input_manifests(input_manifest_id)
);
CREATE TABLE event_deterministic_evaluator_native_child_spawn_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    native_child_spawn_admission_id INTEGER NOT NULL REFERENCES native_child_spawn_admissions(native_child_spawn_admission_id)
);
CREATE TABLE event_pi_adapter_ready_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id), pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id)
);
CREATE TABLE event_pi_create_session_authorized (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id)
);
CREATE TABLE event_pi_create_session_delivery_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id)
);
CREATE TABLE event_pi_session_ready_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id), pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id)
);
CREATE TABLE event_pi_abort_control_delivery_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    pi_abort_control_receipt_id INTEGER NOT NULL REFERENCES pi_abort_control_receipts(pi_abort_control_receipt_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    cancellation_propagation_id INTEGER NOT NULL REFERENCES cancellation_propagations(cancellation_propagation_id),
    correlation_identity TEXT NOT NULL,
    abort_command_digest BLOB NOT NULL CHECK (length(abort_command_digest) = 32),
    physical_write_outcome INTEGER NOT NULL CHECK (physical_write_outcome IN (1, 2, 3, 4))
);
CREATE TABLE event_child_stream_sealed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), native_child_stream_seal_id INTEGER NOT NULL REFERENCES native_child_stream_seals(native_child_stream_seal_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id), stream_kind INTEGER NOT NULL CHECK (stream_kind IN (1, 2, 3, 4)), completeness INTEGER NOT NULL CHECK (completeness IN (1, 2, 3))
);
CREATE TABLE event_child_process_liveness_observed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), native_child_liveness_observation_id INTEGER NOT NULL REFERENCES native_child_liveness_observations(native_child_liveness_observation_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id), liveness INTEGER NOT NULL CHECK (liveness IN (1, 2, 3))
);
CREATE TABLE event_process_signal_receipt_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), process_signal_receipt_id INTEGER NOT NULL REFERENCES process_signal_receipts(process_signal_receipt_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id), signal_action INTEGER NOT NULL CHECK (signal_action IN (1, 2, 3)), delivery INTEGER NOT NULL CHECK (delivery IN (1, 2, 3, 4)),
    observed_liveness INTEGER NOT NULL CHECK (observed_liveness IN (1, 2, 3)),
    cause_kind INTEGER NOT NULL CHECK (cause_kind IN (1, 2)), cancellation_propagation_id INTEGER,
    CHECK ((cause_kind = 1 AND cancellation_propagation_id IS NOT NULL)
        OR (cause_kind = 2 AND cancellation_propagation_id IS NULL)),
    CHECK ((delivery IN (2, 3) AND observed_liveness = 2)
        OR (delivery = 4 AND observed_liveness = 3)
        OR delivery = 1)
);
CREATE TABLE event_direct_child_reaped (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), native_child_reap_receipt_id INTEGER NOT NULL REFERENCES native_child_reap_receipts(native_child_reap_receipt_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id), wait_status_kind INTEGER NOT NULL CHECK (wait_status_kind IN (1, 2, 3)), status_value INTEGER,
    group_liveness_before_cleanup INTEGER NOT NULL CHECK (group_liveness_before_cleanup IN (1, 2, 3)),
    group_liveness_after_cleanup INTEGER NOT NULL CHECK (group_liveness_after_cleanup IN (1, 2, 3)),
    CHECK ((wait_status_kind IN (1, 2) AND status_value IS NOT NULL AND status_value >= 0)
        OR (wait_status_kind = 3 AND status_value IS NULL))
);
CREATE TABLE event_child_recovery_observed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), native_child_recovery_receipt_id INTEGER NOT NULL REFERENCES native_child_recovery_receipts(native_child_recovery_receipt_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id), observation INTEGER NOT NULL CHECK (observation = 1),
    group_liveness_after_restart INTEGER NOT NULL CHECK (group_liveness_after_restart IN (1, 2, 3))
);
CREATE TABLE event_child_process_finalized (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id), native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id), disposition INTEGER NOT NULL CHECK (disposition IN (1, 4, 5))
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
CREATE TABLE event_native_child_spawn_invalidated (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    native_child_spawn_admission_id INTEGER NOT NULL REFERENCES native_child_spawn_admissions(native_child_spawn_admission_id),
    reason INTEGER NOT NULL CHECK (reason IN (1, 2, 3, 4))
);

-- M6 canonical foundation: one persistent Office-session reservation is charged
-- through exact cumulative Pi turn checkpoints. These rows are operational
-- receipt/provenance, not a generic SDK event store or routine turn reserve.
CREATE TABLE office_turn_budget_checkpoints (
    office_turn_id INTEGER PRIMARY KEY REFERENCES office_turns(office_turn_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id),
    baseline_cumulative_micros INTEGER NOT NULL CHECK (baseline_cumulative_micros >= 0),
    authorized_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    settled_cumulative_micros INTEGER CHECK (settled_cumulative_micros >= baseline_cumulative_micros),
    settled_by_command_id INTEGER REFERENCES commands(command_row_id),
    CHECK ((settled_cumulative_micros IS NULL AND settled_by_command_id IS NULL)
        OR (settled_cumulative_micros IS NOT NULL AND settled_by_command_id IS NOT NULL))
);
CREATE TABLE pi_office_turn_prompt_authorizations (
    pi_office_turn_prompt_authorization_id INTEGER PRIMARY KEY,
    office_turn_id INTEGER NOT NULL UNIQUE REFERENCES office_turns(office_turn_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id),
    budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id),
    correlation_identity TEXT NOT NULL UNIQUE,
    prompt_content_object_id INTEGER NOT NULL REFERENCES content_objects(content_object_id),
    prompt_digest BLOB NOT NULL CHECK (length(prompt_digest) = 32),
    frontier_event_id INTEGER NOT NULL REFERENCES events(event_id),
    admission_generation INTEGER NOT NULL CHECK (admission_generation >= 0),
    office_turn_purpose INTEGER NOT NULL CHECK (office_turn_purpose BETWEEN 1 AND 4),
    authorized_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE pi_office_turn_prompt_deliveries (
    office_turn_id INTEGER PRIMARY KEY REFERENCES office_turns(office_turn_id),
    pi_office_turn_prompt_authorization_id INTEGER NOT NULL UNIQUE REFERENCES pi_office_turn_prompt_authorizations(pi_office_turn_prompt_authorization_id),
    correlation_identity TEXT NOT NULL,
    prompt_digest BLOB NOT NULL CHECK (length(prompt_digest) = 32),
    delivered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE pi_office_turn_prompt_acceptances (
    office_turn_id INTEGER PRIMARY KEY REFERENCES office_turns(office_turn_id),
    pi_office_turn_prompt_authorization_id INTEGER NOT NULL UNIQUE REFERENCES pi_office_turn_prompt_authorizations(pi_office_turn_prompt_authorization_id),
    command_result_sequence INTEGER NOT NULL CHECK (command_result_sequence > 0),
    accepted_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id)
);
CREATE TABLE pi_office_turn_usage_receipts (
    pi_office_turn_usage_receipt_id INTEGER PRIMARY KEY,
    office_turn_id INTEGER NOT NULL REFERENCES office_turns(office_turn_id),
    pi_office_turn_prompt_authorization_id INTEGER NOT NULL REFERENCES pi_office_turn_prompt_authorizations(pi_office_turn_prompt_authorization_id),
    pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id),
    correlation_identity TEXT NOT NULL,
    protocol_sequence INTEGER NOT NULL CHECK (protocol_sequence > 0),
    input_tokens INTEGER NOT NULL CHECK (input_tokens >= 0),
    output_tokens INTEGER NOT NULL CHECK (output_tokens >= 0),
    cache_read_tokens INTEGER NOT NULL CHECK (cache_read_tokens >= 0),
    cache_write_tokens INTEGER NOT NULL CHECK (cache_write_tokens >= 0),
    total_tokens INTEGER NOT NULL CHECK (total_tokens >= 0),
    provider_cost_binary64 BLOB NOT NULL CHECK (length(provider_cost_binary64) = 8),
    cumulative_ceiling_micros INTEGER NOT NULL CHECK (cumulative_ceiling_micros >= 0),
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE (pi_session_id, protocol_sequence)
);
CREATE TABLE pi_office_turn_usage_failures (
    pi_office_turn_usage_failure_id INTEGER PRIMARY KEY,
    office_turn_id INTEGER NOT NULL UNIQUE REFERENCES office_turns(office_turn_id),
    pi_office_turn_prompt_authorization_id INTEGER NOT NULL UNIQUE REFERENCES pi_office_turn_prompt_authorizations(pi_office_turn_prompt_authorization_id),
    pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id),
    correlation_identity TEXT NOT NULL,
    protocol_sequence INTEGER NOT NULL CHECK (protocol_sequence > 0),
    failure_kind INTEGER NOT NULL CHECK (failure_kind IN (1, 2)),
    unknown_reason INTEGER,
    unavailable_reason INTEGER,
    budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id),
    cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id),
    cost_postmortem_id INTEGER NOT NULL REFERENCES cost_postmortems(postmortem_id),
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE (pi_session_id, protocol_sequence),
    CHECK ((failure_kind = 1 AND unknown_reason IN (1, 2, 3) AND unavailable_reason IS NULL)
        OR (failure_kind = 2 AND unknown_reason IS NULL AND unavailable_reason IN (1, 2, 3, 4)))
);
CREATE TABLE pi_office_turn_terminal_receipts (
    pi_office_turn_terminal_receipt_id INTEGER PRIMARY KEY,
    office_turn_id INTEGER NOT NULL UNIQUE REFERENCES office_turns(office_turn_id),
    pi_office_turn_prompt_authorization_id INTEGER NOT NULL UNIQUE REFERENCES pi_office_turn_prompt_authorizations(pi_office_turn_prompt_authorization_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id),
    correlation_identity TEXT NOT NULL,
    terminal_evidence_kind INTEGER NOT NULL CHECK (terminal_evidence_kind IN (1, 2)),
    agent_settled_sequence INTEGER CHECK (agent_settled_sequence > 0),
    final_accounting_sequence INTEGER NOT NULL CHECK (final_accounting_sequence > 0),
    settled_sequence INTEGER NOT NULL CHECK (settled_sequence > 0),
    final_usage_receipt_id INTEGER REFERENCES pi_office_turn_usage_receipts(pi_office_turn_usage_receipt_id),
    final_usage_failure_id INTEGER REFERENCES pi_office_turn_usage_failures(pi_office_turn_usage_failure_id),
    disposition INTEGER NOT NULL CHECK (disposition BETWEEN 1 AND 6),
    assistant_outcome INTEGER NOT NULL CHECK (assistant_outcome BETWEEN 1 AND 6),
    transcript_disposition INTEGER NOT NULL CHECK (transcript_disposition = 1),
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    CHECK (((terminal_evidence_kind = 1
                AND agent_settled_sequence IS NOT NULL
                AND agent_settled_sequence < final_accounting_sequence)
            OR (terminal_evidence_kind = 2 AND agent_settled_sequence IS NULL))
       AND final_accounting_sequence + 1 = settled_sequence),
    CHECK ((final_usage_receipt_id IS NOT NULL AND final_usage_failure_id IS NULL)
        OR (final_usage_receipt_id IS NULL AND final_usage_failure_id IS NOT NULL)),
    CHECK ((terminal_evidence_kind = 1 AND assistant_outcome BETWEEN 1 AND 4)
        OR (terminal_evidence_kind = 2 AND assistant_outcome IN (5, 6))),
    CHECK (terminal_evidence_kind != 2 OR final_usage_receipt_id IS NOT NULL),
    CHECK ((disposition = 1 AND assistant_outcome = 1)
        OR (disposition = 2 AND assistant_outcome = 2)
        OR (disposition = 3 AND assistant_outcome = 3)
        OR (disposition = 4 AND assistant_outcome = 4)
        OR (disposition = 5 AND assistant_outcome = 5)
        OR (disposition = 6 AND assistant_outcome = 6))
);

CREATE TRIGGER pi_usage_receipt_sequence_not_failure
BEFORE INSERT ON pi_office_turn_usage_receipts
WHEN EXISTS (
    SELECT 1 FROM pi_office_turn_usage_failures failure
    WHERE failure.pi_session_id = NEW.pi_session_id
      AND failure.protocol_sequence = NEW.protocol_sequence
)
BEGIN SELECT RAISE(ABORT, 'Pi usage/failure sequence collision'); END;
CREATE TRIGGER pi_usage_failure_sequence_not_receipt
BEFORE INSERT ON pi_office_turn_usage_failures
WHEN EXISTS (
    SELECT 1 FROM pi_office_turn_usage_receipts receipt
    WHERE receipt.pi_session_id = NEW.pi_session_id
      AND receipt.protocol_sequence = NEW.protocol_sequence
)
BEGIN SELECT RAISE(ABORT, 'Pi usage/failure sequence collision'); END;

CREATE TABLE command_authorize_pi_office_turn_prompt (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    office_turn_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL,
    prompt_content_object_id INTEGER NOT NULL,
    prompt_digest BLOB NOT NULL CHECK (length(prompt_digest) = 32),
    frontier_event_id INTEGER NOT NULL
);
CREATE TABLE command_record_pi_office_turn_prompt_delivery (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    office_turn_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL,
    prompt_digest BLOB NOT NULL CHECK (length(prompt_digest) = 32)
);
CREATE TABLE command_record_pi_office_turn_prompt_accepted (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    office_turn_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL,
    command_result_sequence INTEGER NOT NULL CHECK (command_result_sequence > 0)
);
CREATE TABLE command_record_pi_office_turn_usage (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    office_turn_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL,
    protocol_sequence INTEGER NOT NULL CHECK (protocol_sequence > 0),
    input_tokens INTEGER NOT NULL CHECK (input_tokens >= 0),
    output_tokens INTEGER NOT NULL CHECK (output_tokens >= 0),
    cache_read_tokens INTEGER NOT NULL CHECK (cache_read_tokens >= 0),
    cache_write_tokens INTEGER NOT NULL CHECK (cache_write_tokens >= 0),
    total_tokens INTEGER NOT NULL CHECK (total_tokens >= 0),
    provider_cost_binary64 BLOB NOT NULL CHECK (length(provider_cost_binary64) = 8),
    cumulative_ceiling_micros INTEGER NOT NULL CHECK (cumulative_ceiling_micros >= 0)
);
CREATE TABLE command_record_pi_office_turn_usage_failure (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    office_turn_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL,
    protocol_sequence INTEGER NOT NULL CHECK (protocol_sequence > 0),
    failure_kind INTEGER NOT NULL CHECK (failure_kind IN (1, 2)),
    unknown_reason INTEGER,
    unavailable_reason INTEGER,
    CHECK ((failure_kind = 1 AND unknown_reason IN (1, 2, 3) AND unavailable_reason IS NULL)
        OR (failure_kind = 2 AND unknown_reason IS NULL AND unavailable_reason IN (1, 2, 3, 4)))
);
CREATE TABLE command_record_pi_office_turn_terminal (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    office_turn_id INTEGER NOT NULL,
    correlation_identity TEXT NOT NULL,
    terminal_evidence_kind INTEGER NOT NULL CHECK (terminal_evidence_kind IN (1, 2)),
    agent_settled_sequence INTEGER CHECK (agent_settled_sequence > 0),
    final_accounting_sequence INTEGER NOT NULL CHECK (final_accounting_sequence > 0),
    settled_sequence INTEGER NOT NULL CHECK (settled_sequence > 0),
    disposition INTEGER NOT NULL CHECK (disposition BETWEEN 1 AND 6),
    assistant_outcome INTEGER NOT NULL CHECK (assistant_outcome BETWEEN 1 AND 6),
    transcript_disposition INTEGER NOT NULL CHECK (transcript_disposition = 1),
    CHECK ((terminal_evidence_kind = 1 AND agent_settled_sequence IS NOT NULL)
        OR (terminal_evidence_kind = 2 AND agent_settled_sequence IS NULL))
);

CREATE TABLE event_pi_office_turn_prompt_authorized (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    pi_office_turn_prompt_authorization_id INTEGER NOT NULL REFERENCES pi_office_turn_prompt_authorizations(pi_office_turn_prompt_authorization_id),
    office_turn_id INTEGER NOT NULL REFERENCES office_turns(office_turn_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    correlation_identity TEXT NOT NULL,
    budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id)
);
CREATE TABLE event_pi_office_turn_prompt_delivered (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    office_turn_id INTEGER NOT NULL REFERENCES office_turns(office_turn_id),
    correlation_identity TEXT NOT NULL
);
CREATE TABLE event_pi_office_turn_prompt_accepted (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    office_turn_id INTEGER NOT NULL REFERENCES office_turns(office_turn_id),
    correlation_identity TEXT NOT NULL,
    command_result_sequence INTEGER NOT NULL CHECK (command_result_sequence > 0)
);
CREATE TABLE event_pi_office_turn_usage_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    pi_office_turn_usage_receipt_id INTEGER NOT NULL REFERENCES pi_office_turn_usage_receipts(pi_office_turn_usage_receipt_id),
    office_turn_id INTEGER NOT NULL REFERENCES office_turns(office_turn_id),
    protocol_sequence INTEGER NOT NULL CHECK (protocol_sequence > 0),
    cumulative_ceiling_micros INTEGER NOT NULL CHECK (cumulative_ceiling_micros >= 0)
);
CREATE TABLE event_pi_office_turn_usage_frozen (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    office_turn_id INTEGER NOT NULL REFERENCES office_turns(office_turn_id),
    budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id),
    cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id),
    cost_postmortem_id INTEGER NOT NULL REFERENCES cost_postmortems(postmortem_id),
    failure_kind INTEGER NOT NULL CHECK (failure_kind IN (1, 2)),
    unknown_reason INTEGER,
    unavailable_reason INTEGER,
    CHECK ((failure_kind = 1 AND unknown_reason IN (1, 2, 3) AND unavailable_reason IS NULL)
        OR (failure_kind = 2 AND unknown_reason IS NULL AND unavailable_reason IN (1, 2, 3, 4)))
);
CREATE TABLE event_pi_office_turn_terminal_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    pi_office_turn_terminal_receipt_id INTEGER NOT NULL REFERENCES pi_office_turn_terminal_receipts(pi_office_turn_terminal_receipt_id),
    office_turn_id INTEGER NOT NULL REFERENCES office_turns(office_turn_id),
    disposition INTEGER NOT NULL CHECK (disposition BETWEEN 1 AND 6),
    assistant_outcome INTEGER NOT NULL CHECK (assistant_outcome BETWEEN 1 AND 6),
    CHECK ((disposition = 1 AND assistant_outcome = 1)
        OR (disposition = 2 AND assistant_outcome = 2)
        OR (disposition = 3 AND assistant_outcome = 3)
        OR (disposition = 4 AND assistant_outcome = 4)
        OR (disposition = 5 AND assistant_outcome = 5)
        OR (disposition = 6 AND assistant_outcome = 6))
);
-- M7 keeps the four peer Dispose boundaries independently durable. The
-- trusted daemon may crash between any frames, so only the terminal receipt
-- carrying final Known usage can release a Root Authority Office parent
-- reservation. An unavailable accounting frame is a frozen recovery duty,
-- not a synthetic Disposed receipt.
CREATE TABLE pi_office_session_dispose_authorizations (
    root_authority_office_session_id INTEGER PRIMARY KEY
        REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id),
    correlation_identity TEXT NOT NULL,
    authorized_generation INTEGER NOT NULL CHECK (authorized_generation >= 0),
    authorized_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE (pi_session_id, correlation_identity)
);
CREATE TABLE pi_office_session_dispose_deliveries (
    root_authority_office_session_id INTEGER PRIMARY KEY
        REFERENCES pi_office_session_dispose_authorizations(root_authority_office_session_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id),
    correlation_identity TEXT NOT NULL,
    delivered_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE (pi_session_id, correlation_identity)
);
CREATE TABLE pi_office_session_dispose_acceptances (
    root_authority_office_session_id INTEGER PRIMARY KEY
        REFERENCES pi_office_session_dispose_deliveries(root_authority_office_session_id),
    command_result_sequence INTEGER NOT NULL CHECK (command_result_sequence > 0),
    accepted_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE (root_authority_office_session_id, command_result_sequence)
);
CREATE TABLE pi_office_session_dispose_usage_receipts (
    root_authority_office_session_id INTEGER PRIMARY KEY
        REFERENCES pi_office_session_dispose_acceptances(root_authority_office_session_id),
    pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id),
    correlation_identity TEXT NOT NULL,
    protocol_sequence INTEGER NOT NULL CHECK (protocol_sequence > 0),
    input_tokens INTEGER NOT NULL CHECK (input_tokens >= 0),
    output_tokens INTEGER NOT NULL CHECK (output_tokens >= 0),
    cache_read_tokens INTEGER NOT NULL CHECK (cache_read_tokens >= 0),
    cache_write_tokens INTEGER NOT NULL CHECK (cache_write_tokens >= 0),
    total_tokens INTEGER NOT NULL CHECK (total_tokens >= 0),
    provider_cost_binary64 BLOB NOT NULL CHECK (length(provider_cost_binary64) = 8),
    cumulative_ceiling_micros INTEGER NOT NULL CHECK (cumulative_ceiling_micros >= 0),
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE (pi_session_id, protocol_sequence)
);
CREATE TABLE pi_office_session_dispose_usage_failures (
    root_authority_office_session_id INTEGER PRIMARY KEY
        REFERENCES pi_office_session_dispose_acceptances(root_authority_office_session_id),
    pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id),
    correlation_identity TEXT NOT NULL,
    protocol_sequence INTEGER NOT NULL CHECK (protocol_sequence > 0),
    failure_kind INTEGER NOT NULL CHECK (failure_kind IN (1, 2)),
    unknown_reason INTEGER,
    unavailable_reason INTEGER,
    budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id),
    cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id),
    cost_postmortem_id INTEGER NOT NULL REFERENCES cost_postmortems(postmortem_id),
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE (pi_session_id, protocol_sequence),
    CHECK ((failure_kind = 1 AND unknown_reason IN (1, 2, 3) AND unavailable_reason IS NULL)
        OR (failure_kind = 2 AND unknown_reason IS NULL AND unavailable_reason IN (1, 2, 3, 4)))
);
CREATE TABLE pi_office_session_dispose_receipts (
    pi_office_session_dispose_receipt_id INTEGER PRIMARY KEY,
    root_authority_office_session_id INTEGER NOT NULL UNIQUE
        REFERENCES pi_office_session_dispose_usage_receipts(root_authority_office_session_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    pi_session_id INTEGER NOT NULL REFERENCES pi_child_sessions(pi_session_id),
    correlation_identity TEXT NOT NULL,
    disposed_sequence INTEGER NOT NULL CHECK (disposed_sequence > 0),
    transcript_kind INTEGER NOT NULL CHECK (transcript_kind IN (1, 2)),
    session_file TEXT NOT NULL,
    session_file_digest BLOB CHECK (session_file_digest IS NULL OR length(session_file_digest) = 32),
    transcript_content_object_id INTEGER REFERENCES content_objects(content_object_id),
    first_user_prompt_kind INTEGER,
    first_user_prompt_digest BLOB CHECK (first_user_prompt_digest IS NULL OR length(first_user_prompt_digest) = 32),
    budget_disposition_kind INTEGER NOT NULL CHECK (budget_disposition_kind IN (1, 2)),
    cancellation_request_id INTEGER REFERENCES cancellation_requests(cancellation_request_id),
    cost_postmortem_id INTEGER REFERENCES cost_postmortems(postmortem_id),
    recorded_by_command_id INTEGER NOT NULL REFERENCES commands(command_row_id),
    UNIQUE (pi_session_id, disposed_sequence),
    CHECK ((transcript_kind = 1
                AND session_file_digest IS NOT NULL
                AND transcript_content_object_id IS NOT NULL
                AND first_user_prompt_kind IN (1, 2)
                AND ((first_user_prompt_kind = 1 AND first_user_prompt_digest IS NULL)
                    OR (first_user_prompt_kind = 2 AND first_user_prompt_digest IS NOT NULL)))
        OR (transcript_kind = 2
                AND session_file_digest IS NULL
                AND transcript_content_object_id IS NULL
                AND first_user_prompt_kind IS NULL
                AND first_user_prompt_digest IS NULL)),
    CHECK ((budget_disposition_kind = 1
                AND cancellation_request_id IS NULL AND cost_postmortem_id IS NULL)
        OR (budget_disposition_kind = 2
                AND cancellation_request_id IS NOT NULL AND cost_postmortem_id IS NOT NULL))
);
CREATE TABLE command_authorize_pi_office_session_dispose (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    correlation_identity TEXT NOT NULL
);
CREATE TABLE command_record_pi_office_session_dispose_delivery (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    correlation_identity TEXT NOT NULL
);
CREATE TABLE command_record_pi_office_session_dispose_accepted (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    correlation_identity TEXT NOT NULL,
    command_result_sequence INTEGER NOT NULL CHECK (command_result_sequence > 0)
);
CREATE TABLE command_record_pi_office_session_dispose_usage (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    correlation_identity TEXT NOT NULL,
    protocol_sequence INTEGER NOT NULL CHECK (protocol_sequence > 0),
    input_tokens INTEGER NOT NULL CHECK (input_tokens >= 0),
    output_tokens INTEGER NOT NULL CHECK (output_tokens >= 0),
    cache_read_tokens INTEGER NOT NULL CHECK (cache_read_tokens >= 0),
    cache_write_tokens INTEGER NOT NULL CHECK (cache_write_tokens >= 0),
    total_tokens INTEGER NOT NULL CHECK (total_tokens >= 0),
    provider_cost_binary64 BLOB NOT NULL CHECK (length(provider_cost_binary64) = 8),
    cumulative_ceiling_micros INTEGER NOT NULL CHECK (cumulative_ceiling_micros >= 0)
);
CREATE TABLE command_record_pi_office_session_dispose_usage_failure (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    correlation_identity TEXT NOT NULL,
    protocol_sequence INTEGER NOT NULL CHECK (protocol_sequence > 0),
    failure_kind INTEGER NOT NULL CHECK (failure_kind IN (1, 2)),
    unknown_reason INTEGER,
    unavailable_reason INTEGER,
    CHECK ((failure_kind = 1 AND unknown_reason IN (1, 2, 3) AND unavailable_reason IS NULL)
        OR (failure_kind = 2 AND unknown_reason IS NULL AND unavailable_reason IN (1, 2, 3, 4)))
);
CREATE TABLE command_record_pi_office_session_disposed (
    command_row_id INTEGER PRIMARY KEY REFERENCES commands(command_row_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    correlation_identity TEXT NOT NULL,
    disposed_sequence INTEGER NOT NULL CHECK (disposed_sequence > 0),
    transcript_kind INTEGER NOT NULL CHECK (transcript_kind IN (1, 2)),
    session_file TEXT NOT NULL,
    session_file_digest BLOB CHECK (session_file_digest IS NULL OR length(session_file_digest) = 32),
    transcript_content_object_id INTEGER REFERENCES content_objects(content_object_id),
    first_user_prompt_kind INTEGER,
    first_user_prompt_digest BLOB CHECK (first_user_prompt_digest IS NULL OR length(first_user_prompt_digest) = 32),
    CHECK ((transcript_kind = 1
                AND session_file_digest IS NOT NULL
                AND transcript_content_object_id IS NOT NULL
                AND first_user_prompt_kind IN (1, 2)
                AND ((first_user_prompt_kind = 1 AND first_user_prompt_digest IS NULL)
                    OR (first_user_prompt_kind = 2 AND first_user_prompt_digest IS NOT NULL)))
        OR (transcript_kind = 2
                AND session_file_digest IS NULL
                AND transcript_content_object_id IS NULL
                AND first_user_prompt_kind IS NULL
                AND first_user_prompt_digest IS NULL))
);
CREATE TABLE event_pi_office_session_dispose_authorized (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    correlation_identity TEXT NOT NULL,
    authorized_generation INTEGER NOT NULL CHECK (authorized_generation >= 0)
);
CREATE TABLE event_pi_office_session_dispose_delivered (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    native_child_id INTEGER NOT NULL REFERENCES native_children(native_child_id),
    correlation_identity TEXT NOT NULL
);
CREATE TABLE event_pi_office_session_dispose_accepted (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    correlation_identity TEXT NOT NULL,
    command_result_sequence INTEGER NOT NULL CHECK (command_result_sequence > 0)
);
CREATE TABLE event_pi_office_session_dispose_usage_recorded (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    protocol_sequence INTEGER NOT NULL CHECK (protocol_sequence > 0),
    cumulative_ceiling_micros INTEGER NOT NULL CHECK (cumulative_ceiling_micros >= 0)
);
CREATE TABLE event_pi_office_session_dispose_usage_frozen (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id),
    cancellation_request_id INTEGER NOT NULL REFERENCES cancellation_requests(cancellation_request_id),
    cost_postmortem_id INTEGER NOT NULL REFERENCES cost_postmortems(postmortem_id),
    failure_kind INTEGER NOT NULL CHECK (failure_kind IN (1, 2)),
    unknown_reason INTEGER,
    unavailable_reason INTEGER,
    CHECK ((failure_kind = 1 AND unknown_reason IN (1, 2, 3) AND unavailable_reason IS NULL)
        OR (failure_kind = 2 AND unknown_reason IS NULL AND unavailable_reason IN (1, 2, 3, 4)))
);
CREATE TABLE event_pi_office_session_disposed (
    event_id INTEGER PRIMARY KEY REFERENCES events(event_id),
    pi_office_session_dispose_receipt_id INTEGER NOT NULL REFERENCES pi_office_session_dispose_receipts(pi_office_session_dispose_receipt_id),
    root_authority_office_session_id INTEGER NOT NULL REFERENCES root_authority_office_sessions(root_authority_office_session_id),
    budget_reservation_id INTEGER NOT NULL REFERENCES budget_reservations(budget_reservation_id),
    observed_cumulative_micros INTEGER NOT NULL CHECK (observed_cumulative_micros >= 0),
    budget_disposition_kind INTEGER NOT NULL CHECK (budget_disposition_kind IN (1, 2)),
    cancellation_request_id INTEGER REFERENCES cancellation_requests(cancellation_request_id),
    cost_postmortem_id INTEGER REFERENCES cost_postmortems(postmortem_id),
    CHECK ((budget_disposition_kind = 1
                AND cancellation_request_id IS NULL AND cost_postmortem_id IS NULL)
        OR (budget_disposition_kind = 2
                AND cancellation_request_id IS NOT NULL AND cost_postmortem_id IS NOT NULL))
);
-- A session has one cumulative protocol-sequence namespace. The four
-- named accounting tables deliberately remain normalized, so reciprocal
-- fresh-schema triggers forbid raw-SQL construction of contradictory Known
-- and Unknown/Unavailable facts at the same sequence.
DROP TRIGGER pi_usage_receipt_sequence_not_failure;
DROP TRIGGER pi_usage_failure_sequence_not_receipt;
CREATE TRIGGER pi_usage_receipt_sequence_not_other_accounting
BEFORE INSERT ON pi_office_turn_usage_receipts
WHEN EXISTS (
    SELECT 1 FROM pi_office_turn_usage_failures
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
    UNION ALL
    SELECT 1 FROM pi_office_session_dispose_usage_receipts
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
    UNION ALL
    SELECT 1 FROM pi_office_session_dispose_usage_failures
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
)
BEGIN SELECT RAISE(ABORT, 'Pi usage/failure sequence collision'); END;
CREATE TRIGGER pi_usage_failure_sequence_not_other_accounting
BEFORE INSERT ON pi_office_turn_usage_failures
WHEN EXISTS (
    SELECT 1 FROM pi_office_turn_usage_receipts
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
    UNION ALL
    SELECT 1 FROM pi_office_session_dispose_usage_receipts
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
    UNION ALL
    SELECT 1 FROM pi_office_session_dispose_usage_failures
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
)
BEGIN SELECT RAISE(ABORT, 'Pi usage/failure sequence collision'); END;
CREATE TRIGGER pi_dispose_usage_receipt_sequence_not_other_accounting
BEFORE INSERT ON pi_office_session_dispose_usage_receipts
WHEN EXISTS (
    SELECT 1 FROM pi_office_turn_usage_receipts
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
    UNION ALL
    SELECT 1 FROM pi_office_turn_usage_failures
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
    UNION ALL
    SELECT 1 FROM pi_office_session_dispose_usage_failures
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
)
BEGIN SELECT RAISE(ABORT, 'Pi usage/failure sequence collision'); END;
CREATE TRIGGER pi_dispose_usage_failure_sequence_not_other_accounting
BEFORE INSERT ON pi_office_session_dispose_usage_failures
WHEN EXISTS (
    SELECT 1 FROM pi_office_turn_usage_receipts
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
    UNION ALL
    SELECT 1 FROM pi_office_turn_usage_failures
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
    UNION ALL
    SELECT 1 FROM pi_office_session_dispose_usage_receipts
    WHERE pi_session_id = NEW.pi_session_id AND protocol_sequence = NEW.protocol_sequence
)
BEGIN SELECT RAISE(ABORT, 'Pi usage/failure sequence collision'); END;
INSERT INTO principals VALUES(1,1,'bootstrap_principal',1);
INSERT INTO principals VALUES(2,2,'kernel_service',1);
INSERT INTO execution_profiles VALUES(1,1,1);
INSERT INTO execution_profiles VALUES(2,2,2);
INSERT INTO execution_profiles VALUES(3,3,1);
INSERT INTO capability_grants VALUES(1,1,1,NULL,NULL,1,1,NULL,NULL);
INSERT INTO capability_grants VALUES(2,1,2,NULL,NULL,1,1,NULL,NULL);
INSERT INTO capability_grants VALUES(3,1,3,NULL,NULL,1,1,NULL,NULL);
INSERT INTO capability_grants VALUES(4,1,4,NULL,NULL,1,1,NULL,NULL);
INSERT INTO capability_grants VALUES(5,1,5,NULL,NULL,1,1,NULL,NULL);
INSERT INTO capability_grants VALUES(6,1,6,NULL,NULL,1,1,NULL,NULL);
INSERT INTO capability_grants VALUES(7,1,7,NULL,NULL,1,1,NULL,NULL);
INSERT INTO capability_grants VALUES(8,1,8,NULL,NULL,1,1,NULL,NULL);
INSERT INTO capability_grants VALUES(9,2,17,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(10,2,18,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(11,2,19,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(12,2,20,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(13,2,21,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(14,2,22,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(15,2,38,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(16,2,46,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(17,2,54,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(18,2,55,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(19,2,58,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(20,2,59,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(21,2,62,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(22,2,63,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(23,2,64,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(24,2,66,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(25,2,67,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(26,2,69,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(27,2,70,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(28,2,71,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(29,2,72,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(30,2,73,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(31,2,74,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(32,2,75,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(33,2,76,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(34,2,77,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(35,2,78,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(36,2,79,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(37,2,80,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(38,2,81,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(39,2,82,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(40,2,83,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(41,2,84,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(42,2,85,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(43,2,86,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(44,2,87,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(45,2,88,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(46,2,89,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(47,2,90,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(48,2,91,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(49,2,92,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(50,2,93,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(51,2,94,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(52,2,95,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(53,2,96,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(54,2,97,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(55,2,98,NULL,NULL,1,3,NULL,NULL);
INSERT INTO capability_grants VALUES(56,2,99,NULL,NULL,1,3,NULL,NULL);
PRAGMA user_version = 14;
COMMIT;
PRAGMA foreign_keys = ON;
