#!/usr/bin/env bash
# Self-test of tools/fresh_test_run.sh (the build-debug/build-release wrapper): the test run passes only if its command passed, it ran at
# least one test binary, and every binary it ran was built during this gate run (mtime >= the gate start) inside this run's target dir.
# A stale binary (the reused one from another worktree's build), a binary outside the target dir, no binary at all, or a failing command
# must each fail.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap fresh_test_run_selftest "$@"   # run log (tools/runlog.sh)
runlog_detach_children
set -u
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
fail() { echo "fresh_test_run selftest FAIL: $*"; exit 1; }
t=$(mktemp -d /tmp/zb-fresh-selftest.XXXXXX); runlog_on_exit "rm -rf \"$t\""
mkdir -p "$t/target/debug/deps" "$t/elsewhere"
t0=$(date +%s)
: > "$t/target/debug/deps/fresh-1"; : > "$t/target/debug/deps/stale-1"; touch -d '1 hour ago' "$t/target/debug/deps/stale-1"; : > "$t/elsewhere/other-1"
fake() { printf 'echo "     Running tests/harness/main.rs (%s)"; echo "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"; exit %s\n' "$1" "$2" > "$t/fake.sh"; }
ftr() { CARGO_TARGET_DIR="$t/target" bash "$here/fresh_test_run.sh" "$t0" bash "$t/fake.sh" > "$t/out" 2>&1; }
fake "$t/target/debug/deps/fresh-1" 0; ftr || fail "a fresh binary in the target dir was rejected: $(cat "$t/out")"
grep -q "test result: ok. 1 passed" "$t/out" || fail "the command's output was not passed through"
fake "$t/target/debug/deps/stale-1" 0; ftr && fail "a binary older than the gate start was accepted"
grep -q "STALE" "$t/out" || fail "stale binary not named: $(cat "$t/out")"
fake "$t/elsewhere/other-1" 0; ftr && fail "a binary outside CARGO_TARGET_DIR was accepted"
fake "$t/target/debug/deps/fresh-1" 101; ftr && fail "a failing test command was accepted"
printf 'echo "no binaries"\n' > "$t/fake.sh"; ftr && fail "a run with no test binary was accepted"
echo "fresh_test_run selftest: ok"
