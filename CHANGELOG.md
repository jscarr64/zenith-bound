# Changelog

## Unreleased

- RUNLOG_PLAIN needs a selftest-only marker (Spence follow-up, non-blocking, 2026-10-23, protected `tools/`, red-first): `runlog.sh` (`runlog_wrap`, so `full_gate.sh` and every wrapped script and one-shot run) exits 2 with a message naming `RUNLOG_SELFTEST` when `RUNLOG_PLAIN=1` is set without `RUNLOG_SELFTEST=1`. Only the self-test fixtures (`full_gate_selftest.sh`, `runlog_selftest.sh`, `fuzz_deep.sh --selftest`) export the marker. New red-first cases in `full_gate_selftest.sh` and `runlog_selftest.sh`.
- One target dir per worktree (Paul, 2026-10-23 1:02 PM CT, protected `tools/`, red-first): `tools/fresh_test_run.sh` fails if `CARGO_TARGET_DIR` is unset or the dir is owned by another worktree (marker `$CARGO_TARGET_DIR/.worktree-owner`, written by the first worktree to use it; `--claim` runs only this check). New gate step `target-dir` runs it before any build; `mutants` no longer falls back to `/tmp/zb-target`. Selftest: two worktrees on one dir fail (SHARED, naming the owner), an unset `CARGO_TARGET_DIR` fails. CONTRIBUTING updated.
- Spence review fixes for c471eca..7b64d57 (2026-10-23, protected `tools/`, red-first):
  - No silent pass (BLOCK): `full_gate.sh` keeps a running `gate_rc`, writes `done` only when every step passed (`failed` otherwise) and ends with `exit $gate_rc`. `runlog.sh`: a missing or unloadable poplog.sh is a hard failure (exit 2); `RUNLOG_PLAIN=1` is the explicit fixture-only plain mode. `runlog_step --tests` enforces 0-tests=FAIL in the zb layer too (with or without poplog). New selftest cases: missing poplog, failed step exits non-zero and writes no `done`, plain-mode zero tests.
  - `fresh_test_run.sh` (Spence's ruling): the age test is gone; a test binary must lie inside `$CARGO_TARGET_DIR`. The 0-binaries check and the manifest-scan gate step stay. The gate may reuse a target dir again; per-job subdirs are optional. Usage is now `tools/fresh_test_run.sh CMD...`.
  - `mutate.sh`: 0 mutants is a failure ("0 mutants ran"), so the gate's mutants step needs at least one mutant; the selftest's empty-manifest case now expects failure.
  - `runlog_wrap` sets `pipefail` (red case in `runlog_selftest`). Pipelines made safe under it: `mutate.sh`'s equivalence lookup is one `awk`; piped `grep -q` became `grep -c > /dev/null` so the writer can't hit SIGPIPE (rc 141); commands whose exit the selftests deliberately ignore (`scan --count-only`, `tightness check/report`) are wrapped in `{ ... || true; }`.
  - `fuzz_deep.sh`: every shard and the replay must report `1 passed` (a renamed or filtered-out target is a failure); `timeout` on the release build (`FUZZ_DEEP_BUILD_TIMEOUT`, default 30m) and each shard (`FUZZ_DEEP_SHARD_TIMEOUT`, default 6h); default deep-lock wait 3600 s, announced; targets come from `tools/fuzz_deep_targets.txt` (new) so Spence's port adds `fuzz_mag_l0a5` there. Shard logs move to `OUT/<target>/shard-N.log`. Selftest fixtures run with `RUNLOG_PLAIN=1`.

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
