# Changelog

## Unreleased

- Goldens: the generators now run only in the isolated oracle venv `~/.local/oracle-venv/bin/python` (mpmath 1.3.0, sympy 1.14.0, gmpy2 2.3.2,
  python-flint 0.9.0, the zenith-float `scripts/oracle-requirements.lock` pins), replacing `/usr/bin/python3` with Ubuntu mpmath 1.2.1 (Jeff,
  2026-10-23). `scripts/goldlib.py` checks the four exact versions, that `sys.prefix` is the venv and that no `~/.local` path is on `sys.path`
  at startup. The golden files' provenance header line changed; every value is byte-identical.
