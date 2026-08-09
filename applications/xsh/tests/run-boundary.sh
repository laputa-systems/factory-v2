#!/bin/sh
set -eu

application_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
repository_root=$(CDPATH= cd -- "$application_root/../.." && pwd)
cd "$application_root"

metadata=$(cargo metadata --format-version 1 --no-deps)
XSH_APPLICATION_METADATA=$metadata \
XSH_APPLICATION_ROOT="$application_root" \
XSH_GENERIC_ROOT="$repository_root/crates" \
node <<'NODE'
const metadata = JSON.parse(process.env.XSH_APPLICATION_METADATA);
const applicationRoot = process.env.XSH_APPLICATION_ROOT;
const genericRoot = process.env.XSH_GENERIC_ROOT;

const members = metadata.workspace_members
    .map((id) => metadata.packages.find((candidate) => candidate.id === id)?.name)
    .sort();
const expectedMembers = ["society-xsh-circuit", "society-xsh-contract"];
if (JSON.stringify(members) !== JSON.stringify(expectedMembers)) {
    throw new Error(`unexpected XSH workspace members: ${members.join(",")}`);
}

const approved = new Map([
    ["society-xsh-circuit", new Set(["society-content"])],
    ["society-xsh-contract", new Set(["society-kernel"])],
]);
for (const package of metadata.packages) {
    if (!package.manifest_path.startsWith(`${applicationRoot}/`)) {
        throw new Error(`${package.name} is not owned by the XSH workspace`);
    }
    for (const dependency of package.dependencies.filter((value) => typeof value.path === "string")) {
        if (!dependency.path.startsWith(`${genericRoot}/`)) {
            throw new Error(`${package.name} dependency escapes generic crates: ${dependency.path}`);
        }
        if (!approved.get(package.name)?.has(dependency.name)) {
            throw new Error(`${package.name} has unapproved generic dependency ${dependency.name}`);
        }
    }
}
NODE

echo 'XSH application workspace owns only approved inward generic dependencies'
