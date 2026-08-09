#!/bin/sh
# Validate one closed C1 curation candidate against its fixed disclosure frontier.
#
# This is a provider-free parser/judge preview. Rust must replace these TSV
# fixtures with `CuratedAccountV1` and normalized child rows before admission;
# passing here never accepts an account or grants Grand Architect authority.
set -eu

usage() {
  printf '%s\n' "usage: $0 --account-dir ABSOLUTE_ACCOUNT_DIRECTORY --frontier-members ABSOLUTE_FRONTIER_MEMBERS_TSV --out EMPTY_OUTPUT_DIRECTORY" >&2
  exit 64
}

account_dir=''
frontier_members=''
out=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --account-dir) [ "$#" -ge 2 ] || usage; account_dir=$2; shift 2 ;;
    --frontier-members) [ "$#" -ge 2 ] || usage; frontier_members=$2; shift 2 ;;
    --out) [ "$#" -ge 2 ] || usage; out=$2; shift 2 ;;
    *) usage ;;
  esac
done
[ -n "$account_dir" ] && [ -n "$frontier_members" ] && [ -n "$out" ] || usage
[ -d "$account_dir" ] && [ -f "$frontier_members" ] || {
  printf '%s\n' 'curation contract: account or frontier input is unavailable' >&2
  exit 66
}
case "$out" in
  /*) ;;
  *) printf '%s\n' 'curation contract: --out must be absolute' >&2; exit 64 ;;
esac
for required_tool in awk find mkdir sed sort; do
  command -v "$required_tool" >/dev/null 2>&1 || {
    printf 'curation contract: missing required host tool: %s\n' "$required_tool" >&2
    exit 69
  }
done
if ! command -v b3sum >/dev/null 2>&1; then
  printf '%s\n' 'missing required host digest tool: b3sum' >&2
  exit 69
fi
if [ -e "$out" ]; then
  [ -d "$out" ] && [ -z "$(find "$out" -mindepth 1 -maxdepth 1 -print -quit)" ] || {
    printf 'curation contract: output directory must be empty: %s\n' "$out" >&2
    exit 73
  }
else
  mkdir -p "$out"
fi

tab=$(printf '\t')
fail() {
  printf 'curation contract: %s\n' "$*" >&2
  exit 1
}
blake3() {
  b3sum --no-names "$1"
}
assert_header() {
  table=$1
  expected=$2
  [ "$(sed -n '1p' "$table")" = "$expected" ] || fail "unexpected schema header: $table"
}
row_count() {
  awk 'END { print NR - 1 }' "$1"
}
assert_exact_rows() {
  table=$1
  expected=$2
  relation=$3
  actual=$(row_count "$table")
  if [ "$actual" -ne "$expected" ]; then
    case "$expected" in
      1) row_word=row ;;
      *) row_word=rows ;;
    esac
    fail "$relation must contain exactly $expected $row_word"
  fi
}

account="$account_dir/account.v1.tsv"
selected="$account_dir/selected-items.v1.tsv"
conflicts="$account_dir/preserved-conflicts.v1.tsv"
unknowns="$account_dir/decision-relevant-unknowns.v1.tsv"
exclusions="$account_dir/exclusions.v1.tsv"
escalations="$account_dir/raw-evidence-escalations.v1.tsv"
for input in "$account" "$selected" "$conflicts" "$unknowns" "$exclusions" "$escalations"; do
  [ -f "$input" ] || fail "missing required account relation: $input"
done
expected_files=$(printf '%s\n' account.v1.tsv decision-relevant-unknowns.v1.tsv exclusions.v1.tsv preserved-conflicts.v1.tsv raw-evidence-escalations.v1.tsv selected-items.v1.tsv)
actual_files=$(find "$account_dir" -maxdepth 1 -type f -print | sed "s|$account_dir/||" | sort)
[ "$actual_files" = "$expected_files" ] || fail 'account contains an unrecognized relation'

input_digests="$out/input-digests.v1.tsv"
printf '%s\n' '# schema: CurationContractInputDigestV1/tsv-v1' > "$input_digests"
printf 'input_kind\tblake3\n' >> "$input_digests"
record_input_digest() {
  printf '%s\t%s\n' "$1" "$(blake3 "$2")" >> "$input_digests"
}
record_input_digest curation_contract_judge "$0"
record_input_digest frontier_members "$frontier_members"
record_input_digest account_relation "$account"
record_input_digest selected_items_relation "$selected"
record_input_digest conflicts_relation "$conflicts"
record_input_digest unknowns_relation "$unknowns"
record_input_digest exclusions_relation "$exclusions"
record_input_digest escalations_relation "$escalations"

# The C1 fixture frontier is a closed seven-member allowlist. The identities
# themselves are checked so an equally sized but semantically different list
# cannot enter the curation contract by accident.
assert_header "$frontier_members" 'source_ref'
assert_exact_rows "$frontier_members" 7 'frontier member relation'
set +e
awk -F '\t' '
  BEGIN {
    split("argument_h1 argument_h2 argument_h3 observation_managed_behavior observation_detached_behavior observation_documentation_conflict conflict_lang_vs_spec", required, " ")
    for (ordinal = 1; ordinal <= 7; ordinal += 1) required_ref[required[ordinal]] = 1
  }
  NR > 1 {
    if (NF != 1 || $1 == "") { error = 1; exit }
    if (seen[$1]++) { error = 2; exit }
    if (!required_ref[$1]) { error = 1; exit }
  }
  END {
    if (error) exit error
    for (ordinal = 1; ordinal <= 7; ordinal += 1) if (!seen[required[ordinal]]) exit 3
  }
' "$frontier_members"
frontier_exit=$?
set -e
case "$frontier_exit" in
  0) ;;
  2) fail 'duplicate frontier source_ref' ;;
  3) fail 'frontier is missing a required source_ref' ;;
  *) fail 'invalid frontier member relation' ;;
esac

assert_header "$account" "kind${tab}purpose${tab}question_revision${tab}disclosure_frontier${tab}curator_configuration${tab}leading_hypothesis${tab}strongest_counterevidence_ref"
assert_exact_rows "$account" 1 'account relation'
awk -F '\t' 'NR == 2 && NF == 7 && $1 == "decision_curation" && $2 == "authorize_spawn_stderr_prototype" && $3 == "q_vs001_spawn_stderr_r1" && $4 == "fc1" && $5 == "curator_v1" && $6 == "h2" && $7 == "argument_h3" { ok = 1 } END { exit(ok ? 0 : 1) }' "$account" || fail 'invalid C1 account identity or counterevidence declaration'

assert_header "$selected" "ordinal${tab}source_ref${tab}role${tab}selection_reason${tab}applicability_scope"
assert_exact_rows "$selected" 6 'selected-item relation'
set +e
awk -F '\t' -v frontier="$frontier_members" '
  BEGIN {
    getline < frontier
    while ((getline line < frontier) > 0) {
      split(line, fields, "\t")
      allowed[fields[1]] = 1
    }
    expected_role["argument_h1"] = "defeating_argument"
    expected_reason["argument_h1"] = "h1_rejected"
    expected_role["argument_h2"] = "supporting_argument"
    expected_reason["argument_h2"] = "h2_partially_supported"
    expected_role["argument_h3"] = "dissent"
    expected_reason["argument_h3"] = "h3_supported"
    expected_role["observation_managed_behavior"] = "observation"
    expected_reason["observation_managed_behavior"] = "direct_owned_path_result"
    expected_role["observation_detached_behavior"] = "dissent"
    expected_reason["observation_detached_behavior"] = "detached_policy_distinction"
    expected_role["observation_documentation_conflict"] = "constraint"
    expected_reason["observation_documentation_conflict"] = "stale_discovery_conflict"
  }
  NR > 1 {
    if (NF != 5 || $1 != NR - 1 || $5 != "spawn_stderr_prototype") { error = 1; exit }
    if (seen[$2]++) { error = 2; exit }
    if (!allowed[$2]) { error = 3; exit }
    if ($3 != expected_role[$2] || $4 != expected_reason[$2]) { error = 1; exit }
  }
  END {
    if (error) exit error
    for (source_ref in expected_role) if (!seen[source_ref]) exit 4
  }
' "$selected"
selected_exit=$?
set -e
case "$selected_exit" in
  0) ;;
  2) fail 'duplicate selected source_ref' ;;
  3) fail 'selection source outside frontier' ;;
  4) fail 'account does not reconstruct exact H1/H2/H3 evidence set' ;;
  *) fail 'invalid selected-item relation' ;;
esac

assert_header "$conflicts" 'conflict_ref'
assert_exact_rows "$conflicts" 1 'preserved-conflicts relation'
awk -F '\t' 'NR == 2 && NF == 1 && $1 == "conflict_lang_vs_spec" { ok = 1 } END { exit(ok ? 0 : 1) }' "$conflicts" || fail 'invalid preserved conflict relation'

assert_header "$unknowns" 'unknown_ref'
assert_exact_rows "$unknowns" 1 'decision-relevant-unknown relation'
awk -F '\t' 'NR == 2 && NF == 1 && $1 == "detached_public_contract_intent" { ok = 1 } END { exit(ok ? 0 : 1) }' "$unknowns" || fail 'invalid decision-relevant unknown relation'

assert_header "$exclusions" "category_or_source${tab}reason${tab}risk_if_wrong"
assert_exact_rows "$exclusions" 2 'exclusions relation'
set +e
awk -F '\t' '
  BEGIN {
    expected_reason["raw_pi_session"] = "no_named_question"
    expected_risk["raw_pi_session"] = "decision_context_may_miss_relevant_reasoning"
    expected_reason["candidate_patch"] = "post_frontier_material"
    expected_risk["candidate_patch"] = "would_leak_future_product_choice"
  }
  NR > 1 {
    if (NF != 3 || $1 == "") { error = 1; exit }
    if (seen[$1]++) { error = 2; exit }
    if ($2 != expected_reason[$1] || $3 != expected_risk[$1]) { error = 1; exit }
  }
  END {
    if (error) exit error
    if (!seen["raw_pi_session"] || !seen["candidate_patch"]) exit 3
  }
' "$exclusions"
exclusions_exit=$?
set -e
case "$exclusions_exit" in
  0) ;;
  2) fail 'duplicate exclusion category_or_source' ;;
  3) fail 'exclusions omit a required semantic category' ;;
  *) fail 'invalid semantic exclusion relation' ;;
esac

assert_header "$escalations" "question_ref${tab}object_ref"
escalation_count=$(row_count "$escalations")
[ "$escalation_count" -le 1 ] || fail 'raw-evidence-escalations relation permits at most one row in C1'
case "$escalation_count" in
  0) raw_escalation_state=none_requested ;;
  1)
    awk -F '\t' 'NR == 2 && NF == 2 && $1 == "resolve_detached_contract_intent" && $2 == "raw_pi_session_object" { ok = 1 } END { exit(ok ? 0 : 1) }' "$escalations" || fail 'raw evidence escalation lacks a valid named question and object'
    raw_escalation_state=named_request_present
    ;;
  *) fail 'invalid raw-evidence-escalations relation' ;;
esac

raw_results="$out/curation-raw-evidence-escalations.v1.tsv"
printf '%s\n' '# schema: CurationRawEvidenceEscalationObservationV1/tsv-v1' > "$raw_results"
printf 'ordinal\tquestion_ref\tobject_ref\n' >> "$raw_results"
if [ "$escalation_count" -eq 1 ]; then
  awk -F '\t' 'NR == 2 { printf "1\t%s\t%s\n", $1, $2 }' "$escalations" >> "$raw_results"
fi

results="$out/curation-contract-observation.v1.tsv"
printf '%s\n' '# schema: CurationContractObservationV1/tsv-v1' > "$results"
printf 'account_kind\tpurpose\thypothesis_coverage\tcounterevidence\tpreserved_conflict\tunknowns\texclusions\traw_escalations\tfrontier_admission\tdisposition\n' >> "$results"
printf 'decision_curation\tauthorize_spawn_stderr_prototype\th1_h2_h3_with_dissent\tdeclared\tpreserved\tdeclared\tsemantic\t%s\tfrontier_constrained\tacceptance_ready\n' "$raw_escalation_state" >> "$results"
printf 'curation contract judge passed; results: %s\n' "$results"
