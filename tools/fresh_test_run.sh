#!/usr/bin/env bash
# usage: tools/fresh_test_run.sh GATE_START_EPOCH CMD...   (full_gate's build-debug/build-release: CMD is the cargo test run)
# Runs CMD with its output passed through, then fails unless CMD passed, it ran at least one test binary ("Running ... (PATH)" lines), and
# every binary it ran lies inside CARGO_TARGET_DIR (default ./target) and was written at or after GATE_START_EPOCH, i.e. built during this
# gate run. Catches a test step that silently reused a binary built earlier, e.g. by another worktree sharing the target dir.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap fresh_test_run "$@"   # run log (tools/runlog.sh)
set -u
t0=${1:?usage: tools/fresh_test_run.sh GATE_START_EPOCH CMD...}; shift
out=$(mktemp /tmp/zb-fresh.XXXXXX) || exit 2
"$@" 2>&1 | tee "$out"; rc=${PIPESTATUS[0]}
td=$(cd "${CARGO_TARGET_DIR:-target}" 2> /dev/null && pwd -P) || { echo "fresh_test_run: target dir ${CARGO_TARGET_DIR:-target} missing"; rm -f "$out"; exit 1; }
bad=0 n=0
while IFS= read -r p; do
  n=$((n + 1))
  real=$(cd "$(dirname "$p")" 2> /dev/null && pwd -P)/$(basename "$p")
  case "$real" in "$td"/*) ;; *) echo "fresh_test_run: FOREIGN binary $p is not inside $td"; bad=1; continue ;; esac
  m=$(stat -c %Y "$real" 2> /dev/null) || { echo "fresh_test_run: binary $p missing"; bad=1; continue; }
  if [ "$m" -lt "$t0" ]; then echo "fresh_test_run: STALE binary $p written $(date -d "@$m" '+%F %T %Z'), before this gate started $(date -d "@$t0" '+%F %T %Z')"; bad=1; fi
done < <(sed -n 's/^ *Running .*(\(\/[^)]*\))$/\1/p' "$out")
rm -f "$out"
[ "$n" -gt 0 ] || { echo "fresh_test_run: no test binary ran"; bad=1; }
[ "$rc" = 0 ] || { echo "fresh_test_run: test command failed (exit $rc)"; exit "$rc"; }
[ "$bad" = 0 ] || exit 1
echo "fresh_test_run: $n test binaries, all built during this gate run in $td"
