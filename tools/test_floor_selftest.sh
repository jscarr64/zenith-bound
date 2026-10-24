#!/usr/bin/env bash
# Self-test for tools/test_floor.sh: synthetic cargo-test step logs under /tmp prove the check PASSES on a complete log at the floor
# and FAILS on (1) a truncated log (the 2052 gate case: a 738-test run whose debug log stopped early, poplog still read PASS),
# (2) a binary whose `Running` line has no `test result:` summary, (3) no summary at all, (4) a complete log below the floor,
# (5) a step with no floor row, (6) a missing log. No repo files are touched.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap test_floor_selftest "$@"   # run log (tools/runlog.sh)
runlog_detach_children
set -u
here="$(cd "$(dirname "$0")" && pwd)"; t="$(mktemp -d /tmp/test-floor-selftest.XXXXXX)" || { echo "mktemp failed"; exit 2; }; runlog_on_exit "rm -rf \"$t\""
fail() { echo "SELFTEST FAIL: $*"; exit 1; }
printf '# fixture\nbuild-debug\t30\n' > "$t/floor.txt"
chk() { TEST_FLOOR_FILE="$t/floor.txt" "$here/test_floor.sh" "$@"; }
bin() { # bin <name> <passed>: one test binary's output
  printf '     Running unittests src/lib.rs (target/debug/deps/%s-0123)\n\nrunning %s tests\n' "$1" "$2"
  printf 'test result: ok. %s passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n\n' "$2"; }
{ bin a 10; bin b 15; printf '   Doc-tests zenith_bound\n\nrunning 5 tests\n'
  printf 'test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s\n'; } > "$t/full.log"
chk build-debug "$t/full.log" || fail "a complete 30-test log at floor 30 must PASS"
head -n 9 "$t/full.log" > "$t/trunc.log"          # cut after binary b's Running line: 10 counted, b and doc-tests lost
chk build-debug "$t/trunc.log" && fail "a truncated log must FAIL"
{ bin a 10; printf '     Running tests/harness/main.rs (target/debug/deps/harness-0123)\n\nrunning 20 tests\n'; bin c 25; } > "$t/nosum.log"
chk build-debug "$t/nosum.log" && fail "a binary with no summary must FAIL even when the total (35) clears the floor"
printf '     Running x\nerror: test failed\n' > "$t/none.log"
chk build-debug "$t/none.log" && fail "a log with no summary must FAIL"
: > "$t/empty.log"
chk build-debug "$t/empty.log" && fail "an empty log must FAIL"
{ bin a 10; bin b 15; } > "$t/low.log"
chk build-debug "$t/low.log" && fail "a complete 25-test log below floor 30 must FAIL"
chk build-release "$t/full.log" && fail "a step with no floor row must FAIL"
chk build-debug "$t/absent.log" && fail "a missing log must FAIL"
# Spence (92ac25f review): the check must not fail open.
{ bin a 20; printf '     Running b\n\nrunning 15 tests\n'; printf 'test result: FAILED. 0 passed; 15 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n'; } > "$t/failed.log"
chk build-debug "$t/failed.log" && fail "20 passed + 15 failed must FAIL a floor of 30 (only passed counts)"
cp "$t/full.log" "$t/unreadable.log"; chmod 000 "$t/unreadable.log"
if [ -r "$t/unreadable.log" ]; then echo "test_floor_selftest: note: running as a user who can read mode-000 files; unreadable case skipped"
else chk build-debug "$t/unreadable.log" && fail "an unreadable log must FAIL"; fi
chmod 600 "$t/unreadable.log"
{ bin a 10; printf 'test result: ok. 99999999999999999999999 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n'; } > "$t/garbage.log"
printf '     Running b\n' | cat - "$t/garbage.log" > "$t/garbage2.log"
chk build-debug "$t/garbage2.log" && fail "a log whose count is not a plain integer must FAIL"
printf '\001\002\377binary garbage\ntest result: ok. x passed; y failed;\n' > "$t/binary.log"
chk build-debug "$t/binary.log" && fail "a garbage log must FAIL"
echo "test_floor_selftest: ok"
