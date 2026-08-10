//! Closed interpretation of the C1 direct evaluator's stdout.
//!
//! This is an XSH-only semantic boundary. Its caller supplies the completed
//! stdout bytes and the byte digest declared for that occurrence. Matching the
//! digest prevents local byte substitution; it does not prove execution,
//! custody, sealing, provenance, process reaping, or evidence admission.
//! Generic custody need not know this TSV grammar or either result variant.

use society_content::ContentDigest;

use crate::{
    CurationContractOutputsV1, Vs001CurationDirectOutputRoleV1, MAX_DIRECT_CURATION_STDOUT_BYTES,
    VS001_CURATION_DIRECT_OUTPUT_PACKAGE_SCHEMA,
};

/// Application-only interpretation of one declared direct-evaluator stdout
/// rendering. It is syntactic classification only, never an authorization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CurationDirectSemanticResultV1 {
    Accepted(CurationDirectSemanticObservationV1),
    Rejected(CurationDirectStdoutRejectionV1),
}

/// The closed XSH meaning of a canonical direct C1 stdout rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurationDirectSemanticObservationV1 {
    pub stdout_blake3: ContentDigest,
    pub outputs: CurationContractOutputsV1,
}

/// The complete failure vocabulary at this application parsing boundary.
/// Deliberately omits transport, process, custody, and authority explanations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CurationDirectStdoutRejectionV1 {
    Empty,
    TooLarge,
    DigestMismatch,
    InvalidOutputPackage,
}

/// Interpret one exact byte rendering under a declared BLAKE3 identity.
///
/// The declaration is only a byte-equality input to this pure application
/// adapter. A later generic bridge may establish where it came from; this
/// function neither receives nor returns any generic identity or receipt.
pub fn interpret_curation_direct_stdout_v1(
    stdout: &[u8],
    declared_blake3: ContentDigest,
) -> CurationDirectSemanticResultV1 {
    if stdout.is_empty() {
        return CurationDirectSemanticResultV1::Rejected(CurationDirectStdoutRejectionV1::Empty);
    }
    if stdout.len() > MAX_DIRECT_CURATION_STDOUT_BYTES {
        return CurationDirectSemanticResultV1::Rejected(CurationDirectStdoutRejectionV1::TooLarge);
    }
    if ContentDigest::of_bytes(stdout) != declared_blake3 {
        return CurationDirectSemanticResultV1::Rejected(
            CurationDirectStdoutRejectionV1::DigestMismatch,
        );
    }
    match parse_output_package(stdout) {
        Ok(outputs) => {
            CurationDirectSemanticResultV1::Accepted(CurationDirectSemanticObservationV1 {
                stdout_blake3: declared_blake3,
                outputs,
            })
        }
        Err(_) => CurationDirectSemanticResultV1::Rejected(
            CurationDirectStdoutRejectionV1::InvalidOutputPackage,
        ),
    }
}

fn parse_output_package(stdout: &[u8]) -> Result<CurationContractOutputsV1, ()> {
    let mut cursor = 0;
    if next_line(stdout, &mut cursor)? != VS001_CURATION_DIRECT_OUTPUT_PACKAGE_SCHEMA.as_bytes() {
        return Err(());
    }

    let mut members = Vec::with_capacity(Vs001CurationDirectOutputRoleV1::ORDERED.len());
    for role in Vs001CurationDirectOutputRoleV1::ORDERED {
        let (actual_role, length) = parse_frame_header(next_line(stdout, &mut cursor)?)?;
        if actual_role != role.wire_name() || length > stdout.len().saturating_sub(cursor) {
            return Err(());
        }
        members.push(&stdout[cursor..cursor + length]);
        cursor += length;
    }
    if cursor != stdout.len() {
        return Err(());
    }
    let [observation, raw_evidence_escalation] = members.try_into().map_err(|_| ())?;
    CurationContractOutputsV1::parse(observation, raw_evidence_escalation).map_err(|_| ())
}

fn next_line<'a>(source: &'a [u8], cursor: &mut usize) -> Result<&'a [u8], ()> {
    let remaining = source.get(*cursor..).ok_or(())?;
    let newline = remaining.iter().position(|byte| *byte == b'\n').ok_or(())?;
    let line = &remaining[..newline];
    *cursor += newline + 1;
    Ok(line)
}

fn parse_frame_header(line: &[u8]) -> Result<(&str, usize), ()> {
    let line = std::str::from_utf8(line).map_err(|_| ())?;
    let (role, length) = line.split_once('\t').ok_or(())?;
    if role.is_empty()
        || length.is_empty()
        || (length.len() > 1 && length.starts_with('0'))
        || !length.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(());
    }
    let length = length.parse().map_err(|_| ())?;
    Ok((role, length))
}
