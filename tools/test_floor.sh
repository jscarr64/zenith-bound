#!/usr/bin/env bash
# tools/test_floor.sh STEP LOG: fail a cargo-test step whose log under-counts (Paul, 2026-10-23; repo-side, poplog's generic guard is Rusty's).
# Reads every `test result:` summary in LOG and sums passed+failed. FAILS (exit 1, one line each) when:
#   NOSUMMARY  LOG has no `test result:` line, or fewer summaries than test binaries started (`Running ...` / `Doc-tests ...` lines):
#              a binary whose output stopped before its summary (a truncated log)
#   BELOW      the total is below STEP's floor in tools/test_floor.txt (env TEST_FLOOR_FILE overrides)
#   NOFLOOR    STEP has no row in the floor file;  NOLOG  LOG is missing
# Floors: tools/test_floor.txt. Raising one is allowed with the tests; lowering one needs a CHANGELOG.md note. Self-test: tools/test_floor_selftest.sh.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap test_floor "$@"   # run log (tools/runlog.sh)
set -u
step=${1:?usage: tools/test_floor.sh STEP LOG}; log=${2:?usage: tools/test_floor.sh STEP LOG}
ff=${TEST_FLOOR_FILE:-"$(cd "$(dirname "$0")" && pwd)/test_floor.txt"}
[ -f "$log" ] || { echo "FAIL NOLOG: $step: $log does not exist"; exit 1; }
floor=$(awk -F'\t' -v s="$step" '$1 == s && $2 ~ /^[0-9]+$/ { print $2; exit }' "$ff" 2>/dev/null)
[ -n "$floor" ] || { echo "FAIL NOFLOOR: $step has no row in $ff"; exit 1; }
read -r starts sums total < <(awk '
  /^[ \t]*(Running|Doc-tests)[ \t]/ { st++ }
  /test result: [A-Za-z]+\. [0-9]+ passed; [0-9]+ failed;/ {
    su++; for (i = 1; i <= NF; i++) { if ($(i+1) ~ /^passed;?$/ || $(i+1) ~ /^failed;?$/) tot += $i } }
  END { print st + 0, su + 0, tot + 0 }' "$log")
rc=0
if [ "$sums" = 0 ]; then echo "FAIL NOSUMMARY: $step: no 'test result:' line in $log"; rc=1
elif [ "$sums" -lt "$starts" ]; then echo "FAIL NOSUMMARY: $step: $starts test binaries started, only $sums summaries (truncated log?) in $log"; rc=1; fi
if [ "$total" -lt "$floor" ]; then echo "FAIL BELOW: $step ran $total tests, floor is $floor ($ff)"; rc=1; fi
[ "$rc" = 0 ] && echo "test_floor: $step ok ($total tests in $sums summaries, floor $floor)"
exit "$rc"
