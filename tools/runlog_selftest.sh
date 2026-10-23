#!/usr/bin/env bash
# Self-test of tools/runlog.sh (Jeff 9:42 AM CT: every Pop run gets a log). Fixtures only, under a throwaway RUNLOG_ROOT:
# a --tests step that runs zero tests is FAIL, a passing cargo-style step is parsed, the RUNNING.tsv row is added (other agents' rows stay)
# and removed on exit and on TERM, the heartbeat fires (RUNLOG_BEAT_SECS=1) and updates the last-beat column, the header and footer are written,
# logs older than 30 days are pruned for this agent only, and a nested script does not open a second log.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap runlog_selftest "$@"   # run log (tools/runlog.sh)
set -u
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
fail() { echo "runlog selftest FAIL: $*"; exit 1; }
t=$(mktemp -d /tmp/zb-runlog-selftest.XXXXXX); trap 'rm -rf "$t"' EXIT
[ -f "$here/runlog.sh" ] || fail "tools/runlog.sh does not exist"
export RUNLOG_ROOT="$t/pop-logs" RUNLOG_AGENT=selftest
unset RUNLOG_FILE RUNLOG_INNER
mkdir -p "$RUNLOG_ROOT"
printf 'paul\tother-job\t1\t/x.log\t2026-01-01T00:00:00\t2026-01-01T00:00:00\n' > "$RUNLOG_ROOT/RUNNING.tsv"
# prune fixtures: an old log of mine (removed), an old log of another agent (kept), a fresh log of mine (kept)
mkdir -p "$RUNLOG_ROOT/selftest/2000-01-01" "$RUNLOG_ROOT/paul/2000-01-01"
touch -d '40 days ago' "$RUNLOG_ROOT/selftest/2000-01-01/0000-old.log" "$RUNLOG_ROOT/paul/2000-01-01/0000-old.log"
touch "$RUNLOG_ROOT/selftest/2000-01-01/0001-new.log"

# fixture script: a zero-test --tests step, then a passing cargo-style step, with a pause so the heartbeat fires
cat > "$t/job.sh" <<'JOB'
#!/usr/bin/env bash
. "$RUNLOG_LIB"; runlog_wrap fixture-job "$@"
runlog_step --tests zero -- bash -c 'echo "running 0 tests"; echo "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"'
echo "zero rc=$?"
runlog_step --tests good -- bash -c 'echo "test a::b ... ok"; sleep 3; echo "test result: ok. 5 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out"; echo "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"'
grep -q "^selftest	fixture-job	" "$RUNLOG_ROOT/RUNNING.tsv" && echo "row present during run"
runlog_warn "fixture warning"
JOB
RUNLOG_LIB="$here/runlog.sh" RUNLOG_BEAT_SECS=1 bash "$t/job.sh" > "$t/out" 2>&1; rc=$?
log=$(find "$RUNLOG_ROOT/selftest" -name '*-fixture-job.log' | head -1)
[ -n "$log" ] || fail "no log under $RUNLOG_ROOT/selftest/<date>/<HHMM>-fixture-job.log: $(cat "$t/out")"
case "$log" in "$RUNLOG_ROOT/selftest/$(date +%F)/"[0-9][0-9][0-9][0-9]-fixture-job.log) ;; *) fail "log path $log";; esac
[ "$rc" != 0 ] || fail "run with a zero-test step exited 0"
grep -q "zero rc=1" "$t/out" || fail "zero-test step did not return 1: $(cat "$t/out")"
grep -q "row present during run" "$t/out" || fail "RUNNING.tsv row missing during the run"
for k in "command:" "repo:" "commit:" "rustc:" "CARGO_TARGET_DIR:" "start:"; do grep -q "^# $k" "$log" || fail "header lacks $k"; done
grep -q "^step zero FAIL .*tests run=0 .*zero tests" "$log" || fail "zero step not FAIL with run=0: $(cat "$log")"
grep -q "^step good PASS rc=0 dur=[0-9]*s tests run=6 passed=6 failed=0 ignored=2" "$log" || fail "good step counts: $(grep '^step good' "$log")"
grep -q "^step good start " "$log" || fail "no step start line"
grep -q "^beat .*step=good.*current=test a::b.*elapsed=[0-9]*s" "$log" || fail "no heartbeat naming the step and current test: $(cat "$log")"
grep -q "^# result: FAIL total=[0-9]*s" "$log" || fail "footer: $(tail -3 "$log")"
grep -q "^# warning: fixture warning" "$log" || fail "warning missing from footer"
grep -q "fixture-job" "$RUNLOG_ROOT/RUNNING.tsv" && fail "row not removed on exit"
grep -q "^paul	other-job	" "$RUNLOG_ROOT/RUNNING.tsv" || fail "another agent's row was removed"
[ ! -e "$RUNLOG_ROOT/selftest/2000-01-01/0000-old.log" ] || fail "old log of mine not pruned"
[ -e "$RUNLOG_ROOT/paul/2000-01-01/0000-old.log" ] || fail "another agent's old log was pruned"
[ -e "$RUNLOG_ROOT/selftest/2000-01-01/0001-new.log" ] || fail "fresh log pruned"

# a passing run: footer PASS; last-beat column updated while running
cat > "$t/ok.sh" <<'JOB'
#!/usr/bin/env bash
. "$RUNLOG_LIB"; runlog_wrap ok-job "$@"
runlog_step --tests t -- bash -c 'sleep 3; echo "test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"'
awk -F'\t' '$2=="ok-job"{print "beatcol " $6}' "$RUNLOG_ROOT/RUNNING.tsv"
JOB
RUNLOG_LIB="$here/runlog.sh" RUNLOG_BEAT_SECS=1 bash "$t/ok.sh" > "$t/ok.out" 2>&1 || fail "passing run exited nonzero: $(cat "$t/ok.out")"
start=$(awk -F'\t' '$2=="ok-job"{print $5}' "$RUNLOG_ROOT/RUNNING.tsv")
[ -z "$start" ] || fail "ok-job row left behind"
grep -q "^beatcol 20" "$t/ok.out" || fail "last-beat column empty: $(cat "$t/ok.out")"
# Piped output must end when the run ends: the heartbeat must not hold the pipe open (default 300 s beat).
p0=$(date +%s); RUNLOG_LIB="$here/runlog.sh" bash "$t/ok.sh" 2>&1 | cat > /dev/null
[ $(( $(date +%s) - p0 )) -lt 30 ] || fail "piped run held its output pipe open after exit (heartbeat)"
okl=$(find "$RUNLOG_ROOT/selftest" -name '*-ok-job.log' | head -1)
grep -q "^# result: PASS" "$okl" || fail "passing footer: $(tail -2 "$okl")"

# TERM: the row is removed and the footer says FAIL
cat > "$t/term.sh" <<'JOB'
#!/usr/bin/env bash
. "$RUNLOG_LIB"; runlog_wrap term-job "$@"
runlog_step slow -- sleep 30
JOB
RUNLOG_LIB="$here/runlog.sh" bash "$t/term.sh" > /dev/null 2>&1 & p=$!
for _ in 1 2 3 4 5 6 7 8 9 10; do grep -q term-job "$RUNLOG_ROOT/RUNNING.tsv" && break; sleep 0.5; done
grep -q term-job "$RUNLOG_ROOT/RUNNING.tsv" || fail "term-job row never added"
kill -TERM "$p"; wait "$p"
grep -q term-job "$RUNLOG_ROOT/RUNNING.tsv" && fail "row not removed on TERM"
tl=$(find "$RUNLOG_ROOT/selftest" -name '*-term-job.log' | head -1)
grep -q "^# result: FAIL" "$tl" || fail "TERM footer: $(tail -2 "$tl")"

# nested: a wrapped script started inside a wrapped run writes no log of its own
cat > "$t/outer.sh" <<'JOB'
#!/usr/bin/env bash
. "$RUNLOG_LIB"; runlog_wrap outer-job "$@"
bash "$T_DIR/ok.sh" > /dev/null
JOB
before=$(find "$RUNLOG_ROOT/selftest" -name '*-ok-job.log' | wc -l)
T_DIR="$t" RUNLOG_LIB="$here/runlog.sh" bash "$t/outer.sh" > /dev/null 2>&1 || fail "outer run failed"
[ "$(find "$RUNLOG_ROOT/selftest" -name '*-ok-job.log' | wc -l)" = "$before" ] || fail "nested run opened its own log"
# one-shot mode: tools/runlog.sh JOB --tests -- CMD
bash "$here/runlog.sh" oneshot --tests -- bash -c 'echo "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"' > /dev/null 2>&1 && fail "one-shot zero-test run passed"
echo "runlog selftest: ok"
