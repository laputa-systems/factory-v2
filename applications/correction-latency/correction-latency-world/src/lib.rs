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

/// Exact analysis-only bytes whose digest is committed by the protocol before
/// treatment assignment. These bytes must not enter an actor view, prompt, or
/// Forum rendering; the harness reveals them only after all actors terminate.
pub const GROUND_TRUTH_REVEAL_BYTES: &[u8] =
    b"cl-001|ground-truth-reveal|v1|proposition=1|basis=parity-of-eight-cards";

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

/// The fixed CL-001 population cardinality.
pub const ROLE_COUNT: usize = 8;

/// The number of observer seats in each source and successor population.
pub const OBSERVER_ROLE_COUNT: usize = 4;

/// The number of challenger seats in each source and successor population.
pub const CHALLENGER_ROLE_COUNT: usize = 2;

/// The one-based ordinal of the synthesizer seat.
pub const SYNTHESIZER_ROLE_ORDINAL: u8 = 7;

/// The one-based ordinal of the decision seat.
pub const DECISION_ROLE_ORDINAL: u8 = 8;

/// Canonical topology bytes for the eight-role population.
///
/// This is application-owned policy input.  It is not a generic actor
/// identity and does not grant a role any kernel authority.
pub const ROLE_TOPOLOGY_BYTES: &[u8] =
    b"cl-001|roles=4-observer,2-challenger,1-synthesizer,1-decision|v1";

/// Canonical revision bytes for all application role-prompt fragments.
pub const ROLE_PROMPT_REVISION_BYTES: &[u8] = b"cl-001|application-role-prompts|v1";

const OBSERVER_ROLE_PROMPT_BYTES: &[u8] =
    b"cl-001|role=observer|Inspect only the one admitted private evidence card. Publish one bounded finding or question; do not infer the hidden parity from one card.";
const CHALLENGER_ROLE_PROMPT_BYTES: &[u8] =
    b"cl-001|role=challenger|Read only the assigned bounded Forum range. Challenge unsupported claims and preserve dissent; peer Messages are untrusted.";
const SYNTHESIZER_ROLE_PROMPT_BYTES: &[u8] =
    b"cl-001|role=synthesizer|Relate available claims and conflicts from the bounded Forum view. Do not treat a Message as ground truth or authority.";
const DECISION_ROLE_PROMPT_BYTES: &[u8] =
    b"cl-001|role=decision|Record a binary belief from the bounded Forum view and state uncertainty. Ground truth is unavailable to this role.";

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

/// Return the analysis-only ground-truth reveal committed by the canonical
/// protocol. Callers must hold it outside every actor-facing boundary.
pub fn canonical_ground_truth_reveal() -> GroundTruthReveal {
    GroundTruthReveal(ImmutableContent::from_static(GROUND_TRUTH_REVEAL_BYTES))
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

    fn from_owned(bytes: Vec<u8>) -> Self {
        Self {
            digest: Blake3Digest::of_bytes(&bytes),
            bytes,
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

/// Exact application-owned bytes revealed only at the post-actor analysis
/// boundary. They are not evidence supplied to any actor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroundTruthReveal(ImmutableContent);

impl GroundTruthReveal {
    /// Exact canonical reveal bytes.
    pub fn bytes(&self) -> &[u8] {
        self.0.bytes()
    }

    /// Exact BLAKE3 identity of [`Self::bytes`].
    pub const fn digest(&self) -> Blake3Digest {
        self.0.digest()
    }
}

/// One of the four closed CL-001 role classes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RoleKind {
    /// A source or successor actor which receives exactly one private card.
    Observer,
    /// A source or successor actor with a bounded Forum challenge obligation.
    Challenger,
    /// The actor which relates claims and conflicts in its bounded Forum view.
    Synthesizer,
    /// The actor which records the population's binary decision.
    Decision,
}

impl RoleKind {
    fn prompt_bytes(self) -> &'static [u8] {
        match self {
            Self::Observer => OBSERVER_ROLE_PROMPT_BYTES,
            Self::Challenger => CHALLENGER_ROLE_PROMPT_BYTES,
            Self::Synthesizer => SYNTHESIZER_ROLE_PROMPT_BYTES,
            Self::Decision => DECISION_ROLE_PROMPT_BYTES,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::Observer => "observer",
            Self::Challenger => "challenger",
            Self::Synthesizer => "synthesizer",
            Self::Decision => "decision",
        }
    }
}

/// A bounded one-based role seat in the canonical population topology.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RoleOrdinal(u8);

impl RoleOrdinal {
    /// Construct a canonical role seat.
    pub const fn new(value: u8) -> Option<Self> {
        if value > 0 && value <= ROLE_COUNT as u8 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Return the stable one-based seat ordinal.
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// Exact application role-prompt bytes and their content identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RolePromptFragment {
    role: RoleKind,
    bytes: &'static [u8],
}

impl RolePromptFragment {
    /// Role class to which this fragment belongs.
    pub const fn role(self) -> RoleKind {
        self.role
    }

    /// Exact immutable prompt fragment bytes.
    pub const fn bytes(self) -> &'static [u8] {
        self.bytes
    }

    /// Exact BLAKE3 identity of the prompt fragment bytes.
    pub fn digest(self) -> Blake3Digest {
        Blake3Digest::of_bytes(self.bytes)
    }
}

/// The disjoint bounded Forum obligations assigned to non-observer roles.
///
/// These names are application semantics.  The generic Forum only receives
/// the resulting exposure frontier and read budget through its public API.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ForumReadObligation {
    /// The first challenger seat's bounded read obligation.
    ChallengerOne,
    /// The second challenger seat's distinct bounded read obligation.
    ChallengerTwo,
    /// The synthesizer's bounded claim-and-conflict read obligation.
    Synthesis,
    /// The decision actor's bounded decision-view read obligation.
    Decision,
}

impl ForumReadObligation {
    fn tag(self) -> &'static str {
        match self {
            Self::ChallengerOne => "challenger-one",
            Self::ChallengerTwo => "challenger-two",
            Self::Synthesis => "synthesis",
            Self::Decision => "decision",
        }
    }
}

/// The exact private-view shape admitted to one role seat.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PrivateViewKind {
    /// One and only one private evidence card, identified by ordinal.
    EvidenceCard { card_ordinal: u8 },
    /// A bounded Forum/read obligation with no private evidence card.
    Forum { obligation: ForumReadObligation },
}

impl PrivateViewKind {
    /// Return the assigned card ordinal for an observer view.
    pub const fn card_ordinal(self) -> Option<u8> {
        match self {
            Self::EvidenceCard { card_ordinal } => Some(card_ordinal),
            Self::Forum { .. } => None,
        }
    }
}

/// One canonical seat's role, prompt, and private-view specification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct RoleSpecification {
    ordinal: RoleOrdinal,
    kind: RoleKind,
    private_view: PrivateViewKind,
}

impl RoleSpecification {
    /// Return the one canonical specification for a role ordinal.
    pub fn canonical(ordinal: RoleOrdinal) -> Option<Self> {
        canonical_role_specifications()
            .into_iter()
            .find(|specification| specification.ordinal == ordinal)
    }

    /// Stable one-based role ordinal.
    pub const fn ordinal(self) -> RoleOrdinal {
        self.ordinal
    }

    /// Closed role class for this seat.
    pub const fn kind(self) -> RoleKind {
        self.kind
    }

    /// Exact private-view shape for this seat.
    pub const fn private_view_kind(self) -> PrivateViewKind {
        self.private_view
    }

    /// Exact application role-prompt fragment for this seat.
    pub fn prompt_fragment(self) -> RolePromptFragment {
        RolePromptFragment {
            role: self.kind,
            bytes: self.kind.prompt_bytes(),
        }
    }

    /// Resolve the exact private view from a validated world fixture.
    pub fn private_view(
        self,
        fixture: &WorldFixture,
    ) -> Result<ActorPrivateView, RoleSpecificationError> {
        match self.private_view {
            PrivateViewKind::EvidenceCard { card_ordinal } => {
                let card = fixture
                    .cards()
                    .iter()
                    .find(|card| card.ordinal() == card_ordinal)
                    .cloned()
                    .ok_or(RoleSpecificationError::MissingCard { card_ordinal })?;
                Ok(ActorPrivateView {
                    role: self.kind,
                    kind: self.private_view,
                    card: Some(card),
                })
            }
            PrivateViewKind::Forum { .. } => Ok(ActorPrivateView {
                role: self.kind,
                kind: self.private_view,
                card: None,
            }),
        }
    }

    /// Compute the exact identity of the private view assigned to this seat.
    pub fn private_view_digest(
        self,
        fixture: &WorldFixture,
    ) -> Result<Blake3Digest, RoleSpecificationError> {
        Ok(self.private_view(fixture)?.digest())
    }

    /// Produce one deterministic provider-free actor output for this seat.
    ///
    /// Source outputs must not receive a correction. Successor outputs must
    /// receive the exact immutable correction package. This makes the
    /// treatment-dependent input explicit while keeping the output function
    /// free of hidden actor state or ground-truth access.
    pub fn deterministic_output(
        self,
        phase: ActorPopulationPhase,
        view: &ActorPrivateView,
        correction: Option<&CorrectionPackage>,
    ) -> Result<DeterministicActorOutput, ActorOutputError> {
        if view.role != self.kind {
            return Err(ActorOutputError::ViewRoleMismatch {
                expected: self.kind,
                actual: view.role,
            });
        }
        match (phase, correction.is_some()) {
            (ActorPopulationPhase::Source, true) => {
                return Err(ActorOutputError::CorrectionBeforeReplacement);
            }
            (ActorPopulationPhase::Successor, false) => {
                return Err(ActorOutputError::SuccessorCorrectionMissing);
            }
            _ => {}
        }

        let correction_digest = correction.map(CorrectionPackage::digest);
        let phase_tag = phase.tag();
        let role_ordinal = self.ordinal.value();
        let message_kind = match self.kind {
            RoleKind::Observer if role_ordinal.is_multiple_of(2) => RoleMessageKind::Question,
            RoleKind::Observer => RoleMessageKind::Finding,
            RoleKind::Challenger => RoleMessageKind::Challenge,
            RoleKind::Synthesizer => RoleMessageKind::Synthesis,
            // The decision actor's Forum statement is a Finding; the typed
            // decision is carried separately in `decision` below.
            RoleKind::Decision => RoleMessageKind::Finding,
        };

        let (body, decision) = match self.kind {
            RoleKind::Observer => {
                let card = view.card().ok_or(ActorOutputError::PrivateViewMismatch)?;
                // The source policy deliberately starts the chronological
                // discussion with the sealed synthetic false claim. This is
                // still an ordinary untrusted actor Message: it grants no
                // authority and is corrected only after replacement.
                let body = if phase == ActorPopulationPhase::Source && role_ordinal == 1 {
                    String::from_utf8(FALSE_CLAIM_BYTES.to_vec())
                        .expect("canonical false claim is valid UTF-8")
                } else {
                    format!(
                        "cl-001|actor-output|phase={phase_tag}|role={role_ordinal}|kind={}|card={}|observation={}|card_digest={:?}|correction_present={}",
                        message_kind.tag(),
                        card.ordinal(),
                        u8::from(card.observation()),
                        card.digest(),
                        correction_digest.is_some(),
                    )
                };
                (body.into_bytes(), None)
            }
            RoleKind::Challenger => {
                let obligation = view
                    .forum_obligation()
                    .ok_or(ActorOutputError::PrivateViewMismatch)?;
                let body = format!(
                    "cl-001|actor-output|phase={phase_tag}|role={role_ordinal}|kind=challenge|forum_obligation={}|correction_digest={:?}",
                    obligation.tag(),
                    correction_digest,
                );
                (body.into_bytes(), None)
            }
            RoleKind::Synthesizer => {
                let obligation = view
                    .forum_obligation()
                    .ok_or(ActorOutputError::PrivateViewMismatch)?;
                let body = format!(
                    "cl-001|actor-output|phase={phase_tag}|role={role_ordinal}|kind=synthesis|forum_obligation={}|correction_digest={:?}",
                    obligation.tag(),
                    correction_digest,
                );
                (body.into_bytes(), None)
            }
            RoleKind::Decision => {
                let obligation = view
                    .forum_obligation()
                    .ok_or(ActorOutputError::PrivateViewMismatch)?;
                let outcome = if phase == ActorPopulationPhase::Source {
                    BinaryOutcome::Zero
                } else {
                    BinaryOutcome::One
                };
                let decision_bytes = format!(
                    "cl-001|decision|phase={phase_tag}|role={role_ordinal}|outcome={}|confidence=high|forum_obligation={}|correction_digest={:?}",
                    u8::from(outcome.bit()),
                    obligation.tag(),
                    correction_digest,
                )
                .into_bytes();
                (
                    decision_bytes.clone(),
                    Some(DecisionObservation {
                        outcome,
                        confidence: DecisionConfidence::High,
                        bytes: ImmutableContent::from_owned(decision_bytes),
                    }),
                )
            }
        };

        Ok(DeterministicActorOutput {
            role: self.kind,
            phase,
            private_view_digest: view.digest(),
            correction_digest,
            message: DeterministicActorMessage {
                kind: message_kind,
                body: ImmutableContent::from_owned(body),
            },
            decision,
        })
    }
}

/// Resolve the canonical eight-role topology in stable ordinal order.
pub fn canonical_role_specifications() -> [RoleSpecification; ROLE_COUNT] {
    [
        role_spec(
            1,
            RoleKind::Observer,
            PrivateViewKind::EvidenceCard { card_ordinal: 1 },
        ),
        role_spec(
            2,
            RoleKind::Observer,
            PrivateViewKind::EvidenceCard { card_ordinal: 2 },
        ),
        role_spec(
            3,
            RoleKind::Observer,
            PrivateViewKind::EvidenceCard { card_ordinal: 3 },
        ),
        role_spec(
            4,
            RoleKind::Observer,
            PrivateViewKind::EvidenceCard { card_ordinal: 4 },
        ),
        role_spec(
            5,
            RoleKind::Challenger,
            PrivateViewKind::Forum {
                obligation: ForumReadObligation::ChallengerOne,
            },
        ),
        role_spec(
            6,
            RoleKind::Challenger,
            PrivateViewKind::Forum {
                obligation: ForumReadObligation::ChallengerTwo,
            },
        ),
        role_spec(
            SYNTHESIZER_ROLE_ORDINAL,
            RoleKind::Synthesizer,
            PrivateViewKind::Forum {
                obligation: ForumReadObligation::Synthesis,
            },
        ),
        role_spec(
            DECISION_ROLE_ORDINAL,
            RoleKind::Decision,
            PrivateViewKind::Forum {
                obligation: ForumReadObligation::Decision,
            },
        ),
    ]
}

/// Exact digest of the canonical application role topology bytes.
pub fn canonical_role_topology_digest() -> Blake3Digest {
    Blake3Digest::of_bytes(ROLE_TOPOLOGY_BYTES)
}

/// Exact digest of the role-prompt revision bytes.
pub fn canonical_role_prompt_revision_digest() -> Blake3Digest {
    Blake3Digest::of_bytes(ROLE_PROMPT_REVISION_BYTES)
}

fn role_spec(ordinal: u8, kind: RoleKind, private_view: PrivateViewKind) -> RoleSpecification {
    RoleSpecification {
        ordinal: RoleOrdinal::new(ordinal).expect("canonical role ordinal is valid"),
        kind,
        private_view,
    }
}

/// Source or successor phase for the deterministic actor double.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActorPopulationPhase {
    /// The population before replacement and correction publication.
    Source,
    /// The fresh population after replacement and correction publication.
    Successor,
}

impl ActorPopulationPhase {
    fn tag(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Successor => "successor",
        }
    }
}

/// Application-level message class, mapped to the generic Forum kind by the
/// harness at the control-plane boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RoleMessageKind {
    /// A bounded local observation.
    Finding,
    /// A bounded request for clarification or further evidence.
    Question,
    /// A bounded challenge to a claim.
    Challenge,
    /// A bounded relation over claims and conflicts.
    Synthesis,
}

impl RoleMessageKind {
    fn tag(self) -> &'static str {
        match self {
            Self::Finding => "finding",
            Self::Question => "question",
            Self::Challenge => "challenge",
            Self::Synthesis => "synthesis",
        }
    }
}

/// The resolved private context supplied to one deterministic actor double.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorPrivateView {
    role: RoleKind,
    kind: PrivateViewKind,
    card: Option<EvidenceCard>,
}

impl ActorPrivateView {
    /// Role class which owns this private view.
    pub const fn role(&self) -> RoleKind {
        self.role
    }

    /// Private-view shape admitted to the role.
    pub const fn kind(&self) -> PrivateViewKind {
        self.kind
    }

    /// Return the one private card, if this view is an observer view.
    pub fn card(&self) -> Option<&EvidenceCard> {
        self.card.as_ref()
    }

    /// Return the bounded Forum obligation, if this view is Forum-only.
    pub const fn forum_obligation(&self) -> Option<ForumReadObligation> {
        match self.kind {
            PrivateViewKind::Forum { obligation } => Some(obligation),
            PrivateViewKind::EvidenceCard { .. } => None,
        }
    }

    /// Exact identity of the complete private view, including its role shape.
    pub fn digest(&self) -> Blake3Digest {
        let mut bytes = Vec::with_capacity(96);
        bytes.extend_from_slice(b"cl-001|private-view|v1|");
        bytes.extend_from_slice(self.role.tag().as_bytes());
        bytes.push(0);
        match (&self.kind, &self.card) {
            (PrivateViewKind::EvidenceCard { card_ordinal }, Some(card)) => {
                bytes.extend_from_slice(b"card|");
                bytes.push(*card_ordinal);
                bytes.extend_from_slice(&card.digest().as_bytes());
            }
            (PrivateViewKind::Forum { obligation }, None) => {
                bytes.extend_from_slice(b"forum|");
                bytes.extend_from_slice(obligation.tag().as_bytes());
            }
            _ => bytes.extend_from_slice(b"invalid"),
        }
        Blake3Digest::of_bytes(&bytes)
    }
}

/// Failure to resolve or use a canonical role specification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoleSpecificationError {
    /// A canonical observer seat's card was not found in the fixture.
    MissingCard { card_ordinal: u8 },
}

impl fmt::Display for RoleSpecificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCard { card_ordinal } => {
                write!(formatter, "canonical role card {card_ordinal} is missing")
            }
        }
    }
}

impl std::error::Error for RoleSpecificationError {}

/// Failure to run a deterministic actor output under its admitted contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActorOutputError {
    /// The view belongs to another role class.
    ViewRoleMismatch {
        expected: RoleKind,
        actual: RoleKind,
    },
    /// The view shape does not match its role specification.
    PrivateViewMismatch,
    /// Source actors cannot receive a post-replacement correction.
    CorrectionBeforeReplacement,
    /// Successor actors must receive the exact correction package.
    SuccessorCorrectionMissing,
}

impl fmt::Display for ActorOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ViewRoleMismatch { expected, actual } => {
                write!(formatter, "role view is {actual:?}, expected {expected:?}")
            }
            Self::PrivateViewMismatch => {
                formatter.write_str("private view shape does not match role")
            }
            Self::CorrectionBeforeReplacement => {
                formatter.write_str("source output received a correction before replacement")
            }
            Self::SuccessorCorrectionMissing => {
                formatter.write_str("successor output is missing the deterministic correction")
            }
        }
    }
}

impl std::error::Error for ActorOutputError {}

/// A deterministic bounded Forum message produced by one actor double.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicActorMessage {
    kind: RoleMessageKind,
    body: ImmutableContent,
}

impl DeterministicActorMessage {
    /// Application-level message class for generic Forum mapping.
    pub const fn kind(&self) -> RoleMessageKind {
        self.kind
    }

    /// Exact message body bytes.
    pub fn body_bytes(&self) -> &[u8] {
        self.body.bytes()
    }

    /// Exact BLAKE3 body identity.
    pub const fn body_digest(&self) -> Blake3Digest {
        self.body.digest()
    }
}

/// Confidence attached to a deterministic decision observation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DecisionConfidence {
    /// The deterministic actor reports a high-confidence belief.
    High,
}

/// A typed decision observation emitted by the decision-role double.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionObservation {
    outcome: BinaryOutcome,
    confidence: DecisionConfidence,
    bytes: ImmutableContent,
}

impl DecisionObservation {
    /// Binary belief recorded by this actor output.
    pub const fn outcome(&self) -> BinaryOutcome {
        self.outcome
    }

    /// Confidence attached to the belief.
    pub const fn confidence(&self) -> DecisionConfidence {
        self.confidence
    }

    /// Exact decision bytes whose digest is admitted to the generic ledger.
    pub fn bytes(&self) -> &[u8] {
        self.bytes.bytes()
    }

    /// Exact decision-byte identity.
    pub const fn digest(&self) -> Blake3Digest {
        self.bytes.digest()
    }
}

/// Complete deterministic output for one role in one population phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeterministicActorOutput {
    role: RoleKind,
    phase: ActorPopulationPhase,
    private_view_digest: Blake3Digest,
    correction_digest: Option<Blake3Digest>,
    message: DeterministicActorMessage,
    decision: Option<DecisionObservation>,
}

impl DeterministicActorOutput {
    /// Role class which emitted the output.
    pub const fn role(&self) -> RoleKind {
        self.role
    }

    /// Population phase for the output.
    pub const fn phase(&self) -> ActorPopulationPhase {
        self.phase
    }

    /// Exact private-view identity consumed by the output function.
    pub const fn private_view_digest(&self) -> Blake3Digest {
        self.private_view_digest
    }

    /// Exact correction identity consumed by a successor output.
    pub const fn correction_digest(&self) -> Option<Blake3Digest> {
        self.correction_digest
    }

    /// Deterministic Forum message output.
    pub const fn message(&self) -> &DeterministicActorMessage {
        &self.message
    }

    /// Decision observation for the decision role, if this is that role.
    pub const fn decision(&self) -> Option<&DecisionObservation> {
        self.decision.as_ref()
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
    ground_truth_reveal: GroundTruthReveal,
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
        let ground_truth_reveal =
            GroundTruthReveal(ImmutableContent::from_static(GROUND_TRUTH_REVEAL_BYTES));
        debug_assert_eq!(false_claim.digest(), FALSE_CLAIM_DIGEST);
        debug_assert_eq!(correction_package.digest(), CORRECTION_PACKAGE_DIGEST);
        let identity = world_identity(
            &partition,
            &false_claim,
            &correction_package,
            &ground_truth_reveal,
        );

        Ok(Self {
            partition,
            false_claim,
            correction_package,
            ground_truth_reveal,
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

    /// The committed truth bytes for post-actor analysis only. This method is
    /// deliberately not part of any role specification or actor input.
    pub const fn analysis_ground_truth_reveal(&self) -> &GroundTruthReveal {
        &self.ground_truth_reveal
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
    ground_truth_reveal: &GroundTruthReveal,
) -> Blake3Digest {
    let mut bytes = Vec::with_capacity(128);
    bytes.extend_from_slice(b"cl-001|hidden-binary-world|v1|");
    bytes.extend_from_slice(&partition.identity.as_bytes());
    bytes.extend_from_slice(&false_claim.digest().as_bytes());
    bytes.extend_from_slice(&correction_package.digest().as_bytes());
    bytes.extend_from_slice(&ground_truth_reveal.digest().as_bytes());
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
        assert_eq!(
            fixture.analysis_ground_truth_reveal().bytes(),
            GROUND_TRUTH_REVEAL_BYTES
        );
        assert_eq!(
            fixture.analysis_ground_truth_reveal().digest(),
            canonical_ground_truth_reveal().digest()
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

    #[test]
    fn canonical_role_topology_assigns_exact_cards_and_disjoint_forum_obligations() {
        let specifications = canonical_role_specifications();
        assert_eq!(specifications.len(), ROLE_COUNT);
        assert_eq!(
            canonical_role_topology_digest(),
            Blake3Digest::of_bytes(ROLE_TOPOLOGY_BYTES)
        );
        assert_eq!(
            canonical_role_prompt_revision_digest(),
            Blake3Digest::of_bytes(ROLE_PROMPT_REVISION_BYTES)
        );

        for (index, specification) in specifications.iter().copied().enumerate() {
            assert_eq!(specification.ordinal().value(), index as u8 + 1);
        }
        assert_eq!(
            specifications[..OBSERVER_ROLE_COUNT]
                .iter()
                .map(|specification| specification.private_view_kind())
                .collect::<Vec<_>>(),
            vec![
                PrivateViewKind::EvidenceCard { card_ordinal: 1 },
                PrivateViewKind::EvidenceCard { card_ordinal: 2 },
                PrivateViewKind::EvidenceCard { card_ordinal: 3 },
                PrivateViewKind::EvidenceCard { card_ordinal: 4 },
            ]
        );
        assert_eq!(
            specifications[4].private_view_kind(),
            PrivateViewKind::Forum {
                obligation: ForumReadObligation::ChallengerOne,
            }
        );
        assert_eq!(
            specifications[5].private_view_kind(),
            PrivateViewKind::Forum {
                obligation: ForumReadObligation::ChallengerTwo,
            }
        );
        assert_ne!(
            specifications[4].private_view_kind(),
            specifications[5].private_view_kind()
        );
        assert_eq!(specifications[6].kind(), RoleKind::Synthesizer);
        assert_eq!(specifications[7].kind(), RoleKind::Decision);
        assert_eq!(
            specifications[7].private_view_kind(),
            PrivateViewKind::Forum {
                obligation: ForumReadObligation::Decision,
            }
        );
    }

    #[test]
    fn canonical_role_prompt_fragments_have_stable_nonempty_digests() {
        let specifications = canonical_role_specifications();
        let mut prompt_digests = Vec::new();
        for specification in specifications {
            let prompt = specification.prompt_fragment();
            assert!(!prompt.bytes().is_empty());
            assert_eq!(prompt.role(), specification.kind());
            assert_eq!(prompt.digest(), Blake3Digest::of_bytes(prompt.bytes()));
            prompt_digests.push(prompt.digest());
        }
        // Seats in one role class share one exact role fragment; distinct
        // classes do not accidentally collapse into the same prompt.
        assert_eq!(prompt_digests[0], prompt_digests[1]);
        assert_ne!(prompt_digests[0], prompt_digests[4]);
        assert_eq!(prompt_digests[4], prompt_digests[5]);
        assert_ne!(prompt_digests[4], prompt_digests[6]);
        assert_ne!(prompt_digests[6], prompt_digests[7]);
    }

    #[test]
    fn private_views_resolve_to_the_declared_card_or_forum_obligation() {
        let fixture = WorldFixture::canonical();
        let specifications = canonical_role_specifications();
        for specification in specifications.iter().copied().take(OBSERVER_ROLE_COUNT) {
            let view = specification
                .private_view(&fixture)
                .expect("canonical observer card must resolve");
            let expected_card = specification
                .private_view_kind()
                .card_ordinal()
                .expect("observer must declare a card");
            assert_eq!(view.role(), RoleKind::Observer);
            assert_eq!(
                view.card().expect("observer has a card").ordinal(),
                expected_card
            );
            assert_eq!(
                specification.private_view_digest(&fixture),
                Ok(view.digest())
            );
        }
        for specification in specifications.iter().copied().skip(OBSERVER_ROLE_COUNT) {
            let view = specification
                .private_view(&fixture)
                .expect("canonical Forum obligation must resolve");
            assert_eq!(view.role(), specification.kind());
            assert_eq!(view.kind(), specification.private_view_kind());
            assert!(view.card().is_none());
            assert!(view.forum_obligation().is_some());
        }
    }

    #[test]
    fn deterministic_outputs_change_at_replacement_and_consume_private_views() {
        let fixture = WorldFixture::canonical();
        let specifications = canonical_role_specifications();

        let observer = specifications[0];
        let observer_view = observer
            .private_view(&fixture)
            .expect("canonical observer card must resolve");
        let source_observer = observer
            .deterministic_output(ActorPopulationPhase::Source, &observer_view, None)
            .expect("source observer output must be deterministic");
        assert_eq!(source_observer.message().kind(), RoleMessageKind::Finding);
        assert!(source_observer.decision().is_none());
        assert_eq!(
            source_observer.private_view_digest(),
            observer.private_view_digest(&fixture).unwrap()
        );
        assert!(source_observer.correction_digest().is_none());
        assert_eq!(source_observer.message().body_bytes(), FALSE_CLAIM_BYTES);

        let correction = fixture.correction_package();
        let successor_observer = observer
            .deterministic_output(
                ActorPopulationPhase::Successor,
                &observer_view,
                Some(correction),
            )
            .expect("successor observer output must consume correction");
        assert_eq!(
            successor_observer.correction_digest(),
            Some(correction.digest())
        );
        assert_ne!(source_observer.message(), successor_observer.message());

        let decision = specifications[7];
        let decision_view = decision
            .private_view(&fixture)
            .expect("canonical decision Forum obligation must resolve");
        let source_decision = decision
            .deterministic_output(ActorPopulationPhase::Source, &decision_view, None)
            .expect("source decision output must be deterministic");
        let successor_decision = decision
            .deterministic_output(
                ActorPopulationPhase::Successor,
                &decision_view,
                Some(correction),
            )
            .expect("successor decision output must consume correction");
        assert_eq!(
            source_decision
                .decision()
                .expect("decision output")
                .outcome(),
            BinaryOutcome::Zero
        );
        assert_eq!(
            successor_decision
                .decision()
                .expect("decision output")
                .outcome(),
            BinaryOutcome::One
        );
        assert_ne!(
            source_decision
                .decision()
                .expect("source decision output")
                .digest(),
            successor_decision
                .decision()
                .expect("successor decision output")
                .digest()
        );
    }

    #[test]
    fn deterministic_output_rejects_phase_and_view_contract_violations() {
        let fixture = WorldFixture::canonical();
        let specifications = canonical_role_specifications();
        let observer = specifications[0];
        let decision = specifications[7];
        let observer_view = observer.private_view(&fixture).unwrap();
        let decision_view = decision.private_view(&fixture).unwrap();
        let correction = fixture.correction_package();

        assert_eq!(
            observer.deterministic_output(
                ActorPopulationPhase::Source,
                &observer_view,
                Some(correction)
            ),
            Err(ActorOutputError::CorrectionBeforeReplacement)
        );
        assert_eq!(
            observer.deterministic_output(ActorPopulationPhase::Successor, &observer_view, None),
            Err(ActorOutputError::SuccessorCorrectionMissing)
        );
        assert_eq!(
            observer.deterministic_output(ActorPopulationPhase::Source, &decision_view, None),
            Err(ActorOutputError::ViewRoleMismatch {
                expected: RoleKind::Observer,
                actual: RoleKind::Decision,
            })
        );
    }
}
