#!/bin/sh
# Produce cited VS-001 documentation and discovery observations.
#
# `baseline-conflict` records the current contradictory world. `candidate-
# reconciled` is a stricter gate for a product candidate: it fails if its inputs
# retain a named contradiction. Neither mode turns prose into a quality score.
set -eu

usage() {
  printf '%s\n' "usage: $0 --xsh-root ABSOLUTE_XSH_SOURCE --xsht ABSOLUTE_XSHT --out EMPTY_OUTPUT_DIRECTORY [--mode baseline-conflict|candidate-reconciled]" >&2
  exit 64
}

xsh_root=''
xsht=''
out=''
mode='baseline-conflict'
while [ "$#" -gt 0 ]; do
  case "$1" in
    --xsh-root)
      [ "$#" -ge 2 ] || usage
      xsh_root=$2
      shift 2
      ;;
    --xsht)
      [ "$#" -ge 2 ] || usage
      xsht=$2
      shift 2
      ;;
    --out)
      [ "$#" -ge 2 ] || usage
      out=$2
      shift 2
      ;;
    --mode)
      [ "$#" -ge 2 ] || usage
      mode=$2
      shift 2
      ;;
    *) usage ;;
  esac
done

[ -n "$xsh_root" ] && [ -n "$xsht" ] && [ -n "$out" ] || usage
[ -d "$xsh_root/.git" ] || {
  printf 'not an XSH checkout: %s\n' "$xsh_root" >&2
  exit 66
}
[ -x "$xsht" ] || {
  printf 'xsht binary is not executable: %s\n' "$xsht" >&2
  exit 66
}
case "$mode" in
  baseline-conflict|candidate-reconciled) ;;
  *)
    printf 'unsupported documentation evaluator mode: %s\n' "$mode" >&2
    exit 64
    ;;
esac
for required_tool in awk cut find git rg sed; do
  command -v "$required_tool" >/dev/null 2>&1 || {
    printf 'missing required host tool: %s\n' "$required_tool" >&2
    exit 69
  }
done
if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
  printf '%s\n' 'missing required host digest tool: sha256sum or shasum' >&2
  exit 69
fi
[ -z "$(git -C "$xsh_root" status --porcelain=v1)" ] || {
  printf 'refusing dirty XSH source checkout: %s\n' "$xsh_root" >&2
  exit 70
}

lang="$xsh_root/LANG.md"
spec="$xsh_root/docs/SPEC.md"
spec_os="$xsh_root/docs/SPEC-OS.md"
runtime="$xsh_root/src/runtime/process.rs"
lowered="$xsh_root/src/runtime/eval/lowered_run.rs"
native_test="$xsh_root/tests/xsh/stdlib/process.xsh"
for input in "$lang" "$spec" "$spec_os" "$runtime" "$lowered" "$native_test"; do
  [ -f "$input" ] || {
    printf 'missing documentation evaluator input: %s\n' "$input" >&2
    exit 66
  }
done

case "$out" in
  /*) ;;
  *) printf '%s\n' '--out must be an absolute path' >&2; exit 64 ;;
esac
if [ -e "$out" ]; then
  [ -d "$out" ] && [ -z "$(find "$out" -mindepth 1 -maxdepth 1 -print -quit)" ] || {
    printf 'output directory must exist as an empty directory: %s\n' "$out" >&2
    exit 73
  }
else
  mkdir -p "$out"
fi

fail() {
  printf 'documentation matrix: %s\n' "$*" >&2
  exit 1
}

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

line_of() {
  pattern=$1
  path=$2
  line=$(rg -n -m 1 -- "$pattern" "$path" | cut -d: -f1 || true)
  [ -n "$line" ] || fail "missing '$pattern' in $path"
  printf '%s' "$line"
}

results="$out/documentation-observations.v1.tsv"
printf '%s\n' '# schema: DocumentationObservationV1/tsv-v1' > "$results"
printf '%s\n' 'source	consumer	field	claim	citation' >> "$results"
conflicts="$out/documentation-conflicts.v1.tsv"
printf '%s\n' '# schema: DocumentationConflictV1/tsv-v1' > "$conflicts"
printf '%s\n' 'conflict_id	left_claim	right_claim	status' >> "$conflicts"
input_digests="$out/input-digests.v1.tsv"
printf '%s\n' '# schema: CircuitInputDigestV1/tsv-v1' > "$input_digests"
printf '%s\n' 'input_kind	sha256' >> "$input_digests"
record_input_digest() {
  printf '%s\t%s\n' "$1" "$(sha256 "$2")" >> "$input_digests"
}

git -C "$xsh_root" rev-parse HEAD > "$out/xsh-source-head.txt"
git -C "$xsh_root" status --porcelain=v1 > "$out/xsh-source-status.txt"
record_input_digest xsht_binary "$xsht"
record_input_digest documentation_evaluator "$0"
record_input_digest lang_source "$lang"
record_input_digest spec_source "$spec"
record_input_digest spec_os_source "$spec_os"
record_input_digest runtime_process_source "$runtime"
record_input_digest lowered_runtime_source "$lowered"
record_input_digest native_process_test "$native_test"

api_command="$out/api-process-command-argv.txt"
api_spawn="$out/api-process-spawn.txt"
api_navigation="$out/api-search-process.txt"
"$xsht" api api:process.command_argv --strict --details full > "$api_command"
"$xsht" api api:process.spawn --strict --details full > "$api_spawn"
"$xsht" api search:process --strict --details basic > "$api_navigation"
record_input_digest api_process_command_argv "$api_command"
record_input_digest api_process_spawn "$api_spawn"
record_input_digest api_process_navigation "$api_navigation"

lang_stale_line=$(rg -n -m 1 'has no way to redirect' "$lang" | cut -d: -f1 || true)
if [ -n "$lang_stale_line" ]; then
  printf 'LANG_md\tspawn_command\tstderr\tclaims_missing\t%s:%s\n' "$(basename "$lang")" "$lang_stale_line" >> "$results"
else
  printf 'LANG_md\tspawn_command\tstderr\tdoes_not_claim_missing\t%s\n' "$(basename "$lang")" >> "$results"
fi

spec_spawn_line=$(line_of 'redirection behavior used by' "$spec")
printf 'SPEC_spawn\tspawn_command\tstderr\tclaims_uses_command_redirections\t%s:%s\n' "$(basename "$spec")" "$spec_spawn_line" >> "$results"
spec_default_line=$(line_of 'inherits stdio by default' "$spec")
printf 'SPEC_spawn\tspawn_command\tdefault\tclaims_inherit_default\t%s:%s\n' "$(basename "$spec")" "$spec_default_line" >> "$results"
spec_field_line=$(line_of 'stderr: Path = default' "$spec")
printf 'SPEC_api\tcommand_plan\tstderr\tclaims_typed_path_field\t%s:%s\n' "$(basename "$spec")" "$spec_field_line" >> "$results"
spec_append_line=$(line_of 'stderr_append: Bool' "$spec")
printf 'SPEC_api\tcommand_plan\tstderr_append\tclaims_typed_append_field\t%s:%s\n' "$(basename "$spec")" "$spec_append_line" >> "$results"
spec_error_line=$(line_of 'setup failures' "$spec")
printf 'SPEC_spawn\tspawn_command\terror\tclaims_setup_failure_is_process_error\t%s:%s\n' "$(basename "$spec")" "$spec_error_line" >> "$results"
spec_detached_line=$(line_of 'detached-record API' "$spec")
printf 'SPEC_spawn\tprocess_spawn\tlifecycle\tclaims_detached_record\t%s:%s\n' "$(basename "$spec")" "$spec_detached_line" >> "$results"

spec_os_ownership_line=$(line_of 'handle owns one child group' "$spec_os")
printf 'SPEC_OS\tspawn_command\townership\tclaims_owned_child_group\t%s:%s\n' "$(basename "$spec_os")" "$spec_os_ownership_line" >> "$results"
spec_os_error_line=$(line_of 'redirection failures' "$spec_os")
printf 'SPEC_OS\tcommand_plan\terror\tclaims_redirection_failure_distinct_from_status\t%s:%s\n' "$(basename "$spec_os")" "$spec_os_error_line" >> "$results"

api_field_line=$(line_of 'stderr: Path' "$api_command")
printf 'xsht_api\tcommand_plan\tstderr\tdiscoverable_typed_path_field\tapi-process-command-argv.txt:%s\n' "$api_field_line" >> "$results"
api_append_line=$(line_of 'stderr_append: Bool' "$api_command")
printf 'xsht_api\tcommand_plan\tstderr_append\tdiscoverable_typed_append_field\tapi-process-command-argv.txt:%s\n' "$api_append_line" >> "$results"
api_spawn_owned_line=$(rg -n -m 1 'owned process handle record' "$api_spawn" | cut -d: -f1 || true)
api_spawn_detached_line=$(rg -n -m 1 'detached' "$api_spawn" | cut -d: -f1 || true)
if [ -n "$api_spawn_owned_line" ]; then
  printf 'xsht_api\tprocess_spawn\tlifecycle\tclaims_owned_handle\tapi-process-spawn.txt:%s\n' "$api_spawn_owned_line" >> "$results"
elif [ -n "$api_spawn_detached_line" ]; then
  printf 'xsht_api\tprocess_spawn\tlifecycle\tclaims_detached_record\tapi-process-spawn.txt:%s\n' "$api_spawn_detached_line" >> "$results"
else
  printf 'xsht_api\tprocess_spawn\tlifecycle\tdoes_not_disclose_lifecycle\tapi-process-spawn.txt\n' >> "$results"
fi
navigation_command_line=$(line_of 'module.process.command_argv' "$api_navigation")
printf 'xsht_navigation\tcommand_plan\tdiscovery\tfinds_command_argv\tapi-search-process.txt:%s\n' "$navigation_command_line" >> "$results"
navigation_spawn_line=$(line_of 'module.process.spawn' "$api_navigation")
printf 'xsht_navigation\tprocess_spawn\tdiscovery\tfinds_process_spawn\tapi-search-process.txt:%s\n' "$navigation_spawn_line" >> "$results"

runtime_spawn_command_line=$(line_of '^pub fn spawn_command' "$runtime")
printf 'runtime\tprocess_spawn\tcall_path\tspawn_command_enters_detached_options\tsrc/runtime/process.rs:%s\n' "$runtime_spawn_command_line" >> "$results"
runtime_detached_line=$(line_of 'apply_redirections: false' "$runtime")
printf 'runtime\tprocess_spawn\tstderr\tdisables_command_redirections\tsrc/runtime/process.rs:%s\n' "$runtime_detached_line" >> "$results"
runtime_apply_line=$(line_of 'if options.apply_redirections' "$runtime")
printf 'runtime\tmanaged_spawn\tstderr\tconditionally_applies_command_redirections\tsrc/runtime/process.rs:%s\n' "$runtime_apply_line" >> "$results"
runtime_managed_line=$(line_of 'apply_redirections: true' "$runtime")
printf 'runtime\tmanaged_spawn\tstderr\tenables_command_redirections\tsrc/runtime/process.rs:%s\n' "$runtime_managed_line" >> "$results"
lowered_detached_line=$(line_of 'RuntimeOp::ProcessSpawn' "$lowered")
printf 'lowered_runtime\tprocess_spawn\tcall_path\tcalls_detached_spawn_command\tsrc/runtime/eval/lowered_run.rs:%s\n' "$lowered_detached_line" >> "$results"
lowered_managed_line=$(line_of 'managed_options.apply_redirections = true' "$lowered")
printf 'lowered_runtime\tspawn_command\tcall_path\tcreates_managed_redirection_path\tsrc/runtime/eval/lowered_run.rs:%s\n' "$lowered_managed_line" >> "$results"

test_start=$(line_of '^proc test_process_command_redirections' "$native_test")
test_end=$(awk -v start="$test_start" 'NR > start && /^}/ { print NR; exit }' "$native_test")
[ -n "$test_end" ] || fail 'could not delimit test_process_command_redirections'
test_block=$(sed -n "${test_start},${test_end}p" "$native_test")
printf '%s\n' "$test_block" | rg -q 'process\.run\(command\)' || fail 'focused test no longer covers process.run redirection'
if printf '%s\n' "$test_block" | rg -q 'spawn command'; then
  fail 'focused run-redirection test unexpectedly contains managed spawn coverage; update the matrix contract'
fi
printf 'native_test\tprocess_run\tstderr\tcovers_run_redirection\ttests/xsh/stdlib/process.xsh:%s-%s\n' "$test_start" "$test_end" >> "$results"
printf 'native_test\tspawn_command\tstderr\tno_managed_stderr_assertion_in_focused_test\ttests/xsh/stdlib/process.xsh:%s-%s\n' "$test_start" "$test_end" >> "$results"

if [ "$mode" = baseline-conflict ]; then
  if [ -n "$lang_stale_line" ]; then
    printf 'D01\tLANG_claims_missing\tSPEC_claims_supported\tpresent\n' >> "$conflicts"
  else
    printf 'D01\tLANG_claims_missing\tSPEC_claims_supported\tabsent\n' >> "$conflicts"
  fi
  if [ -n "$api_spawn_owned_line" ]; then
    printf 'D02\txsht_api_claims_owned_handle\tSPEC_claims_detached_record\tpresent\n' >> "$conflicts"
  else
    printf 'D02\txsht_api_claims_owned_handle\tSPEC_claims_detached_record\tabsent\n' >> "$conflicts"
  fi
  printf 'D03\tprocess_spawn_redirection_ignored\tmanaged_spawn_redirection_enabled\tintentional_semantic_split\n' >> "$conflicts"
  printf 'baseline-conflict documentation matrix passed; results: %s\n' "$results"
  exit 0
fi

candidate_error=''
if [ -n "$lang_stale_line" ]; then
  printf 'D01\tLANG_claims_missing\tSPEC_claims_supported\tpresent\n' >> "$conflicts"
  candidate_error='candidate retains the stale LANG missing-behavior claim'
else
  printf 'D01\tLANG_claims_missing\tSPEC_claims_supported\tresolved\n' >> "$conflicts"
fi
if [ -n "$api_spawn_owned_line" ]; then
  printf 'D02\txsht_api_claims_owned_handle\tSPEC_claims_detached_record\tpresent\n' >> "$conflicts"
  if [ -z "$candidate_error" ]; then
    candidate_error='candidate API still describes process.spawn as an owned handle'
  fi
elif [ -n "$api_spawn_detached_line" ]; then
  printf 'D02\txsht_api_claims_owned_handle\tSPEC_claims_detached_record\tresolved\n' >> "$conflicts"
else
  printf 'D02\txsht_api_claims_owned_handle\tSPEC_claims_detached_record\tpresent\n' >> "$conflicts"
  if [ -z "$candidate_error" ]; then
    candidate_error='candidate API does not disclose process.spawn detached lifecycle'
  fi
fi
printf 'D03\tprocess_spawn_redirection_ignored\tmanaged_spawn_redirection_enabled\tintentional_semantic_split\n' >> "$conflicts"
[ -z "$candidate_error" ] || fail "$candidate_error"
printf 'candidate-reconciled documentation matrix passed; results: %s\n' "$results"
