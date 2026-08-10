//! XSH-owned direct CurationContract evaluator.
//!
//! This module is pure application parsing and evaluation. The dedicated binary
//! owns its bounded manifest read and stdout write; no library module performs
//! filesystem or process work.

use thiserror::Error;

use crate::{CanonicalEvaluatorInputRenderingV1, EvaluatorPortError, Vs001EvaluatorProgramV1};

const INPUT_SCHEMA: &[u8] = b"# schema: Vs001CurationDirectInputManifestV1/framed-v1";
pub const MAX_DIRECT_CURATION_MANIFEST_BYTES: usize = 128 * 1024;
const CURATION_OBSERVATION_SCHEMA: &str = "# schema: CurationContractObservationV1/tsv-v1";
const CURATION_OBSERVATION_HEADER: &str =
    "account_kind\tpurpose\thypothesis_coverage\tcounterevidence\tpreserved_conflict\tunknowns\texclusions\traw_escalations\tfrontier_admission\tdisposition";
const MAX_RELATION_BYTES: usize = 32 * 1024;

/// Fixed member roles for the one self-contained curation evaluator package.
/// Their bytes appear, in this exact order and with canonical lengths, in the
/// one sealed input manifest. This is not a path manifest, metadata map,
/// durable package ID, or build provenance claim.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Vs001CurationDirectEvaluatorPackageV1;

impl Vs001CurationDirectEvaluatorPackageV1 {
    pub const fn program(self) -> Vs001EvaluatorProgramV1 {
        Vs001EvaluatorProgramV1::CurationContract
    }

    pub const fn member_roles(self) -> &'static [Vs001CurationInputRoleV1; 7] {
        &Vs001CurationInputRoleV1::ORDERED
    }
}

/// Closed byte members of the curation evaluator input package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vs001CurationInputRoleV1 {
    Account,
    SelectedItems,
    PreservedConflicts,
    DecisionRelevantUnknowns,
    Exclusions,
    RawEvidenceEscalations,
    FrontierMembers,
}

impl Vs001CurationInputRoleV1 {
    pub const ORDERED: [Self; 7] = [
        Self::Account,
        Self::SelectedItems,
        Self::PreservedConflicts,
        Self::DecisionRelevantUnknowns,
        Self::Exclusions,
        Self::RawEvidenceEscalations,
        Self::FrontierMembers,
    ];

    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::SelectedItems => "selected_items",
            Self::PreservedConflicts => "preserved_conflicts",
            Self::DecisionRelevantUnknowns => "decision_relevant_unknowns",
            Self::Exclusions => "exclusions",
            Self::RawEvidenceEscalations => "raw_evidence_escalations",
            Self::FrontierMembers => "frontier_members",
        }
    }
}

/// Exact one-file curation input. The outer rendering is the sole byte object
/// that generic custody needs to verify and seal. No member becomes an
/// independent path or object identity at this boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vs001DirectEvaluatorInputManifestV1 {
    rendering: CanonicalEvaluatorInputRenderingV1,
    curation: CurationInputRelationsV1,
}

impl Vs001DirectEvaluatorInputManifestV1 {
    pub fn parse(bytes: &[u8]) -> Result<Self, DirectEvaluatorInputManifestError> {
        let rendering = CanonicalEvaluatorInputRenderingV1::from_bytes(bytes.to_vec())
            .map_err(DirectEvaluatorInputManifestError::InvalidRendering)?;
        let mut cursor = 0;
        if next_line(bytes, &mut cursor)? != INPUT_SCHEMA {
            return Err(DirectEvaluatorInputManifestError::UnexpectedSchema);
        }

        let mut relations = Vec::with_capacity(Vs001CurationInputRoleV1::ORDERED.len());
        for role in Vs001CurationInputRoleV1::ORDERED {
            let (actual_role, length) = parse_frame_header(next_line(bytes, &mut cursor)?)?;
            if actual_role != role.wire_name() {
                return Err(DirectEvaluatorInputManifestError::UnexpectedRelationRole);
            }
            if length > MAX_RELATION_BYTES || cursor + length > bytes.len() {
                return Err(DirectEvaluatorInputManifestError::InvalidRelationLength);
            }
            relations.push(bytes[cursor..cursor + length].to_vec());
            cursor += length;
        }
        if cursor != bytes.len() {
            return Err(DirectEvaluatorInputManifestError::TrailingBytes);
        }

        let [account, selected_items, preserved_conflicts, decision_relevant_unknowns, exclusions, raw_evidence_escalations, frontier_members] =
            relations
                .try_into()
                .expect("seven closed roles produce seven relation values");
        Ok(Self {
            rendering,
            curation: CurationInputRelationsV1 {
                account,
                selected_items,
                preserved_conflicts,
                decision_relevant_unknowns,
                exclusions,
                raw_evidence_escalations,
                frontier_members,
            },
        })
    }

    pub const fn program(&self) -> Vs001EvaluatorProgramV1 {
        Vs001EvaluatorProgramV1::CurationContract
    }

    pub const fn canonical_rendering(&self) -> &CanonicalEvaluatorInputRenderingV1 {
        &self.rendering
    }

    pub fn evaluate(&self) -> Result<DirectCurationContractObservationV1, CurationContractError> {
        self.curation.evaluate()
    }
}

fn next_line<'a>(
    source: &'a [u8],
    cursor: &mut usize,
) -> Result<&'a [u8], DirectEvaluatorInputManifestError> {
    let remaining = &source[*cursor..];
    let Some(newline) = remaining.iter().position(|byte| *byte == b'\n') else {
        return Err(DirectEvaluatorInputManifestError::MissingFrameNewline);
    };
    let line = &remaining[..newline];
    *cursor += newline + 1;
    Ok(line)
}

fn parse_frame_header(line: &[u8]) -> Result<(&str, usize), DirectEvaluatorInputManifestError> {
    let line =
        std::str::from_utf8(line).map_err(|_| DirectEvaluatorInputManifestError::InvalidFrame)?;
    let Some((role, length)) = line.split_once('\t') else {
        return Err(DirectEvaluatorInputManifestError::InvalidFrame);
    };
    if role.is_empty()
        || length.is_empty()
        || (length.len() > 1 && length.starts_with('0'))
        || !length.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(DirectEvaluatorInputManifestError::InvalidFrame);
    }
    let length = length
        .parse()
        .map_err(|_| DirectEvaluatorInputManifestError::InvalidFrame)?;
    Ok((role, length))
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum DirectEvaluatorInputManifestError {
    #[error("direct curation input rendering is invalid: {0}")]
    InvalidRendering(EvaluatorPortError),
    #[error("unexpected direct curation input schema")]
    UnexpectedSchema,
    #[error("direct curation input frame is missing its newline")]
    MissingFrameNewline,
    #[error("invalid direct curation input frame")]
    InvalidFrame,
    #[error("unexpected direct curation relation role or order")]
    UnexpectedRelationRole,
    #[error("invalid direct curation relation length")]
    InvalidRelationLength,
    #[error("trailing bytes after the final direct curation relation")]
    TrailingBytes,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CurationInputRelationsV1 {
    account: Vec<u8>,
    selected_items: Vec<u8>,
    preserved_conflicts: Vec<u8>,
    decision_relevant_unknowns: Vec<u8>,
    exclusions: Vec<u8>,
    raw_evidence_escalations: Vec<u8>,
    frontier_members: Vec<u8>,
}

impl CurationInputRelationsV1 {
    fn evaluate(&self) -> Result<DirectCurationContractObservationV1, CurationContractError> {
        let account = ParsedTsvV1::parse(&self.account)?;
        let selected_items = ParsedTsvV1::parse(&self.selected_items)?;
        let preserved_conflicts = ParsedTsvV1::parse(&self.preserved_conflicts)?;
        let decision_relevant_unknowns = ParsedTsvV1::parse(&self.decision_relevant_unknowns)?;
        let exclusions = ParsedTsvV1::parse(&self.exclusions)?;
        let raw_evidence_escalations = ParsedTsvV1::parse(&self.raw_evidence_escalations)?;
        let frontier_members = ParsedTsvV1::parse(&self.frontier_members)?;

        validate_frontier_members(&frontier_members)?;
        validate_account(&account)?;
        validate_selected_items(&selected_items, &frontier_members)?;
        validate_one_column_relation(
            &preserved_conflicts,
            "conflict_ref",
            "conflict_lang_vs_spec",
            "preserved-conflicts",
        )?;
        validate_one_column_relation(
            &decision_relevant_unknowns,
            "unknown_ref",
            "detached_public_contract_intent",
            "decision-relevant-unknowns",
        )?;
        validate_exclusions(&exclusions)?;
        let raw_escalations = validate_raw_evidence_escalations(&raw_evidence_escalations)?;

        Ok(DirectCurationContractObservationV1 { raw_escalations })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectCurationContractObservationV1 {
    raw_escalations: RawEvidenceEscalationStateV1,
}

impl DirectCurationContractObservationV1 {
    pub fn canonical_tsv(self) -> String {
        format!(
            "{CURATION_OBSERVATION_SCHEMA}\n{CURATION_OBSERVATION_HEADER}\ndecision_curation\tauthorize_spawn_stderr_prototype\th1_h2_h3_with_dissent\tdeclared\tpreserved\tdeclared\tsemantic\t{}\tfrontier_constrained\tacceptance_ready\n",
            self.raw_escalations.as_wire(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RawEvidenceEscalationStateV1 {
    NoneRequested,
    NamedRequestPresent,
}

impl RawEvidenceEscalationStateV1 {
    const fn as_wire(self) -> &'static str {
        match self {
            Self::NoneRequested => "none_requested",
            Self::NamedRequestPresent => "named_request_present",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum CurationContractError {
    #[error("curation relation is not valid UTF-8 TSV")]
    NotUtf8,
    #[error("curation relation must terminate each row with a newline")]
    MissingFinalNewline,
    #[error("curation relation has an unexpected header or row shape: {0}")]
    InvalidRelation(&'static str),
    #[error("curation relation violates the C1 contract: {0}")]
    ContractViolation(&'static str),
}

#[derive(Clone, Debug)]
struct ParsedTsvV1 {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl ParsedTsvV1 {
    fn parse(bytes: &[u8]) -> Result<Self, CurationContractError> {
        let source = std::str::from_utf8(bytes).map_err(|_| CurationContractError::NotUtf8)?;
        if !source.ends_with('\n') {
            return Err(CurationContractError::MissingFinalNewline);
        }
        let mut lines = source.split_terminator('\n');
        let header = lines
            .next()
            .ok_or(CurationContractError::InvalidRelation("missing header"))?
            .split('\t')
            .map(str::to_owned)
            .collect();
        let mut rows = Vec::new();
        for line in lines {
            if line.is_empty() {
                return Err(CurationContractError::InvalidRelation("empty row"));
            }
            rows.push(line.split('\t').map(str::to_owned).collect());
        }
        Ok(Self { header, rows })
    }

    fn require_header(&self, expected: &[&str]) -> Result<(), CurationContractError> {
        if self
            .header
            .iter()
            .map(String::as_str)
            .eq(expected.iter().copied())
        {
            Ok(())
        } else {
            Err(CurationContractError::InvalidRelation("unexpected header"))
        }
    }

    fn require_rows(&self, expected: usize) -> Result<(), CurationContractError> {
        if self.rows.len() == expected {
            Ok(())
        } else {
            Err(CurationContractError::InvalidRelation(
                "unexpected row count",
            ))
        }
    }
}

const FRONTIER_MEMBERS: [&str; 7] = [
    "argument_h1",
    "argument_h2",
    "argument_h3",
    "observation_managed_behavior",
    "observation_detached_behavior",
    "observation_documentation_conflict",
    "conflict_lang_vs_spec",
];

fn validate_frontier_members(table: &ParsedTsvV1) -> Result<(), CurationContractError> {
    table.require_header(&["source_ref"])?;
    table.require_rows(FRONTIER_MEMBERS.len())?;
    let mut seen = [false; FRONTIER_MEMBERS.len()];
    for row in &table.rows {
        let [source_ref] = row.as_slice() else {
            return Err(CurationContractError::InvalidRelation("frontier row shape"));
        };
        let Some(index) = FRONTIER_MEMBERS
            .iter()
            .position(|expected| source_ref == expected)
        else {
            return Err(CurationContractError::ContractViolation(
                "frontier source_ref",
            ));
        };
        if std::mem::replace(&mut seen[index], true) {
            return Err(CurationContractError::ContractViolation(
                "duplicate frontier source_ref",
            ));
        }
    }
    if seen.into_iter().all(std::convert::identity) {
        Ok(())
    } else {
        Err(CurationContractError::ContractViolation(
            "missing frontier source_ref",
        ))
    }
}

fn validate_account(table: &ParsedTsvV1) -> Result<(), CurationContractError> {
    table.require_header(&[
        "kind",
        "purpose",
        "question_revision",
        "disclosure_frontier",
        "curator_configuration",
        "leading_hypothesis",
        "strongest_counterevidence_ref",
    ])?;
    table.require_rows(1)?;
    let expected = [
        "decision_curation",
        "authorize_spawn_stderr_prototype",
        "q_vs001_spawn_stderr_r1",
        "fc1",
        "curator_v1",
        "h2",
        "argument_h3",
    ];
    if table.rows[0].iter().map(String::as_str).eq(expected) {
        Ok(())
    } else {
        Err(CurationContractError::ContractViolation("account identity"))
    }
}

const SELECTED_ITEMS: [(&str, &str, &str); 6] = [
    ("argument_h1", "defeating_argument", "h1_rejected"),
    (
        "argument_h2",
        "supporting_argument",
        "h2_partially_supported",
    ),
    ("argument_h3", "dissent", "h3_supported"),
    (
        "observation_managed_behavior",
        "observation",
        "direct_owned_path_result",
    ),
    (
        "observation_detached_behavior",
        "dissent",
        "detached_policy_distinction",
    ),
    (
        "observation_documentation_conflict",
        "constraint",
        "stale_discovery_conflict",
    ),
];

fn validate_selected_items(
    table: &ParsedTsvV1,
    frontier: &ParsedTsvV1,
) -> Result<(), CurationContractError> {
    table.require_header(&[
        "ordinal",
        "source_ref",
        "role",
        "selection_reason",
        "applicability_scope",
    ])?;
    table.require_rows(SELECTED_ITEMS.len())?;
    let mut seen = [false; SELECTED_ITEMS.len()];
    for (ordinal, row) in table.rows.iter().enumerate() {
        let [actual_ordinal, source_ref, role, reason, scope] = row.as_slice() else {
            return Err(CurationContractError::InvalidRelation(
                "selected-item row shape",
            ));
        };
        if actual_ordinal != &(ordinal + 1).to_string() || scope != "spawn_stderr_prototype" {
            return Err(CurationContractError::ContractViolation(
                "selected-item ordinal or scope",
            ));
        }
        if !frontier.rows.iter().any(|frontier_row| {
            frontier_row
                .first()
                .is_some_and(|candidate| candidate == source_ref)
        }) {
            return Err(CurationContractError::ContractViolation(
                "selection source outside frontier",
            ));
        }
        let Some(index) = SELECTED_ITEMS
            .iter()
            .position(|(expected_source, _, _)| source_ref == expected_source)
        else {
            return Err(CurationContractError::ContractViolation(
                "selected source_ref",
            ));
        };
        if std::mem::replace(&mut seen[index], true) {
            return Err(CurationContractError::ContractViolation(
                "duplicate selected source_ref",
            ));
        }
        let (_, expected_role, expected_reason) = SELECTED_ITEMS[index];
        if role != expected_role || reason != expected_reason {
            return Err(CurationContractError::ContractViolation(
                "selected-item semantics",
            ));
        }
    }
    if seen.into_iter().all(std::convert::identity) {
        Ok(())
    } else {
        Err(CurationContractError::ContractViolation(
            "missing selected source_ref",
        ))
    }
}

fn validate_one_column_relation(
    table: &ParsedTsvV1,
    header: &str,
    value: &str,
    relation: &'static str,
) -> Result<(), CurationContractError> {
    table.require_header(&[header])?;
    table.require_rows(1)?;
    if table.rows[0].as_slice() == [value] {
        Ok(())
    } else {
        Err(CurationContractError::ContractViolation(relation))
    }
}

const EXCLUSIONS: [(&str, &str, &str); 2] = [
    (
        "raw_pi_session",
        "no_named_question",
        "decision_context_may_miss_relevant_reasoning",
    ),
    (
        "candidate_patch",
        "post_frontier_material",
        "would_leak_future_product_choice",
    ),
];

fn validate_exclusions(table: &ParsedTsvV1) -> Result<(), CurationContractError> {
    table.require_header(&["category_or_source", "reason", "risk_if_wrong"])?;
    table.require_rows(EXCLUSIONS.len())?;
    let mut seen = [false; EXCLUSIONS.len()];
    for row in &table.rows {
        let [category, reason, risk] = row.as_slice() else {
            return Err(CurationContractError::InvalidRelation(
                "exclusion row shape",
            ));
        };
        let Some(index) = EXCLUSIONS
            .iter()
            .position(|(expected_category, _, _)| category == expected_category)
        else {
            return Err(CurationContractError::ContractViolation(
                "exclusion category",
            ));
        };
        if std::mem::replace(&mut seen[index], true) {
            return Err(CurationContractError::ContractViolation(
                "duplicate exclusion",
            ));
        }
        let (_, expected_reason, expected_risk) = EXCLUSIONS[index];
        if reason != expected_reason || risk != expected_risk {
            return Err(CurationContractError::ContractViolation(
                "exclusion semantics",
            ));
        }
    }
    if seen.into_iter().all(std::convert::identity) {
        Ok(())
    } else {
        Err(CurationContractError::ContractViolation(
            "missing exclusion",
        ))
    }
}

fn validate_raw_evidence_escalations(
    table: &ParsedTsvV1,
) -> Result<RawEvidenceEscalationStateV1, CurationContractError> {
    table.require_header(&["question_ref", "object_ref"])?;
    match table.rows.as_slice() {
        [] => Ok(RawEvidenceEscalationStateV1::NoneRequested),
        [row]
            if row.as_slice() == ["resolve_detached_contract_intent", "raw_pi_session_object"] =>
        {
            Ok(RawEvidenceEscalationStateV1::NamedRequestPresent)
        }
        [_] => Err(CurationContractError::ContractViolation(
            "raw-evidence escalation",
        )),
        _ => Err(CurationContractError::InvalidRelation(
            "raw-evidence escalation count",
        )),
    }
}
