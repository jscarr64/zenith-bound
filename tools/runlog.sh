#!/usr/bin/env bash
# Run log for every zb run on Pop (Jeff 9:42 AM CT 2026-10-23). Two ways to use it:
#   sourced, first thing in a script:   . "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap JOB "$@"
#       then wrap steps:                runlog_step [--tests] NAME -- CMD...     (output goes to stdout and the log)
#                                       runlog_warn MESSAGE
#   one-shot (verify and test runs):    tools/runlog.sh JOB [--tests] -- CMD...
# Log: $RUNLOG_ROOT/$RUNLOG_AGENT/<YYYY-MM-DD>/<HHMM>-<JOB>.log (defaults ~/pop-logs, harper); this agent's logs older than 30 days are deleted.
# Header: command, repo, commit, rustc --version, CARGO_TARGET_DIR, start. Each step: start, end, duration, exit code and, from cargo's
# `test result:` lines, tests run (passed + failed), passed, failed, ignored; a --tests step that runs zero tests is FAIL (rc 1).
# Heartbeat every RUNLOG_BEAT_SECS (default 300): current step, current test or record if known, elapsed. Footer: PASS/FAIL, total, warnings.
# The run exits nonzero if the script did or any step failed. RUNNING.tsv under $RUNLOG_ROOT (columns: agent job pid log start last_beat) gets a
# row at start and loses it at exit (EXIT, INT, TERM, ERR traps), always under flock $RUNLOG_ROOT/.running.lock; other rows are kept.
# Stale-row flagging (15 min) is the shared watcher's job, not this script's. A wrapped script started inside a wrapped run opens no log.
RUNLOG_ROOT=${RUNLOG_ROOT:-$HOME/pop-logs}
RUNLOG_AGENT=${RUNLOG_AGENT:-harper}
RUNLOG_BEAT_SECS=${RUNLOG_BEAT_SECS:-300}

_rl_now() { date +%Y-%m-%dT%H:%M:%S%z; }
_rl_log() { [ -n "${RUNLOG_FILE:-}" ] && printf '%s\n' "$*" >> "$RUNLOG_FILE"; return 0; }
_rl_tsv() { # _rl_tsv add ROW | del PID LOG | beat PID LOG TIME
  local f="$RUNLOG_ROOT/RUNNING.tsv" tmp
  (
    flock 7 || exit 1
    touch "$f"; tmp="$f.tmp.$$"
    case $1 in
      add) printf '%s\n' "$2" >> "$f" ;;
      del) awk -F'\t' -v p="$2" -v l="$3" '!($3==p && $4==l)' "$f" > "$tmp" && cat "$tmp" > "$f" ;;
      beat) awk -F'\t' -v OFS='\t' -v p="$2" -v l="$3" -v b="$4" '$3==p && $4==l {$6=b} {print}' "$f" > "$tmp" && cat "$tmp" > "$f" ;;
    esac
    rm -f "$tmp"
  ) 7> "$RUNLOG_ROOT/.running.lock"
}

_rl_beat() { # background loop; $$ is the wrapper's pid
  local step out cur
  while sleep "$RUNLOG_BEAT_SECS"; do
    kill -0 "$$" 2> /dev/null || exit 0
    step=-; out=; cur=unknown
    if [ -f "$RUNLOG_TMP/state" ]; then IFS=$'\t' read -r step out < "$RUNLOG_TMP/state"; fi
    if [ -n "$out" ] && [ -f "$out" ]; then
      cur=$(grep -E '^test .* \.\.\.|case=|record' "$out" | tail -1); [ -n "$cur" ] || cur=unknown
    fi
    _rl_log "beat $(_rl_now) step=$step current=$cur elapsed=$(( $(date +%s) - RUNLOG_T0 ))s"
    _rl_tsv beat "$$" "$RUNLOG_FILE" "$(_rl_now)"
  done
}

_rl_end() { # _rl_end RC: footer, row removal; idempotent
  local rc=$1 result=PASS w
  [ -n "${_RL_DONE:-}" ] && return 0
  _RL_DONE=1; trap - ERR INT TERM
  [ -n "${_RL_HB:-}" ] && kill "$_RL_HB" 2> /dev/null
  [ -n "${_RL_CHILD:-}" ] && kill -TERM "$_RL_CHILD" 2> /dev/null
  { [ "$rc" = 0 ] && [ ! -s "$RUNLOG_TMP/fail" ]; } || result=FAIL
  _rl_log "# end: $(_rl_now)"
  [ -s "$RUNLOG_TMP/fail" ] && _rl_log "# failed steps: $(tr '\n' ' ' < "$RUNLOG_TMP/fail")"
  if [ -s "$RUNLOG_TMP/warn" ]; then while IFS= read -r w; do _rl_log "# warning: $w"; done < "$RUNLOG_TMP/warn"; else _rl_log "# warnings: none"; fi
  _rl_log "# result: $result total=$(( $(date +%s) - RUNLOG_T0 ))s exit=$rc"
  _rl_tsv del "$$" "$RUNLOG_FILE"
  rm -rf "$RUNLOG_TMP"
  [ "$result" = PASS ] || [ "$rc" != 0 ] || _RL_RC=1
  return 0
}

_rl_run() { # _rl_run JOB CMD...: the wrapper process
  local job=$1 day hm dir n=1 rc start; shift
  day=$(date +%F); hm=$(date +%H%M); dir="$RUNLOG_ROOT/$RUNLOG_AGENT/$day"
  find "$RUNLOG_ROOT/$RUNLOG_AGENT" -type f -name '*.log' -mtime +30 -delete 2> /dev/null
  find "$RUNLOG_ROOT/$RUNLOG_AGENT" -mindepth 1 -type d -empty -delete 2> /dev/null
  mkdir -p "$dir" || { echo "runlog: cannot create $dir" >&2; return 2; }
  RUNLOG_FILE="$dir/$hm-$job.log"
  while [ -e "$RUNLOG_FILE" ]; do n=$((n + 1)); RUNLOG_FILE="$dir/$hm-$job-$n.log"; done
  RUNLOG_TMP=$(mktemp -d /tmp/runlog.XXXXXX) || return 2
  RUNLOG_T0=$(date +%s); start=$(_rl_now)
  export RUNLOG_FILE RUNLOG_TMP RUNLOG_T0; unset RUNLOG_INNER
  {
    printf '# command: %s\n' "$(printf '%q ' "$@")"
    printf '# repo: %s\n' "$(git rev-parse --show-toplevel 2> /dev/null || pwd)"
    printf '# commit: %s\n' "$(git rev-parse HEAD 2> /dev/null || echo none)"
    printf '# rustc: %s\n' "$(rustc --version 2>&1 || echo 'not found')"
    printf '# CARGO_TARGET_DIR: %s\n' "${CARGO_TARGET_DIR:-unset}"
    printf '# start: %s\n' "$start"
    printf '# agent: %s pid: %s\n' "$RUNLOG_AGENT" "$$"
  } >> "$RUNLOG_FILE"
  _rl_tsv add "$(printf '%s\t%s\t%s\t%s\t%s\t%s' "$RUNLOG_AGENT" "$job" "$$" "$RUNLOG_FILE" "$start" "$start")"
  _rl_beat & _RL_HB=$!
  trap '_rl_end "$?"' EXIT
  trap '_rl_end 130; exit 130' INT
  trap '_rl_end 143; exit 143' TERM
  trap '_rl_end 1; exit 1' ERR
  echo "runlog: $RUNLOG_FILE" >&2
  "$@" 0<&0 & _RL_CHILD=$!
  if wait "$_RL_CHILD"; then rc=0; else rc=$?; fi
  _RL_CHILD=
  _RL_RC=$rc
  _rl_end "$rc"
  trap - EXIT
  return "$_RL_RC"
}

runlog_wrap() { # runlog_wrap JOB "$@": re-runs the calling script as a logged child; inside a run, returns at once
  local job=$1; shift
  if [ -n "${RUNLOG_FILE:-}" ]; then
    if [ -z "${RUNLOG_INNER:-}" ]; then export RUNLOG_INNER=1; else RUNLOG_NESTED=1; fi
    return 0
  fi
  _rl_run "$job" "$BASH" "$0" "$@"
  exit $?
}

runlog_warn() { if [ -n "${RUNLOG_TMP:-}" ] && [ -z "${RUNLOG_NESTED:-}" ]; then printf '%s\n' "$*" >> "$RUNLOG_TMP/warn"; _rl_log "warning: $*"; fi; echo "warning: $*" >&2; }

runlog_step() { # runlog_step [--tests] NAME -- CMD...
  local tests=0 name out t0 rc res counts zero=""
  [ "${1:-}" = --tests ] && { tests=1; shift; }
  name=$1; shift; [ "${1:-}" = -- ] && shift
  if [ -z "${RUNLOG_FILE:-}" ] || [ -n "${RUNLOG_NESTED:-}" ] || [ -z "${RUNLOG_TMP:-}" ]; then "$@"; return $?; fi
  out="$RUNLOG_TMP/step-$name.out"; t0=$(date +%s)
  _rl_log "step $name start $(_rl_now)"
  printf '%s\t%s\n' "$name" "$out" > "$RUNLOG_TMP/state"
  "$@" 2>&1 | tee "$out"; rc=${PIPESTATUS[0]}
  counts=$(awk '/test result:/ { for (i = 1; i <= NF; i++) { if ($(i+1) ~ /^passed/) p += $i; if ($(i+1) ~ /^failed/) f += $i; if ($(i+1) ~ /^ignored/) g += $i }; seen = 1 }
                END { if (seen) printf "%d %d %d", p, f, g; else printf "0 0 0" }' "$out")
  local np nf ni; read -r np nf ni <<< "$counts"; set -- "$np" "$nf" "$ni"
  [ "$tests" = 1 ] && [ $(($1 + $2)) = 0 ] && { zero=" (zero tests: FAIL)"; [ "$rc" = 0 ] && rc=1; }
  res=PASS; [ "$rc" = 0 ] || { res=FAIL; echo "$name" >> "$RUNLOG_TMP/fail"; }
  if [ "$tests" = 1 ] || grep -q 'test result:' "$out"; then counts=" tests run=$(($1 + $2)) passed=$1 failed=$2 ignored=$3"; else counts=""; fi
  _rl_log "step $name end $(_rl_now)"
  _rl_log "step $name $res rc=$rc dur=$(( $(date +%s) - t0 ))s$counts$zero"
  : > "$RUNLOG_TMP/state"
  return "$rc"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  if [ "${1:-}" = __step ]; then shift; export RUNLOG_INNER=1; runlog_step "$@"; exit $?; fi
  job=${1:?usage: tools/runlog.sh JOB [--tests] -- CMD...}; shift
  tf=(); [ "${1:-}" = --tests ] && { tf=(--tests); shift; }; [ "${1:-}" = -- ] && shift
  [ -z "${RUNLOG_FILE:-}" ] || { runlog_step "${tf[@]}" "$job" -- "$@"; exit $?; }
  _rl_run "$job" "$BASH" "$0" __step "${tf[@]}" "$job" -- "$@"
  exit $?
fi
