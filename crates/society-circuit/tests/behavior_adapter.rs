#![allow(clippy::unwrap_used)]

use society_circuit::{
    BehaviorCaseId, BehaviorObservationSetV1, BehaviorParseError, ExitOrErrorShape,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/behavior-observations.v1.tsv");

fn replace_once(source: &[u8], from: &str, to: &str) -> Vec<u8> {
    String::from_utf8(source.to_vec())
        .unwrap()
        .replacen(from, to, 1)
        .into_bytes()
}

#[test]
fn actual_behavior_matrix_fixture_becomes_eleven_closed_observations() {
    let parsed = BehaviorObservationSetV1::parse(FIXTURE).unwrap();
    assert_eq!(parsed.observations()[0].case_id, BehaviorCaseId::B01);
    assert_eq!(parsed.observations()[10].case_id, BehaviorCaseId::B11);
    assert!(matches!(
        parsed.observations()[8].exit_shape,
        ExitOrErrorShape::Exited(status) if status.value() == 23
    ));
}

#[test]
fn known_values_cannot_be_recombined_into_another_case_manifest() {
    let changed = replace_once(FIXTURE, "B02\tspawn_command", "B02\tprocess_run");
    assert!(matches!(
        BehaviorObservationSetV1::parse(&changed),
        Err(BehaviorParseError::CaseManifestMismatch { line: 4 })
    ));
}

#[test]
fn duplicate_or_reordered_case_identity_is_not_a_partial_set() {
    let fixture = String::from_utf8(FIXTURE.to_vec()).unwrap();
    let mut lines: Vec<_> = fixture.lines().collect();
    lines[3] = lines[2];
    let changed = format!("{}\n", lines.join("\n")).into_bytes();
    assert!(matches!(
        BehaviorObservationSetV1::parse(&changed),
        Err(BehaviorParseError::CaseOutOfOrder { line: 4, .. })
    ));
}

#[test]
fn stream_kind_and_digest_are_one_closed_union() {
    let changed = replace_once(
        FIXTURE,
        "redirected\tc861c7e9a5d86a3545e8d2bf25b69d692cf3df262f612f9088b5fc9bd220efaf",
        "redirected\t-",
    );
    assert!(matches!(
        BehaviorObservationSetV1::parse(&changed),
        Err(BehaviorParseError::InvalidStreamEvidence { line: 3, .. })
    ));
    let changed = replace_once(
        FIXTURE,
        "inherited_parent_stdout\t-",
        "inherited_parent_stdout\tc861c7e9a5d86a3545e8d2bf25b69d692cf3df262f612f9088b5fc9bd220efaf",
    );
    assert!(matches!(
        BehaviorObservationSetV1::parse(&changed),
        Err(BehaviorParseError::InvalidStreamEvidence { .. })
    ));
    let changed = replace_once(
        FIXTURE,
        "inherited_parent_stdout\t-",
        "inherited_parent_stderr\t-",
    );
    assert!(matches!(
        BehaviorObservationSetV1::parse(&changed),
        Err(BehaviorParseError::InvalidStreamEvidence { line: 7, .. })
    ));
}

#[test]
fn framing_header_arity_utf8_and_size_fail_before_a_partial_parse() {
    let without_terminal_lf = &FIXTURE[..FIXTURE.len() - 1];
    assert_eq!(
        BehaviorObservationSetV1::parse(without_terminal_lf),
        Err(BehaviorParseError::MissingTerminalLf)
    );
    let mut double_terminal_lf = FIXTURE.to_vec();
    double_terminal_lf.push(b'\n');
    assert_eq!(
        BehaviorObservationSetV1::parse(&double_terminal_lf),
        Err(BehaviorParseError::ExtraRow)
    );

    let changed = replace_once(FIXTURE, "# schema:", "# unknown:");
    assert_eq!(
        BehaviorObservationSetV1::parse(&changed),
        Err(BehaviorParseError::WrongSchema)
    );
    let changed = replace_once(FIXTURE, "B01\tprocess_run", "B01\textra\tprocess_run");
    assert!(matches!(
        BehaviorObservationSetV1::parse(&changed),
        Err(BehaviorParseError::WrongFieldCount { line: 3, .. })
    ));
    let mut invalid_utf8 = FIXTURE.to_vec();
    invalid_utf8[0] = 0xff;
    assert_eq!(
        BehaviorObservationSetV1::parse(&invalid_utf8),
        Err(BehaviorParseError::InvalidUtf8)
    );
    assert_eq!(
        BehaviorObservationSetV1::parse(&vec![b'x'; 64 * 1024 + 1]),
        Err(BehaviorParseError::FrameTooLarge)
    );

    let changed = replace_once(FIXTURE, "\n", "\r\n");
    assert_eq!(
        BehaviorObservationSetV1::parse(&changed),
        Err(BehaviorParseError::NonCanonicalLineEnding)
    );
    let mut extra = FIXTURE.to_vec();
    extra.extend_from_slice(b"unexpected\n");
    assert_eq!(
        BehaviorObservationSetV1::parse(&extra),
        Err(BehaviorParseError::ExtraRow)
    );
    let changed = replace_once(FIXTURE, "\nB02\t", "\n\nB02\t");
    assert!(matches!(
        BehaviorObservationSetV1::parse(&changed),
        Err(BehaviorParseError::WrongFieldCount { line: 4, .. })
    ));
    let changed = replace_once(FIXTURE, "exited_23", "exited_023");
    assert!(matches!(
        BehaviorObservationSetV1::parse(&changed),
        Err(BehaviorParseError::UnknownClosedValue { line: 11, .. })
    ));
    let changed = replace_once(
        FIXTURE,
        "999d6c3044cc47a6dac06854d792e8d25e059c501be430cb20e1cadc5eb156fc",
        "999D6C3044CC47A6DAC06854D792E8D25E059C501BE430CB20E1CADC5EB156FC",
    );
    assert!(matches!(
        BehaviorObservationSetV1::parse(&changed),
        Err(BehaviorParseError::InvalidDigest { line: 3, .. })
    ));
}
