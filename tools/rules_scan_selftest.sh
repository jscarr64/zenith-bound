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
copied|let y = it.copied();
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
unwrap_variant|let v = r.unwrap_or_default();
unwrap_variant|let v = r.unwrap_unchecked();
unwrap_variant|let e = r.unwrap_err();
unwrap_variant|let e = r.unwrap_err_unchecked();
unwrap_variant|let e = r.expect_err("x");
assert|assert!(a == b);
assert|assert_eq!(a, b);
assert|assert_ne!(a, b);
assert|assert_matches!(a, Some(_));
assert|debug_assert!(a);
assert|debug_assert_eq!(a, b);
f32|let a: f32 = 1.0;
f64|let a = 1.0_f64; let b: f64 = 2.0;
hwfloat|let r = a.mul_add(b, c);
hwfloat|use core::arch::x86_64::_mm_add_pd;
CASES

# the zero-unwrap rule applies to test code and test helpers too
for p in tests/props.rs tests/model/mod.rs benches/b.rs examples/e.rs; do
  mkdiff $p 'let v = r.unwrap();' > "$t/p.diff"
  out="$(scan --diff-file "$t/p.diff")" && fail "unwrap not flagged in test/bench/example path: $p"
  echo "$out" | grep -q "\[unwrap\]" || fail "wrong pattern label for unwrap in $p: $out"
done

mkdiff tests/props.rs 'let y = x.clone();' > "$t/p.diff"
out="$(scan --diff-file "$t/p.diff")" && fail "clone not flagged in test path: tests/props.rs"
echo "$out" | grep -q "\[clone\]" || fail "wrong pattern label for clone in tests/props.rs: $out"

# the assert family and the unwrap variants are banned in test code too
for p in tests/props.rs tests/model/mod.rs benches/b.rs examples/e.rs; do
  mkdiff $p 'assert!(x);' > "$t/p.diff"
  out="$(scan --diff-file "$t/p.diff")" && fail "assert not flagged in test/bench/example path: $p"
  echo "$out" | grep -q "\[assert\]" || fail "wrong pattern label for assert in $p: $out"
  mkdiff $p 'let v = r.unwrap_or_default();' > "$t/p.diff"
  out="$(scan --diff-file "$t/p.diff")" && fail "unwrap_or_default not flagged in test/bench/example path: $p"
  echo "$out" | grep -q "\[unwrap_variant\]" || fail "wrong pattern label for unwrap_or_default in $p: $out"
done

# a derive spanning several lines is flagged when a following line of the block names Clone or Copy
for names in 'Clone,' 'Copy,' 'Debug, Clone,'; do
  mkdiff tests/props.rs '#[derive(' '    Debug,' "    $names" ')]' 'struct A;' > "$t/p.diff"
  out="$(scan --diff-file "$t/p.diff")" && fail "multi-line derive with $names not flagged"
  echo "$out" | grep -q "\[derive_clone\]" || fail "wrong pattern label for multi-line derive with $names: $out"
done
mkdiff tests/props.rs '#[derive(' '    Debug,' '    PartialEq,' ')]' 'struct A;' 'let c = Clone;' > "$t/p.diff"
scan --diff-file "$t/p.diff" >/dev/null || fail "multi-line derive without Clone/Copy flagged: $(scan --diff-file "$t/p.diff")"

mkdiff tests/props.rs 'let a: f64 = 1.0;' > "$t/p.diff"
out="$(scan --diff-file "$t/p.diff")" && fail "f64 not flagged in test path: tests/props.rs"
echo "$out" | grep -q "\[f64\]" || fail "wrong pattern label for f64 in tests/props.rs: $out"

mkdiff tests/props.rs 'panic!("x");' > "$t/p.diff"
out="$(scan --diff-file "$t/p.diff")" && fail "panic not flagged in test path: tests/props.rs"
echo "$out" | grep -q "\[panic\]" || fail "wrong pattern label for panic in tests/props.rs: $out"

# not flagged: comment, string, removed line, context line, deny attr, unwrap_or, unwrap_or_else, expect_err_x, assertion helpers without the macro bang
mkdiff $LIB '// x.clone() unsafe unwrap() f64' '/// doc: panic! #[allow]' 'let s = "x.clone() unwrap() f32";' '#![forbid(unsafe_code)]' 'let v = r.unwrap_or(0);' 'let v = r.expect_err_x;' 'let v = r.unwrap_or_else(f);' 'let r = assert_eq_x(a, b);' 'let s = "assert!(x) unwrap_or_default()";' > "$t/ok.diff"
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

# hidden-copy count: fn bodies under src/ that rebuild a value from the fields of another value of its type
hc="$t/hc"; mkdir -p "$hc/src" "$hc/tools"
hidden() { RULES_SCAN_ROOT="$hc" "$here/rules_scan.sh" --hidden-copies; }
flagged_hc() { # flagged_hc <label> <rust source>: exactly one site expected
  printf '%s\n' "$2" > "$hc/src/x.rs"
  out="$(hidden)" && fail "hidden copy not flagged: $1"
  echo "$out" | grep -q "hidden_copy_count 1\$" || fail "hidden copy count is not 1 for $1: $out"
}
clean_hc() { # clean_hc <label> <rust source>
  printf '%s\n' "$2" > "$hc/src/x.rs"
  out="$(hidden)" || fail "false hidden-copy flag: $1: $out"
}
flagged_hc "replicate by name" 'impl M { fn replicate(&self) -> M { M::zero() } }'
flagged_hc "copy by name" 'fn copy_of(x: &M) -> M { M::zero() }'
flagged_hc "dup by name" 'fn dup(x: &M) -> M { M::zero() }'
flagged_hc "old replicate body, renamed (pattern-bound fields)" 'impl M { fn again(&self) -> M { match &self.inner { Inner::Infinity => M::infinity(), Inner::Finite { exp, mant } => M::from_canonical(*exp, *mant) } } }'
flagged_hc "field reads in a call" 'fn again(x: &M) -> M { M::from_canonical(x.exp, x.mant) }'
flagged_hc "accessor reads in a call" 'fn again(x: &M) -> Option<M> { M::finite(x.mant()?, x.exp()?).ok() }'
flagged_hc "struct literal from another value" 'fn again(x: &P) -> P { Self { a: x.a, b: x.b } }'
flagged_hc "struct literal from pattern-bound names" 'fn again(x: &P) -> P { let P { a, b } = x; P { a: *a, b: *b } }'
clean_hc "computed args" 'fn next(x: &M) -> M { M::from_canonical(x.exp, x.mant + 1) }'
clean_hc "one field" 'fn up(x: &M) -> M { M::from_canonical(x.exp, TOP) }'
clean_hc "plain params" 'fn successor(exp: i64, mant: u64) -> M { M::from_canonical(exp, mant) }'
clean_hc "pattern-bound but modified" 'fn f(x: &M) -> M { match &x.inner { Inner::Finite { exp, mant } => M::from_canonical(exp.max(0), *mant & 1), _ => M::zero() } }'
clean_hc "type definition and match pattern" 'struct P { a: u8, b: u8 }
fn f(x: &P) -> bool { matches!(x, P { a: 0, .. }) }
fn g(x: &Inner) -> Option<(i64, u64)> { match x { Inner::Finite { exp, mant } if *mant != 0 => Some((*exp, *mant)), _ => None } }'
clean_hc "comment and string" 'fn f() -> M { // M::from_canonical(x.exp, x.mant) and fn replicate
  let _s = "M::from_canonical(x.exp, x.mant)"; M::zero() }'
: > "$hc/src/x.rs"; hidden >/dev/null || fail "empty tree flagged"
# the same rules apply under tests/ benches/ examples/ (no test exemption); one fixture file at a time
flagged_hc_at() { # flagged_hc_at <path> <label> <rust source>: exactly one site expected
  mkdir -p "$(dirname "$hc/$1")"; printf '%s\n' "$3" > "$hc/$1"
  out="$(hidden)" && fail "hidden copy not flagged in $1: $2"
  echo "$out" | grep -q "hidden_copy_count 1\$" || fail "hidden copy count is not 1 in $1 for $2: $out"
  echo "$out" | grep -q "^$1:" || fail "hidden copy site is not reported under $1 for $2: $out"
  rm -f "$hc/$1"
}
clean_hc_at() { # clean_hc_at <path> <label> <rust source>
  mkdir -p "$(dirname "$hc/$1")"; printf '%s\n' "$3" > "$hc/$1"
  out="$(hidden)" || fail "false hidden-copy flag in $1: $2: $out"
  rm -f "$hc/$1"
}
DUP='fn dup(x: &MagV) -> MagV { match x { MagV::Finite { exp, mant } => MagV::Finite { exp: *exp, mant: *mant }, MagV::Infinity => MagV::Infinity } }'
for d in tests/model/direct.rs tests/mag/support.rs benches/b.rs examples/e.rs; do
  flagged_hc_at "$d" "dup-style helper by name" "$DUP"
  flagged_hc_at "$d" "same helper renamed (pattern-bound fields)" 'fn again(x: &MagV) -> MagV { match x { MagV::Finite { exp, mant } => MagV::Finite { exp: *exp, mant: *mant }, MagV::Infinity => MagV::Infinity } }'
done
flagged_hc_at tests/model/direct.rs "field reads in a call" 'fn again(x: &MagV) -> MagV { MagV::new(x.exp, x.mant) }'
flagged_hc_at tests/model/direct.rs "same type behind Option/Result" 'fn again(x: &MagV) -> Option<MagV> { MagV::mk(x.exp, x.mant).ok() }'
flagged_hc_at tests/model/direct.rs "copy of a model value through Result with a pattern" 'fn again(m: &MagV) -> Result<MagV, String> { match m { MagV::Finite { exp, mant } => Ok(MagV::from_parts(*exp, *mant)), MagV::Infinity => Ok(MagV::Infinity) } }'
flagged_hc_at tests/mag/support.rs "a converter still counts when it is named dup" 'fn dup(m: &MagV) -> Result<Mag, String> { Mag::zero() }'
flagged_hc_at tests/model/direct.rs "a method on the type (self receiver) is not a converter" 'impl MagV { fn again(&self) -> Mag { match self { MagV::Finite { exp, mant } => Mag::from_parts(*exp, *mant), MagV::Infinity => Mag::infinity() } } }'
clean_hc_at tests/mag/support.rs "model to library converter (other types in and out)" 'pub fn from_magv(m: &MagV) -> Result<Mag, String> { match m { MagV::Finite { exp, mant } => Mag::finite(*mant, *exp).map_err(|e| format!("{:?}", e)), MagV::Infinity => Ok(Mag::infinity()) } }'
clean_hc_at tests/model/dyadic.rs "model to dyadic converter" 'pub fn from_magv(m: &MagV) -> Option<Dy> { match m { MagV::Finite { exp, mant } => Some(Dy::from_mag(*exp, *mant)), MagV::Infinity => None } }'
clean_hc_at tests/model/direct.rs "from-parts rebuild with computed arguments" 'fn f(a: i128, m: u128) -> Option<MagV> { Some(MagV::Finite { exp: i64::try_from(a).ok()?, mant: u64::try_from(m).ok()? }) }'
clean_hc_at tests/model/direct.rs "comment and string" 'fn f() -> MagV { // MagV::new(x.exp, x.mant) and fn dup
  let _s = "fn dup(x: &MagV)"; zero() }'
hidden >/dev/null || fail "tests fixtures left a flag behind: $(hidden)"

# git modes in a throwaway repo
g="$t/repo"; mkdir -p "$g/src" "$g/tools"; cd "$g" || exit 1
git init -q . && git config user.email a@b && git config user.name t
cp "$t/tools/rules_allowlist.txt" tools/; echo 0 > tools/clone_count_baseline.txt; echo 0 > tools/ref_sum_count_baseline.txt; echo 0 > tools/hidden_copy_count_baseline.txt
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
printf 'fn a(x: &S) { let y = x.clone(); let z = it.copied(); }\n' > src/x.rs; echo 2 > tools/clone_count_baseline.txt
scan --count-only | grep -q "clone count 2 == baseline" || fail "a .copied( use must count with the clone family: $(scan --count-only)"
printf 'fn a(x: &S) { let y = x.clone(); }\n' > src/x.rs
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
echo 0 > tools/ref_sum_count_baseline.txt

# hidden-copy baseline: counts fn bodies, may only go down
printf 'fn replicate(x: &S) -> S { S::new() }\nfn again(x: &S) -> S { S::new(x.a, x.b) }\n' > src/x.rs; git add -A; git commit -qm hc
echo 2 > tools/hidden_copy_count_baseline.txt
scan --count-only | grep -q "hidden-copy count 2 == baseline" || fail "hidden-copy count should be 2: $(scan --count-only)"
echo 1 > tools/hidden_copy_count_baseline.txt; scan --count-only | grep -q "hidden-copy count 2 is ABOVE" || fail "hidden-copy count above baseline not reported"
scan --count-only >/dev/null && fail "hidden-copy count above baseline passed"
echo 9 > tools/hidden_copy_count_baseline.txt; scan --count-only | grep -qi "hidden-copy count 2 is below baseline" || fail "no hint to lower hidden-copy baseline"
scan --update-baseline >/dev/null; [ "$(tr -d '[:space:]' < tools/hidden_copy_count_baseline.txt)" = 2 ] || fail "hidden-copy baseline not lowered"
echo 0 > tools/hidden_copy_count_baseline.txt; scan --update-baseline >/dev/null && fail "raising hidden-copy baseline allowed"; [ "$(tr -d '[:space:]' < tools/hidden_copy_count_baseline.txt)" = 0 ] || fail "hidden-copy baseline raised"
# the hidden-copy count includes tests/: a dup-style helper there raises the count above a 0 baseline
printf 'fn a() {}\n' > src/x.rs; mkdir -p tests; printf 'fn dup(x: &MagV) -> MagV { match x { MagV::Finite { exp, mant } => MagV::Finite { exp: *exp, mant: *mant }, MagV::Infinity => MagV::Infinity } }\n' > tests/m.rs; git add -A; git commit -qm hct
echo 0 > tools/hidden_copy_count_baseline.txt
scan --count-only | grep -q "hidden-copy count 1 is ABOVE" || fail "hidden copy in tests/ not counted: $(scan --count-only)"
scan --count-only >/dev/null && fail "hidden copy in tests/ passed a 0 baseline"
rm -f tools/hidden_copy_count_baseline.txt; scan --count-only >/dev/null && fail "missing hidden-copy baseline passed"
echo "rules_scan selftest: ok"
