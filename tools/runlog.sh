#!/usr/bin/env bash
# zb's thin layer over the shared Pop logger ~/pop-logs/bin/poplog.sh (Rusty's; see ~/pop-logs/README.md; Jeff's rules of 9:42 and 9:59 AM CT
# 2026-10-23, and 10:33 AM CT: rely on poplog, no workarounds). poplog writes the log (header, steps, counts, heartbeat, footer), owns the
# canonical RUNNING.tsv row and its lock, tees step output to stdout, joins nested runs to the parent's log (SUB-RUN sections) and fails the
# run (footer FAIL, non-zero exit) when any step or sub-run failed. This file only adds what zb needs on top:
#   - a wrapped script registers its own exit cleanup with runlog_on_exit CMD, never a raw `trap ... EXIT`, and never execs a command
#   - runlog_wrap opens the run (or the sub-run, when started inside poprun or another wrapped script) and, at exit, finishes it and exits
#     non-zero if any step failed, even when the script itself ends with exit 0;
#   - runlog_step always passes --tests when asked, so a test step that ran zero tests is FAIL;
#   - if poplog.sh is missing the script warns and runs its steps plain.
# Use, first thing in a script:   . "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap JOB "$@"
#   runlog_step [--tests] NAME -- CMD...     runlog_warn MESSAGE
# One-shot (verify and test runs): tools/runlog.sh JOB [--tests] -- CMD...
# Env: RUNLOG_AGENT (default harper), RUNLOG_POPLOG (default ~/pop-logs/bin/poplog.sh); poplog's POPLOG_ROOT, POPLOG_HEARTBEAT, POPLOG_ECHO apply.
RUNLOG_AGENT=${RUNLOG_AGENT:-harper}
RUNLOG_POPLOG=${RUNLOG_POPLOG:-$HOME/pop-logs/bin/poplog.sh}
RUNLOG_ON=""
RUNLOG_EXIT_CMDS=()

_rl_exit() { # EXIT trap: finish the (sub-)run; a failed step makes the script exit non-zero
  local rc=$? c
  for c in "${RUNLOG_EXIT_CMDS[@]}"; do eval "$c"; done
  [ -n "$RUNLOG_ON" ] || exit "$rc"
  poplog_finish "$rc"
  if [ "$rc" = 0 ] && [ "${POPLOG_OVERALL:-0}" != 0 ]; then exit 1; fi
  exit "$rc"
}

runlog_wrap() { # runlog_wrap JOB "$@"
  local job=$1; shift
  trap _rl_exit EXIT
  if [ ! -f "$RUNLOG_POPLOG" ]; then echo "runlog: WARNING $RUNLOG_POPLOG missing; running $job without a run log" >&2; return 0; fi
  # shellcheck source=/dev/null
  . "$RUNLOG_POPLOG" || { echo "runlog: WARNING cannot source $RUNLOG_POPLOG; running $job without a run log" >&2; return 0; }
  poplog_init "$RUNLOG_AGENT" "$job" "$(git rev-parse --show-toplevel 2> /dev/null || pwd)" "$0 $*"
  trap _rl_exit EXIT
  RUNLOG_ON=1
  [ "${POPLOG_NESTED:-0}" = 1 ] || echo "runlog: $POPLOG_FILE" >&2
}

runlog_on_exit() { # runlog_on_exit CMD: run CMD at exit, before the run log is finished (a raw `trap ... EXIT` would replace the finish)
  RUNLOG_EXIT_CMDS+=("$1")
}

runlog_detach_children() { # for self-tests: fixture runs (some fail on purpose) open their own logs instead of joining this run as sub-runs
  export -n POPLOG_FILE POPLOG_STATE_DIR POPLOG_PID POPLOG_PROGRESS_FILE 2> /dev/null || true
}

runlog_warn() {
  if [ -n "$RUNLOG_ON" ]; then POPLOG_WARNINGS+=("$*"); _pl_log "WARNING $*"; fi
  echo "warning: $*" >&2
}

runlog_step() { # runlog_step [--tests] NAME -- CMD...
  local tflag=() name
  [ "${1:-}" = --tests ] && { tflag=(--tests); shift; }
  name=$1; shift; [ "${1:-}" = -- ] && shift
  if [ -z "$RUNLOG_ON" ]; then "$@"; return $?; fi
  poplog_run_step "$name" "${tflag[@]}" -- "$@"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  job=${1:?usage: tools/runlog.sh JOB [--tests] -- CMD...}; shift
  tf=(); [ "${1:-}" = --tests ] && { tf=(--tests); shift; }; [ "${1:-}" = -- ] && shift
  runlog_wrap "$job" "$@"
  runlog_step "${tf[@]}" "$job" -- "$@"
  exit $?
fi
