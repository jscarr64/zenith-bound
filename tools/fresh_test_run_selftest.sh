#!/usr/bin/env bash
# Self-test of tools/fresh_test_run.sh (the build-debug/build-release wrapper): the test run passes only if its command passed, it ran at
# least one test binary, and every binary it ran lies inside this run's CARGO_TARGET_DIR (Spence's ruling: no age test; the gate may reuse a
# target dir). A binary outside the target dir (another worktree's), no binary at all, or a failing command must each fail; an old binary
# inside the target dir passes. Paul's rule (2026-10-23): CARGO_TARGET_DIR must be set, and a target dir belongs to one worktree (marker
# .worktree-owner); a second worktree on it fails, as does an unset CARGO_TARGET_DIR.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap fresh_test_run_selftest "$@"   # run log (tools/runlog.sh)
runlog_detach_children
set -u
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
fail() { echo "fresh_test_run selftest FAIL: $*"; exit 1; }
t=$(mktemp -d /tmp/zb-fresh-selftest.XXXXXX); runlog_on_exit "rm -rf \"$t\""
mkdir -p "$t/target/debug/deps" "$t/elsewhere"
: > "$t/target/debug/deps/fresh-1"; : > "$t/target/debug/deps/stale-1"; touch -d '1 hour ago' "$t/target/debug/deps/stale-1"; : > "$t/elsewhere/other-1"
fake() { printf 'echo "     Running tests/harness/main.rs (%s)"; echo "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"; exit %s\n' "$1" "$2" > "$t/fake.sh"; }
git init -q "$t/wa" && git init -q "$t/wb" || fail "git init failed"   # two worktrees (Paul's rule: one target dir per worktree)
ftr() { (cd "${W:-$t/wa}" && CARGO_TARGET_DIR="$t/target" bash "$here/fresh_test_run.sh" bash "$t/fake.sh") > "$t/out" 2>&1; }
fake "$t/target/debug/deps/fresh-1" 0; ftr || fail "a fresh binary in the target dir was rejected: $(cat "$t/out")"
grep -q "test result: ok. 1 passed" "$t/out" || fail "the command's output was not passed through"
fake "$t/target/debug/deps/stale-1" 0; ftr || fail "an old binary inside CARGO_TARGET_DIR was rejected (no age test): $(cat "$t/out")"
fake "$t/elsewhere/other-1" 0; ftr && fail "a binary outside CARGO_TARGET_DIR was accepted"
grep -q "FOREIGN" "$t/out" || fail "foreign binary not named: $(cat "$t/out")"
fake "$t/target/debug/deps/fresh-1" 101; ftr && fail "a failing test command was accepted"
printf 'echo "no binaries"\n' > "$t/fake.sh"; ftr && fail "a run with no test binary was accepted"
# target dir ownership: the first worktree to use a target dir owns it (marker file in the dir); a second worktree on it fails
[ "$(cat "$t/target/.worktree-owner" 2> /dev/null)" = "$(cd "$t/wa" && pwd -P)" ] || fail "no owner marker naming worktree wa in the target dir"
fake "$t/target/debug/deps/fresh-1" 0; W=$t/wb ftr && fail "two worktrees on one target dir were accepted: $(cat "$t/out")"
grep -q "SHARED" "$t/out" || fail "shared target dir not named SHARED: $(cat "$t/out")"
grep -q "$(cd "$t/wa" && pwd -P)" "$t/out" || fail "shared target dir error does not name the owning worktree: $(cat "$t/out")"
(cd "$t/wb" && CARGO_TARGET_DIR="$t/target" bash "$here/fresh_test_run.sh" --claim) > "$t/out" 2>&1 && fail "--claim by a second worktree was accepted"
(cd "$t/wa" && CARGO_TARGET_DIR="$t/target" bash "$here/fresh_test_run.sh" --claim) > "$t/out" 2>&1 || fail "--claim by the owner failed: $(cat "$t/out")"
(cd "$t/wb" && CARGO_TARGET_DIR="$t/tb" bash "$here/fresh_test_run.sh" --claim) > "$t/out" 2>&1 || fail "--claim of a new target dir failed: $(cat "$t/out")"
[ "$(cat "$t/tb/.worktree-owner" 2> /dev/null)" = "$(cd "$t/wb" && pwd -P)" ] || fail "--claim wrote no owner marker for wb"
(cd "$t/wa" && env -u CARGO_TARGET_DIR bash "$here/fresh_test_run.sh" bash "$t/fake.sh") > "$t/out" 2>&1 && fail "an unset CARGO_TARGET_DIR was accepted"
grep -q "CARGO_TARGET_DIR is unset" "$t/out" || fail "unset CARGO_TARGET_DIR not named: $(cat "$t/out")"
echo "fresh_test_run selftest: ok"
