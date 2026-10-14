# zenith-bound correctness contract

**Status: draft, 2026-10-14.** This file exists before any `mag` code. The clauses below are normative; clauses 1 and 5 are LOCKED. The remaining clauses of `docs/PLAN.md` Part L (value model, invariant, precision, exponent range, rounding, the other radius rules, domains, exactness, correctness vs tightness, determinism) move in here before L0 code is written. If this file and the plan disagree, this file wins and the plan gets fixed. Every test cites a clause number.

Notation: `Mag` is `enum Mag { Finite { exp: i64, mant: u64 }, Infinity }`, an upper bound on a nonnegative real. "Rounded up/down" means toward +∞/−∞.

## 1. The exponent convention — LOCKED (approved by Jeff, 2026-10-14)

1.1 Everywhere in zenith-bound (API docs, `Mag`, the `zf-ball-enc` encoding, traces, tests), a finite nonzero value with exponent `e` and a `k`-bit mantissa `m` (an unsigned integer with its top bit set) means

    value = 0.m · 2^e = m · 2^(e − k),   so value ∈ [2^(e−1), 2^e).

This is zenith-float's `exponent()` convention (`3` is `e = 2`, `m = 0xC000…`). No other convention is used. Where a leading-bit exponent is needed, it is `e − 1` and named `leading_bit_exponent`.

1.2 **Mag:** `k = 64`, `mant ∈ [2^63, 2^64)`, value = `0.mant · 2^exp` = `mant · 2^(exp − 64)`, value ∈ [2^(exp−1), 2^exp). (This replaces the earlier `mant · 2^(exp − 63)`. The fields are unchanged.)

1.3 **`mul_up(a, b)`** (LOCKED with 1.2, from `ee5dbd59`). `P = ma · mb ∈ [2^126, 2^128)` and value = `P · 2^(ea + eb − 128)`.
- If `P ≥ 2^127`: `mant = P >> 64`, `exp = ea + eb`, sticky = low 64 bits ≠ 0.
- If `P ∈ [2^126, 2^127)`: `mant = P >> 63`, `exp = ea + eb − 1`, sticky = low 63 bits ≠ 0.
- If sticky, add 1 to `mant`. A carry to `2^64` gives `mant = 2^63`, `exp + 1`.

1.4 **Half ulp and `smaller_than_half_ulp`** (LOCKED with 1.2, from `ee5dbd59`). For a midpoint with zf exponent `mid_exp` at precision `prec`, ulp = `2^(mid_exp − prec)` and half ulp = `2^(mid_exp − prec − 1)`. A finite Mag is smaller than half an ulp iff `exp ≤ mid_exp − prec − 1`. Boundary set at `mid_exp = 10, prec = 4` (half ulp `2^5`): `exp = 5` with mantissas `{2^63, 2^63 + 1, 2^64 − 1}` is smaller; `exp = 6` is not. Sweep `exp` 3..8 against the independent rational model.

1.5 The encoding's `exp i64` fields (midpoint and Mag) and the trace `e` fields use this convention.

## 2. Zero Mag

2.1 A proven-zero radius is the canonical `Finite { exp: 0, mant: 0 }`. It is the only finite Mag with `mant < 2^63`, and every constructor and the decoder reject any other.

2.2 A radius is zero only when exactness is proven (exact inputs and an `Equal` ternary from the op).

2.3 Every Mag operation early-returns on a zero operand, before any alignment, normalisation or `leading_zeros`: `add_up(0, b) = b`, `mul_up(0, finite) = 0`, `from_exact_num(0) = 0`, `sub_lower(0, b) = 0`, and comparisons with zero go by sign only.

2.4 `0 · Infinity = Infinity`.

2.5 Zero (midpoint 0, radius 0, and both) is a member of every test set.

## 3. Mag underflow (and overflow)

3.1 Exponent arithmetic uses `checked_*` only, never wrapping.

3.2 **Underflow:** a result whose exponent would go below `i64::MIN` rounds **up** to the smallest positive Mag, `Finite { exp: i64::MIN, mant: 2^63 }`. It never becomes zero and never wraps. (A lower-bound helper such as `sub_lower` may return zero, because a smaller lower bound stays sound.)

3.3 **Overflow:** a result whose exponent would go above `i64::MAX` becomes `Infinity`, never a saturated finite value. NaN or Inf inputs from zf map to `Infinity`.

## 4. Division radius

4.1 For `a = m_a ± r_a` and `b = m_b ± r_b`, the midpoint is `q = m_a / m_b`, rounded to `prec` under the caller's `rm`.

4.2 If the divisor touches 0 (`|m_b| ≤ r_b`, decided by `Mag::cmp_exact_num` without converting `r_b` to a zf number), the result is `BoundError::Domain` under every `DomainPolicy`.

4.3 Otherwise

    rad = (r_a + |q|_up · r_b) / (|m_b| − r_b)_down + ε

- numerator: `add_up(r_a, mul_up(qu, r_b))`, where `qu = add_up(from_exact_num(q), ε)` is an upper bound on the **exact** quotient's magnitude (the rounded `q` can be up to `ε` below it);
- denominator: `sub_lower(from_exact_num_lower(m_b), r_b)`, rounded down (positive by 4.2);
- quotient: `div_up`;
- `ε`: `ulp_mag(pre_round_exp, prec)` if rounding `q` was inexact, otherwise 0.

4.4 All radius steps are O(1) Mag operations on u64/u128. Only `q` is computed at `prec`.

## 5. Precision rounding — LOCKED (approved by Jeff, 2026-10-14)

5.1 A midpoint at precision `prec` is rounded to **exactly** `prec` bits (any `prec ≥ 1`, not only a multiple of 64), once, under the caller's `rm`.

5.2 Exact-bit rounding lives in **zenith-float** (PLAN C1 **P15**, tied to zf defect 6). zenith-bound **never** re-chops or re-rounds a zf result to a smaller precision, and has no chop fallback, because a second rounding of an already-rounded value is double rounding.

5.3 If the zf 2.0.0 defect-6 fix delivers exact-bit rounding, P15 folds into 2.0.0; otherwise it stays in 2.1. Either way, L0 does not start until it is green on Pop.
