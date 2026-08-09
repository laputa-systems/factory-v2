# VS-001: native-host epistemic-to-product vertical slice

## Decision

The first V2 slice will reconcile XSH's contradictory `spawn` stderr contract
from source observation through one shipped XSH commit and checked institutional
memory.

The slice uses host-installed Pi actors in owned native working directories and
Git worktrees. It does not build or run Docker images. Environmental austerity
is a future experimental treatment, not the baseline worker architecture.

This is a vertical slice in the strict sense: it touches every architectural
ring and crosses every durable boundary once.

```text
charter objective
  -> typed question and competing hypotheses
  -> native Pi inquiry attempts
  -> deterministic contract experiment
  -> preserved argument and conflict
  -> human-authorized decision
  -> native Pi product worktree
  -> independent review and XSH judges
  -> exact XSH delivery
  -> immediate and scheduled outcomes
  -> scoped lesson and propagation receipt
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

## Episode charter

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
- typed graph revisions preserve what was known before the decision;
- native Pi attempts have pinned assignments, configurations, workspaces,
  sessions, costs, tool events, and outputs;
- observations are admitted by deterministic judges rather than actor prose;
- the decision retains alternatives, dissent, and predictions;
- the exact authorized XSH commit reaches the chosen product ref only after
  validation and human approval;
- a product invariant and one scoped lesson propagate to declared dependents;
- an outcome obligation survives past delivery; and
- a projection can rebuild the episode from the ledger and sealed artifacts.

### It must not claim

- statistically persuasive agent-performance improvement from one paired trial;
- general organizational superiority;
- autonomous language governance;
- security isolation from native working directories;
- a complete actor ecology, learned profession system, or organization genome
  search; or
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
filesystem:         ordinary operator account access; not claimed confined
process:            owned process group with wall timeout and cancellation
environment:        declared allowlist; secret values never sealed
```

The environment manifest records this as a collection of fields, not
`cleanroom: false`.

### Pi invocation

The runner composes an argv equivalent to:

```text
pi
  --provider <provider>
  --model <model>
  --thinking <level>
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

Pi 0.83.0 has no `PI_AUTH_FILE` interface. It resolves authentication from
`auth.json` beneath `PI_CODING_AGENT_DIR`, whose default is `~/.pi/agent`.
The supervisor therefore passes an explicit operator-configured
`PI_CODING_AGENT_DIR` and records a logical configuration identity. It seals
the effective non-secret model configuration needed to reproduce model
selection, but never copies `auth.json`, credential values, or a secret-bearing
environment into the artifact store. The separately explicit `--session` path
keeps attempt transcripts inside the owned workspace rather than the agent
directory. Model API traffic is necessary; `--offline` is not treated as
network confinement.

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

### Tool profiles

VS-001 uses three explicit profiles:

| Profile | Pi tools | Intended use |
| --- | --- | --- |
| `read_source_v1` | `read,bash,grep,find,ls` | Source and contract inquiry in a detached worktree |
| `product_builder_v1` | `read,bash,edit,write,grep,find,ls` | One authorized XSH branch worktree |
| `task_worker_v1` | `read,bash,edit,write,grep,find,ls` | Produce an XSH script in a fixture workspace |

`bash` makes all three profiles broad at the OS level. The profile is an
experimental/tool-availability declaration, not a security capability. The
runner rejects undeclared Pi extensions and preserves every reported tool call
and nonzero tool result.

### Workspace layout

Runtime state is outside Git under a configurable root, initially
`var/` in the V2 checkout:

```text
var/
├── society.sqlite3
├── objects/sha256/<prefix>/<digest>
├── workspaces/<attempt-id>/
│   ├── manifest.json
│   ├── input/
│   │   ├── ASSIGNMENT.md
│   │   ├── SYSTEM.md
│   │   └── context/
│   ├── work/
│   │   └── repo/                 # only when a Git worktree is assigned
│   ├── output/
│   │   └── submission.json
│   ├── pi/
│   │   ├── session.jsonl
│   │   ├── events.jsonl
│   │   └── stderr.log
│   └── receipt.json
└── projections/
```

`society.sqlite3` and `objects/` are durable and backed up together.
`workspaces/` is an owned staging area. Required evidence is sealed into
`objects/` before a workspace becomes cleanup-eligible. `projections/` can be
deleted and rebuilt.

The V2 source repository contains schemas, migrations, policies, prompts,
deterministic fixtures, and tests. It does not commit paid sessions or a second
copy of the durable database.

### Workspace classes

`FixtureWorkspace`

Contains only the declared task, fixtures, assigned XSH/Xsht binaries, and
output paths. It has no Git repository. VS-001 task workers use this class.

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
reruns the required judges, and creates the provenance commit from that exact
tree without invoking repository hooks. The branch and source workspace are
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
9. seals raw and normalized evidence before releasing the lease.

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
supporting_artifacts[]
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

Task-worker submissions contain the requested `supervise.xsh` artifact and a
small receipt naming the XSH binary it used. Reviewer submissions contain
findings with severity, evidence references, and an explicit disposition.

The deterministic runner validates the schema and submits the contribution to
the kernel on behalf of the actor instance. Invalid or missing submissions end
as `protocol_failed`; they are never repaired by parsing narrative prose.

## Minimal implementation architecture

### Source tree

The first implementation target is:

```text
Cargo.toml
crates/
├── society-kernel/        # schema, commands, transitions, ledger, artifacts
└── society-cli/           # operator and XSH-facing protocol client
migrations/                # monotonic V2-owned SQLite migrations
xsh/
└── society/
    ├── client.xsh
    ├── native_runner.xsh
    ├── product.xsh
    ├── evaluate.xsh
    └── project.xsh
circuits/
└── vs-001-spawn-stderr/
    ├── CHARTER.md
    ├── actor-configs/
    ├── assignments/
    ├── contexts/
    ├── fixtures/
    ├── judges/
    └── projections/
tests/
├── kernel/
├── protocol/
├── runner/
└── vs-001/
```

Names may be refined before code exists. Ownership may not collapse into a
generic workflow framework.

### Process topology

```text
human / XSH policy
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
       +----> sealed attempt receipt -----> society-kernel
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
principals
capability_grants
commands
events
objects
object_revisions
edges
episodes
episode_objects
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
resource_budgets
resource_usage
workspaces
artifacts
attempt_artifacts
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
propagation_receipts
projection_cursors
outbox
```

Revision body tables store searchable contract fields such as scope,
resolution condition, prediction horizon, evaluator revision, authority,
revisit trigger, and propagation level in typed columns. Long human prose is a
sealed artifact referenced by digest. Closed structured details may be stored
as schema-validated JSON only when they do not determine authority, readiness,
or transition legality.

### Minimal command set

The versioned protocol needs only these public commands for VS-001:

```text
CreateEpisode
AddObjectRevision
AddEdge
TransitionEpisode
RegisterActorConfiguration
AdmitActorInstance
RegisterWorkItem
ClaimWorkItem
StartAttempt
CompleteAttempt
SealArtifactReference
SubmitContribution
RecordDecision
AuthorizeImplementation
RecordProductCommit
RecordValidation
RecordDelivery
ScheduleOutcome
RecordOutcome
PromoteLesson
RecordPropagationReceipt
ReopenEpisode
CloseEpisode
CancelAttempt
```

Each command includes principal, capability, command id, expected generation,
and a closed payload. Read queries and projection rebuilds are separate from
commands.

### Slice state machines

Episode:

```text
framed -> admitted -> investigating -> deliberating -> decided
       -> implementing -> validating -> observing -> learning -> closed
```

The legal side paths are:

```text
any active state -> blocked | abandoned
investigating <-> deliberating
implementing -> failed | reverted
validating -> implementing | reverted
observing | learning | closed -> reopened
```

A conflict is a graph object, not an operational terminal state. A decision may
preserve a conflict, defer action, or authorize an experiment without deleting
the disagreement.

Attempt:

```text
registered -> claimed -> preparing -> running
           -> submitted -> judged -> accepted | rejected
```

Cancellation, expiry, protocol failure, runner failure, model failure, and
judge failure are separate terminal classifications with attempt lineage for a
retry. Retrying never overwrites the failed attempt.

Product change:

```text
authorized -> worktree_ready -> committed -> validated
           -> reviewed -> integration_ready -> delivered
           -> observed | reverted
```

Lesson:

```text
candidate -> validated -> promoted_l1
          -> delivered_to_targets -> acknowledged
          -> expired | downgraded | revoked
```

VS-001 uses only L1 candidate guidance plus a product invariant. It does not
pretend one episode justifies broad institutional policy.

### Initial capabilities

```text
operator:
  ratify, allocate, decide_c2, authorize_product, deliver_product,
  promote_lesson, cancel_any, reopen

runner:
  claim_work, start_attempt, seal_attempt_artifact, complete_attempt,
  submit_attested_contribution

inquiry_actor:
  no direct durable capability

product_actor:
  no direct durable capability; OS access only inside assigned worktree by
  operating convention

deterministic_judge:
  record_observation for one experiment and evaluator revision

projector:
  read events and revisions, advance only its projection cursor
```

Kernel capabilities govern institutional state. They do not imply that a
native Pi actor is OS-confined; that is an explicitly separate execution fact.

### Artifact contract

Every artifact reference records:

```text
digest_algorithm
digest
byte_length
media_or_schema_type
closed_role
producing_attempt_or_command
source_relative_path
created_time
retention_class
```

VS-001 seals assignments, context packs, Pi sessions, Pi JSON events, stderr,
submissions, source snapshots, experiment manifests, evaluator outputs, Git
diffs, commits/bundles where needed, test logs, decision prose, projection
packets, and outcome receipts.

The content store refuses a different byte sequence at an existing digest path.
The database transaction may refer only to an object already durably sealed.

### Projection contract

The first projection is one self-contained episode packet:

```text
VS-001.md
VS-001.json
```

It shows the charter, graph revisions, attempts, observations, hypotheses,
arguments, conflict, decision, product lineage, validation, outcome obligations,
lesson, and propagation receipts. Every assertion links to an object revision
or sealed artifact. The packet names the latest consumed event id and rebuilds
byte-identically after timestamps and absolute runtime-root paths are
normalized.

## Exact experiment package

### Pinned inputs

Episode admission snapshots:

- the clean XSH base commit current when VS-001 begins;
- `LANG.md`, `docs/SPEC.md`, `docs/SPEC-OS.md`, relevant API registry entries,
  managed-spawn lowering/runtime owners, and focused process tests at that
  commit;
- the two Laputa QEMU TODO call sites as external evidence, without giving V2
  authority over the Laputa repository;
- representative Factory V1 native-host `spawn ... stderr:` call sites as
  historical evidence, not imported implementation;
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

Each task worker receives the same `TASK.md`:

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
│   ├── noisy-child.xsh
│   └── cases.json
└── output/
```

The front of `PATH` points to the assigned `bin/`, and the evaluator invokes the
produced script with the assigned binary by absolute path. The task worker may
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

One worker per arm is sufficient only to prove the experimental path. The
decision packet may use the results as qualitative or case evidence; it cannot
claim a population-level agent improvement. Later organization science should
replicate the probe across tasks and seeds.

### Task-worker judges

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

## Initial actor population

VS-001 uses six Pi instances and deterministic services. These names describe
the seeded function; their configuration digests are authoritative.

### 1. Contract cartographer

- profile: `read_source_v1`;
- workspace: detached XSH read worktree;
- sees: question, hypotheses, pinned source/doc paths, no other actor output;
- must: map every stdio field from syntax and registry through lowering to each
  runtime consumer and identify contradictory records; and
- cannot: propose a product diff as if mapping alone authorized it.

### 2. Boundary skeptic

- profile: `read_source_v1`;
- workspace: independent detached XSH read worktree;
- sees: the same question and hard constraints, but not the cartographer's
  contribution;
- must: search for counterexamples involving detached ownership, setup failure,
  append behavior, cancellation, and misleading V1 precedent; and
- cannot: resolve the question by majority agreement.

### 3. Prototype builder

- profile: `product_builder_v1`;
- workspace: XSH product worktree at the pinned base;
- sees: a human-authorized C1 prototype decision packet, the admitted evidence,
  preserved conflict, exact expected contract, and required XSH gates;
- must: implement the smallest coherent candidate, update all canonical owners,
  remove or revise the stale `LANG.md` proposal, and leave a portable patch; and
- does not: commit, merge, or change the evaluator.

### 4–5. Paired task workers

- profile: `task_worker_v1`;
- workspace: independent fixture directories, one per opaque treatment;
- sees: only the task and its treatment's declared reference/binary inputs; and
- must: produce `supervise.xsh` plus a valid submission.

### 6. Product adversary

- profile: `read_source_v1`;
- workspace: fresh read worktree materialized with the candidate patch;
- sees: question, decision dimensions, evidence matrix, candidate diff, tests,
  and paired-trial results;
- must: challenge contract coherence, missing docs/tests, compatibility,
  lifecycle safety, treatment interpretation, and product readiness; and
- cannot: modify the candidate or deliver it.

The human operator owns C1 prototype admission, the C2 product decision, lesson
promotion, and delivery. Deterministic judges own behavioral observations.

## Pre-registered predictions

Before inquiry attempts run, VS-001 records:

1. Current evidence makes `H2` most likely: managed `spawn command` will honor
   `stderr: Path` correctly, while `LANG.md` and some normative/discovery text
   remain stale or incomplete.
2. At least one consumer or documentation source will require an explicit
   qualification, preventing a one-line “already implemented” closeout.
3. The baseline task worker may solve the task by source/API archaeology, but
   the candidate worker should require no shell workaround and should find a
   direct documented path.
4. The paired trial is too small to establish a reliable token/turn reduction;
   any such difference remains a tentative observation.
5. A coherent reconciliation commit will be smaller and safer than adding a new
   `spawn` parameter, because stderr policy already belongs to `Command`.
6. Existing spawn, wait, cancellation, process, API, and documentation gates
   will remain green after the candidate.

If the behavior matrix contradicts prediction 1, the episode revises the
argument and follows `H1` or `H3`; it does not change the prediction text.

## Exact episode execution

### Step 0: qualify the machinery

Before a paid call, native tests use Pi process doubles and deterministic XSH
fixtures to prove command transactions, runner recovery, session/event
normalization, evaluator negatives, and projection rebuild. The kernel records a
qualified organization configuration.

Gate: no paid work may start while a required R0 test or negative control fails.

### Step 1: frame and admit

The operator creates the Objective, Question, three Hypotheses, hard
constraints, resolution condition, Predictions, initial source Observations,
and an Experiment proposal. The kernel pins XSH and Pi inputs, configuration,
budget, and evaluator revisions in one admitted Episode.

Gate: the episode projection must reconstruct the pre-agent belief state.

### Step 2: run independent inquiry

The scheduler creates the cartographer and skeptic work items concurrently.
Each claims a lease and runs Pi in its own native read worktree. The runner
seals sessions, event streams, submissions, Git receipts, and normalized usage.

Gate: at least one valid contribution and the deterministic behavior matrix are
required. One actor's failure does not erase the other's result; a retry is a
new attempt requiring operator budget.

### Step 3: execute deterministic discovery and behavior judges

The laboratory runs the behavior and documentation matrices. Judge output
becomes Observation nodes. Actor claims become Arguments linked to exact source
revisions and observations. Contradictions become a Conflict node rather than a
manager summary.

Gate: every result has a sealed manifest and evaluator digest; all negative
controls have previously demonstrated their expected failure.

### Step 4: deliberate and authorize one prototype

The episode enters `deliberating`. A human packet presents H1/H2/H3, evidence,
unknowns, proposed product contracts, no-change, and costs. The operator records
a C1 Decision authorizing exactly one prototype contract and changed-path
boundary.

Gate: the prototype builder cannot start from an Argument or queue entry; it
requires the exact decision revision and capability grant.

### Step 5: build the coherent candidate

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

### Step 6: run the paired fluency probe

The laboratory builds and seals baseline and candidate XSH/Xsht inputs,
generates their reference packs, assigns opaque labels, and runs the two fresh
task workers. It evaluates both through the same judge and reveals treatment
mapping only after both submissions are sealed.

Gate: treatment contamination, missing settlement, invalid submission, or
evaluator failure remains a distinct outcome and cannot be imputed as task
failure.

### Step 7: adversarial product review

The product adversary reviews a fresh materialization plus all admitted
evidence. Deterministic tests remain authoritative for their domains; the
adversary supplies Arguments about coherence, compatibility, missing cases, and
the strength of the agent-fluency inference.

Gate: blocking findings create linked conflicts or a descendant implementation
attempt. Narrative approval alone cannot set `integration_ready`.

### Step 8: make the C2 decision

The human packet contains:

- the original predictions and current hypothesis status;
- the behavior and documentation matrices;
- independent inquiry and dissent;
- exact patch, tree digest, test receipts, and adversarial findings;
- paired task-worker vectors with contamination status;
- alternatives, including narrower patch and no delivery;
- expected short- and delayed-horizon outcomes; and
- rollback/reopen triggers.

The operator accepts the exact patch, requests a descendant, defers, or rejects.
The decision does not edit the C1 decision or earlier claims.

### Step 9: materialize and deliver the product commit

On acceptance, a deterministic product materializer:

1. creates a fresh worktree at the recorded XSH base;
2. applies the exact accepted patch;
3. verifies the resulting tree digest;
4. reruns the required focused and broad gates;
5. creates one controlled provenance commit from the verified tree without
   invoking Git hooks;
6. records parent, tree, commit, author policy, message, patch, and test digests;
7. verifies the target XSH checkout is clean and still at the expected base;
8. presents the exact delivery receipt for final operator confirmation; and
9. fast-forwards the local XSH target to that commit.

If XSH `HEAD` moved, delivery refuses. A rebase is not automatic: the patch is
materialized against the new base as a descendant attempt and revalidated.
VS-001 never pushes a remote.

Gate: delivered `HEAD`, authorized commit, tested commit, and recorded commit
must be identical.

### Step 10: observe, propagate, and close

Immediate outcome checks rebuild XSH at delivered `HEAD`, rerun the contract
matrix, query the public API, verify the stale proposal is gone, and confirm the
delivery diff. They create an Outcome, not a rewritten Decision.

The accepted product contract creates an enforced product Invariant backed by
its focused test. The episode may also create one L1 candidate Lesson:

> Before proposing a new systems-glue API, compare the normative registry,
> executable behavior, active proposal corpus, and real call sites; a stale
> proposal is an institutional-memory defect, not evidence that runtime work is
> missing.

The operator may promote that lesson only to L1. Its declared propagation
target is the context pack for the next process-API inquiry. A propagation
receipt proves that the exact lesson revision is present; it does not claim the
lesson changed behavior yet.

A delayed outcome obligation asks whether the two Laputa QEMU call sites can
adopt the delivered contract without a shell wrapper and whether later XSH
process changes preserve the invariant. It has an owner, due condition, and
reopen trigger.

The Retrospective assesses graph sufficiency, runner friction, evaluator
quality, decision latency, and any facts recovered only through raw-session
archaeology. The episode closes only after every required artifact is sealed,
delivery and propagation are acknowledged, and the delayed obligation remains
durably scheduled.

## Graph realized by VS-001

The expected subgraph is concrete:

```text
O1 typed process composition
  motivates Q1 spawn stderr contract

Q1
  has_alternative H1 missing behavior
  has_alternative H2 culturally stale
  has_alternative H3 split behavior
  tested_by E1 behavior matrix
  tested_by E2 documentation matrix
  tested_by E3 paired agent probe

E1 produces observations O-B1..O-B10
E2 produces observation O-D1
inquiry attempts produce arguments A1 and A2
O-D1 contradicts current LANG proposal revision
arguments and observations compose conflict C1

D1 authorizes prototype P1
P1 implemented_by I1 candidate patch
I1 tested_by E3 and deterministic validations
adversary produces A3

D2 authorizes exact product materialization I2
I2 produces XSH commit G1
G1 produces short outcome OUT1
G1 schedules delayed outcome OUT2
OUT1 supports invariant INV1
retrospective R1 learns lesson L1
L1 propagates_to process-inquiry context target T1
```

Object identifiers are kernel-generated. These labels are projection aliases,
not IDs embedded in actor prose.

## Acceptance tests

### Kernel and ledger

- duplicate identical command returns the prior receipt without another event;
- duplicate command id with changed payload is rejected;
- stale generation and unauthorized capability are rejected;
- typed edges reject illegal endpoint kinds and uncommitted revisions;
- episode, attempt, product, and lesson state machines reject impossible
  transitions;
- event append and materialized-state update are atomic under injected crash;
- an expired lease becomes reclaimable without rewriting its attempt;
- artifact reference before successful seal is rejected;
- ledger replay reconstructs all current rows and detects tampering; and
- a projector principal cannot mutate graph, authority, readiness, or events.

### Native Pi runner

- the exact 0.83.0 argv, tool profile, `cwd`, context flags, and prompt inputs
  appear in the manifest;
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
- secret environment values never enter manifest or artifact bytes; and
- workspace cleanup is impossible before all required artifacts are sealed.

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
- the accepted patch applies to a fresh base and reproduces its tree digest;
- a changed XSH target `HEAD` blocks delivery;
- the materialized commit has exactly one expected parent and the authorized
  tree;
- immediate outcome is linked to, not substituted for, the original prediction;
- product invariant propagation updates its declared judge; and
- projection rebuild yields the same normalized episode packet.

### End-to-end judge

The ultimate VS-001 test starts from an empty database and artifact root, uses
Pi process doubles for every actor, and executes the complete episode through a
temporary product repository. It must deliver one exact commit, schedule one
outcome, promote and acknowledge one L1 lesson, close, rebuild projections, then
inject a contradictory observation and reopen the episode.

The paid live run uses the same circuit and schemas. Model output is not part of
the deterministic test oracle.

## Implementation order

### Milestone 1: contracts in executable form

Implement identifiers, commands, typed errors, SQLite migration 1, graph bodies,
edges, episode/attempt state machines, artifact sealing, and a ledger replay
command. Write the four synthetic episode fixtures first: success, actor
failure/retry, preserved conflict, and post-close reopen.

### Milestone 2: native runner with Pi doubles

Implement workspace creation, execution-profile manifests, process ownership,
Pi argv construction, event/session normalization, budget receipts, submission
validation, cancellation, and sealing. Pin real Pi 0.83.0 JSON/session fixtures
from `~/d/pi` types and v1 evidence; do not invoke a provider.

### Milestone 3: product and projection path

Implement XSH read/product worktrees, patch capture, clean materialization,
validation receipts, controlled commit construction, guarded fast-forward,
outcome scheduling, L1 propagation, and the episode projection.

### Milestone 4: VS-001 deterministic package

Implement the noisy-child fixtures, behavior matrix, documentation matrix,
agent-task evaluator, all negative controls, actor configurations, assignments,
and the end-to-end process-double test.

### Milestone 5: native Pi qualification

Run one no-product, read-source Pi qualification attempt in a disposable V2
episode with an intentionally tiny assignment. Verify real 0.83.0 events,
session, usage, submission, cancellation behavior, and sealing against the
normalizer. This is the first paid boundary and requires explicit operator
admission.

### Milestone 6: execute VS-001

Create the live episode, run the six actor instances only as their dependencies
become ready, stop for the C1 and C2 human decisions, and deliver only after the
final exact-commit confirmation.

## Deliberately deferred machinery

VS-001 does not build:

- Docker worker or evaluator images;
- a generic DAG/workflow language;
- a web UI;
- a vector database or transcript retrieval layer;
- multi-host execution or a distributed database;
- autonomous question generation, lesson promotion, product merging, or
  organizational mutation;
- actor reproduction, recombination, learned professions, pheromone scheduling,
  or an internal market;
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
for operator approval in accordance with the repository working contract.

## Completion statement

VS-001 is complete only when the V2 database can answer, with direct links to
sealed evidence:

- Why did institutional records disagree about `spawn` stderr?
- Which behavior was actually present at the pinned XSH commit?
- Which hypothesis survived, and which evidence defeated the others?
- What did each independent actor contribute and at what cost?
- Did the candidate make the contract more discoverable in the paired task?
- Who authorized the prototype and the product delivery?
- Which exact patch, tested tree, and commit reached XSH?
- What happened immediately afterward?
- Which product invariant and candidate lesson entered future context?
- What delayed evidence can reopen the decision?
- Could the whole account be reconstructed without reading raw Pi transcripts?

That is the first credible heartbeat of the XSH society: observation becomes
warranted belief, belief becomes an authorized artifact, the artifact changes
XSH, and the verified lesson changes what the next citizens inherit.
