# Society architecture

## Purpose and boundary

Society is a trusted substrate and experimental control plane for an
institutional model organism. It studies persistent coordination structures
whose behavior may survive replacement of every actor.

The architecture is not a general workflow engine, a swarm framework, an
autonomous software company, or a claim of recursive self-improvement. Its
near-term purpose is to make small institutional hypotheses falsifiable under
bounded, replayable conditions.

The dependency direction is one-way:

```text
applications/<experiment>  ->  generic public boundaries
generic implementation     -X-> experimental-world vocabulary
```

An experimental world owns ground truth, evidence-card meaning, actor-local
obligations, and measurement interpretation. The generic apparatus owns only
the physics and control necessary to run the experiment honestly.

The cross-cutting design law is: **individual agents should be cheap and
disposable. Do not make actors increasingly stateful; make the society
increasingly stateful.** An actor-local session is temporary execution state,
never institutional memory. Any state used after that actor terminates must
cross a typed durable boundary with provenance, scope, and challenge status.

## Four planes

### 1. Trusted physics

The resident Rust authority owns the SQLite ledger, content-store writer,
native process registry, capability checks, budgets, cancellation, and durable
mutation protocol. Clients and actors cannot write SQL, manufacture lifecycle
state, select executable paths, or supervise authority-bearing children.

Each durable command follows one closed path:

```text
authenticate principal and capability
  -> validate jurisdiction, generation, references, and budget
  -> append one immutable event and named materialized facts
  -> return a typed receipt or typed rejection
```

The local control protocol is binary, length-prefixed, and tag-discriminated.
JSON exists only at the separately versioned Pi SDK-host boundary. SQLite has
no generic payload columns, metadata maps, or stringly discriminants.

### 2. Institutional substrate

The intended societal waist consists of eight concepts:

```text
Actor  Artifact  Claim  Evidence  Objective  Work  Institution  Event
```

The implementation has portions of these concepts but not this complete
institutional contract.

- An `Actor` is an admitted, bounded participant with a declared policy/runtime
  identity and local authority. Actor instances are disposable.
- An `Artifact` is immutable content plus a separately admitted production and
  semantic relation.
- A `Claim` is a challengeable proposition under an application-owned schema.
- `Evidence` is an admitted observation or argument linked to a claim and
  scope. Admission does not make it true.
- An `Objective` is a bounded institutional obligation derived from the
  mission; it is not a scalar reward.
- `Work` is leased, budgeted, dependent, expiring, and backpressured.
- An `Institution` is a versioned executable coordination policy plus durable
  state governing information, authority, promotion, and resource flow.
- An `Event` is append-only trusted occurrence history.

Institutional state changes only through typed transitions. A prompt, title,
citation count, or model assertion does not create authority.

The first institutional mechanism is a chronological, episode-scoped Forum.
It provides immutable attributed Threads and Messages, bounded explicit
read/post actions, exact exposure frontiers, and read receipts. It has no
ranking, consensus, reputation, karma, live interrupt, or global feed in
CL-001. Those are potential experimental treatments, not missing conveniences.

### 3. Experimental control

The experiment plane will own:

```text
StudyProtocolRevision
Episode
TreatmentAssignment
PopulationSnapshot
InstitutionRevision
Intervention
Measurement
ExperimentalFork
```

These contracts are implemented as a narrow provider-free generic boundary:
`StudyProtocolRevision`, `Episode`, `TreatmentAssignment`, population
snapshots, F0 Forum state, measurements, and a linked `ExperimentalFork` have
closed Rust/SQLite representations. They are not aliases for nearby legacy
types. A current deterministic evaluator occurrence is not an `Episode`, an
Operating Cycle is not a `StudyProtocolRevision`, and ledger replay is not an
experimental replay. The present `ExperimentalFork` records a new linked
episode and declared treatment delta; it does not itself execute a
counterfactual rerun.

An episode freezes all variables needed to interpret its result: world,
population, institutional state, budget, treatment, intervention schedule,
measurement specification, and randomization identity. A fork creates a new
episode linked to its source and varies only declared treatment fields.

### 4. Experimental world

An application supplies a bounded mission, synthetic world, evidence and
output schemas, deterministic fixtures, and analysis procedures. It may parse
application semantics but cannot acquire resident authority through them.

The first world is the correction-latency laboratory beneath
`applications/correction-latency/`. Its provider-free deterministic harness
uses only the public generic control boundary. It does not stand in for the
later live Pi/native-child custody profile.

## Mission and constitutional input

One active application mission revision gives an experimental world its
normative purpose. It remains distinct from measurements and selection policy.

```text
ApplicationMissionInput
  ApplicationIdentity
  ApplicationRevision
  MissionStatement
  ordered MissionPrinciple relation
  NorthStarQuestionSet
  source-rendering BLAKE3 digest
```

The current kernel normalizes that input, physically binds its separately
provided bounded rendering through daemon-private content custody, and requires
Projects to cite the founding revision through four-field
`ProjectNorthStarAlignment`. This establishes identity and authority, not
scientific validity or semantic truth.

Future institution revisions may interpret a mission differently. They may not
silently mutate the mission, its source bytes, or an earlier episode.

## Actor and process custody

The generic `NativeChild` nucleus owns one PID/PGID, liveness probes, group
signals, direct wait, and bounded stdout/stderr capture. Pi is an optional
strict session/protocol sidecar over that process identity. Evaluator-owned
children carry no Pi identity or protocol state.

The current daemon-private deterministic-evaluator coordinator can claim an
already registered experiment, allocate a private workspace only after the
claim, materialize verified executable/input bytes, spawn with fixed argv and
an empty environment, record custody, reap the full process group, seal both
streams, finalize the child, and derive an exact stdout occurrence. The kernel
can admit an opaque `DeterministicObservation` only for the exact claimed child
with complete streams and direct exit zero.

This is valuable trusted physics. It is not an experimental scheduler: the
resident serving loop does not invoke it, recovery and workspace disposal are
unfinished, application semantics remain uninterpreted, and no population or
treatment assignment exists.

## Information and knowledge

Platform facts and institutional belief occupy separate relations.

```text
raw occurrence
  -> private observation
  -> local claim
  -> candidate knowledge
  -> validated knowledge
  -> institutional knowledge
```

The application defines what claim classes and evidence mean. The generic
substrate enforces identities, authority, cardinality, scope, revision, and
challenge paths. Promotion never deletes dissenting or superseded records.

Checked propagation records where an admitted knowledge revision was made
visible and which later work declared it as an input. Corrections traverse
declared reverse dependencies. These records establish routing and provenance,
not causal effect.

### Forum communication

The Forum occupies the middle layer between actor-private work and governed
knowledge. A Message is durable peer communication. It may be read, replied to,
challenged, corrected, or voluntarily used without becoming Evidence or
institutional truth.

CL-001 preserves Forum history in both arms for audit while varying successor
visibility through one exact exposure frontier. Retained successors may read
the pre-replacement chronological Thread; reset successors may read only
Messages published after their new frontier. A deterministic service publishes
the same correction after both replacement populations and exposure frontiers
are admitted.

Mutable Forum Messages never enter the Pi system prompt. The prompt receives a
sealed, digest-bound Forum awareness fragment describing only tools actually
available, public durability, untrusted peer content, and the obligation's
bounds. Both treatment arms receive byte-identical fragments.

## Observability

The laboratory aims to capture every consequential system-mediated boundary:

- admitted message and recipient visibility;
- claim, citation, challenge, and promotion;
- artifact and exact content identity;
- capability and institutional-policy decision;
- work lease, resource transfer, and budget charge;
- actor/runtime/population identity;
- intervention and measurement; and
- downstream declared dependency.

It does not claim access to latent model state or truthful chain-of-thought.
Whether a collective strategy was distributed must be tested through behavior,
replacement, ablation, and counterfactual intervention.

## Replay and causality

Integrity replay rebuilds materialized state from the event ledger and rejects
tampering. Experimental replay creates a new episode fork. They share content
and lineage primitives but never share identity or authority.

A provenance edge means “declared as input to.” It does not mean “caused.” A
causal-support relation is admitted only under a named experimental design
which identifies its intervention, comparison, and analysis.

## Resources and selection

Budgets are hard integer ceilings, never spend targets. Known cost reconciles
exactly. Unknown cost freezes later paid admission rather than becoming zero.
Work queues are bounded and must expose congestion instead of creating
unbounded tickets.

Selection is absent from the first slice. When later introduced, a
`SelectionPolicyRevision` will be an experimental treatment, not mission
authority. Actor reproduction and institution mutation cannot modify trusted
physics or retroactively alter fitness evidence.

## Safety

- Ordinary tests are provider-free and network-free.
- Live actor runs require fixed runtime identity, bounded authority, explicit
  resource ceilings, and an admitted stop condition.
- Actors cannot mutate the kernel, experimental protocol, raw occurrences, or
  their own evaluation.
- Institutional policies are sealed revisions selected before an episode.
- Every actor and native descendant must be reconciled before episode closure.
- Raw ground truth remains accessible to the experiment authority but is never
  exposed to actors unless the treatment says so.

## Current implementation boundary

Implemented trusted physics includes founding mission custody, project
alignment, authority, Operating Cycles, budgets, actor attempts, cancellation,
native/Pi process custody, content identity, deterministic evaluator receipts,
opaque evidence admission, and integrity replay.

Implemented provider-free are the first-class generic study ledger, matched
episode/treatment/pair admission, F0 Forum Message/exposure/read-receipt
transitions, actor-obligation replacement, atomic matched correction release,
committed post-actor ground-truth reveal, closed measurement statuses,
experimental forks, and materialized-state integrity replay. The
correction-latency application owns the synthetic world, analysis-only
evaluator, deterministic actor doubles, and paired report.

Not implemented are a daemon-owned live study scheduler and paired live
episode runner. The daemon now owns the lower-level Forum-obligation bridge
and durable binding to actual resident Pi/native-child custody; knowledge
promotion and general institution-policy runtime remain out of scope. The Pi host has a separately admitted
closed Forum custom-tool transport and a reduced direct qualification smoke;
that adapter path does not establish daemon custody or canonical study
evidence. Product-delivery mechanics remain dormant and outside the present
slice.

The next implementation work is limited to the contracts required by CL-001.
No general swarm, hierarchy, reproduction, RL, or self-improvement framework is
authorized by this architecture.

## Invariants

- Generic implementation contains no experimental-world vocabulary.
- Application semantics do not become authority through opaque data.
- Raw occurrence, curated belief, and ground truth are distinct.
- Mission, measurement, and selection are distinct.
- Actor, population, institution, and episode revisions are explicit.
- Integrity replay and experimental replay have different types and identities.
- No causal claim is derived from provenance alone.
- Actor replacement cannot silently retain actor-local state.
- An accepted transition has named authoritative rows and replay evidence.
- A null experimental result is permitted; tests validate integrity, not the
  desired direction of an effect.
