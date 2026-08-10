#![allow(clippy::unwrap_used)]

use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

use society_xsh_circuit::{Vs001CurationInputRoleV1, MAX_DIRECT_CURATION_MANIFEST_BYTES};

static NEXT_TEMPORARY_DIRECTORY: AtomicU64 = AtomicU64::new(1);

macro_rules! curation_bytes {
    ($path:literal) => {
        include_bytes!(concat!(
            "../../circuits/vs-001-spawn-stderr/fixtures/curation",
            $path
        ))
    };
}

fn temporary_directory() -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "society-xsh-direct-adapter-binary-{}-{}",
        std::process::id(),
        NEXT_TEMPORARY_DIRECTORY.fetch_add(1, Ordering::Relaxed),
    ));
    fs::create_dir(&directory).unwrap();
    directory
}

fn valid_manifest() -> Vec<u8> {
    let relations = [
        curation_bytes!("/c1-valid/account.v1.tsv").as_slice(),
        curation_bytes!("/c1-valid/selected-items.v1.tsv").as_slice(),
        curation_bytes!("/c1-valid/preserved-conflicts.v1.tsv").as_slice(),
        curation_bytes!("/c1-valid/decision-relevant-unknowns.v1.tsv").as_slice(),
        curation_bytes!("/c1-valid/exclusions.v1.tsv").as_slice(),
        curation_bytes!("/c1-valid/raw-evidence-escalations.v1.tsv").as_slice(),
        curation_bytes!("/frontier-c1-members.v1.tsv").as_slice(),
    ];
    let mut manifest = b"# schema: Vs001CurationDirectInputManifestV1/framed-v1\n".to_vec();
    for (role, relation) in Vs001CurationInputRoleV1::ORDERED.iter().zip(relations) {
        manifest.extend_from_slice(role.wire_name().as_bytes());
        manifest.extend_from_slice(format!("\t{}\n", relation.len()).as_bytes());
        manifest.extend_from_slice(relation);
    }
    manifest
}

#[test]
fn compiled_adapter_accepts_only_the_fixed_verified_manifest_abi() {
    let workspace = temporary_directory();
    let manifest_path = workspace.join("curation-input.v1");
    fs::write(&manifest_path, valid_manifest()).unwrap();
    let adapter = env!("CARGO_BIN_EXE_vs001-direct-evaluator-adapter");

    let accepted = Command::new(adapter)
        .arg("--input-manifest")
        .arg(&manifest_path)
        .env_clear()
        .output()
        .unwrap();
    assert!(accepted.status.success());
    assert_eq!(
        accepted.stdout,
        include_bytes!("fixtures/curation-direct-output.none.v1.framed"),
    );
    assert!(accepted.stderr.is_empty());

    let extra = Command::new(adapter)
        .arg("--input-manifest")
        .arg(&manifest_path)
        .arg("--extra")
        .env_clear()
        .output()
        .unwrap();
    assert!(!extra.status.success());
    assert!(String::from_utf8_lossy(&extra.stderr).contains("usage:"));

    let relative = Command::new(adapter)
        .arg("--input-manifest")
        .arg("curation-input.v1")
        .env_clear()
        .current_dir(&workspace)
        .output()
        .unwrap();
    assert!(!relative.status.success());
    assert!(String::from_utf8_lossy(&relative.stderr).contains("usage:"));

    let over_limit_path = workspace.join("over-limit-input.v1");
    fs::write(
        &over_limit_path,
        vec![b'x'; MAX_DIRECT_CURATION_MANIFEST_BYTES + 1],
    )
    .unwrap();
    let over_limit = Command::new(adapter)
        .arg("--input-manifest")
        .arg(&over_limit_path)
        .env_clear()
        .output()
        .unwrap();
    assert!(!over_limit.status.success());
    assert!(String::from_utf8_lossy(&over_limit.stderr).contains("exceeds 131072 bytes"));

    fs::remove_dir_all(workspace).unwrap();
}
