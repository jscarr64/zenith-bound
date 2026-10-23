#!/usr/bin/env bash
# usage: tools/fresh_test_run.sh CMD...   (full_gate's build-debug/build-release: CMD is the cargo test run)
# Runs CMD with its output passed through, then fails unless CMD passed, it ran at least one test binary ("Running ... (PATH)" lines), and
# every binary it ran lies inside CARGO_TARGET_DIR (default ./target). Catches a test step that ran a binary from elsewhere (another
# worktree's target). No age test (Spence's ruling, 2026-10-23): the gate may reuse a target dir; per-job subdirs are optional.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap fresh_test_run "$@"   # run log (tools/runlog.sh)
set -u
[ $# -gt 0 ] || { echo "usage: tools/fresh_test_run.sh CMD..."; exit 2; }
out=$(mktemp /tmp/zb-fresh.XXXXXX) || exit 2
"$@" 2>&1 | tee "$out"; rc=${PIPESTATUS[0]}
td=$(cd "${CARGO_TARGET_DIR:-target}" 2> /dev/null && pwd -P) || { echo "fresh_test_run: target dir ${CARGO_TARGET_DIR:-target} missing"; rm -f "$out"; exit 1; }
bad=0 n=0
while IFS= read -r p; do
  n=$((n + 1))
  real=$(cd "$(dirname "$p")" 2> /dev/null && pwd -P)/$(basename "$p")
  case "$real" in "$td"/*) ;; *) echo "fresh_test_run: FOREIGN binary $p is not inside $td"; bad=1; continue ;; esac
  [ -f "$real" ] || { echo "fresh_test_run: binary $p missing"; bad=1; continue; }
done < <(sed -n 's/^ *Running .*(\(\/[^)]*\))$/\1/p' "$out")
rm -f "$out"
[ "$n" -gt 0 ] || { echo "fresh_test_run: no test binary ran"; bad=1; }
[ "$rc" = 0 ] || { echo "fresh_test_run: test command failed (exit $rc)"; exit "$rc"; }
[ "$bad" = 0 ] || exit 1
echo "fresh_test_run: $n test binaries, all inside $td"
