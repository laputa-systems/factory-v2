//! Provider-free process-level M4 supervision checks.
//!
//! The fixture is a Node executable double, invoked directly with a verified
//! Node executable and no shell. It never imports Pi or a provider. These
//! tests deliberately exercise the child/pgroup/pipe boundary rather than a
//! mock of `PiSupervisor` internals.

// Test setup is intentionally direct and keeps the transition assertions
// readable. Production supervision paths remain fallible and never unwrap.
#![allow(clippy::unwrap_used)]

use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use sha2::{Digest, Sha256};
use society_pi::{
    AbsolutePath, ActorModelPolicyV1, AdapterVersion, BoundarySequence, CacheWritePerMillionRateV1,
    CanonicalModelSlug, CompactionMode, CompactionPolicyV1, CorrelationIdentity,
    CreateSessionPayload, Disabled, EffectiveModelDescriptorV1, Images, InboundCommand,
    InboundFrame, KnownPerMillionRateV1, MAX_JSONL_FRAME_BYTES, ModelApi, ModelCatalogPolicyV1,
    ModelId, ModelInput, ModelSelection, NodeRuntimeVersion, NonNegativeInteger, OpenRouterBaseUrl,
    PiSdkVersion, PositiveInteger, ProjectTrust, Provider, QueueMode, RetryPolicyV1,
    RuntimeIdentity, SessionIdentity, SessionKind, Sha256Digest, SpawnNonce, ThinkingLevel,
    ToolProfile, Transport, UsdPerMillionDecimal, encode_inbound_jsonl,
};
use societyd::supervision::{
    AdmissionDenied, CancellationMode, CancellationReason, CancellationRequest,
    CancellationRequestId, ChildLifecycle, ChildTerminalDisposition, ControlWriteDeadline,
    HandshakeDeadline, MonotonicTick, NativeHostEnvironment, NativeWorkspace, NativeWorkspaceId,
    NativeWorkspaceRoot, PiSpawnRequest, PiSupervisor, PreCreateAdmissionGate,
    QualifiedHostExecution, SupervisedChildId, SupervisionError, VerifiedArtifact,
};

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(1);

struct Allow;
impl PreCreateAdmissionGate for Allow {
    fn recheck(
        &mut self,
        _: &societyd::supervision::InertChildFacts,
    ) -> Result<(), AdmissionDenied> {
        Ok(())
    }
}

struct Deny;
impl PreCreateAdmissionGate for Deny {
    fn recheck(
        &mut self,
        _: &societyd::supervision::InertChildFacts,
    ) -> Result<(), AdmissionDenied> {
        Err(AdmissionDenied::StaleGeneration)
    }
}

#[test]
fn inert_handshake_create_dispose_and_reap_are_provider_free() {
    let fixture = Fixture::new("m4-happy");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();

    let facts = await_adapter(&mut supervisor, &child_id);
    assert_eq!(
        facts.workspace_identity,
        fixture.workspace.identity().clone()
    );
    assert_eq!(
        supervisor.lifecycle(&child_id),
        Some(ChildLifecycle::InertVerified)
    );

    supervisor
        .send_create_session(
            &child_id,
            &mut Allow,
            MonotonicTick::ZERO,
            control_deadline(),
        )
        .unwrap();
    await_session_ready(&mut supervisor, &child_id);
    assert_eq!(
        supervisor.lifecycle(&child_id),
        Some(ChildLifecycle::SessionReady)
    );

    supervisor
        .send_dispose(
            &child_id,
            correlation("dispose-happy"),
            society_pi::DisposeReason::CycleReconciliation,
            MonotonicTick::ZERO,
            control_deadline(),
        )
        .unwrap();
    await_disposed(&mut supervisor, &child_id);
    let receipt = supervisor.wait_and_reap(&child_id).unwrap();

    assert!(matches!(
        receipt.transient_evidence.stdin.observed_byte_count,
        societyd::supervision::TransientByteCount::Exact(value) if value != 0
    ));
    assert!(matches!(
        receipt.transient_evidence.stdout.observed_byte_count,
        societyd::supervision::TransientByteCount::Exact(value) if value != 0
    ));
    assert!(receipt.canonical_session_file.is_some());
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::NotRunning
    );
    assert!(supervisor.take_reaped_receipt(&child_id).is_some());
    assert!(matches!(
        supervisor.spawn_inert(fixture.spawn_request()),
        Err(SupervisionError::DuplicateChildIdentity)
    ));
    fixture.cleanup();
}

#[test]
fn stale_precreate_recheck_closes_inert_control_pipe_without_creating_a_session() {
    let fixture = Fixture::new("m4-stale");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    await_adapter(&mut supervisor, &child_id);

    assert!(matches!(
        supervisor.send_create_session(
            &child_id,
            &mut Deny,
            MonotonicTick::ZERO,
            control_deadline(),
        ),
        Err(SupervisionError::AdmissionDenied(
            AdmissionDenied::StaleGeneration
        ))
    ));
    let receipt = supervisor.wait_and_reap(&child_id).unwrap();
    assert_eq!(
        receipt
            .transient_evidence
            .logically_admitted_inbound_frame_count,
        0
    );
    assert_eq!(receipt.peer_phase, society_pi::PeerPhase::Inert);
    fixture.cleanup();
}

#[test]
fn graceful_cancellation_escalates_abort_then_term_for_a_live_session() {
    let fixture = Fixture::new("m4-term");
    let child_id = fixture.child_id();
    let mut supervisor = ready_supervisor(&fixture, &child_id);

    supervisor
        .request_cancellation(
            &child_id,
            cancellation("cancel-term", CancellationMode::GracefulCancel),
            MonotonicTick::ZERO,
        )
        .unwrap();
    supervisor
        .request_cancellation(
            &child_id,
            cancellation("cancel-term", CancellationMode::GracefulCancel),
            MonotonicTick::from_milliseconds(1),
        )
        .unwrap();
    assert_eq!(
        supervisor.lifecycle(&child_id),
        Some(ChildLifecycle::AwaitingCooperativeAbort)
    );
    assert!(
        supervisor
            .drive_cancellation(&child_id, MonotonicTick::from_milliseconds(4_999))
            .unwrap()
            .is_none()
    );
    assert!(
        supervisor
            .drive_cancellation(&child_id, MonotonicTick::from_milliseconds(5_000))
            .unwrap()
            .is_none()
    );
    let receipt = supervisor.wait_and_reap(&child_id).unwrap();
    assert!(
        receipt
            .cancellation_deliveries
            .iter()
            .any(|delivery| matches!(
                delivery.delivery,
                societyd::supervision::SignalDelivery::AbortControlWritten
            ))
    );
    assert_eq!(
        receipt
            .cancellation_deliveries
            .iter()
            .filter(|delivery| matches!(
                delivery.delivery,
                societyd::supervision::SignalDelivery::AbortControlWritten
            ))
            .count(),
        1
    );
    assert!(
        receipt
            .cancellation_deliveries
            .iter()
            .any(|delivery| matches!(
                delivery.delivery,
                societyd::supervision::SignalDelivery::TermSent
            ))
    );
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::Terminated
    );
    fixture.cleanup();
}

#[test]
fn emergency_cancellation_escalates_to_kill_when_child_ignores_term() {
    let fixture = Fixture::new("m4-ignore-term");
    let child_id = fixture.child_id();
    let mut supervisor = ready_supervisor(&fixture, &child_id);

    supervisor
        .request_cancellation(
            &child_id,
            cancellation("cancel-kill", CancellationMode::EmergencyStop),
            MonotonicTick::ZERO,
        )
        .unwrap();
    supervisor
        .drive_cancellation(&child_id, MonotonicTick::from_milliseconds(1_000))
        .unwrap();
    supervisor
        .drive_cancellation(&child_id, MonotonicTick::from_milliseconds(3_000))
        .unwrap();
    let receipt = supervisor.wait_and_reap(&child_id).unwrap();
    assert!(
        receipt
            .cancellation_deliveries
            .iter()
            .any(|delivery| matches!(
                delivery.delivery,
                societyd::supervision::SignalDelivery::KillSent
            ))
    );
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::Killed
    );
    fixture.cleanup();
}

#[test]
fn output_loss_and_child_exit_are_contained_with_transient_evidence() {
    let fixture = Fixture::new("m4-exit-before-ready");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    assert!(matches!(
        supervisor.observe_adapter_ready_at(
            &child_id,
            MonotonicTick::from_milliseconds(100),
            HandshakeDeadline::at(MonotonicTick::from_milliseconds(100)),
        ),
        Err(SupervisionError::OutputLost | SupervisionError::HandshakeDeadlineExpired)
    ));
    let receipt = reap_contained_from(
        &mut supervisor,
        &child_id,
        MonotonicTick::from_milliseconds(100),
    );
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::ContainmentFailed
    );
    assert_eq!(
        receipt
            .transient_evidence
            .logically_admitted_inbound_frame_count,
        0
    );
    fixture.cleanup();
}

#[test]
fn child_exit_after_inert_handshake_is_not_mistaken_for_an_authorized_session() {
    let fixture = Fixture::new("m4-exit-after-ready");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    await_adapter(&mut supervisor, &child_id);
    let receipt = supervisor.wait_and_reap(&child_id).unwrap();
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::ContainmentFailed
    );
    assert_eq!(
        receipt
            .transient_evidence
            .logically_admitted_inbound_frame_count,
        0
    );
    fixture.cleanup();
}

#[test]
fn malformed_host_stdout_is_not_normalized_into_a_successful_handshake() {
    let fixture = Fixture::new("m4-malformed-after-ready");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    await_adapter(&mut supervisor, &child_id);
    assert!(
        supervisor
            .observe_live_output_at(&child_id, MonotonicTick::from_milliseconds(1))
            .is_err()
    );
    let receipt = reap_contained(&mut supervisor, &child_id);
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::ContainmentFailed
    );
    assert!(matches!(
        receipt.transient_evidence.stdout.observed_byte_count,
        societyd::supervision::TransientByteCount::Exact(value) if value != 0
    ));
    fixture.cleanup();
}

#[test]
fn symlinked_auth_path_cannot_escape_the_native_workspace_before_spawn() {
    let fixture = Fixture::new("m4-auth-symlink");
    let child_id = fixture.child_id();
    let escaped = fixture.root.join("outside-auth.json");
    fs::write(&escaped, "not-an-agent-auth-file").unwrap();
    fs::remove_file(fixture.create_session.auth_path.as_path()).unwrap();
    std::os::unix::fs::symlink(&escaped, fixture.create_session.auth_path.as_path()).unwrap();

    let mut supervisor = PiSupervisor::new();
    assert!(matches!(
        supervisor.spawn_inert(fixture.spawn_request()),
        Err(SupervisionError::InvalidSpawnRequest)
    ));
    assert_eq!(supervisor.lifecycle(&child_id), None);
    fixture.cleanup();
}

#[test]
fn workspace_allocation_never_chmods_or_reuses_a_caller_selected_directory() {
    let root = temporary_path("workspace-root");
    create_private_directory(&root);
    let root = NativeWorkspaceRoot::open_owned(&root).unwrap();
    let identity = NativeWorkspaceId::parse("fresh-workspace").unwrap();
    let workspace = root.allocate(identity.clone()).unwrap();
    let mode_before = fs::metadata(workspace.directory().as_path())
        .unwrap()
        .mode()
        & 0o777;
    assert_eq!(mode_before, 0o700);
    assert!(matches!(
        root.allocate(identity),
        Err(SupervisionError::WorkspaceAlreadyExists)
    ));
    assert_eq!(
        fs::metadata(workspace.directory().as_path())
            .unwrap()
            .mode()
            & 0o777,
        mode_before
    );
    assert!(
        workspace
            .directory()
            .is_strict_descendant_of(root.directory())
    );
    fs::remove_dir_all(root.directory().as_path()).unwrap();
}

#[test]
fn workspace_root_symlink_is_not_an_authority_alias() {
    let outside = temporary_path("workspace-outside");
    let alias = temporary_path("workspace-alias");
    create_private_directory(&outside);
    std::os::unix::fs::symlink(&outside, &alias).unwrap();
    assert!(matches!(
        NativeWorkspaceRoot::open_owned(&alias),
        Err(SupervisionError::UnsafeWorkspaceRoot)
    ));
    fs::remove_file(alias).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn workspace_allocation_refuses_a_symlinked_existing_child_without_touching_outside_mode() {
    let root_path = temporary_path("workspace-owned-root");
    let outside = temporary_path("workspace-outside-child");
    create_private_directory(&root_path);
    create_private_directory(&outside);
    let root = NativeWorkspaceRoot::open_owned(&root_path).unwrap();
    let alias = root.directory().as_path().join("already-there");
    std::os::unix::fs::symlink(&outside, &alias).unwrap();
    let outside_mode = fs::metadata(&outside).unwrap().mode() & 0o777;
    assert!(matches!(
        root.allocate(NativeWorkspaceId::parse("already-there").unwrap()),
        Err(SupervisionError::WorkspaceAlreadyExists)
    ));
    assert_eq!(fs::metadata(&outside).unwrap().mode() & 0o777, outside_mode);
    fs::remove_file(alias).unwrap();
    fs::remove_dir_all(root_path).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn silent_adapter_hits_a_deadline_then_is_emergency_reaped() {
    let fixture = Fixture::new("m4-never-adapter-ignore-term");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    assert_eq!(
        supervisor
            .observe_adapter_ready_at(
                &child_id,
                MonotonicTick::ZERO,
                HandshakeDeadline::at(MonotonicTick::from_milliseconds(100)),
            )
            .unwrap(),
        None
    );
    assert!(matches!(
        supervisor.observe_adapter_ready_at(
            &child_id,
            MonotonicTick::from_milliseconds(100),
            HandshakeDeadline::at(MonotonicTick::from_milliseconds(100)),
        ),
        Err(SupervisionError::HandshakeDeadlineExpired)
    ));
    let receipt = reap_contained(&mut supervisor, &child_id);
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::ContainmentFailed
    );
    fixture.cleanup();
}

#[test]
fn silent_session_ready_hits_a_deadline_then_is_emergency_reaped() {
    let fixture = Fixture::new("m4-never-session-ready-ignore-term");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    await_adapter(&mut supervisor, &child_id);
    supervisor
        .send_create_session(
            &child_id,
            &mut Allow,
            MonotonicTick::ZERO,
            control_deadline(),
        )
        .unwrap();
    assert!(
        !supervisor
            .observe_session_ready_at(
                &child_id,
                MonotonicTick::ZERO,
                HandshakeDeadline::at(MonotonicTick::from_milliseconds(100)),
            )
            .unwrap()
    );
    assert!(matches!(
        supervisor.observe_session_ready_at(
            &child_id,
            MonotonicTick::from_milliseconds(100),
            HandshakeDeadline::at(MonotonicTick::from_milliseconds(100)),
        ),
        Err(SupervisionError::HandshakeDeadlineExpired)
    ));
    let receipt = reap_contained(&mut supervisor, &child_id);
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::ContainmentFailed
    );
    fixture.cleanup();
}

#[test]
fn unread_control_pipe_cannot_wedge_create_and_expiry_emergency_reaps() {
    let fixture = Fixture::new("m4-never-read-stdin-ignore-term");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    await_adapter(&mut supervisor, &child_id);
    assert_eq!(
        supervisor
            .send_create_session(
                &child_id,
                &mut Allow,
                MonotonicTick::ZERO,
                ControlWriteDeadline::at(MonotonicTick::from_milliseconds(100)),
            )
            .unwrap(),
        societyd::supervision::ControlWriteProgress::Pending
    );
    assert!(matches!(
        supervisor.drive_control_write(&child_id, MonotonicTick::from_milliseconds(100)),
        Err(SupervisionError::ControlWriteDeadlineExpired)
    ));
    let receipt = reap_contained_from(
        &mut supervisor,
        &child_id,
        MonotonicTick::from_milliseconds(100),
    );
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::ContainmentFailed
    );
    assert!(receipt.canonical_session_file.is_none());
    let admitted_bytes = exact_transient_byte_count(
        receipt
            .transient_evidence
            .admitted_control
            .observed_byte_count,
    );
    let physical_stdin_bytes =
        exact_transient_byte_count(receipt.transient_evidence.stdin.observed_byte_count);
    assert!(physical_stdin_bytes > 0);
    assert!(physical_stdin_bytes < admitted_bytes);
    assert_eq!(
        receipt.transient_evidence.stdin.retained_bytes().len() as u64,
        physical_stdin_bytes
    );
    assert_eq!(
        digest_bytes(receipt.transient_evidence.stdin.retained_bytes()),
        receipt.transient_evidence.stdin.sha256
    );
    assert_eq!(
        receipt
            .transient_evidence
            .logically_admitted_inbound_frame_count,
        1
    );
    assert_eq!(
        receipt
            .transient_evidence
            .physically_delivered_inbound_frame_count,
        0
    );
    fixture.cleanup();
}

#[test]
fn cancellation_discards_pending_create_before_a_paused_reader_can_resume() {
    let fixture = Fixture::new("m4-paused-reader-resume");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    await_adapter(&mut supervisor, &child_id);
    assert_eq!(
        supervisor
            .send_create_session(
                &child_id,
                &mut Allow,
                MonotonicTick::ZERO,
                control_deadline(),
            )
            .unwrap(),
        societyd::supervision::ControlWriteProgress::Pending
    );
    supervisor
        .request_cancellation(
            &child_id,
            cancellation("cancel-pending-create", CancellationMode::GracefulCancel),
            MonotonicTick::ZERO,
        )
        .unwrap();
    assert!(matches!(
        supervisor.drive_control_write(&child_id, MonotonicTick::from_milliseconds(1)),
        Err(SupervisionError::InvalidLifecycle)
    ));
    let receipt = supervisor.wait_and_reap(&child_id).unwrap();
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::ContainmentFailed
    );
    assert!(receipt.canonical_session_file.is_none());
    fixture.cleanup();
}

#[test]
fn malformed_live_host_is_contained_then_reaped_without_waiting_forever() {
    let fixture = Fixture::new("m4-malformed-live-ignore-term");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    await_adapter(&mut supervisor, &child_id);
    assert!(
        supervisor
            .observe_live_output_at(&child_id, MonotonicTick::ZERO)
            .is_err()
    );
    let receipt = reap_contained(&mut supervisor, &child_id);
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::ContainmentFailed
    );
    assert!(
        receipt
            .cancellation_deliveries
            .iter()
            .any(|receipt| matches!(
                receipt.delivery,
                societyd::supervision::SignalDelivery::KillSent
            ))
    );
    fixture.cleanup();
}

#[test]
fn overlong_live_host_is_contained_then_reaped_without_waiting_forever() {
    let fixture = Fixture::new("m4-overlong-live-ignore-term");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    await_adapter(&mut supervisor, &child_id);
    for tick in 0..1_000 {
        match supervisor.observe_live_output_at(&child_id, MonotonicTick::from_milliseconds(tick)) {
            Err(SupervisionError::OutboundFrameTooLarge) => break,
            Ok(None) => std::thread::yield_now(),
            Ok(Some(_)) => panic!("overlong raw bytes must not become a protocol observation"),
            Err(error) => panic!("unexpected overlong containment error: {error}"),
        }
        if tick == 999 {
            panic!("overlong double did not reach the bounded reader");
        }
    }
    let receipt = reap_contained(&mut supervisor, &child_id);
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::ContainmentFailed
    );
    fixture.cleanup();
}

#[test]
fn escaped_descendant_cannot_block_direct_child_reap_pipe_drain() {
    let fixture = Fixture::new("m4-escaped-descendant-holds-pipe");
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    await_adapter(&mut supervisor, &child_id);
    let receipt = supervisor.wait_and_reap(&child_id).unwrap();
    assert_eq!(
        receipt.terminal_disposition,
        ChildTerminalDisposition::ContainmentFailed
    );
    assert_eq!(
        receipt.transient_evidence.stdout.retention,
        societyd::supervision::TransientRetention::PrefixBounded
    );
    fixture.cleanup();
}

#[test]
fn second_signal_upgrades_one_cancellation_lineage_to_emergency_deadlines() {
    let fixture = Fixture::new("m4-ignore-term");
    let child_id = fixture.child_id();
    let mut supervisor = ready_supervisor(&fixture, &child_id);
    supervisor
        .request_cancellation(
            &child_id,
            cancellation("same-lineage", CancellationMode::GracefulCancel),
            MonotonicTick::ZERO,
        )
        .unwrap();
    supervisor
        .request_cancellation(
            &child_id,
            cancellation("same-lineage", CancellationMode::EmergencyStop),
            MonotonicTick::from_milliseconds(10),
        )
        .unwrap();
    supervisor
        .drive_cancellation(&child_id, MonotonicTick::from_milliseconds(1_010))
        .unwrap();
    supervisor
        .drive_cancellation(&child_id, MonotonicTick::from_milliseconds(3_010))
        .unwrap();
    let receipt = supervisor.wait_and_reap(&child_id).unwrap();
    assert_eq!(receipt.cancellation_mode_revisions.len(), 1);
    assert_eq!(
        receipt.cancellation_mode_revisions[0].from,
        CancellationMode::GracefulCancel
    );
    assert_eq!(
        receipt.cancellation_mode_revisions[0].to,
        CancellationMode::EmergencyStop
    );
    fixture.cleanup();
}

#[test]
fn boundary_containment_upgrades_an_existing_quiesce_to_bounded_emergency_reap() {
    let fixture = Fixture::new("m4-ignore-term");
    let child_id = fixture.child_id();
    let mut supervisor = ready_supervisor(&fixture, &child_id);
    supervisor
        .request_cancellation(
            &child_id,
            cancellation("quiesce-first", CancellationMode::Quiesce),
            MonotonicTick::ZERO,
        )
        .unwrap();
    supervisor
        .contain_boundary_failure(&child_id, MonotonicTick::ZERO)
        .unwrap();
    let receipt = reap_contained(&mut supervisor, &child_id);
    assert_eq!(
        receipt.cancellation_origin,
        Some(societyd::supervision::CancellationOrigin::AutomaticBoundaryContainment)
    );
    assert!(receipt.cancellation_mode_revisions.iter().any(|revision| {
        revision.from == CancellationMode::Quiesce && revision.to == CancellationMode::EmergencyStop
    }));
    fixture.cleanup();
}

/// This deliberately requires an explicitly supplied build: `dist/` is not a
/// committed artifact. The ordinary suite uses the deterministic double; the
/// build driver may run this ignored test after `npm ci && npm test` with both
/// `SOCIETY_PI_HOST_ENTRYPOINT` and `SOCIETY_PI_HOST_PACKAGE_ROOT` set to
/// canonical paths. No provider/model call occurs: the test stops at
/// CreateSession -> Dispose.
#[test]
#[ignore = "requires an explicitly built society-pi-host entrypoint and package root"]
fn explicit_pinned_host_create_dispose_never_prompts_a_provider() {
    let mut fixture = Fixture::new("m4-explicit-real-host");
    fixture.use_explicit_pinned_host();
    let child_id = fixture.child_id();
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    await_adapter(&mut supervisor, &child_id);
    supervisor
        .send_create_session(
            &child_id,
            &mut Allow,
            MonotonicTick::ZERO,
            control_deadline(),
        )
        .unwrap();
    await_session_ready(&mut supervisor, &child_id);
    supervisor
        .send_dispose(
            &child_id,
            correlation("dispose-real-host"),
            society_pi::DisposeReason::CycleReconciliation,
            MonotonicTick::ZERO,
            control_deadline(),
        )
        .unwrap();
    await_disposed(&mut supervisor, &child_id);
    let receipt = supervisor.wait_and_reap(&child_id).unwrap();
    assert_eq!(receipt.peer_phase, society_pi::PeerPhase::Disposed);
    fixture.cleanup();
}

fn await_adapter(
    supervisor: &mut PiSupervisor,
    child_id: &SupervisedChildId,
) -> societyd::supervision::InertChildFacts {
    for tick in 0..1_000 {
        if let Some(facts) = supervisor
            .observe_adapter_ready_at(
                child_id,
                MonotonicTick::from_milliseconds(tick),
                HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap()
        {
            return facts;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("provider-free host double did not emit AdapterReady before its test deadline");
}

fn await_session_ready(supervisor: &mut PiSupervisor, child_id: &SupervisedChildId) {
    for tick in 0..1_000 {
        if supervisor
            .observe_session_ready_at(
                child_id,
                MonotonicTick::from_milliseconds(tick),
                HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap()
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("provider-free host double did not emit SessionReady before its test deadline");
}

fn await_disposed(supervisor: &mut PiSupervisor, child_id: &SupervisedChildId) {
    for tick in 0..1_000 {
        if supervisor
            .observe_disposed_at(
                child_id,
                MonotonicTick::from_milliseconds(tick),
                HandshakeDeadline::at(MonotonicTick::from_milliseconds(1_000)),
            )
            .unwrap()
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("provider-free host double did not emit Disposed before its test deadline");
}

fn reap_contained(
    supervisor: &mut PiSupervisor,
    child_id: &SupervisedChildId,
) -> societyd::supervision::SupervisionReceipt {
    reap_contained_from(supervisor, child_id, MonotonicTick::ZERO)
}

fn reap_contained_from(
    supervisor: &mut PiSupervisor,
    child_id: &SupervisedChildId,
    containment_started_at: MonotonicTick,
) -> societyd::supervision::SupervisionReceipt {
    assert!(matches!(
        supervisor.wait_and_reap_at(child_id, MonotonicTick::ZERO),
        Err(SupervisionError::ContainmentAwaitingDrive)
    ));
    supervisor
        .drive_cancellation(
            child_id,
            MonotonicTick::from_milliseconds(containment_started_at.milliseconds() + 1_000),
        )
        .unwrap();
    supervisor
        .drive_cancellation(
            child_id,
            MonotonicTick::from_milliseconds(containment_started_at.milliseconds() + 3_000),
        )
        .unwrap();
    for _ in 0..1_000 {
        match supervisor.wait_and_reap_at(
            child_id,
            MonotonicTick::from_milliseconds(containment_started_at.milliseconds() + 3_000),
        ) {
            Ok(receipt) => return receipt,
            Err(SupervisionError::ContainmentAwaitingDrive) => {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Err(error) => panic!("contained child could not be reaped: {error}"),
        }
    }
    panic!("contained child escaped its bounded emergency reaping test");
}

fn ready_supervisor(fixture: &Fixture, child_id: &SupervisedChildId) -> PiSupervisor {
    let mut supervisor = PiSupervisor::new();
    supervisor.spawn_inert(fixture.spawn_request()).unwrap();
    await_adapter(&mut supervisor, child_id);
    supervisor
        .send_create_session(
            child_id,
            &mut Allow,
            MonotonicTick::ZERO,
            control_deadline(),
        )
        .unwrap();
    await_session_ready(&mut supervisor, child_id);
    supervisor
}

fn cancellation(id: &str, mode: CancellationMode) -> CancellationRequest {
    CancellationRequest {
        cancellation_request_id: CancellationRequestId::parse(id).unwrap(),
        mode,
        reason: CancellationReason::OperatorStop,
        observed_admission_generation: 7,
        abort_correlation_identity: correlation(&format!("{id}-abort")),
    }
}

fn correlation(value: &str) -> CorrelationIdentity {
    CorrelationIdentity::parse(value).unwrap()
}

fn control_deadline() -> ControlWriteDeadline {
    ControlWriteDeadline::at(MonotonicTick::from_milliseconds(1_000))
}

fn exact_transient_byte_count(count: societyd::supervision::TransientByteCount) -> u64 {
    match count {
        societyd::supervision::TransientByteCount::Exact(value) => value,
        societyd::supervision::TransientByteCount::Overflowed => {
            panic!("small provider-free fixture unexpectedly overflowed a transient counter")
        }
    }
}

fn maximize_create_session_frame(
    create_session: &mut CreateSessionPayload,
    session_identity: &SessionIdentity,
) {
    let frame = InboundFrame {
        sequence: BoundarySequence::parse(1).unwrap(),
        session_identity: session_identity.clone(),
        correlation_identity: correlation("create-session"),
        command: InboundCommand::CreateSession(Box::new(create_session.clone())),
    };
    let initial_length = encode_inbound_jsonl(&frame).unwrap().len();
    let padding = MAX_JSONL_FRAME_BYTES
        .checked_sub(initial_length)
        .expect("provider-free fixture header fits the v1 JSONL bound");
    create_session.system_prompt.push_str(&"x".repeat(padding));
    create_session.system_prompt_digest = digest_utf8(&create_session.system_prompt);
    let exact_frame = InboundFrame {
        command: InboundCommand::CreateSession(Box::new(create_session.clone())),
        ..frame
    };
    assert_eq!(
        encode_inbound_jsonl(&exact_frame).unwrap().len(),
        MAX_JSONL_FRAME_BYTES,
        "the paused-reader fixture must use the entire closed JSONL frame budget"
    );
}

struct Fixture {
    root: PathBuf,
    workspace: NativeWorkspace,
    session_identity: SessionIdentity,
    spawn_nonce: SpawnNonce,
    host_execution: QualifiedHostExecution,
    create_session: CreateSessionPayload,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let nonce = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "xsh-society-m4-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let workspace_root_path = root.join("workspace-root");
        create_private_directory(&workspace_root_path);
        let workspace = NativeWorkspaceRoot::open_owned(&workspace_root_path)
            .unwrap()
            .allocate(NativeWorkspaceId::parse(format!("workspace-{label}-{nonce}")).unwrap())
            .unwrap();
        let session_identity = SessionIdentity::parse(format!("session-{label}-{nonce}")).unwrap();
        let spawn_nonce = SpawnNonce::parse(format!("spawn-{label}-{nonce}")).unwrap();
        let agent = workspace.directory().as_path().join("agent");
        let session = workspace.directory().as_path().join("sessions");
        fs::create_dir_all(&agent).unwrap();
        fs::create_dir_all(&session).unwrap();
        let auth = agent.join("auth.json");
        let models = agent.join("models.json");
        fs::write(&auth, "{}").unwrap();
        let catalog = admitted_models_json();
        fs::write(&models, &catalog).unwrap();

        let node = node_executable();
        let double = fixture_double();
        let node_digest = digest_file(&node);
        let double_digest = digest_file(&double);
        let node_version = node_version();
        let runtime = RuntimeIdentity {
            node_version: NodeRuntimeVersion::parse(node_version).unwrap(),
            adapter_version: AdapterVersion::V1,
            pi_sdk_version: PiSdkVersion::V0830,
            node_executable_sha256: node_digest.clone(),
            lockfile_sha256: double_digest.clone(),
            adapter_build_sha256: double_digest.clone(),
            pi_transitive_package_set_sha256: double_digest.clone(),
        };
        let host_execution = QualifiedHostExecution {
            node_executable: VerifiedArtifact::inspect(&node, node_digest).unwrap(),
            adapter_entrypoint: VerifiedArtifact::inspect(&double, double_digest.clone()).unwrap(),
            lockfile: VerifiedArtifact::inspect(&double, double_digest.clone()).unwrap(),
            pi_transitive_package_set: VerifiedArtifact::inspect(&double, double_digest).unwrap(),
            runtime,
        };
        let large_control_frame =
            label.contains("never-read-stdin") || label.contains("paused-reader");
        let system_prompt = if large_control_frame {
            "Universe Seed\nM4 pending-control frame".to_owned()
        } else {
            "Universe Seed\nM4 provider-free fixture".to_owned()
        };
        let mut create_session = CreateSessionPayload {
            session_kind: SessionKind::TaskAttempt,
            cwd: workspace.directory().clone(),
            agent_directory: path(agent),
            auth_path: path(auth),
            models_path: path(models),
            session_directory: path(session),
            system_prompt_digest: digest_utf8(&system_prompt),
            system_prompt,
            model: ModelSelection {
                provider: Provider::OpenRouter,
                model_id: ModelId::DeepseekV4Flash0731,
                thinking_level: ThinkingLevel::High,
            },
            model_catalog: {
                let mut catalog_policy = admitted_catalog();
                catalog_policy.catalog_sha256 = digest_utf8(&catalog);
                catalog_policy
            },
            tool_profile: ToolProfile::ReadSourceV1,
            settings: admitted_settings(),
        };
        if large_control_frame {
            maximize_create_session_frame(&mut create_session, &session_identity);
        }
        Self {
            root,
            workspace,
            session_identity,
            spawn_nonce,
            host_execution,
            create_session,
        }
    }

    fn child_id(&self) -> SupervisedChildId {
        SupervisedChildId::parse(format!("child-{}", self.session_identity.as_str())).unwrap()
    }

    fn spawn_request(&self) -> PiSpawnRequest {
        PiSpawnRequest {
            child_process_id: self.child_id(),
            workspace: self.workspace.clone(),
            session_identity: self.session_identity.clone(),
            spawn_nonce: self.spawn_nonce.clone(),
            host_execution: self.host_execution.clone(),
            environment: NativeHostEnvironment::EmptyV1,
            create_correlation_identity: correlation("create-session"),
            create_session: self.create_session.clone(),
        }
    }

    fn use_explicit_pinned_host(&mut self) {
        let entrypoint = canonical_required_environment_path("SOCIETY_PI_HOST_ENTRYPOINT");
        let package_root = canonical_required_environment_path("SOCIETY_PI_HOST_PACKAGE_ROOT");
        let lockfile = package_root.join("package-lock.json");
        let package_set = package_root.join("node_modules/.package-lock.json");
        let node = node_executable();
        let node_digest = digest_file(&node);
        let entrypoint_digest = digest_file(&entrypoint);
        let lockfile_digest = digest_file(&lockfile);
        let package_set_digest = digest_file(&package_set);
        self.host_execution = QualifiedHostExecution {
            node_executable: VerifiedArtifact::inspect(&node, node_digest.clone()).unwrap(),
            adapter_entrypoint: VerifiedArtifact::inspect(&entrypoint, entrypoint_digest.clone())
                .unwrap(),
            lockfile: VerifiedArtifact::inspect(&lockfile, lockfile_digest.clone()).unwrap(),
            pi_transitive_package_set: VerifiedArtifact::inspect(
                &package_set,
                package_set_digest.clone(),
            )
            .unwrap(),
            runtime: RuntimeIdentity {
                node_version: NodeRuntimeVersion::parse(node_version()).unwrap(),
                adapter_version: AdapterVersion::V1,
                pi_sdk_version: PiSdkVersion::V0830,
                node_executable_sha256: node_digest,
                lockfile_sha256: lockfile_digest,
                adapter_build_sha256: entrypoint_digest,
                pi_transitive_package_set_sha256: package_set_digest,
            },
        };
        let catalog = admitted_models_json();
        fs::write(self.create_session.models_path.as_path(), &catalog).unwrap();
        self.create_session.model_catalog.catalog_sha256 = digest_utf8(&catalog);
    }

    fn cleanup(self) {
        fs::remove_dir_all(self.root).unwrap();
    }
}

fn fixture_double() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("tests/supervision/pi-host-double.mjs")
}

fn canonical_required_environment_path(name: &str) -> PathBuf {
    let value = std::env::var_os(name).unwrap_or_else(|| {
        panic!(
            "{name} is required for the explicit pinned-host test; build the host first and supply its canonical path"
        )
    });
    fs::canonicalize(value)
        .unwrap_or_else(|error| panic!("{name} does not name an existing path: {error}"))
}

fn node_executable() -> PathBuf {
    let output = Command::new("node")
        .args(["-p", "process.execPath"])
        .output()
        .expect("M4 process fixture requires Node on PATH; set a direct Node binary in test setup");
    assert!(
        output.status.success(),
        "Node lookup failed without provider access"
    );
    fs::canonicalize(String::from_utf8(output.stdout).unwrap().trim()).unwrap()
}

fn node_version() -> String {
    let output = Command::new("node")
        .arg("--version")
        .output()
        .expect("M4 process fixture requires Node on PATH");
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn path(path: PathBuf) -> AbsolutePath {
    AbsolutePath::parse(fs::canonicalize(path).unwrap().to_str().unwrap()).unwrap()
}

fn digest_file(path: &Path) -> Sha256Digest {
    digest_bytes(&fs::read(path).unwrap())
}

fn digest_utf8(text: &str) -> Sha256Digest {
    digest_bytes(text.as_bytes())
}

fn digest_bytes(bytes: &[u8]) -> Sha256Digest {
    let digest = Sha256::digest(bytes);
    let mut text = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut text, "{byte:02x}").unwrap();
    }
    Sha256Digest::parse(text).unwrap()
}

fn create_private_directory(path: &Path) {
    let text = path.to_str().unwrap();
    let c_path = std::ffi::CString::new(text).unwrap();
    // SAFETY: the test owns its fresh temp root and passes one NUL-free path.
    let result = unsafe { libc::mkdir(c_path.as_ptr(), 0o700) };
    assert_eq!(result, 0, "private test directory creation failed");
}

fn temporary_path(label: &str) -> PathBuf {
    let nonce = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "xsh-society-m4-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn admitted_catalog() -> ModelCatalogPolicyV1 {
    ModelCatalogPolicyV1 {
        catalog_sha256: Sha256Digest::parse("a".repeat(64)).unwrap(),
        effective_model: EffectiveModelDescriptorV1 {
            provider: Provider::OpenRouter,
            base_url: OpenRouterBaseUrl::ApiV1,
            api: ModelApi::OpenAiCompletions,
            model_id: ModelId::DeepseekV4Flash0731,
            canonical_slug: CanonicalModelSlug::DeepseekV4Flash20260731,
            input: ModelInput::TextOnly,
            context_window: PositiveInteger::parse(1_048_576).unwrap(),
            max_tokens: PositiveInteger::parse(384_000).unwrap(),
            input_usd_per_million: rate("0.09"),
            output_usd_per_million: rate("0.18"),
            cache_read_usd_per_million: rate("0.018"),
            cache_write_usd_per_million: CacheWritePerMillionRateV1::Absent,
        },
    }
}

fn admitted_models_json() -> String {
    concat!(
        "{\"providers\":{\"openrouter\":{",
        "\"baseUrl\":\"https://openrouter.ai/api/v1\",",
        "\"api\":\"openai-completions\",",
        "\"models\":[{\"id\":\"deepseek/deepseek-v4-flash-0731\",",
        "\"name\":\"admitted\",\"reasoning\":true,\"input\":[\"text\"],",
        "\"contextWindow\":1048576,\"maxTokens\":384000,",
        "\"cost\":{\"input\":0.00000009,\"output\":0.00000018,",
        "\"cacheRead\":0.000000018,\"cacheWrite\":0}}]}}}"
    )
    .to_owned()
}

fn rate(value: &str) -> KnownPerMillionRateV1 {
    KnownPerMillionRateV1 {
        usd_per_million: UsdPerMillionDecimal::parse(value).unwrap(),
    }
}

fn admitted_settings() -> ActorModelPolicyV1 {
    ActorModelPolicyV1 {
        retry: RetryPolicyV1 {
            max_retries: NonNegativeInteger::parse(2).unwrap(),
            base_delay_milliseconds: NonNegativeInteger::parse(2_000).unwrap(),
            provider_timeout_milliseconds: PositiveInteger::parse(300_000).unwrap(),
            provider_max_retries: NonNegativeInteger::parse(1).unwrap(),
            provider_max_retry_delay_milliseconds: PositiveInteger::parse(30_000).unwrap(),
        },
        compaction: CompactionPolicyV1 {
            mode: CompactionMode::Enabled,
            reserve_tokens: NonNegativeInteger::parse(16_384).unwrap(),
            keep_recent_tokens: NonNegativeInteger::parse(20_000).unwrap(),
        },
        steering_mode: QueueMode::OneAtATime,
        follow_up_mode: QueueMode::OneAtATime,
        transport: Transport::Sse,
        project_trust: ProjectTrust::Never,
        install_telemetry: Disabled::Disabled,
        analytics: Disabled::Disabled,
        images: Images::Blocked,
    }
}
