#!/bin/sh
# Check the distinct delivered, encountered, and applied_once propagation facts.
#
# The closed TSV relations map to future Rust `PropagationContextV1`,
# `InvestigatorSubmissionV1`, and `PropagationObservationV1` types. This judge
# cannot promote a lesson or establish causal propagation support.
set -eu

usage() {
  printf '%s\n' "usage: $0 --context ABSOLUTE_DELIVERY_CONTEXT_TSV --persisted-input ABSOLUTE_PERSISTED_INPUT_TSV --submission ABSOLUTE_INVESTIGATOR_SUBMISSION_TSV --accesses ABSOLUTE_ACCESS_TSV --out EMPTY_OUTPUT_DIRECTORY" >&2
  exit 64
}

context=''
persisted_input=''
submission=''
accesses=''
out=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --context) [ "$#" -ge 2 ] || usage; context=$2; shift 2 ;;
    --persisted-input) [ "$#" -ge 2 ] || usage; persisted_input=$2; shift 2 ;;
    --submission) [ "$#" -ge 2 ] || usage; submission=$2; shift 2 ;;
    --accesses) [ "$#" -ge 2 ] || usage; accesses=$2; shift 2 ;;
    --out) [ "$#" -ge 2 ] || usage; out=$2; shift 2 ;;
    *) usage ;;
  esac
done
[ -n "$context" ] && [ -n "$persisted_input" ] && [ -n "$submission" ] && [ -n "$accesses" ] && [ -n "$out" ] || usage
for input in "$context" "$persisted_input" "$submission" "$accesses"; do
  [ -f "$input" ] || {
    printf 'uptake application: unavailable input: %s\n' "$input" >&2
    exit 66
  }
done
case "$out" in
  /*) ;;
  *) printf '%s\n' 'uptake application: --out must be absolute' >&2; exit 64 ;;
esac
for required_tool in awk find mkdir sed; do
  command -v "$required_tool" >/dev/null 2>&1 || {
    printf 'uptake application: missing required host tool: %s\n' "$required_tool" >&2
    exit 69
  }
done
if ! command -v b3sum >/dev/null 2>&1; then
  printf '%s\n' 'missing required host digest tool: b3sum' >&2
  exit 69
fi
if [ -e "$out" ]; then
  [ -d "$out" ] && [ -z "$(find "$out" -mindepth 1 -maxdepth 1 -print -quit)" ] || {
    printf 'uptake application: output directory must be empty: %s\n' "$out" >&2
    exit 73
  }
else
  mkdir -p "$out"
fi

tab=$(printf '\t')
fail() {
  printf 'uptake application: %s\n' "$*" >&2
  exit 1
}
blake3() {
  b3sum --no-names "$1"
}
assert_header_and_one_row() {
  table=$1
  expected=$2
  [ "$(sed -n '1p' "$table")" = "$expected" ] || fail "unexpected schema header: $table"
  [ "$(awk 'END { print NR - 1 }' "$table")" = 1 ] || fail "expected exactly one data row: $table"
}

results="$out/uptake-application-observation.v1.tsv"
printf '%s\n' '# schema: PropagationObservationV1/tsv-v1' > "$results"
printf 'target_revision\tlesson_revision\tdelivered\tencountered\tapplication\tcontamination\tdisposition\n' >> "$results"
write_result() {
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    t_stdout_capture_inquiry_r1 l1_spawn_stderr_method_r1 "$1" "$2" "$3" "$4" "$5" >> "$results"
}

input_digests="$out/input-digests.v1.tsv"
printf '%s\n' '# schema: UptakeApplicationInputDigestV1/tsv-v1' > "$input_digests"
printf 'input_kind\tblake3\n' >> "$input_digests"
record_input_digest() {
  printf '%s\t%s\n' "$1" "$(blake3 "$2")" >> "$input_digests"
}
record_input_digest uptake_application_judge "$0"
record_input_digest delivery_context "$context"
record_input_digest persisted_input "$persisted_input"
record_input_digest investigator_submission "$submission"
record_input_digest investigator_accesses "$accesses"

assert_header_and_one_row "$context" "target_revision${tab}lesson_revision${tab}lesson_status${tab}applicability_scope${tab}supporting_episode${tab}exclusions_ref"
awk -F '\t' 'NR == 2 && $1 == "t_stdout_capture_inquiry_r1" && $2 == "l1_spawn_stderr_method_r1" && $3 == "l1" && $4 == "stdout_capture_process_api" && $5 == "e_vs001_spawn_stderr_r1" && $6 == "x_vs001_c1_r1" { ok = 1 } END { exit(ok ? 0 : 1) }' "$context" || fail 'invalid L1 target delivery context'
assert_header_and_one_row "$persisted_input" "target_revision${tab}lesson_revision${tab}lesson_status${tab}applicability_scope"
awk -F '\t' 'NR == 2 && $1 == "t_stdout_capture_inquiry_r1" && $2 == "l1_spawn_stderr_method_r1" && $3 == "l1" && $4 == "stdout_capture_process_api" { ok = 1 } END { exit(ok ? 0 : 1) }' "$persisted_input" || fail 'persisted input does not carry the exact delivered lesson manifest'

assert_header_and_one_row "$submission" "lesson_revision${tab}recommendation${tab}normative_registry_state${tab}normative_registry_ref${tab}normative_registry_unavailable_reason${tab}executable_behavior_state${tab}executable_behavior_ref${tab}executable_behavior_unavailable_reason${tab}proposal_corpus_state${tab}proposal_corpus_ref${tab}proposal_corpus_unavailable_reason${tab}real_call_sites_state${tab}real_call_sites_ref${tab}real_call_sites_unavailable_reason"
awk -F '\t' 'NR == 2 && $1 == "l1_spawn_stderr_method_r1" && ($2 == "new_api" || $2 == "use_existing_contract" || $2 == "further_experiment" || $2 == "no_change") && NF == 14 { ok = 1 } END { exit(ok ? 0 : 1) }' "$submission" || fail 'invalid investigator submission or lesson encounter'

if [ "$(sed -n '1p' "$accesses")" != "ordinal${tab}access_class" ]; then
  fail 'unexpected investigator access schema'
fi
set +e
awk -F '\t' '
  NR > 1 {
    if ($1 != NR - 1 || NF != 2) exit 2
    if ($2 == "forbidden_vs001_session" || $2 == "post_target_material") exit 3
    if ($2 != "target_context") exit 2
  }
' "$accesses"
access_exit=$?
set -e
case "$access_exit" in
  0) ;;
  3)
    write_result delivered encountered not_applied contaminated contamination_recorded
    fail 'forbidden VS-001 session or post-target material access'
    ;;
  *) fail 'invalid investigator access relation' ;;
esac

set +e
awk -F '\t' '
  function record_ok(state, ref, reason) {
    if (state == "available") return ref != "-" && reason == "-"
    if (state == "unavailable") return ref == "-" && (reason == "source_not_admitted_at_frontier" || reason == "source_not_available" || reason == "not_applicable_to_question")
    return 0
  }
  NR == 2 {
    if (!record_ok($3, $4, $5)) exit 2
    if (!record_ok($6, $7, $8)) exit 2
    if (!record_ok($9, $10, $11)) exit 2
    if (!record_ok($12, $13, $14)) exit 2
  }
' "$submission"
application_exit=$?
set -e
if [ "$application_exit" -ne 0 ]; then
  write_result delivered encountered not_applied clean rejected_missing_record_class
  fail 'application lacks a required record class or named unavailability explanation'
fi

write_result delivered encountered applied_once clean pass
printf 'uptake application judge passed; results: %s\n' "$results"
