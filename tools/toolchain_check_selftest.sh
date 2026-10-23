#!/usr/bin/env bash
# Self-test of tools/toolchain_check.sh with fake rustc/cargo on PATH and fixture rust-toolchain.toml files: the pinned channel passes,
# any other version (older, a longer prefix such as 1.990), a missing or unpinned toolchain file, or a missing rustc fails.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap toolchain_check_selftest "$@"   # run log (tools/runlog.sh)
runlog_detach_children
set -u
here=$(cd "$(dirname "$0")" && pwd)
t=$(mktemp -d); runlog_on_exit "rm -rf \"$t\""
fail() { echo "toolchain_check selftest FAIL: $*"; exit 1; }
mkfake() { # mkfake <dir> <rustc version line> <cargo version line>
  mkdir -p "$1"
  printf '#!/bin/sh\necho "%s"\n' "$2" > "$1/rustc"; printf '#!/bin/sh\necho "%s"\n' "$3" > "$1/cargo"; chmod +x "$1/rustc" "$1/cargo"
}
printf '[toolchain]\nchannel = "1.99"\ncomponents = ["clippy"]\n' > "$t/pinned.toml"
printf '[toolchain]\nchannel = "stable"\n' > "$t/stable.toml"
mkfake "$t/ok" "rustc 1.99.1 (58cf8f9c3 2026-10-13)" "cargo 1.99.1 (5f94df478 2026-08-27)"
mkfake "$t/old" "rustc 1.95.0 (59807616e 2026-04-14)" "cargo 1.99.1 (5f94df478 2026-08-27)"
mkfake "$t/oldcargo" "rustc 1.99.1 (58cf8f9c3 2026-10-13)" "cargo 1.95.0 (aaaaaaaaa 2026-04-14)"
mkfake "$t/long" "rustc 1.990.0 (0 2027-01-01)" "cargo 1.990.0 (0 2027-01-01)"
sys="$t/sys"; mkdir -p "$sys"; for c in bash sed; do ln -s "$(command -v "$c")" "$sys/$c"; done
# Fixture runs of the real toolchain_check.sh log through poplog into a throwaway POPLOG_ROOT, never the live ~/pop-logs
# (RUNNING.tsv rows, per-run logs). Only the fixture runs get it; this selftest's own run log stays live.
plr=$(mktemp -d); runlog_on_exit "rm -rf \"$plr\""
chk() { POPLOG_ROOT="$plr" PATH="$1:$sys" "$here/toolchain_check.sh" "$2" > "$t/out" 2>&1; }
chk "$t/ok" "$t/pinned.toml" || fail "pinned 1.99.1 rejected: $(cat "$t/out")"
grep -q 'rustc 1.99.1' "$t/out" || fail "version not printed"
chk "$t/old" "$t/pinned.toml" && fail "rustc 1.95.0 accepted for channel 1.99"
grep -q 'TOOLCHAIN MISMATCH' "$t/out" || fail "mismatch not named"
chk "$t/oldcargo" "$t/pinned.toml" && fail "cargo 1.95.0 accepted for channel 1.99"
chk "$t/long" "$t/pinned.toml" && fail "1.990.0 accepted for channel 1.99"
chk "$t/ok" "$t/missing.toml" && fail "missing toolchain file accepted"
chk "$t/ok" "$t/stable.toml" && fail "unpinned channel accepted"
mkdir -p "$t/none"; chk "$t/none" "$t/pinned.toml" && fail "missing rustc accepted"
# (Under RUNLOG_PLAIN=1, e.g. full_gate_selftest's fixture gate, nothing logs anywhere, so there is nothing to check.)
[ "${RUNLOG_PLAIN:-0}" = 1 ] || [ -n "$(find "$plr" -name '*-toolchain_check*.log' -print -quit)" ] || fail "fixture runs did not log under POPLOG_ROOT=$plr (isolation not exercised)"
echo "toolchain_check selftest: ok"
