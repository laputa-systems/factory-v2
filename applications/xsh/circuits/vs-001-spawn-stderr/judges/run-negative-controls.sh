#!/bin/sh
# Demonstrate that the VS-001 evaluator rejects named bad alternatives.
# No product checkout is changed; all disposable evidence is written under --out.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
circuit_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
fixtures_dir="$circuit_dir/fixtures"
canonical_report="$fixtures_dir/negative-controls/positive/negative-controls.v1.tsv"

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
    --xsh)
      [ "$#" -ge 2 ] || usage
      xsh=$2
      shift 2
      ;;
    --xsht)
      [ "$#" -ge 2 ] || usage
      xsht=$2
      shift 2
      ;;
    --xsh-root)
      [ "$#" -ge 2 ] || usage
      xsh_root=$2
      shift 2
      ;;
    --out)
      [ "$#" -ge 2 ] || usage
      out=$2
      shift 2
      ;;
    *) usage ;;
  esac
done
[ -n "$xsh" ] && [ -n "$xsht" ] && [ -n "$xsh_root" ] && [ -n "$out" ] || usage
[ -x "$xsh" ] || {
  printf 'xsh binary is not executable: %s\n' "$xsh" >&2
  exit 66
}
[ -x "$xsht" ] || {
  printf 'xsht binary is not executable: %s\n' "$xsht" >&2
  exit 66
}
[ -d "$xsh_root/.git" ] || {
  printf 'not an XSH checkout: %s\n' "$xsh_root" >&2
  exit 66
}
[ -f "$canonical_report" ] || {
  printf 'missing canonical negative-control report: %s\n' "$canonical_report" >&2
  exit 66
}
command -v cargo >/dev/null 2>&1 || {
  printf '%s\n' 'negative controls: missing required host tool: cargo' >&2
  exit 69
}
case "$out" in
  /*) ;;
  *) printf '%s\n' '--out must be an absolute path' >&2; exit 64 ;;
esac
if [ -e "$out" ]; then
  [ -d "$out" ] && [ -z "$(find "$out" -mindepth 1 -maxdepth 1 -print -quit)" ] || {
    printf 'output directory must be empty: %s\n' "$out" >&2
    exit 73
  }
else
  mkdir -p "$out"
fi

fail() {
  printf 'negative controls: %s\n' "$*" >&2
  exit 1
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

run_inherited_stderr_probe() {
  interpreter=$1
  child_xsh=$2
  probe_dir=$3
  mkdir "$probe_dir"
  probe_marker="$probe_dir/marker"
  set +e
  "$interpreter" "$fixtures_dir/negative/probe-process-run-inherit.xsh" \
    "$child_xsh" "$fixtures_dir/noisy-child.xsh" c05 "$probe_marker" \
    > "$probe_dir/parent.stdout" 2> "$probe_dir/parent.stderr"
  probe_status=$?
  set -e
  [ "$probe_status" -eq 0 ] || fail "inherited-stderr probe exited $probe_status"
  assert_exact 'stdout:c05
process-run-inherit:waited
' "$probe_dir/parent.stdout"
  assert_exact 'complete:c05' "$probe_marker"
}

c05_worktree=''
c05_worktree_created=false
cleanup_c05_worktree() {
  cleanup_status=$?
  trap - 0 HUP INT TERM
  if [ "$c05_worktree_created" = true ]; then
    if ! git -C "$xsh_root" worktree remove --force "$c05_worktree"; then
      printf 'negative controls: failed to remove C05 detached worktree: %s\n' "$c05_worktree" >&2
      [ "$cleanup_status" -ne 0 ] || cleanup_status=1
    fi
  fi
  return "$cleanup_status"
}
on_c05_signal() {
  signal_status=$1
  cleanup_c05_worktree
  exit "$signal_status"
}
trap 'on_c05_signal 129' HUP
trap 'on_c05_signal 130' INT
trap 'on_c05_signal 143' TERM
trap cleanup_c05_worktree 0

results="$out/negative-controls.v1.tsv"
printf '%s\n' '# schema: NegativeControlObservationV1/tsv-v1' > "$results"
printf '%s\t%s\t%s\n' control_id disposition rejection_reason >> "$results"

# First establish the real positive behavior evidence the controls challenge.
"$script_dir/run-behavior-matrix.sh" --xsh "$xsh" --xsht "$xsht" --xsh-root "$xsh_root" --out "$out/behavior-positive" >/dev/null

negative_dir="$out/no-stderr"
mkdir "$negative_dir"
marker="$negative_dir/marker"
stdout="$negative_dir/stdout"
parent_stdout="$negative_dir/parent.stdout"
parent_stderr="$negative_dir/parent.stderr"
set +e
"$xsh" "$fixtures_dir/negative/no-stderr-plan.xsh" "$xsh" "$fixtures_dir/noisy-child.xsh" negative-token "$marker" "$stdout" > "$parent_stdout" 2> "$parent_stderr"
status=$?
set -e
[ "$status" -eq 0 ] || fail 'no-stderr plan fixture did not execute'
[ -f "$marker" ] || fail 'no-stderr plan did not run its child'
printf 'stdout:negative-token\n' > "$negative_dir/expected-stdout"
printf 'stderr:negative-token\n' > "$negative_dir/expected-stderr"
cmp -s "$negative_dir/expected-stdout" "$stdout" || fail 'no-stderr plan unexpectedly lost stdout behavior'
if cmp -s "$negative_dir/expected-stderr" "$parent_stderr"; then
  printf 'C01\trejected\tomitted_stderr_field_leaks_child_stderr_to_parent\n' >> "$results"
else
  fail 'no-stderr plan did not expose the expected parent-stderr mismatch'
fi

rg -q '"sh", "-c"' "$fixtures_dir/negative/shell-wrapper.xsh" || fail 'shell-wrapper control lost its prohibited shape'
printf 'C02\trejected\tshell_string_boundary\n' >> "$results"

c03_fluency_out="$out/c03-fake-fluency"
set +e
"$script_dir/run-fluency-task-evaluator.sh" \
  --xsh "$xsh" \
  --xsht "$xsht" \
  --xsh-root "$xsh_root" \
  --solution "$fixtures_dir/fluency/negative/supervise-no-owned-lifecycle.xsh" \
  --submission "$fixtures_dir/fluency/positive/submission.v1.tsv" \
  --tool-events "$fixtures_dir/fluency/positive/tool-events.v1.tsv" \
  --workspace-label c03-fake \
  --out "$c03_fluency_out" > "$out/C03.fluency.stdout" 2> "$out/C03.fluency.stderr"
c03_fluency_status=$?
set -e
[ "$c03_fluency_status" -ne 0 ] || fail 'fake fluency solution unexpectedly passed the task evaluator'
rg -q 'missing owned spawn command path' "$out/C03.fluency.stderr" || fail 'task evaluator did not reject the missing owned lifecycle'

# The task-evaluator rejection proves this solution has no owned child
# lifecycle. Running it directly with a different child proves its fabricated
# output cannot survive a varied payload, because it never starts that child.
c03_payload_dir="$out/c03-varied-payload"
mkdir "$c03_payload_dir"
c03_payload_stderr="$c03_payload_dir/error log with spaces.txt"
c03_child_marker="$c03_payload_dir/child-ran-marker"
set +e
"$xsh" "$fixtures_dir/fluency/negative/supervise-no-owned-lifecycle.xsh" \
  "$xsh" "$fixtures_dir/fluency/negative/c03-child-marker.xsh" "$c03_payload_stderr" "$c03_child_marker" \
  > "$c03_payload_dir/parent.stdout" 2> "$c03_payload_dir/parent.stderr"
c03_payload_status=$?
set -e
[ "$c03_payload_status" -eq 0 ] || fail "fake fluency solution exited $c03_payload_status on varied payload"
assert_exact '' "$c03_payload_dir/parent.stderr"
[ ! -e "$c03_child_marker" ] || fail 'fake lifecycle control unexpectedly started the child'
printf 'stderr:space
' > "$c03_payload_dir/expected-stderr"
if cmp -s "$c03_payload_dir/expected-stderr" "$c03_payload_stderr"; then
  fail 'fabricated fluency stderr unexpectedly matched the varied payload'
fi
printf 'stdout:space
' > "$c03_payload_dir/expected-parent.stdout"
if cmp -s "$c03_payload_dir/expected-parent.stdout" "$c03_payload_dir/parent.stdout"; then
  fail 'fabricated fluency output unexpectedly matched the varied payload'
fi
printf 'C03\trejected\tfluency_evaluator_rejects_missing_owned_lifecycle_and_varied_payload_mismatches\n' >> "$results"

candidate_stale_out="$out/candidate-stale-docs"
set +e
"$script_dir/run-documentation-matrix.sh" \
  --xsh-root "$xsh_root" \
  --xsht "$xsht" \
  --mode candidate-reconciled \
  --out "$candidate_stale_out" > "$out/C04.stdout" 2> "$out/C04.stderr"
candidate_stale_status=$?
set -e
[ "$candidate_stale_status" -ne 0 ] || fail 'stale candidate documentation unexpectedly reconciled'
rg -q 'retains the stale LANG' "$out/C04.stderr" || fail 'candidate evaluator did not reject the stale LANG claim'
printf 'C04\trejected\tcandidate_reconciled_evaluator_rejects_stale_lang\n' >> "$results"

# C05 proves the fixed inherit expectation against a real changed runtime,
# rather than changing the evaluator's expected bytes. The exact fixture patch
# changes only SpawnManagedOptions::inherited_process_group in a detached
# worktree; Cargo output stays under this disposable evaluator output root.
c05_patch="$fixtures_dir/negative/c05-inherited-process-group-null-stderr.patch"
c05_baseline_dir="$out/c05-baseline"
run_inherited_stderr_probe "$xsh" "$xsh" "$c05_baseline_dir"
assert_exact 'stderr:c05
' "$c05_baseline_dir/parent.stderr"

c05_head=$(git -C "$xsh_root" rev-parse HEAD)
c05_worktree="$out/c05-runtime-worktree"
c05_target="$out/c05-cargo-target"
[ ! -e "$c05_worktree" ] || fail "C05 worktree path already exists: $c05_worktree"
git -C "$xsh_root" worktree add --detach "$c05_worktree" "$c05_head" \
  > "$out/C05.worktree-add.stdout" 2> "$out/C05.worktree-add.stderr"
c05_worktree_created=true
git -C "$c05_worktree" apply --check "$c05_patch" \
  > "$out/C05.patch-check.stdout" 2> "$out/C05.patch-check.stderr"
git -C "$c05_worktree" apply "$c05_patch"
cp "$c05_patch" "$out/C05.runtime.patch"
[ "$(git -C "$c05_worktree" status --porcelain=v1)" = ' M src/runtime/process.rs' ] || {
  fail 'C05 fixture patch changed an unexpected detached-worktree path'
}
(
  cd "$c05_worktree"
  CARGO_TARGET_DIR="$c05_target" cargo build --locked --offline --bin xsh
) > "$out/C05.cargo-build.stdout" 2> "$out/C05.cargo-build.stderr"
c05_candidate="$c05_target/debug/xsh"
[ -x "$c05_candidate" ] || fail 'C05 candidate build did not produce xsh'
c05_candidate_dir="$out/c05-candidate"
run_inherited_stderr_probe "$c05_candidate" "$c05_candidate" "$c05_candidate_dir"
printf 'stderr:c05
' > "$c05_candidate_dir/expected-inherited.stderr"
if cmp -s "$c05_candidate_dir/expected-inherited.stderr" "$c05_candidate_dir/parent.stderr"; then
  fail 'patched runtime unexpectedly preserved inherited stderr'
fi
assert_exact '' "$c05_candidate_dir/parent.stderr"
git -C "$xsh_root" worktree remove --force "$c05_worktree"
c05_worktree_created=false
[ ! -e "$c05_worktree" ] || fail 'C05 detached worktree survived successful cleanup'
[ -z "$(git -C "$xsh_root" status --porcelain=v1)" ] || fail 'C05 changed the assigned XSH checkout'
printf 'C05\trejected\tpatched_runtime_breaks_immutable_inherited_stderr\n' >> "$results"

# The parser consumes the same fixed report fixture. A successful real judge
# must therefore emit the exact closed schema, row order, disposition, and
# reason vocabulary rather than merely a collection of individually useful
# messages. This remains evaluator output, not evidence admission.
cmp -s "$canonical_report" "$results" || fail 'negative-control report drifted from its canonical contract'

printf 'negative controls passed; results: %s\n' "$results"
