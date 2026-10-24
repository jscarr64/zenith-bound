#!/usr/bin/env bash
# Canonical full gate for zenith-bound. usage: tools/full_gate.sh TAG
# Every step runs under `timeout` (STEP_TIMEOUT, default 20m); the rc of each step is written to $T/TAG.rc and its
# output to $T/TAG-<step>.log (T defaults to /tmp/zb-gate-logs). REPO selects the checkout (default: this one);
# CARGO_TARGET_DIR is REQUIRED and belongs to one worktree (Paul, 2026-10-23: /tmp/target-<agent>-<worktree>, e.g.
# /tmp/target-harper-zb-harper-s7); it may be reused across runs of that worktree. target-dir (right after toolchain, before any build)
# fails if it is unset or owned by another worktree (marker $CARGO_TARGET_DIR/.worktree-owner, written by the first worktree to use it);
# build-debug/build-release repeat that check and reject test binaries from outside it.
#
# rules-scan runs the scan self-test, then the scan over SCAN_ARGS (default: RATCHET_BASE..HEAD with RATCHET_BASE defaulting to 3c9f36f,
# the commit main was at when the gate was added; set SCAN_ARGS=--working after `git add -A` to scan uncommitted work).
# build-tests (cargo build --tests; not `cargo test --no-run`, which poplog counts as a zero-test step) separates a compile error from a red test: build-debug/build-release run the tests.
# manifest-scan: no compile-time env!(CARGO_MANIFEST_DIR) (it bakes one checkout's absolute path into the test binary; a binary reused from
# another worktree then reads that worktree's files). Read the variable at run time (cargo test sets it) instead.
# build-debug/build-release run through tools/fresh_test_run.sh: zero test binaries, or a binary outside CARGO_TARGET_DIR (a foreign
# reuse), fails the step; zero tests fails it too (runlog_step --tests).
# derive-scan sees a derive that spans several lines (grep -Pz); no-panic-scan also covers unwrap_or_default/unwrap_unchecked/unwrap_err/expect_err and the assert! family.
# The rules are the zero-clone, no-unwrap/expect/panic/unreachable/todo/unimplemented (test code included), no-allow, no-unsafe,
# no-float rules.
# mutants runs the mutant-runner self-test, then the WHOLE manifest (tools/mutants/manifest.txt) with a persistent MUTATE_TARGET (default
# "$CARGO_TARGET_DIR/mutate", inside the one target dir, outside the repo; default or caller-set, it gets the same ownership claim as
# CARGO_TARGET_DIR via tools/fresh_test_run.sh --claim); any SURVIVED (not in EQUIVALENT.txt) or INVALID mutant fails the step.
# toolchain (first step) prints which rustc/cargo run and fails unless both match rust-toolchain.toml's channel (Pop has rustc 1.95 in
# /usr/bin; a PATH without ~/.cargo/bin first must not build silently with it). Callers: export PATH="$HOME/.cargo/bin:$PATH".
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap full_gate "$@"   # run log (tools/runlog.sh)
set -u
# PYTHONNOUSERSITE=1: nothing from ~/.local outside a venv (the golden generators run in ~/.local/oracle-venv; see tools/goldens_regen.sh).
export PYTHONNOUSERSITE=1
T=${T:-/tmp/zb-gate-logs}; mkdir -p "$T"
cd "${REPO:-$(cd "$(dirname "$0")/.." && pwd)}" || exit 2
tag=${1:?usage: tools/full_gate.sh TAG}; : > "$T/$tag.rc"
# NO VACUOUS PASS: every step starts with `require <tools>` (exit 127 naming the missing tool), and the scans go through no_hits, which
# fails on a hit (rc 1) AND on a grep error or a missing grep (rc 2/127) instead of the old `! grep ... 2>/dev/null` (passed if grep was absent).
# test-floor (tools/test_floor.sh, floors in tools/test_floor.txt) fails when build-debug/build-release logged fewer tests than the floor or a
# binary without its `test result:` summary: a truncated step log cannot pass (Paul, 2026-10-23).
# GATE_STEPS="a b" runs only the named steps (used by tools/full_gate_selftest.sh); unset runs all.
require() { local c; for c in "$@"; do command -v "$c" > /dev/null 2>&1 || { echo "MISSING TOOL: $c (step cannot run)"; return 127; }; done; }
no_hits() { # no_hits <exclude-ERE or ''> <grep args...>  (grep runs over the existing ones of src tests benches examples build.rs)
  require grep || return 127
  local ex=$1 p paths=() out rc hits; shift
  for p in src tests benches examples build.rs; do [ -e "$p" ] && paths+=("$p"); done
  [ "${#paths[@]}" -gt 0 ] || { echo "no_hits: nothing to scan"; return 2; }
  out=$(grep "$@" "${paths[@]}"); rc=$?
  [ "$rc" -le 1 ] || { echo "no_hits: grep failed (rc $rc)"; return 2; }
  [ "$rc" = 1 ] && return 0
  if [ -n "$ex" ]; then hits=$(printf '%s\n' "$out" | grep -vE -- "$ex"); else hits=$out; fi
  [ -z "$hits" ] && return 0
  printf '%s\n' "$hits"; return 1
}
export -f require no_hits
# gate_rc: 1 once any step failed. The gate exits $gate_rc and writes 'done' only when every step passed ('failed' otherwise): no silent pass.
gate_rc=0
run() { name=$1; shift
  case " ${GATE_STEPS:-$name} " in *" $name "*) ;; *) return 0 ;; esac
  local tf=() rc; case $name in build-debug|build-release) tf=(--tests) ;; esac   # a test step that runs zero tests fails (runlog_step)
  echo "start $name $(date +%T)" >> "$T/$tag.rc"; runlog_step "${tf[@]}" "$name" -- timeout "${STEP_TIMEOUT:-20m}" bash -c "$*" > "$T/$tag-$name.log" 2>&1; rc=$?
  echo "$name=$rc" >> "$T/$tag.rc"; [ "$rc" = 0 ] || gate_rc=1; }
run toolchain "require rustc cargo sed && tools/toolchain_check_selftest.sh && tools/toolchain_check.sh"
run target-dir "require git && tools/fresh_test_run.sh --claim"
run build-tests "require cargo && cargo build --tests"
run build-debug "require cargo && tools/fresh_test_run.sh cargo test -- --test-threads=\${RUST_TEST_THREADS:-8}"
run build-release "require cargo && tools/fresh_test_run.sh cargo test --release -- --test-threads=\${RUST_TEST_THREADS:-8}"
run test-floor "require awk grep && tools/test_floor_selftest.sh && tools/test_floor.sh build-debug \"$T/$tag-build-debug.log\" && tools/test_floor.sh build-release \"$T/$tag-build-release.log\""
run rules-scan "require python3 git grep && tools/rules_scan_selftest.sh && tools/rules_scan.sh \${SCAN_ARGS:-\${RATCHET_BASE:-3c9f36f}..HEAD}"
run fmt "require cargo rustfmt && cargo fmt --all -- --check"
run clippy "require cargo cargo-clippy && cargo clippy --all-targets -- -D warnings"
run unsafe-scan "no_hits ':[0-9]+:\s*(//|\*)' -rn --include='*.rs' -E '\bunsafe\b'"
run no-float-scan "no_hits ':[0-9]+:\s*(//|\*)' -rn --include='*.rs' -E '\bf32\b|\bf64\b|\bmul_add\b|core::arch|std::arch|target_feature|is_x86_feature_detected'"
run no-panic-scan "no_hits ':[0-9]+:\s*(//|\*)' -rn --include='*.rs' -E '\bunwrap\(|\bexpect\(|\bpanic!|\bunreachable!|\btodo!|\bunimplemented!|\bunwrap_or_default\(|\bunwrap_unchecked\(|\bunwrap_err\(|\bunwrap_err_unchecked\(|\bexpect_err\(|\b(debug_)?assert(_eq|_ne)?!|\bassert_matches!'"
run no-clone-scan "no_hits ':[0-9]+:\s*(//|\*)' -rn --include='*.rs' -E '\.clone\(|\.clone_from\(|\.to_vec\(|\.cloned\(|\.copied\(|\.to_owned\(|Clone::clone|derive\([^)]*(Clone|Copy)|impl(<[^>]*>)?\s+(Clone|Copy)\s+for'"
run derive-scan "no_hits '' -rPzl --include='*.rs' '#\\[derive\\([^)]*\\b(Clone|Copy)\\b'"
run manifest-scan "no_hits ':[0-9]+:\s*(//|\*|//!)' -rn --include='*.rs' -F 'env!(\"CARGO_MANIFEST_DIR\")'"
run gate-selftest "require mktemp ln sed grep wc stat tee touch && tools/full_gate_selftest.sh && tools/fresh_test_run_selftest.sh"
run pubfn-coverage "require awk grep git && tools/pubfn_coverage_selftest.sh && tools/pubfn_coverage.sh"
run goldens-regen "require sha256sum grep cmp && tools/goldens_regen.sh"
run gen-la-guard "require python3 env && tools/gen_la_guard_selftest.sh"
run wildcard-enum "require cargo awk grep && tools/wildcard_enum_selftest.sh"
run tightness "require /usr/bin/python3 && tools/tightness_selftest.sh && tools/tightness_check.sh"
run digests "require cargo grep && tools/digests_selftest.sh && tools/digests.sh all"
run mutants "require cargo git timeout grep && mt=\${MUTATE_TARGET:-\${CARGO_TARGET_DIR:?}/mutate} && CARGO_TARGET_DIR=\$mt tools/fresh_test_run.sh --claim && tools/mutate_selftest.sh && MUTATE_TARGET=\$mt tools/mutate.sh"
if [ "$gate_rc" = 0 ]; then echo "done" >> "$T/$tag.rc"; else echo "failed" >> "$T/$tag.rc"; fi
exit "$gate_rc"
