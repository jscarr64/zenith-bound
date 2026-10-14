# Contributing to zenith-bound

zenith-bound is pre-release. Every rule below is enforced in review, and most of them in CI as well. The plan is [`docs/PLAN.md`](docs/PLAN.md); the correctness contract will be `docs/CONTRACT.md`.

## Hard rules

1. **100% Rust, `#![forbid(unsafe_code)]`.** No exceptions in the published crate. The only `unsafe` in the repo is the FPU-rounding behaviour test in the separate dev-only crate `fpu-check/` (`publish = false`), which zenith-bound never depends on.
2. **No IEEE floats in the math.** No `f32`/`f64`, no `as f64`, no `mul_add`/FMA, no FPU rounding-mode control, no `core::arch`/`std::arch`, no `#[target_feature]`, no CPU-feature detection. Directed rounding comes only from zenith-float's `RoundingMode::Up`/`Down`, and radii come from the integer-only `Mag` type. CI checks this with clippy lints, `scripts/check_no_float.sh`, a HIR grep, a `thumbv6m-none-eabi` (no-FPU) build and symbol check, and the `fpu-check` behaviour test.
3. **No `unwrap`/`expect`, no `#[allow]`, no dead code.**
4. **MSRV is the frozen pin, Rust 1.99** (`rust-toolchain.toml`, `rust-version = "1.99"`). Do not use features newer than 1.99. The pin moves only by a deliberate decision, in a minor (0.x) or major release, named in `CHANGELOG.md`.
5. **License:** contributions are accepted under `MIT OR Apache-2.0`. Files ported from puremp keep their SPDX header naming the puremp commit (see `NOTICE-puremp`).

## Clean-room rule (LGPL oracles)

Arb/FLINT and MPFR are licensed under the **LGPL**. We use them (and mpmath) only as **dev-only test oracles**.

- Implement algorithms **from the published papers and textbooks only** (for example Johansson's Arb papers, Petras, Rump, Lohner, Olver, Carlson). **Never read, copy, translate or paraphrase Arb/FLINT or MPFR source code** while working on zenith-bound, and don't paste their code into prompts for coding models.
- Cite the paper in the module doc of anything that implements a published bound.
- Oracles are never a dependency in any `Cargo.toml` of the published crate, and they are never shipped. `cargo-deny` rejects LGPL licenses in the dependency graph.
- Goldens derived from oracle output live under `tests/` and `scripts/` and are excluded from the published package (see `NOTICE`).

## Tests

- **Red first.** Every change starts as failing tests that are committed (or shown in review) before the fix.
- **No single exact-input tests.** Every test is a **property set**, an **oracle set**, an **adversarial set** (a named, enumerated generator) or an **overestimation measurement**. A one-off `assert_eq!(f(3), …)` is rejected. proptest regression files keep failing seeds.
- **Zero is in every set**: zero midpoint, zero radius, and both.
- **Truth must be independent of zenith-float.** Never use zf's `*_full_prec` ops or any other zf result as truth, because that is circular. Use the dev-only rational/bigint model (`tests/model/`) or Python-generated goldens.
- **Containment failures are P0** (and security-grade, see `SECURITY.md`). Tightness misses are performance bugs and are tracked as overestimation ratios.
- **Mutation checks:** shrinking any radius by 1 ulp (`cfg(zb_shrink_radius)`) must make the suite fail, and `cargo-mutants` runs nightly on `src/mag`, `src/ball` and `src/encode.rs`.

## Oracle setup (dev only)

```sh
python3 -m venv .venv && . .venv/bin/activate
pip install -r scripts/requirements.lock   # pinned: mpmath, python-flint
python3 scripts/gen_goldens.py             # writes tests/goldens/*.jsonl
```

MPFR is reached only through zenith-float's own test suite (via `rug`); zenith-bound has no MPFR dependency. Goldens are regenerated only in a commit that says why, and a byte-golden change also needs an `op_ver` bump.

## Review

- Each diff is hunk-sized and gets a bot review. Until 2026-10-30, code is written by the local model qwen3-coder:30b under hunk-sized directives.
- **Every file under `src/mag/` also needs one review by a stronger reviewer (Paul or Cassie)** before it merges.
- CI must be green: fmt, clippy (lint config reused from zenith-float), the no-float guards, tests on the CI matrix, `cargo-deny`, `cargo-semver-checks`.
- The README status table may call a function **Guaranteed** only after its contract tests pass in CI.
