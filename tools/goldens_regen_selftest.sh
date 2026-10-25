#!/usr/bin/env bash
# Self-test for the per-slice checks of tools/goldens_regen.sh (Paul, 2026-10-24). Runs `goldens_regen.sh --slices-only` (no oracle,
# no generators, nothing under the repo written) on fixtures under $(zb_scratch). Proves: a SHA256SUMS with no entries passes only when
# it carries the explicit `# EMPTY` marker; a marked file that also lists entries FAILS; a malformed line FAILS (sha256sum --strict);
# a changed, unlisted or missing golden FAILS; an untracked *_heavy.txt (hashes pinned elsewhere, .gitignored) is not demanded; a slice
# without GENERATORS FAILS.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap goldens_regen_selftest "$@"   # run log (tools/runlog.sh)
set -u
unset ZB_GATE_MODE GOLDENS_REGEN_ARGS   # fixtures run regular mode even inside a merge-gate run (gate 1707: inherited ZB_GATE_MODE=merge refused every fixture)
here="$(cd "$(dirname "$0")" && pwd)"; t="$(mktemp -d "$(zb_scratch)/goldens-regen-selftest.XXXXXX")"; runlog_on_exit "rm -rf \"$t\""
. "${BASH_SOURCE[0]%/*}/selftest_cases.sh"; st_init "$t"
fail() { echo "SELFTEST FAIL: $*"; exit 1; }
n=0
fx() { n=$((n + 1)); root="$t/c$n"; d="$root/tests/goldens/slices/x"; mkdir -p "$d"; printf '# no generators\n' > "$d/GENERATORS"; }
gr() { GOLDENS_ROOT="$root" RUNLOG_PLAIN=1 RUNLOG_SELFTEST=1 "$here/goldens_regen.sh" --slices-only 2>&1; }
sum() { (cd "$d" && sha256sum "$1"); }
fx; printf '# header\n# EMPTY: no goldens yet\n' > "$d/SHA256SUMS"
ck; out=$(gr) || fail "a marked-empty SHA256SUMS failed: $out"
fx; printf '# header only\n' > "$d/SHA256SUMS"
ck; out=$(gr) && fail "an unmarked empty SHA256SUMS passed: $out"
ck; printf '%s\n' "$out" | grep -q "EMPTY" || fail "unmarked empty: message does not name the marker: $out"
fx; printf 'a\n' > "$d/a.txt"; { printf '# EMPTY\n'; sum a.txt; } > "$d/SHA256SUMS"
ck; out=$(gr) && fail "a marked-empty SHA256SUMS that lists entries passed: $out"
fx; printf 'a\n' > "$d/a.txt"; { sum a.txt; printf 'deadbeef  a.txt\n'; } > "$d/SHA256SUMS"
ck; out=$(gr) && fail "a malformed SHA256SUMS line passed: $out"
ck; printf '%s\n' "$out" | grep -q "malformed" || fail "malformed line: unclear message: $out"
fx; printf 'a\n' > "$d/a.txt"; sum a.txt > "$d/SHA256SUMS"
ck; out=$(gr) || fail "a correct SHA256SUMS failed: $out"
printf 'b\n' > "$d/a.txt"
ck; out=$(gr) && fail "a changed golden passed: $out"
fx; printf 'a\n' > "$d/a.txt"; printf 'b\n' > "$d/b.txt"; sum a.txt > "$d/SHA256SUMS"
ck; out=$(gr) && fail "an unlisted golden passed: $out"
fx; printf 'a\n' > "$d/a.txt"; printf 'h\n' > "$d/big_heavy.txt"; sum a.txt > "$d/SHA256SUMS"
ck; out=$(gr) || fail "an untracked *_heavy.txt was demanded in SHA256SUMS: $out"
fx; printf 'a\n' > "$d/a.txt"; sum a.txt > "$d/SHA256SUMS"; rm "$d/a.txt"
ck; out=$(gr) && fail "a listed but missing golden passed: $out"
fx; printf '# EMPTY\n' > "$d/SHA256SUMS"; rm "$d/GENERATORS"
ck; out=$(gr) && fail "a slice without GENERATORS passed: $out"
# Spence (2026-10-24, 1158-zb-l0a1-goldens-regen-proof): a '# EMPTY' slice made the ERR trap log `entries=$(grep ...)` rc=1 once per
# slice. Run with a real run log (fresh POPLOG_ROOT, not nested in this selftest's log): it passes and the log holds no ERR line.
fx; printf '# header\n# EMPTY: no goldens yet\n' > "$d/SHA256SUMS"
out=$( unset POPLOG_FILE POPLOG_PID POPLOG_STATE_DIR POPLOG_PROGRESS_FILE POPLOG_RUNNING RUNLOG_INNER RUNLOG_ACTIVE
       POPLOG_ROOT="$t/pl" RUNLOG_AGENT=selftest GOLDENS_ROOT="$root" "$here/goldens_regen.sh" --slices-only 2>&1 )
ck; [ $? = 0 ] || fail "logged marked-empty run failed: $out"
lg=$(find "$t/pl/selftest" -name '*-goldens_regen.log' | head -1)
ck; [ -n "$lg" ] || fail "logged marked-empty run: no run log: $out"
ck; grep -q ' ERR ' "$lg" && fail "a marked-empty slice logged ERR trap lines: $(grep ' ERR ' "$lg")"
ck; grep -q "slice x: SHA256SUMS marked '# EMPTY', skipped" <<< "$out" || fail "a marked-empty slice was not reported as skipped: $out"
# a '# EMPTY' slice with a golden next to it FAILS, naming the marker
fx; printf 'a\n' > "$d/a.txt"; printf '# EMPTY\n' > "$d/SHA256SUMS"
ck; out=$(gr) && fail "a marked-empty slice with a.txt passed: $out"
ck; printf '%s\n' "$out" | grep -q "marked '# EMPTY' but has a.txt" || fail "marked-empty with a.txt: unclear message: $out"
# '# EMPTY' plus a malformed line FAILS
fx; printf '# EMPTY\nnot a sum\n' > "$d/SHA256SUMS"
ck; out=$(gr) && fail "a marked-empty SHA256SUMS with a malformed line passed: $out"
# Harper (2026-10-24, gate 1520-zb-mutq-merge-full-gate: 5x `ERR rc=1 line=65 bad=$(... grep -v ...)`): when every SHA256SUMS line is
# valid, `grep -v` finds nothing (rc 1); that is the normal case and must log no ERR. A malformed line, or a real grep error (rc 2), stays loud.
lgr() { local n=$1; shift   # goldens_regen.sh with its own real run log (ERR trap on); sets out, grc, lg
  out=$( unset POPLOG_FILE POPLOG_PID POPLOG_STATE_DIR POPLOG_PROGRESS_FILE POPLOG_RUNNING RUNLOG_INNER RUNLOG_ACTIVE RUNLOG_PLAIN
         env "$@" POPLOG_ROOT="$t/pl-$n" RUNLOG_AGENT=selftest GOLDENS_ROOT="$root" "$here/goldens_regen.sh" --slices-only 2>&1 ); grc=$?
  lg=$(find "$t/pl-$n/selftest" -name '*-goldens_regen.log' 2>/dev/null | head -1); }
fx; printf 'a\n' > "$d/a.txt"; printf 'b\n' > "$d/b.txt"; { printf '# header\n'; sum a.txt; sum b.txt; } > "$d/SHA256SUMS"
lgr valid
ck; [ "$grc" = 0 ] && [ -n "$lg" ] || fail "logged all-valid run failed or wrote no run log (rc $grc): $out"
ck; grep -q ' ERR ' "$lg" && fail "an all-valid SHA256SUMS logged ERR trap lines: $(grep ' ERR ' "$lg")"
{ cat "$d/SHA256SUMS"; printf 'zz  a.txt\n'; } > "$d/S2"; mv "$d/S2" "$d/SHA256SUMS"
lgr badline
ck; [ "$grc" != 0 ] || fail "logged run with a malformed line passed: $out"
ck; grep -q "malformed SHA256SUMS line(s): zz  a.txt" <<< "$out" || fail "logged malformed line: not named: $out"
# a real grep error (rc 2) in the malformed-line check must fail the slice, not read as "no bad lines"
fx; printf 'a\n' > "$d/a.txt"; sum a.txt > "$d/SHA256SUMS"
mkdir -p "$t/shim"; rg_=$(command -v grep)
printf '#!/bin/sh\ncase "$*" in *"{64}  [^/"*) echo "grep: shim: simulated I/O error" >&2; exit 2;; esac\nexec %s "$@"\n' "$rg_" > "$t/shim/grep"; chmod +x "$t/shim/grep"
lgr grep2 PATH="$t/shim:$PATH"
ck; [ "$grc" != 0 ] || fail "a grep error (rc 2) in the malformed-line check passed: $out"
ck; grep -q "grep failed (rc 2)" <<< "$out" || fail "grep rc 2: no clear message: $out"
# Paul (2026-10-24, oracle python flat out 10+ min in every gate): goldens_regen keys on a sha256 of the generator scripts, their
# committed inputs (gate files, SHA256SUMS, HEAVY.SHA256, GENERATORS) and the oracle versions. Key unchanged since the last passing
# run: python is skipped entirely, only the pinned sums are verified (mode=cached). Key changed: python runs, heavy rows by pin only
# (--skip-heavy). --regen: full regen, heavy rows recomputed. ZB_GATE_MODE=merge REFUSES (exit 2) without --regen. Mode and reason logged.
mkdir -p "$t/venv/bin"; printf '#!/bin/sh\necho "$*" >> "%s/genargs"\nexit "${FAKEPY_RC:-0}"\n' "$t" > "$t/venv/bin/python"; chmod +x "$t/venv/bin/python"
printf '#!/bin/sh\nexit 0\n' > "$t/okguard"; chmod +x "$t/okguard"
hm() { : > "$t/genargs"; out=$(env ZB_GATE_MODE= "$@" GOLDENS_PYTHON="$t/venv/bin/python" GOLDENS_KEY_DIR="$t/keys" POPGUARD="$t/okguard" GOLDENS_ROOT="$root" RUNLOG_PLAIN=1 RUNLOG_SELFTEST=1 "$here/goldens_regen.sh" --slices-only $HMARGS 2>&1); grc=$?; }
ran() { [ -s "$t/genargs" ]; }
fx; mkdir -p "$root/scripts"; printf 'v1\n' > "$root/scripts/fake.py"; printf 'scripts/fake.py\n' > "$d/GENERATORS"; printf 'a\n' > "$d/a.txt"; sum a.txt > "$d/SHA256SUMS"
HMARGS=; hm
ck; [ "$grc" = 0 ] && ran || fail "first run (no key stamp) did not run python (rc $grc): $out"
ck; grep -q '^goldens-regen: mode=regen-gate (key [0-9a-f]\{16\} changed: no passing run recorded); heavy rows by HEAVY.SHA256 pin only$' <<< "$out" || fail "first-run mode/reason not logged: $out"
ck; grep -qx -- "scripts/fake.py --check .* --skip-heavy" "$t/genargs" || fail "key-changed run did not pass --skip-heavy: $(cat "$t/genargs")"
hm
ck; [ "$grc" = 0 ] || fail "unchanged-key run failed (rc $grc): $out"
ck; ran && fail "unchanged key still ran python: $(cat "$t/genargs")"
ck; grep -q '^goldens-regen: mode=cached (key [0-9a-f]\{16\} unchanged since the last passing run); python skipped, pinned sums verified only$' <<< "$out" || fail "cached mode not logged: $out"
printf 'b\n' > "$d/a.txt"; hm
ck; [ "$grc" != 0 ] || fail "cached mode passed a changed golden: $out"
sum a.txt > "$d/SHA256SUMS"; hm
ck; ran && grep -q 'mode=regen-gate (key [0-9a-f]\{16\} changed: inputs differ' <<< "$out" || fail "changed golden+sums did not rerun python: $out"
hm; ck; ran && fail "second run after a golden change ran python again"
printf 'v2\n' > "$root/scripts/fake.py"; hm
ck; ran || fail "a changed generator script did not rerun python: $out"
mkdir -p "$t/venv/lib/python3.12/site-packages/sympy-9.9.dist-info"; hm
ck; ran || fail "a changed oracle version did not rerun python: $out"
printf '%064d  la_x_heavy.txt\n' 1 > "$d/HEAVY.SHA256"; hm
ck; ran || fail "a changed HEAVY.SHA256 did not rerun python: $out"
printf 'zz  la_x_heavy.txt\n' > "$d/HEAVY.SHA256"; FAKEPY_RC=0 hm; hm
ck; [ "$grc" != 0 ] || fail "cached mode passed a malformed HEAVY.SHA256: $out"
printf '%064d  la_x_heavy.txt\n' 1 > "$d/HEAVY.SHA256"; hm; hm
printf 'v3\n' > "$root/scripts/fake.py"; hm FAKEPY_RC=1
ck; [ "$grc" != 0 ] || fail "a failing generator passed"
hm; ck; ran || fail "a failed run was recorded as passing (key stamped): $out"
HMARGS=--regen; hm
ck; [ "$grc" = 0 ] && ran || fail "--regen did not run python (rc $grc): $out"
ck; grep -q '^goldens-regen: mode=regen-full (--regen); heavy rows recomputed$' <<< "$out" || fail "--regen mode not logged: $out"
ck; grep -q -- '--skip-heavy' "$t/genargs" && fail "--regen passed --skip-heavy: $(cat "$t/genargs")"
HMARGS=; hm ZB_GATE_MODE=merge
ck; [ "$grc" = 2 ] || fail "merge-gate mode without --regen exited $grc, want 2: $out"
ck; grep -q 'goldens-regen: REFUSING: ZB_GATE_MODE=merge needs --regen' <<< "$out" || fail "merge refusal unclear: $out"
ck; ran && fail "merge refusal still ran python"
HMARGS=--regen; hm ZB_GATE_MODE=merge
ck; [ "$grc" = 0 ] && ran && grep -q 'mode=regen-full (--regen' <<< "$out" || fail "merge-gate --regen failed or not logged (rc $grc): $out"
HMARGS=; hm ZB_GATE_MODE=bogus
ck; [ "$grc" = 2 ] || fail "an unknown ZB_GATE_MODE exited $grc, want 2: $out"
echo "goldens_regen selftest: ok"
st_done goldens_regen 48   # WANT: cases this script runs (tools/selftest_cases.sh)
