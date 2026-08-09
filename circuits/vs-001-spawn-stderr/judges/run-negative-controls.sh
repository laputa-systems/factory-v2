#!/bin/sh
# Demonstrate that the VS-001 evaluator rejects named bad alternatives.
# No product checkout is changed; all disposable evidence is written under --out.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
circuit_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
fixtures_dir="$circuit_dir/fixtures"

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

fake_dir="$out/fake-log"
mkdir "$fake_dir"
fake_log="$fake_dir/stderr"
"$xsh" "$fixtures_dir/negative/fake-log.xsh" "$fake_log" > "$fake_dir/parent.stdout" 2> "$fake_dir/parent.stderr"
printf 'stderr:varied-token\n' > "$fake_dir/expected-stderr"
if ! cmp -s "$fake_dir/expected-stderr" "$fake_log" && ! rg -q 'spawn ' "$fixtures_dir/negative/fake-log.xsh"; then
  printf 'C03\trejected\tvarying_payload_and_owned_lifecycle_absent\n' >> "$results"
else
  fail 'fake-log control could satisfy the evaluator'
fi

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

suppressed_out="$out/default-suppressed"
set +e
"$script_dir/run-behavior-matrix.sh" \
  --xsh "$xsh" \
  --xsht "$xsht" \
  --xsh-root "$xsh_root" \
  --default-stderr suppressed \
  --out "$suppressed_out" > "$out/C05.stdout" 2> "$out/C05.stderr"
suppressed_status=$?
set -e
[ "$suppressed_status" -ne 0 ] || fail 'suppressed-default candidate unexpectedly matched inherited behavior'
rg -q 'byte comparison failed' "$out/C05.stderr" || fail 'behavior evaluator did not reject changed default inheritance'
printf 'C05\trejected\tbehavior_evaluator_rejects_default_stderr_suppression\n' >> "$results"

printf 'negative controls passed; results: %s\n' "$results"
