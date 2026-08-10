.PHONY: run-society

run-society:
	cargo run --quiet --manifest-path applications/correction-latency/Cargo.toml -p correction-latency-harness
