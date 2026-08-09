#!/bin/sh
# Evaluate one sealed paired-fluency task submission without a provider call.
#
# The TSV outputs are an intentionally closed transport for a future Rust
# `FluencyProbeObservationV1` parser. They are evidence candidates only: this
# script does not attest a Pi attempt, settle cost, reveal a treatment mapping,
# or mutate durable society state.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
circuit_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
fixture_dir="$circuit_dir/fixtures/fluency"
tab=$(printf '\t')

usage() {
  printf '%s\n' "usage: $0 --xsh ABSOLUTE_XSH --xsht ABSOLUTE_XSHT --xsh-root ABSOLUTE_XSH_SOURCE --solution ABSOLUTE_SUPERVISE_XSH --submission ABSOLUTE_SUBMISSION_TSV --tool-events ABSOLUTE_TOOL_EVENTS_TSV --workspace-label OPAQUE_LABEL --out EMPTY_OUTPUT_DIRECTORY" >&2
  exit 64
}

xsh=''
xsht=''
xsh_root=''
solution=''
submission=''
tool_events=''
workspace_label=''
out=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --xsh) [ "$#" -ge 2 ] || usage; xsh=$2; shift 2 ;;
    --xsht) [ "$#" -ge 2 ] || usage; xsht=$2; shift 2 ;;
    --xsh-root) [ "$#" -ge 2 ] || usage; xsh_root=$2; shift 2 ;;
    --solution) [ "$#" -ge 2 ] || usage; solution=$2; shift 2 ;;
    --submission) [ "$#" -ge 2 ] || usage; submission=$2; shift 2 ;;
    --tool-events) [ "$#" -ge 2 ] || usage; tool_events=$2; shift 2 ;;
    --workspace-label) [ "$#" -ge 2 ] || usage; workspace_label=$2; shift 2 ;;
    --out) [ "$#" -ge 2 ] || usage; out=$2; shift 2 ;;
    *) usage ;;
  esac
done

[ -n "$xsh" ] && [ -n "$xsht" ] && [ -n "$xsh_root" ] && [ -n "$solution" ] && [ -n "$submission" ] && [ -n "$tool_events" ] && [ -n "$workspace_label" ] && [ -n "$out" ] || usage
[ -x "$xsh" ] && [ -x "$xsht" ] && [ -f "$solution" ] && [ -f "$submission" ] && [ -f "$tool_events" ] || {
  printf '%s\n' 'fluency evaluator: assigned binary or submission input is unavailable' >&2
  exit 66
}
[ -d "$xsh_root/.git" ] || {
  printf 'fluency evaluator: not an XSH checkout: %s\n' "$xsh_root" >&2
  exit 66
}
case "$workspace_label" in
  *[!abcdefghijklmnopqrstuvwxyz0123456789-]* | '')
    printf 'fluency evaluator: workspace label is not opaque-safe: %s\n' "$workspace_label" >&2
    exit 64
    ;;
esac
# The label becomes one native path component. Its alphabet is ASCII, so the
# shell character count is also the exact byte count consumed by the Rust
# `OpaqueWorkspaceLabel` boundary.
if [ "${#workspace_label}" -gt 64 ]; then
  printf 'fluency evaluator: workspace label is not opaque-safe: %s\n' "$workspace_label" >&2
  exit 64
fi
case "$out" in
  /*) ;;
  *) printf '%s\n' 'fluency evaluator: --out must be absolute' >&2; exit 64 ;;
esac
for required_tool in awk cmp cp env find git mkdir od rg sed; do
  command -v "$required_tool" >/dev/null 2>&1 || {
    printf 'fluency evaluator: missing required host tool: %s\n' "$required_tool" >&2
    exit 69
  }
done
if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
  printf '%s\n' 'fluency evaluator: missing sha256sum or shasum' >&2
  exit 69
fi
[ -z "$(git -C "$xsh_root" status --porcelain=v1)" ] || {
  printf 'fluency evaluator: refusing dirty XSH source checkout: %s\n' "$xsh_root" >&2
  exit 70
}
if [ -e "$out" ]; then
  [ -d "$out" ] && [ -z "$(find "$out" -mindepth 1 -maxdepth 1 -print -quit)" ] || {
    printf 'fluency evaluator: output directory must be empty: %s\n' "$out" >&2
    exit 73
  }
else
  mkdir -p "$out"
fi

fail() {
  printf 'fluency evaluator: %s\n' "$*" >&2
  exit 1
}

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

assert_exact() {
  expected=$1
  actual=$2
  expected_file="$actual.expected"
  printf '%s' "$expected" > "$expected_file"
  if ! cmp -s "$expected_file" "$actual"; then
    printf 'expected bytes for %s:\n' "$actual" >&2
    od -An -tx1 -v "$expected_file" >&2
    printf 'actual bytes for %s:\n' "$actual" >&2
    od -An -tx1 -v "$actual" >&2
    rm -f "$expected_file"
    fail 'byte comparison failed'
  fi
  rm -f "$expected_file"
}

assert_header_and_rows() {
  table=$1
  expected_header=$2
  expected_rows=$3
  [ "$(sed -n '1p' "$table")" = "$expected_header" ] || fail "unexpected schema header: $table"
  [ "$(awk 'END { print NR - 1 }' "$table")" = "$expected_rows" ] || fail "unexpected row count: $table"
}

assert_submission() {
  assert_header_and_rows "$submission" "submission_kind${tab}solution_filename${tab}contract_revision" 1
  awk -F '\t' 'NR == 2 && $1 == "fluency_probe_solution" && $2 == "supervise.xsh" && $3 == "task_actor_v1" { ok = 1 } END { exit(ok ? 0 : 1) }' "$submission" || fail 'invalid closed task-actor submission'
}

assert_tool_events() {
  [ "$(sed -n '1p' "$tool_events")" = "ordinal${tab}action${tab}path_class" ] || fail 'unexpected tool-event schema'
  set +e
  awk -F '\t' '
    NR > 1 {
      if ($1 != NR - 1) exit 1
      if ($3 == "known_xsh_source" || $3 == "known_v1" || $3 == "known_treatment" || $3 == "unknown_external") exit 2
      if ($2 != "read_reference" && $2 != "run_xsht_api" && $2 != "write_solution") exit 1
      if ($3 != "workspace") exit 1
    }
  ' "$tool_events"
  event_exit=$?
  set -e
  case "$event_exit" in
    0) ;;
    2) fail 'unexpected host path access' ;;
    *) fail 'invalid closed task-actor tool event' ;;
  esac
}

assert_source_boundary() {
  if rg -q 'process\.spawn' "$supervise"; then
    fail 'forbidden process.spawn detached API'
  fi
  rg -q 'process\.command_argv' "$supervise" || fail 'missing typed process.command_argv path'
  rg -q 'spawn[[:space:]]+command\?' "$supervise" || fail 'missing owned spawn command path'
  rg -q 'wait[[:space:]]+handle\?' "$supervise" || fail 'missing owned wait path'
  rg -q 'stderr:' "$supervise" || fail 'missing typed stderr destination'
  if rg -q 'process\.run|spawn[[:space:]]+run' "$supervise"; then
    fail 'forbidden non-owned task execution path'
  fi
  if rg -q 'sh -c|bash -c|"-c"' "$supervise"; then
    fail 'forbidden shell wrapper'
  fi
  if rg -q '(^|[[:space:]])(print|eprint)[[:space:]]|stdout:(alpha|space|nonzero)|stderr:(alpha|space|nonzero)' "$supervise"; then
    fail 'hard-coded fixture output'
  fi
  if rg -q 'stdout:' "$supervise"; then
    fail 'stdout policy is not inherited-only'
  fi
}

workspace="$out/work/$workspace_label"
controlled_home="$workspace/home"
controlled_config="$workspace/config"
controlled_cache="$workspace/cache"
controlled_data="$workspace/data"
controlled_tmp="$workspace/tmp"
controlled_path="$workspace/bin:/usr/bin:/bin"
env_command=$(command -v env)
mkdir -p "$workspace/bin" "$workspace/fixtures" "$workspace/output" "$workspace/submission" "$controlled_home" "$controlled_config" "$controlled_cache" "$controlled_data" "$controlled_tmp"
cp "$fixture_dir/TASK.md" "$workspace/TASK.md"
cp "$fixture_dir/REFERENCE.md" "$workspace/REFERENCE.md"
cp "$xsh" "$workspace/bin/xsh"
cp "$xsht" "$workspace/bin/xsht"
cp "$fixture_dir/children/child-alpha.xsh" "$workspace/fixtures/child-alpha.xsh"
cp "$fixture_dir/children/child with spaces.xsh" "$workspace/fixtures/child with spaces.xsh"
cp "$fixture_dir/children/child-nonzero.xsh" "$workspace/fixtures/child-nonzero.xsh"
cp "$solution" "$workspace/submission/supervise.xsh"
cp "$submission" "$workspace/submission/submission.v1.tsv"
supervise="$workspace/submission/supervise.xsh"

# The actor-facing execution cannot inherit a caller's cwd, HOME, XDG state,
# temp root, or arbitrary host variables. Binary and fixture paths stay
# absolute so the task contract does not depend on a search-path coincidence.
run_controlled() {
  (
    cd "$workspace"
    "$env_command" -i \
      PATH="$controlled_path" \
      HOME="$controlled_home" \
      XDG_CONFIG_HOME="$controlled_config" \
      XDG_CACHE_HOME="$controlled_cache" \
      XDG_DATA_HOME="$controlled_data" \
      XSH_CONFIG_HOME="$controlled_config" \
      TMPDIR="$controlled_tmp" \
      TMP="$controlled_tmp" \
      TEMP="$controlled_tmp" \
      PWD="$workspace" \
      LANG=C \
      LC_ALL=C \
      TERM=dumb \
      NO_COLOR=1 \
      "$@"
  )
}

git -C "$xsh_root" rev-parse HEAD > "$out/xsh-source-head.txt"
git -C "$xsh_root" status --porcelain=v1 > "$out/xsh-source-status.txt"
input_digests="$out/input-digests.v1.tsv"
printf '%s\n' '# schema: FluencyProbeInputDigestV1/tsv-v1' > "$input_digests"
printf 'input_kind\tsha256\n' >> "$input_digests"
record_input_digest() {
  printf '%s\t%s\n' "$1" "$(sha256 "$2")" >> "$input_digests"
}
record_input_digest xsh_binary "$xsh"
record_input_digest xsht_binary "$xsht"
record_input_digest task_actor_evaluator "$0"
record_input_digest task_instruction "$fixture_dir/TASK.md"
record_input_digest reference_pack "$fixture_dir/REFERENCE.md"
record_input_digest task_solution "$solution"
record_input_digest task_submission "$submission"
record_input_digest task_tool_events "$tool_events"
record_input_digest child_alpha "$fixture_dir/children/child-alpha.xsh"
record_input_digest child_spaces "$fixture_dir/children/child with spaces.xsh"
record_input_digest child_nonzero "$fixture_dir/children/child-nonzero.xsh"

assert_submission
assert_tool_events
run_controlled "$workspace/bin/xsht" check --strict "$supervise" > "$out/xsht-check.stdout" 2> "$out/xsht-check.stderr" || fail 'xsht check rejected supervise.xsh'
assert_source_boundary

results="$out/fluency-task-observations.v1.tsv"
printf '%s\n' '# schema: FluencyProbeObservationV1/tsv-v1' > "$results"
printf 'case_id\tinput_manifest\texpected_exit\tsupervisor_exit\tparent_stdout_sha256\tparent_stderr_sha256\tredirected_stderr_sha256\tcorrectness\ttyped_boundary\townership_lifecycle\thost_path_access\tdisposition\n' >> "$results"
execution_surface="$out/fluency-task-execution-surface.v1.tsv"
printf '%s\n' '# schema: FluencyProbeExecutionSurfaceV1/tsv-v1' > "$execution_surface"
printf 'execution_kind\ttool_errors\tturns\tactive_wall\ttokens\treasoning_tokens\tcost\n' >> "$execution_surface"
printf 'deterministic_fixture\tnot_observed_no_provider\tnot_observed_no_provider\tnot_observed_no_provider\tnot_observed_no_provider\tnot_observed_no_provider\tnot_observed_no_provider\n' >> "$execution_surface"
execution_envelope="$out/fluency-execution-envelope.v1.tsv"
printf '%s\n' '# schema: FluencyExecutionEnvelopeV1/tsv-v1' > "$execution_envelope"
printf 'workspace_label\tworking_directory\tenvironment\thome\tconfig\ttemp\tpath\n' >> "$execution_envelope"
printf '%s\topaque_workspace\tminimal_explicit\tworkspace_local\tworkspace_local\tworkspace_local\tassigned_bin_front\n' "$workspace_label" >> "$execution_envelope"

run_case() {
  case_id=$1
  input_manifest=$2
  child_name=$3
  expected_stdout=$4
  expected_stderr=$5
  expected_exit=$6
  case_dir="$workspace/output/$case_id"
  mkdir "$case_dir"
  child="$workspace/fixtures/$child_name"
  stderr="$case_dir/error log with spaces.txt"
  printf 'old-stderr\n' > "$stderr"
  set +e
  run_controlled "$workspace/bin/xsh" "$supervise" "$workspace/bin/xsh" "$child" "$stderr" > "$case_dir/parent.stdout" 2> "$case_dir/parent.stderr"
  supervisor_exit=$?
  set -e
  [ "$supervisor_exit" -eq "$expected_exit" ] || fail "$case_id supervisor exit was $supervisor_exit, expected $expected_exit"
  assert_exact "$expected_stdout" "$case_dir/parent.stdout"
  assert_exact '' "$case_dir/parent.stderr"
  assert_exact "$expected_stderr" "$stderr"
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\tpassed\tcompliant\towned_waited\tclean\tpass\n' \
    "$case_id" "$input_manifest" "$expected_exit" "$supervisor_exit" \
    "$(sha256 "$case_dir/parent.stdout")" "$(sha256 "$case_dir/parent.stderr")" "$(sha256 "$stderr")" >> "$results"
}

run_case F01 preexisting_log_truncate child-alpha.xsh 'stdout:alpha
' 'stderr:alpha
' 0
run_case F02 path_with_spaces 'child with spaces.xsh' 'stdout:space
' 'stderr:space
' 0
run_case F03 nonzero_child_status child-nonzero.xsh 'stdout:nonzero
' 'stderr:nonzero
' 23

printf 'fluency task evaluator passed; results: %s\n' "$results"
