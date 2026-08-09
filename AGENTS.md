# XSH Society V2 engineering guide

## Working contract

Implementation is cheap; ambiguity is not. Spend care where meaning becomes
durable: names, types, schemas, interfaces, state transitions, permissions,
tests, and explanations. Let implementation be a candidate; let the contract
and evidence around it be what survives.

- Use the domain spellings in `GLOSSARY.md`. New synonyms do not create new
  concepts.
- Prefer closed Rust enums, identifier newtypes, normalized SQLite tables, and
  narrow protocols which make invalid state difficult to express.
- JSON is permitted only at the Pi SDK-host boundary described in
  `VERTICAL-SLICE.md`. The Rust control protocol and SQLite schema must not gain
  generic payloads, metadata maps, EAV tables, or stringly typed discriminants.
- Test observable transitions and cross-boundary invariants. Favor integration,
  replay, migration, fault-injection, and process tests over trivial units.
- Add no dependency casually. Keep features narrow, pin exact JavaScript
  dependencies, and document why a new dependency belongs in trusted physics.
- Do not run pre-commit hooks. Never push a remote. Product materialization
  must explicitly suppress repository hooks as specified by the vertical slice.

## Authoritative documents

- `GLOSSARY.md` owns canonical terms.
- `ARCHITECTURE.md` owns general V2 behavior and trust boundaries.
- `VERTICAL-SLICE.md` owns the VS-001 executable contract, gates, budgets, and
  acceptance tests.
- `RSI.md` is the originating research conversation, not an executable schema.

When implementation evidence contradicts a plan, update code, tests, and the
owning document in the same cohesive commit. Do not leave a stale path looking
canonical.

## Code map

The repository is currently at the executable-contract stage. Update this map
whenever a tranche creates, moves, or retires an implementation boundary.

```text
AGENTS.md                  engineering contract and living code map
ARCHITECTURE.md            general society architecture
GLOSSARY.md                canonical domain vocabulary
VERTICAL-SLICE.md          VS-001 executable specification and progress gates
RSI.md                     research source conversation
rust-toolchain.toml        exact Rust toolchain for trusted physics

Cargo.toml                 Rust workspace manifest (Milestone 1)
crates/society-kernel/     domain types, transitions, SQLite, ledger, content
crates/society-pi/         typed Rust peer for the Pi SDK-host boundary
crates/societyd/           resident authority and process supervisor
crates/societyctl/         typed local control/query client
migrations/                normalized monotonic SQLite migrations

packages/society-pi-host/  pinned TypeScript Pi SDK execution adapter
circuits/vs-001-spawn-stderr/
                           VS-001 prompts, fixtures, judges, and projections
tests/                     cross-crate and end-to-end integration fixtures
var/                       ignored runtime database, objects, sessions, workspaces
```

Paths marked with a milestone may not exist yet. Remove that annotation once
the boundary is implemented; never keep both a legacy and replacement path
unmarked.

## Nearest hard judges

Run the narrowest relevant judge first, then broaden:

```text
cargo test -p society-kernel --test <contract>
cargo test --workspace
npm test --prefix packages/society-pi-host
git diff --check
```

Provider calls are never part of an ordinary test. The paid qualification and
live VS-001 run require their named typed admission and budget gates.
