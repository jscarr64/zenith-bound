#!/usr/bin/env bash
# Self-test of tools/runlog.sh, zb's thin layer over the shared Pop logger ~/pop-logs/bin/poplog.sh (Jeff 9:42 and 9:59 AM CT 2026-10-23).
# Fixtures only, under a throwaway POPLOG_ROOT, using the real poplog.sh: a --tests step that runs zero tests is FAIL and returns 1, a passing
# cargo-style step is counted, the canonical 8-column RUNNING.tsv row (ISO-8601 times with offset, state RUNNING, slot -) is added and removed
# on exit and on TERM with other agents' rows kept, the heartbeat fires (POPLOG_HEARTBEAT=1), header and footer are poplog's, a piped run
# ends when the run ends, a wrapped script inside a wrapped run or inside poprun opens no log of its own, a nested full_gate under poprun
# with a failing step exits non-zero with FAIL and the step's output in poprun's log, a wrapped script's own cleanup (runlog_on_exit) still lets the run finish, no wrapped tool sets a raw EXIT
# trap or execs a command, and one-shot mode fails zero tests.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap runlog_selftest "$@"   # run log (tools/runlog.sh)
runlog_detach_children
set -u
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
fail() { echo "runlog selftest FAIL: $*"; exit 1; }
t=$(mktemp -d /tmp/zb-runlog-selftest.XXXXXX); runlog_on_exit "rm -rf \"$t\""
# fixtures run in a subshell: they repoint POPLOG_ROOT and unset POPLOG_*, which this script's own run log still needs at exit
(
lib=${RUNLOG_POPLOG:-$HOME/pop-logs/bin/poplog.sh}
[ -f "$lib" ] || fail "shared logger $lib missing"
export POPLOG_ROOT="$t/pop-logs" RUNLOG_AGENT=selftest RUNLOG_LIB="$here/runlog.sh"
unset POPLOG_FILE POPLOG_PID POPLOG_STATE_DIR POPLOG_PROGRESS_FILE POPLOG_RUNNING RUNLOG_INNER RUNLOG_ACTIVE
mkdir -p "$POPLOG_ROOT"
hdr=$'#agent\tjob\tpid\tlog\tstart\tlast_heartbeat\tstate\tslot'
printf '%s\npaul\tother-job\t1\t/x.log\t2026-01-01T00:00:00-06:00\t2026-01-01T00:00:00-06:00\tRUNNING\t-\n' "$hdr" > "$POPLOG_ROOT/RUNNING.tsv"
iso='20[0-9][0-9]-[0-9][0-9]-[0-9][0-9]T[0-9][0-9]:[0-9][0-9]:[0-9][0-9][-+][0-9][0-9]:[0-9][0-9]'

cat > "$t/job.sh" <<'JOB'
#!/usr/bin/env bash
. "$RUNLOG_LIB"; runlog_wrap fixture-job "$@"
runlog_step --tests zero -- bash -c 'echo "running 0 tests"; echo "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"'
echo "zero rc=$?"
runlog_step --tests good -- bash -c 'echo "test a::b ... ok"; sleep 3; echo "test result: ok. 5 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out"; echo "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"'
awk -F'\t' '$2=="fixture-job" {print "row " NF " " $5 " " $6 " " $7 " " $8}' "$POPLOG_ROOT/RUNNING.tsv"
runlog_warn "fixture warning"
JOB
POPLOG_HEARTBEAT=1 bash "$t/job.sh" > "$t/out" 2>&1; rc=$?
log=$(find "$POPLOG_ROOT/selftest" -name '*-fixture-job.log' | head -1)
[ -n "$log" ] || fail "no log under $POPLOG_ROOT/selftest/<date>/<HHMM>-fixture-job.log: $(cat "$t/out")"
case "$log" in "$POPLOG_ROOT/selftest/$(date +%F)/"[0-9][0-9][0-9][0-9]-fixture-job.log) ;; *) fail "log path $log";; esac
[ "$rc" != 0 ] || fail "run with a zero-test step exited 0"
grep -q "zero rc=1" "$t/out" || fail "zero-test step did not return 1: $(cat "$t/out")"
grep -Eq "^row 8 $iso $iso RUNNING -$" "$t/out" || fail "RUNNING.tsv row not canonical during the run: $(grep '^row' "$t/out")"
grep -q "test a::b ... ok" "$t/out" || fail "step output not on stdout"
grep -q "===== POPLOG HEADER" "$log" || fail "not a poplog log: $(head -3 "$log")"
for k in agent job command repo commit rustc CARGO_TARGET_DIR start; do grep -q "^$k: " "$log" || fail "header lacks $k"; done
grep -q "STEP END zero verdict=FAIL .*run=0 " "$log" || fail "zero step not FAIL: $(cat "$log")"
grep -q "STEP END good verdict=PASS exit=0 .*tests: run=6 passed=6 failed=0 ignored=2" "$log" || fail "good step counts: $(grep 'STEP END good' "$log")"
grep -q "STEP START good" "$log" || fail "no step start line"
grep -q "HEARTBEAT .*step=good .*current_test=test a::b" "$log" || fail "no heartbeat naming the step and test: $(cat "$log")"
grep -q "^RESULT: FAIL" "$log" || fail "footer: $(tail -6 "$log")"
grep -q "fixture warning" "$log" || fail "warning missing"
grep -q "fixture-job" "$POPLOG_ROOT/RUNNING.tsv" && fail "row not removed on exit"
grep -q "^paul	other-job	" "$POPLOG_ROOT/RUNNING.tsv" || fail "another agent's row was removed"
[ "$(head -1 "$POPLOG_ROOT/RUNNING.tsv")" = "$hdr" ] || fail "RUNNING.tsv header changed"

cat > "$t/ok.sh" <<'JOB'
#!/usr/bin/env bash
. "$RUNLOG_LIB"; runlog_wrap ok-job "$@"
runlog_step --tests t -- bash -c 'sleep 2; echo "test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"'
JOB
bash "$t/ok.sh" > "$t/ok.out" 2>&1 || fail "passing run exited nonzero: $(cat "$t/ok.out")"
grep -q ok-job "$POPLOG_ROOT/RUNNING.tsv" && fail "ok-job row left behind"
okl=$(find "$POPLOG_ROOT/selftest" -name '*-ok-job.log' | head -1)
grep -q "^RESULT: PASS" "$okl" || fail "passing footer: $(tail -6 "$okl")"
# piped output must end when the run ends (the 300 s heartbeat must not hold the pipe)
p0=$(date +%s); bash "$t/ok.sh" 2>&1 | cat > /dev/null
[ $(( $(date +%s) - p0 )) -lt 30 ] || fail "piped run held its output pipe open after exit (heartbeat)"

cat > "$t/term.sh" <<'JOB'
#!/usr/bin/env bash
. "$RUNLOG_LIB"; runlog_wrap term-job "$@"
runlog_step slow -- sleep 30
JOB
bash "$t/term.sh" > /dev/null 2>&1 & p=$!
for _ in $(seq 20); do grep -q term-job "$POPLOG_ROOT/RUNNING.tsv" && break; sleep 0.5; done
grep -q term-job "$POPLOG_ROOT/RUNNING.tsv" || fail "term-job row never added"
sleep 1; kill -TERM "$p"; wait "$p"
grep -q term-job "$POPLOG_ROOT/RUNNING.tsv" && fail "row not removed on TERM"
tl=$(find "$POPLOG_ROOT/selftest" -name '*-term-job.log' | head -1)
grep -q "^RESULT: FAIL" "$tl" || fail "TERM footer: $(tail -6 "$tl")"

# nested inside a wrapped run, and inside poprun: no log of its own; the steps land in the outer log
cat > "$t/outer.sh" <<'JOB'
#!/usr/bin/env bash
. "$RUNLOG_LIB"; runlog_wrap outer-job "$@"
bash "$T_DIR/ok.sh" > /dev/null
JOB
before=$(find "$POPLOG_ROOT/selftest" -name '*-ok-job.log' | wc -l)
T_DIR="$t" bash "$t/outer.sh" > /dev/null 2>&1 || fail "outer run failed"
"$(dirname "$lib")/poprun" --agent selftest --job pr-outer -- bash "$t/ok.sh" > /dev/null 2>&1 || fail "poprun outer failed"
[ "$(find "$POPLOG_ROOT/selftest" -name '*-ok-job.log' | wc -l)" = "$before" ] || fail "nested run opened its own log"
prl=$(find "$POPLOG_ROOT/selftest" -name '*-pr-outer.log' | head -1)
grep -q "STEP END t verdict=PASS" "$prl" || fail "step of a script under poprun not in poprun's log: $(cat "$prl")"
# a nested full_gate under poprun with one failing step: non-zero exit, FAIL in poprun's log, the step's output in that log
fx="$t/fx"; mkdir -p "$fx/src"; printf 'fn f() { unsafe { } }\n' > "$fx/src/lib.rs"
"$(dirname "$lib")/poprun" --agent selftest --job pr-gate -- env REPO="$fx" GATE_STEPS="unsafe-scan" T="$t/gl" bash "$here/full_gate.sh" fx > "$t/gate.out" 2>&1 \
  && fail "nested full_gate with a failing step exited 0 under poprun: $(cat "$t/gate.out")"
gl=$(find "$POPLOG_ROOT/selftest" -name '*-pr-gate.log' | head -1)
[ -n "$gl" ] || fail "poprun wrote no pr-gate log"
grep -q "SUB-RUN START selftest/full_gate" "$gl" || fail "full_gate did not join poprun's log as a sub-run: $(cat "$gl")"
grep -q "STEP END unsafe-scan verdict=FAIL" "$gl" || fail "failing gate step not FAIL in the log: $(cat "$gl")"
grep -q "src/lib.rs:1:fn f() { unsafe" "$gl" || fail "the failing step's output is not in the log: $(cat "$gl")"
grep -q "^RESULT: FAIL" "$gl" || fail "poprun footer not FAIL for a failed nested gate: $(tail -6 "$gl")"
find "$POPLOG_ROOT/selftest" -name '*-full_gate.log' | grep -q . && fail "nested full_gate opened its own log"
# a wrapped script's own cleanup goes through runlog_on_exit and the run still finishes: footer, cleanup ran, RUNNING.tsv row gone
cat > "$t/clean.sh" <<'JOB'
#!/usr/bin/env bash
. "$RUNLOG_LIB"; runlog_wrap clean-job "$@"
runlog_on_exit 'touch "$T_DIR/cleaned"'
echo body
JOB
T_DIR="$t" bash "$t/clean.sh" > "$t/clean.out" 2>&1 || fail "clean-job exited non-zero: $(cat "$t/clean.out")"
cl=$(find "$POPLOG_ROOT/selftest" -name '*-clean-job.log' | head -1)
[ -f "$t/cleaned" ] || fail "runlog_on_exit cleanup did not run"
grep -q "^RESULT: PASS" "$cl" || fail "run with an on-exit cleanup has no PASS footer: $(tail -4 "$cl")"
awk -F'\t' '$2=="clean-job"' "$POPLOG_ROOT/RUNNING.tsv" | grep -q . && fail "clean-job RUNNING.tsv row left behind"
# every wrapped tool must keep runlog's EXIT trap: no raw EXIT trap (it replaces the finish) and no exec of a command (no trap runs)
for f in "$here"/*.sh; do
  [ "$f" = "$here/runlog.sh" ] && continue
  grep -q 'runlog_wrap' "$f" || continue
  grep -nE "trap .*EXIT" "$f" | grep -v runlog_on_exit && fail "${f##*/} sets its own EXIT trap; use runlog_on_exit so the run log is finished"
  grep -nE '(^|[;&|]) *exec +[^0-9 <>]' "$f" && fail "${f##*/} execs a command, so the run log is never finished"
done
# one-shot mode
bash "$here/runlog.sh" oneshot --tests -- bash -c 'echo "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"' > /dev/null 2>&1 && fail "one-shot zero-test run passed"
find "$POPLOG_ROOT/selftest" -name '*-oneshot.log' | grep -q . || fail "one-shot wrote no log"
# missing poplog.sh is a hard failure (not a warning); RUNLOG_PLAIN=1 is the explicit fixture-only plain mode
env -u RUNLOG_PLAIN RUNLOG_POPLOG=/nonexistent bash "$here/runlog.sh" nolib -- true > "$t/nolib.out" 2>&1 && fail "a run with poplog.sh missing passed"
grep -q 'missing' "$t/nolib.out" || fail "missing poplog.sh not named: $(cat "$t/nolib.out")"
# zb's own 0-tests=FAIL rule holds without poplog too (plain mode), and a real test count passes
RUNLOG_PLAIN=1 RUNLOG_POPLOG=/nonexistent bash "$here/runlog.sh" plain0 --tests -- bash -c 'echo "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"' > "$t/p0.out" 2>&1 && fail "plain-mode zero-test step passed"
RUNLOG_PLAIN=1 RUNLOG_POPLOG=/nonexistent bash "$here/runlog.sh" plain1 --tests -- bash -c 'echo "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"' > "$t/p1.out" 2>&1 || fail "plain-mode passing test step failed: $(cat "$t/p1.out")"
echo "runlog selftest: ok"
)
