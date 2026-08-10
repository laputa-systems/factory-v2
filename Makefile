.PHONY: test postgres-test-ready run-society run-society-paid

PROVIDER ?= openrouter
MODEL ?= inclusionai/ling-2.6-flash
POSTGRES_EXPECTED_VERSION_NUM ?= 180004
SOCIETY_POSTGRES_TEST_URL ?= postgresql://$(shell id -un)@localhost/postgres

postgres-test-ready:
	@set -eu; \
	case "$$(uname -s)" in \
	Darwin) \
		command -v brew >/dev/null 2>&1 || { echo 'make test requires Homebrew on macOS' >&2; exit 1; }; \
		brew services start postgresql@18; \
		;; \
	esac; \
	command -v pg_isready >/dev/null 2>&1 || { echo 'make test requires pg_isready' >&2; exit 1; }; \
	ready=0; \
	attempt=0; \
	while [ "$$attempt" -lt 30 ]; do \
		if pg_isready -q -d '$(SOCIETY_POSTGRES_TEST_URL)'; then ready=1; break; fi; \
		attempt=$$((attempt + 1)); \
		sleep 1; \
	done; \
	if [ "$$ready" -ne 1 ]; then \
		echo 'PostgreSQL did not become ready for the test URL' >&2; \
		exit 1; \
	fi; \
	command -v psql >/dev/null 2>&1 || { echo 'make test requires psql' >&2; exit 1; }; \
	version_num="$$(psql '$(SOCIETY_POSTGRES_TEST_URL)' -Atqc 'SELECT current_setting('"'"'server_version_num'"'"')')"; \
	if [ "$$version_num" != '$(POSTGRES_EXPECTED_VERSION_NUM)' ]; then \
		echo "expected PostgreSQL 18.4 (server_version_num $(POSTGRES_EXPECTED_VERSION_NUM)); got $$(psql '$(SOCIETY_POSTGRES_TEST_URL)' -Atqc 'SHOW server_version')" >&2; \
		exit 1; \
	fi; \
	echo "PostgreSQL version check passed: $$(psql '$(SOCIETY_POSTGRES_TEST_URL)' -Atqc 'SHOW server_version')"

test: postgres-test-ready
	@set -eu; \
	npm run build --prefix packages/society-pi-host; \
	host_entrypoint="$$(cd packages/society-pi-host && pwd)/dist/src/main.js"; \
	host_build_blake3="$$(cd packages/society-pi-host && node --input-type=module -e 'import { readFile } from "node:fs/promises"; import { blake3 } from "@noble/hashes/blake3.js"; console.log(Buffer.from(blake3(await readFile(process.argv[1]))).toString("hex"));' dist/src/main.js)"; \
	export SOCIETY_POSTGRES_TEST_URL='$(SOCIETY_POSTGRES_TEST_URL)'; \
	export SOCIETY_PI_HOST_ENTRYPOINT="$$host_entrypoint"; \
	export SOCIETY_PI_HOST_BUILD_BLAKE3="$$host_build_blake3"; \
	cargo test --workspace --all-features; \
	cargo test --manifest-path applications/correction-latency/Cargo.toml --workspace --all-features; \
	npm test --prefix packages/society-pi-host; \
	tests/generic-boundary/run-no-application-knowledge.sh

run-society:
	cargo run --quiet --manifest-path applications/correction-latency/Cargo.toml -p correction-latency-harness

# Explicitly paid, noncanonical qualification smoke: 16 actors total and
# eight native Pi hosts at once, versus the 32/16 canonical CL-001 topology.
run-society-paid:
	npm run build --prefix packages/society-pi-host
	SOCIETY_PI_PROVIDER="$(PROVIDER)" SOCIETY_PI_MODEL="$(MODEL)" node packages/society-pi-host/dist/src/paid-run.js
