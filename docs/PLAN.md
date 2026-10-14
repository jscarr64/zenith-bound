# Plan: zenith-bound, rigorous ball arithmetic on zenith-float (revision 2026-10-14)

- **Branch under review:** `jscarr64/zenith-float` `cursor/oracle-step1-2-0-0-79c6`, head `508a40039fcb6118d4eb07f14596f0153639298f` (committed 2026-10-14 10:57 CT; `04ba6a8` is in its history). Read through the GitHub connector only. No clone, no push, no PR.
- **This document:** a plan only. It revises `zenith-bound_PLAN_2026-10-13.md` and folds in Jeff's briefs B1–B9. Part K is the Task 1 audit of zf's rounding that this revision rests on.
- **Decisions (locked):**
  - Name: **zenith-bound**. A new, separate open-source crate and repo on top of zenith-float (zf). Not a feature inside zf.
  - License: **MIT OR Apache-2.0**. Files ported from puremp keep puremp's MIT notice (`NOTICE-puremp`).
  - Ports puremp (MIT, KarpelesLab) with credit. puremp's `f64` lattice path is dropped.
  - Rust 1.99, 100% Rust, `#![forbid(unsafe_code)]`, no IEEE `f32`/`f64` anywhere in the math, no `unwrap`/`expect`, no `#[allow]`, no dead code.
  - Scope L0–L4 before the Accumath launch. Order: zf 2.0.0 intrinsics fix (Pop gate) → zf 2.1 prerequisites (C1) → zenith-bound L0 → L4.
  - The `Mag` design is **locked** (brief 8 + brief 9, Part C6).
  - Tests are written red first. **No single exact-input tests** (Jeff's ban, B3): every test is a property, oracle or adversarial *set*, or an overestimation measurement.
- **Core principle (B2):** every result either carries a defensible enclosure or explicitly reports that the guarantee could not be established. Correctness is kept separate from tightness.
- **Constraint:** MPFR, mpmath and FLINT/Arb (via python-flint) are dev-only oracles. Never shipped, never a dependency.
- **Coding constraint until 2026-10-30:** coding is done by Pop's only local model, qwen3-coder:30b (MoE, temp ≤ 0.1, low top_p, fixed seed), with hunk-sized directives that bots review. The old ≤10 KB/15 KB limits came from the deleted qwen2.5-coder 7B's context and are lifted; L0 keeps one responsibility per file with a soft target of ≤ ~30 KB per file so review diffs stay small (Part D, L0 file plan).

## Summary in plain language

zf already has a small prototype `Ball` (midpoint ± radius), but its radius is a full-precision number, some functions add a guessed "8 ulps", and nothing proves the true answer is inside.

**zenith-bound** does this properly, Arb-style. A ball is `Ball { mid: ExactNum, rad: Mag, prec: u32 }`. The radius `Mag` is a tiny integer-only type (a 64-bit mantissa and a wide exponent) that is always rounded **up**. Every operation computes the midpoint, measures the rounding error it made, and adds that error to the radius. Nothing is ever dropped. When a result can't be enclosed (division by a ball that touches 0, a log of a ball that reaches 0, an overflow), the API says so with an explicit error or "indeterminate" value instead of returning a confident-looking finite ball.

**What the Task 1 audit (Part K) changed.** zf's `Up` and `Down` modes really are directed roundings toward +∞ and −∞ for add, sub, mul, div and sqrt in the finite range, and MPFR checks them bit-exactly. But three things block using them as-is for bounds:
1. **Overflow ignores the rounding mode.** An overflowing result always becomes ±Inf, even in `Down` for a positive value, where the correct answer is the largest finite number. A lower bound of +Inf is wrong. The tests and the golden generator both encode this behaviour, so nothing catches it.
2. **The `inexact` flag is inherited from inputs**, so it can't tell whether *this* operation rounded. Zero radii need a per-op ternary result.
3. **The exponent is `i32`, and is a quarter of that range on 32-bit targets.** That breaks cross-target determinism near the extremes and is too narrow for a radius exponent. `Mag` therefore uses an `i64` exponent.

zf precision is also rounded up to a multiple of 64 bits. zenith-bound's `prec` (`u32`) is documented as "at least this many bits", and its tight-rounding tests use that.

**Order:** zf 2.0.0 fix → zf 2.1 prerequisites (now including the overflow fix) → L0 contract and real balls → L1 elementary functions and constants → L2 complex, polynomials, series, matrices, then selected specials → L3 quadrature → L4 validated workflows (root isolation, verified linear solve, ODE) and adaptive precision. Each layer starts with failing tests and is green on Pop before the next.

**Rough size: ~173–331 runs** (was ~147–268). Jeff discounts AI timeline estimates, so read these only as relative sizes between items. The growth comes from the bigger zf 2.1 set, the contract layer, `ComplexBall`/polynomial/series/matrix work, and review-sized L0 steps for the local model.

zf's old `Ball` is deprecated in zf 2.1 (pointing to zenith-bound) and removed in zf 3.0. zenith-bound starts at 0.0.0 (scaffold) and 0.1.0 (L0).

## Part A. What exists in zf (surveyed on `16141841`; rounding re-audited on `508a400` in Part K)

| Area | What is there | File | Rigorous? |
|---|---|---|---|
| Real ball | `Ball { mid: ExactNum, rad: ExactNum }`. Has `new`, `add`, `mul`, `exp`, `sin`, `cos`, `ln`, `sqrt`, `erf`, `bessel_j0`, `bessel_j1`, `contains`, and private `strictly_positive` and `excludes_zero` | `zenith-float-num/src/ball.rs:17-239` | Partly. See gaps |
| Complex ball | `ComplexBall { mid: ExactComplex, rad: ExactNum }`, a disk. Has `add`, `mul`, `exp`, `ln`, `sin`, `cos`, `contains` | `ball.rs:241-~360` | Partly |
| Ziv retry | `ziv_round`, `ziv_round_vec`. They use the ball to certify correct rounding | `ball.rs:~380-430` | Yes, for its purpose |
| Ball goldens | `balls.jsonl`, run by `methods_oracle.rs::balls`. Covers `ball_exp`, `ball_sin`, `ball_add`, `ball_mul`. It checks only that the truth is inside the ball, not how tight the ball is | `zenith-float-num/tests/oracle/goldens/balls.jsonl`, `tests/methods_oracle.rs:336-360,429` | Inclusion only |
| Rounding modes | `RoundingMode { None=1, Up=2, Down=4, ToZero=8, FromZero=16, ToEven=32, ToOdd=64 }`. `Up`/`Down` are directed (the docs wrongly say "round half") | `zenith-float-num/src/defs.rs:251-272` | **Finite range only, tested not proven; overflow ignores the mode (Part K3).** `None` truncates, unsafe for bounds |
| Mag / radius type | **None.** The radius is a full `ExactNum` at working precision `p` | `ball.rs:21` | — |
| Error terms in functions | Ziv-style correct rounding inside the scalar kernels (`ops/*.rs`, `ops/series.rs`). Constants live in `ops/consts/{pi,e,ln2,ln10}.rs`. No function exports a proven error bound | `zenith-float-num/src/ops/` | Internally, not exposed |
| GK quadrature | `GkResult.abs_err_est` is `|K−G|` plus 1 ulp, rounded away from zero. A heuristic, **not** a bound. Also has `integrate_adaptive_gk` and `wynn_epsilon` | `zenith-float-num/src/gauss_kronrod.rs:102-179,647-684,747,799-830` | No |
| Other quadrature | `tanh_sinh` and the rest | `zenith-float-num/src/quadrature.rs` | No |
| ODE | `rk4`, `euler`, `rk45_adaptive`. The error estimate is `|y5−y4|`, computed with `RoundingMode::None` | `zenith-float-num/src/ode.rs:75,174,457-540` | No |
| Linear algebra | Point arithmetic only. Has benchmarks, and SVD/Householder is mentioned in clippy notes | `benches/linalg.rs`, `clippy.toml` | No |
| Roots | Point root-finding | `zenith-float-num/src/roots.rs` | No |

**Gaps in the current `Ball` that L0 must fix:**
1. `contains` computes `|mid − x|` with `RoundingMode::None` (`ball.rs:236`). That truncates, so it can wrongly report "contained". It must round up, or compare exactly.
2. `bessel_j0` uses `transcendental_slack`, which is `BALL_TRANSCENDENTAL_ERROR_TERMS = 8` ulps (`ball.rs:14,166-171,192-199`). That number is a guess with no proof. `sin` and `cos` add `self.rad + 1 ulp`. That is right only if the midpoint kernel is truly correctly rounded, and nothing checks that the kernel met its target.
3. There is no `sub`, `div`, `neg`, `pow`, `atan`, or `tan`. There is no tri-state comparison, no `overlaps`, `union`, `intersect` or `is_exact`, and no well-defined meaning for an infinite radius.
4. The radius is at precision `p`. That makes it slow, and it is rounded up at precision `p`, which is fine but wasteful. zenith-bound uses the locked `Mag` (64-bit mantissa, `i64` exponent, C6).
5. `nan_ball` sets the radius to NaN. zenith-bound never stores a NaN mid: it returns `BoundError` or `Indeterminate`, and `rad = Infinity` means "all reals" (Part L).
6. The `rm` argument rounds the midpoint. For a ball, the midpoint mode should only affect tightness, never correctness. Tests must show this across all 7 modes.
7. (New, K5) zf's `inexact` flag is inherited from inputs, so the prototype can't prove a zero radius.

## Part B. No hardware floats (Jeff's hard requirement)

**Rule.** Ball midpoints, radii, and every piece of error-bound math use only zenith-float software types. That means no `f32` or `f64`, no FPU rounding-mode control, no SIMD or CPU intrinsics, and no hardware FMA (`mul_add`). Directed rounding comes only from zf's own `RoundingMode::Up` and `RoundingMode::Down`.

**The `Mag` type is software.** The locked design (C6): `enum Mag { Finite { exp: i64, mant: u64 }, Infinity }`, value `mant·2^(exp−63)`, always rounded upward with integer ops only (`u128` products, `leading_zeros`, shifts). Exponent overflow becomes `Infinity`, never saturation.

**What the ball path touches today:**
- `zenith-float-num/src/ieee_soft/mod.rs:135` defines `Ieee64::mul_add`. It is software (integer bit ops), but the name matches the banned family. The ball path must not call it. zenith-bound bans `#[allow]`, so instead its `clippy.toml` `disallowed-methods` lists `Ieee64::mul_add` and the module is never imported.
- `zenith-float-num/src/ieee_soft/simd.rs:52,62,112,218,300` uses `core::arch::{x86_64,aarch64}` *integer* intrinsics (`_mm_add_epi32`, `_mm_mul_epu32`, `vaddq_u32`). The module doc says no hardware IEEE is used. The ball path must not reach this module. If `mantissa/*` reaches it through array ops, that needs to be checked.
- Connector code search found `f64` only in `ieee_soft/mod.rs` and `ops/asinh.rs`. Reading `ops/asinh.rs` turned up no `f64` token, so the hit was probably in a comment or a stale index entry. Code search is incomplete, so step L0.0 below runs a full local `rg` over every file the ball path depends on and records the result.
- `conv.rs` has public conversions from host types (for example `from_f64`-style entry points). These are fine at the API edge, but they are banned inside zenith-bound.

**Enforcement (all four are CI-blocking):**
All four guards live in the **zenith-bound repo's CI**, using the same pinned toolchain as zf (`rust-toolchain.toml`, Rust 1.99). zf keeps its own guards for the kernels zenith-bound calls.

1. **Clippy config.** zenith-bound's `clippy.toml`, plus `#![deny(clippy::disallowed_types, clippy::disallowed_methods)]` in its `lib.rs`:
   ```toml
   disallowed-types = ["f32", "f64", "core::arch", "std::arch"]   # module paths flagged via disallowed_methods below
   disallowed-methods = ["f32::mul_add", "f64::mul_add", "f64::to_bits", "f64::from_bits",
     "core::intrinsics::fmaf64", "core::intrinsics::fmaf32", "std::arch::is_x86_feature_detected"]
   ```
   Clippy can't reliably ban the primitive types themselves, so add `#![deny(clippy::float_arithmetic, clippy::float_cmp, clippy::cast_precision_loss, clippy::as_conversions)]` in `lib.rs`. `float_arithmetic` is the lint that catches any `f64` op.
2. **CI scan.** `scripts/check_no_float.sh` runs `rg -n '\bf(32|64)\b|as f(32|64)|std::arch|core::arch|target_feature|mul_add|fma' src/` in zenith-bound, plus the zf modules it calls. Any match fails CI. A second pass runs `cargo +nightly rustc -- -Zunpretty=hir` on the ball modules and greps the HIR for `f64` and `f32` types, so float use hidden behind macros is caught too.
3. **No-float build.** zenith-bound is `#![no_std]` (with `alloc`). Build it for `thumbv6m-none-eabi`, a target with **no FPU** (`--target thumbv6m-none-eabi`). Any `f64` op then lowers to a soft-float libcall, so also link with `-C link-arg=--no-undefined` and check `nm` output for `__aeabi_d*`, `__adddf3`, `__muldf3` and `fma`. Any such symbol fails the check. This proves that no float code got in. It copies zf's `nostd-tests` pattern (`nostd-tests/`, `nostd-qemu`).
4. **Behavioural test.** On x86_64, set the host FPU to round-toward-+inf with `fesetround`, from a `libc` dev-dep in a test binary only, before running the ball suite. The ball results must stay **bit-identical** to the golden hashes. Run the same check under `qemu` for `thumbv6m`, where no FPU exists. If any result depends on hardware floats, the hashes differ.

## Part K. Task 1 audit: does zf round correctly toward −∞ and +∞? (head `508a400`)

Line numbers are on `508a400`. Paths are under `zenith-float-num/` unless stated.

### K1. `RoundingMode` variants (`src/defs.rs:251-272`)

| Variant | Value | What the code does (`src/mantissa/mantissa.rs` `round_mantissa`, `:761`) |
|---|---:|---|
| `None` | 1 | No rounding. Bits beyond `p` are left as they are (truncation once stored); only `inexact` is set (`:775-786`). The golden generator treats it like `ToZero` (`scripts/gen_oracle_goldens.py:100`). **Unsafe for bounds.** |
| `Up` | 2 | Directed toward **+∞**: truncate, then add one unit at the last place if any discarded bit is set and the value is positive (`:958-962`) |
| `Down` | 4 | Directed toward **−∞**: same, when the value is negative (`:964-968`) |
| `ToZero` | 8 | Truncate (`:951`) |
| `FromZero` | 16 | Add one unit when anything is discarded (`:952-956`) |
| `ToEven` | 32 | Nearest, ties to even (`:795-879`) |
| `ToOdd` | 64 | Nearest, ties to odd (`:872-877`). This is *not* MPFR's round-to-odd (sticky) mode |

**Doc bug:** the doc comments say `Up` = "Round half toward positive infinity", `Down` = "Round half toward negative infinity", and so on (`src/defs.rs:253-271`). The code is directed rounding, not a tie rule. zf 2.1 must fix the docs (C1 row K-doc).

### K2. Does each op honour every mode?

| Op | Entry | Mode handling | Finite range | Underflow (subnormal) | Overflow |
|---|---|---|---|---|---|
| add / sub | `src/num.rs:244`, `:257` → `add_sub` `:588`; mantissa `abs_add`/`abs_sub` take `rm` and the sign (`:642-695`). Exact cancellation gives −0 under `Down` (`:666`) | all 7 | correct for Up/Down/ToZero/FromZero/ToEven/ToOdd | `subnormalize` `:724` + `flush_to_min` `:798-813` are mode-aware and look right | **wrong for directed modes** (see K3) |
| mul | `src/num.rs:296` → `mul_general_case` | all 7 | correct | mode-aware | **wrong** |
| div | `src/num.rs:384` | all 7 | correct | mode-aware | **wrong** |
| sqrt | `src/ops/sqrt.rs:21` (passes `rm` to `mantissa.sqrt`) | all 7 | correct | mode-aware | n/a (can't overflow) |

Exact full-precision ops: `add_full_prec`/`sub_full_prec`/`mul_full_prec` (`src/num.rs:270,283,309`; public `src/ext.rs:132,182,236`) use `RoundingMode::None` with `full_prec = true`, so the result keeps every bit. There is no exact division (correct, since quotients are not finite in general).

### K3. Overflow ignores the mode (blocking for bounds)

- add/mul/div return `Err(ExponentOverflow(sign))` when `e > EXPONENT_MAX` (mul `src/num.rs:350-352`, div `:416-418` and `:466`, add/sub `:707-709`), whatever `rm` is.
- `ExactNum::result_to_ext` maps that error to ±Inf for every mode (`src/ext.rs:1162-1167`).
- Correct directed behaviour (IEEE/MPFR): `Down` on a positive overflow, `Up` on a negative one, and `ToZero` all return ±`max_value`, not ±Inf. So a `Down` lower bound of a huge positive value comes back as **+Inf, which is above the true value**.
- **Why tests miss it:** `assert_float_close` accepts any zf Inf whenever MPFR raised its overflow flag (`tests/mpfr/common.rs:196-206`), even if MPFR returned the largest finite number. The golden generator returns `inf(sign)` for every mode on overflow (`scripts/gen_oracle_goldens.py:166-167,177-178`), so the goldens enshrine the bug.
- **Fix in zf 2.1** (row K3 in C1): directed overflow returns ±`max_value(p)` in the mode-appropriate direction; tests and the generator are corrected first (red).
- **zenith-bound workaround until then:** never trust a finite-looking `Down`/`Up` result near `EXPONENT_MAX`. If either input exponent is within `2·64` of `EXPONENT_MAX`, or the result is Inf, treat the ball as overflowed (`rad = Infinity`, error kind `Overflow`).

### K4. Proven or tested?

**Nothing is proven.** There is no formal proof or written error analysis of the rounding code. It is **tested**, by two independent oracles:

| Evidence | Oracle | Modes | Ops | Size |
|---|---|---|---|---|
| `tests/mpfr/compare_ops_test.rs` `mpfr_compare_ops` (`:24-31`, 1000 iterations) and `_large` (`:33-40`, 5 iterations at ≥1563 words); bit-exact (`eq = true`) | MPFR via `rug`/`gmp_mpfr_sys` | **one random mode per iteration** from ToEven/Up/Down/FromZero/ToZero (`tests/mpfr/common.rs:279-288`), so ~200 per mode per op. **ToOdd and None are never MPFR-checked** | prec-round (`:54-79`), add/sub (`:99-128`), mul/div/recip (`:132-192`), sqrt (`:490-510`), plus elementary functions | precision up to ~10k bits, exponents across the full range |
| `tests/mpfr/fuzz_round_modes.rs` `fuzz_mpfr_bit_exact_all_round_modes` (`:22`) | MPFR | the same 5, **every mode for each case** (`:14-20`) | add, mul, sqrt only. **No sub or div** | 24 pairs at p = 128, exponents ±64 |
| `tests/oracle/goldens/arith.jsonl`, run by `tests/oracle.rs` (`eval_row` `:203`, ops `:243-256`, modes `:44-54`) | Python exact integer rounding (`round_ratio`, `gen_oracle_goldens.py:96-117`) cross-checked against `scripts/mpfr_oracle.c`; a row is written only when both agree | **all 7** | add 35, sub 35, mul 28, div 28, sqrt 14 rows **per mode** (e.g. add Up from line 53, add Down from 79, sqrt Up 69-70, sqrt Down 95-96) | p ∈ {24, 53, 64, 113, 128, 256, 1024}. **zf rounds p up to a multiple of 64**, so 24 and 53 actually test 64 bits |
| `tests/oracle/goldens/rounding.jsonl` (`chop`, 28 rows per mode, all 7) | same | all 7 | re-rounding (`set_precision`) | same precisions |
| `tests/oracle/goldens/edges.jsonl` | same | all 7 | specials, min/3min/max tokens (`tests/oracle.rs:142-180`) | edge values |

Subnormal results in the MPFR differential only check MPFR's underflow flag, not the value (`tests/mpfr/common.rs:207-212`); the goldens' `min`/`3min` rows cover the values.

**Answer.** In the finite range, zf rounds toward −∞ (`Down`) and +∞ (`Up`) correctly for add, sub, mul, div and sqrt at any precision that is a multiple of 64 bits, **tested** (not proven) against MPFR and an independent exact-integer oracle in every mode. At **overflow** it is **wrong** for directed modes (K3), and the test suite hides it. The `inexact` flag is not a per-op result (K5). zenith-bound must not depend on directed rounding until the zf 2.1 overflow fix is green on Pop.

### K5. Other findings that change the plan

- **`inexact` is inherited:** `let mut inexact = self.inexact || d2.inexact` (`src/num.rs:337`, `:409`, `:633`). A result is marked inexact if any input was, even when this op was exact. It cannot give zero radii. A per-op ternary is required.
- **Precision is word-quantised:** "Precision is rounded upwards to the word size" (`src/num.rs` op docs, `round_p`). A zf number cannot be rounded to 4 bits. Ball `prec` is "at least `prec` bits". Rounding tests at tiny precisions must use zenith-bound's own integer model (C6), not zf.
- **Exponent convention:** value = 0.m × 2^e (mantissa in [1/2, 1)); `3` is stored as `e = 2`, `m = 0xC000…` (`arith.jsonl:1`). So the **leading-bit exponent is `exponent() − 1`**.
- **Exponent range is target-dependent:** `EXPONENT_MAX = i32::MAX`, `EXPONENT_MIN = i32::MIN` on 64-bit targets, but `/4` on 32-bit targets (`src/defs.rs:42-58`). That breaks bit-identical results across i686/wasm32 near the extremes (Part J).
- **`Consts` re-rounds from its cache to `p`** (`src/ops/consts/mod.rs:70-120`), so call order should not change bits; still needs the A→B vs B→A test.

### K6. The checklist for zf 2.1

| Item | Status on `508a400` | Evidence | zf 2.1 action |
|---|---|---|---|
| `exponent()` / leading-bit exponent | **EXISTS** (exponent); leading-bit is `exponent() − 1`, no named helper | `src/ext.rs:1189` → `Option<Exponent>`; `src/num.rs:995` | add `leading_bit_exponent() -> Option<i64>` (docs + test) |
| `top_u64_mantissa_and_sticky` (or similar) | **MISSING**. Parts exist: `mantissa_digits()` and `as_raw_parts()` | `src/ext.rs:1402`, `:1257` | add `top_u64_mantissa_and_sticky() -> Option<(u64, bool, i64)>` (word-size independent) |
| `from_mag_parts` / pow2 construction | **MISSING** as such. `from_words(m, s, e)` and `ldexp` can build `m·2^e` | `src/ext.rs:1296`; `ldexp` used at `tests/oracle.rs:172` | add `from_mag_parts(mant: u64, exp: i64, p) -> Result` (exact, error if out of range) and `pow2(exp: i64, p)` |
| exact add and mul (no rounding) | **EXISTS** | `add_full_prec` `src/ext.rs:132`, `sub_full_prec` `:182`, `mul_full_prec` `:236` | document exactness; overflow must be an error, not Inf |
| exponent limits and min-normal exponent | **PARTIAL**. `EXPONENT_MIN/MAX` and `min_positive_normal(p)` exist; no named min-normal exponent; range differs on 32-bit | `src/defs.rs:42-58`; `src/ext.rs:1234` | add `MIN_NORMAL_EXPONENT` const; make 32-bit range equal to 64-bit or record it in the ball encoding |
| directed sqrt and div | **EXISTS**, tested (K4); overflow defect applies to div | `src/num.rs:384`; `src/ops/sqrt.rs:21` | K3 overflow fix |
| ops returning the pre-round exponent | **MISSING** | none | `add/sub/mul/div/sqrt_with_info(..) -> (ExactNum, Ternary, pre_round_exp: i64)` |
| `ulp`, `next_up`, `next_down` | **MISSING**. `next_after(toward, p, rm)` exists | `src/ext.rs:1697` | add `ulp(p)`, `next_up(p)`, `next_down(p)` |
| `*_ternary` ops (per-op inexact flag) | **MISSING**; `inexact()` is inherited | `src/ext.rs:1497`; `src/num.rs:337,409,633` | add `*_ternary` returning `Ordering` (or fold into `_with_info`) |
| `mantissa_words` and `from_raw_parts` | **EXIST** as `mantissa_digits()` and public `ExactNum::from_raw_parts`; but `from_raw_parts` uses `from_raw_unchecked`, so the documented exponent-range check is not done | `src/ext.rs:1402`, `:1280-1288`; the validated version is `#[cfg(test)]` `src/num.rs:888-889` | validate `e` range in the public path; add `mantissa_words` alias only if wanted |
| `Consts::get_exact_prec` | **MISSING** (constants take `(p, rm)` and re-round from cache) | `src/ops/consts/mod.rs:213-245` | add it, or document that `pi(p, rm)` already re-rounds and test call-order independence |
| K3 directed overflow | **DEFECT** | K3 | fix, fix tests and generator |
| K-doc rounding-mode docs | **DEFECT** (docs) | `src/defs.rs:253-271` | fix docs |

**Exponent width.** `Exponent = i32` (`src/defs.rs:42`), full `i32` range on 64-bit targets and a quarter of it on 32-bit. `i32` covers *midpoint* exponents but **not** `Mag`: a half ulp at precision `p` needs `exp(mid) − p − 1`, which goes below `i32::MIN` for small mids, and `rad·rad` products double the exponent. **`Mag` uses `i64`**, overflow to `Infinity` (C6).

## Part C. Crate boundary, zf prerequisites, versions, puremp port, Mag

### C0. Order of work
1. **zf 2.0.0 intrinsics-fix layer** on the current branch, green on Pop.
2. **zf 2.1 prerequisites** (C1), red-first, each additive except the K3 overflow fix (a behaviour fix for directed modes only, called out in the changelog).
3. **zenith-bound L0** (contract + real balls), then L1 → L4, each with its own Pop gate.

### C1. zf 2.1 prerequisites (status from Part K)

| # | Prerequisite | Status on `508a400` | Needed by |
|---|---|---|---|
| P1 | `exponent()` and a named leading-bit exponent | EXISTS (`exponent`); helper MISSING | Mag `from_exact_num`, `ulp_mag` |
| P2 | `top_u64_mantissa_and_sticky` | MISSING | Mag `from_exact_num` (round up) |
| P3 | `from_mag_parts` / `pow2` | MISSING (`from_words`, `ldexp` are workarounds) | Mag `to_exact_num`, half-ulp construction |
| P4 | exact add/mul | EXISTS (`*_full_prec`) | exact mid, `is_unique_at`, exact domain checks |
| P5 | exponent limits + `MIN_NORMAL_EXPONENT`, same range on every target | PARTIAL | overflow/underflow contract, `ulp_mag` for mid 0 |
| P6 | directed sqrt and div | EXISTS (tested, not proven) | sqrt/div radius paths |
| P7 | ops returning `pre_round_exp` (and the ternary) | MISSING | `ulp_mag(pre_round_exp, prec)` |
| P8 | `ulp`, `next_up`, `next_down` | MISSING (`next_after` exists) | tests, `refine`, printer |
| P9 | `*_ternary` (per-op inexact) | MISSING (`inexact` is inherited) | zero radius only when exactness is proven |
| P10 | `mantissa_words` + validated `from_raw_parts` | EXISTS as `mantissa_digits`; `from_raw_parts` public but unvalidated | canonical encoding (Part J) |
| P11 | `Consts::get_exact_prec` | MISSING | constant determinism |
| **P12 (new)** | **Directed rounding at overflow** returns ±`max_value` | **DEFECT** (K3) | every directed bound |
| **P13 (new)** | Rounding-mode docs say "directed", MPFR differential covers **every** mode for **every** op each case (including sub/div), and the Inf branch of `assert_float_close` checks MPFR's actual value | test gap (K4) | trust in P6 |

P1, P4, P6 and P10 exist (P10 needs validation). P2, P3, P7, P8, P9, P11 are missing. P5 is partial. P12 and P13 are new because of Task 1. **The zf 2.1 set grows from 5 items to 11** (effect: zf 2.1 roughly doubles, see Part G).

### C2. zf's prototype `Ball` and `ComplexBall`
Unchanged from 10-13: deprecated in zf 2.1 (pointing to zenith-bound), `contains` truncation bug fixed (`ball.rs:236`), removed in zf 3.0. `ziv_round` stays in zf. `GkResult.abs_err_est` is renamed `heuristic_err` in 3.0.

### C3. Versions and release story
- 0.0.0 = scaffold (Part N). 0.1 = L0, 0.2 = L1, 0.3 = L2, 0.4 = L3, 0.5 = L4. 1.0 when L0–L4 are green and the Part J encoding and Part I trace format are frozen.
- Own `CHANGELOG.md`, tags, docs. Records its zf range (`zenith-float = ">=2.1, <3"`).
- Accumath pins both crates exactly and commits `Cargo.lock`.
- **License decided: MIT OR Apache-2.0.** puremp-derived files keep puremp's MIT notice; the crate remains `MIT OR Apache-2.0` because MIT-licensed portions are compatible with both and their notice is preserved in `NOTICE-puremp`.

### C4. The puremp port (github.com/KarpelesLab/puremp, MIT, © 2026 Karpelès Lab Inc.; HEAD `283471d` on 2026-10-14)

| puremp file | Port into zenith-bound | Change |
|---|---|---|
| `src/ball.rs` | L0 `ball/*.rs` (split into small files) | Rebuilt on `ExactNum`; radius becomes the locked `Mag` (C6) instead of a 30-bit Float. Drop `from_f64` |
| `src/interval.rs` | L0 `interval.rs` (for subdivision and the inf–sup conversion, B1) | onto zf directed ops; no f64 |
| `src/ball_solve.rs` | L4 `certify/root_isolate.rs` | add interval Newton / Krawczyk |
| `src/float_mp.rs` (f64 Babai lattice) | **Not ported** | dropped by the no-float rule |
| `tests/*.rs` | not ported as single cases (B3 ban); their *inputs* seed property and adversarial sets | |

Provenance: `NOTICE-puremp` at the repo root holds the full MIT text; each ported file carries an SPDX header naming the puremp commit; CI fails if a header goes missing.

### C5. API shape (B2, B3)

- **Types**
  - `Mag`: locked design (C6).
  - `Ball { mid: ExactNum, rad: Mag, prec: u32 }` (B3). `prec` is "at least `prec` bits"; zf stores `round_up_to_64(prec)` (K5).
  - `ComplexBall` in **two forms** (B3): `Rect { re: Ball, im: Ball }` (Arb `acb`) and `Disk { mid: ExactComplex, rad: Mag }`, with conversions that only ever enlarge (`rect→disk`: `rad = hypot_up(r_re, r_im)`; `disk→rect`: each component gets `rad`).
  - `Interval { lo, hi }` (inf–sup) with `Ball::to_interval` (lo rounded Down, hi rounded Up) and `Interval::to_ball`, for subdivision (B1).
  - `BallPoly`, `BallSeries` (truncated power series with an order), `BallMat`, `ComplexBallMat` (B1).
  - `Tri { True, False, Unknown }`.
- **Precision is explicit** on every op: `fn add(&self, b: &Ball, prec: u32, rm: RoundingMode) -> BoundResult<Ball>`. `rm` only shapes the midpoint; it never affects correctness (B1: "mid rounded to prec p with rm").
- **Results and error kinds** (B2):
  ```text
  enum BoundError {
      Domain { op, detail },          // log of a ball reaching ≤ 0, sqrt of a ball below 0, division by a ball containing 0
      Resource { op, limit },         // precision cap, iteration/evaluation budget, memory
      Overflow { op },                // exponent beyond zf range (K3)
  }
  enum Enclosure<T> { Ball(T), Indeterminate { reason } }   // valid but uninformative
  type BoundResult<T> = Result<Enclosure<T>, BoundError>;
  ```
  "Valid but uninformative" (radius `Infinity`, or a ball wider than the caller's tolerance) is an `Indeterminate`/flagged state, never silently a normal ball.
- **Queries:** `contains(&ExactNum)`, `contains_ball`, `overlaps`, `is_exact` (radius zero and proven), `accuracy_bits` (relative accuracy, rounded down), `is_finite`, `lower_down()`, `upper_up()`.
- **Guaranteed vs heuristic:** guaranteed ops live in `ball::*`, `elementary::*`, `special::*`, `certify::*`. Heuristic helpers (error *estimates*, point solvers) live only under `heuristic::*` and return plain `ExactNum` with a `Heuristic` marker type; they can't be passed where a `Ball` is expected.
- **No implicit uncertainty-dropping conversions:** no `From<Ball> for ExactNum`; extracting the midpoint is the explicitly named `midpoint_unchecked()`. No `From<f64>`.
- **Serialization** preserves the enclosure exactly (Part J encoding: mid bits, `Mag` bits, `prec`); decoding rejects non-canonical input.
- **Determinism (B1):** the radius uses software integer limbs only, so it is bit-identical even if a midpoint is computed in parallel or (later) on a GPU; a GPU/parallel mid must be byte-identical to the sequential mid, or its rounding error is bounded and added to `rad` by the integer path.

### C6. `Mag`, the locked design (briefs 8 + 9)

```text
enum Mag { Finite { exp: i64, mant: u64 }, Infinity }
value(Finite) = mant · 2^(exp − 63),   mant ∈ [2^63, 2^64)    (the one exception: proven zero is Finite{exp: 0, mant: 0}, see below)
```
- **Exponent width.** Brief 8 says `exp: i32` "check the range against zf; widen if needed". Task 1: zf mids use `i32`, but `Mag` needs `exp(mid) − prec − 1` (below `i32::MIN` for small mids at large precision) and products of radii (double the exponent). **Widened to `i64`.** Any `i64` overflow → `Infinity`.
- **Zero.** A radius of zero is allowed only when exactness is proven (B2). Represented as `Mag::Zero` would add a variant to a locked enum, so zero is the canonical `Finite { exp: 0, mant: 0 }`, the only finite value with `mant < 2^63`. Every constructor checks this invariant.
- **`add_up(a, b)`**: align to the larger exponent; shifted-out bits OR into a sticky bit; add with carry (carry renormalises by one, its lost bit also goes to sticky); if sticky, add 1 at the last place and renormalise. If the shift is ≥ 64, return `larger + 1 ulp` (the smaller is nonzero, so this is the rounded-up result).
- **`mul_up(a, b)`**: the `u128` product `P = ma·mb` lies in `[2^126, 2^128)` and value = `P·2^(ea+eb−126)`. If `P ≥ 2^127`: `mant = P >> 64`, `exp = ea + eb + 1`, sticky = low 64 bits ≠ 0. If `P ∈ [2^126, 2^127)`: `mant = P >> 63`, `exp = ea + eb`, sticky = low 63 bits ≠ 0 (this is the "verify the −63 normalisation" case). If sticky, +1 at the last place and renormalise (a carry to `2^64` gives `mant = 2^63`, `exp + 1`). Tests are sets over both product ranges, including `2^63·2^63` and `(2^64−1)²`, checked against exact `u128`/rational arithmetic.
- **Overflow** of the exponent → `Infinity`, never saturation. NaN or Inf inputs from zf map to `Infinity`. A non-finite scale in `mul` gives `Infinity`.
- **`from_exact_num(x)`** = `|x|` rounded **up** to 64 bits via `top_u64_mantissa_and_sticky` (P2): `mant = top + sticky` with carry renormalisation, `exp = leading_bit_exponent(x) + 1` adjusted to the 2^(exp−63) convention. **`to_exact_num(m)`** = exact, via `from_mag_parts` (P3).
- **Half ulp.** With `mid_exp` = zf's `exponent()` (value = 0.m × 2^e, so mid ∈ [2^(mid_exp−1), 2^mid_exp)), ulp at `prec` is `2^(mid_exp − prec)` and **half ulp = `2^(mid_exp − prec − 1)`**. A finite `Mag` lies in `[2^exp, 2^(exp+1))`, so **`smaller_than_half_ulp` is `exp < mid_exp − prec − 1`**. **Boundary set** at `mid_exp = 10, prec = 4` (half ulp `2^5`): `exp = 4` with every mantissa class `{2^63, 2^63+1, 2^64−1}` must be smaller; `exp = 5` must not; sweep `exp` 2..7 against an exact-rational model. It runs in zenith-bound's integer model, because zf can't hold a 4-bit midpoint (K5).
- **`ulp_mag(pre_round_exp, prec)`** uses the leading-bit exponent of the **exact pre-round result** (P7), not of the rounded mid (rounding can carry into a new binade). **For a mid of 0** (exact cancellation), the rounding error is 0 if the op was exact; otherwise use the inputs' exponents or the exact difference, never the rounded mid. `min_normal_exponent` is defined from zf's `EXPONENT_MIN` (P5); below it, ulp is the subnormal ulp `2^(EXPONENT_MIN − prec)`.
- **The midpoint is computed exactly** (exact add/mul of finite binaries, P4) and then rounded once to `prec` with `rm`, or every intermediate rounding is bounded and added to the radius. **Never** `prec + 4` with `RoundingMode::None` (the prototype's pattern).
- **`is_unique_at(prec, rm)`**: `lo = m − r` rounded **Down** and `hi = m + r` rounded **Up** (exact via P4 where the sizes allow, otherwise directed at a working precision above `prec`), then both rounded to `prec` under `rm`; unique iff `lo == hi == mid`. Because the whole interval rounds to one value, this covers every mode and every tie. This is the B1 contract: the mid is unique and correctly rounded whenever the ball is unique.
- **`sqrt`**: the mid uses `rm`. The radius path: `lo = m − r` rounded Down; `s = sqrt(lo)` rounded Down; `d = 2·s` (exact) rounded Down; `q = 1/d` rounded Up; `rad = mul_up(r, q) + ulp` with `add_up`. The domain check compares **exactly** (`mid` against `rad.to_exact_num()`, exact subtraction): if `m − r > 0`, use the formula. If `m − r ≤ 0 ≤ m + r` (the ball touches 0), return a ball enclosing `[0, √(m+r)]`, with `√(m+r)` rounded Up, mid = half of it, and rad rounded up to cover both ends. If `m + r < 0`, return `BoundError::Domain`; a caller that asked for a strict domain gets `Indeterminate`. **Never** a finite ball that pretends.
## Part L. The correctness contract (B2 step 1, lands in L0 before any code)

`docs/CONTRACT.md` in zenith-bound, written and reviewed before L0 code. Every later test cites a clause.

1. **Value model.** A `Ball` denotes the closed set `[mid − rad, mid + rad]` of reals. `rad = Infinity` denotes all reals. A NaN midpoint is never stored; operations that would produce one return `Indeterminate` or an error.
2. **Invariant (stated):** for every guaranteed op `f` and inputs `x_i ∈ X_i`, `f(x_1, …) ∈ f_ball(X_1, …)`, or the op returns `BoundError`/`Indeterminate`. Nothing else.
3. **Precision model.** `prec: u32` bits, at least 1; zf stores a multiple of 64 bits (K5). `prec` shapes only the midpoint. The radius is always a `Mag` (64-bit mantissa, `i64` exponent).
4. **Exponent range.** Midpoints: zf's `EXPONENT_MIN..=EXPONENT_MAX` on 64-bit targets; on 32-bit targets zenith-bound refuses (`Overflow`) anything outside the 32-bit range so that results never differ by target (until P5 makes the ranges equal). Radii: `i64`, then `Infinity`.
5. **Rounding.** The midpoint is rounded once to `prec` under the caller's `rm` from an exact or fully accounted value. The radius is rounded **up** at every step. No rounding error is ever dropped.
6. **Radius rules.** Nonnegative. Zero only when exactness is proven (exact inputs and a ternary of `Equal`). Add: `(m+n) ± (r + s + ε)`. Mul: `mn ± (|m|s + |n|r + rs + ε)`, every term and sum evaluated with `mul_up`/`add_up`. `ε` = `ulp_mag(pre_round_exp, prec)` if the mid rounding was inexact, else 0 (a half ulp under nearest modes, a full ulp under directed ones).
7. **Special values.** Division by a ball containing 0 never returns an ordinary finite ball (`Domain`, or `Indeterminate` when the caller opted into "whole line" semantics). Overflow: `Overflow` error or a ball with `rad = Infinity`, by caller choice; never saturation. Underflow: the radius absorbs the lost value, never zero.
8. **Exactness.** `from_integer`, `from_rational` (exact when the denominator is a power of 2, otherwise a correctly enclosed ball), `is_exact` true only with proof.
9. **Correctness vs tightness.** Correctness failures are P0 bugs. Tightness targets are measured (overestimation ratio vs the oracle's radius) and tracked; missing them is a performance bug.
10. **Determinism.** Same `(op, inputs, prec, rm, version)` gives the same bytes on every supported target, build profile and thread count.

## Part D. Layers (each: tests red → green on Pop → next)

B2's order maps onto the layers like this: (1) contract + (2) real balls = **L0**; (3) elementary functions and constants = **L1**; (4) complex, polynomials, power series, matrices, then selected specials = **L2**; quadrature = **L3**; (5) validation, adaptive precision and validated workflows = **L4**. The 10-13 order (L2 specials, L4 linear algebra) is reshuffled: dense ball linear algebra moves into L2 with the other containers; L4 keeps verified *solving* and workflows.

### L0 Contract and real balls (zenith-bound 0.1)

- **Gate:** zf 2.0.0 fix green on Pop; zf 2.1 P1–P13 merged and green on Pop. Until P12 lands, L0 can be written against the K3 workaround, but not released.
- **Order inside L0:** `CONTRACT.md` → `mag/*` → `ball/*` construction → add/sub/mul → div → sqrt → compare/contains/overlap → precision reduction → encoding.
- **File plan for the local qwen3-coder:30b (until 2026-10-30), one responsibility per file, soft target ≤ ~30 KB:**

  | File | Content |
  |---|---|
  | `src/lib.rs` | `#![forbid(unsafe_code)]`, lint denies, module list |
  | `src/error.rs` | `BoundError`, `Enclosure`, `BoundResult` |
  | `src/mag/mod.rs` | `Mag` enum, invariant check, constants |
  | `src/mag/add.rs` | `add_up` |
  | `src/mag/mul.rs` | `mul_up` |
  | `src/mag/convert.rs` | `from_exact_num` (up), `to_exact_num` |
  | `src/mag/ulp.rs` | `ulp_mag`, `half_ulp`, `smaller_than_half_ulp` |
  | `src/ball/mod.rs` | `Ball` struct, constructors from integers and rationals |
  | `src/ball/arith_add.rs` | add, sub, neg |
  | `src/ball/arith_mul.rs` | mul, `mul_2exp` |
  | `src/ball/arith_div.rs` | div, inv (domain rule) |
  | `src/ball/sqrt.rs` | sqrt (C6) |
  | `src/ball/compare.rs` | `Tri` comparisons, `contains`, `overlaps`, `is_exact`, `accuracy_bits` |
  | `src/ball/precision.rs` | `set_prec` (reduction enlarges radius), `refine(new_prec)`, `is_unique_at` |
  | `src/interval.rs` | inf–sup `Interval` and conversions |
  | `src/encode.rs` | canonical encoding (Part J) |
  | `tests/<topic>_props.rs` | one property/oracle/adversarial set per file |

  Modules later in the crate (B3): `mag`, `ball`, `complex`, `arith`, `elementary`, `special`, `certify`. `arith` holds the shared radius formulas used by `ball` and `complex`.
- **Real ball ops (B2 step 2):** exact construction from integers and rationals; add/sub/mul/div per the contract; compare (`Tri`), `contains`, `overlaps`; precision reduction that rounds the mid and adds the rounding error to the radius; division by a ball containing 0 returns `Domain`/`Indeterminate`, never a finite ball.
- **Tests (red first, sets only):** containment properties (random points inside input balls, exact-rational truth), rounding sets for every `rm` (the mid equals the correctly rounded exact result when the inputs are exact), `is_unique_at` sets including ties, Mag boundary sets (C6), adversarial sets (cancellation `(1+2^-k)−1` over k, near-zero denominators, extreme exponents near `EXPONENT_MIN/MAX`, the K3 overflow edge), overestimation measurements against python-flint's `arb` radius, and reproducibility (two runs, two targets, byte-equal).

### L1 Elementary functions and constants (B2 step 3; B3 radius rules)

- Functions: sqrt (from L0), exp, log, sin, cos, tan, atan, pow; constants pi, e, ln2, ln10.
- **First-order radius bounds (B1, B3)**, every expression evaluated rounding **up** in `Mag`, plus the mid's own error bound:
  - exp: `rad' = |exp(m)|·(exp(r) − 1) + ulp`.
  - sin/cos: `rad' = min(r, 2) + ulp`. Correct first; tighten later (`|cos m|·r + r²/2`-style bounds) only behind overestimation measurements.
  - ln: `rad' = r/(m − r) + ulp` for `m > r` (exact check); otherwise `Domain`/`Indeterminate`.
  - sqrt: `rad' = r/(2√(m − r)) + ulp` (C6 path).
  - Here "ulp" is the proven error bound of the midpoint evaluation. zf's kernels are correctly rounded by Ziv loops but don't export a bound, so the mid is evaluated with directed `Down`/`Up` calls and the gap is added (no API change), or through certified kernels with explicit truncation, rounding and argument-reduction terms where that is too slow.
- **Domains, poles, branch cuts, argument reduction** documented per function in `CONTRACT.md`; reduction uses π/ln2 enclosed at `prec + exponent + guard` bits and adds `|n|·rad(π)`.
- **Compositions (B1):** higher-order or derivative-based propagation (mean-value form `f(m) ± sup|f'|·r` over the ball, or a degree-2 Taylor form) for composed expressions; when the dependency problem widens a result, convert to `Interval` and **subdivide**, then re-hull. Lands in `arith`/`elementary`; used heavily in L4.
- **Tests:** sets (oracle containment vs python-flint `arb` and mpmath at 2×prec, plus exact rational checks where values are algebraic), adversarial (`sin(10^100)`, near multiples of π, `log(1+2^-400)`, `exp(±2^30)` overflow → `Overflow`/`Infinity`), overestimation ratios recorded per function.

### L2 Complex, polynomials, power series, matrices, then selected specials (B1, B2 step 4)

**L2a ComplexBall (first).** `Rect` and `Disk` forms with enlarging conversions (C5). Rect `mul` uses the contract's mul rule per component; Disk `mul` uses `|a||b|`-style radius `|m_a|r_b + |m_b|r_a + r_a r_b + ε`. Complex exp/log/sqrt with documented branch cuts (principal branch; a ball straddling a cut returns an enclosure of the union, documented). `refine(new_prec)` re-evaluates from stored exact inputs where available.

**L2b Ball polynomials and power series.** `BallPoly` (coefficients are balls; evaluation by Horner with rounding errors accumulated in `Mag`; root bounds by Cauchy/Fujiwara rounded up). `BallSeries` (truncated power series to order `n` with ball coefficients, mul/compose/inverse/exp/log of series; the truncation error is not dropped: a series result either states its order or carries a tail bound).

**L2c Ball matrices with dense linear-algebra enclosures.** `BallMat`/`ComplexBallMat`: products by the midpoint–radius formula `|A|·rB + rA·(|B| + rB)` plus a rounding bound, all in `Mag`, sequential index order (Part J). Dense enclosures: determinant, inverse, and the residual `‖I − RA‖` used by L4's verified solve.

**L2d Selected specials (after a–c).** Priority by Accumath use: gamma/lgamma, erf/erfc, Bessel J0/J1, zeta. **B3 rule:** erf, J0, J1 use **derivative bounds** (`erf' = 2/√π·e^{−x²}`, `J0' = −J1`, `J1' = J0 − J1/x`, each bounded over the ball). Higher specials (pFq, elliptic, polylog, expint, Airy, the rest) use **either** a series with a rigorous remainder **or** zf's correctly rounded value at the midpoint plus a separately proven bound (derivative or remainder). Anything without a proof returns `Indeterminate`. The 10-13 method table for specials still applies:

| Function | Method | Error term needed |
|---|---|---|
| gamma, lgamma | Stirling with a Bernoulli remainder bound, reflection for Re<0.5, rising factorial | Stirling remainder `|B_2N|/(2N(2N−1)|z|^(2N−1))` with sector factor; pole → inf rad |
| erf/erfc | Taylor (small) / continued fraction or asymptotic (large) | Taylor tail; asymptotic remainder bound (alternating for real) |
| zeta | Euler–Maclaurin (Arb `acb_zeta`) | EM remainder bound |
| Bessel | Power series / Hankel asymptotic | series tail; Hankel remainder (Olver bounds) |
| pFq | Series with tail bound (Arb `hypgeom` bound); 2F1 transformations | ratio-test tail bound; asymptotic bounds for 1F1/U |
| Elliptic K/E/Π, `_phi` forms | Carlson R_F/R_D/R_J with AGM/duplication | duplication-step remainder (Carlson's bound, as in Arb `acb_elliptic_rf`) |
| polylog | Series / Bernoulli expansion near 1 | series tail |
| expint, Airy | Series / asymptotic / via 1F1/U | asymptotic remainder |
| Meijer/Fox/Heun, Lambert W | Lambert W: certified Newton (interval Newton). Others: **refuse (inf radius) in v1** | — |

Tests: sets as for L1, plus poles (gamma at −n), branch cuts approached from both sides, and huge arguments. Tightness is measured as an overestimation ratio against python-flint, not asserted per single case.

### L3 Rigorous quadrature
- `integrate_ball(f, a, b, prec, opts) -> CBall`, following Arb's `acb_calc_integrate` (Johansson 2018, after Petras).
- `f` is supplied as a **ball function on complex balls** that also reports *analytic or not* on the input box, as in Arb's `analytic` flag.
- On each subinterval, use Gauss–Legendre with a rigorous bound. If `f` is analytic and `|f| ≤ M` on the Bernstein ellipse `E_ρ`, the error is at most `M·(64/15)·ρ^(−2n)/(ρ²−1)` (Petras bound). M comes from evaluating `f` on a ball covering the ellipse. Split the interval adaptively whenever M is infinite.
- **Non-analytic integrands** (abs, floor, piecewise, sqrt at an endpoint): use a degree-0 bound, the interval width times the hull of `f` on the box. That bound is rigorous but wide, so use it only on tiny pieces, inside an evaluation budget. When the budget runs out, return the partial sum with the remaining-piece bound, which may be wide. **Never** return a narrow uncertified result.
- Keep `integrate_adaptive_gk` unchanged. Document `abs_err_est` as heuristic.
- Tests:
  - Sets: `quad.jsonl` inputs plus python-flint `acb.integral` truths. Inclusion always.
  - Tightness `rad ≤ 2^-(prec−10)·|I|` for entire integrands.
  - Adversarial: `∫ exp(-x²)` with a huge interval, `∫ sin(1/x)` (must come back wide), a near-singular `1/(x²+ε²)`, the oscillatory `∫ sin(1000x)`, and `∫|x|` across 0.

### L4 Validation, adaptive precision and validated workflows (B2 step 5)
- **Adaptive precision:** `evaluate_to(expr, target_bits, budget)` doubles precision Ziv-style until `accuracy_bits ≥ target` or the budget (bits/evaluations, never wall-clock) runs out, then returns the best ball with `Resource` metadata.
- **Validated workflows (B2 differentiator):**
  - **Root isolation:** port puremp `bisect_root` (certified sign change) plus interval Newton / Krawczyk for existence and uniqueness; output is a list of isolating balls each tagged `Unique`/`Exists`/`Unknown`.
  - **Verified linear solve:** approximate inverse `R` in point arithmetic, then Krawczyk/Rump with L2c's `‖I − RA‖ < 1` check; enclosure `x̃ ± ‖R(b − Ax̃)‖/(1 − ‖I − RA‖)`. Reports failure, never a narrow wrong answer.
  - **Bounded residuals:** every point solution Accumath shows carries a ball residual `‖f(x̃)‖` and, where the theory allows, an error bound on `x̃`.
  - **Eigen-enclosures** for symmetric matrices (Weyl + residual); general matrices refuse.
  - **ODE:** Lohner-style validated integrator (Taylor series by ball AD, Picard a-priori enclosure, QR wrapping). v1: polynomial/elementary right-hand sides, small systems.
- **Accumath integration (B2 differentiator):** symbolic expression → certified enclosure, via Part F.
- **Tests:** Hilbert matrices n = 5..12 as a set (contain the exact rational solution or report failure), ODE sets (`y′ = −y`, harmonic oscillator, short-horizon Lorenz), root-isolation sets with clustered and multiple roots.

## Part E. Test strategy (all layers; B2, B3)

- **Red first.** Every item starts as failing tests. **No single exact-input tests** (B3 ban): each test file is a property set, an oracle set, an adversarial set or an overestimation measurement.
- **Containment invariants:** random points inside the input balls (exact rationals) map inside the output ball; inclusion monotonicity (`A ⊆ A′ ⇒ f(A) ⊆ f(A′)`); `x − x ∋ 0`; `div(mul(a, b), b) ∋ a`.
- **Rounding sets:** for exact inputs, the mid equals the exact result correctly rounded under each of the 7 modes (truth from exact integer arithmetic); `rm` never changes containment.
- **Adversarial sets:** cancellation, near-zero denominators, extreme exponents (both ends, subnormal mids, the K3 overflow edge), branch cuts, poles, wide input balls.
- **Transcendentals:** containment against python-flint at 2× precision and mpmath, plus independent checks (below).
- **Refinement:** `refine(p2)` with `p2 > p1` gives a ball contained in the `p1` result (when inputs are exact) and `accuracy_bits` grows.
- **Reproducibility and cross-platform:** byte goldens (Part J) on every CI target, debug and release, 1 vs N threads.
- **Overestimation measurements:** `rad / rad_arb` distribution per op and precision, tracked over time. Tightness regressions are performance bugs; containment failures are P0.
- **Oracles (B2):** FLINT/Arb through **python-flint**, dev-only, next to mpmath (and MPFR for scalar midpoints). Never shipped, never a dependency, never in `Cargo.toml`. Golden generation runs in `scripts/` with a pinned python environment. **Agreeing with Arb isn't proof**, so every oracle family is backed by:
  - exact rational checks wherever the truth is rational or algebraic (`sqrt` of squares, `exp(0)`, polynomial values, Hilbert solutions);
  - independent algorithms (series vs asymptotic, AGM vs Taylor for log, two argument-reduction paths), compared by containment;
  - the mutation check: shrinking any radius by 1 ulp in a test build must make the suite fail.
- **No-hardware tests:** Part B.4.

## Part F. Accumath: proven digits only

Differentiator (B2): Accumath goes from a symbolic expression to a certified enclosure. Switch these outputs to ball-backed evaluation, in this order:
1. Numeric evaluation `N[expr, d]` and the precision display
2. Special-function evaluation
3. Root-finding (`roots.rs` → interval Newton)
4. LA solve and determinant
5. Quadrature (`NIntegrate`-style)
6. ODE (last; opt-in)

**Printer rule:** convert the ball to decimal by rounding outward. Print the longest digit string that is the same for every point in `[mid−rad, mid+rad]`, so all printed digits are proven. Then either:
- (a) show `d.ddd…ddd ± r` with `r` to 2 significant digits, rounded up (Arb's `arb_printd` style), or
- (b) truncate, adding `…` when the asked-for precision isn't reached.

If the ball holds 0 and has a large radius, print `0 ± r` or `[±r]`. Never pad unproven digits. The default is (b) for `N[]` and (a) in "verbose" mode. If the user asks for d digits and fewer are proven, retry at a higher precision (Ziv-style doubling, capped). If the cap is hit, show what is proven and a warning.

## Part G. Effort (relative sizes; one run ≈ one agent session)

Jeff discounts AI timeline estimates, so this table gives **relative runs only**. Weeks/months are dropped. Before 2026-10-30, L0 runs are qwen3-coder:30b hunk steps with bot review; the extra run overhead from the old 7B context limit is removed.

| Item | 10-13 runs | 10-14 runs | Why it changed |
|---|---:|---:|---|
| Pre-gate: zf 2.0.0 intrinsics fix + Pop gate | 4–8 | 4–8 | — |
| zf 2.1 prerequisites (C1 P1–P13) + deprecations | 6–10 | 12–20 | 5 → 11 items; K3 overflow fix with corrected tests/generator; P13 test coverage |
| Scaffold, NOTICE, CI on 1.99 | 2–4 | 2–3 | scaffold prepared (Part N) |
| B: no-float guards + audit | 4–8 | 4–8 | — |
| L0: contract + Mag + real balls | 12–20 | 16–26 | contract doc; locked Mag (u64/i64, add_up/mul_up/ulp/unique); error kinds; one-responsibility file split |
| L1: elementary + constants | 20–35 | 20–35 | first-order rules (B3) are simpler, but composition/subdivision adds about as much |
| L2a–c: ComplexBall (rect+disk), polynomials, series, matrices | (partly in L0/L4) | 18–32 | new scope (B1) |
| L2d: selected specials | 35–60 | 30–55 | derivative bounds for erf/J0/J1; unprovable cases return `Indeterminate` instead of research |
| L3: Petras quadrature | 10–20 | 10–20 | — |
| L4: validation, adaptive precision, root isolation, verified solve, residuals | 10–17 | 14–24 | adaptive precision and bounded residuals added; dense LA moved to L2c |
| L4: Lohner ODE | 10–20 | 10–20 | — |
| Accumath printer + integration | 6–12 | 8–14 | expression → enclosure workflow (B2) |
| I: step traces + checker | 12–24 | 12–24 | — |
| J: determinism, encoding, CI matrix | 16–30 | 13–22 | partly folded into L0 tests; 32-bit exponent issue tracked under P5 |
| **Total** | **~147–268** | **~175–335** | about +19–25% |

## Part H. Risks

- **The zf pre-gate slips.** zenith-bound L0 can't start until the zf intrinsics fix and the C1 APIs are green on Pop. Mitigation: keep C1 small and additive, and land it right after the fix.
- **Two-repo drift.** zf changes break zenith-bound, or move its bytes. Mitigation: zenith-bound CI also runs against zf `main` nightly, Accumath pins exact versions, and zf runs zenith-bound's byte goldens as an allowed-to-fail downstream job.
- **Directed-rounding correctness in zf becomes load-bearing.** Option A in C1 trusts that zf `Up`/`Down` really bracket the result. Mitigation: zf's MPFR oracle must cover `Up`/`Down` for every kernel zenith-bound uses, at 256 and 512 bits. The current gap is `ToOdd`/`None` only (`tests/mpfr/common.rs:279-287`). Make this a C1 gate.
- **puremp provenance.** Missing a header or the SHA is a license defect. Mitigation: the NOTICE CI check (C4). Also, puremp's design differs from zf's (a Float radius, `Ordering`-ternary ops), so the port is an adaptation, not a copy, and the savings may be smaller than 20–30%.

- **Trace volume.** Big solutions could log 10^5–10^6 steps. Mitigation: hash-chain the steps, keep the full record only for user-visible steps, and store sub-steps on demand. Replaying from the root always rebuilds them, because results are deterministic (Part J).
- **Determinism versus speed.** Ordered reductions and no parallel float sums cost speed in LA and quadrature. Accept it on the ball path. Point paths may stay parallel.

- **Existing kernels can't report their own error.** If correct rounding is achieved by Ziv retries without an internal bound, certified versions must be rewritten rather than wrapped. This is the main L1 cost driver.
- **Radii that grow too wide** (the dependency problem) for Accumath expressions like `x−x`. Mitigation: re-evaluate at higher precision, and later use a symbolic simplify pass.
- **Performance.** Balls cost 1.3–3× point arithmetic. Fast paths are banned (no `f64`), so `Mag` is a lean integer type (`u64` + `i64`).
- **Overflow.** `Mag` uses `i64` exponents; overflow becomes `Infinity` (never saturation), which is wide but safe.
- **Branch cuts and complex balls straddling them.** The convention must match Arb, and the result must be documented as covering the union.
- **The no-float CI check can produce false positives** from `conv.rs` re-exports. Keep the ball crate's imports narrow.
- **Oracle disagreements** between mpmath and Arb near cuts. Neither wins by default: agreeing with Arb isn't proof, so disagreements go to exact rational checks or an independent algorithm (Part E).
- **Silent semantic changes to `abs_err_est`.** Don't make any in zf 2.x. The rename waits for 3.0 (C2).
- **zf directed rounding at overflow (K3)** is wrong today and hidden by the tests. Mitigation: P12/P13 are zf 2.1 gates; L0 treats any near-`EXPONENT_MAX` directed result as overflowed until then.
- **Directed rounding is tested, not proven.** Mitigation: P13 runs every mode per case for every op; zenith-bound's own sets check containment against exact rationals independent of zf's rounding.
- **32-bit exponent range differs (K5).** Mitigation: P5, and until then refuse out-of-range values on 32-bit targets so bytes never differ.
- **Local-model quality until 2026-10-30 (qwen3-coder:30b).** Mitigation: hunk-sized directives, bot review of every diff, red→green gate on Pop; very large refactors wait for cloud runs.
- **The B3 single-case test ban** makes some regressions harder to pin. Mitigation: proptest regression files store failing seeds, and each adversarial set is a named, enumerated generator.

## Part I. Accumath step-by-step solutions and trace logs

**Goal.** Every step of an Accumath step-by-step solution carries its ball (midpoint plus proven radius). Any single step can then be audited without trusting the rest of the run.

**Trace record (exact, serialisable).** One record per step, written as JSON Lines with a canonical CBOR twin:
```json
{"v":1, "step":17, "parent":[12,15], "op":"zb.ball.mul", "op_ver":"2.2.0",
 "prec":256, "rm":"ToEven",
 "in":[{"mid":{"s":1,"m":"0x8000…","e":-3,"p":256},"rad":{"m":"0x8000_0000_0000_0000","e":-260}}, …],
 "params":{…},
 "out":{"mid":{…},"rad":{…}},
 "kind":"exact|arith|elementary|special|quad|la|ode",
 "digest":"blake3:…"}
```
- **`mid`** is the sign, the mantissa as hex (no trailing zero limbs), the binary exponent `e` (value = `m·2^e`), and the precision `p`. Special values are encoded as `{"nan":true}`, `{"inf":±1}` or `{"zero":±1}`.
- **`rad`** is the `Mag` mantissa as hex (64 bits, top bit set, or 0 for a proven-exact radius) and its `i64` exponent, or `{"inf":1}`.
- **`digest`** is BLAKE3 over the Part J canonical encoding of this record without the digest, plus the parent digests. That makes the trace a Merkle chain, so editing any step breaks every digest after it. BLAKE3 is pure Rust and integer-only.
- **No decimal strings in records.** Decimal goes only in an optional `"display"` field, which the checker ignores.

**Threading through Accumath.**
- Each step in the evaluator is a node `Step { id, op, inputs: Vec<StepRef>, prec, out: RBall|CBall|BallMat }`.
- The evaluator passes a `TraceSink`, owned by the session, into every zenith-bound call. zenith-bound emits one record per *public* op. Internal sub-ops (for example the Taylor terms inside `exp`) are not emitted, because the op's proven error bound covers them.
- Symbolic rewrites (simplify, substitution) are recorded as `kind:"exact"` steps with the rule id, and they carry no ball. Only numeric steps carry balls.
- A precision bump (a Ziv-style retry) is its own step, linked to the step it retried. The display shows the final retry, and the log keeps every attempt.
- Each user-facing solution line maps to one or more step ids, so the UI can say "step 4 proves 1.41421356 ± 3e-9, source records 31–33".

**Independent checker (`zenith-bound-check`, a binary in the zenith-bound repo).**
- A separate small binary crate. It depends only on zenith-bound's `RBall` core (L0–L1, plus the specific L2/L3/L4 op being checked), not on Accumath.
- Given a record, it decodes the inputs, recomputes `op(inputs, prec)` **independently**, and confirms `out ⊇ recomputed`. The check is containment, not equality, so a checker built at a higher precision or with a different algorithm still works.
- Two modes:
  - **replay**: same op and precision. It expects byte equality (Part J) and also checks containment.
  - **audit**: precision `2·prec`, by default through a different kernel path where one exists, such as MPFR or Arb in test builds, or a series against an asymptotic expansion.
- It also checks the digest chain and that every parent's `out` byte-equals the child's `in`.
- Exit codes are 0 = verified, 1 = containment failed, 2 = cannot check (op not supported or refused). Output is a short JSON report per step.
- **Tests:** mutation tests flip one mantissa bit in `out.rad` (shrink it) or in an input, and the checker must reject every one. A round-trip test runs encode → decode → encode and requires identical bytes. Fuzz the decoder with `cargo fuzz`, reusing the `zenith-float-num/fuzz` setup.

**Cost.**
- Runtime with the sink off: 0, since the sink is a generic no-op that compiles out.
- Runtime with the sink on: about 5–20% for coarse steps. Sub-step tracing can add 2–3×, and is opt-in.
- Storage: about 150–400 bytes per record at 256 bits.
- Check time: about the cost of one step at `2·prec`.
- Build effort: 12–24 runs, 3–5 weeks (see Part G). Most of it falls in the L0 and Accumath integration windows.

## Part J. Bit-identical ball results across machines and builds

**Goal.** For a given (op, inputs, prec, rm, crate version), the output midpoint *and* radius are byte-identical on every supported platform and in debug and release builds. Golden hash files enforce this.

**Sources of nondeterminism to ban (each has a lint or test):**

| Source | Ban / fix | Guard |
|---|---|---|
| Hash iteration order (`HashMap`/`HashSet`, randomized `RandomState`) | Use `BTreeMap` or `Vec` on the ball path. Any hash map must use a fixed hasher and must never be iterated in an order that affects results | clippy `disallowed-types = ["std::collections::HashMap", "std::collections::HashSet"]` in zenith-bound; CI `rg` |
| Parallel reduction order (rayon, threads) | Sums and dot products are sequential, in index order. Parallelism only over independent outputs, with a fixed-order combine (a pairwise tree keyed by index, not by thread) | ban `rayon` in the ball crate's deps; test: 1 thread vs N threads byte-equal |
| `usize` width (32- vs 64-bit), and zf's 32-bit exponent range (`EXPONENT_MAX/4`, K5) | Mantissa limbs, exponents and term counts use fixed-width types (`u64` limbs, `i64` exponents). Values outside the 32-bit zf range are refused on every target until P5. `usize` is only for indexing. Series term counts are computed in `u64` and capped identically | `clippy::cast_possible_truncation` deny; i686 and wasm32 targets in the CI matrix |
| Limb-size differences (`WORD_BIT_SIZE` in `defs.rs`) | The canonical encoding is limb-independent: a mantissa normalised to bytes with trailing zeros stripped. Algorithms that choose guard bits or term counts from `WORD_BIT_SIZE` must use a fixed 64-bit constant on the ball path | 32-bit golden run |
| Debug-only paths (`cfg(debug_assertions)`, overflow-check behaviour) | No `cfg(debug_assertions)` branch may change a value. Use explicit `wrapping_*` or `checked_*`. Debug asserts are read-only | debug vs release golden run |
| Platform intrinsics and target features (`ieee_soft/simd.rs` `core::arch`, `#[target_feature]`, `is_x86_feature_detected`) | Banned in the ball path (Part B). Mantissa kernels used by balls must take the portable branch, or prove that their SIMD branch is bit-identical with a forced-portable comparison test | Part B scan; `ZF_FORCE_PORTABLE=1` vs default byte compare |
| Algorithm switch thresholds (Karatsuba, Toom, FFT in `mantissa/`) | Products are exact, so any switch is fine as long as the result is exact. Any *inexact* path (FFT with rounding, truncated products) must make its choice from `prec` alone, never from CPU or timing | goldens around thresholds |
| Lazily cached constants (`Consts` π/e/ln2 cached at whatever precision was asked first) | The cache key is `(constant, prec_rounded_up_to_64)`. A value computed at higher precision must be **re-rounded** to the requested precision before use, and its radius re-derived, so the result never depends on call history | test: call order A→B vs B→A byte-equal; fresh vs warm `Consts` byte-equal |
| Time and evaluation budgets (L3 quadrature, Ziv retries) | Budgets are counts (evaluations or bits), never wall-clock time | review rule + rg for `Instant`/`SystemTime` in zenith-bound |
| Allocator, address-based ordering, `ptr` comparisons | Banned | rg |
| Compiler or `opt-level` differences | Integer-only code is well-defined. The guard is the debug vs release golden run | matrix |

**Canonical encoding (`zf-ball-enc v1`).**
- Byte layout, big-endian:
  - `tag u8` (0 finite, 1 zero, 2 inf, 3 nan)
  - `sign u8`
  - `prec u32`
  - `exp i64`
  - `len u32` + mantissa bytes, MSB first, trailing zero bytes stripped, top bit set
  - then `Mag`: tag `u8` (0 finite, 1 infinity), `exp i64`, `mant u64` (top bit set, or `exp = 0, mant = 0` for a proven-zero radius)
- Complex values are re then im. Matrices are `rows u32, cols u32`, row-major.
- One value has exactly one encoding, and the decoder rejects anything non-canonical.
- The same bytes feed the Part I digest and the golden hashes.
- Version byte up front. Changing the format requires a new major `v`.

**Golden hash files.**
- `zenith-bound/tests/goldens/ball_bytes/<layer>.jsonl` holds one row per case: `{case_id, op, inputs_enc_hex, prec, rm, out_blake3}`. Each layer also gets a file-level BLAKE3 of all outputs.
- A golden only changes in a commit that bumps `op_ver` and says why. CI rejects a hash change without a version bump.
- These are **our own** outputs. They test determinism, not correctness. Correctness is still the oracle-inclusion test (Part E).

**CI matrix.** This lives in the **zenith-bound repo**, using the same pinned toolchain as zf (Rust 1.99 via `rust-toolchain.toml`) and the Part B no-hardware guards. Every cell runs the byte goldens in both **debug and release**:

| Target | Runner | Status |
|---|---|---|
| x86_64-unknown-linux-gnu | GitHub Linux, plus Pop locally | required |
| aarch64-unknown-linux-gnu | GitHub arm64 Linux runner, or qemu-user | required |
| aarch64-apple-darwin | GitHub macOS runner | required if minutes allow, otherwise nightly |
| x86_64-pc-windows-msvc | GitHub Windows | nightly (feasible; integer-only code) |
| i686-unknown-linux-gnu | cross / multilib on Linux | required (32-bit `usize`) |
| wasm32-wasip1 | wasmtime | required (32-bit, no SIMD) |
| thumbv6m-none-eabi (no FPU) | qemu, existing `nostd-qemu` | required (Part B) |

**Cross-check.**
- Every matrix job uploads its `ball_bytes.out` (the full encoded outputs, not only hashes) as an artifact.
- A final `determinism-diff` job downloads them all and runs `cmp`. On a mismatch it prints the first case id whose bytes differ, decoded on both sides.
- It also runs a sample of each platform's traces (Part I) through the checker on x86_64, which proves that cross-platform traces verify.
- A nightly job runs 100k random cases from a fixed seed (`common/test_rng.rs`), so the case list is the same everywhere, and compares their bytes across all cells.

**Why the headless AI-tool engine needs I and J.** The engine calls Accumath with no human watching. It needs to:
- cite each step with a proof that a third party can check (Part I);
- get exactly the same answer when the same request runs again on another worker, architecture or build, so caching, deduplication, regression detection and signed results all work (Part J).

Without J, a trace made on one machine might not byte-replay on another. Without I, a byte-identical answer is still unaudited.

**Effort.** 16–30 runs, 3–6 weeks: about 4–8 runs for the bans and lints, 4–6 for the encoding and hashes, 6–12 for the CI matrix and cross-diff, and 2–4 to fix whatever the matrix finds, most likely in `Consts` caching and `mantissa/` thresholds.

## Part M. Where each brief item lands

Columns: layer; whether it changes the zf 2.1 prerequisites; relative effect on the estimate.

| Brief | Item | Layer | zf 2.1 change? | Estimate effect |
|---|---|---|---|---|
| B1 | Mag radius type (Arb `mag_t`-like) | L0 | yes: P1, P2, P3 | + (L0) |
| B1 | Mid rounded to `prec` with `rm`, rounding error added to rad; strict unique-mid contract (`is_unique_at`) | L0 | yes: P4, P7, P9 | + (L0) |
| B1 | First-order radius bounds for elementary functions | L1 | no | ≈ (simpler than full certified kernels) |
| B1 | Higher-order/derivative propagation for compositions; inf–sup conversion for subdivision | L1 (`arith`, `interval.rs` from L0) | no | + (L1) |
| B1 | ComplexBall | L2a | no | + new |
| B1 | Ball polynomials, power series, matrices with dense LA enclosures | L2b, L2c | no | + new (moved from L4) |
| B1 | Determinism: integer-limb radius, bit-identical even with parallel/GPU mid | L0 (C5), Part J | yes: P5 (same range on all targets) | small + |
| B2 | Core principle: enclosure or explicit "couldn't establish" | L0 (`error.rs`, contract) | no | small + |
| B2 | (1) Correctness contract | L0 (Part L) | yes: P5, P12 | + |
| B2 | (2) Real balls: exact construction, arithmetic, compare/contains/overlap, precision reduction, div-by-0 rule | L0 | yes: P4, P6, P9, P12 | included in L0 |
| B2 | (3) Elementary functions and constants with domains, poles, cuts, reduction | L1 | P11 for constants | ≈ |
| B2 | (4) Complex → polynomials/series/matrices → selected specials | L2 | no | + (new L2a–c), − (fewer specials attempted) |
| B2 | (5) Validation and adaptive precision | L4 | no | + |
| B2 | Radius rules (nonnegative, rounded up, wide exponent, zero only if proven, no dropped error; add/mul formulas; overflow/underflow) | L0 | yes: P9, P12; Mag exponent widened to `i64` | included in L0 |
| B2 | API: explicit precision, three error kinds, queries, heuristic separation, no uncertainty-dropping conversions, enclosure-preserving serialization | L0 (C5) | P10 for serialization | small + |
| B2 | Tests: containment, rounding, adversarial, transcendentals, refinement, reproducibility, cross-platform | all (Part E) | P13 | ≈ |
| B2 | Oracles: python-flint dev-only next to mpmath, backed by exact rational and independent algorithms | all (Part E) | no | small + |
| B2 | Differentiators: Rust-native, deterministic documented semantics, Accumath expression → enclosure, validated workflows | L4, Part F | no | + (Accumath, L4) |
| B3 | Thin crate on ExactNum/zf limbs; `Ball{mid, rad: Mag, prec: u32}` | L0 | no | ≈ |
| B3 | Radius rules exp, sin/cos, ln, sqrt | L1 (sqrt in L0) | P6, P12 | ≈ |
| B3 | erf/J0/J1 derivative bounds; higher specials by remainder series or zf-mid + proven bound | L2d | no | − (less research risk) |
| B3 | ComplexBall rect + disk with conversions; `refine(new_prec)` | L2a; `refine` in L0 | no | + |
| B3 | Modules mag, ball, complex, arith, elementary, special, certify | L0–L4 | no | ≈ |
| B3 | No single exact-input tests; overestimation measurements and broad sets | all | no | small + |
| B4–B9 | Locked Mag: enum, `u64` mantissa, `Infinity`, value `mant·2^(exp−63)` | L0 | P1–P3; **exponent widened to `i64`** | included |
| B4–B9 | `add_up` (sticky, +1 ulp, shift ≥ 64), `mul_up` (`u128`, both product ranges) | L0 | no | included |
| B4–B9 | Overflow → Infinity, NaN/Inf → Infinity | L0 | P12 for the mid side | included |
| B4–B9 | `from_exact_num`/`to_exact_num` | L0 | P2, P3 | included |
| B4–B9 | Half ulp and `smaller_than_half_ulp` boundary set | L0 | no (integer model, K5) | included |
| B4–B9 | `ulp_mag(pre_round_exp, prec)`; mid 0 rule; `min_normal_exponent` | L0 | P5, P7 | included |
| B4–B9 | Exact mid or accounted rounding; never `prec+4` with `None` | L0 | P4 | included |
| B4–B9 | `is_unique_at` | L0 | P4 | included |
| B4–B9 | sqrt radius path, exact domain check, touch-0 rule, domain error | L0 | P6 | included |
| B4–B9 | zf 2.1 prerequisites marked EXISTS/MISSING | C1 | yes (P12, P13 added) | + (zf 2.1 roughly doubles) |
| Note | qwen3-coder:30b local model until 2026-10-30, hunk-sized directives | L0 file plan | no | ≈ (no extra runs from context) |

## Part N. Repo scaffold (prepared, not created)

Prepared locally at `/workspace/zenith-bound-scaffold/` (not a git repo, nothing pushed, no GitHub repo created):
- `Cargo.toml` (zenith-bound 0.0.0, edition 2021, rust-version 1.99, `MIT OR Apache-2.0`, `publish = false`), `rust-toolchain.toml` (channel 1.99), `src/lib.rs` (crate doc + `#![forbid(unsafe_code)]` only).
- `LICENSE-MIT` and `LICENSE-APACHE` (Apache text fetched from apache.org; MIT copyright Jeff Carr), `NOTICE-puremp` (puremp's MIT license verbatim from its repo, HEAD `283471d`), `README.md` (pre-release, nothing guaranteed), `docs/PLAN.md` (this plan).
- `jscarr64/zenith-bound` did not exist on GitHub on 2026-10-14 (repository search returned 0). Creating it needs Jeff's approval.

