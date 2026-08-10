#![allow(clippy::unwrap_used)]

use society_content::ContentDigest;
use society_xsh_circuit::{
    interpret_curation_direct_stdout_v1, CurationDirectSemanticResultV1,
    CurationDirectStdoutRejectionV1, CurationRawEscalationState, MAX_DIRECT_CURATION_STDOUT_BYTES,
};

const NONE_REQUESTED: &[u8] = include_bytes!("fixtures/curation-direct-output.none.v1.framed");
const NAMED_REQUEST: &[u8] = include_bytes!("fixtures/curation-direct-output.named.v1.framed");
const NONE_REQUESTED_OBSERVATION: &[u8] =
    include_bytes!("fixtures/curation-contract-observation.none.v1.tsv");
const NAMED_REQUEST_RAW_ESCALATION: &[u8] = concat!(
    "# schema: CurationRawEvidenceEscalationObservationV1/tsv-v1\n",
    "ordinal\tquestion_ref\tobject_ref\n",
    "1\tresolve_detached_contract_intent\traw_pi_session_object\n"
)
.as_bytes();
const NONE_REQUESTED_RAW_ESCALATION: &[u8] = concat!(
    "# schema: CurationRawEvidenceEscalationObservationV1/tsv-v1\n",
    "ordinal\tquestion_ref\tobject_ref\n"
)
.as_bytes();

#[test]
fn direct_curation_stdout_accepts_each_checked_in_complete_canonical_output_package() {
    let cases = [
        (
            NONE_REQUESTED,
            CurationRawEscalationState::NoneRequested,
            false,
        ),
        (
            NAMED_REQUEST,
            CurationRawEscalationState::NamedRequestPresent,
            true,
        ),
    ];

    for (stdout, expected_escalations, expected_named_request) in cases {
        let digest = ContentDigest::of_bytes(stdout);
        assert!(matches!(
            interpret_curation_direct_stdout_v1(stdout, digest),
            CurationDirectSemanticResultV1::Accepted(observation)
                if observation.stdout_blake3 == digest
                    && observation.outputs.observation.raw_escalations == expected_escalations
                    && observation.outputs.raw_evidence_escalation.request().is_some()
                        == expected_named_request
        ));
    }
}

#[test]
fn direct_curation_stdout_rejects_recombined_complete_outputs() {
    let changed = framed_output(NONE_REQUESTED_OBSERVATION, NAMED_REQUEST_RAW_ESCALATION);

    assert_eq!(
        interpret_curation_direct_stdout_v1(&changed, ContentDigest::of_bytes(&changed)),
        CurationDirectSemanticResultV1::Rejected(
            CurationDirectStdoutRejectionV1::InvalidOutputPackage
        )
    );
}

#[test]
fn direct_curation_stdout_rejects_malformed_role_order_length_and_trailing_bytes() {
    let mut wrong_role = NONE_REQUESTED.to_vec();
    let position = wrong_role
        .windows(b"contract_observation".len())
        .position(|window| window == b"contract_observation")
        .unwrap();
    wrong_role[position] = b'x';

    let wrong_order = framed_output_members(
        "raw_evidence_escalation_observation",
        NONE_REQUESTED_RAW_ESCALATION,
        "contract_observation",
        NONE_REQUESTED_OBSERVATION,
    );

    let mut wrong_length = NONE_REQUESTED.to_vec();
    let length_position = wrong_length
        .windows(b"contract_observation\t353".len())
        .position(|window| window == b"contract_observation\t353")
        .unwrap()
        + b"contract_observation\t".len();
    wrong_length[length_position..length_position + 3].copy_from_slice(b"354");

    let mut trailing = NONE_REQUESTED.to_vec();
    trailing.extend_from_slice(b"unexpected");

    for malformed in [wrong_role, wrong_order, wrong_length, trailing] {
        assert_eq!(
            interpret_curation_direct_stdout_v1(&malformed, ContentDigest::of_bytes(&malformed)),
            CurationDirectSemanticResultV1::Rejected(
                CurationDirectStdoutRejectionV1::InvalidOutputPackage
            )
        );
    }
}

#[test]
fn direct_curation_stdout_rejects_a_digest_for_different_bytes() {
    assert_eq!(
        interpret_curation_direct_stdout_v1(NAMED_REQUEST, ContentDigest::of_bytes(NONE_REQUESTED)),
        CurationDirectSemanticResultV1::Rejected(CurationDirectStdoutRejectionV1::DigestMismatch)
    );
}

fn framed_output(observation: &[u8], raw_evidence_escalation: &[u8]) -> Vec<u8> {
    framed_output_members(
        "contract_observation",
        observation,
        "raw_evidence_escalation_observation",
        raw_evidence_escalation,
    )
}

fn framed_output_members(
    first_role: &str,
    first_member: &[u8],
    second_role: &str,
    second_member: &[u8],
) -> Vec<u8> {
    let mut output = b"# schema: Vs001CurationDirectOutputPackageV1/framed-v1\n".to_vec();
    output.extend_from_slice(format!("{first_role}\t{}\n", first_member.len()).as_bytes());
    output.extend_from_slice(first_member);
    output.extend_from_slice(format!("{second_role}\t{}\n", second_member.len()).as_bytes());
    output.extend_from_slice(second_member);
    output
}

#[test]
fn direct_curation_stdout_rejects_empty_and_over_bound_renderings() {
    assert_eq!(
        interpret_curation_direct_stdout_v1(b"", ContentDigest::of_bytes(b"")),
        CurationDirectSemanticResultV1::Rejected(CurationDirectStdoutRejectionV1::Empty)
    );

    let oversized = vec![b'x'; MAX_DIRECT_CURATION_STDOUT_BYTES + 1];
    assert_eq!(
        interpret_curation_direct_stdout_v1(&oversized, ContentDigest::of_bytes(&oversized)),
        CurationDirectSemanticResultV1::Rejected(CurationDirectStdoutRejectionV1::TooLarge)
    );
}
