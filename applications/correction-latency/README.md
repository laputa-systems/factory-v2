# Correction-latency laboratory

This is Society's first experimental world. It studies whether access to
pre-replacement chronological Forum history changes how a fresh population
incorporates identical corrective evidence.

The world is deliberately synthetic. Each episode has hidden ground truth and
a fixed collection of bounded evidence cards. No actor receives enough private
evidence to reconstruct the whole answer alone. Actors may communicate only
through Society's recorded boundaries.

At the intervention point every actor is terminated and replaced with a fresh
instance drawn from the same policy distribution. One matched arm can read the
old Forum Thread. The other begins after its old head. Only then does a
deterministic service publish the same correction to both arms. Prompts, tools,
evidence, timing, and resource ceilings remain matched.

The desired outcome is a valid comparison, not evidence that retained memory
helps. Faster correction, slower correction, persistent error, and no
measurable difference are all legitimate results.

See [`VERTICAL-SLICE.md`](VERTICAL-SLICE.md) for the protocol.

## Provider-free acceptance run

The isolated deterministic paired run is available without a provider call:

```text
cargo run --manifest-path applications/correction-latency/Cargo.toml -p correction-latency-harness
```

From the repository root, the same provider-free run and its end-of-world
status report are available as:

```text
make run-society
```

The report includes per-arm actor, Forum, study-budget, measurement, baseline,
and replay-integrity facts. Monetary cost is explicitly reported as not
applicable: this fixture makes no provider calls and has no provider-backed
agent cost to inspect. Live Pi Office cost is a separate ledger-backed path.

It admits the canonical eight-role source and successor populations in both
arms, freezes the source head, proves reset-history denial and source-authority
loss, releases the same correction through one matched service transition,
reveals the protocol-committed truth only after all actors terminate, records
typed measurements, and validates fresh materialized-state replay. It is not a
live Pi/native-child study; those custody facts remain prerequisites for a
separately admitted live profile.
