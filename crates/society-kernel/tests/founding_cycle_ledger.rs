// These integration fixtures deliberately fail at the exact setup/action
// boundary, so `unwrap` keeps the behavior under test visible rather than
// forcing every assertion through unrelated error plumbing.
#![allow(clippy::unwrap_used)]

use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use society_kernel::{
    AdmissionGeneration, BudgetFreezeReason, BudgetReservationId, CancellationRequestId,
    Capability, CommandBody, CommandDisposition, CommandId, CommandReceipt, CommandRequest,
    CostObservation, CostPostmortemResolution, CostUnavailableReason, CostUnknownReason, EventBody,
    ExpectedGeneration, GrandArchitectOfficeSessionId, KernelStore, OfficeSessionTerminalState,
    OfficeTurnId, OfficeTurnPurpose, OperatingCycleId, OperatingCycleState,
    OperatingCycleTreatment, PrincipalDisplayName, PrincipalId, Rejection, Sha256Digest,
    SocietyName, UsdMicros,
};

fn submit(
    store: &mut KernelStore,
    command_id: &str,
    principal_id: PrincipalId,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> CommandReceipt {
    let request = CommandRequest {
        command_id: CommandId::parse(command_id).unwrap(),
        principal_id,
        capability_grant_id: store
            .active_capability_grant(principal_id, capability)
            .unwrap()
            .expect("active test grant"),
        capability,
        expected_generation,
        body,
    };
    store.execute(request).unwrap()
}

fn accepted(
    store: &mut KernelStore,
    command_id: &str,
    principal_id: PrincipalId,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
) -> CommandReceipt {
    let receipt = submit(
        store,
        command_id,
        principal_id,
        capability,
        expected_generation,
        body,
    );
    assert!(
        matches!(receipt.disposition, CommandDisposition::Accepted(_)),
        "{command_id}: unexpected receipt: {receipt:?}"
    );
    receipt
}

fn rejected(
    store: &mut KernelStore,
    command_id: &str,
    principal_id: PrincipalId,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: CommandBody,
    expected: Rejection,
) -> CommandReceipt {
    let receipt = submit(
        store,
        command_id,
        principal_id,
        capability,
        expected_generation,
        body,
    );
    assert_eq!(receipt.disposition, CommandDisposition::Rejected(expected));
    receipt
}

fn found_cycle(store: &mut KernelStore) -> (PrincipalId, OperatingCycleId) {
    let bootstrap = PrincipalId::BOOTSTRAP;
    accepted(
        store,
        "found-create-society",
        bootstrap,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        CommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("XSH Society VS-001").unwrap(),
        },
    );
    accepted(
        store,
        "found-install-seed",
        bootstrap,
        Capability::InstallFoundingUniverseSeed,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallFoundingUniverseSeed {
            rendering_digest: Sha256Digest::of_bytes(b"UniverseSeed revision 1"),
        },
    );
    accepted(
        store,
        "found-install-office",
        bootstrap,
        Capability::InstallGrandArchitectOffice,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallGrandArchitectOffice,
    );
    accepted(
        store,
        "found-appoint-grand-architect",
        bootstrap,
        Capability::AppointInitialGrandArchitect,
        ExpectedGeneration::NotApplicable,
        CommandBody::AppointInitialGrandArchitect {
            actor_display_name: PrincipalDisplayName::parse("GA1 actor").unwrap(),
        },
    );
    accepted(
        store,
        "found-set-r0-ceiling",
        bootstrap,
        Capability::SetR0HardCeiling,
        ExpectedGeneration::NotApplicable,
        CommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::VS001_SOCIETY_HARD_CEILING,
        },
    );
    accepted(
        store,
        "found-bootstrap",
        bootstrap,
        Capability::BootstrapSociety,
        ExpectedGeneration::NotApplicable,
        CommandBody::BootstrapSociety,
    );
    accepted(
        store,
        "found-propose-cycle",
        bootstrap,
        Capability::ProposeOperatingCycle,
        ExpectedGeneration::NotApplicable,
        CommandBody::ProposeOperatingCycle {
            treatment: OperatingCycleTreatment::Vs001LiveV1,
        },
    );
    let cycle_id = OperatingCycleId::new(1).unwrap();
    accepted(
        store,
        "found-admit-cycle",
        bootstrap,
        Capability::AdmitOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::AdmitOperatingCycle { cycle_id },
    );
    (PrincipalId::new(3).unwrap(), cycle_id)
}

#[test]
fn founding_cycle_is_idempotent_fenced_and_replayable() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);

    let first = submit(
        &mut store,
        "ga-quiesce-generation-zero",
        grand_architect,
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    let repeat = submit(
        &mut store,
        "ga-quiesce-generation-zero",
        grand_architect,
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    assert_eq!(repeat.disposition, first.disposition);
    assert!(repeat.idempotent);

    rejected(
        &mut store,
        "ga-resume-stale-generation-zero",
        grand_architect,
        Capability::ResumeOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::ResumeOperatingCycle { cycle_id },
        Rejection::StaleAdmissionGeneration,
    );
    accepted(
        &mut store,
        "kernel-cycle-drained-one",
        PrincipalId::KERNEL,
        Capability::RecordCycleDrained,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordCycleDrained { cycle_id },
    );
    accepted(
        &mut store,
        "ga-resume-generation-one",
        grand_architect,
        Capability::ResumeOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap()),
        CommandBody::ResumeOperatingCycle { cycle_id },
    );
    accepted(
        &mut store,
        "ga-quiesce-generation-one",
        grand_architect,
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap()),
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    accepted(
        &mut store,
        "kernel-cycle-drained-two",
        PrincipalId::KERNEL,
        Capability::RecordCycleDrained,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordCycleDrained { cycle_id },
    );
    accepted(
        &mut store,
        "ga-reconcile-cycle",
        grand_architect,
        Capability::ReconcileOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(2).unwrap()),
        CommandBody::ReconcileOperatingCycle { cycle_id },
    );
    accepted(
        &mut store,
        "ga-close-cycle",
        grand_architect,
        Capability::CloseOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::try_from(2).unwrap()),
        CommandBody::CloseOperatingCycle { cycle_id },
    );

    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::OperatingCycleStateChanged {
            state: OperatingCycleState::Closed,
            ..
        }
    )));
    assert!(store.validate_replayed_materialized_state().is_ok());
    assert_eq!(store.command_count().unwrap(), 16);
}

#[test]
fn vs001_foundation_uses_closed_exact_cycle_treatments() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xsh-qualification-treatment-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let bootstrap = PrincipalId::BOOTSTRAP;
    accepted(
        &mut store,
        "policy-create-society",
        bootstrap,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        CommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("XSH Society policy test").unwrap(),
        },
    );
    accepted(
        &mut store,
        "policy-install-seed",
        bootstrap,
        Capability::InstallFoundingUniverseSeed,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallFoundingUniverseSeed {
            rendering_digest: Sha256Digest::of_bytes(b"policy seed"),
        },
    );
    accepted(
        &mut store,
        "policy-install-office",
        bootstrap,
        Capability::InstallGrandArchitectOffice,
        ExpectedGeneration::NotApplicable,
        CommandBody::InstallGrandArchitectOffice,
    );
    accepted(
        &mut store,
        "policy-appoint-ga",
        bootstrap,
        Capability::AppointInitialGrandArchitect,
        ExpectedGeneration::NotApplicable,
        CommandBody::AppointInitialGrandArchitect {
            actor_display_name: PrincipalDisplayName::parse("policy GA").unwrap(),
        },
    );
    rejected(
        &mut store,
        "policy-reject-wrong-r0-ceiling",
        bootstrap,
        Capability::SetR0HardCeiling,
        ExpectedGeneration::NotApplicable,
        CommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::VS001_CYCLE_CEILING,
        },
        Rejection::BudgetPolicyViolation,
    );
    accepted(
        &mut store,
        "policy-set-r0-ceiling",
        bootstrap,
        Capability::SetR0HardCeiling,
        ExpectedGeneration::NotApplicable,
        CommandBody::SetR0HardCeiling {
            ceiling: UsdMicros::VS001_SOCIETY_HARD_CEILING,
        },
    );
    accepted(
        &mut store,
        "policy-bootstrap",
        bootstrap,
        Capability::BootstrapSociety,
        ExpectedGeneration::NotApplicable,
        CommandBody::BootstrapSociety,
    );
    accepted(
        &mut store,
        "policy-propose-qualification-treatment",
        bootstrap,
        Capability::ProposeOperatingCycle,
        ExpectedGeneration::NotApplicable,
        CommandBody::ProposeOperatingCycle {
            treatment: OperatingCycleTreatment::PiSdkQualificationV1,
        },
    );
    accepted(
        &mut store,
        "policy-admit-qualification-treatment",
        bootstrap,
        Capability::AdmitOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::AdmitOperatingCycle {
            cycle_id: OperatingCycleId::new(1).unwrap(),
        },
    );
    assert_eq!(
        OperatingCycleTreatment::PiSdkQualificationV1.budget_ceiling(),
        UsdMicros::VS001_QUALIFICATION_CEILING
    );
    assert_eq!(
        OperatingCycleTreatment::Vs001LiveV1.budget_ceiling(),
        UsdMicros::VS001_CYCLE_CEILING
    );
    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::OperatingCycleProposed {
            treatment: OperatingCycleTreatment::PiSdkQualificationV1,
            ..
        }
    )));
    let inspection = rusqlite::Connection::open(&path).unwrap();
    let (treatment, ceiling): (i64, i64) = inspection
        .query_row(
            "SELECT c.treatment, e.ceiling_micros
             FROM operating_cycles c
             JOIN budget_envelope_constraints b
               ON b.operating_cycle_id = c.operating_cycle_id
             JOIN budget_envelopes e ON e.budget_envelope_id = b.budget_envelope_id
             WHERE c.operating_cycle_id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        (treatment, ceiling),
        (
            OperatingCycleTreatment::PiSdkQualificationV1 as i64,
            UsdMicros::VS001_QUALIFICATION_CEILING.value(),
        )
    );
    drop(inspection);
    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn actor_grant_must_match_the_cycle_pinned_office_occupancy() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xsh-occupancy-scope-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (_, cycle_id) = found_cycle(&mut store);

    // Succession is not a production command in this bounded slice. This
    // direct fixture creates a second, active office occupancy and its exact
    // grant, while the already proposed cycle remains pinned to occupancy 1.
    let fixture = rusqlite::Connection::open(&path).unwrap();
    fixture
        .execute(
            "UPDATE office_occupancies SET active = 0 WHERE office_occupancy_id = 1",
            [],
        )
        .unwrap();
    fixture
        .execute(
            "INSERT INTO principals(principal_id, principal_kind, display_name, active)
             VALUES (4, 3, 'successor fixture', 1)",
            [],
        )
        .unwrap();
    fixture
        .execute(
            "INSERT INTO office_occupancies(office_id, principal_id, active, appointed_by_command_id)
             VALUES (1, 4, 1, 1)",
            [],
        )
        .unwrap();
    fixture
        .execute(
            "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id,
                                              grant_state, granted_by_command_id, consumed_by_command_id)
             VALUES (4, 9, 2, 1, 1, NULL)",
            [],
        )
        .unwrap();
    drop(fixture);

    rejected(
        &mut store,
        "successor-cannot-quiesce-predecessor-cycle",
        PrincipalId::new(4).unwrap(),
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle { cycle_id },
        Rejection::CapabilityNoLongerActive,
    );

    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn forged_grant_principal_cannot_borrow_an_active_occupancy() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xsh-forged-grant-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (_, cycle_id) = found_cycle(&mut store);
    let fixture = rusqlite::Connection::open(&path).unwrap();
    fixture
        .execute(
            "INSERT INTO principals(principal_id, principal_kind, display_name, active)
             VALUES (4, 3, 'forged grant fixture', 1)",
            [],
        )
        .unwrap();
    // Normal SQL cannot create a grant whose principal does not hold the
    // named occupancy.
    assert!(fixture
        .execute(
            "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id,
                                              grant_state, granted_by_command_id, consumed_by_command_id)
             VALUES (4, 9, 1, 1, 1, NULL)",
            [],
        )
        .is_err());
    // An attacker able to remove the schema defense can still manufacture the
    // row. The runtime authorization join must reject it before transition.
    fixture
        .execute_batch(
            "DROP TRIGGER capability_grant_principal_matches_occupancy_on_insert;
             DROP TRIGGER capability_grant_principal_matches_occupancy_on_update;
             DROP TRIGGER occupancy_principal_matches_existing_grants;",
        )
        .unwrap();
    fixture
        .execute(
            "INSERT INTO capability_grants(principal_id, capability_kind, office_occupancy_id,
                                              grant_state, granted_by_command_id, consumed_by_command_id)
             VALUES (4, 9, 1, 1, 1, NULL)",
            [],
        )
        .unwrap();
    drop(fixture);

    rejected(
        &mut store,
        "forged-principal-cannot-quiesce",
        PrincipalId::new(4).unwrap(),
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle { cycle_id },
        Rejection::CapabilityNoLongerActive,
    );

    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn office_turn_and_cross_cut_budget_freeze_unknown_cost() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-office-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "kernel-office-ready",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        zero,
        CommandBody::RecordOfficeSessionReady { session_id },
    );
    accepted(
        &mut store,
        "ga-open-office-turn",
        grand_architect,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
    );
    rejected(
        &mut store,
        "ga-open-concurrent-office-turn",
        grand_architect,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "kernel-settle-office-turn",
        PrincipalId::KERNEL,
        Capability::SettleOfficeTurn,
        ExpectedGeneration::NotApplicable,
        CommandBody::SettleOfficeTurn {
            turn_id: OfficeTurnId::new(1).unwrap(),
        },
    );

    rejected(
        &mut store,
        "ga-reject-zero-budget-reservation",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::ZERO,
        },
        Rejection::BudgetCeilingExceeded,
    );
    accepted(
        &mut store,
        "ga-reserve-six-hundred-thousand",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(600_000).unwrap(),
        },
    );
    rejected(
        &mut store,
        "ga-reject-cycle-cap-overrun",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(500_000).unwrap(),
        },
        Rejection::BudgetCeilingExceeded,
    );
    accepted(
        &mut store,
        "kernel-reconcile-three-hundred-thousand",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Known(UsdMicros::new(300_000).unwrap()),
        },
    );
    accepted(
        &mut store,
        "ga-reserve-remaining-seven-hundred-thousand",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(700_000).unwrap(),
        },
    );
    accepted(
        &mut store,
        "kernel-freeze-unknown-cost",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(2).unwrap(),
            observation: CostObservation::Unknown(CostUnknownReason::AdapterStreamInterrupted),
        },
    );
    let stale = rejected(
        &mut store,
        "ga-reserve-after-unknown-cost",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(1).unwrap(),
        },
        Rejection::StaleAdmissionGeneration,
    );
    assert_eq!(
        store
            .command_receipt(&CommandId::parse("ga-reserve-after-unknown-cost").unwrap())
            .unwrap()
            .unwrap()
            .disposition,
        stale.disposition
    );
    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::BudgetAdmissionFrozen {
            reason: BudgetFreezeReason::Unknown(CostUnknownReason::AdapterStreamInterrupted),
            ..
        }
    )));
}

#[test]
fn unavailable_cost_reason_survives_event_replay() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);

    accepted(
        &mut store,
        "ga-start-unavailable-cost-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "ga-reserve-unavailable-cost",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(10_000).unwrap(),
        },
    );
    accepted(
        &mut store,
        "kernel-freeze-unavailable-cost",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Unavailable(CostUnavailableReason::ProviderUnavailable),
        },
    );

    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::BudgetAdmissionFrozen {
            reason: BudgetFreezeReason::Unavailable(CostUnavailableReason::ProviderUnavailable),
            ..
        }
    )));
}

#[test]
fn quiesce_cancellation_must_reconcile_before_resuming_admission() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());

    accepted(
        &mut store,
        "ga-request-quiesce-cancellation",
        grand_architect,
        Capability::RequestCancellation,
        zero,
        CommandBody::RequestCancellation {
            cycle_id,
            mode: society_kernel::CancellationMode::Quiesce,
        },
    );
    rejected(
        &mut store,
        "kernel-cannot-reconcile-quiescing-cancellation",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "kernel-record-quiesce-drained",
        PrincipalId::KERNEL,
        Capability::RecordCycleDrained,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordCycleDrained { cycle_id },
    );
    rejected(
        &mut store,
        "ga-cannot-resume-unreconciled-cancellation",
        grand_architect,
        Capability::ResumeOperatingCycle,
        one,
        CommandBody::ResumeOperatingCycle { cycle_id },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "kernel-reconcile-quiesce-cancellation",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
    );
    accepted(
        &mut store,
        "ga-resume-reconciled-cancellation",
        grand_architect,
        Capability::ResumeOperatingCycle,
        one,
        CommandBody::ResumeOperatingCycle { cycle_id },
    );
    rejected(
        &mut store,
        "kernel-cannot-reconcile-cancellation-after-resume",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id: CancellationRequestId::new(1).unwrap(),
        },
        Rejection::CancellationAlreadyTerminal,
    );
}

#[test]
fn office_turn_purpose_and_session_fences_follow_cycle_state() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-purpose-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "kernel-ready-purpose-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        zero,
        CommandBody::RecordOfficeSessionReady { session_id },
    );
    accepted(
        &mut store,
        "ga-quiesce-purpose-cycle",
        grand_architect,
        Capability::QuiesceOperatingCycle,
        zero,
        CommandBody::QuiesceOperatingCycle { cycle_id },
    );
    rejected(
        &mut store,
        "kernel-stale-ready-after-quiesce",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        zero,
        CommandBody::RecordOfficeSessionReady { session_id },
        Rejection::StaleAdmissionGeneration,
    );
    rejected(
        &mut store,
        "ga-ordinary-turn-rejected-while-quiescing",
        grand_architect,
        Capability::OpenOfficeTurn,
        one,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
        Rejection::InvalidLifecycleTransition,
    );
    accepted(
        &mut store,
        "ga-recovery-turn-while-quiescing",
        grand_architect,
        Capability::OpenOfficeTurn,
        one,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::Recovery,
        },
    );
    accepted(
        &mut store,
        "kernel-settle-recovery-turn",
        PrincipalId::KERNEL,
        Capability::SettleOfficeTurn,
        ExpectedGeneration::NotApplicable,
        CommandBody::SettleOfficeTurn {
            turn_id: OfficeTurnId::new(1).unwrap(),
        },
    );
    accepted(
        &mut store,
        "kernel-record-purpose-cycle-drained",
        PrincipalId::KERNEL,
        Capability::RecordCycleDrained,
        ExpectedGeneration::NotApplicable,
        CommandBody::RecordCycleDrained { cycle_id },
    );
    accepted(
        &mut store,
        "ga-closure-turn-while-drained",
        grand_architect,
        Capability::OpenOfficeTurn,
        one,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::Closure,
        },
    );
    accepted(
        &mut store,
        "kernel-settle-closure-turn",
        PrincipalId::KERNEL,
        Capability::SettleOfficeTurn,
        ExpectedGeneration::NotApplicable,
        CommandBody::SettleOfficeTurn {
            turn_id: OfficeTurnId::new(2).unwrap(),
        },
    );
    accepted(
        &mut store,
        "ga-begin-purpose-reconciliation",
        grand_architect,
        Capability::ReconcileOperatingCycle,
        one,
        CommandBody::ReconcileOperatingCycle { cycle_id },
    );
    accepted(
        &mut store,
        "ga-recovery-turn-while-reconciling",
        grand_architect,
        Capability::OpenOfficeTurn,
        one,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::Recovery,
        },
    );
    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::OfficeTurnOpened {
            purpose: OfficeTurnPurpose::Recovery,
            ..
        }
    )));
}

#[test]
fn terminal_session_fact_cannot_close_an_active_turn() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-terminal-fence-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "kernel-ready-terminal-fence-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        zero,
        CommandBody::RecordOfficeSessionReady { session_id },
    );
    accepted(
        &mut store,
        "ga-open-terminal-fence-turn",
        grand_architect,
        Capability::OpenOfficeTurn,
        zero,
        CommandBody::OpenOfficeTurn {
            session_id,
            purpose: OfficeTurnPurpose::OrdinaryWork,
        },
    );
    accepted(
        &mut store,
        "ga-cancel-terminal-fence-cycle",
        grand_architect,
        Capability::RequestCancellation,
        zero,
        CommandBody::RequestCancellation {
            cycle_id,
            mode: society_kernel::CancellationMode::GracefulCancel,
        },
    );
    rejected(
        &mut store,
        "kernel-cannot-terminal-active-turn",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionTerminal,
        one,
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state: OfficeSessionTerminalState::Cancelled,
        },
        Rejection::InvalidLifecycleTransition,
    );
}

#[test]
fn known_overrun_fences_admission_and_frozen_charge_blocks_cycle_close() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-known-overrun-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "kernel-ready-known-overrun-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        zero,
        CommandBody::RecordOfficeSessionReady { session_id },
    );
    accepted(
        &mut store,
        "ga-reserve-known-overrun",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(600_000).unwrap(),
        },
    );
    accepted(
        &mut store,
        "kernel-record-known-overrun",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Known(UsdMicros::new(700_000).unwrap()),
        },
    );
    let cancellation_request_id = store
        .replay_ledger()
        .unwrap()
        .into_iter()
        .find_map(|event| match event.body {
            EventBody::BudgetAdmissionFrozen {
                cancellation_request_id,
                reason: BudgetFreezeReason::KnownOverrun { observed, reserved },
                ..
            } if observed == UsdMicros::new(700_000).unwrap()
                && reserved == UsdMicros::new(600_000).unwrap() =>
            {
                Some(cancellation_request_id)
            }
            _ => None,
        })
        .expect("known overrun must become a typed frozen fact");
    assert_eq!(
        cancellation_request_id,
        CancellationRequestId::new(1).unwrap()
    );

    rejected(
        &mut store,
        "ga-admission-fenced-by-known-overrun",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(1).unwrap(),
        },
        Rejection::StaleAdmissionGeneration,
    );
    accepted(
        &mut store,
        "kernel-close-known-overrun-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionTerminal,
        one,
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state: OfficeSessionTerminalState::Cancelled,
        },
    );
    accepted(
        &mut store,
        "kernel-reconcile-known-overrun-cancellation",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        },
    );
    rejected(
        &mut store,
        "ga-cannot-resume-frozen-known-overrun",
        grand_architect,
        Capability::ResumeOperatingCycle,
        one,
        CommandBody::ResumeOperatingCycle { cycle_id },
        Rejection::IncompleteCycleReconciliation,
    );
    accepted(
        &mut store,
        "ga-begin-known-overrun-reconciliation",
        grand_architect,
        Capability::ReconcileOperatingCycle,
        one,
        CommandBody::ReconcileOperatingCycle { cycle_id },
    );
    rejected(
        &mut store,
        "ga-cannot-close-frozen-known-overrun",
        grand_architect,
        Capability::CloseOperatingCycle,
        one,
        CommandBody::CloseOperatingCycle { cycle_id },
        Rejection::IncompleteCycleReconciliation,
    );
}

#[test]
fn cost_postmortem_is_the_only_conservative_frozen_cost_resolution() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-postmortem-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "kernel-ready-postmortem-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        zero,
        CommandBody::RecordOfficeSessionReady { session_id },
    );
    accepted(
        &mut store,
        "ga-reserve-postmortem-cost",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(10_000).unwrap(),
        },
    );
    accepted(
        &mut store,
        "kernel-freeze-postmortem-cost",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Unknown(CostUnknownReason::ProviderDidNotReport),
        },
    );
    let (postmortem_id, cancellation_request_id) = store
        .replay_ledger()
        .unwrap()
        .into_iter()
        .find_map(|event| match event.body {
            EventBody::BudgetAdmissionFrozen {
                postmortem_id,
                cancellation_request_id,
                reason: BudgetFreezeReason::Unknown(CostUnknownReason::ProviderDidNotReport),
                ..
            } => Some((postmortem_id, cancellation_request_id)),
            _ => None,
        })
        .expect("freeze opens a linked cost postmortem");

    rejected(
        &mut store,
        "ga-cannot-resolve-before-postmortem-is-closable",
        grand_architect,
        Capability::CloseCostPostmortem,
        one,
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution: CostPostmortemResolution::ConservativeFullReservation,
        },
        Rejection::IncompleteCycleReconciliation,
    );
    accepted(
        &mut store,
        "kernel-cancel-postmortem-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionTerminal,
        one,
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state: OfficeSessionTerminalState::Cancelled,
        },
    );
    accepted(
        &mut store,
        "kernel-reconcile-postmortem-cancellation",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        },
    );
    rejected(
        &mut store,
        "ga-reject-overrun-resolution-for-unknown-cost",
        grand_architect,
        Capability::CloseCostPostmortem,
        one,
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution: CostPostmortemResolution::ChargeObservedOverrun,
        },
        Rejection::InvalidCostPostmortemResolution,
    );
    accepted(
        &mut store,
        "ga-close-unknown-cost-postmortem",
        grand_architect,
        Capability::CloseCostPostmortem,
        one,
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution: CostPostmortemResolution::ConservativeFullReservation,
        },
    );
    accepted(
        &mut store,
        "ga-resume-after-conservative-postmortem-resolution",
        grand_architect,
        Capability::ResumeOperatingCycle,
        one,
        CommandBody::ResumeOperatingCycle { cycle_id },
    );
    assert!(store.validate_replayed_materialized_state().is_ok());
}

#[test]
fn known_overrun_postmortem_records_actual_spend_even_above_admission_ceiling() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xsh-known-overrun-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);
    let one = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    let session_id = GrandArchitectOfficeSessionId::new(1).unwrap();

    accepted(
        &mut store,
        "ga-start-actual-overrun-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "kernel-ready-actual-overrun-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionReady,
        zero,
        CommandBody::RecordOfficeSessionReady { session_id },
    );
    accepted(
        &mut store,
        "ga-reserve-actual-overrun",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(600_000).unwrap(),
        },
    );
    accepted(
        &mut store,
        "kernel-record-actual-overrun",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Known(UsdMicros::new(1_500_000).unwrap()),
        },
    );
    let (postmortem_id, cancellation_request_id) = store
        .replay_ledger()
        .unwrap()
        .into_iter()
        .find_map(|event| match event.body {
            EventBody::BudgetAdmissionFrozen {
                postmortem_id,
                cancellation_request_id,
                reason: BudgetFreezeReason::KnownOverrun { observed, reserved },
                ..
            } if observed == UsdMicros::new(1_500_000).unwrap()
                && reserved == UsdMicros::new(600_000).unwrap() =>
            {
                Some((postmortem_id, cancellation_request_id))
            }
            _ => None,
        })
        .expect("known overrun is preserved with its linked postmortem");
    accepted(
        &mut store,
        "kernel-cancel-actual-overrun-session",
        PrincipalId::KERNEL,
        Capability::RecordOfficeSessionTerminal,
        one,
        CommandBody::RecordOfficeSessionTerminal {
            session_id,
            terminal_state: OfficeSessionTerminalState::Cancelled,
        },
    );
    accepted(
        &mut store,
        "kernel-reconcile-actual-overrun-cancellation",
        PrincipalId::KERNEL,
        Capability::ReconcileCancellation,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileCancellation {
            cancellation_request_id,
        },
    );
    accepted(
        &mut store,
        "ga-close-known-overrun-postmortem",
        grand_architect,
        Capability::CloseCostPostmortem,
        one,
        CommandBody::CloseCostPostmortem {
            postmortem_id,
            resolution: CostPostmortemResolution::ChargeObservedOverrun,
        },
    );
    let accounting = rusqlite::Connection::open(&path).unwrap();
    let mut statement = accounting
        .prepare("SELECT reserved_micros, spent_micros FROM budget_envelopes ORDER BY budget_envelope_id")
        .unwrap();
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows, vec![(0, 1_500_000), (0, 1_500_000)]);
    assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
        event.body,
        EventBody::CostPostmortemClosed {
            resolution: CostPostmortemResolution::ChargeObservedOverrun,
            charged,
            ..
        } if charged == UsdMicros::new(1_500_000).unwrap()
    )));
    assert!(store.validate_replayed_materialized_state().is_ok());
    drop(statement);
    drop(accounting);
    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn changed_typed_body_reusing_a_command_id_is_an_idempotency_conflict() {
    let mut store = KernelStore::open_in_memory().unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let command_id = CommandId::parse("same-command-id-different-body").unwrap();
    let quiesce_grant = store
        .active_capability_grant(grand_architect, Capability::QuiesceOperatingCycle)
        .unwrap()
        .unwrap();
    let first = store
        .execute(CommandRequest {
            command_id: command_id.clone(),
            principal_id: grand_architect,
            capability_grant_id: quiesce_grant,
            capability: Capability::QuiesceOperatingCycle,
            expected_generation: ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
            body: CommandBody::QuiesceOperatingCycle { cycle_id },
        })
        .unwrap();
    assert!(matches!(first.disposition, CommandDisposition::Accepted(_)));
    let count_after_first = store.command_count().unwrap();

    let resume_grant = store
        .active_capability_grant(grand_architect, Capability::ResumeOperatingCycle)
        .unwrap()
        .unwrap();
    assert!(matches!(
        store.execute(CommandRequest {
            command_id,
            principal_id: grand_architect,
            capability_grant_id: resume_grant,
            capability: Capability::ResumeOperatingCycle,
            expected_generation: ExpectedGeneration::Exact(
                AdmissionGeneration::try_from(1).unwrap()
            ),
            body: CommandBody::ResumeOperatingCycle { cycle_id },
        }),
        Err(society_kernel::StoreError::IdempotencyConflict)
    ));
    assert_eq!(store.command_count().unwrap(), count_after_first);
}

#[test]
fn duplicate_cost_incident_cannot_open_another_postmortem_or_cancellation() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xsh-duplicate-cost-incident-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let zero = ExpectedGeneration::Exact(AdmissionGeneration::INITIAL);

    accepted(
        &mut store,
        "ga-start-duplicate-cost-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        zero,
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    accepted(
        &mut store,
        "ga-reserve-duplicate-cost",
        grand_architect,
        Capability::ReserveBudget,
        zero,
        CommandBody::ReserveBudget {
            cycle_id,
            amount: UsdMicros::new(10_000).unwrap(),
        },
    );
    let first = accepted(
        &mut store,
        "kernel-freeze-duplicate-cost",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Unknown(CostUnknownReason::ReconciliationMismatch),
        },
    );
    let replay = submit(
        &mut store,
        "kernel-freeze-duplicate-cost",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Unknown(CostUnknownReason::ReconciliationMismatch),
        },
    );
    assert_eq!(replay.disposition, first.disposition);
    assert!(replay.idempotent);
    rejected(
        &mut store,
        "kernel-reject-second-cost-incident",
        PrincipalId::KERNEL,
        Capability::ReconcileBudget,
        ExpectedGeneration::NotApplicable,
        CommandBody::ReconcileBudget {
            reservation_id: BudgetReservationId::new(1).unwrap(),
            observation: CostObservation::Unknown(CostUnknownReason::ReconciliationMismatch),
        },
        Rejection::ReservationNotActive,
    );

    let inspection = rusqlite::Connection::open(&path).unwrap();
    let postmortems: i64 = inspection
        .query_row("SELECT COUNT(*) FROM cost_postmortems", [], |row| {
            row.get(0)
        })
        .unwrap();
    let cancellations: i64 = inspection
        .query_row("SELECT COUNT(*) FROM cancellation_requests", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!((postmortems, cancellations), (1, 1));
    drop(inspection);
    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn fresh_replay_detects_operating_cycle_budget_and_session_row_tampering() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xsh-materialized-replay-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    accepted(
        &mut store,
        "ga-start-material-replay-session",
        grand_architect,
        Capability::StartGrandArchitectOfficeSession,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::StartGrandArchitectOfficeSession { cycle_id },
    );
    assert!(store.validate_replayed_materialized_state().is_ok());

    let tamper = rusqlite::Connection::open(&path).unwrap();
    tamper
        .execute(
            "UPDATE operating_cycles SET lifecycle_state = ?1 WHERE operating_cycle_id = 1",
            [OperatingCycleState::Failed as i64],
        )
        .unwrap();
    assert!(matches!(
        store.validate_replayed_materialized_state(),
        Err(society_kernel::StoreError::LedgerCorruption(_))
    ));
    tamper
        .execute(
            "UPDATE operating_cycles SET lifecycle_state = ?1 WHERE operating_cycle_id = 1",
            [OperatingCycleState::Running as i64],
        )
        .unwrap();

    tamper
        .execute(
            "UPDATE budget_envelopes SET spent_micros = 1 WHERE budget_envelope_id = 1",
            [],
        )
        .unwrap();
    assert!(matches!(
        store.validate_replayed_materialized_state(),
        Err(society_kernel::StoreError::LedgerCorruption(_))
    ));
    tamper
        .execute(
            "UPDATE budget_envelopes SET spent_micros = 0 WHERE budget_envelope_id = 1",
            [],
        )
        .unwrap();

    tamper
        .execute(
            "UPDATE grand_architect_office_sessions SET lifecycle_state = ?1
             WHERE grand_architect_office_session_id = 1",
            [11_i64],
        )
        .unwrap();
    assert!(matches!(
        store.validate_replayed_materialized_state(),
        Err(society_kernel::StoreError::LedgerCorruption(_))
    ));
    tamper
        .execute(
            "UPDATE grand_architect_office_sessions SET lifecycle_state = 1
             WHERE grand_architect_office_session_id = 1",
            [],
        )
        .unwrap();
    assert!(store.validate_replayed_materialized_state().is_ok());

    drop(tamper);
    drop(store);
    fs::remove_file(path).unwrap();
}

#[test]
fn on_disk_reopen_preserves_treatment_and_detects_materialized_tampering() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xsh-reopen-material-replay-{unique}.sqlite3"));
    {
        let mut store = KernelStore::open(&path).unwrap();
        let (_, cycle_id) = found_cycle(&mut store);
        assert!(store.replay_ledger().unwrap().iter().any(|event| matches!(
            event.body,
            EventBody::OperatingCycleProposed {
                cycle_id: event_cycle_id,
                treatment: OperatingCycleTreatment::Vs001LiveV1,
                ..
            } if event_cycle_id == cycle_id
        )));
        assert!(store.validate_replayed_materialized_state().is_ok());
    }

    {
        let reopened = KernelStore::open(&path).unwrap();
        assert!(reopened.replay_ledger().is_ok());
        assert!(reopened.validate_replayed_materialized_state().is_ok());
    }

    let tamper = rusqlite::Connection::open(&path).unwrap();
    let (treatment, ceiling): (i64, i64) = tamper
        .query_row(
            "SELECT c.treatment, e.ceiling_micros
             FROM operating_cycles c
             JOIN budget_envelope_constraints b
               ON b.operating_cycle_id = c.operating_cycle_id
             JOIN budget_envelopes e ON e.budget_envelope_id = b.budget_envelope_id
             WHERE c.operating_cycle_id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(
        (treatment, ceiling),
        (
            OperatingCycleTreatment::Vs001LiveV1 as i64,
            UsdMicros::VS001_CYCLE_CEILING.value(),
        )
    );
    tamper
        .execute(
            "UPDATE budget_envelopes SET spent_micros = 1 WHERE budget_envelope_id = 1",
            [],
        )
        .unwrap();
    drop(tamper);

    let reopened_after_tamper = KernelStore::open(&path).unwrap();
    assert!(reopened_after_tamper.replay_ledger().is_ok());
    assert!(matches!(
        reopened_after_tamper.validate_replayed_materialized_state(),
        Err(society_kernel::StoreError::LedgerCorruption(_))
    ));
    drop(reopened_after_tamper);
    fs::remove_file(path).unwrap();
}

#[test]
fn rejected_missing_subject_and_tampered_extra_body_are_detected() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xsh-society-kernel-{unique}.sqlite3"));
    let mut store = KernelStore::open(&path).unwrap();
    let (grand_architect, cycle_id) = found_cycle(&mut store);
    let invalid_grant_id = CommandId::parse("ga-invalid-capability-grant").unwrap();
    let invalid_grant = store
        .execute(CommandRequest {
            command_id: invalid_grant_id.clone(),
            principal_id: grand_architect,
            capability_grant_id: society_kernel::CapabilityGrantId::new(999).unwrap(),
            capability: Capability::QuiesceOperatingCycle,
            expected_generation: ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
            body: CommandBody::QuiesceOperatingCycle { cycle_id },
        })
        .unwrap();
    assert_eq!(
        invalid_grant.disposition,
        CommandDisposition::Rejected(Rejection::CapabilityNotGranted)
    );
    assert_eq!(
        store
            .command_receipt(&invalid_grant_id)
            .unwrap()
            .unwrap()
            .disposition,
        invalid_grant.disposition
    );
    let absent = rejected(
        &mut store,
        "ga-quiesce-absent-cycle",
        grand_architect,
        Capability::QuiesceOperatingCycle,
        ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
        CommandBody::QuiesceOperatingCycle {
            cycle_id: OperatingCycleId::new(999).unwrap(),
        },
        Rejection::SubjectNotFound,
    );
    assert_eq!(
        store
            .command_receipt(&CommandId::parse("ga-quiesce-absent-cycle").unwrap())
            .unwrap()
            .unwrap()
            .disposition,
        absent.disposition
    );
    assert!(store.replay_ledger().is_ok());
    assert!(CommandId::parse("contains a space").is_err());
    assert!(CommandId::parse("contains\na-newline").is_err());

    let tamper = rusqlite::Connection::open(&path).unwrap();
    let stored_grant: i64 = tamper
        .query_row(
            "SELECT capability_grant_id FROM commands WHERE command_id = ?1",
            [invalid_grant_id.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored_grant, 999);
    let opaque_or_json_tables: i64 = tamper
        .query_row(
            "SELECT COUNT(*) FROM sqlite_schema
             WHERE type = 'table' AND (upper(sql) LIKE '%JSON%' OR lower(sql) LIKE '%payload%' OR lower(sql) LIKE '%metadata%')",
            [],
            |row| row.get(0),
    )
    .unwrap();
    assert_eq!(opaque_or_json_tables, 0);
    tamper
        .execute(
            "UPDATE commands SET command_id = ?1 WHERE command_row_id = 1",
            ["mutated-but-unique-command-id"],
        )
        .unwrap();
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    tamper
        .execute(
            "UPDATE commands SET command_id = ?1 WHERE command_row_id = 1",
            ["found-create-society"],
        )
        .unwrap();
    assert!(store.replay_ledger().is_ok());
    tamper
        .execute(
            "UPDATE command_create_society_identity SET name = ?1 WHERE command_row_id = 1",
            ["altered durable command body"],
        )
        .unwrap();
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    tamper
        .execute(
            "UPDATE command_create_society_identity SET name = ?1 WHERE command_row_id = 1",
            ["XSH Society VS-001"],
        )
        .unwrap();
    assert!(store.replay_ledger().is_ok());
    tamper
        .execute(
            "UPDATE event_r0_hard_ceiling_set SET ceiling_micros = ?1 WHERE event_id = 5",
            [1_i64],
        )
        .unwrap();
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    tamper
        .execute(
            "UPDATE event_r0_hard_ceiling_set SET ceiling_micros = ?1 WHERE event_id = 5",
            [UsdMicros::VS001_SOCIETY_HARD_CEILING.value()],
        )
        .unwrap();
    assert!(store.replay_ledger().is_ok());
    tamper
        .execute(
            "INSERT INTO command_bootstrap_society(command_row_id) VALUES (?1)",
            [1_i64],
        )
        .unwrap();
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    tamper
        .execute(
            "DELETE FROM command_bootstrap_society WHERE command_row_id = ?1",
            [1_i64],
        )
        .unwrap();
    assert!(store.replay_ledger().is_ok());
    // Swap two accepted event links while preserving one event per accepted
    // command and their reciprocal receipt references. The event commitments
    // still bind each body to the original command identity, so this remains
    // detectable without relying on a missing-link side effect.
    tamper
        .execute(
            "UPDATE events SET command_row_id = 9 WHERE event_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE events SET command_row_id = 1 WHERE event_id = 2",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE events SET command_row_id = 2 WHERE event_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE commands SET accepted_event_id = 2 WHERE command_row_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE commands SET accepted_event_id = 1 WHERE command_row_id = 2",
            [],
        )
        .unwrap();
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    tamper
        .execute(
            "UPDATE events SET command_row_id = 9 WHERE event_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE events SET command_row_id = 2 WHERE event_id = 2",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE events SET command_row_id = 1 WHERE event_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE commands SET accepted_event_id = 1 WHERE command_row_id = 1",
            [],
        )
        .unwrap();
    tamper
        .execute(
            "UPDATE commands SET accepted_event_id = 2 WHERE command_row_id = 2",
            [],
        )
        .unwrap();
    assert!(store.replay_ledger().is_ok());
    tamper
        .execute(
            "INSERT INTO event_society_bootstrapped(event_id, society_id) VALUES (?1, ?2)",
            rusqlite::params![1_i64, 1_i64],
        )
        .unwrap();
    drop(tamper);
    assert!(matches!(
        store.replay_ledger().unwrap_err(),
        society_kernel::StoreError::LedgerCorruption(_)
    ));
    drop(store);
    fs::remove_file(path).unwrap();
}
