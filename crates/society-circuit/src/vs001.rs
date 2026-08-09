//! Closed decoders for the remaining deterministic VS-001 TSV boundaries.
//!
//! Every type in this module describes only evaluator-emitted or actor-emitted
//! bytes.  In particular, successful parsing does not admit evidence, bind an
//! actor, establish independence, attest process reaping, or grant disclosure
//! authority.  Those decisions belong to later named kernel commands.

use society_content::ContentDigest;
use thiserror::Error;

const MAX_VS001_TABLE_BYTES: usize = 128 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vs001Schema {
    InputDigestManifest,
    DocumentationObservation,
    DocumentationConflict,
    FluencyProbeObservation,
    FluencyExecutionSurface,
    FluencyExecutionEnvelope,
    CurationFrontierMembers,
    CurationAccount,
    CurationSelectedItems,
    CurationPreservedConflicts,
    CurationUnknowns,
    CurationExclusions,
    CurationEscalations,
    CurationContractObservation,
    CurationEscalationObservation,
    UptakeDeliveryContext,
    UptakePersistedInput,
    UptakeInvestigatorSubmission,
    UptakeAccesses,
    PropagationObservation,
    FrontierMembers,
    FrontierSequestered,
    FrontierAccessObservation,
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum Vs001ParseError {
    #[error("{schema:?} exceeds the VS-001 byte bound")]
    FrameTooLarge { schema: Vs001Schema },
    #[error("{schema:?} is not UTF-8")]
    InvalidUtf8 { schema: Vs001Schema },
    #[error("{schema:?} must use LF line endings")]
    NonCanonicalLineEnding { schema: Vs001Schema },
    #[error("{schema:?} must end in one LF-terminated record")]
    MissingTerminalLf { schema: Vs001Schema },
    #[error("{schema:?} schema line is not exact")]
    WrongSchema { schema: Vs001Schema },
    #[error("{schema:?} header line is not exact")]
    WrongHeader { schema: Vs001Schema },
    #[error("{schema:?} has {actual} data rows; expected {expected}")]
    WrongRowCount {
        schema: Vs001Schema,
        expected: usize,
        actual: usize,
    },
    #[error("{schema:?} row {line} has {actual} fields; expected {expected}")]
    WrongFieldCount {
        schema: Vs001Schema,
        line: usize,
        expected: usize,
        actual: usize,
    },
    #[error("{schema:?} row {line} has an unknown or invalid value in {column}")]
    InvalidValue {
        schema: Vs001Schema,
        line: usize,
        column: &'static str,
    },
    #[error("{schema:?} row {line} has a noncanonical content digest in {column}")]
    InvalidDigest {
        schema: Vs001Schema,
        line: usize,
        column: &'static str,
    },
    #[error("{schema:?} row {line} repeats {identity}")]
    DuplicateIdentity {
        schema: Vs001Schema,
        line: usize,
        identity: &'static str,
    },
    #[error("{schema:?} row {line} is not in its closed expected position")]
    OutOfOrder { schema: Vs001Schema, line: usize },
    #[error("{schema:?} row {line} recombines fields from distinct closed rows")]
    RecombinedRow { schema: Vs001Schema, line: usize },
}

struct Table<'a> {
    rows: Vec<&'a str>,
}

fn table<'a>(
    bytes: &'a [u8],
    schema: Vs001Schema,
    expected_schema: &str,
    expected_header: &str,
) -> Result<Table<'a>, Vs001ParseError> {
    if bytes.len() > MAX_VS001_TABLE_BYTES {
        return Err(Vs001ParseError::FrameTooLarge { schema });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Vs001ParseError::InvalidUtf8 { schema })?;
    if text.contains('\r') {
        return Err(Vs001ParseError::NonCanonicalLineEnding { schema });
    }
    if !text.ends_with('\n') {
        return Err(Vs001ParseError::MissingTerminalLf { schema });
    }
    let mut lines = text[..text.len() - 1].split('\n');
    if lines.next() != Some(expected_schema) {
        return Err(Vs001ParseError::WrongSchema { schema });
    }
    if lines.next() != Some(expected_header) {
        return Err(Vs001ParseError::WrongHeader { schema });
    }
    Ok(Table {
        rows: lines.collect(),
    })
}

/// Parse a closed fixture input relation. Unlike evaluator output, these
/// relations deliberately have no schema line: the typed caller and relation
/// file select the schema. Canonical framing still prevents reinterpretation.
fn relation<'a>(
    bytes: &'a [u8],
    schema: Vs001Schema,
    expected_header: &str,
) -> Result<Table<'a>, Vs001ParseError> {
    if bytes.len() > MAX_VS001_TABLE_BYTES {
        return Err(Vs001ParseError::FrameTooLarge { schema });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Vs001ParseError::InvalidUtf8 { schema })?;
    if text.contains('\r') {
        return Err(Vs001ParseError::NonCanonicalLineEnding { schema });
    }
    if !text.ends_with('\n') {
        return Err(Vs001ParseError::MissingTerminalLf { schema });
    }
    let mut lines = text[..text.len() - 1].split('\n');
    if lines.next() != Some(expected_header) {
        return Err(Vs001ParseError::WrongHeader { schema });
    }
    Ok(Table {
        rows: lines.collect(),
    })
}

fn row(
    line: &str,
    line_number: usize,
    schema: Vs001Schema,
    expected_fields: usize,
) -> Result<Vec<&str>, Vs001ParseError> {
    let fields: Vec<_> = line.split('\t').collect();
    if fields.len() != expected_fields {
        return Err(Vs001ParseError::WrongFieldCount {
            schema,
            line: line_number,
            expected: expected_fields,
            actual: fields.len(),
        });
    }
    Ok(fields)
}

fn digest(
    value: &str,
    schema: Vs001Schema,
    line: usize,
    column: &'static str,
) -> Result<ContentDigest, Vs001ParseError> {
    ContentDigest::parse(value).map_err(|_| Vs001ParseError::InvalidDigest {
        schema,
        line,
        column,
    })
}

fn exact_rows(
    table: &Table<'_>,
    schema: Vs001Schema,
    expected: usize,
) -> Result<(), Vs001ParseError> {
    if table.rows.len() != expected {
        return Err(Vs001ParseError::WrongRowCount {
            schema,
            expected,
            actual: table.rows.len(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Input-digest manifests

const INPUT_HEADER: &str = "input_kind\tsha256";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputDigestProducer {
    BehaviorMatrix,
    DocumentationMatrix,
    FluencyProbe,
    CurationContract,
    UptakeApplication,
    FrontierLeakage,
}

impl InputDigestProducer {
    const fn schema(self) -> &'static str {
        match self {
            // Both producers intentionally use the same evaluator-output
            // schema. The typed caller is therefore required to name one.
            Self::BehaviorMatrix | Self::DocumentationMatrix => {
                "# schema: CircuitInputDigestV1/tsv-v1"
            }
            Self::FluencyProbe => "# schema: FluencyProbeInputDigestV1/tsv-v1",
            Self::CurationContract => "# schema: CurationContractInputDigestV1/tsv-v1",
            Self::UptakeApplication => "# schema: UptakeApplicationInputDigestV1/tsv-v1",
            Self::FrontierLeakage => "# schema: FrontierLeakageInputDigestV1/tsv-v1",
        }
    }

    fn expected_kinds(self) -> &'static [InputDigestKind] {
        match self {
            Self::BehaviorMatrix => &[
                InputDigestKind::XshBinary,
                InputDigestKind::XshtBinary,
                InputDigestKind::BehaviorEvaluator,
                InputDigestKind::FixtureNoisyChild,
                InputDigestKind::FixtureNoisySleeper,
                InputDigestKind::FixtureNoisyChildNonzero,
                InputDigestKind::FixtureProbeProcessRun,
                InputDigestKind::FixtureProbeOwnedSpawn,
                InputDigestKind::FixtureProbeSpawnRun,
                InputDigestKind::FixtureProbeDetachedSpawn,
                InputDigestKind::FixtureProbeOwnedDefault,
                InputDigestKind::FixtureProbeOwnedInvalidStderr,
                InputDigestKind::FixtureProbeOwnedNonzero,
                InputDigestKind::FixtureProbeOwnedCancel,
                InputDigestKind::FixtureProbeProcessRunNull,
            ],
            Self::DocumentationMatrix => &[
                InputDigestKind::XshtBinary,
                InputDigestKind::DocumentationEvaluator,
                InputDigestKind::LangSource,
                InputDigestKind::SpecSource,
                InputDigestKind::SpecOsSource,
                InputDigestKind::RuntimeProcessSource,
                InputDigestKind::LoweredRuntimeSource,
                InputDigestKind::NativeProcessTest,
                InputDigestKind::ApiProcessCommandArgv,
                InputDigestKind::ApiProcessSpawn,
                InputDigestKind::ApiProcessNavigation,
            ],
            Self::FluencyProbe => &[
                InputDigestKind::XshBinary,
                InputDigestKind::XshtBinary,
                InputDigestKind::TaskActorEvaluator,
                InputDigestKind::TaskInstruction,
                InputDigestKind::ReferencePack,
                InputDigestKind::TaskSolution,
                InputDigestKind::TaskSubmission,
                InputDigestKind::TaskToolEvents,
                InputDigestKind::ChildAlpha,
                InputDigestKind::ChildSpaces,
                InputDigestKind::ChildNonzero,
            ],
            Self::CurationContract => &[
                InputDigestKind::CurationContractJudge,
                InputDigestKind::FrontierMembers,
                InputDigestKind::AccountRelation,
                InputDigestKind::SelectedItemsRelation,
                InputDigestKind::ConflictsRelation,
                InputDigestKind::UnknownsRelation,
                InputDigestKind::ExclusionsRelation,
                InputDigestKind::EscalationsRelation,
            ],
            Self::UptakeApplication => &[
                InputDigestKind::UptakeApplicationJudge,
                InputDigestKind::DeliveryContext,
                InputDigestKind::PersistedInput,
                InputDigestKind::InvestigatorSubmission,
                InputDigestKind::InvestigatorAccesses,
            ],
            Self::FrontierLeakage => &[
                InputDigestKind::FrontierLeakageJudge,
                InputDigestKind::FrontierMembers,
                InputDigestKind::SequesteredRelations,
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputDigestKind {
    XshBinary,
    XshtBinary,
    BehaviorEvaluator,
    FixtureNoisyChild,
    FixtureNoisySleeper,
    FixtureNoisyChildNonzero,
    FixtureProbeProcessRun,
    FixtureProbeOwnedSpawn,
    FixtureProbeSpawnRun,
    FixtureProbeDetachedSpawn,
    FixtureProbeOwnedDefault,
    FixtureProbeOwnedInvalidStderr,
    FixtureProbeOwnedNonzero,
    FixtureProbeOwnedCancel,
    FixtureProbeProcessRunNull,
    DocumentationEvaluator,
    LangSource,
    SpecSource,
    SpecOsSource,
    RuntimeProcessSource,
    LoweredRuntimeSource,
    NativeProcessTest,
    ApiProcessCommandArgv,
    ApiProcessSpawn,
    ApiProcessNavigation,
    TaskActorEvaluator,
    TaskInstruction,
    ReferencePack,
    TaskSolution,
    TaskSubmission,
    TaskToolEvents,
    ChildAlpha,
    ChildSpaces,
    ChildNonzero,
    CurationContractJudge,
    FrontierMembers,
    AccountRelation,
    SelectedItemsRelation,
    ConflictsRelation,
    UnknownsRelation,
    ExclusionsRelation,
    EscalationsRelation,
    UptakeApplicationJudge,
    DeliveryContext,
    PersistedInput,
    InvestigatorSubmission,
    InvestigatorAccesses,
    FrontierLeakageJudge,
    SequesteredRelations,
}

impl InputDigestKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "xsh_binary" => Some(Self::XshBinary),
            "xsht_binary" => Some(Self::XshtBinary),
            "behavior_evaluator" => Some(Self::BehaviorEvaluator),
            "fixture_noisy_child" => Some(Self::FixtureNoisyChild),
            "fixture_noisy_sleeper" => Some(Self::FixtureNoisySleeper),
            "fixture_noisy_child_nonzero" => Some(Self::FixtureNoisyChildNonzero),
            "fixture_probe_process_run" => Some(Self::FixtureProbeProcessRun),
            "fixture_probe_owned_spawn" => Some(Self::FixtureProbeOwnedSpawn),
            "fixture_probe_spawn_run" => Some(Self::FixtureProbeSpawnRun),
            "fixture_probe_detached_spawn" => Some(Self::FixtureProbeDetachedSpawn),
            "fixture_probe_owned_default" => Some(Self::FixtureProbeOwnedDefault),
            "fixture_probe_owned_invalid_stderr" => Some(Self::FixtureProbeOwnedInvalidStderr),
            "fixture_probe_owned_nonzero" => Some(Self::FixtureProbeOwnedNonzero),
            "fixture_probe_owned_cancel" => Some(Self::FixtureProbeOwnedCancel),
            "fixture_probe_process_run_null" => Some(Self::FixtureProbeProcessRunNull),
            "documentation_evaluator" => Some(Self::DocumentationEvaluator),
            "lang_source" => Some(Self::LangSource),
            "spec_source" => Some(Self::SpecSource),
            "spec_os_source" => Some(Self::SpecOsSource),
            "runtime_process_source" => Some(Self::RuntimeProcessSource),
            "lowered_runtime_source" => Some(Self::LoweredRuntimeSource),
            "native_process_test" => Some(Self::NativeProcessTest),
            "api_process_command_argv" => Some(Self::ApiProcessCommandArgv),
            "api_process_spawn" => Some(Self::ApiProcessSpawn),
            "api_process_navigation" => Some(Self::ApiProcessNavigation),
            "task_actor_evaluator" => Some(Self::TaskActorEvaluator),
            "task_instruction" => Some(Self::TaskInstruction),
            "reference_pack" => Some(Self::ReferencePack),
            "task_solution" => Some(Self::TaskSolution),
            "task_submission" => Some(Self::TaskSubmission),
            "task_tool_events" => Some(Self::TaskToolEvents),
            "child_alpha" => Some(Self::ChildAlpha),
            "child_spaces" => Some(Self::ChildSpaces),
            "child_nonzero" => Some(Self::ChildNonzero),
            "curation_contract_judge" => Some(Self::CurationContractJudge),
            "frontier_members" => Some(Self::FrontierMembers),
            "account_relation" => Some(Self::AccountRelation),
            "selected_items_relation" => Some(Self::SelectedItemsRelation),
            "conflicts_relation" => Some(Self::ConflictsRelation),
            "unknowns_relation" => Some(Self::UnknownsRelation),
            "exclusions_relation" => Some(Self::ExclusionsRelation),
            "escalations_relation" => Some(Self::EscalationsRelation),
            "uptake_application_judge" => Some(Self::UptakeApplicationJudge),
            "delivery_context" => Some(Self::DeliveryContext),
            "persisted_input" => Some(Self::PersistedInput),
            "investigator_submission" => Some(Self::InvestigatorSubmission),
            "investigator_accesses" => Some(Self::InvestigatorAccesses),
            "frontier_leakage_judge" => Some(Self::FrontierLeakageJudge),
            "sequestered_relations" => Some(Self::SequesteredRelations),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::XshBinary => "xsh_binary",
            Self::XshtBinary => "xsht_binary",
            Self::BehaviorEvaluator => "behavior_evaluator",
            Self::FixtureNoisyChild => "fixture_noisy_child",
            Self::FixtureNoisySleeper => "fixture_noisy_sleeper",
            Self::FixtureNoisyChildNonzero => "fixture_noisy_child_nonzero",
            Self::FixtureProbeProcessRun => "fixture_probe_process_run",
            Self::FixtureProbeOwnedSpawn => "fixture_probe_owned_spawn",
            Self::FixtureProbeSpawnRun => "fixture_probe_spawn_run",
            Self::FixtureProbeDetachedSpawn => "fixture_probe_detached_spawn",
            Self::FixtureProbeOwnedDefault => "fixture_probe_owned_default",
            Self::FixtureProbeOwnedInvalidStderr => "fixture_probe_owned_invalid_stderr",
            Self::FixtureProbeOwnedNonzero => "fixture_probe_owned_nonzero",
            Self::FixtureProbeOwnedCancel => "fixture_probe_owned_cancel",
            Self::FixtureProbeProcessRunNull => "fixture_probe_process_run_null",
            Self::DocumentationEvaluator => "documentation_evaluator",
            Self::LangSource => "lang_source",
            Self::SpecSource => "spec_source",
            Self::SpecOsSource => "spec_os_source",
            Self::RuntimeProcessSource => "runtime_process_source",
            Self::LoweredRuntimeSource => "lowered_runtime_source",
            Self::NativeProcessTest => "native_process_test",
            Self::ApiProcessCommandArgv => "api_process_command_argv",
            Self::ApiProcessSpawn => "api_process_spawn",
            Self::ApiProcessNavigation => "api_process_navigation",
            Self::TaskActorEvaluator => "task_actor_evaluator",
            Self::TaskInstruction => "task_instruction",
            Self::ReferencePack => "reference_pack",
            Self::TaskSolution => "task_solution",
            Self::TaskSubmission => "task_submission",
            Self::TaskToolEvents => "task_tool_events",
            Self::ChildAlpha => "child_alpha",
            Self::ChildSpaces => "child_spaces",
            Self::ChildNonzero => "child_nonzero",
            Self::CurationContractJudge => "curation_contract_judge",
            Self::FrontierMembers => "frontier_members",
            Self::AccountRelation => "account_relation",
            Self::SelectedItemsRelation => "selected_items_relation",
            Self::ConflictsRelation => "conflicts_relation",
            Self::UnknownsRelation => "unknowns_relation",
            Self::ExclusionsRelation => "exclusions_relation",
            Self::EscalationsRelation => "escalations_relation",
            Self::UptakeApplicationJudge => "uptake_application_judge",
            Self::DeliveryContext => "delivery_context",
            Self::PersistedInput => "persisted_input",
            Self::InvestigatorSubmission => "investigator_submission",
            Self::InvestigatorAccesses => "investigator_accesses",
            Self::FrontierLeakageJudge => "frontier_leakage_judge",
            Self::SequesteredRelations => "sequestered_relations",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputDigestEntryV1 {
    pub kind: InputDigestKind,
    pub digest: ContentDigest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputDigestManifestV1 {
    pub producer: InputDigestProducer,
    entries: Vec<InputDigestEntryV1>,
}

impl InputDigestManifestV1 {
    /// `CircuitInputDigestV1` is shared by B01--B11 and the documentation
    /// evaluator. A caller must name which producer it received, rather than
    /// allowing common schema text to accidentally cross an evaluator boundary.
    pub fn parse(producer: InputDigestProducer, bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let parsed = table(
            bytes,
            Vs001Schema::InputDigestManifest,
            producer.schema(),
            INPUT_HEADER,
        )?;
        let rows = parsed.rows;
        let expected = producer.expected_kinds();
        if rows.len() != expected.len() {
            return Err(Vs001ParseError::WrongRowCount {
                schema: Vs001Schema::InputDigestManifest,
                expected: expected.len(),
                actual: rows.len(),
            });
        }
        let mut entries = Vec::with_capacity(expected.len());
        for (index, (line, expected_kind)) in rows.iter().zip(expected).enumerate() {
            let line_number = index + 3;
            let fields = row(line, line_number, Vs001Schema::InputDigestManifest, 2)?;
            let observed =
                InputDigestKind::parse(fields[0]).ok_or(Vs001ParseError::InvalidValue {
                    schema: Vs001Schema::InputDigestManifest,
                    line: line_number,
                    column: "input_kind",
                })?;
            if observed != *expected_kind {
                return Err(Vs001ParseError::OutOfOrder {
                    schema: Vs001Schema::InputDigestManifest,
                    line: line_number,
                });
            }
            entries.push(InputDigestEntryV1 {
                kind: observed,
                digest: digest(
                    fields[1],
                    Vs001Schema::InputDigestManifest,
                    line_number,
                    "sha256",
                )?,
            });
        }
        Ok(Self { producer, entries })
    }

    pub fn entries(&self) -> &[InputDigestEntryV1] {
        &self.entries
    }
}

// ---------------------------------------------------------------------------
// Documentation/discovery observations

const DOCUMENTATION_SCHEMA: &str = "# schema: DocumentationObservationV1/tsv-v1";
const DOCUMENTATION_HEADER: &str = "source\tconsumer\tfield\tclaim\tcitation";
const DOCUMENTATION_CONFLICT_SCHEMA: &str = "# schema: DocumentationConflictV1/tsv-v1";
const DOCUMENTATION_CONFLICT_HEADER: &str = "conflict_id\tleft_claim\tright_claim\tstatus";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentationSource {
    LangMd,
    SpecSpawn,
    SpecApi,
    SpecOs,
    XshtApi,
    XshtNavigation,
    Runtime,
    LoweredRuntime,
    NativeTest,
}

impl DocumentationSource {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "LANG_md" => Some(Self::LangMd),
            "SPEC_spawn" => Some(Self::SpecSpawn),
            "SPEC_api" => Some(Self::SpecApi),
            "SPEC_OS" => Some(Self::SpecOs),
            "xsht_api" => Some(Self::XshtApi),
            "xsht_navigation" => Some(Self::XshtNavigation),
            "runtime" => Some(Self::Runtime),
            "lowered_runtime" => Some(Self::LoweredRuntime),
            "native_test" => Some(Self::NativeTest),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentationConsumer {
    SpawnCommand,
    CommandPlan,
    ProcessSpawn,
    ManagedSpawn,
    ProcessRun,
}

impl DocumentationConsumer {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "spawn_command" => Some(Self::SpawnCommand),
            "command_plan" => Some(Self::CommandPlan),
            "process_spawn" => Some(Self::ProcessSpawn),
            "managed_spawn" => Some(Self::ManagedSpawn),
            "process_run" => Some(Self::ProcessRun),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentationField {
    Stderr,
    Default,
    StderrAppend,
    Error,
    Lifecycle,
    Ownership,
    Discovery,
    CallPath,
}

impl DocumentationField {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "stderr" => Some(Self::Stderr),
            "default" => Some(Self::Default),
            "stderr_append" => Some(Self::StderrAppend),
            "error" => Some(Self::Error),
            "lifecycle" => Some(Self::Lifecycle),
            "ownership" => Some(Self::Ownership),
            "discovery" => Some(Self::Discovery),
            "call_path" => Some(Self::CallPath),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentationClaim {
    ClaimsMissing,
    DoesNotClaimMissing,
    ClaimsUsesCommandRedirections,
    ClaimsInheritDefault,
    ClaimsTypedPathField,
    ClaimsTypedAppendField,
    ClaimsSetupFailureIsProcessError,
    ClaimsDetachedRecord,
    ClaimsOwnedChildGroup,
    ClaimsRedirectionFailureDistinctFromStatus,
    DiscoverableTypedPathField,
    DiscoverableTypedAppendField,
    ClaimsOwnedHandle,
    DoesNotDiscloseLifecycle,
    FindsCommandArgv,
    FindsProcessSpawn,
    SpawnCommandEntersDetachedOptions,
    DisablesCommandRedirections,
    ConditionallyAppliesCommandRedirections,
    EnablesCommandRedirections,
    CallsDetachedSpawnCommand,
    CreatesManagedRedirectionPath,
    CoversRunRedirection,
    NoManagedStderrAssertionInFocusedTest,
}

impl DocumentationClaim {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "claims_missing" => Some(Self::ClaimsMissing),
            "does_not_claim_missing" => Some(Self::DoesNotClaimMissing),
            "claims_uses_command_redirections" => Some(Self::ClaimsUsesCommandRedirections),
            "claims_inherit_default" => Some(Self::ClaimsInheritDefault),
            "claims_typed_path_field" => Some(Self::ClaimsTypedPathField),
            "claims_typed_append_field" => Some(Self::ClaimsTypedAppendField),
            "claims_setup_failure_is_process_error" => Some(Self::ClaimsSetupFailureIsProcessError),
            "claims_detached_record" => Some(Self::ClaimsDetachedRecord),
            "claims_owned_child_group" => Some(Self::ClaimsOwnedChildGroup),
            "claims_redirection_failure_distinct_from_status" => {
                Some(Self::ClaimsRedirectionFailureDistinctFromStatus)
            }
            "discoverable_typed_path_field" => Some(Self::DiscoverableTypedPathField),
            "discoverable_typed_append_field" => Some(Self::DiscoverableTypedAppendField),
            "claims_owned_handle" => Some(Self::ClaimsOwnedHandle),
            "does_not_disclose_lifecycle" => Some(Self::DoesNotDiscloseLifecycle),
            "finds_command_argv" => Some(Self::FindsCommandArgv),
            "finds_process_spawn" => Some(Self::FindsProcessSpawn),
            "spawn_command_enters_detached_options" => {
                Some(Self::SpawnCommandEntersDetachedOptions)
            }
            "disables_command_redirections" => Some(Self::DisablesCommandRedirections),
            "conditionally_applies_command_redirections" => {
                Some(Self::ConditionallyAppliesCommandRedirections)
            }
            "enables_command_redirections" => Some(Self::EnablesCommandRedirections),
            "calls_detached_spawn_command" => Some(Self::CallsDetachedSpawnCommand),
            "creates_managed_redirection_path" => Some(Self::CreatesManagedRedirectionPath),
            "covers_run_redirection" => Some(Self::CoversRunRedirection),
            "no_managed_stderr_assertion_in_focused_test" => {
                Some(Self::NoManagedStderrAssertionInFocusedTest)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CitationFile {
    LangMd,
    SpecMd,
    SpecOsMd,
    ApiProcessCommandArgv,
    ApiProcessSpawn,
    ApiSearchProcess,
    RuntimeProcess,
    LoweredRun,
    NativeProcessTest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceCitation {
    File(CitationFile),
    Line {
        file: CitationFile,
        line: u32,
    },
    LineRange {
        file: CitationFile,
        start: u32,
        end: u32,
    },
}

impl SourceCitation {
    fn parse(value: &str) -> Option<Self> {
        let (file, suffix) = match value.split_once(':') {
            Some((file, suffix)) => (parse_citation_file(file)?, Some(suffix)),
            None => (parse_citation_file(value)?, None),
        };
        let Some(suffix) = suffix else {
            return Some(Self::File(file));
        };
        if let Some((start, end)) = suffix.split_once('-') {
            let start = parse_positive_u32(start)?;
            let end = parse_positive_u32(end)?;
            if start > end {
                return None;
            }
            return Some(Self::LineRange { file, start, end });
        }
        Some(Self::Line {
            file,
            line: parse_positive_u32(suffix)?,
        })
    }
}

fn parse_citation_file(value: &str) -> Option<CitationFile> {
    match value {
        "LANG.md" => Some(CitationFile::LangMd),
        "SPEC.md" => Some(CitationFile::SpecMd),
        "SPEC-OS.md" => Some(CitationFile::SpecOsMd),
        "api-process-command-argv.txt" => Some(CitationFile::ApiProcessCommandArgv),
        "api-process-spawn.txt" => Some(CitationFile::ApiProcessSpawn),
        "api-search-process.txt" => Some(CitationFile::ApiSearchProcess),
        "src/runtime/process.rs" => Some(CitationFile::RuntimeProcess),
        "src/runtime/eval/lowered_run.rs" => Some(CitationFile::LoweredRun),
        "tests/xsh/stdlib/process.xsh" => Some(CitationFile::NativeProcessTest),
        _ => None,
    }
}

fn parse_positive_u32(value: &str) -> Option<u32> {
    let parsed = value.parse::<u32>().ok()?;
    if parsed == 0 || value != parsed.to_string() {
        return None;
    }
    Some(parsed)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentationObservationV1 {
    pub source: DocumentationSource,
    pub consumer: DocumentationConsumer,
    pub field: DocumentationField,
    pub claim: DocumentationClaim,
    pub citation: SourceCitation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentationObservationSetV1 {
    observations: [DocumentationObservationV1; 22],
}

impl DocumentationObservationSetV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::DocumentationObservation;
        let parsed = table(bytes, schema, DOCUMENTATION_SCHEMA, DOCUMENTATION_HEADER)?;
        exact_rows(&parsed, schema, 22)?;
        let mut observations = Vec::with_capacity(22);
        for (index, line) in parsed.rows.iter().enumerate() {
            let line_number = index + 3;
            let fields = row(line, line_number, schema, 5)?;
            let observation = DocumentationObservationV1 {
                source: DocumentationSource::parse(fields[0]).ok_or(
                    Vs001ParseError::InvalidValue {
                        schema,
                        line: line_number,
                        column: "source",
                    },
                )?,
                consumer: DocumentationConsumer::parse(fields[1]).ok_or(
                    Vs001ParseError::InvalidValue {
                        schema,
                        line: line_number,
                        column: "consumer",
                    },
                )?,
                field: DocumentationField::parse(fields[2]).ok_or(
                    Vs001ParseError::InvalidValue {
                        schema,
                        line: line_number,
                        column: "field",
                    },
                )?,
                claim: DocumentationClaim::parse(fields[3]).ok_or(
                    Vs001ParseError::InvalidValue {
                        schema,
                        line: line_number,
                        column: "claim",
                    },
                )?,
                citation: SourceCitation::parse(fields[4]).ok_or(
                    Vs001ParseError::InvalidValue {
                        schema,
                        line: line_number,
                        column: "citation",
                    },
                )?,
            };
            if !documentation_slot_matches(index, observation) {
                return Err(Vs001ParseError::RecombinedRow {
                    schema,
                    line: line_number,
                });
            }
            observations.push(observation);
        }
        let observations = observations
            .try_into()
            .map_err(|_| Vs001ParseError::WrongRowCount {
                schema,
                expected: 22,
                actual: 0,
            })?;
        Ok(Self { observations })
    }

    pub const fn observations(&self) -> &[DocumentationObservationV1; 22] {
        &self.observations
    }
}

fn documentation_slot_matches(index: usize, row: DocumentationObservationV1) -> bool {
    use CitationFile::*;
    use DocumentationClaim::*;
    use DocumentationConsumer::*;
    use DocumentationField::*;
    use DocumentationSource::{
        LangMd as SourceLangMd, LoweredRuntime, NativeTest, Runtime, SpecApi, SpecOs, SpecSpawn,
        XshtApi, XshtNavigation,
    };
    let line = |file| matches!(row.citation, SourceCitation::Line { file: observed, .. } if observed == file);
    let file =
        |expected| matches!(row.citation, SourceCitation::File(observed) if observed == expected);
    let range = |expected| matches!(row.citation, SourceCitation::LineRange { file: observed, .. } if observed == expected);
    match index {
        0 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (
                    SourceLangMd,
                    SpawnCommand,
                    Stderr,
                    ClaimsMissing | DoesNotClaimMissing
                )
            ) && match row.claim {
                ClaimsMissing => line(LangMd),
                DoesNotClaimMissing => file(LangMd),
                _ => false,
            }
        }
        1 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (
                    SpecSpawn,
                    SpawnCommand,
                    Stderr,
                    ClaimsUsesCommandRedirections
                )
            ) && line(SpecMd)
        }
        2 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (SpecSpawn, SpawnCommand, Default, ClaimsInheritDefault)
            ) && line(SpecMd)
        }
        3 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (SpecApi, CommandPlan, Stderr, ClaimsTypedPathField)
            ) && line(SpecMd)
        }
        4 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (SpecApi, CommandPlan, StderrAppend, ClaimsTypedAppendField)
            ) && line(SpecMd)
        }
        5 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (
                    SpecSpawn,
                    SpawnCommand,
                    Error,
                    ClaimsSetupFailureIsProcessError
                )
            ) && line(SpecMd)
        }
        6 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (SpecSpawn, ProcessSpawn, Lifecycle, ClaimsDetachedRecord)
            ) && line(SpecMd)
        }
        7 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (SpecOs, SpawnCommand, Ownership, ClaimsOwnedChildGroup)
            ) && line(SpecOsMd)
        }
        8 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (
                    SpecOs,
                    CommandPlan,
                    Error,
                    ClaimsRedirectionFailureDistinctFromStatus
                )
            ) && line(SpecOsMd)
        }
        9 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (XshtApi, CommandPlan, Stderr, DiscoverableTypedPathField)
            ) && line(ApiProcessCommandArgv)
        }
        10 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (
                    XshtApi,
                    CommandPlan,
                    StderrAppend,
                    DiscoverableTypedAppendField
                )
            ) && line(ApiProcessCommandArgv)
        }
        11 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (
                    XshtApi,
                    ProcessSpawn,
                    Lifecycle,
                    ClaimsOwnedHandle | ClaimsDetachedRecord | DoesNotDiscloseLifecycle
                )
            ) && match row.claim {
                DoesNotDiscloseLifecycle => file(ApiProcessSpawn),
                _ => line(ApiProcessSpawn),
            }
        }
        12 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (XshtNavigation, CommandPlan, Discovery, FindsCommandArgv)
            ) && line(ApiSearchProcess)
        }
        13 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (XshtNavigation, ProcessSpawn, Discovery, FindsProcessSpawn)
            ) && line(ApiSearchProcess)
        }
        14 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (
                    Runtime,
                    ProcessSpawn,
                    CallPath,
                    SpawnCommandEntersDetachedOptions
                )
            ) && line(RuntimeProcess)
        }
        15 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (Runtime, ProcessSpawn, Stderr, DisablesCommandRedirections)
            ) && line(RuntimeProcess)
        }
        16 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (
                    Runtime,
                    ManagedSpawn,
                    Stderr,
                    ConditionallyAppliesCommandRedirections
                )
            ) && line(RuntimeProcess)
        }
        17 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (Runtime, ManagedSpawn, Stderr, EnablesCommandRedirections)
            ) && line(RuntimeProcess)
        }
        18 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (
                    LoweredRuntime,
                    ProcessSpawn,
                    CallPath,
                    CallsDetachedSpawnCommand
                )
            ) && line(LoweredRun)
        }
        19 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (
                    LoweredRuntime,
                    SpawnCommand,
                    CallPath,
                    CreatesManagedRedirectionPath
                )
            ) && line(LoweredRun)
        }
        20 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (NativeTest, ProcessRun, Stderr, CoversRunRedirection)
            ) && range(NativeProcessTest)
        }
        21 => {
            matches!(
                (row.source, row.consumer, row.field, row.claim),
                (
                    NativeTest,
                    SpawnCommand,
                    Stderr,
                    NoManagedStderrAssertionInFocusedTest
                )
            ) && range(NativeProcessTest)
        }
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentationConflictId {
    D01,
    D02,
    D03,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentationConflictStatus {
    Present,
    Absent,
    Resolved,
    IntentionalSemanticSplit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentationConflictObservationV1 {
    pub id: DocumentationConflictId,
    pub status: DocumentationConflictStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentationConflictSetV1 {
    conflicts: [DocumentationConflictObservationV1; 3],
}

impl DocumentationConflictSetV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::DocumentationConflict;
        let parsed = table(
            bytes,
            schema,
            DOCUMENTATION_CONFLICT_SCHEMA,
            DOCUMENTATION_CONFLICT_HEADER,
        )?;
        exact_rows(&parsed, schema, 3)?;
        let mut conflicts = Vec::with_capacity(3);
        for (index, line) in parsed.rows.iter().enumerate() {
            let line_number = index + 3;
            let fields = row(line, line_number, schema, 4)?;
            let (id, expected_left, expected_right, valid_status) = match index {
                0 => (
                    DocumentationConflictId::D01,
                    "LANG_claims_missing",
                    "SPEC_claims_supported",
                    matches!(fields[3], "present" | "absent" | "resolved"),
                ),
                1 => (
                    DocumentationConflictId::D02,
                    "xsht_api_claims_owned_handle",
                    "SPEC_claims_detached_record",
                    matches!(fields[3], "present" | "absent" | "resolved"),
                ),
                2 => (
                    DocumentationConflictId::D03,
                    "process_spawn_redirection_ignored",
                    "managed_spawn_redirection_enabled",
                    fields[3] == "intentional_semantic_split",
                ),
                _ => unreachable!(),
            };
            if fields[0]
                != match id {
                    DocumentationConflictId::D01 => "D01",
                    DocumentationConflictId::D02 => "D02",
                    DocumentationConflictId::D03 => "D03",
                }
                || fields[1] != expected_left
                || fields[2] != expected_right
                || !valid_status
            {
                return Err(Vs001ParseError::RecombinedRow {
                    schema,
                    line: line_number,
                });
            }
            let status = match fields[3] {
                "present" => DocumentationConflictStatus::Present,
                "absent" => DocumentationConflictStatus::Absent,
                "resolved" => DocumentationConflictStatus::Resolved,
                "intentional_semantic_split" => {
                    DocumentationConflictStatus::IntentionalSemanticSplit
                }
                _ => {
                    return Err(Vs001ParseError::InvalidValue {
                        schema,
                        line: line_number,
                        column: "status",
                    });
                }
            };
            conflicts.push(DocumentationConflictObservationV1 { id, status });
        }
        Ok(Self {
            conflicts: conflicts
                .try_into()
                .map_err(|_| Vs001ParseError::WrongRowCount {
                    schema,
                    expected: 3,
                    actual: 0,
                })?,
        })
    }

    pub const fn conflicts(&self) -> &[DocumentationConflictObservationV1; 3] {
        &self.conflicts
    }
}

// ---------------------------------------------------------------------------
// Fluency output relations

const FLUENCY_SCHEMA: &str = "# schema: FluencyProbeObservationV1/tsv-v1";
const FLUENCY_HEADER: &str = "case_id\tinput_manifest\texpected_exit\tsupervisor_exit\tparent_stdout_sha256\tparent_stderr_sha256\tredirected_stderr_sha256\tcorrectness\ttyped_boundary\townership_lifecycle\thost_path_access\tdisposition";
const FLUENCY_SURFACE_SCHEMA: &str = "# schema: FluencyProbeExecutionSurfaceV1/tsv-v1";
const FLUENCY_SURFACE_HEADER: &str =
    "execution_kind\ttool_errors\tturns\tactive_wall\ttokens\treasoning_tokens\tcost";
const FLUENCY_ENVELOPE_SCHEMA: &str = "# schema: FluencyExecutionEnvelopeV1/tsv-v1";
const FLUENCY_ENVELOPE_HEADER: &str =
    "workspace_label\tworking_directory\tenvironment\thome\tconfig\ttemp\tpath";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyCaseId {
    F01,
    F02,
    F03,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyInputManifest {
    PreexistingLogTruncate,
    PathWithSpaces,
    NonzeroChildStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyCorrectness {
    Passed,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyTypedBoundary {
    Compliant,
}
/// This is evaluator-level ownership evidence only, never a daemon reaping receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyEvaluatorOwnershipLifecycle {
    OwnedWaited,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyHostPathAccess {
    Clean,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyDisposition {
    Pass,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FluencyProbeObservationV1 {
    pub case_id: FluencyCaseId,
    pub input_manifest: FluencyInputManifest,
    pub expected_exit: u8,
    pub supervisor_exit: u8,
    pub parent_stdout: ContentDigest,
    pub parent_stderr: ContentDigest,
    pub redirected_stderr: ContentDigest,
    pub correctness: FluencyCorrectness,
    pub typed_boundary: FluencyTypedBoundary,
    pub ownership_lifecycle: FluencyEvaluatorOwnershipLifecycle,
    pub host_path_access: FluencyHostPathAccess,
    pub disposition: FluencyDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FluencyProbeObservationSetV1 {
    observations: [FluencyProbeObservationV1; 3],
}

impl FluencyProbeObservationSetV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::FluencyProbeObservation;
        let parsed = table(bytes, schema, FLUENCY_SCHEMA, FLUENCY_HEADER)?;
        exact_rows(&parsed, schema, 3)?;
        let mut observations = Vec::with_capacity(3);
        for (index, line) in parsed.rows.iter().enumerate() {
            let line_number = index + 3;
            let fields = row(line, line_number, schema, 12)?;
            let (case_id, input_manifest, expected_exit) = match index {
                0 if fields[0] == "F01" && fields[1] == "preexisting_log_truncate" => (
                    FluencyCaseId::F01,
                    FluencyInputManifest::PreexistingLogTruncate,
                    0,
                ),
                1 if fields[0] == "F02" && fields[1] == "path_with_spaces" => {
                    (FluencyCaseId::F02, FluencyInputManifest::PathWithSpaces, 0)
                }
                2 if fields[0] == "F03" && fields[1] == "nonzero_child_status" => (
                    FluencyCaseId::F03,
                    FluencyInputManifest::NonzeroChildStatus,
                    23,
                ),
                _ => {
                    return Err(Vs001ParseError::OutOfOrder {
                        schema,
                        line: line_number,
                    });
                }
            };
            let actual_expected =
                parse_canonical_u8(fields[2]).ok_or(Vs001ParseError::InvalidValue {
                    schema,
                    line: line_number,
                    column: "expected_exit",
                })?;
            let supervisor_exit =
                parse_canonical_u8(fields[3]).ok_or(Vs001ParseError::InvalidValue {
                    schema,
                    line: line_number,
                    column: "supervisor_exit",
                })?;
            if actual_expected != expected_exit
                || supervisor_exit != expected_exit
                || fields[7] != "passed"
                || fields[8] != "compliant"
                || fields[9] != "owned_waited"
                || fields[10] != "clean"
                || fields[11] != "pass"
            {
                return Err(Vs001ParseError::RecombinedRow {
                    schema,
                    line: line_number,
                });
            }
            observations.push(FluencyProbeObservationV1 {
                case_id,
                input_manifest,
                expected_exit,
                supervisor_exit,
                parent_stdout: digest(fields[4], schema, line_number, "parent_stdout_sha256")?,
                parent_stderr: digest(fields[5], schema, line_number, "parent_stderr_sha256")?,
                redirected_stderr: digest(
                    fields[6],
                    schema,
                    line_number,
                    "redirected_stderr_sha256",
                )?,
                correctness: FluencyCorrectness::Passed,
                typed_boundary: FluencyTypedBoundary::Compliant,
                ownership_lifecycle: FluencyEvaluatorOwnershipLifecycle::OwnedWaited,
                host_path_access: FluencyHostPathAccess::Clean,
                disposition: FluencyDisposition::Pass,
            });
        }
        Ok(Self {
            observations: observations
                .try_into()
                .map_err(|_| Vs001ParseError::WrongRowCount {
                    schema,
                    expected: 3,
                    actual: 0,
                })?,
        })
    }

    pub const fn observations(&self) -> &[FluencyProbeObservationV1; 3] {
        &self.observations
    }
}

fn parse_canonical_u8(value: &str) -> Option<u8> {
    let parsed = value.parse::<u8>().ok()?;
    (value == parsed.to_string()).then_some(parsed)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyExecutionKind {
    DeterministicFixture,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnobservedProviderResource {
    NotObservedNoProvider,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FluencyProbeExecutionSurfaceV1 {
    pub execution_kind: FluencyExecutionKind,
    pub tool_errors: UnobservedProviderResource,
    pub turns: UnobservedProviderResource,
    pub active_wall: UnobservedProviderResource,
    pub tokens: UnobservedProviderResource,
    pub reasoning_tokens: UnobservedProviderResource,
    pub cost: UnobservedProviderResource,
}

impl FluencyProbeExecutionSurfaceV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::FluencyExecutionSurface;
        let parsed = table(
            bytes,
            schema,
            FLUENCY_SURFACE_SCHEMA,
            FLUENCY_SURFACE_HEADER,
        )?;
        exact_rows(&parsed, schema, 1)?;
        let fields = row(parsed.rows[0], 3, schema, 7)?;
        if fields[0] != "deterministic_fixture"
            || fields[1..]
                .iter()
                .any(|value| *value != "not_observed_no_provider")
        {
            return Err(Vs001ParseError::RecombinedRow { schema, line: 3 });
        }
        Ok(Self {
            execution_kind: FluencyExecutionKind::DeterministicFixture,
            tool_errors: UnobservedProviderResource::NotObservedNoProvider,
            turns: UnobservedProviderResource::NotObservedNoProvider,
            active_wall: UnobservedProviderResource::NotObservedNoProvider,
            tokens: UnobservedProviderResource::NotObservedNoProvider,
            reasoning_tokens: UnobservedProviderResource::NotObservedNoProvider,
            cost: UnobservedProviderResource::NotObservedNoProvider,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpaqueWorkspaceLabel(String);

impl OpaqueWorkspaceLabel {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    fn parse(value: &str) -> Option<Self> {
        (!value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'))
        .then(|| Self(value.to_owned()))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyWorkingDirectory {
    OpaqueWorkspace,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyEnvironment {
    MinimalExplicit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyWorkspaceRoot {
    WorkspaceLocal,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FluencyPathPolicy {
    AssignedBinFront,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FluencyExecutionEnvelopeV1 {
    pub workspace_label: OpaqueWorkspaceLabel,
    pub working_directory: FluencyWorkingDirectory,
    pub environment: FluencyEnvironment,
    pub home: FluencyWorkspaceRoot,
    pub config: FluencyWorkspaceRoot,
    pub temp: FluencyWorkspaceRoot,
    pub path: FluencyPathPolicy,
}

impl FluencyExecutionEnvelopeV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::FluencyExecutionEnvelope;
        let parsed = table(
            bytes,
            schema,
            FLUENCY_ENVELOPE_SCHEMA,
            FLUENCY_ENVELOPE_HEADER,
        )?;
        exact_rows(&parsed, schema, 1)?;
        let fields = row(parsed.rows[0], 3, schema, 7)?;
        let workspace_label =
            OpaqueWorkspaceLabel::parse(fields[0]).ok_or(Vs001ParseError::InvalidValue {
                schema,
                line: 3,
                column: "workspace_label",
            })?;
        if fields[1] != "opaque_workspace"
            || fields[2] != "minimal_explicit"
            || fields[3] != "workspace_local"
            || fields[4] != "workspace_local"
            || fields[5] != "workspace_local"
            || fields[6] != "assigned_bin_front"
        {
            return Err(Vs001ParseError::RecombinedRow { schema, line: 3 });
        }
        Ok(Self {
            workspace_label,
            working_directory: FluencyWorkingDirectory::OpaqueWorkspace,
            environment: FluencyEnvironment::MinimalExplicit,
            home: FluencyWorkspaceRoot::WorkspaceLocal,
            config: FluencyWorkspaceRoot::WorkspaceLocal,
            temp: FluencyWorkspaceRoot::WorkspaceLocal,
            path: FluencyPathPolicy::AssignedBinFront,
        })
    }
}

// ---------------------------------------------------------------------------
// C1 curation account relations and evaluator output

const CURATION_FRONTIER_HEADER: &str = "source_ref";
const CURATION_ACCOUNT_HEADER: &str = "kind\tpurpose\tquestion_revision\tdisclosure_frontier\tcurator_configuration\tleading_hypothesis\tstrongest_counterevidence_ref";
const CURATION_SELECTED_HEADER: &str =
    "ordinal\tsource_ref\trole\tselection_reason\tapplicability_scope";
const CURATION_CONFLICTS_HEADER: &str = "conflict_ref";
const CURATION_UNKNOWNS_HEADER: &str = "unknown_ref";
const CURATION_EXCLUSIONS_HEADER: &str = "category_or_source\treason\trisk_if_wrong";
const CURATION_ESCALATIONS_HEADER: &str = "question_ref\tobject_ref";
const CURATION_OBSERVATION_SCHEMA: &str = "# schema: CurationContractObservationV1/tsv-v1";
const CURATION_OBSERVATION_HEADER: &str = "account_kind\tpurpose\thypothesis_coverage\tcounterevidence\tpreserved_conflict\tunknowns\texclusions\traw_escalations\tfrontier_admission\tdisposition";
const CURATION_ESCALATION_OBSERVATION_SCHEMA: &str =
    "# schema: CurationRawEvidenceEscalationObservationV1/tsv-v1";
const CURATION_ESCALATION_OBSERVATION_HEADER: &str = "ordinal\tquestion_ref\tobject_ref";

/// Named inputs known to C1.  Parsing their spelling does not decide whether
/// any source is admitted in a future kernel; it only prevents an opaque TSV
/// reference from being substituted for one of the closed fixture identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CurationSourceRef {
    ArgumentH1,
    ArgumentH2,
    ArgumentH3,
    ObservationManagedBehavior,
    ObservationDetachedBehavior,
    ObservationDocumentationConflict,
    ConflictLangVsSpec,
}

impl CurationSourceRef {
    const FRONTIER: [Self; 7] = [
        Self::ArgumentH1,
        Self::ArgumentH2,
        Self::ArgumentH3,
        Self::ObservationManagedBehavior,
        Self::ObservationDetachedBehavior,
        Self::ObservationDocumentationConflict,
        Self::ConflictLangVsSpec,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ArgumentH1 => "argument_h1",
            Self::ArgumentH2 => "argument_h2",
            Self::ArgumentH3 => "argument_h3",
            Self::ObservationManagedBehavior => "observation_managed_behavior",
            Self::ObservationDetachedBehavior => "observation_detached_behavior",
            Self::ObservationDocumentationConflict => "observation_documentation_conflict",
            Self::ConflictLangVsSpec => "conflict_lang_vs_spec",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::FRONTIER
            .into_iter()
            .find(|entry| entry.as_str() == value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurationFrontierV1 {
    members: [CurationSourceRef; 7],
}

impl CurationFrontierV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::CurationFrontierMembers;
        let parsed = relation(bytes, schema, CURATION_FRONTIER_HEADER)?;
        exact_rows(&parsed, schema, 7)?;
        let mut members = Vec::with_capacity(7);
        for (index, line) in parsed.rows.iter().enumerate() {
            let line_number = index + 2;
            let fields = row(line, line_number, schema, 1)?;
            let member =
                CurationSourceRef::parse(fields[0]).ok_or(Vs001ParseError::InvalidValue {
                    schema,
                    line: line_number,
                    column: "source_ref",
                })?;
            if members.contains(&member) {
                return Err(Vs001ParseError::DuplicateIdentity {
                    schema,
                    line: line_number,
                    identity: "source_ref",
                });
            }
            members.push(member);
        }
        if !CurationSourceRef::FRONTIER
            .iter()
            .all(|required| members.contains(required))
        {
            return Err(Vs001ParseError::RecombinedRow { schema, line: 2 });
        }
        Ok(Self {
            members: members
                .try_into()
                .map_err(|_| Vs001ParseError::WrongRowCount {
                    schema,
                    expected: 7,
                    actual: 0,
                })?,
        })
    }

    pub const fn members(&self) -> &[CurationSourceRef; 7] {
        &self.members
    }

    fn contains(&self, source: CurationSourceRef) -> bool {
        self.members.contains(&source)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CurationSelectionRole {
    DefeatingArgument,
    SupportingArgument,
    Dissent,
    Observation,
    Constraint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CurationSelectionReason {
    H1Rejected,
    H2PartiallySupported,
    H3Supported,
    DirectOwnedPathResult,
    DetachedPolicyDistinction,
    StaleDiscoveryConflict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CurationSelectionV1 {
    pub ordinal: u8,
    pub source: CurationSourceRef,
    pub role: CurationSelectionRole,
    pub reason: CurationSelectionReason,
}

fn expected_curation_selection(
    index: usize,
) -> (
    CurationSourceRef,
    CurationSelectionRole,
    CurationSelectionReason,
) {
    match index {
        0 => (
            CurationSourceRef::ArgumentH1,
            CurationSelectionRole::DefeatingArgument,
            CurationSelectionReason::H1Rejected,
        ),
        1 => (
            CurationSourceRef::ArgumentH2,
            CurationSelectionRole::SupportingArgument,
            CurationSelectionReason::H2PartiallySupported,
        ),
        2 => (
            CurationSourceRef::ArgumentH3,
            CurationSelectionRole::Dissent,
            CurationSelectionReason::H3Supported,
        ),
        3 => (
            CurationSourceRef::ObservationManagedBehavior,
            CurationSelectionRole::Observation,
            CurationSelectionReason::DirectOwnedPathResult,
        ),
        4 => (
            CurationSourceRef::ObservationDetachedBehavior,
            CurationSelectionRole::Dissent,
            CurationSelectionReason::DetachedPolicyDistinction,
        ),
        5 => (
            CurationSourceRef::ObservationDocumentationConflict,
            CurationSelectionRole::Constraint,
            CurationSelectionReason::StaleDiscoveryConflict,
        ),
        _ => unreachable!("the six-row curation relation bounds this index"),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RawEvidenceEscalationV1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CuratedAccountV1 {
    selections: [CurationSelectionV1; 6],
    raw_evidence_escalation: Option<RawEvidenceEscalationV1>,
}

impl CuratedAccountV1 {
    /// Parse all six normalized C1 account relations.  `frontier` is a parsed
    /// declared relation, not an admission or disclosure decision.
    pub fn parse(
        frontier: &CurationFrontierV1,
        account: &[u8],
        selected: &[u8],
        conflicts: &[u8],
        unknowns: &[u8],
        exclusions: &[u8],
        escalations: &[u8],
    ) -> Result<Self, Vs001ParseError> {
        parse_exact_single_relation(
            account,
            Vs001Schema::CurationAccount,
            CURATION_ACCOUNT_HEADER,
            &[
                "decision_curation",
                "authorize_spawn_stderr_prototype",
                "q_vs001_spawn_stderr_r1",
                "fc1",
                "curator_v1",
                "h2",
                "argument_h3",
            ],
        )?;

        let schema = Vs001Schema::CurationSelectedItems;
        let selected_table = relation(selected, schema, CURATION_SELECTED_HEADER)?;
        exact_rows(&selected_table, schema, 6)?;
        let mut selections = Vec::with_capacity(6);
        for (index, line) in selected_table.rows.iter().enumerate() {
            let line_number = index + 2;
            let fields = row(line, line_number, schema, 5)?;
            let (source, role, reason) = expected_curation_selection(index);
            if fields[0] != (index + 1).to_string()
                || fields[1] != source.as_str()
                || fields[2] != curation_role_string(role)
                || fields[3] != curation_reason_string(reason)
                || fields[4] != "spawn_stderr_prototype"
                || !frontier.contains(source)
            {
                return Err(Vs001ParseError::RecombinedRow {
                    schema,
                    line: line_number,
                });
            }
            selections.push(CurationSelectionV1 {
                ordinal: (index + 1) as u8,
                source,
                role,
                reason,
            });
        }

        parse_exact_single_relation(
            conflicts,
            Vs001Schema::CurationPreservedConflicts,
            CURATION_CONFLICTS_HEADER,
            &["conflict_lang_vs_spec"],
        )?;
        parse_exact_single_relation(
            unknowns,
            Vs001Schema::CurationUnknowns,
            CURATION_UNKNOWNS_HEADER,
            &["detached_public_contract_intent"],
        )?;
        parse_exact_unordered_relation_rows(
            exclusions,
            Vs001Schema::CurationExclusions,
            CURATION_EXCLUSIONS_HEADER,
            &[
                &[
                    "raw_pi_session",
                    "no_named_question",
                    "decision_context_may_miss_relevant_reasoning",
                ],
                &[
                    "candidate_patch",
                    "post_frontier_material",
                    "would_leak_future_product_choice",
                ],
            ],
        )?;
        let raw_evidence_escalation = parse_raw_evidence_escalation_relation(escalations)?;
        Ok(Self {
            selections: selections
                .try_into()
                .map_err(|_| Vs001ParseError::WrongRowCount {
                    schema,
                    expected: 6,
                    actual: 0,
                })?,
            raw_evidence_escalation,
        })
    }

    pub const fn selections(&self) -> &[CurationSelectionV1; 6] {
        &self.selections
    }

    pub const fn raw_evidence_escalation(&self) -> Option<RawEvidenceEscalationV1> {
        self.raw_evidence_escalation
    }
}

const fn curation_role_string(role: CurationSelectionRole) -> &'static str {
    match role {
        CurationSelectionRole::DefeatingArgument => "defeating_argument",
        CurationSelectionRole::SupportingArgument => "supporting_argument",
        CurationSelectionRole::Dissent => "dissent",
        CurationSelectionRole::Observation => "observation",
        CurationSelectionRole::Constraint => "constraint",
    }
}

const fn curation_reason_string(reason: CurationSelectionReason) -> &'static str {
    match reason {
        CurationSelectionReason::H1Rejected => "h1_rejected",
        CurationSelectionReason::H2PartiallySupported => "h2_partially_supported",
        CurationSelectionReason::H3Supported => "h3_supported",
        CurationSelectionReason::DirectOwnedPathResult => "direct_owned_path_result",
        CurationSelectionReason::DetachedPolicyDistinction => "detached_policy_distinction",
        CurationSelectionReason::StaleDiscoveryConflict => "stale_discovery_conflict",
    }
}

fn parse_exact_single_relation(
    bytes: &[u8],
    schema: Vs001Schema,
    header: &str,
    expected: &[&str],
) -> Result<(), Vs001ParseError> {
    parse_exact_relation_rows(bytes, schema, header, &[expected])
}

fn parse_exact_relation_rows(
    bytes: &[u8],
    schema: Vs001Schema,
    header: &str,
    expected: &[&[&str]],
) -> Result<(), Vs001ParseError> {
    let parsed = relation(bytes, schema, header)?;
    exact_rows(&parsed, schema, expected.len())?;
    for (index, (line, expected_row)) in parsed.rows.iter().zip(expected).enumerate() {
        let line_number = index + 2;
        let fields = row(line, line_number, schema, expected_row.len())?;
        if fields != *expected_row {
            return Err(Vs001ParseError::RecombinedRow {
                schema,
                line: line_number,
            });
        }
    }
    Ok(())
}

/// Some relations use a declared identity set rather than a meaningful row
/// order. The parser retains that contract while still rejecting duplicate,
/// missing, extra, and recombined rows.
fn parse_exact_unordered_relation_rows(
    bytes: &[u8],
    schema: Vs001Schema,
    header: &str,
    expected: &[&[&str]],
) -> Result<(), Vs001ParseError> {
    let parsed = relation(bytes, schema, header)?;
    exact_rows(&parsed, schema, expected.len())?;
    let mut seen = vec![false; expected.len()];
    for (index, line) in parsed.rows.iter().enumerate() {
        let line_number = index + 2;
        let fields = row(line, line_number, schema, expected[0].len())?;
        let Some(expected_index) = expected
            .iter()
            .position(|candidate| candidate[0] == fields[0])
        else {
            return Err(Vs001ParseError::InvalidValue {
                schema,
                line: line_number,
                column: "identity",
            });
        };
        if seen[expected_index] {
            return Err(Vs001ParseError::DuplicateIdentity {
                schema,
                line: line_number,
                identity: "identity",
            });
        }
        if fields != expected[expected_index] {
            return Err(Vs001ParseError::RecombinedRow {
                schema,
                line: line_number,
            });
        }
        seen[expected_index] = true;
    }
    if seen.iter().any(|found| !found) {
        return Err(Vs001ParseError::RecombinedRow { schema, line: 2 });
    }
    Ok(())
}

fn parse_raw_evidence_escalation_relation(
    bytes: &[u8],
) -> Result<Option<RawEvidenceEscalationV1>, Vs001ParseError> {
    let schema = Vs001Schema::CurationEscalations;
    let parsed = relation(bytes, schema, CURATION_ESCALATIONS_HEADER)?;
    if parsed.rows.len() > 1 {
        return Err(Vs001ParseError::WrongRowCount {
            schema,
            expected: 1,
            actual: parsed.rows.len(),
        });
    }
    let Some(line) = parsed.rows.first() else {
        return Ok(None);
    };
    let fields = row(line, 2, schema, 2)?;
    if fields != ["resolve_detached_contract_intent", "raw_pi_session_object"] {
        return Err(Vs001ParseError::RecombinedRow { schema, line: 2 });
    }
    Ok(Some(RawEvidenceEscalationV1))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CurationRawEscalationState {
    NoneRequested,
    NamedRequestPresent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CurationContractObservationV1 {
    pub raw_escalations: CurationRawEscalationState,
}

impl CurationContractObservationV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::CurationContractObservation;
        let parsed = table(
            bytes,
            schema,
            CURATION_OBSERVATION_SCHEMA,
            CURATION_OBSERVATION_HEADER,
        )?;
        exact_rows(&parsed, schema, 1)?;
        let fields = row(parsed.rows[0], 3, schema, 10)?;
        let raw_escalations = match fields[7] {
            "none_requested" => CurationRawEscalationState::NoneRequested,
            "named_request_present" => CurationRawEscalationState::NamedRequestPresent,
            _ => {
                return Err(Vs001ParseError::InvalidValue {
                    schema,
                    line: 3,
                    column: "raw_escalations",
                });
            }
        };
        if fields[..7]
            != [
                "decision_curation",
                "authorize_spawn_stderr_prototype",
                "h1_h2_h3_with_dissent",
                "declared",
                "preserved",
                "declared",
                "semantic",
            ]
            || fields[8] != "frontier_constrained"
            || fields[9] != "acceptance_ready"
        {
            return Err(Vs001ParseError::RecombinedRow { schema, line: 3 });
        }
        Ok(Self { raw_escalations })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurationRawEvidenceEscalationObservationSetV1 {
    request: Option<RawEvidenceEscalationV1>,
}

impl CurationRawEvidenceEscalationObservationSetV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::CurationEscalationObservation;
        let parsed = table(
            bytes,
            schema,
            CURATION_ESCALATION_OBSERVATION_SCHEMA,
            CURATION_ESCALATION_OBSERVATION_HEADER,
        )?;
        if parsed.rows.len() > 1 {
            return Err(Vs001ParseError::WrongRowCount {
                schema,
                expected: 1,
                actual: parsed.rows.len(),
            });
        }
        let Some(line) = parsed.rows.first() else {
            return Ok(Self { request: None });
        };
        let fields = row(line, 3, schema, 3)?;
        if fields
            != [
                "1",
                "resolve_detached_contract_intent",
                "raw_pi_session_object",
            ]
        {
            return Err(Vs001ParseError::RecombinedRow { schema, line: 3 });
        }
        Ok(Self {
            request: Some(RawEvidenceEscalationV1),
        })
    }

    pub const fn request(&self) -> Option<RawEvidenceEscalationV1> {
        self.request
    }
}

/// The two evaluator outputs form one syntactic result boundary. This checks
/// only their internally declared escalation state; it does not authorize the
/// requested object, curation account, or any future disclosure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurationContractOutputsV1 {
    pub observation: CurationContractObservationV1,
    pub raw_evidence_escalation: CurationRawEvidenceEscalationObservationSetV1,
}

impl CurationContractOutputsV1 {
    pub fn parse(
        observation: &[u8],
        raw_evidence_escalation: &[u8],
    ) -> Result<Self, Vs001ParseError> {
        let observation = CurationContractObservationV1::parse(observation)?;
        let raw_evidence_escalation =
            CurationRawEvidenceEscalationObservationSetV1::parse(raw_evidence_escalation)?;
        let expected_request = matches!(
            observation.raw_escalations,
            CurationRawEscalationState::NamedRequestPresent
        );
        if raw_evidence_escalation.request().is_some() != expected_request {
            return Err(Vs001ParseError::RecombinedRow {
                schema: Vs001Schema::CurationContractObservation,
                line: 3,
            });
        }
        Ok(Self {
            observation,
            raw_evidence_escalation,
        })
    }
}

// ---------------------------------------------------------------------------
// Checked-propagation uptake relations and evaluator output

const UPTAKE_CONTEXT_HEADER: &str = "target_revision\tlesson_revision\tlesson_status\tapplicability_scope\tsupporting_episode\texclusions_ref";
const UPTAKE_PERSISTED_HEADER: &str =
    "target_revision\tlesson_revision\tlesson_status\tapplicability_scope";
const UPTAKE_SUBMISSION_HEADER: &str = "lesson_revision\trecommendation\tnormative_registry_state\tnormative_registry_ref\tnormative_registry_unavailable_reason\texecutable_behavior_state\texecutable_behavior_ref\texecutable_behavior_unavailable_reason\tproposal_corpus_state\tproposal_corpus_ref\tproposal_corpus_unavailable_reason\treal_call_sites_state\treal_call_sites_ref\treal_call_sites_unavailable_reason";
const UPTAKE_ACCESSES_HEADER: &str = "ordinal\taccess_class";
const PROPAGATION_SCHEMA: &str = "# schema: PropagationObservationV1/tsv-v1";
const PROPAGATION_HEADER: &str = "target_revision\tlesson_revision\tdelivered\tencountered\tapplication\tcontamination\tdisposition";
const MAX_UPTAKE_ACCESS_ROWS: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UptakeReference(String);

impl UptakeReference {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parse(value: &str) -> Option<Self> {
        (!value.is_empty()
            && value.len() <= 128
            && value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
            }))
        .then(|| Self(value.to_owned()))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UptakeDeliveryContextV1;

impl UptakeDeliveryContextV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        parse_exact_single_relation(
            bytes,
            Vs001Schema::UptakeDeliveryContext,
            UPTAKE_CONTEXT_HEADER,
            &[
                "t_stdout_capture_inquiry_r1",
                "l1_spawn_stderr_method_r1",
                "l1",
                "stdout_capture_process_api",
                "e_vs001_spawn_stderr_r1",
                "x_vs001_c1_r1",
            ],
        )?;
        Ok(Self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UptakePersistedInputV1;

impl UptakePersistedInputV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        parse_exact_single_relation(
            bytes,
            Vs001Schema::UptakePersistedInput,
            UPTAKE_PERSISTED_HEADER,
            &[
                "t_stdout_capture_inquiry_r1",
                "l1_spawn_stderr_method_r1",
                "l1",
                "stdout_capture_process_api",
            ],
        )?;
        Ok(Self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UptakeRecommendation {
    NewApi,
    UseExistingContract,
    FurtherExperiment,
    NoChange,
}

impl UptakeRecommendation {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "new_api" => Some(Self::NewApi),
            "use_existing_contract" => Some(Self::UseExistingContract),
            "further_experiment" => Some(Self::FurtherExperiment),
            "no_change" => Some(Self::NoChange),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvestigatorUnavailableReason {
    SourceNotAdmittedAtFrontier,
    SourceNotAvailable,
    NotApplicableToQuestion,
}

impl InvestigatorUnavailableReason {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "source_not_admitted_at_frontier" => Some(Self::SourceNotAdmittedAtFrontier),
            "source_not_available" => Some(Self::SourceNotAvailable),
            "not_applicable_to_question" => Some(Self::NotApplicableToQuestion),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InvestigatorRecordV1 {
    Available {
        reference: UptakeReference,
    },
    Unavailable {
        reason: InvestigatorUnavailableReason,
    },
}

fn parse_investigator_record(
    state: &str,
    reference: &str,
    unavailable_reason: &str,
    schema: Vs001Schema,
    line: usize,
    column: &'static str,
) -> Result<InvestigatorRecordV1, Vs001ParseError> {
    match state {
        "available" if unavailable_reason == "-" => UptakeReference::parse(reference)
            .filter(|value| value.as_str() != "-")
            .map(|reference| InvestigatorRecordV1::Available { reference })
            .ok_or(Vs001ParseError::InvalidValue {
                schema,
                line,
                column,
            }),
        "unavailable" if reference == "-" => {
            InvestigatorUnavailableReason::parse(unavailable_reason)
                .map(|reason| InvestigatorRecordV1::Unavailable { reason })
                .ok_or(Vs001ParseError::InvalidValue {
                    schema,
                    line,
                    column,
                })
        }
        _ => Err(Vs001ParseError::RecombinedRow { schema, line }),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvestigatorSubmissionV1 {
    pub recommendation: UptakeRecommendation,
    pub normative_registry: InvestigatorRecordV1,
    pub executable_behavior: InvestigatorRecordV1,
    pub proposal_corpus: InvestigatorRecordV1,
    pub real_call_sites: InvestigatorRecordV1,
}

impl InvestigatorSubmissionV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::UptakeInvestigatorSubmission;
        let parsed = relation(bytes, schema, UPTAKE_SUBMISSION_HEADER)?;
        exact_rows(&parsed, schema, 1)?;
        let fields = row(parsed.rows[0], 2, schema, 14)?;
        if fields[0] != "l1_spawn_stderr_method_r1" {
            return Err(Vs001ParseError::RecombinedRow { schema, line: 2 });
        }
        Ok(Self {
            recommendation: UptakeRecommendation::parse(fields[1]).ok_or(
                Vs001ParseError::InvalidValue {
                    schema,
                    line: 2,
                    column: "recommendation",
                },
            )?,
            normative_registry: parse_investigator_record(
                fields[2],
                fields[3],
                fields[4],
                schema,
                2,
                "normative_registry_state",
            )?,
            executable_behavior: parse_investigator_record(
                fields[5],
                fields[6],
                fields[7],
                schema,
                2,
                "executable_behavior_state",
            )?,
            proposal_corpus: parse_investigator_record(
                fields[8],
                fields[9],
                fields[10],
                schema,
                2,
                "proposal_corpus_state",
            )?,
            real_call_sites: parse_investigator_record(
                fields[11],
                fields[12],
                fields[13],
                schema,
                2,
                "real_call_sites_state",
            )?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvestigatorAccessClass {
    TargetContext,
    ForbiddenVs001Session,
    PostTargetMaterial,
}

impl InvestigatorAccessClass {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "target_context" => Some(Self::TargetContext),
            "forbidden_vs001_session" => Some(Self::ForbiddenVs001Session),
            "post_target_material" => Some(Self::PostTargetMaterial),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvestigatorAccessV1 {
    pub ordinal: u8,
    pub class: InvestigatorAccessClass,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvestigatorAccessSetV1 {
    accesses: Vec<InvestigatorAccessV1>,
}

impl InvestigatorAccessSetV1 {
    /// This preserves all closed access classifications. It deliberately does
    /// not turn a forbidden access into a parsing failure; the evaluator's
    /// propagation disposition records that later semantic consequence.
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::UptakeAccesses;
        let parsed = relation(bytes, schema, UPTAKE_ACCESSES_HEADER)?;
        if parsed.rows.len() > MAX_UPTAKE_ACCESS_ROWS {
            return Err(Vs001ParseError::WrongRowCount {
                schema,
                expected: MAX_UPTAKE_ACCESS_ROWS,
                actual: parsed.rows.len(),
            });
        }
        let mut accesses = Vec::with_capacity(parsed.rows.len());
        for (index, line) in parsed.rows.iter().enumerate() {
            let line_number = index + 2;
            let fields = row(line, line_number, schema, 2)?;
            let ordinal = parse_canonical_u8(fields[0]).ok_or(Vs001ParseError::InvalidValue {
                schema,
                line: line_number,
                column: "ordinal",
            })?;
            if ordinal != (index + 1) as u8 {
                return Err(Vs001ParseError::OutOfOrder {
                    schema,
                    line: line_number,
                });
            }
            accesses.push(InvestigatorAccessV1 {
                ordinal,
                class: InvestigatorAccessClass::parse(fields[1]).ok_or(
                    Vs001ParseError::InvalidValue {
                        schema,
                        line: line_number,
                        column: "access_class",
                    },
                )?,
            });
        }
        Ok(Self { accesses })
    }

    pub fn accesses(&self) -> &[InvestigatorAccessV1] {
        &self.accesses
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropagationApplication {
    AppliedOnce,
    NotApplied,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropagationContamination {
    Clean,
    Contaminated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropagationDisposition {
    Pass,
    RejectedMissingRecordClass,
    ContaminationRecorded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PropagationObservationV1 {
    pub application: PropagationApplication,
    pub contamination: PropagationContamination,
    pub disposition: PropagationDisposition,
}

impl PropagationObservationV1 {
    /// The resulting parse says nothing about causal support: the circuit
    /// intentionally records delivery, encounter, and application separately.
    pub fn parse(bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::PropagationObservation;
        let parsed = table(bytes, schema, PROPAGATION_SCHEMA, PROPAGATION_HEADER)?;
        exact_rows(&parsed, schema, 1)?;
        let fields = row(parsed.rows[0], 3, schema, 7)?;
        if fields[0] != "t_stdout_capture_inquiry_r1"
            || fields[1] != "l1_spawn_stderr_method_r1"
            || fields[2] != "delivered"
            || fields[3] != "encountered"
        {
            return Err(Vs001ParseError::RecombinedRow { schema, line: 3 });
        }
        let (application, contamination, disposition) = match (fields[4], fields[5], fields[6]) {
            ("applied_once", "clean", "pass") => (
                PropagationApplication::AppliedOnce,
                PropagationContamination::Clean,
                PropagationDisposition::Pass,
            ),
            ("not_applied", "clean", "rejected_missing_record_class") => (
                PropagationApplication::NotApplied,
                PropagationContamination::Clean,
                PropagationDisposition::RejectedMissingRecordClass,
            ),
            ("not_applied", "contaminated", "contamination_recorded") => (
                PropagationApplication::NotApplied,
                PropagationContamination::Contaminated,
                PropagationDisposition::ContaminationRecorded,
            ),
            _ => return Err(Vs001ParseError::RecombinedRow { schema, line: 3 }),
        };
        Ok(Self {
            application,
            contamination,
            disposition,
        })
    }
}

// ---------------------------------------------------------------------------
// W1 disclosure frontier and leakage observation relation

const FRONTIER_MEMBERS_HEADER: &str = "opaque_ref";
const FRONTIER_SEQUESTERED_HEADER: &str = "reference_class\topaque_ref";
const FRONTIER_ACCESS_SCHEMA: &str = "# schema: FrontierAccessObservationV1/tsv-v1";
const FRONTIER_ACCESS_HEADER: &str =
    "principal\tlookup_route\treference_class\topaque_ref\tdisposition\taudit_placement";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierMember {
    SeedR1,
    ProjectCharterR1,
    HypothesisGraphR1,
    BaseSourceSnapshotR1,
    BehaviorObservationSetR1,
    DocumentationObservationSetR1,
    C1CuratedAccountR1,
}

impl FrontierMember {
    const ALL: [Self; 7] = [
        Self::SeedR1,
        Self::ProjectCharterR1,
        Self::HypothesisGraphR1,
        Self::BaseSourceSnapshotR1,
        Self::BehaviorObservationSetR1,
        Self::DocumentationObservationSetR1,
        Self::C1CuratedAccountR1,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SeedR1 => "seed_r1",
            Self::ProjectCharterR1 => "project_charter_r1",
            Self::HypothesisGraphR1 => "hypothesis_graph_r1",
            Self::BaseSourceSnapshotR1 => "base_source_snapshot_r1",
            Self::BehaviorObservationSetR1 => "behavior_observation_set_r1",
            Self::DocumentationObservationSetR1 => "documentation_observation_set_r1",
            Self::C1CuratedAccountR1 => "c1_curated_account_r1",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|member| member.as_str() == value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequesteredReferenceClass {
    C1Decision,
    CandidatePatch,
    PairedTaskTreatment,
    AdversarialReview,
    DeliveredCommit,
    Outcome,
    Retrospective,
    L1Lesson,
    RawPiSession,
    CurrentXshSnapshot,
}

impl SequesteredReferenceClass {
    const ALL: [Self; 10] = [
        Self::C1Decision,
        Self::CandidatePatch,
        Self::PairedTaskTreatment,
        Self::AdversarialReview,
        Self::DeliveredCommit,
        Self::Outcome,
        Self::Retrospective,
        Self::L1Lesson,
        Self::RawPiSession,
        Self::CurrentXshSnapshot,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::C1Decision => "c1_decision",
            Self::CandidatePatch => "candidate_patch",
            Self::PairedTaskTreatment => "paired_task_treatment",
            Self::AdversarialReview => "adversarial_review",
            Self::DeliveredCommit => "delivered_commit",
            Self::Outcome => "outcome",
            Self::Retrospective => "retrospective",
            Self::L1Lesson => "l1_lesson",
            Self::RawPiSession => "raw_pi_session",
            Self::CurrentXshSnapshot => "current_xsh_snapshot",
        }
    }

    pub const fn opaque_ref(self) -> &'static str {
        match self {
            Self::C1Decision => "sequestered_c1_decision_r1",
            Self::CandidatePatch => "sequestered_candidate_patch_r1",
            Self::PairedTaskTreatment => "sequestered_paired_task_r1",
            Self::AdversarialReview => "sequestered_adversarial_review_r1",
            Self::DeliveredCommit => "sequestered_delivered_commit_r1",
            Self::Outcome => "sequestered_outcome_r1",
            Self::Retrospective => "sequestered_retrospective_r1",
            Self::L1Lesson => "sequestered_l1_lesson_r1",
            Self::RawPiSession => "sequestered_raw_pi_session_r1",
            Self::CurrentXshSnapshot => "sequestered_current_xsh_r1",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|class| class.as_str() == value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SequesteredReferenceV1 {
    pub class: SequesteredReferenceClass,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisclosureFrontierV1 {
    members: [FrontierMember; 7],
    sequestered: [SequesteredReferenceV1; 10],
}

impl DisclosureFrontierV1 {
    /// Parses the two C1/W1 fixture relations.  This describes a declared
    /// frontier only; it is neither a capability check nor a disclosure grant.
    pub fn parse(members: &[u8], sequestered: &[u8]) -> Result<Self, Vs001ParseError> {
        let member_schema = Vs001Schema::FrontierMembers;
        let member_table = relation(members, member_schema, FRONTIER_MEMBERS_HEADER)?;
        exact_rows(&member_table, member_schema, 7)?;
        let mut parsed_members = Vec::with_capacity(7);
        for (index, line) in member_table.rows.iter().enumerate() {
            let line_number = index + 2;
            let fields = row(line, line_number, member_schema, 1)?;
            let member = FrontierMember::parse(fields[0]).ok_or(Vs001ParseError::InvalidValue {
                schema: member_schema,
                line: line_number,
                column: "opaque_ref",
            })?;
            if parsed_members.contains(&member) {
                return Err(Vs001ParseError::DuplicateIdentity {
                    schema: member_schema,
                    line: line_number,
                    identity: "opaque_ref",
                });
            }
            parsed_members.push(member);
        }
        if !FrontierMember::ALL
            .iter()
            .all(|required| parsed_members.contains(required))
        {
            return Err(Vs001ParseError::RecombinedRow {
                schema: member_schema,
                line: 2,
            });
        }

        let sequestered_schema = Vs001Schema::FrontierSequestered;
        let sequestered_table =
            relation(sequestered, sequestered_schema, FRONTIER_SEQUESTERED_HEADER)?;
        exact_rows(&sequestered_table, sequestered_schema, 10)?;
        let mut parsed_sequestered = Vec::with_capacity(10);
        for (index, line) in sequestered_table.rows.iter().enumerate() {
            let line_number = index + 2;
            let fields = row(line, line_number, sequestered_schema, 2)?;
            let class = SequesteredReferenceClass::parse(fields[0]).ok_or(
                Vs001ParseError::InvalidValue {
                    schema: sequestered_schema,
                    line: line_number,
                    column: "reference_class",
                },
            )?;
            if fields[1] != class.opaque_ref() {
                return Err(Vs001ParseError::RecombinedRow {
                    schema: sequestered_schema,
                    line: line_number,
                });
            }
            if parsed_sequestered
                .iter()
                .any(|entry: &SequesteredReferenceV1| entry.class == class)
            {
                return Err(Vs001ParseError::DuplicateIdentity {
                    schema: sequestered_schema,
                    line: line_number,
                    identity: "reference_class",
                });
            }
            parsed_sequestered.push(SequesteredReferenceV1 { class });
        }
        if !SequesteredReferenceClass::ALL.iter().all(|required| {
            parsed_sequestered
                .iter()
                .any(|entry| entry.class == *required)
        }) {
            return Err(Vs001ParseError::RecombinedRow {
                schema: sequestered_schema,
                line: 2,
            });
        }
        Ok(Self {
            members: parsed_members
                .try_into()
                .map_err(|_| Vs001ParseError::WrongRowCount {
                    schema: member_schema,
                    expected: 7,
                    actual: 0,
                })?,
            sequestered: parsed_sequestered.try_into().map_err(|_| {
                Vs001ParseError::WrongRowCount {
                    schema: sequestered_schema,
                    expected: 10,
                    actual: 0,
                }
            })?,
        })
    }

    pub const fn members(&self) -> &[FrontierMember; 7] {
        &self.members
    }

    pub const fn sequestered(&self) -> &[SequesteredReferenceV1; 10] {
        &self.sequestered
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierPrincipal {
    ReplayActor,
    Projector,
    OrdinaryInvestigator,
    GrandArchitectQueryClient,
}

impl FrontierPrincipal {
    const ALL: [Self; 4] = [
        Self::ReplayActor,
        Self::Projector,
        Self::OrdinaryInvestigator,
        Self::GrandArchitectQueryClient,
    ];

    const fn as_str(self) -> &'static str {
        match self {
            Self::ReplayActor => "replay_actor",
            Self::Projector => "projector",
            Self::OrdinaryInvestigator => "ordinary_investigator",
            Self::GrandArchitectQueryClient => "grand_architect_query_client",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierLookupRoute {
    DirectIdentity,
    GraphTraversal,
    ObjectDigest,
    CurrentRepositoryPath,
    CultureLookup,
    ProjectionLookup,
}

impl FrontierLookupRoute {
    const DENIED: [Self; 6] = [
        Self::DirectIdentity,
        Self::GraphTraversal,
        Self::ObjectDigest,
        Self::CurrentRepositoryPath,
        Self::CultureLookup,
        Self::ProjectionLookup,
    ];

    const fn as_str(self) -> &'static str {
        match self {
            Self::DirectIdentity => "direct_identity",
            Self::GraphTraversal => "graph_traversal",
            Self::ObjectDigest => "object_digest",
            Self::CurrentRepositoryPath => "current_repository_path",
            Self::CultureLookup => "culture_lookup",
            Self::ProjectionLookup => "projection_lookup",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierAccessReferenceV1 {
    Member(FrontierMember),
    Sequestered(SequesteredReferenceClass),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierAccessDisposition {
    Allowed,
    Denied,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierAuditPlacement {
    NoAudit,
    ContaminationAuditOutsideW1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrontierAccessObservationV1 {
    pub principal: FrontierPrincipal,
    pub lookup_route: FrontierLookupRoute,
    pub reference: FrontierAccessReferenceV1,
    pub disposition: FrontierAccessDisposition,
    pub audit_placement: FrontierAuditPlacement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierAccessObservationSetV1 {
    observations: Vec<FrontierAccessObservationV1>,
}

impl FrontierAccessObservationSetV1 {
    /// Parse the mechanically expanded W1 evidence matrix. An `Allowed` row
    /// is evidence emitted by this circuit, never a parsed permission grant.
    pub fn parse(frontier: &DisclosureFrontierV1, bytes: &[u8]) -> Result<Self, Vs001ParseError> {
        let schema = Vs001Schema::FrontierAccessObservation;
        let parsed = table(
            bytes,
            schema,
            FRONTIER_ACCESS_SCHEMA,
            FRONTIER_ACCESS_HEADER,
        )?;
        const EXPECTED_ROWS: usize = 4 * (7 + 10 * 6);
        exact_rows(&parsed, schema, EXPECTED_ROWS)?;
        let mut observations = Vec::with_capacity(EXPECTED_ROWS);
        let mut index = 0;
        for principal in FrontierPrincipal::ALL {
            for member in frontier.members {
                let line_number = index + 3;
                let expected = FrontierAccessObservationV1 {
                    principal,
                    lookup_route: FrontierLookupRoute::DirectIdentity,
                    reference: FrontierAccessReferenceV1::Member(member),
                    disposition: FrontierAccessDisposition::Allowed,
                    audit_placement: FrontierAuditPlacement::NoAudit,
                };
                let observed =
                    parse_frontier_access_row(parsed.rows[index], line_number, expected)?;
                observations.push(observed);
                index += 1;
            }
            for sequestered in frontier.sequestered {
                for route in FrontierLookupRoute::DENIED {
                    let line_number = index + 3;
                    let expected = FrontierAccessObservationV1 {
                        principal,
                        lookup_route: route,
                        reference: FrontierAccessReferenceV1::Sequestered(sequestered.class),
                        disposition: FrontierAccessDisposition::Denied,
                        audit_placement: FrontierAuditPlacement::ContaminationAuditOutsideW1,
                    };
                    let observed =
                        parse_frontier_access_row(parsed.rows[index], line_number, expected)?;
                    observations.push(observed);
                    index += 1;
                }
            }
        }
        Ok(Self { observations })
    }

    pub fn observations(&self) -> &[FrontierAccessObservationV1] {
        &self.observations
    }
}

fn parse_frontier_access_row(
    line: &str,
    line_number: usize,
    expected: FrontierAccessObservationV1,
) -> Result<FrontierAccessObservationV1, Vs001ParseError> {
    let schema = Vs001Schema::FrontierAccessObservation;
    let fields = row(line, line_number, schema, 6)?;
    let (reference_class, opaque_ref) = match expected.reference {
        FrontierAccessReferenceV1::Member(member) => ("frontier_member", member.as_str()),
        FrontierAccessReferenceV1::Sequestered(class) => (class.as_str(), class.opaque_ref()),
    };
    let disposition_text = match expected.disposition {
        FrontierAccessDisposition::Allowed => "allowed",
        FrontierAccessDisposition::Denied => "denied",
    };
    let audit_text = match expected.audit_placement {
        FrontierAuditPlacement::NoAudit => "no_audit",
        FrontierAuditPlacement::ContaminationAuditOutsideW1 => "contamination_audit_outside_w1",
    };
    if fields
        != [
            expected.principal.as_str(),
            expected.lookup_route.as_str(),
            reference_class,
            opaque_ref,
            disposition_text,
            audit_text,
        ]
    {
        return Err(Vs001ParseError::RecombinedRow {
            schema,
            line: line_number,
        });
    }
    Ok(expected)
}
