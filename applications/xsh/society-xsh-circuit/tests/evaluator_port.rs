#![allow(clippy::unwrap_used)]

use society_content::ContentDigest;
use society_xsh_circuit::{
    CanonicalEvaluatorInputRenderingV1, EvaluatorPortError, Vs001EvaluatorConstructionV1,
    Vs001EvaluatorInvocationV1, Vs001EvaluatorOutputContractV1, Vs001EvaluatorProfileV1,
    Vs001EvaluatorProgramV1,
};

#[test]
fn construction_derives_the_checked_in_entrypoint_and_declares_only_bounded_input_identity() {
    let input =
        CanonicalEvaluatorInputRenderingV1::from_bytes(b"input manifest v1\n".to_vec()).unwrap();
    let construction =
        Vs001EvaluatorConstructionV1::new(Vs001EvaluatorProgramV1::BehaviorMatrix, input.clone());

    assert_eq!(
        construction.entrypoint_rendering().bytes(),
        include_bytes!("../../circuits/vs-001-spawn-stderr/judges/run-behavior-matrix.sh")
    );
    assert_eq!(
        construction.input_rendering().bytes(),
        b"input manifest v1\n"
    );
    assert_eq!(
        construction.input_rendering().declared_blake3(),
        ContentDigest::of_bytes(b"input manifest v1\n")
    );
    assert_eq!(
        construction.profile(),
        Vs001EvaluatorProfileV1::DirectAdapterAdmissionPendingV1
    );
    assert_eq!(
        construction.invocation(),
        Vs001EvaluatorInvocationV1::BehaviorMatrix
    );
    assert_eq!(
        construction.invocation().script_relative_path(),
        "circuits/vs-001-spawn-stderr/judges/run-behavior-matrix.sh"
    );
    assert_eq!(
        construction.expected_output(),
        Vs001EvaluatorOutputContractV1::BehaviorMatrix
    );
}

#[test]
fn input_rendering_rejects_empty_and_over_limit_values_before_any_sealing_or_request() {
    assert_eq!(
        CanonicalEvaluatorInputRenderingV1::from_bytes(Vec::new()),
        Err(EvaluatorPortError::EmptyInputRendering)
    );
    assert!(matches!(
        CanonicalEvaluatorInputRenderingV1::from_bytes(vec![b'x'; 128 * 1024 + 1]),
        Err(EvaluatorPortError::InputRenderingTooLarge {
            limit,
            actual,
        }) if limit == 128 * 1024 && actual == 128 * 1024 + 1
    ));
}

#[test]
fn every_named_judge_has_one_checked_in_entrypoint_digest_invocation_and_output_contract() {
    let cases = [
        (
            Vs001EvaluatorProgramV1::BehaviorMatrix,
            "7d7798af7573696f4a69450e8eb8c74dc3e7f0b8f1b634b8ceac499736549e84",
            Vs001EvaluatorInvocationV1::BehaviorMatrix,
            Vs001EvaluatorOutputContractV1::BehaviorMatrix,
            "circuits/vs-001-spawn-stderr/judges/run-behavior-matrix.sh",
        ),
        (
            Vs001EvaluatorProgramV1::DocumentationMatrix,
            "89ba96283d7d88d6b4ddc5c2768b8ae402ac6ffac04b9ec628bdd3305e005edc",
            Vs001EvaluatorInvocationV1::DocumentationMatrix,
            Vs001EvaluatorOutputContractV1::DocumentationMatrix,
            "circuits/vs-001-spawn-stderr/judges/run-documentation-matrix.sh",
        ),
        (
            Vs001EvaluatorProgramV1::NegativeControls,
            "a42cf49d73e2d9b70dc5bf9f634a11d281902f7f4ee0c98e6c17de3960b2e3fe",
            Vs001EvaluatorInvocationV1::NegativeControls,
            Vs001EvaluatorOutputContractV1::NegativeControls,
            "circuits/vs-001-spawn-stderr/judges/run-negative-controls.sh",
        ),
        (
            Vs001EvaluatorProgramV1::FluencyTask,
            "9b60e8dec818b98dd53dac534847596af2ab6c17107bb71e845a9a2127610396",
            Vs001EvaluatorInvocationV1::FluencyTask,
            Vs001EvaluatorOutputContractV1::FluencyTask,
            "circuits/vs-001-spawn-stderr/judges/run-fluency-task-evaluator.sh",
        ),
        (
            Vs001EvaluatorProgramV1::CurationContract,
            "f3167cf4c3d2b2ec3d5914c20dab8e70f38ba0681285ce3375b80f0325962e5f",
            Vs001EvaluatorInvocationV1::CurationContract,
            Vs001EvaluatorOutputContractV1::CurationContract,
            "circuits/vs-001-spawn-stderr/judges/run-curation-contract-judge.sh",
        ),
        (
            Vs001EvaluatorProgramV1::UptakeApplication,
            "c3d761dbaefb1444d80c72cceb34f8f6b8edaea2092390ec195d042f6f0f26f0",
            Vs001EvaluatorInvocationV1::UptakeApplication,
            Vs001EvaluatorOutputContractV1::UptakeApplication,
            "circuits/vs-001-spawn-stderr/judges/run-uptake-application-judge.sh",
        ),
        (
            Vs001EvaluatorProgramV1::FrontierLeakage,
            "d2acaf08290803537a61473d0c0fcffe998f4be97e086bdf1eb8a02210b83d93",
            Vs001EvaluatorInvocationV1::FrontierLeakage,
            Vs001EvaluatorOutputContractV1::FrontierLeakage,
            "circuits/vs-001-spawn-stderr/judges/run-frontier-leakage-controls.sh",
        ),
        (
            Vs001EvaluatorProgramV1::SocietyNegativeControls,
            "e71b8c82c6fcc0cbcd54d65eb644f3368d9e860b6ecab61b3d49bd3a0b13860c",
            Vs001EvaluatorInvocationV1::SocietyNegativeControls,
            Vs001EvaluatorOutputContractV1::SocietyNegativeControls,
            "circuits/vs-001-spawn-stderr/judges/run-society-negative-controls.sh",
        ),
    ];

    for (program, expected_digest, invocation, output, script_path) in cases {
        let construction = Vs001EvaluatorConstructionV1::new(
            program,
            CanonicalEvaluatorInputRenderingV1::from_bytes(b"input".to_vec()).unwrap(),
        );
        assert_eq!(
            construction
                .entrypoint_rendering()
                .declared_blake3()
                .to_hex(),
            expected_digest
        );
        assert_eq!(construction.invocation(), invocation);
        assert_eq!(
            construction.invocation().script_relative_path(),
            script_path
        );
        assert_eq!(construction.expected_output(), output);
    }
}
