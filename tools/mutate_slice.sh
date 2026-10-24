#!/usr/bin/env bash
# usage: tools/mutate_slice.sh SLICE [ID_REGEX]   (with ID_REGEX only matching ids run and the floor is not checked)
# Runs every mutant of tools/mutants/manifest.d/SLICE.txt with tools/mutate.sh --only and fails (exit 1) if any mutant is
# INVALID (patch does not apply or does not build) or SURVIVED without an equivalent.d entry, or if the slice's killed and
# killed+equivalent counts are below tools/mutants/floor.d/SLICE.txt. `mutate.sh --list` and `--floor` alone never apply a
# patch, so a stale patch passed the gate (slice C merge 8597039, 2026-10-24); this step closes that hole.
set -uo pipefail
slice=${1:?usage: tools/mutate_slice.sh SLICE}
repo=$(git rev-parse --show-toplevel)
man=$repo/tools/mutants/manifest.d/$slice.txt
only=${2:-}
floor=$repo/tools/mutants/floor.d/$slice.txt
[ -f "$man" ] || { echo "mutate_slice: no manifest $man"; exit 1; }
[ -f "$floor" ] || { echo "mutate_slice: no floor $floor"; exit 1; }
nk=0 ne=0 bad=0 n=0
while IFS=$'\t' read -r _ id _ _; do
  case "$id" in ''|'#'*) continue ;; esac
  if [ -n "$only" ] && ! [[ "$id" =~ $only ]]; then continue; fi
  n=$((n + 1))
  out=$("$repo/tools/mutate.sh" --only "$id" 2>&1)
  line=$(printf '%s\n' "$out" | grep -E "^(KILLED|SURVIVED-EQUIVALENT|SURVIVED|INVALID) $id\b" | head -1)
  echo "${line:-NORESULT $id}"
  case "$line" in
    "KILLED "*) nk=$((nk + 1)) ;;
    "SURVIVED-EQUIVALENT "*) ne=$((ne + 1)) ;;
    *) bad=$((bad + 1)); printf '%s\n' "$out" | tail -8 | sed 's/^/    /' ;;
  esac
done < <(grep -v '^#' "$man")
fk=$(awk -F'\t' '$1=="killed"{print $2}' "$floor")
fe=$(awk -F'\t' '$1=="killed+equivalent"{print $2}' "$floor")
if [ -n "$only" ]; then fk=0 fe=0; fi
echo "mutate_slice $slice: $n mutants, killed $nk (floor $fk), killed+equivalent $((nk + ne)) (floor $fe), bad $bad"
if [ "$n" -eq 0 ] || [ "$bad" -ne 0 ] || [ "$nk" -lt "${fk:-0}" ] || [ $((nk + ne)) -lt "${fe:-0}" ]; then
  echo "mutate_slice $slice: FAIL"; exit 1
fi
echo "mutate_slice $slice: PASS"
