#!/usr/bin/env bash
# S4 deep fuzz run (TEST_PLAN: the gate runs 1e6 seeded cases; this runs many more, OUTSIDE the gate).
# Runs tests/harness/fuzz_mag.rs `fuzz_mag_gate` in release, split into independent shards (one process each, one distinct base seed each),
# with ZB_FUZZ_SEED / ZB_FUZZ_CASES set per shard. Every failure prints its replay line; rerun one case with
#   tools/fuzz_deep.sh --replay <seed hex>:<case>
# The gate lock /tmp/pop-gate.lock is NOT taken (this is not a gate step); use a CARGO_TARGET_DIR of your own.
#   tools/fuzz_deep.sh [--cases N] [--shards K] [--seed HEX] [--out DIR]     defaults: 100000000 cases in total, 7 shards, seed 0x5eed
#   tools/fuzz_deep.sh --replay <seed hex>:<case>                           one case, the replay line's seed=... and case=...
#   tools/fuzz_deep.sh --selftest                                             proves argument checks, shard seeds, a small run and replay
# Deep runs are env-driven: CARGO_TARGET_DIR (default /tmp/target-fuzz-deep), CARGO_BUILD_JOBS (default 7). No repo file is written.
set -u
export LC_ALL=C
root=$(cd "$(dirname "$0")/.." && pwd)
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-/tmp/target-fuzz-deep}
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-7}
fail() { echo "fuzz_deep FAIL: $*" >&2; exit 2; }
cases=100000000; shards=7; seed=5eed; out=; replay=; selftest=0
while [ $# -gt 0 ]; do
  case "$1" in
    --cases) cases=${2:-}; shift 2 ;;
    --shards) shards=${2:-}; shift 2 ;;
    --seed) seed=${2:-}; shift 2 ;;
    --out) out=${2:-}; shift 2 ;;
    --replay) replay=${2:-}; shift 2 ;;
    --selftest) selftest=1; shift ;;
    *) fail "unknown argument $1" ;;
  esac
done
seed=${seed#0x}
case "$cases" in ''|*[!0-9]*) fail "--cases must be a decimal number" ;; esac
case "$shards" in ''|*[!0-9]*) fail "--shards must be a decimal number" ;; esac
case "$seed" in ''|*[!0-9a-fA-F]*) fail "--seed must be hex" ;; esac
[ "${#seed}" -le 16 ] || fail "--seed has more than 16 hex digits"
[ "$cases" -ge 1 ] || fail "--cases must be at least 1"
[ "$shards" -ge 1 ] && [ "$shards" -le 64 ] || fail "--shards must be 1..64"
[ "$cases" -ge "$shards" ] || fail "--cases must be at least --shards"

shard_seed() { # shard_seed <base hex> <shard index>: base + index * 0x9E3779B97F4A7C15 mod 2^64 (bash arithmetic is 64-bit modular; the Rust side only sees the hex)
  printf '%016x' $(( 0x$1 + $2 * 0x9E3779B97F4A7C15 ))
}
run_test() { (cd "$root" && cargo test --release --test harness fuzz_mag::fuzz_mag_gate -- --exact --test-threads=1 --nocapture 2>&1); }

if [ "$selftest" = 1 ]; then
  me=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
  bad() { "$me" "$@" >/dev/null 2>&1 && fail "selftest: expected failure for: $*"; return 0; }
  bad --cases 0; bad --shards 0; bad --shards 65; bad --cases 3 --shards 7; bad --seed zz; bad --cases x; bad --bogus; bad --replay nocolon
  a=$(shard_seed 5eed 0); b=$(shard_seed 5eed 1); c=$(shard_seed 5eed 2)
  [ "$a" = 0000000000005eed ] || fail "selftest: shard 0 seed is $a"
  [ "$b" = 9e3779b97f4adb02 ] || fail "selftest: shard 1 seed is $b"
  [ "$a" != "$b" ] && [ "$b" != "$c" ] || fail "selftest: shard seeds collide"
  t=$(mktemp -d /tmp/fuzz-deep-selftest.XXXXXX); trap 'rm -rf "$t"' EXIT
  "$me" --cases 4000 --shards 2 --out "$t" >"$t/run.txt" 2>&1 || { cat "$t/run.txt"; fail "selftest: small run failed"; }
  grep -q 'fuzz_deep: 2 shards ok, 4000 cases' "$t/run.txt" || { cat "$t/run.txt"; fail "selftest: no summary line"; }
  [ -s "$t/shard-0.log" ] && [ -s "$t/shard-1.log" ] || fail "selftest: shard logs missing"
  "$me" --replay 5eed:17 >"$t/replay.txt" 2>&1 || { cat "$t/replay.txt"; fail "selftest: replay failed"; }
  grep -q 'fuzz_deep: replay 5eed:17 ok' "$t/replay.txt" || fail "selftest: no replay summary"
  echo "fuzz_deep selftest ok"; exit 0
fi

if [ -n "$replay" ]; then
  case "$replay" in *:*) ;; *) fail "--replay must be <seed hex>:<case>" ;; esac
  log=$(ZB_REPLAY="$replay" run_test); rc=$?
  echo "$log" | grep -E 'replay|why:|test result' | head -20
  [ "$rc" = 0 ] || { echo "fuzz_deep: replay $replay FAILED"; exit 1; }
  echo "fuzz_deep: replay $replay ok"; exit 0
fi

[ -n "$out" ] || out=$(mktemp -d /tmp/fuzz-deep.XXXXXX)
mkdir -p "$out" || fail "cannot create $out"
(cd "$root" && cargo test --release --test harness --no-run >"$out/build.log" 2>&1) || { cat "$out/build.log"; fail "release build failed"; }
per=$(( (cases + shards - 1) / shards ))
start=$(date +%s)
pids=()
i=0
while [ "$i" -lt "$shards" ]; do
  s=$(shard_seed "$seed" "$i")
  ( ZB_FUZZ_SEED="$s" ZB_FUZZ_CASES="$per" run_test >"$out/shard-$i.log" ) &
  pids+=($!)
  i=$(( i + 1 ))
done
bad=0; i=0
for p in "${pids[@]}"; do
  wait "$p"; rc=$?
  if [ "$rc" != 0 ]; then bad=$(( bad + 1 )); echo "fuzz_deep: shard $i FAILED (seed $(shard_seed "$seed" "$i")); log $out/shard-$i.log"; grep -E 'suite=|why:' "$out/shard-$i.log" | head -6; fi
  i=$(( i + 1 ))
done
secs=$(( $(date +%s) - start ))
total=$(( per * shards ))
[ "$bad" = 0 ] || { echo "fuzz_deep: $bad of $shards shards failed after ${secs}s; replay with: tools/fuzz_deep.sh --replay <seed>:<case> (the failing line's seed= and case=)"; exit 1; }
echo "fuzz_deep: $shards shards ok, $total cases in ${secs}s (logs in $out)"
