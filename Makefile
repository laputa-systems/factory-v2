.PHONY: test postgres-test-ready run-society run-society-paid

PROVIDER ?= openrouter
MODEL ?= inclusionai/ling-2.6-flash
POSTGRES_EXPECTED_VERSION_NUM ?= 180004
CARGO_TEST_THREADS ?= 8
SOCIETY_POSTGRES_TEST_URL ?= postgresql://$(shell id -un)@localhost/postgres
SOCIETY_POSTGRES_TEST_TEMPLATE_DB ?= society_test_template
SOCIETY_POSTGRES_TEST_TEMPLATE_URL ?= postgresql://$(shell id -un)@localhost/template1
SOCIETY_POSTGRES_TEST_TEMPLATE_DATABASE_URL ?= postgresql://$(shell id -un)@localhost/$(SOCIETY_POSTGRES_TEST_TEMPLATE_DB)

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
	public_ready="$$(psql '$(SOCIETY_POSTGRES_TEST_URL)' -Atqc "SELECT to_regclass('public.commands') IS NOT NULL AND to_regclass('public.events') IS NOT NULL AND to_regclass('public.study_forum_read_renderings') IS NULL AND to_regclass('public.study_forum_read_receipt_renderings') IS NULL AND (SELECT COUNT(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'public' AND c.relkind = 'r') = 321")"; \
	if [ "$$public_ready" != 't' ]; then \
		PGOPTIONS='-c client_min_messages=warning' psql '$(SOCIETY_POSTGRES_TEST_URL)' -v ON_ERROR_STOP=1 -q -c 'DROP SCHEMA public CASCADE; CREATE SCHEMA public'; \
		PGOPTIONS='-c client_min_messages=warning' psql '$(SOCIETY_POSTGRES_TEST_URL)' -v ON_ERROR_STOP=1 -q -f schema/postgres/kernel.sql; \
	fi; \
	psql '$(SOCIETY_POSTGRES_TEST_URL)' -Atqc "SELECT datname FROM pg_database WHERE datname LIKE 'society_test_db_%'" | while IFS= read -r database; do \
		psql '$(SOCIETY_POSTGRES_TEST_TEMPLATE_URL)' -v ON_ERROR_STOP=1 -q -c "DROP DATABASE IF EXISTS \"$$database\" WITH (FORCE)" >/dev/null; \
	done; \
	psql '$(SOCIETY_POSTGRES_TEST_URL)' -Atqc "SELECT nspname FROM pg_namespace WHERE nspname LIKE 'society_test_%'" | while IFS= read -r schema; do \
		psql '$(SOCIETY_POSTGRES_TEST_URL)' -v ON_ERROR_STOP=1 -q -c "DROP SCHEMA \"$$schema\" CASCADE" >/dev/null 2>&1; \
	done; \
	template_ready="$$(psql '$(SOCIETY_POSTGRES_TEST_TEMPLATE_DATABASE_URL)' -Atqc "SELECT to_regclass('public.commands') IS NOT NULL AND to_regclass('public.events') IS NOT NULL AND to_regclass('public.study_forum_read_renderings') IS NULL AND to_regclass('public.study_forum_read_receipt_renderings') IS NULL AND (SELECT COUNT(*) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'public' AND c.relkind = 'r') = 321" 2>/dev/null || true)"; \
	if [ "$$template_ready" != 't' ]; then \
		psql '$(SOCIETY_POSTGRES_TEST_URL)' -v ON_ERROR_STOP=1 -q -c 'DROP DATABASE IF EXISTS "$(SOCIETY_POSTGRES_TEST_TEMPLATE_DB)" WITH (FORCE)'; \
		psql '$(SOCIETY_POSTGRES_TEST_URL)' -v ON_ERROR_STOP=1 -q -c 'CREATE DATABASE "$(SOCIETY_POSTGRES_TEST_TEMPLATE_DB)" TEMPLATE template1'; \
		PGOPTIONS='-c client_min_messages=warning' psql '$(SOCIETY_POSTGRES_TEST_TEMPLATE_DATABASE_URL)' -v ON_ERROR_STOP=1 -q -f schema/postgres/kernel.sql; \
	else \
		echo 'Reusing validated PostgreSQL test template'; \
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
	cargo test --workspace --all-features -- --test-threads=$(CARGO_TEST_THREADS); \
	cargo test --manifest-path applications/correction-latency/Cargo.toml --workspace --all-features -- --test-threads=$(CARGO_TEST_THREADS); \
	npm test --prefix packages/society-pi-host; \
	tests/generic-boundary/run-no-application-knowledge.sh

run-society: postgres-test-ready
	SOCIETY_POSTGRES_TEST_URL='$(SOCIETY_POSTGRES_TEST_URL)' cargo run --quiet --manifest-path applications/correction-latency/Cargo.toml -p correction-latency-harness

# Explicitly paid, noncanonical qualification smoke: 16 actors total and
# eight native Pi hosts at once, versus the 32/16 canonical CL-001 topology.
run-society-paid:
	npm run build --prefix packages/society-pi-host
	SOCIETY_PI_PROVIDER="$(PROVIDER)" SOCIETY_PI_MODEL="$(MODEL)" node packages/society-pi-host/dist/src/paid-run.js
