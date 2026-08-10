#!/bin/sh
set -eu

repository_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$repository_root"

# Keep this expression token-shaped.  In particular, do not reject unrelated
# lockfile integrity text that happens to contain the same three characters.
application_vocabulary='(^|[^[:alnum:]_])([Xx][Ss][Hh][Tt]?|[Vv][Ss][-_]?001)([^[:alnum:]_]|$)'

if rg -n --pcre2 "$application_vocabulary" \
    AGENTS.md ARCHITECTURE.md DEPENDENCIES.md FORUM.md GLOSSARY.md README.md \
    RESEARCH-PROGRAM.md VERTICAL-SLICE.md \
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

# The root governance vocabulary deliberately describes one generic root
# authority and its founding mission.  Product constitutions must not revive
# the former application institution under any casing or separator spelling.
if rg -n -i 'grand[_ -]?architect|thegrandarchitect|universe[_ -]?seed' \
    AGENTS.md ARCHITECTURE.md DEPENDENCIES.md FORUM.md GLOSSARY.md README.md \
    RESEARCH-PROGRAM.md VERTICAL-SLICE.md \
    Cargo.toml crates migrations packages tests \
    --glob '!generic-boundary/**' \
    --glob '!tests/generic-boundary/**'
then
    echo 'generic boundary contains retired application governance vocabulary' >&2
    exit 1
fi

# Tool admission is generic mechanism. Role names belong to the application
# which selected the capability set, never to the Pi wire or resident kernel.
if rg -n \
    'CuratorV1|ProductBuilderV1|TaskActorV1|ReadSourceV1|curator_v1|product_builder_v1|task_actor_v1|read_source_v1' \
    crates migrations packages tests \
    --glob '!generic-boundary/**' \
    --glob '!tests/generic-boundary/**'
then
    echo 'generic boundary contains an application-role tool profile' >&2
    exit 1
fi

if rg -n 'applications/' Cargo.toml Cargo.lock
then
    echo 'root workspace manifest or lockfile depends on an application workspace' >&2
    exit 1
fi

if rg -n 'applications/' crates migrations packages \
    --glob '!target/**' \
    --glob '!node_modules/**'
then
    echo 'generic implementation names an application path' >&2
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

for application_manifest in applications/*/Cargo.toml
do
    test -e "$application_manifest" || continue
    application_root=$(dirname "$application_manifest")
    application_metadata=$(cargo metadata \
        --manifest-path "$application_manifest" \
        --format-version 1 \
        --no-deps)
    APPLICATION_BOUNDARY_METADATA=$application_metadata \
    APPLICATION_BOUNDARY_ROOT="$repository_root/$application_root" \
    GENERIC_CRATE_ROOT="$repository_root/crates" \
    node <<'NODE'
// Applications may consume public generic domain crates, but may not gain the
// resident authority or its supervisor client by a path dependency.
const metadata = JSON.parse(process.env.APPLICATION_BOUNDARY_METADATA);
const applicationRoot = process.env.APPLICATION_BOUNDARY_ROOT;
const genericCrateRoot = process.env.GENERIC_CRATE_ROOT;
const residentAuthorityPackages = new Set(["societyd", "societyctl"]);
const residentAuthorityRoots = new Set(
    [...residentAuthorityPackages].map((name) => `${genericCrateRoot}/${name}`),
);
if (metadata.workspace_root !== applicationRoot) {
    throw new Error(`application workspace root escaped its boundary: ${metadata.workspace_root}`);
}
for (const package of metadata.packages) {
    for (const dependency of package.dependencies.filter((value) => typeof value.path === "string")) {
        const isApplicationLocal = dependency.path.startsWith(`${applicationRoot}/`);
        const isGenericInwardDependency = dependency.path.startsWith(`${genericCrateRoot}/`);
        if (!isApplicationLocal && !isGenericInwardDependency) {
            throw new Error(`${package.name} path dependency escapes allowed boundaries: ${dependency.path}`);
        }
        if (residentAuthorityPackages.has(dependency.name) || residentAuthorityRoots.has(dependency.path)) {
            throw new Error(`${package.name} depends on resident authority package ${dependency.name}`);
        }
    }
}
NODE
done

echo 'generic boundary contains no application knowledge'
