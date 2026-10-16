#!/usr/bin/env bash
# Hand-written mutant runner (TEST_PLAN section 11, default plan D3). No `cfg` switch is added to src/; a mutant is a patch.
#
# usage: tools/mutate.sh [--only MUTANT_ID] [--list]
#   env: MUTATE_REPO=DIR            repo to mutate (default: the parent of tools/). Mutants apply to its COMMITTED HEAD.
#        MUTATE_MANIFEST=FILE       default $MUTATE_REPO/tools/mutants/manifest.txt (patches are read from that file's directory)
#        MUTATE_EQUIVALENT=FILE     default: EQUIVALENT.txt next to the manifest
#        MUTATE_TIMEOUT=DURATION    per `cargo test` run of a suite (default 20m); a timeout counts as a kill
#        MUTATE_BUILD_TIMEOUT=DUR   per build (default 20m)
#        MUTATE_TMP=DIR             parent of the throwaway worktree and target dir (default /tmp)
#
# For each manifest row it creates a throwaway `git worktree` under /tmp (never inside the repo), applies ONE patch, builds the suite,
# runs `cargo test --test <suite>` under `timeout`, and REQUIRES the suite to fail. The worktree is always removed (EXIT/INT/TERM).
#   KILLED               the suite failed (a test failed, or the run timed out)
#   SURVIVED             the suite passed on the mutant: exit 1 (unless the id is in EQUIVALENT.txt: SURVIVED-EQUIVALENT, not a failure)
#   INVALID              the patch does not apply or the mutant does not build (proves nothing): exit 1
# Exit 0 only if every mutant was KILLED or SURVIVED-EQUIVALENT (0 mutants is exit 0).
set -u
export LC_ALL=C
repo=${MUTATE_REPO:-$(cd "$(dirname "$0")/.." && pwd)}
manifest=${MUTATE_MANIFEST:-$repo/tools/mutants/manifest.txt}
equiv=${MUTATE_EQUIVALENT:-$(dirname "$manifest")/EQUIVALENT.txt}
ttl=${MUTATE_TIMEOUT:-20m}; bttl=${MUTATE_BUILD_TIMEOUT:-20m}
only=""; list=0
while [ $# -gt 0 ]; do
  case "$1" in
    --only) only=${2:-}; shift 2 ;;
    --list) list=1; shift ;;
    *) echo "usage: tools/mutate.sh [--only MUTANT_ID] [--list]"; exit 2 ;;
  esac
done
[ -f "$manifest" ] || { echo "mutate: no manifest at $manifest"; exit 2; }
rows=$(grep -v -E '^[[:space:]]*(#|$)' "$manifest" || true)
if [ "$list" = 1 ]; then printf '%s\n' "$rows" | cut -f2; exit 0; fi

base=${MUTATE_TMP:-/tmp}; case "$base" in /home/jsc64/MathWorks*|"$repo"*) echo "mutate: MUTATE_TMP must be outside the repo"; exit 2 ;; esac
work=""; tgt=""
cleanup() {
  if [ -n "$work" ]; then
    git -C "$repo" worktree remove --force "$work" >/dev/null 2>&1
    rm -rf "$work"; git -C "$repo" worktree prune >/dev/null 2>&1
  fi
  [ -n "$tgt" ] && rm -rf "$tgt"
  work=""; tgt=""
}
trap cleanup EXIT; trap 'cleanup; exit 130' INT TERM

nk=0; ns=0; ne=0; ni=0; total=0
tgt=$(mktemp -d "$base/zb-mutate-target.XXXXXX")
if [ -n "$rows" ]; then
while IFS="$(printf '\t')" read -r fnpath id patch suite; do
  [ -n "$only" ] && [ "$only" != "$id" ] && continue
  total=$((total + 1))
  if [ -z "$fnpath" ] || [ -z "$id" ] || [ -z "$patch" ] || [ -z "$suite" ]; then echo "INVALID $id: manifest row needs 4 tab-separated columns"; ni=$((ni + 1)); continue; fi
  pfile=$(dirname "$manifest")/$patch
  [ -f "$pfile" ] || { echo "INVALID $id: patch file $pfile not found"; ni=$((ni + 1)); continue; }
  work=$(mktemp -d "$base/zb-mutate.XXXXXX"); rmdir "$work"
  if ! git -C "$repo" worktree add --detach -q "$work" HEAD >/dev/null 2>&1; then echo "INVALID $id: cannot create worktree"; ni=$((ni + 1)); work=""; continue; fi
  src=${fnpath%%:*}
  if [ ! -f "$work/$src" ]; then echo "INVALID $id: $src does not exist at HEAD"; ni=$((ni + 1)); cleanup; tgt=$(mktemp -d "$base/zb-mutate-target.XXXXXX"); continue; fi
  if ! git -C "$work" apply "$pfile" >/dev/null 2>&1; then echo "INVALID $id: patch does not apply"; ni=$((ni + 1)); cleanup; tgt=$(mktemp -d "$base/zb-mutate-target.XXXXXX"); continue; fi
  if ! (cd "$work" && CARGO_TARGET_DIR="$tgt" timeout "$bttl" cargo test --test "$suite" --no-run >"$tgt/build.log" 2>&1 </dev/null); then
    echo "INVALID $id: mutant does not build suite $suite (see: first lines below)"; head -5 "$tgt/build.log" | sed 's/^/    /'; ni=$((ni + 1))
  else
    (cd "$work" && CARGO_TARGET_DIR="$tgt" timeout "$ttl" cargo test --test "$suite" >"$tgt/run.log" 2>&1 </dev/null); rc=$?
    if [ "$rc" = 0 ]; then
      if [ -f "$equiv" ] && grep -v -E '^[[:space:]]*(#|$)' "$equiv" | cut -f1 | grep -qxF -- "$id"; then
        echo "SURVIVED-EQUIVALENT $id ($fnpath, suite $suite): documented in $(basename "$equiv")"; ne=$((ne + 1))
      else
        echo "SURVIVED $id ($fnpath, suite $suite): the suite passed on the mutant"; ns=$((ns + 1))
      fi
    elif [ "$rc" = 124 ]; then echo "KILLED $id ($fnpath, suite $suite): timed out after $ttl"; nk=$((nk + 1))
    else echo "KILLED $id ($fnpath, suite $suite): suite failed (rc $rc)"; nk=$((nk + 1)); fi
  fi
  cleanup; tgt=$(mktemp -d "$base/zb-mutate-target.XXXXXX")
done <<< "$rows"
fi
if [ -n "$only" ] && [ "$total" = 0 ]; then echo "mutate: no mutant with id $only"; exit 2; fi
echo "mutate: $total mutants: $nk killed, $ns survived, $ne survived-equivalent, $ni invalid"
[ "$ns" = 0 ] && [ "$ni" = 0 ]
