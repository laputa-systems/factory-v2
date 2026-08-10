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
const fs = require("node:fs");
const path = require("node:path");

const metadata = JSON.parse(process.env.XSH_APPLICATION_METADATA);
const applicationRoot = process.env.XSH_APPLICATION_ROOT;
const genericRoot = process.env.XSH_GENERIC_ROOT;
const repositoryRoot = path.resolve(applicationRoot, "../..");

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

const sourceFiles = (root, predicate) => {
    const files = [];
    const visit = (directory) => {
        for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
            if (entry.name === ".git" || entry.name === "target" || entry.name === "node_modules") {
                continue;
            }
            const entryPath = path.join(directory, entry.name);
            if (entry.isDirectory()) {
                visit(entryPath);
            } else if (entry.isFile() && predicate(entry.name)) {
                files.push(entryPath);
            }
        }
    };
    visit(root);
    return files;
};

const assertNoMatch = (files, expression, contract) => {
    for (const file of files) {
        const contents = fs.readFileSync(file, "utf8");
        if (expression.test(contents)) {
            throw new Error(`${contract}: ${file}`);
        }
    }
};

const applicationRust = sourceFiles(applicationRoot, (name) => name.endsWith(".rs"));
assertNoMatch(
    applicationRust,
    /\b(?:societyd|societyctl|society_product|ContentObjectId)\b/,
    "XSH application source received resident, product, or content-writer authority",
);
assertNoMatch(
    applicationRust,
    /\b(?:std::process|std::fs|rusqlite)\b/,
    "XSH application port performs resident process, filesystem, or SQLite work",
);

// Generic trusted physics must remain application-blind.  XSH command names,
// fixtures, output grammar, and evidence interpretation belong under this
// workspace; a generic crate may carry only typed sealed identities and native
// child custody facts.
const genericSources = [
    ...sourceFiles(genericRoot, (name) =>
        name.endsWith(".rs") || name.endsWith(".toml") || name.endsWith(".md"),
    ),
    ...sourceFiles(path.join(repositoryRoot, "migrations"), (name) => name.endsWith(".sql")),
    ...["AGENTS.md", "ARCHITECTURE.md", "GLOSSARY.md", "VERTICAL-SLICE.md", "DEPENDENCIES.md", "Cargo.toml"]
        .map((name) => path.join(repositoryRoot, name)),
];
assertNoMatch(
    genericSources,
    /\b(?:xsh|xsht|society[-_]xsh)\b/i,
    "generic trusted physics names an XSH evaluator semantic",
);
NODE

if rg -n \
    'AdmitDeterministicEvaluatorNativeChild|RecordDeterministicEvaluatorNativeChildSpawn|RecordNativeChildNotSpawned|FinalizeDeterministicExperiment' \
    "$repository_root/crates/societyd/src/protocol.rs" \
    "$repository_root/crates/societyctl"
then
    echo 'resident public protocol exposes native evaluator scheduling authority' >&2
    exit 1
fi

"$repository_root/tests/generic-boundary/run-no-application-knowledge.sh"

echo 'XSH application workspace owns only approved inward generic dependencies and evaluator semantics'
