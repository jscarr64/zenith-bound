#!/usr/bin/env bash
# Self-test for tools/mutate.sh: a tiny fixture crate (a toy function and a toy suite) is generated under /tmp at run time as its own git repo.
# Proves the runner KILLS a real mutant, reports a surviving mutant as SURVIVED (exit nonzero), accepts it only when documented as
# equivalent, flags a non-building mutant and a non-applying patch as INVALID, counts a timeout as a kill, FAILS with 0 mutants,
# and ALWAYS removes its worktree. The repo under test is never touched.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap mutate_selftest "$@"   # run log (tools/runlog.sh)
runlog_detach_children
set -u
here="$(cd "$(dirname "$0")" && pwd)"; t="$(mktemp -d /tmp/mutate-selftest.XXXXXX)"; runlog_on_exit "rm -rf \"$t\""
fail() { echo "SELFTEST FAIL: $*"; exit 1; }
r="$t/fix"; mkdir -p "$r/src" "$r/tests" "$r/tools/mutants"; cd "$r" || exit 1
git init -q . && git config user.email a@b && git config user.name t
cat > Cargo.toml <<'TOML'
[package]
name = "toyfix"
version = "0.0.0"
edition = "2021"
[dependencies]
TOML
cat > src/lib.rs <<'RS'
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}
pub fn is_small(x: u32) -> bool {
    x < 10
}
RS
cat > tests/toy.rs <<'RS'
use toyfix::{add, is_small};
#[test]
fn add_works() {
    assert_eq!(add(2, 3), 5);
}
#[test]
fn small_works() {
    assert!(is_small(9));
    assert!(!is_small(10));
}
RS
git add -A; git commit -qm fixture
mk() { # mk <name> <sed-expr>: a patch made by editing src/lib.rs, then reverting
  sed -i "$2" src/lib.rs; git diff > "tools/mutants/$1.patch"; git checkout -q src/lib.rs
}
mk 001-add-mul 's/a + b/a * b/'                 # killed: add(2,3) becomes 6
mk 002-add-commute 's/a + b/b + a/'             # equivalent: addition commutes, the suite cannot tell
mk 003-nobuild 's/a + b/a + undefined_name/'    # does not build
mk 004-hang 's/a + b/loop {}/'                  # never returns: timeout counts as a kill
printf 'garbage that is not a patch\n' > tools/mutants/005-badpatch.patch
printf 'src/lib.rs:add\t001-add-mul\t001-add-mul.patch\ttoy\n' > m.killed
printf 'src/lib.rs:add\t002-add-commute\t002-add-commute.patch\ttoy\n' > m.surv
printf 'src/lib.rs:add\t003-nobuild\t003-nobuild.patch\ttoy\n' > m.nobuild
printf 'src/lib.rs:add\t004-hang\t004-hang.patch\ttoy\n' > m.hang
printf 'src/lib.rs:add\t005-badpatch\t005-badpatch.patch\ttoy\n' > m.bad
printf '# header only\n' > tools/mutants/EQUIVALENT.txt
git add -A; git commit -qm mutants
export MUTATE_REPO="$r" MUTATE_TMP="$t"
run() { MUTATE_MANIFEST="$r/tools/mutants/manifest.txt" "$here/mutate.sh" "$@"; }
leftovers() { git -C "$r" worktree list | grep -c 'zb-mutate\.' ; find "$t" -maxdepth 1 \( -name "zb-mutate.*" -o -name "zb-mutate-target.*" \) | wc -l; }
setm() { cp "$r/$1" "$r/tools/mutants/manifest.txt"; }

# 0 mutants: a header-only manifest FAILS (Spence review: the gate's mutants step must run at least one mutant)
printf '# empty\n' > "$r/tools/mutants/manifest.txt"
out="$(run)" && fail "empty manifest passed (0 mutants proves nothing): $out"; echo "$out" | grep -c "0 mutants ran" > /dev/null || fail "empty manifest: wrong message: $out"

# a real mutant must be KILLED
setm m.killed; out="$(run)" || fail "killable mutant not accepted: $out"
echo "$out" | grep -c "^KILLED 001-add-mul" > /dev/null || fail "killed: wrong report: $out"

# a surviving mutant makes the runner exit nonzero and is reported as SURVIVED
setm m.surv; out="$(run)" && fail "surviving mutant passed (runner exited 0)"
echo "$out" | grep -c "^SURVIVED 002-add-commute" > /dev/null || fail "survivor: wrong report: $out"
# ... unless it is documented as equivalent (reported differently, exit 0)
printf '002-add-commute\taddition commutes\n' > "$r/tools/mutants/EQUIVALENT.txt"
out="$(run)" || fail "documented-equivalent survivor failed the run: $out"
echo "$out" | grep -c "^SURVIVED-EQUIVALENT 002-add-commute" > /dev/null || fail "equivalent: wrong report: $out"
printf '# header only\n' > "$r/tools/mutants/EQUIVALENT.txt"

# a mutant that does not build and a patch that does not apply are INVALID (nonzero)
setm m.nobuild; out="$(run)" && fail "non-building mutant passed"; echo "$out" | grep -c "^INVALID 003-nobuild" > /dev/null || fail "nobuild: wrong report: $out"
setm m.bad; out="$(run)" && fail "non-applying patch passed"; echo "$out" | grep -c "^INVALID 005-badpatch" > /dev/null || fail "badpatch: wrong report: $out"

# a hang is killed by the timeout
setm m.hang; out="$(MUTATE_TIMEOUT=5s run)" || fail "hanging mutant not accepted as killed: $out"
echo "$out" | grep -c "^KILLED 004-hang.*timed out" > /dev/null || fail "hang: wrong report: $out"

# --only selects one row; an unknown id is an error
cat m.killed m.surv > "$r/tools/mutants/manifest.txt"
out="$(run --only 001-add-mul)" || fail "--only killed row failed: $out"; echo "$out" | grep -c "1 mutants" > /dev/null || fail "--only: wrong count: $out"
run --only nope >/dev/null 2>&1 && fail "unknown --only id passed"

# MUTATE_TARGET: a persistent target dir is reused and kept (same verdicts), the fixed worktree is still removed; bad values are refused
pt="$t/persist-target"; setm m.killed
out="$(MUTATE_TARGET="$pt" run)" || fail "persistent target: killable mutant not accepted: $out"
echo "$out" | grep -c "^KILLED 001-add-mul" > /dev/null || fail "persistent target: wrong report: $out"
[ -d "$pt/debug" ] || fail "persistent target: $pt/debug was not kept"
touch "$pt/marker"
setm m.surv; out="$(MUTATE_TARGET="$pt" run)" && fail "persistent target: surviving mutant passed"
echo "$out" | grep -c "^SURVIVED 002-add-commute" > /dev/null || fail "persistent target: survivor wrong report: $out"
[ -f "$pt/marker" ] || fail "persistent target: the dir was not reused (marker gone)"
[ -e "$t/zb-mutate.persist" ] && fail "persistent target: fixed worktree left behind"
MUTATE_TARGET="relative/dir" run >/dev/null 2>&1 && fail "relative MUTATE_TARGET accepted"
MUTATE_TARGET="$r/target" run >/dev/null 2>&1 && fail "MUTATE_TARGET inside the repo accepted"
rm -rf "$pt"

# every worktree and target dir was removed, in every case above
n="$(leftovers)"; [ "$(printf '%s' "$n" | tr -d '\n 0')" = "" ] || fail "leftover worktrees or dirs: $n"
echo "mutate selftest: ok"
