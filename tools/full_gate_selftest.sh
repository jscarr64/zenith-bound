#!/usr/bin/env bash
# Self-test of tools/full_gate.sh: no step may pass VACUOUSLY when the tool it calls is missing (a `! grep ...` step passes if grep is absent),
# and the scan steps must still catch a planted hit and pass a clean tree. Runs the gate on throwaway fixtures under /tmp with GATE_STEPS.
set -u
export LC_ALL=C
root=$(cd "$(dirname "$0")/.." && pwd)
fail() { echo "full_gate selftest FAIL: $*"; exit 1; }
t=$(mktemp -d /tmp/zb-gate-selftest.XXXXXX); trap 'rm -rf "$t"' EXIT
SCANS="unsafe-scan no-float-scan no-panic-scan no-clone-scan derive-scan"
rcof() { sed -n "s/^$2=//p" "$t/logs/$1.rc"; }

# 1. stripped PATH: only what the gate driver itself needs; every tool a step calls (cargo, grep, python3, ...) is missing
mkdir -p "$t/bin" "$t/logs"
for c in bash timeout mkdir date; do ln -s "$(command -v "$c")" "$t/bin/$c" || fail "cannot link $c"; done
T="$t/logs" PATH="$t/bin" "$t/bin/bash" "$root/tools/full_gate.sh" stripped > "$t/stripped.out" 2>&1
grep -q '^done$' "$t/logs/stripped.rc" || fail "stripped run did not finish: $(cat "$t/stripped.out")"
steps=$(sed -n 's/^\([a-z-]*\)=[0-9]*$/\1/p' "$t/logs/stripped.rc")
[ "$(echo "$steps" | wc -w)" -ge 17 ] || fail "stripped run reported too few steps: $steps"
for s in $steps; do
  [ "$(rcof stripped "$s")" != 0 ] || fail "step $s PASSED with its tools missing (vacuous pass)"
done
for s in $SCANS; do grep -q 'MISSING TOOL' "$t/logs/stripped-$s.log" || fail "step $s does not name the missing tool"; done

# 2. full PATH, fixture trees: a planted hit fails each scan, a clean tree passes, a missing scanned dir is not an error
mk() { mkdir -p "$t/$1/src" "$t/$1/tests"; printf 'pub fn f(x: u8) -> u8 {\n    x\n}\n' > "$t/$1/src/lib.rs"; }
mk clean
GATE_STEPS="$SCANS" T="$t/logs" REPO="$t/clean" "$root/tools/full_gate.sh" clean > /dev/null 2>&1
for s in $SCANS; do [ "$(rcof clean "$s")" = 0 ] || fail "step $s fails on a clean tree: $(cat "$t/logs/clean-$s.log")"; done
plant() { # plant <step> <line>
  mk "p-$1"; printf '%s\n' "$2" >> "$t/p-$1/src/lib.rs"
  GATE_STEPS="$1" T="$t/logs" REPO="$t/p-$1" "$root/tools/full_gate.sh" "p-$1" > /dev/null 2>&1
  [ "$(rcof "p-$1" "$1")" = 1 ] || fail "step $1 missed a planted hit ($(rcof "p-$1" "$1"))"
  grep -q 'src/lib.rs' "$t/logs/p-$1-$1.log" || fail "step $1 does not show the hit"
}
plant unsafe-scan 'pub fn g() { unsafe { } }'
plant no-float-scan 'pub fn g(x: f64) -> f64 { x }'
plant no-panic-scan 'pub fn g(x: Option<u8>) -> u8 { x.unwrap() }'
plant no-clone-scan 'pub fn g(x: &[u8]) -> Vec<u8> { x.to_vec() }'
plant derive-scan '#[derive(Clone)]'
# GATE_STEPS is honoured: only the named step ran
[ "$(grep -c '^[a-z-]*=' "$t/logs/p-unsafe-scan.rc")" = 1 ] || fail "GATE_STEPS ran other steps"
# 3. toolchain step: a rustc that is not the pinned channel (Pop's /usr/bin/rustc 1.95 ahead of ~/.cargo/bin) fails the gate
mk tc; mkdir -p "$t/tc/tools"; cp "$root/tools/toolchain_check.sh" "$root/tools/toolchain_check_selftest.sh" "$t/tc/tools/"; printf '[toolchain]\nchannel = "1.99"\n' > "$t/tc/rust-toolchain.toml"
for v in 1.95.0 1.99.1; do
  mkdir -p "$t/fake-$v"; for c in rustc cargo; do printf '#!/bin/sh\necho "%s %s (x 2026-01-01)"\n' "$c" "$v" > "$t/fake-$v/$c"; chmod +x "$t/fake-$v/$c"; done
  GATE_STEPS=toolchain T="$t/logs" REPO="$t/tc" PATH="$t/fake-$v:$PATH" "$root/tools/full_gate.sh" "tc-$v" > /dev/null 2>&1
done
[ "$(rcof tc-1.95.0 toolchain)" = 1 ] || fail "toolchain step passed with rustc 1.95.0 for channel 1.99 ($(rcof tc-1.95.0 toolchain))"
grep -q 'TOOLCHAIN MISMATCH' "$t/logs/tc-1.95.0-toolchain.log" || fail "toolchain step does not name the mismatch"
[ "$(rcof tc-1.99.1 toolchain)" = 0 ] || fail "toolchain step fails on the pinned rustc: $(cat "$t/logs/tc-1.99.1-toolchain.log")"
echo "full_gate selftest: ok"
