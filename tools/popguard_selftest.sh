#!/usr/bin/env bash
# Self-test for the Pop direct-run guard (~/pop-logs/bin/popguard.sh; 11:49 2026-10-24 incident: a bare 58 GB goldens python run
# global-OOMed Pop). Heavy golden generation must refuse (exit 3, clear message) outside a popslot/poprun memory scope:
# tools/goldens_regen.sh before it starts any generator, and scripts/goldlib.py (imported by every mpmath generator) at import.
# Proves: no POPSLOT_SCOPE -> refused, nothing ran; a forged POPSLOT_SCOPE (not our cgroup) -> refused; inside a real
# popslot-*.scope -> runs; no popguard.sh (CI, other hosts) -> runs unguarded. Uses its own throwaway systemd --user scope, no gate slot.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap popguard_selftest "$@"   # run log (tools/runlog.sh)
set -u
here="$(cd "$(dirname "$0")" && pwd)"; repo="$(cd "$here/.." && pwd)"
t="$(mktemp -d "$(zb_scratch)/popguard-selftest.XXXXXX")"; runlog_on_exit "rm -rf \"$t\""
. "${BASH_SOURCE[0]%/*}/selftest_cases.sh"; st_init "$t"
fail() { echo "SELFTEST FAIL: $*"; exit 1; }
PG=${POPGUARD:-$HOME/pop-logs/bin/popguard.sh}
[ -x "$PG" ] || { echo "popguard selftest: skipped ($PG missing: not Pop)"; exit 0; }
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
# fixture: one slice whose generator is a stub interpreter that only leaves a marker
root="$t/r"; d="$root/tests/goldens/slices/x"; mkdir -p "$d"; printf 'gen_x.py\n' > "$d/GENERATORS"; printf '# EMPTY\n' > "$d/SHA256SUMS"
stub="$t/stubpy"; printf '#!/bin/sh\ntouch "%s/ran"\n' "$t" > "$stub"; chmod +x "$stub"
gr() { GOLDENS_ROOT="$root" GOLDENS_PYTHON="$stub" RUNLOG_PLAIN=1 RUNLOG_SELFTEST=1 "$@" "$here/goldens_regen.sh" --slices-only 2>&1; }
inscope() { local u="popslot-zbselftest-$$-$RANDOM.scope"; POPSLOT_SCOPE=$u systemd-run --user --scope -q --collect "--unit=$u" -p MemoryMax=256M -p MemorySwapMax=0 -- "$@"; }
ck; rm -f "$t/ran"; out=$(gr env -u POPSLOT_SCOPE); rc=$?; [ "$rc" = 3 ] || fail "bare goldens_regen: want exit 3, got $rc: $out"
ck; printf '%s\n' "$out" | grep -q "REFUSING.*goldens_regen.*popslot" || fail "bare goldens_regen: unclear refusal: $out"
ck; [ ! -e "$t/ran" ] || fail "bare goldens_regen still started a generator"
ck; out=$(gr env POPSLOT_SCOPE=popslot-forged.scope); rc=$?; [ "$rc" = 3 ] || fail "forged POPSLOT_SCOPE accepted (rc $rc): $out"
ck; rm -f "$t/ran"; out=$(gr inscope env); rc=$?; [ "$rc" = 0 ] && [ -e "$t/ran" ] || fail "goldens_regen inside a popslot scope did not run (rc $rc): $out"
ck; rm -f "$t/ran"; out=$(gr env -u POPSLOT_SCOPE POPGUARD="$t/none"); rc=$?; [ "$rc" = 0 ] && [ -e "$t/ran" ] || fail "no popguard.sh: goldens_regen should run unguarded (rc $rc): $out"
ck; out=$(cd "$repo" && env -u POPSLOT_SCOPE PYTHONNOUSERSITE=1 /usr/bin/python3 scripts/goldlib.py --selftest 2>&1); rc=$?
  [ "$rc" = 3 ] && printf '%s\n' "$out" | grep -q REFUSING || fail "bare goldlib import: want popguard refusal exit 3, got $rc: $out"
ck; out=$(cd "$repo" && inscope env PYTHONNOUSERSITE=1 /usr/bin/python3 scripts/goldlib.py --selftest 2>&1); rc=$?
  printf '%s\n' "$out" | grep -q REFUSING && fail "goldlib refused inside a popslot scope: $out"
echo "popguard selftest: ok"
st_done popguard 8   # WANT: cases this script runs (tools/selftest_cases.sh)
