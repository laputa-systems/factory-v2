# Institutional model-organism research program

## Thesis

Most agent evaluations treat one model invocation as the unit of agency.
Society studies a different possibility: persistent institutions can become
the effective unit of capability, error, adaptation, and self-preservation even
when no constituent actor represents the whole behavior.

The laboratory therefore uses individually limited, highly observable actors
inside durable coordination structures. It asks whether the population and its
institutions exhibit behaviors that the same actors, given the same total
resources, do not reliably exhibit alone or in an unstructured group.

The intended result is experimental sociology for artificial agents. It is not
a claim that a multi-agent system is intrinsically intelligent, safe, or
aligned.

## Governing design law

**Individual agents should be cheap and disposable. Do not make actors
increasingly stateful; make the society increasingly stateful.**

This is both an engineering constraint and the central experimental lever.
Long-lived agent sessions confound actor capability with institutional
capability, hide memory from provenance, and make actor-replacement experiments
meaningless. Durable competence must live in explicit artifacts, claims,
evidence, policies, work state, and institutional memory which a fresh actor
can receive under new bounded authority.

## Primary research question

> Under a fixed actor-policy distribution and resource budget, how does
> accumulated institutional state change collective capability, collective
> error, and recovery from error?

The first program studies information propagation and correction. Later work
may study evaluator capture, epistemic monoculture, coalition formation,
institutional goal drift, work creation for self-preservation, distributed
deception, and beneficial institutional learning. Those are research targets,
not implemented capabilities.

## Experimental unit

The unit of analysis is an `Episode`, not a model call. An episode freezes:

- one experimental protocol revision;
- one synthetic-world revision and hidden ground truth;
- one actor-policy distribution and population assignment;
- one institution revision and initial institutional state;
- one resource and communication budget;
- one intervention schedule;
- one measurement specification; and
- one randomization identity.

An accepted episode produces raw occurrences and derived measurements. It does
not produce a preferred scientific conclusion by construction.

## Weak and replaceable actors

“Weak” is an empirical qualification relative to an episode obligation. Before
a collective result may be called emergent, isolated and unstructured baselines
must establish that individual actors do not reliably exhibit the target
behavior under the same accessible evidence and aggregate inference budget.

Actors therefore remain cheap and disposable:

```text
admit narrow authority and evidence
  -> perform one local obligation
  -> emit claims, artifacts, evidence, and uncertainty
  -> relinquish authority
  -> terminate
```

Knowledge survives only when an admitted institutional transition preserves
it. Actor replacement is an intervention, not a recovery accident.

## Four planes

### Trusted physics

The slowly changing Rust, SQLite, content, and operating-system layer owns
identity, authority, budgets, byte custody, process custody, cancellation,
append-only events, and integrity replay. It never decides whether an
application claim is true.

### Institutional substrate

The societal layer owns actors, bounded work, claims, evidence, institutional
memory, knowledge promotion, communication topology, and versioned coordination
policies. An institution is executable policy plus durable state—not a title or
a long prompt.

### Experimental control

The experimental layer owns protocols, episodes, treatment assignment,
population snapshots, interventions, measurements, and counterfactual forks.
It can compare histories but cannot rewrite either history to match a desired
result.

### Experimental world

An application owns the environment's ground truth, evidence semantics, actor
obligations, and metric interpretation. It cannot mutate the generic ledger or
acquire process, budget, or scheduling authority by emitting data.

## Epistemic model

Raw platform facts and institutional interpretations remain separate.

```text
private observation
  -> local claim
  -> candidate knowledge
  -> validated knowledge
  -> institutional knowledge
```

Each promotion names its evidence requirement, authority, institution revision,
and challenge path. Corrections preserve the superseded claim and propagate to
known dependents; they do not erase history.

The system records system-mediated messages and dependencies. It does not claim
that a cited input caused an outcome or that model internals are observable.
Causal conclusions require an identified intervention, ablation, randomized
assignment, or equivalent design.

## Mission, measurement, and selection

These must never collapse into one scalar:

- `Mission` records normative purpose, constraints, open tradeoffs, and revisit
  conditions.
- `Measurement` records an observation under a named analysis procedure.
- `SelectionPolicy` determines which actors or institution revisions receive
  future resources.

Early experiments have no endogenous reproduction or policy mutation. They use
frozen actors and externally assigned institution revisions. Selection and
reinforcement learning are later treatments, admitted only after measurement
and counterfactual replay are trustworthy.

## Replay

Society needs two distinct mechanisms:

- **Integrity replay** reconstructs materialized state from the accepted event
  ledger and detects tampering or impossible transitions.
- **Experimental replay** creates a new episode fork from declared source state,
  changes exactly the named treatment variables, and retains a cross-link to
  the source episode.

An experimental fork never edits the source history. Matching seeds do not
imply matching model samples unless the runtime contract actually guarantees
that property.

## Measurements

The headline quantity is institutional leverage:

> The difference in outcome between a retained institution and a fresh
> institution under the same actor-policy distribution and episode budget.

Report it in two ways:

- **operational leverage**, which holds the measured episode budget fixed; and
- **amortized leverage**, which also charges the cost of producing and
  maintaining the retained institutional state.

Other candidate measurements include correction latency, false-belief
half-life, validated propagation latency, dissent survival, duplicate-work
rate, institutional-memory utilization, authority concentration, evaluator
bottleneck ratio, cross-population transfer, and recovery after intervention.
Each becomes durable only when its exact observation and analysis contract are
defined by an experimental world.

## Research sequence

1. **Correction latency.** Build CL-001 with synthetic ground truth, partitioned
   evidence, actor replacement, retained versus reset institutional memory, and
   one delayed correction.
2. **Propagation topology.** Compare centralized and decentralized correction
   routing without changing actors or evidence.
3. **Authority amplification.** Vary how source role and citation count affect
   promotion while retaining the same raw observations.
4. **Institutional transfer.** Move mature institutional state to a fresh actor
   population and measure retained competence and retained error.
5. **Institution mutation.** Only after the earlier experiments are
   reproducible, admit externally proposed policy revisions and compare them in
   forked episodes.
6. **Selection and learning.** Introduce population or policy selection as an
   explicit treatment. Do not allow reward to become mission authority.

Open-ended software work may later become a demanding experimental world. It
is deliberately not the first one.

## Safety and scientific integrity

- Ordinary tests are provider-free and network-free.
- Live studies use fixed runtime identities, bounded budgets, explicit stop
  conditions, and no authority to modify trusted physics.
- Raw occurrences are retained even when a curator excludes them.
- Null results are valid outcomes. Acceptance tests prove experimental
  integrity, not a desired institutional effect.
- No result is called emergent without isolated and unstructured baselines.
- No lineage graph is called causal without an intervention design.
- No application parser, prompt, or model output becomes generic authority.

## Immediate stop condition

Do not broaden the governance hierarchy, add actor reproduction, or introduce
RL until CL-001 can be executed, forked, measured, and independently audited.
The next durable type must be justified by that experiment.
