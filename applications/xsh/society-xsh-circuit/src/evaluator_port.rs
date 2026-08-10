//! Application-owned construction for VS-001 evaluator custody.
//!
//! This module deliberately stops before durable authority. It binds one
//! closed XSH evaluator entrypoint, its canonical input rendering and declared
//! BLAKE3 identity, a future direct-adapter profile, and the exact
//! application output contract. A future resident bridge may verify and seal
//! the bytes, resolve its own durable identities, and use the generic
//! native-child custody contract. No such scheduler path exists today, and no
//! application value here is a content object, child identity, process
//! request, or evidence admission.
//!
//! The canonical program value below covers the entrypoint script only. The
//! VS-001 judges also consume fixtures, external XSH/Xsht binaries, and, for
//! some judges, other scripts. A closed evaluator-package manifest that binds
//! those transitive application artifacts remains a separate follow-on; this
//! entrypoint identity must not be mistaken for package provenance or complete
//! evaluator authentication.

use society_content::ContentDigest;
use thiserror::Error;

const MAX_INPUT_RENDERING_BYTES: usize = 128 * 1024;

/// The bounded canonical input rendering presented by the application for
/// later daemon-private verification and sealing. Its digest is byte identity
/// only; constructing this value neither seals nor registers bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalEvaluatorInputRenderingV1 {
    bytes: Vec<u8>,
    declared_blake3: ContentDigest,
}

impl CanonicalEvaluatorInputRenderingV1 {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, EvaluatorPortError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(EvaluatorPortError::EmptyInputRendering);
        }
        if bytes.len() > MAX_INPUT_RENDERING_BYTES {
            return Err(EvaluatorPortError::InputRenderingTooLarge {
                limit: MAX_INPUT_RENDERING_BYTES,
                actual: bytes.len(),
            });
        }
        Ok(Self {
            declared_blake3: ContentDigest::of_bytes(&bytes),
            bytes,
        })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub const fn declared_blake3(&self) -> ContentDigest {
        self.declared_blake3
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum EvaluatorPortError {
    #[error("canonical evaluator input rendering must not be empty")]
    EmptyInputRendering,
    #[error("canonical evaluator input rendering exceeds {limit} bytes: {actual}")]
    InputRenderingTooLarge { limit: usize, actual: usize },
}

/// The immutable entrypoint rendering selected by a closed program variant.
/// There is intentionally no public constructor for arbitrary bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanonicalEvaluatorEntrypointRenderingV1 {
    bytes: &'static [u8],
    declared_blake3: ContentDigest,
}

impl CanonicalEvaluatorEntrypointRenderingV1 {
    fn from_checked_in(bytes: &'static [u8]) -> Self {
        Self {
            bytes,
            declared_blake3: ContentDigest::of_bytes(bytes),
        }
    }

    pub const fn bytes(self) -> &'static [u8] {
        self.bytes
    }

    pub const fn declared_blake3(self) -> ContentDigest {
        self.declared_blake3
    }
}

/// Closed VS-001 evaluator entrypoints. These are application names, never a
/// generic evaluator discriminant or a durable identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vs001EvaluatorProgramV1 {
    BehaviorMatrix,
    DocumentationMatrix,
    NegativeControls,
    FluencyTask,
    CurationContract,
    UptakeApplication,
    FrontierLeakage,
}

impl Vs001EvaluatorProgramV1 {
    const fn expected_output(self) -> Vs001EvaluatorOutputContractV1 {
        match self {
            Self::BehaviorMatrix => Vs001EvaluatorOutputContractV1::BehaviorMatrix,
            Self::DocumentationMatrix => Vs001EvaluatorOutputContractV1::DocumentationMatrix,
            Self::NegativeControls => Vs001EvaluatorOutputContractV1::NegativeControls,
            Self::FluencyTask => Vs001EvaluatorOutputContractV1::FluencyTask,
            Self::CurationContract => Vs001EvaluatorOutputContractV1::CurationContract,
            Self::UptakeApplication => Vs001EvaluatorOutputContractV1::UptakeApplication,
            Self::FrontierLeakage => Vs001EvaluatorOutputContractV1::FrontierLeakage,
        }
    }

    const fn invocation(self) -> Vs001EvaluatorInvocationV1 {
        match self {
            Self::BehaviorMatrix => Vs001EvaluatorInvocationV1::BehaviorMatrix,
            Self::DocumentationMatrix => Vs001EvaluatorInvocationV1::DocumentationMatrix,
            Self::NegativeControls => Vs001EvaluatorInvocationV1::NegativeControls,
            Self::FluencyTask => Vs001EvaluatorInvocationV1::FluencyTask,
            Self::CurationContract => Vs001EvaluatorInvocationV1::CurationContract,
            Self::UptakeApplication => Vs001EvaluatorInvocationV1::UptakeApplication,
            Self::FrontierLeakage => Vs001EvaluatorInvocationV1::FrontierLeakage,
        }
    }

    pub fn canonical_entrypoint(self) -> CanonicalEvaluatorEntrypointRenderingV1 {
        match self {
            Self::BehaviorMatrix => CanonicalEvaluatorEntrypointRenderingV1::from_checked_in(
                include_bytes!("../../circuits/vs-001-spawn-stderr/judges/run-behavior-matrix.sh"),
            ),
            Self::DocumentationMatrix => {
                CanonicalEvaluatorEntrypointRenderingV1::from_checked_in(include_bytes!(
                    "../../circuits/vs-001-spawn-stderr/judges/run-documentation-matrix.sh"
                ))
            }
            Self::NegativeControls => {
                CanonicalEvaluatorEntrypointRenderingV1::from_checked_in(include_bytes!(
                    "../../circuits/vs-001-spawn-stderr/judges/run-negative-controls.sh"
                ))
            }
            Self::FluencyTask => {
                CanonicalEvaluatorEntrypointRenderingV1::from_checked_in(include_bytes!(
                    "../../circuits/vs-001-spawn-stderr/judges/run-fluency-task-evaluator.sh"
                ))
            }
            Self::CurationContract => {
                CanonicalEvaluatorEntrypointRenderingV1::from_checked_in(include_bytes!(
                    "../../circuits/vs-001-spawn-stderr/judges/run-curation-contract-judge.sh"
                ))
            }
            Self::UptakeApplication => {
                CanonicalEvaluatorEntrypointRenderingV1::from_checked_in(include_bytes!(
                    "../../circuits/vs-001-spawn-stderr/judges/run-uptake-application-judge.sh"
                ))
            }
            Self::FrontierLeakage => {
                CanonicalEvaluatorEntrypointRenderingV1::from_checked_in(include_bytes!(
                    "../../circuits/vs-001-spawn-stderr/judges/run-frontier-leakage-controls.sh"
                ))
            }
        }
    }
}

/// The application profile for a future direct evaluator adapter. The resident
/// accepts direct executables only, while the checked-in VS-001 judges remain
/// shell source today. This variant therefore names pending application work,
/// not a generic interpreter profile or an executable custody claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vs001EvaluatorProfileV1 {
    DirectAdapterPendingV1,
}

/// The application-owned invocation selector. The daemon must resolve this
/// selector from the same sealed entrypoint rendering; it is not an executable
/// path or caller-supplied argv.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vs001EvaluatorInvocationV1 {
    BehaviorMatrix,
    DocumentationMatrix,
    NegativeControls,
    FluencyTask,
    CurationContract,
    UptakeApplication,
    FrontierLeakage,
}

impl Vs001EvaluatorInvocationV1 {
    /// The application-owned entrypoint path selected after the daemon
    /// verifies the matching entrypoint rendering. This is deliberately a
    /// fixed circuit path, not a caller-supplied executable or shell fragment.
    pub const fn script_relative_path(self) -> &'static str {
        match self {
            Self::BehaviorMatrix => "circuits/vs-001-spawn-stderr/judges/run-behavior-matrix.sh",
            Self::DocumentationMatrix => {
                "circuits/vs-001-spawn-stderr/judges/run-documentation-matrix.sh"
            }
            Self::NegativeControls => {
                "circuits/vs-001-spawn-stderr/judges/run-negative-controls.sh"
            }
            Self::FluencyTask => {
                "circuits/vs-001-spawn-stderr/judges/run-fluency-task-evaluator.sh"
            }
            Self::CurationContract => {
                "circuits/vs-001-spawn-stderr/judges/run-curation-contract-judge.sh"
            }
            Self::UptakeApplication => {
                "circuits/vs-001-spawn-stderr/judges/run-uptake-application-judge.sh"
            }
            Self::FrontierLeakage => {
                "circuits/vs-001-spawn-stderr/judges/run-frontier-leakage-controls.sh"
            }
        }
    }
}

/// The output grammar that remains owned by the XSH parser boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Vs001EvaluatorOutputContractV1 {
    BehaviorMatrix,
    DocumentationMatrix,
    NegativeControls,
    FluencyTask,
    CurationContract,
    UptakeApplication,
    FrontierLeakage,
}

/// One complete application construction that a daemon may preflight,
/// byte-verify, and seal before it translates the result to generic custody.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vs001EvaluatorConstructionV1 {
    program: Vs001EvaluatorProgramV1,
    profile: Vs001EvaluatorProfileV1,
    invocation: Vs001EvaluatorInvocationV1,
    expected_output: Vs001EvaluatorOutputContractV1,
    entrypoint_rendering: CanonicalEvaluatorEntrypointRenderingV1,
    input_rendering: CanonicalEvaluatorInputRenderingV1,
}

impl Vs001EvaluatorConstructionV1 {
    pub fn new(
        program: Vs001EvaluatorProgramV1,
        input_rendering: CanonicalEvaluatorInputRenderingV1,
    ) -> Self {
        Self {
            program,
            profile: Vs001EvaluatorProfileV1::DirectAdapterPendingV1,
            invocation: program.invocation(),
            expected_output: program.expected_output(),
            entrypoint_rendering: program.canonical_entrypoint(),
            input_rendering,
        }
    }

    pub const fn program(&self) -> Vs001EvaluatorProgramV1 {
        self.program
    }

    pub const fn profile(&self) -> Vs001EvaluatorProfileV1 {
        self.profile
    }

    pub const fn invocation(&self) -> Vs001EvaluatorInvocationV1 {
        self.invocation
    }

    pub const fn expected_output(&self) -> Vs001EvaluatorOutputContractV1 {
        self.expected_output
    }

    pub const fn entrypoint_rendering(&self) -> CanonicalEvaluatorEntrypointRenderingV1 {
        self.entrypoint_rendering
    }

    pub const fn input_rendering(&self) -> &CanonicalEvaluatorInputRenderingV1 {
        &self.input_rendering
    }
}
