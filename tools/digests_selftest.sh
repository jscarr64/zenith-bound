#!/usr/bin/env bash
# Self-test for tools/digests.sh (S13): fixtures under /tmp prove `compare` PASSES on identical digests (ignoring the '#' header lines, which
# name the arch) and FAILS, naming the first differing line, on each failure mode (a changed digest, a changed word count, a missing suite row,
# an extra row, runs that agree with each other but not with the expected file, no runs at all, a missing expected file); and a fake cargo
# proves `run` builds the right command lines (profile, --target, test threads) and fails when a run fails or writes no file.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap digests_selftest "$@"   # run log (tools/runlog.sh)
runlog_detach_children
set -u
export LC_ALL=C
here="$(cd "$(dirname "$0")" && pwd)"; t="$(mktemp -d /tmp/digests-selftest.XXXXXX)"; runlog_on_exit "rm -rf \"$t\""
fail() { echo "digests SELFTEST FAIL: $*"; exit 1; }
d="$here/digests.sh"
ROWS='splitmix 65536 824ce7a05302bb45
generators 160000 625983fa73534e58
mag_grid 5288814 923017e7fb7370da
ball_rows 3824433 0cd9e03b19629d8a'
mkrun() { mkdir -p "$1"; printf '# header arch=%s\n# columns\n%s\n' "${3:-x86_64}" "$2" > "$1/$4"; }
printf '# expected\n%s\n' "$ROWS" > "$t/expected.txt"
# 1. identical runs (different arch comments): pass
mkdir -p "$t/ok"; printf '# header arch=x86_64\n%s\n' "$ROWS" > "$t/ok/host-debug-t1.txt"; printf '# header arch=x86\n# more\n%s\n' "$ROWS" > "$t/ok/i686-release-t7.txt"
"$d" compare "$t/ok" "$t/expected.txt" >/dev/null 2>&1 || fail "identical runs were rejected"
# 2. a changed digest: fail, naming the file and the line
mkdir -p "$t/bad1"; cp "$t/ok/host-debug-t1.txt" "$t/bad1/"; sed 's/^mag_grid 5288814 923017e7fb7370da$/mag_grid 5288814 923017e7fb7370db/' "$t/ok/i686-release-t7.txt" > "$t/bad1/i686-release-t7.txt"
out=$("$d" compare "$t/bad1" "$t/expected.txt" 2>&1); [ $? -ne 0 ] || fail "a changed digest was accepted"
echo "$out" | grep -q 'i686-release-t7.txt' || fail "the differing run is not named: $out"
echo "$out" | grep -q 'mag_grid 5288814 923017e7fb7370db' || fail "the first differing line is not shown: $out"
# 3. a changed word count
mkdir -p "$t/bad2"; sed 's/^splitmix 65536 /splitmix 65535 /' "$t/ok/host-debug-t1.txt" > "$t/bad2/host-debug-t1.txt"
"$d" compare "$t/bad2" "$t/expected.txt" >/dev/null 2>&1 && fail "a changed word count was accepted"
# 4. a missing row, and an extra row
mkdir -p "$t/bad3"; grep -v '^ball_rows' "$t/ok/host-debug-t1.txt" > "$t/bad3/host-debug-t1.txt"
"$d" compare "$t/bad3" "$t/expected.txt" >/dev/null 2>&1 && fail "a missing suite row was accepted"
mkdir -p "$t/bad4"; { cat "$t/ok/host-debug-t1.txt"; echo 'extra 1 0000000000000001'; } > "$t/bad4/host-debug-t1.txt"
"$d" compare "$t/bad4" "$t/expected.txt" >/dev/null 2>&1 && fail "an extra suite row was accepted"
# 5. runs agree with each other but not with the expected file
mkdir -p "$t/bad5"; for n in a b; do sed 's/^splitmix 65536 824ce7a05302bb45$/splitmix 65536 0000000000000000/' "$t/ok/host-debug-t1.txt" > "$t/bad5/$n.txt"; done
"$d" compare "$t/bad5" "$t/expected.txt" >/dev/null 2>&1 && fail "runs that match each other but not the expected digests were accepted"
# 6. no runs, empty file, missing expected file
mkdir -p "$t/none"; "$d" compare "$t/none" "$t/expected.txt" >/dev/null 2>&1 && fail "an empty run directory was accepted"
mkdir -p "$t/empty"; printf '# only comments\n' > "$t/empty/x.txt"; "$d" compare "$t/empty" "$t/expected.txt" >/dev/null 2>&1 && fail "a run file without rows was accepted"
"$d" compare "$t/ok" "$t/no-such-expected.txt" >/dev/null 2>&1 && fail "a missing expected file was accepted"
# 7. `run` with a fake cargo: builds the command lines and collects one file per (target, profile, threads)
cat > "$t/fakecargo" <<'FC'
#!/usr/bin/env bash
echo "$*" >> "$FAKE_LOG"
[ -n "${FAKE_FAIL:-}" ] && exit 1
[ -n "${FAKE_NOFILE:-}" ] && exit 0
mkdir -p "$DIGESTS_TMP"; printf '# fake\n%s\n' "$FAKE_ROWS" > "$DIGESTS_TMP/grid_digests.txt"
[ -n "${FAKE_FAIL_AFTER_WRITE:-}" ] && exit 1
exit 0
FC
chmod +x "$t/fakecargo"
printf '# manifest\nhost\ni686-unknown-linux-musl\n' > "$t/manifest.txt"
export DIGESTS_CARGO="$t/fakecargo" DIGESTS_TMP="$t/tmp" FAKE_LOG="$t/log" FAKE_ROWS="$ROWS"
"$d" run "$t/manifest.txt" "$t/runs" >/dev/null 2>&1 || fail "run failed with a working fake cargo"
[ "$(ls "$t/runs"/*.txt | wc -l)" = 8 ] || fail "run did not collect 2 targets x 2 profiles x 2 thread counts: $(ls "$t/runs")"
[ "$(wc -l < "$t/log")" = 8 ] || fail "run made $(wc -l < "$t/log") cargo calls, want 8"
grep -q -- '--release .*--target i686-unknown-linux-musl' "$t/log" || grep -q -- '--target i686-unknown-linux-musl.*--release' "$t/log" || fail "no release run for the cross target: $(cat "$t/log")"
[ "$(grep -c -- '--target' "$t/log")" = 4 ] || fail "the host runs must not pass --target"
[ "$(grep -c -- '--test-threads=1' "$t/log")" = 4 ] && [ "$(grep -c -- '--test-threads=7' "$t/log")" = 4 ] || fail "thread counts 1 and 7 are not both run for every cell"
"$d" compare "$t/runs" "$t/expected.txt" >/dev/null 2>&1 || fail "the collected fake runs do not compare equal"
FAKE_FAIL=1 "$d" run "$t/manifest.txt" "$t/runs2" >/dev/null 2>&1 && fail "a failing cargo run was accepted"
FAKE_FAIL_AFTER_WRITE=1 "$d" run "$t/manifest.txt" "$t/runs5" >/dev/null 2>&1 && fail "a cargo run that failed after writing the file was accepted"
FAKE_NOFILE=1 rm -f "$t/tmp/grid_digests.txt"; FAKE_NOFILE=1 "$d" run "$t/manifest.txt" "$t/runs3" >/dev/null 2>&1 && fail "a run that wrote no digest file was accepted"
printf '# manifest only comments\n' > "$t/m0.txt"; "$d" run "$t/m0.txt" "$t/runs4" >/dev/null 2>&1 && fail "an empty manifest was accepted"
echo "digests selftest: ok"
