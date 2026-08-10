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
  mission and north-star records defined in `VERTICAL-SLICE.md`. An application
  retains its canonical bounded `MissionSourceRendering` bytes and declares
  their BLAKE3 digest in `ApplicationMissionInput`; it neither receives nor
  supplies a `ContentObjectId`. The resident's private founding-source path
  checks those bytes against that digest, side-effect-free preflights the outer
  command, physically seals them, records the existing receipt/object chain,
  and only then permits `InstallFoundingMission` to bind the registered object.
  The supervisor carries `MissionSourceRendering` only with that mission
  command; it has neither a generic content-mutation command nor content-writer
  authority. This is byte custody, not provenance, semantic admission, or an
  end-to-end application execution claim. A restart remains `RecoveryFenced`
  rather than resuming a partial source operation. Kernel-issued product
  authorization remains unimplemented.
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
crates/societyd/           resident authority, generic NativeChild custody,
                           optional Pi sidecar, and monitor
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

The implemented generic Office-session Dispose foundation records the exact
`Authorize-before-write -> delivered -> accepted -> final Known/failure ->
Disposed` boundary. The final branch is exclusive: final Known usage followed
by the peer's transcript-flush receipt can record `Disposed` and reconcile the
parent reservation; a final accounting failure freezes that reservation and
begins containment without a synthetic `Disposed` receipt. This foundation is
not a resident scheduler/control-loop call site, post-restart recovery,
workspace disposal, semantic submission, paid/native qualification, or an
end-to-end application execution claim. Direct-child reap remains a separate
process-custody fact.

The current generic `NativeChild` foundation owns one PID/PGID, group
liveness, signal delivery, direct wait, bounded stdout/stderr capture, and
post-reap receipt shape. Pi composes that custody nucleus through a strict
session/protocol sidecar rather than defining the base process identity. A
daemon-private deterministic-evaluator pre-bridge admits only a verified direct
executable, its verified input-manifest path, a fixed argv grammar, and an
empty environment; it is unscheduled and has no supervisor/public execution
command. It does not run an application evaluator, seal semantic evidence, or
claim an end-to-end application result.

## Nearest hard judges

Run the narrowest relevant judge first, then broaden:

```text
cargo test -p society-kernel --test <contract>
cargo test -p society-pi --lib
npm test --prefix packages/society-pi-host
cargo test -p society-content
cargo test -p society-product
cargo test -p societyd --lib native_child
cargo test -p societyd --test supervision -- --test-threads=1
tests/generic-boundary/run-no-application-knowledge.sh

pi_host_entry="$PWD/packages/society-pi-host/dist/src/main.js"
pi_host_digest="$(cd packages/society-pi-host && node --input-type=module -e \
  'import { readFileSync } from "node:fs"; import { blake3Hex } from "./dist/src/digest.js"; console.log(blake3Hex(readFileSync(process.argv[1])))' \
  "$pi_host_entry")"
SOCIETY_PI_HOST_ENTRYPOINT="$pi_host_entry" \
SOCIETY_PI_HOST_BUILD_BLAKE3="$pi_host_digest" \
cargo test --workspace

cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Provider calls are never part of an ordinary test. A paid qualification or a
live application execution requires its separately typed admission, profile,
and budget gates.
