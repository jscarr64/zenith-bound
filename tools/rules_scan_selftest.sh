#!/usr/bin/env bash
# Self-test for tools/rules_scan.sh (zenith-bound): every banned pattern is flagged on ADDED lines in src/, tests/, benches/ and examples/ alike (there is no test-code exemption), removed/context lines, comments, strings and non-Rust paths are not flagged, the allowlist works, git modes work, and both count baselines only go down.
set -u
here="$(cd "$(dirname "$0")" && pwd)"; t="$(mktemp -d)"; trap 'rm -rf "$t"' EXIT
scan() { "$here/rules_scan.sh" "$@"; }
fail() { echo "SELFTEST FAIL: $*"; exit 1; }
mkdiff() { # mkdiff <path> <added line...>  -> fixture diff on stdout
  local p="$1"; shift
  printf 'diff --git a/%s b/%s\n--- a/%s\n+++ b/%s\n@@ -1,1 +1,%d @@\n context\n' "$p" "$p" "$p" "$p" "$(( $# + 1 ))"
  for l in "$@"; do printf '+%s\n' "$l"; done
}
LIB=src/foo.rs
export RULES_SCAN_ROOT="$t"; mkdir -p "$t/tools"; : > "$t/tools/rules_allowlist.txt"; echo 0 > "$t/tools/clone_count_baseline.txt"; echo 0 > "$t/tools/ref_sum_count_baseline.txt"
export RULES_SCAN_SKIP_COUNT=1   # fixture diffs have no tree to count

mkdiff $LIB 'let a = 1 + 2;' 'let s = a.max(b);' > "$t/clean.diff"
scan --diff-file "$t/clean.diff" >/dev/null || fail "clean diff flagged"

while IFS='|' read -r name line; do
  mkdiff $LIB "$line" > "$t/d.diff"
  out="$(scan --diff-file "$t/d.diff")" && fail "not flagged: $name"
  echo "$out" | grep -q "\[$name\]" || fail "wrong pattern label for $name: $out"
done <<'CASES'
clone|let y = x.clone();
clone_from|y.clone_from(&x);
clone_path|let y = Clone::clone(&x);
to_vec|let y = s.to_vec();
cloned|let y = it.cloned();
to_owned|let y = s.to_owned();
derive_clone|#[derive(Clone)]
derive_clone|#[derive(Debug, Clone, Copy)]
impl_clone|impl Clone for Big {
impl_clone|impl Copy for Big {
unsafe|unsafe { core::hint::unreachable_unchecked() }
allow|#[allow(dead_code)]
allow|#![allow(clippy::all)]
unwrap|let v = r.unwrap();
expect|let v = r.expect("boom");
panic|panic!("no");
unreachable|unreachable!();
todo|todo!();
unimplemented|unimplemented!();
f32|let a: f32 = 1.0;
f64|let a = 1.0_f64; let b: f64 = 2.0;
hwfloat|let r = a.mul_add(b, c);
hwfloat|use core::arch::x86_64::_mm_add_pd;
neg_sub_method|let d = a.sub(&b, p, rm).neg();
neg_sub_method|let d = a.sub_ref(&b).neg();
neg_sub_call|let d = neg(a.sub(&b));
neg_sub_call|let d = Neg::neg(Sub::sub(a, b));
neg_sub_op|let d = -(b - a);
neg_sub_op|let d = -(b-a);
neg_sub_op|let d = x + -(b.exp - a.exp);
neg_sub_op|return -(b - a);
neg_sub_op|let d = -a.sub(&b);
neg_sub_op|let d = -(a.sub(&b));
CASES

# S9: the neg-of-sub forms are refused in test paths as well, and clean subtractions are not flagged
for p in tests/props.rs tests/model/mod.rs; do
  mkdiff $p 'let d = a.sub(&b).neg();' > "$t/p.diff"
  out="$(scan --diff-file "$t/p.diff")" && fail "neg_sub_method not flagged in test path: $p"
  echo "$out" | grep -q "\[neg_sub_method\]" || fail "wrong pattern label for neg_sub_method in $p: $out"
  mkdiff $p 'let d = -(b - a);' > "$t/p.diff"
  out="$(scan --diff-file "$t/p.diff")" && fail "neg_sub_op not flagged in test path: $p"
  echo "$out" | grep -q "\[neg_sub_op\]" || fail "wrong pattern label for neg_sub_op in $p: $out"
done
mkdiff $LIB 'let d = a.sub(&b, p, rm);' 'let d = x - (a - b);' 'let d = -1 - (a - b);' 'let d = a.sub_lower(&b);' 'let n = x.neg();' 'let d = a.sub(&b).abs();' 'let d = -a;' 'let d = -(a + b);' 'let d = a - -b;' 'fn f(x: u8) -> (u8, u8) { (x - 1, x) }' 'let d = (a - b).neg();' 'let d = neg(a).sub(b);' 'let s = "-(b - a) and a.sub(b).neg()";' '// -(b - a)' 'let d = x.neg().sub(y);' > "$t/negok.diff"
scan --diff-file "$t/negok.diff" >/dev/null || fail "clean subtraction forms flagged: $(scan --diff-file "$t/negok.diff")"

# the zero-unwrap rule applies to test code and test helpers too
for p in tests/props.rs tests/model/mod.rs benches/b.rs examples/e.rs; do
  mkdiff $p 'let v = r.unwrap();' > "$t/p.diff"
  out="$(scan --diff-file "$t/p.diff")" && fail "unwrap not flagged in test/bench/example path: $p"
  echo "$out" | grep -q "\[unwrap\]" || fail "wrong pattern label for unwrap in $p: $out"
done

mkdiff tests/props.rs 'let y = x.clone();' > "$t/p.diff"
out="$(scan --diff-file "$t/p.diff")" && fail "clone not flagged in test path: tests/props.rs"
echo "$out" | grep -q "\[clone\]" || fail "wrong pattern label for clone in tests/props.rs: $out"

mkdiff tests/props.rs 'let a: f64 = 1.0;' > "$t/p.diff"
out="$(scan --diff-file "$t/p.diff")" && fail "f64 not flagged in test path: tests/props.rs"
echo "$out" | grep -q "\[f64\]" || fail "wrong pattern label for f64 in tests/props.rs: $out"

mkdiff tests/props.rs 'panic!("x");' > "$t/p.diff"
out="$(scan --diff-file "$t/p.diff")" && fail "panic not flagged in test path: tests/props.rs"
echo "$out" | grep -q "\[panic\]" || fail "wrong pattern label for panic in tests/props.rs: $out"

# not flagged: comment, string, removed line, context line, deny attr, unwrap_or, expect_err_x, unwrap_or_default, assert_eq
mkdiff $LIB '// x.clone() unsafe unwrap() f64' '/// doc: panic! #[allow]' 'let s = "x.clone() unwrap() f32";' '#![forbid(unsafe_code)]' 'let v = r.unwrap_or(0);' 'let v = r.expect_err_x;' 'let v = r.unwrap_or_default();' 'let r = assert_eq!(a, b);' > "$t/ok.diff"
scan --diff-file "$t/ok.diff" >/dev/null || fail "comment/string/benign lines flagged: $(scan --diff-file "$t/ok.diff")"
for p in README.md docs/CONTRACT.md scripts/gen.py tools/t.py; do
  mkdiff $p 'let y = x.clone(); unsafe {} r.unwrap(); let a: f64 = 1.0;' > "$t/p.diff"
  scan --diff-file "$t/p.diff" >/dev/null || fail "non-lib path flagged: $p"
done
printf 'diff --git a/%s b/%s\n--- a/%s\n+++ b/%s\n@@ -1,2 +1,1 @@\n-let y = x.clone();\n context\n' $LIB $LIB $LIB $LIB > "$t/rm.diff"
scan --diff-file "$t/rm.diff" >/dev/null || fail "removed line flagged"

# allowlist: file:hash<TAB>reason
mkdiff $LIB 'let y = x.clone();' > "$t/d.diff"
key="$(scan --diff-file "$t/d.diff" | grep -o 'allow-key=[^ ]*' | head -1 | cut -d= -f2)"; [ -n "$key" ] || fail "no allow-key printed"
printf '%s\tproof + Jeff approval\n' "$key" > "$t/tools/rules_allowlist.txt"
scan --diff-file "$t/d.diff" >/dev/null || fail "allowlisted site still flagged"
: > "$t/tools/rules_allowlist.txt"

# git modes in a throwaway repo
g="$t/repo"; mkdir -p "$g/src" "$g/tools"; cd "$g" || exit 1
git init -q . && git config user.email a@b && git config user.name t
cp "$t/tools/rules_allowlist.txt" tools/; echo 0 > tools/clone_count_baseline.txt; echo 0 > tools/ref_sum_count_baseline.txt
printf 'fn a() {}\n' > src/x.rs; git add -A; git commit -qm base; base=$(git rev-parse HEAD)
export RULES_SCAN_ROOT="$g"; unset RULES_SCAN_SKIP_COUNT
scan "$base..HEAD" >/dev/null || fail "empty range flagged"
printf 'fn a() {}\nfn b(x: &S) { let y = x.clone(); }\n' > src/x.rs
scan --working >/dev/null && fail "working diff clone not flagged"
git add -A; scan --staged >/dev/null && fail "staged diff clone not flagged"
scan --working >/dev/null && fail "working mode (git diff HEAD) must still see staged additions"
git commit -qm bad; scan "$base..HEAD" >/dev/null && fail "range clone not flagged"
# a #[cfg(test)] module gets no exemption
printf 'fn a() {}\n#[cfg(test)]\nmod tests {\n    fn t(x: &S) { let y = x.clone(); let _ = r.unwrap(); }\n}\n' > src/x.rs
git add -A; git commit -qm t; scan "$base..HEAD" >/dev/null && fail "cfg(test) additions not flagged"

# clone-count baseline: may only go down
printf 'fn a(x: &S) { let y = x.clone(); }\n' > src/x.rs; git add -A; git commit -qm c
echo 1 > tools/clone_count_baseline.txt
scan --count-only >/dev/null || fail "count == baseline should pass"
echo 0 > tools/clone_count_baseline.txt; scan --count-only >/dev/null && fail "count above baseline passed"
echo 5 > tools/clone_count_baseline.txt; scan --count-only | grep -qi "lower" || fail "no hint to lower baseline"
scan --update-baseline >/dev/null || fail "lowering baseline failed"; [ "$(tr -d '[:space:]' < tools/clone_count_baseline.txt)" = 1 ] || fail "baseline not lowered"
echo 0 > tools/clone_count_baseline.txt; scan --update-baseline >/dev/null && fail "raising baseline allowed"; [ "$(tr -d '[:space:]' < tools/clone_count_baseline.txt)" = 0 ] || fail "baseline raised"

# add_ref/sub_ref/mul_ref/div_ref use-count baseline: counts every use incl. tests (not definitions, comments, strings), may only go down
cat > src/x.rs <<'EOF'
impl N { pub fn add_ref(a: &N, b: &N) -> N { a.go(b) } pub fn sub_ref(a: &N, b: &N) -> N { a.go(b) } pub fn mul_ref(a: &N, b: &N) -> N { a.go(b) } pub fn div_ref(a: &N, b: &N) -> N { a.go(b) } }
fn u(a: &N, b: &N) { let s = N::add_ref(a, b); let d = N::sub_ref(a, b); let m = N::mul_ref(a, b); let q = N::div_ref(a, b); }
// N::add_ref(a, b) in a comment
fn s() { let x = "N::sub_ref(a, b)"; }
#[cfg(test)]
mod tests { fn t(a: &N, b: &N) { let s = N::add_ref(a, b); } }
EOF
git add -A; git commit -qm refs
echo 5 > tools/ref_sum_count_baseline.txt; echo 7 > tools/clone_count_baseline.txt
scan --count-only | grep -q "use count 5 == baseline" || fail "ref count should be 5 (definitions, comments, strings excluded; the test-module use counts): $(scan --count-only)"
echo 4 > tools/ref_sum_count_baseline.txt; scan --count-only | grep -q "ABOVE" || fail "ref count above baseline not reported"
scan --count-only >/dev/null && fail "ref count above baseline passed"
echo 9 > tools/ref_sum_count_baseline.txt; scan --count-only | grep -qi "lower the baseline" || fail "no hint to lower ref baseline"
scan --update-baseline >/dev/null; [ "$(tr -d '[:space:]' < tools/ref_sum_count_baseline.txt)" = 5 ] || fail "ref baseline not lowered"
echo 0 > tools/ref_sum_count_baseline.txt; scan --update-baseline >/dev/null && fail "raising ref baseline allowed"; [ "$(tr -d '[:space:]' < tools/ref_sum_count_baseline.txt)" = 0 ] || fail "ref baseline raised"
rm -f tools/ref_sum_count_baseline.txt; scan --count-only >/dev/null && fail "missing ref baseline passed"
echo "rules_scan selftest: ok"
