#!/usr/bin/env bash
# zb's thin layer over the shared Pop logger ~/pop-logs/bin/poplog.sh (Rusty's; see ~/pop-logs/README.md; Jeff's rules of 9:42 and 9:59 AM CT
# 2026-10-23). poplog writes the log (~/pop-logs/<agent>/<date>/<HHMM>-<job>.log: header, steps, counts, heartbeat, footer), owns the
# canonical RUNNING.tsv row and its lock, and prunes old logs (poprotate). This file only adds what zb needs on top:
#   - step output still goes to stdout (callers redirect it, e.g. full_gate.sh's per-step logs), and also into the run log;
#   - a failed or zero-test step returns nonzero, and the run exits 1 when any step failed even if the script exits 0;
#   - a wrapped script started inside a wrapped run, or inside poprun, opens no log of its own: its steps go into the outer log
#     (the first script under the context logs steps; deeper ones run their steps plain);
#   - the heartbeat is kept off the run's stdout, so a reader of a piped run is not held open by the heartbeat's sleep.
# Use, first thing in a script:   . "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap JOB "$@"
#   runlog_step [--tests] NAME -- CMD...     runlog_warn MESSAGE
# One-shot (verify and test runs): tools/runlog.sh JOB [--tests] -- CMD...
# Env: RUNLOG_AGENT (default harper), RUNLOG_POPLOG (default ~/pop-logs/bin/poplog.sh); poplog's POPLOG_ROOT and POPLOG_HEARTBEAT apply.
# Nested/no-op paths use only builtins (full_gate_selftest runs full_gate.sh with a stripped PATH).
RUNLOG_AGENT=${RUNLOG_AGENT:-harper}
RUNLOG_POPLOG=${RUNLOG_POPLOG:-$HOME/pop-logs/bin/poplog.sh}

_rl_ctx() { [ -n "${POPLOG_FILE:-}" ] && [ -n "${POPLOG_STATE_DIR:-}" ] && [ -d "$POPLOG_STATE_DIR" ]; }
_rl_lib() { declare -F log_step_end > /dev/null && return 0
  # shellcheck source=/dev/null
  . "$RUNLOG_POPLOG"; }

_rl_run() { # _rl_run JOB CMD...: a poplog run whose only work is CMD (the re-run script, or a one-shot step)
  local job=$1 rc=0 w; shift
  _rl_lib || { echo "runlog: cannot source $RUNLOG_POPLOG" >&2; return 2; }
  poplog_init "$RUNLOG_AGENT" "$job" "$(git rev-parse --show-toplevel 2> /dev/null || pwd)" "$(printf '%q ' "$@")" < /dev/null > /dev/null 2>&1
  unset RUNLOG_INNER
  echo "runlog: $POPLOG_FILE" >&2
  "$@" 0<&0 & POPLOG_CHILD_PID=$!
  wait "$POPLOG_CHILD_PID" || rc=$?
  POPLOG_CHILD_PID=""
  if [ -s "$POPLOG_STATE_DIR/zb-warn" ]; then while IFS= read -r w; do POPLOG_WARNINGS+=("$w"); done < "$POPLOG_STATE_DIR/zb-warn"; fi
  if [ -s "$POPLOG_STATE_DIR/zb-fail" ]; then
    while IFS= read -r w; do POPLOG_WARNINGS+=("$w"); done < "$POPLOG_STATE_DIR/zb-fail"
    [ "$rc" = 0 ] && rc=1
  fi
  poplog_finish "$rc"
  trap - EXIT INT TERM HUP
  return "$rc"
}

runlog_wrap() { # runlog_wrap JOB "$@"
  local job=$1; shift
  if _rl_ctx; then
    if [ -z "${RUNLOG_INNER:-}" ]; then export RUNLOG_INNER=1; else RUNLOG_NESTED=1; fi
    return 0
  fi
  if [ ! -f "$RUNLOG_POPLOG" ]; then echo "runlog: WARNING $RUNLOG_POPLOG missing; running $job without a run log" >&2; RUNLOG_NESTED=1; return 0; fi
  _rl_run "$job" "$BASH" "$0" "$@"
  exit $?
}

runlog_warn() {
  if _rl_ctx && [ -z "${RUNLOG_NESTED:-}" ]; then printf '%s\n' "$*" >> "$POPLOG_STATE_DIR/zb-warn"; printf '[%s] WARNING %s\n' "$(date +%H:%M:%S)" "$*" >> "$POPLOG_FILE"; fi
  echo "warning: $*" >&2
}

runlog_step() { # runlog_step [--tests] NAME -- CMD...
  local tflag="" name rc
  [ "${1:-}" = --tests ] && { tflag=--tests; shift; }
  name=$1; shift; [ "${1:-}" = -- ] && shift
  if ! _rl_ctx || [ -n "${RUNLOG_NESTED:-}" ]; then "$@"; return $?; fi
  _rl_lib || { "$@"; return $?; }
  log_step_start "$name"; _pl_log "CMD $*"
  "$@" 2>&1 | tee -a "$POPLOG_STATE_DIR/out" "$POPLOG_FILE"; rc=${PIPESTATUS[0]}
  if log_step_end "$name" "$rc" "$POPLOG_STATE_DIR/out" "$tflag"; then return 0; fi
  echo "step $name FAIL (exit $rc)" >> "$POPLOG_STATE_DIR/zb-fail"
  [ "$rc" = 0 ] && rc=1
  return "$rc"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  if [ "${1:-}" = __step ]; then shift; export RUNLOG_INNER=1; runlog_step "$@"; exit $?; fi
  job=${1:?usage: tools/runlog.sh JOB [--tests] -- CMD...}; shift
  tf=(); [ "${1:-}" = --tests ] && { tf=(--tests); shift; }; [ "${1:-}" = -- ] && shift
  if _rl_ctx; then runlog_step "${tf[@]}" "$job" -- "$@"; exit $?; fi
  _rl_run "$job" "$BASH" "$0" __step "${tf[@]}" "$job" -- "$@"
  exit $?
fi
