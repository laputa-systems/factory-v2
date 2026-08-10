#![allow(clippy::unwrap_used)]

use society_xsh_circuit::{
    Vs001CurationDirectEvaluatorPackageV1, Vs001CurationInputRoleV1,
    Vs001DirectEvaluatorInputManifestV1, Vs001EvaluatorProgramV1,
};

macro_rules! curation_bytes {
    ($path:literal) => {
        include_bytes!(concat!(
            "../../circuits/vs-001-spawn-stderr/fixtures/curation",
            $path
        ))
    };
}

fn framed_curation(
    account: &[u8],
    selected_items: &[u8],
    preserved_conflicts: &[u8],
    decision_relevant_unknowns: &[u8],
    exclusions: &[u8],
    raw_evidence_escalations: &[u8],
    frontier_members: &[u8],
) -> Vec<u8> {
    let relations = [
        account,
        selected_items,
        preserved_conflicts,
        decision_relevant_unknowns,
        exclusions,
        raw_evidence_escalations,
        frontier_members,
    ];
    let mut manifest = b"# schema: Vs001CurationDirectInputManifestV1/framed-v1\n".to_vec();
    for (role, relation) in Vs001CurationInputRoleV1::ORDERED.iter().zip(relations) {
        manifest.extend_from_slice(role.wire_name().as_bytes());
        manifest.extend_from_slice(format!("\t{}\n", relation.len()).as_bytes());
        manifest.extend_from_slice(relation);
    }
    manifest
}

fn valid_curation_manifest() -> Vec<u8> {
    framed_curation(
        curation_bytes!("/c1-valid/account.v1.tsv"),
        curation_bytes!("/c1-valid/selected-items.v1.tsv"),
        curation_bytes!("/c1-valid/preserved-conflicts.v1.tsv"),
        curation_bytes!("/c1-valid/decision-relevant-unknowns.v1.tsv"),
        curation_bytes!("/c1-valid/exclusions.v1.tsv"),
        curation_bytes!("/c1-valid/raw-evidence-escalations.v1.tsv"),
        curation_bytes!("/frontier-c1-members.v1.tsv"),
    )
}

fn named_escalation_curation_manifest() -> Vec<u8> {
    framed_curation(
        curation_bytes!("/c1-valid-named-escalation/account.v1.tsv"),
        curation_bytes!("/c1-valid-named-escalation/selected-items.v1.tsv"),
        curation_bytes!("/c1-valid-named-escalation/preserved-conflicts.v1.tsv"),
        curation_bytes!("/c1-valid-named-escalation/decision-relevant-unknowns.v1.tsv"),
        curation_bytes!("/c1-valid-named-escalation/exclusions.v1.tsv"),
        curation_bytes!("/c1-valid-named-escalation/raw-evidence-escalations.v1.tsv"),
        curation_bytes!("/frontier-c1-members.v1.tsv"),
    )
}

#[test]
fn curation_direct_package_has_only_the_seven_length_framed_member_roles() {
    let package = Vs001CurationDirectEvaluatorPackageV1;

    assert_eq!(package.program(), Vs001EvaluatorProgramV1::CurationContract);
    assert_eq!(
        package.member_roles(),
        &[
            Vs001CurationInputRoleV1::Account,
            Vs001CurationInputRoleV1::SelectedItems,
            Vs001CurationInputRoleV1::PreservedConflicts,
            Vs001CurationInputRoleV1::DecisionRelevantUnknowns,
            Vs001CurationInputRoleV1::Exclusions,
            Vs001CurationInputRoleV1::RawEvidenceEscalations,
            Vs001CurationInputRoleV1::FrontierMembers,
        ]
    );
}

#[test]
fn direct_curation_adapter_matches_the_checked_in_shell_judge_observation() {
    let manifest = valid_curation_manifest();
    let input = Vs001DirectEvaluatorInputManifestV1::parse(&manifest).unwrap();

    assert_eq!(input.program(), Vs001EvaluatorProgramV1::CurationContract);
    assert_eq!(input.canonical_rendering().bytes(), manifest);
    assert_eq!(
        input.evaluate().unwrap().canonical_tsv().as_bytes(),
        include_bytes!("fixtures/curation-contract-observation.none.v1.tsv"),
    );
}

#[test]
fn direct_curation_adapter_preserves_the_checked_in_named_escalation_output() {
    let input =
        Vs001DirectEvaluatorInputManifestV1::parse(&named_escalation_curation_manifest()).unwrap();

    assert_eq!(
        input.evaluate().unwrap().canonical_tsv().as_bytes(),
        include_bytes!("fixtures/curation-contract-observation.named.v1.tsv"),
    );
}

#[test]
fn direct_curation_adapter_rejects_each_checked_in_shell_negative_relation() {
    let account = curation_bytes!("/c1-valid/account.v1.tsv");
    let selected_items = curation_bytes!("/c1-valid/selected-items.v1.tsv");
    let preserved_conflicts = curation_bytes!("/c1-valid/preserved-conflicts.v1.tsv");
    let unknowns = curation_bytes!("/c1-valid/decision-relevant-unknowns.v1.tsv");
    let exclusions = curation_bytes!("/c1-valid/exclusions.v1.tsv");
    let escalations = curation_bytes!("/c1-valid/raw-evidence-escalations.v1.tsv");
    let frontier = curation_bytes!("/frontier-c1-members.v1.tsv");

    let negative_manifests = [
        framed_curation(
            account,
            curation_bytes!("/negative-selected-items-unadmitted.v1.tsv"),
            preserved_conflicts,
            unknowns,
            exclusions,
            escalations,
            frontier,
        ),
        framed_curation(
            account,
            selected_items,
            preserved_conflicts,
            unknowns,
            exclusions,
            curation_bytes!("/negative-raw-evidence-escalations-unnamed.v1.tsv"),
            frontier,
        ),
        framed_curation(
            account,
            selected_items,
            preserved_conflicts,
            unknowns,
            curation_bytes!("/negative-exclusions-duplicate-category.v1.tsv"),
            escalations,
            frontier,
        ),
        framed_curation(
            account,
            selected_items,
            curation_bytes!("/negative-preserved-conflicts-extra-row.v1.tsv"),
            unknowns,
            exclusions,
            escalations,
            frontier,
        ),
    ];

    for manifest in negative_manifests {
        assert!(Vs001DirectEvaluatorInputManifestV1::parse(&manifest)
            .unwrap()
            .evaluate()
            .is_err());
    }
}

#[test]
fn relation_order_and_length_are_part_of_the_sealed_curation_input_contract() {
    let mut manifest = valid_curation_manifest();
    let first_role = b"account";
    let replacement = b"frontier";
    let position = manifest
        .windows(first_role.len())
        .position(|window| window == first_role)
        .unwrap();
    manifest[position..position + replacement.len()].copy_from_slice(replacement);
    assert!(Vs001DirectEvaluatorInputManifestV1::parse(&manifest).is_err());
}
