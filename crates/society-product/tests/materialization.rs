//! Provider-free integration evidence for the guarded local Git boundary.
//!
//! Every repository is created under one unique temporary directory.  These
//! tests never touch an existing checkout, invoke a provider, or rely on a
//! shell for materialization itself.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use society_product::{
    ApplicationRevisionId, AssignedValidationProgram, AuthorizingDecisionId, BuilderAttemptId,
    CleanupEvidence, CommitIdentity, CommitMessage, CommitTimestamp, ControlledCommitSpec,
    DeliveryAuthorizationId, DeliveryDisposition, ExternallySupervisedValidationReceipt,
    ExternallySupervisedValidationStepReceipt, LocalBranchRef, OutputDigest, PatchArtifactRoot,
    ProductChangeAuthorizationInput, ProductChangeId, ProductError, ProductMaterializer,
    ProductState, ProductWorktreeBranch, ValidationCommand, ValidationProfile, ValidationProfileId,
    ValidationProgramArgument, ValidationProgramInvocation, WorktreeRoot,
};

const GIT: &str = "/usr/bin/git";
static TEMPORARY_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn exact_tree_delivery_is_controlled_no_hook_and_builder_worktree_is_retired() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree = fixture.create_builder(&qualification, "change-exact", "attempt-exact");
    fs::write(worktree.path().join("answer.txt"), "candidate exact tree\n").unwrap();
    let capture = fixture.capture(&worktree);
    let marker = fixture.root.path().join("poison-hook-ran");
    fixture.install_poison_pre_commit_hook(&marker);

    let materialized = fixture.materialize(&qualification, &capture, "delivery-exact");
    assert_eq!(
        materialized.materialized().state(),
        ProductState::Materialized
    );
    assert_eq!(materialized.materialized().base_tree(), capture.base_tree());
    assert_eq!(materialized.materialized().tree(), capture.candidate_tree());
    assert_eq!(
        materialized.commit_validated().state(),
        ProductState::CommitValidated
    );
    assert_eq!(
        materialized.commit_validated().commit().tree(),
        capture.candidate_tree()
    );
    assert_eq!(
        materialized.commit_validated().commit().parent(),
        qualification.admitted_base()
    );
    assert_eq!(
        materialized.commit_validated().commit().hook_policy(),
        society_product::HookPolicy::GitCommitTreeWithHooksDisabled
    );
    assert!(matches!(
        materialized.cleanup(),
        CleanupEvidence::Removed { .. }
    ));
    assert!(
        !marker.exists(),
        "commit-tree must not invoke pre-commit hooks"
    );

    let delivery = fixture
        .materializer
        .deliver(&qualification, &materialized)
        .unwrap();
    assert_eq!(delivery.disposition(), DeliveryDisposition::FastForwarded);
    assert_eq!(delivery.delivered_tree(), capture.candidate_tree());
    assert_eq!(&fixture.head(), delivery.delivered_commit());
    assert_eq!(&fixture.head_tree(), capture.candidate_tree());
    assert_eq!(
        fs::read_to_string(fixture.repo.join("answer.txt")).unwrap(),
        "candidate exact tree\n"
    );
    assert!(!marker.exists(), "delivery must not invoke the poison hook");

    let cleanup = fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
    assert!(matches!(cleanup, CleanupEvidence::Removed { .. }));
}

#[test]
fn moved_target_head_refuses_delivery_without_rebase() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree = fixture.create_builder(&qualification, "change-moved", "attempt-moved");
    fs::write(
        worktree.path().join("answer.txt"),
        "candidate must not rebase\n",
    )
    .unwrap();
    let capture = fixture.capture(&worktree);
    assert_eq!(
        capture
            .changed_paths()
            .iter()
            .map(society_product::RepositoryPath::as_str)
            .collect::<Vec<_>>(),
        vec!["answer.txt"]
    );
    let materialized = fixture.materialize(&qualification, &capture, "delivery-moved");

    fs::write(
        fixture.repo.join("other.txt"),
        "independent target advance\n",
    )
    .unwrap();
    fixture.git_ok(["add", "other.txt"]);
    fixture.git_ok(["commit", "-m", "advance target separately"]);
    let independently_moved = fixture.head();

    let error = fixture
        .materializer
        .deliver(&qualification, &materialized)
        .unwrap_err();
    assert!(matches!(
        error,
        ProductError::TargetHeadMoved { expected: _, actual } if actual == independently_moved
    ));
    assert_eq!(fixture.head(), independently_moved);
    assert_eq!(
        fs::read_to_string(fixture.repo.join("answer.txt")).unwrap(),
        "baseline\n"
    );
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

#[test]
fn tracked_and_untracked_source_contamination_are_refused() {
    let fixture = Fixture::new();
    fs::write(
        fixture.repo.join("answer.txt"),
        "uncommitted tracked edit\n",
    )
    .unwrap();
    let tracked = fixture
        .materializer
        .qualify_clean_source(&fixture.repository, fixture.target_ref())
        .unwrap_err();
    assert!(matches!(tracked, ProductError::SourceNotClean { .. }));

    fixture.git_ok(["reset", "--hard", "HEAD"]);
    fs::write(
        fixture.repo.join("untracked-evidence.txt"),
        "must not leak into a patch\n",
    )
    .unwrap();
    let untracked = fixture
        .materializer
        .qualify_clean_source(&fixture.repository, fixture.target_ref())
        .unwrap_err();
    assert!(matches!(untracked, ProductError::SourceNotClean { .. }));
}

#[test]
fn untracked_builder_file_is_rejected_instead_of_becoming_an_incomplete_patch() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree = fixture.create_builder(&qualification, "change-untracked", "attempt-untracked");
    fs::write(
        worktree.path().join("answer.txt"),
        "tracked candidate edit\n",
    )
    .unwrap();
    fs::write(
        worktree.path().join("not-in-portable-patch.txt"),
        "untracked\n",
    )
    .unwrap();

    let error = fixture
        .materializer
        .capture_candidate(&worktree, &fixture.artifacts)
        .unwrap_err();
    match error {
        ProductError::UntrackedCandidateFiles(paths) => {
            assert_eq!(paths.len(), 1);
            assert_eq!(paths[0].as_str(), "not-in-portable-patch.txt");
        }
        other => panic!("expected untracked candidate refusal, got {other:?}"),
    }
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

#[test]
fn builder_created_commit_is_refused_as_a_candidate_authority_boundary() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree = fixture.create_builder(&qualification, "change-commit", "attempt-commit");
    fs::write(worktree.path().join("answer.txt"), "actor-created commit\n").unwrap();
    git_ok_at(worktree.path(), ["add", "answer.txt"]);
    git_ok_at(
        worktree.path(),
        ["commit", "-m", "actor-created candidate commit"],
    );

    let error = fixture
        .materializer
        .capture_candidate(&worktree, &fixture.artifacts)
        .unwrap_err();
    assert!(matches!(
        error,
        ProductError::BuilderCommittedOrHeadMoved { .. }
    ));
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

#[test]
fn validation_failure_is_distinct_and_fresh_worktree_cleanup_is_recorded() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree =
        fixture.create_builder(&qualification, "change-validation", "attempt-validation");
    // `git diff --check --cached` is a deterministic built-in judge and fails
    // this exact trailing-whitespace candidate after fresh patch application.
    fs::write(
        worktree.path().join("answer.txt"),
        "candidate with trailing space \n",
    )
    .unwrap();
    let capture = fixture.capture(&worktree);
    let authorization_input =
        fixture.authorization_input(&qualification, &capture, "delivery-validation");
    let profile = fixture.validation_profile();
    let error = fixture
        .materializer
        .materialize(
            authorization_input,
            &capture,
            &profile,
            &fixture.commit_spec(),
            &fixture.worktrees,
        )
        .unwrap_err();
    match error {
        ProductError::MaterializationFailed { source, cleanup } => {
            assert!(matches!(*source, ProductError::ValidationFailed(_)));
            assert!(matches!(cleanup, CleanupEvidence::Removed { .. }));
        }
        other => panic!("expected materialization validation failure, got {other:?}"),
    }
    assert_eq!(&fixture.head(), qualification.admitted_base());
    assert_eq!(
        fs::read_to_string(fixture.repo.join("answer.txt")).unwrap(),
        "baseline\n"
    );
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

#[test]
fn tampered_patch_artifact_is_refused_before_any_fresh_worktree_is_created() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree = fixture.create_builder(&qualification, "change-tampered", "attempt-tampered");
    fs::write(
        worktree.path().join("answer.txt"),
        "candidate before artifact tamper\n",
    )
    .unwrap();
    let capture = fixture.capture(&worktree);
    fs::write(capture.patch().path(), b"not the accepted portable patch\n").unwrap();

    let error = fixture
        .materializer
        .materialize(
            fixture.authorization_input(&qualification, &capture, "delivery-tampered"),
            &capture,
            &fixture.validation_profile(),
            &fixture.commit_spec(),
            &fixture.worktrees,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ProductError::AcceptedPatchDigestMismatch { .. }
    ));
    assert_eq!(&fixture.head(), qualification.admitted_base());
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

#[test]
fn assigned_validation_program_refuses_a_shell_even_through_a_symlink() {
    let fixture = Fixture::new();
    let alias = fixture.root.path().join("assigned-validator");
    #[cfg(unix)]
    std::os::unix::fs::symlink("/bin/sh", &alias).unwrap();
    #[cfg(not(unix))]
    fs::copy("/bin/sh", &alias).unwrap();
    let error = AssignedValidationProgram::open(&alias).unwrap_err();
    assert!(matches!(
        error,
        ProductError::ShellValidationProgramDenied(_)
    ));
}

#[test]
fn derived_builder_branch_identity_is_unambiguous_across_hyphenated_ids() {
    let left = ProductWorktreeBranch::derive(
        &ProductChangeId::parse("alpha-beta").unwrap(),
        &BuilderAttemptId::parse("gamma").unwrap(),
    )
    .unwrap();
    let right = ProductWorktreeBranch::derive(
        &ProductChangeId::parse("alpha").unwrap(),
        &BuilderAttemptId::parse("beta-gamma").unwrap(),
    )
    .unwrap();
    assert_ne!(left.as_str(), right.as_str());
}

#[test]
fn validation_profile_rejects_more_than_its_closed_step_budget() {
    let commands = std::iter::repeat_n(ValidationCommand::GitDiffCheck, 33).collect();
    let error = ValidationProfile::new(
        ValidationProfileId::parse("too-many-validation-steps").unwrap(),
        commands,
    )
    .unwrap_err();
    assert!(matches!(error, ProductError::InvalidValidationProfile));
}

#[test]
fn preexisting_temporary_index_lock_is_a_collision_and_is_never_removed() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree =
        fixture.create_builder(&qualification, "change-index-lock", "attempt-index-lock");
    fs::write(worktree.path().join("answer.txt"), "candidate index lock\n").unwrap();
    let index_name = format!(
        ".guarded-materialization-index-{}-{}.lock",
        worktree.change().as_str(),
        worktree.attempt().as_str()
    );
    let excludes = fixture.root.path().join("candidate-index-excludes");
    fs::write(&excludes, format!("{index_name}\n")).unwrap();
    git_ok_at(
        worktree.path(),
        [
            "config",
            "core.excludesFile",
            excludes.to_str().expect("temporary path is UTF-8"),
        ],
    );
    let preexisting_lock = worktree.path().join(index_name);
    fs::write(&preexisting_lock, "not-owned\n").unwrap();

    let error = fixture
        .materializer
        .capture_candidate(&worktree, &fixture.artifacts)
        .unwrap_err();
    assert!(matches!(
        error,
        ProductError::TemporaryIndexAlreadyExists(path) if path == preexisting_lock
    ));
    assert_eq!(
        fs::read_to_string(&preexisting_lock).unwrap(),
        "not-owned\n"
    );

    fs::remove_file(&preexisting_lock).unwrap();
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

#[cfg(unix)]
#[test]
fn dangling_preexisting_temporary_index_lock_is_rejected_and_preserved() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree = fixture.create_builder(
        &qualification,
        "change-dangling-index-lock",
        "attempt-dangling-index-lock",
    );
    fs::write(
        worktree.path().join("answer.txt"),
        "candidate dangling index lock\n",
    )
    .unwrap();
    let index_name = format!(
        ".guarded-materialization-index-{}-{}.lock",
        worktree.change().as_str(),
        worktree.attempt().as_str()
    );
    let excludes = fixture.root.path().join("dangling-index-excludes");
    fs::write(&excludes, format!("{index_name}\n")).unwrap();
    git_ok_at(
        worktree.path(),
        [
            "config",
            "core.excludesFile",
            excludes.to_str().expect("temporary path is UTF-8"),
        ],
    );
    let dangling_lock = worktree.path().join(index_name);
    std::os::unix::fs::symlink("missing-index-lock-target", &dangling_lock).unwrap();

    let error = fixture
        .materializer
        .capture_candidate(&worktree, &fixture.artifacts)
        .unwrap_err();
    assert!(matches!(
        error,
        ProductError::TemporaryIndexAlreadyExists(path) if path == dangling_lock
    ));
    assert!(
        fs::symlink_metadata(&dangling_lock)
            .unwrap()
            .file_type()
            .is_symlink()
    );

    fs::remove_file(&dangling_lock).unwrap();
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

#[cfg(unix)]
#[test]
fn dangling_managed_worktree_entry_is_rejected_before_git_worktree_add() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let change = "change-dangling-worktree";
    let attempt = "attempt-dangling-worktree";
    let managed_name = format!(
        "builder-{}-{}-{}-{}",
        change.len(),
        change,
        attempt.len(),
        attempt
    );
    let dangling_path = fs::canonicalize(fixture.worktrees_path())
        .unwrap()
        .join(managed_name);
    std::os::unix::fs::symlink("missing-worktree-target", &dangling_path).unwrap();

    let error = fixture
        .materializer
        .create_product_worktree(
            &qualification,
            ProductChangeId::parse(change).unwrap(),
            BuilderAttemptId::parse(attempt).unwrap(),
            &fixture.worktrees,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ProductError::ManagedPathAlreadyExists(ref path) if path == &dangling_path
    ));
    assert!(
        fs::symlink_metadata(&dangling_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[cfg(unix)]
#[test]
fn dangling_managed_worktree_entry_is_not_declared_removed() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree = fixture.create_builder(
        &qualification,
        "change-dangling-retire",
        "attempt-dangling-retire",
    );
    let worktree_path = worktree.path().to_path_buf();
    git_ok_at(
        &fixture.repo,
        [
            "worktree",
            "remove",
            "--force",
            worktree_path.to_str().unwrap(),
        ],
    );
    std::os::unix::fs::symlink("missing-removed-worktree-target", &worktree_path).unwrap();
    let pretending_to_remove =
        ProductMaterializer::new(fixture.worktree_remove_success_git()).unwrap();

    let error = pretending_to_remove
        .retire_product_worktree(worktree)
        .unwrap_err();
    assert!(
        matches!(error, ProductError::WorktreeRemovalNotVerified(path) if path == worktree_path)
    );
    assert!(
        fs::symlink_metadata(&worktree_path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    fs::remove_file(worktree_path).unwrap();
}

#[test]
fn delivery_retry_is_idempotent_and_reopen_requires_a_descendant() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree = fixture.create_builder(&qualification, "change-retry", "attempt-retry");
    fs::write(worktree.path().join("answer.txt"), "candidate retry\n").unwrap();
    let capture = fixture.capture(&worktree);
    let materialized = fixture.materialize(&qualification, &capture, "delivery-retry");

    let first = fixture
        .materializer
        .deliver(&qualification, &materialized)
        .unwrap();
    let retry = fixture
        .materializer
        .deliver(&qualification, &materialized)
        .unwrap();
    assert_eq!(first.disposition(), DeliveryDisposition::FastForwarded);
    assert_eq!(retry.disposition(), DeliveryDisposition::AlreadyDelivered);
    assert_eq!(first.delivered_commit(), retry.delivered_commit());
    assert_eq!(first.delivered_tree(), retry.delivered_tree());

    let reopen =
        retry.reopen_as_descendant(ProductChangeId::parse("change-retry-descendant").unwrap());
    assert_eq!(reopen.state(), ProductState::Reopened);
    assert_eq!(reopen.prior_delivery(), &retry);
    assert_eq!(
        reopen.descendant_change().as_str(),
        "change-retry-descendant"
    );
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

#[cfg(unix)]
#[test]
fn post_cas_checkout_failure_requires_explicit_recovery_before_idempotent_retry() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree = fixture.create_builder(&qualification, "change-recovery", "attempt-recovery");
    fs::write(worktree.path().join("answer.txt"), "candidate recovery\n").unwrap();
    let capture = fixture.capture(&worktree);
    let materialized = fixture.materialize(&qualification, &capture, "delivery-recovery");
    let delivered_commit = materialized.commit_validated().commit().commit().clone();

    let failing_git = fixture.read_tree_failure_git();
    let failing_materializer = ProductMaterializer::new(&failing_git).unwrap();
    let error = failing_materializer
        .deliver(&qualification, &materialized)
        .unwrap_err();
    assert!(matches!(
        error,
        ProductError::DeliveryCheckoutRecoveryRequired {
            delivered_commit: observed,
            ..
        } if observed == delivered_commit
    ));
    assert_eq!(
        git_revision(&fixture.repo, "HEAD"),
        delivered_commit.to_hex()
    );

    let retry_error = fixture
        .materializer
        .deliver(&qualification, &materialized)
        .unwrap_err();
    assert!(matches!(
        retry_error,
        ProductError::DeliveryCheckoutRecoveryRequired {
            delivered_commit: observed,
            ..
        } if observed == delivered_commit
    ));

    let commit_hex = delivered_commit.to_hex();
    fixture.git_ok(["read-tree", "--reset", "-u", &commit_hex]);
    let recovered = fixture
        .materializer
        .deliver(&qualification, &materialized)
        .unwrap();
    assert_eq!(
        recovered.disposition(),
        DeliveryDisposition::AlreadyDelivered
    );
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

#[test]
fn snapshot_tree_prevents_late_builder_edits_from_recombining_patch_or_paths() {
    let fixture = Fixture::new();
    let qualification = fixture.qualification();
    let worktree = fixture.create_builder(&qualification, "change-snapshot", "attempt-snapshot");
    fs::write(worktree.path().join("answer.txt"), "sealed snapshot\n").unwrap();
    let capture = fixture.capture(&worktree);

    // This simulates a concurrent builder write after the temporary index has
    // become a Git tree object but before a caller consumes the receipt.
    fs::write(worktree.path().join("answer.txt"), "late mutable rewrite\n").unwrap();
    let materialized = fixture.materialize(&qualification, &capture, "delivery-snapshot");
    let delivery = fixture
        .materializer
        .deliver(&qualification, &materialized)
        .unwrap();
    assert_eq!(delivery.delivered_tree(), capture.candidate_tree());
    assert_eq!(
        fs::read_to_string(fixture.repo.join("answer.txt")).unwrap(),
        "sealed snapshot\n"
    );
    assert_eq!(
        capture
            .changed_paths()
            .iter()
            .map(society_product::RepositoryPath::as_str)
            .collect::<Vec<_>>(),
        vec!["answer.txt"]
    );
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

#[test]
fn executable_repository_filter_config_is_rejected_before_product_operations() {
    let fixture = Fixture::new();
    fixture.git_ok(["config", "filter.hostile.clean", "/bin/false"]);
    let error = fixture
        .materializer
        .qualify_clean_source(&fixture.repository, fixture.target_ref())
        .unwrap_err();
    assert!(matches!(
        error,
        ProductError::RepositoryExecutableConfigDenied { key } if key == "filter.hostile.clean"
    ));
}

#[test]
fn fsmonitor_and_diff_driver_config_are_neutralized_for_all_product_git_calls() {
    let fixture = Fixture::new();
    fixture.git_ok(["config", "core.fsmonitor", "/bin/false"]);
    fixture.git_ok(["config", "diff.external", "/bin/false"]);
    fixture.git_ok(["config", "diff.answer.textconv", "/bin/false"]);
    fs::write(
        fixture.repo.join(".gitattributes"),
        "answer.txt diff=answer\n",
    )
    .unwrap();
    fixture.git_ok(["add", ".gitattributes"]);
    fixture.git_ok([
        "commit",
        "-m",
        "configure hostile but neutralized diff paths",
    ]);

    let qualification = fixture.qualification();
    let worktree =
        fixture.create_builder(&qualification, "change-neutralized", "attempt-neutralized");
    fs::write(
        worktree.path().join("answer.txt"),
        "still captured without a driver\n",
    )
    .unwrap();
    let capture = fixture.capture(&worktree);
    assert_eq!(capture.changed_paths().len(), 1);
    let materialized = fixture.materialize(&qualification, &capture, "delivery-neutralized");
    assert_eq!(materialized.materialized().tree(), capture.candidate_tree());
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

#[cfg(unix)]
#[test]
fn git_stdout_is_bounded_before_a_fake_git_can_exhaust_process_memory() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    let fake_git = fixture.root.path().join("overflowing-git");
    fs::write(
        &fake_git,
        "#!/bin/sh\n/bin/dd if=/dev/zero bs=1048576 count=33\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&fake_git).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_git, permissions).unwrap();
    let overflowing = ProductMaterializer::new(&fake_git).unwrap();
    let error = overflowing.open_repository(&fixture.repo).unwrap_err();
    assert!(matches!(
        error,
        ProductError::GitOutputLimitExceeded {
            stream: "stdout",
            ..
        }
    ));
}

#[test]
fn structural_externally_supervised_receipt_verification_preserves_application_and_decision_cross_links_through_delivery()
 {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.repo.join("artifacts")).unwrap();
    fs::write(fixture.repo.join("artifacts/unit.bin"), b"\0\x01\x02\x03").unwrap();
    fixture.git_ok(["add", "artifacts/unit.bin"]);
    fixture.git_ok(["commit", "-m", "add generic artifact"]);
    let qualification = fixture.qualification();
    let worktree = fixture.create_builder(&qualification, "artifact-change", "artifact-builder-7");
    fs::write(
        worktree.path().join("artifacts/unit.bin"),
        b"\0\x01\x02\x04",
    )
    .unwrap();
    let capture = fixture.capture(&worktree);
    let invocation = ValidationProgramInvocation::new(
        AssignedValidationProgram::open("/bin/echo").unwrap(),
        vec![ValidationProgramArgument::parse("--verify-artifact").unwrap()],
    );
    let profile = ValidationProfile::new(
        ValidationProfileId::parse("artifact-validation-v1").unwrap(),
        vec![
            ValidationCommand::GitDiffCheck,
            ValidationCommand::ExternallySupervisedProgram(invocation.clone()),
        ],
    )
    .unwrap();
    let application_revision = ApplicationRevisionId::parse("revision-17").unwrap();
    let authorizing_decision = AuthorizingDecisionId::parse("decision-42").unwrap();
    let authorization_input = fixture.authorization_input_for_profile(
        &qualification,
        &capture,
        "delivery-artifact",
        application_revision.clone(),
        authorizing_decision.clone(),
        profile.id().clone(),
    );

    let prepared = fixture
        .materializer
        .prepare_materialization(
            authorization_input.clone(),
            &capture,
            &profile,
            &fixture.worktrees,
        )
        .unwrap();
    assert_eq!(
        prepared.authorization_input().application_revision(),
        &application_revision
    );
    assert_eq!(
        prepared.authorization_input().authorizing_decision(),
        &authorizing_decision
    );
    let receipt = ExternallySupervisedValidationReceipt::new(
        profile.id().clone(),
        prepared.tree().clone(),
        vec![ExternallySupervisedValidationStepReceipt::new(
            invocation.clone(),
            OutputDigest::parse("0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap(),
            OutputDigest::parse("1111111111111111111111111111111111111111111111111111111111111111")
                .unwrap(),
        )],
    )
    .unwrap();
    let materialized = fixture
        .materializer
        .finalize_materialization(prepared, &profile, &receipt, &fixture.commit_spec())
        .unwrap();
    assert_eq!(
        materialized.commit_validated().validation().steps().len(),
        2
    );
    assert!(matches!(
        materialized.commit_validated().validation().steps()[1].command(),
        ValidationCommand::ExternallySupervisedProgram(observed) if observed == &invocation
    ));
    assert_eq!(
        materialized.authorization_input().application_revision(),
        &application_revision
    );
    assert_eq!(
        materialized.authorization_input().authorizing_decision(),
        &authorizing_decision
    );
    assert_eq!(
        materialized.materialized().application_revision(),
        &application_revision
    );
    assert_eq!(
        materialized.materialized().authorizing_decision(),
        &authorizing_decision
    );
    assert_eq!(
        materialized.commit_validated().application_revision(),
        &application_revision
    );
    assert_eq!(
        materialized.commit_validated().authorizing_decision(),
        &authorizing_decision
    );
    assert_eq!(
        materialized
            .commit_validated()
            .validation()
            .application_revision(),
        &application_revision
    );
    assert_eq!(
        materialized
            .commit_validated()
            .validation()
            .authorizing_decision(),
        &authorizing_decision
    );
    assert_eq!(
        materialized
            .commit_validated()
            .commit()
            .application_revision(),
        &application_revision
    );
    assert_eq!(
        materialized
            .commit_validated()
            .commit()
            .authorizing_decision(),
        &authorizing_decision
    );

    let prepared = fixture
        .materializer
        .prepare_materialization(authorization_input, &capture, &profile, &fixture.worktrees)
        .unwrap();
    let wrong_tree = ExternallySupervisedValidationReceipt::new(
        profile.id().clone(),
        capture.base_tree().clone(),
        receipt.steps().to_vec(),
    )
    .unwrap();
    let error = fixture
        .materializer
        .finalize_materialization(prepared, &profile, &wrong_tree, &fixture.commit_spec())
        .unwrap_err();
    assert!(matches!(
        error,
        ProductError::MaterializationFailed { source, .. }
            if matches!(*source, ProductError::ExternallySupervisedValidationReceiptMismatch)
    ));

    let delivery = fixture
        .materializer
        .deliver(&qualification, &materialized)
        .unwrap();
    assert_eq!(delivery.disposition(), DeliveryDisposition::FastForwarded);
    assert_eq!(delivery.delivered_tree(), capture.candidate_tree());
    assert_eq!(delivery.application_revision(), &application_revision);
    assert_eq!(delivery.authorizing_decision(), &authorizing_decision);
    assert_eq!(
        fs::read(fixture.repo.join("artifacts/unit.bin")).unwrap(),
        b"\0\x01\x02\x04"
    );
    fixture
        .materializer
        .retire_product_worktree(worktree)
        .unwrap();
}

struct Fixture {
    root: TemporaryDirectory,
    repo: PathBuf,
    materializer: ProductMaterializer,
    repository: society_product::SourceRepository,
    worktrees: WorktreeRoot,
    artifacts: PatchArtifactRoot,
}

impl Fixture {
    fn new() -> Self {
        let root = TemporaryDirectory::new("guarded-materialization");
        let repo = root.path().join("source");
        let worktrees_path = root.path().join("isolated-worktrees");
        let artifacts_path = root.path().join("patch-artifacts");
        fs::create_dir_all(&repo).unwrap();
        fs::create_dir_all(&worktrees_path).unwrap();
        fs::create_dir_all(&artifacts_path).unwrap();
        git_ok_at(&repo, ["init", "-b", "main"]);
        git_ok_at(&repo, ["config", "user.name", "Fixture Author"]);
        git_ok_at(&repo, ["config", "user.email", "fixture@example.test"]);
        fs::write(repo.join("answer.txt"), "baseline\n").unwrap();
        git_ok_at(&repo, ["add", "answer.txt"]);
        git_ok_at(&repo, ["commit", "-m", "baseline"]);

        let materializer = ProductMaterializer::new(GIT).unwrap();
        let repository = materializer.open_repository(&repo).unwrap();
        Self {
            root,
            repo,
            materializer,
            repository,
            worktrees: WorktreeRoot::open(worktrees_path).unwrap(),
            artifacts: PatchArtifactRoot::open(artifacts_path).unwrap(),
        }
    }

    fn target_ref(&self) -> LocalBranchRef {
        LocalBranchRef::parse("refs/heads/main").unwrap()
    }

    fn worktrees_path(&self) -> PathBuf {
        self.root.path().join("isolated-worktrees")
    }

    fn qualification(&self) -> society_product::CleanSourceQualification {
        self.materializer
            .qualify_clean_source(&self.repository, self.target_ref())
            .unwrap()
    }

    fn create_builder(
        &self,
        qualification: &society_product::CleanSourceQualification,
        change: &str,
        attempt: &str,
    ) -> society_product::ProductWorktree {
        self.materializer
            .create_product_worktree(
                qualification,
                ProductChangeId::parse(change).unwrap(),
                BuilderAttemptId::parse(attempt).unwrap(),
                &self.worktrees,
            )
            .unwrap()
    }

    fn capture(
        &self,
        worktree: &society_product::ProductWorktree,
    ) -> society_product::CandidateCaptureReceipt {
        self.materializer
            .capture_candidate(worktree, &self.artifacts)
            .unwrap()
    }

    fn validation_profile(&self) -> ValidationProfile {
        ValidationProfile::new(
            ValidationProfileId::parse("git-diff-check-v1").unwrap(),
            vec![ValidationCommand::GitDiffCheck],
        )
        .unwrap()
    }

    fn authorization_input(
        &self,
        qualification: &society_product::CleanSourceQualification,
        capture: &society_product::CandidateCaptureReceipt,
        delivery_id: &str,
    ) -> ProductChangeAuthorizationInput {
        self.authorization_input_for_profile(
            qualification,
            capture,
            delivery_id,
            ApplicationRevisionId::parse(format!("{delivery_id}-revision")).unwrap(),
            AuthorizingDecisionId::parse(format!("{delivery_id}-decision")).unwrap(),
            ValidationProfileId::parse("git-diff-check-v1").unwrap(),
        )
    }

    fn authorization_input_for_profile(
        &self,
        qualification: &society_product::CleanSourceQualification,
        capture: &society_product::CandidateCaptureReceipt,
        delivery_id: &str,
        application_revision: ApplicationRevisionId,
        authorizing_decision: AuthorizingDecisionId,
        validation_profile: ValidationProfileId,
    ) -> ProductChangeAuthorizationInput {
        ProductChangeAuthorizationInput::new(
            DeliveryAuthorizationId::parse(delivery_id).unwrap(),
            application_revision,
            authorizing_decision,
            capture.change().clone(),
            capture.repository().clone(),
            qualification.target_ref().clone(),
            qualification.admitted_base().clone(),
            capture.patch().digest().clone(),
            capture.candidate_tree().clone(),
            validation_profile,
        )
    }

    fn commit_spec(&self) -> ControlledCommitSpec {
        ControlledCommitSpec {
            author: CommitIdentity::new("Guarded Materializer", "materializer@example.test")
                .unwrap(),
            author_time: CommitTimestamp::new(1_700_000_000, 0).unwrap(),
            committer: CommitIdentity::new("Guarded Materializer", "materializer@example.test")
                .unwrap(),
            committer_time: CommitTimestamp::new(1_700_000_000, 0).unwrap(),
            message: CommitMessage::new("materializer: controlled candidate materialization\n")
                .unwrap(),
        }
    }

    fn materialize(
        &self,
        qualification: &society_product::CleanSourceQualification,
        capture: &society_product::CandidateCaptureReceipt,
        delivery_id: &str,
    ) -> society_product::MaterializationReceipt {
        self.materializer
            .materialize(
                self.authorization_input(qualification, capture, delivery_id),
                capture,
                &self.validation_profile(),
                &self.commit_spec(),
                &self.worktrees,
            )
            .unwrap()
    }

    fn head(&self) -> society_product::CommitId {
        let qualification = self
            .materializer
            .qualify_clean_source(&self.repository, self.target_ref())
            .unwrap();
        qualification.admitted_base().clone()
    }

    fn head_tree(&self) -> society_product::TreeId {
        let qualification = self
            .materializer
            .qualify_clean_source(&self.repository, self.target_ref())
            .unwrap();
        qualification.admitted_base_tree().clone()
    }

    fn git_ok<const N: usize>(&self, arguments: [&str; N]) {
        git_ok_at(&self.repo, arguments);
    }

    fn install_poison_pre_commit_hook(&self, marker: &Path) {
        let hooks = self.repo.join(".git").join("hooks");
        fs::create_dir_all(&hooks).unwrap();
        let hook = hooks.join("pre-commit");
        fs::write(
            &hook,
            format!(
                "#!/bin/sh\nprintf hook-ran > {}\nexit 97\n",
                marker.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&hook).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(hook, permissions).unwrap();
        }
    }

    #[cfg(unix)]
    fn read_tree_failure_git(&self) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let wrapper = self.root.path().join("git-fail-read-tree");
        fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\nfor argument in \"$@\"; do\n  if [ \"$argument\" = read-tree ]; then\n    exit 71\n  fi\ndone\nexec {GIT} \"$@\"\n"
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(&wrapper).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&wrapper, permissions).unwrap();
        wrapper
    }

    #[cfg(unix)]
    fn worktree_remove_success_git(&self) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let wrapper = self.root.path().join("git-pretend-worktree-remove");
        fs::write(
            &wrapper,
            format!(
                "#!/bin/sh\nseen_worktree=0\nfor argument in \"$@\"; do\n  if [ \"$argument\" = worktree ]; then\n    seen_worktree=1\n  elif [ \"$seen_worktree\" = 1 ] && [ \"$argument\" = remove ]; then\n    exit 0\n  fi\ndone\nexec {GIT} \"$@\"\n"
            ),
        )
        .unwrap();
        let mut permissions = fs::metadata(&wrapper).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&wrapper, permissions).unwrap();
        wrapper
    }
}

fn git_ok_at<const N: usize>(repository: &Path, arguments: [&str; N]) {
    let output = git_at(repository, arguments);
    assert!(
        output.status.success(),
        "git {:?} failed: stdout={} stderr={}",
        arguments,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_at<const N: usize>(repository: &Path, arguments: [&str; N]) -> Output {
    Command::new(GIT)
        .arg("-C")
        .arg(repository)
        .args(arguments)
        .env_clear()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap()
}

fn git_revision(repository: &Path, revision: &str) -> String {
    let output = git_at(repository, ["rev-parse", "--verify", revision]);
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new(prefix: &str) -> Self {
        let process = std::process::id();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        for _ in 0..128 {
            let serial = TEMPORARY_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("{prefix}-{process}-{timestamp}-{serial}"));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create temporary test directory {path:?}: {error}"),
            }
        }
        panic!("could not allocate an isolated temporary test directory")
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
