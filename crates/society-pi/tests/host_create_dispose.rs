//! A real-pipe, provider-free integration of the committed TypeScript host.
//!
//! The test intentionally never sends `Prompt`: `CreateSession` and `Dispose`
//! exercise exact Pi SDK construction, the no-prompt transcript flush path,
//! and Rust's closed stdout peer without funding a model turn.

// This executable fixture deliberately fails at its setup and I/O boundaries;
// concise panics keep those failures local without weakening production code.
#![allow(clippy::unwrap_used)]

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use miniserde::json::{self, Number, Value};
use society_pi::{
    AdapterVersion, Blake3Digest, BoundaryPeer, HostProcessId, NodeRuntimeVersion, PiSdkVersion,
    RuntimeIdentity, SessionIdentity, SpawnNonce,
};

const SESSION: &str = "rust-pipe-session-001";
const NONCE: &str = "rust-pipe-nonce-001";
const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn committed_host_create_then_dispose_is_sealed_without_a_provider_call() {
    let root = temporary_directory("host-create-dispose");
    let agent = root.join("agent");
    let sessions = root.join("sessions");
    fs::create_dir_all(&agent).unwrap();
    fs::create_dir_all(&sessions).unwrap();
    let auth = agent.join("auth.json");
    let models = agent.join("models.json");
    fs::write(&auth, "{}").unwrap();
    let catalog = admitted_catalog();
    fs::write(&models, &catalog).unwrap();

    let system_prompt = "Founding Mission\nProvider-free construction receipt";
    let catalog_blake3 = blake3(&catalog);
    let create = envelope(
        1,
        "create-correlation-001",
        "CreateSession",
        object([
            ("sessionKind", string("TaskAttempt")),
            ("cwd", string(root.to_str().unwrap())),
            ("agentDirectory", string(agent.to_str().unwrap())),
            ("authPath", string(auth.to_str().unwrap())),
            ("modelsPath", string(models.to_str().unwrap())),
            ("sessionDirectory", string(sessions.to_str().unwrap())),
            ("systemPrompt", string(system_prompt)),
            ("systemPromptDigest", string(blake3(system_prompt))),
            (
                "model",
                object([
                    ("provider", string("openrouter")),
                    ("modelId", string("deepseek/deepseek-v4-flash-0731")),
                    ("thinkingLevel", string("high")),
                ]),
            ),
            (
                "modelCatalog",
                object([
                    ("catalogBlake3", string(catalog_blake3)),
                    ("effectiveModel", admitted_effective_model()),
                ]),
            ),
            ("toolProfile", string("read_execute_v1")),
            ("settings", admitted_settings()),
            (
                "forumContract",
                object([
                    ("kind", string("forum_enabled_v1")),
                    (
                        "awarenessBlake3",
                        string("b058dadccdc7c3fb8e2e3558bd16e726e1f00aa60fda5a849da20eb6e86ad46a"),
                    ),
                    (
                        "toolContractBlake3",
                        string("738e664f66be09dfb7f8e5e4873521d7b9f1600d385dd0c8a41c80ca087566be"),
                    ),
                ]),
            ),
        ]),
    );
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository = manifest.parent().and_then(std::path::Path::parent).unwrap();
    // `dist/` is ignored: a fresh checkout must not pretend it contains a
    // compiled adapter. The provider-free workspace judge builds the pinned
    // host with `npm ci && npm test --prefix packages/society-pi-host`, hashes
    // its exact entrypoint, then supplies both values to this test.
    let entry = std::env::var_os("SOCIETY_PI_HOST_ENTRYPOINT")
        .map(std::path::PathBuf::from)
        .expect("set SOCIETY_PI_HOST_ENTRYPOINT to the tested pinned host build");
    let adapter_build_blake3 = std::env::var("SOCIETY_PI_HOST_BUILD_BLAKE3")
        .expect("set SOCIETY_PI_HOST_BUILD_BLAKE3 to BLAKE3(SOCIETY_PI_HOST_ENTRYPOINT)");
    assert_eq!(
        blake3(fs::read(&entry).expect("host entrypoint must be readable")),
        adapter_build_blake3
    );
    assert!(
        entry.is_file(),
        "SOCIETY_PI_HOST_ENTRYPOINT must be a regular built adapter file"
    );
    let node_version = String::from_utf8(
        Command::new("node")
            .arg("--version")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let runtime = RuntimeIdentity {
        node_version: NodeRuntimeVersion::parse(node_version.trim()).unwrap(),
        adapter_version: AdapterVersion::V1,
        pi_sdk_version: PiSdkVersion::V0830,
        node_executable_blake3: Blake3Digest::parse(DIGEST).unwrap(),
        lockfile_blake3: Blake3Digest::parse(DIGEST).unwrap(),
        adapter_build_blake3: Blake3Digest::parse(&adapter_build_blake3).unwrap(),
        pi_transitive_package_set_blake3: Blake3Digest::parse(DIGEST).unwrap(),
    };
    let mut child = Command::new("node")
        .arg(entry)
        .args([
            "--session-identity",
            SESSION,
            "--spawn-nonce",
            NONCE,
            "--node-executable-blake3",
            DIGEST,
            "--lockfile-blake3",
            DIGEST,
            "--adapter-build-blake3",
            &adapter_build_blake3,
            "--pi-transitive-package-set-blake3",
            DIGEST,
        ])
        .current_dir(repository)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut peer = BoundaryPeer::new(
        SessionIdentity::parse(SESSION).unwrap(),
        HostProcessId::parse(u64::from(child.id())).unwrap(),
        SpawnNonce::parse(NONCE).unwrap(),
        runtime,
    )
    .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stdout = BufReader::new(stdout);
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    peer.observe_outbound_jsonl(line.trim_end()).unwrap(); // AdapterReady
    peer.admit_inbound_jsonl(&create).unwrap();
    writeln!(stdin, "{create}").unwrap();
    line.clear();
    stdout.read_line(&mut line).unwrap();
    peer.observe_outbound_jsonl(line.trim_end()).unwrap(); // Create result
    line.clear();
    stdout.read_line(&mut line).unwrap();
    peer.observe_outbound_jsonl(line.trim_end()).unwrap(); // SessionReady
    let dispose = envelope(
        2,
        "dispose-correlation-001",
        "Dispose",
        object([("reason", string("ProcessRecovery"))]),
    );
    peer.admit_inbound_jsonl(&dispose).unwrap();
    writeln!(stdin, "{dispose}").unwrap();
    drop(stdin);
    for _ in 0..3 {
        line.clear();
        stdout.read_line(&mut line).unwrap();
        peer.observe_outbound_jsonl(line.trim_end()).unwrap();
    }
    assert!(stdout.read_line(&mut line).unwrap() == 0);
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "no Prompt means no provider call"
    );
    assert!(output.stderr.is_empty());
    assert_eq!(peer.phase(), society_pi::PeerPhase::Disposed);
    assert_eq!(peer.inbound_seals().len(), 2);
    assert_eq!(peer.outbound_seals().len(), 6);
    let _ = fs::remove_dir_all(&root);
}

fn envelope(sequence: u64, correlation: &str, command: &str, payload: Value) -> String {
    json::to_string(&object([
        ("protocolVersion", string("society-pi-host/v4")),
        ("sequence", number_u64(sequence)),
        ("sessionIdentity", string(SESSION)),
        ("correlationIdentity", string(correlation)),
        ("command", string(command)),
        ("payload", payload),
    ]))
}
fn admitted_catalog() -> String {
    json::to_string(&object([(
        "providers",
        object([(
            "openrouter",
            object([
                ("baseUrl", string("https://openrouter.ai/api/v1")),
                ("api", string("openai-completions")),
                (
                    "models",
                    array([object([
                        ("id", string("deepseek/deepseek-v4-flash-0731")),
                        ("name", string("admitted")),
                        ("reasoning", Value::Bool(true)),
                        ("input", array([string("text")])),
                        ("contextWindow", number_u64(1_048_576)),
                        ("maxTokens", number_u64(384_000)),
                        (
                            "cost",
                            object([
                                ("input", number_f64(0.00000009)),
                                ("output", number_f64(0.00000018)),
                                ("cacheRead", number_f64(0.000000018)),
                                ("cacheWrite", number_u64(0)),
                            ]),
                        ),
                    ])]),
                ),
            ]),
        )]),
    )]))
}
fn admitted_effective_model() -> Value {
    object([
        ("provider", string("openrouter")),
        ("baseUrl", string("https://openrouter.ai/api/v1")),
        ("api", string("openai-completions")),
        ("modelId", string("deepseek/deepseek-v4-flash-0731")),
        (
            "canonicalSlug",
            string("deepseek/deepseek-v4-flash-20260731"),
        ),
        ("input", string("text_only")),
        ("contextWindow", number_u64(1_048_576)),
        ("maxTokens", number_u64(384_000)),
        (
            "inputUsdPerMillion",
            object([("kind", string("Known")), ("usdPerMillion", string("0.09"))]),
        ),
        (
            "outputUsdPerMillion",
            object([("kind", string("Known")), ("usdPerMillion", string("0.18"))]),
        ),
        (
            "cacheReadUsdPerMillion",
            object([
                ("kind", string("Known")),
                ("usdPerMillion", string("0.018")),
            ]),
        ),
        (
            "cacheWriteUsdPerMillion",
            object([("kind", string("Absent"))]),
        ),
    ])
}
fn admitted_settings() -> Value {
    object([
        (
            "retry",
            object([
                ("maxRetries", number_u64(2)),
                ("baseDelayMilliseconds", number_u64(2_000)),
                ("providerTimeoutMilliseconds", number_u64(300_000)),
                ("providerMaxRetries", number_u64(1)),
                ("providerMaxRetryDelayMilliseconds", number_u64(30_000)),
            ]),
        ),
        (
            "compaction",
            object([
                ("mode", string("enabled")),
                ("reserveTokens", number_u64(16_384)),
                ("keepRecentTokens", number_u64(20_000)),
            ]),
        ),
        ("steeringMode", string("one-at-a-time")),
        ("followUpMode", string("one-at-a-time")),
        ("transport", string("sse")),
        ("projectTrust", string("never")),
        ("installTelemetryEnabled", Value::Bool(false)),
        ("analyticsEnabled", Value::Bool(false)),
        ("images", string("blocked")),
    ])
}
fn object(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}
fn array(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(values.into_iter().collect())
}
fn string(value: impl Into<String>) -> Value {
    Value::String(value.into())
}
fn number_u64(value: u64) -> Value {
    Value::Number(Number::U64(value))
}
fn number_f64(value: f64) -> Value {
    Value::Number(Number::F64(value))
}
fn blake3(value: impl AsRef<[u8]>) -> String {
    let digest = blake3::hash(value.as_ref());
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn temporary_directory(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("society-pi-{label}-{}-{nonce}", std::process::id()))
}
