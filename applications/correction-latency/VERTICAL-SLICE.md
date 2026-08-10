# CL-001: correction latency under actor replacement

## Research question

Holding actor policy, evidence, correction timing, role topology, and total
resource budget fixed, does retaining admitted institutional memory after total
actor replacement change correction adoption latency or final correctness?

This is a two-sided question. The protocol does not presume that retention is
beneficial.

## World

One world instance contains:

- a hidden binary proposition selected before treatment assignment;
- eight signed evidence cards whose joint relation determines the proposition;
- an early, plausible false claim with a declared high-authority source;
- corrective evidence partitioned across multiple cards;
- a fixed message and work topology; and
- a deterministic ground-truth evaluator unavailable to actors.

No single actor view contains enough cards to establish the proposition. The
world contract validates that condition mechanically before episode admission.

Evidence cards are immutable, content-addressed, and episode-scoped. A card's
signature is synthetic source identity, not epistemic authority granted by the
generic kernel.

## Population

The first population contains eight disposable actor instances drawn from one
fixed weak actor-policy revision:

- four observers, each receiving one private card;
- two challengers receiving disjoint subsets of submitted claims;
- one curator able to propose institutional-memory promotion; and
- one decision actor receiving only promoted memory and visible challenges.

Roles describe local obligations and views, not personalities or durable agent
identities. Each actor has one bounded turn budget and terminates after its
obligation.

Provider-free tests use deterministic actor doubles. A later live study pins
one exact weak model/runtime profile, sampling contract, prompt revision, tool
surface, and aggregate budget. Provider-free and live results are never pooled.

## Timeline

```text
t0  admit matched episode pair and treatment assignments
t1  source actors inspect private cards and emit local claims
t2  institution promotes or challenges available claims
t3  early decision records current institutional belief
t4  release correction cards according to the fixed schedule
t5  close every source actor and prove process/session reconciliation
t6  atomically retain or reset institutional memory; admit fresh population
t7  successor actors receive bounded views and perform local obligations
t8  final decision
t9  derive measurements, close budgets and actors, integrity replay each arm
```

The correction is released before source actors close but may reach successor
work only through recorded propagation and institutional state. This makes
correction routing, not hidden actor continuity, the object of study.

## Treatment arms

The matched pair differs in exactly one field:

### Retained

Successor actors may receive pre-replacement records which reached the admitted
institutional-memory level, including their provenance, challenges, and
supersession status. Private actor observations and scratch state do not
survive.

### Reset

Successor actors receive an empty institutional-memory snapshot. The source
episode history remains preserved for audit but is not visible to successor
actors or selectable by their work.

Both arms reuse the same world revision and card identities only as immutable
source content. They have distinct episode, actor, authority, message, budget,
and result identities.

## Baselines

Before interpreting institutional effects, run:

1. isolated actors with the same per-actor view and no communication;
2. an unstructured population with direct messages but no promotion or durable
   memory; and
3. a fresh-population/fresh-memory episode under the same aggregate budget.

The report must state actor-level and baseline uncertainty. It may call a
behavior emergent only if the pre-registered criterion excludes those
baselines.

## Raw occurrences

Retain at minimum:

- exact ground-truth commitment and later authorized reveal;
- every actor-policy, role, view, budget, and replacement identity;
- evidence-card delivery and visibility;
- message bytes, sender, recipients, and delivery result;
- claim, challenge, promotion, supersession, and consultation;
- correction-release and propagation events;
- actor completion, expiry, and process reconciliation;
- final decisions and uncertainty; and
- measurement inputs, exclusions, and failures.

The curator cannot delete or rewrite raw occurrences. A measurement can exclude
an occurrence only through a named rule retained in the result.

## Measurements

### Primary

- `CorrectionAdoptionLatency`: admitted steps from correction release to the
  first final-decision-relevant corrected institutional belief.
- `FinalDecisionCorrect`: whether the final decision matches the hidden truth.

### Secondary

- `FalseClaimPersistence`: whether the early false claim remains promoted at
  closure and for how many steps.
- `CorrectionVisibility`: fraction of successor roles shown an admitted
  correction before acting.
- `DissentSurvival`: whether a valid challenge remains available after
  replacement.
- `InstitutionalMemoryUtilization`: consulted retained items divided by visible
  retained items.
- `ResourceUse`: actor turns and admitted runtime cost by arm.

Every measurement has `Observed`, `Unavailable`, and `Invalidated` outcomes.
There is no default zero.

## Analysis

The first engineering acceptance run proves exact paired execution over fixed
fixtures; it makes no statistical claim. The first scientific run pre-registers
the number of independent world seeds and paired comparisons before any result
is inspected.

Report the paired retained-minus-reset difference, raw arm values, missingness,
and confidence interval. Report both operational and amortized institutional
cost when retained memory was created before the measured episode.

## Falsification and invalidation

The institutional-effect hypothesis is not supported if retained and reset
arms are indistinguishable within the pre-registered precision. The run is
invalid rather than negative if:

- treatment is visible before assignment or actor views diverge unexpectedly;
- any source actor or private session state survives replacement;
- the reset arm can access pre-replacement memory;
- actors observe ground truth outside their admitted cards;
- correction timing or budgets differ between arms;
- raw messages or visibility decisions are missing;
- measurement code changed after outcomes were visible; or
- actor baselines already reliably exhibit the target behavior.

## Explicit omissions

CL-001 has no software repository, product patch, endogenous institution
mutation, actor reproduction, RL update, open network, hidden collaboration
channel, or autonomous scientific conclusion. Authority amplification and
alternative propagation topologies are later protocols, not extra fields in
this one.

## Implementation order

1. Canonical provider-free world fixtures and ground-truth evaluator.
2. Generic protocol, episode, treatment, population, and institution identities.
3. Recorded message visibility and institutional-memory promotion.
4. Replacement and retained/reset intervention transaction.
5. Raw measurement derivation and independent integrity replay.
6. Deterministic paired episode harness and negative controls.
7. Only then, a separately authorized weak-model pilot.
