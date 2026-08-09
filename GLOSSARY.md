# XSH Society canonical glossary

## Status and use

This file is the canonical vocabulary for Factory V2.
[`ARCHITECTURE.md`](ARCHITECTURE.md) defines how these concepts compose;
[`VERTICAL-SLICE.md`](VERTICAL-SLICE.md) defines the first executable proof.
Rust types, SQLite tables, XSH commands, tests, projections, and prompts use the
spellings here. A synonym in prose does not create a new domain concept.

Terms are intentionally narrower than their ordinary-language meanings. When
a proposed implementation cannot name which term it realizes, the contract is
not yet clear enough to persist.

## Terms

### Actor attempt

One supervised execution of an actor instance against a sealed assignment,
context pack, execution profile, and budget reservation. A retry is a new
attempt with lineage; it never overwrites the failed attempt. Every attempt
belongs to exactly one Operating Cycle.

### Actor configuration

A versioned policy for constructing bounded cognitive work: model policy,
system instructions, tool profile, context policy, developmental attractors,
resource envelope, and lineage. It is not a job title or a persistent persona.

### Actor instance

A principal admitted from one exact actor configuration for a bounded scope and
lifetime. An actor instance receives authority only through explicit capability
grants or through an office it is authorized to occupy.

### Execution profile

A closed, revisioned description of the trusted runtime boundary eligible to
execute an actor attempt. Runtime identity and readiness are separate: the
provider-free deterministic Pi-host double is eligible only in
`Vs001DeterministicV1`, while the pinned native Pi SDK profile remains
`Unqualified` until the bootstrap qualification institution records its typed
result. An execution profile is not a caller-supplied executable path or a
claim inferred from a successful prompt.

### Admission generation

A monotonically increasing fence on an admission scope. A spawn reservation
captures the current generation, and the Rust kernel rechecks it immediately
before permitting `society-pi-host` to construct a Pi `AgentSession`.
Quiescence or cancellation increments the generation transactionally so stale
readiness cannot become paid execution.

### Admitted evidence

A typed observation or sealed content object assigned a semantic role under a
named experiment, evaluator revision, scope, and admitting authority. Sealing
bytes proves identity; admission makes an evidence claim. Admission alone does
not make the claim true or decision-relevant.

### Adversarial review

A bounded institution for producing explicit failure hypotheses and
`ReviewChallenge` records against a named revision. It preserves disagreement
and tests evidence, product, cost, compatibility, security, or institutional
claims. It is neither free-form hostility nor a generic veto.

### Attention bid

A versioned, explainable request for a scarce consumer—such as a project,
review, context slot, or inquiry—to consider an eligible derived signal. A bid
can lose without the underlying evidence disappearing.

### Budget envelope and reservation

A `BudgetEnvelope` is a hard integer-micro-dollar ceiling over one society,
portfolio, Project, Operating Cycle, Office session/turn, Ticket, or
ActorAttempt scope. A `BudgetReservation` transactionally charges every
applicable constraint before paid work begins. Most constraints are
hierarchical, but Operating Cycle and Project constraints cross-cut the same
execution; neither is a fictitious parent of the other. Unknown cost is not
zero; unused reservation is not a target to spend.

### Cancellation request

A typed control-plane command over a named scope, represented as
`CancellationRequest`. It records mode (`Quiesce`, `GracefulCancel`, or
`EmergencyStop`), reason, authority or trusted breaker, admission generation,
grace deadline, propagation rule, and evidence-retention policy. Cancellation
is not a derived demand signal and cannot be created by signal pressure alone.

### Capability

A narrow durable permission to execute a named command over a named
jurisdiction. Capability, jurisdiction, expiry, and current office occupancy—not
titles in prompts—determine authority.

### Causal episode

A durable chain from objective and question through hypotheses, experiment,
observation, arguments, decision, action, outcome, retrospective, and lessons.
Its graph contains causal claims and decision provenance; it does not assert
that recorded edges mechanically prove causality.

### Change class

The governance class assigned to a mutation: C0 observation, C1 reversible
inquiry, C2 product mutation, C3 institutional mutation, C4 constitutional
amendment, or C5 trusted-kernel mutation. Higher class means stronger evidence,
challenge, rollback, and authority requirements—not necessarily more prose.

### Checked propagation

The process by which warranted knowledge changes carrier, reaches only a valid
scope, is encountered, is applied, is evaluated for effect, and can be
retracted. Delivery is not uptake; uptake is not causal support; causal support
is not institutionalization.

### Circuit

A versioned composition of institutions, actor configurations, transitions,
context edges, and judges for a problem class. A circuit is a reusable workflow
contract, not the sole ontology of the work it processes.

### Behavior observation set

The exact eleven-row B01-B11 deterministic spawn/stderr measurement decoded by
`society-circuit` into closed Rust types. It preserves parent-stream versus
redirected-artifact evidence and evaluator-observed lifecycle without claiming
that `owned_waited` proves daemon process-group reaping. Parsing the set is not
evidence admission; its TSV, evaluator, inputs, and every referenced digest
must first bind to separately sealed content under kernel authority.

### Constitutional inheritance

The exact `UniverseSeed` revision, constitutional rules, and Grand Architect
office contract inherited by a society run. This term is preferred over
“DNA” or “genome” when discussing durable purpose and authority.

### Content object

Immutable bytes sealed under a digest with a media or schema contract,
retention class, and capture provenance. Content objects are forensic material;
they do not become graph knowledge merely because they exist.

### Coordination pulse

A cheap, usually deterministic projection of changed work, blockers, decisions
due, budget burn and reserve, signal movement, outcome obligations, and next
actions. It provides the useful function of a standup without requiring a paid
meeting or treating narrative status as truth.

### Cost observation

A typed provider-accounting state: known integer micro-US-dollars with source
identity, unknown with a closed reason, or unavailable with a closed reason.
Unknown and unavailable states freeze paid admission under the initial policy;
they never silently coerce to zero.

### Curated account

A revisioned, purpose-specific representation selecting consequential admitted
evidence, arguments, unknowns, conflicts, transformations, and exclusions for a
named audience. It is a small default path into provenance, never a replacement
for its sources.

### Curation

The accountable semantic transformation from a larger evidence boundary into
a smaller representation for decision, replay, learning, or context. Curation
is judged by later sufficiency, reversal sensitivity, dissent fidelity, scope,
and escalation cost—not byte or token compression alone.

### Decision packet

The complete, revisioned input to a consequential Grand Architect choice:
authority, alternatives including no change, active Universe Seed values,
evidence and limits, arguments, dissent, unknowns, cost, reversibility, blast
radius, predictions, revisit/rollback triggers, and organization configuration.
It supports a decision; it does not mathematically force one.

### Decision world

A historical decision boundary exported under an epistemic disclosure
frontier. It contains exactly what was available then and excludes outcomes and
descendant knowledge, permitting honest replay of organizational alternatives.

### Demand signal

A rebuildable local indication that a named scope needs attention, such as an
unresolved contract conflict or missing behavioral evidence. It is evidence
for allocation, not an order and not epistemic truth.

### Derived signal

A typed, versioned projection from eligible provenance and current operational
state. It records its signal family, derivation policy, source lineage, scope,
uncertainty, decay, and current validity. Raw telemetry never becomes influence
without this derivation boundary.

### Developmental attractor

A heritable bias in how an actor notices, questions, experiments, checks, or
communicates. Its realized behavior depends on local demand, culture, tools,
peers, and resources. It is not a disguised corporate title.

### Disclosure frontier

An immutable positive allowlist of revisions, evidence, snapshots, culture,
and policy visible at a named historical decision boundary. Anything absent is
sequestered; contamination attempts are recorded.

### Epistemic graph

The typed, revisioned account of what is claimed, observed, disputed, decided,
and currently applicable. It differs from the immutable event ledger and the
content-object store.

### Event ledger

The append-only record of accepted commands, state transitions, resources,
authority, and responsible principals. It establishes operational history; it
is not by itself the society's world model.

### Final assistant outcome

The closed terminal model observation associated with the non-retried Pi
`agent_end`: `stop`, `length`, `error`, or `aborted`. It is carried separately
from SDK-promise resolution, `agent_settled`, host-process exit, submission
validity, and Attempt success. Missing or contradictory terminal evidence
makes the SDK-host session protocol-failed and unavailable for further spend.

### Forensic evidence

Sealed source material preserved so a later judge can support, defeat, or
reinterpret a claim. It remains available through controlled escalation even
when no curator selects it.

### Grand Architect

The display name of the highest constitutional office, represented in code as
`TheGrandArchitect`. Its current occupant may be the user or an assigned coding
agent. The office can ratify and amend the Universe Seed, charter and terminate
projects, allocate resources within kernel ceilings, appoint subordinate
offices, authorize C2–C4 changes, accept risk, and require review or
postmortems. Mandatory challenge and provenance constrain the quality and
visibility of its choices; they do not smuggle in a human-only veto.

### Grand Architect brief

A bounded, rebuildable projection of the active Universe Seed, portfolio,
coordination pulse, decisions due, strongest eligible influence and dissent,
cost reserve, delivery blocks, and overdue outcomes. It is the office's default
attention surface, not a model-written source of truth.

### Grand Architect office session

One supervised Pi SDK-host process and canonical Pi session through which an actor
occupying `TheGrandArchitect` monitors and governs one Operating Cycle. It
receives bounded `OperationalNotice` batches, produces separately validated
decision submissions, has its own budget, and is sealed at cycle
reconciliation. A user occupant uses the equivalent typed control and monitor
interfaces without a paid Office session.

### Graph object

A durable semantic identity with typed revisions, such as a Question,
Hypothesis, Observation, Decision, Lesson, or ReviewChallenge. A `GraphObject`
and a `ContentObject` are different types even when ordinary prose calls both
artifacts.

### Influence

A recorded consequence of eligible, curated, scoped information: visibility,
matched retrieval, an attention bid, required review, reprioritization, or a
kernel-enforced block. Influence is allocated under bounded attention and must
remain explainable back to sources. It is not a global reputation score.

### Influence candidate

A derived signal that passed eligibility gates and requested one specific
effect on one target. It carries the comparison class, pressure inputs, policy
revision, expiry, and full source lineage. A separate `InfluenceDecision`
records whether and why it took effect.

### Institution

A durable protocol combining jurisdiction, admission, required records,
authority, state transitions, and exit conditions. Institutions outlive actor
instances and may be mutated through C3 evidence.

### Invariant

A mechanically enforceable rule with a named scope, owner, evaluator, evidence
lineage, and revocation path. An invariant is the strongest cultural carrier;
it must not be inferred from prose guidance alone.

### Lesson

A scoped, revisable claim that a discovery should alter future behavior. Its
promotion level and carrier are separate from its epistemic status, and it has
explicit contradiction, expiry, downgrade, and retraction paths.

### Mission

The purpose and worldview content inside the Universe Seed: why XSH exists,
who it serves, what domain it claims, what it preserves, what it rejects, and
what it refuses to optimize away.

### Model catalog policy

The non-secret, qualified identity of the exact Pi model catalog bytes and the
effective model treatment selected from them: provider, endpoint, API shape,
requested model, canonical provider slug, context and completion limits, input
kinds, and every present or absent billing rate. `ModelCatalogPolicyV1` binds
configuration to a SHA-256 digest and is checked before and after SDK model
resolution. It is not authentication material, an ambient `models.json`, or a
provider-discovery result.

### North-star alignment

The operational test derived from the mission. A project, ticket, decision,
review, or postmortem states how it affects XSH capability or actor behavior,
what evidence would distinguish general improvement from noise, how it honors
explicit boundaries and composability, and when the claim will be revisited.
The canonical questions live in the Universe Seed; each work object cites a
typed `NorthStarAlignment` containing its answers. Mission and north star are
not separate competing authorities.

### Office

A durable bundle of jurisdiction and capabilities that can have one explicitly
recorded occupant at a time. Office authority is independent of whether its
occupant is human or model-based. An office differs from an actor configuration
(how cognition is produced), a profession (a learned social interface), and a
project role (a bounded responsibility within one project).

### Operating Cycle

A bounded operational epoch inside the continuously running Rust kernel. It
pins Universe Seed, Grand Architect occupancy, organization/model/admission
policies, budget, WIP, start cursor, rollover conditions, and one cancellation
root. Actor attempts belong to exactly one Operating Cycle; Projects, causal
Episodes, lessons, and outcome obligations may span many. It is not a product
quota or one fixed workflow.

### Operating Cycle treatment

A closed admission policy selecting the purpose and exact hard ceiling of one
Operating Cycle. VS-001 has three non-interchangeable treatments:
`PiSdkQualificationV1` is the bootstrap/kernel-only native laboratory at
`UsdMicros(30_000)`; `Vs001DeterministicV1` is the provider-denied process-
double circuit at `UsdMicros(1_000_000)`; and `Vs001LiveV1` is the qualified
native run at `UsdMicros(1_000_000)`. Neither a deterministic success nor a
live-cycle label can create native qualification. A treatment is durable
authority, not a caller-provided label or arbitrary budget value.

### Work item

A typed assignment binding one Ticket, actor instance, context pack, work kind,
and—when applicable—the exact Adversarial Review. It may be claimed only by its
named active actor. A retry preserves the failed Attempt and reopens the same
assignment under explicit lineage.

### Work lease

The exclusive, revocable claim connecting one actor instance to one ready Work
Item. Expiry before an Attempt returns the Work Item to readiness; a live
Attempt requires separate cancellation and terminal reconciliation rather than
silently expiring its lease.

### Operational audit

Kernel- or supervisor-produced facts establishing which command, tool, process,
transaction, or cost event occurred. Audit data is continuously available to
deterministic projections and forensics, but does not directly gain epistemic
or organizational influence.

### Operational notice

A typed, bounded monitoring projection derived from committed lifecycle,
denial, anomaly, or recovery events. `OperationalNotice` is the input to the live Grand
Architect monitor and the human console. It carries stable identities and
severity but no authority; tracing text is a rendering of it, never parsed back
into durable state.

### Outcome obligation

A durable commitment to observe a prediction, delivery, lesson, or
institutional mutation at a named horizon. Closing immediate work does not
delete it.

### Pi boundary

The only V2 protocol boundary allowed to use JSON. Rust and the pinned
TypeScript `society-pi-host` exchange a closed JSONL control/event protocol;
Pi's SDK `SessionManager` writes its canonical JSONL transcript; bounded actors
emit one closed-schema `submission.json`. The Rust kernel seals and parses
those bytes into typed values before they can affect SQLite state. No Pi CLI
mode and no other durable V2 workflow uses JSON payloads, columns, manifests,
or projections.

### Pi boundary peer

The typed Rust state machine in `crates/society-pi/` that admits and seals exact
stdin/stdout records at the Pi boundary. It verifies the expected child PID,
spawn nonce, runtime and model-catalog identities, sequence and correlation,
one-prompt-at-a-time lifecycle, terminal event ordering, and monotonic
prompt-attributed usage before returning a closed observation to `societyd`.
It is deliberately not the durable supervisor: process ownership, SQLite
correlation, evidence-object sealing, budget charging, and reaping remain
`PiSupervisor` responsibilities.

### Pi SDK host

The V2-owned TypeScript executable `society-pi-host`, pinned with
`@earendil-works/pi-coding-agent` 0.83.0 and an exact dependency lock. One host
embeds one SDK `AgentSession`; task hosts are one-shot and the Grand Architect
host persists for one Operating Cycle. It is a trusted execution-evidence
adapter but has no database, capability, budget, scheduling, or Git authority.

### Pi transcript flush receipt

A closed host-boundary observation emitted only after Pi session disposal and
transcript verification. It distinguishes an intentionally unmaterialized
pre-prompt session from a canonical SessionManager JSONL file whose session
identity, real path, header cwd and timestamp, file digest, and first user
prompt have been checked. It proves what the host observed; Rust must still
seal and admit the bytes.

### Pi supervisor

The Rust `PiSupervisor` subsystem inside `societyd`. It exclusively reserves,
spawns, registers, observes, costs, cancels, reaps, and seals one-shot Pi task
processes and persistent Pi SDK Office sessions. It starts each
`society-pi-host` inert, records `AdapterReady`, rechecks admission, and only
then permits `CreateSession`. It is trusted physics, not an actor, Office, or
XSH program.

The landed native process-physics boundary currently implements only owned
workspace allocation, artifact verification, inert spawn, nonblocking control
delivery, bounded handshake observation, process-group cancellation/reaping,
and transient typed receipts. Its `admitted_control` capture is the logical
JSONL frame accepted by the Rust peer; its `stdin` capture is only the exact
byte prefix successfully written to the OS pipe. Neither capture is a sealed
`ContentObject`. Durable child registration, budget/cost reconciliation,
content sealing, evidence admission, restart recovery, and execution-profile
qualification remain separate kernel/daemon transitions.

### Portfolio

The Grand-Architect-governed set of stewardship, frontier, measurement,
institutional, and resilience obligations under explicit resource envelopes
and reserves. It is a partial-order allocation surface, not one scalar backlog.

### Postmortem

A triggered, structured investigation of an incident, cost breach, revert,
process escape, invariant failure, or repeated project miss. It preserves a
ledger-derived timeline, competing causal claims, containment, contributing
conditions, counterfactuals, and separately admitted corrective actions. It is
not a blame ritual and cannot silently enact its recommendations.

### Principal

An authenticated source of commands or office occupancy. User principals,
actor principals, deterministic-service principals, and bootstrap principals
obey the same capability and jurisdiction machinery.

### Profession

A learned, durable social interface recognized only after a repeated phenotype
serves a stable niche with a predictable contract at acceptable coordination
cost. A profession may dissolve when demand disappears. It is not predeclared
by giving an actor a familiar title.

### Project

A durable portfolio container chartered by the Grand Architect for a coherent
objective, risk and resource envelope, milestones, stop conditions, outcome
obligations, episodes, and tickets. A project coordinates work; it is not a
claim about reality.

### Project steward

A principal or subordinate Office delegated to plan and coordinate one Project
within its charter and resource envelope. Stewardship does not imply authority
to amend the Universe Seed, enlarge the Project's purpose, accept C2–C4 risk, or
deliver XSH.

### Provenance

The curated, queryable chain explaining where a consequential claim, signal,
decision, product, or institution came from, under what authority and scope,
through which transformations and dissent. Provenance is the undercurrent of
observability: always available for derivation and explanation, but only
semantically admitted and curated distinctions may acquire actionable
influence.

### Public monitor socket

The `0600` named Unix socket exposed by the resident `societyd`. Its closed
protocol can query daemon status and durable command receipts only; it has no
execute tag and cannot submit a claimed principal, grant, capability, or kernel
command. Same-UID peer checks provide attribution, not containment against a
hostile process running as that user.

### Quiescence

The reversible Operating Cycle condition in which the kernel has incremented
the admission generation and admits no new task work while already running work
settles. Only bounded Grand Architect recovery, cancellation, or closure turns
may occur. Quiescence is not cancellation and does not imply that Projects or
causal Episodes are complete.

### Recovery-fenced

The current conservative daemon startup mode for any nonempty ledger. Replay
and public monitoring remain available, and an exact duplicate command can
recover its receipt, but new mutation is refused until a future typed recovery
workflow proves the interrupted process, session, budget, and evidence state.

### Retrospective

A routine account of what an episode, project, or circuit learned. Unlike a
postmortem, it does not imply a triggering failure or containment duty.

### Review challenge

The typed output of adversarial review: target revision, failure hypothesis,
scope, evidence or reproducer, severity, falsification condition, requested
disposition, author lineage, and response state. It cannot edit the challenged
object.

### Rust kernel (`societyd`)

The continuously running Rust authority that exclusively owns SQLite,
content-object storage, typed commands, capabilities, Operating Cycles,
budgets, work admission, Pi and deterministic child processes, sessions,
workspaces, Git materialization, cancellation, tracing, and projections. XSH and
actor processes are supervised clients, never co-owners of trusted state.

### Signal family

A versioned definition of one comparable class of derived signals, including
eligibility, uncertainty semantics, pressure formula, decay, hysteresis,
allowed influence effects, and replay tests. Values from different families
are not assumed to share a universal scale.

### Society

The XSH-directed complex of constitutional purpose, the Grand Architect office,
replaceable actors, institutions, projects, circuits, epistemic memory,
checked culture, scarce resources, and product artifacts. The term licenses
only mechanisms with demonstrated engineering value; it does not imply money,
trade, electoral politics, simulated emotions, autonomous demography, or a
general social world simulator.

### Supervisor authority channel

The pre-opened connected Unix stream over which a trusted process supervisor
may submit the resident daemon's closed command language. It has no path or
credential-file constructor, is validated as a same-UID `AF_UNIX` stream, and
is marked close-on-exec. Those checks do not prove how the descriptor was
created or inherited: trusted spawner custody is the process-boundary
assumption. Loss, malformed framing, or reply failure permanently closes that
daemon instance's mutation admission while its public monitor remains alive.

### Ticket

A typed operational work order inside a project, linked to its graph motivation,
acceptance judge, budget, capability need, and completion evidence. A ticket is
useful corporate machinery, not the society's ontology or a substitute for
inquiry.

### Tracing

The live, redaction-safe rendering of curated Rust-kernel lifecycle and failure
events through `tracing` and `tracing-subscriber`. `INFO` and higher form the
mandatory Grand Architect operator stream; `DEBUG` and `TRACE` are bounded
diagnostics. Trace text is never the event ledger, provenance, durable workflow
state, or raw context supplied to an actor.

### Trusted physics

The slowly changing Rust/SQLite mechanism layer that makes identity, authority,
transactions, budgets, process ownership, cancellation, session evidence,
content identity, state transitions, Git lineage, observability, and replay
real. Trusted physics is not a human governance tier. The Grand Architect
governs through it and cannot cause an invalid transition by writing a
persuasive prompt.

### Universe Seed

The human-readable name of the first-class `UniverseSeed` construct: the
versioned origin purpose from which a society is bootstrapped. It combines the
XSH mission with north-star alignment, domain commitments, preserve/reject
principles, non-goals, amendment provenance, and the Grand Architect office
contract. Its canonical rendering is the first prompt supplied to every actor
attempt and Grand Architect actor session. Revision 1 is installed once through
a consumed founding-bootstrap capability because it is the input which starts
the apparatus; descendant revisions require C4 challenge and Grand Architect
ratification. One society has exactly one active revision at a time.

## Required distinctions

These pairs must not collapse in schemas or prose:

| Do not collapse | Distinction |
| --- | --- |
| Mission / north-star alignment | Purpose and worldview / operational alignment questions derived from it |
| Grand Architect / actor | Constitutional office / whichever principal currently occupies it |
| Office occupancy / Grand Architect office session / Office turn | Durable authority assignment / supervised Pi SDK-host lifetime / one bounded model interaction |
| Office / profession / project role | Durable authority / learned interface / bounded responsibility |
| Event / evidence / curated account / lesson | Occurrence / semantic admission / decision representation / proposed inheritance |
| Content object / graph object | Immutable bytes / revisioned meaning |
| Demand signal / command | Local need indication / authorized state mutation |
| Derived signal / influence | Explainable input / recorded organizational consequence |
| Event ledger / OperationalNotice / trace line | Durable accepted history / curated actionable monitoring projection / live rendering |
| Delivery / encounter / application / causal support | Carrier arrived / actor saw it / behavior matched it / comparison supports effect |
| Project / causal episode / ticket | Coordination envelope / knowledge-and-action chain / operational work order |
| Operating Cycle / Project / causal episode | Finite execution epoch / coordination envelope / epistemic-action history |
| Quiescence / cancellation / closure | Stop new admission / propagate stop control / reconcile a terminal epoch |
| Retrospective / postmortem | Routine learning / triggered failure investigation and containment |
| Legitimacy / correctness | Authorized process / later outcome judgment |
| Budget / target | Maximum authorized spend / desired expenditure; V2 has only the former |
| Model stop / attempt success | Pi stopped normally / protocol and judges accepted the attempt |
| Process exit / reap / Pi settlement | Child stopped / parent collected status / Pi protocol reached its terminal event |

## Retired or discouraged vocabulary

| Retired term | Use instead | Reason |
| --- | --- | --- |
| CTO | The Grand Architect / `TheGrandArchitect` | V2's highest office is constitutional and holder-agnostic |
| human approval or human ratification | Grand Architect authorization or ratification | Authority follows office occupancy, not actor species |
| actor genome | actor configuration | Avoid overstating biological metaphor |
| society genome | organization configuration or constitutional inheritance | Name the actual mutable layer |
| fitness score | health vector, Pareto relation, or named evaluator | There is no universal scalar objective |
| artifact | `ContentObject`, `ProductArtifact`, or named graph object | The bare word hides byte/meaning/product distinctions |
| worker | actor instance, deterministic service, or task owner | Name the kind of work producer |
| cleanroom boolean | execution profile with explicit fields | Isolation is multidimensional |
| `VS-001.json` | typed SQLite state plus `VS-001.md` projection | JSON is restricted to the Pi boundary |
| standup meeting | coordination pulse | Default status synchronization is deterministic and cheap |
| cycle | Operating Cycle, causal Episode, or propagation loop | Name the exact bounded epoch or semantic loop |
| `native_runner.xsh` | Rust `PiSupervisor` inside `societyd` | Process ownership, session parsing, cost, and cancellation belong to trusted physics |
