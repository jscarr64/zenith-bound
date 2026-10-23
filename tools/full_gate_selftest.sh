#!/usr/bin/env bash
# Self-test of tools/full_gate.sh: no step may pass VACUOUSLY when the tool it calls is missing (a `! grep ...` step passes if grep is absent),
# and the scan steps must still catch a planted hit and pass a clean tree. Runs the gate on throwaway fixtures under /tmp with GATE_STEPS.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap full_gate_selftest "$@"   # run log (tools/runlog.sh)
runlog_detach_children
set -u
export LC_ALL=C
root=$(cd "$(dirname "$0")/.." && pwd)
fail() { echo "full_gate selftest FAIL: $*"; exit 1; }
t=$(mktemp -d /tmp/zb-gate-selftest.XXXXXX); runlog_on_exit "rm -rf \"$t\""
SCANS="unsafe-scan no-float-scan no-panic-scan no-clone-scan derive-scan manifest-scan"
# The fixture gates run outside this self-test's own run log (their planted failures must not fail it), with no poplog at all.
fe=(env -u POPLOG_FILE -u POPLOG_STATE_DIR -u POPLOG_PID -u POPLOG_PROGRESS_FILE RUNLOG_POPLOG=/nonexistent RUNLOG_PLAIN=1 RUNLOG_SELFTEST=1)
rcof() { sed -n "s/^$2=//p" "$t/logs/$1.rc"; }

# 1. stripped PATH: only what the gate driver itself needs; every tool a step calls (cargo, grep, python3, ...) is missing
mkdir -p "$t/bin" "$t/logs"
for c in bash timeout mkdir date; do ln -s "$(command -v "$c")" "$t/bin/$c" || fail "cannot link $c"; done
"${fe[@]}" T="$t/logs" PATH="$t/bin" "$t/bin/bash" "$root/tools/full_gate.sh" stripped > "$t/stripped.out" 2>&1 && fail "stripped run (every step failing) exited 0"
grep -q '^failed$' "$t/logs/stripped.rc" || fail "stripped run did not finish with 'failed': $(cat "$t/stripped.out")"
grep -q '^done$' "$t/logs/stripped.rc" && fail "stripped run wrote 'done' although steps failed"
steps=$(sed -n 's/^\([a-z-]*\)=[0-9]*$/\1/p' "$t/logs/stripped.rc")
[ "$(echo "$steps" | wc -w)" -ge 17 ] || fail "stripped run reported too few steps: $steps"
for s in $steps; do
  [ "$(rcof stripped "$s")" != 0 ] || fail "step $s PASSED with its tools missing (vacuous pass)"
done
for s in $SCANS; do grep -q 'MISSING TOOL' "$t/logs/stripped-$s.log" || fail "step $s does not name the missing tool"; done

# 2. full PATH, fixture trees: a planted hit fails each scan, a clean tree passes, a missing scanned dir is not an error
mk() { mkdir -p "$t/$1/src" "$t/$1/tests"; printf 'pub fn f(x: u8) -> u8 {\n    x\n}\n' > "$t/$1/src/lib.rs"; }
mk clean
"${fe[@]}" GATE_STEPS="$SCANS" T="$t/logs" REPO="$t/clean" "$root/tools/full_gate.sh" clean > /dev/null 2>&1 || fail "clean gate run exited non-zero"
grep -q '^done$' "$t/logs/clean.rc" || fail "clean gate run did not write 'done'"
for s in $SCANS; do [ "$(rcof clean "$s")" = 0 ] || fail "step $s fails on a clean tree: $(cat "$t/logs/clean-$s.log")"; done
plant() { # plant <step> <line>
  mk "p-$1"; printf '%s\n' "$2" >> "$t/p-$1/src/lib.rs"
  "${fe[@]}" GATE_STEPS="$1" T="$t/logs" REPO="$t/p-$1" "$root/tools/full_gate.sh" "p-$1" > /dev/null 2>&1 && fail "gate with failed step $1 exited 0 (silent pass)"
  grep -q '^done$' "$t/logs/p-$1.rc" && fail "gate with failed step $1 wrote 'done'"
  [ "$(rcof "p-$1" "$1")" = 1 ] || fail "step $1 missed a planted hit ($(rcof "p-$1" "$1"))"
  grep -q 'src/lib.rs' "$t/logs/p-$1-$1.log" || fail "step $1 does not show the hit"
}
plant unsafe-scan 'pub fn g() { unsafe { } }'
plant no-float-scan 'pub fn g(x: f64) -> f64 { x }'
plant no-panic-scan 'pub fn g(x: Option<u8>) -> u8 { x.unwrap() }'
plant no-clone-scan 'pub fn g(x: &[u8]) -> Vec<u8> { x.to_vec() }'
plant derive-scan '#[derive(Clone)]'
plant manifest-scan 'pub fn g() -> &'"'"'static str { env!("CARGO_MANIFEST_DIR") }'
# GATE_STEPS is honoured: only the named step ran
[ "$(grep -c '^[a-z-]*=' "$t/logs/p-unsafe-scan.rc")" = 1 ] || fail "GATE_STEPS ran other steps"
# missing poplog: a hard failure (exit non-zero, nothing runs), not a warning; only RUNLOG_PLAIN=1 (self-test fixtures) runs without it
env -u POPLOG_FILE -u POPLOG_STATE_DIR -u POPLOG_PID -u POPLOG_PROGRESS_FILE -u RUNLOG_PLAIN RUNLOG_POPLOG=/nonexistent GATE_STEPS=unsafe-scan T="$t/logs" REPO="$t/clean" \
  "$root/tools/full_gate.sh" nopoplog > "$t/nopoplog.out" 2>&1 && fail "gate ran with poplog.sh missing: $(cat "$t/nopoplog.out")"
grep -q 'missing' "$t/nopoplog.out" || fail "missing poplog not named: $(cat "$t/nopoplog.out")"
grep -q '^unsafe-scan=' "$t/logs/nopoplog.rc" 2>/dev/null && fail "a step ran with poplog.sh missing"
# RUNLOG_PLAIN without the selftest-only marker RUNLOG_SELFTEST=1 is refused (exit non-zero, nothing runs): plain mode is for fixtures only
env -u POPLOG_FILE -u POPLOG_STATE_DIR -u POPLOG_PID -u POPLOG_PROGRESS_FILE -u RUNLOG_SELFTEST RUNLOG_PLAIN=1 GATE_STEPS=unsafe-scan T="$t/logs" REPO="$t/clean" \
  "$root/tools/full_gate.sh" plainnomark > "$t/plainnomark.out" 2>&1 && fail "gate ran with RUNLOG_PLAIN=1 but no RUNLOG_SELFTEST=1: $(cat "$t/plainnomark.out")"
grep -q 'RUNLOG_SELFTEST' "$t/plainnomark.out" || fail "refused plain run does not name RUNLOG_SELFTEST: $(cat "$t/plainnomark.out")"
grep -q '^unsafe-scan=' "$t/logs/plainnomark.rc" 2>/dev/null && fail "a step ran with RUNLOG_PLAIN=1 but no marker"
# 3. toolchain step: a rustc that is not the pinned channel (Pop's /usr/bin/rustc 1.95 ahead of ~/.cargo/bin) fails the gate
mk tc; mkdir -p "$t/tc/tools"; cp "$root/tools/toolchain_check.sh" "$root/tools/toolchain_check_selftest.sh" "$t/tc/tools/"; printf '[toolchain]\nchannel = "1.99"\n' > "$t/tc/rust-toolchain.toml"
for v in 1.95.0 1.99.1; do
  mkdir -p "$t/fake-$v"; for c in rustc cargo; do printf '#!/bin/sh\necho "%s %s (x 2026-01-01)"\n' "$c" "$v" > "$t/fake-$v/$c"; chmod +x "$t/fake-$v/$c"; done
  "${fe[@]}" GATE_STEPS=toolchain T="$t/logs" REPO="$t/tc" PATH="$t/fake-$v:$PATH" "$root/tools/full_gate.sh" "tc-$v" > /dev/null 2>&1
done
[ "$(rcof tc-1.95.0 toolchain)" = 1 ] || fail "toolchain step passed with rustc 1.95.0 for channel 1.99 ($(rcof tc-1.95.0 toolchain))"
grep -q 'TOOLCHAIN MISMATCH' "$t/logs/tc-1.95.0-toolchain.log" || fail "toolchain step does not name the mismatch"
[ "$(rcof tc-1.99.1 toolchain)" = 0 ] || fail "toolchain step fails on the pinned rustc: $(cat "$t/logs/tc-1.99.1-toolchain.log")"
# poplog treats any step whose command says `cargo test` as a test step, so a build-only `cargo test --no-run` step reads as zero tests -> FAIL
grep -nE '^run [a-z-]+ .*cargo test[^"]*--no-run' "${BASH_SOURCE[0]%/*}/full_gate.sh" && fail "a gate step builds with cargo test --no-run; poplog counts it as zero tests (use cargo build --tests)"
# 4. mutants step: MUTATE_TARGET (caller-set or default) gets the same ownership claim as CARGO_TARGET_DIR (Spence, 2026-10-23); the fixture's
# mutate.sh/mutate_selftest.sh are stubs that pass, so only the claim can fail the step
mk mu; mkdir -p "$t/mu/tools" "$t/mu-bin"; cp "$root/tools/fresh_test_run.sh" "$root/tools/runlog.sh" "$t/mu/tools/"
for f in mutate.sh mutate_selftest.sh; do printf '#!/bin/sh\necho stub %s\n' "$f" > "$t/mu/tools/$f"; chmod +x "$t/mu/tools/$f"; done
printf '#!/bin/sh\nexit 0\n' > "$t/mu-bin/cargo"; chmod +x "$t/mu-bin/cargo"; git init -q "$t/mu" || fail "git init mu failed"
mu() { "${fe[@]}" GATE_STEPS=mutants T="$t/logs" REPO="$t/mu" PATH="$t/mu-bin:$PATH" CARGO_TARGET_DIR="$t/mu-target" "$@" "$root/tools/full_gate.sh" "$tag" > /dev/null 2>&1; }
mkdir -p "$t/mu-unmarked/debug/deps"; : > "$t/mu-unmarked/debug/deps/harness-deadbeef"
tag=mu-unmarked; mu env MUTATE_TARGET="$t/mu-unmarked" && fail "mutants ran with a non-empty unmarked MUTATE_TARGET override"
[ "$(rcof mu-unmarked mutants)" != 0 ] || fail "mutants step passed with a non-empty unmarked MUTATE_TARGET override"
grep -q UNMARKED "$t/logs/mu-unmarked-mutants.log" || fail "mutants step does not name the UNMARKED MUTATE_TARGET: $(cat "$t/logs/mu-unmarked-mutants.log")"
mkdir -p "$t/mu-foreign"; printf '/some/other/worktree\n' > "$t/mu-foreign/.worktree-owner"
tag=mu-foreign; mu env MUTATE_TARGET="$t/mu-foreign" && fail "mutants ran with a MUTATE_TARGET owned by another worktree"
grep -q SHARED "$t/logs/mu-foreign-mutants.log" || fail "mutants step does not name the SHARED MUTATE_TARGET: $(cat "$t/logs/mu-foreign-mutants.log")"
tag=mu-default; mu env || fail "mutants step failed on a fresh default MUTATE_TARGET: $(cat "$t/logs/mu-default-mutants.log")"
[ "$(cat "$t/mu-target/mutate/.worktree-owner" 2> /dev/null)" = "$(cd "$t/mu" && pwd -P)" ] || fail "default MUTATE_TARGET got no owner marker"
echo "full_gate selftest: ok"
