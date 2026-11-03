#!/usr/bin/env bash
# Self-test for .gitattributes (ZB-TD41): goldens must be byte-exact (-text)
# and ordinary text files must check out LF. Fails on a tree with no
# .gitattributes, which is the red state for the .gitattributes change.
# Also proves the hazard the .gitattributes prevents: a CRLF-converted
# golden fails its SHA256SUMS entry. Plain bash on purpose: this script
# must run in Vera's VM too, where tools/runlog.sh refuses (no poplog).
set -u
here="$(cd "$(dirname "$0")/.." && pwd)"
fail() { echo "GITATTRIBUTES SELFTEST FAIL: $*"; exit 1; }
[ -f "$here/.gitattributes" ] || fail "no .gitattributes at repo root"
attr() { git -C "$here" check-attr -a -- "$1"; }
golden="$(attr tests/goldens/mag_ops.txt)"
echo "$golden" | grep -q "text: unset" || fail "mag_ops.txt check-attr: $golden"
text="$(attr README.md)"
echo "$text" | grep -q "eol: lf" || fail "README.md check-attr: $text"
# Spence review of PR #13: do not put the scratch dir under $HOME; use
# $TMPDIR (falling back to /tmp only when $TMPDIR is unset, as in Vera's VM).
tmp="$(mktemp -d "${TMPDIR:-/tmp}/.gitattributes-selftest.XXXXXX")" || fail "mktemp failed"
hash="$(sha256sum "$here/tests/goldens/mag_ops.txt" | cut -d' ' -f1)"
sed 's/$/\r/' "$here/tests/goldens/mag_ops.txt" > "$tmp/mag_ops_crlf.txt"
printf '%s  mag_ops_crlf.txt\n' "$hash" > "$tmp/SHA256SUMS"
(cd "$tmp" && sha256sum --strict -c SHA256SUMS) && {
  rm -rf "$tmp"
  fail "CRLF-converted golden passed its SHA256SUMS entry"
}
rm -rf "$tmp"
echo "gitattributes selftest: ok"
