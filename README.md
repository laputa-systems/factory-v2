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
and the resident daemon has the typed transition/result and TaskAttempt-native
child/prompt/disposal bridge. The ledger also admits an opaque sealed run plan
and its finite matched-pair set for restart-safe coordination. A full daemon-owned live
scheduler and paired live episode runner remain; the next work is to close
those named experimental boundaries, not to fill in a general-purpose swarm
framework.

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
Each run retains a `qualification-artifact.json` in its printed run directory.
That host-bound artifact carries a BLAKE3 digest and contains the exact actor
usage/cost rows, Forum post bodies and read intervals, model/catalog identity,
and explicit `absent` PostgreSQL/daemon and `not_executed` CL-001 lifecycle
markers. It makes the smoke auditable without presenting its in-memory shared
Forum as institutional evidence.
The credential-free provider catalog, retaining DeepSeek alongside free Ling,
Laguna, and paid Ling 2.6, is saved at
`packages/society-pi-host/catalogs/openrouter-admitted-models-v1.json`.

The provider-free CL-001 world summary is runnable with `make run-society`.
It reports accepted actor and Forum activity, study budget units, measurements,
baselines, and replay integrity; monetary cost is explicitly not applicable to
that deterministic fixture. Live Pi Office usage already records exact
provider-cost evidence in the trusted ledger, but a live end-of-cycle report
remains a separately scoped runtime/reporting surface.

## PostgreSQL operations

PostgreSQL 18 is the local baseline. Start it with
`brew services start postgresql@18`, create an administrator-approved database,
and provide the daemon role URL:

```sh
export SOCIETY_DATABASE_URL='postgresql://society_runtime@localhost/society'
```

Set `SOCIETY_DATABASE_SCHEMA` when the ledger lives in a private PostgreSQL
schema; otherwise the daemon uses the database's default schema.

The schema is a single authoritative fresh bootstrap from
`schema/postgres/kernel.sql`. On a new, empty database, apply that bootstrap
before starting the daemon (it is intentionally not an online migration):

```sh
if [ -n "${SOCIETY_DATABASE_SCHEMA:-}" ]; then
  psql "$SOCIETY_DATABASE_URL" -v ON_ERROR_STOP=1 -c \
    "CREATE SCHEMA IF NOT EXISTS \"$SOCIETY_DATABASE_SCHEMA\""
  PGOPTIONS="-c search_path=$SOCIETY_DATABASE_SCHEMA" \
    psql "$SOCIETY_DATABASE_URL" -v ON_ERROR_STOP=1 \
      -f schema/postgres/kernel.sql
else
  psql "$SOCIETY_DATABASE_URL" -v ON_ERROR_STOP=1 \
    -f schema/postgres/kernel.sql
fi
```

The command is expected to run against an administrator-approved blank
database; it does not import or transform the former SQLite database. Once
bootstrapped, `societyd` connects to that schema,
validates ledger replay before binding its socket, and holds a dedicated
PostgreSQL advisory lock for its lifetime.
The bootstrap also writes the exact schema identity as a PostgreSQL schema
comment. `make postgres-test-ready` checks that identity before reusing either
its public database or its template, so it rebuilds rather than silently
testing against a stale schema with coincidentally matching table names.
It intentionally does not delete active private test fixtures. When no
Society test process is running, `make postgres-test-clean` removes stale
`society_test_*` fixtures without force-disconnecting a concurrent process.
The filesystem `societyd.lock` remains responsible only for the runtime root.

For health and ownership diagnostics, query the selected database and inspect
the daemon status socket:

```sh
psql "$SOCIETY_DATABASE_URL" -c \
  "SELECT current_database(), current_schema(), pg_is_in_recovery();"
psql "$SOCIETY_DATABASE_URL" -c \
  "SELECT pid, classid, objid, granted FROM pg_locks WHERE locktype = 'advisory';"
societyctl --socket <runtime-root>/societyd.sock status
```

Back up and restore the authoritative ledger with PostgreSQL tools. Restore
into a blank database, point both URLs at the restored database, and start the
daemon; bind-time replay validation must pass before any new command is
accepted:

```sh
pg_dump --format=custom --file=society.dump "$SOCIETY_DATABASE_URL"
createdb society_restore
pg_restore --dbname=society_restore society.dump
```

If validation fails, the daemon reports the replay error and does not expose a
serving socket. Do not copy runtime directories as a database backup.

## Documents

- [`RESEARCH-PROGRAM.md`](RESEARCH-PROGRAM.md) — thesis and research sequence
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — generic boundaries and invariants
- [`GLOSSARY.md`](GLOSSARY.md) — canonical terms and implementation status
- [`FORUM.md`](FORUM.md) — staged communication substrate and deferrals
- [`VERTICAL-SLICE.md`](VERTICAL-SLICE.md) — generic requirements for CL-001
- [`AGENTS.md`](AGENTS.md) — engineering contract

Historical application work is intentionally absent from the current research
line. This branch starts from the smaller institutional experiment.
