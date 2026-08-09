#!/bin/sh
# Prove that the Milestone-7 circuit judges reject named invalid alternatives.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
circuit_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
fixtures="$circuit_dir/fixtures"

usage() {
  printf '%s\n' "usage: $0 --xsh ABSOLUTE_XSH --xsht ABSOLUTE_XSHT --xsh-root ABSOLUTE_XSH_SOURCE --out EMPTY_OUTPUT_DIRECTORY" >&2
  exit 64
}

xsh=''
xsht=''
xsh_root=''
out=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --xsh) [ "$#" -ge 2 ] || usage; xsh=$2; shift 2 ;;
    --xsht) [ "$#" -ge 2 ] || usage; xsht=$2; shift 2 ;;
    --xsh-root) [ "$#" -ge 2 ] || usage; xsh_root=$2; shift 2 ;;
    --out) [ "$#" -ge 2 ] || usage; out=$2; shift 2 ;;
    *) usage ;;
  esac
done
[ -n "$xsh" ] && [ -n "$xsht" ] && [ -n "$xsh_root" ] && [ -n "$out" ] || usage
[ -x "$xsh" ] && [ -x "$xsht" ] && [ -d "$xsh_root/.git" ] || {
  printf '%s\n' 'society negatives: assigned XSH inputs are unavailable' >&2
  exit 66
}
case "$out" in
  /*) ;;
  *) printf '%s\n' 'society negatives: --out must be absolute' >&2; exit 64 ;;
esac
for required_tool in awk cp env find mkdir rg; do
  command -v "$required_tool" >/dev/null 2>&1 || {
    printf 'society negatives: missing required host tool: %s\n' "$required_tool" >&2
    exit 69
  }
done
if [ -e "$out" ]; then
  [ -d "$out" ] && [ -z "$(find "$out" -mindepth 1 -maxdepth 1 -print -quit)" ] || {
    printf 'society negatives: output directory must be empty: %s\n' "$out" >&2
    exit 73
  }
else
  mkdir -p "$out"
fi

fail() {
  printf 'society negatives: %s\n' "$*" >&2
  exit 1
}
results="$out/society-negative-controls.v1.tsv"
printf '%s\n' '# schema: SocietyNegativeControlObservationV1/tsv-v1' > "$results"
printf 'control_id\tdisposition\trejection_reason\n' >> "$results"

expect_rejected() {
  control_id=$1
  expected_error=$2
  rejection_reason=$3
  shift 3
  set +e
  "$@" > "$out/$control_id.stdout" 2> "$out/$control_id.stderr"
  command_exit=$?
  set -e
  [ "$command_exit" -ne 0 ] || fail "$control_id unexpectedly passed"
  rg -q -- "$expected_error" "$out/$control_id.stderr" || fail "$control_id did not report its named rejection"
  printf '%s\trejected\t%s\n' "$control_id" "$rejection_reason" >> "$results"
}

positive_submission="$fixtures/fluency/positive/submission.v1.tsv"
positive_events="$fixtures/fluency/positive/tool-events.v1.tsv"
expect_rejected C06 'forbidden process.spawn detached API' detached_process_spawn \
  "$script_dir/run-fluency-task-evaluator.sh" \
  --xsh "$xsh" --xsht "$xsht" --xsh-root "$xsh_root" \
  --solution "$fixtures/fluency/negative/supervise-process-spawn.xsh" \
  --submission "$positive_submission" --tool-events "$positive_events" \
  --workspace-label nc6 --out "$out/C06-work"
expect_rejected C07 'hard-coded fixture output' fabricated_fixture_output \
  "$script_dir/run-fluency-task-evaluator.sh" \
  --xsh "$xsh" --xsht "$xsht" --xsh-root "$xsh_root" \
  --solution "$fixtures/fluency/negative/supervise-fake-output.xsh" \
  --submission "$positive_submission" --tool-events "$positive_events" \
  --workspace-label nc7 --out "$out/C07-work"
expect_rejected C08 'unexpected host path access' undeclared_xsh_source_access \
  "$script_dir/run-fluency-task-evaluator.sh" \
  --xsh "$xsh" --xsht "$xsht" --xsh-root "$xsh_root" \
  --solution "$fixtures/fluency/positive/supervise.xsh" \
  --submission "$positive_submission" --tool-events "$fixtures/fluency/negative/tool-events-known-xsh.v1.tsv" \
  --workspace-label nc8 --out "$out/C08-work"

mkdir "$out/C09-account"
cp "$fixtures/curation/c1-valid/"* "$out/C09-account/"
cp "$fixtures/curation/negative-selected-items-unadmitted.v1.tsv" "$out/C09-account/selected-items.v1.tsv"
expect_rejected C09 'selection source outside frontier' unadmitted_curation_source \
  "$script_dir/run-curation-contract-judge.sh" \
  --account-dir "$out/C09-account" --frontier-members "$fixtures/curation/frontier-c1-members.v1.tsv" --out "$out/C09-work"

mkdir "$out/C10-account"
cp "$fixtures/curation/c1-valid/"* "$out/C10-account/"
cp "$fixtures/curation/negative-raw-evidence-escalations-unnamed.v1.tsv" "$out/C10-account/raw-evidence-escalations.v1.tsv"
expect_rejected C10 'raw evidence escalation lacks a valid named question and object' unnamed_raw_evidence_request \
  "$script_dir/run-curation-contract-judge.sh" \
  --account-dir "$out/C10-account" --frontier-members "$fixtures/curation/frontier-c1-members.v1.tsv" --out "$out/C10-work"

mkdir "$out/C14-account"
cp "$fixtures/curation/c1-valid/"* "$out/C14-account/"
cp "$fixtures/curation/negative-selected-items-duplicate-source.v1.tsv" "$out/C14-account/selected-items.v1.tsv"
expect_rejected C14 'duplicate selected source_ref' duplicate_curation_source_ref \
  "$script_dir/run-curation-contract-judge.sh" \
  --account-dir "$out/C14-account" --frontier-members "$fixtures/curation/frontier-c1-members.v1.tsv" --out "$out/C14-work"

mkdir "$out/C15-account"
cp "$fixtures/curation/c1-valid/"* "$out/C15-account/"
cp "$fixtures/curation/negative-exclusions-duplicate-category.v1.tsv" "$out/C15-account/exclusions.v1.tsv"
expect_rejected C15 'duplicate exclusion category_or_source' duplicate_curation_exclusion \
  "$script_dir/run-curation-contract-judge.sh" \
  --account-dir "$out/C15-account" --frontier-members "$fixtures/curation/frontier-c1-members.v1.tsv" --out "$out/C15-work"

mkdir "$out/C16-account"
cp "$fixtures/curation/c1-valid/"* "$out/C16-account/"
cp "$fixtures/curation/negative-preserved-conflicts-extra-row.v1.tsv" "$out/C16-account/preserved-conflicts.v1.tsv"
expect_rejected C16 'preserved-conflicts relation must contain exactly 1 row' malformed_curation_extra_row \
  "$script_dir/run-curation-contract-judge.sh" \
  --account-dir "$out/C16-account" --frontier-members "$fixtures/curation/frontier-c1-members.v1.tsv" --out "$out/C16-work"

ambient_cwd=$(pwd)
expect_rejected C17 'F01 supervisor exit was' controlled_environment_rejects_inherited_host_context \
  env XSH_FLUENCY_HOST_SENTINEL=ambient-host-only "XSH_FLUENCY_HOST_CWD=$ambient_cwd" \
  "$script_dir/run-fluency-task-evaluator.sh" \
  --xsh "$xsh" --xsht "$xsht" --xsh-root "$xsh_root" \
  --solution "$fixtures/fluency/negative/supervise-inherited-context.xsh" \
  --submission "$positive_submission" --tool-events "$positive_events" \
  --workspace-label nc17 --out "$out/C17-work"

expect_rejected C11 'application lacks a required record class or named unavailability explanation' missing_uptake_record_class \
  "$script_dir/run-uptake-application-judge.sh" \
  --context "$fixtures/uptake/positive/delivery-context.v1.tsv" \
  --persisted-input "$fixtures/uptake/positive/persisted-input.v1.tsv" \
  --submission "$fixtures/uptake/negative/investigator-submission-missing-call-sites.v1.tsv" \
  --accesses "$fixtures/uptake/positive/accesses.v1.tsv" --out "$out/C11-work"
[ "$(awk -F '\t' 'NR == 3 { print $7 }' "$out/C11-work/uptake-application-observation.v1.tsv")" = rejected_missing_record_class ] || fail 'C11 omitted its explicit non-application disposition'

expect_rejected C12 'forbidden VS-001 session or post-target material access' uptake_contamination \
  "$script_dir/run-uptake-application-judge.sh" \
  --context "$fixtures/uptake/positive/delivery-context.v1.tsv" \
  --persisted-input "$fixtures/uptake/positive/persisted-input.v1.tsv" \
  --submission "$fixtures/uptake/positive/investigator-submission.v1.tsv" \
  --accesses "$fixtures/uptake/negative/accesses-forbidden-session.v1.tsv" --out "$out/C12-work"
[ "$(awk -F '\t' 'NR == 3 { print $6 " " $7 }' "$out/C12-work/uptake-application-observation.v1.tsv")" = 'contaminated contamination_recorded' ] || fail 'C12 did not preserve contamination separately from application'

expect_rejected C13 'frontier member overlaps sequestered aftermath material' frontier_overlap_leak \
  "$script_dir/run-frontier-leakage-controls.sh" \
  --frontier-dir "$fixtures/frontier/w1-overlap" --out "$out/C13-work"

expect_rejected C18 'frontier is missing a required positive member' missing_frontier_positive_member \
  "$script_dir/run-frontier-leakage-controls.sh" \
  --frontier-dir "$fixtures/frontier/w1-missing-positive" --out "$out/C18-work"

expect_rejected C19 'duplicate sequestered reference_class' duplicate_sequestered_aftermath_class \
  "$script_dir/run-frontier-leakage-controls.sh" \
  --frontier-dir "$fixtures/frontier/w1-duplicate-class" --out "$out/C19-work"

overlong_workspace_label='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
expect_rejected C20 'workspace label is not opaque-safe' overlong_opaque_workspace_label \
  "$script_dir/run-fluency-task-evaluator.sh" \
  --xsh "$xsh" --xsht "$xsht" --xsh-root "$xsh_root" \
  --solution "$fixtures/fluency/positive/supervise.xsh" \
  --submission "$positive_submission" --tool-events "$positive_events" \
  --workspace-label "$overlong_workspace_label" --out "$out/C20-work"

printf 'society negative controls passed; results: %s\n' "$results"
