# VS-001: native-host epistemic-to-product vertical slice

[`GLOSSARY.md`](GLOSSARY.md) is canonical for every domain term and spelling in
this contract. [`ARCHITECTURE.md`](ARCHITECTURE.md) owns the general behavior;
this file owns the first executable proof.

## Decision

The first V2 slice will reconcile XSH's contradictory `spawn` stderr contract
from source observation through one shipped XSH commit and checked institutional
memory.

The slice uses host-installed Pi actors in owned native working directories and
Git worktrees. It does not build or run Docker images. Environmental austerity
is a future experimental treatment, not the baseline actor architecture.

This is a vertical slice in the strict sense: it touches every architectural
ring and crosses every durable boundary once.

```text
UniverseSeed bootstrap and Grand Architect Project charter
  -> typed Project, Tickets, budgets, and first coordination pulse
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
```

If the implementation stops at a graph demo, a Pi runner, an evaluator, or a
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
and the active `TheGrandArchitect` office contract.

The bootstrap transaction creates exactly one active seed, installs exactly
one Grand Architect occupant, and records `ActorModelPolicyV1` and the society
hard cost ceiling. Every Project, Ticket, Episode, Decision, ReviewChallenge,
Postmortem, and ActorAttempt below stores the exact seed revision. Every actor's
`SYSTEM.md` begins with the same canonical `UNIVERSE-SEED.md` rendering before
the scoped assignment or context frontier.

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
- one active Universe Seed is rendered into every actor attempt and traceable
  through Project, Ticket, decision, product, and outcome;
- one Grand Architect occupant exercises C1 and C2 authority without a
  human-only authorization path;
- typed Project/Ticket state, a deterministic coordination pulse, and one
  `AdversarialReview` coordinate the episode without becoming its world model;
- typed graph revisions preserve what was known before the decision;
- native Pi attempts have pinned assignments, configurations, workspaces,
  sessions, costs, tool events, and outputs;
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
runner:             host V2 runner
pi:                 resolved host `pi` executable and version
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
```

The environment manifest records this as a collection of fields, not
`cleanroom: false`.

### Model and cost envelope

Every VS-001 actor attempt uses exactly:

```text
Pi:        0.83.0
provider:  openrouter
model:     deepseek/deepseek-v4-flash-0731
thinking:  high
fallback:  none
```

The runner substitutes none of those values from ambient configuration. A
different effective provider, requested model, thinking level, Pi version, or
response model fails qualification rather than silently creating a new
treatment.

The live Project hard cap is `$1.00`, represented durably as
`UsdMicros(1_000_000)`. Before any attempt starts, the kernel reserves these
per-attempt maxima:

| Attempt | Cap | Turn cap | Wall cap |
| --- | ---: | ---: | ---: |
| Contract cartographer | $0.08 | 48 | 900 s |
| Boundary skeptic | $0.08 | 48 | 900 s |
| C1 decision-account curator | $0.06 | 32 | 600 s |
| C2 descendant curator | $0.06 | 32 | 600 s |
| Prototype builder | $0.20 | 160 | 1,800 s |
| Baseline task actor | $0.08 | 80 | 900 s |
| Candidate task actor | $0.08 | 80 | 900 s |
| Product adversary | $0.10 | 64 | 900 s |
| Lesson-uptake investigator | $0.08 | 48 | 600 s |

The nine maxima reserve `$0.82`; `$0.18` is Project contingency. It is not
available to ordinary scheduling. The Grand Architect may allocate it to a
named descendant attempt after reviewing the failed attempt, known spend,
remaining evidence need, and stop condition. A retry always receives a new
Attempt identity and reservation. The Grand Architect is occupied by the user
for the initial live slice and therefore adds no model cost; later succession
to an actor occupant is a separate architecture-stage proof.

The one native Pi qualification attempt preceding VS-001 belongs to a separate
laboratory Project with a `$0.03` cap, 16 turns, and 300-second wall limit. It
cannot borrow from the live slice or be reported as VS-001 discovery work.

Cost is stored as integer micro-US-dollars. The supervisor observes the Pi
event and session files at most 250 ms behind filesystem visibility, normalizes
each cumulative provider cost idempotently, and compares known spend to the
Attempt, Episode, Project, and society ceilings. `Unknown` or `Unavailable` is
not zero: after paid work begins it freezes new admission, terminates the owned
process group, preserves partial evidence, and triggers a cost Postmortem.
Known cap breach does the same. Turn and wall caps bound continuation but cannot
promise no provider-side overshoot between a response and cancellation.

No paid attempt runs for projection, coordination pulse, schema qualification,
deterministic judge, product materialization, replay, or fault injection. All
runner and end-to-end tests use pinned Pi doubles. Unused reservation returns
to its parent only after cost reconciliation; it is never interpreted as a
throughput target.

### Pi invocation

The runner composes an argv equivalent to:

```text
pi
  --provider openrouter
  --model deepseek/deepseek-v4-flash-0731
  --thinking high
  --mode json
  --no-approve
  --system-prompt <owned-system-prompt-path>
  --no-extensions
  --no-skills
  --no-prompt-templates
  --no-themes
  --no-context-files
  --tools <comma-separated-profile>
  --session <owned-session-path>
  --print
  @<sealed-assignment-copy>
```

The runner is written against the installed Pi CLI contract and records the
resolved executable digest, `pi --version`, argv with secret-bearing values
redacted, provider, exact model, thinking level, enabled tools, and session
format version. It clears inherited `PI_PACKAGE_DIR` and
`PI_STANDALONE_BINARY` so a parent Pi installation cannot redirect the child to
a partial embedded package.

Pi 0.83.0 treats an existing `--system-prompt` value as a file path and reads
its content. It expands `@ASSIGNMENT.md` into a `<file name="...">` block in the
initial user message. VS-001 therefore seals both source files and records the
actual persisted user message; the latter is the exact context evidence seen by
the model, including Pi's absolute-path wrapper.

`SYSTEM.md` is deterministically rendered, in order, from the canonical active
`UNIVERSE-SEED.md`, the actor-attempt and authority boundary, the closed
submission contract, and the scoped context/frontier policy. The seed is the
first byte-bearing prompt component, not an optional attachment. The renderer
records its own revision and rejects a mismatched seed reference before Pi can
start.

Pi 0.83.0 has no `PI_AUTH_FILE` interface. It resolves authentication from
`auth.json` beneath `PI_CODING_AGENT_DIR`, whose default is `~/.pi/agent`.
The supervisor therefore passes an explicit host-admin-configured
`PI_CODING_AGENT_DIR` and records a logical configuration identity. It seals
the effective non-secret model configuration needed to reproduce model
selection, but never copies `auth.json`, credential values, or a secret-bearing
environment into the content-object store. The separately explicit `--session`
path keeps attempt transcripts inside the owned workspace rather than the
agent directory. Model API traffic is necessary; `--offline` is not treated as
network confinement.

The old factory's `factory/entrypoints/run-agent.xsh` is prior integration
evidence for JSON print mode, explicit sessions, disabled ambient resources,
tool allowlists, and sealed assignments. V2 intentionally does not copy its
`--approve` choice or its `PI_AUTH_FILE` environment assumption. The former
trusts project-local resources; the latter is not an interface in Pi 0.83.0.

The normative compatibility references in the pinned `~/d/pi` checkout are the
CLI argument parser, print mode, agent-session event contract, session manager,
usage aggregation, resource loader, file-input expansion, and configuration
path functions under `packages/coding-agent/src/`, plus the shared agent/message
types. VS-001 checks real fixtures derived from those exact 0.83.0 types into its
test package. Runtime does not depend on reading `~/d/pi`; that checkout is the
qualification reference for the pinned executable.

### Pi evidence semantics

In Pi 0.83.0 JSON print mode, stdout is a JSONL **session event stream**. Its
first record is the session header and subsequent records are
`AgentSessionEvent` values such as:

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

`agent_end` may be followed by automatic retry. `agent_settled` is the public
event that says retry and post-run handling have finished. A clean attempt
expects one final `agent_settled` before normal Pi exit.

The `--session` file is a separate canonical JSONL transcript. Its v3 header
contains `type`, `version`, `id`, `timestamp`, and `cwd`; its tree entries
persist user, assistant, and tool-result messages plus model/thinking changes
and any compaction or branch-summary usage. Assistant messages contain
provider, requested model, optional response model, stop reason, optional error,
token usage, optional provider-reported reasoning tokens, and calculated cost.

The two files serve different evidence roles:

- the event stream preserves execution order, streaming/tool lifecycle, retry,
  and settlement; and
- the session file preserves the durable transcript tree and billable usage.

VS-001 seals both. Its normalizer aggregates usage from assistant, tool-result,
compaction, and branch-summary entries exactly as Pi does. It reports reasoning
tokens only when the provider supplies them and never estimates them from
thinking text.

Pi JSON mode does not make its process exit code a sufficient statement of
model success. The normalizer classifies the final settled assistant
`stopReason` (`stop`, `length`, `error`, or `aborted`), required submission,
tool failures, retry history, and process exit separately. A zero Pi exit with
an invalid submission is still `protocol_failed`; an interrupted event stream
without `agent_settled` is still incomplete even if a partial session exists.

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
expected to resolve and creates a visible escalation; the runner supplies only
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
experimental/tool-availability declaration, not a security capability. The
runner rejects undeclared Pi extensions and preserves every reported tool call
and nonzero tool result.

### Workspace layout

Runtime state is outside Git under a configurable root, initially
`var/` in the V2 checkout:

```text
var/
├── society.sqlite3
├── content/sha256/<prefix>/<digest>
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
│   │   ├── session.jsonl
│   │   ├── events.jsonl
│   │   └── stderr.log
└── projections/
```

`society.sqlite3` and `content/` are durable and backed up together.
`workspaces/` is an owned staging area. Required forensic content is sealed into
`content/` before a workspace becomes cleanup-eligible; semantic evidence
admission remains a separate database transition. `projections/` can be deleted
and rebuilt.

The V2 source repository contains schemas, migrations, policies, prompts,
deterministic fixtures, and tests. It does not commit paid sessions or a second
copy of the durable database.

Workspace identity, execution profile, input membership, environment allowlist,
pre/post Git state, process status, usage, cost, settlement, and cleanup
eligibility are typed SQLite rows. They are materialized into the workspace by
the runner; there is no `manifest.json` or `receipt.json` authority. The only
JSON files are Pi's `session.jsonl`, Pi's `events.jsonl`, and the actor's
closed-schema `submission.json`, all inside the Pi boundary. A diagnostic
Markdown receipt may be projected on demand but is disposable.

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
builder owns it. The runner records status and tree digests before and after,
the parent, portable patch digest, changed paths, and untracked files. The Pi
builder does not create the authoritative product commit. After a C2 decision,
a deterministic materializer applies the accepted patch to a fresh worktree,
reruns the required judges, and creates the controlled product commit from that
exact tree without invoking repository hooks. The branch and source workspace are
retained until delivery or explicit retirement.

### Native supervisor

The supervisor:

1. creates the workspace and sealed input copies;
2. records a pre-execution filesystem and Git receipt;
3. launches Pi as an owned process group in the declared `cwd`;
4. streams Pi's JSONL session-event output to a runner-owned file while Pi
   writes its separate session JSONL;
5. accounts wall time and provider telemetry as it becomes available;
6. sends TERM on cancellation or wall-budget expiry, waits a short declared
   grace period, then escalates to KILL;
7. waits and reaps the complete owned process group;
8. reconciles event settlement, final assistant stop reason, retry history,
   session entries, submission protocol, exit status, signal, duration, budget
   state, and post-workspace receipt even on interruption; and
9. seals raw content, registers its forensic manifest and normalized attempt
   facts, then releases the lease.

macOS does not provide Linux cgroups. VS-001 therefore treats model cost,
memory, CPU, and descendant containment as observed or best-effort except for
wall timeout and the runner's owned process group. The receipt says which
limits were enforced and which were only measured.

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
evidence request is a separate submission and attempt descendant; the runner
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

The deterministic runner validates the schema and submits the contribution to
the kernel on behalf of the actor instance. Invalid or missing submissions end
as `protocol_failed`; they are never repaired by parsing narrative prose.
Narrative chat and tool traces remain forensic evidence and do not become
culture merely because the model described a general lesson fluently.

## Minimal implementation architecture

### Source tree

The first implementation target is:

```text
Cargo.toml
crates/
├── society-kernel/        # typed contracts, transitions, ledger, content/evidence
└── society-cli/           # Office-holder and XSH-facing protocol client
migrations/                # monotonic V2-owned SQLite migrations
xsh/
└── society/
    ├── client.xsh
    ├── native_runner.xsh
    ├── product.xsh
    ├── evaluate.xsh
    ├── coordinate.xsh
    └── project.xsh
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
├── protocol/
├── runner/
└── vs-001/
```

Names may be refined before code exists. Ownership may not collapse into a
generic workflow framework.

Seed fixtures, state transitions, actor/model policies, budget values, and
behavior cases are compiled Rust types registered through kernel commands.
Markdown files in the circuit tree are prompt/projection templates or human
explanations. The source tree contains no alternate JSON workflow definition.

### Process topology

```text
Grand Architect occupant / XSH policy
       |
       v
society-cli ----typed command----> society-kernel ----> SQLite + objects
       |                                  |
       | claim receipt                    +----> projection outbox
       v
native_runner.xsh
       |
       +----> Pi actor in owned cwd
       |
       +----> deterministic XSH/Rust judges
       |
       +----> sealed forensic receipt ----> society-kernel
```

The first kernel may be an invoked CLI rather than a resident daemon. SQLite
still owns concurrency and transactions. A process boundary is not required
for authority if every command goes through the same library and no caller gets
raw SQL access.

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
context_packs
work_items
leases
attempts
pi_sessions
pi_event_normalizations
resource_budgets
budget_reservations
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
ProposeUniverseSeed
RatifyUniverseSeed
BootstrapSociety
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
CancelAttempt
```

Each command includes principal, capability, command id, expected generation,
and one closed Rust `CommandBody` variant. The CLI maps each variant to a
closed subcommand and typed flags; it does not accept a JSON command envelope.
Read queries and projection rebuilds are separate from commands.

### Slice state machines

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

Cancellation, expiry, protocol failure, runner failure, model failure, and
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
  set_r0_hard_ceiling

TheGrandArchitect:
  ratify_universe_seed, bootstrap_society, charter_project, allocate,
  decide_c1, decide_c2, accept_curation, resolve_review_challenge,
  authorize_prototype, authorize_delivery, deliver_product,
  promote_lesson_l1, register_propagation_target,
  create_disclosure_frontier, accept_risk, cancel_any, reopen

project_steward:
  revise_ticket_within_charter, admit_ready_work, allocate_reserved_project_work,
  acknowledge_coordination_pulse; no seed, C2, C3, or delivery authority

runner:
  claim_work, start_attempt, seal_content_object, register_forensic_manifest,
  complete_attempt, submit_attested_contribution

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
  record normalized cost, freeze admission and cancel descendants on exact
  cap or unknown-cost rules; no authority to enlarge a budget

projector:
  read events and revisions, advance only its projection cursor
```

Kernel capabilities govern institutional state. They do not imply that a
native Pi actor is OS-confined; that is an explicitly separate execution fact.

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

VS-001 uses eight Pi instances and deterministic services. These names are
episode-local experimental labels, not recognized professions or kernel types.
Each configuration records broad attractor biases and signal responses; its
digest, not the friendly name, is authoritative.

All eight instances use `ActorModelPolicyV1`: Pi 0.83.0, `openrouter`,
`deepseek/deepseek-v4-flash-0731`, and `high` thinking. The curator instance
executes two separate attempts at the C1 and C2 frontiers, making nine paid
attempts. No instance shares a Pi session, hidden conversation, or fallback
model with another.

“Independent” in this slice means separate instance, session, workspace,
assignment frontier, contribution, and no access to the sibling output. It does
not mean foundation-model independence: every paid actor deliberately uses the
same required DeepSeek model. Review and influence records expose that shared
model ancestry and do not count the two attempts as fully independent
replications.

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

The active Grand Architect occupant owns curation acceptance, C1 prototype
admission, the C2 product decision, L1 promotion, disclosure-frontier creation,
ReviewChallenge disposition, and delivery. Deterministic judges own behavioral,
cost-normalization, and propagation observations.

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
fixtures to prove command transactions, runner recovery, session/event
normalization, evaluator negatives, evidence admission, curation exclusion,
disclosure-frontier denial, propagation-state separation, and projection
rebuild. The kernel records a qualified organization configuration.

The bootstrap principal creates the Society identity and installs the initial
Grand Architect occupant and R0 cost ceiling. That occupant ratifies
`UniverseSeed` revision 1 and calls `BootstrapSociety` against the exact seed,
office, organization, model-policy, and budget revisions. The Grand Architect
then charters Project `P-VS-001-SPAWN-STDERR`, creates T00–T11 and their typed
acceptance conditions, reserves the `$1.00` Project envelope, and emits the
first deterministic coordination pulse.

T00 also runs one Pi double that reports a paid assistant response with an
unavailable cost. The supervisor must freeze admission, terminate the owned
process group, seal partial evidence, and trigger a `CostAccountingFailure`
Postmortem. Its ledger-derived timeline, independent deterministic challenge,
containment, causal claims, and one separately judged hardening Ticket pass
through the complete Postmortem lifecycle. This is a fault-injection proof, not
live model spend or a claim that an incident occurred in VS-001 product work.

Gate: no paid work may start while a required R0 test or negative control
fails, while the society lacks exactly one active seed or Grand Architect
occupant, or while any Project/Ticket/Attempt fails to cite the active seed.

### Step 1: frame and admit

The Grand Architect creates the Objective, Question, three Hypotheses, hard
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

The runner seals sessions, event streams, submissions, Git receipts, and
normalized usage into forensic manifests. It admits execution classification
facts but does not turn narrative chat or every tool event into graph evidence.
Validated submissions become contribution proposals.

Known cost updates the hierarchy and the next coordination pulse. At 50% and
80% of an Attempt or Project cap, `cost_reserve_at_risk` may become an eligible
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
contract. The runner captures the patch and tree digest. Focused deterministic
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

The Retrospective assesses graph sufficiency, runner friction, evaluator
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

## Graph realized by VS-001

The expected subgraph is concrete:

```text
UniverseSeed U1 governs Society S1
TheGrandArchitect occupancy GA1 ratifies U1 and charters Project P1
P1 coordinates Tickets T00..T11 and Episode EP1
T00 fault injection produces cost Postmortem PM1 and hardening Ticket PT1

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

E1 produces observations O-B1..O-B10
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
```

Object identifiers are kernel-generated. These labels are projection aliases,
not IDs embedded in actor prose.

## Acceptance tests

### Kernel and ledger

- society bootstrap rejects zero or multiple active Universe Seeds, zero or
  multiple Grand Architect occupants, an unratified seed, or an unreserved
  society budget;
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
- budget reservation is atomic across Attempt, Episode, Project, and society,
  and integer micro-dollar reconciliation cannot overspend by rounding;
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

### Native Pi runner

- the exact 0.83.0 argv, `openrouter` provider,
  `deepseek/deepseek-v4-flash-0731` model, `high` thinking, tool profile, `cwd`,
  context flags, and prompt inputs appear in typed execution rows;
- a mismatched Pi version is rejected unless a new execution-profile revision is
  authorized;
- project context, extensions, skills, templates, and themes remain unloaded;
- session header cwd and id match the attempt;
- event normalization distinguishes `agent_end` from `agent_settled` and handles
  an automatic retry;
- usage includes assistant and compaction costs without double counting;
- optional reasoning tokens remain absent when the provider omits them;
- a zero process exit plus assistant `error` is not success;
- a valid model stop plus missing/invalid `submission.json` is
  `protocol_failed`;
- TERM and forced KILL preserve partial event, session, stderr, and workspace
  receipts;
- read-worktree mutation is detected;
- the curator context excludes raw session and narrative artifacts unless an
  authorized escalation names them;
- secret environment values never enter typed environment records or sealed content-object
  bytes; and
- workspace cleanup is impossible before all required forensic objects are
  sealed.

A filesystem allowlist test asserts that `.json` or `.jsonl` workspace files
exist only at `pi/session.jsonl`, `pi/events.jsonl`, and
`output/submission.json`. No Project, Ticket, environment, receipt, command, or
projection JSON file may appear.

All runner tests use a process double which emits pinned Pi header, event, and
session fixtures. They do not call a provider.

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
uses Pi process doubles for every actor, and executes the complete episode
through a temporary product repository. It must install the Grand Architect
office and Universe Seed, charter the Project and T00–T11, complete the
fault-injection cost Postmortem, run eight actor instances through nine
attempts, accept two linked curations, dispose every adversarial challenge,
deliver one exact commit, schedule one outcome, promote one L1 lesson through
`applied_once`, export one leakage-clean C1 decision world, close and reconcile
the Project, rebuild projections, then inject a contradictory observation and
reopen the Episode and Project.

The paid live run uses the same circuit and schemas. Model output is not part of
the deterministic test oracle.

## Implementation order

### Milestone 1: contracts in executable form

Implement identifiers, commands, typed errors, SQLite migration 1, graph bodies,
Universe Seed and Office contracts, Project/Ticket/review/Postmortem state
machines, integer budget reservations, edges, content-object sealing, forensic
manifests, evidence admission, curation, disclosure frontiers, signal/influence
projection, propagation observations, and a ledger replay command. Migration 1
has no JSON columns or generic payloads. Write synthetic fixtures first:
success, actor failure/retry, unknown-cost Postmortem, preserved conflict, and
post-close reopen.

### Milestone 2: native runner with Pi doubles

Implement workspace creation, execution-profile manifests, process ownership,
Pi argv construction, event/session normalization, budget receipts, submission
validation, cancellation, and sealing. Pin real Pi 0.83.0 JSON/session fixtures
from `~/d/pi` types and v1 evidence; do not invoke a provider.

### Milestone 3: product and projection path

Implement XSH read/product worktrees, patch capture, clean materialization,
validation receipts, controlled commit construction, guarded fast-forward,
outcome scheduling, L1 target-state transitions, episode projection, and blinded
decision-world export.

### Milestone 4: VS-001 deterministic package

Implement the noisy-child fixtures, behavior matrix, documentation matrix,
agent-task evaluator, curation contract judge, uptake application judge,
frontier leakage controls, actor configurations, assignments, and the
end-to-end process-double test.

### Milestone 5: native Pi qualification

Run one no-product, read-source Pi qualification attempt in a disposable V2
episode with an intentionally tiny assignment. Verify real 0.83.0 events,
session, usage, submission, cancellation behavior, and sealing against the
normalizer. This is the first paid boundary and requires explicit admission by
the active Grand Architect.

### Milestone 6: execute VS-001

Create the live Project and Episode, run the eight actor instances through nine
attempts only as their Tickets and reservations become ready, obtain Grand
Architect decisions for curation, C1, C2, L1 promotion, frontier creation, and
final delivery authority, and deliver only after the exact-commit
authorization. Reconcile Tickets, review, budgets, pulse, Episode, and Project.

## Deliberately deferred machinery

VS-001 does not build:

- Docker actor or evaluator images;
- a generic DAG/workflow language;
- a web UI;
- a vector database or transcript retrieval layer;
- multi-host execution or a distributed database;
- agent occupancy of the Grand Architect office, autonomous question
  generation, or organizational mutation;
- actor branching, recombination, learned professions, autonomous ecological
  scheduling, or an internal market;
- general Pareto optimization infrastructure; or
- every node and relation imagined by `ARCHITECTURE.md` beyond the typed fields
  exercised by this episode.

The schema leaves room for those concepts without implementing speculative
frameworks. VS-001 builds the smallest system in which their eventual evidence
could be honest.

## Dependency gate

The Rust kernel will need an SQLite binding and ordinary serialization/hash
support unless those already exist in the chosen workspace. No dependency is
added by this design document. Before implementation, the exact minimal crate
set, versions, vendoring/feature policy, and migration tooling must be presented
to the user before implementation in accordance with the repository working
contract.

## Completion statement

VS-001 is complete only when the V2 database can answer, with direct links to
curated claims, admitted evidence, and expandable forensic sources:

- Why did institutional records disagree about `spawn` stderr?
- Which behavior was actually present at the pinned XSH commit?
- Which hypothesis survived, and which evidence defeated the others?
- What did each independent actor contribute and at what cost?
- Why were these attractor-biased treatments admitted in response to these
  demand signals, without calling them professions?
- Which evidence did each curated account select, preserve as dissent, exclude,
  or escalate to raw inspection—and why?
- Did the candidate make the contract more discoverable in the paired task?
- Who authorized the prototype and the product delivery?
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

That is the first credible heartbeat of the XSH society: observation becomes
warranted belief, selected meaning becomes an authorized product, the product
changes XSH, and a scoped lesson begins—without overclaiming—to change what the
next citizens inherit.
