# Generic society architecture

## Status and boundary

This document defines the generic apparatus for a durable, bounded society of
actors and institutions. It is deliberately independent of any named product.
An application lives beneath `applications/<product>/`, provides its own
mission, evaluators, product rules, and executable slice, and depends only on
the generic public boundaries.

The apparatus is not a general-purpose workflow engine. It is a narrow trusted
substrate for identity, authority, state transitions, evidence, content,
budgets, owned processes, cancellation, replay, and guarded product delivery.
It does not decide whether an application claim is true or whether a product
change is desirable.

The intended dependency direction is one-way:

```text
applications/<product>  ->  generic public Rust crates
generic Rust crates     -X-> applications/<product>
```

The generic workspace must not load application crates, identify applications
through strings, or persist an application payload. Application identity and
revision are typed identifiers; application meaning remains in typed fields,
sealed content, and application-owned contracts.

## Constitutional input

One active application mission revision gives a society its purpose. A mission
is not ambient README text and is not a prompt-only convention. It is a typed,
revisioned input. The apparatus records its
identity, authority, revision, activation basis, and lineage without
interpreting the product-specific meaning.

```text
ApplicationMissionInput
  ApplicationIdentity
  ApplicationRevision
  MissionStatement
  ordered MissionPrinciple relation
  NorthStarQuestionSet
  source rendering BLAKE3 digest

NorthStarQuestionSet
  Change | ImprovementEvidence | BoundaryCommitment | Revisit
```

Every consequential project, ticket, decision, review, retrospective, and
postmortem cites its exact mission revision. Its `ProjectNorthStarAlignment`
answers the four named questions through separate typed fields. Mission gives
purpose; north-star alignment tests a particular proposed action. Neither is a
second authority and neither is an extensible document-shaped field.

The current fresh schema normalizes this mission and requires every Project to
persist the exact four-field alignment against its founding-mission revision.
The application returns bounded `MissionSourceRendering` bytes separately from
its digest-only `ApplicationMissionInput`; it does not know a `ContentObjectId`.
Before founding installation, the resident's private source path checks the
bytes against that digest, preflights the outer command without mutation,
physically seals them, records the existing seal receipt and global object,
then issues `InstallFoundingMission`. The kernel atomically resolves the digest
to that registered object and persists the derived private binding. Its
deterministic internal operation identities make the content primitive
retry-stable while that authority is retained; they do not prove a supervisor
request can continue after handler failure. A handler failure ends the request,
and a nonempty restart is `RecoveryFenced` rather than source-operation
recovery. The supervisor carries `MissionSourceRendering` only with that
mission command; no public or supervisor generic content mutation or
content-writer authority exists. This establishes byte identity and custody only, not producer
provenance, semantic/evidence admission, or a complete application execution.
The same owned store can stream one object through an explicit read bound while
rehashing the same no-follow file handle. A successful read receipt proves only
the copied digest and byte count; the caller discards every failed destination,
and executable qualification or semantic role remains a separate authority.

## Trusted substrate

The resident Rust authority owns one SQLite ledger, one content-store custody
boundary, the process registry, and the durable mutation protocol. Clients,
actors, application evaluators, and projections do not write arbitrary SQL,
construct lifecycle state from files, or supervise authority-bearing children.

Each durable command is a closed Rust enum variant with named fields:

```text
Command
  -> authenticate principal and capability
  -> validate jurisdiction, references, generation, and budget
  -> append an immutable event and update named materialized relations
  -> write projection/outbox facts
  -> return a typed receipt or a typed rejection
```

The local control protocol is binary, length-prefixed, and tag-discriminated.
JSON is reserved for the separately versioned SDK-host boundary. SQLite must
not acquire generic payloads, metadata maps, EAV tables, or stringly typed
discriminants.

## Authority, work, and physical custody

Capability plus jurisdiction, current office occupancy, expiry, and exact
state determine authority. Prompts, titles, source repositories, and tool
access do not grant it. Every admitted actor attempt belongs to one bounded
Operating Cycle with a budget reservation, an admission generation, a
cancellation root, and a named execution profile.

The current fresh-schema prototype preserves one `RootAuthorityOffice` during
bootstrap. Its active occupancy receives the closed root-authority capability
bundle, and its `RootAuthorityOfficeSession` provides the bounded resident
control session for an Operating Cycle. `FoundingMission` is the single
installed application mission revision from which that bootstrap derives its
generic cross-links. These are root-governance mechanics, not an
application-owned office title, constitution, or prompt contract.

The daemon owns child-process groups, direct-child reaping, control pipes, and
the final recheck immediately before a paid session can be constructed.
Cancellation first closes admission, then expands durable obligations, signals
owned children, seals partial evidence, reconciles cost, and proves closure.
An OS signal, a process exit, a direct-child reap, and protocol completion are
distinct facts.

One generic `NativeChild` custody nucleus owns PID/PGID identity, liveness
probes, signal races, direct wait, and bounded stream capture. Pi execution is
an optional strict session/protocol sidecar over that base; evaluator-owned
children have no Pi session, nonce, Create/Abort grammar, or Pi stream
requirements. The only current evaluator process treatment is a verified
direct executable with one verified input-manifest argument and an empty
environment. It rejects shebang scripts and supplies no interpreter or shell
launcher, retains bounded stdout/stderr, and permits sealing only after direct
reap and owned-group absence.

The kernel's daemon-facing scheduler claim treats an existing registered
deterministic experiment as the sole scheduling authorization, chooses the
oldest eligible experiment, and atomically derives its exact
evaluator/input/workspace admission. The daemon-private coordinator computes a
prospective workspace without mutating the filesystem, claims first, and
allocates and materializes the fixed evaluator/input paths only for an accepted
claim. It records spawn, signals, direct reap, any later owned-group liveness,
and both complete stream seals. Only after child finalization may a constrained
transition derive the forensic output from that child's stdout seal while also
requiring complete stderr; callers cannot substitute an unrelated global
object. Provider-free integration covers the ordinary and lingering-descendant
paths. The resident serving loop still has no call site for this coordinator,
and there is no public execution command, post-restart reconciliation,
workspace disposal, semantic evidence admission, or application execution
claim.

Office Prompt completion also has two deliberately distinct closed sequence
shapes. Observed assistant results require `AgentSettled`, a later exact final
accounting fact, and the immediately following `Settled`. SDK-level failures
may occur before any agent lifecycle; they require only a final
Prompt-correlated Known usage fact immediately followed by `Settled`. The
kernel never fabricates an agent event to make those shapes look alike, and a
non-ready or protocol-failed terminal cannot reopen Office authority.

Office-session disposal has its own durable closing grammar:
`Authorize-before-write -> delivered -> accepted -> final Known/failure ->
Disposed`. The authorization freezes the exact Ready session, child, Pi
session, correlation, and generation before a Dispose byte enters the host
pipe; delivery records only a complete physical JSONL write. After host
acceptance, the very next sequence is one final cumulative Known usage fact or
a typed accounting failure. Only Known usage may be followed by the next
`Disposed` receipt. That receipt binds the same correlation and a peer-checked
transcript flush. For a materialized transcript, daemon custody verifies the
owned session path and exact digest, reads the bounded regular file without
following its final path component, and sends its bytes to the daemon's sole
content writer; the terminal receipt requires that resulting content identity
and digest. A no-Prompt session may still have a materialized transcript with
an explicitly absent first prompt, which is custody-sealed normally; only the
lazy missing-file arm is unmaterialized and has no content object. Neither arm
may invent a first prompt or content where it is absent.

The final Known `Disposed` transition reconciles the single existing
Office-session parent reservation: it charges any final delta and releases its
unused reserve, or freezes a known overrun with its cancellation/postmortem
duty. A final accounting failure freezes the parent and starts containment; it
has no `Disposed` successor and awaits a later recovery tranche. Session
disposal neither proves process exit nor reaps the child—direct-child reap is
separate OS-custody evidence. This same-lifetime foundation has no resident
scheduler/control-loop caller, post-restart recovery, workspace disposal,
semantic submission, paid/native qualification, or end-to-end application
claim.

An execution profile is a qualified runtime identity and readiness state, not
a caller-selected executable path. A product application may select only an
already admitted profile through typed application policy; it cannot alter the
generic process or budget physics by changing a prompt.

## Evidence and content

The apparatus preserves three distinct records:

| Record | Meaning | Rule |
| --- | --- | --- |
| Event ledger | Accepted commands and trusted transitions | Append-only and replay-auditable |
| Content object | Immutable sealed bytes | Identity only; no provenance claim by itself |
| Epistemic record | Typed admitted observation, argument, decision, or lesson | Revisioned and challengeable |

Physical sealing proves byte identity. Evidence admission gives sealed bytes a
typed role under a named evaluator and scope. Curation selects a
decision-relevant account while retaining exclusions and challenge paths.
Delivery, encounter, application, and causal support remain separate facts.

## Budgets and operating cycles

Budgets are hard integer ceilings, never spend targets. A reservation charges
every applicable envelope before paid work begins. Known costs reconcile
exactly; unknown or unavailable costs freeze later paid admission under the
named policy and cannot silently become zero.

An Operating Cycle is finite even while the daemon remains resident. It pins
configuration, mission revision, budget, cancellation root, and admission
generation. Quiescence closes new admission; cancellation controls owned work;
closure follows only after reconciliation. A successor cycle is new typed
state, not a relabeling of an old cycle.

## Product-change output

The apparatus can eventually issue an `AuthorizedProductChange` only after a
typed decision, required review, and product-specific evidence gates succeed.
The generic authorization binds one application revision, decision,
repository, admitted base, patch, expected tree, validation profile, and local
delivery target. Isolated materialization returns typed receipts and guarded
delivery performs an exact compare-and-swap; neither step creates authority.

```text
decision and review
  -> AuthorizedProductChange
  -> isolated exact-tree materialization
  -> typed validation receipt
  -> guarded local delivery receipt
  -> outcome obligation and replay
```

This durable authorization output is also not yet implemented by the current
prototype. Existing materialization mechanics are a provider-free local
boundary and must not be described as a completed generic delivery path.

## Application boundary

An application owns:

- mission prose, source corpus, alignment wording, and product-specific
  principles;
- evaluator and validation semantics, fixture corpora, and acceptance judges;
- application-facing prompts, actor assignments, and product conclusions; and
- any product-specific documentation, research history, and operating slice.

The generic apparatus owns only the typed interfaces and their enforcement.
It may record an application revision and sealed inputs, but it does not
interpret source language semantics, run an application evaluator as a trusted
decision, or import a product controller.

## Fresh-schema prototype rule

The current prototype uses one atomic fresh SQLite bootstrap. A schema change
must change the canonical bootstrap, schema identity, Rust command/event
codecs, replay validation, tests, and this contract together. Historical
prototype databases are rejected without mutation; there is no implied upgrade
or compatibility path. Runtime roots are operator-owned and must be replaced
deliberately when a new fresh schema is introduced.

## Invariants

- No application child writes durable state, controls a process group, or
  moves a delivery ref directly.
- No application semantics cross the generic boundary through an opaque JSON
  document, metadata map, EAV relation, or string discriminator.
- No stale admission generation, unqualified profile, budget overrun, or
  missing cancellation/reap fact can become a successful transition.
- No sealed object is automatically evidence, memory, authority, or product
  authorization.
- No delivery succeeds when authorization, repository, base, patch, tree,
  validation profile, or target differs from the exact admitted value.
- Every accepted transition has named authoritative rows and replay evidence.
