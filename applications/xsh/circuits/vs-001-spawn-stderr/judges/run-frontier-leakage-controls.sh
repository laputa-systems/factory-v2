#!/bin/sh
# Exercise a closed W1 frontier allowlist and aftermath-denial matrix.
#
# This provider-free judge previews typed Rust query authorization. Its TSV
# relations map to closed `DisclosureFrontierV1` parser types; they are not
# authority and cannot disclose an actual forensic object.
set -eu

usage() {
  printf '%s\n' "usage: $0 --frontier-dir ABSOLUTE_FRONTIER_DIRECTORY --out EMPTY_OUTPUT_DIRECTORY" >&2
  exit 64
}

frontier_dir=''
out=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --frontier-dir) [ "$#" -ge 2 ] || usage; frontier_dir=$2; shift 2 ;;
    --out) [ "$#" -ge 2 ] || usage; out=$2; shift 2 ;;
    *) usage ;;
  esac
done
[ -n "$frontier_dir" ] && [ -n "$out" ] || usage
[ -d "$frontier_dir" ] || {
  printf 'frontier leakage: not a frontier directory: %s\n' "$frontier_dir" >&2
  exit 66
}
case "$out" in
  /*) ;;
  *) printf '%s\n' 'frontier leakage: --out must be absolute' >&2; exit 64 ;;
esac
for required_tool in awk find mkdir sed sort; do
  command -v "$required_tool" >/dev/null 2>&1 || {
    printf 'frontier leakage: missing required host tool: %s\n' "$required_tool" >&2
    exit 69
  }
done
if ! command -v b3sum >/dev/null 2>&1; then
  printf '%s\n' 'missing required host digest tool: b3sum' >&2
  exit 69
fi
if [ -e "$out" ]; then
  [ -d "$out" ] && [ -z "$(find "$out" -mindepth 1 -maxdepth 1 -print -quit)" ] || {
    printf 'frontier leakage: output directory must be empty: %s\n' "$out" >&2
    exit 73
  }
else
  mkdir -p "$out"
fi

tab=$(printf '\t')
fail() {
  printf 'frontier leakage: %s\n' "$*" >&2
  exit 1
}
blake3() {
  b3sum --no-names "$1"
}
row_count() {
  awk 'END { print NR - 1 }' "$1"
}
assert_exact_rows() {
  table=$1
  expected=$2
  relation=$3
  actual=$(row_count "$table")
  [ "$actual" -eq "$expected" ] || fail "$relation must contain exactly $expected rows"
}

members="$frontier_dir/frontier-members.v1.tsv"
sequestered="$frontier_dir/sequestered.v1.tsv"
[ -f "$members" ] && [ -f "$sequestered" ] || fail 'missing frontier relation'
input_digests="$out/input-digests.v1.tsv"
printf '%s\n' '# schema: FrontierLeakageInputDigestV1/tsv-v1' > "$input_digests"
printf 'input_kind\tblake3\n' >> "$input_digests"
printf 'frontier_leakage_judge\t%s\n' "$(blake3 "$0")" >> "$input_digests"
printf 'frontier_members\t%s\n' "$(blake3 "$members")" >> "$input_digests"
printf 'sequestered_relations\t%s\n' "$(blake3 "$sequestered")" >> "$input_digests"

expected_files=$(printf '%s\n' frontier-members.v1.tsv sequestered.v1.tsv)
actual_files=$(find "$frontier_dir" -maxdepth 1 -type f -print | sed "s|$frontier_dir/||" | sort)
[ "$actual_files" = "$expected_files" ] || fail 'frontier contains an unrecognized relation'
[ "$(sed -n '1p' "$members")" = 'opaque_ref' ] || fail 'unexpected frontier-member schema'
[ "$(sed -n '1p' "$sequestered")" = "reference_class${tab}opaque_ref" ] || fail 'unexpected sequestered schema'

# W1 has a seven-member positive allowlist. All are required; a merely nonempty
# list would let an outcome-revealing or decision-insufficient world slip by.
assert_exact_rows "$members" 7 'frontier member relation'
assert_exact_rows "$sequestered" 10 'sequestered relation'
set +e
awk -F '\t' -v members="$members" '
  BEGIN {
    getline < members
    while ((getline line < members) > 0) allowed[line] = 1
  }
  NR > 1 && NF == 2 && allowed[$2] { exit 1 }
' "$sequestered"
overlap_exit=$?
set -e
[ "$overlap_exit" -eq 0 ] || fail 'frontier member overlaps sequestered aftermath material'
set +e
awk -F '\t' '
  BEGIN {
    split("seed_r1 project_charter_r1 hypothesis_graph_r1 base_source_snapshot_r1 behavior_observation_set_r1 documentation_observation_set_r1 c1_curated_account_r1", required, " ")
    for (ordinal = 1; ordinal <= 7; ordinal += 1) required_ref[required[ordinal]] = 1
  }
  NR > 1 {
    if (NF != 1 || $1 == "") { error = 1; exit }
    if (seen[$1]++) { error = 2; exit }
    if (!required_ref[$1]) unexpected = 1
  }
  END {
    if (error) exit error
    if (unexpected) exit 3
    for (ordinal = 1; ordinal <= 7; ordinal += 1) if (!seen[required[ordinal]]) exit 3
  }
' "$members"
members_exit=$?
set -e
case "$members_exit" in
  0) ;;
  2) fail 'duplicate frontier member opaque_ref' ;;
  3) fail 'frontier is missing a required positive member' ;;
  *) fail 'invalid frontier member relation' ;;
esac

# The ten aftermath classes are closed and distinct. The opaque fixture refs
# are likewise exact so output rows are derived from a validated relation, not
# from a hard-coded synthetic list.
set +e
awk -F '\t' '
  BEGIN {
    expected_ref["c1_decision"] = "sequestered_c1_decision_r1"
    expected_ref["candidate_patch"] = "sequestered_candidate_patch_r1"
    expected_ref["paired_task_treatment"] = "sequestered_paired_task_r1"
    expected_ref["adversarial_review"] = "sequestered_adversarial_review_r1"
    expected_ref["delivered_commit"] = "sequestered_delivered_commit_r1"
    expected_ref["outcome"] = "sequestered_outcome_r1"
    expected_ref["retrospective"] = "sequestered_retrospective_r1"
    expected_ref["l1_lesson"] = "sequestered_l1_lesson_r1"
    expected_ref["raw_pi_session"] = "sequestered_raw_pi_session_r1"
    expected_ref["current_xsh_snapshot"] = "sequestered_current_xsh_r1"
  }
  NR > 1 {
    if (NF != 2 || $1 == "" || $2 == "") { error = 1; exit }
    if (seen_class[$1]++) { error = 2; exit }
    if (seen_ref[$2]++) { error = 3; exit }
    if ($2 != expected_ref[$1]) { error = 1; exit }
  }
  END {
    if (error) exit error
    for (reference_class in expected_ref) if (!seen_class[reference_class]) exit 4
  }
' "$sequestered"
sequestered_exit=$?
set -e
case "$sequestered_exit" in
  0) ;;
  2) fail 'duplicate sequestered reference_class' ;;
  3) fail 'duplicate sequestered opaque_ref' ;;
  4) fail 'sequestered relation is missing a required aftermath class' ;;
  *) fail 'invalid sequestered relation' ;;
esac

results="$out/frontier-leakage-observations.v1.tsv"
printf '%s\n' '# schema: FrontierAccessObservationV1/tsv-v1' > "$results"
printf 'principal\tlookup_route\treference_class\topaque_ref\tdisposition\taudit_placement\n' >> "$results"

# The rows below are mechanically expanded from the validated relations. A
# future Rust query layer must perform the same check per request rather than
# treating this fixture output as a permission grant.
for principal in replay_actor projector ordinary_investigator grand_architect_query_client; do
  while IFS="$tab" read -r opaque_ref; do
    [ "$opaque_ref" = opaque_ref ] && continue
    printf '%s\tdirect_identity\tfrontier_member\t%s\tallowed\tno_audit\n' "$principal" "$opaque_ref" >> "$results"
  done < "$members"
  while IFS="$tab" read -r reference_class opaque_ref; do
    [ "$reference_class" = reference_class ] && continue
    for lookup_route in direct_identity graph_traversal object_digest current_repository_path culture_lookup projection_lookup; do
      printf '%s\t%s\t%s\t%s\tdenied\tcontamination_audit_outside_w1\n' "$principal" "$lookup_route" "$reference_class" "$opaque_ref" >> "$results"
    done
  done < "$sequestered"
done

allowed_count=$(awk -F '\t' 'NR > 2 && $5 == "allowed" { count += 1 } END { print count + 0 }' "$results")
denied_count=$(awk -F '\t' 'NR > 2 && $5 == "denied" { count += 1 } END { print count + 0 }' "$results")
[ "$allowed_count" -eq 28 ] || fail "incomplete positive frontier matrix: $allowed_count allowed reads"
[ "$denied_count" -eq 240 ] || fail "incomplete negative leakage matrix: $denied_count denied reads"
printf 'frontier leakage controls passed; results: %s\n' "$results"
