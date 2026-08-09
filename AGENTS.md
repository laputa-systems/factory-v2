# Generic society engineering guide

## Working contract

Implementation is cheap; ambiguity is not. Spend care where meaning becomes
durable: names, types, schemas, interfaces, state transitions, permissions,
tests, and explanations. Let implementation be a candidate. Let the contract
and the evidence around it be what survives.

- Prefer closed Rust enums, identifier newtypes, normalized SQLite tables, and
  narrow protocols which make invalid state difficult to express.
- JSON is permitted only at the Pi SDK-host boundary. The Rust control protocol
  and SQLite schema must not gain generic payloads, metadata maps, EAV tables,
  or stringly typed discriminants.
- Application purpose crosses the generic boundary only through the typed
  mission and north-star records defined in `VERTICAL-SLICE.md`; that target is
  not implemented yet. A sealed rendering is not a substitute for the typed
  boundary.
- Test observable transitions and cross-boundary invariants. Favor
  integration, replay, migration, fault-injection, and process tests over
  trivial units.
- Add no dependency casually. Keep features narrow, pin exact dependencies,
  and document why a dependency belongs in trusted physics.
- Do not run pre-commit hooks. Never push a remote. Product materialization
  must explicitly suppress repository hooks as its typed contract requires.

## Authority documents

- `GLOSSARY.md` owns generic vocabulary.
- `ARCHITECTURE.md` owns generic behavior and trust boundaries.
- `VERTICAL-SLICE.md` owns the generic execution sequence and typed application
  port.
- `DEPENDENCIES.md` owns generic dependency allowance.
- `RSI.md` is originating research, not an executable schema.
- `applications/<product>/` owns application vocabulary, architecture,
  executable slices, evaluators, fixtures, and product evidence.

An application depends on generic public crates and protocols only. Generic
crates, root manifests, and root documents must not depend on, import, or
describe a named application. When implementation evidence contradicts a plan,
update code, tests, and the owning document in one cohesive change.

## Code map

```text
AGENTS.md                  generic engineering contract and code map
ARCHITECTURE.md            generic society architecture
GLOSSARY.md                generic canonical vocabulary
VERTICAL-SLICE.md          generic execution sequence and application port
DEPENDENCIES.md            exact generic dependency contract
RSI.md                     research source conversation

Cargo.toml                 generic Rust workspace manifest
crates/society-kernel/     trusted domain, ledger, and SQLite authority
crates/society-content/    physical byte-seal store without evidence meaning
crates/society-pi/         typed Rust peer for the SDK-host boundary
crates/societyd/           resident authority, process custody, and monitor
crates/societyctl/         public query and supervisor-stream client
crates/society-product/    root-workspace guarded local materialization
                           mechanics
migrations/                one atomic canonical fresh-schema bootstrap
packages/society-pi-host/  provider-free Pi SDK execution adapter
tests/daemon/              resident-protocol integration fixtures
tests/supervision/         native Pi-host process/race fixtures
applications/<product>/    isolated application workspace and contracts
var/                       ignored runtime database, objects, and workspaces
```

The root workspace intentionally excludes application workspaces. An
application may use generic public crates by path or published interface; it
does not become a generic workspace member merely by being stored in this
repository.

## Nearest hard judges

Run the narrowest relevant judge first, then broaden:

```text
cargo test -p society-kernel --test <contract>
cargo test -p society-pi --lib
npm test --prefix packages/society-pi-host
cargo test -p society-content
cargo test -p society-product
cargo test -p societyd --test supervision -- --test-threads=1

pi_host_entry="$PWD/packages/society-pi-host/dist/src/main.js"
pi_host_digest="$(shasum -a 256 "$pi_host_entry" | awk '{print $1}')"
SOCIETY_PI_HOST_ENTRYPOINT="$pi_host_entry" \
SOCIETY_PI_HOST_BUILD_SHA256="$pi_host_digest" \
cargo test --workspace

cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Provider calls are never part of an ordinary test. A paid qualification or a
live application execution requires its separately typed admission, profile,
and budget gates.
