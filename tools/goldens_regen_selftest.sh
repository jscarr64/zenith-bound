#!/usr/bin/env bash
# Self-test for the per-slice checks of tools/goldens_regen.sh (Paul, 2026-10-24). Runs `goldens_regen.sh --slices-only` (no oracle,
# no generators, nothing under the repo written) on fixtures under ${TMPDIR:-/tmp}. Proves: a SHA256SUMS with no entries passes only when
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
echo "goldens_regen selftest: ok"
st_done goldens_regen 12   # WANT: cases this script runs (tools/selftest_cases.sh)
