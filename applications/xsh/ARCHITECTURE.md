# XSH Society: application architecture

## Status and purpose

This document is the XSH application charter. It preserves the Factory V1
evidence and the recursive self-improvement research agenda in
[`../../RSI.md`](../../RSI.md) as the application-specific design source. The
repository-root architecture defines the generic society apparatus which this
application is intended to use.

[`GLOSSARY.md`](GLOSSARY.md) is canonical for domain terms and spellings. This document owns
their composition and behavior. In particular, “mission” and “north-star
alignment” are two parts of one `UniverseSeed`, not independent authorities.

“Factory V2” is retained only as lineage. The thing being designed is an **XSH
society**: a persistent population of replaceable actors, durable institutions,
shared epistemic memory, executable culture, scarce-resource governance, and
technical artifacts, all dedicated to furthering XSH as a next-century
systems-glue language.

This is not a claim that current models can autonomously govern a language. It
is a design for making that claim experimentally testable without allowing the
system to manufacture its own evidence.

## Decision

Factory V1 is preserved as a completed experiment and evidence corpus. Its
reproducible workspaces, exact assignments, content hashes, evaluator ownership,
budgets, lifecycle checks, rollback, and product provenance are valuable
laboratory physics. Its governing abstraction is not carried forward.

V1 organized work as:

```text
evaluation -> ticket -> approval -> implementation -> replay -> merge
```

That works when a good observation and an obvious intervention already exist.
It does not reliably decide what is worth knowing, form competing explanations,
preserve qualitative conflict, observe long-horizon consequences, or improve
the organization which does those things. A ticket buffer cannot create an
epistemic supply.

V2 therefore has a different primary artifact and a different unit of progress:

> The primary artifact is a typed, revisioned epistemic graph. The unit of
> progress is a causal episode that changes warranted belief, XSH, or the
> society's demonstrated capacity to improve XSH.

Commits remain indispensable outcomes. They are neither a substitute for
inquiry nor a cycle quota.

## Origin purpose

### Universe Seed: mission made executable

The society begins from one first-class `UniverseSeed`, not from an ambient
README convention and not from a mission paragraph copied opportunistically
into prompts. The seed is the origin construct from which the first society,
Grand Architect office occupancy, portfolio, projects, and actor attempts are
derived. One society has exactly one active seed revision.

Its philosophical source is XSH's `docs/CHAPTER-01-why-xsh.md`. XSH is a
clean-slate systems scripting language for modern Linux userspace: the strong
glue between processes, files, paths, byte streams, structured data, and system
state. It aims to replace archaeological stacks of shell, Make, m4, Perl,
Python, text filters, and private DSLs without reproducing their quoting,
ambient-state, implicit-evaluation, or text-only boundary failures. It
preserves Unix's coarse-grained composability, ordinary files, visible process
boundaries, pipeline flow, and the ability for a script to grow into a tool. It
is not a POSIX compatibility shell, an interactive terminal, or a claim to be
the best general application runtime.

The initial mission is therefore:

> Make XSH a practical, coherent, easy-to-learn, token-efficient, and
> trustworthy systems-glue language for humans and coding agents, capable of
> replacing fragile Unix glue with typed paths, explicit process and effect
> boundaries, structured streams and errors, reproducible execution, and
> inspectable policy while preserving the composability that makes Unix
> systems useful.

The operational north star is the mission's alignment test. Every admitted
Project, Ticket, consequential Decision, ReviewChallenge, Retrospective, and
Postmortem answers:

1. What XSH capability or actor behavior would change?
2. What evidence distinguishes a general improvement from a local workaround,
   movement of complexity, or noise?
3. How does the change honor clarity, explicit boundaries, composability, and
   XSH's systems-glue scope?
4. At which review, replay, outcome horizon, or Grand Architect decision will
   the claim be revisited?

The mission supplies purpose; north-star alignment supplies a repeatable way
to interrogate proposed action. They are stored in the same revision and may
only diverge through a rejected or incomplete constitutional amendment.

`society-xsh-contract` owns the exact revision-1 rendering and constructs the
generic `ApplicationMissionInput` and `ProjectNorthStarAlignment` values. It is
an application input factory, not an authority: it cannot install the mission,
resolve the kernel-issued `ApplicationRevisionId`, seal content, or create a
Project. It returns only `ApplicationMissionInput`, the canonical bounded
`MissionSourceRendering` from `UNIVERSE-SEED.v1.md`, and their matching BLAKE3
digest; it imports neither `societyd` nor `societyctl` and knows no
`ContentObjectId`. The resident's private founding-source path checks those
bytes against the declared digest, preflights the outer command without
mutation, physically seals them, records the receipt/object chain, and only
then lets `InstallFoundingMission` bind the registered object. The supervisor
carries `MissionSourceRendering` only beside that mission command; it has no
generic content mutation or content-writer authority. Deterministic internal
operation identities make the content primitive retry-stable while that daemon
authority is retained; they do not resume a failed supervisor handler. The
request ends on handler failure, and restart is `RecoveryFenced`, not
source-operation recovery. This records byte custody, not producer provenance,
semantic/evidence admission, or VS-001 execution.
The versioned source artifact is `UNIVERSE-SEED.v1.md`; a future prompt builder
may materialize those exact bytes under the session-local name
`UNIVERSE-SEED.md`, but that alias cannot identify a different rendering.
The application office `TheGrandArchitect` is realized through the generic
`RootAuthorityOffice`; the application title and reserved-power prose never
become a generic enum, table, protocol tag, or daemon branch.

### `UniverseSeed` contract

The durable contract is relational and strongly typed. Collections below are
normalized child tables with closed enum kinds, not a JSON document:

```text
UniverseSeed {
    founding_mission_id
    society_id
    revision
    status: Proposed | Active | Superseded | Rejected
    mission_statement
    xsh_domain_scope
    grand_architect_office_contract_revision
    amendment_origin
    activation_basis_kind: FoundingBootstrap | GrandArchitectRatification
}

UniverseSeedFoundingActivation {
    seed_revision
    bootstrap_command_id
    consumed_bootstrap_capability_id
}

UniverseSeedRatifiedActivation {
    seed_revision
    ratification_decision_id
    grand_architect_occupancy_revision
}

UniverseSeedPrinciple {
    seed_revision
    ordinal
    kind: Preserve | Reject | Value | NonGoal | Beneficiary
    statement
}

UniverseSeedAlignmentQuestion {
    seed_revision
    ordinal
    question
}

UniverseSeedSource {
    seed_revision
    source_kind: XshChapter | FactoryNorthStar | ConstitutionalDecision
    content_object_id
    source_revision_or_commit
    selected_scope
}

NorthStarAlignment {
    north_star_alignment_id
    universe_seed_revision
    capability_or_behavior_change
    general_improvement_discriminator
    clarity_boundary_composability_treatment
    revisit_horizon_or_decision
}
```

The seed owns the canonical questions; each `NorthStarAlignment` stores one
work object's four answers against the exact seed revision. Project, Ticket,
Decision, ReviewChallenge, Retrospective, and Postmortem tables reference an
alignment identity through named foreign keys rather than a polymorphic target
or copied mission prose. Mission is stable purpose; alignment is its
work-specific application.

The implemented generic bootstrap commands are:

```text
CreateSocietyIdentity
InstallRootAuthorityOffice
InstallFoundingMission  # one-time bootstrap capability
BootstrapSociety        # consumes one active founding-mission revision

```

The application contract reserves this still-unimplemented descendant
amendment path; these are not current generic command variants:

```text
ProposeUniverseSeed     # descendant amendment path
RatifyFoundingMission   # TheGrandArchitect application decision
```

`BootstrapSociety` fails unless the seed, office contract, active occupant, R0
execution policy, and global budget ceiling are exact revisions. The resulting
`SocietyBootstrap` records all five. Bootstrap is idempotent for those inputs
and cannot silently pick “latest.”

The founding seed is the input which starts the apparatus, so requiring an
already-running Grand Architect actor to ratify it would create a false causal
loop. `InstallFoundingMission` is therefore a consumed root capability: it
can activate exactly revision 1 exactly once and cannot amend it. This is not a
standing human-ratification layer. After bootstrap, every descendant seed uses
the ordinary proposal, challenge, C4 evidence, and Grand Architect
ratification path. The two activation variants have distinct body tables so a
missing ratification decision can never masquerade as an ordinary descendant
activation.

The canonical `UNIVERSE-SEED.md` rendering is the first prompt segment supplied
to every actor attempt, including narrow task actors and adversarial reviewers.
It is a projection of typed state, not durable state itself. The attempt records
the exact seed revision and prompt-renderer revision; mission context is never
removed as a token-saving optimization. Scoped assignment, disclosure frontier,
and culture follow it in that order.

Changing an XSH source chapter does not mutate the active seed by filesystem
side effect. It creates a detectable source divergence and a C4 amendment
candidate. The Grand Architect may ratify a descendant seed after the amendment
process described below. The system preserves the old revision, challenges,
predicted effects, dissent, and reason for continuity or change.

### Values

Consequential decisions must account for, without pretending to reduce to one
number:

- semantic coherence and correctness;
- explicit authority, effects, and system boundaries;
- compatibility and migration cost;
- implementation and language simplicity;
- runtime performance and resource behavior;
- human learnability and agent fluency;
- observability, reproducibility, and repairability;
- ecosystem usefulness and actual systems-workload coverage; and
- future capacity to understand, test, and safely improve XSH.

The values are a decision vocabulary, not fixed weights. An authorized decision
may trade among them, but must expose the trade, the available evidence, the
dissent, and the predicted consequences.

### Definition of improvement

The society has improved only when evidence supports at least one of these
claims under an explicit scope:

1. XSH or its ecosystem became better on one or more Universe Seed values without an
   undisclosed or disqualifying regression.
2. The society learned something that materially changes a future decision or
   eliminates a meaningful region of uncertainty.
3. An institutional mutation improved decision, experiment, propagation, or
   delivery performance on held-out or later work under a comparable resource
   envelope.

Actor activity, generated prose, graph size, token spend, and commit count are
not themselves improvement.

## Why society is the correct abstraction

An organism suggests one objective, one body, and tightly coupled control. The
desired system instead depends on semi-autonomous actors, persistent dissent,
multiple legitimate kinds of expertise, institutions that outlive workers,
shared culture, resource politics, constitutional authority, and artifacts
which change the society that created them.

The recursively improving entity is therefore not an agent and not an org
chart. It is this complex:

```text
                     Universe Seed
                           |
                    shared XSH world
                           |
          +----------------+----------------+
          |                |                |
        actors        institutions       artifacts
          |                |                |
          +--------- culture and memory ---+
                           |
                 evidence and governance
                           |
             resource allocation and change
                           |
             new actors, tools, and norms
                           +-----------------> shared XSH world
```

Individual actors are replaceable. Continuity lives in the Universe Seed,
verified history, knowledge, offices, institutions, and executable artifacts.
Better foundation models should enter as better citizens or office occupants,
not cause institutional amnesia.

### How far the society metaphor goes

“Society” is an engineering abstraction, not a mandate to simulate a
civilization. V2 adopts mechanisms where plural cognition, durable authority,
institutional memory, checked communication, and scarce shared resources solve
observed coordination problems better than one agent loop.

The initial admission set is deliberately Pareto-oriented:

| Keep | Engineering function |
| --- | --- |
| Constitution and a highest office | Durable purpose, coherent authority, succession, and accountable exceptions |
| Institutions and circuits | Reusable boundaries, state transitions, specialization, and organizational experiments |
| Projects, tickets, milestones, and coordination pulses | Practical planning, ownership, WIP control, and synchronization |
| Dissent and adversarial review | Error discovery without requiring consensus or erasing minority arguments |
| Professions and developmental attractors | Learned specialization when repeated demand demonstrates value |
| Local demand signals and bounded influence | Decentralized sensing without unbounded broadcast or a universal planner score |
| Curated culture and checked propagation | Accumulation, scope control, uptake measurement, and retraction |
| Portfolio and resource treasury | Explicit scarcity, reserves, cost ceilings, and opportunity tradeoffs |
| Retrospectives and postmortems | Routine learning plus structured response to failures and breaches |

V2 explicitly does **not** model internal money, transferable credit, trade,
auctions, synthetic property, consumer preferences, elections, political
parties, prestige, emotions, friendship, demographic reproduction, autonomous
population growth, or persistent fictional social lives. It does not pay
agents to converse for realism, manufacture social conflict, or construct a
market where a typed scheduler and budget reservation solve the actual
problem. Contributions are not currency and office occupancy is not status.

A new society-inspired mechanism is admitted only as a C3 hypothesis after:

1. an observed coordination or epistemic bottleneck is named;
2. the cheapest deterministic or ordinary software-engineering baseline is
   implemented or credibly specified;
3. the proposed mechanism identifies a discriminating benefit, resource cost,
   failure modes, and removal condition;
4. a replay, shadow, or canary comparison can observe the claimed benefit; and
5. the Grand Architect authorizes the trial within a bounded envelope.

This rule lets the system become more socially sophisticated where evidence
demands it while keeping the initial apparatus comprehensible and affordable.

## Architectural thesis

The system joins mechanisms which are usually separated:

- scientific inquiry chooses what to learn;
- governance makes contextual decisions among incompatible values;
- evolutionary search supplies variation and organizational alternatives;
- software engineering supplies deterministic tests and executable outcomes;
- distributed-systems machinery supplies identity, scarcity, concurrency,
  backpressure, and recovery; and
- cumulative culture turns local discoveries into future default behavior.

The full recursion is:

```text
observe XSH and the society
        |
form competing explanations
        |
choose high-information interventions
        |
execute reproducible experiments
        |
evaluate, disagree, and decide
        |
ship an XSH change, retain knowledge, or both
        |
observe delayed consequences
        |
propagate warranted lessons
        |
mutate tools, language, actors, or institutions
        +---------------------------------------> repeat
```

The loop is recursive only when a demonstrated institutional lesson can alter
the machinery which produces later lessons. Re-running a goal prompt, editing a
worker prompt, or selecting the patch that passes the most tests is not by
itself recursive self-improvement.

## The dynamics of recursion

The society is not defined only by its objects and institutions. It is defined
by three coupled rates:

```text
discovery
  decision-relevant uncertainty becomes warranted knowledge

propagation
  warranted knowledge changes relevant behavior in its valid scope

metamorphosis
  accumulated knowledge changes the machinery that produces later knowledge
```

These are vectors over problem classes and evidence strengths, not three
universal counters. A deterministic soundness discovery and a provisional
ergonomics lesson propagate at legitimately different speeds. A prompt edit and
a replicated governance improvement are also not equivalent metamorphoses.

Commit throughput is a fourth, downstream concern: delivery. A society can
discover rapidly but propagate nothing, propagate doctrine without testing its
effect, or deliver many patches while never improving how it learns. None of
those is recursive institutional improvement.

The rates form a constrained system:

```text
discovery outruns curation/evaluation
  -> unresolved evidence congestion
  -> narrower admission and more validation capacity

propagation outruns warrant
  -> contamination and correlated error
  -> slower promotion and wider contradiction search

propagation lags warrant
  -> repeated rediscovery and inconsistent active work
  -> more retrieval, transduction, and uptake capacity

institutional evidence accumulates but metamorphosis stalls
  -> organizational knowledge without organizational consequence
  -> reserve trials for R2 mutations and held-out replication
```

The observatory exposes these imbalances; resource policy responds to them.
The trusted substrate enforces the real scarcity involved but never invents a
single combined RSI score. The governing question is which constrained rate is
preventing warranted discoveries from changing XSH or the society, and what
experiment could discriminate among explanations for that bottleneck.

## Constitutional topology

The architecture has four layers with deliberately different mutation rules.

```text
  R3  XSH world
      source, compiler, docs, workloads, tools, product changes
                           ^
                           |
  R2  mutable society
      actor prompts and tasks, professions, circuits, institutions, projects,
      typed policies, evaluators, context, propagation, organization configurations
                           ^
                           |
  R1  amendable constitution
      Universe Seed, TheGrandArchitect office, change classes,
      evidence standards, promotion rules, succession
                           ^
                           |
  R0  trusted physics
      resident Rust authority, identity, ledger, transactions, capabilities,
      Operating Cycles, Pi/process supervision, cancellation, observability,
      resource accounting, content, Git lineage, replay, rollback
```

### R0: trusted physics

R0 says what happened and what a principal was allowed to do. The society
cannot rewrite it through ordinary commands. Kernel changes are software
changes outside the authority of the running kernel instance and enter through
an explicit C5 deployment boundary. This is not a human-ratified layer: it is
the unavoidable distinction between the state machine and replacement of the
machine enforcing that state.

### R1: amendable constitution

R1 says what the society is for and how highest authority may change. It is
versioned and explicit. The Grand Architect may ratify a C4 amendment after the
kernel verifies the required decision packet, independent challenge, dissent,
cooling interval or documented urgency exception, predictions, rollback or
succession treatment, and exact descendant revision. Challenge is mandatory;
assent is not. There is no separate human veto or actor-species check.

### R2: mutable society

R2 is the experimental phenotype. Actor configurations, routing, professions,
prompts, assignments, Projects, communication topology, evaluators, trace
curation, resource policy, and decision circuits are candidates for bounded
mutation. This is where evolutionary fluidity belongs: in the creative
organization using trusted mechanisms, not in the mechanisms which decide
whether a process is owned or a dollar was spent.

### R3: XSH world

R3 is the mission target and an optional R2 user-space medium for experiments,
evaluators, tooling, and policy prototypes. It is not the seed language of R0
and does not sit on an authority, persistence, supervision, cancellation, or
evidence-critical path. Product mutations remain independently reviewable and
revertible. Self-reference never grants additional authority.

This topology resolves an otherwise dangerous ambiguity. “Immutable” does not
mean every initial policy is frozen into Rust, and “autonomous” does not mean a
prompt can bypass transactions, budgets, or history. The society may redefine
its active mission and constitutional scoreboard through an explicit Grand
Architect amendment; it cannot pretend that the descendant seed governed its
ancestors or erase the evidence used to amend it.

### XSH evaluator port

The XSH evaluator port is the narrow application adapter between a canonical
VS-001 evaluator contract and generic native-child custody. It owns the named
judge program, XSH/Xsht invocation, closed fixture and case manifests, expected
output shape, and the semantic parser that interprets a report. The port
constructs bounded canonical program and input renderings, their declared
BLAKE3 identities, an opaque application evaluator-profile identity, and a
closed application-owned invocation description. It neither assigns a sealed
identity nor sends XSH command names or result semantics across the generic
boundary. A future daemon-private bridge may check the declared identities
against an already durable generic admission, use the direct-executable custody
core, and seal fully reaped output before a separate evidence command refers to
it. The present daemon has no evaluator scheduler or application execution
entry point, so the port does not yet submit a generic execution request.

The port does not create a content object, assign an evaluator or input
revision, select a durable child identity, run a process, write SQLite, or
admit evidence. A parsed `BehaviorObservationSetV1`, documentation matrix,
fluency report, curation report, uptake report, or frontier report remains an
XSH application claim until a separate generic admission command binds the
privately sealed occurrence and authorized semantic role. This preserves a
useful split: the generic layer can prove which evaluator/input/output
occurrence was custodied, while XSH alone can say what its cases and outputs
mean.

The application now supplies `vs001-direct-evaluator-adapter`, a separately
compiled direct executable whose first candidate is deliberately narrow:
`CurationContract` only. The adapter reads the verified input-manifest file,
parses seven closed length-framed TSV members in their fixed order, evaluates
the existing C1 curation contract in Rust, and writes its canonical two-member
output package to stdout. The package retains both the aggregate observation
and the separately typed raw-evidence escalation relation, including a named
question/object request. It starts no shell or child, accepts no external path from the
manifest, and depends on no XSH/Xsht binary, source checkout, `PATH`, or host
tool. The direct profile consequently has one bounded input object rather than
a false multi-file package provenance claim.

`Vs001CurationDirectEvaluatorPackageV1` names exactly those seven application
member roles. `Vs001DirectEvaluatorInputManifestV1` is their canonical bounded
outer rendering; it has no durable identity and does not itself seal, admit,
or authorize anything. The Rust adapter is cross-checked against the existing
shell judge's checked-in positive output and negative relations. The other
VS-001 judges, their scripts/fixtures, and their external XSH/Xsht/source-tree
requirements remain application-owned but are explicitly pending a separate
sealed-materialization design.

No application evaluator is registered or scheduled. A later generic bridge
may seal an actual adapter build and one exact input manifest, materialize only
that verified manifest path in its private workspace, and invoke the fixed
`--input-manifest` ABI. This application construction carries no path,
environment, child ID, receipt, content object, evaluator revision, authority,
or evidence claim.

The corresponding C1 stdout interpretation stays equally narrow and wholly in
the application. `interpret_curation_direct_stdout_v1` receives only a byte
slice and its caller-declared BLAKE3 value. It rejects empty, oversized,
changed, and noncanonical renderings, then returns the closed
`CurationDirectSemanticResultV1` (`Accepted` with the complete typed curation
outputs, or one fixed XSH rejection reason). Equality with the declaration
is a local byte-substitution check, not a claim about execution, custody,
sealing, provenance, reaping, or evidence admission. The generic side need not
know the TSV schema, semantic values, or rejection vocabulary. This pure parser
is an application consumer/check of the self-validating direct evaluator's
canonical package, not a second process. It does not make the current direct
candidate scheduled or executable through generic custody.

## Three forms of durable truth

The implementation must not collapse three different records into one.

| Record | Meaning | Mutation rule |
| --- | --- | --- |
| Event ledger | Commands accepted, transitions made, resources spent, principals responsible | Append-only; replay-auditable |
| Content-object store | Immutable forensic bytes captured in a named environment | Sealed; new evidence may contradict but not edit old content |
| Epistemic graph | What is claimed, believed, disputed, decided, and applicable now | Revisioned; status can change with provenance |

The ledger is not the world model. An artifact is not an interpretation. A
current belief is not historical fact. Keeping those distinctions hard is what
makes later retrospection and retraction honest.

The phrase “causal graph” is shorthand for a graph of **causal claims and
decision provenance**. An edge which says an outcome supports an attribution
does not make causality mechanically true.

### Evidence is not automatically memory

The evidence-to-culture path has four deliberately different depths:

| Depth | Purpose | Default audience | Admission rule |
| --- | --- | --- | --- |
| Operational audit | Establish what command, process, tool, or transaction occurred | Kernel, supervisor, forensic inspection | Produced by trusted machinery |
| Forensic evidence | Preserve source material which may later support or defeat a claim | Judges and explicit investigators | Sealed with capture method and origin |
| Curated episode account | Preserve the few distinctions needed to understand a consequential choice | Deliberation and later replay | Selected with causal role, scope, omissions, and challenge path |
| Cultural inheritance | Change how later citizens perceive or act | Matching future work only | Checked validation, promotion, delivery, and uptake evidence |

Moving downward in that table is not copying. It is an accountable semantic
transformation. Most audit events should never become graph nodes. Most sealed
artifacts should never become institutional memory. Most episode conclusions
should never become default culture.

Conversely, the society must not discard inconvenient source evidence merely
because it was not selected into the current account. A curated claim remains
challengeable through its cited evidence boundary and stated exclusions. This
separation permits a small epistemic commons without granting curators the
ability to rewrite forensic reality.

## The trusted substrate

### Resident Rust authority

The kernel is a continuously running Rust service named `societyd`, not a
library independently invoked by XSH programs and not a collection of scripts
coordinating through files. It exclusively owns one organization-wide SQLite
database, one digest-addressed content-object store, the live child-process
registry, and the local command/monitor interfaces. SQLite is never opened by a
client with write authority.

The trusted core is intentionally larger than a fashionable microkernel. XSH is
not yet a suitable seed language for authority, process supervision, or durable
evidence physics. The criterion is not minimum Rust line count; it is whether a
bug or mutation could forge authority, lose cost, orphan a child, rewrite
history, or make an invalid transition appear valid.

`societyd` is authoritative for:

- object, revision, configuration, principal, actor, session, and episode
  identity;
- typed graph edges and the allowed endpoint kinds for each edge;
- commands, optimistic generations, idempotency, transactions, and ordered
  immutable events;
- capabilities, Office occupancy, jurisdiction, leases, claims, expiry,
  Operating Cycles, admission generations, cancellation, and exact resource
  ownership;
- budgets for model work, compute, wall time, worker slots, integration risk, and
  organizational experiments;
- actor prompt rendering; TypeScript Pi SDK-host and deterministic process
  spawning; process-group ownership; closed SDK-adapter protocol parsing;
  session, event, usage, and cost normalization; graceful termination and
  reaping;
- workspace and Git-worktree creation, input materialization, post-run
  inspection, content sealing, evaluator execution, controlled product
  materialization, and cleanup eligibility;
- sealed input snapshots, environment records, raw sessions, evaluator
  revisions, observations, Git commits, patches, and outcome artifacts;
- state-machine validation, retries and attempt lineage, rollback references,
  and retention status;
- scheduler-visible readiness, work claims, cycle admission, and trusted
  circuit breakers;
- structured tracing, `OperationalNotice` derivation, live monitoring, and
  Grand Architect Pi SDK Office-session supervision; and
- projection cursors and a transactional outbox for rebuildable views and
  notifications.

Node-specific data that affects readiness, authority, propagation, or
evaluation is relational and constrained. An opaque JSON field or EAV table
must not become an escape hatch from the protocol.

The kernel owns mechanisms and effects; the society owns creative content and
strategy. Prompts, assignments, hypotheses, Project purposes, actor
configurations, organization topology, circuit selection, curation policy,
signal-family parameters, and proposed mutations remain versioned R2 data. The
kernel validates and executes their closed contracts but does not hard-code the
research answer. Moving a mechanism into Rust does not move its policy choice
out of the evolutionary loop.

### Command boundary

All durable mutation crosses one versioned command protocol served by
`societyd`:

```text
Command {
    command_id
    principal_id
    capability
    expected_generation
    body: CommandBody
}

Command
  -> authenticate principal and capability
  -> validate state, references, budget, and jurisdiction
  -> atomically append event and update materialized state
  -> write outbox records
  -> return typed receipt or explicit conflict
```

No XSH program, actor, projection, Office helper, or host-admin helper writes
arbitrary SQL, edits workflow state files, supervises Pi independently, or
infers completion by scanning a directory.
Content-sensitive idempotency prevents one command identifier from being reused
with different semantics.

`CommandBody` is a closed Rust enum whose variants contain named structs with
domain newtypes. The named local Unix socket is a query-only monitor surface;
the trusted process supervisor submits the closed mutation language over a
separate pre-opened Unix stream. `societyctl` exposes the public queries and a
typed library peer for that supervisor-held stream; neither serializes the enum
into a generic command document. Both versioned wire frames are
length-prefixed and tag-discriminated, with explicit encodings for domain
newtypes and no JSON. The exact codecs remain inside the Rust protocol crate so
clients cannot invent fields. SQLite
uses normalized tables, foreign keys, checked enum representations, unique
constraints, and explicit nullable-state rules. V2 permits no JSON columns,
generic `payload` or `metadata` columns, EAV tables, state files, workflow
manifests, or machine JSON projections.

Command and event headers use discriminants plus named one-to-one variant body
tables. Rust exhaustively decodes the matching body and ledger replay treats a
missing, duplicate, or mismatched body as corruption. Strong typing therefore
survives persistence; it is not lost at the first database write.

The sole JSON exception is the unavoidable Pi 0.83.0 boundary: the closed
Rust-to-`society-pi-host` control stream, SDK event/result stream, the v3
session transcript, external `auth.json`, and an actor's bounded
`submission.json`. The Rust Pi supervisor
treats all of them as untrusted boundary bytes, seals the evidentiary streams,
parses them into closed Rust types, validates settlement and submission
separately, and only then invokes the in-process typed command handler. No JSON object
becomes durable society state merely by parsing successfully. XSH itself may
of course manipulate JSON as part of its systems-glue mission; that product
capability does not relax V2's persistence contract.

### Identity and lineage

The following identities are distinct:

- a `Principal` is an authenticated source of authority;
- an `ActorConfiguration` is a versioned heritable configuration;
- an `ActorInstance` is one admitted realization of that configuration;
- an `ActorAttempt` is one bounded task execution;
- a `RootAuthorityOfficeSession` is one supervised Pi SDK-host process for an
  agent occupant during an Operating Cycle;
- a `PiSession` is Pi's canonical conversation/session identity and may back an
  Actor Attempt or Grand Architect Office session;
- an `OperatingCycle` is one resource, configuration, monitoring, and
  cancellation epoch inside the continuous daemon;
- an `OrganizationConfiguration` identifies the complete R2 setup relevant to
  an episode;
- an `Episode` groups causally related inquiry and action; and
- a `Revision` is immutable content for one graph object.

“The reviewer learned” must resolve to a new cultural artifact, actor
configuration revision, or institution policy. It cannot mean that valuable
state lives only inside an unaddressed chat session.

Every descendant configuration records parents, mutation operators, changed
fields, birth evidence, and responsible authority. Recombination records all
parents. A friendly name is a projection; the content identity is canonical.

### Artifact and execution contract

The content-object store identifies sealed bytes by digest. File length,
storage path, ingest timestamp, and similar bookkeeping may be derived or
retained inside the storage implementation for safety and repair, but they are
not curated provenance and do not enter the epistemic graph merely because they
are cheap to collect.

An evidence admission reference instead records only facts with interpretive or
reconstruction value: object digest, closed semantic role, producing attempt or
command, capture/evaluator method, applicable scope, schema or media contract,
and retention/access class. Its graph edge states *why* the evidence matters to
a claim, experiment, decision, or challenge. Sealing an object never admits it
as knowledge.

Execution evidence pins the variables needed to reconstruct the declared
world:

```text
repository commits and dirty-state declaration
toolchain and dependency lockfiles
host execution profile, OS, hardware class, and optional environment boundary
input and workload corpus revisions
evaluator, policy, prompt, model, and tool revisions
organization configuration and capability envelope
budget, seeds, start/stop conditions, and observed resource use
```

Replay means reconstructing the recorded world closely enough to evaluate the
declared claim. It does not promise bit-for-bit model generation. Deterministic
components must be exact; stochastic components must preserve model parameters,
seeds where meaningful, sample count, and uncertainty.

### The Pi SDK is the actor runtime

V2 installs and pins `@earendil-works/pi-coding-agent` 0.83.0 and embeds its
TypeScript SDK through a small V2-owned executable named `society-pi-host`.
Actors do not invoke the `pi` CLI, `--print`, or Pi RPC mode. The exact Node
runtime, compiled host digest, package version, lockfile integrity, Pi package
graph, and adapter-protocol version are execution-profile inputs. Pi 0.83.0
requires Node 22.19.0 or newer; the admitted profile pins one exact qualified
Node build rather than accepting that range at runtime.

Rust cannot load a TypeScript SDK in-process without introducing a much larger
JavaScript-runtime trust boundary. `societyd::PiSupervisor` therefore starts
one `society-pi-host` OS process per Pi `AgentSession`. A one-shot task gets one
host for one Attempt; an actor occupying The Grand Architect gets one long-lived
host for its Operating Cycle. The first implementation deliberately does not
pool unrelated sessions in one TypeScript process. Per-session processes keep
workspace, tools, abort, crash, usage, evidence, and TERM/KILL containment
aligned with one kernel identity. Pooling may later be tested as a C3
efficiency mutation, not smuggled into the founding runtime.

The TypeScript host is a pinned trusted **execution adapter**, but never an
institutional authority. A defect can misrender a prompt, expose a tool, or
misreport an SDK event, so its source, dependency lock, event conversion, and
qualification belong to the R0 evidence boundary. It cannot open SQLite, issue
daemon commands, allocate budget, change capabilities, admit evidence, dispose
a review, materialize Git, or authorize delivery. Rust owns those effects and
independently reconciles session files, process state, submissions, and costs.

`society-pi-host` constructs each SDK session explicitly:

```text
ModelRuntime       exact auth/models paths and exact admitted model
SettingsManager    in-memory, versioned retry/compaction/queue settings
ResourceLoader     V2-owned implementation: exact system prompt, no discovery,
                   no extensions, skills, prompt templates, themes, or context files
SessionManager     owned persistent session file in the session workspace
createAgentSession exact cwd, agentDir, model, thinking level, tool allowlist,
                   runtime, loader, settings, and session manager
```

No SDK default which can discover ambient resources or choose a model is
accepted silently. The host validates the effective provider
`openrouter`, model `deepseek/deepseek-v4-flash-0731`, thinking `high`, tools,
settings, cwd, system-prompt digest, and session identity before reporting the
session ready. Model-cycling, session replacement, ambient resource loading,
and extension-defined tools are absent from the adapter protocol.

The JSON exception remains exactly at the Pi boundary. Rust and the TypeScript
host exchange a closed, versioned JSONL protocol over kernel-owned pipes. Rust
sends only `CreateSession`, `Prompt`, `FollowUp`, `Steer`, `Abort`,
`GetState`, and `Dispose`; the host emits `AdapterReady`, `SessionReady`,
`CommandResult`, an exhaustive JSON-safe projection of each
`AgentSessionEvent`, `UsageSnapshot`, `Settled`, `Disposed`, or `Fatal`. Every
frame has a session identity, monotonically checked sequence, and correlation
id where applicable. Unknown event variants, missing terminal events, sequence
gaps, invalid JSON, or incompatible adapter versions fail the session rather
than falling through to narrative parsing.

The host calls `session.prompt()`, `session.followUp()`, `session.steer()`,
`session.abort()`, `session.subscribe()`, and `session.dispose()` directly.
Task and Grand Architect actors therefore use the same SDK surface; “one-shot”
versus “Office” is a V2 lifecycle policy, not two Pi CLI modes. The canonical Pi
session file remains forensic transcript and usage evidence. At Office-session
`Dispose`, Rust verifies the flush receipt and materialized file under owned
filesystem custody, then seals its exact bytes as content; that seal is not a
semantic transcript parse, submission, or evaluator result.

The Grand Architect Office session begins with the exact Universe Seed and
cycle brief. `OperationalNotice` batches normally enter through `followUp()`
only after the current turn settles. `steer()` is reserved for genuinely
decision-changing urgency because interrupting active reasoning with routine
progress damages coherence. `abort()` is the cooperative first step of graceful
cancellation. Each Office turn has a named reason, disclosure frontier, cost
reservation, and closed decision-submission contract.

The kernel does not feed every trace line into the Office session. That would
turn observability into context flooding and continuous token spend. A bounded
monitor layer coalesces accepted INFO/WARN/ERROR notices by scope and reason,
retains the strongest unresolved item, and triggers an Office turn only for a
decision horizon, blocker/escalation, budget danger, cancellation, delivery,
cycle rollover, or an explicit request. A user occupant sees the equivalent
live console and typed monitor query without a paid Pi session.

Process ownership begins before the SDK may create an `AgentSession` or make a
model request:

```text
reserve Attempt/OfficeSession and commit exact pre-spawn admission in SQLite
  -> spawn society-pi-host inert in a new owned process group
  -> immediately register direct PID and process-group ownership
  -> host reports AdapterReady with pid, nonce and Node/adapter/Pi identities
  -> Rust cross-checks pid and identities, then records AdapterReady
  -> kernel rechecks generation, cancellation, owner, profile and budget and
     commits final CreateSession authorization
  -> resident driver writes the exact CreateSession frame and, only after a
     complete pipe write, records the correlated delivery attestation
  -> Rust validates SessionReady from the still-running child
  -> a distinct turn authorization is required before Prompt
```

The M5 kernel stores the authorized/delivered Create correlation and digest but
does not itself observe pipe I/O. The daemon-private provider-free Office
bridge now binds that delivery command to `PiSupervisor`'s complete physical
write before admitting `SessionReady`. This is a same-resident-lifetime bridge,
not an independent OS attestation or a resident scheduler: no Prompt, usage,
cost, semantic settlement, or restart recovery follows from the row.

The M6 kernel adds the durable authority on the other side of that missing
Prompt boundary. One deterministic Office turn binds exact prompt content,
correlation, the current ledger event head, live session, admission generation,
and the existing Office-session budget reservation. KERNEL-service delivery
and accepted-result attestations, session-cumulative usage or an accounting
failure, and terminal disposition are separate ordered facts under one
session-wide sequence watermark. Only a
`Completed`/`ObservedStop` terminal can atomically checkpoint its new cumulative
cost delta and restore Office `Ready`; no per-turn reservation double-books the
Society or Cycle envelope. The resident driver has no scheduler/control-loop
caller for Prompt projection. The generic Office-session Dispose foundation is
now defined independently: `Authorize-before-write -> delivered -> accepted ->
final Known/failure -> Disposed`. It authorizes before the physical write,
records only a complete delivery, then requires final Known usage immediately
after acceptance before the next transcript-flush `Disposed` receipt. That
Known terminal reconciles the one parent reservation and releases its unused
reserve; a known overrun or final accounting failure freezes the reservation,
and the failure branch has no synthetic `Disposed` receipt. A materialized
transcript is verified under daemon-owned filesystem custody and content-sealed
before the terminal receipt; a no-Prompt session may still be materialized and
sealed with an absent first prompt. Only the lazy missing-file arm has no
content object, and neither absence arm may invent a first prompt or content.
Child reap is separate process custody, not a Dispose consequence.

This does not add post-restart recovery, workspace disposal, semantic
submission, paid/native qualification, or an XSH end-to-end claim.

Pipe EOF normally makes a still-inert host exit without constructing a session,
but a daemon restart does not infer that physical outcome. A durable admission
without a spawn receipt remains an explicit unresolved obligation; a registered
group enters recovery containment without inventing parentage or `wait(2)`.
If cancellation wins the generation race, Rust closes or kills the host and
records a cancelled pre-session Attempt. During an active session, cooperative
cancellation calls SDK `abort()`; deadline escalation sends TERM and then KILL
to the host process group. This removes the unowned paid-execution window
without pretending process launch and SQLite commit are one OS transaction.

A working directory is an ownership and reproducibility boundary, not a
security sandbox. A Pi actor with the `bash`, `read`, `edit`, or `write` tools
can potentially reach other host paths allowed by the host OS account.
The first implementation relies on explicit assignments, narrow Pi tool
profiles where practical, omission of control credentials from actor
environments, daemon-side submission on behalf of the supervised principal,
process supervision, before/after workspace and Git checks, and forensic tool
events. The capability system prevents unauthenticated institutional commands;
it must not claim that application capabilities confine a hostile same-user
native process at the OS filesystem or network layer.

The Docker cleanroom used by V1 remains a valuable optional treatment for
measuring self-sufficiency under a sparse environment. It is not the V2 worker
runtime and organization mutations do not build new container images. Future
OS sandboxing, a separate user, or a container runner may implement a stronger
`ExecutionProfile`, but native-host results remain separately identified rather
than treated as equivalent evidence.

Environmental self-sufficiency is multidimensional. Repository visibility,
context policy, installed tools, network access, documentation, model, host
packages, and filesystem enforcement are recorded as separate treatment
variables. “Cleanroom” is never one unexplained Boolean.

## Kernel observability

Observability covers every trusted mechanism without promoting every
measurement into provenance. `societyd` uses the explicitly approved `tracing`
and `tracing-subscriber` crates. No additional logging dependency is implied.
The initial subscriber has a curated target filter compiled from trusted
configuration, a compact INFO-and-higher console layer, and a bounded monitor
layer which constructs `OperationalNotice` values. Ambient `RUST_LOG` may make
local output more verbose for diagnosis but cannot suppress the fixed Grand
Architect INFO/WARN/ERROR surface or required safety notices.

### Span and correlation contract

Long-lived spans mirror real ownership rather than source-module nesting:

```text
society
  operating_cycle
    project / ticket / causal_episode
      actor_attempt | root_authority_office_session | deterministic_run
        pi_process / evaluator_process / product_materialization
          pi_turn / tool_execution / command
```

Applicable events carry typed display fields such as:

```text
society_id operating_cycle_id universe_seed_revision
project_id ticket_id episode_id
actor_instance_id actor_attempt_id office_session_id office_turn_id
pi_session_id process_id command_id budget_id
event_id trace_correlation_id
```

Identifiers are safe display encodings, not arbitrary `Debug` output. Prompts,
model messages, credentials, environment values, source contents, and full tool
arguments never appear at INFO. Raw boundary content remains in sealed Pi and
forensic objects under its access policy.

### Curated level policy

| Level | Intended content | Examples |
| --- | --- | --- |
| TRACE | High-volume protocol mechanics, disabled by default | Pi event kind, SDK-adapter frame direction/id, stream chunk metadata, SQLite statement class |
| DEBUG | Reconstructable mechanism decisions | readiness factors, signal derivation inputs, context membership, lease poll, process receipt details |
| INFO | Meaningful operational progress | cycle/project/ticket transition, Attempt start/settle, Office turn, decision due/made, budget reservation/summary, review, delivery, cancellation phase |
| WARN | Degradation or attention needed | retry, blocked work, malformed boundary item, nonzero tool aggregate, unknown cost, stale lease, missed deadline, TERM-to-KILL escalation |
| ERROR | Trusted mechanism or containment failure | database/content corruption, invalid ledger body, unowned/orphan child, evidence loss, failed reaping, invariant breach |

An ordinary task failure is INFO or WARN according to its operational
consequence; it is not an ERROR merely because the candidate was wrong. ERROR
means trusted machinery or containment failed. Conversely, repeated retries are
not hidden at DEBUG merely because the kernel recovered.

### One fact, three surfaces

For an accepted transition, the ordering is:

```text
validate command
  -> commit materialized state + durable event + outbox atomically
  -> construct typed OperationalNotice from the committed event
  -> emit tracing event with the same identities
  -> stream/render to eligible monitors
```

The ledger is durable truth. `OperationalNotice` is a bounded typed monitor
projection. Tracing text is an ephemeral rendering. Neither notice nor text is
parsed back to manufacture a command, graph fact, readiness condition, or
epistemic claim. A failure before commit may be traced and noticed as
`TransitionRejected` or `MechanismFailure`, but it cannot claim the transition
occurred.

```text
OperationalNotice {
    notice_id
    source_event_id_or_failure_correlation
    severity: Info | Warn | Error
    kind: ClosedOperationalNoticeKind
    scope
    summary_template
    typed_fields
    first_observed_at
    last_observed_at
    occurrence_count
    resolution_state
}
```

The summary is rendered from a closed kind and typed fields, not supplied as
arbitrary authority-bearing prose. Duplicate heartbeats and equivalent progress
events coalesce. WARN/ERROR notices cannot be displaced by INFO volume. Each
monitor has queue and byte limits; overflow coalesces or drops eligible INFO
with an explicit loss counter, never blocks the state machine or silently drops
safety notices.

### Grand Architect monitoring

A user occupant runs `societyctl monitor --level info` and may narrow by cycle,
Project, Ticket, or Attempt. An actor occupant receives bounded notice batches
through its supervised Pi SDK Office session. Both see the same notice
identities and can expand a notice through typed queries. The actor path does
not receive console escape sequences or depend on parsing a human log format.

Default Office-turn triggers are:

```text
decision or review disposition ready
Project/cycle blocker or deadline escalation
50% and 80% budget-reserve warning; any unknown cost
cancellation requested, escalated, or incompletely reconciled
product delivery or rollback ready
Operating Cycle quiesced, close-ready, failed, or rollover-ready
explicit Grand Architect monitor/query request
```

Routine INFO is batched until settlement or a short bounded interval. An idle
Office Pi process consumes no model tokens; sending progress text does. Monitor
policy therefore records notice count, input tokens, Office-turn cost,
decisions produced, and notices later expanded, so excessive monitoring becomes
an observable coordination cost rather than a free aesthetic preference.

### Product boundary

The XSH checkout and the society checkout are separate authority domains. A
product experiment runs in a dedicated worktree at a pinned commit. A product
change can reach XSH only through an authorized decision, exact patch or commit
lineage, required XSH tests, independent review, and a revert reference.

The institution that proposes or implements a product mutation cannot gain
merge authority from success prose in its own output.

## The epistemic commons

The society's shared world model is a persistent typed graph. It is a commons
because no actor owns reality: actors contribute scoped claims and evidence
under authority, while the graph preserves incompatible interpretations.

### Core vocabulary

| Node kind | Durable meaning |
| --- | --- |
| UniverseSeedValue | An active-seed value used to explain tradeoffs, never a hidden weight |
| Objective | A scoped desired capability or condition derived from the Universe Seed |
| Question | A consequential uncertainty, with resolution and abandonment conditions |
| Hypothesis | A falsifiable explanation with scope, confidence form, and alternatives |
| Prediction | A pre-outcome expectation with measure, horizon, and failure condition |
| Proposal | A possible intervention with alternatives, cost, reversibility, and blast radius |
| Experiment | A preregistered attempt with evaluator, inputs, budget, and stop rule |
| Observation | A sealed result or externally sourced fact, separate from interpretation |
| Argument | A scoped interpretation linking claims and evidence |
| Conflict | Preserved incompatible claims, values, evidence interpretations, or authorities |
| Decision | An authorized choice with rationale, dissent, predictions, and revisit triggers |
| Implementation | A bounded product or society mutation and its exact lineage |
| Outcome | A short- or long-horizon consequence observed after a decision |
| Retrospective | A revisioned assessment of prediction, process, and possible attribution |
| Lesson | A validated, scoped knowledge claim with propagation and revocation policy |
| Invariant | An externally or constitutionally enforced condition and its executable judge |
| OrganizationTrial | A controlled comparison of R2 configurations |
| ActorConfiguration | A versioned policy for constructing bounded cognitive work |
| OrganizationConfiguration | The exact R2 configuration active for specified work |

This is the kernel vocabulary, not a promise that no node kinds will ever be
added. Adding a kind which affects durable meaning is a schema and
constitutional compatibility change, not an actor inventing a JSON `type`.

### Semantic separations

The graph enforces several separations that prose workflows routinely blur:

- an `Observation` says what was seen; an `Argument` says what it might mean;
- a `Hypothesis` explains; a `Proposal` intervenes;
- a `Prediction` is written before the result; an `Outcome` is recorded after;
- a `Decision` records legitimate authority under uncertainty; it is not proof
  that the choice was correct;
- a `Retrospective` may propose an attribution; it may not rewrite the original
  rationale; and
- a `Lesson` is reusable only within its declared applicability scope.

### Typed relations

Initial relations include:

```text
derives_from    motivates       decomposes
competes_with   assumes         predicts
tests           uses_evaluator  observes
supports        contradicts     qualifies
argues_for      argues_against  preserves_dissent
decides         authorizes      implements
produces        affects         revisits
attributes_to   learns          propagates_to
depends_on      invalidates     supersedes
configured_by   performed_by    descended_from
```

Each relation declares legal endpoint kinds, authorship authority, and whether
it pins a source revision. Derived transitive views such as “decisions affected
by a retracted lesson” are projections, not unrecorded edges asserted as fact.

### Identity, revisions, and status

A graph object has a stable identity and append-only content revisions. An edge
identifies the source and target revisions whose content justified it. A later
revision does not silently alter an old decision packet.

Statuses such as `supported`, `contested`, `refuted`, `superseded`, `decided`,
and `reopened` are either explicit governed transitions or rebuildable
derivations from recorded facts. They are never overwritten labels with no
event history.

Confidence is typed by evidence domain. Deterministic reproduction, a sampled
rate with uncertainty, expert judgment, and speculative design intuition are
not interchangeable values on one `0..1` scale. Where numeric calibration is
meaningful, the method and reference class travel with the number.

### A causal episode

An `Episode` is a bounded subgraph with an accountable beginning, resource
envelope, organization configuration, and follow-up obligations. The normal
shape is:

```text
objective
  -> question
  -> competing hypotheses and explicit unknowns
  -> predictions
  -> proposals and experiment plan
  -> admitted execution attempts
  -> observations and arguments
  -> conflict or discriminating follow-up
  -> decision
  -> implementation, no-change decision, or further inquiry
  -> short-horizon outcome
  -> delayed outcome
  -> retrospective
  -> lesson, retraction, or preserved uncertainty
```

An episode may finish without an implementation. It may not finish merely
because an actor stopped. Abandonment requires a recorded reason such as low
value, infeasibility, superseding evidence, exhausted budget, or an explicit
Grand Architect decision.

### Episode state contract

The kernel owns legal operational transitions while the graph owns their
meaning:

```text
framed
  -> admitted | deferred | rejected
admitted
  -> investigating
investigating
  -> deliberating | abandoned
deliberating
  -> decided | investigating | closed_preserved_conflict
decided
  -> implementing | observing | closed_no_change | investigating
implementing
  -> validating | observing_failed | reverted
validating
  -> observing | implementing | reverted | investigating
observing
  -> learning
learning
  -> closed
closed | closed_no_change | closed_preserved_conflict
  -> reopened
reopened
  -> investigating
```

Each transition has required nodes, evidence, authority, and outstanding
obligations. A `closed` episode may still be reopened by a scheduled outcome or
contradictory evidence. Operational state never deletes epistemic conflict.

Blocking is an orthogonal operational condition, not a lifecycle bucket. A
blocked episode records the lifecycle state it remains in, blocker, owner,
expiry or escalation, and budget consequences. Removing the blocker resumes
that same state. This prevents a generic `blocked -> active` transition from
silently inventing which epistemic gate was satisfied.

An episode may contain several decision revisions—for example authorization to
prototype, then authorization to deliver. The generic `decided` state means one
declared action is authorized; a circuit may refine it into multiple explicit
gates. Later decisions descend from rather than overwrite earlier decisions.

## Curated provenance

Raw traces and scalar rewards are opposite information failures. Raw traces
preserve incidental detail until relevant structure becomes difficult to find;
scalar rewards preserve a verdict after destroying the argument which could
later correct it. V2 therefore treats curation as a primary cognitive and
institutional act.

The compression path is:

```text
sealed raw session and execution artifacts
        |
normalized attempt receipt and observations
        |
arguments, conflicts, and decision packet
        |
episode retrospective and scoped lessons
        |
provisional guidance, policy, evaluator, or invariant
```

Each arrow is a new warranted representation with a narrower purpose. It is not
an automatic summary job.

### Curation contract

A `CuratedAccount` is a revisioned semantic object for a named use: preparing a
decision, reconstructing an episode, training a profession, assembling future
context, or testing an organization. It contains only consequential structure:

```text
CuratedAccount {
    purpose_and_audience
    question_and_episode_scope
    epistemic_disclosure_frontier

    selected_items[] {
        source_revision_or_evidence
        causal_or_argumentative_role
        selection_reason
        applicability_scope
    }

    preserved_conflicts_and_minorities[]
    decision_relevant_unknowns[]
    exclusions[] {
        excluded_category_or_source
        reason
        risk_if_wrong
    }

    transformations[] {
        input_references
        method_or_policy_revision
        output_claim
        information_known_lost
    }

    curator_configuration_and_authority
    challenges_and_superseding_accounts[]
}
```

These fields record semantic choices, not every available measurement. Byte
counts, filenames, token timestamps, streaming deltas, and tool chatter do not
become provenance unless a particular claim makes them consequential. A tool
timeout may matter; the number of bytes in an ordinary Markdown artifact almost
never does.

Every account names what it intentionally excludes. Negative space is part of
the contract because later investigators need to distinguish “considered and
excluded as irrelevant,” “unavailable at the time,” and “silently overlooked.”
The account also preserves dissent in the strongest available form rather than
compressing it into an averaged confidence.

Exclusions are semantic categories and particular high-risk sources, not the
set-theoretic complement of every captured file or event. The forensic manifest
already answers what bytes existed. Repeating that inventory inside the account
would recreate provenance bloat without explaining what the curator considered
consequential.

An account never replaces its sources. It gives later citizens a small default
path and an explicit escalation path into admitted evidence or raw forensics.
Accessing raw material for a decision is itself visible so repeated transcript
archaeology can be diagnosed as curation failure.

### Curation lifecycle

```text
proposed
  -> challenged | accepted_for_scope
challenged
  -> revised_descendant | accepted_with_dissent | rejected
accepted_for_scope
  -> superseded | retracted | expired
```

Acceptance is purpose-specific. An account sufficient for a product decision
may be unsuitable as a training case because it assumes expert context. A
lesson derived from an accepted account still requires independent promotion;
curation does not smuggle claims directly into culture.

The curator may be an actor, a mixed deterministic/model circuit, or a human.
The producer of a proposal may not be the sole curator of the evidence used to
approve it for a consequential change. Curator identity means the exact
configuration and policy revision, not a title such as “historian.”

### Curation quality

Curation is evaluated against later use, not against compression ratio alone.
Historical and live probes ask:

- **decision sufficiency:** could an authorized reader reconstruct the real
  alternatives, constraints, evidence, and disagreement without raw-session
  archaeology?
- **relevance:** how much supplied material never affected a legitimate query
  or decision?
- **reversal sensitivity:** were facts later responsible for reversal preserved
  or explicitly marked unavailable?
- **dissent fidelity:** could the strongest minority argument be recovered
  without relying on the majority's paraphrase?
- **scope calibration:** did the account invite application outside the
  evidence domain?
- **causal honesty:** did it distinguish observation, attribution, choice, and
  outcome?
- **contamination:** did hindsight, descendant lessons, or unrelated episodes
  leak across the declared disclosure frontier?
- **escalation cost:** when raw evidence was genuinely necessary, could it be
  located through the account's references?

No single score certifies a curator. Different policies may occupy different
Pareto regions: terse accounts for routine deterministic changes, richer
accounts for semantic decisions with delayed consequences, and deliberately
plural accounts where disagreement remains productive.

### Curation is part of the mutable society

Trace vocabulary, selection rules, compression prompts, independence policy,
retention, and context assembly are R2 variables. A proposed curation mutation
is a C3 institutional change. It must be compared on blinded historical or
held-out decision worlds and cannot certify itself by changing the questions
used to judge missing information.

The trusted substrate protects source identity, chronology, disclosure
boundaries, and lineage. It does not freeze one theory of relevance into the
database. This is the deepest recursive seam: the society can improve the
representations through which it understands its own improvement while being
unable to rewrite the evidence from which those representations were made.

## Provenance as active observability and bounded influence

Provenance is not opaque storage awaiting an audit. It is the undercurrent from
which context, review, scheduling, resource allocation, reopening, and
institutional learning are continuously derived. The important boundary is not
“active versus archived.” It is **which semantic transformation is allowed to
produce which organizational effect**.

The active path is:

```text
ledger events + sealed content + typed observations
        |
        | deterministic normalization; no organizational effect
        v
ProvenanceFact
        |
        | evidence admission and named curation policy
        v
eligible source set
        |
        | versioned SignalFamily derivation
        v
DerivedSignal
        |
        | scope, warrant, freshness, independence, and jurisdiction gates
        v
InfluenceCandidate
        |
        | bounded comparison, attention allocation, and authority check
        v
InfluenceDecision
        |
        +--> visible in a coordination pulse
        +--> retrieved into matching context
        +--> bids for inquiry, review, or portfolio attention
        +--> requires a named review or outcome check
        +--> reprioritizes within a comparable queue
        +--> blocks a transition when an R1/R0 rule explicitly permits it
```

Each arrow produces a new durable type with source lineage and a policy
revision. Raw telemetry cannot directly raise priority, enter an actor prompt,
damage an actor configuration's standing, or block delivery. A curated account
does not automatically do those things either; it supplies eligible semantic
inputs to a named signal family.

### Provenance facts

A `ProvenanceFact` is the smallest normalized statement the trusted machinery
can make about origin or transformation:

```text
ProvenanceFact {
    provenance_fact_id
    source_event_id
    subject: ClosedProvenanceSubject
    relation: ProducedBy | Consumed | DerivedFrom | SelectedInto |
              ExcludedFrom | ChallengedBy | AuthorizedBy | SpentUnder |
              ObservedBy | SupersededBy | RetractedBy
    object: ClosedProvenanceObject
    policy_or_evaluator_revision
    jurisdiction
}
```

Facts are relational edges over existing typed identities. There is no
free-form property bag and no duplicated byte length, path, timestamp, token
delta, or message chatter merely because it is cheap to record. Storage and
audit queries can recover physical facts from their owning tables. A semantic
fact exists only because a named reconstruction, invariant, derivation, or
decision query requires it.

Operational audit facts may feed deterministic health alarms—such as a budget
watcher detecting unknown cost—without first pretending to be epistemic
evidence. They may not support a product or institutional conclusion until an
evaluator admits their semantic role and scope.

### Signal families are local mathematical contracts

A `SignalFamilyRevision` defines one reference class in which mathematical
comparison is meaningful:

```text
SignalFamilyRevision {
    signal_family_revision_id
    name
    eligible_source_kinds
    required_warrant
    independence_rule
    scope_rule
    uncertainty_semantics
    pressure_formula_revision
    decay_rule
    hysteresis_rule
    attention_quota
    allowed_effects
    replay_evaluator_revision
    authority_class
}
```

Examples include `unresolved_contract_conflict`,
`repeated_agent_discovery_failure`, `prediction_due`,
`cost_reserve_at_risk`, `curation_escalation_hotspot`,
`lesson_contradicted`, `integration_ready`, and
`institutional_trial_regression`. These families have different evidence and
uncertainty semantics. Their raw numeric values are not commensurable.

For a family whose factors have calibrated meanings, a versioned pressure
function may be:

```text
pressure(s, now) =
    severity(s)
  * affected_exposure(s)
  * warrant_lower_bound(s)
  * time_pressure(s, now)
  * independence_factor(s)
  -------------------------------------------------
    estimated_response_cost(s)
  * (1 + already_committed_capacity(s))
```

This is illustrative structure, not one formula frozen for every family.
`severity`, `exposure`, and `warrant_lower_bound` must be defined and tested for
that reference class. `time_pressure` may rise toward a prediction deadline but
decay for a stale speculative opportunity. `independence_factor` prevents five
cloned reports from appearing to be five confirmations. Denominators have
typed floors so a zero estimate cannot create infinite priority.

The result answers only: “within this signal family and policy revision, which
eligible candidate exerts more pressure on this bounded consumer?” It does not
answer whether a semantic conflict is more valuable than a security incident
or a delayed outcome. Across families, constitutional precedence, explicit
portfolio envelopes, Pareto dominance, due obligations, and reserved capacity
govern. A deterministic tie-breaker applies only after that partial order.

### Eligibility before arithmetic

No signal bubbles by multiplying cheap measurements. Before pressure is
computed, the kernel or a deterministic projector verifies:

1. every source revision exists and is visible to the target jurisdiction;
2. the source kind satisfies the family's evidence-admission and curation rule;
3. required challenges, independence, or replication exist;
4. applicability scope overlaps the proposed target;
5. the source and its carriers are not retracted, expired, contaminated, or
   superseded for this use;
6. uncertainty is represented in the form required by the family;
7. the derivation policy passed its replay tests; and
8. the target has remaining family-specific attention capacity.

An ineligible signal receives a typed reason and remains queryable. It does not
get coerced to zero, because “not currently eligible,” “measured no effect,”
and “lost a comparison” are different facts.

### Influence is a scarce, recorded effect

`DerivedSignal`, `InfluenceCandidate`, `InfluenceDecision`, and
`InfluenceEffect` preserve the complete chain:

```text
DerivedSignal {
    signal_id
    signal_family_revision_id
    source_set_revision
    target_scope
    typed_measurements
    uncertainty
    valid_from
    expires_or_recompute_at
}

InfluenceCandidate {
    influence_candidate_id
    signal_id
    target: Project | Ticket | Review | ContextSlot | OutcomeObligation |
            PortfolioPartition | GrandArchitectBrief
    requested_effect: Visible | RetrieveOnMatch | AttentionBid |
                      ReviewRequired | Reprioritize | AdmissionBlock
    comparison_class
    pressure_inputs
    pressure_formula_revision
}

InfluenceDecision {
    influence_decision_id
    candidate_id
    disposition: Applied | Deferred | Rejected | Expired | Displaced
    authority_or_policy_revision
    reason
    displaced_candidate_id
    attention_consumed
}

InfluenceEffect {
    influence_decision_id
    target_revision_before
    target_revision_after
    observed_at
    effect_status: Pending | Observed | Failed | Reverted
}
```

Most signals should become `Visible`, `RetrieveOnMatch`, or an `AttentionBid`.
`ReviewRequired` is reserved for a named governance contract.
`AdmissionBlock` is rare and legal only when an exact R0 or R1 rule names the
signal family, target transition, clearance condition, and authority. A model
cannot invent a blocking signal by describing an issue as “critical.”

Every consumer has an influence budget: context slots, open challenge count,
project WIP, portfolio capacity, or Grand Architect brief length. Each signal
family has a quota or reserve so a high-volume family cannot monopolize the
surface. Diversity reserves protect minority hypotheses and under-observed
niches. Hysteresis prevents repeated rank oscillation near a threshold. Decay
and expiry remove stale pressure without deleting history. Reopening and
retraction follow dependency edges rather than broadcast to everyone.

### Active consumers

The first consumers of curated provenance are concrete:

- the Grand Architect brief shows constitutional conflicts, decisions due,
  cost reserve, delivery blockers, dissent, and expired obligations with a
  bounded top set plus queryable remainder;
- portfolio admission and project chartering receive family-local attention
  bids, never an unexplained universal priority;
- ticket readiness derives from graph motivation, prerequisites, budget,
  capability, and judge availability;
- context assembly retrieves the active Universe Seed unconditionally and
  lessons, accounts, conflicts, and source excerpts only on scoped matches;
- adversarial-review assignment reacts to risk, ancestry, unresolved dissent,
  and evidence weakness;
- coordination pulses expose newly eligible, displaced, decayed, contradicted,
  or budget-relevant signals;
- outcome obligations and failed predictions reopen named dependents;
- postmortems receive the ledger-derived event and cost timeline plus relevant
  prior challenges; and
- institutional science compares whether influence policies improved later
  decisions under matched cost rather than celebrating signal volume.

Every applied influence answers ordinary typed queries: Which source
distinctions produced it? Which curation admitted them? Which family and
formula revision compared them? Which other candidates were displaced? Which
authority or policy applied the effect? What later outcome supported or
defeated the intervention? If a result can only be explained from a model's
summary prose, it is not an acceptable influence mechanism.

### Influence policy can improve, but cannot self-certify

Signal admission rules, formulas, decay, quotas, context retrieval, and
cross-family portfolio policy are R2 institutional machinery. Changing one is
C3. A proposed descendant is replayed against hindsight-sequestered decision
worlds, then shadowed, then canaried under a fixed resource envelope. It is
judged on decision sufficiency, missed reversals, contamination, attention
cost, calibration, diversity, and downstream outcomes.

The candidate policy cannot select its own evaluation cases, redefine its own
success measure, conceal displaced signals, or promote itself. The Grand
Architect authorizes promotion after the required challenge record; no human
ratification is implied. The ancestral policy and non-dominated alternatives
remain available for rollback and niche-specific use.

## Governance and legitimacy

Evaluation is a process because interesting XSH decisions have no universal
`quality(candidate) -> float`. Governance turns evidence and values into an
authorized, revisitable choice without pretending the choice was mathematically
forced.

### Authority is capability plus jurisdiction

Capabilities are narrow verbs over specific object kinds and scopes: propose,
run, observe, challenge, decide, allocate, propagate, amend, integrate, revert.
Jurisdiction limits where and under what risk class a capability applies.

An actor may be excellent at semantic review and still have no authority to
merge, allocate its own budget, promote its own lesson, or change its evaluator.
An informal role name in a prompt implies no authority. Occupancy of a typed
Office does imply its exact capability bundle and jurisdiction. The database
records the office contract, occupant, delegation, succession, and expiry.

Native actors receive no reusable daemon credential. They write through their
closed Pi submission boundary; `societyd` validates the submission against the
owned process, Attempt, disclosure frontier, and expected contract, then invokes
the typed handler on behalf of that exact principal and capability. An actor
Grand Architect uses the same pattern for Office decision submissions over its
supervised SDK-host session. This prevents accidental authority from being smuggled
through `bash` or a CLI invocation.

This is institutional permission enforcement, not a false OS-isolation claim.
A hostile process running under the same host account may be able to inspect
other host files despite lacking a valid command capability. Strong
same-host-adversary containment would require a distinct OS execution profile;
the evidence records whether such a boundary exists.

### The Grand Architect

`TheGrandArchitect` is the highest constitutional Office and the sole final
decision authority inside the running society. Its display name is **The Grand
Architect**. The kernel does not ask whether its occupant is a human or a coding
agent; it authenticates the occupant principal and current office grant.

The office exists to concentrate coherent direction without making every
subsystem a plebiscite. Its reserved powers are:

```text
ratify and amend the active UniverseSeed
charter, pause, resume, terminate, and rank Projects
allocate and reallocate resource envelopes within R0 hard ceilings
appoint and remove occupants of subordinate Offices
approve circuit and organization configurations
authorize, reject, or accept risk for C2, C3, and C4 changes
require an AdversarialReview, Postmortem, replay, or outcome observation
resolve cross-institution conflicts and documented policy exceptions
reopen any non-forensically-destroyed work or constitutional question
designate a successor under the active succession contract
```

The office does not gain raw SQL, content-store mutation, secret access outside
an execution profile, or the ability to forge evidence, alter old events,
create unreserved spend, or force an invalid state transition. Those are not
competing centers of political power; they are R0 physics. The Grand Architect
may authorize a C5 replacement proposal, but the running kernel cannot deploy
its own replacement through ordinary state commands.

Concentration of decision authority is paired with concentration of
accountability. Every reserved-power command cites an exact Decision and
Universe Seed revision. C2–C4 decisions require a complete packet and the
specified independent challenge. The Grand Architect may reject the challenge
or accept the disclosed risk, but must answer it in a typed disposition. It
cannot make dissent disappear, retrospectively change the decision frontier,
or certify evidence by decree.

The initial occupant is installed during society creation by the bootstrap
principal. Later occupancy changes use `TransferRootAuthorityOffice`, which is
atomic: there is never more than one active occupant, the predecessor and
successor are explicit, in-flight capabilities are reconciled, and a sealed
succession packet records the active seed, budget, projects, due decisions,
open challenges, and emergency state. The office may be occupied directly by
the user for hands-on operation or by one assigned coding agent for autonomous
operation without changing any downstream contract.

The Grand Architect reads curated decision surfaces by default: the active
seed, portfolio, coordination pulse, eligible influence, strongest dissent,
and decision packets. Raw-session access is an explicit forensic escalation,
recorded because repeated escalation indicates failed curation. If a coding
agent occupies the office, its attempts use the same pinned model and cost
policy as every other paid actor unless a later C3 policy change is authorized.

### Decision packet

Every consequential decision preserves:

```text
question and decision authority
eligible alternatives, including no change
applicable Universe Seed values and hard constraints
evidence and its limitations
arguments for and against each alternative
unresolved unknowns and named dissent
resource, reversibility, and blast-radius assessment
chosen action and rationale
pre-registered predictions at named horizons
revisit, rollback, and escalation triggers
organization configuration and circuit used
```

Legitimacy means the authorized process was followed and honestly recorded.
Correctness is assessed later against outcomes. This permits the society to
learn from a reasonable decision that went badly without laundering a reckless
decision that got lucky.

### Change classes

| Class | Example | Minimum governance |
| --- | --- | --- |
| C0 observation | Run a read-only benchmark | Scoped capability, budget, sealed receipt |
| C1 reversible inquiry | Prototype in an owned directory or Git worktree | Preregistered experiment and cleanup |
| C2 product mutation | Deliver an XSH fix or feature | Independent evidence, product review, tests, revert path, Grand Architect authorization |
| C3 institutional mutation | Change routing, actor population, context, or evaluator policy | Organization trial, baseline, diversity guard, canary scope, rollback |
| C4 constitutional amendment | Change Universe Seed, reserved authority, succession, or promotion standard | Explicit amendment episode, independent challenge, cooling or urgency record, Grand Architect ratification |
| C5 trusted-kernel mutation | Change ledger, capability, accounting, or evidence physics | External implementation and adversarial review; the running kernel cannot deploy its replacement |

A change can only move inward through the authority rings by a stricter process.
The proposer, beneficiary, implementer, evaluator, and promoter need not always
be five actors, but independence requirements increase with the change class.
No mutation certifies its own evaluator or widens its own command authority.
The Grand Architect may grant wider authority in a separate decision after the
required evidence and challenge; the mutation itself cannot make that grant a
side effect.

### Holder-agnostic autonomy

Autonomy is determined by who currently occupies the Grand Architect office
and which bounded capabilities it delegates, not by a system-wide “human in the
loop” flag. A user occupant may personally exercise each reserved power. An
agent occupant may exercise the same powers through supervised, budgeted
attempts. Subordinate circuits remain autonomous only within their delegated
class, scope, resource envelope, and expiry.

Emergency stop and C5 deployment controls may remain available to the host
administrator because no process can remove the authority of its operating
environment. They are recorded as external interventions, never credited to
the society, and do not form a standing ratification tier. Evaluation of
autonomous performance reports those interventions, failed attempts, cost, and
changed external conditions explicitly.

## Actors, culture, and professions

### Actor configuration is a developmental policy, not a job title

An actor is a versioned policy for creating bounded cognitive work, not a
persistent chat persona. Its heritable configuration may include:

```text
ActorConfiguration {
    model_and_inference_policy
    cognitive_and_epistemic_biases
    exploration_exploitation_bias
    contradiction_and_risk_sensitivity
    tool_and_repository_capabilities
    memory_retrieval_and_context_policy
    communication_edges_and_bandwidth
    authority_and_budget_ceiling
    demand_signal_response_policy
    developmental_attractor_biases
    differentiation_and_dedifferentiation_policy
    persistence_and_retirement_policy
    branching_recombination_and_mutation_policy
}
```

The configuration identifies predisposition. The phenotype is observed
behavior on a problem distribution: what the actor notices, which work it
selects, how well calibrated it is, how it interacts, and what downstream
effects its contributions have. Biological language is explanatory shorthand,
not a schema design: configuration, lineage, branch, and retirement are the
canonical terms.

“Researcher,” “challenger,” and “integrator” are initial developmental
attractors. They are not hard-coded classes. Useful starting biases are:

```text
explore  build  measure  challenge  synthesize  integrate  remember  coordinate
```

These name basic functions rather than human management ranks.

### Development is an interaction, not configuration lookup

An attractor is a broad basin toward which behavior may stabilize under
particular conditions. It is neither a prompt template nor a row named
`reviewer`. An actor configuration expresses sensitivities and possible
developmental responses along several axes:

```text
AttractorBias {
    functional_axis
    initial_strength
    demand_signal_sensitivity
    activation_and_decay_policy
    compatible_and_antagonistic_axes
    context_and_tool_affinities
    evidence_needed_to_stabilize
}
```

The realized phenotype depends on the whole developmental context:

```text
actor predisposition
  × current demand gradients
  × admitted cultural material
  × tools and authority
  × collaborators and opponents
  × resource history and time horizon
  × reinforcement from downstream consequences
  -> observed phenotype
```

The same configuration may become a proof-oriented skeptic in a semantic
episode, a failure-minimizing test designer in a runtime episode, or remain
undifferentiated when no matching niche exists. Conversely, several different
lineages may converge on similar useful behavior. This is why neither actor
configuration nor model identity is a profession.

Early experiments may deliberately instantiate named treatments such as
“contract cartographer” or “product adversary.” Those names describe the
assignment and experimental condition. They do not establish durable species
in the ontology. Only repeated phenotype evidence across episodes can justify a
learned social compression.

Development itself is observable. The society records which signals an actor
responded to, which work it claimed or ignored, how its behavior changed across
bounded sessions, which practices it adopted from culture or peers, and which
downstream contributions made the apparent specialization useful. It does not
infer an inner trait merely from fluent self-description.

### Culture is separate heredity

Useful knowledge must not all become prompt genetics. The society has four
hereditary layers operating at different speeds:

| Layer | Inherited material | Typical adaptation speed |
| --- | --- | --- |
| Actor | cognitive policy, tools, sensitivities, authority ceiling | Fast, experimentally varied |
| Cultural | lessons, techniques, examples, concepts, training cases | Fast after checked validation |
| Institutional | circuits, jurisdictions, professions, resource and propagation rules | Slower, controlled trials |
| Technical | XSH, compiler, tests, tools, trusted infrastructure | Slowest as blast radius rises |

A new fuzzing sequence is probably culture. Persistent counterexample-seeking
may become an actor trait. Requiring independent migration analysis for a broad
semantic change is institutional heredity. A soundness rule with a regression
test may become technical heredity.

Context construction is the main interface between culture and an actor. A
context pack is a declared, content-addressed projection containing the
question, permissions, relevant graph revisions, applicable lessons, live
contradictions, and explicit omissions. Its retrieval policy and resulting
contents are recorded. Actors do not receive an untraceable vector-search soup.

### Actor lifecycle

```text
proposed configuration
  -> configuration qualification
  -> admitted instance with niche and capability lease
  -> bounded sessions and contribution history
  -> periodic phenotype and calibration assessment
  -> continued use | mutation | recombination | dormancy | retirement
```

Retirement ends authority and scheduling; it does not erase lineage or useful
work. Replication creates a new instance identity. Mutation creates a new
configuration revision. Acquired session state is inherited only when curated
into cultural or genomic form.

An actor cannot reproduce merely by claiming success. Population influence is
based on attributed downstream evidence, calibration, information efficiency,
and niche contribution under a diversity policy. These remain a vector rather
than one reproductive reward.

### Local signals and ecological niches

The graph produces decaying, scoped demand signals such as:

```text
unresolved semantic conflict
missing counterexample coverage
experiment capacity available
evidence review congestion
integration-ready decision
delayed outcome due
lesson applicability uncertain
repeated regression pattern
underrepresented actor behavior
```

Signals are rebuildable projections with source facts, scope, strength, and
expiry. They are machine “pheromones”: useful for local coordination but not
new epistemic truth.

Actor selection policies respond differently to those gradients. One actor may
prefer high-uncertainty questions, another stalled integrations, and another
contradictions to widely propagated lessons. The scheduler admits a claim only
when capability, budget, dependency, and WIP rules permit it.

This produces a hybrid nervous system:

- strategic governance allocates portfolios and imposes constraints;
- local signals let actors discover work without a central manager assigning
  every task; and
- explicit escalation handles conflicts or reallocations which local policy
  cannot resolve.

The kernel does not issue semantic work merely because a signal is strong.
Signals advertise conditions; actor policies express interest; the scheduler
checks capability, dependencies, WIP, diversity, and budget; governance may
reserve or redirect scarce capacity. This separates ecological attraction from
authorization.

### Ecological state and homeostasis

A demand signal is derived from named source facts and has:

```text
kind and problem scope
source revisions and projection policy
strength or ordering evidence
birth, decay, saturation, and expiry rules
capacity already responding
neglect and over-response indicators
```

Strength need not be a scalar shared across signal kinds. “Three unresolved
soundness contradictions” and “integration latency above its historical band”
can both attract work without pretending to be commensurable.

The society observes ecological failures:

- **starvation:** a persistent warranted need attracts no capable lineage;
- **swarming:** many correlated actors respond to the same visible signal while
  other needs are neglected;
- **herding:** shared culture or ancestry causes apparently independent actors
  to make the same selection;
- **predation:** a phenotype consumes scarce evaluation or integration capacity
  while externalizing cleanup;
- **signal gaming:** actors create or amplify demand facts which preferentially
  allocate resources back to them;
- **ossification:** an established profession monopolizes a niche despite
  cheaper or more accurate variants; and
- **ecological collapse:** diversity or downstream capacity falls below the
  level needed to test upstream production.

Homeostatic responses are governed policies: WIP reduction, capacity reserve,
novelty admission, signal dampening, independent challenge, actor dormancy, or
a deliberate profession-birth experiment. They are not evidence that the
underlying epistemic claim is true. The institution must be able to alter its
coordination field without altering the observations from which it was derived.

### Endogenous professions

A profession is a learned compression over useful phenotypes, methods,
jurisdiction, and interfaces. It is born through evidence:

```text
recurring unmet need
  -> actors attempt variants
  -> useful behaviors cluster
  -> shared method and training cases are curated
  -> a profession trial tests reliability and cost
  -> jurisdiction and interfaces are provisionally recognized
  -> expiry, fork, recombination, or retirement follows later evidence
```

Recognition may create a scheduler label, qualification suite, cultural packet,
and limited authority. The label never becomes proof of competence.

This permits professions with no human equivalent—for example a counterfactual
lineage auditor or epistemic contamination investigator—while avoiding a
permanent CEO/manager/engineer ontology.

A recognized profession is therefore a small social institution, not a model
alias. Its revision may include:

```text
recognized_niche_and_failure_history
phenotype_cluster_and_lineage_diversity
shared_method_and_training_cases
interfaces_with_other_professions
qualification_and_calibration_evidence
bounded_jurisdiction_and_authority
professional_norms_and_dissent_duties
resource_claim_and_expected_externalities
review, fork, dissolution, and succession rules
```

Formalization is useful when it compresses a recurring coordination pattern:
others know what evidence to supply, what output to expect, which authority is
legitimate, and how to challenge malpractice. Formalization is harmful when it
turns one early implementation into a protected caste.

Profession birth consequently needs more than repeated task success. Evidence
must show a persistent need, a behavior cluster which transfers across cases, a
predictable interface, benefit after coordination cost, and no simpler cultural
or tooling intervention that solves the need. Recognition begins with expiry
and narrow jurisdiction. Profession death is equally ordinary when the niche
disappears, its method becomes infrastructure, or another phenotype dominates
within the same constraints.

Knowledge can move between hereditary layers during this process. A useful
personal technique may become professional culture; a professional checklist
may become an evaluator; an evaluator may become an XSH invariant; after that,
the original profession may shrink because its former expertise is now part of
the environment. Society-level development includes this continual movement of
cognition into institutions and infrastructure.

### Diversity is infrastructure

Naive success selection creates monoculture and correlated blindness. The
population archive must preserve:

- model, prompt, tool, and ancestry diversity;
- behavioral and epistemic distance;
- niche and failure-mode coverage;
- contrarian configurations with calibrated minority value; and
- exploration capacity not justified by immediate task completion.

Diversity reserves are an explicit resource policy. An organization mutation
cannot eliminate the last representative of a protected behavior without a
recorded exception. Agreement among cloned or closely related actors is not
independent evidence.

## Institutions

An institution is a durable protocol combining jurisdiction, admission,
required artifacts, authority, state transitions, and exit conditions. It
outlives individual actors and is instantiated through actors and deterministic
services. Institutions are mutable R2 configurations, not Rust subclasses.

V2 preseeds the following functional institutions because the need for their
separation is already supported by V1 and ordinary scientific practice.

### Grand Architect office

Maintains one accountable apex for purpose, portfolio direction, allocation,
cross-institution resolution, C2–C4 authorization, and succession. The office
contract and occupancy are R1 state; its staff circuit, context policy, and
coordination surface are R2 and may improve through bounded trials. The office
does not own evidence or implementation merely because it owns the final
decision.

### Constitutional stewardship

Maintains Universe Seed revisions, values, reserved powers, risk classes,
succession terms, source divergence, and amendment history. It prepares
constitutional proposals and challenges; only the Grand Architect ratifies an
active descendant.

### Observatory

Continuously derives candidate questions from XSH failures, open proposals,
tests, benchmarks, docs, real systems workloads, V1 episodes, user reports,
dependency behavior, and the society's own bottlenecks. It separates anomaly
detection from explanation and may not turn every anomaly into work.

### Inquiry commons

Frames questions, elicits competing hypotheses, retrieves prior knowledge,
identifies unknowns, and proposes discriminating experiments. It is rewarded
for calibrated reduction of uncertainty, not volume of ideas.

### Laboratory

Registers and executes reproducible experiments under sealed manifests,
capabilities, budgets, owned workspaces, supervision, and stop rules. It records observations but
does not decide what they mean.

### Deliberation forum

Hosts arguments, adversarial challenge, tradeoff analysis, conflict
preservation, and decision packet construction. It ensures no-change and
reversible alternatives remain visible.

### XSH stewardship

Owns the product-change circuit: implementation worktrees, exact patch lineage,
canonical XSH contracts, focused tests, compatibility and migration review,
integration readiness, delivery, and rollback observation.

### Epistemic library

Curates trace compression, lessons, applicability scopes, contradiction links,
retrieval policies, and revalidation schedules. It owns neither the truth of a
claim nor unilateral promotion power.

### Outcome observatory

Maintains temporal obligations after a merge or institutional promotion. It
runs scheduled checks, imports ecosystem consequences, detects prediction
failures, and reopens episodes when triggers fire. Without it, the society
systematically selects for short-horizon appearance.

### Institutional science

Studies the society itself. It defines held-out case sets, preregisters
organization trials, preserves baselines, prevents outcome leakage, measures
calibration and cost, and proposes bounded R2 mutations. It cannot promote its
own favored variant without independent authority.

### Resource treasury

Accounts for all scarce resources and enforces portfolio envelopes, leases,
WIP limits, cancellation, and emergency stops. It makes allocation decisions
visible; it does not convert resource wealth into epistemic truth.

These institutions are seed constitutional organs, not a final social anatomy.
Their boundaries may split, compose, or be replaced through C3 trials, provided
the required functions and independence constraints remain satisfied.

## Corporate operating system

The epistemic graph is the society's account of reality; corporate structures
are its durable means of coordinating action. V1 was right that tickets,
projects, review, leadership, budgets, and postmortems reduce ambiguity. Its
mistake was allowing a ticket pipeline to stand in for inquiry. V2 preserves
both layers and links them with typed references.

```text
UniverseSeed
    |
    v
TheGrandArchitect -----> Portfolio envelopes and Decisions
    |                           |
    +---- charters ------------+
                |
             Projects
          /       |       \
   Episodes    Tickets    Milestones
      |           |
   knowledge   Attempts -> Submissions -> Judges
          \       |       /
           ProductChanges
                |
       Outcomes and obligations

CoordinationPulse observes the whole operating surface.
AdversarialReview can challenge any named revision.
Retrospective learns routinely; Postmortem responds to a trigger.
```

A Project or Ticket may cite graph motivation; it never becomes the only place
where that motivation exists. A Question does not become actionable simply by
having a ticket. Conversely, deterministic maintenance need not manufacture a
grand Hypothesis when a reproduced condition, acceptance judge, and product
contract are sufficient.

### Projects

A `Project` is a Grand-Architect-chartered portfolio container for a coherent
objective and resource/risk envelope:

```text
Project {
    project_id
    project_revision
    universe_seed_revision
    title
    purpose
    north_star_alignment_id
    portfolio_partition
    steward_principal_or_office
    risk_class
    integration_jurisdiction
    resource_envelope_id
    start_condition
    stop_conditions
    outcome_obligation_policy
    status
}

ProjectObjective {
    project_revision
    objective_revision_id
    ordinal
}

ProjectMilestone {
    milestone_id
    project_revision
    acceptance_judge_revision
    due_condition_or_horizon
    status
}
```

The state machine is:

```text
proposed -> challenged -> chartered -> active -> observing -> closed
proposed | challenged -> rejected
chartered | active | observing -> paused -> active
chartered | active | paused | observing -> terminated
closed | terminated -> reopened -> active
```

`blocked` is an orthogonal condition with owner, cause, escalation horizon, and
budget consequence. `closed` requires milestone dispositions, Ticket and
Episode dispositions, budget reconciliation, due outcome obligations, and a
Retrospective. `terminated` preserves unfinished obligations and requires a
reason; it is not a successful close.

The Grand Architect charters, pauses, terminates, or reopens a Project through
a Decision. A Project steward may plan and allocate only within its envelope.
An actor cannot enlarge its own project or convert unspent budget into a new
purpose without a charter revision.

### Tickets

A `Ticket` is a typed operational work order inside one Project:

```text
Ticket {
    ticket_id
    ticket_revision
    project_revision
    universe_seed_revision
    graph_motivation
    title
    requested_change_or_observation
    bounded_scope
    non_goals
    acceptance_judge_revision
    required_capability
    resource_reservation_id
    risk_and_reversibility
    independence_requirement
    owner
    status
}

TicketAcceptance {
    ticket_revision
    ordinal
    observable_condition
    evidence_kind
}
```

Its lifecycle is:

```text
draft -> admitted -> ready -> claimed -> submitted -> verified -> completed
draft | admitted -> rejected
admitted | ready | claimed -> cancelled
claimed -> expired -> ready
submitted -> changes_requested -> claimed
submitted | verified -> failed
completed | failed | cancelled -> reopened -> admitted
```

Readiness is a kernel derivation over prerequisite graph revisions, Project
state, reserved cost, required capability, workspace availability, and judge
availability. A worker may not mark its own ticket `verified` unless the ticket
contract explicitly names a deterministic self-verifiable judge. Product
implementation and product delivery remain separate tickets or gates when
their authorities differ.

Ticket text is a projection. Rust command variants and normalized SQLite rows
are authoritative. There is no `ticket.json`, generic payload, label-driven
transition, or completion inferred from a directory. Revisions preserve scope
changes instead of editing acceptance after seeing a weak result.

### Adversarial reviews

An `AdversarialReview` is scheduled work with a target, challenge budget,
independence rule, and required disposition. Review kinds are closed and may be
combined deliberately:

```text
Assumption
EvidenceAndProvenance
CurationAndDisclosure
ProductAndApi
CompatibilityAndMigration
SafetyAndSecurity
CostAndEfficiency
InstitutionAndGovernance
```

The review output is not “approve/reject” prose. It is zero or more typed
`ReviewChallenge` revisions:

```text
ReviewChallenge {
    challenge_id
    review_id
    challenged_object_revision
    kind
    failure_hypothesis
    applicability_scope
    evidence_or_reproducer
    severity
    falsification_condition
    requested_disposition:
        Correct | AddEvidence | NarrowScope | AcceptRisk | Revert | Escalate
    reviewer_configuration_and_ancestry
    disclosure_frontier
    status
}
```

Review lifecycle:

```text
requested -> assigned -> active -> findings_submitted -> responses_due
responses_due -> resolved | accepted_risk | superseded | escalated
requested | assigned -> cancelled
active -> failed | expired
resolved | accepted_risk -> reopened
```

The challenged owner responds to each finding with evidence, correction, scope
narrowing, or a reasoned rejection. For C2–C4, the Grand Architect issues the
final disposition. The reviewer cannot edit the target, allocate itself more
time, or create a generic veto. Only an exact R1/R0 rule can make an unresolved
challenge block a transition. Review independence considers actor and
organization ancestry, shared context, source authorship, and evaluator
ownership; two labels over the same configuration are not independent.

Adversarial review is itself costed and challenge capacity is scarce. Risk,
uncertainty, reversibility, blast radius, weak evidence, lineage correlation,
and unresolved minority arguments determine its depth. Routine deterministic
changes use a fast review path. Review theater is measured as coordination cost
and findings that never discriminate a decision.

### Coordination pulses (standups without theater)

The useful core of a standup is common situational awareness. The default V2
implementation is a deterministic `CoordinationPulse` generated at an event or
time boundary, not a paid multi-agent meeting:

```text
CoordinationPulse {
    pulse_id
    scope: Society | Portfolio | Project | Circuit
    source_event_cursor
    changed_projects_and_tickets
    active_blockers_and_escalations
    decisions_and_reviews_due
    cost_spent_reserved_remaining_unknown
    worker_slots_and_wip
    new_displaced_decayed_and_contradicted_signals
    deliveries_and_outcome_obligations_due
    next_committed_actions_and_owners
    generated_at
}
```

The pulse contains typed references and rebuilds from authority; Markdown is
only its view. It cannot declare progress, resolve conflict, or invent a next
action. The Grand Architect and Project stewards acknowledge the pulse or issue
commands in response. A model-synthesized briefing is optional, separately
budgeted, cites the exact pulse, and gains no authority from eloquence.

Pulse triggers include start-of-operation, material state transition, new
blocker, cost threshold, decision horizon, delivery, failed prediction,
retraction, and a bounded quiet interval. Coalescing prevents event spam while
urgent constitutional, safety, and budget conditions bypass ordinary batching.
Repeated unchanged pulses do not consume actor calls.

### Postmortems

A `Postmortem` is mandatory after any configured trigger:

```text
cost cap breach or unknown-cost forced stop
security or process-boundary escape
delivered regression or revert
trusted invariant or evidence-physics failure
lost, duplicated, or irreconstructible durable state
constitutional process violation
repeated Project milestone miss beyond policy threshold
Grand Architect order with a stated trigger
```

The contract is:

```text
Postmortem {
    postmortem_id
    trigger_kind
    affected_scope
    universe_seed_revision
    incident_start_and_detection_events
    immediate_containment
    current_impact
    owner
    independent_challenger
    status
}

PostmortemCausalClaim {
    postmortem_id
    claim_revision
    kind: Proximate | Contributing | Systemic | Detection | Recovery
    evidence
    confidence_form
    competing_claim
    falsification_or_followup
}

PostmortemActionProposal {
    postmortem_id
    proposed_project_ticket_lesson_or_invariant
    expected_prevention_or_detection_effect
    judge
    cost
    owner
}
```

Lifecycle:

```text
triggered -> contained -> evidence_collecting -> causal_review
causal_review -> actions_proposed -> Grand_Architect_disposition -> observing
observing -> closed | reopened
```

The event and cost timeline is derived from the ledger, not reconstructed from
participant memory. Participant accounts and dissent are additional evidence.
The review distinguishes triggering condition, root-enabling conditions,
detection latency, containment quality, and recovery. It records
counterfactuals without pretending one story is proven merely because it is
coherent.

Corrective proposals do not take effect from the postmortem document. They
become separately admitted Tickets, Lessons, Invariants, Project revisions, or
C3/C4 proposals with their own authority and evaluation. The Grand Architect
may accept residual risk but must state horizon and reopen trigger. Blame,
punitive reputation, and ritual “action items” with no judge are excluded.

### Retrospectives

Every completed Episode and Project receives a routine `Retrospective` covering
prediction accuracy, decision sufficiency, curation, circuit fit, cost,
delivery, propagation, and remaining uncertainty. A Retrospective may propose
work but carries no incident or containment semantics. The distinction matters:
if every ordinary lesson becomes a postmortem, failure response becomes noise;
if every breach becomes a retrospective, accountability and containment vanish.

## Continuous service and bounded Operating Cycles

`societyd` runs continuously until its host stops it. Paid and effectful work
does not occur in one unbounded lifetime, however. It is admitted through a
typed `OperatingCycle`: a bounded resource, configuration, monitoring,
cancellation, and reconciliation epoch.

V1's cycle preserved four valuable contracts:

- one aggregate budget and admission ceiling;
- one pinned request/configuration world;
- one ownership root from which every descendant could be stopped; and
- one mandatory closeout after success, failure, or partial execution.

V2 retains those contracts and discards the accidental ones. An Operating Cycle
is not one eval-to-ticket pipeline, one user prompt, a commit quota, or the
epistemic unit of progress. It does not require all Projects to finish, convert
inquiry into a Ticket, or produce a product change.

### Operating Cycle contract

```text
OperatingCycle {
    operating_cycle_id
    sequence
    universe_seed_revision
    grand_architect_occupancy_revision
    organization_configuration_revision
    actor_model_policy_revision
    admission_and_scheduling_policy_revision
    tracing_and_monitor_policy_revision
    resource_envelope_id
    maximum_wip
    start_event_cursor
    completion_and_rollover_conditions
    cancellation_root_id
    predecessor_cycle_id
    status
}
```

An Actor Attempt, deterministic run, or product materialization belongs to
exactly one Operating Cycle. A Project, Ticket, causal Episode, Lesson,
Postmortem, and OutcomeObligation may span several cycles. The membership
record says which revision was admitted, why, and under which reservation; the
cycle never becomes the only record of the work's purpose.

Initially there is at most one nonterminal Operating Cycle. This makes the
society-wide budget, Grand Architect monitor, configuration frontier, and
aggregate cancellation root unambiguous. Later concurrent cycles require a C3
trial with disjoint jurisdictions and ancestor-budget proof; concurrency is not
enabled merely to keep more agents busy.

### Lifecycle

```text
proposed -> admitted -> running
running -> quiescing -> drained
drained -> running | reconciling
reconciling -> closed

running | quiescing | drained
  -> cancelling -> reaping -> reconciling
  -> cancelled | failed
```

`QuiesceOperatingCycle` atomically closes new admission while allowing already
running children to reach their declared completion or grace deadline. A
drained cycle may resume if its configuration and budget remain valid, or enter
reconciliation. `CancelOperatingCycle` propagates a typed cancellation request
to every owned descendant. `EmergencyStop` may skip cooperative grace but not
reaping, evidence preservation, or reconciliation.

Cycle reconciliation requires:

```text
no live or unowned descendant process
every Attempt and lease has an explicit disposition
Pi sessions/events and required partial evidence are sealed
known/unknown cost and all reservations are reconciled
all cancellation requests have terminal propagation state
workspace and worktree retention/cleanup state is explicit
Grand Architect Office session is settled, sealed, and costed when present
open Projects/Tickets/Episodes name their successor-cycle disposition
due decisions, blockers, challenges, and outcome obligations remain visible
final coordination pulse, cycle report, and ledger cursor are rebuildable
```

Closing a cycle never closes a Project or outcome obligation by implication.
The next cycle inherits them through explicit admissions or leaves them waiting
with a reason. A failed cycle remains valid experimental history rather than a
run directory to repair in place.

### Continuous rollover

After clean reconciliation, the Grand Architect may open a successor
immediately. A versioned `CycleRolloverPolicy` may later authorize automatic
rollover when all of these hold:

```text
predecessor reconciled without unresolved containment failure
new exact configuration and budget are admissible
no C4/C5 decision or Postmortem gate requires Office disposition
Grand Architect occupancy remains valid
host halt/quiesce has not been requested
```

Automatic rollover creates a new identity, budget, event frontier, Office
session, and cancellation root; it never “resets” the old rows. Thus the system
can operate continuously while keeping experiments, spend, context, and
shutdown auditable in finite units.

For an actor Grand Architect, one Pi SDK Office session normally lives exactly
as long as the cycle. This supplies a natural context-compaction boundary:
cycle close seals the session, and the successor begins from the active
Universe Seed plus a curated handoff instead of accumulating an immortal chat.

## Organizational circuits

A circuit is a versioned composition of institutions, actors, transitions,
context edges, and judges for a problem class. There is no universal
planner-to-coder-to-reviewer workflow.

Initial circuit families should include:

| Circuit | Intended distribution | Characteristic path |
| --- | --- | --- |
| Repair | Reproduced narrow defect | reproduce -> patch -> focused judge -> independent review -> integrate |
| Empirical inquiry | Tool, docs, or agent-boundary uncertainty | competing hypotheses -> matched trials -> analysis -> intervention |
| Semantic change | Syntax, type, effect, or compatibility contract | alternatives -> formal/compatibility challenge -> prototypes -> deliberation -> staged integration |
| Performance tournament | Objective hot path with deterministic judge | many bounded candidates -> correctness gate -> Pareto archive -> synthesis |
| Incident | Soundness, security, or severe regression | contain -> reproduce -> propagate invariant -> repair -> causal review |
| Cultural change | Guidance or recurring method | evidence synthesis -> scoped lesson -> shadow retrieval -> promote/retract |
| Organization trial | Proposed R2 mutation | historical replay -> held-out shadow -> bounded canary -> promote/retire |

Circuit selection is itself a recorded, challengeable decision. Risk, expected
information gain, reversibility, novelty, ecosystem reach, and evaluator
quality affect selection. A fast circuit must be available for routine fixes so
that scientific ceremony does not consume deterministic work. A semantic
change must not enter that circuit merely to improve throughput.

### Circuit ecology rather than one evolving workflow

The society does not search for a globally optimal organization. It learns a
repertoire of circuits whose usefulness is conditional on problem distribution,
resource regime, and maturity of the relevant evidence. A circuit is closer to
a reusable physiological or legal process than a department hierarchy.

Circuits may:

- compete on matched cases;
- compose, with one circuit producing an obligation consumed by another;
- fork when the same name hides behaviorally distinct variants;
- share actors or cultural methods without sharing authority;
- lie dormant until a matching signal recurs;
- transfer a successful subcircuit into another problem class; and
- dissolve when their function becomes an evaluator, policy, or XSH primitive.

Selection records both the chosen circuit and plausible rejected alternatives.
The observatory later conditions outcomes on problem features instead of
concluding that one circuit “wins.” Repeated routing errors are evidence about
the classifier, the repertoire, or the observed problem description—not
permission to make the most elaborate circuit universal.

A useful organizational mutation may therefore change the map from conditions
to circuits rather than any circuit internally. Another may alter the balance
between deliberate institutional reasoning and cheap speculative leaves. Both
are society-level heredity and require the same blinded, held-out evidence as a
topology mutation.

## Work, scarcity, and backpressure

### The portfolio, not the backlog

The society maintains a governed portfolio of epistemic and product
obligations. Initial partitions are:

- **stewardship**: confirmed defects, specification coherence, tests, docs,
  migration, and integration maintenance;
- **frontier**: high-uncertainty language, tooling, and workload research;
- **measurement**: evaluators, representative corpora, ecosystem observation,
  and replication;
- **institutional**: propagation, actor ecology, circuit, and governance trials;
- **resilience**: soundness, security, rollback readiness, debt, and reserved
  capacity for unexpected failures.

Partition shares are explicit policies with ranges and exceptions, not a hidden
weighted objective. Under-filled partitions do not fabricate work. Unused
capacity can be temporarily loaned with a recorded recall rule.

### Admission

A candidate obligation is admitted when it has:

```text
Universe Seed relevance and north-star alignment
explicit uncertainty or actionable reproduced condition
expected outcome or information value
required prerequisites and judges
cost and constrained resource estimate
reversibility and blast-radius classification
appropriate circuit and authority
abandonment or stop condition
```

Information gain need not be a precise number. A comparative argument with
uncertainty is preferable to false precision.

### Queues are projections

Queues allocate one scarce resource over graph facts:

```text
question triage       -- inquiry attention
experiment admission  -- model/compute/worker capacity
evidence challenge    -- independent evaluation attention
deliberation           -- decision authority
integration            -- product risk and merge bandwidth
outcome follow-up      -- temporal observation capacity
organization trials   -- constitutional risk budget
```

A queue item carries a graph identity and readiness proof. The queue is never
the only record of why the work exists.

Backpressure propagates through declared dependencies. If evidence challenge is
congested, new speculative experiment admission tightens. If integration is
congested, reversible prototypes may continue while product mutations slow. If
outcome follow-up exceeds capacity, the society must reduce new long-horizon
commitments rather than silently dropping old ones.

### Scheduling

Scheduling combines:

1. constitutional constraints and portfolio envelopes;
2. readiness, dependency, urgency, expiry, and WIP rules;
3. value-of-information and delivery arguments;
4. actor fit, diversity, and independence requirements;
5. bids or local responses to demand signals; and
6. starvation prevention and reserved exploration.

The resulting allocation and rejected alternatives are recorded for costly
work. There is no universal scalar priority. Deterministic tie-breaking may be
used after the governing partial order leaves candidates equivalent.

### Paid actor and cost policy

The initial society has one mandatory model policy for **every** paid actor
attempt, including the Grand Architect when occupied by an agent, inquiry,
curation, implementation, paired probes, adversarial review, and postmortem
analysis:

```text
ActorModelPolicyV1 {
    pi_version: 0.83.0
    provider: openrouter
    model: deepseek/deepseek-v4-flash-0731
    thinking: high
    fallback: None
}
```

There is no quiet provider fallback, model alias, lowered thinking level, or
role-specific exception. Each `ActorConfiguration` references this exact policy
revision. A change is a C3 model-policy mutation with cost, capability,
contamination, and comparison consequences; it does not happen because a local
Pi default changed.

Money is stored and calculated as integer micro-US-dollars:

```text
UsdMicros(u64)

CostObservation =
    Known { cumulative: UsdMicros, source_event_identity }
  | Unknown { reason: ClosedUnknownCostReason }
  | Unavailable { reason: ClosedUnavailableCostReason }

BudgetEnvelope {
    budget_id
    scope_kind
    scope_id
    hard_cap: UsdMicros
    reserved: UsdMicros
    observed: UsdMicros
    status: Open | Exhausted | Frozen | Reconciled
}

BudgetEnvelopeConstraint {
    constrained_budget_id
    ancestor_or_crosscut_budget_id
}

BudgetReservationCharge {
    reservation_id
    budget_id
    reserved: UsdMicros
    observed: UsdMicros
}
```

Floating point does not cross the durable accounting boundary. Provider values
are parsed once with an exact, versioned rounding policy. `Unknown` and
`Unavailable` are states, never zero. Duplicate Pi usage events are
idempotently normalized by session and source-event identity so assistant,
tool, retry, and compaction costs are not double counted.

Budget control is compositional and precommitted. Portfolio/Project/Ticket
constraints are mostly hierarchical; an Operating Cycle is a cross-cutting
aggregate over every execution admitted during that epoch:

```text
society hard ceiling
  ├── portfolio -> Project -> Ticket / Episode / circuit envelopes
  └── active Operating Cycle aggregate envelope

ActorAttempt or OfficeTurn reservation
  -> charges every applicable open envelope atomically
```

Before `StartAttempt`, the kernel transactionally reserves the attempt cap
against every applicable envelope, including its Operating Cycle and Project
lineage. Insufficient unreserved capacity in any one rejects the command;
“probably cheap” is not a budget. Completion reconciles known actual cost and
returns the unused reservation. A retry, forensic re-read by a model, extended
review, or follow-up is a new reservation. Unused money is not a mandate to run
more agents.

The Rust `PiSupervisor` watches the SDK-host event stream and Pi session continuously. On
per-attempt or ancestor-cap breach, malformed cumulative cost, cost regression,
or cost becoming unknown/unavailable after paid work begins, it stops admission,
cancels descendants, terminates the owned process group, seals partial
evidence, and emits a typed budget incident. Provider accounting arrives after
responses, so the hard cap limits authorized continuation rather than claiming
physically impossible zero overshoot. Turn and wall limits bound additional
exposure.

Any aggregate breach or unknown-cost forced stop triggers a Postmortem. The
cost ledger records reservation, known spend, possible unobserved exposure,
cancellation latency, and responsible policy. A provider outage or missing cost
field is not charged as successful agent productivity. The Grand Architect may
lower an envelope or spend explicitly reserved contingency; raising the society
hard ceiling requires a separate budget-ceiling decision through the bootstrap
authority boundary, so an agent cannot fund its own continued deliberation.

Deterministic Rust/XSH judges, projection rebuilds, transaction tests, cached
fixtures, and Pi doubles are the default development path. Paid calls are used
only for preregistered actor work that a deterministic service cannot supply.
Coordination pulses and ordinary status queries never invoke a model. An actor
Grand Architect receives selected pulse/notice batches only through separately
reserved Office turns.

### Resource and contribution accounting

The treasury records tokens, model calls, wall time, CPU, memory, storage,
worker slots, human review, merge bandwidth, follow-up obligations, and risk
exposure. A budget is a capability ceiling, not a target to spend.

Actor and institution contribution records are multi-dimensional and delayed:

```text
calibrated discoveries and counterexamples
decision-relevant evidence
prediction quality by reference class
downstream product and cultural impact
reversals or regressions attributable with uncertainty
compute, context, and coordination cost
niche coverage and diversity contribution
propagation precision and useful retrieval
```

These records guide experiments and population allocation. They are not
transferable currency, social status, or proof of a claim. This avoids creating
a gameable internal economy before the society understands its own credit
assignment.

## Shipping XSH reliably

The failure of a commit quota does not make delivery optional. A society which
only produces refined explanations becomes an academy detached from its
mission. V2 therefore maintains an explicit product metabolism.

### Two clocks

The society runs on two coupled clocks:

- the **stewardship clock** continuously turns reproduced, sufficiently
  understood product needs into small verified XSH changes; and
- the **research clock** resolves uncertainties whose value and horizon do not
  fit a commit cadence.

Research may generate stewardship work. Stewardship outcomes may generate new
questions. Neither clock is allowed to impersonate the other.

### Delivery state

An authorized product proposal progresses through:

```text
decision_ready
  -> implementation_claimed
  -> patch_produced
  -> product_contracts_verified
  -> independent_reviewed
  -> integration_ready
  -> delivered
  -> short_horizon_observed
  -> long_horizon_observed | reopened | reverted
```

Each state has exact Git, evidence, ownership, and expiry requirements. An
integration-ready change cannot disappear because the research portfolio moved
on. A delivery blockage becomes a visible institutional bottleneck.

### Throughput evidence

Useful delivery measures include:

- time from reproduced need to authorized decision;
- time from authorized decision to integration-ready patch;
- integration dwell time and causes;
- verified, shipped changes per unit of implementation and review capacity;
- survival, rollback, and later-regression rates;
- user- or workload-relevant impact; and
- the fraction of ready work blocked by institutional rather than product
  uncertainty.

This makes respectable commit throughput a real observable outcome while
denying it authority to admit weak work. The desired result is not fewer
commits; it is a system that can explain and improve why worthwhile commits do
or do not ship.

## Checked propagation

Knowledge is cumulative only when a warranted local discovery changes relevant
future behavior. Propagation is therefore a stateful institutional operation,
not a handbook edit or broadcast message.

### Lesson contract

A lesson carries:

```text
claim and exact source revision
applicability and exclusion scope
evidence, arguments, and live contradictions
confidence form and calibration basis
owner and independent validator
target audiences and machine-enforced dependents
delivery and behavior-change policy
expiry, revalidation, downgrade, and revocation rules
```

### Knowledge changes media

Propagation is not movement of one immutable document. A warranted distinction
can be transduced through increasingly consequential carriers:

```text
observation
  -> scoped claim
  -> curated lesson or worked example
  -> retrieval/context rule
  -> professional method or qualification case
  -> circuit or governance policy
  -> deterministic evaluator
  -> XSH API, type, effect, or enforced invariant
```

Each conversion has its own authority, scope, evidence standard, rollback, and
failure mode. The source claim remains linked, but the representations need not
have the same shape. “A reviewer should remember to check process ownership” is
weak cultural inheritance; a typed API which makes unowned process lifetime
explicit is technical inheritance of the same underlying knowledge.

This movement from culture into infrastructure is one of the society's
strongest forms of cumulative evolution. It can also remove a former niche: if
an evaluator or language construct cheaply enforces what once required expert
attention, that attention should migrate to unresolved work rather than defend
the old profession.

Transduction is not automatically monotonic. A rigid evaluator may overfit a
provisional lesson; a context rule may contaminate unrelated inquiries; a type
system may encode the wrong causal account. Every target representation names
the lesson revision and scope it operationalizes so contradictory evidence can
find and challenge the derived machinery.

### Propagation ladder

| Level | Meaning | Permitted effect |
| --- | --- | --- |
| L0 observation | One bounded occurrence | Visible within its episode |
| L1 candidate lesson | Curated interpretation awaiting independent support | Discoverable by explicit search; no default context |
| L2 provisional guidance | Supported within a named scope | Included in matching context packs with confidence and contradiction links |
| L3 institutional policy | Repeatedly supported practice with governance approval | Changes a circuit, default, or qualification suite within jurisdiction |
| L4 enforced invariant | Deterministic or constitutionally required boundary | Blocks violating actions through an external judge |

Promotion is not monotonic. A lesson can remain useful at L2 indefinitely.
Ergonomic judgment rarely belongs at L4; a reproduced soundness condition may
reach it quickly once an independent regression judge exists.

### Propagation pipeline

```text
observation
  -> interpretation and contradiction search
  -> independent validation
  -> applicability classification
  -> dependent-work query
  -> authorized promotion
  -> context, policy, evaluator, or invariant update
  -> delivery receipts to affected active work
  -> later check that behavior actually changed
```

Notification is not propagation success. The system measures whether relevant
work received the correct revision and whether the intended decision or judge
changed. It also samples non-target work to estimate contamination.

The propagation record distinguishes increasingly strong observations:

| State | What is established |
| --- | --- |
| Targeted | A declared audience or machine dependent should receive the knowledge |
| Delivered | The exact revision reached the target representation or context |
| Encountered | A target actor, circuit, or judge actually processed that representation |
| Applied | The expected distinction is visible in one relevant behavior or decision |
| Causally supported | A matched or otherwise discriminating comparison supports attribution to the propagation |
| Institutionalized | The behavior persists across target cases, citizens, and time within scope |

These are observations about a target, not promotion levels of the lesson
itself. A high-confidence L3 policy can be delivered but not yet encountered; an
L1 lesson can be explicitly encountered in a research probe without becoming
default guidance. The database must not turn delivery acknowledgement into
behavior change through a generic `completed` flag.

Propagation latency is measured from the warrant or promotion event to each of
these boundaries, with censored targets left visible. Precision asks whether
targets were relevant; recall asks which relevant targets were missed;
contamination asks whether non-target behavior changed; impact asks whether the
knowledge altered a consequential decision or invariant. Fast delivery with low
warrant or indiscriminate scope is a failure, not an optimization victory.

### Retraction is symmetric

Contradictory evidence can downgrade or revoke a lesson. Retraction:

1. preserves the old lesson and original promotion basis;
2. records the contradictory evidence and retraction authority;
3. finds policies, evaluators, decisions, context packs, and active episodes
   which depended on it;
4. invalidates derived readiness where required;
5. notifies or reopens affected work; and
6. records acknowledgements and unresolved exposure.

Propagation quality is consequently a precision, recall, latency, and impact
problem. Minimum latency for warranted knowledge and maximum resistance to
unwarranted knowledge is the target—not global instant agreement.

## Evaluation without a lie

### Partial order before judgment

Candidate products and organizations retain a vector of observations,
constraints, and uncertainties:

```text
candidate A dominates on:
    semantic regularity, implementation simplicity, compile time

candidate B dominates on:
    migration cost, runtime throughput

hard constraints:
    no soundness regression
    product change must remain independently revertible

unknown or contested:
    agent learnability, ecosystem adoption, future optionality
```

Hard constraints can disqualify. Pareto dominance can remove strictly worse
alternatives. Neither eliminates the need for contextual judgment when values
conflict. The decision authority chooses and records why; later outcomes test
the choice.

Scalar metrics remain strong evidence when their measurement contract is
sound. They are not promoted to universal values merely because a scheduler can
sort them.

### Predictions and temporal fitness

Every significant proposal and organization promotion makes predictions at
named horizons. Outcomes may be:

- deterministic and immediate, such as a test result;
- sampled, such as task success over a workload distribution;
- delayed, such as migration pain or maintenance burden;
- censored because the observation window ended; or
- confounded, with an explicit competing attribution.

An outcome obligation is a durable scheduled object with an owner and budget.
Missing follow-up is visible debt, not an implicit success.

The system scores prediction calibration only within a comparable reference
class. It separately evaluates decision process, outcome, and attribution so
that lucky outcomes do not teach reckless governance.

### Society health vector

The observatory should maintain at least these dimensions, with definitions and
uncertainty rather than one dashboard score:

| Dimension | Questions |
| --- | --- |
| Product | Is XSH more correct, coherent, compatible, useful, and maintainable? |
| Discovery | How much decision-relevant uncertainty is resolved per scarce resource? |
| Delivery | How reliably do evidenced decisions become surviving shipped changes? |
| Calibration | Do predictions and confidence forms match later outcomes? |
| Propagation | Does warranted knowledge reach relevant work quickly and selectively? |
| Memory | Can later actors reconstruct why a decision happened without transcript archaeology? |
| Reversal | Are decisions revisited at the right rate and for the predicted reasons? |
| Diversity | Are important niches and independent lines of reasoning preserved? |
| Resilience | Can work be cancelled, replayed, challenged, reverted, and recovered? |
| Metamorphosis | Do validated institutional lessons produce out-of-sample organizational gains? |
| Cost | What model, compute, human, coordination, risk, and follow-up resources were consumed? |
| Future capacity | Did XSH or the society become easier to understand and safely modify next time? |

Metric definitions, sampling policy, and collection code are versioned evidence.
A metric whose target has become gameable can be downgraded without deleting its
history.

## Organizational heredity and experimentation

### Organization configuration

An `OrganizationConfiguration` pins the R2 variables which could affect an
episode:

```text
OrganizationConfiguration {
    actor_population_and_diversity_policy
    actor_configurations_and_model_assignments
    institutions_and_circuits
    communication_and_context_topology
    jurisdiction_and_escalation_rules
    work_selection_and_resource_policy
    evaluator_and_challenge_assignment
    memory_retrieval_and_trace_curation
    propagation_and_retraction_policy
    outcome_followup_policy
    branching_mutation_and_retirement_policy
}
```

Unpinned ambient behavior is an experimental defect. Secret model routing,
changed prompts, evaluator drift, and corpus drift must not be attributed to an
organization mutation.

### Mutation protocol

An institutional mutation begins as a causal claim, not an edit:

```text
observed failure cluster
  -> hypothesis about an organizational cause
  -> proposed changed fields and mechanism
  -> preregistered affected problem distribution
  -> baseline and descendant configurations
  -> budget, safety envelope, judges, and stop rule
  -> replay, shadow, or canary evidence
  -> decision to retain, revise, combine, or retire
  -> delayed outcome and replication
```

Semantic mutations proposed by a meta-actor are welcome: for example, “failures
cluster around assumptions made before implementation; insert an independent
assumption challenge for C2 semantic work.” Blind mutations are also useful in
bounded search spaces. Both share lineage and evaluation requirements.

### Decision worlds and epistemic disclosure frontiers

Every consequential episode should be capable of yielding a historical
decision world. A decision world is not a transcript replay. It reconstructs
the actionable reality at one legitimate knowledge boundary:

```text
DecisionWorld {
    source_episode_and_decision
    frontier_event_and_time
    allowed_object_revisions
    admitted_source_and_repository_snapshots
    organization_and_execution_configuration
    culture_and_policy_available_as_of_frontier
    unresolved_unknowns_and_conflicts
    authority_and_resource_envelope
    explicitly_sequestered_descendant_material
}
```

The authoritative registry may link the world to its source episode and
decision so an independent judge can later open the aftermath. The replay
principal receives a frontier-local opaque world identity; source names or
identifiers which reveal the decision are not members of its view.

The `EpistemicDisclosureFrontier` is an enforceable allowlist derived from
source chronology and explicit exceptions. It excludes later outcomes,
retrospectives, reversals, lessons derived from the case, current source which
embodies the answer, and indirect identifiers which trivially reveal the
aftermath. A replay context is assembled only from frontier members. Any raw
forensic access outside the frontier contaminates the attempt and remains
visible rather than being repaired after the fact.

The frontier captures availability, not truth. It can contain a stale proposal,
an unresolved conflict, or evidence later shown misleading if those were
legitimately part of the original world. Hindsight must not make the ancestor
look irrational or the descendant clairvoyant.

The aftermath is retained separately:

```text
decision world W at frontier F
  -> original action and rationale
  -> immediate observations
  -> delayed, censored, or confounded outcomes
  -> reversals and later lessons
```

This split makes historical reality an expanding experimental environment. It
supports tests of judgment, routing, curation, profession candidates,
propagation policies, and entire organization configurations without requiring
a universal quality function or waiting for every new variant to accumulate
years of live consequences.

Counterfactual output never edits the historical episode. It creates a new
attempt linked by `replays_world`, with its own proposed decision and cost.
Comparison asks which relevant distinctions the variant recovered, which
outcomes its predictions anticipated, what it would have changed, which new
errors it introduced, and which resources it consumed. The original decision
is evidence, not an answer key.

### Experimental ladder

Organization variants advance through increasingly realistic but risky worlds:

1. **Qualification** verifies schema, capability, budget, and deterministic
   invariants.
2. **Historical counterfactual replay** shows a prior episode without its
   outcome and compares the descendant under a matched envelope.
3. **Held-out shadow** runs on new work with no decision authority.
4. **Bounded canary** grants authority for a narrow problem class with a live
   baseline, rollback, and expiry.
5. **Scoped promotion** makes the configuration the default only in the
   demonstrated jurisdiction.
6. **Replication and revalidation** test whether the effect survives corpus,
   model, and time changes.

Historical replays must sequester later outcomes, retrospectives, current
lessons derived from the case, and other leakage. A replay is a test of behavior
under a reconstructed information set, not an opportunity to recite history.

The historical corpus is stratified by problem class, risk, ambiguity, outcome
horizon, and known institutional failure—not optimized into one leaderboard.
Cases used to hypothesize or train a mutation cannot certify it. Held-out worlds
remain sealed until the organization configuration and predictions are fixed.
As model families, XSH revisions, and external conditions change, live shadow
and canary evidence determine whether historical performance still transfers.

Profession formation uses the same machinery. A recurring failure pattern may
justify several candidate phenotypes, but a profession is recognized only when
one or more variants recover useful distinctions across blinded worlds and
later work at acceptable coordination cost. Thus the fossil record creates
niches without allowing hindsight to manufacture expertise.

### Selection preserves an archive

The result of an organization trial is not necessarily a winner. The archive
retains non-dominated variants, behavioral novelty, failure regions, resource
cost, and scope. Different circuits may be optimal for different problem
distributions.

Promotion requires evidence on held-out or later cases. Training directly on a
historical corpus may produce a new candidate; performance on those same cases
cannot certify recursive improvement.

### Strict RSI criterion

V2 may claim one bounded instance of recursive institutional improvement only
when all of the following are true:

1. An episode produced a supported causal hypothesis about the society's own
   machinery.
2. The resulting R2 mutation was implemented with lineage and without changing
   its certification standard.
3. A matched trial showed a meaningful non-dominated improvement under an
   equivalent authority and resource envelope.
4. The effect replicated on held-out or later work.
5. The society promoted the mutation through its prior constitutional process.
6. The descendant retained replayability, diversity, safety, and revocation.

This is a deliberately higher bar than “the system edited itself.”

## XSH and the society co-evolve

### XSH's initial jurisdiction

XSH is an R2 experimental and actor-facing medium, always supervised as a child
of the Rust kernel. Initial XSH surfaces may include:

- reproducible experiment descriptions and workload composition;
- bounded process, filesystem, JSON, text, byte, and host-state operations;
- proposal builders for policy and circuit variants which the kernel validates
  as typed configuration;
- evaluator programs and deterministic judges executed under exact profiles;
- context and human-readable decision projections;
- actor tools, replay experiments, and outcome-observation scripts; and
- the society's own native behavior tests at the XSH boundary.

Rust owns `societyd`, SQLite, transactions, the canonical schema, capabilities,
Offices,
Operating Cycles, admission, Pi/process supervision, sessions, costs,
cancellation, tracing, workspaces, Git materialization, content sealing, ledger
replay, and crash recovery. XSH never becomes a database DSL, alternate durable
workflow engine, independent Pi runner, cleanup authority, or replacement for
the trusted core merely because self-hosting feels elegant.

### XSH is cognitive and social technology

An XSH construct changes more than executable programs. It changes which
distinctions are cheap to express, visible to tools, enforceable by evaluators,
recoverable from source, and transmissible between citizens. The language is
therefore part of the society's developmental environment.

Examples include:

- explicit process ownership making cancellation and reaping available to both
  static checks and review actors;
- typed paths eliminating a class of shell-quoting folklore from cultural
  memory;
- stable semantic identities enabling transformations and decisions to refer to
  meaning rather than source coordinates;
- structured errors making failure observations comparable across episodes;
- effect declarations creating new niches for effect auditors, transformation
  tools, or local scheduling policies; and
- executable invariants moving fragile professional knowledge into durable
  technical heredity.

A language change may also destroy social capacity: obscure a distinction,
force agents into shell escape hatches, make historical programs unreadable,
or centralize expertise in a costly profession. Product evaluation must
therefore consider the institutions and phenotypes a feature enables, changes,
or makes obsolete—not only runtime and surface syntax.

The society must not prefer self-use for its own sake. XSH remains a language
for humans and real systems-glue workloads. Institutional leverage is one
Universe Seed value whose trade with simplicity, compatibility, performance, and
human legibility remains explicit.

### The closed-loop opportunity

```text
better XSH boundaries and representations
  -> clearer experiments, policies, and systems work
  -> more reliable actor behavior and evidence
  -> better institutional decisions and implementation capacity
  -> better XSH boundaries and representations
```

An XSH change is unusually consequential when it improves how the society
expresses intent, invariants, effects, process ownership, structured evidence,
or transformation provenance. The society should explicitly measure those
effects rather than assuming self-use makes a feature good.

### Co-evolution ladder

XSH may earn broader **creative** responsibility in stages without entering
trusted physics:

1. **Experimental medium**: fresh experiments and deterministic evaluators are
   written in XSH.
2. **Policy-proposal medium**: scheduling, context, and circuit variants are
   constructed or simulated in XSH and admitted as typed kernel configuration.
3. **Cultural medium**: lessons include executable XSH examples and native
   qualification cases.
4. **Institutional user-space medium**: mature institutions may use XSH actors,
   evaluators, and tools while Rust continues to enforce every effectful
   transition.
5. **Evolution medium**: organization trials can demonstrate that a candidate
   XSH feature improves institutional productivity under matched conditions.

Advancement is per surface, reversible, and evidence-based. There is no ladder
rung for rewriting `societyd`, its database, Pi supervisor, or cancellation
core in XSH.

### Self-hosting in the relevant sense

XSH is intentionally a systems-glue language, not a general-purpose service
runtime. The society must not distort the language merely to satisfy a poetic
compiler-self-hosting milestone.

The meaningful self-hosting invariant is instead:

> The society can reconstruct, explain, test, and increasingly conduct its own
> XSH-facing creative work using the version of XSH produced by its lineage,
> while a separately versioned Rust kernel preserves authority, execution,
> evidence, and cancellation physics.

Every generation pins a bootstrap chain and remains testable against ancestral
workloads. A descendant cannot declare improvement by deleting a workload or
changing a judge without exposing that as a separate governed mutation.

### Improvement productivity experiment

The most revealing XSH fitness question is sometimes not whether a feature
makes one program better, but whether it makes future improvement more reliable:

```text
same task distribution, models, tools, authority, and budget

society using XSH revision A -> decision quality, delivery, cost, failures
society using XSH revision B -> decision quality, delivery, cost, failures
```

This is **lineage utility**, distinct from local product utility. A feature can
be locally awkward yet lineage-positive if it makes later changes safer,
creates better experiments, or enables a valuable institutional form. It can
also be locally elegant yet lineage-negative if it hides effects or makes
future transformation brittle. Neither claim is licensed by intuition alone.

An improvement-productivity trial defines:

```text
target problem distribution and held-out tasks
baseline and candidate XSH worlds
matched models, actor configurations, tools, authority, and budgets
learning/wash-in period and contamination barriers
circuits permitted to adapt versus variables deliberately held fixed
product, discovery, propagation, and coordination observations
failure severity and external assistance or override
delayed follow-up and ancestral compatibility
```

The observations remain multidimensional:

- correct task completion and surviving delivered changes;
- time and scarce evaluation/integration attention per warranted change;
- ability to explain and review effects without transcript archaeology;
- prediction calibration and reversal frequency;
- shell escape hatches, workaround complexity, and hidden host assumptions;
- lesson uptake and the cost of moving a discovery into an evaluator or
  invariant;
- maximum safe change scope achieved under the same authority envelope;
- new useful circuits or phenotypes made practicable; and
- regressions for human users, external workloads, and bootstrap lineage.

The society first uses microprobes—for example, matched actors implementing the
same process-supervision task against two API/reference worlds. Those probes
generate hypotheses and evaluator evidence; they do not establish general
lineage fitness. Promotion needs several held-out tasks and delayed evidence.

Organization adaptation creates a special experimental choice. Holding the
organization fixed estimates whether a language world is immediately easier to
use. Allowing bounded adaptation estimates the larger question: which new
social forms become possible in that world? Both are valuable and must not be
mixed into one result.

A demonstrated gain can justify a visible trade, such as a small local runtime
or syntax cost in exchange for materially safer modification and review. It
cannot make the trade disappear into a self-improvement score. The decision
still exposes whose work improved, whose worsened, over what horizon, and under
which society configuration.

This is the strongest closure available to V2: XSH improves the society's
capacity to produce warranted XSH improvements, and the society evaluates that
claim without allowing XSH or the organization to rewrite the historical worlds
or standards used to judge it.

## External world and ecosystem

A repository-only society will overfit to what its tests can see. The
observatory must maintain versioned interfaces to:

- representative XSH systems-administration and build workloads;
- the current specification, examples, native tests, and open proposal corpus;
- downstream scripts and migration fixtures where available;
- user reports and human qualitative evaluations;
- platform, dependency, and toolchain variation;
- performance hardware classes; and
- historical V1 and V2 episodes.

External evidence records source, collection time, consent or license where
relevant, selection method, trust classification, and known bias. Untrusted
repository text and issue content are data, never authority-bearing
instructions.

Workload corpora have lineage and governance. Adding, deleting, or reweighting
a case can be a legitimate measurement improvement, but it cannot be smuggled
into the evaluation of a variant which benefits from the change.

Human taste remains valuable evidence for naming, learnability, diagnostics,
and semantic coherence. The architecture makes that judgment attributable and
revisitable; it does not demand that every qualitative value be replaced by a
proxy.

## Safety, security, and failure containment

### Threat model

| Failure pressure | Required defense |
| --- | --- |
| History rewriting | Append-only events, sealed artifacts, independent replay audit |
| Self-certification | Change classes, pinned evaluators, separation of promotion authority |
| Reward or benchmark gaming | Multidimensional evidence, held-out worlds, corpus lineage, later outcomes |
| Capability escalation | Kernel-enforced narrow capabilities, jurisdiction, expiry, denial events |
| Resource capture or replication storm | External ceilings, exact accounting, population and concurrency limits |
| Monoculture and collusion | Ancestry-aware independence, diversity reserves, adversarial assignment |
| False global lesson | Scoped propagation ladder, independent validation, symmetric retraction |
| Epistemic flooding | Admission, curation, WIP limits, decay and retirement of low-value claims |
| Outcome leakage | Sequestered replay views and contamination audits |
| Prompt injection from artifacts | Trust labels, content/instruction separation, tool capability boundaries |
| Metric drift | Versioned definitions, reference classes, calibration, challenge and retirement |
| Institutional ossification | Expiry, sunset review, shadow challengers, reversible circuit trials |
| Short-horizon selection | Durable follow-up obligations and outcome reserve |
| Product regression | Isolated worktrees, independent tests/review, exact lineage, canary and revert |
| Kernel corruption | External review/deployment, backups, integrity checks, ledger rebuild |
| Stale or orphaned execution | Transactional admission generation, inert SDK-host handshake, durable child registry, process groups, recovery containment |
| Cancellation race or partial stop | Typed propagation, abort/TERM/KILL deadlines, signal receipts, reaping, partial-evidence and cost reconciliation |
| Observability flood or secret leak | Curated targets/levels, typed redacted fields, bounded sinks, durable OperationalNotices rather than raw trace feedback |

### Circuit breakers

The kernel and constitution support:

- immediate admission freeze by scope or change class;
- cancellation of owned process trees and leases;
- budget cutoff which preserves partial evidence;
- quarantine of an actor, model, evaluator, lesson, or organization revision;
- product and policy rollback to an exact parent configuration;
- retraction fan-out to affected active work;
- read-only forensic mode; and
- full materialized-state rebuild from the canonical schema, ledger, and sealed
  artifacts.

Emergency action records authority and reason. It may stop harm before normal
deliberation, but requires a later retrospective and cannot erase the evidence
which triggered it.

### Cancellation is a control-plane primitive

Cancellation is designed into admission, process ownership, costs, notices,
and coordination from the first schema. It is not cleanup after a failed
workflow and not an ordinary `DerivedSignal`. A signal may create an attention
bid asking the Grand Architect to quiesce or cancel; only an authorized command
or an exact trusted circuit breaker changes control state.

```text
CancellationRequest {
    cancellation_request_id
    scope: Society | OperatingCycle | Project | Ticket | ActorAttempt |
           RootAuthorityOfficeSession | DeterministicRun
    mode: Quiesce | GracefulCancel | EmergencyStop
    reason: ClosedCancellationReason
    requested_by: Principal | TrustedBreaker
    source_event_or_breaker
    admission_generation
    propagation_policy_revision
    cooperative_deadline
    term_deadline
    evidence_retention_policy_revision
    status
}
```

The lifecycle separates intent, process control, and reconciliation:

```text
requested -> admission_closed -> propagating -> cooperative_stop
cooperative_stop -> terminating -> killing -> reaping
cooperative_stop | terminating | killing | reaping -> evidence_sealing
evidence_sealing -> cost_reconciling -> reconciled

reconciled -> cancelled_cleanly | killed_after_grace | partial_failure
```

States may be skipped only when their action is inapplicable and the receipt
records why—for example a queued Ticket has no child to signal. `Quiesce`
usually stops at `admission_closed` for the scope and allows active Attempts to
finish. `GracefulCancel` asks the Pi SDK host to call `session.abort()`, closes or cancels pending
one-shot work, then sends TERM to each exact process group after the cooperative
deadline. `EmergencyStop` may send KILL immediately. Every mode still reaps,
seals partial evidence, and reconciles cost.

Cancellation propagation follows declared ownership edges:

```text
Society -> active Operating Cycle and all daemon-owned children
Operating Cycle -> admitted Office session, Attempts, deterministic runs,
                   materializations and watchers
Project/Ticket -> only currently owned descendants across the active cycle
ActorAttempt -> Pi SDK-host/process group, evaluator descendants and leases
```

A child failure does not cancel a sibling unless the cycle/circuit policy names
that dependency. Epistemic objects are never cancelled; their associated work
is. Partial observations retain their actual evidence status rather than being
deleted or promoted to failure evidence automatically.

Every admission scope has a monotonically increasing admission generation.
A pre-spawn admission captures the generation; the SDK host receives
`CreateSession` only after a separately committed final authorization confirms
it remains current. Quiesce or cancellation increments the generation
transactionally before propagation, so a racing scheduler cannot start a child
from a stale readiness result. Recording later buffered delivery or
`SessionReady` evidence never reopens the cancelled authority.

The cancellation root remains nonterminal until:

- every registered child is settled or explicitly classified orphaned;
- owned process groups have been reaped or containment failure is ERROR;
- Pi SDK-host streams, session, stderr, workspace and Git receipts are sealed
  to the required partial-evidence policy;
- known spend and possible unobserved exposure are reconciled;
- leases and reservations have terminal dispositions; and
- dependents, coordination pulse, Grand Architect notice surface, and any
  mandatory Postmortem reflect the same cancellation identity.

Host SIGINT/SIGTERM, Grand Architect commands, cycle stop conditions, budget
breach/unknown cost, and R0 invariant failure all enter this one contract. No
handler performs an unrecorded parallel cleanup path. Recovery after daemon
restart resumes nonterminal requests from SQLite and compares them with the
live process registry before taking further action.

### Safety invariants

At minimum, deterministic tests prove that:

- no principal mutates facts outside its current capability and jurisdiction;
- no accepted command changes historical payload under an existing identity;
- no actor expands its own budget, authority, population, or evaluator;
- no mutable projection can create graph or readiness facts;
- no result cites an unsealed or mismatched input/evaluator revision;
- no product or institutional mutation promotes itself;
- cancellation releases only exactly owned resources and preserves evidence;
- closing or changing an admission generation prevents every stale reserved
  spawn from executing Pi;
- every terminated process has one owning scope, signal/escalation receipt,
  reap result, partial-evidence disposition, and cost state;
- a projection can rebuild and the ledger can independently reproduce current
  materialized state; and
- a revoked invariant blocks new work and exposes every unresolved dependent.

## Interfaces and projections

The database and content-object store are authoritative; people and actors
interact through typed commands and rebuildable projections.

### Required views

Initial projections include:

- an Operating Cycle monitor showing pinned configuration, admission generation,
  Grand Architect Office session, live children, WIP, cross-cutting budgets,
  cancellation/reconciliation state, notices, and closure blockers;
- an episode view containing the complete decision and outcome chain;
- a question landscape showing hypotheses, evidence coverage, and conflicts;
- a resource view showing current leases, WIP, congestion, and obligations;
- an integration view showing exactly why each product change is or is not
  ready;
- a propagation view showing lesson scope, delivery, dependents, contradictions,
  and revalidation;
- an organization-lineage view comparing configurations and trial outcomes;
- an actor ecology view showing niche demand, phenotype evidence, ancestry, and
  diversity coverage;
- a constitutional view showing the active Universe Seed, delegations, amendments,
  and reserved powers; and
- a Grand Architect review packet that links every summary claim to graph revisions and
  sealed evidence.

Markdown is an excellent human-or-actor projection. XSH may later become an
actor-side query or policy-proposal surface, but not the daemon protocol,
authority client, or durable truth. All projections carry the source event
cursor and can be discarded and rebuilt.

### Questions the system must answer

The architecture is successful only if ordinary typed queries can answer:

- What did the society believe and not know before this change?
- Which Operating Cycle and pinned policy frontier admitted each action, what
  is alive now, and what prevents quiescence, reconciliation, or closure?
- Which curated facts became OperationalNotices or influence, which were
  coalesced or suppressed, and what authorized action—if any—followed?
- Where is the Grand Architect Office session in its lifecycle, which decision
  turn is active, and how much context, time, and money remain?
- For a cancellation, which generation was fenced, which descendants received
  abort/TERM/KILL, which were reaped, and which evidence or cost is incomplete?
- Which independent evidence changed the decision?
- What dissent was preserved and what would vindicate it?
- Which predictions are due, failed, censored, or still unresolved?
- Why has this integration-ready change not shipped?
- Which active work relied on a lesson that was just contradicted?
- Which actor and organization configurations produced the evidence, at what
  cost and under what permissions?
- Where does a proposed organization variant improve, regress, or remain
  untested relative to its parent?
- Has an XSH feature improved later modification productivity outside the cases
  used to propose it?
- Which essential niche or independent lineage would a population change erase?

If these require transcript archaeology or manager memory, the institutional
memory has failed.

## RSI primitive coverage audit

[`../../RSI.md`](../../RSI.md) is a research conversation, not a requirements checklist, but its
innovative primitives are too central to leave implicit. This audit names the
architectural home, first executable evidence, and threshold beyond which V2
may claim the primitive works. “Represented in schema” is deliberately weaker
than “demonstrated.”

| RSI primitive | Durable V2 contract | First vertical evidence | Demonstrated only when |
| --- | --- | --- | --- |
| Purpose before recursion | `UniverseSeed`, active revision, canonical actor prompt, north-star fields, C4 lineage | VS bootstraps from one seed and every durable work object and attempt cites it | A descendant seed can be challenged, ratified by the Grand Architect, adopted, audited, and compared without rewriting ancestral purpose |
| Curated provenance | Content/evidence/account separation, semantic selection and exclusions, source escalation, curation lineage | C1 and C2 curated accounts preserve decisive evidence, unknowns, dissent, and excluded categories | Historical and later reversals show accounts retain decision-sufficient distinctions at lower retrieval cost than raw traces |
| Epistemic graph | Closed graph kinds, typed revisions and endpoint-checked relations, separate operational state | One complete objective-to-outcome causal episode with a preserved conflict | Ordinary queries reconstruct decisions, dependencies, dissent, and reopen consequences across many episodes without ontology escape hatches |
| Active provenance and bounded influence | `SignalFamilyRevision`, `DerivedSignal`, `InfluenceCandidate`, eligibility, quotas, effect lineage | Two scoped demand families and one cost or contradiction signal alter a bounded surface with an explanation | Influence improves decisions or attention under matched cost without high-volume capture, contamination, or hidden scalar ranking |
| Checked propagation | Lesson scope/status, carrier transformations, target ladder, uptake observation, symmetric retraction | One lesson is delivered, encountered, and applied once; no causal claim is made | Matched or randomized evidence supports behavior change, irrelevant scopes remain uncontaminated, and contradiction reopens every dependent carrier |
| Backpressure and resource accounting | Integer cost, hierarchical and cross-cutting envelope constraints, leases, WIP, queue projections, upstream pressure, circuit breakers | Aggregate, Operating Cycle, Office-session, Project, and per-attempt hard caps; unknown-cost stop; no-action validity; durable outcome capacity | Sustained work avoids unbounded evidence/integration/follow-up queues and allocation changes are attributable to real congestion |
| Multidimensional evaluation | Charter values, hard gates, typed uncertainty, partial orders, Pareto archive, decision process | Behavior, product, discovery, agent-fluency, cost, and propagation evidence remain separate | Later outcomes calibrate tradeoffs and preserve non-dominated alternatives better than a scalar baseline |
| Reproducible execution | Pinned snapshot, assignment, model policy, profile, Node/adapter/Pi-SDK dependency identities, Pi session, evaluator, content digests, retry lineage | Native Pi-SDK and deterministic runs replay from exact inputs; hindsight frontier is enforced | Independent re-execution reproduces relevant observations and organizational comparisons across environment upgrades |
| Organizational polymorphism | Versioned circuits selected by problem class; Projects and Tickets are generic coordination, not one forced workflow | VS instantiates a semantic/inquiry-product circuit with two decision gates | Multiple circuit families show different non-dominated scopes under matched cases and resources |
| Organizational heredity | Exact organization and actor configurations on attempts, decisions, products, outcomes, and influence | VS records the full configuration and produces one replayable decision world | Descendant performance can be attributed to named differences, ancestry is preserved, and failed/non-dominated branches remain reproducible |
| Meta-experimentation | C3 hypotheses, baseline, disclosure frontier, replay/shadow/canary ladder, independent promotion | The historical seed proves outcome-sequestered replay mechanics, not an organization win | A canaried organizational mutation improves held-out or later work and survives rollback and replication criteria |
| Trusted substrate | Resident Rust authority, typed local protocol, normalized SQLite, ledger, capabilities, Operating Cycles, per-session Pi SDK-host/process ownership, cancellation, tracing, budget physics, content identity, state machines, Git lineage | Kernel tests and VS execute without XSH in the trusted path, Pi CLI modes, JSON workflow state, direct SQL mutation, or unowned SDK hosts | Audit reconstructs state; fault injection defeats forgery, stale writes/spawns, authority escalation, overspend, orphan processes, cancellation loss, contamination, and history editing |
| Governance as evaluation process | Grand Architect Office, Decision packets, typed dissent/challenge, C0–C5 classes, revisit triggers | C1 and C2 decisions answer independent challenges and retain no-change paths | Calibration and later outcomes improve without consensus erasure or a hidden actor-species veto |
| Directed intelligence plus evolutionary search | Deliberative trunks allocate bounded speculative branches; branch width is resource policy | VS uses independent inquiry and paired candidates but makes no selection-system claim | Deterministic domains benefit from broader tournaments while ambiguous domains benefit from deliberation under comparable total cost |
| Developmental attractors and local niches | Attractor biases, demand signals, observed phenotype evidence, profession-recognition lifecycle | Initial labels are preregistered treatment biases only | A phenotype transfers across tasks, answers stable demand cheaply, and can be recognized or dissolved without hard-coded title authority |
| Diversity and minority preservation | Ancestry-aware independence, lineage and behavior reserves, preserved conflicts, portfolio exploration | Skeptic/adversary branches and minority arguments remain visible through C2 | Diversity prevents a later correlated failure at acceptable coordination cost and does not reduce to cloned prompt labels |
| Scarcity-shaped corporate intelligence | Grand Architect, Projects, Tickets, pulses, reviews, postmortems, portfolio envelopes | VS runs as one budgeted Project with durable tickets and a zero-cost pulse | The structure improves coherent throughput and recovery relative to a simpler baseline without bureaucratic model-call overhead |
| Continuous operation with finite evidence | Continuously running `societyd`; bounded Operating Cycles pin configuration, budget, Office session, cancellation root and reconciliation | VS runs one fully reconciled cycle while its delayed outcome survives closure | Successive cycles roll without idle stop/start or historical/configuration blur, and Projects safely span their frontiers |
| Historical replay and epistemic disclosure | Immutable positive frontier, contamination probes, `DecisionWorld` export | C1 world excludes candidate, aftermath, lessons, and current source | Organizational variants can be compared on stratified histories without direct or indirect hindsight leakage |
| XSH/product/society co-evolution | XSH product boundary, user-space co-evolution ladder, improvement-productivity experiments | VS ships an XSH invariant and uses XSH only for bounded actor-facing experiments, workloads, or evaluators | An XSH revision improves held-out society modification capacity under fixed and adaptive organization comparisons without entering trusted kernel physics |
| Discovery/propagation/metamorphosis rates | Separate vectors, obligations, congestion signals, and strict institutional-improvement criterion | VS measures discovery, delivery, and `applied_once`; metamorphosis remains explicitly unproven | A warranted institutional lesson changes later machinery and improves later work under a comparable envelope |

The philosophical principles are also guarded explicitly:

- **evaluation is a process, not a function:** Decision and review institutions
  own judgment under uncertainty;
- **preserve disagreement:** Conflict, dissent, competing causal claims, and
  accepted risk remain durable;
- **optimize information gain as well as immediate product movement:** honest
  no-action and failed experiments can resolve valuable uncertainty;
- **intelligence is cumulative institutional memory:** promotion requires
  provenance, carrier, uptake, and outcome rather than transcript storage;
- **scarcity creates structure:** money, tokens, context, evaluation,
  integration, and follow-up are enforced constraints;
- **directed reasoning and search are complementary:** deliberation chooses
  valuable search regions and deterministic judges support wider leaves;
- **organizational structure is mutable:** configurations and influence policy
  are testable R2 artifacts; and
- **recursion requires a trusted boundary:** the system may improve its
  representation and governance without editing the history used to judge it.

This audit is maintained with architecture changes. A primitive is not removed
by omission; removing or merging one requires an explicit architectural
decision that states which invariant and experiment replace it.

## Physical implementation map

The architecture should remain logically modular even though the first
deployment is one resident daemon, one control client, per-session TypeScript
Pi SDK hosts, and one database.

### Rust kernel

The first Rust workspace should expose narrow components for:

```text
protocol        closed commands, receipts, errors, version negotiation
wire            length-prefixed local socket frames and exhaustive codecs
daemon          societyd startup, control/monitor sockets and recovery mode
identity        principals, configurations, revisions, sessions, episodes
purpose         Universe Seed revisions, sources, renderings and bootstrap
authority       capabilities, jurisdiction, delegation, expiry
offices         office contracts, occupancy, succession and reserved powers
graph           node/edge contracts and revision validation
workflow        operational state machines and readiness facts
cycles          Operating Cycle admission, rollover and reconciliation
corporate       Projects, Tickets, milestones and coordination pulses
ledger          events, generations, idempotency, replay audit
content_objects content addressing, sealing, access and retention
evidence        semantic admission, capture method and claim relations
curation        accounts, exclusions, challenges and disclosure frontiers
influence       signal families, derivation, eligibility, quotas and effects
review          adversarial reviews, challenge dispositions and postmortems
resources       integer budgets, reservations, leases, cancellation, accounting
supervision     child registry, process groups, SDK-host handshake and reaping
pi              SDK-host protocol, sessions, events, usage, cost and submissions
cancellation    generation fences, propagation, TERM/KILL and recovery
observability   tracing spans/levels, notices, coalescing and monitor streams
execution       typed profiles, attempt lineage, workspaces and receipts
ecology         derived demand signals, response bids and diversity facts
propagation     target, delivery, encounter, application and retraction state
projections     cursors, outbox, rebuild contracts
repository      Git/worktree identity and product-delivery receipts
```

Module boundaries may change; these ownership boundaries may not disappear into
one generic job table. The SQLite schema and protocol versions are reviewed
contracts from the first vertical slice. During prototype development the
repository owns one canonical fresh-schema bootstrap and deliberately carries
no historical schema upgrade path.

### TypeScript Pi SDK host

The V2 repository contains a small `packages/society-pi-host/` package with a
locked dependency on `@earendil-works/pi-coding-agent` 0.83.0. Its source owns
only explicit SDK construction, empty resource loading, the closed JSONL
adapter, exhaustive event conversion, submission-file validation handoff, and
clean disposal. It has no database library, daemon client, Git delivery logic,
organization scheduler, or authority types. Rust integration tests spawn its
compiled entry point against provider-free model/session doubles; separate
qualification tests exercise the real pinned SDK.

### XSH surface

XSH is an optional R2 client and experimental medium, not part of trusted
bootstrap. Initial XSH packages may provide:

```text
society.query        read-only typed queries and projection rendering
society.propose      untrusted proposal construction submitted to societyd
society.experiment   actor-owned experiment and workload programs
society.evaluate     candidate deterministic/sampled evaluators as children
society.observe      candidate ecosystem and delayed-outcome probes
society.actor_tools  evolving tools used inside declared actor workspaces
```

These use typed paths, structured data, explicit `Result` errors, and stable
content identifiers. They do not open SQLite, spawn Pi, own a process registry,
interpret capabilities, apply product commits, or decide a lifecycle
transition. `societyd` supervises them as untrusted deterministic or actor
children. If XSH cannot express one clearly, Rust remains the explicit host
boundary; trusted physics is not weakened to advance a self-hosting story.

### One resident authority, many children

`societyd` is the only SQLite writer, process supervisor, and content-store
committer. `societyctl` and monitors communicate through local Unix-domain
sockets. Agent and deterministic children are horizontally parallel outside
the daemon. They execute in owned native directories or worktrees and return
closed results through kernel-owned pipes/files; the daemon attributes and
submits those results on their behalf. No child receives a reusable control
credential merely because it has `bash`.

The service can use ordinary threads and bounded channels initially. Selecting
an async runtime, SQLite crate, or extra tracing writer is a dependency decision
to present separately; this architecture authorizes `tracing` and
`tracing-subscriber`, not an undeclared framework.

If scale later requires another store, the migration must preserve command,
ledger, ordering, lease, and replay semantics. Distribution is not itself an
institutional intelligence gain.

## Bootstrap program

The society should be built in stages which each close a meaningful evidence
loop. Later-stage vocabulary may exist in the schema before autonomous machinery
uses it.

### Stage 0: install the origin contract

Create the society identity, install the founding Universe Seed through its
one-time consumed bootstrap capability, install and occupy the Grand Architect
office, then write the node and relation
schemas, command protocol, change classes, capability lattice, episode
transitions, evidence-admission and curation contracts, disclosure-frontier
rules, and V1 import contract. Define which clauses are R0, R1, or R2 before
implementation makes the answer expensive.

Exit evidence:

- schema and transition examples cover a successful, failed, contested, and
  reopened episode;
- every mutable surface has a named promotion authority; and
- every external invariant has a deterministic owner.

### Stage 1: build trusted physics

Implement resident `societyd`, `societyctl`, TypeScript `society-pi-host`, SQLite
migrations, the binary local protocol, content-object store, ledger, authority,
Operating Cycles, budgets, Pi SDK-host supervision, tracing and monitor
notices, hierarchical cancellation, execution/worktree receipts, projections,
recovery, and replay audit.
Use deterministic transaction, state-machine, concurrency, recovery, and
fault-injection tests. No paid model work is necessary.

Exit evidence:

- an interrupted attempt recovers without duplicated work or lost evidence;
- forged identity, stale generation, authority escalation, content mismatch,
  and resource overrun are rejected;
- INFO-and-higher progress streams with exact IDs while raw Pi content and
  secrets remain outside the log surface;
- quiesce, graceful cancel, emergency stop, daemon crash before/after
  `CreateSession`,
  ignored TERM, orphan detection, partial sealing, and restart reconciliation
  pass with process doubles;
- an agent Grand Architect SDK-host double receives bounded notices, returns a typed
  decision, and can be aborted without receiving control credentials;
- all projections rebuild from authoritative data; and
- an independent audit reconstructs the materialized state from the ledger.

### Stage 2: run one complete causal episode

Execute the contract in [`VERTICAL-SLICE.md`](VERTICAL-SLICE.md). Its seed
question is:

> At current XSH `HEAD`, what is the intended and actual stderr-redirection
> contract of a typed `Command` when consumed by managed `spawn` and
> `process.spawn`, and what smallest coherent XSH change reconciles runtime,
> specification, API discovery, tests, `LANG.md`, and demonstrated downstream
> need?

The episode must include:

1. one bounded Operating Cycle inside a continuously running daemon, with an
   aggregate budget, tracing/monitor stream, Grand Architect Office session,
   cancellation root, and reconciliation;
2. a scoped objective and resolution condition;
3. three competing hypotheses: missing behavior, culturally stale records, and
   split or accidental behavior;
4. curated V1 usage as historical evidence without importing its controller;
5. an explicit curated account which selects consequential evidence, preserves
   dissent, and states exclusions;
6. a deterministic behavior/documentation matrix plus a paired native-Pi
   baseline/candidate task microprobe;
7. a preserved conflict if the evidence is underdetermined;
8. a decision packet with no-change option, predictions, dissent, and revisit
   triggers;
9. one bounded XSH reconciliation commit if authorized;
10. short- and delayed-horizon outcomes;
11. one L1 lesson delivered to, encountered by, and applied once in a fresh
    matching inquiry, without overclaiming causal behavior change;
12. an epistemic disclosure frontier which exports the episode as a future
    blinded decision world; and
13. a retrospective on the XSH decision, curation, circuit, monitoring cost,
    cancellation readiness, and Operating Cycle used.

Exit evidence:

- a new actor can reconstruct the decision without raw transcript archaeology;
- replay reproduces deterministic observations from sealed inputs;
- contradictory test evidence can reopen the episode;
- the complete episode is queryable by configuration, cost, prediction,
  curation, knowledge-uptake state, and product lineage;
- the exact validated commit reaches the local XSH target only after explicit
  Grand Architect authorization; and
- the first measurements distinguish discovery, delivery, propagation uptake,
  and the intentionally unproven metamorphosis rate.

### Stage 3: establish product metabolism

Run the Repair and Empirical Inquiry circuits concurrently under explicit
stewardship and research portfolio envelopes. Demonstrate that ready product
work flows continuously without forcing uncertain research into tickets.

Exit evidence:

- several independently valuable XSH changes ship and survive their first
  outcome windows;
- delivery dwell and bottlenecks are attributable; and
- zero admitted weak proposals exist solely to maintain utilization.

### Stage 4: establish cumulative culture

Promote a supported lesson through L0-L2, transduce it through at least two
carriers such as context plus evaluator or policy, measure delivery,
encounter, application, and a matched behavioral effect, then introduce
contradictory evidence and exercise downgrade or retraction.

Exit evidence:

- relevant active work receives and encounters the correct revision;
- a discriminating comparison supports or defeats the claimed behavior change;
- irrelevant work is sampled for contamination; and
- dependents and derived carriers reopen, downgrade, or acknowledge the
  retraction without history edits.

### Stage 5: compare organizational circuits

Create a small stratified corpus of historical and held-out XSH decision worlds
with tested disclosure frontiers. Compare at least two circuits under matched
capability and resource envelopes, with outcome sequestration, contamination
detection, and an explicit Pareto analysis.

Exit evidence:

- a variant has a demonstrated scope rather than a universal win claim;
- no replay actor can retrieve aftermath through source, culture, identifiers,
  or raw-artifact access;
- the organization archive retains non-dominated and behaviorally distinct
  configurations; and
- the trial can be rerun from its configuration and evidence manifests.

### Stage 6: permit bounded metamorphosis

Promote one organization mutation into a narrow canary jurisdiction, observe
later work, and either retain or roll it back. Early stages may derive bounded
local demand signals and instantiate seeded attractor treatments; this stage
begins selection, differentiation, recombination, and profession-recognition
experiments only after attribution is credible enough to tell developmental
change from noise.

Exit evidence:

- the mutation satisfies the strict RSI criterion or is honestly reported as a
  failed organizational hypothesis;
- population diversity and reserved authority survive the trial; and
- a proposed profession either demonstrates a transferable niche and interface
  or remains an episode-local phenotype label.

### Stage 7: measure XSH-society co-evolution

Choose an XSH feature directly relevant to experiments or policy expression.
Compare ancestral and descendant XSH revisions on held-out society work under a
matched envelope.

Exit evidence:

- local product tradeoffs and institutional productivity effects are both
  visible;
- fixed-organization and bounded-adaptation results remain separate, including
  any new circuit or phenotype enabled by the candidate language world; and
- the result can revise either the language feature or the institution which
  selected it.

### Stage 8: transfer and exercise the Grand Architect office

Replay a complete succession from a user occupant to an assigned coding-agent
occupant, then let that occupant exercise C2 and one bounded C3 decision under
the same constitutional contract, cost ceiling, challenge requirements, and
rollback physics. Transfer the office back or to a successor without losing
open decisions, dissent, budgets, or outcome obligations.

Exit evidence:

- actor species does not change command legality or durable decision shape;
- the agent occupant can direct useful work without an ambient human
  ratification gate;
- every external host intervention remains visible and excluded from claims of
  autonomous performance; and
- succession, pause, revocation, and recovery are independently replayable.

## V1 migration

V1 remains frozen and runnable. V2 imports a deliberately selected evidence
corpus, not controllers or live workflow state.

Each imported episode records:

- immutable source paths and content hashes;
- the V1 run, evaluator, worker, report, ticket, replay, and product commits
  which actually exist;
- the curator and mapping from selected V1 material into V2 forensic objects,
  evidence admissions, and graph revisions;
- which hypotheses, alternatives, predictions, dissent, or delayed outcomes
  were never captured; and
- why the episode was selected and what future comparisons may use it for.

V1 ticket status and throughput qualification are historical observations. They
never become V2 readiness or authority. Trusted mechanisms may be reimplemented
or ported behind direct V2 contract tests; no compatibility controller imports
V1 ontology by accident.

## Anti-patterns

V2 rejects:

- a universal goal loop over `score(candidate)`;
- a fixed CEO/planner/coder/reviewer hierarchy presented as final architecture;
- raw transcripts as memory or summaries as evidence replacements;
- cheap storage and telemetry fields presented as curated provenance merely
  because they are easy to collect;
- tickets, files, or queues as the world model;
- one generic node or job table with meaning hidden in JSON;
- confidence numbers without evidence type and reference class;
- consensus which deletes dissent or manager prose which resolves it silently;
- actor cloning counted as independent review;
- prompt mutation presented as RSI without held-out institutional evidence;
- organization trials whose descendant sees historical outcomes or current
  lessons from the test cases;
- contribution scores used as currency, status, or truth;
- propagation measured by message delivery rather than relevant behavior;
- lessons with no retraction path or outcomes with no follow-up owner;
- product throughput maintained by weak proposals or research value inferred
  from commit count;
- XSH self-use used to evade its systems-glue scope or the Rust durability
  boundary;
- an XSH Pi runner, SQLite client, process reaper, cancellation script, or Git
  materializer placed on the trusted path for aesthetic self-hosting;
- an unbounded “continuous run” with no pinned Operating Cycle, finite budget,
  cancellation root, reconciliation, or successor frontier;
- OS signals, PID files, log messages, or process-table scraping treated as the
  cancellation state machine;
- tracing treated as a ledger, provenance corpus, scheduler input, or raw
  context stream to the Grand Architect;
- a mutable evaluator certifying itself, a circuit granting itself authority,
  or a seed amendment rewriting the purpose attributed to ancestral work; and
- claims of autonomous improvement that omit external interventions, failed
  descendants, cost, or changed external conditions.

## Open research program

Some architecture must remain hypothesis rather than doctrine. Early episodes
should investigate:

1. What is the smallest graph vocabulary that preserves causal utility without
   becoming an unqueryable ontology project?
2. Which trace compression retains facts that later reverse an XSH decision?
3. How should qualitative confidence and numeric calibration coexist without a
   fake common scale?
4. Which XSH problem classes genuinely benefit from competing hypotheses and
   adversarial circuits, and which only acquire ceremony?
5. How can information gain be compared well enough for admission without
   becoming another gameable scalar?
6. What ancestry and behavioral tests establish meaningful evaluator
   independence among model actors?
7. Which local demand signals reduce coordination cost, and which produce
   herding or neglected maintenance?
8. When should a repeated phenotype become a recognized profession, and how can
   that profession later dissolve?
9. What propagation precision, recall, latency, and behavior-change measures
   are practical at small scale?
10. How much historical replay predicts live organizational quality after model
    and XSH revisions change?
11. Which delayed outcomes are valuable enough to reserve capacity for, and how
    should censored outcomes affect promotion?
12. Which XSH constructs measurably improve agent modification and review
    reliability while remaining excellent systems-glue design for humans?
13. How should the society value long-term optionality without allowing vague
    future claims to dominate present evidence?
14. What evidence and challenge surface lets an agent occupy the Grand
    Architect office autonomously without fragmenting authority or concealing
    catastrophic error?
15. Which semantic distinctions deserve curated retention, and which cheap
    metadata merely creates the appearance of provenance richness?
16. How can excluded information and curator uncertainty be represented without
    turning every account back into a raw-trace swamp?
17. What disclosure-frontier tests detect indirect hindsight leakage through
    current source, identifiers, cultural lessons, or model familiarity?
18. Which developmental attractor axes transfer across XSH problem classes, and
    which are disguised job prompts with no stable phenotype evidence?
19. When does formalizing a phenotype as a profession reduce coordination cost,
    and when does it create authority-seeking institutional ossification?
20. Which knowledge is best inherited as context, professional method,
    evaluator, policy, XSH construct, or several linked carriers?
21. How should homeostatic policy respond when discovery, warranted propagation,
    delivery, and metamorphosis are constrained by different scarce resources?

The society should ingest its own answers as scoped lessons, not quietly bake
the first plausible answer into infrastructure.

## The complete invariant

The architecture can be summarized as follows:

> Bootstrap a constitutional machine society from one Universe Seed; let a
> holder-agnostic Grand Architect direct replaceable actors through a shared,
> typed epistemic commons; let institutions turn disagreement and scarce
> resources into reproducible decisions and shipped artifacts; let checked
> culture carry warranted discoveries into future behavior; and let product,
> organization, influence policy, and even constitutional purpose evolve
> without changing the trusted history that makes those changes intelligible.

If V2 can do that at small scale, it is already more than a higher-throughput
software factory. It is a contained experiment in cumulative machine culture,
with XSH simultaneously serving as its public work, its evolving systems
language, and an increasingly capable medium for the society's own thought in
action.
