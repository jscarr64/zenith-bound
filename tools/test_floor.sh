#!/usr/bin/env bash
# tools/test_floor.sh STEP LOG: fail a cargo-test step whose log under-counts (Paul, 2026-10-23; repo-side, poplog's generic guard is Rusty's).
# Reads every `test result:` summary in LOG and sums `passed` only. FAILS (exit 1, one line each) when:
#   NOSUMMARY  LOG has no `test result:` line, or fewer summaries than test binaries started (`Running ...` / `Doc-tests ...` lines):
#              a binary whose output stopped before its summary (a truncated log)
#   BELOW      the total is below STEP's floor in tools/test_floor.txt (env TEST_FLOOR_FILE overrides)
#   NOFLOOR    STEP has no row in the floor file;  NOLOG  LOG is missing or unreadable
#   BADCOUNT   any parsed count (floor, starts, summaries, passed) is not a plain integer: the check fails closed (set -euo pipefail)
# Floors: tools/test_floor.txt. Raising one is allowed with the tests; lowering one needs a CHANGELOG.md note. Self-test: tools/test_floor_selftest.sh.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap test_floor "$@"   # run log (tools/runlog.sh)
set -euo pipefail
step=${1:?usage: tools/test_floor.sh STEP LOG}; log=${2:?usage: tools/test_floor.sh STEP LOG}
ff=${TEST_FLOOR_FILE:-"$(cd "$(dirname "$0")" && pwd)/test_floor.txt"}
int() { [[ $2 =~ ^[0-9]{1,18}$ ]] || { echo "FAIL BADCOUNT: $step: $1 is '$2', not a plain integer of at most 18 digits ($log)"; exit 1; }; }   # fail closed on any parse surprise (18 digits: inside bash's i64 test)
[ -f "$log" ] && [ -r "$log" ] || { echo "FAIL NOLOG: $step: $log is missing or unreadable"; exit 1; }
[ -f "$ff" ] && [ -r "$ff" ] || { echo "FAIL NOFLOOR: floor file $ff is missing or unreadable"; exit 1; }
floor=$(awk -F'\t' -v s="$step" '$1 == s { print $2; exit }' "$ff") || { echo "FAIL NOFLOOR: cannot read $ff"; exit 1; }
[ -n "$floor" ] || { echo "FAIL NOFLOOR: $step has no row in $ff"; exit 1; }
int floor "$floor"
# Only `passed` counts toward the floor: failed tests are not coverage (a red step fails on its own exit code).
counts=$(awk '
  /^[ \t]*(Running|Doc-tests)[ \t]/ { st++ }
  /test result: [A-Za-z]+\. [0-9]+ passed; [0-9]+ failed;/ {
    su++; for (i = 1; i < NF; i++) { if ($(i+1) ~ /^passed;?$/) tot += $i } }
  END { print st + 0, su + 0, tot + 0 }' "$log") || { echo "FAIL BADCOUNT: $step: awk failed on $log"; exit 1; }
read -r starts sums total extra <<< "$counts" || true
[ -z "${extra:-}" ] || { echo "FAIL BADCOUNT: $step: unexpected parse output '$counts'"; exit 1; }
int starts "${starts:-}"; int summaries "${sums:-}"; int passed "${total:-}"
rc=0
# Each test is written so that a comparison error (it cannot happen after int) lands on the FAIL branch, never on a pass.
if [ "$sums" -gt 0 ]; then :; else echo "FAIL NOSUMMARY: $step: no 'test result:' line in $log"; rc=1; fi
if [ "$sums" -ge "$starts" ]; then :; else echo "FAIL NOSUMMARY: $step: $starts test binaries started, only $sums summaries (truncated log?) in $log"; rc=1; fi
if [ "$total" -ge "$floor" ]; then :; else echo "FAIL BELOW: $step passed $total tests, floor is $floor ($ff)"; rc=1; fi
if [ "$rc" -eq 0 ]; then echo "test_floor: $step ok ($total passed in $sums summaries, floor $floor)"; fi
exit "$rc"
