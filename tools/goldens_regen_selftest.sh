#!/usr/bin/env bash
# Self-test for the per-slice checks of tools/goldens_regen.sh (Paul, 2026-10-24). Runs `goldens_regen.sh --slices-only` (no oracle,
# no generators, nothing under the repo written) on fixtures under $(zb_scratch). Proves: a SHA256SUMS with no entries passes only when
# it carries the explicit `# EMPTY` marker; a marked file that also lists entries FAILS; a malformed line FAILS (sha256sum --strict);
# a changed, unlisted or missing golden FAILS; an untracked *_heavy.txt (hashes pinned elsewhere, .gitignored) is not demanded; a slice
# without GENERATORS FAILS.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap goldens_regen_selftest "$@"   # run log (tools/runlog.sh)
set -u
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
echo "goldens_regen selftest: ok"
st_done goldens_regen 25   # WANT: cases this script runs (tools/selftest_cases.sh)
