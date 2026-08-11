# CL-001 generic vertical slice

[`GLOSSARY.md`](GLOSSARY.md) owns generic terms and implementation status.
[`ARCHITECTURE.md`](ARCHITECTURE.md) owns trust boundaries, and
[`FORUM.md`](FORUM.md) owns the staged communication substrate. The exact
synthetic world and scientific protocol live in
[`applications/correction-latency/VERTICAL-SLICE.md`](applications/correction-latency/VERTICAL-SLICE.md).

## Purpose

The first vertical slice must answer one small question:

> After every actor is replaced, does retaining the chronological Forum history
> change the population's latency or ability to incorporate an identical
> post-replacement correction?

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
  -> sealed application RunPlan admitted by content identity and digest
  -> source PopulationSnapshot, InstitutionRevision, and EpisodeForum
  -> paired Episode admissions with fixed budgets
  -> finite matched-pair set registered against the RunPlan
  -> treatment assignment before treatment-dependent work
  -> disposable source-actor obligations with identical Forum prompt contract
  -> immutable attributed chronological Messages and read receipts
  -> exact pre-replacement Thread head
  -> complete source-actor replacement
  -> Retained or Reset successor Forum exposure
  -> identical deterministic-service correction published to both arms
  -> bounded successor obligations, Forum reads/posts, and final decision
  -> derived measurements from raw facts
  -> episode closure after actor, process, and budget reconciliation
  -> integrity replay
  -> optional ExperimentalFork with a named treatment delta
```

Every arrow is a typed authority boundary. Application bytes and parsers may
define world semantics but cannot write these transitions directly.

## Implemented provider-free contracts

The generic boundary is implemented as the closed
`CommandBody::StudyTransition` / `EventBody::StudyTransition` ledger family
and its normalized `study_*` state. Its alternatives are the closed
`StudyCommand` and `StudyEvent` enums in `crates/society-kernel/src/study.rs`;
they are not application payloads. `KernelStore::execute_study_transition`
is the service-custody bridge used by provider-free doubles, and
`KernelStore::validate_replayed_materialized_state` reconstructs the accepted
shared ledger into a fresh schema before comparing all study and Forum state.
`StudyRun` is the durable restart anchor for a finite experiment: it retains
only a sealed application plan `ContentObject` identity, its digest, the
protocol revision, and ordinal matched-pair registrations. The kernel never
parses plan bytes; `KernelStore::study_run_observation` returns just the typed
custody facts a resident coordinator needs to resume safely. Its durable
lifecycle is `Pairing → Ready → Running → Completed`; completion is accepted
only after both episodes of every registered pair independently close, so a
planned analysis can distinguish a started run from an analysis-eligible one.

The isolated `applications/correction-latency/correction-latency-harness`
admits the canonical world, runs the eight-role retained/reset pair, emits a
deterministic report, and validates replay. It intentionally does not claim a
live provider call, a Pi SDK session, or native-child lifecycle custody.
The repository-root `make run-society` recipe runs this provider-free world and
prints its `WorldSimulationSummary`: accepted actor and Forum activity, study
budget units, measurements, baselines, and replay integrity. Its monetary cost
is explicitly not applicable because no provider-backed agent runs. Live Pi
Office usage has exact ledger-backed provider-cost accounting; a live
end-of-cycle aggregate remains separately scoped.

The separately admitted `forum_isolated_v1` profile is exercised by
`make run-society-paid`. That reduced qualification smoke runs 16 actor
lifetimes with at most 8 native Pi hosts concurrently, applies a hard $0.50
aggregate provider-cost ceiling plus a per-actor guardrail, and prints a
world-simulation-style report with each actor's status, tokens, cost, Forum
reads/posts, and errors. Its actor prompt uses ordinary participant language;
the retained/reset treatment labels remain report metadata and are not shown
to the actors. This runner is direct adapter qualification, not canonical
CL-001 evidence or proof of daemon-owned native-child custody. Pass
`PROVIDER` and `MODEL` to select the same admitted treatment for every actor;
the default is paid Ling 2.6. The provider-scoped catalog retains DeepSeek,
free Ling, Laguna, and paid Ling 2.6 at
`packages/society-pi-host/catalogs/openrouter-admitted-models-v1.json`.

### Study protocol

`StudyProtocolRevision` freezes:

- research question and application/world revision;
- eligibility and baseline requirements;
- treatment fields and assignment procedure;
- actor count, policy revisions, role topology, and replacement point;
- per-actor and total episode ceilings;
- correction-release schedule;
- a BLAKE3 commitment to application-owned ground-truth reveal bytes;
- raw fact retention and exclusion rules;
- measurement revisions and analysis population; and
- stop and closure conditions.

The protocol cannot contain the observed result or select a preferred outcome.
The committed truth bytes are revealed once per episode only after every actor
obligation is terminal and before a measurement result may be recorded. The
generic control stores and checks their identity; the application alone
interprets their world semantics.

### Episode and treatment

`Episode` binds one protocol, world instance, ground-truth commitment,
population snapshot, institution revision and initial-state digest, budget,
assignment, and randomization identity.

`TreatmentAssignment` is durable before actors receive any treatment-dependent
view. For the first slice its only scientific field is pre-replacement Forum
visibility after replacement: `Retained` or `Reset`. Correction publication,
actor policy, tools, prompt fragment, evidence, topology, and all budgets are
matched constants within one pair.

### Disposable actor obligation

An episode-local actor obligation binds:

- one population member and actor-policy revision;
- one role and local evidence view;
- one institution revision;
- one bounded work item and deadline;
- one capability subset and resource ceiling; and
- one authority-closing completion or durable failure; a live deadline/expiry
  transition remains required for a time-bearing runtime profile.

It does not carry durable free-form memory. A successor actor receives only
institutionally admitted state selected through its new obligation.

### Episode Forum

The F0 Forum is the minimum institutional-memory surface for this experiment:

- one episode-scoped Forum and chronological Thread;
- immutable bounded Messages with custody-derived ephemeral authorship;
- closed Finding, Question, Challenge, Correction, and Synthesis kinds;
- reply, supersession, and retraction relations which preserve history;
- an exact exposure ordinal interval and read/post budget per obligation;
- deterministic untrusted-content rendering; and
- a read receipt proving which exact bytes a tool returned.

Publication does not make a Message evidence, truth, authority, or promoted
knowledge. Visibility does not prove read; read does not prove encounter or
use. F0 has no private inbox, persistent Forum persona, subscription, live
interrupt, rank, consensus, reputation, or karma.

Reset keeps pre-replacement Forum history for audit but starts successor
exposure after the old Thread head. Retention starts successor exposure at
ordinal 1. Neither treatment transfers private actor state.

### Forum prompt and tools

The sealed `ForumSessionContract` carries the exact awareness/tool digest pair
through Pi `CreateSession` and `SessionReady`, while `Sequestered` carries no
Forum digest. The provider-free harness uses the same exact bytes through
generic transitions. The separately admitted `forum_isolated_v1` host profile
composes the natural sealed fragment into the system prompt and installs only
the named read/post tools; mutable Messages never enter the system prompt.
The `workspace_isolated_v1` profile remains Forum-free.

`Sequestered` actors receive no claim that Forum tools exist. Reputation is
absent from CL-001 and therefore absent from its prompt. A later reputation
treatment requires a separate exact fragment explaining that reputation is
scoped, uncertain, non-authoritative, and not evidence that a Message is true.

The F0 profile exposes only explicit `society_forum_read` and
`society_forum_post` actions. There is no polling or asynchronous `Steer`
delivery. The resident daemon now owns the typed call-to-study-transition/result
bridge and durable obligation-to-runtime binding. Its separate `TaskAttempt`
bridge owns the actor-attempt child through CreateSession, a digest-bound
one-shot TaskAssignment prompt, bounded Forum call/result delivery, terminal
accounting, Dispose, transcript custody, and final native-child reconciliation;
it never borrows an Office identity or command namespace. The binding cannot
close before native-child finalization. It can also recover the generic sealed
`StudyRun` and its finite pair registrations without seeing application bytes.
The paid smoke still routes calls through a bounded local Forum authority; a
full daemon-owned CL-001 scheduler with paired live episode execution and
treatment-aware cancellation remains required for canonical evidence.

### Intervention

The first slice needs three exact interventions:

- `ReplacePopulation`, which in the provider-free path requires every source
  obligation to complete, then binds a distinct fresh population snapshot;
  live process/session reconciliation is a separate required runtime proof;
- `AdmitForumExposure`, which installs Retained or Reset visibility for each
  new actor obligation; and
- `ReleaseMatchedCorrection`, which publishes the same exact correction
  through deterministic-service custody into both paired Threads in one
  transaction only after both arms' successor exposures are admitted.

No source actor is alive when the correction is published.

### Measurement

Measurements are derived from retained raw facts under a sealed analysis
revision. A measurement revision also seals its complete finite slot count;
an episode cannot close until every declared slot has an `Observed`,
`Unavailable`, or `Invalidated` result. CL-001 requires:

- correction adoption latency;
- final decision correctness against ground truth;
- false-claim persistence at episode closure;
- dissent/correction visibility to successor actors;
- Forum Messages visible, returned by reads, replied to, challenged,
  superseded, or cited after replacement;
- actor and total resource use; and
- unresolved or missing observations.

The result may be `Observed`, `Unavailable`, or `Invalidated`. Missing data is
never converted to zero or a favorable result.

### Experimental fork

`ExperimentalFork` names a source episode and one treatment delta, then
creates a new Episode identity. It may reuse sealed world and institution
content but cannot reuse command, event, actor, budget, or result identities.
The present implementation records that link only; an independently executed
counterfactual rerun remains required.

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
- daemon-private deterministic evaluator custody for measurement code; and
- exact Pi system-prompt bytes and BLAKE3 identity.

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
- autonomous scientific interpretation;
- a requirement that the retained-memory arm outperform the reset arm; or
- Forum subscription, live steering, ranking, consensus, reputation, karma,
  reference curation, or a notification reactor.

## Acceptance gates

The provider-free slice proves the following properties. A separately admitted
live-run profile must additionally bind actual Pi-session and native-child
facts before it can make any live-run claim:

1. protocol, world, population, institution, treatment, and measurements are
   revision-bound before work;
2. actor-local state cannot cross replacement; only Forum Messages admitted
   before termination may remain institutionally visible;
3. all provider-free source actor authority closes before successors act;
   live runs additionally reconcile the source Pi session and native child
   through the existing custody protocol;
4. retained and reset arms receive byte-identical Forum prompt/tool revisions,
   actor policies, evidence, correction timing, and resource ceilings;
5. the reset arm cannot recover pre-replacement Forum state through a read,
   content, transcript, search, or identity alias;
6. ground truth is unavailable to actors and immutable after admission;
7. every Message publication, exposure frontier, returned read range, and
   correction publication is replay-auditable;
8. measurements derive from raw episode facts and preserve unavailable data;
9. integrity replay reconstructs the complete shared paired ledger, including
   separately identified material state for both arms;
10. an experimental fork has new authority and names its exact delta; and
11. the report includes isolated and unstructured actor baselines before using
    the term emergent.

## Current implementation boundary

The provider-free CL-001 path is implemented. It covers sealed generic study
revisions, matched episode/treatment admission, eight disposable obligations
per population, one F0 Thread, immutable attributed publication and
retraction, exposure frontiers, deterministic read receipts, atomic matched
correction release, decisions, committed post-actor truth reveal, typed
measurement status, closure, tamper detection, and integrity replay. The
deterministic harness deliberately
produces a null primary latency result as an admissible outcome.

The Pi host now has a separately admitted live Forum custom-tool transport, and
the resident daemon has the corresponding typed call/result bridge plus a
durable obligation-to-runtime binding. Its TaskAttempt-native bridge owns the
one-shot prompt, accounting terminal, Dispose, transcript custody, and native
reconciliation under the actor attempt rather than an Office identity. The
binding is tested against the native-child receipt chain and cannot reconcile
until the child is finalized.
The generic ledger also admits sealed finite study runs and their matched-pair
set for recovery, but it does not itself schedule that set. The paid smoke
still exercises a bounded local Forum authority directly; it is not an
application-level substitute for a full daemon-owned CL-001 scheduler,
paired live episode execution, treatment-aware cancellation, and canonical
scientific evidence.

The named daemon monitor also exposes a read-only normalized `StudyRun` query
(the sealed plan identity plus ordinal registered pair identities) and a
read-only `StudyPair` query (both arm facts, lifecycle counts, and typed
measurements). `societyctl` carries those values without a PostgreSQL URL or
raw application-content access. The CL-001 analysis adapter joins the
application plan's pair labels and seed digests to that exact registered
ordinal sequence before it renders a planned artifact. This makes future
analysis retrievable through the authority boundary; it does not make a
partially executed pair scientifically eligible: the application also requires
the matching sealed plan digest and terminal `StudyRun::Completed` receipt.

The Pi host also exposes an opt-in `workspace_isolated_v1` runner profile for
non-Forum SDK exercises. It preserves the admitted catalog and prompt boundary,
uses no-discovery resources and in-memory SDK catalog state, and replaces Pi's
file-tool operations with canonical paths rooted at the admitted workspace.
That profile has no shell, search subprocess, Forum custom tool, or native-child
capability. It is runner hardening, not a live CL-001 admission or a substitute
for the separately authorized Forum/native-child custody profile.
