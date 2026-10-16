#!/usr/bin/env bash
# Canonical full gate for zenith-bound. usage: tools/full_gate.sh TAG
# Every step runs under `timeout` (STEP_TIMEOUT, default 20m); the rc of each step is written to $T/TAG.rc and its
# output to $T/TAG-<step>.log (T defaults to /tmp/zb-gate-logs). REPO selects the checkout (default: this one);
# CARGO_TARGET_DIR is honoured.
#
# rules-scan runs the scan self-test, then the scan over SCAN_ARGS (default: RATCHET_BASE..HEAD with RATCHET_BASE defaulting to 3c9f36f,
# the commit main was at when the gate was added; set SCAN_ARGS=--working after `git add -A` to scan uncommitted work).
# build-tests (cargo test --no-run) separates a compile error from a red test: build-debug/build-release run the tests.
# derive-scan sees a derive that spans several lines (grep -Pz); no-panic-scan also covers unwrap_or_default/unwrap_unchecked/unwrap_err/expect_err and the assert! family.
# The rules are the zero-clone, no-unwrap/expect/panic/unreachable/todo/unimplemented (test code included), no-allow, no-unsafe,
# no-float rules.
set -u
T=${T:-/tmp/zb-gate-logs}; mkdir -p "$T"
cd "${REPO:-$(cd "$(dirname "$0")/.." && pwd)}" || exit 2
tag=${1:?usage: tools/full_gate.sh TAG}; : > "$T/$tag.rc"
run() { name=$1; shift; echo "start $name $(date +%T)" >> "$T/$tag.rc"; timeout "${STEP_TIMEOUT:-20m}" bash -c "$*" > "$T/$tag-$name.log" 2>&1; echo "$name=$?" >> "$T/$tag.rc"; }
run build-tests "cargo test --no-run"
run build-debug "cargo test"
run build-release "cargo test --release"
run rules-scan "tools/rules_scan_selftest.sh && tools/rules_scan.sh \${SCAN_ARGS:-\${RATCHET_BASE:-3c9f36f}..HEAD}"
run fmt "cargo fmt --all -- --check"
run clippy "cargo clippy --all-targets -- -D warnings"
run unsafe-scan "! grep -rn --include='*.rs' -E '\bunsafe\b' src tests benches examples build.rs 2>/dev/null | grep -vE ':[0-9]+:\s*(//|\*)'"
run no-float-scan "! grep -rn --include='*.rs' -E '\bf32\b|\bf64\b|\bmul_add\b|core::arch|std::arch|target_feature|is_x86_feature_detected' src tests benches examples build.rs 2>/dev/null | grep -vE ':[0-9]+:\s*(//|\*)'"
run no-panic-scan "! grep -rn --include='*.rs' -E '\bunwrap\(|\bexpect\(|\bpanic!|\bunreachable!|\btodo!|\bunimplemented!|\bunwrap_or_default\(|\bunwrap_unchecked\(|\bunwrap_err\(|\bunwrap_err_unchecked\(|\bexpect_err\(|\b(debug_)?assert(_eq|_ne)?!|\bassert_matches!' src tests benches examples build.rs 2>/dev/null | grep -vE ':[0-9]+:\s*(//|\*)'"
run no-clone-scan "! grep -rn --include='*.rs' -E '\.clone\(|\.clone_from\(|\.to_vec\(|\.cloned\(|\.to_owned\(|Clone::clone|derive\([^)]*(Clone|Copy)|impl(<[^>]*>)?\s+(Clone|Copy)\s+for' src tests benches examples build.rs 2>/dev/null | grep -vE ':[0-9]+:\s*(//|\*)'"
run derive-scan "! grep -rPzl --include='*.rs' '#\\[derive\\([^)]*\\b(Clone|Copy)\\b' src tests benches examples build.rs 2>/dev/null"
echo done >> "$T/$tag.rc"
