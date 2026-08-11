#![allow(clippy::unwrap_used)]

use std::{
    fs,
    io::Write,
    net::Shutdown,
    os::{
        fd::{FromRawFd, IntoRawFd},
        unix::{fs::PermissionsExt, net::UnixStream},
    },
    path::{Path, PathBuf},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use society_kernel::{
    AdmissionGeneration, ApplicationIdentity, ApplicationMissionInput, ApplicationName,
    ApplicationRevisionOrdinal, Blake3Digest, Capability, CapabilityGrantId, CommandId,
    ContentIdentityState, ExpectedGeneration, KernelStore, MissionPrinciple, MissionPrincipleKind,
    MissionPrincipleText, MissionPrinciples, MissionSourceRendering, MissionStatement,
    NorthStarBoundaryCommitmentQuestion, NorthStarChangeQuestion,
    NorthStarImprovementEvidenceQuestion, NorthStarQuestionSet, NorthStarRevisitQuestion,
    OperatingCycleId, OperatingCycleTreatment, PrincipalDisplayName, PrincipalId, SocietyName,
    StudyPairId, StudyRunId, UsdMicros,
};
use societyctl::{SocietyctlClient, SocietyctlError, SupervisorClient};
use societyd::protocol::{
    self, ClientCommandBody, ClientCommandRequest, CommandReceiptView, CorrelationId, DaemonStatus,
    ProtocolErrorCode, SupervisorRequest,
};
use societyd::{Daemon, DaemonConfig, DaemonError, FaultInjection, ShutdownHandle, StartupMode};

const BOOTSTRAP: PrincipalId = PrincipalId::BOOTSTRAP;
const ROOT_AUTHORITY: PrincipalId = PrincipalId::new(3).expect("compiled principal id");

fn resident_application_mission() -> ApplicationMissionInput {
    ApplicationMissionInput {
        application_identity: ApplicationIdentity::parse("resident-protocol-fixture").unwrap(),
        application_name: ApplicationName::parse("Resident protocol fixture").unwrap(),
        revision_ordinal: ApplicationRevisionOrdinal::new(1).unwrap(),
        statement: MissionStatement::parse("Exercise the bounded resident protocol.").unwrap(),
        principles: MissionPrinciples::new(vec![
            MissionPrinciple {
                kind: MissionPrincipleKind::Purpose,
                text: MissionPrincipleText::parse("Keep the fixture generic and legible.").unwrap(),
            },
            MissionPrinciple {
                kind: MissionPrincipleKind::Boundary,
                text: MissionPrincipleText::parse("Do not widen daemon authority.").unwrap(),
            },
        ])
        .unwrap(),
        north_star_questions: NorthStarQuestionSet {
            change: NorthStarChangeQuestion::parse("What bounded change is needed?").unwrap(),
            improvement_evidence: NorthStarImprovementEvidenceQuestion::parse(
                "What evidence proves the improvement?",
            )
            .unwrap(),
            boundary_commitment: NorthStarBoundaryCommitmentQuestion::parse(
                "Which authority boundary must remain intact?",
            )
            .unwrap(),
            revisit: NorthStarRevisitQuestion::parse("When should this mission be revisited?")
                .unwrap(),
        },
        source_rendering_digest: Blake3Digest::of_bytes(b"resident-protocol-fixture-mission"),
    }
}

fn resident_mission_source_rendering() -> MissionSourceRendering {
    MissionSourceRendering::parse(b"resident-protocol-fixture-mission".to_vec()).unwrap()
}

fn changed_resident_mission_source_rendering() -> MissionSourceRendering {
    MissionSourceRendering::parse(b"resident-protocol-fixture-mission-changed".to_vec()).unwrap()
}

fn founding_mission_body() -> ClientCommandBody {
    ClientCommandBody::InstallFoundingMission {
        mission: Box::new(resident_application_mission()),
        source_rendering: resident_mission_source_rendering(),
    }
}

fn mission_source_operation_label(digest: Blake3Digest) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut label = String::from("mission-source-");
    for byte in digest.as_bytes() {
        label.push(char::from(HEX[(byte >> 4) as usize]));
        label.push(char::from(HEX[(byte & 0x0F) as usize]));
    }
    label
}

fn temporary_runtime_root(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from("/tmp").join(format!("xsd-{label}-{}-{unique}", std::process::id()))
}

fn test_database_url() -> society_kernel::KernelDatabaseUrl {
    society_kernel::KernelDatabaseUrl::from_env("SOCIETY_POSTGRES_TEST_URL").unwrap()
}

fn test_daemon_config(root: &Path) -> DaemonConfig {
    let schema_path = root.join("society.pg-test-schema");
    KernelStore::connect_test_path(&schema_path).unwrap();
    DaemonConfig::new(root)
        .with_database_url(test_database_url())
        .with_database_schema(KernelStore::test_schema_for_path(schema_path))
}

fn start(
    root: &Path,
    fault: FaultInjection,
) -> (
    SocietyctlClient,
    SupervisorClient,
    ShutdownHandle,
    JoinHandle<Result<(), DaemonError>>,
    PathBuf,
    StartupMode,
) {
    let (client, supervisor_stream, shutdown, join, socket_path, mode) =
        start_with_supervisor_stream(root, fault);
    let supervisor = SupervisorClient::from_inherited_stream(supervisor_stream).unwrap();
    (client, supervisor, shutdown, join, socket_path, mode)
}

fn start_with_supervisor_stream(
    root: &Path,
    fault: FaultInjection,
) -> (
    SocietyctlClient,
    UnixStream,
    ShutdownHandle,
    JoinHandle<Result<(), DaemonError>>,
    PathBuf,
    StartupMode,
) {
    let (supervisor_stream, daemon_stream) = UnixStream::pair().unwrap();
    let daemon = Daemon::bind(
        test_daemon_config(root)
            .with_fault_injection(fault)
            .with_supervisor_stream(daemon_stream),
    )
    .unwrap();
    let socket_path = daemon.socket_path();
    let mode = daemon.startup_mode();
    let shutdown = daemon.shutdown_handle();
    let control_loop_shutdown = shutdown.clone();
    let join = thread::spawn(move || {
        let mut daemon = daemon;
        daemon.serve_until(&control_loop_shutdown)
    });
    let client = SocietyctlClient::connect(&socket_path);
    for _ in 0..100 {
        if client.status(correlation(99)).is_ok() {
            return (client, supervisor_stream, shutdown, join, socket_path, mode);
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("daemon did not accept a status request");
}

fn stop(shutdown: ShutdownHandle, join: JoinHandle<Result<(), DaemonError>>) {
    shutdown.request_shutdown();
    assert!(join.join().unwrap().is_ok());
}

fn correlation(value: u64) -> CorrelationId {
    CorrelationId::new(value).unwrap()
}

#[test]
fn public_study_observation_queries_are_read_only_and_report_absence() {
    let root = temporary_runtime_root("public-study-observations");
    let (client, _supervisor, shutdown, join, _socket_path, _) = start(&root, FaultInjection::None);

    assert_eq!(
        client
            .study_pair_observation(correlation(70), StudyPairId::new(1).unwrap())
            .unwrap(),
        None
    );
    assert_eq!(
        client
            .study_run_observation(correlation(71), StudyRunId::new(1).unwrap())
            .unwrap(),
        None
    );
    assert_eq!(
        client.status(correlation(72)).unwrap(),
        DaemonStatus::FreshServing { command_count: 0 }
    );

    stop(shutdown, join);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn named_monitor_socket_rejects_supervisor_execute_frames() {
    // The public socket must remain a query-only transport even when a peer
    // knows the version and correlation framing.  In particular, a raw
    // supervisor Execute tag cannot become a study-run admission or scheduler
    // capability by being sent to the named monitor socket.
    let mut payload = Vec::new();
    payload.extend_from_slice(&protocol::PROTOCOL_VERSION.to_be_bytes());
    payload.push(0x41); // SUPERVISOR_EXECUTE_TAG, intentionally not public.
    payload.extend_from_slice(&1_u64.to_be_bytes()); // nonzero correlation.
    let mut frame = Vec::new();
    protocol::write_frame(&mut frame, &payload).unwrap();

    assert!(matches!(
        protocol::read_public_request(&mut frame.as_slice()),
        Err(protocol::WireError::UnknownTag)
    ));
}

fn command(
    command_id: &str,
    principal_id: PrincipalId,
    grant: i64,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: ClientCommandBody,
) -> ClientCommandRequest {
    ClientCommandRequest {
        command_id: CommandId::parse(command_id).unwrap(),
        principal_id,
        capability_grant_id: CapabilityGrantId::new(grant).unwrap(),
        capability,
        expected_generation,
        body,
    }
}

fn active_grant(
    supervisor: &mut SupervisorClient,
    principal_id: PrincipalId,
    capability: Capability,
) -> i64 {
    supervisor
        .active_capability_grant(
            correlation(90_000 + capability as u64),
            principal_id,
            capability,
        )
        .unwrap()
        .expect("current kernel grant for declared command family")
        .value()
}

fn accepted(receipt: CommandReceiptView) {
    assert!(
        matches!(receipt, CommandReceiptView::Accepted { .. }),
        "{receipt:?}"
    );
}

fn execute_with_active_grant(
    supervisor: &mut SupervisorClient,
    correlation: CorrelationId,
    command_id: &str,
    principal_id: PrincipalId,
    capability: Capability,
    expected_generation: ExpectedGeneration,
    body: ClientCommandBody,
) -> Result<CommandReceiptView, SocietyctlError> {
    let grant = active_grant(supervisor, principal_id, capability);
    supervisor.execute(
        correlation,
        command(
            command_id,
            principal_id,
            grant,
            capability,
            expected_generation,
            body,
        ),
    )
}

fn create_society(supervisor: &mut SupervisorClient, correlation_value: u64, command_id: &str) {
    accepted(
        execute_with_active_grant(
            supervisor,
            correlation(correlation_value),
            command_id,
            BOOTSTRAP,
            Capability::CreateSocietyIdentity,
            ExpectedGeneration::NotApplicable,
            ClientCommandBody::CreateSocietyIdentity {
                name: SocietyName::parse("daemon sealed mission society").unwrap(),
            },
        )
        .unwrap(),
    );
}

fn founding_commands(client: &mut SupervisorClient, start_correlation: u64) {
    accepted(
        execute_with_active_grant(
            client,
            correlation(start_correlation),
            "daemon-found-society",
            BOOTSTRAP,
            Capability::CreateSocietyIdentity,
            ExpectedGeneration::NotApplicable,
            ClientCommandBody::CreateSocietyIdentity {
                name: SocietyName::parse("daemon protocol society").unwrap(),
            },
        )
        .unwrap(),
    );
    accepted(
        execute_with_active_grant(
            client,
            correlation(start_correlation + 1),
            "daemon-found-founding-mission",
            BOOTSTRAP,
            Capability::InstallFoundingMission,
            ExpectedGeneration::NotApplicable,
            ClientCommandBody::InstallFoundingMission {
                mission: Box::new(resident_application_mission()),
                source_rendering: resident_mission_source_rendering(),
            },
        )
        .unwrap(),
    );
    accepted(
        execute_with_active_grant(
            client,
            correlation(start_correlation + 2),
            "daemon-found-office",
            BOOTSTRAP,
            Capability::InstallRootAuthorityOffice,
            ExpectedGeneration::NotApplicable,
            ClientCommandBody::InstallRootAuthorityOffice,
        )
        .unwrap(),
    );
    accepted(
        execute_with_active_grant(
            client,
            correlation(start_correlation + 3),
            "daemon-found-root_authority",
            BOOTSTRAP,
            Capability::AppointInitialRootAuthority,
            ExpectedGeneration::NotApplicable,
            ClientCommandBody::AppointInitialRootAuthority {
                actor_display_name: PrincipalDisplayName::parse("daemon root authority").unwrap(),
            },
        )
        .unwrap(),
    );
    accepted(
        execute_with_active_grant(
            client,
            correlation(start_correlation + 4),
            "daemon-found-ceiling",
            BOOTSTRAP,
            Capability::SetR0HardCeiling,
            ExpectedGeneration::NotApplicable,
            ClientCommandBody::SetR0HardCeiling {
                ceiling: UsdMicros::new(1_030_000).unwrap(),
            },
        )
        .unwrap(),
    );
    accepted(
        execute_with_active_grant(
            client,
            correlation(start_correlation + 5),
            "daemon-found-bootstrap",
            BOOTSTRAP,
            Capability::BootstrapSociety,
            ExpectedGeneration::NotApplicable,
            ClientCommandBody::BootstrapSociety,
        )
        .unwrap(),
    );
}

fn admit_and_close_empty_cycle(
    client: &mut SupervisorClient,
    correlation_start: u64,
    treatment: OperatingCycleTreatment,
    budget_ceiling: UsdMicros,
    cycle_id: OperatingCycleId,
) {
    accepted(
        execute_with_active_grant(
            client,
            correlation(correlation_start),
            &format!("daemon-propose-{}", cycle_id.value()),
            ROOT_AUTHORITY,
            Capability::ProposeOperatingCycle,
            ExpectedGeneration::NotApplicable,
            ClientCommandBody::ProposeOperatingCycle {
                treatment,
                budget_ceiling,
            },
        )
        .unwrap(),
    );
    accepted(
        execute_with_active_grant(
            client,
            correlation(correlation_start + 1),
            &format!("daemon-admit-{}", cycle_id.value()),
            ROOT_AUTHORITY,
            Capability::AdmitOperatingCycle,
            ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
            ClientCommandBody::AdmitOperatingCycle { cycle_id },
        )
        .unwrap(),
    );
    accepted(
        execute_with_active_grant(
            client,
            correlation(correlation_start + 2),
            &format!("daemon-quiesce-{}", cycle_id.value()),
            ROOT_AUTHORITY,
            Capability::QuiesceOperatingCycle,
            ExpectedGeneration::Exact(AdmissionGeneration::INITIAL),
            ClientCommandBody::QuiesceOperatingCycle { cycle_id },
        )
        .unwrap(),
    );
    let fenced = ExpectedGeneration::Exact(AdmissionGeneration::try_from(1).unwrap());
    accepted(
        execute_with_active_grant(
            client,
            correlation(correlation_start + 3),
            &format!("daemon-reconcile-{}", cycle_id.value()),
            ROOT_AUTHORITY,
            Capability::ReconcileOperatingCycle,
            fenced,
            ClientCommandBody::ReconcileOperatingCycle { cycle_id },
        )
        .unwrap(),
    );
    accepted(
        execute_with_active_grant(
            client,
            correlation(correlation_start + 4),
            &format!("daemon-close-{}", cycle_id.value()),
            ROOT_AUTHORITY,
            Capability::CloseOperatingCycle,
            fenced,
            ClientCommandBody::CloseOperatingCycle { cycle_id },
        )
        .unwrap(),
    );
}

#[test]
fn owns_one_runtime_root_and_exposes_a_user_only_socket() {
    let root = temporary_runtime_root("single-owner");
    let (_client, _supervisor, shutdown, join, socket_path, _) = start(&root, FaultInjection::None);
    let mode = fs::metadata(socket_path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    assert!(!root.join("bootstrap.admission").exists());
    assert!(!root.join("root-authority.admission").exists());
    assert!(matches!(
        Daemon::bind(test_daemon_config(&root)),
        Err(DaemonError::AlreadyRunning)
    ));
    stop(shutdown, join);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_frames_never_create_a_command_row() {
    let root = temporary_runtime_root("malformed");
    let (client, _supervisor, shutdown, join, socket_path, _) = start(&root, FaultInjection::None);
    let valid = SupervisorRequest::Execute {
        correlation: correlation(1),
        command: command(
            "wire-valid-command",
            BOOTSTRAP,
            1,
            Capability::CreateSocietyIdentity,
            ExpectedGeneration::NotApplicable,
            ClientCommandBody::CreateSocietyIdentity {
                name: SocietyName::parse("wire valid society").unwrap(),
            },
        ),
    };
    let mut encoded = Vec::new();
    protocol::write_supervisor_request(&mut encoded, &valid).unwrap();
    let payload = encoded[4..].to_vec();
    let mut invalid_utf8 = payload.clone();
    let value_position = invalid_utf8
        .windows(b"wire valid society".len())
        .position(|window| window == b"wire valid society")
        .unwrap();
    invalid_utf8[value_position] = 0xff;
    let mut trailing = payload.clone();
    trailing.push(0);

    for malformed in [
        vec![0, 2, 3, 0, 0, 0, 0, 0, 0, 0, 1],
        vec![0, 1, 0xff, 0, 0, 0, 0, 0, 0, 0, 1],
        b"{\"kind\":\"generic_json_command\"}".to_vec(),
        invalid_utf8,
        trailing,
    ] {
        raw_frame(&socket_path, &malformed);
        thread::sleep(Duration::from_millis(10));
        if join.is_finished() {
            panic!("daemon stopped on malformed frame: {:?}", join.join());
        }
    }
    let mut short = UnixStream::connect(&socket_path).unwrap();
    short.write_all(&8_u32.to_be_bytes()).unwrap();
    short.write_all(&[0, 1]).unwrap();
    drop(short);
    thread::sleep(Duration::from_millis(10));
    assert!(!join.is_finished(), "daemon stopped on short frame");
    let mut oversized = UnixStream::connect(&socket_path).unwrap();
    oversized
        .write_all(&((protocol::MAX_FRAME_BYTES as u32) + 1).to_be_bytes())
        .unwrap();
    drop(oversized);
    thread::sleep(Duration::from_millis(10));
    assert!(!join.is_finished(), "daemon stopped on oversized frame");

    thread::sleep(Duration::from_millis(30));
    if join.is_finished() {
        panic!("malformed-frame daemon result: {:?}", join.join());
    }
    assert_eq!(
        client.status(correlation(2)).unwrap(),
        DaemonStatus::FreshServing { command_count: 0 }
    );
    stop(shutdown, join);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn same_uid_public_peer_cannot_acquire_privileged_admission() {
    let root = temporary_runtime_root("forged-kernel");
    let (client, mut supervisor, shutdown, join, socket_path, _) =
        start_with_supervisor_stream(&root, FaultInjection::None);

    // The test runs as the same OS user as the daemon. Known former credential
    // paths contain no reusable material. A raw command frame is unknown to
    // the named monitor socket, while even a peer holding the test-only
    // supervisor endpoint cannot claim daemon-only kernel authority.
    assert!(!root.join("bootstrap.admission").exists());
    assert!(!root.join("root-authority.admission").exists());
    let forged_command = command(
        "forged-kernel-service",
        PrincipalId::KERNEL,
        9,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        ClientCommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("forged kernel identity").unwrap(),
        },
    );
    let forged = SupervisorRequest::Execute {
        correlation: correlation(3),
        command: forged_command.clone(),
    };
    let mut encoded = Vec::new();
    protocol::write_supervisor_request(&mut encoded, &forged).unwrap();
    let mut stream = UnixStream::connect(&socket_path).unwrap();
    stream.write_all(&encoded).unwrap();
    assert!(matches!(
        protocol::read_response(&mut stream),
        Err(protocol::WireError::EndOfStream)
    ));
    supervisor.write_all(&encoded).unwrap();
    assert!(matches!(
        protocol::read_response(&mut supervisor).unwrap(),
        protocol::Response::Error {
            correlation: received,
            code: ProtocolErrorCode::PeerNotAuthorized,
        } if received == correlation(3)
    ));

    let normal_command = command(
        "forged-daemon-only-capability",
        ROOT_AUTHORITY,
        15,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        ClientCommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("forged lifecycle capability").unwrap(),
        },
    );
    let mut daemon_only = Vec::new();
    protocol::write_supervisor_request(
        &mut daemon_only,
        &SupervisorRequest::Execute {
            correlation: correlation(4),
            command: normal_command,
        },
    )
    .unwrap();
    let capability_offset = 4 + 2 + 1 + 8 + 4 + "forged-daemon-only-capability".len() + 8 + 8;
    daemon_only[capability_offset] = Capability::RecordCycleDrained as u8;
    supervisor.write_all(&daemon_only).unwrap();
    assert!(matches!(
        protocol::read_response(&mut supervisor).unwrap(),
        protocol::Response::Error {
            correlation: received,
            code: ProtocolErrorCode::PeerNotAuthorized,
        } if received == correlation(4)
    ));

    protocol::write_supervisor_request(
        &mut supervisor,
        &SupervisorRequest::ActiveCapabilityGrant {
            correlation: correlation(5),
            principal_id: PrincipalId::KERNEL,
            capability: Capability::CreateSocietyIdentity,
        },
    )
    .unwrap();
    assert!(matches!(
        protocol::read_response(&mut supervisor).unwrap(),
        protocol::Response::Error {
            correlation: received,
            code: ProtocolErrorCode::PeerNotAuthorized,
        } if received == correlation(5)
    ));
    assert_eq!(
        client.status(correlation(7)).unwrap(),
        DaemonStatus::FreshServing { command_count: 0 }
    );
    stop(shutdown, join);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fragmented_supervisor_frame_preserves_authority_until_one_complete_request() {
    let root = temporary_runtime_root("fragmented-supervisor");
    let (client, mut supervisor, shutdown, join, _socket_path, _) =
        start_with_supervisor_stream(&root, FaultInjection::None);
    let command_id = CommandId::parse("fragmented-supervisor-create").unwrap();
    let request = SupervisorRequest::Execute {
        correlation: correlation(30),
        command: command(
            command_id.as_str(),
            BOOTSTRAP,
            1,
            Capability::CreateSocietyIdentity,
            ExpectedGeneration::NotApplicable,
            ClientCommandBody::CreateSocietyIdentity {
                name: SocietyName::parse("fragmented supervisor society").unwrap(),
            },
        ),
    };
    let mut encoded = Vec::new();
    protocol::write_supervisor_request(&mut encoded, &request).unwrap();

    // The daemon sees a partial frame first. It must retain those bytes and
    // complete the bounded blocking read after the stream becomes readable,
    // rather than treating `WouldBlock` as authority loss.
    supervisor.write_all(&encoded[..3]).unwrap();
    thread::sleep(Duration::from_millis(25));
    supervisor.write_all(&encoded[3..]).unwrap();
    assert!(matches!(
        protocol::read_response(&mut supervisor).unwrap(),
        protocol::Response::CommandReceipt {
            correlation: received,
            receipt: CommandReceiptView::Accepted {
                idempotent: false,
                ..
            },
        } if received == correlation(30)
    ));
    assert!(matches!(
        client.command_receipt(correlation(31), command_id).unwrap(),
        Some(CommandReceiptView::Accepted {
            idempotent: true,
            ..
        })
    ));
    assert!(
        !join.is_finished(),
        "fragmented authority frame stopped daemon"
    );
    stop(shutdown, join);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supervisor_reply_loss_preserves_committed_receipt_but_closes_admission() {
    let root = temporary_runtime_root("supervisor-reply-loss");
    let (client, mut supervisor, shutdown, join, socket_path, _) =
        start_with_supervisor_stream(&root, FaultInjection::None);
    let command_id = CommandId::parse("reply-loss-create").unwrap();
    let request = SupervisorRequest::Execute {
        correlation: correlation(40),
        command: command(
            command_id.as_str(),
            BOOTSTRAP,
            1,
            Capability::CreateSocietyIdentity,
            ExpectedGeneration::NotApplicable,
            ClientCommandBody::CreateSocietyIdentity {
                name: SocietyName::parse("reply loss society").unwrap(),
            },
        ),
    };
    let mut encoded = Vec::new();
    protocol::write_supervisor_request(&mut encoded, &request).unwrap();
    supervisor.write_all(&encoded).unwrap();
    supervisor.shutdown(Shutdown::Both).unwrap();
    drop(supervisor);

    let receipt = (0..100)
        .find_map(
            |_| match client.command_receipt(correlation(41), command_id.clone()) {
                Ok(receipt @ Some(_)) => Some(receipt),
                Ok(None) | Err(_) => {
                    thread::sleep(Duration::from_millis(5));
                    None
                }
            },
        )
        .expect("committed receipt remains publicly queryable after reply loss");
    assert!(matches!(
        receipt,
        Some(CommandReceiptView::Accepted {
            idempotent: true,
            ..
        })
    ));

    // Replaying a command-family frame over the named monitor socket cannot
    // reopen authority after the anonymous supervisor peer has gone away.
    let mut public_stream = UnixStream::connect(socket_path).unwrap();
    public_stream.write_all(&encoded).unwrap();
    assert!(matches!(
        protocol::read_response(&mut public_stream),
        Err(protocol::WireError::EndOfStream)
    ));
    assert!(!join.is_finished(), "supervisor reply loss stopped daemon");
    stop(shutdown, join);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supervisor_authority_requires_connected_unix_stream_and_closes_on_exec() {
    let regular_root = temporary_runtime_root("regular-supervisor-fd");
    let regular_path = temporary_runtime_root("regular-supervisor-file");
    let regular = fs::File::create(&regular_path).unwrap();
    let regular_fd = regular.into_raw_fd();
    // SAFETY: `regular_fd` transfers exactly once into this temporary stream
    // wrapper so `Daemon::bind` can prove that a non-socket is rejected.
    let regular_stream = unsafe { UnixStream::from_raw_fd(regular_fd) };
    assert!(matches!(
        Daemon::bind(test_daemon_config(&regular_root).with_supervisor_stream(regular_stream)),
        Err(DaemonError::InvalidSupervisorStream)
    ));
    fs::remove_file(regular_path).unwrap();
    fs::remove_dir_all(regular_root).unwrap();

    let datagram_root = temporary_runtime_root("datagram-supervisor-fd");
    let mut descriptors = [-1_i32; 2];
    assert_eq!(
        // SAFETY: the output array has room for exactly the two descriptors
        // made by `socketpair`; this creates a connected Unix datagram pair.
        unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_DGRAM, 0, descriptors.as_mut_ptr()) },
        0
    );
    // SAFETY: descriptor zero is transferred exactly once into the wrapper;
    // it deliberately has the wrong socket type for a supervisor stream.
    let datagram_as_stream = unsafe { UnixStream::from_raw_fd(descriptors[0]) };
    // SAFETY: descriptor one has no Rust owner and is no longer needed.
    assert_eq!(unsafe { libc::close(descriptors[1]) }, 0);
    assert!(matches!(
        Daemon::bind(test_daemon_config(&datagram_root).with_supervisor_stream(datagram_as_stream),),
        Err(DaemonError::InvalidSupervisorStream)
    ));
    fs::remove_dir_all(datagram_root).unwrap();

    let contained_root = temporary_runtime_root("supervisor-cloexec");
    let (_supervisor, daemon_stream) = UnixStream::pair().unwrap();
    let daemon =
        Daemon::bind(test_daemon_config(&contained_root).with_supervisor_stream(daemon_stream))
            .unwrap();
    assert_eq!(
        daemon.supervisor_authority_close_on_exec().unwrap(),
        Some(true)
    );
    drop(daemon);
    fs::remove_dir_all(contained_root).unwrap();
}

#[test]
fn refuses_runtime_root_indirection_or_unsafe_modes() {
    let root_target = temporary_runtime_root("root-target");
    fs::create_dir(&root_target).unwrap();
    fs::set_permissions(&root_target, fs::Permissions::from_mode(0o700)).unwrap();
    let root_alias = temporary_runtime_root("root-alias");
    std::os::unix::fs::symlink(&root_target, &root_alias).unwrap();
    assert!(matches!(
        Daemon::bind(test_daemon_config(&root_alias)),
        Err(DaemonError::UnsafeRuntimeRoot)
    ));
    fs::remove_file(root_alias).unwrap();
    fs::remove_dir_all(root_target).unwrap();

    let public_root = temporary_runtime_root("root-mode");
    fs::create_dir(&public_root).unwrap();
    fs::set_permissions(&public_root, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        Daemon::bind(test_daemon_config(&public_root)),
        Err(DaemonError::UnsafeRuntimeRoot)
    ));
    fs::remove_dir_all(public_root).unwrap();
}

#[test]
fn sigint_and_sigterm_request_one_orderly_process_shutdown() {
    let root = temporary_runtime_root("signals");
    let daemon = Daemon::bind(test_daemon_config(&root)).unwrap();
    let shutdown = daemon.shutdown_handle().with_process_signals().unwrap();
    assert!(matches!(
        daemon.shutdown_handle().with_process_signals(),
        Err(DaemonError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists
    ));
    let loop_shutdown = shutdown.clone();
    let join = thread::spawn(move || {
        let mut daemon = daemon;
        daemon.serve_until(&loop_shutdown)
    });
    // SAFETY: this test has installed the process-local bridge; it sends the
    // two handled termination signals to this same test process only.
    unsafe {
        assert_eq!(libc::raise(libc::SIGINT), 0);
        assert_eq!(libc::raise(libc::SIGTERM), 0);
    }
    assert!(shutdown.is_requested());
    assert!(join.join().unwrap().is_ok());
    drop(shutdown);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reconnect_is_idempotent_qualification_is_closed_and_empty_deterministic_then_live_cycles_close()
{
    let root = temporary_runtime_root("cycles");
    let (_client, mut supervisor, shutdown, join, _socket_path, mode) =
        start(&root, FaultInjection::None);
    assert_eq!(mode, StartupMode::FreshServing);
    founding_commands(&mut supervisor, 10);
    let repeated = supervisor
        .execute(
            correlation(16),
            command(
                "daemon-found-bootstrap",
                BOOTSTRAP,
                6,
                Capability::BootstrapSociety,
                ExpectedGeneration::NotApplicable,
                ClientCommandBody::BootstrapSociety,
            ),
        )
        .unwrap();
    assert!(matches!(
        repeated,
        CommandReceiptView::Accepted {
            idempotent: true,
            ..
        }
    ));

    let qualification = execute_with_active_grant(
        &mut supervisor,
        correlation(30),
        "daemon-propose-disallowed-qualification",
        ROOT_AUTHORITY,
        Capability::ProposeOperatingCycle,
        ExpectedGeneration::NotApplicable,
        ClientCommandBody::ProposeOperatingCycle {
            treatment: OperatingCycleTreatment::PiSdkQualificationV1,
            budget_ceiling: UsdMicros::new(10_000).unwrap(),
        },
    )
    .unwrap();
    assert!(matches!(
        qualification,
        CommandReceiptView::Rejected {
            rejection: society_kernel::Rejection::QualificationTreatmentRestricted,
            idempotent: false,
        }
    ));

    admit_and_close_empty_cycle(
        &mut supervisor,
        40,
        OperatingCycleTreatment::DeterministicPiHostFixtureV1,
        UsdMicros::new(10_000).unwrap(),
        OperatingCycleId::new(1).unwrap(),
    );
    admit_and_close_empty_cycle(
        &mut supervisor,
        50,
        OperatingCycleTreatment::PinnedPiSdkLiveV1,
        UsdMicros::new(10_000).unwrap(),
        OperatingCycleId::new(2).unwrap(),
    );
    stop(shutdown, join);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn restart_replays_receipts_but_refuses_new_work_until_kernel_recovery_exists() {
    let root = temporary_runtime_root("restart");
    let (_client, mut supervisor, shutdown, join, _socket_path, _) =
        start(&root, FaultInjection::None);
    let original = command(
        "restart-create-society",
        BOOTSTRAP,
        1,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        ClientCommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("restart society").unwrap(),
        },
    );
    accepted(
        supervisor
            .execute(correlation(1), original.clone())
            .unwrap(),
    );
    stop(shutdown, join);

    let (_client, mut supervisor, shutdown, join, _socket_path, mode) =
        start(&root, FaultInjection::None);
    assert_eq!(mode, StartupMode::RecoveryFenced);
    assert!(matches!(
        supervisor.execute(correlation(2), original).unwrap(),
        CommandReceiptView::Accepted {
            idempotent: true,
            ..
        }
    ));
    assert!(matches!(
        supervisor.execute(
            correlation(3),
            command(
                "restart-new-command-is-fenced",
                BOOTSTRAP,
                2,
                Capability::InstallRootAuthorityOffice,
                ExpectedGeneration::NotApplicable,
                ClientCommandBody::InstallRootAuthorityOffice,
            ),
        ),
        Err(SocietyctlError::Daemon(ProtocolErrorCode::RecoveryFenced))
    ));
    stop(shutdown, join);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn deterministic_crash_seams_preserve_the_commit_boundary() {
    let before_root = temporary_runtime_root("crash-before");
    let (_client, mut supervisor, _shutdown, join, _socket_path, _) =
        start(&before_root, FaultInjection::BeforeNextCommandCommit);
    let before = command(
        "crash-before-create",
        BOOTSTRAP,
        1,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        ClientCommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("before crash society").unwrap(),
        },
    );
    assert!(supervisor.execute(correlation(1), before.clone()).is_err());
    assert!(matches!(
        join.join().unwrap(),
        Err(DaemonError::InjectedCrashBeforeCommit)
    ));
    let (_client, mut supervisor, shutdown, join, _socket_path, mode) =
        start(&before_root, FaultInjection::None);
    assert_eq!(mode, StartupMode::FreshServing);
    assert!(matches!(
        supervisor.execute(correlation(2), before).unwrap(),
        CommandReceiptView::Accepted {
            idempotent: false,
            ..
        }
    ));
    stop(shutdown, join);
    fs::remove_dir_all(before_root).unwrap();

    let after_root = temporary_runtime_root("crash-after");
    let (_client, mut supervisor, _shutdown, join, _socket_path, _) =
        start(&after_root, FaultInjection::AfterNextCommandCommit);
    let after = command(
        "crash-after-create",
        BOOTSTRAP,
        1,
        Capability::CreateSocietyIdentity,
        ExpectedGeneration::NotApplicable,
        ClientCommandBody::CreateSocietyIdentity {
            name: SocietyName::parse("after crash society").unwrap(),
        },
    );
    assert!(supervisor.execute(correlation(3), after.clone()).is_err());
    assert!(matches!(
        join.join().unwrap(),
        Err(DaemonError::InjectedCrashAfterCommit)
    ));
    let (_client, mut supervisor, shutdown, join, _socket_path, mode) =
        start(&after_root, FaultInjection::None);
    assert_eq!(mode, StartupMode::RecoveryFenced);
    assert!(matches!(
        supervisor.execute(correlation(4), after).unwrap(),
        CommandReceiptView::Accepted {
            idempotent: true,
            ..
        }
    ));
    stop(shutdown, join);
    fs::remove_dir_all(after_root).unwrap();
}

#[test]
fn founding_mission_digest_mismatch_and_preflight_rejection_leave_no_content_side_effect() {
    let mismatch_root = temporary_runtime_root("mission-digest-mismatch");
    let (_client, mut supervisor, shutdown, join, _socket_path, _) =
        start(&mismatch_root, FaultInjection::None);
    create_society(&mut supervisor, 10, "mission-mismatch-society");
    let mismatched = ClientCommandBody::InstallFoundingMission {
        mission: Box::new(resident_application_mission()),
        source_rendering: changed_resident_mission_source_rendering(),
    };
    assert!(matches!(
        execute_with_active_grant(
            &mut supervisor,
            correlation(11),
            "mission-mismatch",
            BOOTSTRAP,
            Capability::InstallFoundingMission,
            ExpectedGeneration::NotApplicable,
            mismatched,
        ),
        Err(SocietyctlError::Daemon(
            ProtocolErrorCode::MissionSourceDigestMismatch
        ))
    ));
    stop(shutdown, join);
    let kernel =
        KernelStore::connect_test_path(mismatch_root.join("society.pg-test-schema")).unwrap();
    assert_eq!(kernel.command_count().unwrap(), 1);
    assert!(
        kernel
            .command_receipt(&CommandId::parse("mission-mismatch").unwrap())
            .unwrap()
            .is_none()
    );
    assert_eq!(
        kernel
            .content_identity_state(resident_application_mission().source_rendering_digest)
            .unwrap(),
        ContentIdentityState::Absent
    );
    drop(kernel);
    fs::remove_dir_all(mismatch_root).unwrap();

    let rejected_root = temporary_runtime_root("mission-preflight-rejection");
    let (_client, mut supervisor, shutdown, join, _socket_path, _) =
        start(&rejected_root, FaultInjection::None);
    let receipt = execute_with_active_grant(
        &mut supervisor,
        correlation(20),
        "mission-preflight-rejected",
        BOOTSTRAP,
        Capability::InstallFoundingMission,
        ExpectedGeneration::NotApplicable,
        founding_mission_body(),
    )
    .unwrap();
    assert!(matches!(receipt, CommandReceiptView::Rejected { .. }));
    stop(shutdown, join);
    let kernel =
        KernelStore::connect_test_path(rejected_root.join("society.pg-test-schema")).unwrap();
    assert_eq!(kernel.command_count().unwrap(), 1);
    assert_eq!(
        kernel
            .content_identity_state(resident_application_mission().source_rendering_digest)
            .unwrap(),
        ContentIdentityState::Absent
    );
    drop(kernel);
    fs::remove_dir_all(rejected_root).unwrap();
}

#[test]
fn founding_mission_exact_retry_and_conflict_are_decided_before_resealing() {
    let root = temporary_runtime_root("mission-exact-retry");
    let (_client, mut supervisor, shutdown, join, _socket_path, _) =
        start(&root, FaultInjection::None);
    create_society(&mut supervisor, 30, "mission-retry-society");
    let grant = active_grant(
        &mut supervisor,
        BOOTSTRAP,
        Capability::InstallFoundingMission,
    );
    let original = command(
        "mission-retry",
        BOOTSTRAP,
        grant,
        Capability::InstallFoundingMission,
        ExpectedGeneration::NotApplicable,
        founding_mission_body(),
    );
    let first = supervisor
        .execute(correlation(31), original.clone())
        .unwrap();
    assert!(matches!(
        first,
        CommandReceiptView::Accepted {
            idempotent: false,
            ..
        }
    ));
    let retry = supervisor.execute(correlation(32), original).unwrap();
    assert!(matches!(
        retry,
        CommandReceiptView::Accepted {
            idempotent: true,
            ..
        }
    ));

    let mut changed_mission = resident_application_mission();
    changed_mission.source_rendering_digest =
        Blake3Digest::of_bytes(changed_resident_mission_source_rendering().as_bytes());
    assert!(matches!(
        supervisor.execute(
            correlation(33),
            command(
                "mission-retry",
                BOOTSTRAP,
                grant,
                Capability::InstallFoundingMission,
                ExpectedGeneration::NotApplicable,
                ClientCommandBody::InstallFoundingMission {
                    mission: Box::new(changed_mission),
                    source_rendering: changed_resident_mission_source_rendering(),
                },
            )
        ),
        Err(SocietyctlError::Daemon(
            ProtocolErrorCode::IdempotencyConflict
        ))
    ));
    stop(shutdown, join);
    let kernel = KernelStore::connect_test_path(root.join("society.pg-test-schema")).unwrap();
    assert_eq!(kernel.command_count().unwrap(), 4);
    assert!(matches!(
        kernel
            .content_identity_state(resident_application_mission().source_rendering_digest)
            .unwrap(),
        ContentIdentityState::Registered { .. }
    ));
    let operation =
        mission_source_operation_label(resident_application_mission().source_rendering_digest);
    assert!(
        kernel
            .command_receipt(
                &CommandId::parse(format!("content-seal-v1/{operation}/receipt")).unwrap()
            )
            .unwrap()
            .is_some()
    );
    assert!(
        kernel
            .command_receipt(
                &CommandId::parse(format!("content-seal-v1/{operation}/object")).unwrap()
            )
            .unwrap()
            .is_some()
    );
    assert_eq!(
        kernel
            .content_identity_state(Blake3Digest::of_bytes(
                changed_resident_mission_source_rendering().as_bytes()
            ))
            .unwrap(),
        ContentIdentityState::Absent
    );
    drop(kernel);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn founding_mission_crash_boundaries_leave_a_recovery_fenced_successor() {
    for (label, fault, expected_command_count, expected_identity) in [
        (
            "physical",
            FaultInjection::AfterFoundingMissionPhysicalSeal,
            1,
            ContentIdentityState::Absent,
        ),
        (
            "receipt",
            FaultInjection::AfterFoundingMissionReceipt,
            2,
            ContentIdentityState::SealReceiptOnly {
                content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(1).unwrap(),
            },
        ),
        (
            "object",
            FaultInjection::AfterFoundingMissionObjectRegistrationBeforeOuterCommand,
            3,
            ContentIdentityState::Registered {
                content_seal_receipt_id: society_kernel::ContentSealReceiptId::new(1).unwrap(),
                content_object_id: society_kernel::ContentObjectId::new(1).unwrap(),
            },
        ),
    ] {
        let root = temporary_runtime_root(&format!("mission-crash-{label}"));
        let (_client, mut supervisor, _shutdown, join, _socket_path, _) = start(&root, fault);
        create_society(
            &mut supervisor,
            40,
            &format!("mission-crash-society-{label}"),
        );
        let original = command(
            &format!("mission-crash-{label}"),
            BOOTSTRAP,
            active_grant(
                &mut supervisor,
                BOOTSTRAP,
                Capability::InstallFoundingMission,
            ),
            Capability::InstallFoundingMission,
            ExpectedGeneration::NotApplicable,
            founding_mission_body(),
        );
        assert!(
            supervisor
                .execute(correlation(41), original.clone())
                .is_err()
        );
        assert!(matches!(
            join.join().unwrap(),
            Err(DaemonError::InjectedCrashAfterFoundingMissionPhysicalSeal)
                | Err(DaemonError::InjectedCrashAfterFoundingMissionReceipt)
                | Err(DaemonError::InjectedCrashAfterFoundingMissionObjectRegistration)
        ));
        let kernel = KernelStore::connect_test_path(root.join("society.pg-test-schema")).unwrap();
        assert_eq!(kernel.command_count().unwrap(), expected_command_count);
        assert_eq!(
            kernel
                .content_identity_state(resident_application_mission().source_rendering_digest)
                .unwrap(),
            expected_identity
        );
        drop(kernel);

        let (_client, mut supervisor, shutdown, join, _socket_path, mode) =
            start(&root, FaultInjection::None);
        assert_eq!(mode, StartupMode::RecoveryFenced);
        assert!(matches!(
            supervisor.execute(correlation(42), original),
            Err(SocietyctlError::Daemon(ProtocolErrorCode::RecoveryFenced))
        ));
        stop(shutdown, join);
        fs::remove_dir_all(root).unwrap();
    }

    let root = temporary_runtime_root("mission-crash-outer-commit");
    let (_client, mut supervisor, _shutdown, join, _socket_path, _) = start(
        &root,
        FaultInjection::AfterFoundingMissionOuterCommitBeforeResponse,
    );
    create_society(&mut supervisor, 50, "mission-outer-commit-society");
    let original = command(
        "mission-crash-outer-commit",
        BOOTSTRAP,
        active_grant(
            &mut supervisor,
            BOOTSTRAP,
            Capability::InstallFoundingMission,
        ),
        Capability::InstallFoundingMission,
        ExpectedGeneration::NotApplicable,
        founding_mission_body(),
    );
    assert!(
        supervisor
            .execute(correlation(51), original.clone())
            .is_err()
    );
    assert!(matches!(
        join.join().unwrap(),
        Err(DaemonError::InjectedCrashAfterFoundingMissionOuterCommit)
    ));
    let (_client, mut supervisor, shutdown, join, _socket_path, mode) =
        start(&root, FaultInjection::None);
    assert_eq!(mode, StartupMode::RecoveryFenced);
    assert!(matches!(
        supervisor.execute(correlation(52), original).unwrap(),
        CommandReceiptView::Accepted {
            idempotent: true,
            ..
        }
    ));
    stop(shutdown, join);
    fs::remove_dir_all(root).unwrap();
}

fn raw_frame(socket_path: &PathBuf, payload: &[u8]) {
    let mut stream = UnixStream::connect(socket_path).unwrap();
    protocol::write_frame(&mut stream, payload).unwrap();
}
