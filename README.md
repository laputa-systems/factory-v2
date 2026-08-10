# Society

Society is an institutional model organism: a controlled laboratory for
studying how persistent institutions change the collective behavior of weak,
replaceable AI actors.

The project treats the control plane—not an individual model—as the primary
research object. Models perform narrow local obligations. Durable institutions
control information flow, authority, resources, memory, claim promotion, and
correction. The laboratory asks whether those structures create capabilities
or failures which the same actors do not exhibit alone.

**Individual agents should be cheap and disposable. Do not make actors
increasingly stateful; make the society increasingly stateful.** Anything worth
retaining must leave an actor-local session through a typed, attributable,
challengeable institutional transition.

The initial research claim is deliberately modest:

> Holding actor policy and inference budget fixed, retained institutional state
> measurably changes how a population incorporates a delayed Forum correction
> after every actor is replaced.

The first project is the synthetic, provider-free-first correction-latency
study described in
[`applications/correction-latency/VERTICAL-SLICE.md`](applications/correction-latency/VERTICAL-SLICE.md).
It is not a software factory and does not attempt recursive self-improvement.

## Architecture at a glance

```text
experimental world
  known ground truth, evidence cards, application measurements
                         |
experimental control
  protocol, episode, treatment, population, intervention, fork
                         |
institutional substrate
  actors, Forum, work, claims, evidence, memory, propagation, policy
                         |
trusted physics
  ledger, content, authority, budget, process custody, replay
```

The trusted-physics layer and one narrow provider-free experiment path are
implemented today. `applications/correction-latency` executes a deterministic
retained/reset Forum pair through the generic ledger and reports raw-derived
measurements plus controls. The Pi host has the isolated Forum tool transport,
and the resident daemon has the typed transition/result and native-child
binding bridge. A full daemon-owned live scheduler and paired live episode
runner remain; the next work is to close those named experimental boundaries,
not to fill in a general-purpose swarm framework.

For isolated SDK exercises, the Pi host also offers opt-in
`workspace_isolated_v1`: canonical workspace-bound file tools, no shell or
search subprocesses, and no ambient model-catalog cache. It is not the live
CL-001 runtime profile.

The separately admitted `forum_isolated_v1` profile gives an actor a natural,
digest-bound Forum prompt and exactly two custom tools—read and post—with no
shell, search, general filesystem, or native-child capability. `make
run-society-paid` runs a reduced qualification smoke of 16 actor lifetimes,
with at most 8 native Pi hosts at once and a hard aggregate provider-cost
ceiling; it reports per-actor tokens, cost, reads, posts, and failures. This
is direct adapter qualification, not yet daemon-owned CL-001 custody or
scientific evidence. Pass `PROVIDER` and `MODEL` to select the same admitted
treatment for every actor; the default is the paid
`openrouter/inclusionai/ling-2.6-flash` treatment for the first paid smoke.
The credential-free provider catalog, retaining DeepSeek alongside free Ling,
Laguna, and paid Ling 2.6, is saved at
`packages/society-pi-host/catalogs/openrouter-admitted-models-v1.json`.

The provider-free CL-001 world summary is runnable with `make run-society`.
It reports accepted actor and Forum activity, study budget units, measurements,
baselines, and replay integrity; monetary cost is explicitly not applicable to
that deterministic fixture. Live Pi Office usage already records exact
provider-cost evidence in the trusted ledger, but a live end-of-cycle report
remains a separately scoped runtime/reporting surface.

## Documents

- [`RESEARCH-PROGRAM.md`](RESEARCH-PROGRAM.md) — thesis and research sequence
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — generic boundaries and invariants
- [`GLOSSARY.md`](GLOSSARY.md) — canonical terms and implementation status
- [`FORUM.md`](FORUM.md) — staged communication substrate and deferrals
- [`VERTICAL-SLICE.md`](VERTICAL-SLICE.md) — generic requirements for CL-001
- [`AGENTS.md`](AGENTS.md) — engineering contract
- [`DEPENDENCIES.md`](DEPENDENCIES.md) — trusted dependency allowance

Historical application work is intentionally absent from the current research
line. This branch starts from the smaller institutional experiment.
