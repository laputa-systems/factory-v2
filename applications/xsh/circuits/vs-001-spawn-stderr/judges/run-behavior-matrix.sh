#!/bin/sh
# Execute the provider-free VS-001 process behavior matrix.
#
# This evaluator accepts only already-built XSH/Xsht binaries. It never invokes
# Cargo, rejects a dirty source checkout, and keeps every observation under
# --out so the kernel can later seal that directory as forensic evidence.
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
for required_tool in awk cmp find git od rg sed; do
  command -v "$required_tool" >/dev/null 2>&1 || {
    printf 'missing required host tool: %s\n' "$required_tool" >&2
    exit 69
  }
done
if ! command -v b3sum >/dev/null 2>&1; then
  printf '%s\n' 'missing required host digest tool: b3sum' >&2
  exit 69
fi
[ -e /dev/null ] || {
  printf '%s\n' 'VS-001 requires a Unix /dev/null path sink' >&2
  exit 69
}
[ -z "$(git -C "$xsh_root" status --porcelain=v1)" ] || {
  printf 'refusing dirty XSH source checkout: %s\n' "$xsh_root" >&2
  exit 70
}
case "$out" in
  /*) ;;
  *) printf '%s\n' '--out must be an absolute path' >&2; exit 64 ;;
esac

if [ -e "$out" ]; then
  [ -d "$out" ] || {
    printf 'output path is not a directory: %s\n' "$out" >&2
    exit 73
  }
  [ -z "$(find "$out" -mindepth 1 -maxdepth 1 -print -quit)" ] || {
    printf 'output directory must be empty: %s\n' "$out" >&2
    exit 73
  }
else
  mkdir -p "$out"
fi

artifacts="$out/artifacts"
mkdir "$artifacts"
results="$out/behavior-observations.v1.tsv"
printf '%s\n' '# schema: BehaviorObservationV1/tsv-v1' > "$results"
printf '%s\n' 'case_id	consumer	input_manifest	expected_contract_source	disposition	exit_shape	parent_stdout_blake3	parent_stderr_blake3	stdout_evidence_kind	stdout_evidence_blake3	stderr_evidence_kind	stderr_evidence_blake3	lifecycle' >> "$results"

blake3() {
  b3sum --no-names "$1"
}

fail() {
  printf 'behavior matrix: %s\n' "$*" >&2
  exit 1
}

input_digests="$out/input-digests.v1.tsv"
printf '%s\n' '# schema: CircuitInputDigestV1/tsv-v1' > "$input_digests"
printf '%s\n' 'input_kind	blake3' >> "$input_digests"
record_input_digest() {
  printf '%s\t%s\n' "$1" "$(blake3 "$2")" >> "$input_digests"
}

git -C "$xsh_root" rev-parse HEAD > "$out/xsh-source-head.txt"
git -C "$xsh_root" status --porcelain=v1 > "$out/xsh-source-status.txt"
record_input_digest xsh_binary "$xsh"
record_input_digest xsht_binary "$xsht"
record_input_digest behavior_evaluator "$0"

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

assert_absent() {
  [ ! -e "$1" ] || fail "expected absent path: $1"
}

run_fixture() {
  case_id=$1
  fixture=$2
  shift 2
  parent_stdout="$artifacts/$case_id.parent.stdout"
  parent_stderr="$artifacts/$case_id.parent.stderr"
  set +e
  "$xsh" "$fixture" "$@" > "$parent_stdout" 2> "$parent_stderr"
  status=$?
  set -e
  printf '%s\n' "$status" > "$artifacts/$case_id.exit-status"
  [ "$status" -eq 0 ] || fail "$case_id fixture failed with status $status; inspect $parent_stderr"
}

record() {
  case_id=$1
  consumer=$2
  input_manifest=$3
  expected_contract_source=$4
  disposition=$5
  exit_shape=$6
  parent_stdout=$7
  parent_stderr=$8
  stdout_kind=$9
  stdout_digest=${10}
  stderr_kind=${11}
  stderr_digest=${12}
  lifecycle=${13}
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$case_id" "$consumer" "$input_manifest" "$expected_contract_source" "$disposition" "$exit_shape" \
    "$(blake3 "$parent_stdout")" "$(blake3 "$parent_stderr")" \
    "$stdout_kind" "$stdout_digest" "$stderr_kind" "$stderr_digest" \
    "$lifecycle" >> "$results"
}

child="$fixtures_dir/noisy-child.xsh"
sleeper="$fixtures_dir/noisy-sleeper.xsh"
nonzero_child="$fixtures_dir/noisy-child-nonzero.xsh"
record_input_digest fixture_noisy_child "$child"
record_input_digest fixture_noisy_sleeper "$sleeper"
record_input_digest fixture_noisy_child_nonzero "$nonzero_child"
record_input_digest fixture_probe_process_run "$fixtures_dir/probe-process-run.xsh"
record_input_digest fixture_probe_owned_spawn "$fixtures_dir/probe-owned-spawn.xsh"
record_input_digest fixture_probe_spawn_run "$fixtures_dir/probe-spawn-run.xsh"
record_input_digest fixture_probe_detached_spawn "$fixtures_dir/probe-detached-spawn.xsh"
record_input_digest fixture_probe_owned_default "$fixtures_dir/probe-owned-default.xsh"
record_input_digest fixture_probe_owned_invalid_stderr "$fixtures_dir/probe-owned-invalid-stderr.xsh"
record_input_digest fixture_probe_owned_nonzero "$fixtures_dir/probe-owned-nonzero.xsh"
record_input_digest fixture_probe_owned_cancel "$fixtures_dir/probe-owned-cancel.xsh"
record_input_digest fixture_probe_process_run_null "$fixtures_dir/probe-process-run-null.xsh"

# B01: process.run consumes both typed stream fields.
b01_marker="$artifacts/B01.marker"
b01_stdout="$artifacts/B01.redirected.stdout"
b01_stderr="$artifacts/B01.redirected.stderr"
run_fixture B01 "$fixtures_dir/probe-process-run.xsh" "$xsh" "$child" b01 "$b01_marker" "$b01_stdout" "$b01_stderr" false
assert_exact 'process-run:waited
' "$artifacts/B01.parent.stdout"
assert_exact '' "$artifacts/B01.parent.stderr"
assert_exact 'stdout:b01
' "$b01_stdout"
assert_exact 'stderr:b01
' "$b01_stderr"
assert_exact 'complete:b01' "$b01_marker"
record B01 process_run command_path_redirection spec_command_plan_stdio pass exited_0 "$artifacts/B01.parent.stdout" "$artifacts/B01.parent.stderr" redirected "$(blake3 "$b01_stdout")" redirected "$(blake3 "$b01_stderr")" completed_status

# B02: the primary managed form must preserve both redirection and wait.
b02_marker="$artifacts/B02.marker"
b02_stdout="$artifacts/B02.redirected.stdout"
b02_stderr="$artifacts/B02.redirected.stderr"
run_fixture B02 "$fixtures_dir/probe-owned-spawn.xsh" "$xsh" "$child" b02 "$b02_marker" "$b02_stdout" "$b02_stderr"
assert_exact 'owned-spawn:waited
' "$artifacts/B02.parent.stdout"
assert_exact '' "$artifacts/B02.parent.stderr"
assert_exact 'stdout:b02
' "$b02_stdout"
assert_exact 'stderr:b02
' "$b02_stderr"
assert_exact 'complete:b02' "$b02_marker"
record B02 spawn_command owned_command_path_redirection spec_spawn_command_redirection pass exited_0 "$artifacts/B02.parent.stdout" "$artifacts/B02.parent.stderr" redirected "$(blake3 "$b02_stdout")" redirected "$(blake3 "$b02_stderr")" owned_waited

# B03: syntax-level spawn run must preserve independent direct redirections.
b03_marker="$artifacts/B03.marker"
b03_stdout="$artifacts/B03.redirected.stdout"
b03_stderr="$artifacts/B03.redirected.stderr"
run_fixture B03 "$fixtures_dir/probe-spawn-run.xsh" "$xsh" "$child" b03 "$b03_marker" "$b03_stdout" "$b03_stderr"
assert_exact 'spawn-run:waited
' "$artifacts/B03.parent.stdout"
assert_exact '' "$artifacts/B03.parent.stderr"
assert_exact 'stdout:b03
' "$b03_stdout"
assert_exact 'stderr:b03
' "$b03_stderr"
assert_exact 'complete:b03' "$b03_marker"
record B03 spawn_run direct_spawn_redirection spec_spawn_run_redirection pass exited_0 "$artifacts/B03.parent.stdout" "$artifacts/B03.parent.stderr" redirected "$(blake3 "$b03_stdout")" redirected "$(blake3 "$b03_stderr")" owned_waited

# B04: detached process.spawn is measured, not forced into owned-handle parity.
b04_marker="$artifacts/B04.marker"
b04_stdout="$artifacts/B04.redirected.stdout"
b04_stderr="$artifacts/B04.redirected.stderr"
printf 'stdout-sentinel\n' > "$b04_stdout"
printf 'stderr-sentinel\n' > "$b04_stderr"
run_fixture B04 "$fixtures_dir/probe-detached-spawn.xsh" "$xsh" "$child" b04 "$b04_marker" "$b04_stdout" "$b04_stderr"
assert_exact 'detached-spawn:started
' "$artifacts/B04.parent.stdout"
assert_exact '' "$artifacts/B04.parent.stderr"
assert_exact 'stdout-sentinel
' "$b04_stdout"
assert_exact 'stderr-sentinel
' "$b04_stderr"
assert_exact 'complete:b04' "$b04_marker"
record B04 process_spawn detached_command_path_redirection spec_process_spawn_detached not_applicable detached_started "$artifacts/B04.parent.stdout" "$artifacts/B04.parent.stderr" redirection_ignored "$(blake3 "$b04_stdout")" redirection_ignored "$(blake3 "$b04_stderr")" detached_record_no_wait

# B05: absent stderr policy always inherits child stderr to the parent.
b05_marker="$artifacts/B05.marker"
run_fixture B05 "$fixtures_dir/probe-owned-default.xsh" "$xsh" "$child" b05 "$b05_marker"
assert_exact 'stdout:b05
owned-default:waited
' "$artifacts/B05.parent.stdout"
assert_exact 'complete:b05' "$b05_marker"
assert_exact 'stderr:b05
' "$artifacts/B05.parent.stderr"
record B05 spawn_command owned_command_default_stdio spec_spawn_default_inherit pass exited_0 "$artifacts/B05.parent.stdout" "$artifacts/B05.parent.stderr" inherited_parent_stdout - inherited_parent_stderr - default_inherit_waited

# B06 and B07 distinguish truncate from append without changing stdout policy.
b06_marker="$artifacts/B06.marker"
b06_stdout="$artifacts/B06.redirected.stdout"
b06_stderr="$artifacts/B06.redirected.stderr"
printf 'old-stderr\n' > "$b06_stderr"
run_fixture B06 "$fixtures_dir/probe-process-run.xsh" "$xsh" "$child" b06 "$b06_marker" "$b06_stdout" "$b06_stderr" false
assert_exact 'stdout:b06
' "$b06_stdout"
assert_exact 'stderr:b06
' "$b06_stderr"
assert_exact 'process-run:waited
' "$artifacts/B06.parent.stdout"
assert_exact '' "$artifacts/B06.parent.stderr"
assert_exact 'complete:b06' "$b06_marker"
record B06 process_run command_stderr_truncate spec_command_stderr_truncate pass exited_0 "$artifacts/B06.parent.stdout" "$artifacts/B06.parent.stderr" redirected "$(blake3 "$b06_stdout")" redirected "$(blake3 "$b06_stderr")" completed_status

b07_marker="$artifacts/B07.marker"
b07_stdout="$artifacts/B07.redirected.stdout"
b07_stderr="$artifacts/B07.redirected.stderr"
printf 'old-stderr\n' > "$b07_stderr"
run_fixture B07 "$fixtures_dir/probe-process-run.xsh" "$xsh" "$child" b07 "$b07_marker" "$b07_stdout" "$b07_stderr" true
assert_exact 'stdout:b07
' "$b07_stdout"
assert_exact 'old-stderr
stderr:b07
' "$b07_stderr"
assert_exact 'process-run:waited
' "$artifacts/B07.parent.stdout"
assert_exact '' "$artifacts/B07.parent.stderr"
assert_exact 'complete:b07' "$b07_marker"
record B07 process_run command_stderr_append spec_command_stderr_append pass exited_0 "$artifacts/B07.parent.stdout" "$artifacts/B07.parent.stderr" redirected "$(blake3 "$b07_stdout")" redirected "$(blake3 "$b07_stderr")" completed_status

# B08: redirection setup fails before a child can publish its completion marker.
b08_marker="$artifacts/B08.marker"
b08_invalid="$artifacts/B08.missing-parent/stderr.log"
run_fixture B08 "$fixtures_dir/probe-owned-invalid-stderr.xsh" "$xsh" "$child" b08 "$b08_marker" "$b08_invalid"
assert_exact 'owned-invalid-stderr:setup-error
' "$artifacts/B08.parent.stdout"
assert_exact '' "$artifacts/B08.parent.stderr"
assert_absent "$b08_marker"
assert_absent "$b08_invalid"
record B08 spawn_command owned_invalid_stderr_destination spec_process_setup_error pass setup_error "$artifacts/B08.parent.stdout" "$artifacts/B08.parent.stderr" not_produced - not_produced - setup_failed_before_handle

# B09: nonzero exit remains status data and leaves redirected stderr intact.
b09_stderr="$artifacts/B09.redirected.stderr"
run_fixture B09 "$fixtures_dir/probe-owned-nonzero.xsh" "$xsh" "$nonzero_child" "$b09_stderr"
assert_exact 'nonzero-stdout
owned-nonzero:status-23
' "$artifacts/B09.parent.stdout"
assert_exact '' "$artifacts/B09.parent.stderr"
assert_exact 'nonzero-stderr
' "$b09_stderr"
record B09 spawn_command owned_nonzero_status spec_spawn_status_data pass exited_23 "$artifacts/B09.parent.stdout" "$artifacts/B09.parent.stderr" inherited_parent_stdout - redirected "$(blake3 "$b09_stderr")" owned_waited_nonzero_status

# B10: the owned handle's cancellation path must prevent delayed completion.
b10_ready="$artifacts/B10.ready"
b10_completed="$artifacts/B10.completed"
b10_stdout="$artifacts/B10.redirected.stdout"
b10_stderr="$artifacts/B10.redirected.stderr"
run_fixture B10 "$fixtures_dir/probe-owned-cancel.xsh" "$xsh" "$sleeper" b10 "$b10_ready" "$b10_completed" "$b10_stdout" "$b10_stderr"
assert_exact 'owned-cancel:returned
' "$artifacts/B10.parent.stdout"
assert_exact '' "$artifacts/B10.parent.stderr"
assert_exact 'ready:b10' "$b10_ready"
assert_absent "$b10_completed"
assert_exact 'stdout:b10
' "$b10_stdout"
assert_exact 'stderr:b10
' "$b10_stderr"
record B10 spawn_command owned_cancelled_sleeper spec_os_owned_cancellation pass cancelled "$artifacts/B10.parent.stdout" "$artifacts/B10.parent.stderr" redirected "$(blake3 "$b10_stdout")" redirected "$(blake3 "$b10_stderr")" cancel_returned_no_delayed_effect

# B11: /dev/null is an ordinary typed Path sink for stderr.
b11_marker="$artifacts/B11.marker"
b11_stdout="$artifacts/B11.redirected.stdout"
run_fixture B11 "$fixtures_dir/probe-process-run-null.xsh" "$xsh" "$child" b11 "$b11_marker" "$b11_stdout"
assert_exact 'process-run-null:waited
' "$artifacts/B11.parent.stdout"
assert_exact '' "$artifacts/B11.parent.stderr"
assert_exact 'stdout:b11
' "$b11_stdout"
assert_exact 'complete:b11' "$b11_marker"
record B11 process_run command_stderr_dev_null spec_command_path_sink pass exited_0 "$artifacts/B11.parent.stdout" "$artifacts/B11.parent.stderr" redirected "$(blake3 "$b11_stdout")" redirected_dev_null - completed_status

printf 'behavior matrix passed; results: %s\n' "$results"
