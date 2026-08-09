#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$repository_root"

# Keep this expression token-shaped.  In particular, do not reject unrelated
# lockfile integrity text that happens to contain the same three characters.
application_vocabulary='(^|[^[:alnum:]_])([Xx][Ss][Hh][Tt]?|[Vv][Ss][-_]?001)([^[:alnum:]_]|$)'

if rg -n --pcre2 "$application_vocabulary" \
    AGENTS.md ARCHITECTURE.md DEPENDENCIES.md GLOSSARY.md RSI.md VERTICAL-SLICE.md \
    Cargo.toml crates migrations packages tests \
    --glob '!generic-boundary/**' \
    --glob '!tests/generic-boundary/**'
then
    echo 'generic boundary contains application-owned vocabulary' >&2
    exit 1
fi

if rg -n \
    'FOUNDING_SOCIETY_HARD_CEILING|PI_SDK_QUALIFICATION_CEILING|PINNED_PI_SDK_CYCLE_CEILING' \
    crates migrations packages tests \
    --glob '!generic-boundary/**' \
    --glob '!tests/generic-boundary/**'
then
    echo 'generic boundary contains a retired application budget policy' >&2
    exit 1
fi

# These names belonged to a former synthetic catalog application.  Even a
# provider-free example must not smuggle product/evaluator semantics back into
# generic crate tests under the guise of proving genericity.
if rg -n -i 'aurora-catalog|price_cents|verify-catalog' crates migrations packages tests \
    --glob '!generic-boundary/**' \
    --glob '!tests/generic-boundary/**'
then
    echo 'generic boundary contains an application-shaped fixture' >&2
    exit 1
fi

if rg -n 'applications/' Cargo.toml Cargo.lock
then
    echo 'root workspace manifest or lockfile depends on an application workspace' >&2
    exit 1
fi


if test -e crates/society-product/Cargo.lock
then
    echo 'root workspace member retains a stale nested lockfile' >&2
    exit 1
fi

root_metadata=$(cargo metadata --format-version 1 --no-deps)
case "$root_metadata" in
    *'/applications/'*)
        echo 'root Cargo metadata includes an application package' >&2
        exit 1
        ;;
esac

echo 'generic boundary contains no application knowledge'
