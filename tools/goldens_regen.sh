#!/usr/bin/env bash
# Golden regeneration gate (TEST_PLAN section 9): the goldlib self-test, then every generator with `--check DIR` into a fresh temp dir
# under /tmp (byte-compared with tests/goldens), then `sha256sum -c tests/goldens/SHA256SUMS`. Uses /usr/bin/python3 (mpmath 1.2.1). No network.
# Nothing under the repo is written. To add a generator, add a line to the GENERATORS list.
set -u
export LC_ALL=C
root=${GOLDENS_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}
PY=${GOLDENS_PYTHON:-/usr/bin/python3}
GENERATORS="scripts/gen_goldens.py"
cd "$root" || exit 2
tmp=$(mktemp -d /tmp/zb-goldens-regen.XXXXXX); trap 'rm -rf "$tmp"' EXIT
rc=0
"$PY" scripts/goldlib.py --selftest || rc=1
for g in $GENERATORS; do
  "$PY" "$g" --check "$tmp/$(basename "$g" .py)" || { echo "goldens-regen: $g FAILED"; rc=1; }
done
(cd tests/goldens && sha256sum -c SHA256SUMS) || { echo "goldens-regen: sha256sum -c FAILED"; rc=1; }
# every committed golden file must be listed in SHA256SUMS
for f in tests/goldens/*.txt; do
  grep -q -E "^[0-9a-f]{64}  $(basename "$f")\$" tests/goldens/SHA256SUMS || { echo "goldens-regen: $(basename "$f") is not in SHA256SUMS"; rc=1; }
done
[ "$rc" = 0 ] && echo "goldens-regen: ok" || echo "goldens-regen: FAILED"
exit "$rc"
