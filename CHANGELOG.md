# Changelog

## Unreleased

- Run logs now go through the shared Pop logger (Jeff, 2026-10-23 9:59 AM CT; Paul and Rusty): `tools/runlog.sh` sources
  `~/pop-logs/bin/poplog.sh` instead of writing logs and `RUNNING.tsv` itself, so logs, the canonical 8-column `RUNNING.tsv` row (lock
  `RUNNING.tsv.lock`) and 30-day pruning (`poprotate`) are poplog's. zb keeps a thin layer: step output still on stdout, a failed or zero-test
  step returns nonzero and fails the run, no second log for a wrapped script inside a wrapped run or inside `poprun`, heartbeat kept off stdout.
- Run logs (Jeff, 2026-10-23 9:42 AM CT): new `tools/runlog.sh`, sourced by every `tools/*.sh` (`runlog_wrap JOB "$@"`), and usable one-shot
  for verify and test runs: `tools/runlog.sh JOB [--tests] -- CMD`. Each run logs to `~/pop-logs/<agent>/<date>/<HHMM>-<job>.log` (header,
  per-step timing, exit code and cargo test counts, 5-minute heartbeat, PASS/FAIL footer with warnings) and holds a row in
  `~/pop-logs/RUNNING.tsv` under `flock ~/pop-logs/.running.lock`. A `--tests` step that runs zero tests fails. In `full_gate.sh` every step is
  a run-log step (build-debug and build-release are `--tests`), and a wrapped run now exits 1 when any step failed, even if the script exits 0.
  Self-test: `tools/runlog_selftest.sh`.
- Goldens: the generators now run only in the isolated oracle venv `~/.local/oracle-venv/bin/python` (mpmath 1.3.0, sympy 1.14.0, gmpy2 2.3.2,
  python-flint 0.9.0, the zenith-float `scripts/oracle-requirements.lock` pins), replacing `/usr/bin/python3` with Ubuntu mpmath 1.2.1 (Jeff,
  2026-10-23). `scripts/goldlib.py` checks the four exact versions, that `sys.prefix` is the venv and that no `~/.local` path is on `sys.path`
  at startup. The golden files' provenance header line changed; every value is byte-identical.
