#!/bin/sh
# Cross-check the self-contained Rust direct adapter against the existing
# provider-free shell curation judge. This script creates only a temporary
# application test workspace and never calls a provider.
set -eu

application_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
circuit_root="$application_root/circuits/vs-001-spawn-stderr"
fixture_root="$circuit_root/fixtures/curation"

cargo build --quiet -p society-xsh-circuit --bin vs001-direct-evaluator-adapter
adapter="$application_root/target/debug/vs001-direct-evaluator-adapter"
shell_judge="$circuit_root/judges/run-curation-contract-judge.sh"
test_root=$(mktemp -d "${TMPDIR:-/tmp}/society-xsh-curation-direct.XXXXXX")

cleanup() {
  status=$?
  rm -rf "$test_root"
  exit "$status"
}
trap cleanup 0 HUP INT TERM

fail() {
  printf 'direct curation cross-check: %s\n' "$*" >&2
  exit 1
}

write_frame() {
  role=$1
  source=$2
  set -- $(wc -c < "$source")
  printf '%s\t%s\n' "$role" "$1"
  cat "$source"
}

write_manifest() {
  account_dir=$1
  manifest=$2
  {
    printf '%s\n' '# schema: Vs001CurationDirectInputManifestV1/framed-v1'
    write_frame account "$account_dir/account.v1.tsv"
    write_frame selected_items "$account_dir/selected-items.v1.tsv"
    write_frame preserved_conflicts "$account_dir/preserved-conflicts.v1.tsv"
    write_frame decision_relevant_unknowns "$account_dir/decision-relevant-unknowns.v1.tsv"
    write_frame exclusions "$account_dir/exclusions.v1.tsv"
    write_frame raw_evidence_escalations "$account_dir/raw-evidence-escalations.v1.tsv"
    write_frame frontier_members "$fixture_root/frontier-c1-members.v1.tsv"
  } > "$manifest"
}

copy_account() {
  source=$1
  destination=$2
  mkdir "$destination"
  cp "$source/"* "$destination/"
}

run_positive() {
  name=$1
  source=$2
  account_dir="$test_root/$name-account"
  manifest="$test_root/$name-input.v1"
  direct_output="$test_root/$name-direct.tsv"
  shell_output="$test_root/$name-shell"
  copy_account "$source" "$account_dir"
  write_manifest "$account_dir" "$manifest"
  env -i "$adapter" --input-manifest "$manifest" > "$direct_output"
  "$shell_judge" \
    --account-dir "$account_dir" \
    --frontier-members "$fixture_root/frontier-c1-members.v1.tsv" \
    --out "$shell_output" >/dev/null
  cmp -s "$direct_output" "$shell_output/curation-contract-observation.v1.tsv" || \
    fail 'direct adapter output differs from shell curation observation'
}

run_negative() {
  name=$1
  target=$2
  replacement=$3
  account_dir="$test_root/$name-account"
  manifest="$test_root/$name-input.v1"
  shell_output="$test_root/$name-shell"
  copy_account "$fixture_root/c1-valid" "$account_dir"
  cp "$replacement" "$account_dir/$target"
  write_manifest "$account_dir" "$manifest"
  if env -i "$adapter" --input-manifest "$manifest" >/dev/null 2>&1; then
    fail "$name direct adapter unexpectedly accepted shell-negative relation"
  fi
  if "$shell_judge" \
    --account-dir "$account_dir" \
    --frontier-members "$fixture_root/frontier-c1-members.v1.tsv" \
    --out "$shell_output" >/dev/null 2>&1
  then
    fail "$name shell judge unexpectedly accepted its negative relation"
  fi
}

run_positive positive-none "$fixture_root/c1-valid"
run_positive positive-named "$fixture_root/c1-valid-named-escalation"
run_negative selected-unadmitted selected-items.v1.tsv \
  "$fixture_root/negative-selected-items-unadmitted.v1.tsv"
run_negative escalation-unnamed raw-evidence-escalations.v1.tsv \
  "$fixture_root/negative-raw-evidence-escalations-unnamed.v1.tsv"
run_negative exclusions-duplicate exclusions.v1.tsv \
  "$fixture_root/negative-exclusions-duplicate-category.v1.tsv"
run_negative conflicts-extra preserved-conflicts.v1.tsv \
  "$fixture_root/negative-preserved-conflicts-extra-row.v1.tsv"

printf '%s\n' 'direct curation adapter matches the shell curation judge on checked-in fixtures'
