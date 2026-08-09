//! Guarded, provider-free Git materialization for a single authorized product
//! change.
//!
//! This crate deliberately has no durable workflow, network, or remote
//! authority. Its caller supplies already-authorized identities and persists
//! the returned receipts. The only side effects here are narrowly-scoped local
//! Git worktrees, patch artifacts, and a guarded local branch fast-forward.
//!
//! The boundary is intentionally shell-free.  Every Git operation is an argv
//! invocation of a configured absolute Git executable. External validation
//! programs are declared here but executed only by a separately supervised
//! owner; this core verifies their typed receipts against the exact prepared
//! tree and never spawns an arbitrary validator.

use std::{
    ffi::{OsStr, OsString},
    fmt,
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};
use thiserror::Error;

const MAX_IDENTITY_BYTES: usize = 128;
const MAX_DERIVED_NAME_BYTES: usize = MAX_IDENTITY_BYTES * 2 + 64;
const MAX_MESSAGE_BYTES: usize = 65_536;
const MAX_VALIDATION_STEPS: usize = 32;
const MAX_PORTABLE_PATCH_BYTES: usize = 32 * 1024 * 1024;
const MAX_GIT_STDOUT_BYTES: usize = MAX_PORTABLE_PATCH_BYTES;
const MAX_GIT_STDERR_BYTES: usize = 1024 * 1024;
const GIT_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

/// A closed product-change identity supplied by the future durable authority.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProductChangeId(String);

/// A closed builder-attempt identity supplied by the future durable authority.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BuilderAttemptId(String);

/// A closed delivery-authorization identity supplied by the caller.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeliveryAuthorizationId(String);

/// A closed identity for one requested validation profile.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ValidationProfileId(String);

macro_rules! closed_identity {
    ($type_name:ident, $description:literal) => {
        impl $type_name {
            pub fn parse(value: impl Into<String>) -> Result<Self, ProductError> {
                let value = value.into();
                if !is_closed_identity(&value) {
                    return Err(ProductError::InvalidIdentity { kind: $description });
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $type_name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

closed_identity!(ProductChangeId, "product change identity");
closed_identity!(BuilderAttemptId, "builder attempt identity");
closed_identity!(DeliveryAuthorizationId, "delivery authorization identity");
closed_identity!(ValidationProfileId, "validation profile identity");

/// The object format asserted by an opened repository.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitObjectFormat {
    Sha1,
    Sha256,
}

impl GitObjectFormat {
    fn parse(value: &str) -> Result<Self, ProductError> {
        match value {
            "sha1" => Ok(Self::Sha1),
            "sha256" => Ok(Self::Sha256),
            _ => Err(ProductError::UnsupportedObjectFormat(value.to_owned())),
        }
    }

    fn hex_bytes(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
        }
    }
}

/// Exact Git object bytes, retained with their repository object format.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitObjectId {
    Sha1([u8; 20]),
    Sha256([u8; 32]),
}

impl GitObjectId {
    fn parse(value: &str, format: GitObjectFormat) -> Result<Self, ProductError> {
        if value.len() != format.hex_bytes() * 2
            || !value.as_bytes().iter().all(u8::is_ascii_hexdigit)
        {
            return Err(ProductError::InvalidGitObjectId(value.to_owned()));
        }
        let mut decoded = Vec::with_capacity(format.hex_bytes());
        for pair in value.as_bytes().as_chunks::<2>().0 {
            let high = hex_nibble(pair[0])
                .ok_or_else(|| ProductError::InvalidGitObjectId(value.to_owned()))?;
            let low = hex_nibble(pair[1])
                .ok_or_else(|| ProductError::InvalidGitObjectId(value.to_owned()))?;
            decoded.push((high << 4) | low);
        }
        match format {
            GitObjectFormat::Sha1 => {
                let bytes: [u8; 20] = decoded
                    .try_into()
                    .map_err(|_| ProductError::InvalidGitObjectId(value.to_owned()))?;
                Ok(Self::Sha1(bytes))
            }
            GitObjectFormat::Sha256 => {
                let bytes: [u8; 32] = decoded
                    .try_into()
                    .map_err(|_| ProductError::InvalidGitObjectId(value.to_owned()))?;
                Ok(Self::Sha256(bytes))
            }
        }
    }

    fn format(&self) -> GitObjectFormat {
        match self {
            Self::Sha1(_) => GitObjectFormat::Sha1,
            Self::Sha256(_) => GitObjectFormat::Sha256,
        }
    }

    pub fn to_hex(&self) -> String {
        match self {
            Self::Sha1(bytes) => hex(bytes),
            Self::Sha256(bytes) => hex(bytes),
        }
    }
}

impl fmt::Display for GitObjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

/// A Git commit identity.  It is not a mutable branch name.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CommitId(GitObjectId);

impl CommitId {
    fn parse(value: &str, format: GitObjectFormat) -> Result<Self, ProductError> {
        Ok(Self(GitObjectId::parse(value, format)?))
    }

    pub fn object_format(&self) -> GitObjectFormat {
        self.0.format()
    }

    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }
}

impl fmt::Display for CommitId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A Git tree identity.  It identifies the exact materialized file tree.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TreeId(GitObjectId);

impl TreeId {
    fn parse(value: &str, format: GitObjectFormat) -> Result<Self, ProductError> {
        Ok(Self(GitObjectId::parse(value, format)?))
    }

    pub fn object_format(&self) -> GitObjectFormat {
        self.0.format()
    }

    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }
}

impl fmt::Display for TreeId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

macro_rules! sha256_identity {
    ($type_name:ident, $description:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $type_name([u8; 32]);

        impl $type_name {
            fn of_bytes(bytes: &[u8]) -> Self {
                Self(Sha256::digest(bytes).into())
            }

            pub fn to_hex(&self) -> String {
                hex(&self.0)
            }
        }

        impl fmt::Display for $type_name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}", self.to_hex())
            }
        }

        impl $type_name {
            /// The digest names exact bytes, never an inferred semantic proxy.
            pub const DESCRIPTION: &'static str = $description;
        }
    };
}

sha256_identity!(PatchDigest, "portable patch bytes");
sha256_identity!(ValidationDigest, "closed validation receipt");
sha256_identity!(OutputDigest, "process output bytes");
sha256_identity!(CommitMessageDigest, "controlled commit message bytes");

impl OutputDigest {
    /// Admit a digest emitted by an owned external supervisor. The caller does
    /// not thereby gain a way to provide arbitrary process output to this core.
    pub fn parse(value: &str) -> Result<Self, ProductError> {
        let bytes =
            decode_exact_sha256(value).ok_or_else(|| ProductError::InvalidOutputDigest {
                value: value.to_owned(),
            })?;
        Ok(Self(bytes))
    }
}

/// A verified local, non-bare repository worktree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRepository {
    worktree_root: PathBuf,
    git_common_dir: PathBuf,
    object_format: GitObjectFormat,
}

impl SourceRepository {
    pub fn worktree_root(&self) -> &Path {
        &self.worktree_root
    }

    pub fn git_common_dir(&self) -> &Path {
        &self.git_common_dir
    }

    pub fn object_format(&self) -> GitObjectFormat {
        self.object_format
    }
}

/// A local branch ref that may be guarded and fast-forwarded.  Tags, remotes,
/// detached HEADs, and arbitrary revisions are not delivery targets.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalBranchRef(String);

impl LocalBranchRef {
    pub fn parse(value: impl Into<String>) -> Result<Self, ProductError> {
        let value = value.into();
        let Some(short) = value.strip_prefix("refs/heads/") else {
            return Err(ProductError::InvalidLocalBranchRef(value));
        };
        if !is_valid_ref_component_path(short) {
            return Err(ProductError::InvalidLocalBranchRef(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LocalBranchRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A repository-relative UTF-8 path captured from Git's NUL-delimited output.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RepositoryPath(String);

impl RepositoryPath {
    fn from_git_output(bytes: &[u8]) -> Result<Self, ProductError> {
        let value = std::str::from_utf8(bytes).map_err(|_| ProductError::NonUtf8GitPath)?;
        if value.is_empty()
            || Path::new(value).is_absolute()
            || Path::new(value).components().any(|part| {
                matches!(
                    part,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(ProductError::InvalidRepositoryPath(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RepositoryPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A caller-owned root for all temporary Git worktrees created by this crate.
/// It must already exist and be outside the source checkout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeRoot(PathBuf);

impl WorktreeRoot {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProductError> {
        let path = fs::canonicalize(path.as_ref()).map_err(|source| ProductError::Io {
            operation: "canonicalizing worktree root",
            path: path.as_ref().to_path_buf(),
            source,
        })?;
        let metadata = fs::metadata(&path).map_err(|source| ProductError::Io {
            operation: "reading worktree-root metadata",
            path: path.clone(),
            source,
        })?;
        if !metadata.is_dir() {
            return Err(ProductError::NotDirectory(path));
        }
        Ok(Self(path))
    }

    fn allocate(&self, name: &WorktreeName) -> ManagedWorktreePath {
        ManagedWorktreePath {
            root: self.0.clone(),
            path: self.0.join(name.as_str()),
            name: name.clone(),
        }
    }
}

/// A caller-owned root for exact portable patch artifacts.  It is deliberately
/// separate from every Git worktree, so recording a patch never makes an
/// otherwise clean candidate untracked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatchArtifactRoot(PathBuf);

impl PatchArtifactRoot {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProductError> {
        let path = fs::canonicalize(path.as_ref()).map_err(|source| ProductError::Io {
            operation: "canonicalizing patch artifact root",
            path: path.as_ref().to_path_buf(),
            source,
        })?;
        let metadata = fs::metadata(&path).map_err(|source| ProductError::Io {
            operation: "reading patch-artifact-root metadata",
            path: path.clone(),
            source,
        })?;
        if !metadata.is_dir() {
            return Err(ProductError::NotDirectory(path));
        }
        Ok(Self(path))
    }

    fn allocate(&self, name: &PatchArtifactName) -> PathBuf {
        self.0.join(name.as_str())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct WorktreeName(String);

impl WorktreeName {
    fn parse(value: String) -> Result<Self, ProductError> {
        if !is_derived_name(&value) {
            return Err(ProductError::InvalidIdentity {
                kind: "worktree identity",
            });
        }
        Ok(Self(value))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct PatchArtifactName(String);

impl PatchArtifactName {
    fn for_capture(
        change: &ProductChangeId,
        attempt: &BuilderAttemptId,
    ) -> Result<Self, ProductError> {
        Self::parse(format!("patch-{}.diff", derived_pair_name(change, attempt)))
    }

    fn parse(value: String) -> Result<Self, ProductError> {
        if value.len() > MAX_DERIVED_NAME_BYTES + 16
            || !value.ends_with(".diff")
            || !is_derived_name(&value[..value.len() - ".diff".len()])
        {
            return Err(ProductError::InvalidIdentity {
                kind: "patch artifact identity",
            });
        }
        Ok(Self(value))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ManagedWorktreePath {
    root: PathBuf,
    path: PathBuf,
    name: WorktreeName,
}

impl ManagedWorktreePath {
    fn assert_absent(&self) -> Result<(), ProductError> {
        if directory_entry_exists_no_follow(&self.path, "checking managed worktree absence")? {
            return Err(ProductError::ManagedPathAlreadyExists(self.path.clone()));
        }
        Ok(())
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn assert_owned(&self) -> Result<(), ProductError> {
        if self.path.parent() != Some(self.root.as_path()) || self.path.file_name().is_none() {
            return Err(ProductError::UnmanagedWorktreePath(self.path.clone()));
        }
        Ok(())
    }
}

/// The deterministic branch name for a builder-owned `ProductWorktree`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProductWorktreeBranch(String);

impl ProductWorktreeBranch {
    pub fn derive(
        change: &ProductChangeId,
        attempt: &BuilderAttemptId,
    ) -> Result<Self, ProductError> {
        let value = format!(
            "guarded-materialization/{}",
            derived_pair_name(change, attempt)
        );
        if !is_valid_ref_component_path_with_limit(&value, MAX_DERIVED_NAME_BYTES + 16) {
            return Err(ProductError::InvalidProductWorktreeBranch(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The allowed state vocabulary for this local materialization boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProductState {
    WorktreeReady,
    CandidateSubmitted,
    Materialized,
    CommitValidated,
    DeliveryReady,
    Delivered,
    Reopened,
}

/// A clean source/target qualification at one exact local branch head.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanSourceQualification {
    repository: SourceRepository,
    target_ref: LocalBranchRef,
    admitted_base: CommitId,
    admitted_base_tree: TreeId,
}

impl CleanSourceQualification {
    pub fn repository(&self) -> &SourceRepository {
        &self.repository
    }

    pub fn target_ref(&self) -> &LocalBranchRef {
        &self.target_ref
    }

    pub fn admitted_base(&self) -> &CommitId {
        &self.admitted_base
    }

    pub fn admitted_base_tree(&self) -> &TreeId {
        &self.admitted_base_tree
    }
}

/// One builder-owned branch worktree.  It intentionally cannot make an
/// authoritative commit: this type only captures an uncommitted candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductWorktree {
    repository: SourceRepository,
    location: ManagedWorktreePath,
    state: ProductState,
    change: ProductChangeId,
    attempt: BuilderAttemptId,
    branch: ProductWorktreeBranch,
    base: CommitId,
    base_tree: TreeId,
}

impl ProductWorktree {
    pub fn path(&self) -> &Path {
        self.location.path()
    }

    pub fn state(&self) -> ProductState {
        self.state
    }

    pub fn change(&self) -> &ProductChangeId {
        &self.change
    }

    pub fn attempt(&self) -> &BuilderAttemptId {
        &self.attempt
    }

    pub fn branch(&self) -> &ProductWorktreeBranch {
        &self.branch
    }

    pub fn base(&self) -> &CommitId {
        &self.base
    }

    pub fn base_tree(&self) -> &TreeId {
        &self.base_tree
    }
}

/// Exact portable patch storage and identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortablePatch {
    path: PathBuf,
    digest: PatchDigest,
}

impl PortablePatch {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn digest(&self) -> &PatchDigest {
        &self.digest
    }
}

/// The candidate facts captured from a builder worktree.  Untracked files are
/// refused rather than silently omitted from the portable patch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateCaptureReceipt {
    state: ProductState,
    change: ProductChangeId,
    attempt: BuilderAttemptId,
    repository: SourceRepository,
    base: CommitId,
    base_tree: TreeId,
    candidate_tree: TreeId,
    patch: PortablePatch,
    changed_paths: Vec<RepositoryPath>,
}

impl CandidateCaptureReceipt {
    pub fn state(&self) -> ProductState {
        self.state
    }

    pub fn change(&self) -> &ProductChangeId {
        &self.change
    }

    pub fn attempt(&self) -> &BuilderAttemptId {
        &self.attempt
    }

    pub fn repository(&self) -> &SourceRepository {
        &self.repository
    }

    pub fn base(&self) -> &CommitId {
        &self.base
    }

    pub fn base_tree(&self) -> &TreeId {
        &self.base_tree
    }

    pub fn candidate_tree(&self) -> &TreeId {
        &self.candidate_tree
    }

    pub fn patch(&self) -> &PortablePatch {
        &self.patch
    }

    pub fn changed_paths(&self) -> &[RepositoryPath] {
        &self.changed_paths
    }
}

/// Closed validation requirements for one materialized tree. `GitDiffCheck`
/// is a bounded internal Git operation. An `ExternallySupervisedProgram`
/// names one exact validation-program invocation, but this crate deliberately
/// does not execute it: it never owns that program's process group or output
/// pipes, so it cannot safely supervise it itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationCommand {
    GitDiffCheck,
    ExternallySupervisedProgram(ValidationProgramInvocation),
}

impl ValidationCommand {
    const fn requires_external_supervision(&self) -> bool {
        matches!(self, Self::ExternallySupervisedProgram(_))
    }
}

/// A closed, named list of required validation operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationProfile {
    id: ValidationProfileId,
    commands: Vec<ValidationCommand>,
}

impl ValidationProfile {
    pub fn new(
        id: ValidationProfileId,
        commands: Vec<ValidationCommand>,
    ) -> Result<Self, ProductError> {
        if commands.is_empty() || commands.len() > MAX_VALIDATION_STEPS {
            return Err(ProductError::InvalidValidationProfile);
        }
        Ok(Self { id, commands })
    }

    pub fn id(&self) -> &ValidationProfileId {
        &self.id
    }

    pub fn commands(&self) -> &[ValidationCommand] {
        &self.commands
    }
}

/// The exact canonical absolute executable assigned to one external validation
/// program. It cannot be a shell interpreter.
///
/// Equality is the canonical path identity established by `open`, so an
/// external receipt cannot substitute a different executable behind an alias.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignedValidationProgram(PathBuf);

impl AssignedValidationProgram {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProductError> {
        let supplied = path.as_ref();
        if !supplied.is_absolute() {
            return Err(ProductError::ProgramPathNotAbsolute(supplied.to_path_buf()));
        }
        // Canonicalizing before the shell-name check closes the easy symlink
        // bypass (`/tmp/validator` -> `/bin/sh`) without pretending this
        // boundary can attest an arbitrary executable's implementation.
        let path = fs::canonicalize(supplied).map_err(|source| ProductError::Io {
            operation: "canonicalizing assigned validation program",
            path: supplied.to_path_buf(),
            source,
        })?;
        let metadata = fs::metadata(&path).map_err(|source| ProductError::Io {
            operation: "reading assigned validation-program metadata",
            path: path.to_path_buf(),
            source,
        })?;
        if !metadata.is_file() {
            return Err(ProductError::ProgramNotRegular(path.to_path_buf()));
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if matches!(
            name,
            "sh" | "bash" | "zsh" | "fish" | "dash" | "cmd" | "powershell"
        ) {
            return Err(ProductError::ShellValidationProgramDenied(
                path.to_path_buf(),
            ));
        }
        Ok(Self(path))
    }

    /// The canonical absolute path whose identity is bound into an invocation
    /// and its validation receipt.
    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// One exact argv element for an assigned external validation program.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ValidationProgramArgument(String);

impl ValidationProgramArgument {
    pub fn parse(value: impl Into<String>) -> Result<Self, ProductError> {
        let value = value.into();
        if value.contains('\0') || value.len() > MAX_MESSAGE_BYTES {
            return Err(ProductError::InvalidValidationProgramArgument);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An exact argv-only invocation of an assigned external validation program.
/// The program identity and every argument have narrow types; callers cannot
/// smuggle a shell command, argument map, or generic payload through a
/// validation profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationProgramInvocation {
    program: AssignedValidationProgram,
    arguments: Vec<ValidationProgramArgument>,
}

impl ValidationProgramInvocation {
    pub fn new(
        program: AssignedValidationProgram,
        arguments: Vec<ValidationProgramArgument>,
    ) -> Self {
        Self { program, arguments }
    }

    pub fn program(&self) -> &AssignedValidationProgram {
        &self.program
    }

    pub fn arguments(&self) -> &[ValidationProgramArgument] {
        &self.arguments
    }
}

/// A digest-only execution result.  Output is evidence for the future content
/// store, not an opaque workflow field in this core.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationStepReceipt {
    command: ValidationCommand,
    stdout: OutputDigest,
    stderr: OutputDigest,
}

impl ValidationStepReceipt {
    pub fn command(&self) -> &ValidationCommand {
        &self.command
    }

    pub fn stdout(&self) -> &OutputDigest {
        &self.stdout
    }

    pub fn stderr(&self) -> &OutputDigest {
        &self.stderr
    }
}

/// A typed externally supervised validation-program step over the prepared
/// worktree. Its output digests are supplied by the supervisor's owned process
/// boundary; constructing this receipt does not attest that the program was
/// actually run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternallySupervisedValidationStepReceipt {
    invocation: ValidationProgramInvocation,
    stdout: OutputDigest,
    stderr: OutputDigest,
}

impl ExternallySupervisedValidationStepReceipt {
    pub fn new(
        invocation: ValidationProgramInvocation,
        stdout: OutputDigest,
        stderr: OutputDigest,
    ) -> Self {
        Self {
            invocation,
            stdout,
            stderr,
        }
    }

    pub fn invocation(&self) -> &ValidationProgramInvocation {
        &self.invocation
    }

    pub fn stdout(&self) -> &OutputDigest {
        &self.stdout
    }

    pub fn stderr(&self) -> &OutputDigest {
        &self.stderr
    }
}

/// The externally supervised portion of a profile. `profile` and `tree` bind
/// the receipt to one prepared materialization, while `steps` must exactly
/// reconstruct the profile's non-Git requirements in order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternallySupervisedValidationReceipt {
    profile: ValidationProfileId,
    tree: TreeId,
    steps: Vec<ExternallySupervisedValidationStepReceipt>,
}

impl ExternallySupervisedValidationReceipt {
    pub fn new(
        profile: ValidationProfileId,
        tree: TreeId,
        steps: Vec<ExternallySupervisedValidationStepReceipt>,
    ) -> Result<Self, ProductError> {
        if steps.len() > MAX_VALIDATION_STEPS {
            return Err(ProductError::InvalidExternallySupervisedValidationReceipt);
        }
        Ok(Self {
            profile,
            tree,
            steps,
        })
    }

    pub fn profile(&self) -> &ValidationProfileId {
        &self.profile
    }

    pub fn tree(&self) -> &TreeId {
        &self.tree
    }

    pub fn steps(&self) -> &[ExternallySupervisedValidationStepReceipt] {
        &self.steps
    }
}

/// A successful closed validation run over one exact tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationReceipt {
    profile: ValidationProfileId,
    tree: TreeId,
    steps: Vec<ValidationStepReceipt>,
    digest: ValidationDigest,
}

impl ValidationReceipt {
    pub fn profile(&self) -> &ValidationProfileId {
        &self.profile
    }

    pub fn tree(&self) -> &TreeId {
        &self.tree
    }

    pub fn steps(&self) -> &[ValidationStepReceipt] {
        &self.steps
    }

    pub fn digest(&self) -> &ValidationDigest {
        &self.digest
    }
}

/// A failed validation remains distinct from patch/application failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationFailureReceipt {
    profile: ValidationProfileId,
    tree: TreeId,
    completed_steps: Vec<ValidationStepReceipt>,
    failed_command: ValidationCommand,
    stdout: OutputDigest,
    stderr: OutputDigest,
}

impl ValidationFailureReceipt {
    pub fn profile(&self) -> &ValidationProfileId {
        &self.profile
    }

    pub fn tree(&self) -> &TreeId {
        &self.tree
    }

    pub fn completed_steps(&self) -> &[ValidationStepReceipt] {
        &self.completed_steps
    }

    pub fn failed_command(&self) -> &ValidationCommand {
        &self.failed_command
    }

    pub fn stdout(&self) -> &OutputDigest {
        &self.stdout
    }

    pub fn stderr(&self) -> &OutputDigest {
        &self.stderr
    }
}

/// Explicit commit identity and exact author/committer clock inputs.  There is
/// no ambient Git identity or wall-clock fallback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitIdentity {
    name: String,
    email: String,
}

impl CommitIdentity {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Result<Self, ProductError> {
        let name = name.into();
        let email = email.into();
        if name.is_empty()
            || email.is_empty()
            || name.contains(['\n', '\r', '\0'])
            || email.contains(['\n', '\r', '\0'])
            || !email.contains('@')
        {
            return Err(ProductError::InvalidCommitIdentity);
        }
        Ok(Self { name, email })
    }
}

/// A Git-compatible UTC offset in minutes.  The unix seconds and offset are
/// supplied by authorization; materialization never reads the current clock.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommitTimestamp {
    unix_seconds: i64,
    utc_offset_minutes: i16,
}

impl CommitTimestamp {
    pub fn new(unix_seconds: i64, utc_offset_minutes: i16) -> Result<Self, ProductError> {
        if !(-23 * 60 - 59..=23 * 60 + 59).contains(&utc_offset_minutes) {
            return Err(ProductError::InvalidCommitTimestamp);
        }
        Ok(Self {
            unix_seconds,
            utc_offset_minutes,
        })
    }

    fn git_value(self) -> String {
        let sign = if self.utc_offset_minutes < 0 {
            '-'
        } else {
            '+'
        };
        let minutes = self.utc_offset_minutes.unsigned_abs();
        format!(
            "{} {}{:02}{:02}",
            self.unix_seconds,
            sign,
            minutes / 60,
            minutes % 60
        )
    }
}

/// The exact message bytes used by plumbing-level commit construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitMessage(String);

impl CommitMessage {
    pub fn new(value: impl Into<String>) -> Result<Self, ProductError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_MESSAGE_BYTES
            || value.contains(['\0', '\r'])
            || !value.ends_with('\n')
        {
            return Err(ProductError::InvalidCommitMessage);
        }
        Ok(Self(value))
    }

    fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// Every controlled commit is made by Git plumbing, which never invokes
/// repository hooks.  The receipt records that policy rather than inferring it
/// from a hook marker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookPolicy {
    GitCommitTreeWithHooksDisabled,
}

/// Exact inputs for deterministic controlled commit construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlledCommitSpec {
    pub author: CommitIdentity,
    pub author_time: CommitTimestamp,
    pub committer: CommitIdentity,
    pub committer_time: CommitTimestamp,
    pub message: CommitMessage,
}

/// The product commit produced only after an exact-tree and validation pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlledCommitReceipt {
    commit: CommitId,
    parent: CommitId,
    tree: TreeId,
    message: CommitMessageDigest,
    hook_policy: HookPolicy,
}

impl ControlledCommitReceipt {
    pub fn commit(&self) -> &CommitId {
        &self.commit
    }

    pub fn parent(&self) -> &CommitId {
        &self.parent
    }

    pub fn tree(&self) -> &TreeId {
        &self.tree
    }

    pub fn message(&self) -> &CommitMessageDigest {
        &self.message
    }

    pub fn hook_policy(&self) -> HookPolicy {
        self.hook_policy
    }
}

/// Delivery inputs that this crate verifies but never authorizes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductDeliveryAuthorization {
    id: DeliveryAuthorizationId,
    change: ProductChangeId,
    /// The exact opened repository identity. A receipt from another local
    /// checkout cannot be delivered merely because it names a matching object.
    repository: SourceRepository,
    target_ref: LocalBranchRef,
    admitted_base: CommitId,
    accepted_patch: PatchDigest,
    accepted_tree: TreeId,
    validation_profile: ValidationProfileId,
}

impl ProductDeliveryAuthorization {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: DeliveryAuthorizationId,
        change: ProductChangeId,
        repository: SourceRepository,
        target_ref: LocalBranchRef,
        admitted_base: CommitId,
        accepted_patch: PatchDigest,
        accepted_tree: TreeId,
        validation_profile: ValidationProfileId,
    ) -> Self {
        Self {
            id,
            change,
            repository,
            target_ref,
            admitted_base,
            accepted_patch,
            accepted_tree,
            validation_profile,
        }
    }

    pub fn id(&self) -> &DeliveryAuthorizationId {
        &self.id
    }

    pub fn change(&self) -> &ProductChangeId {
        &self.change
    }

    pub fn repository(&self) -> &SourceRepository {
        &self.repository
    }

    pub fn target_ref(&self) -> &LocalBranchRef {
        &self.target_ref
    }

    pub fn admitted_base(&self) -> &CommitId {
        &self.admitted_base
    }

    pub fn accepted_patch(&self) -> &PatchDigest {
        &self.accepted_patch
    }

    pub fn accepted_tree(&self) -> &TreeId {
        &self.accepted_tree
    }

    pub fn validation_profile(&self) -> &ValidationProfileId {
        &self.validation_profile
    }
}

/// Successful materialization, deterministic commit construction, and cleanup
/// facts. This is not durable authority; the caller must persist it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializationReceipt {
    state: ProductState,
    authorization: ProductDeliveryAuthorization,
    materialized: MaterializedTreeReceipt,
    commit_validated: CommitValidatedReceipt,
    cleanup: CleanupEvidence,
}

impl MaterializationReceipt {
    pub fn state(&self) -> ProductState {
        self.state
    }

    pub fn authorization(&self) -> &ProductDeliveryAuthorization {
        &self.authorization
    }

    pub fn materialized(&self) -> &MaterializedTreeReceipt {
        &self.materialized
    }

    pub fn commit_validated(&self) -> &CommitValidatedReceipt {
        &self.commit_validated
    }

    pub fn cleanup(&self) -> &CleanupEvidence {
        &self.cleanup
    }
}

/// A fresh worktree whose index and working copy both materialize the accepted
/// tree. It is a single-use cleanup responsibility: pass it exactly once to
/// `finalize_materialization` or `abandon_prepared_materialization`.
///
/// Dropping this value intentionally performs no best-effort deletion, because
/// destruction cannot return durable `CleanupEvidence`. A supervisor that
/// abandons it must call the explicit abandon operation and persist the result.
/// An external validator may use only `path()` and must bind its result to
/// `tree()`; it receives no Git delivery authority.
#[must_use = "prepared worktrees require explicit finalize or abandon cleanup"]
#[derive(Debug, Eq, PartialEq)]
pub struct PreparedMaterialization {
    authorization: ProductDeliveryAuthorization,
    repository: SourceRepository,
    location: ManagedWorktreePath,
    materialized: MaterializedTreeReceipt,
}

impl PreparedMaterialization {
    pub fn path(&self) -> &Path {
        self.location.path()
    }

    pub fn tree(&self) -> &TreeId {
        &self.materialized.tree
    }

    pub fn validation_profile(&self) -> &ValidationProfileId {
        &self.authorization.validation_profile
    }
}

/// The exact-tree transition made immediately after applying an accepted patch
/// in a fresh worktree.  It intentionally precedes validation and commit
/// construction in the receipt graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterializedTreeReceipt {
    state: ProductState,
    base: CommitId,
    base_tree: TreeId,
    tree: TreeId,
    patch: PatchDigest,
}

impl MaterializedTreeReceipt {
    pub fn state(&self) -> ProductState {
        self.state
    }

    pub fn base(&self) -> &CommitId {
        &self.base
    }

    pub fn base_tree(&self) -> &TreeId {
        &self.base_tree
    }

    pub fn tree(&self) -> &TreeId {
        &self.tree
    }

    pub fn patch(&self) -> &PatchDigest {
        &self.patch
    }
}

/// The separately inspectable `commit_validated` transition.  It binds the
/// successful judge receipt to the controlled one-parent commit for the same
/// exact materialized tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitValidatedReceipt {
    state: ProductState,
    validation: ValidationReceipt,
    commit: ControlledCommitReceipt,
}

impl CommitValidatedReceipt {
    pub fn state(&self) -> ProductState {
        self.state
    }

    pub fn validation(&self) -> &ValidationReceipt {
        &self.validation
    }

    pub fn commit(&self) -> &ControlledCommitReceipt {
        &self.commit
    }
}

/// Worktree cleanup status.  A successful materialization cleans its fresh
/// worktree; a failure returns explicit retained/removed evidence in the error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CleanupEvidence {
    Removed { worktree_name: String },
    RetainedAfterFailure { worktree_name: String },
}

/// Receipt for a local guarded delivery.  `AlreadyDelivered` makes a retry
/// idempotent only when the named target already points at this exact commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeliveryReceipt {
    state: ProductState,
    authorization: DeliveryAuthorizationId,
    target_ref: LocalBranchRef,
    expected_old_head: CommitId,
    delivered_commit: CommitId,
    delivered_tree: TreeId,
    disposition: DeliveryDisposition,
}

impl DeliveryReceipt {
    pub fn state(&self) -> ProductState {
        self.state
    }

    pub fn authorization(&self) -> &DeliveryAuthorizationId {
        &self.authorization
    }

    pub fn target_ref(&self) -> &LocalBranchRef {
        &self.target_ref
    }

    pub fn expected_old_head(&self) -> &CommitId {
        &self.expected_old_head
    }

    pub fn delivered_commit(&self) -> &CommitId {
        &self.delivered_commit
    }

    pub fn delivered_tree(&self) -> &TreeId {
        &self.delivered_tree
    }

    pub fn disposition(&self) -> DeliveryDisposition {
        self.disposition
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeliveryDisposition {
    FastForwarded,
    AlreadyDelivered,
}

/// A typed reopen directive.  It deliberately does not rebase, mutate Git, or
/// reopen durable state: a descendant needs a new base qualification and full
/// materialization/validation by the future authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReopenDirective {
    state: ProductState,
    prior_delivery: DeliveryReceipt,
    descendant_change: ProductChangeId,
}

impl ReopenDirective {
    pub fn state(&self) -> ProductState {
        self.state
    }

    pub fn prior_delivery(&self) -> &DeliveryReceipt {
        &self.prior_delivery
    }

    pub fn descendant_change(&self) -> &ProductChangeId {
        &self.descendant_change
    }
}

impl DeliveryReceipt {
    pub fn reopen_as_descendant(&self, descendant_change: ProductChangeId) -> ReopenDirective {
        ReopenDirective {
            state: ProductState::Reopened,
            prior_delivery: self.clone(),
            descendant_change,
        }
    }
}

/// A configured local product materializer.  The caller must pass an absolute
/// Git executable so this core never consults ambient `PATH`.
#[derive(Clone, Debug)]
pub struct ProductMaterializer {
    git: PathBuf,
}

impl ProductMaterializer {
    pub fn new(git_executable: impl AsRef<Path>) -> Result<Self, ProductError> {
        let git = git_executable.as_ref();
        if !git.is_absolute() {
            return Err(ProductError::GitPathNotAbsolute(git.to_path_buf()));
        }
        let metadata = fs::metadata(git).map_err(|source| ProductError::Io {
            operation: "reading Git executable metadata",
            path: git.to_path_buf(),
            source,
        })?;
        if !metadata.is_file() {
            return Err(ProductError::GitNotRegular(git.to_path_buf()));
        }
        Ok(Self {
            git: git.to_path_buf(),
        })
    }

    /// Opens a local non-bare worktree and binds its Git object format.
    pub fn open_repository(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<SourceRepository, ProductError> {
        let supplied = path.as_ref();
        let worktree_root = fs::canonicalize(supplied).map_err(|source| ProductError::Io {
            operation: "canonicalizing source repository",
            path: supplied.to_path_buf(),
            source,
        })?;
        if !fs::metadata(&worktree_root)
            .map_err(|source| ProductError::Io {
                operation: "reading source repository metadata",
                path: worktree_root.clone(),
                source,
            })?
            .is_dir()
        {
            return Err(ProductError::NotDirectory(worktree_root));
        }
        let top_level = self.git_text(
            &worktree_root,
            "resolving repository top level",
            ["rev-parse", "--show-toplevel"],
        )?;
        let reported_root =
            fs::canonicalize(top_level.trim()).map_err(|source| ProductError::Io {
                operation: "canonicalizing Git repository top level",
                path: PathBuf::from(top_level.trim()),
                source,
            })?;
        if reported_root != worktree_root {
            return Err(ProductError::RepositoryPathIsNotWorktreeRoot {
                supplied: worktree_root,
                reported: reported_root,
            });
        }
        self.assert_repository_executable_config_safe(&reported_root)?;
        if self
            .git_text(
                &reported_root,
                "checking bare repository",
                ["rev-parse", "--is-bare-repository"],
            )?
            .trim()
            != "false"
        {
            return Err(ProductError::BareRepository(reported_root));
        }
        let object_format = GitObjectFormat::parse(
            self.git_text(
                &reported_root,
                "reading Git object format",
                ["rev-parse", "--show-object-format"],
            )?
            .trim(),
        )?;
        let common_raw = self.git_text(
            &reported_root,
            "resolving Git common directory",
            ["rev-parse", "--git-common-dir"],
        )?;
        let common_path = PathBuf::from(common_raw.trim());
        let git_common_dir = fs::canonicalize(if common_path.is_absolute() {
            common_path
        } else {
            reported_root.join(common_path)
        })
        .map_err(|source| ProductError::Io {
            operation: "canonicalizing Git common directory",
            path: PathBuf::from(common_raw.trim()),
            source,
        })?;
        Ok(SourceRepository {
            worktree_root: reported_root,
            git_common_dir,
            object_format,
        })
    }

    /// Refuses every tracked, staged, or untracked source modification and
    /// records the exact branch head/tree that a later delivery must still see.
    pub fn qualify_clean_source(
        &self,
        repository: &SourceRepository,
        target_ref: LocalBranchRef,
    ) -> Result<CleanSourceQualification, ProductError> {
        self.assert_repository_executable_config_safe(repository.worktree_root())?;
        self.assert_clean(repository, DirtyScope::Source)?;
        self.assert_head_is_ref(repository, &target_ref, HeadRefScope::Source)?;
        let admitted_base = self.resolve_commit(repository, target_ref.as_str())?;
        let admitted_base_tree = self.resolve_tree_for_commit(repository, &admitted_base)?;
        Ok(CleanSourceQualification {
            repository: repository.clone(),
            target_ref,
            admitted_base,
            admitted_base_tree,
        })
    }

    /// Creates the branch worktree owned by a builder.  The branch name is
    /// derived from the product and attempt identities, and the source must
    /// have been cleanly qualified at the same base immediately beforehand.
    pub fn create_product_worktree(
        &self,
        qualification: &CleanSourceQualification,
        change: ProductChangeId,
        attempt: BuilderAttemptId,
        worktree_root: &WorktreeRoot,
    ) -> Result<ProductWorktree, ProductError> {
        self.assert_repository_executable_config_safe(qualification.repository.worktree_root())?;
        self.assert_worktree_root_is_isolated(&qualification.repository, worktree_root)?;
        self.assert_clean(&qualification.repository, DirtyScope::Source)?;
        let current =
            self.resolve_commit(&qualification.repository, qualification.target_ref.as_str())?;
        if current != qualification.admitted_base {
            return Err(ProductError::SourceHeadMoved {
                expected: qualification.admitted_base.clone(),
                actual: current,
            });
        }
        let branch = ProductWorktreeBranch::derive(&change, &attempt)?;
        let name =
            WorktreeName::parse(format!("builder-{}", derived_pair_name(&change, &attempt)))?;
        let location = worktree_root.allocate(&name);
        location.assert_absent()?;
        self.git_ok(
            qualification.repository.worktree_root(),
            "creating builder product worktree",
            [
                OsString::from("worktree"),
                OsString::from("add"),
                OsString::from("-b"),
                OsString::from(branch.as_str()),
                location.path().as_os_str().to_os_string(),
                OsString::from(qualification.admitted_base.to_hex()),
            ],
            None,
            &[],
        )?;
        let worktree = ProductWorktree {
            repository: qualification.repository.clone(),
            location,
            state: ProductState::WorktreeReady,
            change,
            attempt,
            branch,
            base: qualification.admitted_base.clone(),
            base_tree: qualification.admitted_base_tree.clone(),
        };
        self.assert_clean_worktree(&worktree)?;
        Ok(worktree)
    }

    /// Captures exactly one portable, binary-safe patch and computes the
    /// candidate tree using a temporary index.  A builder commit, source-head
    /// move, empty patch, or untracked file is refused rather than normalized.
    pub fn capture_candidate(
        &self,
        worktree: &ProductWorktree,
        artifacts: &PatchArtifactRoot,
    ) -> Result<CandidateCaptureReceipt, ProductError> {
        self.assert_repository_executable_config_safe(worktree.path())?;
        self.assert_patch_root_is_isolated(&worktree.repository, artifacts)?;
        self.assert_clean_worktree_head(worktree)?;
        let untracked = self.list_untracked(worktree.path())?;
        if !untracked.is_empty() {
            return Err(ProductError::UntrackedCandidateFiles(untracked));
        }
        // `write-tree` seals the temporary-index contents as an immutable Git
        // tree object. Patch bytes and changed paths below are both derived
        // from that one tree pair, never from the mutable builder worktree.
        let candidate_tree = self.candidate_tree_from_temporary_index(worktree)?;
        let patch_bytes = self.git_bytes(
            worktree.path(),
            "capturing binary candidate patch",
            [
                OsString::from("diff"),
                OsString::from("--binary"),
                OsString::from("--full-index"),
                OsString::from("--no-ext-diff"),
                OsString::from("--no-textconv"),
                OsString::from(worktree.base_tree.to_hex()),
                OsString::from(candidate_tree.to_hex()),
                OsString::from("--"),
            ],
            None,
            &[],
        )?;
        if patch_bytes.is_empty() {
            return Err(ProductError::EmptyPatch);
        }
        let changed_paths = self.git_nul_paths(
            worktree.path(),
            "listing candidate changed paths",
            [
                OsString::from("diff"),
                OsString::from("--name-only"),
                OsString::from("-z"),
                OsString::from("--no-ext-diff"),
                OsString::from("--no-textconv"),
                OsString::from(worktree.base_tree.to_hex()),
                OsString::from(candidate_tree.to_hex()),
                OsString::from("--"),
            ],
        )?;
        if changed_paths.is_empty() {
            return Err(ProductError::EmptyChangedPathSet);
        }
        self.assert_clean_worktree_head(worktree)?;
        let untracked_after_snapshot = self.list_untracked(worktree.path())?;
        if !untracked_after_snapshot.is_empty() {
            return Err(ProductError::UntrackedCandidateFiles(
                untracked_after_snapshot,
            ));
        }
        let artifact_name = PatchArtifactName::for_capture(&worktree.change, &worktree.attempt)?;
        let patch_path = artifacts.allocate(&artifact_name);
        self.write_new_artifact(&patch_path, &patch_bytes)?;
        let patch = PortablePatch {
            path: patch_path,
            digest: PatchDigest::of_bytes(&patch_bytes),
        };
        Ok(CandidateCaptureReceipt {
            state: ProductState::CandidateSubmitted,
            change: worktree.change.clone(),
            attempt: worktree.attempt.clone(),
            repository: worktree.repository.clone(),
            base: worktree.base.clone(),
            base_tree: worktree.base_tree.clone(),
            candidate_tree,
            patch,
            changed_paths,
        })
    }

    /// Removes one crate-created builder worktree and returns evidence that its
    /// exact managed path no longer exists.  It never removes arbitrary paths.
    pub fn retire_product_worktree(
        &self,
        worktree: ProductWorktree,
    ) -> Result<CleanupEvidence, ProductError> {
        self.remove_worktree(&worktree.repository, &worktree.location)
    }

    /// Convenience closure for profiles containing only bounded internal Git
    /// checks. Profiles with external-program requirements must use
    /// `prepare_materialization` and `finalize_materialization` so an owner
    /// with cancellation/process-group control can supervise those programs.
    pub fn materialize(
        &self,
        authorization: ProductDeliveryAuthorization,
        capture: &CandidateCaptureReceipt,
        validation_profile: &ValidationProfile,
        commit_spec: &ControlledCommitSpec,
        worktree_root: &WorktreeRoot,
    ) -> Result<MaterializationReceipt, ProductError> {
        // Do this before creating a worktree. `materialize` has no process
        // supervisor and therefore has no truthful cleanup receipt to return
        // for an externally supervised profile.
        if validation_profile
            .commands
            .iter()
            .any(ValidationCommand::requires_external_supervision)
        {
            return Err(ProductError::ExternallySupervisedValidationRequired);
        }
        let prepared = self.prepare_materialization(
            authorization,
            capture,
            validation_profile,
            worktree_root,
        )?;
        let receipt = ExternallySupervisedValidationReceipt {
            profile: validation_profile.id.clone(),
            tree: prepared.tree().clone(),
            steps: Vec::new(),
        };
        self.finalize_materialization(prepared, validation_profile, &receipt, commit_spec)
    }

    /// Creates and patches a fresh detached worktree without spawning an
    /// external validation program. The returned worktree must be finalized
    /// with a matching supervised receipt or explicitly abandoned.
    pub fn prepare_materialization(
        &self,
        authorization: ProductDeliveryAuthorization,
        capture: &CandidateCaptureReceipt,
        validation_profile: &ValidationProfile,
        worktree_root: &WorktreeRoot,
    ) -> Result<PreparedMaterialization, ProductError> {
        self.assert_repository_executable_config_safe(capture.repository.worktree_root())?;
        self.assert_authorization_matches_capture(&authorization, capture, validation_profile)?;
        self.assert_worktree_root_is_isolated(&capture.repository, worktree_root)?;
        let patch_bytes = self.read_bounded_patch_artifact(&capture.patch)?;
        let name = WorktreeName::parse(format!("materialize-{}", authorization.id.as_str()))?;
        let location = worktree_root.allocate(&name);
        location.assert_absent()?;
        self.git_ok(
            capture.repository.worktree_root(),
            "creating fresh materialization worktree",
            [
                OsString::from("worktree"),
                OsString::from("add"),
                OsString::from("--detach"),
                location.path().as_os_str().to_os_string(),
                OsString::from(authorization.admitted_base.to_hex()),
            ],
            None,
            &[],
        )?;
        let result = self.apply_patch_to_fresh_worktree(
            &authorization,
            &capture.repository,
            &location,
            &patch_bytes,
        );
        match result {
            Ok(tree) => Ok(PreparedMaterialization {
                authorization: authorization.clone(),
                repository: capture.repository.clone(),
                location,
                materialized: MaterializedTreeReceipt {
                    state: ProductState::Materialized,
                    base: authorization.admitted_base.clone(),
                    base_tree: capture.base_tree.clone(),
                    tree,
                    patch: authorization.accepted_patch.clone(),
                },
            }),
            Err(error) => {
                let cleanup = self
                    .remove_worktree(&capture.repository, &location)
                    .unwrap_or(CleanupEvidence::RetainedAfterFailure {
                        worktree_name: location.name.0.clone(),
                    });
                Err(ProductError::MaterializationFailed {
                    source: Box::new(error),
                    cleanup,
                })
            }
        }
    }

    /// Verifies the externally supervised validation-program receipt, runs
    /// bounded Git checks, proves the prepared worktree still names the
    /// expected tree, and constructs one deterministic no-hook commit before
    /// cleanup.
    pub fn finalize_materialization(
        &self,
        prepared: PreparedMaterialization,
        validation_profile: &ValidationProfile,
        supervised: &ExternallySupervisedValidationReceipt,
        commit_spec: &ControlledCommitSpec,
    ) -> Result<MaterializationReceipt, ProductError> {
        let result = (|| {
            self.assert_repository_executable_config_safe(prepared.path())?;
            self.assert_prepared_worktree_intact(&prepared)?;
            let validation = self.run_validation(
                prepared.path(),
                prepared.tree(),
                validation_profile,
                supervised,
            )?;
            self.assert_prepared_worktree_intact(&prepared)?;
            let commit = self.construct_controlled_commit(
                prepared.path(),
                prepared.repository.object_format,
                &prepared.authorization.admitted_base,
                prepared.tree(),
                commit_spec,
            )?;
            Ok((validation, commit))
        })();
        match result {
            Ok((validation, commit)) => {
                let cleanup = self.remove_worktree(&prepared.repository, &prepared.location)?;
                Ok(MaterializationReceipt {
                    state: ProductState::DeliveryReady,
                    authorization: prepared.authorization,
                    materialized: prepared.materialized,
                    commit_validated: CommitValidatedReceipt {
                        state: ProductState::CommitValidated,
                        validation,
                        commit,
                    },
                    cleanup,
                })
            }
            Err(error) => {
                let cleanup = self
                    .remove_worktree(&prepared.repository, &prepared.location)
                    .unwrap_or(CleanupEvidence::RetainedAfterFailure {
                        worktree_name: prepared.location.name.0.clone(),
                    });
                Err(ProductError::MaterializationFailed {
                    source: Box::new(error),
                    cleanup,
                })
            }
        }
    }

    /// Retires a prepared worktree when its external supervisor declines to
    /// issue a receipt. No delivery or commit is attempted.
    pub fn abandon_prepared_materialization(
        &self,
        prepared: PreparedMaterialization,
    ) -> Result<CleanupEvidence, ProductError> {
        self.remove_worktree(&prepared.repository, &prepared.location)
    }

    /// Fast-forwards the named local branch only when its checked-out head is
    /// still the admitted base.  No rebase or remote operation exists here.
    pub fn deliver(
        &self,
        qualification: &CleanSourceQualification,
        materialization: &MaterializationReceipt,
    ) -> Result<DeliveryReceipt, ProductError> {
        self.assert_repository_executable_config_safe(qualification.repository.worktree_root())?;
        self.assert_materialization_cross_links(materialization)?;
        if qualification.repository != materialization.authorization.repository {
            return Err(ProductError::DeliveryRepositoryMismatch);
        }
        if qualification.target_ref != materialization.authorization.target_ref
            || qualification.admitted_base != materialization.authorization.admitted_base
        {
            return Err(ProductError::DeliveryAdmissionMismatch);
        }
        self.assert_head_is_ref(
            &qualification.repository,
            &qualification.target_ref,
            HeadRefScope::Target,
        )?;
        let current =
            self.resolve_commit(&qualification.repository, qualification.target_ref.as_str())?;
        if current == materialization.commit_validated.commit.commit {
            self.assert_delivered_checkout_consistent(&qualification.repository, materialization)
                .map_err(|source| ProductError::DeliveryCheckoutRecoveryRequired {
                    delivered_commit: materialization.commit_validated.commit.commit.clone(),
                    source: Box::new(source),
                })?;
            return Ok(DeliveryReceipt {
                state: ProductState::Delivered,
                authorization: materialization.authorization.id.clone(),
                target_ref: qualification.target_ref.clone(),
                expected_old_head: qualification.admitted_base.clone(),
                delivered_commit: materialization.commit_validated.commit.commit.clone(),
                delivered_tree: materialization.materialized.tree.clone(),
                disposition: DeliveryDisposition::AlreadyDelivered,
            });
        }
        if current != qualification.admitted_base {
            return Err(ProductError::TargetHeadMoved {
                expected: qualification.admitted_base.clone(),
                actual: current,
            });
        }
        self.assert_clean(&qualification.repository, DirtyScope::Target)?;
        self.assert_controlled_commit(&qualification.repository, materialization)?;
        self.git_ok(
            qualification.repository.worktree_root(),
            "guarded local branch fast-forward",
            [
                OsString::from("update-ref"),
                OsString::from(qualification.target_ref.as_str()),
                OsString::from(materialization.commit_validated.commit.commit.to_hex()),
                OsString::from(qualification.admitted_base.to_hex()),
            ],
            None,
            &[],
        )?;
        // `update-ref` supplies the compare-and-swap guard.  With a source
        // proved clean before the CAS, `read-tree --reset -u` updates its index
        // and files without checkout/merge hooks.  A filesystem failure after
        // the CAS is surfaced, never hidden as a complete delivery receipt.
        self.update_delivered_checkout(&qualification.repository, materialization)
            .map_err(|source| ProductError::DeliveryCheckoutRecoveryRequired {
                delivered_commit: materialization.commit_validated.commit.commit.clone(),
                source: Box::new(source),
            })?;
        Ok(DeliveryReceipt {
            state: ProductState::Delivered,
            authorization: materialization.authorization.id.clone(),
            target_ref: qualification.target_ref.clone(),
            expected_old_head: qualification.admitted_base.clone(),
            delivered_commit: materialization.commit_validated.commit.commit.clone(),
            delivered_tree: materialization.materialized.tree.clone(),
            disposition: DeliveryDisposition::FastForwarded,
        })
    }

    fn apply_patch_to_fresh_worktree(
        &self,
        authorization: &ProductDeliveryAuthorization,
        repository: &SourceRepository,
        location: &ManagedWorktreePath,
        patch_bytes: &[u8],
    ) -> Result<TreeId, ProductError> {
        let head = self.resolve_commit_at_path(location.path(), repository.object_format)?;
        if head != authorization.admitted_base {
            return Err(ProductError::FreshWorktreeBaseMismatch {
                expected: authorization.admitted_base.clone(),
                actual: head,
            });
        }
        self.git_ok(
            location.path(),
            "applying accepted binary patch",
            [
                OsString::from("apply"),
                OsString::from("--index"),
                OsString::from("--binary"),
                OsString::from("--whitespace=nowarn"),
            ],
            Some(patch_bytes),
            &[],
        )?;
        let materialized_tree = self.write_tree(location.path(), repository.object_format)?;
        if materialized_tree != authorization.accepted_tree {
            return Err(ProductError::MaterializedTreeMismatch {
                expected: authorization.accepted_tree.clone(),
                actual: materialized_tree,
            });
        }
        Ok(materialized_tree)
    }

    fn assert_authorization_matches_capture(
        &self,
        authorization: &ProductDeliveryAuthorization,
        capture: &CandidateCaptureReceipt,
        validation: &ValidationProfile,
    ) -> Result<(), ProductError> {
        if authorization.change != capture.change
            || authorization.repository != capture.repository
            || authorization.admitted_base != capture.base
            || authorization.accepted_tree != capture.candidate_tree
            || authorization.accepted_patch != capture.patch.digest
        {
            return Err(ProductError::AuthorizationDoesNotMatchCapture);
        }
        if authorization.validation_profile != validation.id {
            return Err(ProductError::AuthorizationDoesNotMatchValidationProfile);
        }
        Ok(())
    }

    /// Defense in depth for a receipt which may have crossed a future durable
    /// boundary. Safe Rust callers cannot replace receipt bodies, but delivery
    /// still reconstructs every authorization-to-materialization edge before
    /// touching the target branch.
    fn assert_materialization_cross_links(
        &self,
        materialization: &MaterializationReceipt,
    ) -> Result<(), ProductError> {
        let authorization = &materialization.authorization;
        let materialized = &materialization.materialized;
        let commit_validated = &materialization.commit_validated;
        let validation = &commit_validated.validation;
        let commit = &commit_validated.commit;
        if materialization.state != ProductState::DeliveryReady
            || materialized.state != ProductState::Materialized
            || commit_validated.state != ProductState::CommitValidated
            || materialized.base != authorization.admitted_base
            || materialized.tree != authorization.accepted_tree
            || materialized.patch != authorization.accepted_patch
            || validation.profile != authorization.validation_profile
            || validation.tree != authorization.accepted_tree
            || validation.digest
                != validation_digest(&validation.profile, &validation.tree, &validation.steps)
            || commit.parent != authorization.admitted_base
            || commit.tree != authorization.accepted_tree
            || commit.tree != materialized.tree
        {
            return Err(ProductError::MaterializationReceiptCrossLinkMismatch);
        }
        Ok(())
    }

    /// Performs the only checkout synchronization after the guarded branch
    /// CAS, then proves both the index and working copy are exact. Any error
    /// is raised through the recovery fence at the caller.
    fn update_delivered_checkout(
        &self,
        repository: &SourceRepository,
        materialization: &MaterializationReceipt,
    ) -> Result<(), ProductError> {
        self.git_ok(
            repository.worktree_root(),
            "updating delivered checkout tree",
            [
                OsString::from("read-tree"),
                OsString::from("--reset"),
                OsString::from("-u"),
                OsString::from(materialization.commit_validated.commit.commit.to_hex()),
            ],
            None,
            &[],
        )?;
        self.assert_delivered_checkout_consistent(repository, materialization)
    }

    /// This check is deliberately non-mutating. On a retry after a partial
    /// post-CAS failure it forces a manual exact-tree recovery instead of
    /// silently using an idempotent call to repair a checkout whose state was
    /// never recorded as delivered.
    fn assert_delivered_checkout_consistent(
        &self,
        repository: &SourceRepository,
        materialization: &MaterializationReceipt,
    ) -> Result<(), ProductError> {
        let delivered_head = self.resolve_commit(
            repository,
            materialization.authorization.target_ref.as_str(),
        )?;
        if delivered_head != materialization.commit_validated.commit.commit {
            return Err(ProductError::DeliveredHeadMismatch {
                expected: materialization.commit_validated.commit.commit.clone(),
                actual: delivered_head,
            });
        }
        let working_copy = self.git_output(
            repository.worktree_root(),
            "checking delivered working copy",
            [
                OsString::from("diff"),
                OsString::from("--quiet"),
                OsString::from("--no-ext-diff"),
                OsString::from("--no-textconv"),
            ],
            None,
            &[],
        )?;
        if !working_copy.status.success() {
            return Err(ProductError::DeliveryCheckoutInconsistent);
        }
        let index = self.git_output(
            repository.worktree_root(),
            "checking delivered index",
            [
                OsString::from("diff"),
                OsString::from("--cached"),
                OsString::from("--quiet"),
                OsString::from("--no-ext-diff"),
                OsString::from("--no-textconv"),
            ],
            None,
            &[],
        )?;
        if !index.status.success() {
            return Err(ProductError::DeliveryCheckoutInconsistent);
        }
        self.assert_clean(repository, DirtyScope::Target)
    }

    fn assert_supervised_validation_matches(
        &self,
        profile: &ValidationProfile,
        tree: &TreeId,
        supervised: &ExternallySupervisedValidationReceipt,
    ) -> Result<(), ProductError> {
        if supervised.profile() != &profile.id || supervised.tree() != tree {
            return Err(ProductError::ExternallySupervisedValidationReceiptMismatch);
        }
        let expected: Vec<_> = profile
            .commands
            .iter()
            .filter_map(|command| match command {
                ValidationCommand::GitDiffCheck => None,
                ValidationCommand::ExternallySupervisedProgram(invocation) => Some(invocation),
            })
            .collect();
        if expected.len() != supervised.steps().len()
            || expected
                .iter()
                .zip(supervised.steps())
                .any(|(expected, observed)| *expected != observed.invocation())
        {
            return Err(ProductError::ExternallySupervisedValidationReceiptMismatch);
        }
        Ok(())
    }

    fn assert_prepared_worktree_intact(
        &self,
        prepared: &PreparedMaterialization,
    ) -> Result<(), ProductError> {
        let tree = self.write_tree(prepared.path(), prepared.repository.object_format)?;
        if tree != *prepared.tree() {
            return Err(ProductError::PreparedTreeChanged {
                expected: prepared.tree().clone(),
                actual: tree,
            });
        }
        let untracked = self.list_untracked(prepared.path())?;
        if !untracked.is_empty() {
            return Err(ProductError::PreparedWorktreeUntrackedFiles(untracked));
        }
        let output = self.git_output(
            prepared.path(),
            "checking prepared worktree against its index",
            [
                OsString::from("diff"),
                OsString::from("--quiet"),
                OsString::from("--no-ext-diff"),
                OsString::from("--no-textconv"),
            ],
            None,
            &[],
        )?;
        if !output.status.success() {
            return Err(ProductError::PreparedWorktreeModified {
                stdout: OutputDigest::of_bytes(&output.stdout),
                stderr: OutputDigest::of_bytes(&output.stderr),
            });
        }
        Ok(())
    }

    fn run_validation(
        &self,
        worktree: &Path,
        tree: &TreeId,
        profile: &ValidationProfile,
        supervised: &ExternallySupervisedValidationReceipt,
    ) -> Result<ValidationReceipt, ProductError> {
        self.assert_supervised_validation_matches(profile, tree, supervised)?;
        let mut completed = Vec::with_capacity(profile.commands.len());
        let mut supervised_steps = supervised.steps().iter();
        for command in &profile.commands {
            let output = match command {
                ValidationCommand::GitDiffCheck => self.git_output(
                    worktree,
                    "running Git whitespace validation",
                    [
                        OsString::from("diff"),
                        OsString::from("--check"),
                        OsString::from("--no-ext-diff"),
                        OsString::from("--no-textconv"),
                        OsString::from("--cached"),
                    ],
                    None,
                    &[],
                )?,
                ValidationCommand::ExternallySupervisedProgram(_) => {
                    let step = supervised_steps
                        .next()
                        .ok_or(ProductError::ExternallySupervisedValidationReceiptMismatch)?;
                    completed.push(ValidationStepReceipt {
                        command: command.clone(),
                        stdout: step.stdout().clone(),
                        stderr: step.stderr().clone(),
                    });
                    continue;
                }
            };
            let stdout = OutputDigest::of_bytes(&output.stdout);
            let stderr = OutputDigest::of_bytes(&output.stderr);
            if !output.status.success() {
                return Err(ProductError::ValidationFailed(Box::new(
                    ValidationFailureReceipt {
                        profile: profile.id.clone(),
                        tree: tree.clone(),
                        completed_steps: completed,
                        failed_command: command.clone(),
                        stdout,
                        stderr,
                    },
                )));
            }
            completed.push(ValidationStepReceipt {
                command: command.clone(),
                stdout,
                stderr,
            });
        }
        let digest = validation_digest(&profile.id, tree, &completed);
        Ok(ValidationReceipt {
            profile: profile.id.clone(),
            tree: tree.clone(),
            steps: completed,
            digest,
        })
    }

    fn construct_controlled_commit(
        &self,
        worktree: &Path,
        format: GitObjectFormat,
        parent: &CommitId,
        tree: &TreeId,
        specification: &ControlledCommitSpec,
    ) -> Result<ControlledCommitReceipt, ProductError> {
        let author_time = specification.author_time.git_value();
        let committer_time = specification.committer_time.git_value();
        let extra_env = [
            ("GIT_AUTHOR_NAME", OsStr::new(&specification.author.name)),
            ("GIT_AUTHOR_EMAIL", OsStr::new(&specification.author.email)),
            ("GIT_AUTHOR_DATE", OsStr::new(&author_time)),
            (
                "GIT_COMMITTER_NAME",
                OsStr::new(&specification.committer.name),
            ),
            (
                "GIT_COMMITTER_EMAIL",
                OsStr::new(&specification.committer.email),
            ),
            ("GIT_COMMITTER_DATE", OsStr::new(&committer_time)),
        ];
        let output = self.git_ok(
            worktree,
            "constructing controlled no-hook commit",
            [
                OsString::from("commit-tree"),
                OsString::from(tree.to_hex()),
                OsString::from("-p"),
                OsString::from(parent.to_hex()),
            ],
            Some(specification.message.as_bytes()),
            &extra_env,
        )?;
        let commit = CommitId::parse(
            single_line_utf8(&output, "controlled commit identity")?,
            format,
        )?;
        let parsed_tree = self.resolve_tree_for_commit_at_path(worktree, &commit, format)?;
        if parsed_tree != *tree {
            return Err(ProductError::ConstructedCommitTreeMismatch {
                expected: tree.clone(),
                actual: parsed_tree,
            });
        }
        let parents = self.commit_parents(worktree, &commit, format)?;
        if parents.as_slice() != [parent.clone()] {
            return Err(ProductError::ConstructedCommitParentMismatch {
                expected: parent.clone(),
                actual: parents,
            });
        }
        Ok(ControlledCommitReceipt {
            commit,
            parent: parent.clone(),
            tree: tree.clone(),
            message: CommitMessageDigest::of_bytes(specification.message.as_bytes()),
            hook_policy: HookPolicy::GitCommitTreeWithHooksDisabled,
        })
    }

    fn assert_controlled_commit(
        &self,
        repository: &SourceRepository,
        receipt: &MaterializationReceipt,
    ) -> Result<(), ProductError> {
        let tree =
            self.resolve_tree_for_commit(repository, &receipt.commit_validated.commit.commit)?;
        if tree != receipt.materialized.tree || tree != receipt.commit_validated.commit.tree {
            return Err(ProductError::DeliveryTreeMismatch {
                expected: receipt.materialized.tree.clone(),
                actual: tree,
            });
        }
        let parents = self.commit_parents(
            repository.worktree_root(),
            &receipt.commit_validated.commit.commit,
            repository.object_format,
        )?;
        if parents.as_slice() != [receipt.authorization.admitted_base.clone()] {
            return Err(ProductError::DeliveryParentMismatch {
                expected: receipt.authorization.admitted_base.clone(),
                actual: parents,
            });
        }
        Ok(())
    }

    fn assert_worktree_root_is_isolated(
        &self,
        repository: &SourceRepository,
        worktree_root: &WorktreeRoot,
    ) -> Result<(), ProductError> {
        if worktree_root.0 == repository.worktree_root
            || worktree_root.0.starts_with(&repository.worktree_root)
        {
            return Err(ProductError::WorktreeRootNotIsolated(
                worktree_root.0.clone(),
            ));
        }
        Ok(())
    }

    fn assert_patch_root_is_isolated(
        &self,
        repository: &SourceRepository,
        artifacts: &PatchArtifactRoot,
    ) -> Result<(), ProductError> {
        if artifacts.0 == repository.worktree_root
            || artifacts.0.starts_with(&repository.worktree_root)
        {
            return Err(ProductError::PatchArtifactRootNotIsolated(
                artifacts.0.clone(),
            ));
        }
        Ok(())
    }

    fn assert_clean_worktree(&self, worktree: &ProductWorktree) -> Result<(), ProductError> {
        self.assert_clean_at_path(worktree.path(), DirtyScope::Builder)?;
        self.assert_clean_worktree_head(worktree)
    }

    fn assert_clean_worktree_head(&self, worktree: &ProductWorktree) -> Result<(), ProductError> {
        let head =
            self.resolve_commit_at_path(worktree.path(), worktree.repository.object_format)?;
        if head != worktree.base {
            return Err(ProductError::BuilderCommittedOrHeadMoved {
                expected: worktree.base.clone(),
                actual: head,
            });
        }
        Ok(())
    }

    fn assert_clean(
        &self,
        repository: &SourceRepository,
        scope: DirtyScope,
    ) -> Result<(), ProductError> {
        self.assert_clean_at_path(repository.worktree_root(), scope)
    }

    fn assert_clean_at_path(&self, path: &Path, scope: DirtyScope) -> Result<(), ProductError> {
        let status = self.git_bytes(
            path,
            "checking clean Git status",
            [
                OsString::from("status"),
                OsString::from("--porcelain=v1"),
                OsString::from("-z"),
                OsString::from("--untracked-files=all"),
            ],
            None,
            &[],
        )?;
        if status.is_empty() {
            return Ok(());
        }
        let digest = OutputDigest::of_bytes(&status);
        match scope {
            DirtyScope::Source => Err(ProductError::SourceNotClean { status: digest }),
            DirtyScope::Target => Err(ProductError::TargetNotClean { status: digest }),
            DirtyScope::Builder => Err(ProductError::BuilderWorktreeNotClean { status: digest }),
        }
    }

    fn assert_head_is_ref(
        &self,
        repository: &SourceRepository,
        target: &LocalBranchRef,
        scope: HeadRefScope,
    ) -> Result<(), ProductError> {
        let output = self.git_output(
            repository.worktree_root(),
            "checking symbolic HEAD",
            [
                OsString::from("symbolic-ref"),
                OsString::from("-q"),
                OsString::from("HEAD"),
            ],
            None,
            &[],
        )?;
        if !output.status.success() {
            return match scope {
                HeadRefScope::Source => Err(ProductError::SourceHeadNotTargetRef),
                HeadRefScope::Target => Err(ProductError::TargetHeadNotTargetRef),
            };
        }
        let actual = single_line_utf8(&output.stdout, "symbolic HEAD")?;
        if actual != target.as_str() {
            return match scope {
                HeadRefScope::Source => Err(ProductError::SourceHeadNotTargetRef),
                HeadRefScope::Target => Err(ProductError::TargetHeadNotTargetRef),
            };
        }
        Ok(())
    }

    fn list_untracked(&self, path: &Path) -> Result<Vec<RepositoryPath>, ProductError> {
        self.git_nul_paths(
            path,
            "listing candidate untracked paths",
            [
                OsString::from("ls-files"),
                OsString::from("--others"),
                OsString::from("--exclude-standard"),
                OsString::from("-z"),
            ],
        )
    }

    fn git_nul_paths<const N: usize>(
        &self,
        path: &Path,
        operation: &'static str,
        arguments: [OsString; N],
    ) -> Result<Vec<RepositoryPath>, ProductError> {
        let bytes = self.git_bytes(path, operation, arguments, None, &[])?;
        if bytes.is_empty() {
            return Ok(Vec::new());
        }
        if !bytes.ends_with(&[0]) {
            return Err(ProductError::MalformedNulDelimitedGitOutput(operation));
        }
        bytes[..bytes.len() - 1]
            .split(|byte| *byte == 0)
            .map(RepositoryPath::from_git_output)
            .collect()
    }

    fn candidate_tree_from_temporary_index(
        &self,
        worktree: &ProductWorktree,
    ) -> Result<TreeId, ProductError> {
        let index_path = worktree.path().join(format!(
            ".guarded-materialization-index-{}-{}",
            worktree.change.as_str(),
            worktree.attempt.as_str()
        ));
        let index_lock_path = index_path.with_file_name(format!(
            "{}.lock",
            index_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
        ));
        // Neither path is owned before `read-tree` starts. In particular, a
        // stale or hostile preexisting lock must never be erased by cleanup.
        if directory_entry_exists_no_follow(&index_path, "checking candidate temporary index")? {
            return Err(ProductError::TemporaryIndexAlreadyExists(index_path));
        }
        if directory_entry_exists_no_follow(
            &index_lock_path,
            "checking candidate temporary index lock",
        )? {
            return Err(ProductError::TemporaryIndexAlreadyExists(index_lock_path));
        }
        let index_value = index_path.as_os_str().to_os_string();
        let index_env = [("GIT_INDEX_FILE", index_value.as_os_str())];
        let result = (|| {
            self.git_ok(
                worktree.path(),
                "creating candidate temporary index",
                [
                    OsString::from("read-tree"),
                    OsString::from(worktree.base.to_hex()),
                ],
                None,
                &index_env,
            )?;
            self.git_ok(
                worktree.path(),
                "staging tracked candidate tree",
                [OsString::from("add"), OsString::from("-u")],
                None,
                &index_env,
            )?;
            let output = self.git_ok(
                worktree.path(),
                "writing candidate tree",
                [OsString::from("write-tree")],
                None,
                &index_env,
            )?;
            TreeId::parse(
                single_line_utf8(&output, "candidate tree identity")?,
                worktree.repository.object_format,
            )
        })();
        let index_cleanup = fs::remove_file(&index_path);
        let lock_cleanup = fs::remove_file(&index_lock_path);
        if let Err(source) = index_cleanup
            && source.kind() != io::ErrorKind::NotFound
        {
            return Err(ProductError::Io {
                operation: "removing candidate temporary index",
                path: index_path,
                source,
            });
        }
        if let Err(source) = lock_cleanup
            && source.kind() != io::ErrorKind::NotFound
        {
            return Err(ProductError::Io {
                operation: "removing candidate temporary index lock",
                path: index_lock_path,
                source,
            });
        }
        result
    }

    fn write_tree(&self, path: &Path, format: GitObjectFormat) -> Result<TreeId, ProductError> {
        let output = self.git_ok(
            path,
            "writing materialized tree",
            [OsString::from("write-tree")],
            None,
            &[],
        )?;
        TreeId::parse(
            single_line_utf8(&output, "materialized tree identity")?,
            format,
        )
    }

    fn resolve_commit(
        &self,
        repository: &SourceRepository,
        revision: &str,
    ) -> Result<CommitId, ProductError> {
        self.resolve_commit_at_revision(
            repository.worktree_root(),
            revision,
            repository.object_format,
        )
    }

    fn resolve_commit_at_path(
        &self,
        path: &Path,
        format: GitObjectFormat,
    ) -> Result<CommitId, ProductError> {
        self.resolve_commit_at_revision(path, "HEAD", format)
    }

    fn resolve_commit_at_revision(
        &self,
        path: &Path,
        revision: &str,
        format: GitObjectFormat,
    ) -> Result<CommitId, ProductError> {
        let output = self.git_ok(
            path,
            "resolving commit",
            [
                OsString::from("rev-parse"),
                OsString::from("--verify"),
                OsString::from(format!("{revision}^{{commit}}")),
            ],
            None,
            &[],
        )?;
        CommitId::parse(single_line_utf8(&output, "commit identity")?, format)
    }

    fn resolve_tree_for_commit(
        &self,
        repository: &SourceRepository,
        commit: &CommitId,
    ) -> Result<TreeId, ProductError> {
        self.resolve_tree_for_commit_at_path(
            repository.worktree_root(),
            commit,
            repository.object_format,
        )
    }

    fn resolve_tree_for_commit_at_path(
        &self,
        path: &Path,
        commit: &CommitId,
        format: GitObjectFormat,
    ) -> Result<TreeId, ProductError> {
        let output = self.git_ok(
            path,
            "resolving commit tree",
            [
                OsString::from("rev-parse"),
                OsString::from("--verify"),
                OsString::from(format!("{}^{{tree}}", commit.to_hex())),
            ],
            None,
            &[],
        )?;
        TreeId::parse(single_line_utf8(&output, "tree identity")?, format)
    }

    fn commit_parents(
        &self,
        path: &Path,
        commit: &CommitId,
        format: GitObjectFormat,
    ) -> Result<Vec<CommitId>, ProductError> {
        let output = self.git_text(
            path,
            "reading controlled commit parents",
            ["rev-list", "--parents", "-n", "1", &commit.to_hex()],
        )?;
        let mut identities = output.split_ascii_whitespace();
        let Some(reported_commit) = identities.next() else {
            return Err(ProductError::MalformedCommitParentOutput);
        };
        if CommitId::parse(reported_commit, format)? != *commit {
            return Err(ProductError::MalformedCommitParentOutput);
        }
        identities
            .map(|identity| CommitId::parse(identity, format))
            .collect()
    }

    fn remove_worktree(
        &self,
        repository: &SourceRepository,
        location: &ManagedWorktreePath,
    ) -> Result<CleanupEvidence, ProductError> {
        location.assert_owned()?;
        self.git_ok(
            repository.worktree_root(),
            "removing managed Git worktree",
            [
                OsString::from("worktree"),
                OsString::from("remove"),
                OsString::from("--force"),
                location.path().as_os_str().to_os_string(),
            ],
            None,
            &[],
        )?;
        if directory_entry_exists_no_follow(location.path(), "verifying managed worktree removal")?
        {
            return Err(ProductError::WorktreeRemovalNotVerified(
                location.path().to_path_buf(),
            ));
        }
        Ok(CleanupEvidence::Removed {
            worktree_name: location.name.0.clone(),
        })
    }

    fn read_bounded_patch_artifact(&self, patch: &PortablePatch) -> Result<Vec<u8>, ProductError> {
        let metadata = fs::metadata(&patch.path).map_err(|source| ProductError::Io {
            operation: "reading accepted patch artifact metadata",
            path: patch.path.clone(),
            source,
        })?;
        if !metadata.is_file() || metadata.len() > MAX_PORTABLE_PATCH_BYTES as u64 {
            return Err(ProductError::PortablePatchTooLarge {
                actual: metadata.len(),
                limit: MAX_PORTABLE_PATCH_BYTES,
            });
        }
        let file = fs::File::open(&patch.path).map_err(|source| ProductError::Io {
            operation: "opening accepted patch artifact",
            path: patch.path.clone(),
            source,
        })?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        // Metadata can race with an artifact-path replacement. Bound the read
        // itself too, so the race cannot recover an unbounded allocation.
        file.take(MAX_PORTABLE_PATCH_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|source| ProductError::Io {
                operation: "reading accepted patch artifact",
                path: patch.path.clone(),
                source,
            })?;
        if bytes.len() > MAX_PORTABLE_PATCH_BYTES {
            return Err(ProductError::PortablePatchTooLarge {
                actual: bytes.len() as u64,
                limit: MAX_PORTABLE_PATCH_BYTES,
            });
        }
        let actual_digest = PatchDigest::of_bytes(&bytes);
        if actual_digest != patch.digest {
            return Err(ProductError::AcceptedPatchDigestMismatch {
                expected: patch.digest.clone(),
                actual: actual_digest,
            });
        }
        Ok(bytes)
    }

    fn write_new_artifact(&self, path: &Path, bytes: &[u8]) -> Result<(), ProductError> {
        if bytes.len() > MAX_PORTABLE_PATCH_BYTES {
            return Err(ProductError::PortablePatchTooLarge {
                actual: bytes.len() as u64,
                limit: MAX_PORTABLE_PATCH_BYTES,
            });
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|source| ProductError::Io {
                operation: "creating portable patch artifact",
                path: path.to_path_buf(),
                source,
            })?;
        file.write_all(bytes).map_err(|source| ProductError::Io {
            operation: "writing portable patch artifact",
            path: path.to_path_buf(),
            source,
        })?;
        file.sync_all().map_err(|source| ProductError::Io {
            operation: "syncing portable patch artifact",
            path: path.to_path_buf(),
            source,
        })
    }

    /// Git's local/worktree config can otherwise name executable filter,
    /// fsmonitor, or diff programs even when the ambient environment is empty.
    /// This scan uses `--no-includes`; included config is itself rejected so the
    /// normal invocations cannot reintroduce a hidden executable surface.
    fn assert_repository_executable_config_safe(&self, path: &Path) -> Result<(), ProductError> {
        let keys = self.git_bytes(
            path,
            "enumerating repository-local Git configuration",
            [
                OsString::from("config"),
                OsString::from("--no-includes"),
                OsString::from("--null"),
                OsString::from("--name-only"),
                OsString::from("--list"),
            ],
            None,
            &[],
        )?;
        if keys.is_empty() {
            return Ok(());
        }
        if !keys.ends_with(&[0]) {
            return Err(ProductError::MalformedNulDelimitedGitOutput(
                "enumerating repository-local Git configuration",
            ));
        }
        for raw_key in keys[..keys.len() - 1].split(|byte| *byte == 0) {
            let key = std::str::from_utf8(raw_key)
                .map_err(|_| ProductError::NonUtf8GitOutput("reading Git configuration key"))?;
            if is_executable_git_config_key(key) {
                return Err(ProductError::RepositoryExecutableConfigDenied {
                    key: key.to_owned(),
                });
            }
        }
        Ok(())
    }

    fn git_text<const N: usize>(
        &self,
        path: &Path,
        operation: &'static str,
        arguments: [&str; N],
    ) -> Result<String, ProductError> {
        let output = self.git_ok(path, operation, arguments.map(OsString::from), None, &[])?;
        String::from_utf8(output).map_err(|_| ProductError::NonUtf8GitOutput(operation))
    }

    fn git_bytes<const N: usize>(
        &self,
        path: &Path,
        operation: &'static str,
        arguments: [OsString; N],
        input: Option<&[u8]>,
        extra_env: &[(&str, &std::ffi::OsStr)],
    ) -> Result<Vec<u8>, ProductError> {
        self.git_ok(path, operation, arguments, input, extra_env)
    }

    fn git_ok<const N: usize>(
        &self,
        path: &Path,
        operation: &'static str,
        arguments: [OsString; N],
        input: Option<&[u8]>,
        extra_env: &[(&str, &std::ffi::OsStr)],
    ) -> Result<Vec<u8>, ProductError> {
        let output = self.git_output(path, operation, arguments, input, extra_env)?;
        if !output.status.success() {
            return Err(ProductError::GitFailure {
                operation,
                status: output.status.code(),
                stdout: OutputDigest::of_bytes(&output.stdout),
                stderr: OutputDigest::of_bytes(&output.stderr),
            });
        }
        Ok(output.stdout)
    }

    fn git_output<const N: usize>(
        &self,
        path: &Path,
        operation: &'static str,
        arguments: [OsString; N],
        input: Option<&[u8]>,
        extra_env: &[(&str, &std::ffi::OsStr)],
    ) -> Result<Output, ProductError> {
        let mut command = Command::new(&self.git);
        command
            .current_dir(path)
            .env_clear()
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_PAGER", "cat")
            // Repository-local replace refs and any interactive prompt are not
            // valid inputs to a deterministic product receipt.
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .arg("-c")
            .arg("core.hooksPath=/dev/null")
            .arg("-c")
            .arg("core.fsmonitor=false")
            .arg("-c")
            .arg("core.attributesfile=/dev/null")
            .arg("-c")
            .arg("diff.external=")
            .args(arguments)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in extra_env {
            command.env(key, value);
        }
        let mut child = command.spawn().map_err(|source| ProductError::Io {
            operation: "spawning Git argv operation",
            path: self.git.clone(),
            source,
        })?;
        let stdout = child.stdout.take().ok_or(ProductError::MissingGitStdout)?;
        let stderr = child.stderr.take().ok_or(ProductError::MissingGitStderr)?;
        let stdout_reader = spawn_bounded_reader(stdout, MAX_GIT_STDOUT_BYTES);
        let stderr_reader = spawn_bounded_reader(stderr, MAX_GIT_STDERR_BYTES);
        let stdin_writer = input
            .map(|bytes| {
                let mut stdin = child.stdin.take().ok_or(ProductError::MissingGitStdin)?;
                let bytes = bytes.to_vec();
                Ok(thread::spawn(move || stdin.write_all(&bytes)))
            })
            .transpose()?;

        let started = Instant::now();
        let mut timed_out = false;
        let status = loop {
            match child.try_wait().map_err(|source| ProductError::Io {
                operation: "polling Git argv operation",
                path: self.git.clone(),
                source,
            })? {
                Some(status) => break status,
                None if started.elapsed() >= GIT_OPERATION_TIMEOUT => {
                    timed_out = true;
                    child.kill().map_err(|source| ProductError::Io {
                        operation: "terminating timed-out Git argv operation",
                        path: self.git.clone(),
                        source,
                    })?;
                    break child.wait().map_err(|source| ProductError::Io {
                        operation: "waiting for terminated Git argv operation",
                        path: self.git.clone(),
                        source,
                    })?;
                }
                None => thread::sleep(Duration::from_millis(10)),
            }
        };
        let stdout = join_bounded_reader(stdout_reader, operation, "stdout")?;
        let stderr = join_bounded_reader(stderr_reader, operation, "stderr")?;
        if let Some(writer) = stdin_writer {
            let write_result = writer
                .join()
                .map_err(|_| ProductError::ProcessCollectorPanicked("Git stdin writer"))?;
            if let Err(source) = write_result
                && status.success()
            {
                return Err(ProductError::Io {
                    operation: "writing Git argv stdin",
                    path: self.git.clone(),
                    source,
                });
            }
        }
        if timed_out {
            return Err(ProductError::GitOperationTimedOut { operation });
        }
        if stdout.exceeded {
            return Err(ProductError::GitOutputLimitExceeded {
                operation,
                stream: "stdout",
                limit: MAX_GIT_STDOUT_BYTES,
            });
        }
        if stderr.exceeded {
            return Err(ProductError::GitOutputLimitExceeded {
                operation,
                stream: "stderr",
                limit: MAX_GIT_STDERR_BYTES,
            });
        }
        Ok(Output {
            status,
            stdout: stdout.bytes,
            stderr: stderr.bytes,
        })
    }
}

struct BoundedOutput {
    bytes: Vec<u8>,
    exceeded: bool,
}

fn spawn_bounded_reader<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
) -> thread::JoinHandle<io::Result<BoundedOutput>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut exceeded = false;
        let mut buffer = [0_u8; 8192];
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                return Ok(BoundedOutput { bytes, exceeded });
            }
            let remaining = limit.saturating_sub(bytes.len());
            let retained = remaining.min(read);
            bytes.extend_from_slice(&buffer[..retained]);
            if retained != read {
                exceeded = true;
            }
        }
    })
}

fn join_bounded_reader(
    reader: thread::JoinHandle<io::Result<BoundedOutput>>,
    operation: &'static str,
    stream: &'static str,
) -> Result<BoundedOutput, ProductError> {
    reader
        .join()
        .map_err(|_| ProductError::ProcessCollectorPanicked(stream))?
        .map_err(|source| ProductError::Io {
            operation,
            path: PathBuf::from(stream),
            source,
        })
}

#[derive(Clone, Copy)]
enum DirtyScope {
    Source,
    Target,
    Builder,
}

#[derive(Clone, Copy)]
enum HeadRefScope {
    Source,
    Target,
}

/// Materialization-boundary failures are closed semantic outcomes; raw Git
/// output is reduced to exact digests rather than promoted as an opaque control
/// channel.
#[derive(Debug, Error)]
pub enum ProductError {
    #[error("invalid {kind}")]
    InvalidIdentity { kind: &'static str },
    #[error("unsupported Git object format `{0}`")]
    UnsupportedObjectFormat(String),
    #[error("invalid Git object identity `{0}`")]
    InvalidGitObjectId(String),
    #[error("invalid output digest `{value}`")]
    InvalidOutputDigest { value: String },
    #[error("invalid local delivery branch ref `{0}`")]
    InvalidLocalBranchRef(String),
    #[error("invalid product worktree branch `{0}`")]
    InvalidProductWorktreeBranch(String),
    #[error("Git path must be absolute: {0}")]
    GitPathNotAbsolute(PathBuf),
    #[error("Git executable is not a regular file: {0}")]
    GitNotRegular(PathBuf),
    #[error("validation program path must be absolute: {0}")]
    ProgramPathNotAbsolute(PathBuf),
    #[error("validation program is not a regular file: {0}")]
    ProgramNotRegular(PathBuf),
    #[error("shell validation program is denied: {0}")]
    ShellValidationProgramDenied(PathBuf),
    #[error("invalid validation-program argument")]
    InvalidValidationProgramArgument,
    #[error("externally supervised validation receipt is malformed")]
    InvalidExternallySupervisedValidationReceipt,
    #[error("validation profile must contain one to {MAX_VALIDATION_STEPS} commands")]
    InvalidValidationProfile,
    #[error("invalid commit identity")]
    InvalidCommitIdentity,
    #[error("invalid commit timestamp")]
    InvalidCommitTimestamp,
    #[error("invalid commit message")]
    InvalidCommitMessage,
    #[error("not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error(
        "source path is not the repository worktree root: supplied {supplied}, Git reports {reported}"
    )]
    RepositoryPathIsNotWorktreeRoot {
        supplied: PathBuf,
        reported: PathBuf,
    },
    #[error("bare repository is not a delivery target: {0}")]
    BareRepository(PathBuf),
    #[error("source worktree is not clean (status digest {status})")]
    SourceNotClean { status: OutputDigest },
    #[error("delivery target worktree is not clean (status digest {status})")]
    TargetNotClean { status: OutputDigest },
    #[error("builder product worktree is not clean (status digest {status})")]
    BuilderWorktreeNotClean { status: OutputDigest },
    #[error("source HEAD is not the requested local target ref")]
    SourceHeadNotTargetRef,
    #[error("delivery target HEAD is not the admitted local target ref")]
    TargetHeadNotTargetRef,
    #[error("source target moved from {expected} to {actual}")]
    SourceHeadMoved {
        expected: CommitId,
        actual: CommitId,
    },
    #[error("delivery target moved from {expected} to {actual}")]
    TargetHeadMoved {
        expected: CommitId,
        actual: CommitId,
    },
    #[error("builder made a commit or moved its worktree HEAD from {expected} to {actual}")]
    BuilderCommittedOrHeadMoved {
        expected: CommitId,
        actual: CommitId,
    },
    #[error("worktree root is not isolated from the source checkout: {0}")]
    WorktreeRootNotIsolated(PathBuf),
    #[error("patch-artifact root is not isolated from the source checkout: {0}")]
    PatchArtifactRootNotIsolated(PathBuf),
    #[error("managed path already exists: {0}")]
    ManagedPathAlreadyExists(PathBuf),
    #[error("path is not a direct child of its managed worktree root: {0}")]
    UnmanagedWorktreePath(PathBuf),
    #[error("candidate contains untracked files that a portable patch cannot carry")]
    UntrackedCandidateFiles(Vec<RepositoryPath>),
    #[error("candidate patch is empty")]
    EmptyPatch,
    #[error("candidate changed-path set is empty")]
    EmptyChangedPathSet,
    #[error("candidate temporary index already exists: {0}")]
    TemporaryIndexAlreadyExists(PathBuf),
    #[error("portable patch artifact is too large: {actual} bytes, limit {limit}")]
    PortablePatchTooLarge { actual: u64, limit: usize },
    #[error("Git emitted a non-UTF-8 path")]
    NonUtf8GitPath,
    #[error("invalid repository-relative Git path `{0}`")]
    InvalidRepositoryPath(String),
    #[error("Git did not emit a NUL-terminated path sequence while {0}")]
    MalformedNulDelimitedGitOutput(&'static str),
    #[error("Git emitted non-UTF-8 output while {0}")]
    NonUtf8GitOutput(&'static str),
    #[error("Git emitted an invalid single-line {0}")]
    InvalidSingleLineGitOutput(&'static str),
    #[error("accepted patch digest mismatch: expected {expected}, actual {actual}")]
    AcceptedPatchDigestMismatch {
        expected: PatchDigest,
        actual: PatchDigest,
    },
    #[error("delivery authorization does not match the captured candidate")]
    AuthorizationDoesNotMatchCapture,
    #[error("delivery authorization does not name the supplied validation profile")]
    AuthorizationDoesNotMatchValidationProfile,
    #[error("this profile requires externally supervised validation")]
    ExternallySupervisedValidationRequired,
    #[error("externally supervised validation receipt does not exactly match profile and tree")]
    ExternallySupervisedValidationReceiptMismatch,
    #[error("fresh materialization worktree started at {actual}, expected {expected}")]
    FreshWorktreeBaseMismatch {
        expected: CommitId,
        actual: CommitId,
    },
    #[error("freshly materialized tree mismatch: expected {expected}, actual {actual}")]
    MaterializedTreeMismatch { expected: TreeId, actual: TreeId },
    #[error("validation failed")]
    ValidationFailed(Box<ValidationFailureReceipt>),
    #[error("prepared materialization tree changed: expected {expected}, actual {actual}")]
    PreparedTreeChanged { expected: TreeId, actual: TreeId },
    #[error("prepared materialization contains untracked files")]
    PreparedWorktreeUntrackedFiles(Vec<RepositoryPath>),
    #[error("prepared materialization working copy differs from its index")]
    PreparedWorktreeModified {
        stdout: OutputDigest,
        stderr: OutputDigest,
    },
    #[error("materialization failed; cleanup evidence is {cleanup:?}")]
    MaterializationFailed {
        #[source]
        source: Box<ProductError>,
        cleanup: CleanupEvidence,
    },
    #[error("controlled commit tree mismatch: expected {expected}, actual {actual}")]
    ConstructedCommitTreeMismatch { expected: TreeId, actual: TreeId },
    #[error("controlled commit parent mismatch: expected {expected}, actual {actual:?}")]
    ConstructedCommitParentMismatch {
        expected: CommitId,
        actual: Vec<CommitId>,
    },
    #[error("malformed controlled commit parent output")]
    MalformedCommitParentOutput,
    #[error("delivery qualification does not match materialization authorization")]
    DeliveryAdmissionMismatch,
    #[error("delivery target repository does not match materialization authorization")]
    DeliveryRepositoryMismatch,
    #[error(
        "materialization receipt does not reconstruct one authorized tree, patch, validation, and commit"
    )]
    MaterializationReceiptCrossLinkMismatch,
    #[error("materialized commit tree mismatch at delivery: expected {expected}, actual {actual}")]
    DeliveryTreeMismatch { expected: TreeId, actual: TreeId },
    #[error(
        "materialized commit parent mismatch at delivery: expected {expected}, actual {actual:?}"
    )]
    DeliveryParentMismatch {
        expected: CommitId,
        actual: Vec<CommitId>,
    },
    #[error("delivered branch head mismatch: expected {expected}, actual {actual}")]
    DeliveredHeadMismatch {
        expected: CommitId,
        actual: CommitId,
    },
    #[error("delivery checkout is not an exact clean view of the delivered commit")]
    DeliveryCheckoutInconsistent,
    #[error(
        "delivery branch advanced to {delivered_commit}, but checkout synchronization did not complete; manual exact-tree recovery is required"
    )]
    DeliveryCheckoutRecoveryRequired {
        delivered_commit: CommitId,
        #[source]
        source: Box<ProductError>,
    },
    #[error("managed worktree removal was not verified: {0}")]
    WorktreeRemovalNotVerified(PathBuf),
    #[error("Git stdin was unexpectedly unavailable")]
    MissingGitStdin,
    #[error("Git stdout was unexpectedly unavailable")]
    MissingGitStdout,
    #[error("Git stderr was unexpectedly unavailable")]
    MissingGitStderr,
    #[error("Git operation `{operation}` exceeded its {stream} limit of {limit} bytes")]
    GitOutputLimitExceeded {
        operation: &'static str,
        stream: &'static str,
        limit: usize,
    },
    #[error("Git operation `{operation}` exceeded its deadline")]
    GitOperationTimedOut { operation: &'static str },
    #[error("process collector panicked: {0}")]
    ProcessCollectorPanicked(&'static str),
    #[error("repository config enables an executable surface: {key}")]
    RepositoryExecutableConfigDenied { key: String },
    #[error(
        "Git operation `{operation}` failed with status {status:?}; stdout={stdout}, stderr={stderr}"
    )]
    GitFailure {
        operation: &'static str,
        status: Option<i32>,
        stdout: OutputDigest,
        stderr: OutputDigest,
    },
    #[error("{operation} failed at {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Tests one final pathname entry without following it. A dangling symlink is
/// still an occupied entry: only a `NotFound` response means a crate-managed
/// pathname is absent and may be created or declared removed.
fn directory_entry_exists_no_follow(
    path: &Path,
    operation: &'static str,
) -> Result<bool, ProductError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(ProductError::Io {
            operation,
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn is_closed_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTITY_BYTES
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn is_derived_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_DERIVED_NAME_BYTES
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// Length-delimited rather than separator-delimited, so `a-b`/`c` and
/// `a`/`b-c` remain distinct branch/worktree/artifact identities.
fn derived_pair_name(change: &ProductChangeId, attempt: &BuilderAttemptId) -> String {
    format!(
        "{}-{}-{}-{}",
        change.as_str().len(),
        change.as_str(),
        attempt.as_str().len(),
        attempt.as_str()
    )
}

fn is_valid_ref_component_path(value: &str) -> bool {
    is_valid_ref_component_path_with_limit(value, MAX_IDENTITY_BYTES * 2)
}

fn is_valid_ref_component_path_with_limit(value: &str, length_limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= length_limit
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.ends_with('.')
        && !value.contains("..")
        && !value.contains("@{")
        && value.split('/').all(|component| {
            !component.is_empty()
                && component != "."
                && component != ".."
                && component.bytes().all(|byte| {
                    !byte.is_ascii_control()
                        && !matches!(byte, b' ' | b'~' | b'^' | b':' | b'?' | b'*' | b'[' | b'\\')
                })
        })
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use fmt::Write as _;
        let _ = write!(text, "{byte:02x}");
    }
    text
}

fn decode_exact_sha256(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 || !value.as_bytes().iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let mut bytes = [0_u8; 32];
    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return None;
    }
    for (index, pair) in pairs.iter().enumerate() {
        bytes[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Some(bytes)
}

fn is_executable_git_config_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.starts_with("filter.") || key.starts_with("include.path") || key.starts_with("includeif.")
}

fn single_line_utf8<'a>(
    bytes: &'a [u8],
    description: &'static str,
) -> Result<&'a str, ProductError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| ProductError::NonUtf8GitOutput(description))?;
    let Some(value) = text.strip_suffix('\n') else {
        return Err(ProductError::InvalidSingleLineGitOutput(description));
    };
    if value.is_empty() || value.contains(['\n', '\r', '\0']) {
        return Err(ProductError::InvalidSingleLineGitOutput(description));
    }
    Ok(value)
}

fn validation_digest(
    profile: &ValidationProfileId,
    tree: &TreeId,
    steps: &[ValidationStepReceipt],
) -> ValidationDigest {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(profile.as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(tree.to_hex().as_bytes());
    bytes.push(0);
    for step in steps {
        match &step.command {
            ValidationCommand::GitDiffCheck => bytes.extend_from_slice(b"git-diff-check"),
            ValidationCommand::ExternallySupervisedProgram(invocation) => {
                bytes.extend_from_slice(b"externally-supervised-program-v1");
                bytes.push(0);
                bytes.extend_from_slice(invocation.program.path().as_os_str().as_encoded_bytes());
                append_arguments(&mut bytes, invocation.arguments());
            }
        }
        bytes.push(0);
        bytes.extend_from_slice(step.stdout.to_hex().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(step.stderr.to_hex().as_bytes());
        bytes.push(0);
    }
    ValidationDigest::of_bytes(&bytes)
}

fn append_arguments(bytes: &mut Vec<u8>, arguments: &[ValidationProgramArgument]) {
    for argument in arguments {
        bytes.push(0);
        bytes.extend_from_slice(argument.as_str().as_bytes());
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod receipt_regressions {
    use super::*;

    fn sha1_commit(hex_byte: u8) -> CommitId {
        CommitId::parse(&format!("{hex_byte:02x}").repeat(20), GitObjectFormat::Sha1).unwrap()
    }

    fn sha1_tree(hex_byte: u8) -> TreeId {
        TreeId::parse(&format!("{hex_byte:02x}").repeat(20), GitObjectFormat::Sha1).unwrap()
    }

    fn receipt(
        authorization: ProductDeliveryAuthorization,
        tree: TreeId,
        patch: PatchDigest,
    ) -> MaterializationReceipt {
        let validation = ValidationReceipt {
            profile: authorization.validation_profile.clone(),
            tree: tree.clone(),
            steps: Vec::new(),
            digest: validation_digest(&authorization.validation_profile, &tree, &[]),
        };
        MaterializationReceipt {
            state: ProductState::DeliveryReady,
            materialized: MaterializedTreeReceipt {
                state: ProductState::Materialized,
                base: authorization.admitted_base.clone(),
                base_tree: sha1_tree(0x10),
                tree: tree.clone(),
                patch,
            },
            commit_validated: CommitValidatedReceipt {
                state: ProductState::CommitValidated,
                validation,
                commit: ControlledCommitReceipt {
                    commit: sha1_commit(if tree == sha1_tree(0x20) { 0x30 } else { 0x31 }),
                    parent: authorization.admitted_base.clone(),
                    tree,
                    message: CommitMessageDigest::of_bytes(b"message\n"),
                    hook_policy: HookPolicy::GitCommitTreeWithHooksDisabled,
                },
            },
            authorization,
            cleanup: CleanupEvidence::Removed {
                worktree_name: "test-materialization".to_owned(),
            },
        }
    }

    #[test]
    fn delivery_rejects_recombined_authorization_and_materialization_receipts() {
        let repository = SourceRepository {
            worktree_root: PathBuf::from("/receipt-regression/source"),
            git_common_dir: PathBuf::from("/receipt-regression/source/.git"),
            object_format: GitObjectFormat::Sha1,
        };
        let base = sha1_commit(0x01);
        let profile = ValidationProfileId::parse("receipt-cross-link-v1").unwrap();
        let authorization_a = ProductDeliveryAuthorization::new(
            DeliveryAuthorizationId::parse("authorization-a").unwrap(),
            ProductChangeId::parse("change-a").unwrap(),
            repository.clone(),
            LocalBranchRef::parse("refs/heads/main").unwrap(),
            base.clone(),
            PatchDigest::of_bytes(b"patch-a"),
            sha1_tree(0x20),
            profile.clone(),
        );
        let authorization_b = ProductDeliveryAuthorization::new(
            DeliveryAuthorizationId::parse("authorization-b").unwrap(),
            ProductChangeId::parse("change-b").unwrap(),
            repository,
            LocalBranchRef::parse("refs/heads/main").unwrap(),
            base,
            PatchDigest::of_bytes(b"patch-b"),
            sha1_tree(0x21),
            profile,
        );
        let receipt_a = receipt(
            authorization_a,
            sha1_tree(0x20),
            PatchDigest::of_bytes(b"patch-a"),
        );
        let mut receipt_b = receipt(
            authorization_b,
            sha1_tree(0x21),
            PatchDigest::of_bytes(b"patch-b"),
        );

        // This is the former public-field attack: same base, then replace B's
        // authorization with A while keeping B's tree/patch/commit body.
        receipt_b.authorization = receipt_a.authorization.clone();
        let materializer = ProductMaterializer {
            git: PathBuf::from("/receipt-regression/git"),
        };
        assert!(matches!(
            materializer.assert_materialization_cross_links(&receipt_b),
            Err(ProductError::MaterializationReceiptCrossLinkMismatch)
        ));
    }
}
