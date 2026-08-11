//! Resident-owned pinned Pi host/session launch material.
//!
//! A [`PinnedPiLaunchProfile`] is supervisor configuration, not an
//! application execution request.  It contains the host artifacts and the
//! closed session policy which the resident is willing to execute.  The only
//! function which turns it into a [`PiSpawnRequest`] is crate-private and
//! allocates a fresh workspace below the daemon-owned root.  In particular,
//! callers cannot select an executable, environment, session path, or
//! workspace path while composing a study.

use std::{
    fs::{self, OpenOptions},
    io,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
};

use society_kernel::Blake3Digest as RuntimeProfileDigest;
use society_pi::{
    AbsolutePath, ActorModelPolicyV1, Blake3Digest, CorrelationIdentity, CreateSessionPayload,
    ForumSessionContractV1, ModelCatalogPolicyV1, ModelSelection, SessionIdentity, SessionKind,
    SpawnNonce, ToolProfile, model_thinking_level_is_admitted,
};
use thiserror::Error;

use crate::supervision::{
    NativeHostEnvironment, NativeWorkspaceId, NativeWorkspaceRoot, PiSpawnRequest,
    QualifiedHostExecution, SupervisedChildId, SupervisionError,
};

/// The resident's fixed on-disk names for the host-owned session materials.
/// These names are not caller input and are always children of the freshly
/// allocated native workspace.
const AGENT_DIRECTORY_NAME: &str = "agent";
const SESSION_DIRECTORY_NAME: &str = "sessions";
const AUTH_FILE_NAME: &str = "auth.json";
const MODELS_FILE_NAME: &str = "models.json";
const MAX_SESSION_MATERIAL_BYTES: usize = 8 * 1024 * 1024;
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;

/// The one resident-owned session policy used by live TaskAttempt and native
/// qualification children.  `SessionKind` remains selected by the daemon's
/// typed owner path; an application cannot change the host policy or turn a
/// qualification child into an Office session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PinnedPiSessionProfile {
    system_prompt: String,
    system_prompt_digest: Blake3Digest,
    model: ModelSelection,
    model_catalog: ModelCatalogPolicyV1,
    settings: ActorModelPolicyV1,
    forum_contract: ForumSessionContractV1,
    auth_material: Vec<u8>,
    models_material: Vec<u8>,
}

impl PinnedPiSessionProfile {
    /// Constructs and validates the resident session policy.  The supplied
    /// bytes are copied into each fresh daemon-owned workspace; they are not
    /// interpreted as a generic payload or exposed through the local wire.
    pub fn new(
        system_prompt: impl Into<String>,
        model: ModelSelection,
        model_catalog: ModelCatalogPolicyV1,
        settings: ActorModelPolicyV1,
        forum_contract: ForumSessionContractV1,
        auth_material: Vec<u8>,
        models_material: Vec<u8>,
    ) -> Result<Self, PinnedPiProfileError> {
        let system_prompt = system_prompt.into();
        let system_prompt_digest = digest(system_prompt.as_bytes());
        let profile = Self {
            system_prompt,
            system_prompt_digest,
            model,
            model_catalog,
            settings,
            forum_contract,
            auth_material,
            models_material,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn system_prompt_digest(&self) -> &Blake3Digest {
        &self.system_prompt_digest
    }

    pub fn model(&self) -> &ModelSelection {
        &self.model
    }

    pub fn model_catalog(&self) -> &ModelCatalogPolicyV1 {
        &self.model_catalog
    }

    pub fn assert_valid(&self) -> Result<(), PinnedPiProfileError> {
        self.validate()
    }

    fn validate(&self) -> Result<(), PinnedPiProfileError> {
        if self.system_prompt.is_empty()
            || self.system_prompt.len() > MAX_SESSION_MATERIAL_BYTES
            || self.auth_material.is_empty()
            || self.models_material.is_empty()
            || self.auth_material.len() > MAX_SESSION_MATERIAL_BYTES
            || self.models_material.len() > MAX_SESSION_MATERIAL_BYTES
        {
            return Err(PinnedPiProfileError::InvalidSessionMaterial);
        }
        if self.system_prompt_digest != digest(self.system_prompt.as_bytes()) {
            return Err(PinnedPiProfileError::SystemPromptDigestDrift);
        }
        self.model_catalog
            .assert_pinned()
            .map_err(PinnedPiProfileError::Protocol)?;
        self.settings
            .assert_pinned()
            .map_err(PinnedPiProfileError::Protocol)?;
        self.forum_contract
            .assert_pinned()
            .map_err(PinnedPiProfileError::Protocol)?;
        if self.model.provider != self.model_catalog.effective_model.provider
            || self.model.model_id != self.model_catalog.effective_model.model_id
            || !model_thinking_level_is_admitted(self.model.model_id, self.model.thinking_level)
        {
            return Err(PinnedPiProfileError::ModelPolicyMismatch);
        }
        if digest(&self.models_material) != self.model_catalog.catalog_blake3 {
            return Err(PinnedPiProfileError::ModelCatalogDigestMismatch);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn materialize(
        &self,
        workspace_root: &NativeWorkspaceRoot,
        workspace_identity: NativeWorkspaceId,
        child_process_id: SupervisedChildId,
        session_identity: SessionIdentity,
        spawn_nonce: SpawnNonce,
        create_correlation_identity: CorrelationIdentity,
        session_kind: SessionKind,
        host_execution: QualifiedHostExecution,
    ) -> Result<PiSpawnRequest, PinnedPiProfileError> {
        self.validate()?;
        let workspace = workspace_root.allocate(workspace_identity)?;
        let agent_directory =
            create_private_directory(workspace.directory().as_path(), AGENT_DIRECTORY_NAME)?;
        let session_directory =
            create_private_directory(workspace.directory().as_path(), SESSION_DIRECTORY_NAME)?;
        let auth_path =
            write_private_material(&agent_directory, AUTH_FILE_NAME, &self.auth_material)?;
        let models_path =
            write_private_material(&agent_directory, MODELS_FILE_NAME, &self.models_material)?;
        let cwd = workspace.directory().clone();
        let agent_directory = absolute_path(&agent_directory)?;
        let auth_path = absolute_path(&auth_path)?;
        let models_path = absolute_path(&models_path)?;
        let session_directory = absolute_path(&session_directory)?;
        let create_session = CreateSessionPayload {
            session_kind,
            cwd,
            agent_directory,
            auth_path,
            models_path,
            session_directory,
            system_prompt: self.system_prompt.clone(),
            system_prompt_digest: self.system_prompt_digest.clone(),
            model: self.model.clone(),
            model_catalog: self.model_catalog.clone(),
            tool_profile: ToolProfile::ForumIsolatedV1,
            settings: self.settings.clone(),
            forum_contract: self.forum_contract.clone(),
        };
        Ok(PiSpawnRequest {
            child_process_id,
            workspace,
            session_identity,
            spawn_nonce,
            host_execution,
            environment: NativeHostEnvironment::EmptyV1,
            create_correlation_identity,
            create_session,
        })
    }
}

/// Supervisor-owned native host and session configuration.  Host artifact
/// paths/digests are verified when this value is installed and rechecked by
/// the existing supervisor immediately before `exec`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PinnedPiLaunchProfile {
    host_execution: QualifiedHostExecution,
    session: PinnedPiSessionProfile,
    /// Opaque digest of the complete study runtime profile selected before
    /// outcomes are visible. It carries no credentials or application bytes,
    /// but lets a study runner reject a qualified host/session profile which
    /// does not match its sealed policy/runtime contract.
    runtime_profile_digest: RuntimeProfileDigest,
}

impl PinnedPiLaunchProfile {
    pub fn new(
        host_execution: QualifiedHostExecution,
        session: PinnedPiSessionProfile,
        runtime_profile_digest: RuntimeProfileDigest,
    ) -> Result<Self, PinnedPiProfileError> {
        host_execution
            .verify_before_spawn()
            .map_err(PinnedPiProfileError::Supervision)?;
        session.assert_valid()?;
        Ok(Self {
            host_execution,
            session,
            runtime_profile_digest,
        })
    }

    pub fn host_execution(&self) -> &QualifiedHostExecution {
        &self.host_execution
    }

    pub fn session(&self) -> &PinnedPiSessionProfile {
        &self.session
    }

    /// The exact sealed study-runtime profile that this supervisor-installed
    /// host/session configuration may execute. This is an opaque identity,
    /// not a route to host paths, auth material, or model-catalog bytes.
    pub const fn runtime_profile_digest(&self) -> RuntimeProfileDigest {
        self.runtime_profile_digest
    }

    pub fn assert_valid(&self) -> Result<(), PinnedPiProfileError> {
        self.host_execution
            .verify_before_spawn()
            .map_err(PinnedPiProfileError::Supervision)?;
        self.session.assert_valid()
    }

    pub(crate) fn materialize_task_attempt(
        &self,
        workspace_root: &NativeWorkspaceRoot,
        workspace_identity: NativeWorkspaceId,
        child_process_id: SupervisedChildId,
        session_identity: SessionIdentity,
        spawn_nonce: SpawnNonce,
        create_correlation_identity: CorrelationIdentity,
    ) -> Result<PiSpawnRequest, PinnedPiProfileError> {
        let mut request = self.session.materialize(
            workspace_root,
            workspace_identity,
            child_process_id,
            session_identity,
            spawn_nonce,
            create_correlation_identity,
            SessionKind::TaskAttempt,
            self.host_execution.clone(),
        )?;
        request.host_execution = self.host_execution.clone();
        Ok(request)
    }

    pub(crate) fn materialize_native_profile_qualification(
        &self,
        workspace_root: &NativeWorkspaceRoot,
        workspace_identity: NativeWorkspaceId,
        child_process_id: SupervisedChildId,
        session_identity: SessionIdentity,
        spawn_nonce: SpawnNonce,
        create_correlation_identity: CorrelationIdentity,
    ) -> Result<PiSpawnRequest, PinnedPiProfileError> {
        let mut request = self.session.materialize(
            workspace_root,
            workspace_identity,
            child_process_id,
            session_identity,
            spawn_nonce,
            create_correlation_identity,
            SessionKind::TaskAttempt,
            self.host_execution.clone(),
        )?;
        request.host_execution = self.host_execution.clone();
        Ok(request)
    }
}

/// Errors are deliberately typed at the resident configuration boundary.  No
/// provider secret, path, or process error is returned through the public
/// supervisor protocol.
#[derive(Debug, Error)]
pub enum PinnedPiProfileError {
    #[error("the daemon has no supervisor-installed pinned Pi launch profile")]
    NotConfigured,
    #[error("the sealed study runtime profile does not match the supervisor-installed Pi profile")]
    RuntimeProfileMismatch,
    #[error("pinned host/session profile contains invalid session material")]
    InvalidSessionMaterial,
    #[error("pinned host/session profile system prompt digest drifted")]
    SystemPromptDigestDrift,
    #[error("pinned host/session profile model policy does not match its catalog")]
    ModelPolicyMismatch,
    #[error("pinned host/session profile model catalog bytes do not match its digest")]
    ModelCatalogDigestMismatch,
    #[error("pinned host/session profile violates the closed Pi protocol: {0}")]
    Protocol(#[source] society_pi::ProtocolError),
    #[error("pinned host/session profile failed native validation: {0}")]
    Supervision(#[source] SupervisionError),
    #[error("pinned host/session profile filesystem operation failed: {0}")]
    Io(#[source] io::Error),
    #[error("kernel launch claim workspace path is outside the resident workspace root")]
    WorkspacePathMismatch,
    #[error("resident launch operation identity is not canonical")]
    InvalidOperationIdentity,
}

impl From<SupervisionError> for PinnedPiProfileError {
    fn from(value: SupervisionError) -> Self {
        Self::Supervision(value)
    }
}

fn absolute_path(path: &Path) -> Result<AbsolutePath, PinnedPiProfileError> {
    AbsolutePath::parse(
        path.to_str()
            .ok_or(PinnedPiProfileError::InvalidSessionMaterial)?,
    )
    .map_err(PinnedPiProfileError::Protocol)
}

fn digest(bytes: &[u8]) -> Blake3Digest {
    // The Pi boundary deliberately exposes only a parser for digest strings;
    // the resident computes the canonical hex representation at this private
    // configuration boundary.
    Blake3Digest::parse(blake3::hash(bytes).to_hex().to_string())
        .expect("BLAKE3 hex output is always a valid Pi digest")
}

fn create_private_directory(
    workspace: &Path,
    name: &str,
) -> Result<std::path::PathBuf, PinnedPiProfileError> {
    let path = workspace.join(name);
    fs::create_dir(&path).map_err(PinnedPiProfileError::Io)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE))
        .map_err(PinnedPiProfileError::Io)?;
    let metadata = fs::metadata(&path).map_err(PinnedPiProfileError::Io)?;
    if !metadata.is_dir() || metadata.permissions().mode() & 0o777 != PRIVATE_DIRECTORY_MODE {
        return Err(PinnedPiProfileError::InvalidSessionMaterial);
    }
    Ok(path)
}

fn write_private_material(
    directory: &Path,
    name: &str,
    bytes: &[u8],
) -> Result<std::path::PathBuf, PinnedPiProfileError> {
    let path = directory.join(name);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).mode(PRIVATE_FILE_MODE);
    let mut file = options.open(&path).map_err(PinnedPiProfileError::Io)?;
    std::io::Write::write_all(&mut file, bytes).map_err(PinnedPiProfileError::Io)?;
    file.sync_all().map_err(PinnedPiProfileError::Io)?;
    let metadata = fs::metadata(&path).map_err(PinnedPiProfileError::Io)?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o777 != PRIVATE_FILE_MODE {
        return Err(PinnedPiProfileError::InvalidSessionMaterial);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn material_files_are_created_once_with_daemon_private_modes() {
        let root = std::env::temp_dir().join(format!(
            "societyd-launch-profile-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock is after the Unix epoch")
                .as_nanos()
        ));
        fs::create_dir(&root).expect("test root creation");
        fs::set_permissions(&root, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE))
            .expect("test root mode");
        let agent = create_private_directory(&root, AGENT_DIRECTORY_NAME)
            .expect("agent directory must be resident-private");
        let material = write_private_material(&agent, AUTH_FILE_NAME, b"{}")
            .expect("auth material must be resident-private");
        assert_eq!(
            fs::metadata(&material)
                .expect("resident-private material metadata")
                .permissions()
                .mode()
                & 0o777,
            PRIVATE_FILE_MODE
        );
        assert!(write_private_material(&agent, AUTH_FILE_NAME, b"{}").is_err());
        fs::remove_dir_all(root).expect("test root cleanup");
    }
}
