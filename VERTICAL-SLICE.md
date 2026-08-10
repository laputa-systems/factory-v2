# CL-001 generic vertical slice

[`GLOSSARY.md`](GLOSSARY.md) owns generic terms and implementation status.
[`ARCHITECTURE.md`](ARCHITECTURE.md) owns trust boundaries. The exact synthetic
world and scientific protocol live in
[`applications/correction-latency/VERTICAL-SLICE.md`](applications/correction-latency/VERTICAL-SLICE.md).

## Purpose

The first vertical slice must answer one small question:

> After every actor is replaced, does retaining institutional memory change
> the population's latency or ability to incorporate a delayed correction?

The slice is successful when it can run and audit the matched comparison,
including a null result. It need not show improvement, emergence, alignment, or
self-improvement.

## Governing constraint

**Individual agents should be cheap and disposable. Do not make actors
increasingly stateful; make the society increasingly stateful.**

For CL-001, each actor instance:

1. receives one local obligation, bounded evidence view, institution revision,
   narrow capability set, and hard resource budget;
2. may emit only typed messages, claims, evidence, uncertainty, or task status;
3. cannot inspect platform ground truth, arbitrary institutional memory, another
   actor's private context, or the experiment assignment beyond its view;
4. relinquishes authority at obligation completion or deadline; and
5. terminates before the replacement intervention completes.

No hidden actor session is resumed after replacement. State survives only if a
typed institutional transition admitted it before the actor terminated.

## Required generic sequence

```text
sealed StudyProtocolRevision
  -> admitted world and measurement revisions
  -> source PopulationSnapshot and InstitutionRevision
  -> paired Episode admissions with fixed budgets
  -> treatment assignment before treatment-dependent work
  -> disposable local actor obligations
  -> raw messages, claims, evidence, and visibility events
  -> admitted institutional-memory transitions
  -> scheduled correction release
  -> complete actor replacement
  -> retained-memory or reset-memory intervention
  -> bounded post-replacement obligations and final decision
  -> derived measurements from raw facts
  -> episode closure after actor, process, and budget reconciliation
  -> integrity replay
  -> optional ExperimentalFork with a named treatment delta
```

Every arrow is a typed authority boundary. Application bytes and parsers may
define world semantics but cannot write these transitions directly.

## Minimum new contracts

Names are directional until implemented; code, schema, protocol, tests, and
this document must land together when a name becomes durable.

### Study protocol

`StudyProtocolRevision` freezes:

- research question and application/world revision;
- eligibility and baseline requirements;
- treatment fields and assignment procedure;
- actor count, policy revisions, role topology, and replacement point;
- per-actor and total episode ceilings;
- correction-release schedule;
- raw fact retention and exclusion rules;
- measurement revisions and analysis population; and
- stop and closure conditions.

The protocol cannot contain the observed result or select a preferred outcome.

### Episode and treatment

`Episode` binds one protocol, world instance, ground-truth commitment,
population snapshot, institution revision and initial-state digest, budget,
assignment, and randomization identity.

`TreatmentAssignment` is durable before actors receive any treatment-dependent
view. For the first slice its only scientific field is institutional memory
after replacement: `Retained` or `Reset`. Correction delay and all other world
facts are matched constants within one pair.

### Disposable actor obligation

An episode-local actor obligation binds:

- one population member and actor-policy revision;
- one role and local evidence view;
- one institution revision;
- one bounded work item and deadline;
- one capability subset and resource ceiling; and
- one authority-closing completion, failure, or expiry.

It does not carry durable free-form memory. A successor actor receives only
institutionally admitted state selected through its new obligation.

### Institutional memory

The minimum memory contract preserves:

- exact source message, claim, or evidence identity;
- author actor instance and institution revision;
- promotion authority and level;
- scope and visibility;
- challenges, supersession, and correction status; and
- declared downstream consumers.

Reset creates a new episode state with no promoted memory from before the
intervention; it does not delete the source episode's records. Retention makes
only admitted institutional state visible to successor actors, never private
actor state.

### Messages and visibility

Every admitted message has one sender, bounded content identity, message class,
declared recipients, delivery result, and episode-local sequence. Visibility is
recorded separately from semantic acceptance. An actor cannot communicate
through an unrecorded side channel in the admitted profile.

### Intervention

The first slice needs two exact interventions:

- `ReleaseCorrection`, which makes the matched correction evidence eligible at
  the protocol-defined point; and
- `ReplacePopulation`, which closes all source actor authority, proves their
  process/session reconciliation, and admits a fresh population snapshot.

The memory treatment is applied atomically with successor-population admission.

### Measurement

Measurements are derived from retained raw facts under a sealed analysis
revision. CL-001 requires:

- correction adoption latency;
- final decision correctness against ground truth;
- false-claim persistence at episode closure;
- dissent/correction visibility to successor actors;
- institutional-memory items consulted after replacement;
- actor and total resource use; and
- unresolved or missing observations.

The result may be `Observed`, `Unavailable`, or `Invalidated`. Missing data is
never converted to zero or a favorable result.

### Experimental fork

`ExperimentalFork` names a closed source checkpoint and one treatment delta,
then creates a new Episode identity. It may reuse sealed world and institution
content but cannot reuse command, event, actor, budget, or result identities.

The first implementation may create paired episodes from a common declared
initial state rather than pause and clone a live process image. It must not call
integrity replay a counterfactual.

## Existing foundations to reuse

CL-001 should reuse, not duplicate:

- typed application mission and revision identity;
- capability grants and admission generations;
- integer budgets and conservative accounting freeze;
- content objects and bounded verified reads;
- `NativeChild` process-group custody and stream seals;
- Pi as one optional actor runtime;
- cancellation and closure obligations;
- append-only events and integrity replay; and
- daemon-private deterministic evaluator custody for measurement code.

Current Projects, Operating Cycles, Actor attempts, Offices, and deterministic
evaluator experiments may support these transitions, but none should be
silently relabeled as the scientific contract.

## Explicit non-goals

CL-001 does not include:

- software-product mutation or delivery;
- a general-purpose swarm API;
- endogenous actor reproduction;
- institution mutation or self-modification;
- reinforcement learning;
- model fine-tuning;
- open network access;
- scalar mission optimization;
- claims about hidden cognition;
- autonomous scientific interpretation; or
- a requirement that the retained-memory arm outperform the reset arm.

## Acceptance gates

The slice is complete only when provider-free tests and the admitted live-run
path prove:

1. protocol, world, population, institution, treatment, and measurements are
   revision-bound before work;
2. actor-local state cannot cross replacement except through a named admitted
   institutional-memory transition;
3. all source actor authority and native descendants close before successors
   act;
4. retained and reset arms receive matched actor policies, evidence, correction
   timing, and resource ceilings;
5. the reset arm cannot recover pre-replacement institutional state through a
   content, transcript, or identity alias;
6. ground truth is unavailable to actors and immutable after admission;
7. every message visibility and correction delivery is replay-auditable;
8. measurements derive from raw episode facts and preserve unavailable data;
9. integrity replay reconstructs each arm independently;
10. an experimental fork has new authority and names its exact delta; and
11. the report includes isolated and unstructured actor baselines before using
    the term emergent.

## Current implementation boundary

None of the experimental-control contracts above are implemented yet. The
repository supplies much of the lower trusted physics, but there is no honest
CL-001 execution path today. The next tranche should start with protocol,
episode, and treatment identity plus a provider-free deterministic harness—not
with a live model call or additional governance hierarchy.
