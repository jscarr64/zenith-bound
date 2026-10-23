#!/usr/bin/env bash
# Golden regeneration gate (TEST_PLAN section 9): the goldlib self-test, then every generator with `--check DIR` into a fresh temp dir
# under /tmp (byte-compared with tests/goldens), then `sha256sum -c tests/goldens/SHA256SUMS`. Uses the oracle venv ~/.local/oracle-venv/bin/python (scripts/goldlib.py require_oracle). No network.
# Nothing under the repo is written. To add a generator, add a line to the GENERATORS list.
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
cd "$root" || exit 2
tmp=$(mktemp -d /tmp/zb-goldens-regen.XXXXXX); trap 'rm -rf "$tmp"' EXIT
rc=0
"$PY" scripts/goldlib.py --selftest || rc=1
for g in $GENERATORS; do
  "$PY" "$g" --check "$tmp/$(basename "$g" .py)" || { echo "goldens-regen: $g FAILED"; rc=1; }
done
for e in $STDOUT_GENERATORS; do
  g=${e%%:*}; f=${e#*:}
  { "$PY" "$g" > "$tmp/$f" && cmp "$tmp/$f" "tests/goldens/$f" && echo "check: OK $f"; } || { echo "goldens-regen: $g FAILED"; rc=1; }
done
(cd tests/goldens && sha256sum -c SHA256SUMS) || { echo "goldens-regen: sha256sum -c FAILED"; rc=1; }
# every committed golden file must be listed in SHA256SUMS
for f in tests/goldens/*.txt; do
  grep -q -E "^[0-9a-f]{64}  $(basename "$f")\$" tests/goldens/SHA256SUMS || { echo "goldens-regen: $(basename "$f") is not in SHA256SUMS"; rc=1; }
done
[ "$rc" = 0 ] && echo "goldens-regen: ok" || echo "goldens-regen: FAILED"
exit "$rc"
