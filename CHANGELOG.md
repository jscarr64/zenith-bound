# Changelog

## Unreleased

- Spence review fixes for c471eca..7b64d57 (2026-10-23, protected `tools/`, red-first):
  - No silent pass (BLOCK): `full_gate.sh` keeps a running `gate_rc`, writes `done` only when every step passed (`failed` otherwise) and ends with `exit $gate_rc`. `runlog.sh`: a missing or unloadable poplog.sh is a hard failure (exit 2); `RUNLOG_PLAIN=1` is the explicit fixture-only plain mode. `runlog_step --tests` enforces 0-tests=FAIL in the zb layer too (with or without poplog). New selftest cases: missing poplog, failed step exits non-zero and writes no `done`, plain-mode zero tests.
- Gate rerun fixes (Jeff, 2026-10-23 10:33 AM CT). These touch protected `tools/` files, so they need Spence's review:
  - `tools/runlog.sh` v3 relies on poplog's FAIL verdict and exit code. The `/dev/null` stdout workaround and the zb-fail markers are gone. A nested full_gate under poprun now exits nonzero, shows FAIL and writes step output into the parent log. There is a red-first selftest case for this.
  - Harness goldens resolve at runtime through `common::goldens::repo_root()`, not compile-time `env!("CARGO_MANIFEST_DIR")`. A binary built in one worktree used to read another worktree's goldens. The new gate step `manifest-scan` bans the `env!` form.
  - A wrapped tool registers its exit cleanup with `runlog_on_exit`, not a raw `trap ... EXIT`, and does not `exec` a command. Both used to skip poplog's finish, which left logs without a footer and RUNNING.tsv rows stuck in RUNNING. Self-tests call `runlog_detach_children` so their deliberately failing fixtures do not FAIL the self-test's own run. A lint in `runlog_selftest.sh` enforces both rules, red first.
  - The gate's build-tests step runs `cargo build --tests`. poplog treats any `cargo test` command as a test step, so the old `cargo test --no-run` read as zero tests and FAILed (first seen in the 11:15 AM CT gate). `full_gate_selftest.sh` now rejects the old form.
  - New `tools/fresh_test_run.sh` is used by build-debug and build-release. It fails if a step runs 0 test binaries, a binary from outside `CARGO_TARGET_DIR`, or a binary older than the gate start. As a result, the gate needs a fresh per-job target dir, `/tmp/target-harper/<job>`.

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
