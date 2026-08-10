.PHONY: run-society run-society-paid

PROVIDER ?= openrouter
MODEL ?= inclusionai/ling-2.6-flash

run-society:
	cargo run --quiet --manifest-path applications/correction-latency/Cargo.toml -p correction-latency-harness

# Explicitly paid, noncanonical qualification smoke: 16 actors total and
# eight native Pi hosts at once, versus the 32/16 canonical CL-001 topology.
run-society-paid:
	npm run build --prefix packages/society-pi-host
	SOCIETY_PI_PROVIDER="$(PROVIDER)" SOCIETY_PI_MODEL="$(MODEL)" node packages/society-pi-host/dist/src/paid-run.js
