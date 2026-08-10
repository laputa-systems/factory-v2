# Society glossary

## Status and use

This file owns generic vocabulary. A term marked **current** has an implemented
durable or runtime contract. A term marked **planned** is research direction and
must not be represented by a nearby current type.

Experimental-world vocabulary belongs beneath `applications/<experiment>/`.

## Research terms

### Institutional model organism — direction

A bounded artificial society used to study collective behavior whose effective
unit of agency may be the institution rather than an individual model. It is a
laboratory object, not a claim that the society is biological, conscious, or
self-improving.

### Disposable actor — direction

An actor instance which receives one narrow obligation, the minimum admitted
context and authority required for it, and a hard resource budget; emits typed
artifacts, claims, evidence, or uncertainty; relinquishes authority; and
terminates.

Individual agents should be cheap and disposable. Long-lived competence must
not depend on a privileged actor session, hidden scratchpad, or ever-growing
prompt. If state matters after an actor dies, an explicit institutional
transition must preserve it with provenance and scope.

### Stateful society — direction

The durable combination of institutional memory, artifacts, claims, evidence,
policies, work state, and event history. Society increases its state through
typed, challengeable transitions while actor instances remain replaceable.

The design rule is: **do not make actors increasingly stateful; make the
society increasingly stateful.**

### Institutional leverage — planned

The outcome difference between a retained institution and a fresh institution
under the same actor-policy distribution and measured-episode resource budget.
Operational leverage excludes earlier institution-building cost; amortized
leverage includes it.

### Emergent institutional behavior — planned

A population-level behavior which isolated and unstructured actor baselines do
not reliably exhibit under the same evidence access and aggregate resources.
It requires a declared baseline and uncertainty, not an impression that a
multi-agent transcript looks sophisticated.

## Experimental-control terms

### Study protocol revision — planned

An immutable specification of the research question, eligibility rules,
treatment variables, assignment procedure, episode budget, interventions,
measurements, exclusions, and analysis. It cannot require a preferred outcome.

### Episode — planned

One bounded execution under an exact protocol, world revision, population,
institution revision and initial state, treatment assignment, budget,
intervention schedule, measurement specification, and randomization identity.

An Operating Cycle is not an Episode; the former is current authority and
budget machinery, while the latter is a not-yet-implemented scientific unit.

### Treatment assignment — planned

The immutable relation binding an episode to its controlled variable values.
Assignment occurs before treatment-dependent work and remains distinct from an
institution's own decisions.

### Population snapshot — planned

The complete episode-local roster of actor-policy revisions, runtime profiles,
role assignments, and replacement status. It records which actors existed; it
does not preserve their hidden local state.

### Actor-policy revision — planned

The fixed model/runtime/prompt/tool contract from which disposable actor
instances are created for an episode. It is not an actor biography or durable
memory container.

### Institution revision — planned

A sealed executable coordination policy defining information visibility,
knowledge-promotion rules, authority routing, work allocation, and resource
decisions. Its mutable institutional state is separately snapshotted and may be
retained or reset by treatment.

An office title or system prompt alone is not an Institution revision.

### Intervention — planned

A protocol-authorized manipulation at an exact episode point, such as actor
replacement, memory reset, correction release, or communication-edge removal.

### Measurement — planned

A typed observation derived under a named analysis procedure from retained raw
facts. A measurement is not mission authority, reward, selection, or truth by
fiat.

### Experimental fork — planned

A new episode derived from a declared source state with explicit treatment
changes. It preserves a cross-link to the source but has new identity,
authority, events, and outcomes.

### Integrity replay — current

Reconstruction and validation of materialized kernel state from the event
ledger and named command/event bodies. Integrity replay audits one history; it
does not rerun actors or create a counterfactual.

## Institutional terms

### Actor — partial

A bounded participant acting under a declared policy/runtime revision, local
authority, admitted context, and budget. Current Actor attempts provide some
execution custody, but population and disposable-actor contracts are planned.

### Artifact — partial

Immutable content plus separately admitted production, provenance, and
semantic relations. A content object alone proves byte identity only.

### Claim — planned

A challengeable proposition under an experimental-world schema. It preserves
author, scope, institution revision, supporting evidence, status, and
supersession without becoming true merely because it was recorded.

### Evidence — partial

A typed observation or content object admitted under a role, scope, evaluator,
and authority. Admission says the evidence is eligible to be considered; it
does not make a claim true, sufficient, causal, or culturally inherited.

### Objective — partial

A bounded institutional obligation derived from a Mission and local authority.
It is neither the Mission itself nor a scalar fitness score.

### Work — partial

A leased, budgeted, dependent, expiring obligation with explicit completion or
failure. Work admission must respect downstream capacity rather than generate
unbounded queues.

### Institution — planned

A versioned executable coordination policy plus durable state. It governs
information, claim promotion, authority, resource flow, and memory. It survives
actor replacement only through explicit state, never through a hidden immortal
agent.

### Event ledger — current

The append-only record of accepted commands, transitions, resources, and
principals. It is replay-auditable and distinct from content bytes, application
ground truth, and current institutional interpretation.

### Institutional memory — planned

Durable, typed state admitted from actor outputs or institutional operations.
It retains provenance, scope, revision, challenge status, and downstream
dependencies. Transcript accumulation by itself is not institutional memory.

### Knowledge promotion — planned

An authorized transition from private observation or local claim toward
candidate, validated, or institutional knowledge. Each level has a named
evidence rule and challenge path.

### Checked propagation — planned

The recorded release of a particular knowledge revision to named recipients or
work, plus reverse dependencies used to route corrections. Visibility is not
semantic acceptance, and provenance is not causality.

## Current trusted-physics terms

### Application identity and revision

The opaque generic identity of an experimental world and one immutable revision
of its contracts. The generic apparatus persists identity and lineage without
interpreting world semantics.

### Mission

A typed normative input consisting of a statement, ordered principles, four
north-star questions, and an exact source-rendering digest. Mission is distinct
from measurements and selection policy.

### Founding mission source operation

The daemon-private sequence which digest-checks bounded mission bytes,
preflights without mutation, seals the bytes, records their receipt/object, and
then installs the founding mission. It proves byte identity, not producer
provenance or scientific validity.

### Project north-star alignment

The four-field application-revision-bound explanation of a Project's proposed
change, improvement evidence, boundary commitment, and revisit condition. It is
not a second mission.

### Capability

Typed permission for one command family under exact jurisdiction, occupancy,
expiry, generation, and state. Prompts and titles do not grant capabilities.

### Operating Cycle

A bounded current execution/governance interval with a pinned mission revision,
budget envelope, admission generation, cancellation root, and execution
profile. It is not yet a scientific Episode.

### Budget envelope and reservation

A hard integer resource ceiling and an exact held allocation beneath it. A
budget is not a reward target. Unknown or unavailable cost freezes admission
rather than becoming zero.

### Cancellation request and propagation

The durable close-admission and owned-work reconciliation line. Cancellation
propagates obligations to exact targets and does not become complete merely
because a signal was attempted.

### Native child lifecycle

The generic OS custody of one admitted child: PID/PGID, liveness, signals,
direct wait, bounded streams, sealing, and finalization. Pi may attach a strict
session/protocol sidecar; a deterministic evaluator does not.

### Content object and content seal receipt

A global immutable byte identity and the narrow attestation that the resident
physical boundary sealed it. Sealing establishes identity only, not schema,
meaning, provenance, or evidence.

### Deterministic evaluator schedule claim

A daemon-facing transaction selecting the oldest eligible registered evaluator
experiment and deriving its exact native-child admission. It is current private
custody machinery, not a population scheduler or experimental treatment.

### Deterministic observation

The current generic evidence role derivable only from an exact claimed
evaluator receipt, complete streams, finalized child, application-revision
alignment, and direct exit zero. Its limitation remains
`ApplicationSemanticsUninterpreted`.

### Pi session boundary

The strict SDK-host control and observation grammar attached to an admitted
native child. It is one replaceable actor-runtime adapter, not the definition
of an Actor or Institution.

### Controlled product materialization

Dormant provider-free Git mechanics for applying an already-authorized patch,
validating a tree, constructing a commit, and guarded local delivery. It is
outside CL-001 and currently receives caller-supplied structural authorization,
not kernel-issued product authority.

## Required distinctions

| Keep separate | Reason |
| --- | --- |
| Actor instance / actor-policy revision | Disposable execution / reproducible phenotype |
| Actor state / institutional state | Dies with one actor / survives through admitted transitions |
| Mission / measurement / selection | Normative purpose / observation / resource choice |
| Ground truth / institutional belief | Experimental fact / society's challengeable interpretation |
| Content identity / evidence | Exact bytes / typed epistemic role |
| Provenance / causality | Declared lineage / intervention-supported effect |
| Integrity replay / experimental fork | Audit one history / run a new comparison |
| Native process success / application success | Physical exit and seals / world-owned semantics |
| Office authority / Institution | Current root governance / versioned research treatment |
