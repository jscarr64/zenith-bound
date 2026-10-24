#!/usr/bin/env bash
# Self-test for the per-slice include files of tools/mutate.sh (S0, 2026-10-24): tools/mutants/manifest.d/<slice>.txt (rows),
# tools/mutants/equivalent.d/<slice>.txt (documented equivalents) and tools/mutants/floor.d/<slice>.txt (floor shares).
# Runs only `mutate.sh --list` and `mutate.sh --floor` (no cargo, no worktree) on a fixture under $(zb_scratch).
# Proves: include rows are listed after the central manifest; an id outside its file's `# range: LO-HI` FAILS; a file without a
# range line FAILS; two files with overlapping ranges FAIL; an id duplicated across files FAILS; floor shares add to floor.txt.
# Hardening (Paul, 2026-10-24): a central id inside a slice's range FAILS; an include row whose patch is not under <slice>/ FAILS;
# suffixed ids sharing a number (205-a, 205-b) FAIL the duplicate check; a row without 4 tab-separated columns FAILS; equivalent.d
# and floor.d entries are validated (known id in the slice's range with a reason; a matching manifest.d file; each key once).
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap mutate_include_selftest "$@"   # run log (tools/runlog.sh)
set -u
here="$(cd "$(dirname "$0")" && pwd)"; t="$(mktemp -d "$(zb_scratch)/mutate-include-selftest.XXXXXX")"; runlog_on_exit "rm -rf \"$t\""
. "${BASH_SOURCE[0]%/*}/selftest_cases.sh"; st_init "$t"
fail() { echo "SELFTEST FAIL: $*"; exit 1; }
m="$t/mutants"; mkdir -p "$m/manifest.d" "$m/floor.d" "$m/equivalent.d"
tab=$'\t'
printf '# central\nsrc/a.rs:f%s001-A%s001-A.patch%ssuite\n' "$tab" "$tab" "$tab" > "$m/manifest.txt"
printf 'killed\t1\nkilled+equivalent\t1\n' > "$m/floor.txt"
printf '# range: 200-299\nsrc/b.rs:g%s201-B%ss1/201-B.patch%ssuite\n' "$tab" "$tab" "$tab" > "$m/manifest.d/s1.txt"
printf '# range: 300-399\n' > "$m/manifest.d/s2.txt"
printf 'killed\t2\nkilled+equivalent\t3\n' > "$m/floor.d/s1.txt"
printf 'killed\t0\nkilled+equivalent\t0\n' > "$m/floor.d/s2.txt"
mkdir -p "$m/s1"; touch "$m/001-A.patch" "$m/s1/201-B.patch"
mu() { MUTATE_MANIFEST="$m/manifest.txt" RUNLOG_PLAIN=1 RUNLOG_SELFTEST=1 "$here/mutate.sh" "$@" 2>&1; }
reset() { printf '# central\nsrc/a.rs:f%s001-A%s001-A.patch%ssuite\n' "$tab" "$tab" "$tab" > "$m/manifest.txt"
  printf '# range: 200-299\nsrc/b.rs:g%s201-B%ss1/201-B.patch%ssuite\n' "$tab" "$tab" "$tab" > "$m/manifest.d/s1.txt"
  rm -f "$m"/equivalent.d/*.txt; printf 'killed\t2\nkilled+equivalent\t3\n' > "$m/floor.d/s1.txt"; rm -f "$m/floor.d/s9.txt"; }
reset; ck; out=$(mu --list) || fail "reset fixture fails: $out"
printf 'src/a.rs:f%s250-Z%s001-A.patch%ssuite\n' "$tab" "$tab" "$tab" >> "$m/manifest.txt"
ck; out=$(mu --list) && fail "central id 250 inside s1's range passed: $out"
ck; printf '%s\n' "$out" | grep -q 'owned by slice' || fail "central-in-range message missing: $out"
reset; printf 'src/b.rs:g%s202-E%s001-A.patch%ssuite\n' "$tab" "$tab" "$tab" >> "$m/manifest.d/s1.txt"
ck; out=$(mu --list) && fail "s1 row whose patch is outside s1/ passed: $out"
ck; printf '%s\n' "$out" | grep -q 'not under s1/' || fail "patch-ownership message missing: $out"
reset; printf 'src/b.rs:g%s205-a%ss1/201-B.patch%ssuite\nsrc/b.rs:g%s205-b%ss1/201-B.patch%ssuite\n' "$tab" "$tab" "$tab" "$tab" "$tab" "$tab" >> "$m/manifest.d/s1.txt"
ck; out=$(mu --list) && fail "suffixed ids 205-a and 205-b passed the duplicate check: $out"
ck; printf '%s\n' "$out" | grep -q 'duplicate manifest id number 205' || fail "suffix-dup message missing: $out"
reset; printf 'src/a.rs:f 003-N 001-A.patch suite\n' >> "$m/manifest.txt"
ck; out=$(mu --list) && fail "central row without tabs passed: $out"
ck; printf '%s\n' "$out" | grep -q '4 tab-separated columns' || fail "no-tab message missing: $out"
reset; printf 'src/b.rs:g 203-N s1/201-B.patch suite\n' >> "$m/manifest.d/s1.txt"
ck; out=$(mu --list) && fail "include row without tabs passed: $out"
reset; printf '201-B\tunobservable by construction\n' > "$m/equivalent.d/s1.txt"
ck; out=$(mu --list) || fail "a valid equivalent.d entry failed: $out"
printf '299-X\tno such mutant\n' > "$m/equivalent.d/s1.txt"
ck; out=$(mu --list) && fail "equivalent.d id not in the manifest passed: $out"
printf '201-B\n' > "$m/equivalent.d/s1.txt"
ck; out=$(mu --list) && fail "equivalent.d entry without a reason passed: $out"
reset; printf '201-B\treason\n' > "$m/equivalent.d/s2.txt"
ck; out=$(mu --list) && fail "equivalent.d/s2 claiming s1's id passed: $out"
reset; printf '201-B\treason\n' > "$m/equivalent.d/s9.txt"
ck; out=$(mu --list) && fail "equivalent.d file without a manifest.d slice passed: $out"
reset; printf 'killed\t0\nkilled+equivalent\t0\n' > "$m/floor.d/s9.txt"
ck; out=$(mu --floor) && fail "floor.d file without a manifest.d slice passed: $out"
reset; printf 'killed\t1\nkilled\t2\nkilled+equivalent\t3\n' > "$m/floor.d/s1.txt"
ck; out=$(mu --floor) && fail "floor.d with a duplicate key passed: $out"
reset; printf 'killed\t1\nkilled+equivalent\t3\nkiled\t5\n' > "$m/floor.d/s1.txt"
ck; out=$(mu --floor) && fail "floor.d with an unknown key passed: $out"
reset
ck; out=$(mu --list) || fail "--list with includes failed: $out"
ck; [ "$(printf '%s\n' "$out" | tr '\n' ' ')" = "001-A 201-B " ] || fail "--list gave '$out', want 001-A then 201-B"
ck; out=$(mu --floor) || fail "--floor failed: $out"
ck; [ "$(printf '%s\n' "$out" | tr '\n' ' ')" = "killed 3 killed+equivalent 4 " ] || fail "--floor gave '$out', want killed 3, killed+equivalent 4"
printf 'src/c.rs:h%s305-C%s201-B.patch%ssuite\n' "$tab" "$tab" "$tab" >> "$m/manifest.d/s1.txt"
ck; out=$(mu --list) && fail "id 305 outside s1's range 200-299 passed: $out"
ck; printf '%s\n' "$out" | grep -q 'outside' || fail "out-of-range message missing: $out"
printf '# range: 200-299\nsrc/b.rs:g%s201-B%ss1/201-B.patch%ssuite\n' "$tab" "$tab" "$tab" > "$m/manifest.d/s1.txt"
printf 'src/b.rs:g%s301-D%s201-B.patch%ssuite\n' "$tab" "$tab" "$tab" > "$m/manifest.d/s3.txt"
ck; out=$(mu --list) && fail "include file without a range line passed: $out"
printf '# range: 250-320\n' > "$m/manifest.d/s3.txt"
ck; out=$(mu --list) && fail "overlapping ranges passed: $out"
printf '# range: 400-499\nsrc/b.rs:g%s201-B%ss1/201-B.patch%ssuite\n' "$tab" "$tab" "$tab" > "$m/manifest.d/s3.txt"
ck; out=$(mu --list) && fail "id duplicated across include files passed: $out"
rm "$m/manifest.d/s3.txt"
printf 'killed\tx\n' > "$m/floor.d/s2.txt"
ck; out=$(mu --floor) && fail "non-integer floor share passed: $out"
echo "mutate_include selftest: ok"
st_done mutate_include 28   # WANT: cases this script runs (tools/selftest_cases.sh)
