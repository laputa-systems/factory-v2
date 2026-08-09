# XSH Society: Factory V2 architecture

## Status and purpose

This document is the architectural charter for the cleanroom successor to the
XSH factory. It synthesizes the Factory V1 evidence and the recursive
self-improvement research agenda in `RSI.md` into one system that can be built,
tested, and amended.

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

## The charter

### Mission

The society exists to make XSH a practical, coherent, easy-to-learn,
token-efficient, and trustworthy systems-glue language for humans and coding
agents. It should increase XSH's ability to replace fragile layers of Unix
sludge with typed paths, explicit process boundaries, structured data,
structured errors, reproducible execution, and inspectable policy.

The society may propose subordinate objectives. It may not autonomously replace
this mission.

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

1. XSH or its ecosystem became better on one or more charter values without an
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
                        charter
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

Individual actors are replaceable. Continuity lives in the charter, verified
history, knowledge, institutions, and executable artifacts. Better foundation
models should enter as better citizens, not cause institutional amnesia.

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

## Constitutional topology

The architecture has four layers with deliberately different mutation rules.

```text
  R3  XSH world
      source, compiler, docs, workloads, tools, product changes
                           ^
                           |
  R2  mutable society
      actors, professions, circuits, institutions, policies, evaluators,
      graph views, context assembly, propagation, organization genomes
                           ^
                           |
  R1  amendable constitution
      mission, reserved authorities, change classes, evidence standards,
      promotion rules, autonomy envelope, human ratification
                           ^
                           |
  R0  trusted physics
      identity, ledger, transactions, capabilities, artifacts, supervision,
      resource accounting, Git lineage, replay, rollback
```

### R0: trusted physics

R0 says what happened and what a principal was allowed to do. The society
cannot rewrite it through ordinary commands. Kernel changes are external
software changes, reviewed and deployed by the human operator under the most
restrictive change class.

### R1: amendable constitution

R1 says what the society is for and how authority may change. It is versioned,
explicit, and human-ratified. The society may generate constitutional
proposals and evidence, but cannot enact them by the same circuit that benefits
from them.

### R2: mutable society

R2 is the experimental phenotype. Actor configurations, routing, professions,
communication topology, evaluators, trace curation, resource policy, and most
decision circuits are candidates for bounded mutation.

### R3: XSH world

R3 is both the mission target and, increasingly, the society's policy and
execution medium. Product mutations remain independently reviewable and
revertible. Self-reference never grants additional authority.

This topology resolves an otherwise dangerous ambiguity. “Immutable” does not
mean every initial policy is frozen into Rust, and “self-improving” does not
mean the system may redefine its mission or scoreboard.

## Three forms of durable truth

The implementation must not collapse three different records into one.

| Record | Meaning | Mutation rule |
| --- | --- | --- |
| Event ledger | Commands accepted, transitions made, resources spent, principals responsible | Append-only; replay-auditable |
| Evidence store | Immutable bytes and observations produced in a named environment | Sealed; new evidence may contradict but not edit old evidence |
| Epistemic graph | What is claimed, believed, disputed, decided, and applicable now | Revisioned; status can change with provenance |

The ledger is not the world model. An artifact is not an interpretation. A
current belief is not historical fact. Keeping those distinctions hard is what
makes later retrospection and retraction honest.

The phrase “causal graph” is shorthand for a graph of **causal claims and
decision provenance**. An edge which says an outcome supports an attribution
does not make causality mechanically true.

## The trusted substrate

### Kernel responsibilities

A small V2-owned Rust kernel initially owns one organization-wide SQLite
database and one content-addressed artifact root. It is authoritative for:

- object, revision, configuration, principal, actor, session, and episode
  identity;
- typed graph edges and the allowed endpoint kinds for each edge;
- commands, optimistic generations, idempotency, transactions, and ordered
  immutable events;
- capabilities, jurisdiction, leases, claims, expiry, cancellation, and exact
  resource ownership;
- budgets for model work, compute, wall time, worker slots, integration risk, and
  organizational experiments;
- sealed input snapshots, environment manifests, raw sessions, evaluator
  revisions, observations, Git commits, patches, and outcome artifacts;
- state-machine validation, retries and attempt lineage, rollback references,
  and retention status;
- scheduler-visible readiness facts and work claims; and
- projection cursors and a transactional outbox for rebuildable views and
  notifications.

Node-specific data that affects readiness, authority, propagation, or
evaluation is relational and constrained. An opaque JSON field or EAV table
must not become an escape hatch from the protocol.

### Command boundary

All durable mutation crosses one versioned command protocol:

```text
Command {
    command_id
    principal_id
    capability
    expected_generation
    payload: ClosedCommandVariant
}

Command
  -> authenticate principal and capability
  -> validate state, references, budget, and jurisdiction
  -> atomically append event and update materialized state
  -> write outbox records
  -> return typed receipt or explicit conflict
```

No XSH program, actor, projection, or operator helper writes arbitrary SQL,
edits workflow state files, or infers completion by scanning a directory.
Content-sensitive idempotency prevents one command identifier from being reused
with different semantics.

### Identity and lineage

The following identities are distinct:

- a `Principal` is an authenticated source of authority;
- an `ActorConfiguration` is a versioned heritable configuration;
- an `ActorInstance` is one admitted realization of that configuration;
- a `Session` is one bounded model or deterministic execution attempt;
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

An artifact reference includes a safe relative path, byte length, digest,
media/schema type, producing attempt, closed role, and retention class.
Execution evidence additionally pins:

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

### Native Pi execution is the baseline

Actors run through the host-installed `pi` executable in owned native working
directories. Product-changing actors receive dedicated Git worktrees; other
actors receive ordinary attempt directories containing only their declared
inputs and output contract. The runner pins Pi provider, model, thinking level,
tool allowlist, context-discovery flags, system prompt, session path, current
directory, environment allowlist, wall budget, and input digests in the attempt
manifest.

A working directory is an ownership and reproducibility boundary, not a
security sandbox. A Pi actor with the `bash`, `read`, `edit`, or `write` tools
can potentially reach other host paths allowed by the operator's OS account.
The first implementation relies on explicit assignments, narrow Pi tool
profiles where practical, process supervision, before/after workspace and Git
checks, and forensic tool events. It must not claim that kernel graph
capabilities enforce host filesystem or network confinement.

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
| CharterValue | A ratified value used to explain tradeoffs, never a hidden weight |
| Objective | A scoped desired capability or condition derived from the charter |
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
| ActorConfiguration | A heritable actor genome revision |
| OrganizationConfiguration | The heritable society-level genome active for specified work |

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
value, infeasibility, superseding evidence, exhausted budget, or explicit
operator choice.

### Episode state contract

The kernel owns legal operational transitions while the graph owns their
meaning:

```text
framed
  -> admitted | deferred | rejected
admitted
  -> investigating
investigating
  -> deliberating | blocked | abandoned
deliberating
  -> decided | investigating | preserved_conflict
decided
  -> implementing | observing | closed_no_change
implementing
  -> validating | reverted | failed
validating
  -> observing | reverted | investigating
observing
  -> learning | reopened
learning
  -> closed | reopened
```

Each transition has required nodes, evidence, authority, and outstanding
obligations. A `closed` episode may still be reopened by a scheduled outcome or
contradictory evidence. Operational state never deletes epistemic conflict.

## Curated provenance

Raw traces and scalar rewards are opposite information failures. V2 preserves a
layered compression path:

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

Every compression points to its sources, identifies its curator and method,
states excluded or unavailable information, and remains challengeable. Raw
sessions are forensic evidence under retention policy, not the default memory
shown to every actor. Summaries are navigation aids, not replacement evidence.

Trace curation is itself an evaluated institutional function. Historical
episodes periodically test whether a curator omitted facts which later mattered,
overstated attribution, erased dissent, or caused irrelevant context to flood
future work.

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
No role name implies authority. The database records the exact delegation and
its expiry.

### Decision packet

Every consequential decision preserves:

```text
question and decision authority
eligible alternatives, including no change
applicable charter values and hard constraints
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
| C2 product mutation | Merge an XSH fix or feature | Independent evidence, product review, tests, revert path, human authority initially |
| C3 institutional mutation | Change routing, actor population, context, or evaluator policy | Organization trial, baseline, diversity guard, canary scope, rollback |
| C4 constitutional amendment | Change values, reserved authority, or promotion standard | Explicit amendment episode and human ratification |
| C5 trusted-kernel mutation | Change ledger, capability, accounting, or evidence physics | External implementation and review; society cannot deploy it |

A change can only move inward through the authority rings by a stricter process.
The proposer, beneficiary, implementer, evaluator, and promoter need not always
be five actors, but independence requirements increase with the change class.
No mutation certifies its own evaluator or grants itself wider authority.

### Human sovereignty and graduated autonomy

Initially, a human operator ratifies the charter, controls C2-C5 promotion,
defines external resource ceilings, and may pause, veto, or revert the society.
The society should make human judgment higher leverage by delivering compact
decision packets, not conceal judgment behind automation.

Autonomy is a capability granted for a defined class, scope, budget, and expiry
after observed calibration. It is not a global maturity level. The system might
earn autonomous low-risk documentation integration while semantic changes
remain human-controlled indefinitely.

## Actors, culture, and professions

### Actor configuration is a genome, not a job title

An actor is a versioned policy for creating bounded cognitive work, not a
persistent chat persona. Its heritable configuration may include:

```text
ActorGenome {
    model_and_inference_policy
    cognitive_and_epistemic_biases
    exploration_exploitation_bias
    contradiction_and_risk_sensitivity
    tool_and_repository_capabilities
    memory_retrieval_and_context_policy
    communication_edges_and_bandwidth
    authority_and_budget_ceiling
    demand_signal_response_policy
    persistence_and_retirement_policy
    reproduction_recombination_and_mutation_policy
}
```

The genome identifies predisposition. The phenotype is observed behavior on a
problem distribution: what the actor notices, which work it selects, how well
calibrated it is, how it interacts, and what downstream effects its
contributions have.

“Researcher,” “challenger,” and “integrator” are initial developmental
attractors. They are not hard-coded classes. Useful starting biases are:

```text
explore  build  measure  challenge  synthesize  integrate  remember  coordinate
```

These name basic functions rather than human management ranks.

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

### Charter stewardship

Maintains the ratified mission, values, reserved powers, risk classes, and
amendment history. It accepts challenges and constitutional proposals but
cannot self-ratify them.

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
charter relevance
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

### The society-level genome

An `OrganizationConfiguration` pins the R2 variables which could affect an
episode:

```text
OrganizationGenome {
    actor_population_and_diversity_policy
    actor_genomes_and_model_assignments
    institutions_and_circuits
    communication_and_context_topology
    jurisdiction_and_escalation_rules
    work_selection_and_resource_policy
    evaluator_and_challenge_assignment
    memory_retrieval_and_trace_curation
    propagation_and_retraction_policy
    outcome_followup_policy
    reproduction_mutation_and_retirement_policy
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

XSH is the society's policy and execution medium wherever durable transaction
semantics do not belong in the kernel. Initial XSH-owned surfaces include:

- reproducible experiment descriptions and workload composition;
- bounded process, filesystem, JSON, text, byte, and host-state operations;
- policy and circuit variants expressed through typed kernel commands;
- evaluator programs and deterministic judges;
- context and human-readable decision projections;
- integration, replay, and outcome-observation scripts; and
- the society's own native behavior tests at the XSH boundary.

Rust owns SQLite, transactions, migrations, access enforcement, leases,
artifact sealing, ledger replay, and crash recovery. XSH never becomes a
database DSL or an alternate durable workflow engine.

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

XSH earns deeper social responsibility in stages:

1. **Experimental medium**: fresh experiments and deterministic evaluators are
   written in XSH.
2. **Policy medium**: scheduling, context, and circuit variants are XSH modules
   over the typed protocol.
3. **Cultural medium**: lessons include executable XSH examples and native
   qualification cases.
4. **Institutional medium**: mature institutions run predominantly through
   XSH policies while the Rust kernel retains durable physics.
5. **Evolution medium**: organization trials can demonstrate that a candidate
   XSH feature improves institutional productivity under matched conditions.

Advancement is per surface, reversible, and evidence-based. There is no
deadline to rewrite the compiler or database kernel in XSH.

### Self-hosting in the relevant sense

XSH is intentionally a systems-glue language, not a general-purpose service
runtime. The society must not distort the language merely to satisfy a poetic
compiler-self-hosting milestone.

The meaningful self-hosting invariant is instead:

> The society can reconstruct, explain, test, and increasingly conduct its own
> XSH-facing work using the version of XSH produced by its lineage, from a small
> frozen host and kernel boundary.

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

Such trials need multiple tasks and contamination controls. A result can justify
a feature whose local runtime cost is outweighed by a demonstrated gain in
correct modification, review, or institutional expressiveness. The trade stays
visible rather than becoming one self-improvement score.

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

### Circuit breakers

The kernel and constitution support:

- immediate admission freeze by scope or change class;
- cancellation of owned process trees and leases;
- budget cutoff which preserves partial evidence;
- quarantine of an actor, model, evaluator, lesson, or organization revision;
- product and policy rollback to an exact parent configuration;
- retraction fan-out to affected active work;
- read-only forensic mode; and
- full materialized-state rebuild from schema migrations, ledger, and sealed
  artifacts.

Emergency action records authority and reason. It may stop harm before normal
deliberation, but requires a later retrospective and cannot erase the evidence
which triggered it.

### Safety invariants

At minimum, deterministic tests prove that:

- no principal mutates facts outside its current capability and jurisdiction;
- no accepted command changes historical payload under an existing identity;
- no actor expands its own budget, authority, population, or evaluator;
- no mutable projection can create graph or readiness facts;
- no result cites an unsealed or mismatched input/evaluator revision;
- no product or institutional mutation promotes itself;
- cancellation releases only exactly owned resources and preserves evidence;
- a projection can rebuild and the ledger can independently reproduce current
  materialized state; and
- a revoked invariant blocks new work and exposes every unresolved dependent.

## Interfaces and projections

The database and artifact store are authoritative; people and actors interact
through typed commands and rebuildable projections.

### Required views

Initial projections include:

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
- a constitutional view showing effective charter, delegations, amendments,
  and reserved powers; and
- a human review packet that links every summary claim to graph revisions and
  sealed evidence.

Markdown is an excellent human projection and XSH is an excellent policy and
query client. Neither is durable truth. All projections carry the source event
cursor and can be discarded and rebuilt.

### Questions the system must answer

The architecture is successful only if ordinary typed queries can answer:

- What did the society believe and not know before this change?
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

## Physical implementation map

The architecture should remain logically modular even if the first deployment
is one process and one database.

### Rust kernel

The first Rust workspace should expose narrow components for:

```text
protocol        closed commands, receipts, errors, version negotiation
identity        principals, configurations, revisions, sessions, episodes
authority       capabilities, jurisdiction, delegation, expiry
graph           node/edge contracts and revision validation
workflow        operational state machines and readiness facts
ledger          events, generations, idempotency, replay audit
artifacts       content addressing, sealing, role and retention
resources       budgets, leases, claims, cancellation, accounting
execution       manifests, attempt lineage, supervisor and workspace receipts
projections     cursors, outbox, rebuild contracts
repository      Git/worktree identity and product-delivery receipts
```

Module boundaries may change; these ownership boundaries may not disappear into
one generic job table. SQLite migrations and protocol versions are reviewed
contracts from the first vertical slice.

### XSH surface

Initial XSH modules should own:

```text
society.client       typed protocol invocation and error handling
society.experiment   manifest construction and bounded execution requests
society.policy       circuit, admission, and scheduling variants
society.context      declared graph/context projections
society.evaluate     deterministic and sampled evaluator composition
society.product      isolated XSH change and verification workflows
society.observe      scheduled outcome and ecosystem probes
society.review       human-readable decision and evidence packets
```

These are domain targets, not prematurely frozen names. They use typed paths,
structured data, explicit `Result` errors, capability-aware process execution,
and stable content identifiers. If XSH cannot express one clearly, the gap
becomes a Question with a minimal host boundary, not an invisible Python or
shell escape.

### One authority, many workers

SQLite is initially one-writer transactional authority. Agent and deterministic
workers are horizontally parallel outside it. They claim bounded work, execute
in owned native directories or worktrees, and submit typed results. The design
does not require a distributed database to demonstrate a society.

If scale later requires another store, the migration must preserve command,
ledger, ordering, lease, and replay semantics. Distribution is not itself an
institutional intelligence gain.

## Bootstrap program

The society should be built in stages which each close a meaningful evidence
loop. Later-stage vocabulary may exist in the schema before autonomous machinery
uses it.

### Stage 0: ratify the laboratory contract

Write the initial charter, node and relation schemas, command protocol, change
classes, capability lattice, episode transitions, artifact manifest, and V1
import contract. Define which clauses are R0, R1, or R2 before implementation
makes the answer expensive.

Exit evidence:

- schema and transition examples cover a successful, failed, contested, and
  reopened episode;
- every mutable surface has a named promotion authority; and
- every external invariant has a deterministic owner.

### Stage 1: build trusted physics

Implement the Rust kernel, SQLite migrations, content store, ledger, authority,
resources, execution receipts, worktree lineage, projections, and replay audit.
Use deterministic transaction, state-machine, concurrency, recovery, and
fault-injection tests. No paid model work is necessary.

Exit evidence:

- an interrupted attempt recovers without duplicated work or lost evidence;
- forged identity, stale generation, authority escalation, artifact mismatch,
  and resource overrun are rejected;
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

1. a scoped objective and resolution condition;
2. three competing hypotheses: missing behavior, culturally stale records, and
   split or accidental behavior;
3. curated V1 usage as historical evidence without importing its controller;
4. a deterministic behavior/documentation matrix plus a paired native-Pi
   baseline/candidate task probe;
5. a preserved conflict if the evidence is underdetermined;
6. a decision packet with no-change option, predictions, dissent, and revisit
   triggers;
7. one bounded XSH reconciliation commit if authorized;
8. short- and delayed-horizon outcomes; and
9. a retrospective on both the XSH decision and the circuit used.

Exit evidence:

- a new actor can reconstruct the decision without raw transcript archaeology;
- replay reproduces deterministic observations from sealed inputs;
- contradictory test evidence can reopen the episode; and
- the complete episode is queryable by configuration, cost, prediction, and
  product lineage; and
- the exact validated commit reaches the local XSH target only after explicit
  human confirmation.

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

Promote a supported lesson through L0-L2, inject it into a matching context,
measure a relevant behavior change, then introduce contradictory evidence and
exercise downgrade or retraction.

Exit evidence:

- relevant active work receives the correct revision;
- irrelevant work is sampled for contamination; and
- dependents reopen or acknowledge the retraction without history edits.

### Stage 5: compare organizational circuits

Create a small stratified corpus of historical and held-out XSH episodes.
Compare at least two circuits under matched capability and resource envelopes,
with outcome sequestration and an explicit Pareto analysis.

Exit evidence:

- a variant has a demonstrated scope rather than a universal win claim;
- the organization archive retains non-dominated and behaviorally distinct
  configurations; and
- the trial can be rerun from its configuration and evidence manifests.

### Stage 6: permit bounded metamorphosis

Promote one organization mutation into a narrow canary jurisdiction, observe
later work, and either retain or roll it back. Begin actor variation and local
demand signaling only after attribution is credible enough to tell population
change from noise.

Exit evidence:

- the mutation satisfies the strict RSI criterion or is honestly reported as a
  failed organizational hypothesis; and
- population diversity and reserved authority survive the trial.

### Stage 7: measure XSH-society co-evolution

Choose an XSH feature directly relevant to experiments or policy expression.
Compare ancestral and descendant XSH revisions on held-out society work under a
matched envelope.

Exit evidence:

- local product tradeoffs and institutional productivity effects are both
  visible; and
- the result can revise either the language feature or the institution which
  selected it.

### Stage 8: expand autonomy by evidence

Grant narrow, expiring C2 or C3 capabilities only where the prior stages show
calibration, containment, rollback, and human review leverage. There is no
milestone called “fully autonomous society.” Authority remains decomposable and
revocable.

## V1 migration

V1 remains frozen and runnable. V2 imports a deliberately selected evidence
corpus, not controllers or live workflow state.

Each imported episode records:

- immutable source paths and content hashes;
- the V1 run, evaluator, worker, report, ticket, replay, and product commits
  which actually exist;
- the curator and mapping from V1 artifacts into V2 node revisions;
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
- a mutable evaluator certifying itself, a circuit granting itself authority,
  or a society rewriting its charter; and
- claims of autonomous improvement that omit human interventions, failed
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
14. What is the minimum human constitutional role compatible with honest,
    bounded recursive institutional improvement?

The society should ingest its own answers as scoped lessons, not quietly bake
the first plausible answer into infrastructure.

## The complete invariant

The architecture can be summarized as follows:

> Build a constitutional machine society whose replaceable actors use a shared,
> typed epistemic commons to investigate and improve XSH; whose institutions
> turn disagreement and scarce resources into reproducible decisions and
> shipped artifacts; whose checked culture carries warranted discoveries into
> future behavior; and whose organizational variants can be compared, selected,
> and revoked without changing the trusted history, mission, or scoreboard that
> makes improvement meaningful.

If V2 can do that at small scale, it is already more than a higher-throughput
software factory. It is a contained experiment in cumulative machine culture,
with XSH simultaneously serving as its public work, its evolving systems
language, and an increasingly capable medium for the society's own thought in
action.
