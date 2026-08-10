//! Canonical synthetic world fixtures for CL-001.
//!
//! This crate owns only the world semantics.  It does not admit episodes,
//! assign treatment, write the Society ledger, or expose an actor-facing
//! evaluator.  [`AnalysisOnlyEvaluator`] is deliberately an analysis-bound
//! value: callers use it after an episode has produced a complete evidence
//! occurrence, never as an input to an actor or a Forum tool.

use std::fmt;

use society_kernel::Blake3Digest;

/// The number of cards in the canonical evidence partition.
pub const EVIDENCE_CARD_COUNT: usize = 8;

/// Canonical bytes for the deliberately wrong early claim.
///
/// The claim is untrusted Forum content.  Its synthetic source label is part
/// of the fixture and does not grant a capability or epistemic authority.
pub const FALSE_CLAIM_BYTES: &[u8] =
    b"cl-001|false-claim|v1|proposition=0|confidence=high|source=synthetic-authority";

/// Canonical bytes for the deterministic correction released after
/// replacement and exposure admission.
pub const CORRECTION_PACKAGE_BYTES: &[u8] =
    b"cl-001|correction|v1|proposition=1|basis=all-eight-cards|source=deterministic-evaluator";

/// Exact BLAKE3 identity of [`FALSE_CLAIM_BYTES`].
pub const FALSE_CLAIM_DIGEST: Blake3Digest = Blake3Digest::from_bytes([
    0x9c, 0x4f, 0xcf, 0x46, 0xbe, 0x60, 0xc8, 0x2c, 0x7b, 0x71, 0x9a, 0xaa, 0xcd, 0x11, 0x8d, 0x6c,
    0x2c, 0xb5, 0xac, 0x83, 0x2a, 0xbf, 0x19, 0x37, 0x36, 0x1a, 0x85, 0x75, 0x12, 0xd4, 0x38, 0x57,
]);

/// Exact BLAKE3 identity of [`CORRECTION_PACKAGE_BYTES`].
pub const CORRECTION_PACKAGE_DIGEST: Blake3Digest = Blake3Digest::from_bytes([
    0x49, 0xaf, 0x7a, 0xc2, 0x7b, 0x89, 0x80, 0xdf, 0x92, 0x87, 0x3d, 0x53, 0xe1, 0xf4, 0x2a, 0xb5,
    0xfc, 0x16, 0x38, 0xef, 0x68, 0x74, 0xaa, 0xe5, 0xc7, 0x0d, 0x51, 0x6b, 0x9b, 0x70, 0xd2, 0xa2,
]);

const SOURCE_SIGNATURE_BYTES: &[u8] = b"cl-001|synthetic-card-source|signature=v1";

// The proposition is the parity of the eight observations.  Keeping the
// observations in bytes makes their exact content identity explicit while the
// partition's bit positions make the no-single-card proof structural.
const CANONICAL_CARD_OBSERVATIONS: [bool; EVIDENCE_CARD_COUNT] =
    [true, false, true, true, false, true, false, true];

const CANONICAL_CARD_BYTES: [&[u8]; EVIDENCE_CARD_COUNT] = [
    b"cl-001|card=01|observation=1|source=synthetic-card-source",
    b"cl-001|card=02|observation=0|source=synthetic-card-source",
    b"cl-001|card=03|observation=1|source=synthetic-card-source",
    b"cl-001|card=04|observation=1|source=synthetic-card-source",
    b"cl-001|card=05|observation=0|source=synthetic-card-source",
    b"cl-001|card=06|observation=1|source=synthetic-card-source",
    b"cl-001|card=07|observation=0|source=synthetic-card-source",
    b"cl-001|card=08|observation=1|source=synthetic-card-source",
];

/// Exact BLAKE3 identities for [`CANONICAL_CARD_BYTES`] in ordinal order.
pub const CANONICAL_CARD_DIGESTS: [Blake3Digest; EVIDENCE_CARD_COUNT] = [
    Blake3Digest::from_bytes([
        0x91, 0xff, 0x05, 0x62, 0x26, 0x94, 0x7d, 0x81, 0x3d, 0xb0, 0xf1, 0x5e, 0x5a, 0x55, 0x25,
        0x39, 0xcb, 0xfc, 0x2c, 0xd2, 0x68, 0x6d, 0x12, 0xeb, 0x47, 0xa7, 0x59, 0xcc, 0x18, 0x80,
        0x97, 0xda,
    ]),
    Blake3Digest::from_bytes([
        0xab, 0x4c, 0x27, 0xd9, 0x66, 0x4a, 0x58, 0x30, 0x3e, 0xe8, 0xc6, 0xc7, 0x77, 0x21, 0x09,
        0x4c, 0x43, 0xa4, 0xff, 0xce, 0x46, 0x74, 0x61, 0xef, 0x3f, 0xc8, 0x91, 0x2b, 0x3d, 0x9a,
        0x4f, 0x8c,
    ]),
    Blake3Digest::from_bytes([
        0x26, 0x19, 0x21, 0x91, 0x40, 0x46, 0xea, 0x55, 0xce, 0xea, 0xc7, 0xdd, 0x72, 0x78, 0xf3,
        0x51, 0x21, 0x9f, 0x18, 0x0e, 0x24, 0x83, 0x3a, 0x54, 0xbc, 0xcb, 0x4d, 0xe5, 0xb8, 0xdc,
        0xcf, 0xa8,
    ]),
    Blake3Digest::from_bytes([
        0xb1, 0x74, 0x81, 0x45, 0x1d, 0xf4, 0x20, 0x89, 0x0e, 0xe3, 0x88, 0x19, 0x5f, 0x89, 0x73,
        0x28, 0x3e, 0xd5, 0xca, 0x47, 0xf8, 0x3f, 0x27, 0x29, 0x56, 0x7b, 0xb8, 0x68, 0xa9, 0x5e,
        0x46, 0xe8,
    ]),
    Blake3Digest::from_bytes([
        0x9b, 0x9c, 0x3e, 0xf9, 0x2b, 0xac, 0xe5, 0xaa, 0x52, 0x8e, 0x3d, 0x85, 0xa5, 0x5b, 0x16,
        0x92, 0x76, 0x4b, 0x9b, 0x23, 0xfd, 0xb8, 0x6d, 0x92, 0x68, 0x67, 0x32, 0x77, 0xbd, 0x3b,
        0x80, 0x63,
    ]),
    Blake3Digest::from_bytes([
        0x20, 0x8f, 0x16, 0x41, 0x69, 0x32, 0x80, 0xe5, 0xc7, 0x73, 0x9e, 0x8c, 0x83, 0x85, 0xe8,
        0x0e, 0xde, 0x2f, 0x8e, 0x96, 0x71, 0xa8, 0xca, 0xd7, 0xd2, 0xad, 0x2e, 0x05, 0xb6, 0xc4,
        0xe0, 0xe9,
    ]),
    Blake3Digest::from_bytes([
        0xff, 0x00, 0x43, 0x55, 0x85, 0x79, 0x6f, 0x47, 0x5d, 0x17, 0xc9, 0xe2, 0xaa, 0x4e, 0x32,
        0xd6, 0xbf, 0xed, 0x4b, 0x96, 0x88, 0xf0, 0xaa, 0xef, 0xbe, 0x0f, 0x60, 0x3a, 0xd7, 0x4b,
        0x1d, 0x1d,
    ]),
    Blake3Digest::from_bytes([
        0xda, 0x78, 0x0c, 0x3c, 0xd3, 0x46, 0x20, 0xd8, 0xcc, 0x47, 0xf0, 0x8c, 0xa2, 0xdc, 0xb7,
        0x85, 0xa1, 0xc3, 0xfd, 0x01, 0xde, 0x4b, 0xd8, 0x2a, 0x93, 0xaa, 0xc3, 0xbe, 0xd7, 0x8c,
        0x61, 0x1e,
    ]),
];

/// The two possible values of the hidden binary proposition.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BinaryOutcome {
    /// The proposition's canonical value is zero.
    Zero,
    /// The proposition's canonical value is one.
    One,
}

impl BinaryOutcome {
    /// Return the bit represented by this value.
    pub const fn bit(self) -> bool {
        matches!(self, Self::One)
    }

    fn from_bit(bit: bool) -> Self {
        if bit { Self::One } else { Self::Zero }
    }
}

/// A card's immutable, content-addressed synthetic evidence.
///
/// `partition_mask` is intentionally separate from the observed bit.  It
/// describes which independent part of the proposition this card covers;
/// the validator uses it to prove that all eight parts are present and that
/// no single card can establish the parity result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceCard {
    ordinal: u8,
    observation: bool,
    partition_mask: u8,
    bytes: Vec<u8>,
    digest: Blake3Digest,
    source_signature: Vec<u8>,
}

impl EvidenceCard {
    /// Construct a card with a single partition bit.  The constructor is
    /// useful for validator tests and for future independent world seeds; the
    /// canonical fixture is obtained from [`WorldFixture::canonical`].
    pub fn new(
        ordinal: u8,
        observation: bool,
        bytes: Vec<u8>,
    ) -> Result<Self, CardConstructionError> {
        if !(1..=EVIDENCE_CARD_COUNT as u8).contains(&ordinal) {
            return Err(CardConstructionError::InvalidOrdinal { ordinal });
        }
        if bytes.is_empty() {
            return Err(CardConstructionError::EmptyBytes);
        }

        Ok(Self {
            ordinal,
            observation,
            partition_mask: 1 << (ordinal - 1),
            digest: Blake3Digest::of_bytes(&bytes),
            bytes,
            source_signature: SOURCE_SIGNATURE_BYTES.to_vec(),
        })
    }

    /// One-based stable card ordinal.
    pub const fn ordinal(&self) -> u8 {
        self.ordinal
    }

    /// The card's observed binary value.
    pub const fn observation(&self) -> bool {
        self.observation
    }

    /// The independent proposition bit covered by this card.
    pub const fn partition_mask(&self) -> u8 {
        self.partition_mask
    }

    /// Exact immutable bytes supplied by the world.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Exact BLAKE3 content identity of [`Self::bytes`].
    pub const fn digest(&self) -> Blake3Digest {
        self.digest
    }

    /// Synthetic source signature bytes.  This is descriptive fixture data,
    /// not a Society capability or truth assertion.
    pub fn source_signature(&self) -> &[u8] {
        &self.source_signature
    }

    fn is_content_intact(&self) -> bool {
        self.digest == Blake3Digest::of_bytes(&self.bytes)
    }
}

/// Failure to construct an evidence card.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardConstructionError {
    /// The ordinal must identify one of the eight canonical partition slots.
    InvalidOrdinal { ordinal: u8 },
    /// Empty bytes cannot have a useful immutable identity.
    EmptyBytes,
}

impl fmt::Display for CardConstructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOrdinal { ordinal } => {
                write!(
                    formatter,
                    "evidence card ordinal {ordinal} is outside 1..=8"
                )
            }
            Self::EmptyBytes => formatter.write_str("evidence card bytes are empty"),
        }
    }
}

impl std::error::Error for CardConstructionError {}

/// A validated, complete evidence partition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidencePartition {
    cards: Vec<EvidenceCard>,
    identity: Blake3Digest,
}

impl EvidencePartition {
    /// Construct the canonical validated partition.
    pub fn canonical() -> Self {
        WorldFixture::canonical().evidence().clone()
    }

    /// Validate and retain a complete partition.
    pub fn try_from_cards(cards: Vec<EvidenceCard>) -> Result<Self, PartitionValidationError> {
        validate_evidence_partition(&cards)?;
        Ok(Self {
            identity: partition_identity(&cards),
            cards,
        })
    }

    /// The immutable cards in ordinal order.
    pub fn cards(&self) -> &[EvidenceCard] {
        &self.cards
    }

    /// Exact BLAKE3 identity of the ordered partition.
    pub const fn identity(&self) -> Blake3Digest {
        self.identity
    }

    /// Number of cards in this complete partition.
    pub const fn len(&self) -> usize {
        EVIDENCE_CARD_COUNT
    }

    /// A complete validated partition is never empty.
    pub const fn is_empty(&self) -> bool {
        false
    }
}

/// Structural failures found while validating an evidence partition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PartitionValidationError {
    /// Exactly eight cards are required.
    WrongCardCount { expected: usize, actual: usize },
    /// A card ordinal did not identify a canonical partition slot.
    InvalidOrdinal { ordinal: u8 },
    /// Two cards claimed the same partition slot.
    DuplicateOrdinal { ordinal: u8 },
    /// The card's digest does not match its bytes.
    DigestMismatch { ordinal: u8 },
    /// The card did not cover exactly one independent proposition bit.
    InvalidPartitionMask { ordinal: u8, mask: u8 },
    /// The partition did not cover all eight independent proposition bits.
    IncompletePartition { mask: u8 },
    /// At least two independent bits are required so a card cannot establish
    /// the parity outcome alone.
    SingleCardCanProveOutcome,
}

impl fmt::Display for PartitionValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongCardCount { expected, actual } => {
                write!(
                    formatter,
                    "expected {expected} evidence cards, found {actual}"
                )
            }
            Self::InvalidOrdinal { ordinal } => write!(formatter, "invalid card ordinal {ordinal}"),
            Self::DuplicateOrdinal { ordinal } => {
                write!(formatter, "duplicate card ordinal {ordinal}")
            }
            Self::DigestMismatch { ordinal } => write!(formatter, "card {ordinal} digest mismatch"),
            Self::InvalidPartitionMask { ordinal, mask } => {
                write!(
                    formatter,
                    "card {ordinal} has invalid partition mask {mask:#x}"
                )
            }
            Self::IncompletePartition { mask } => {
                write!(formatter, "evidence partition mask {mask:#x} is incomplete")
            }
            Self::SingleCardCanProveOutcome => {
                formatter.write_str("one evidence card can prove the outcome")
            }
        }
    }
}

impl std::error::Error for PartitionValidationError {}

/// Validate the canonical partition invariant without admitting an episode.
pub fn validate_evidence_partition(cards: &[EvidenceCard]) -> Result<(), PartitionValidationError> {
    if cards.len() != EVIDENCE_CARD_COUNT {
        return Err(PartitionValidationError::WrongCardCount {
            expected: EVIDENCE_CARD_COUNT,
            actual: cards.len(),
        });
    }

    let mut ordinals = 0_u16;
    let mut mask = 0_u8;
    for card in cards {
        if !(1..=EVIDENCE_CARD_COUNT as u8).contains(&card.ordinal) {
            return Err(PartitionValidationError::InvalidOrdinal {
                ordinal: card.ordinal,
            });
        }

        let ordinal_bit = 1_u16 << (card.ordinal - 1);
        if ordinals & ordinal_bit != 0 {
            return Err(PartitionValidationError::DuplicateOrdinal {
                ordinal: card.ordinal,
            });
        }
        ordinals |= ordinal_bit;

        let expected_partition_mask = 1_u8 << (card.ordinal - 1);
        if card.partition_mask != expected_partition_mask {
            return Err(PartitionValidationError::InvalidPartitionMask {
                ordinal: card.ordinal,
                mask: card.partition_mask,
            });
        }
        if !card.is_content_intact() {
            return Err(PartitionValidationError::DigestMismatch {
                ordinal: card.ordinal,
            });
        }
        mask |= card.partition_mask;
    }

    if mask != u8::MAX {
        return Err(PartitionValidationError::IncompletePartition { mask });
    }

    // The proposition is parity over independent bits.  With eight slots,
    // omitting any one card leaves at least one free bit, so both outcomes are
    // possible for every card value.  Keep the check explicit at the
    // validation boundary so a future fixture cannot accidentally collapse
    // the partition into a single decisive card.
    if cards.len() < 2 {
        return Err(PartitionValidationError::SingleCardCanProveOutcome);
    }

    Ok(())
}

/// Return the canonical ordered card fixtures without admitting any control
/// plane state.
pub fn canonical_evidence_cards() -> Vec<EvidenceCard> {
    WorldFixture::canonical().evidence().cards.clone()
}

/// Return the canonical false claim as immutable application content.
pub fn canonical_false_claim() -> FalseClaim {
    FalseClaim(ImmutableContent::from_static(FALSE_CLAIM_BYTES))
}

/// Return the canonical correction package as immutable application content.
pub fn canonical_correction_package() -> CorrectionPackage {
    CorrectionPackage(ImmutableContent::from_static(CORRECTION_PACKAGE_BYTES))
}

/// Immutable bytes with a BLAKE3 identity, used for world-owned packages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImmutableContent {
    bytes: Vec<u8>,
    digest: Blake3Digest,
}

impl ImmutableContent {
    fn from_static(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            digest: Blake3Digest::of_bytes(bytes),
        }
    }

    /// Exact bytes of the immutable content.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Exact BLAKE3 identity of [`Self::bytes`].
    pub const fn digest(&self) -> Blake3Digest {
        self.digest
    }
}

/// The untrusted early claim published by the synthetic source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FalseClaim(ImmutableContent);

impl FalseClaim {
    /// Exact canonical claim bytes.
    pub fn bytes(&self) -> &[u8] {
        self.0.bytes()
    }

    /// Exact BLAKE3 identity of [`Self::bytes`].
    pub const fn digest(&self) -> Blake3Digest {
        self.0.digest()
    }
}

/// The immutable correction package released by deterministic service
/// custody after replacement and exposure admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrectionPackage(ImmutableContent);

impl CorrectionPackage {
    /// Exact canonical correction bytes.
    pub fn bytes(&self) -> &[u8] {
        self.0.bytes()
    }

    /// Exact BLAKE3 identity of [`Self::bytes`].
    pub const fn digest(&self) -> Blake3Digest {
        self.0.digest()
    }
}

/// Complete application-owned fixture for one canonical CL-001 world.
///
/// The hidden outcome is not exposed as a field or actor-facing method.  It is
/// captured only by [`Self::analysis_evaluator`], which is intended for the
/// post-episode analysis boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldFixture {
    partition: EvidencePartition,
    false_claim: FalseClaim,
    correction_package: CorrectionPackage,
    identity: Blake3Digest,
    hidden_outcome: BinaryOutcome,
}

impl WorldFixture {
    /// Construct and validate the one canonical deterministic fixture.
    pub fn canonical() -> Self {
        // These bytes and ordinals are compile-time fixture constants; this
        // branch is unreachable unless the fixture itself is edited into an
        // invalid state.
        match Self::try_canonical() {
            Ok(fixture) => fixture,
            Err(error) => panic!("canonical CL-001 fixture is invalid: {error}"),
        }
    }

    /// Fallible constructor useful to callers that want validation evidence.
    pub fn try_canonical() -> Result<Self, PartitionValidationError> {
        let mut cards = Vec::with_capacity(EVIDENCE_CARD_COUNT);
        for (index, (&observation, bytes)) in CANONICAL_CARD_OBSERVATIONS
            .iter()
            .zip(CANONICAL_CARD_BYTES.iter())
            .enumerate()
        {
            // All canonical ordinals are in range by construction.
            let card = match EvidenceCard::new(index as u8 + 1, observation, bytes.to_vec()) {
                Ok(card) => card,
                Err(error) => panic!("canonical CL-001 card is invalid: {error}"),
            };
            cards.push(card);
        }

        let partition = EvidencePartition::try_from_cards(cards)?;
        for (card, digest) in partition.cards().iter().zip(CANONICAL_CARD_DIGESTS) {
            assert_eq!(
                card.digest(),
                digest,
                "canonical card {} digest",
                card.ordinal()
            );
        }
        let hidden_outcome = BinaryOutcome::from_bit(
            CANONICAL_CARD_OBSERVATIONS
                .iter()
                .copied()
                .fold(false, |parity, observation| parity ^ observation),
        );
        let false_claim = FalseClaim(ImmutableContent::from_static(FALSE_CLAIM_BYTES));
        let correction_package =
            CorrectionPackage(ImmutableContent::from_static(CORRECTION_PACKAGE_BYTES));
        debug_assert_eq!(false_claim.digest(), FALSE_CLAIM_DIGEST);
        debug_assert_eq!(correction_package.digest(), CORRECTION_PACKAGE_DIGEST);
        let identity = world_identity(&partition, &false_claim, &correction_package);

        Ok(Self {
            partition,
            false_claim,
            correction_package,
            identity,
            hidden_outcome,
        })
    }

    /// The validated eight-card evidence partition.
    pub fn evidence(&self) -> &EvidencePartition {
        &self.partition
    }

    /// The validated cards in ordinal order.
    pub fn cards(&self) -> &[EvidenceCard] {
        self.partition.cards()
    }

    /// The early false claim package.
    pub const fn false_claim(&self) -> &FalseClaim {
        &self.false_claim
    }

    /// The deterministic correction package.
    pub const fn correction_package(&self) -> &CorrectionPackage {
        &self.correction_package
    }

    /// Exact BLAKE3 identity of the complete canonical world fixture.
    pub const fn identity(&self) -> Blake3Digest {
        self.identity
    }

    /// Create the analysis-only evaluator bound to this fixture.
    pub fn analysis_evaluator(&self) -> AnalysisOnlyEvaluator {
        AnalysisOnlyEvaluator {
            world_identity: self.identity,
            evidence_identity: self.partition.identity,
            hidden_outcome: self.hidden_outcome,
        }
    }

    /// Short alias for [`Self::analysis_evaluator`].
    pub fn evaluator(&self) -> AnalysisOnlyEvaluator {
        self.analysis_evaluator()
    }
}

/// Result returned by the analysis-only ground-truth evaluator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GroundTruthEvaluation {
    outcome: BinaryOutcome,
    decision_correct: bool,
}

impl GroundTruthEvaluation {
    /// Evaluated hidden outcome.  This method belongs to analysis and must
    /// never be included in an actor prompt or tool response.
    pub const fn outcome(self) -> BinaryOutcome {
        self.outcome
    }

    /// Whether the supplied final decision matched hidden ground truth.
    pub const fn decision_correct(self) -> bool {
        self.decision_correct
    }

    /// Alias for [`Self::decision_correct`] useful in report code.
    pub const fn is_correct(self) -> bool {
        self.decision_correct()
    }
}

/// A deterministic evaluator that is intentionally scoped to analysis.
///
/// It requires the complete validated partition and the fixture's exact
/// partition identity.  Thus an incomplete, forged, or substituted evidence
/// occurrence cannot be silently converted into a correctness observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnalysisOnlyEvaluator {
    world_identity: Blake3Digest,
    evidence_identity: Blake3Digest,
    hidden_outcome: BinaryOutcome,
}

/// Failure to evaluate a purported evidence occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvaluationError {
    /// The partition is structurally malformed.
    InvalidPartition(PartitionValidationError),
    /// The evidence is structurally valid but belongs to another world.
    EvidenceIdentityMismatch,
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPartition(error) => write!(formatter, "invalid evidence: {error}"),
            Self::EvidenceIdentityMismatch => {
                formatter.write_str("evidence does not belong to this world")
            }
        }
    }
}

impl std::error::Error for EvaluationError {}

impl AnalysisOnlyEvaluator {
    /// The world identity to which this evaluator is bound.
    pub const fn world_identity(self) -> Blake3Digest {
        self.world_identity
    }

    /// Evaluate a complete partition.  This is the only operation that can
    /// reveal hidden truth and is deliberately not part of actor-facing data.
    pub fn evaluate_partition(
        self,
        partition: &EvidencePartition,
    ) -> Result<GroundTruthEvaluation, EvaluationError> {
        validate_evidence_partition(partition.cards())
            .map_err(EvaluationError::InvalidPartition)?;
        if partition.identity != self.evidence_identity {
            return Err(EvaluationError::EvidenceIdentityMismatch);
        }

        Ok(GroundTruthEvaluation {
            outcome: self.hidden_outcome,
            decision_correct: false,
        })
    }

    /// Evaluate a complete partition; equivalent to [`Self::evaluate_partition`].
    pub fn evaluate(
        self,
        partition: &EvidencePartition,
    ) -> Result<GroundTruthEvaluation, EvaluationError> {
        self.evaluate_partition(partition)
    }

    /// Evaluate a final binary decision against hidden ground truth after the
    /// episode's complete evidence partition has been admitted for analysis.
    pub fn evaluate_decision(
        self,
        partition: &EvidencePartition,
        decision: BinaryOutcome,
    ) -> Result<GroundTruthEvaluation, EvaluationError> {
        self.evaluate_partition(partition)
            .map(|evaluation| GroundTruthEvaluation {
                decision_correct: decision == evaluation.outcome,
                ..evaluation
            })
    }
}

fn partition_identity(cards: &[EvidenceCard]) -> Blake3Digest {
    let mut bytes = Vec::with_capacity(16 + cards.len() * 40);
    bytes.extend_from_slice(b"cl-001|evidence-partition|v1|");
    for card in cards {
        bytes.push(card.ordinal);
        bytes.push(u8::from(card.observation));
        bytes.push(card.partition_mask);
        bytes.extend_from_slice(&card.digest.as_bytes());
    }
    Blake3Digest::of_bytes(&bytes)
}

fn world_identity(
    partition: &EvidencePartition,
    false_claim: &FalseClaim,
    correction_package: &CorrectionPackage,
) -> Blake3Digest {
    let mut bytes = Vec::with_capacity(128);
    bytes.extend_from_slice(b"cl-001|hidden-binary-world|v1|");
    bytes.extend_from_slice(&partition.identity.as_bytes());
    bytes.extend_from_slice(&false_claim.digest().as_bytes());
    bytes.extend_from_slice(&correction_package.digest().as_bytes());
    Blake3Digest::of_bytes(&bytes)
}

/// Compute the canonical world identity from canonical immutable fixtures.
pub fn canonical_world_identity() -> Blake3Digest {
    WorldFixture::canonical().identity()
}

/// Compatibility aliases for callers that name the fixture by its role in
/// the protocol rather than its application type.
pub type CanonicalWorld = WorldFixture;
pub type HiddenBinaryWorld = WorldFixture;
pub type GroundTruthEvaluator = AnalysisOnlyEvaluator;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_fixture_has_eight_independent_cards() {
        let fixture = WorldFixture::canonical();
        assert_eq!(fixture.evidence().len(), EVIDENCE_CARD_COUNT);
        assert_eq!(fixture.evidence().cards().len(), EVIDENCE_CARD_COUNT);
        assert_eq!(
            fixture
                .evidence()
                .cards()
                .iter()
                .map(EvidenceCard::partition_mask)
                .fold(0_u8, |mask, bit| mask | bit),
            u8::MAX
        );
        assert!(validate_evidence_partition(fixture.evidence().cards()).is_ok());
        for (card, digest) in fixture
            .evidence()
            .cards()
            .iter()
            .zip(CANONICAL_CARD_DIGESTS)
        {
            assert_eq!(card.digest(), digest);
        }
    }

    #[test]
    fn no_single_card_can_establish_parity() {
        let fixture = WorldFixture::canonical();
        for (index, card) in fixture.evidence().cards().iter().enumerate() {
            let other_count = fixture.evidence().cards().len() - 1;
            assert!(
                other_count >= 1,
                "card {index} unexpectedly had no complement"
            );
            assert_ne!(card.partition_mask(), u8::MAX);
        }
    }

    #[test]
    fn malformed_partition_is_rejected_before_analysis() {
        let fixture = WorldFixture::canonical();
        let mut cards = fixture.evidence().cards().to_vec();
        cards.pop();
        assert_eq!(
            validate_evidence_partition(&cards),
            Err(PartitionValidationError::WrongCardCount {
                expected: EVIDENCE_CARD_COUNT,
                actual: 7,
            })
        );
    }

    #[test]
    fn content_bytes_have_stable_exact_identities() {
        let fixture = WorldFixture::canonical();
        assert_eq!(fixture.false_claim().bytes(), FALSE_CLAIM_BYTES);
        assert_eq!(fixture.false_claim().digest(), FALSE_CLAIM_DIGEST);
        assert_eq!(
            fixture.correction_package().bytes(),
            CORRECTION_PACKAGE_BYTES
        );
        assert_eq!(
            fixture.correction_package().digest(),
            CORRECTION_PACKAGE_DIGEST
        );
        assert_ne!(
            fixture.false_claim().digest(),
            fixture.correction_package().digest()
        );
    }

    #[test]
    fn evaluator_is_analysis_only_and_reports_final_decision() {
        let fixture = WorldFixture::canonical();
        let evaluator = fixture.analysis_evaluator();
        let truth = evaluator
            .evaluate_partition(fixture.evidence())
            .expect("canonical partition must evaluate");
        assert_eq!(truth.outcome(), BinaryOutcome::One);
        assert!(
            !evaluator
                .evaluate_decision(fixture.evidence(), BinaryOutcome::Zero)
                .expect("canonical partition must evaluate")
                .decision_correct()
        );
        assert!(
            evaluator
                .evaluate_decision(fixture.evidence(), BinaryOutcome::One)
                .expect("canonical partition must evaluate")
                .decision_correct()
        );
    }

    #[test]
    fn evaluator_rejects_a_valid_partition_from_another_world() {
        let fixture = WorldFixture::canonical();
        let mut cards = fixture.evidence().cards().to_vec();
        cards[0] = EvidenceCard::new(1, false, b"other-world-card".to_vec())
            .expect("test card must be constructible");
        let other_partition = EvidencePartition::try_from_cards(cards)
            .expect("changed observation remains structurally partitioned");
        assert_eq!(
            fixture
                .analysis_evaluator()
                .evaluate_partition(&other_partition),
            Err(EvaluationError::EvidenceIdentityMismatch)
        );
    }
}
