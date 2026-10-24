#!/usr/bin/env bash
# Per-slice mutant run for the gate (Spence-approved design, 2026-10-24). It does NOT trust tools/mutate.sh's exit code: it checks the
# result lines itself and FAILS on any of
#   TIMEOUT     the slice ran past its budget: names every id that never ran
#   INCOMPLETE  a manifest id produced no result line (e.g. 257 results for 268 ids): names the missing ids
#   INVALID     an INVALID mutant
#   SURVIVED    a survivor with no equivalent entry (mutate.sh prints SURVIVED-EQUIVALENT for documented ones)
#   BELOW floor the slice's KILLED / KILLED+SURVIVED-EQUIVALENT counts are under its floor (mutate.sh --floor --slice)
# and on a duplicate or unknown result id, an unreadable id list or floor, or a nonzero runner exit.
#   TIMEOUT <id> one mutant ran past MUTANT_TIMEOUT_S (each id runs as its own `mutate.sh --slice S --only ID` under that limit)
# usage: tools/mutate_slice.sh SLICE     one slice (central, or a manifest.d/<slice>.txt name)
#        tools/mutate_slice.sh SLICE ID_REGEX   PARTIAL: only the slice ids matching the ERE; floor and completeness are NOT checked (says so,
#                                        never prints a full pass); a regex matching no id fails. --all takes no regex (the gate never runs partial).
#        tools/mutate_slice.sh --all     every slice from `mutate.sh --slices`, then the whole-manifest completeness check
#        tools/mutate_slice.sh --step-timeout   prints the seconds the gate's mutants step may take (sum of slice timeouts + 600)
# Per-slice timeout = ids x MUTANT_BUDGET_S + MUTATE_SETUP_S (fetch, plus the first build of a slice's suite into the persistent target).
# BUDGET (measured, see CHANGELOG 2026-10-24): BUDGET_S and SETUP_S below come from a real tiny-slice run; the values and their source are
# in the CHANGELOG entry for this file. env: MUTANT_BUDGET_S, MUTATE_SETUP_S (positive integers; override for selftests only), MUTATE_SH;
# tools/full_gate.sh unsets all of them (and MUTANT_TIMEOUT_S, MUTATE_FLOOR_ZERO_OK) for the gate.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap mutate_slice "$@"   # run log (tools/runlog.sh)
set -u
export LC_ALL=C
M=${MUTATE_SH:-${BASH_SOURCE[0]%/*}/mutate.sh}
# Measured 2026-10-24 (poprun log harper/2026-10-24/1644-zb-mslice-measure, f854a9d, tiny 4-mutant slice from a fresh MUTATE_TARGET,
# one per suite mag/array/ball/harness): per-mutant max 14s (178-U-underflow-zero), whole run 21s (so setup ~1s). Also 1520-zb-mutq-merge-full-gate:
# 184 mutants in a 4m19s step (~1.4s mean). BUDGET_S = max 14s x ~4 margin = 60s; SETUP_S = 300s (fetch + first suite build under load).
BUDGET_S=60; SETUP_S=300
# Per-mutant limit: the same measurement (max 14s, 178-U-underflow-zero, 1644-zb-mslice-measure) x ~4 = 60s. A mutant past it is
# `TIMEOUT <id>` and fails the slice (mutate.sh's own 20m test timeout would count a hang as KILLED). env MUTANT_TIMEOUT_S (selftests only).
MUTANT_TIMEOUT_DEFAULT_S=60
mto=${MUTANT_TIMEOUT_S:-$MUTANT_TIMEOUT_DEFAULT_S}
[[ $mto =~ ^[0-9]{1,6}$ ]] && [ "$mto" -gt 0 ] || { echo "mutate_slice: FAIL MUTANT_TIMEOUT_S '$mto' is not a positive integer"; exit 2; }
budget=${MUTANT_BUDGET_S:-$BUDGET_S}; setup=${MUTATE_SETUP_S:-$SETUP_S}
[[ $budget =~ ^[0-9]{1,6}$ ]] && [ "$budget" -gt 0 ] || { echo "mutate_slice: FAIL MUTANT_BUDGET_S '$budget' is not a positive integer"; exit 2; }
[[ $setup =~ ^[0-9]{1,6}$ ]] || { echo "mutate_slice: FAIL MUTATE_SETUP_S '$setup' is not an integer"; exit 2; }
w=$(mktemp -d "$(zb_scratch)/zb-mutate-slice.XXXXXX") || { echo "mutate_slice: FAIL cannot create scratch dir"; exit 2; }
runlog_on_exit "rm -rf \"$w\""
words() { printf '%s\n' "$1" | awk 'NF' | tr '\n' ' ' | sed 's/ $//'; }
slices() { local sl; sl=$("$M" --slices) || { echo "mutate_slice: FAIL mutate.sh --slices failed" >&2; return 1; }
  [ -n "$sl" ] || { echo "mutate_slice: FAIL mutate.sh --slices listed nothing" >&2; return 1; }; printf '%s\n' "$sl"; }
# slice_ids SLICE: the slice's manifest ids (fails closed)
slice_ids() { "$M" --list --slice "$1" || { echo "mutate_slice: FAIL mutate.sh --list --slice $1 failed" >&2; return 1; }; }
# one SLICE: run and check one slice; its result ids go to $w/SLICE.got
one() {
  local s=$1 re=${2-} partial=0 ids n fl id left lim t0 irc tmo="" cut=""  fk fke to rc=0 out=$w/$1.out res got nres miss dup unk inv sur nk ne bad=0 tmax
  ids=$(slice_ids "$s") || return 1; ids=$(printf '%s\n' "$ids" | awk 'NF')
  if [ $# -ge 2 ]; then
    partial=1
    irc=0; ids=$(printf '%s\n' "$ids" | grep -E -- "$re") || irc=$?
    [ "$irc" -le 1 ] || { echo "mutate_slice: FAIL ID_REGEX '$re' is not a valid ERE"; return 1; }
    [ -n "$ids" ] || { echo "mutate_slice: FAIL ID_REGEX '$re' matches no id in slice $s"; return 1; }
  fi
  n=$(printf '%s\n' "$ids" | awk 'NF' | wc -l)
  fl=$("$M" --floor --slice "$s") || { echo "mutate_slice: FAIL slice $s: mutate.sh --floor --slice failed"; return 1; }
  fk=$(printf '%s\n' "$fl" | awk '$1 == "killed" { print $2 }'); fke=$(printf '%s\n' "$fl" | awk '$1 == "killed+equivalent" { print $2 }')
  [[ $fk =~ ^[0-9]+$ && $fke =~ ^[0-9]+$ ]] || { echo "mutate_slice: FAIL slice $s: unreadable floor: $(words "$fl")"; return 1; }
  to=$((n * budget + setup))
  echo "mutate_slice: slice $s: $n ids, timeout ${to}s ($n x ${budget}s + ${setup}s setup), floor killed $fk killed+equivalent $fke"
  : > "$w/$s.got"
  if [ "$n" -gt 0 ]; then
    : > "$out"; t0=$SECONDS
    for id in $ids; do
      left=$((to - (SECONDS - t0)))
      if [ "$left" -le 0 ]; then cut+=" $id"; continue; fi
      lim=$mto; [ "$left" -lt "$lim" ] && lim=$left
      irc=0; timeout -k 30 "$lim" "$M" --slice "$s" --only "$id" > "$out.1" 2>&1 < /dev/null || irc=$?
      cat "$out.1"; cat "$out.1" >> "$out"
      if [ "$irc" = 124 ] || [ "$irc" = 137 ]; then
        if [ "$lim" = "$mto" ]; then echo "TIMEOUT $id: ran past the ${mto}s per-mutant limit"; tmo+=" $id"; else cut+=" $id"; fi
      elif [ "$irc" != 0 ] && [ "$rc" = 0 ]; then rc=$irc; fi
    done
    [ -z "$cut" ] || rc=124
  else : > "$out"; fi
  res=$(awk '$1 ~ /^(KILLED|SURVIVED-EQUIVALENT|SURVIVED|INVALID)$/ { id = $2; sub(/:$/, "", id); print $1, id }' "$out")
  got=$(printf '%s\n' "$res" | awk 'NF { print $2 }'); printf '%s\n' "$got" | awk 'NF' > "$w/$s.got"
  nres=$(awk 'NF' "$w/$s.got" | wc -l)
  miss=$(printf '%s\n' "$ids" | awk -v f="$w/$s.got" 'BEGIN { while ((getline l < f) > 0) g[l] = 1 } NF && !($1 in g)')
  dup=$(sort "$w/$s.got" | uniq -d)
  unk=$(printf '%s\n' "$ids" | awk -v f="$w/$s.got" 'NF { k[$1] = 1 } END { while ((getline l < f) > 0) if (!(l in k)) print l }')
  [ -z "$tmo" ] || { echo "mutate_slice: FAIL TIMEOUT (per-mutant ${mto}s): $(words "$(printf '%s\n' $tmo)") (slice $s)"; bad=1
    miss=$(printf '%s\n' "$miss" | grep -vxF -f <(printf '%s\n' $tmo) || true); }
  if [ "$partial" = 1 ]; then
    [ -z "$cut" ] || { echo "mutate_slice: FAIL TIMEOUT: slice $s ran past ${to}s; never ran: $(words "$(printf '%s\n' $cut)")"; bad=1; }
  elif [ "$rc" = 124 ] || [ "$rc" = 137 ]; then
    echo "mutate_slice: FAIL TIMEOUT: slice $s ran past ${to}s ($n x ${budget}s + ${setup}s): $nres results for $n ids; never ran: $(words "$miss")"; bad=1
  elif [ -n "$miss" ]; then
    echo "mutate_slice: FAIL INCOMPLETE: $nres results for $n ids in slice $s; missing: $(words "$miss")"; bad=1
  fi
  [ -z "$dup" ] || { echo "mutate_slice: FAIL slice $s: duplicate result for $(words "$dup")"; bad=1; }
  [ -z "$unk" ] || { echo "mutate_slice: FAIL slice $s: result for an id not in the slice: $(words "$unk")"; bad=1; }
  inv=$(printf '%s\n' "$res" | awk '$1 == "INVALID" { print $2 }'); sur=$(printf '%s\n' "$res" | awk '$1 == "SURVIVED" { print $2 }')
  [ -z "$inv" ] || { echo "mutate_slice: FAIL INVALID: $(words "$inv") (slice $s)"; bad=1; }
  [ -z "$sur" ] || { echo "mutate_slice: FAIL SURVIVED (no equivalent entry): $(words "$sur") (slice $s)"; bad=1; }
  nk=$(printf '%s\n' "$res" | awk '$1 == "KILLED"' | wc -l); ne=$(printf '%s\n' "$res" | awk '$1 == "SURVIVED-EQUIVALENT"' | wc -l)
  if [ "$partial" = 0 ] && { [ "$nk" -lt "$fk" ] || [ "$((nk + ne))" -lt "$fke" ]; }; then
    echo "mutate_slice: FAIL BELOW floor: slice $s: $nk killed (floor $fk), $((nk + ne)) killed+equivalent (floor $fke)"; bad=1
  fi
  case $rc in 0|124|137) ;; *) echo "mutate_slice: FAIL slice $s: mutate.sh exited $rc"; bad=1 ;; esac
  tmax=$(awk '$1 == "TIME" { v = $3; sub(/s$/, "", v); if (v + 0 > m) { m = v + 0; id = $2 } } END { if (id != "") print m "s (" id ")"; else print "-" }' "$out")
  if [ "$partial" = 1 ]; then
    echo "mutate_slice: PARTIAL run (ID_REGEX=$re): floor/completeness not checked"
    echo "mutate_slice: slice $s PARTIAL: $nres results for $n matching ids, $nk killed, $ne survived-equivalent, max per-mutant time $tmax (limit ${mto}s) -> $([ "$bad" = 0 ] && echo 'PARTIAL, not a gate pass' || echo FAIL)"
    return "$bad"
  fi
  echo "mutate_slice: slice $s: $nres/$n results, $nk killed, $ne survived-equivalent, max per-mutant time $tmax (budget ${budget}s) -> $([ "$bad" = 0 ] && echo ok || echo FAIL)"
  return "$bad"
}
case ${1:-} in
  --step-timeout)
    sl=$(slices) || exit 1; tot=600
    for s in $sl; do ids=$(slice_ids "$s") || exit 1; n=$(printf '%s\n' "$ids" | awk 'NF' | wc -l); [ "$n" = 0 ] || tot=$((tot + n * budget + setup)); done
    echo "$tot"; exit 0 ;;
  --all)
    [ $# = 1 ] || { echo "mutate_slice: FAIL --all takes no ID_REGEX (the gate never runs a partial slice)"; exit 2; }
    sl=$(slices) || exit 1; frc=0; failed=""
    for s in $sl; do one "$s" || { frc=1; failed+=" $s"; }; done
    # whole-manifest completeness: every id of `mutate.sh --list` belongs to a slice and produced exactly one result
    all=$("$M" --list) || { echo "mutate_slice: FAIL mutate.sh --list failed"; exit 1; }
    cat "$w"/*.got 2>/dev/null | awk 'NF' > "$w/all.got"
    na=$(printf '%s\n' "$all" | awk 'NF' | wc -l); ng=$(awk 'NF' "$w/all.got" | wc -l)
    amiss=$(printf '%s\n' "$all" | awk -v f="$w/all.got" 'BEGIN { while ((getline l < f) > 0) g[l] = 1 } NF && !($1 in g)')
    if [ -n "$amiss" ] || [ "$na" = 0 ]; then echo "mutate_slice: FAIL INCOMPLETE: $ng results for $na manifest ids; missing: $(words "$amiss")"; frc=1; fi
    if [ "$frc" = 0 ]; then echo "mutate_slice: all slices ok ($ng results for $na manifest ids)"; else echo "mutate_slice: FAIL slices:$failed ($ng results for $na manifest ids)"; fi
    exit "$frc" ;;
  ''|-*) echo "usage: tools/mutate_slice.sh SLICE [ID_REGEX] | --all | --step-timeout"; exit 2 ;;
  *) [ $# -le 2 ] || { echo "usage: tools/mutate_slice.sh SLICE [ID_REGEX] | --all | --step-timeout"; exit 2; }; one "$@"; exit $? ;;
esac
