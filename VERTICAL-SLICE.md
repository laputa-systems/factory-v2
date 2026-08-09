# VS-001: native-host epistemic-to-product vertical slice

[`GLOSSARY.md`](GLOSSARY.md) is canonical for every domain term and spelling in
this contract. [`ARCHITECTURE.md`](ARCHITECTURE.md) owns the general behavior;
this file owns the first executable proof.

## Decision

The first V2 slice will reconcile XSH's contradictory `spawn` stderr contract
from source observation through one shipped XSH commit and checked institutional
memory.

The slice uses Pi 0.83.0 through its installed TypeScript SDK inside
V2-owned `society-pi-host` processes, with actors in native working directories
and Git worktrees. It does not invoke the Pi CLI and does not build or run
Docker images. Environmental austerity is a future experimental treatment, not
the baseline actor architecture.

The slice runs inside one continuously resident Rust `societyd`. Paid and
effectful work belongs to one bounded `OperatingCycle`; the daemon itself does
not stop between useful epochs. Rust owns SQLite, Pi SDK-host processes and sessions,
process groups, workspaces, Git materialization, costs, tracing, cancellation,
and reconciliation. XSH is exercised as the product and may be used inside
bounded actor-facing experiments, but it is not in any trusted control path.

`TheGrandArchitect` is occupied by one Pi actor for this slice through one
persistent Pi SDK Office session. The user may watch the same curated `INFO`-and-
higher operational stream and may invoke the typed emergency stop capability,
but does not form a second approval layer. Office authority is identical
whether a later cycle assigns the role to the user or to another actor.

This is a vertical slice in the strict sense: it touches every architectural
ring and crosses every durable boundary once.

```text
UniverseSeed bootstrap and Grand Architect Project charter
  -> admitted Operating Cycle and persistent Grand Architect Office session
  -> typed Project, Tickets, cross-cutting budgets, and first coordination pulse
  -> typed question and competing hypotheses
  -> native Pi inquiry attempts
  -> deterministic contract experiment
  -> curated account with preserved argument, conflict, and exclusions
  -> Grand-Architect-authorized decision
  -> native Pi product worktree
  -> independent review and XSH judges
  -> exact XSH delivery
  -> immediate and scheduled outcomes
  -> scoped lesson, delivery, encounter, and application observation
  -> blinded historical decision-world export
  -> replayable episode close
  -> cycle quiescence, child reaping, cost reconciliation, and closure
```

If the implementation stops at a graph demo, a Pi SDK adapter, an evaluator, or a
patch waiting in a directory, it is not VS-001.

## Why this episode

At the XSH snapshot inspected while writing this plan, the available records
disagree:

- `LANG.md` has an open proposal claiming `spawn` cannot redirect stderr;
- `docs/SPEC.md` defines `process.command_argv(..., stderr: Path, ...)`;
- the managed-spawn runtime sets `apply_redirections = true`;
- Factory V1 routinely uses
  `spawn process.command_argv(..., stderr: some_path)`;
- Laputa QEMU call sites still contain TODOs waiting for this capability; and
- the spawn prose in `docs/SPEC.md` does not state the stdio-field contract
  consistently across managed `spawn`, `process.spawn`, and command plans.

This is not an invented benchmark. It is a live institutional-memory failure
at exactly the seam V2 is meant to improve: implementation, specification,
proposal corpus, downstream need, and agent discoverability do not currently
form one warranted account.

The work is narrow enough that a first system can finish it. It is rich enough
to exercise observation versus interpretation, competing explanations,
behavioral evidence, API judgment, product integration, and propagation.

## Universe Seed and Project charter

VS-001 is bootstrapped from `UniverseSeed` revision 1. The canonical prompt
rendering includes XSH's purpose as a clean-slate, typed systems-glue language;
its Linux userspace scope; the Unix properties it preserves; its rejection of
quoting puzzles, ambient state, implicit evaluation, text-only boundaries, and
stacked private DSLs; its non-goals; the four north-star alignment questions;
and the active `TheGrandArchitect` office contract. This rendering is also the
first durable prompt component of the Grand Architect's persistent Office
session; Office identity does not exempt its occupant from the Universe Seed.

The bootstrap transaction creates exactly one active seed, installs exactly
one Grand Architect Office and actor occupant, and records
`ActorModelPolicyV1` and the society hard cost ceiling of
`UsdMicros(1_030_000)`. That `$1.03` ceiling is the cross-cutting authorization
for the separate `$0.03` qualification cycle plus the `$1.00` live slice; it is
not a spend target, and neither child envelope may borrow from the other. A
subsequent transaction admits `OC-VS-001` with the exact seed, Office
occupancy, organization configuration, model policy, trace policy, Project
envelope, WIP limits,
admission generation, and cancellation root. Every Project, Ticket, Episode,
Decision, ReviewChallenge, Postmortem, ActorAttempt, and OfficeTurn below stores
that seed revision; every paid execution also stores the Operating Cycle.
Every actor's `SYSTEM.md` begins with the same canonical `UNIVERSE-SEED.md`
rendering before the scoped assignment or context frontier.

The Grand Architect charters Project `P-VS-001-SPAWN-STDERR` with this
north-star alignment:

```text
capability_or_behavior:
  XSH users and agents can express ordinary child stderr policy through typed
  Command composition without a shell boundary.

general_improvement_evidence:
  normative registry, executable behavior, proposal corpus, public discovery,
  focused tests, and real call sites converge at one pinned XSH revision;
  a paired task provides only qualitative agent-fluency evidence.

clarity_boundary_composability:
  preserve explicit Command fields, owned process lifecycle, structured setup
  errors, default inheritance, and ordinary Path sinks.

revisit:
  C1 prototype review, C2 delivery review, immediate contract outcome, delayed
  Laputa use-site obligation, and any contradictory process test.
```

The Project's initial resource envelope, milestones, stop conditions, Tickets,
and review requirements are relational rows created by closed Rust commands.
The four answers above are one typed `NorthStarAlignment` row referenced by the
Project and inherited only by exact reference; Tickets and Decisions add their
own scoped alignments when their answers differ. There is no Project manifest
or JSON workflow document.

### Objective

> Keep XSH process composition typed, discoverable, ownership-safe, and free of
> shell wrappers when ordinary child stdio policy is required.

### Question

> At current XSH `HEAD`, what is the intended and actual stderr-redirection
> contract of a typed `Command` when consumed by managed `spawn` and
> `process.spawn`, and what smallest coherent XSH change reconciles runtime,
> specification, API discovery, tests, `LANG.md`, and demonstrated downstream
> need?

### Resolution condition

The question is resolved when all named command consumers have deterministic
behavior evidence, the normative contract contains no live contradiction, an
agent can discover and use the supported path without a shell wrapper, and the
resulting XSH change is either delivered or explicitly rejected with a recorded
reason.

### Initial hypotheses

`H1 — missing behavior`

The open proposal is substantially correct. Command stderr fields are not
honored by the relevant owned-spawn path, so XSH needs runtime, checker, API,
documentation, and test work.

`H2 — implemented but culturally stale`

The desired behavior is already correct. `LANG.md`, spawn documentation, API
discovery, and downstream TODOs lag the implementation. The product change is a
contract/test/documentation reconciliation rather than a new runtime feature.

`H3 — split or accidental behavior`

Some command consumers or evaluation paths honor stderr while others ignore or
mis-handle it. V1 usage proves only one path. XSH needs a scoped consistency fix
and must document any intentional difference between owned and detached spawn.

These hypotheses are not equally likely after preliminary inspection. The
episode records that current evidence favors `H2`; it does not manufacture
symmetry for the sake of deliberation. `H1` and `H3` remain live until the
behavior matrix and independent source review close them.

### Hard constraints

- No shell-string execution or `sh -c` workaround becomes canonical.
- Default spawn behavior remains stderr inheritance for compatibility.
- Redirection setup failure is a structured process error; it must not produce
  an apparently live handle.
- Owned process-group, wait, cancellation, timeout, and reaping semantics do
  not regress.
- The implementation does not add a second command or stdio abstraction when
  the existing typed `Command` is sufficient.
- Normative behavior, public API discovery, focused tests, `LANG.md`, and the
  implementation agree in the delivered XSH commit.
- No Factory V1 code or controller is imported into V2.

### Decision dimensions

The decision packet preserves:

- behavioral correctness across command consumers;
- ownership and cancellation semantics;
- API coherence with `run` and typed `Command` construction;
- compatibility and migration cost;
- discoverability for a Pi actor;
- implementation and documentation simplicity;
- downstream usefulness to real QEMU supervision; and
- future optionality for stdout capture, null sinks, append policy, and richer
  stdio without prematurely designing all of them.

## What VS-001 proves—and does not prove

### It must prove

- one database-authoritative episode can survive process failure and resume;
- one continuously running Rust daemon can admit, quiesce, cancel, reconcile,
  and close one finite Operating Cycle without losing a child or accepting a
  stale spawn;
- one active Universe Seed is rendered into every actor attempt and traceable
  through Project, Ticket, decision, product, and outcome;
- one Grand Architect occupant exercises C1 and C2 authority without a
  human-only authorization path;
- one persistent Pi SDK Office session receives checked operational notices,
  makes bounded typed decisions, survives unrelated task attempts, and is
  cancelled through the same process-ownership root;
- typed Project/Ticket state, a deterministic coordination pulse, and one
  `AdversarialReview` coordinate the episode without becoming its world model;
- typed graph revisions preserve what was known before the decision;
- native Pi attempts have pinned assignments, configurations, workspaces,
  sessions, costs, tool events, and outputs;
- accepted transitions produce reconstructable ledger events, curated
  `OperationalNotice` records, and redaction-safe tracing without treating logs
  or raw provenance as authority;
- raw audit/evidence, the curated episode account, and inherited culture remain
  distinct, with low-signal storage metadata excluded from provenance;
- observations are admitted by deterministic judges rather than actor prose;
- one curator selects consequential evidence, preserves dissent, and states
  exclusions before the first product decision;
- the decision retains alternatives, dissent, and predictions;
- the exact authorized XSH commit reaches the chosen product ref only after
  validation and Grand Architect authorization;
- a product invariant becomes technical heredity and one scoped lesson is
  delivered to, encountered by, and observably applied in one matching inquiry;
- an outcome obligation survives past delivery;
- a disclosure frontier exports a hindsight-sequestered decision world; and
- a projection can rebuild the episode from the ledger, admitted evidence, and
  curated account without treating every sealed content object as knowledge.
- graceful cancellation stops admission before signalling children, preserves
  partial evidence, reconciles cost, and proves the cycle has no live owned
  descendants before closure.

### It must not claim

- statistically persuasive agent-performance improvement from one paired trial;
- causal behavior change from one unpaired lesson-uptake probe;
- general organizational superiority;
- general autonomous language governance from one office-guided episode;
- security isolation from native working directories;
- a complete actor ecology, learned profession system, or organization-
  configuration search; or
- recursive self-improvement.

VS-001 establishes the laboratory and one complete cultural/product heartbeat.
Organization comparison begins only after several such episodes exist.

## Native execution profile

### Baseline profile

The first named profile is `native_workspace_v1`:

```text
authority:          resident Rust `societyd`
cycle:              exactly one admitted Operating Cycle
supervisor:         Rust `PiSupervisor` supervising `society-pi-host`
adapter:            pinned V2 TypeScript host, one process per AgentSession
node:               exact qualified Node runtime and executable digest
pi:                 `@earendil-works/pi-coding-agent` 0.83.0 plus lock integrity
cwd:                owned attempt working directory
repository:         none, detached read worktree, or product branch worktree
context discovery:  disabled; declared context pack supplied explicitly
project resources:  explicitly untrusted
extensions:         disabled
skills:             disabled
prompt templates:   disabled
themes:             disabled
tools:              per-attempt Pi allowlist
network:            ordinary host/model access; not claimed confined
filesystem:         ordinary host account access; not claimed confined
process:            owned process group with wall timeout and cancellation
environment:        declared allowlist; secret values never sealed
observability:      curated tracing policy plus typed OperationalNotices
```

The environment manifest records this as a collection of fields, not
`cleanroom: false`.

### Model and cost envelope

Every VS-001 task Attempt and Grand Architect Office session uses exactly:

```text
Pi:        0.83.0
provider:  openrouter
model:     deepseek/deepseek-v4-flash-0731
thinking:  high
fallback:  none
```

`ActorModelPolicyV1` also pins every SDK setting which can change execution:

```text
transport:                    sse
steeringMode:                 one-at-a-time
followUpMode:                 one-at-a-time
compaction.enabled:           true
compaction.reserveTokens:     16384
compaction.keepRecentTokens:  20000
retry.enabled:                true
retry.maxRetries:             2
retry.baseDelayMs:            2000
retry.provider.timeoutMs:     300000
retry.provider.maxRetries:    1
retry.provider.maxRetryDelayMs: 30000
defaultProjectTrust:          never
enableInstallTelemetry:       false
enableAnalytics:              false
images.blockImages:           true
```

Rust wall/turn/cost cancellation remains authoritative over SDK retry. Retry
events and provider attempts count against the same reservation; the host may
not extend a deadline because Pi is retrying or compacting.

The SDK host substitutes none of those values from ambient configuration. A
different effective provider, requested model, thinking level, Node/adapter/Pi
package identity, or
response model fails qualification rather than silently creating a new
treatment.

The live Project and Operating Cycle each have a `$1.00` hard cap, represented
durably as `UsdMicros(1_000_000)`. They are different constraints over the same
charges, not a parent/child pair and not `$2.00` of available spend. Before any
paid execution starts, one transaction reserves the charge against every
applicable society, cycle, Project, Office-session, and Attempt constraint.
VS-001 pre-allocates these maxima:

| Paid execution | Cap | Turn cap | Active wall cap |
| --- | ---: | ---: | ---: |
| Grand Architect Office session | $0.16 | 96 total; 12 Office turns | 3,600 s total; 600 s/turn |
| Contract cartographer | $0.06 | 36 | 900 s |
| Boundary skeptic | $0.06 | 36 | 900 s |
| C1 decision-account curator | $0.05 | 24 | 600 s |
| C2 descendant curator | $0.05 | 24 | 600 s |
| Prototype builder | $0.18 | 128 | 1,800 s |
| Baseline task actor | $0.06 | 64 | 900 s |
| Candidate task actor | $0.06 | 64 | 900 s |
| Product adversary | $0.08 | 48 | 900 s |
| Lesson-uptake investigator | $0.06 | 36 | 600 s |

The Office session and nine task-attempt maxima reserve `$0.82`; `$0.18` is
Project and cycle contingency. It is unavailable to ordinary scheduling. The
Grand Architect may allocate it to a named descendant attempt or Office-turn
extension only after reviewing the failed execution, known spend, remaining
evidence need, and stop condition. The allocation is a typed decision and
atomic budget reservation, not a prompt suggestion. A retry always receives a
new Attempt identity and reservation. A new Grand Architect process would
likewise receive a descendant OfficeSession identity; it never overwrites the
failed session.

The first named contingency allocation is `$0.01` for T00's Pi process double.
The double makes no provider call, but its injected `Unknown` cost is
conservatively reconciled as the full `$0.01` reservation and leaves `$0.17`
contingency for live descendants. Projections label that charge
`fault_injection_reserve_consumed`; they do not misreport it as provider spend.

The Operating Cycle has a 14,400-second elapsed horizon, a maximum of one live
Grand Architect Office process, two simultaneous task processes, ten task
attempt admissions absent contingency, and twelve kernel-originated Office
turns. Time between Office turns consumes neither model turns nor active-process
wall allowance, but it does consume the cycle horizon. Reaching a WIP or turn
limit blocks admission; reaching a cost or cycle hard limit initiates typed
quiescence or cancellation according to the registered policy.

The native Pi qualification preceding VS-001 belongs to a separate laboratory
Project and cycle with a `$0.03` cap. It reserves `$0.015` for one one-shot
Attempt with 16 turns/300 seconds and `$0.015` for one SDK Office session with
two Office turns/300 active seconds. It cannot borrow from the live slice or be
reported as VS-001 discovery work. If either boundary cannot qualify inside its
reservation, qualification stops rather than reallocating product budget.

Provider-free end-to-end execution uses a third closed treatment,
`Vs001DeterministicV1`. It has the same `$1.00` logical envelope needed to
exercise VS-001's reservations, fault injection, and closure rules, but model
network access is denied and only the deterministic Pi-host process-double
profile is eligible. It cannot emit `PiSdkQualificationV1`, cannot admit the
native profile, and cannot be relabeled or resumed as `Vs001LiveV1`. The paid
`PiSdkQualificationV1` laboratory remains bootstrap/kernel-only: its minimal
Office-shaped SDK session has no Grand Architect Office capability or ordinary
society-command surface.

Cost is stored as integer micro-US-dollars. The Pi supervisor normalizes each
cumulative provider cost idempotently and atomically charges all applicable
constraints. Task-attempt and Office-session events are observed from their Pi
event/session streams; an Office turn is reconciled before another is admitted.
`Unknown` or `Unavailable` is not zero: after paid work begins it freezes the
Operating Cycle's admission generation, requests cancellation of the affected
process, preserves partial evidence, and triggers a cost Postmortem. Known cap
breach does the same. Turn and wall caps bound continuation but cannot promise
no provider-side overshoot between a response and cancellation.

No paid attempt runs for projection, coordination pulse, schema qualification,
deterministic judge, product materialization, replay, or fault injection. All
supervision and end-to-end tests use pinned SDK-host doubles. Unused reservation returns
to its parent only after cost reconciliation; it is never interpreted as a
throughput target.

### Pi SDK construction and adapter boundary

#### Installed and pinned runtime

V2 installs `@earendil-works/pi-coding-agent` 0.83.0 in
`packages/society-pi-host/` and commits the package manifest and exact lockfile.
The execution profile records the lockfile digest, compiled adapter digest,
resolved package version, transitive Pi-package versions, and one exact
qualified Node executable/version satisfying Pi's `>=22.19.0` requirement.
Neither a globally installed `pi` executable nor ambient Node package resolution
participates in actor execution.

The pinned `~/d/pi` checkout remains the source-level qualification reference
for `createAgentSession`, `AgentSession`, `SessionManager`, `ModelRuntime`,
`SettingsManager`, `ResourceLoader`, event types, session format, usage, and
tool construction. Runtime imports the installed locked package and never reads
that checkout. A package, lock, adapter, Node, session-format, or event-union
change creates a new execution-profile revision and fails closed until its
fixtures and qualification pass.

#### Explicit SDK session construction

Every actor runs in a separate `society-pi-host` process supervised by Rust.
For each task Attempt or Grand Architect Office session, the host constructs:

```text
ModelRuntime.create({ authPath, modelsPath })
  -> resolve exact openrouter/deepseek/deepseek-v4-flash-0731
SettingsManager.inMemory(<registered retry, compaction, queue policy>)
V2 ResourceLoader(<exact SYSTEM text; every discovered resource collection empty>)
SessionManager.create(<owned cwd>, <owned session directory>)
createAgentSession({
  cwd, agentDir, model, thinkingLevel: "high", modelRuntime,
  resourceLoader, settingsManager, sessionManager, tools: <exact allowlist>
})
```

The V2 `ResourceLoader` returns no extensions, skills, prompt templates, themes,
agent/context files, append prompts, or resource-discovery errors. It returns
only the exact kernel-rendered system prompt. `tools` is always an explicit
allowlist; no SDK default tool set is accepted. `scopedModels`, custom tools,
session replacement, model cycling, and settings mutation are absent from the
adapter contract. Retry, compaction, steering/follow-up queue mode, and token
thresholds are exact fields of `ActorModelPolicyV1`, not ambient Pi settings.

`SYSTEM.md` is deterministically rendered from the active
`UNIVERSE-SEED.md`, actor/Office authority boundary, closed submission
contract, and scoped context/frontier policy. The seed is the first byte-bearing
component. A task's first `session.prompt()` receives the exact sealed
assignment/context rendering directly; the adapter does not rely on Pi CLI
`@file` expansion. The persisted first user message must equal that rendering.

The Grand Architect uses the same SDK, not a separate RPC mode. Its long-lived
host constructs one `AgentSession` for `OC-VS-001`; `OFFICE-SYSTEM.md` begins
with the Universe Seed and continues with the Office contract, cycle brief,
notice policy, allowed typed submissions, and reminder that narrative text has
no authority. Kernel `Prompt`, `FollowUp`, `Steer`, and `Abort` adapter commands
call `session.prompt()`, `session.followUp()`, `session.steer()`, and
`session.abort()` respectively. Cycle reconciliation calls `session.dispose()`
only after the final turn and evidence checks.

Pi authentication remains in the explicit host-admin-configured `agentDir`.
The adapter passes exact `authPath` and `modelsPath` to `ModelRuntime.create()`
and records a logical non-secret configuration identity. It never emits
credentials, resolved secret headers, `auth.json`, or a secret-bearing
environment. The owned `SessionManager` directory keeps transcripts in the
session workspace. Required model traffic is not described as network
confinement.

#### Closed Rust/TypeScript protocol

The only cross-language JSON is the Pi boundary between Rust and
`society-pi-host`. Kernel-owned stdin/stdout pipes carry closed, versioned JSONL
frames. The initial command set is:

```text
CreateSession  exact profile, cwd, prompt digest/content, tools, session dir
Prompt         one task assignment or one idle Office turn
FollowUp       bounded Office notice/decision packet queued after current turn
Steer          urgent decision-relevant correction plus durable reason
Abort          cooperative cancellation
GetState       liveness and reconciliation query
Dispose        close an idle session and flush its SessionManager
```

The host emits only:

```text
AdapterReady   pid/nonce plus Node, adapter, package and protocol identity;
               Rust supplies and records process-group identity
SessionReady   effective model/thinking/tools/settings/cwd/session identity
CommandResult  correlated acceptance or typed adapter failure
AgentEvent     exhaustive JSON-safe projection of AgentSessionEvent
UsageSnapshot  cumulative typed usage/cost source
Settled        prompt/turn terminal classification
Disposed       session flushed and SDK resources released
Fatal          adapter/session invariant failure
```

Every frame carries adapter-protocol version, session identity, monotonic
sequence, and correlation id where applicable. Braces in implementation are
Pi-boundary transport only, never workflow state. Unknown event variants,
sequence gaps, duplicate conflicting frames, invalid JSON, effective-config
drift, or a missing terminal event make the execution `protocol_failed`.
Rust seals both raw directions, parses closed Rust types, and normalizes
session/event/usage facts into SQLite.

For an Office turn, `Prompt` is legal only while idle. `FollowUp` is bounded and
queued; `Steer` requires an urgent stale/unsafe premise; `Abort` is the first
graceful-cancellation action. The adapter awaits the SDK promise and terminal
events before emitting `Settled`. The kernel batches only eligible
`OperationalNotice` rows and named decision packets at one ledger frontier,
never trace lines or the whole provenance graph. The occupant's closed Office
submission is still reauthorized command by command by `societyd`.

If a host exits, loses protocol, exceeds cost, or is cancelled, its actor/Office
identity remains visible but unavailable. No sibling host inherits authority or
session memory. Resumption creates a descendant `GrandArchitectOfficeSession`
from a durable recovery packet, not from unsealed chat. A shared multi-session
host is deferred because it would couple tool state, crashes, abort, evidence,
and kill escalation across unrelated actors.

### Pi evidence semantics

Pi 0.83.0 `AgentSession.subscribe()` produces `AgentSessionEvent` values such
as:

```text
agent_start
turn_start / turn_end
message_start / message_update / message_end
tool_execution_start / tool_execution_update / tool_execution_end
agent_end
agent_settled
entry_appended
auto_retry_start / auto_retry_end
compaction_start / compaction_end
```

`society-pi-host` converts that union exhaustively into `AgentEvent` boundary
frames without dropping unknown variants. `agent_end` may be followed by
automatic retry. `agent_settled` says retry and post-run handling have
finished. A clean Prompt/Office turn expects one final `agent_settled`, SDK
promise resolution, and a `Settled` host frame before Rust accepts normal
completion.

`SessionManager.create()` writes a separate canonical JSONL transcript in the
owned session directory. Its v3 header
contains `type`, `version`, `id`, `timestamp`, and `cwd`; its tree entries
persist user, assistant, and tool-result messages plus model/thinking changes
and any compaction or branch-summary usage. Assistant messages contain
provider, requested model, optional response model, stop reason, optional error,
token usage, optional provider-reported reasoning tokens, and calculated cost.

The two streams serve different evidence roles:

- the sealed host control/event JSONL preserves adapter identity, commands,
  execution order, streaming/tool lifecycle, retry, and settlement; and
- the session file preserves the durable transcript tree and billable usage.

VS-001 seals both. Its normalizer aggregates usage from assistant, tool-result,
compaction, and branch-summary entries exactly as Pi does. It reports reasoning
tokens only when the provider supplies them and never estimates them from
thinking text.

An SDK-host process exit is not a sufficient statement of model success. The
normalizer classifies adapter protocol, SDK promise, final settled assistant
`stopReason` (`stop`, `length`, `error`, or `aborted`), required submission,
tool failures, retry history, disposal, and process exit separately. A zero host
exit with an invalid submission is still `protocol_failed`; an interrupted event stream
without `agent_settled` is still incomplete even if a partial session exists.

### Kernel observability contract

VS-001 uses the already selected `tracing` and `tracing-subscriber` crates.
`societyd` installs a mandatory monitor layer which streams curated `INFO`,
`WARN`, and `ERROR` events while the service runs. A separate diagnostic layer
may enable narrower `DEBUG` or `TRACE` targets during qualification. Ambient
filter configuration may make diagnostics more verbose; it may not suppress
the mandatory monitor layer or redefine the meaning of a level.

The initial targets are closed and domain-specific:

```text
society.cycle       admission, quiescence, reconciliation, closure
society.office      occupancy, Office turns, decisions, recovery
society.attempt     work admission and Attempt lifecycle
society.pi          process and Pi protocol lifecycle
society.cost        reservation, known spend, uncertainty, exhaustion
society.cancel      request, propagation, signal escalation, reap
society.git         worktree, materialization, validation, delivery
society.ledger      committed transition and recovery failures
society.notice      notice eligibility, batching, delivery, suppression
society.projection  cursor and rebuild failures
```

Each span carries typed renderings of the smallest useful identifiers:
`society_id`, `operating_cycle_id`, and then, as applicable, `project_id`,
`ticket_id`, `episode_id`, `office_session_id`, `office_turn_id`, `attempt_id`,
`child_process_id`, `cancellation_request_id`, and `command_id`. Digests are
short display prefixes in traces but remain full identities in SQLite. Actor
prompt text, model reasoning, source contents, credentials, environment values,
raw JSON, and submission bodies never appear as trace fields.

The level contract is:

| Level | Meaning in VS-001 | Examples |
| --- | --- | --- |
| `TRACE` | raw-boundary diagnostics, disabled in the live run | redacted Pi frame classification, parser cursor movement |
| `DEBUG` | rebuildable mechanism choices | influence eligibility, notice suppression reason, lease poll |
| `INFO` | accepted lifecycle fact useful to an operator | cycle/attempt/Office-turn start or finish, reservation aggregate, decision, delivery, reconciliation |
| `WARN` | degraded but contained condition requiring attention | stale generation, retry, unknown cost, contamination, cancellation escalation |
| `ERROR` | trusted mechanism or containment failure | ledger invariant failure, unreaped child, corrupt Pi boundary, failed crash recovery |

Tracing is not the ledger, provenance, evidence, or a scheduler input. For an
accepted state change, the kernel commits the typed event and materialized state
first. It then derives zero or one typed `OperationalNotice` from the committed
fact and renders a trace event. A trace sink failure cannot roll back state; a
database failure cannot be papered over with a reassuring log line. Replay can
reconstruct authoritative state and notices without parsing trace text.

An `OperationalNotice` contains a closed `NoticeKind`, severity, subject,
committed event cursor, concise typed summary fields, eligibility reason,
deduplication key, creation time, optional expiry, and allowed audiences. It
does not contain arbitrary provenance or prompt text. VS-001 emits notices for:

- Operating Cycle admission, quiescence, cancellation, and close readiness;
- Grand Architect Office availability, turn settlement, and decision due;
- Attempt start, terminal classification, retry need, and anomalous workspace;
- budget thresholds, unknown cost, cap exhaustion, and reconciliation;
- unresolved review challenges, blocked milestones, and due outcomes;
- cancellation escalation or a child which has not yet been reaped;
- delivery readiness, target-ref movement, and product delivery; and
- daemon recovery requiring Grand Architect attention.

Repeated notices with the same semantic key are coalesced until their severity,
subject generation, or actionable state changes. `INFO` delivery to the live
console is nonblocking and may drop a repeated rendering under backpressure;
the durable notice and ledger event remain queryable. `WARN` and `ERROR`
renderings use a bounded high-priority queue and surface a dropped-count summary
if the sink cannot keep up. Neither queue can block SQLite ownership, Pi
cancellation, or child reaping.

A user-held Grand Architect monitors the fixed `INFO`-and-higher stream plus
typed queries. The agent-held Grand Architect in VS-001 receives bounded notice
batches through its SDK Office session. The two surfaces derive from the same
notice rows, but raw trace lines are never fed back into Pi. Notice delivery
records audience, Office turn, ledger frontier, suppression/coalescing state,
and spend; delivery alone grants no authority and proves no epistemic uptake.

### Evidence and curation boundary

VS-001 implements four depths without conflating them:

```text
operational audit
  Pi events, session entries, process exits, tool lifecycle, Git receipts

forensic evidence
  sealed source snapshots, submissions, patches, test and evaluator outputs

curated episode account
  selected observations, arguments, conflicts, unknowns, and exclusions

inherited meaning
  one XSH invariant and one scoped L1 lesson with target-specific uptake state
```

Sealing is deliberately cheap and semantically weak. A sealed Pi transcript
does not become a graph claim. The normalizer admits only execution facts needed
to classify the attempt and resource use. A deterministic judge admits an
observation only under its pinned evaluator revision. An actor contribution is
an argument proposal until the kernel links its cited sources and a curator
selects it for a named account.

VS-001's C1 decision account has this closed semantic contract:

```text
kind: decision_curation
purpose: authorize_spawn_stderr_prototype
question_revision
epistemic_disclosure_frontier

selected_items[] {
  source_revision_or_evidence
  role: observation | supporting_argument | defeating_argument |
        constraint | unknown | dissent
  selection_reason
  applicability_scope
}

preserved_conflicts[]
decision_relevant_unknowns[]
exclusions[] {
  category_or_source
  reason
  risk_if_wrong
}
raw_evidence_escalations[]
curator_configuration
```

The bracketed collections describe Rust vectors at the protocol boundary and
normalized ordinal child rows in SQLite. They are not serialized into a JSON
column. The curator's `submission.json` is parsed into these types at the Pi
boundary and discarded as workflow authority after its sealed content identity
and typed contribution are recorded.

It does not contain byte counts, generic filenames, streaming deltas, or a list
of every tool call. Those remain discoverable from forensic manifests when a
specific challenge makes them relevant. Storage length can be computed by the
content-object store for safety; it is not persisted as epistemic provenance.

`exclusions[]` names omitted semantic categories and any particular source whose
absence could change the decision. It does not enumerate every unselected
content object; the forensic manifests already provide that inventory.

The curator sees admitted observations, validated actor submissions, source
references, and the pre-agent hypothesis graph. Raw Pi transcripts are absent
from its default context. A request for raw evidence names the question it is
expected to resolve and creates a visible escalation; the supervisor supplies only
the requested artifact. The product adversary later receives the accepted
account plus its exclusions and may challenge an omission by citing admitted or
raw evidence.

The account lifecycle is:

```text
proposed -> challenged | accepted_for_c1
challenged -> descendant_proposed | accepted_with_dissent | rejected
accepted_for_c1 -> superseded | retracted
```

Acceptance means sufficient for this decision, not globally authoritative. The
C2 packet uses a descendant account which adds prototype, fluency-probe, and
review evidence without changing the C1 account.

### Tool profiles

VS-001 uses four explicit profiles:

| Profile | Pi tools | Intended use |
| --- | --- | --- |
| `read_source_v1` | `read,bash,grep,find,ls` | Source and contract inquiry in a detached worktree |
| `curator_v1` | `read,write` | Select a decision account from declared admitted evidence |
| `product_builder_v1` | `read,bash,edit,write,grep,find,ls` | One authorized XSH branch worktree |
| `task_actor_v1` | `read,bash,edit,write,grep,find,ls` | Produce an XSH script or an uptake-probe submission in a fixture workspace |

`bash` makes three profiles broad at the OS level; `read` and `write` are also
not an OS confinement boundary. The profile is an
experimental/tool-availability declaration, not a security capability. The SDK
adapter exposes no extension runtime and preserves every reported tool call
and nonzero tool result.

### Workspace layout

Runtime state is outside Git under a configurable root, initially
`var/` in the V2 checkout:

```text
var/
├── society.sqlite3
├── content/sha256/<prefix>/<digest>
├── runtime/
│   └── society.sock              # disposable local control socket
├── office-sessions/<office-session-id>/
│   ├── input/
│   │   └── OFFICE-SYSTEM.md
│   ├── pi/
│   │   ├── commands.jsonl        # exact Rust-to-SDK-host boundary
│   │   ├── events.jsonl          # host results and projected SDK events
│   │   ├── session/              # canonical SessionManager JSONL
│   │   └── stderr.log
│   └── output/
│       └── submission.json       # one closed Office submission at a time
├── workspaces/<attempt-id>/
│   ├── input/
│   │   ├── UNIVERSE-SEED.md
│   │   ├── SYSTEM.md
│   │   ├── ASSIGNMENT.md
│   │   └── context/
│   ├── work/
│   │   └── repo/                 # only when a Git worktree is assigned
│   ├── output/
│   │   └── submission.json
│   ├── pi/
│   │   ├── commands.jsonl
│   │   ├── events.jsonl
│   │   ├── session/              # canonical SessionManager JSONL
│   │   └── stderr.log
└── projections/
```

`society.sqlite3` and `content/` are durable and backed up together.
`runtime/` contains no authority and may be recreated only after single-daemon
ownership is established. `office-sessions/` and `workspaces/` are owned
staging areas. Required forensic content is sealed into
`content/` before a workspace becomes cleanup-eligible; semantic evidence
admission remains a separate database transition. `projections/` can be deleted
and rebuilt.

The V2 source repository contains schemas, migrations, policies, prompts,
deterministic fixtures, and tests. It does not commit paid sessions or a second
copy of the durable database.

Workspace identity, execution profile, input membership, environment allowlist,
pre/post Git state, process status, usage, cost, settlement, and cleanup
eligibility are typed SQLite rows. They are materialized into the workspace by
`societyd`; there is no `manifest.json` or `receipt.json` authority. JSON exists
only at a Pi boundary: per-session SDK-host `commands.jsonl`, `events.jsonl`,
canonical SessionManager JSONL beneath `pi/session/`, and the current closed
`submission.json`. A diagnostic Markdown
receipt may be projected on demand but is disposable.

### Workspace classes

`FixtureWorkspace`

Contains only the declared task, fixtures, assigned XSH/Xsht binaries, and
output paths. It has no Git repository. VS-001 task actors use this class.

`ReadWorktree`

Is a detached XSH worktree at the episode's pinned base commit. The actor gets
read-oriented Pi tools. A post-attempt Git receipt records any modification;
modified read work is preserved as anomalous evidence and cannot be admitted as
an observation source without review.

`ProductWorktree`

Is a branch worktree created at the pinned XSH base with a branch name derived
from the authorized implementation and attempt identities. Only the assigned
builder owns it. `societyd` records status and tree digests before and after,
the parent, portable patch digest, changed paths, and untracked files. The Pi
builder does not create the authoritative product commit. After a C2 decision,
a deterministic materializer applies the accepted patch to a fresh worktree,
reruns the required judges, and creates the controlled product commit from that
exact tree without invoking repository hooks. The branch and source workspace are
retained until delivery or explicit retirement.

### Native Pi supervision and cancellation

`PiSupervisor` is a Rust subsystem inside `societyd`. It owns both one-shot
task attempts and the Grand Architect SDK Office process. No XSH program, actor,
or detached cleanup script can create a Pi `AgentSession`. For every child it:

1. reserves all applicable budgets and obtains the current Operating Cycle
   admission generation in one transaction;
2. creates the workspace, sealed input copies, pipes, and pre-execution
   filesystem/Git receipt;
3. spawns the pinned compiled `society-pi-host` entry point under the exact Node
   executable as a new owned process group; the host is inert and may not call
   `createAgentSession()` yet;
4. waits for `AdapterReady { pid, spawn_nonce, identities }`, cross-checks the
   PID, records it with the Rust-created process group in `child_processes`, and
   rechecks cycle state, generation, capability, reservation, and cancellation
   ancestry;
5. sends `CreateSession` only after that recheck; the host exits on denial or
   control-pipe EOF, constructs the exact SDK session after authorization, and
   must return a matching effective `SessionReady` before Rust sends `Prompt`;
6. streams and incrementally normalizes host results, SDK event projections,
   the canonical SessionManager file, turns, usage, and provider cost;
7. on normal process completion, waits and reaps the owned process group,
   reconciles event settlement, final assistant stop reason, retry history,
   session entries, submission protocol, exit status, signal, duration, budget,
   and post-workspace receipt;
8. on interruption, executes the cancellation protocol below and preserves
   partial streams, session, stderr, workspace, cost knowledge, and signal
   receipts rather than manufacturing normal settlement; and
9. seals required forensic content, registers the manifest and normalized
   terminal facts, reconciles reservations, then releases the lease.

The handshake closes the dangerous interval in which a stale scheduler could
create a paid SDK session after quiescence but before the daemon records a PID.
A generation fence alone is insufficient: reservation, child registration,
and the final pre-`CreateSession` check are all required. Later session commands
use the same closed adapter stream and sequence; `AdapterReady` does not imply
that a model or session has started.

Cancellation is a typed control-plane command, not a `DerivedSignal`, trace
message, or convention. VS-001 implements:

```text
CancellationRequest {
  cancellation_request_id
  scope: Society | OperatingCycle | Project | OfficeSession | Attempt
  mode: Quiesce | GracefulCancel | EmergencyStop
  reason: closed reason plus optional sealed explanation
  authority: capability grant | registered circuit breaker
  observed_admission_generation
  cooperative_abort_deadline
  terminate_deadline
  kill_deadline
  partial_evidence_policy
}
```

Accepting a request atomically changes the affected scope to non-admitting and
increments its admission generation before any signal is sent. All reserved but
not-yet-`CreateSession` children fail their recheck. The daemon expands the request into
typed propagation rows for the Grand Architect Office session, Attempts, and
live child process groups in the scope. Each target has one terminal
disposition: `not_running`, `completed_before_delivery`, `cooperatively_aborted`,
`terminated`, `killed`, or `containment_failed`.

`Quiesce` permits already running work and the current Office turn to settle,
but admits no new task work or ordinary Office turn; bounded recovery,
cancellation, and close turns remain available. `GracefulCancel` first sends
adapter `Abort`, which calls SDK `session.abort()`, where a live host channel
exists, waits up to five seconds, sends TERM to
each remaining process group, waits five more seconds, then sends KILL.
`EmergencyStop` fences admission identically, allows at most one second for SDK
abort, then sends TERM and escalates to KILL after two seconds. Wall-budget
expiry uses `GracefulCancel`; known containment failure or a second host stop
signal uses `EmergencyStop`.

Every delivery and escalation records the intended process identity, signal or
SDK-host command, time, result, and subsequent liveness observation. `societyd`
waits for and reaps children it still parents. If the daemon itself crashed,
restart begins in recovery mode with admission closed, signals any recorded
live process groups, waits until they are absent, and classifies their Attempts
as `supervision_lost`; POSIX cannot restore parentage or a missing wait status,
so the kernel records that evidence gap and triggers a Postmortem rather than
inventing a reap receipt.

The request is complete only when every propagation target is terminal, every
owned process is reaped or proven absent after crash recovery, required partial
evidence is sealed, all known cost is reconciled or explicitly `Unknown`, and
the target scope records its final cancellation disposition. A cycle cannot
close while any child identity is live or indeterminate.

The local control surface maps the first SIGINT or SIGTERM to a cycle-scoped
`GracefulCancel`; a second stop signal while cancellation is active upgrades the
same request lineage to `EmergencyStop`. `societyctl cycle quiesce`,
`societyctl cancel ...`, and the registered budget breaker use the identical
kernel path. Signal handlers themselves only wake the control loop; they never
write SQLite, allocate, log, or kill children directly.

macOS does not provide Linux cgroups. VS-001 therefore treats model cost,
memory, CPU, and descendants which deliberately escape the owned process group
as observed or best-effort. Wall timeout, admission fencing, registered process
groups, signalling, and direct-child reaping are enforced. Every receipt states
which limit was enforced, measured, unknown, or violated; native workspaces are
not described as a security sandbox.

### Submission boundary

Actors do not write the database or issue durable commands. They write a
closed-schema `output/submission.json`; their final chat text is preserved but
is not a control channel.

Inquiry submissions contain:

```text
kind: inquiry_contribution
question_revision
claims[]
source_references[]
supporting_evidence[]
contradictions[]
unknowns[]
recommended_experiments[]
confidence_form
```

Product submissions contain:

```text
kind: product_contribution
decision_revision
base_commit
candidate_tree_digest
patch_digest
changed_paths[]
tests_run[]
known_failures[]
contract_updates[]
```

Curator submissions use the closed `decision_curation` contract above. A raw
evidence request is a separate submission and attempt descendant; the supervisor
does not silently widen the curator's context and rerun the same identity.

Task-actor submissions contain the requested `supervise.xsh` artifact and a
small receipt naming the XSH binary it used. The lesson-uptake probe instead
submits:

```text
kind: lesson_uptake_contribution
lesson_revision_encountered
inquiry_question
sources_compared[]
distinctions_applied[]
proposal_or_no_change_recommendation
remaining_unknowns[]
```

Reviewer submissions contain findings with severity, evidence references,
curation-omission challenges, and an explicit disposition.

The Rust Pi adapter validates the schema and submits the contribution to
`societyd` on behalf of the actor instance. Invalid or missing submissions end
as `protocol_failed`; they are never repaired by parsing narrative prose.
Narrative chat and tool traces remain forensic evidence and do not become
culture merely because the model described a general lesson fluently.

## Minimal implementation architecture

### Source tree

The first implementation target is:

```text
Cargo.toml
crates/
├── society-kernel/        # domain types, transitions, SQLite, ledger, objects
├── society-pi/            # SDK-host protocol, normalizer, cost/session contracts
├── societyd/              # resident authority, scheduler, supervisor, observability
└── societyctl/            # typed local control/query client
packages/
└── society-pi-host/       # pinned TypeScript SDK adapter; one process/session
    ├── package.json
    ├── package-lock.json
    ├── tsconfig.json
    ├── src/
    └── tests/
migrations/                # monotonic V2-owned SQLite migrations
xsh/
└── experiments/           # optional R2 XSH workloads/evaluators; no authority
circuits/
└── vs-001-spawn-stderr/
    ├── PROJECT.md
    ├── actor-configs/
    ├── assignments/
    ├── contexts/
    ├── curation/
    ├── fixtures/
    ├── judges/
    ├── replay-frontiers/
    └── projections/
tests/
├── kernel/
├── daemon/
├── protocol/
├── pi/
├── cancellation/
├── observability/
└── vs-001/
```

These names are the initial searchable contract. A later rename must update
types, migrations, tests, diagrams, and the canonical glossary together.
Ownership may not collapse into a generic workflow framework.

Seed fixtures, state transitions, actor/model policies, budget values, and
behavior cases are compiled Rust types registered through kernel commands.
Markdown files in the circuit tree are prompt/projection templates or human
explanations. The source tree contains no alternate JSON workflow definition.

### Process topology

```text
user monitor <----- curated INFO/WARN/ERROR tracing -----+
                                                       |
societyctl ----versioned typed local protocol----> societyd
                                                       |
Grand Architect Pi SDK host <---bounded adapter/notices--- PiSupervisor
                                                       |
task Pi SDK hosts <------session adapters------ PiSupervisor
                                                       |
deterministic Rust/XSH product judges <-------- execution
                                                       |
                                  +--------------------+-----------+
                                  |                                |
                                  v                                v
                       SQLite + content objects          Git worktrees/target
```

`societyd` is the sole SQLite writer and now owns the private physical content
writer; child-process persistence and Git materialization remain later
resident integrations. Its named
`0600` Unix socket is a query-only public monitor surface used by `societyctl`;
it has no mutation tag. Commands arrive only over a distinct pre-opened
connected `AF_UNIX` supervisor stream. Stream type, peer family, same effective
UID, and close-on-exec are checked, but descriptor provenance remains a trusted
spawner/process-boundary assumption—same-UID is attribution, not hostile-user
containment. Both local wire formats are versioned, length-prefixed tagged
protocols with checked lengths, closed discriminants, fixed integer encodings,
UTF-8 validation, correlation id, and generation. They have no generic JSON
envelope or opaque payload variant. Each wire body decodes directly into one
closed Rust request type; unknown versions, tags, trailing bytes, oversized
frames, and missing required fields fail before authorization.

Task actors receive no socket path or daemon credential. The Pi adapter reads a
closed `submission.json` only after its child settles and submits the parsed
typed contribution under the supervisor's narrowly scoped capability. The
Grand Architect's narrative output likewise has no direct effect: its closed
Office submission is parsed, attributed to the active occupancy and turn, and
independently revalidated by `societyd` before any command is committed.

### Minimal relational schema

VS-001 implements the complete episode vocabulary, but only fields needed by
the slice. Each node revision has a typed one-to-one body table; node meaning is
not hidden in a generic JSON payload.

Identity and history:

```text
societies
universe_seeds
universe_seed_principles
universe_seed_alignment_questions
universe_seed_sources
north_star_alignments
society_bootstraps
principals
capability_grants
office_contracts
office_occupancies
office_transfers
commands
events
objects
object_revisions
edges
episodes
episode_objects
```

Continuous operations and control:

```text
operating_cycles
operating_cycle_resource_limits
operating_cycle_admissions
operating_cycle_reconciliations
grand_architect_office_sessions
office_turns
office_turn_submissions
child_processes
child_process_liveness_observations
process_signal_receipts
cancellation_requests
cancellation_propagations
trace_policy_revisions
operational_notices
operational_notice_audiences
operational_notice_deliveries
```

Corporate coordination and review:

```text
projects
project_objectives
project_milestones
project_stop_conditions
tickets
ticket_acceptance_conditions
ticket_prerequisites
coordination_pulses
coordination_pulse_items
adversarial_reviews
review_challenges
review_challenge_responses
review_dispositions
postmortems
postmortem_causal_claims
postmortem_action_proposals
```

Evidence, curation, and replay boundaries:

```text
content_objects
forensic_manifests
evidence_admissions
curated_accounts
curated_account_items
curation_exclusions
curation_challenges
disclosure_frontiers
disclosure_frontier_members
```

Typed epistemic bodies:

```text
objective_revisions
question_revisions
hypothesis_revisions
prediction_revisions
proposal_revisions
experiment_revisions
observation_revisions
argument_revisions
conflict_revisions
decision_revisions
implementation_revisions
outcome_revisions
retrospective_revisions
lesson_revisions
invariant_revisions
```

Actors and execution:

```text
organization_configurations
actor_configurations
actor_instances
execution_profiles
execution_profile_qualifications
context_packs
work_items
leases
attempts
pi_sessions
pi_adapter_commands
pi_adapter_results
pi_event_normalizations
budget_envelopes
budget_envelope_constraints
budget_reservations
budget_reservation_charges
resource_usage
cost_observations
workspaces
```

Active provenance and influence:

```text
provenance_facts
signal_family_revisions
derived_signals
influence_candidates
influence_decisions
influence_effects
demand_signal_responses
```

Product, propagation, and projections:

```text
repository_snapshots
product_changes
validations
deliveries
outcome_obligations
lesson_promotions
propagation_targets
propagation_observations
projection_cursors
outbox
```

Revision body tables store searchable contract fields such as scope,
resolution condition, prediction horizon, evaluator revision, authority,
revisit trigger, and propagation level in typed columns. Long human prose is a
sealed content object referenced by digest. Closed structured details receive a
named Rust struct and normalized table even when they do not determine
authority. Migration 1 contains no JSON column, generic `payload`, `metadata`,
or EAV escape hatch.

`operating_cycles` stores the pinned seed, Grand Architect occupancy,
organization/model/trace/admission-policy revisions, admission generation,
start and stop conditions, and lifecycle state. Attempts and Office sessions
have a non-null cycle foreign key. Project, Episode, Decision,
OutcomeObligation, Lesson, and Retrospective rows do not: those objects may span
cycles, while their creating or revising command records the cycle in which the
event occurred. This prevents a cycle boundary from becoming a false knowledge
boundary.

`child_processes` records the reserved execution, spawn nonce, PID, process
group, `AdapterReady`/`CreateSession` state, owner session or Attempt, and last liveness
classification. `process_signal_receipts` and `cancellation_propagations` are
append-only typed evidence of control delivery; neither claims a process died
until a wait status or subsequent absence observation says so.
`grand_architect_office_sessions` is a Pi execution identity, not an actor
identity or Office occupancy. `office_turns` bound each model interaction and
ledger frontier. Raw SDK-host frames remain sealed Pi evidence; normalized
commands and results are typed rows keyed by correlation id.

`operational_notices` contains only the curated notice contract. Trace lines
are intentionally absent from SQLite: accepted events and notice rows are the
replayable sources, while `tracing` is a live rendering. Notice delivery and
coalescing rows make information propagation inspectable without persisting a
high-volume copy of every log event.

`budget_envelope_constraints` links an envelope to its institutional subject;
`budget_reservation_charges` links one reservation to every applicable
envelope. The Project and Operating Cycle constraints therefore cross-cut the
same Office-turn or Attempt charge. Reservation and charge rows are accepted
atomically or not at all; there is no ambiguous single `parent_budget_id`.

`content_objects` is deliberately not a provenance table. It establishes byte
identity and storage status. A `forensic_manifest` groups content objects produced by an
attempt or deterministic run. Only `evidence_admissions` state that a sealed
object has a semantic role in an experiment or claim. Only curated-account
membership states that admitted evidence belongs in a decision representation.
The graph does not gain one node per file.

`GraphObject` and `ContentObject` are distinct domain concepts despite sharing
the ordinary word “object.” The former has typed semantic revisions in
`objects`; the latter is immutable byte content in `content_objects`. Protocol
types and command payloads use the full names and never a bare `ObjectId`.

Storage paths, byte lengths, and timestamps derivable from the event ledger or
content-object store are not duplicated into semantic records. Fields are
added only when a named invariant, reconstruction requirement, evaluator, or
query needs them. This is a guard against mistaking high-dimensional telemetry
for useful provenance.

`derived_signals` is rebuildable influence input, not epistemic truth. VS-001
uses four families: `unresolved_contract_conflict`,
`missing_behavioral_evidence`, `decision_due`, and `cost_reserve_at_risk`.
`demand_signal_responses` records why a configuration expressed interest.
`influence_candidates` and `influence_decisions` record which eligible signal
requested or received a visibility, retrieval, review, or scheduling effect.
Authority still comes from an Office/capability and readiness still comes from
typed workflow state.

`disclosure_frontiers` are immutable allowlists over object revisions,
admitted evidence, repository snapshots, culture, and policy available at a
named decision boundary. Frontier members are positive grants; absence means
sequestered. Exported replay packets are projections and never become new facts
in the source episode.

### Slice influence policy

VS-001 implements the full provenance-to-influence seam for only four signal
families. Eligibility is family-specific:

| Family | Eligible source | Allowed effect |
| --- | --- | --- |
| `unresolved_contract_conflict` | Active Conflict with at least two incompatible source-backed revisions in Project scope | Visible, attention bid, matched retrieval |
| `missing_behavioral_evidence` | Admitted Question/Experiment contract with named unresolved matrix cells and a registered deterministic judge | Visible, attention bid, matched retrieval |
| `decision_due` | Ready typed decision gate with complete prerequisites and an unresolved Grand Architect Decision | Visible in pulse and Grand Architect brief |
| `cost_reserve_at_risk` | Known integer spend at a configured fraction of an active reservation, or an explicit unknown/unavailable cost state | Visible in pulse and Grand Architect brief |

Within one family, fixed-point millionths compute:

```text
pressure =
  severity_ppm * applicable_exposure_ppm * warrant_lower_bound_ppm
  * time_pressure_ppm * independence_ppm
  / (1_000_000^4 * max(response_cost_units, 1))
```

Each factor is a closed lookup or deterministic quantity in that family's
revision; the derivation records every input. It is not inferred from prose or
the volume of trace records. Multiplication uses checked wide integers and a
versioned round-down rule. Exact ties use oldest-eligible then typed identity.
There is no comparison of pressure values across families: constitutional
precedence, due obligations, family quotas, and Project WIP form the cross-family
partial order.

The slice grants one attention slot per inquiry family, one decision-due slot,
and one cost slot. Duplicate or ancestry-correlated sources contribute no extra
independence. Hysteresis retains an applied candidate until a challenger is at
least 10% stronger or the incumbent expires, preventing pulse churn. Signals
recompute after cited revision, scope, reservation, or deadline changes and
become ineligible on retraction or contamination.

None of the four families may emit `AdmissionBlock`. Budget termination remains
an exact R0 state rule and required review remains an exact Project/C2 rule.
This is intentional: the vertical slice proves curated information bubbling
into bounded attention before it attempts learned blocking policy.

Rust newtypes distinguish `UniverseSeedId`, `ProjectId`, `TicketId`,
`GraphObjectId`, `ContentObjectId`, `ActorAttemptId`, `BudgetId`, and
`UsdMicros`; ordinary integers and strings do not cross those interfaces.
Closed enums own every state and relation. Optional columns represent one named
domain absence, not a loosely shaped record. Foreign keys, unique active-seed
and office-occupancy indexes, generation checks, and transaction tests make
invalid bootstrap, authority, reservation, and transition states difficult to
express.

`commands` and `events` hold only common identity, generation, authority, and
ordering columns. Each closed `CommandBody` and `EventBody` variant has one
named one-to-one body table; a deferred foreign-key/trigger invariant requires
exactly the body table named by the discriminant. The Rust decoder is
exhaustive and treats missing, duplicate, or mismatched bodies as ledger
corruption. Replay reads those typed rows in event order. It never deserializes
an opaque historical blob.

### Minimal command set

The versioned protocol needs only these public commands for VS-001:

```text
CreateSocietyIdentity
InstallGrandArchitectOffice
InstallFoundingUniverseSeed
BootstrapSociety
ProposeOperatingCycle
AdmitOperatingCycle
QuiesceOperatingCycle
ResumeOperatingCycle
ReconcileOperatingCycle
CloseOperatingCycle
StartGrandArchitectOfficeSession
OpenOfficeTurn
SubmitOfficeTurn
RecoverGrandArchitectOfficeSession
CreateProject
CharterProject
TransitionProject
CreateTicket
TransitionTicket
ReserveBudget
ReconcileBudget
EmitCoordinationPulse
CreateEpisode
AddGraphObjectRevision
AddGraphEdge
TransitionEpisode
RegisterActorConfiguration
AdmitActorInstance
RegisterWorkItem
ClaimWorkItem
StartAttempt
CompleteAttempt
SealContentObject
RegisterForensicManifest
AdmitEvidence
SubmitContribution
ProposeCuratedAccount
ChallengeCuratedAccount
AcceptCuratedAccount
RequestAdversarialReview
SubmitReviewChallenges
RespondToReviewChallenge
ResolveAdversarialReview
RecordDecision
AuthorizePrototype
AuthorizeDelivery
RecordProductCommit
RecordValidation
RecordDelivery
ScheduleOutcome
RecordOutcome
PromoteLesson
RegisterPropagationTarget
RecordPropagationObservation
RegisterSignalFamilyRevision
RecordInfluenceDecision
TriggerPostmortem
RecordPostmortemCausalClaim
ProposePostmortemAction
ClosePostmortem
CreateDisclosureFrontier
ReopenEpisode
CloseEpisode
RequestCancellation
UpgradeCancellation
```

Each command includes principal, capability, command id, expected generation,
and one closed Rust `CommandBody` variant. The CLI maps each variant to a
closed subcommand and typed flags; it does not accept a JSON command envelope.
Read queries and projection rebuilds are separate from commands.

Lifecycle facts owned by the daemon—`AdapterReady`/`CreateSession`, adapter
result, process
exit, signal receipt, cost sample, notice derivation, and recovery
classification—use equally closed internal commands attributed to a narrow
kernel principal. They are not public socket variants and cannot be forged by
an Office occupant. Internal does not mean unrecorded: they pass the same
transaction, idempotency, typed-event, and replay invariants.

### Slice state machines

Operating Cycle:

```text
proposed -> admitted -> running
running -> quiescing -> drained
drained -> running | reconciling
reconciling -> closed

running | quiescing | drained
  -> cancelling -> reaping -> reconciling
  -> cancelled | failed
```

Only `running` admits task Attempts or ordinary decision turns.
`quiescing`/`drained` may admit a Grand Architect recovery, cancellation, or
close turn which names that control purpose and uses an already reserved Office
turn; this exception cannot spawn task work. `QuiesceOperatingCycle` increments
the admission generation in the same transaction as `quiescing`.
`drained` means no active task Attempt and no active Office turn; the idle
Office process may still exist. Reconciliation closes that process, accounts
it, disposes every lease and reservation, proves every child terminal, and
names the successor-cycle disposition of open work. `closed`, `cancelled`, and
`failed` are immutable; resumption always creates a successor cycle.

Grand Architect Office session:

```text
reserved -> starting -> ready
ready -> turn_active -> ready
ready -> quiescing -> process_ended -> evidence_sealing -> closed

starting | ready | turn_active | quiescing
  -> cancelling -> process_ended -> evidence_sealing
  -> cancelled | failed
```

`turn_active` has exactly one nonterminal `OfficeTurn`. Notice batches arriving
during it may call SDK `followUp()` only within the registered limit; they do
not create concurrent turns. Normal cycle reconciliation sends adapter
`Dispose` only after the final turn settles, requires the SDK host to flush the
SessionManager, emit `Disposed`, and exit, then waits for the process. A process exit
before requested quiescence is a failure even if its last assistant message
looked complete.

Cancellation request:

```text
requested -> accepted -> propagating -> awaiting_grace
awaiting_grace -> terminating -> killing -> reconciling
awaiting_grace | terminating | killing -> reconciling
reconciling -> completed | containment_failed
```

The mode determines which intermediate deadlines may be zero; it never skips
admission fencing, target enumeration, reconciliation, or terminal evidence.
An upgrade from `GracefulCancel` to `EmergencyStop` creates a linked mode
revision and shortens remaining deadlines. It does not replace the original
request or repeat already terminal propagation targets.

Project:

```text
proposed -> challenged -> chartered -> active -> observing -> closed
active -> paused -> active
chartered | active | paused | observing -> terminated
closed | terminated -> reopened -> active
```

`P-VS-001-SPAWN-STDERR` has milestones `machinery_qualified`,
`c1_prototype_decided`, `candidate_judged`, `c2_delivery_decided`,
`product_disposition_explicit`, `lesson_uptake_observed`, and
`decision_world_exported`. Valid no-action/no-delivery decisions terminate the
live experiment honestly but do not satisfy the slice's delivery milestone.

Ticket:

```text
draft -> admitted -> ready -> claimed -> submitted -> verified -> completed
submitted -> changes_requested -> claimed
claimed -> expired -> ready
draft | admitted -> rejected
admitted | ready | claimed -> cancelled
submitted | verified -> failed
completed | failed | cancelled -> reopened -> admitted
```

The charter creates these Ticket identities; later scope changes create
revisions rather than ad hoc replacement files:

```text
T00 qualify deterministic machinery
T01 map source and contract lineage
T02 challenge boundaries and counterexamples
T03 run behavior and documentation matrices
T04 produce and adjudicate the C1 curated account
T05 build and judge the authorized candidate
T06A run opaque baseline fluency probe
T06B run opaque candidate fluency probe
T07 conduct adversarial product/curation review
T08 produce and adjudicate the C2 descendant account
T09 materialize, validate, authorize, and deliver exact product commit
T10 target and observe one L1 lesson application
T11 export the C1 decision world, retrospect, and close
```

Where one Ticket contains several authorities—particularly T04, T08, and
T09—its acceptance conditions point to independently authorized substate. The
actor submission cannot complete the Ticket by itself.

Adversarial review:

```text
requested -> assigned -> active -> findings_submitted -> responses_due
responses_due -> resolved | accepted_risk | superseded | escalated
active -> failed | expired
resolved | accepted_risk -> reopened
```

VS-001 requires one combined `ProductAndApi`, `CompatibilityAndMigration`,
`CurationAndDisclosure`, and `CostAndEfficiency` review at T07. Every finding
is a `ReviewChallenge`; narrative approval and an empty chat response cannot
advance `reviewed`.

Coordination pulses are emitted without model calls after bootstrap, both C1
and C2 decisions, any blocker, cost reaching 50% or 80% of a scope cap, any
review escalation, delivery, and close. Each pulse is a rebuildable typed view
over the latest event cursor; acknowledgement has no epistemic effect.

Episode:

```text
framed -> admitted -> investigating -> prototype_deliberating
prototype_deliberating -> prototyping | investigating | closed_no_action
prototyping -> candidate_validating
candidate_validating -> prototyping | delivery_deliberating
delivery_deliberating -> delivery_authorized | prototyping | closed_no_delivery
delivery_authorized -> materializing -> observing -> learning -> closed
```

The legal lifecycle side paths are:

```text
any nonterminal lifecycle state -> abandoned
materializing -> delivery_authorized | reverted
observing | learning | closed | closed_no_action | closed_no_delivery -> reopened
reopened -> investigating
```

`active | blocked` is an orthogonal operational condition with a recorded
blocker, owner, and `blocked_from_lifecycle_state`. Clearing the blocker resumes
the same lifecycle state; it does not guess a transition from a generic
`blocked` bucket. Cancellation and abandonment are explicit terminal decisions.

The C1 prototype decision and C2 delivery decision are different graph
revisions and different lifecycle gates. A conflict is a graph object, not an
operational terminal state. Either decision may preserve a conflict, defer
action, or authorize further inquiry without deleting disagreement.

`closed_no_action` and `closed_no_delivery` are valid kernel outcomes so the
Grand Architect is never coerced to authorize a weak product merely to satisfy the
demo. They resolve or stop that episode honestly but do not satisfy the VS-001
completion contract, which requires one delivered reconciliation commit. A
later descendant episode may continue from the recorded rejection; the rejected
decision is not rewritten into success.

Attempt:

```text
registered -> claimed -> preparing -> running
           -> process_finished -> evidence_sealed
           -> protocol_validated -> judged -> accepted | rejected
```

Cancellation, expiry, protocol failure, supervisor failure, SDK-host failure,
model failure, and
judge failure are separate terminal classifications with attempt lineage for a
retry. Pi event settlement, assistant stop reason, process exit, submission
validity, and judge disposition remain separate receipt fields; none is inferred
from another. Retrying never overwrites the failed attempt.

Product change:

```text
prototype_authorized -> worktree_ready -> candidate_submitted
                     -> candidate_validated -> reviewed
                     -> delivery_authorized -> materialized
                     -> commit_validated -> delivery_ready -> delivered
delivered -> observed | reverted
delivery_authorized | materialized | commit_validated | delivery_ready -> reverted
```

The builder produces `candidate_submitted`; only the deterministic materializer
can produce `materialized` and `commit_validated` after C2 authorization. This
prevents an actor-created commit from becoming authoritative merely by existing.

Lesson epistemic status:

```text
candidate -> validated | rejected
validated -> promoted_l1 | expired
promoted_l1 -> expired | downgraded | revoked
```

Propagation target status:

```text
targeted -> delivered -> encountered -> applied_once
         -> causally_supported -> institutionalized

any nonterminal target -> missed | contaminated | retracted
```

VS-001 stops at `applied_once`. That observation does not establish causal
support or institutionalization. The separately enacted product test is
technical heredity: an exact XSH invariant authorized by C2, not an L1 lesson
pretending to have L4 epistemic support.

### Initial capabilities

```text
bootstrap_principal:
  create_society_identity, install_initial_grand_architect_office,
  install_founding_universe_seed, appoint_initial_grand_architect,
  set_r0_hard_ceiling, bootstrap_society,
  admit_initial_pi_sdk_qualification, admit_initial_operating_cycle;
  each founding capability is consumed exactly once

TheGrandArchitect:
  propose_and_ratify_descendant_universe_seed,
  propose_admit_quiesce_resume_and_close_successor_cycle,
  charter_project, allocate,
  decide_c1, decide_c2, accept_curation, resolve_review_challenge,
  authorize_prototype, authorize_delivery, deliver_product,
  promote_lesson_l1, register_propagation_target,
  create_disclosure_frontier, accept_risk, request_cancellation, reopen

host_operator:
  observe typed views and mandatory monitor stream; request cycle-scoped
  graceful cancellation or emergency stop; no epistemic, Project, product,
  budget-enlargement, seed, or Office-decision authority

project_steward:
  revise_ticket_within_charter, admit_ready_work, allocate_reserved_project_work,
  acknowledge_coordination_pulse; no seed, C2, C3, or delivery authority

pi_supervisor:
  claim_work, start_attempt, seal_content_object, register_forensic_manifest,
  complete_attempt, start_and_settle_office_turn,
  submit_attested_contribution, record_pi_boundary_and_child_lifecycle;
  no Project, decision, or budget-enlargement authority

inquiry_actor:
  no direct durable capability

product_actor:
  no direct durable capability; OS access only inside assigned worktree by
  operating convention

deterministic_judge:
  admit_evidence, record_observation, and record_validation for one experiment
  and evaluator revision

curation_service:
  propose_curation or record_curation_challenge on behalf of one admitted actor
  contribution; no acceptance authority

product_materializer:
  materialize one delivery-authorized patch, record its product commit, and
  submit validation evidence; no decision or delivery authority

propagation_observer:
  record one target-scoped delivery, encounter, or application observation

influence_projector:
  derive typed signals and candidates from admitted sources; no command,
  scheduling, review-disposition, or blocking authority

cost_supervisor:
  record normalized cost, fence admission and request cancellation on exact
  cap or unknown-cost rules; no authority to enlarge a budget

cancellation_supervisor:
  propagate an accepted request, send SDK abort and process signals, record
  liveness/reap evidence, and advance only the named request; no authority to
  originate a cancellation except through a registered R0 circuit breaker

projector:
  read events and revisions, advance only its projection cursor
```

Kernel capabilities govern institutional state. They do not imply that a
native Pi actor is OS-confined; that is an explicitly separate execution fact.
The Grand Architect's capability belongs to the Office occupancy and is
exercised only through validated Office submissions. Possession of the daemon
socket, a PID, a workspace, or a Pi transcript is never an authority token.

### Sealed-object and evidence-admission contracts

The content-object store has one semantic operation: seal bytes under a digest.
It refuses different bytes at an existing digest location. Length, physical
path, and ingest time are storage observations derivable during verification;
they are not fields in the curated provenance contract.

A forensic manifest records:

```text
producing_attempt_or_command
closed_object_roles[] { role, digest, media_or_schema_contract }
capture_or_normalization_policy
retention_and_access_class
```

The closed role prevents a session from being substituted where an evaluator
output is required. It does not claim the object is decision-relevant.

An evidence admission separately records:

```text
content_object_or_typed_observation
experiment_and_evaluator_revision
semantic_role
claim_or_question_relation
applicability_scope
known_capture_limitations
admitting_authority
```

VS-001 seals assignments, context packs, Pi sessions, Pi JSON events, stderr,
submissions, source snapshots, experiment manifests, evaluator outputs, Git
diffs, commits/bundles where needed, test logs, decision prose, replay packets,
and outcome observations. Most remain forensic. Only explicitly admitted
evidence can support an Observation, Argument, curation item, or decision.

The database may refer to a sealed content object only after durable seal
succeeds. A curated account may refer to evidence only after admission succeeds.
These are different transaction gates.

### Projection contract

The first projection is one self-contained episode packet:

```text
VS-001.md
VS-001-C1-DECISION-WORLD.md
VS-001-COORDINATION-PULSE.md
VS-001-GRAND-ARCHITECT-BRIEF.md
VS-001-OPERATING-CYCLE.md
```

It shows the Universe Seed and Project revisions, graph revisions, Tickets,
attempts, observations, hypotheses,
arguments, conflict, accepted curation and exclusions, decisions, product
lineage, ReviewChallenges and dispositions, cost reservation and spend,
influence decisions, validation, outcome obligations, lesson, propagation
observations, and disclosure frontier. Every semantic assertion links to a
typed row, object revision, or admitted evidence; forensic objects are
reachable through a separate expansion path and are not rendered as
equal-weight provenance. Each packet names the latest consumed event id and
rebuilds byte-identically after nondeterministic presentation fields and
absolute runtime-root paths are normalized.

The Operating Cycle projection is the exact operations view needed during a
run. It shows pinned seed/occupancy/organization/model/admission/trace-policy
revisions; current lifecycle and admission generation; Project, cycle, and
society budget constraints; Office session/turn state and spend; each admitted
Attempt, lease, workspace, and child process; current cancellation roots and
deadlines; reconciliation blockers; open successor dispositions; last durable
notice cursor; and counts of coalesced or dropped trace renderings. It answers
“what is alive, authorized, affordable, cancelling, or preventing closure?”
without scraping a process table or log file.

There is no machine JSON projection. Machine consumers query typed Rust views
over SQLite. Markdown is a disposable human/actor surface and cannot be parsed
back into authority. The decision-world Markdown is likewise a projection of a
typed `DisclosureFrontier` and frontier-local identity map.

The decision-world projection includes only frontier members and opaque source
identities which do not reveal the aftermath. A test build intentionally asks
for a post-frontier lesson, outcome, current source snapshot, and raw artifact;
all four requests must be denied and recorded as contamination attempts.

## Exact experiment package

### Pinned inputs

Episode admission snapshots:

- the clean XSH base commit current when VS-001 begins;
- `LANG.md`, `docs/SPEC.md`, `docs/SPEC-OS.md`, relevant API registry entries,
  managed-spawn lowering/runtime owners, and focused process tests at that
  commit;
- the two Laputa QEMU TODO call sites as external evidence, without giving V2
  authority over the Laputa repository;
- representative Factory V1 `spawn ... stderr:` call sites as historical
  evidence, with their recorded execution environments, not imported
  implementation;
- Pi source and executable at exactly version 0.83.0;
- all actor configurations, assignments, system prompts, tool profiles,
  evaluator programs, and task fixtures; and
- the initial organization configuration and resource envelope.

The planning reference was XSH `04fb98f8c63b63cccffce7ef2c3cabde81bb05ba`.
Execution must query and record the then-current clean `HEAD`; it must not assume
that planning reference remains current.

### Behavior matrix

The deterministic experiment builds the pinned XSH binary and exercises each
public command consumer that might own the contract:

| Consumer | Why it is included |
| --- | --- |
| `process.run(command)` | Completed-command control for the typed plan |
| `spawn command` + `wait` | Primary owned concurrent path in the question |
| `process.spawn(command)` | Lower-level detached path named by the public contract |
| `spawn run ...` | Syntax control; establishes whether direct run redirection differs intentionally |

The minimum behavioral cases are:

1. default stderr inherits to the parent;
2. `stderr: Path` redirects exact child bytes and removes them from parent
   stderr;
3. stdout remains independently inherited or redirected as declared;
4. non-append stderr truncates a pre-existing destination;
5. `stderr_append: true` appends without altering stdout policy;
6. `/dev/null` is an ordinary Path sink;
7. an invalid destination returns a setup error before a managed handle becomes
   observable;
8. a nonzero child exit remains status data and does not erase captured stderr;
9. wait and cancellation close the redirected stream and reap the owned child
   group; and
10. command construction, managed spawn, and any relevant lowered path agree on
    the same input.

Where a consumer is intentionally detached or cannot expose the same lifecycle,
the evaluator records `not_applicable` with a contract citation rather than
forcing false parity.

Each case emits a closed observation record:

```text
case_id
consumer
input_manifest
expected_contract_source
exit_or_error_shape
parent_stdout_digest
parent_stderr_digest
redirected_file_digest
process_lifecycle_receipt
pass | fail | not_applicable
```

The evaluator is checked into the V2 circuit package. An actor cannot edit it in
the attempt which the evaluator judges.

### Documentation and discovery matrix

The experiment separately asks what a user or actor can learn from:

- the normative `docs/SPEC.md` process and builder sections;
- `docs/SPEC-OS.md` process-ownership guidance;
- exact `xsht api` entries for `process.command_argv`, `process.spawn`, and
  spawn-related reference navigation;
- the open proposal in `LANG.md`; and
- source behavior.

For every source it records the claimed stdin/stdout/stderr fields, consumer,
default, append/truncate rule, ownership semantics, and error behavior. A
machine-produced conflict report identifies missing or incompatible cells. This
report is an `Observation`; the conclusion that a source is stale is an
`Argument`.

### Negative controls

The package proves its judges can fail:

- a command which omits the stderr field fails the redirect case;
- a shell wrapper which uses `sh -c '... 2>...'` may satisfy byte behavior but
  fails the typed-boundary constraint;
- a fake task solution which writes the expected log without spawning the child
  fails lifecycle and varying-payload cases;
- a candidate documentation patch that leaves `LANG.md` contradictory fails
  contract reconciliation; and
- a candidate runtime patch that changes default inheritance fails compatibility.

These run without Pi and before any paid episode work.

## The paired agent-fluency probe

The probe asks whether the effective XSH contract is usable by an agent. It is
evidence for this decision, not a benchmark leaderboard.

### Task

Each task actor receives the same `TASK.md`:

> Write `supervise.xsh`. It receives paths to an XSH binary, a child XSH script,
> and an error-log destination. Start the child concurrently through XSH's typed
> process API, redirect only child stderr to the destination, preserve child
> stdout, wait through an owned handle, and propagate a nonzero child status.
> Do not invoke a shell, construct a shell command string, use `process.spawn`,
> or fake the child's output. The evaluator will vary paths, output, prior log
> contents, and exit status.

The prohibition on `process.spawn` keeps the task focused on the owned
`spawn command` contract rather than detached lifecycle polling.

### Fixture workspace

```text
work/
├── TASK.md
├── REFERENCE.md
├── bin/
│   ├── xsh
│   └── xsht
├── fixtures/
│   └── noisy-child.xsh
└── output/
```

The varying cases are registered as typed `FluencyProbeCase` rows and selected
by the deterministic judge. They are not a fixture JSON file. Their child
arguments, expected stream policy, preexisting-log condition, and expected exit
class are closed Rust fields; the generated invocation is recorded in SQLite
and relevant output bytes are sealed.

The front of `PATH` points to the assigned `bin/`, and the evaluator invokes the
produced script with the assigned binary by absolute path. The task actor may
inspect `REFERENCE.md` and run `xsht api`; it receives no XSH source checkout.
Because the host remains reachable, the attempt receipt flags any tool event
which names known XSH, V1, or treatment paths outside its workspace. Such access
does not disappear; it is a contamination finding.

### Treatments

`A — baseline`

Uses the pinned base XSH/Xsht binaries and a reference pack generated from the
base specification and API registry.

`B — candidate`

Uses binaries and a reference pack generated from the prototype tree. If the
candidate is documentation/API-only, the runtime binary may be byte-identical
while `REFERENCE.md` and Xsht registry output differ. If a runtime correction is
needed, both binary and reference inputs differ together because the product
proposal is one coherent treatment.

The two workspaces use opaque treatment labels until their submissions are
sealed. They pin the same Pi actor configuration, model, thinking level, tool
profile, assignment, wall budget, and fixtures. They run as fresh sessions with
no shared actor memory.

One actor per arm is sufficient only to prove the experimental path. The
decision packet may use the results as qualitative or case evidence; it cannot
claim a population-level agent improvement. Later organization science should
replicate the probe across tasks and seeds.

This is VS-001's first **improvement-productivity microprobe**. It asks whether
the candidate XSH world makes one future systems task easier to understand and
implement under a fixed citizen and organization configuration. It deliberately
does not permit the actor population or circuit to adapt, so it cannot reveal
new professions or social forms enabled by the candidate language world.

The result remains a vector: contract discovery, correct construction,
ownership safety, workaround use, interaction cost, and contamination. A
documentation-only improvement can be consequential if it removes false
inference and shell escape; a locally shorter solution is not automatically
better if ownership or error semantics become implicit. The C2 authority weighs
this evidence with human legibility, runtime compatibility, implementation
simplicity, and the deterministic contract matrix.

### Task-actor judges

The package checks:

- `xsht check` accepts the script;
- the source uses a typed command and owned `spawn`/`wait` path;
- no shell wrapper, shell string, `process.spawn`, or hard-coded fixture output
  is present;
- inherited stdout is byte-exact;
- redirected stderr is byte-exact and absent from parent stderr;
- a pre-existing log is truncated under the chosen default;
- paths containing spaces work without quoting tricks;
- a nonzero child status produces the declared nonzero supervisor behavior;
- the child is reaped; and
- tool errors, turns, wall time, tokens, optional reasoning tokens, and cost are
  reported separately from correctness.

The judge returns a vector, not a combined score:

```text
correctness cases
typed-boundary compliance
ownership/lifecycle compliance
contract-discovery path
tool failures
turn and resource use
unexpected host-path access
```

## Historical decision-world seed

VS-001 does not compare organization variants, but it must leave behind the
first world on which such comparison can later be honest. `W1` reconstructs the
moment immediately before the C1 prototype decision.

Its positive frontier contains:

- the Universe Seed revision, Project charter, objective, question, resolution condition, hard
  constraints, and preregistered predictions;
- H1/H2/H3 and their status at the frontier;
- the pinned XSH repository/source snapshots and external call-site evidence
  available to the episode;
- the qualified organization configuration, actor treatment configurations,
  capabilities, resource envelope, execution profile, and circuit revision;
- deterministic behavior and documentation observations admitted before C1;
- validated inquiry contributions and preserved conflicts;
- the accepted C1 curated account, including exclusions and raw-evidence
  escalations; and
- only cultural lessons and policies which were active before episode admission.

It sequesters:

- the C1 choice itself and all explanation text which reveals it;
- product worktree, candidate patch, changed tree, and validation results;
- paired-task treatments, outputs, and label mapping;
- product-adversary arguments and C2 curation/decision;
- materialized commit, current post-delivery XSH checkout, and derived API pack;
- immediate and delayed outcomes, retrospective, invariant, and L1 lesson; and
- raw session material not explicitly admitted into the frontier.

Current paths and friendly identifiers are replaced by frontier-local opaque
identities where their names would disclose an outcome. The exported source
checkout is reconstructed from the pinned ancestral commit; it is never a view
of the current XSH working tree.

Frontier qualification performs negative reads as four principals: replay
actor, projector, ordinary investigator, and Grand Architect query client without an
explicit forensic override. It tries direct identity, graph traversal, object
digest, current repository path, culture lookup, and projection lookup for
seeded aftermath records. All accesses must fail closed and produce a
contamination audit event outside `W1`.

The trusted machinery cannot remove outcome knowledge already latent in a
foundation model. VS-001 records model identity and treats recognizable prior
knowledge or unexplained answer leakage as contamination. Future replay suites
should prefer genuinely private cases, opaque identifiers, and outcomes which
postdate the model when practical. The frontier guarantees institutional
non-disclosure, not model amnesia.

The aftermath bundle remains linked but sealed from replay participants. It
contains the original decisions, predictions, action, costs, outcomes, and
later reversals. A future organization trial preregisters its proposed decision
before an independent judge opens only the aftermath fields required for
comparison.

## Checked-propagation uptake probe

The L1 lesson's first target is a new, explicitly matching inquiry:

> A downstream supervisor needs to capture child stdout while retaining typed
> process ownership. Determine whether XSH needs a new process API. Before
> proposing anything, compare the normative API registry, executable behavior,
> active proposal corpus, and representative real call sites. Recommend a new
> API, use of an existing contract, or further experiment, and state remaining
> uncertainty. Do not edit XSH.

The target context contains the exact L1 revision, its L1 status, applicability
scope, supporting episode, exclusions, and explicit statement that it is
candidate guidance rather than policy. It excludes the VS-001 retrospective and
prior actor prose so the investigator cannot simply repeat their conclusion.

Propagation observations have separate judges:

```text
delivered
  context manifest contains the exact target and lesson revisions

encountered
  persisted Pi input contains that manifest and the valid submission names the
  lesson revision it processed

applied_once
  submission identifies evidence from all four required record classes before
  making its recommendation, or explicitly explains why a class is unavailable
```

Mentioning the lesson without doing the comparisons fails application. Doing
the comparisons while reaching a different recommendation can pass; the lesson
is a method, not a required conclusion. Accessing forbidden VS-001 sessions or
post-target material marks contamination rather than success.

This probe has no baseline arm, so it cannot establish `causally_supported`.
Its purpose is to prove the transduction machinery and observe one uptake. A
later Stage 4 trial should compare lesson/no-lesson or prior/new propagation
policies across several blinded inquiries and sample non-target work for
contamination.

## Initial actor population

VS-001 uses nine Pi actor instances and deterministic services. One instance
occupies `TheGrandArchitect`; eight are episode-local task actors. The friendly
task names below are experimental labels, not recognized professions or kernel
types. Each configuration records broad attractor biases and signal responses;
its digest, not the friendly name, is authoritative.

All nine instances use `ActorModelPolicyV1`: Pi SDK 0.83.0, `openrouter`,
`deepseek/deepseek-v4-flash-0731`, and `high` thinking. The Grand Architect has
one persistent SDK Office session. The curator instance executes two separate
one-shot attempts at the C1 and C2 frontiers, so the other eight instances
produce nine task Attempts in total. No task instance shares a Pi session,
hidden conversation, or fallback model with another. The Office session
persists across its own bounded turns by design but receives task results only
through typed decision packets and eligible notice batches.

“Independent” in this slice means separate instance, session, workspace,
assignment frontier, contribution, and no access to the sibling output. It does
not mean foundation-model independence: every paid actor deliberately uses the
same required DeepSeek model. Review and influence records expose that shared
model ancestry and do not count the two attempts as fully independent
replications.

### 0. Grand Architect occupant

- attractor bias: `orient + integrate + govern + remember`;
- profile: `grand_architect_office_v1` over the Pi SDK adapter with read-only projection
  access and closed Office submissions;
- lifetime: one Office session for exactly `OC-VS-001`;
- sees: the Universe Seed, Office contract, cycle charter, typed Project and
  decision views, curated accounts, eligible OperationalNotices, budget and
  cancellation state, and requested source expansions;
- must: charter the Project, allocate contingency, resolve curation/review,
  make C1/C2/product/lesson decisions, govern recovery, and order cycle close;
  and
- cannot: write SQLite, execute SQL, spawn Pi, alter a judge, enlarge a hard
  ceiling, turn raw logs into provenance, or make narrative text authoritative.

### 1. Contract cartographer

- attractor bias: `explore + remember + synthesize`;
- responds to: `unresolved_contract_conflict`;
- profile: `read_source_v1`;
- workspace: detached XSH read worktree;
- sees: question, hypotheses, pinned source/doc paths, no other actor output;
- must: map every stdio field from syntax and registry through lowering to each
  runtime consumer and identify contradictory records; and
- cannot: propose a product diff as if mapping alone authorized it.

### 2. Boundary skeptic

- attractor bias: `challenge + measure` with low implementation authority;
- responds to: `missing_behavioral_evidence` and high-confidence contradiction;
- profile: `read_source_v1`;
- workspace: independent detached XSH read worktree;
- sees: the same question and hard constraints, but not the cartographer's
  contribution;
- must: search for counterexamples involving detached ownership, setup failure,
  append behavior, cancellation, and misleading V1 precedent; and
- cannot: resolve the question by majority agreement.

### 3. Decision-account curator

- attractor bias: `synthesize + remember`, with explicit dissent preservation;
- responds to: admitted evidence ready for decision and unresolved curation
  exclusions;
- profile: `curator_v1`;
- workspace: fixture directory containing admitted evidence and contribution
  projections, not an XSH checkout;
- sees: pre-agent hypotheses, deterministic observations, validated inquiry
  submissions, source references, and the curation contract, but no raw Pi
  session by default;
- must: propose the C1 curated account, including selection reasons, strongest
  disagreement, unknowns, exclusions, and any scoped raw-evidence request; and
- cannot: accept its own account, authorize a prototype, promote a lesson, or
  turn an omitted source into a negative finding.

### 4. Prototype builder

- attractor bias: `build + integrate` under a narrow authorized contract;
- profile: `product_builder_v1`;
- workspace: XSH product worktree at the pinned base;
- sees: a Grand-Architect-authorized C1 prototype decision packet, the admitted evidence,
  preserved conflict, exact expected contract, and required XSH gates;
- must: implement the smallest coherent candidate, update all canonical owners,
  remove or revise the stale `LANG.md` proposal, and leave a portable patch; and
- does not: commit, merge, or change the evaluator.

### 5–6. Paired task actors

- attractor bias: matched `build + explore`; neither configuration is selected
  or reproduced from this one comparison;
- profile: `task_actor_v1`;
- workspace: independent fixture directories, one per opaque treatment;
- sees: only the task and its treatment's declared reference/binary inputs; and
- must: produce `supervise.xsh` plus a valid submission.

### 7. Product adversary

- attractor bias: `challenge + integrate + measure`;
- profile: `read_source_v1`;
- workspace: fresh read worktree materialized with the candidate patch;
- sees: question, decision dimensions, evidence matrix, candidate diff, tests,
  paired-trial results, accepted curation, and stated exclusions;
- must: challenge contract coherence, missing docs/tests, compatibility,
  lifecycle safety, treatment interpretation, product readiness, and any
  decision-relevant omission from the curated account; and
- cannot: modify the candidate or deliver it.

### 8. Lesson-uptake investigator

- attractor bias: `explore + challenge`; it is a fresh instance of an
  independently registered inquiry configuration with no session inheritance
  from actors 1 or 2;
- responds to: the explicitly targeted L1 lesson and a new process-API question;
- profile: `read_source_v1`;
- workspace: fresh read worktree at delivered XSH `HEAD`;
- sees: a new question about whether supervised stdout capture requires a new
  XSH API, the exact promoted L1 lesson revision, and ordinary declared source
  paths; it does not see the VS-001 retrospective or prior actor submissions;
- must: compare normative registry, executable behavior, active proposal corpus,
  and real call sites before recommending a new API or no change; and
- cannot: establish that the lesson caused its behavior, edit XSH, or promote
  the lesson beyond L1.

The active Grand Architect Office owns curation acceptance, C1 prototype
admission, the C2 product decision, L1 promotion, disclosure-frontier creation,
ReviewChallenge disposition, and delivery. Deterministic judges own behavioral,
cost-normalization, and propagation observations. Every use of Office authority
is attributed to an `OfficeTurn`, parsed from a closed submission, and accepted
by the same capability checks used for any future user occupant.

## Pre-registered predictions

Before inquiry attempts run, VS-001 records:

1. Current evidence makes `H2` most likely: managed `spawn command` will honor
   `stderr: Path` correctly, while `LANG.md` and some normative/discovery text
   remain stale or incomplete.
2. At least one consumer or documentation source will require an explicit
   qualification, preventing a one-line “already implemented” closeout.
3. The baseline task actor may solve the task by source/API archaeology, but
   the candidate actor should require no shell workaround and should find a
   direct documented path.
4. The paired trial is too small to establish a reliable token/turn reduction;
   any such difference remains a tentative observation.
5. A coherent reconciliation commit will be smaller and safer than adding a new
   `spawn` parameter, because stderr policy already belongs to `Command`.
6. Existing spawn, wait, cancellation, process, API, and documentation gates
   will remain green after the candidate.
7. A compact curated account can support both decisions without including raw
   Pi transcripts; any transcript access should resolve a named uncertainty.
8. A fresh investigator explicitly given the L1 lesson will compare all four
   named institutional records before proposing a stdout-capture API, but the
   single unpaired observation will not establish causal propagation impact.
9. VS-001 will establish one discovery/delivery cycle and bounded propagation
   uptake while producing no evidence of metamorphosis.

If the behavior matrix contradicts prediction 1, the episode revises the
argument and follows `H1` or `H3`; it does not change the prediction text.

## Exact episode execution

### Step 0: qualify, originate, and charter

Before a paid call, native tests use Pi process doubles and deterministic XSH
fixtures to prove command transactions, daemon single-writer recovery, local
protocol rejection, `AdapterReady`/`CreateSession` fencing, SDK-host normalization,
tracing redaction and level policy, cancellation/reaping, evaluator negatives,
evidence admission, curation exclusion, disclosure-frontier denial,
propagation-state separation, and projection rebuild. The kernel records a
qualified organization configuration.

The bootstrap principal starts `societyd`, creates the Society identity,
installs the founding Universe Seed revision 1, installs the Grand Architect
Office with its initial actor occupant, records the R0 hard ceiling, and calls
`BootstrapSociety` against the exact seed, Office, organization, model-policy,
trace-policy, and budget revisions. Founding installation is a one-time root
operation, not a human ratification tier; later seed revisions require C4
proposal, challenge, and Grand Architect ratification.

The same principal consumes its one-time capability to propose and admit
the separate `$0.03` SDK qualification treatment: one one-shot session and one
persistent Office-shaped session with no Office capabilities. Exact Node,
adapter, lock, SDK construction, model, event, usage, abort, disposal, and cost
checks produce `PiSdkQualificationV1`; failure stops bootstrap and cannot borrow
from VS-001. The qualification sessions are not discovery actors and cannot
issue society commands.

The principal then consumes its one-time capability to propose and admit
`OC-VS-001`. `PiSupervisor` starts the Grand Architect SDK Office session
through the inert-host handshake. Its first SDK `prompt()` includes the cycle charter and
empty decision surface. The occupant's first valid Office submission charters
Project `P-VS-001-SPAWN-STDERR`, creates T00–T11 and their typed acceptance
conditions, reserves the `$1.00` Project envelope against the already active
cycle and society constraints, and requests the first deterministic
coordination pulse. The kernel commits those commands individually and emits
the corresponding notices/traces; the chat response itself does nothing.

T00 also runs one Pi double that reports a paid assistant response with an
unavailable cost. The cost breaker atomically quiesces the cycle, increments its
admission generation, and requests `GracefulCancel` for the synthetic Attempt.
The supervisor must prevent any reserved stale child from receiving
`CreateSession`, abort
and terminate the owned double, reap it, seal partial evidence, conservatively
charge its full reserved maximum, and trigger a `CostAccountingFailure`
Postmortem. Its ledger-derived timeline, independent deterministic challenge,
containment, causal claims, and one separately judged hardening Ticket pass
through the complete Postmortem lifecycle.

The Grand Architect receives one high-priority recovery notice batch and uses a
control-purpose Office turn to dispose the Postmortem action. Only after the
unknown charge, cancellation propagation, child, Attempt, and reservation are
terminal may it resume the drained Operating Cycle under the incremented
generation. This is a fault-injection proof, not live model spend or a claim
that a provider incident occurred in VS-001 product work.

Gate: no paid work may start while a required R0 test or negative control
fails, while the society lacks exactly one active seed or Grand Architect
occupant, while the Office SDK session is not `ready`, while the Operating
Cycle is not `running`, or while any Project/Ticket/Attempt fails to cite the
active seed and admitted cycle event.

For every later Grand Architect gate, the same control loop applies: a committed
readiness transition derives a `decision_due` or higher-severity
`OperationalNotice`; the notice projector forms one bounded packet at a named
ledger frontier; the SDK adapter opens an Office turn with `Prompt` or queues it
with `FollowUp`; Pi settles; the closed Office submission is parsed; and each
proposed command is independently authorized and committed. The Office turn is
then cost-reconciled before another ordinary turn opens. Invalid submissions,
stale generations, and rejected commands remain visible and do not advance the
gate. `steer` is used only to correct an urgent stale or unsafe premise while a
turn is streaming, never to optimize conversational style.

### Step 1: frame and admit

Through its next Office turn, the Grand Architect creates the Objective,
Question, three Hypotheses, hard
constraints, resolution condition, Predictions, initial source Observations,
and an Experiment proposal. The kernel pins XSH and Pi inputs, configuration,
budget, and evaluator revisions in one admitted Episode.

The influence projector derives `unresolved_contract_conflict` from the
admitted incompatible normative records and `missing_behavioral_evidence` from
the open hypotheses and absent matrix observations. Each signal records its
eligible source set, scope, family revision, uncertainty, pressure inputs,
decay, and full provenance. They produce bounded attention bids to T01 and T02
and matched retrieval candidates for the cartographer and skeptic contexts.
Project readiness, Grand Architect charter, reservations, and leases—not the
signals—authorize model work.

Gate: the episode projection must reconstruct the pre-agent belief state and
derive both demand signals without creating new epistemic claims. A synthetic
high-volume set of duplicate source records must not increase pressure or
displace an independent signal.

### Step 2: run independent inquiry

The cartographer and skeptic configurations independently emit scoped responses
to their matching signals. The scheduler verifies fit, independence,
capability, budget, and WIP, then creates the two already-preregistered treatment
work items concurrently. Each claims a lease and runs Pi in its own native read
worktree.

`PiSupervisor` seals sessions, event streams, submissions, Git receipts, and
normalized usage into forensic manifests. It admits execution classification
facts but does not turn narrative chat or every tool event into graph evidence.
Validated submissions become contribution proposals.

Known cost updates every applicable constraint and the next coordination pulse.
At 50% and 80% of an Attempt, Office-session, Project, or Operating Cycle cap,
`cost_reserve_at_risk` may become an eligible
visibility signal for the Grand Architect brief. It cannot cancel work; the R0
budget rule cancels independently at the exact cap or unknown-cost condition.
This separation proves that provenance can shape attention without turning a
derived score into authority.

Gate: both independent outcomes must remain visible. At least one valid
contribution is required to continue; one actor's failure does not erase the
other's result, and a retry is a new attempt requiring a Grand Architect
contingency allocation.

### Step 3: execute deterministic discovery and behavior judges

The laboratory runs the behavior and documentation matrices. Judge output is
sealed first, then semantically admitted under the exact evaluator revision.
Admitted results become Observation nodes. Actor claims become Arguments only
after their cited source revisions resolve; contradictions become a Conflict
node rather than a manager summary.

Gate: every observation resolves to admitted evidence, every admitted object
resolves to a forensic manifest, and all negative controls have previously
demonstrated their expected failure.

### Step 4: curate the C1 decision account

The Grand Architect creates a curator disclosure frontier containing the pre-agent
graph, admitted deterministic observations, validated inquiry contributions,
and named source references. It excludes raw Pi sessions and narrative output.
The decision-account curator runs against only that frontier and proposes the
closed C1 account.

If the curator requests raw evidence, the request names a question and object.
An authorized descendant attempt receives only that object and records whether
it changed the account. The Grand Architect accepts, challenges, or rejects the
proposed account; acceptance preserves its exclusions and any dissent.

Gate: the accepted account must reconstruct H1/H2/H3, the strongest evidence
against the leading hypothesis, unresolved unknowns, and why excluded material
is not currently decision-relevant. Raw access with no named question fails the
gate.

### Step 5: deliberate and authorize one prototype

The episode enters `prototype_deliberating`. A Grand Architect packet presents the
accepted curated account, its source expansion path, H1/H2/H3, evidence,
unknowns, proposed product contracts, no-change, and costs. Immediately before
the decision, the Grand Architect seals the immutable C1 replay frontier:
everything a future counterfactual authority may know when attempting the same
decision.

The Grand Architect records a C1 Decision authorizing exactly one prototype contract
and changed-path boundary, returning to inquiry, or closing with no action.

Gate: the prototype builder cannot start from an Argument or queue entry; it
requires the exact decision revision and capability grant.

### Step 6: build the coherent candidate

The prototype builder receives a product worktree and implements the authorized
contract. `societyd` captures the patch and tree digest. Focused deterministic
gates run first, followed by the relevant broader XSH gate named by
`docs/TEST-MAP.md`. Formatting and pre-commit hooks are not run.

The expected `H2` patch must at least:

- correct the normative spawn/Command stdio description;
- make exact API/reference discovery sufficient;
- add or tighten an observable behavior test for owned spawn stderr;
- remove the implemented/stale proposal from `LANG.md`; and
- leave runtime code unchanged unless evidence demonstrates a real gap.

Gate: the patch must be portable to a fresh worktree, touch only authorized
paths, and make the contract matrix internally consistent.

### Step 7: run the paired fluency probe

The laboratory builds and seals baseline and candidate XSH/Xsht inputs,
generates their reference packs, assigns opaque labels, and runs the two fresh
task actors. It evaluates both through the same judge and reveals treatment
mapping only after both submissions are sealed.

Gate: treatment contamination, missing settlement, invalid submission, or
evaluator failure remains a distinct outcome and cannot be imputed as task
failure.

### Step 8: adversarial product and curation review

The product adversary executes the required `AdversarialReview` against a fresh
materialization plus all admitted evidence. Deterministic tests remain
authoritative for their domains. The adversary submits typed
`ReviewChallenge`s covering product/API coherence, compatibility/migration,
curation/disclosure, and cost/efficiency. It also receives the accepted C1
account and exclusions. A decision-relevant omission becomes a curation
challenge linked to the omitted evidence; it is not silently pasted into the
old account.

Gate: every finding receives an evidence-backed response, correction, scope
narrowing, escalation, or Grand Architect accepted-risk disposition. Required
unresolved findings create linked conflicts, a descendant curation, or a
descendant implementation attempt. Narrative approval alone cannot set
`reviewed` or authorize delivery.

### Step 9: curate and make the C2 decision

The decision-account curator runs a new bounded session over the accepted C1
account plus candidate, validation, fluency-probe, challenge, and adversarial
evidence. It proposes a descendant C2 account. The original account and any
defeated omission challenge remain unchanged.

The Grand Architect packet contains:

- the original predictions and current hypothesis status;
- the behavior and documentation matrices;
- independent inquiry and dissent;
- exact patch, tree digest, test receipts, and adversarial findings;
- paired task-actor vectors with contamination status;
- alternatives, including narrower patch and no delivery;
- expected short- and delayed-horizon outcomes; and
- rollback/reopen triggers.

The Grand Architect first accepts or challenges the C2 account, then accepts the exact
patch, requests a descendant, defers, or rejects delivery. The decision does not
edit the C1 decision, either curation revision, or earlier claims.

### Step 10: materialize and deliver the product commit

On acceptance, a deterministic product materializer:

1. creates a fresh worktree at the recorded XSH base;
2. applies the exact accepted patch;
3. verifies the resulting tree digest;
4. reruns the required focused and broad gates;
5. creates one controlled product commit from the verified tree without
   invoking Git hooks;
6. records parent, tree, commit, author policy, message, patch, and test digests;
7. verifies the target XSH checkout is clean and still at the expected base;
8. presents the exact delivery receipt for final Grand Architect authorization; and
9. fast-forwards the local XSH target to that commit.

If XSH `HEAD` moved, delivery refuses. A rebase is not automatic: the patch is
materialized against the new base as a descendant attempt and revalidated.
VS-001 never pushes a remote.

Gate: delivered `HEAD`, authorized commit, tested commit, and recorded commit
must be identical.

### Step 11: observe and transduce one lesson

Immediate outcome checks rebuild XSH at delivered `HEAD`, rerun the contract
matrix, query the public API, verify the stale proposal is gone, and confirm the
delivery diff. They create an Outcome, not a rewritten Decision.

The accepted product contract creates an enforced product Invariant backed by
its focused test. The episode may also create one L1 candidate Lesson:

> Before proposing a new systems-glue API, compare the normative registry,
> executable behavior, active proposal corpus, and real call sites; a stale
> proposal is an institutional-memory defect, not evidence that runtime work is
> missing.

The Grand Architect may promote that lesson only to L1. Its explicit target is
a fresh stdout-capture process-API inquiry—not default context for all XSH work. Context
assembly copies the exact lesson revision into the target pack and records
`delivered`.

The lesson-uptake investigator runs as a new session. Its persisted input and
valid submission establish that it encountered the named revision. A
deterministic application judge asks whether the submission actually compared
the normative registry, executable behavior, active proposal corpus, and real
call sites before recommending a new API or no change. Passing records
`applied_once`; failing records the exact missing distinction. Neither outcome
establishes causation because VS-001 has no matched no-lesson arm.

A delayed outcome obligation asks whether the two Laputa QEMU call sites can
adopt the delivered contract without a shell wrapper and whether later XSH
process changes preserve the invariant. It has an owner, due condition, and
reopen trigger.

Gate: technical-invariant installation, lesson promotion, target delivery,
encounter, and application remain five separately queryable facts. No generic
acknowledgement advances more than one boundary.

### Step 12: export history, reflect, and close

The replay projector exports the C1 decision world from the immutable frontier.
It uses the original repository and cultural snapshots and excludes C1/C2
decisions, candidate source, trial results, outcomes, retrospective, promoted
lesson, and current XSH source which contains the answer. Negative leakage
queries for each class must be denied. Exporting the world does not run or score
an organization variant yet.

The Retrospective assesses graph sufficiency, supervisor/SDK-adapter friction, evaluator
quality, decision latency, curation sufficiency, raw-session archaeology,
demand-signal usefulness, propagation uptake, and replay-frontier leakage. It
records the first bounded dynamic observations:

```text
discovery: one contract uncertainty resolved at recorded cost
delivery: one authorized product lineage delivered or explicitly rejected
propagation: one target observed through delivered/encountered/applied_once
metamorphosis: not measured and not claimed
```

The episode closes only after required forensic objects are sealed, required
evidence is admitted, both curations are resolved, delivery and propagation
states are explicit, the replay export passes leakage checks, and the delayed
obligation remains durably scheduled.

The Project enters `observing`, reconciles every Attempt reservation and T00–T11
disposition, retains the delayed outcome obligation, and emits a final
coordination pulse. The Grand Architect closes it only after its milestones,
AdversarialReview dispositions, fault-injection Postmortem action, resource
reconciliation, and Retrospective are queryably complete. Project close does
not discharge the delayed obligation; a failed prediction can reopen both the
Episode and Project through typed dependency edges.

The final Grand Architect control turn submits `QuiesceOperatingCycle` with a
`close_when_reconciled` disposition. Acceptance increments the admission
generation before the turn settles. No further task Attempt or ordinary Office
turn may start. After `agent_settled`, the daemon sends adapter `Dispose`,
requires the SDK host to flush its SessionManager and emit `Disposed`, waits for
the host process, reaps it, seals commands/events/session/stderr and the
last Office submission, reconciles its `$0.16` reservation, and advances the
Office session to `closed`. Abnormal exit instead follows the cancellation path
and prevents clean cycle closure.

The cycle becomes `drained` only after all task processes and the Office process
are terminal. Deterministic reconciliation then verifies:

- no child is live, unregistered, or of indeterminate liveness;
- every Attempt, OfficeTurn, lease, cancellation target, workspace, and
  worktree has an explicit disposition;
- all required normal or partial Pi evidence is sealed;
- society, Operating Cycle, Project, Office-session, and Attempt charges are
  reconciled without treating unknown cost as zero;
- the final notice, coordination pulse, trace-policy revision, projection
  cursor, and cycle report are rebuildable from committed state; and
- the delayed Laputa outcome obligation is explicitly waiting for admission by
  a successor cycle rather than silently discharged.

If every check passes, the pre-authorized close condition moves the cycle
through `reconciling -> closed`; it does not require a hidden human click after
the actor Office session has ended. Any failed check leaves the cycle in
`reconciling` or `failed`, streams a `WARN`/`ERROR`, and records a recovery
notice for a descendant Grand Architect session. `societyd` remains resident
after cycle closure and may admit a separately identified successor; VS-001
does not automatically spend into one.

## Graph realized by VS-001

The expected subgraph is concrete:

```text
UniverseSeed U1 governs Society S1
founding bootstrap installs U1 and actor occupancy GA1
OperatingCycle OC1 pins U1, GA1, policies, budgets, and cancellation root CR1
GrandArchitectOfficeSession GOS1 realizes GA1 through bounded OfficeTurns
GA1 charters Project P1 from an authorized GOS1 submission
P1 coordinates Tickets T00..T11 and Episode EP1
T00 fault injection quiesces OC1, cancels/reaps child CP0,
    produces cost Postmortem PM1 and hardening Ticket PT1,
    then a recovery OfficeTurn resumes OC1 under a new admission generation

O1 typed process composition
  motivates Q1 spawn stderr contract

Q1
  has_alternative H1 missing behavior
  has_alternative H2 culturally stale
  has_alternative H3 split behavior
  tested_by E1 behavior matrix
  tested_by E2 documentation matrix
  tested_by E3 paired agent probe

Q1 and initial observations derive signals S1 contract_conflict
                                    and S2 missing_behavioral_evidence
S1 produces attention/retrieval candidate IC1 for T01/cartographer context
S2 produces attention/retrieval candidate IC2 for T02/skeptic context
GA1 charter plus readiness and reservations authorize T01/T02

E1 produces observations O-B1..O-B11
E2 produces observation O-D1
inquiry attempts produce arguments A1 and A2
O-D1 contradicts current LANG proposal revision
arguments and observations compose conflict C1

curator frontier FC1 permits admitted pre-decision evidence
curator proposes account CA1
CA1 selects observations and arguments
CA1 preserves C1 and records exclusions X1..
D1 authorized_from CA1 and authorizes prototype P1
P1 implemented_by I1 candidate patch
I1 tested_by E3 and deterministic validations
adversarial review AR1 produces typed challenges RC1..
GA1 responds to and disposes every RC1..
AR1 may challenge CA1 through a linked curation challenge

curator proposes descendant account CA2 over candidate evidence
D2 authorized_from CA2 and authorizes exact product materialization I2
I2 produces XSH commit G1
G1 produces short outcome OUT1
G1 schedules delayed outcome OUT2
OUT1 supports invariant INV1
retrospective R1 learns lesson L1
L1 targets process-inquiry context T1
T1 records delivered -> encountered -> applied_once

pre-D1 frontier F1 exports blinded decision world W1
W1 excludes D1, I1, E3, AR1, RC1.., CA2, D2, G1, OUT1, R1, and L1

final GOS1 turn quiesces OC1 with close_when_reconciled
OC1 closes only after GOS1, Attempts, children, cancellation, evidence,
    budgets, workspaces, notices, and successor dispositions reconcile
```

Object identifiers are kernel-generated. These labels are projection aliases,
not IDs embedded in actor prose.

## Acceptance tests

### Kernel and ledger

- society bootstrap rejects zero or multiple active Universe Seeds, zero or
  multiple Grand Architect occupants, a non-founding/non-ratified active seed,
  or an unreserved society budget; founding-seed and initial-cycle capabilities
  are consumed exactly once;
- at most one Operating Cycle is nonterminal; every Attempt, deterministic run,
  materialization, and Office session cites exactly one cycle, while a Project,
  Episode, Lesson, and OutcomeObligation can retain identity across a successor;
- quiescence increments admission generation atomically, permits only bounded
  control-purpose Office turns, and rejects new task admissions;
- cycle close rejects a live/indeterminate child, active lease or turn,
  unreconciled charge, unsealed required evidence, incomplete cancellation,
  undisposed workspace, or missing successor disposition;
- canonical prompt rendering begins every ActorAttempt context and resolves to
  the same seed revision cited by its Project and Ticket;
- a user principal and an actor principal occupying `TheGrandArchitect` pass
  the same authorization tests; an unoccupied actor does not;
- Project and Ticket state machines reject close with undisposed milestones,
  reviews, attempts, reservations, or required outcome obligations;
- duplicate identical command returns the prior receipt without another event;
- duplicate command id with changed `CommandBody` is rejected;
- stale generation and unauthorized capability are rejected;
- typed edges reject illegal endpoint kinds and uncommitted revisions;
- episode, attempt, product, lesson, curation, and propagation state machines
  reject impossible transitions;
- event append and materialized-state update are atomic under injected crash;
- an expired lease becomes reclaimable without rewriting its attempt;
- content-object reference before successful seal is rejected;
- sealing an object never creates evidence admission or a graph node;
- curation rejects an unadmitted source and preserves exclusion/challenge
  lineage across descendants;
- storage length and path are absent from semantic evidence and curation rows;
- migration inspection finds no SQLite JSON column and no generic `payload`,
  `metadata`, or EAV table;
- a disclosure frontier denies every nonmember regardless of projector or actor
  request text;
- a signal rebuild creates no epistemic event or authority;
- duplicate correlated sources do not inflate family pressure; an ineligible
  signal records a reason rather than becoming zero;
- every applied influence traces through candidate, signal family, derivation,
  curated/admitted sources, and displaced alternatives;
- `cost_reserve_at_risk` can change a pulse or attention surface but cannot
  cancel work; only the budget rule can cancel;
- budget reservation charges are atomic across Attempt or Office session,
  Project, Operating Cycle, and society constraints, and integer micro-dollar
  reconciliation cannot overspend by rounding;
- `Unknown` and `Unavailable` cost freeze admission and trigger the configured
  Postmortem rather than recording zero;
- review findings cannot edit their target or resolve themselves, and every C2
  challenge has a Grand Architect disposition;
- the fault-injection Postmortem reconstructs its timeline from ledger events
  and cannot enact its action proposal without a separate Ticket command;
- one propagation observation cannot skip `delivered`, `encountered`, or
  `applied_once` boundaries;
- ledger replay reconstructs all current rows and detects tampering; and
- a projector principal cannot mutate graph, authority, readiness, or events.

### Resident daemon and typed protocol

- `societyd` is the sole SQLite/content writer; a second daemon against the same
  runtime root fails before serving commands, and `societyctl` has no direct
  database-write mode;
- socket permission, peer attribution, principal, capability, expected
  generation, and command id are checked independently;
- unknown protocol version or tag, oversized/short frame, invalid UTF-8,
  trailing bytes, missing field, and a JSON command envelope all fail without a
  command or event row;
- duplicate valid request returns the original receipt across client reconnect
  and daemon restart;
- an injected crash before transaction commit has no accepted event, while a
  crash after commit replays the accepted state before the socket reopens;
- daemon restart enters recovery with admission fenced before examining any
  registered child or outstanding cancellation request; and
- task workspaces and Pi environments contain neither the control socket path
  nor a credential capable of issuing daemon commands.

### Grand Architect Pi SDK Office

- the exact Node, adapter, lockfile and Pi SDK 0.83.0 identities, seed-first
  `OFFICE-SYSTEM.md`, provider, model, thinking, tools, settings, session
  directory, and cycle identity are recorded;
- the host reaches `AdapterReady`, durable child registration, generation
  recheck, and `CreateSession` before the SDK constructs an `AgentSession`; an
  initial `Prompt` is never sent to an unregistered or unvalidated host;
- only `CreateSession`, `Prompt`, `FollowUp`, reason-bearing `Steer`, `Abort`,
  `GetState`, and `Dispose` outbound variants are encodable; model/session
  mutation, discovered resources, and arbitrary host-side commands are rejected;
- correlation ids match results while SDK events interleave, and an Office
  turn cannot settle before both its response and final `agent_settled`;
- at most one turn is active; follow-ups obey batch/turn/context limits, and an
  urgent steer records the stale or unsafe premise it corrected;
- Office submissions cannot bypass closed parsing, occupancy attribution,
  capability, generation, readiness, budget, or ordinary command idempotency;
- narrative approval, a trace line, notice delivery, or Pi tool output changes
  no durable state;
- each Office turn is independently reserved and reconciled within the session
  and all cross-cutting caps before another ordinary turn begins;
- abnormal Office exit makes authority unavailable; a descendant recovery
  session reconstructs from a ledger frontier and never inherits unsealed chat;
  and
- the same command authorization fixtures pass for actor and user occupants of
  the Office, while VS-001's live treatment uses the actor occupant.

### Cancellation and process ownership

- a child reserved under generation N but reaching `AdapterReady` after
  quiescence to N+1 never receives `CreateSession`, makes no provider call, and
  is reaped;
- control-pipe EOF before `CreateSession` makes the inert host exit without
  constructing an AgentSession or making a provider call;
- acceptance fences admission and writes the complete target set before any SDK
  abort or OS signal receipt is recorded;
- `Quiesce` lets running work settle, `GracefulCancel` follows
  abort/5-second/TERM/5-second/KILL ordering, and `EmergencyStop` follows
  abort/1-second/TERM/2-second/KILL ordering in deterministic clock tests;
- process doubles cover cooperative abort, TERM exit, forced KILL, already-dead
  child, signal error, escaped descendant, and containment failure;
- duplicate cancellation is idempotent; a second host stop signal creates a
  linked emergency upgrade without repeating terminal targets;
- cancellation preserves partial Pi streams, session, stderr, submission state,
  workspace/Git receipt, known-or-unknown cost, and the distinction between
  process exit and Pi settlement;
- a crash before `CreateSession` cannot leak a Pi `AgentSession`; a crash after
  `CreateSession` restarts with admission closed, terminates the recorded SDK
  host process group, records lost
  parentage/wait evidence, and triggers a Postmortem;
- PID or process-group reuse inconsistent with the recorded spawn nonce/liveness
  evidence is a containment failure and never authorizes signalling an
  unrelated process;
- signal handlers only wake the control loop, and repeated SIGINT/SIGTERM during
  injected SQLite or trace-sink failure still reaches reconciliation; and
- no Operating Cycle terminal state is reachable while cancellation or child
  liveness is nonterminal.

### Observability and checked notice propagation

- the mandatory monitor captures every registered `INFO`/`WARN`/`ERROR`
  lifecycle event even when the diagnostic filter requests a quieter level;
- target/level fixtures reject raw boundary data above `TRACE`, mechanism noise
  above `DEBUG`, ordinary lifecycle facts above `INFO`, or trusted-mechanism
  failures below `ERROR`;
- trace fields contain the required typed correlation path and never prompt,
  reasoning, source, credential, secret environment, raw JSON, or submission
  content;
- an accepted transition commits before its notice/trace rendering; injected
  commit failure emits no success notice, and trace-sink failure cannot roll
  back or advance state;
- notice eligibility is derived only from committed typed facts and produces
  no authority or epistemic claim;
- same-key notices coalesce until severity, generation, or actionable state
  changes, while delivery/suppression remains reconstructable;
- bounded console and Office queues cannot block cancellation, SQLite ownership,
  Pi stream draining, or child reaping, and a saturation test surfaces dropped
  rendering counts without losing durable notices;
- the agent Grand Architect receives only allowed notice batches at a named
  ledger frontier, never raw trace lines or the whole provenance graph; and
- ledger replay rebuilds notices and the Operating Cycle view without parsing
  trace output.

### Native Pi supervision

- the exact Node/adapter/lock/Pi SDK 0.83.0 identities, `openrouter` provider,
  `deepseek/deepseek-v4-flash-0731` model, `high` thinking, tool profile, `cwd`,
  resource-loader/settings policy, and prompt inputs appear in typed execution
  rows;
- a mismatched Node, adapter, dependency-lock, Pi package, session-format, or
  event-union version is rejected unless a new execution-profile revision is
  authorized and qualified;
- project context, extensions, skills, templates, and themes remain unloaded;
- SessionManager header cwd and id match the Attempt/Office session;
- event normalization distinguishes `agent_end` from `agent_settled` and handles
  an automatic retry;
- usage includes assistant and compaction costs without double counting;
- optional reasoning tokens remain absent when the provider omits them;
- a zero SDK-host process exit plus assistant `error` is not success;
- a valid model stop plus missing/invalid `submission.json` is
  `protocol_failed`;
- TERM and forced KILL preserve partial event, session, stderr, and workspace
  receipts;
- read-worktree mutation is detected;
- the curator context excludes raw session and narrative artifacts unless an
  authorized escalation names them;
- secret environment values never enter typed environment records or sealed
  content-object bytes; and
- workspace cleanup is impossible before all required forensic objects are
  sealed.

A filesystem allowlist test asserts that every task/Office runtime `.json` or
`.jsonl` file exists only at `pi/commands.jsonl`, `pi/events.jsonl`, the one
canonical SessionManager file beneath `pi/session/`, or the current
`output/submission.json`. No Project,
Ticket, environment, receipt, command-ledger, projection, trace, or
cancellation JSON file may appear.

All Pi-supervision tests use `society-pi-host` doubles which emit pinned adapter
ready/result, SDK-event, usage, settlement, and SessionManager fixtures. They do
not call a provider.

### Experiment package

- every behavior case passes on a known-good fixture and its relevant negative
  control fails;
- documentation conflict output changes when a deliberately stale source is
  introduced;
- treatment labels remain sealed until both task submissions close;
- evaluator failure cannot become candidate failure;
- host-path contamination is separately reported;
- a curated account missing the strongest defeating evidence or an exclusion
  reason fails its contract;
- a raw-evidence escalation without a named uncertainty is rejected;
- the accepted patch applies to a fresh base and reproduces its tree digest;
- a changed XSH target `HEAD` blocks delivery;
- the materialized commit has exactly one expected parent and the authorized
  tree;
- immediate outcome is linked to, not substituted for, the original prediction;
- the product invariant is installed through the exact committed XSH test;
- L1 delivery, encounter, and application are distinct observations, and the
  report explicitly refuses causal attribution;
- the C1 decision-world export omits every seeded aftermath leak and records
  denied access attempts; and
- coordination-pulse and Grand Architect brief rebuilds cite their event cursor
  and cannot mutate state; and
- projection rebuild yields the same normalized Markdown packets with no
  machine JSON projection.

### End-to-end judge

The ultimate VS-001 test starts from an empty database and content-object root,
starts `societyd`, and uses SDK-host doubles for every actor while executing the
complete episode through a temporary product repository. It must install the
founding Universe Seed and actor-held Grand Architect Office, admit one
Operating Cycle, pass the Office `AdapterReady`/`CreateSession` SDK bootstrap,
charter the Project and T00–T11 through Office submissions, exercise
quiesce/cancel/reap/resume for the fault-injection cost Postmortem, run nine
actor instances through one
persistent Office session and nine task Attempts, accept two linked curations,
dispose every adversarial challenge, deliver one exact commit, schedule one
outcome, promote one L1 lesson through `applied_once`, and export one
leakage-clean C1 decision world.

Before final quiescence it closes and reconciles the Project, rebuilds every
projection, injects a contradictory observation, proves typed Episode/Project
reopen, disposes the descendant state, and closes them again. The last Office
turn then orders cycle closure. The judge closes and seals the Office session,
reconciles every child/cancellation/budget/workspace/successor disposition,
closes the Operating Cycle, rebuilds state and notices from the ledger, and
asserts that `societyd` remains resident with no live child and no automatic
successor spend.

The paid live run uses the same circuit and schemas. Model output is not part of
the deterministic test oracle.

## Implementation order

### Landed implementation evidence

This ledger records bounded implementation evidence, not milestone waivers.
An enclosing milestone remains open until its exit judge passes.

| Commit | Boundary proven | Explicitly still open |
| --- | --- | --- |
| `b96b545` | deterministic behavior/documentation, fluency, curation-shape, uptake-shape, and disclosure-frontier fixture judges, including C01-C19 rejection controls | kernel admission, process-group reaping, actor/treatment identity, curation authority, real disclosure enforcement, and the Milestone-7 end-to-end judge |
| `f7873e3` | pinned Pi 0.83.0 TypeScript host construction and lifecycle, exact model/catalog policy, closed JSONL, lossless SDK-event projection, transcript/usage receipts, terminal outcome classification, and output-loss containment across 43 provider-free tests | Rust peer, durable correlation and charging, process supervision, sealed evidence, package-advisory resolution, native qualification, and Milestone 5 as a whole |
| `3cabe90` | typed founding bootstrap, exact qualification/live cycle treatments, occupancy-scoped authority, Office session/turn and cancellation terminal facts, cross-cutting budget freeze/Postmortem resolution, one-to-one command/event bodies, idempotent receipts, and fresh SQLite replay across 17 integration tests | remaining Milestone-1 graph/project/evidence/product domains; daemon authentication/recovery; child, signal, reap, and evidence-sealing transitions; qualification prerequisite and sub-reservations |
| `62a8bbe` | byte-sealed Rust Pi-boundary peer with exact child/runtime/profile validation, fail-closed JSONL decoding, pending-command and ordered-turn lifecycles, prompt-attributed monotonic usage, closed downstream observations, and symmetric host hardening; 48 TypeScript tests, 36 Rust unit tests, a public-consumer test, and a real provider-free `CreateSession` to `Dispose` pipe test | durable stream/transcript objects, ledger correlation, budget reservation and charging, process ownership/reaping, package-advisory resolution, native qualification, and Milestone 5 as a whole |
| `8c5f4f4` | root-workspace resident single-writer spine with a query-only named socket, distinct supervisor command stream, one-writer/runtime-file enforcement, replay-fenced restart, crash seams, fixed INFO+ monitor, signal wakeup, and literal no-child empty-cycle closure across 11 integration tests | content/object store, outbox/projections, Pi/process ownership and reaping, recovery reconciliation, full typed notices/spans/redaction, and an external trusted supervisor launcher |
| `1c12b84` | typed Project/Ticket planning, Observation/Hypothesis revision bodies in named one-to-one tables, finite edge matrix, Episode, conservative review/Postmortem blockers, exact authority/provenance, migration-step fencing, and semantic replay/tamper detection across 20 integration tests | independent Actor provisioning, WorkItem/Attempt and Ticket execution, review execution/resolution, evidence/delivery, content/outbox, supervision, recovery, and remaining Milestone-1 domains |
| `b5cc365` | isolated single-writer physical content store with canonical SHA-256 paths, atomic non-overwriting seals, file/directory sync ordering, verified idempotent reuse, strict stale-ingest recovery, cross-process locking, and tamper/symlink/limit controls across nine tests | root/daemon ownership, typed `SealContentObject`, durable database reference after its receipt, forensic manifests, evidence admission, retention/access policy, and any provenance or graph meaning |
| `5058188` | isolated closed parser for the exact B01-B11 behavior matrix, including static case-manifest binding, typed stream-evidence/digest union, canonical status/framing, fixed cardinality/order, and adversarial parsing controls | sealed evaluator/input/TSV/artifact binding, input-digest and documentation adapters, durable behavior-observation bodies, evidence admission, process reaping, and the Milestone-7 end-to-end judge |
| `0d1dc2e` | parsing-only closed adapters for all six deterministic producer digest manifests and the documentation, fluency, C1 curation, uptake, and W1 frontier relations; exact LF framing, row identity/order, closed value unions, 64-byte opaque workspace labels, and C20 producer/parser symmetry are covered across 11 Rust tests and the real negative judge | sealed producer/input/output identity, evaluator execution, actor/treatment/capability authority, process reaping, durable evidence admission, real disclosure enforcement, and the Milestone-7 end-to-end judge |
| `ebfaa81` | third provider-free deterministic treatment, explicitly unqualified native profile, closed actor configuration/instance/context, Work Item/lease/Attempt reservation and retry lineage, terminal/pause/treatment fences, exact independent-review binding and resolution, outcome closure blockers, typed capability-grant origins, and migration/replay/tamper controls across 28 integration tests | Pi/process/submission/evaluator receipts; native qualification; Project, Office-session, and exact per-Attempt accounting; configuration mutation/retirement; full Attempt lifecycle; content/evidence/delivery/notices/recovery; and Milestone 1 as a whole |
| `748fec0` | provider-free native Pi-host process physics: private fresh workspaces, verified artifacts, inert process-group spawn, nonblocking and deadline-bounded control/handshake streams, pending-create cancellation fences, honest logical-versus-physical transient byte receipts, typed TERM/KILL race outcomes, escalation, direct-child reap, and Drop containment across 20 process tests, five library tests, and one exact built-host `CreateSession` to `Dispose` smoke | resident daemon ownership and durable child/session/signal/reap rows; content sealing; budget reservation, charge, and cancellation transactions; restart recovery; package import/qualification proof; detached-descendant containment; and Milestone 4 as a whole |
| `7931d32` | isolated local product mechanics: clean exact-ref qualification, immutable candidate-tree capture, binary patch/path binding, opaque anti-recombination receipts, fresh materialization, bounded trusted-Git validation, externally supervised XSH/Xsht receipt seam, controlled no-hook commit, guarded CAS delivery, explicit checkout-recovery fence, and no-follow cleanup ownership across 22 provider-free tests | kernel C2/delivery authority, SQLite persistence/idempotency, daemon workspace/process custody, authentic validation/process evidence, content sealing, budgets/cancellation, outcome scheduling, remote delivery, and Milestone 6 as a whole |
| `71ad51e` | normalized deterministic content/evidence foundation: global digest identity is separated from run-specific manifest producer/schema/retention, evaluator/input revisions and evaluation receipts are exact, evidence admission preserves semantic role/applicability/limitations, two experiments may reuse identical output bytes without merging provenance, and all command/event bodies, material replay, migration rollback, and resident rejection/treatment wire values are closed across 30 kernel and 11 daemon integration tests | physical content-store invocation, evaluator execution and artifact authentication, parsed observation persistence, Pi/process receipts, curation/graph conversion, daemon command integration, influence/disclosure/propagation, and Milestone 1 as a whole |
| `64a5977` | root-workspace physical content integration: the resident daemon exclusively owns the physical store, seals exact bytes before issuing the existing receipt and global-object commands, resumes the closed `Absent`/`SealReceiptOnly`/`Registered` split within one live authority using retry-stable command identities, rejects tamper/symlink/limit and changed-byte recombination, and exposes no public or supervisor content mutation tag across 14 daemon integration tests plus the nine physical-store tests | post-process restart completion or reconciliation (restart remains `RecoveryFenced`); evaluator execution or artifact authentication; media/schema, producer, retention, provenance, evidence, graph, or influence meaning; durable child/process integration; and Milestone 1 as a whole |

The current coordination, M3 execution, and deterministic-evidence kernel is
still a bounded foundation,
not Milestone-1 completion. It can provision a non-Grand-Architect actor, bind
an exact Work Item/lease/Attempt and deterministic execution profile, preserve
retry lineage, and execute the complete typed review-response/closure blocker
chain. Its terminal and validation commands are receipt-free trusted-kernel
fixture attestations: they do not prove Pi settlement, process exit, sealed
submission/evaluator evidence, or judgment. M3 Attempt reservations debit only
the Society and Operating Cycle envelopes; Project, Office-session, and exact
per-Attempt constraints remain part of durable accounting integration. Global
content identity, run-specific deterministic manifests, evaluation receipts,
and narrow evidence admissions are typed trusted-kernel attestations. The
daemon now invokes its private physical store before recording the narrow seal
receipt and global object identity, but does not invoke an evaluator,
authenticate evaluator artifacts, persist parsed observations, or turn an
admission into graph truth. Its two split-transition retry seams are
same-lifetime evidence only: a restarted nonempty daemon remains
`RecoveryFenced` and cannot complete a half-recorded content operation.
Curation, delivery, notices/outbox, process receipt binding, native
qualification, recovery, and the full graph vocabulary are still open.

The current M4 supervisor is likewise a bounded native process-physics
boundary, not durable supervision. `PiSupervisor` is exported by `societyd`,
but the resident control loop does not yet reserve or register its child,
persist stream or signal receipts, seal their bytes, reconcile cost, or recover
the process after daemon restart. A supplied host artifact and package-set
digest are verified bytes; this provider-free tranche does not prove that an
arbitrary adapter dynamically imports the claimed Pi package set and does not
qualify the native execution profile.

### Milestone 1: contracts in executable form

Write observable transition tests first, then implement newtypes, closed enums,
commands/events and their one-to-one bodies, typed errors, SQLite migration 1,
ledger replay, and idempotent receipts. Include founding bootstrap, Universe
Seed, Office/occupancy, Operating Cycle, Office session/turn, cancellation,
child process, cross-cutting budget, Project/Ticket/review/Postmortem, graph,
content/evidence/curation, disclosure, influence, propagation, and product
states.

Exit judge: migration inspection has no JSON/generic payload/EAV escape hatch;
the compiler exhaustively handles every discriminant; transition tests cover
success, stale generation, actor failure/retry, cycle quiesce/resume/close,
unknown-cost Postmortem, preserved conflict, cancellation failure, and
post-close reopen.

### Milestone 2: resident authority and local protocol

Implement `societyd` single-instance ownership, transaction loop, startup
replay/recovery mode, `societyctl`, the versioned length-prefixed local codec,
permissioned Unix socket, command/query dispatch, content-store ownership, and
projection cursor/outbox. Implement Operating Cycle admission and
reconciliation without spawning Pi.

Exit judge: two-daemon exclusion, malformed-frame corpus, authorization and
idempotency tests, crash-before/after-commit tests, restart replay, and an empty
cycle which can admit, quiesce, reconcile, and close while the daemon remains
resident.

### Milestone 3: observability and checked information propagation

Install `tracing`/`tracing-subscriber` with the fixed mandatory monitor layer,
diagnostic filters, typed span constructors, redacted field wrappers, and the
registered target/level policy. Implement `OperationalNotice` derivation,
deduplication, bounded console/Office queues, delivery receipts, cycle monitor
query, and deterministic notice replay. Do not persist trace lines or send them
to Pi.

Exit judge: golden level/redaction tests, commit-before-notice fault tests,
trace-sink and queue-saturation tests, byte-identical notice/cycle-view replay,
and proof that cancellation/reaping progress under monitor backpressure.

### Milestone 4: process supervision and cancellation physics

Implement `PiSupervisor`, workspace preparation, process-group ownership,
an inert SDK-host double with the `AdapterReady`/`CreateSession` handshake,
child registry,
liveness/signal receipts, typed cancellation requests and propagation,
deadline escalation, signal-to-control-loop bridging, partial-evidence sealing,
and restart containment. Use deterministic clocks and process doubles only.

Exit judge: every pre/post-`CreateSession` race, quiesce, cooperative abort,
TERM, KILL,
already-dead child, daemon crash, second-signal upgrade, stale generation,
escaped descendant, and cycle-close blocker in the cancellation acceptance
suite passes with no provider access.

### Milestone 5: pinned Pi SDK adapter

Install and lock `@earendil-works/pi-coding-agent` 0.83.0 in
`packages/society-pi-host/`. Implement the TypeScript adapter, empty explicit
`ResourceLoader`, exact `ModelRuntime`/`SettingsManager`/`SessionManager` and
`createAgentSession()` construction, control/event JSONL protocol, exhaustive
event conversion, prompt/FollowUp/Steer/Abort/Dispose calls, and fatal cleanup.
Implement the Rust peer, identity/config validation, correlation and sequence
checks, event/session normalization, usage/cost charging, closed task/Office
submissions, Office-turn admission, notice batching, and recovery packets. Pin
real 0.83.0 type-derived fixtures from `~/d/pi`; runtime must not read that
checkout.

Exit judge: task and Office process doubles pass happy, retry, compaction,
protocol-error, invalid-submission, unknown-cost, abort, recovery, and budget
cases; filesystem JSON allowlists and secret-exclusion scans pass. No provider
is invoked.

### Milestone 6: product, knowledge, and projection path

Implement XSH read/product worktrees, patch capture, clean materialization,
validation receipts, controlled commit construction, guarded local
fast-forward, outcome scheduling, L1 target-state transitions, curated episode
and Office/cycle projections, and blinded decision-world export. XSH programs
may serve as bounded product workloads/evaluators; none gains database, Pi,
process-cleanup, capability, cancellation, or Git-delivery authority.

Exit judge: exact-tree delivery and moved-HEAD refusal, no-hook commit receipt,
reopen, propagation separation, leakage denial, and projection rebuild tests
pass against temporary repositories.

### Milestone 7: VS-001 deterministic circuit

Implement noisy-child fixtures, behavior and documentation matrices,
agent-task evaluator, curation contract judge, uptake application judge,
frontier leakage controls, nine actor configurations, assignments, notice
policy, trace policy, cycle charter, and the complete process-double end-to-end
judge. Run the end-to-end judge from an empty runtime root through reconciled
cycle closure.

Exit judge: every acceptance section above passes; the final database answers
the completion questions without raw transcript parsing; `societyd` has no
live children and remains ready but does not create a successor cycle.

### Milestone 8: native Pi qualification

Run one no-product, read-source one-shot Pi-SDK qualification Attempt and one
minimal persistent Office-shaped SDK session in a disposable
laboratory cycle. Give the laboratory a separate `$0.03` hard cap; if both
cannot fit, stop rather than borrow. Verify real Node/adapter/package/lock
identity, `createAgentSession` configuration, handshake, events/results,
SessionManager transcript, usage, submission, abort, sealing, and Office close
against the normalizer. This is the first paid boundary and uses the consumed
bootstrap qualification capability because no agent Grand Architect can safely
exist before the SDK boundary qualifies. The Office-shaped session has no
Office capability or daemon credential; success creates the exact
`PiSdkQualificationV1` prerequisite for the initial actor Office session.

Exit judge: provider/requested/response model, thinking, cost, settlement,
process, cancellation, and sealed raw evidence agree; any unknown cost or
protocol drift fails qualification and keeps VS-001 unadmitted.

### Milestone 9: execute and reconcile VS-001

Admit `OC-VS-001`, start the actor Grand Architect Office session, create the
live Project/Episode through Office submissions, and run the eight task
instances through nine Attempts only as Tickets, WIP, reservations, and notice-
driven gates become ready. Obtain actor Grand Architect decisions for recovery,
curation, C1, C2, review risk, L1 promotion, frontier creation, exact commit,
delivery, Project disposition, and cycle close. Never exceed the twelve Office
turns or `$1.00` cross-cutting cap.

Exit judge: the exact XSH commit is locally delivered; Tickets, review,
Postmortem, budgets, propagation, Episode, Project, Office session,
cancellation roots, children, workspaces, notices, and successor obligations
are reconciled; the Operating Cycle is closed; and the resident daemon has not
spent into a successor.

## Deliberately deferred machinery

VS-001 does not build:

- Docker actor or evaluator images;
- a generic DAG/workflow language;
- a web UI;
- a vector database or transcript retrieval layer;
- multi-host execution or a distributed database;
- Office succession/transfer during a live cycle, autonomous question
  origination, or organizational mutation;
- concurrent Operating Cycles, automatic rollover, or unattended successor
  spending after VS-001 closes;
- actor branching, recombination, learned professions, autonomous ecological
  scheduling, or an internal market;
- XSH ownership of SQLite, Pi sessions, process supervision, cancellation,
  tracing, capabilities, or Git delivery;
- durable archival of every trace rendering or feeding logs back into actor
  context;
- general Pareto optimization infrastructure; or
- every node and relation imagined by `ARCHITECTURE.md` beyond the typed fields
  exercised by this episode.

The schema leaves room for those concepts without implementing speculative
frameworks. VS-001 builds the smallest system in which their eventual evidence
could be honest.

## Dependency gate

The dependency gate is resolved by `DEPENDENCIES.md` and the exact workspace
pins in `Cargo.toml`. The allowed Rust surface is `rusqlite` with bundled SQLite
and default features disabled, `thiserror`, `sha2`, `tracing`,
`tracing-subscriber`, Pi-boundary-only `serde`/`serde_json`, and narrow Unix
`libc` calls. The TypeScript package exact-pins
`@earendil-works/pi-coding-agent` 0.83.0 and its build/test support in
`package-lock.json`. Migrations are monotonic embedded SQL run by the kernel;
there is no migration framework.

This decision does not authorize an async runtime, tracing appender,
process-control framework, ORM, workflow engine, generic schema framework, or
any other crate. Any addition or version change reopens the gate and must
update `DEPENDENCIES.md`, the relevant lockfile, boundary tests, and execution
profile qualification before paid use.

## Completion statement

VS-001 is complete only when the V2 database can answer, with direct links to
curated claims, admitted evidence, and expandable forensic sources:

- Why did institutional records disagree about `spawn` stderr?
- Which behavior was actually present at the pinned XSH commit?
- Which hypothesis survived, and which evidence defeated the others?
- What did each independent actor contribute and at what cost?
- Which Operating Cycle, Office session, policy revisions, admission generation,
  reservations, and child processes governed each paid action?
- Which curated provenance became an influence candidate or OperationalNotice,
  which alternatives were suppressed or coalesced, and what action followed?
- Why were these attractor-biased treatments admitted in response to these
  demand signals, without calling them professions?
- Which evidence did each curated account select, preserve as dissent, exclude,
  or escalate to raw inspection—and why?
- Did the candidate make the contract more discoverable in the paired task?
- Who authorized the prototype and the product delivery?
- Which Grand Architect Office turn proposed each authoritative command, what
  did that turn cost, and which checks accepted or rejected it?
- Which exact patch, tested tree, and commit reached XSH?
- What happened immediately afterward?
- Which product invariant entered technical heredity?
- Did the L1 lesson merely reach a context, get encountered, get applied once,
  or earn any stronger propagation claim?
- What delayed evidence can reopen the decision?
- What exactly can a counterfactual citizen know in the exported C1 decision
  world, and which aftermath is provably sequestered?
- What did this episode establish about discovery, delivery, propagation, and
  the still-unmeasured metamorphosis rate?
- Could the whole account be reconstructed without reading raw Pi transcripts?
- Did quiescence fence a stale spawn, did cancellation abort/signal/reap every
  target while preserving partial evidence, and what containment limits remain
  native-host best effort?
- Why was the Operating Cycle eligible to close, what obligation crossed its
  frontier, and did the resident daemon remain idle without silently creating
  successor spend?

That is the first credible heartbeat of the XSH society: observation becomes
warranted belief, selected meaning becomes an authorized product, the product
changes XSH, and a scoped lesson begins—without overclaiming—to change what the
next citizens inherit.
