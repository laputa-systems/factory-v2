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

use serde_json::json;
use sha2::{Digest, Sha256};
use society_pi::{
    AdapterVersion, BoundaryPeer, HostProcessId, NodeRuntimeVersion, PiSdkVersion, RuntimeIdentity,
    SessionIdentity, Sha256Digest, SpawnNonce,
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

    let system_prompt = "Universe Seed\nProvider-free construction receipt";
    let catalog_sha256 = sha256(&catalog);
    let create = envelope(
        1,
        "create-correlation-001",
        "CreateSession",
        json!({
            "sessionKind": "TaskAttempt",
            "cwd": root.to_str().unwrap(),
            "agentDirectory": agent.to_str().unwrap(),
            "authPath": auth.to_str().unwrap(),
            "modelsPath": models.to_str().unwrap(),
            "sessionDirectory": sessions.to_str().unwrap(),
            "systemPrompt": system_prompt,
            "systemPromptDigest": sha256(system_prompt),
            "model": { "provider": "openrouter", "modelId": "deepseek/deepseek-v4-flash-0731", "thinkingLevel": "high" },
            "modelCatalog": { "catalogSha256": catalog_sha256, "effectiveModel": admitted_effective_model() },
            "toolProfile": "read_source_v1",
            "settings": admitted_settings(),
        }),
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
    let adapter_build_sha256 = std::env::var("SOCIETY_PI_HOST_BUILD_SHA256")
        .expect("set SOCIETY_PI_HOST_BUILD_SHA256 to SHA-256(SOCIETY_PI_HOST_ENTRYPOINT)");
    assert_eq!(
        sha256(fs::read(&entry).expect("host entrypoint must be readable")),
        adapter_build_sha256
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
        node_executable_sha256: Sha256Digest::parse(DIGEST).unwrap(),
        lockfile_sha256: Sha256Digest::parse(DIGEST).unwrap(),
        adapter_build_sha256: Sha256Digest::parse(&adapter_build_sha256).unwrap(),
        pi_transitive_package_set_sha256: Sha256Digest::parse(DIGEST).unwrap(),
    };
    let mut child = Command::new("node")
        .arg(entry)
        .args([
            "--session-identity",
            SESSION,
            "--spawn-nonce",
            NONCE,
            "--node-executable-sha256",
            DIGEST,
            "--lockfile-sha256",
            DIGEST,
            "--adapter-build-sha256",
            &adapter_build_sha256,
            "--pi-transitive-package-set-sha256",
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
        json!({ "reason": "ProcessRecovery" }),
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

fn envelope(sequence: u64, correlation: &str, command: &str, payload: serde_json::Value) -> String {
    json!({ "protocolVersion":"society-pi-host/v1", "sequence":sequence, "sessionIdentity":SESSION, "correlationIdentity":correlation, "command":command, "payload":payload }).to_string()
}
fn admitted_catalog() -> String {
    json!({ "providers": { "openrouter": { "baseUrl":"https://openrouter.ai/api/v1", "api":"openai-completions", "models":[{ "id":"deepseek/deepseek-v4-flash-0731", "name":"admitted", "reasoning":true, "input":["text"], "contextWindow":1_048_576, "maxTokens":384_000, "cost":{"input":0.00000009,"output":0.00000018,"cacheRead":0.000000018,"cacheWrite":0} }] } } }).to_string()
}
fn admitted_effective_model() -> serde_json::Value {
    json!({ "provider":"openrouter", "baseUrl":"https://openrouter.ai/api/v1", "api":"openai-completions", "modelId":"deepseek/deepseek-v4-flash-0731", "canonicalSlug":"deepseek/deepseek-v4-flash-20260731", "input":"text_only", "contextWindow":1_048_576, "maxTokens":384_000, "inputUsdPerMillion":{"kind":"Known","usdPerMillion":"0.09"}, "outputUsdPerMillion":{"kind":"Known","usdPerMillion":"0.18"}, "cacheReadUsdPerMillion":{"kind":"Known","usdPerMillion":"0.018"}, "cacheWriteUsdPerMillion":{"kind":"Absent"} })
}
fn admitted_settings() -> serde_json::Value {
    json!({ "retry":{"maxRetries":2,"baseDelayMilliseconds":2_000,"providerTimeoutMilliseconds":300_000,"providerMaxRetries":1,"providerMaxRetryDelayMilliseconds":30_000}, "compaction":{"mode":"enabled","reserveTokens":16_384,"keepRecentTokens":20_000}, "steeringMode":"one-at-a-time", "followUpMode":"one-at-a-time", "transport":"sse", "projectTrust":"never", "installTelemetryEnabled":false, "analyticsEnabled":false, "images":"blocked" })
}
fn sha256(value: impl AsRef<[u8]>) -> String {
    let digest = Sha256::digest(value);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn temporary_directory(label: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("society-pi-{label}-{}-{nonce}", std::process::id()))
}
