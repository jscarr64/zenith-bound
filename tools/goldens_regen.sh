#!/usr/bin/env bash
# Golden regeneration gate (TEST_PLAN section 9): the goldlib self-test, then every generator with `--check DIR` into a fresh temp dir
# under $(zb_scratch) (byte-compared with tests/goldens), then `sha256sum -c tests/goldens/SHA256SUMS`. Uses the oracle venv ~/.local/oracle-venv/bin/python (scripts/goldlib.py require_oracle). No network.
# Temp files go under $(zb_scratch). `--slices-only` runs only the per-slice checks (no oracle; tools/goldens_regen_selftest.sh uses it).
# Per-slice SHA256SUMS: entries in strict sha256sum format, or, with no entries, an explicit `# EMPTY` marker line (never both).
# Untracked *_heavy.txt goldens (.gitignored; hashes pinned elsewhere) are not demanded in SHA256SUMS. Nothing under the repo is written. To add a generator, add a line to the GENERATORS list.
# shellcheck source=tools/runlog.sh
. "${BASH_SOURCE[0]%/*}/runlog.sh"; runlog_wrap goldens_regen "$@"   # run log (tools/runlog.sh)
set -u
export LC_ALL=C
# PYTHONNOUSERSITE=1: nothing from ~/.local outside the venv (its 'flint' is an unrelated tool). GOLDENS_PYTHON may override the interpreter,
# but goldlib's startup check (four exact pins, sys.prefix is the venv, no ~/.local on sys.path) still applies, so only the venv passes.
export PYTHONNOUSERSITE=1
root=${GOLDENS_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}
PY=${GOLDENS_PYTHON:-$HOME/.local/oracle-venv/bin/python}
GENERATORS="scripts/gen_goldens.py scripts/gen_ball_goldens.py scripts/gen_ball_enclosure.py"
# stdout generators (no --check): name and committed file
STDOUT_GENERATORS="scripts/gen_review_goldens.py:mag_ops_review.txt"
slices_only=0; [ "${1:-}" = --slices-only ] && slices_only=1
cd "$root" || exit 2
tmp=$(mktemp -d "$(zb_scratch)"/zb-goldens-regen.XXXXXX) || { echo "goldens-regen: mktemp failed"; exit 2; }; runlog_on_exit "rm -rf \"$tmp\""
# Pop direct-run guard (~/pop-logs/bin/popguard.sh, 11:49 2026-10-24: a bare 58 GB generator run global-OOMed Pop): before the
# first generator, refuse (exit 3) outside a popslot/poprun memory scope. Hosts without popguard.sh (CI) run unguarded.
POPGUARD=${POPGUARD:-$HOME/pop-logs/bin/popguard.sh}; pg_ok=0
pg_require() { [ "$pg_ok" = 1 ] && return 0; if [ -x "$POPGUARD" ]; then "$POPGUARD" goldens_regen || exit 3; fi; pg_ok=1; }
rc=0
if [ "$slices_only" = 0 ]; then
pg_require
runlog_step goldlib-selftest -- "$PY" scripts/goldlib.py --selftest || rc=1
for g in $GENERATORS; do
  runlog_step "$(basename "$g" .py)" -- "$PY" "$g" --check "$tmp/$(basename "$g" .py)" || { echo "goldens-regen: $g FAILED"; rc=1; }
done
for e in $STDOUT_GENERATORS; do
  g=${e%%:*}; f=${e#*:}
  { "$PY" "$g" > "$tmp/$f" && cmp "$tmp/$f" "tests/goldens/$f" && echo "check: OK $f"; } || { echo "goldens-regen: $g FAILED"; rc=1; }
done
(cd tests/goldens && sha256sum --strict -c SHA256SUMS) || { echo "goldens-regen: sha256sum -c FAILED"; rc=1; }
fi
# per-slice golden dirs (S0, 2026-10-24): tests/goldens/slices/<slice>/ with its own GENERATORS and SHA256SUMS, so slices never share a file
for d in tests/goldens/slices/*/; do
  [ -d "$d" ] || continue
  s=$(basename "$d")
  [ -f "$d/SHA256SUMS" ] && [ -f "$d/GENERATORS" ] || { echo "goldens-regen: slice $s lacks SHA256SUMS or GENERATORS"; rc=1; continue; }
  for g in $(grep -v -E '^[[:space:]]*(#|$)' "$d/GENERATORS"); do
    pg_require
    runlog_step "$s-$(basename "$g" .py)" -- "$PY" "$g" --check "$tmp/slice-$s-$(basename "$g" .py)" || { echo "goldens-regen: $g FAILED"; rc=1; }
  done
  entries=$(grep -v -E '^[[:space:]]*(#|$)' "$d/SHA256SUMS")
  marked=0; grep -q -E '^# EMPTY([: ]|$)' "$d/SHA256SUMS" && marked=1
  if [ -z "$entries" ]; then
    [ "$marked" = 1 ] || { echo "goldens-regen: slice $s: SHA256SUMS has neither entries nor a '# EMPTY' marker line"; rc=1; }
  else
    [ "$marked" = 0 ] || { echo "goldens-regen: slice $s: SHA256SUMS is marked '# EMPTY' but lists entries"; rc=1; }
    bad=$(printf '%s\n' "$entries" | grep -v -E '^[0-9a-f]{64}  [^/[:space:]]+$' | head -3)
    [ -z "$bad" ] || { echo "goldens-regen: slice $s: malformed SHA256SUMS line(s): $bad"; rc=1; }
    (cd "$d" && printf '%s\n' "$entries" | sha256sum --strict -c -) || { echo "goldens-regen: slice $s sha256sum -c FAILED"; rc=1; }
  fi
  for f in "$d"*.txt; do
    [ -f "$f" ] || continue
    case "$f" in *_heavy.txt) continue ;; esac   # heavy goldens: .gitignored, pinned outside SHA256SUMS
    grep -q -E "^[0-9a-f]{64}  $(basename "$f")\$" "$d/SHA256SUMS" || { echo "goldens-regen: slice $s: $(basename "$f") is not in its SHA256SUMS"; rc=1; }
  done
done
# every committed golden file must be listed in SHA256SUMS
[ "$slices_only" = 1 ] || for f in tests/goldens/*.txt; do
  grep -q -E "^[0-9a-f]{64}  $(basename "$f")\$" tests/goldens/SHA256SUMS || { echo "goldens-regen: $(basename "$f") is not in SHA256SUMS"; rc=1; }
done
[ "$rc" = 0 ] && echo "goldens-regen: ok" || echo "goldens-regen: FAILED"
exit "$rc"
