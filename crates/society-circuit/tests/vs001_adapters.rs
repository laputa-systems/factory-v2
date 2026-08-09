#![allow(clippy::unwrap_used)]

use society_circuit::{
    CuratedAccountV1, CurationContractObservationV1, CurationContractOutputsV1, CurationFrontierV1,
    CurationRawEvidenceEscalationObservationSetV1, DisclosureFrontierV1,
    DocumentationConflictSetV1, DocumentationObservationSetV1, FluencyExecutionEnvelopeV1,
    FluencyProbeExecutionSurfaceV1, FluencyProbeObservationSetV1, FrontierAccessObservationSetV1,
    InputDigestManifestV1, InputDigestProducer, InvestigatorAccessClass, InvestigatorAccessSetV1,
    InvestigatorSubmissionV1, PropagationObservationV1, UptakeDeliveryContextV1,
    UptakePersistedInputV1, Vs001ParseError,
};

const CURATION_FRONTIER: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/curation/frontier-c1-members.v1.tsv"
);
const CURATION_ACCOUNT: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/curation/c1-valid/account.v1.tsv"
);
const CURATION_SELECTED: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/curation/c1-valid/selected-items.v1.tsv"
);
const CURATION_CONFLICTS: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/curation/c1-valid/preserved-conflicts.v1.tsv"
);
const CURATION_UNKNOWNS: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/curation/c1-valid/decision-relevant-unknowns.v1.tsv"
);
const CURATION_EXCLUSIONS: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/curation/c1-valid/exclusions.v1.tsv"
);
const CURATION_ESCALATIONS: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/curation/c1-valid/raw-evidence-escalations.v1.tsv"
);
const CURATION_NAMED_ESCALATIONS: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/curation/c1-valid-named-escalation/raw-evidence-escalations.v1.tsv"
);

const UPTAKE_CONTEXT: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/uptake/positive/delivery-context.v1.tsv"
);
const UPTAKE_PERSISTED: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/uptake/positive/persisted-input.v1.tsv"
);
const UPTAKE_SUBMISSION: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/uptake/positive/investigator-submission.v1.tsv"
);
const UPTAKE_EMPTY_ACCESSES: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/uptake/positive/accesses.v1.tsv"
);
const UPTAKE_BAD_SUBMISSION: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/uptake/negative/investigator-submission-missing-call-sites.v1.tsv"
);
const UPTAKE_FORBIDDEN_ACCESSES: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/uptake/negative/accesses-forbidden-session.v1.tsv"
);

const W1_MEMBERS: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/frontier/w1-valid/frontier-members.v1.tsv"
);
const W1_SEQUESTERED: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/frontier/w1-valid/sequestered.v1.tsv"
);
const W1_MISSING_MEMBER: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/frontier/w1-missing-positive/frontier-members.v1.tsv"
);
const W1_DUPLICATE_CLASS: &[u8] = include_bytes!(
    "../../../circuits/vs-001-spawn-stderr/fixtures/frontier/w1-duplicate-class/sequestered.v1.tsv"
);

fn replace_once(source: &[u8], from: &str, to: &str) -> Vec<u8> {
    String::from_utf8(source.to_vec())
        .unwrap()
        .replacen(from, to, 1)
        .into_bytes()
}

fn digest_manifest(schema: &str, kinds: &[&str]) -> Vec<u8> {
    let mut text = format!("{schema}\ninput_kind\tsha256\n");
    for kind in kinds {
        text.push_str(kind);
        text.push_str("\t0000000000000000000000000000000000000000000000000000000000000000\n");
    }
    text.into_bytes()
}

#[test]
fn input_manifest_requires_a_typed_producer_when_schema_text_is_shared() {
    let documentation = digest_manifest(
        "# schema: CircuitInputDigestV1/tsv-v1",
        &[
            "xsht_binary",
            "documentation_evaluator",
            "lang_source",
            "spec_source",
            "spec_os_source",
            "runtime_process_source",
            "lowered_runtime_source",
            "native_process_test",
            "api_process_command_argv",
            "api_process_spawn",
            "api_process_navigation",
        ],
    );
    let parsed =
        InputDigestManifestV1::parse(InputDigestProducer::DocumentationMatrix, &documentation)
            .unwrap();
    assert_eq!(parsed.entries().len(), 11);
    assert!(matches!(
        InputDigestManifestV1::parse(InputDigestProducer::BehaviorMatrix, &documentation),
        Err(Vs001ParseError::WrongRowCount { .. })
    ));

    let curation = digest_manifest(
        "# schema: CurationContractInputDigestV1/tsv-v1",
        &[
            "curation_contract_judge",
            "frontier_members",
            "account_relation",
            "selected_items_relation",
            "conflicts_relation",
            "unknowns_relation",
            "exclusions_relation",
            "escalations_relation",
        ],
    );
    assert_eq!(
        InputDigestManifestV1::parse(InputDigestProducer::CurationContract, &curation)
            .unwrap()
            .entries()
            .len(),
        8
    );
    let noncanonical = replace_once(&curation, "\n", "\r\n");
    assert!(matches!(
        InputDigestManifestV1::parse(InputDigestProducer::CurationContract, &noncanonical),
        Err(Vs001ParseError::NonCanonicalLineEnding { .. })
    ));
    let mut invalid_utf8 = curation;
    invalid_utf8[0] = 0xff;
    assert!(matches!(
        InputDigestManifestV1::parse(InputDigestProducer::CurationContract, &invalid_utf8),
        Err(Vs001ParseError::InvalidUtf8 { .. })
    ));
    assert!(matches!(
        InputDigestManifestV1::parse(
            InputDigestProducer::CurationContract,
            &vec![b'x'; 128 * 1024 + 1]
        ),
        Err(Vs001ParseError::FrameTooLarge { .. })
    ));
}

#[test]
fn documentation_observations_and_conflicts_are_exact_closed_rows() {
    let documentation = concat!(
        "# schema: DocumentationObservationV1/tsv-v1\n",
        "source\tconsumer\tfield\tclaim\tcitation\n",
        "LANG_md\tspawn_command\tstderr\tclaims_missing\tLANG.md:1\n",
        "SPEC_spawn\tspawn_command\tstderr\tclaims_uses_command_redirections\tSPEC.md:2\n",
        "SPEC_spawn\tspawn_command\tdefault\tclaims_inherit_default\tSPEC.md:3\n",
        "SPEC_api\tcommand_plan\tstderr\tclaims_typed_path_field\tSPEC.md:4\n",
        "SPEC_api\tcommand_plan\tstderr_append\tclaims_typed_append_field\tSPEC.md:5\n",
        "SPEC_spawn\tspawn_command\terror\tclaims_setup_failure_is_process_error\tSPEC.md:6\n",
        "SPEC_spawn\tprocess_spawn\tlifecycle\tclaims_detached_record\tSPEC.md:7\n",
        "SPEC_OS\tspawn_command\townership\tclaims_owned_child_group\tSPEC-OS.md:8\n",
        "SPEC_OS\tcommand_plan\terror\tclaims_redirection_failure_distinct_from_status\tSPEC-OS.md:9\n",
        "xsht_api\tcommand_plan\tstderr\tdiscoverable_typed_path_field\tapi-process-command-argv.txt:10\n",
        "xsht_api\tcommand_plan\tstderr_append\tdiscoverable_typed_append_field\tapi-process-command-argv.txt:11\n",
        "xsht_api\tprocess_spawn\tlifecycle\tdoes_not_disclose_lifecycle\tapi-process-spawn.txt\n",
        "xsht_navigation\tcommand_plan\tdiscovery\tfinds_command_argv\tapi-search-process.txt:12\n",
        "xsht_navigation\tprocess_spawn\tdiscovery\tfinds_process_spawn\tapi-search-process.txt:13\n",
        "runtime\tprocess_spawn\tcall_path\tspawn_command_enters_detached_options\tsrc/runtime/process.rs:14\n",
        "runtime\tprocess_spawn\tstderr\tdisables_command_redirections\tsrc/runtime/process.rs:15\n",
        "runtime\tmanaged_spawn\tstderr\tconditionally_applies_command_redirections\tsrc/runtime/process.rs:16\n",
        "runtime\tmanaged_spawn\tstderr\tenables_command_redirections\tsrc/runtime/process.rs:17\n",
        "lowered_runtime\tprocess_spawn\tcall_path\tcalls_detached_spawn_command\tsrc/runtime/eval/lowered_run.rs:18\n",
        "lowered_runtime\tspawn_command\tcall_path\tcreates_managed_redirection_path\tsrc/runtime/eval/lowered_run.rs:19\n",
        "native_test\tprocess_run\tstderr\tcovers_run_redirection\ttests/xsh/stdlib/process.xsh:20-21\n",
        "native_test\tspawn_command\tstderr\tno_managed_stderr_assertion_in_focused_test\ttests/xsh/stdlib/process.xsh:22-23\n"
    );
    assert_eq!(
        DocumentationObservationSetV1::parse(documentation.as_bytes())
            .unwrap()
            .observations()
            .len(),
        22
    );
    let recombined = documentation.replace(
        "runtime\tprocess_spawn\tstderr",
        "runtime\tmanaged_spawn\tstderr",
    );
    assert!(matches!(
        DocumentationObservationSetV1::parse(recombined.as_bytes()),
        Err(Vs001ParseError::RecombinedRow { line: 18, .. })
    ));

    let conflicts = concat!(
        "# schema: DocumentationConflictV1/tsv-v1\n",
        "conflict_id\tleft_claim\tright_claim\tstatus\n",
        "D01\tLANG_claims_missing\tSPEC_claims_supported\tresolved\n",
        "D02\txsht_api_claims_owned_handle\tSPEC_claims_detached_record\tresolved\n",
        "D03\tprocess_spawn_redirection_ignored\tmanaged_spawn_redirection_enabled\tintentional_semantic_split\n"
    );
    assert_eq!(
        DocumentationConflictSetV1::parse(conflicts.as_bytes())
            .unwrap()
            .conflicts()
            .len(),
        3
    );
}

#[test]
fn fluency_relations_preserve_fixture_boundaries_without_actor_inference() {
    let zero = "0000000000000000000000000000000000000000000000000000000000000000";
    let observations = format!(
        "# schema: FluencyProbeObservationV1/tsv-v1\ncase_id\tinput_manifest\texpected_exit\tsupervisor_exit\tparent_stdout_sha256\tparent_stderr_sha256\tredirected_stderr_sha256\tcorrectness\ttyped_boundary\townership_lifecycle\thost_path_access\tdisposition\nF01\tpreexisting_log_truncate\t0\t0\t{zero}\t{zero}\t{zero}\tpassed\tcompliant\towned_waited\tclean\tpass\nF02\tpath_with_spaces\t0\t0\t{zero}\t{zero}\t{zero}\tpassed\tcompliant\towned_waited\tclean\tpass\nF03\tnonzero_child_status\t23\t23\t{zero}\t{zero}\t{zero}\tpassed\tcompliant\towned_waited\tclean\tpass\n"
    );
    assert_eq!(
        FluencyProbeObservationSetV1::parse(observations.as_bytes())
            .unwrap()
            .observations()
            .len(),
        3
    );
    let surface = concat!(
        "# schema: FluencyProbeExecutionSurfaceV1/tsv-v1\n",
        "execution_kind\ttool_errors\tturns\tactive_wall\ttokens\treasoning_tokens\tcost\n",
        "deterministic_fixture\tnot_observed_no_provider\tnot_observed_no_provider\tnot_observed_no_provider\tnot_observed_no_provider\tnot_observed_no_provider\tnot_observed_no_provider\n"
    );
    FluencyProbeExecutionSurfaceV1::parse(surface.as_bytes()).unwrap();
    let envelope = concat!(
        "# schema: FluencyExecutionEnvelopeV1/tsv-v1\n",
        "workspace_label\tworking_directory\tenvironment\thome\tconfig\ttemp\tpath\n",
        "q7f3a\topaque_workspace\tminimal_explicit\tworkspace_local\tworkspace_local\tworkspace_local\tassigned_bin_front\n"
    );
    FluencyExecutionEnvelopeV1::parse(envelope.as_bytes()).unwrap();
    let label_64 = "a".repeat(64);
    let bounded_envelope = envelope.replace("q7f3a", &label_64);
    FluencyExecutionEnvelopeV1::parse(bounded_envelope.as_bytes()).unwrap();
    let label_65 = "a".repeat(65);
    let overlong_envelope = envelope.replace("q7f3a", &label_65);
    assert!(matches!(
        FluencyExecutionEnvelopeV1::parse(overlong_envelope.as_bytes()),
        Err(Vs001ParseError::InvalidValue {
            column: "workspace_label",
            ..
        })
    ));
    assert!(matches!(
        FluencyProbeObservationSetV1::parse(
            observations.replace("owned_waited", "reaped").as_bytes()
        ),
        Err(Vs001ParseError::RecombinedRow { .. })
    ));
}

#[test]
fn curation_parses_real_relations_and_named_escalation_without_admission_claims() {
    let frontier = CurationFrontierV1::parse(CURATION_FRONTIER).unwrap();
    let account = CuratedAccountV1::parse(
        &frontier,
        CURATION_ACCOUNT,
        CURATION_SELECTED,
        CURATION_CONFLICTS,
        CURATION_UNKNOWNS,
        CURATION_EXCLUSIONS,
        CURATION_ESCALATIONS,
    )
    .unwrap();
    assert_eq!(account.selections().len(), 6);
    assert!(account.raw_evidence_escalation().is_none());
    let named = CuratedAccountV1::parse(
        &frontier,
        CURATION_ACCOUNT,
        CURATION_SELECTED,
        CURATION_CONFLICTS,
        CURATION_UNKNOWNS,
        CURATION_EXCLUSIONS,
        CURATION_NAMED_ESCALATIONS,
    )
    .unwrap();
    assert!(named.raw_evidence_escalation().is_some());
    let duplicate_selected = include_bytes!(
        "../../../circuits/vs-001-spawn-stderr/fixtures/curation/negative-selected-items-duplicate-source.v1.tsv"
    );
    assert!(
        CuratedAccountV1::parse(
            &frontier,
            CURATION_ACCOUNT,
            duplicate_selected,
            CURATION_CONFLICTS,
            CURATION_UNKNOWNS,
            CURATION_EXCLUSIONS,
            CURATION_ESCALATIONS,
        )
        .is_err()
    );

    let no_request = concat!(
        "# schema: CurationRawEvidenceEscalationObservationV1/tsv-v1\n",
        "ordinal\tquestion_ref\tobject_ref\n"
    );
    assert!(
        CurationRawEvidenceEscalationObservationSetV1::parse(no_request.as_bytes())
            .unwrap()
            .request()
            .is_none()
    );
    let result = concat!(
        "# schema: CurationContractObservationV1/tsv-v1\n",
        "account_kind\tpurpose\thypothesis_coverage\tcounterevidence\tpreserved_conflict\tunknowns\texclusions\traw_escalations\tfrontier_admission\tdisposition\n",
        "decision_curation\tauthorize_spawn_stderr_prototype\th1_h2_h3_with_dissent\tdeclared\tpreserved\tdeclared\tsemantic\tnamed_request_present\tfrontier_constrained\tacceptance_ready\n"
    );
    CurationContractObservationV1::parse(result.as_bytes()).unwrap();
    let named_request = concat!(
        "# schema: CurationRawEvidenceEscalationObservationV1/tsv-v1\n",
        "ordinal\tquestion_ref\tobject_ref\n",
        "1\tresolve_detached_contract_intent\traw_pi_session_object\n"
    );
    CurationContractOutputsV1::parse(result.as_bytes(), named_request.as_bytes()).unwrap();
    assert!(matches!(
        CurationContractOutputsV1::parse(result.as_bytes(), no_request.as_bytes()),
        Err(Vs001ParseError::RecombinedRow { .. })
    ));
}

#[test]
fn uptake_keeps_forbidden_access_as_an_observation_but_rejects_malformed_record_unions() {
    UptakeDeliveryContextV1::parse(UPTAKE_CONTEXT).unwrap();
    UptakePersistedInputV1::parse(UPTAKE_PERSISTED).unwrap();
    InvestigatorSubmissionV1::parse(UPTAKE_SUBMISSION).unwrap();
    assert!(matches!(
        InvestigatorSubmissionV1::parse(UPTAKE_BAD_SUBMISSION),
        Err(Vs001ParseError::InvalidValue { .. })
    ));
    assert!(
        InvestigatorAccessSetV1::parse(UPTAKE_EMPTY_ACCESSES)
            .unwrap()
            .accesses()
            .is_empty()
    );
    let forbidden = InvestigatorAccessSetV1::parse(UPTAKE_FORBIDDEN_ACCESSES).unwrap();
    assert_eq!(
        forbidden.accesses()[0].class,
        InvestigatorAccessClass::ForbiddenVs001Session
    );

    let contamination = concat!(
        "# schema: PropagationObservationV1/tsv-v1\n",
        "target_revision\tlesson_revision\tdelivered\tencountered\tapplication\tcontamination\tdisposition\n",
        "t_stdout_capture_inquiry_r1\tl1_spawn_stderr_method_r1\tdelivered\tencountered\tnot_applied\tcontaminated\tcontamination_recorded\n"
    );
    PropagationObservationV1::parse(contamination.as_bytes()).unwrap();
}

fn valid_frontier_matrix(frontier: &DisclosureFrontierV1) -> Vec<u8> {
    let principals = [
        "replay_actor",
        "projector",
        "ordinary_investigator",
        "grand_architect_query_client",
    ];
    let routes = [
        "direct_identity",
        "graph_traversal",
        "object_digest",
        "current_repository_path",
        "culture_lookup",
        "projection_lookup",
    ];
    let mut matrix = String::from(
        "# schema: FrontierAccessObservationV1/tsv-v1\nprincipal\tlookup_route\treference_class\topaque_ref\tdisposition\taudit_placement\n",
    );
    for principal in principals {
        for member in frontier.members() {
            matrix.push_str(&format!(
                "{principal}\tdirect_identity\tfrontier_member\t{}\tallowed\tno_audit\n",
                member.as_str()
            ));
        }
        for sequestered in frontier.sequestered() {
            for route in routes {
                matrix.push_str(&format!(
                    "{principal}\t{route}\t{}\t{}\tdenied\tcontamination_audit_outside_w1\n",
                    sequestered.class.as_str(),
                    sequestered.class.opaque_ref()
                ));
            }
        }
    }
    matrix.into_bytes()
}

#[test]
fn frontier_requires_the_closed_allowlist_aftermath_classes_and_derived_matrix() {
    let frontier = DisclosureFrontierV1::parse(W1_MEMBERS, W1_SEQUESTERED).unwrap();
    let matrix = valid_frontier_matrix(&frontier);
    assert_eq!(
        FrontierAccessObservationSetV1::parse(&frontier, &matrix)
            .unwrap()
            .observations()
            .len(),
        268
    );
    assert!(DisclosureFrontierV1::parse(W1_MISSING_MEMBER, W1_SEQUESTERED).is_err());
    assert!(DisclosureFrontierV1::parse(W1_MEMBERS, W1_DUPLICATE_CLASS).is_err());
    assert!(matches!(
        FrontierAccessObservationSetV1::parse(
            &frontier,
            &replace_once(
                &matrix,
                "\tdirect_identity\tfrontier_member",
                "\tgraph_traversal\tfrontier_member"
            )
        ),
        Err(Vs001ParseError::RecombinedRow { .. })
    ));
}
