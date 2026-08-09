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

The repository is implementing the executable contract in bounded tranches.
`implemented` below means only the stated boundary has passed its own judge; it
does not imply that the enclosing milestone is complete. Update this map
whenever a tranche creates, moves, or retires an implementation boundary.

```text
AGENTS.md                  engineering contract and living code map
ARCHITECTURE.md            general society architecture
GLOSSARY.md                canonical domain vocabulary
VERTICAL-SLICE.md          VS-001 executable specification and progress gates
RSI.md                     research source conversation
DEPENDENCIES.md            exact trusted dependency and vendoring contract
rust-toolchain.toml        exact Rust toolchain for trusted physics

Cargo.toml                 Rust workspace manifest
crates/society-kernel/     implemented bootstrap, coordination, and bounded
                           actor/work/attempt execution foundations;
                           remaining Milestone-1 domains grow here
migrations/                normalized monotonic SQLite migrations; migrations
                           1 through 3 own the current kernel foundation schema

packages/society-pi-host/  implemented and provider-free-audited Pi SDK host;
                           durable Rust authority remains outside this boundary
circuits/vs-001-spawn-stderr/
                           implemented deterministic fixture/evaluator tranche;
                           process ownership and authority remain outside it

crates/society-pi/         implemented provider-free-audited typed Rust peer;
                           durable sealing, charging, and process ownership remain
                           in the planned resident authority
crates/society-content/    implemented isolated physical byte-seal store; it
                           confers no evidence or provenance meaning
crates/society-circuit/    implemented isolated closed B01-B11 observation
                           parser; durable evaluator/evidence admission remains
crates/societyd/           implemented bounded resident SQLite authority,
                           monitor, and native Pi child/process-group physics;
                           durable Pi/content/recovery integration remains
crates/societyctl/         implemented public query and supervisor-stream client
tests/daemon/              implemented resident-protocol integration fixtures
tests/supervision/         provider-free native Pi-host process/race fixture
tests/                     remaining cross-crate and end-to-end fixtures grow here
var/                       ignored runtime database, objects, sessions, workspaces
```

Planned paths may not exist yet. Remove that annotation only when the boundary
it names has landed; never keep both a legacy and replacement path unmarked.

## Nearest hard judges

Run the narrowest relevant judge first, then broaden:

```text
cargo test -p society-kernel --test <contract>
cargo test -p society-pi --lib
npm test --prefix packages/society-pi-host
cargo test --manifest-path crates/society-content/Cargo.toml
cargo test --manifest-path crates/society-circuit/Cargo.toml
cargo test -p societyd --test supervision -- --test-threads=1

pi_host_entry="$PWD/packages/society-pi-host/dist/src/main.js"
pi_host_digest="$(shasum -a 256 "$pi_host_entry" | awk '{print $1}')"
SOCIETY_PI_HOST_ENTRYPOINT="$pi_host_entry" \
SOCIETY_PI_HOST_BUILD_SHA256="$pi_host_digest" \
cargo test --workspace

SOCIETY_PI_HOST_ENTRYPOINT="$pi_host_entry" \
SOCIETY_PI_HOST_PACKAGE_ROOT="$PWD/packages/society-pi-host" \
cargo test -p societyd --test supervision \
  explicit_pinned_host_create_dispose_never_prompts_a_provider \
  -- --ignored --exact --test-threads=1

cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Provider calls are never part of an ordinary test. The paid qualification and
live VS-001 run require their named typed admission and budget gates.
