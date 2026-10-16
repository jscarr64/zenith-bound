# zenith-bound correctness contract

**Status: draft, 2026-10-15 (numbering and L0 clauses added; clauses 1 and 5 unchanged and LOCKED).** This file exists before any `mag` code. The clauses below are normative; clauses 1 and 5 are LOCKED. The open items in the section "Open clauses" are **not normative** until Jeff or Rusty decides them, and no test may assert them. If this file and `docs/PLAN.md` disagree, this file wins and the plan gets fixed. Every test cites a clause number (`CONTRACT n.m`) and names its oracle.

**Numbering.** Clauses 1 to 5 keep the numbers they had on 2026-10-14, because 1 and 5 are LOCKED and `docs/PLAN.md`, the handoff and review notes cite "clause 1", "clause 4.3" and "clause 5". The remaining clauses of `docs/PLAN.md` Part L are numbered 6 and up, in Part L order. The table at the end maps every Part L clause, and the plan's older citations ("4a", "clauses 6–7", "clause 6 (division)"), to the clause numbers used here.

Notation: `Mag` is `enum Mag { Finite { exp: i64, mant: u64 }, Infinity }`, an upper bound on a nonnegative real. "Rounded up/down" means toward +∞/−∞. `Mag` methods take `&Mag` (no `Copy`, no `Clone`).

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

## 6. Value model (PLAN Part L 1)

6.1 A `Ball` denotes the closed set `[mid − rad, mid + rad]` of reals. `rad = Infinity` denotes all reals; its canonical form has `mid = +0`.

6.2 A NaN or infinite midpoint is never stored. An operation that would produce one returns `BoundError` or an `Indeterminate` result, never a finite ball that excludes a possible value.

6.3 **Signed zero.** A −0 midpoint (for example exact cancellation under `Down`) is normalised to +0 at the ball boundary, in every constructor and every op result, so `identical()` and the encoding see one zero.

## 7. Invariant (PLAN Part L 2)

7.1 For every guaranteed op `f` and inputs `x_i ∈ X_i`, `f(x_1, …) ∈ f_ball(X_1, …)`, or the op returns `BoundError` or an `Indeterminate` result. Nothing else is allowed.

## 8. Precision and exponent range (PLAN Part L 3 and 4; precision rounding itself is clause 5)

8.1 `prec` is at least 1 and shapes only the midpoint. The radius is always a `Mag` (64-bit mantissa, `i64` exponent). zenith-float's precision type is `usize`, so `prec: u32` becomes `usize` only through `usize::try_from` (a failure is `BoundError::Resource`), never `as`.

8.2 Radii: `Mag` exponents are `i64`; beyond that the radius is `Infinity` (clause 3).

8.3 Midpoint exponents are zenith-float's `EXPONENT_MIN..=EXPONENT_MAX`. (The 32-bit-target sentence of PLAN Part L 4 is open item O3.)

## 9. Rounding and radius rules (PLAN Part L 5 and 6; division is clause 4)

9.1 **Rounding.** The midpoint is rounded once to `prec` under the caller's `rm` from an exact or fully accounted value (clause 5). The radius is rounded **up** at every step. No rounding error is ever dropped. `rm` may change tightness only, never containment. `RoundingMode::None` truncates and is never used in a bound path.

9.2 **Radius rules.** A radius is nonnegative. It is zero only when exactness is proven (clause 2.2). A zero radius early-returns through every Mag operation (clause 2.3).

9.3 **Add:** `(m + n) ± (r + s + ε)`. **Mul:** `m·n ± (|m|·s + |n|·r + r·s + ε)`. Every term and every sum is evaluated with `add_up`/`mul_up`.

9.4 **ε** is `ulp_mag(pre_round_exp, prec)` if rounding the midpoint was inexact, and 0 otherwise. The text of PLAN Part L 6 adds "a half ulp under nearest modes, a full ulp under directed ones"; which of the two applies to ε, and which exponent `pre_round_exp` is, are open items O4 and O5.

## 10. Domains and special values (PLAN Part L 7)

10.1 There is one `DomainPolicy { Strict, Clip }`. Every op with a restricted domain (sqrt, the log family, asin/acos, acosh, atanh, `pow` with a non-integer exponent, …) applies it the same way.

10.2 **`Strict`** returns `BoundError::Domain` if any point of the input ball lies outside the domain. **`Clip`** encloses `f` over the intersection of the ball with the domain, and returns `Domain` only if that intersection is empty or is a single pole.

10.3 Division by a ball touching 0 returns `Domain` under both policies (clause 4.2).

10.4 **Underflow:** the radius absorbs the lost value and is never zero; `Mag` exponent underflow rounds up to the smallest positive `Mag` (clause 3.2). `0 · Infinity` in radius arithmetic is `Infinity` (clause 2.4). Overflow never saturates (clause 3.3); how an overflow is reported is open item O2. The default policy is open item O6.

## 11. Exactness (PLAN Part L 8)

11.1 `from_integer` and `from_rational` are exact when the denominator is a power of 2 and the value fits `prec`; otherwise they return a correctly enclosed ball. `is_exact` is true only with proof.

## 12. Correctness vs tightness (PLAN Part L 9)

12.1 A correctness failure (a result that excludes a possible true value) is a P0 bug. A tightness miss is measured as an overestimation ratio against the oracle's radius and tracked; it is a performance bug.

## 13. Determinism (PLAN Part L 10)

13.1 The same `(op, inputs, prec, rm, version)` gives the same bytes on every supported target, build profile and thread count.

## 14. Mag operations (L0; from PLAN C6 and `docs/PLAN.md` Part D, L0 file plan)

All of this section is integer-only (`u64`/`u128`/`i64` and wider integers in tests). For a finite `Mag` `x`, `value(x)` is defined by clause 1.2, and the canonical zero by clause 2.1. "Exact" means the real number the operands denote.

14.1 **Round-up operations** `add_up`, `mul_up`, `div_up`, `sqrt_up`: the result is the smallest representable `Mag` that is ≥ the exact result (rounding to 64 significant bits, toward +∞), subject to clauses 2 and 3. This is what clause 1.3 states for `mul_up`. Where the exact result is not a `Mag` value (it exceeds the exponent range), clause 3.3 applies. A round-up result is therefore never below the exact result and its predecessor `Mag` is below the exact result (that is the "within one Mag-ulp" tightness property).

14.2 **Round-down operations** `sub_lower`, `mul_lower`, `div_lower`, `sqrt_lower`: the result is the largest representable `Mag` that is ≤ the exact result (rounding to 64 significant bits, toward −∞), except that a result may be the canonical zero where clause 3.2 or the rule below allows it. For an exact result above the largest `Mag` see open item O7. `sub_lower(a, b)` is the canonical zero when `a ≤ b`.

14.3 **`add_up(a, b)`** (PLAN C6): align to the larger exponent; shifted-out bits go to a sticky bit; add with carry (a carry renormalises by one and its lost bit also goes to sticky); if sticky, add 1 at the last place and renormalise. If the exponent gap is ≥ 64 and the smaller operand is nonzero, the result is the larger operand plus one unit in its last place (which is the round-up result by 14.1).

14.4 **`div_up`/`div_lower`** are computed on `u128 / u64` with a sticky remainder; **`sqrt_up`/`sqrt_lower`** by the integer square root of a `u128` with the exponent parity fixed first (PLAN L0 file plan). The results are those of 14.1 and 14.2.

14.5 **`cmp(&Mag, &Mag) -> Ordering`** compares the denoted values. Canonical zero is the least element and equals only itself; two finite nonzero values compare by `(exp, mant)` lexicographically. `Infinity` compares greater than every finite `Mag` and equal to `Infinity`. (The ordering of `Infinity` is derived from the meaning of the type, and is listed in the open items as O8 for confirmation.)

14.6 **`Infinity` operands** of the round-up operations: `add_up(x, Infinity)`, `add_up(Infinity, x)`, `mul_up(x, Infinity)` and `mul_up(Infinity, x)` are `Infinity` (for `mul_up`, including `x = 0`, clause 2.4), and `sqrt_up(Infinity)` and `div_up(Infinity, b)` for a finite nonzero `b` are `Infinity`, because no finite upper bound would be sound. Every other combination involving `Infinity` (the round-down operations, a divisor of zero or `Infinity`) is open item O7 and is not tested. Likewise the overflow of a round-down operation (an exact result above the largest `Mag`) is O7: clause 3.3 says `Infinity`, which is not a lower bound.

14.7 **`ulp_mag(pre_round_exp, prec)`** is `2^(pre_round_exp − prec)`, i.e. `Finite { exp: pre_round_exp − prec + 1, mant: 2^63 }` (clause 1.2), with `pre_round_exp` in the `0.m·2^e` convention of clause 1.1. **`half_ulp(mid_exp, prec)`** is `2^(mid_exp − prec − 1)`, i.e. `Finite { exp: mid_exp − prec, mant: 2^63 }`. **`smaller_than_half_ulp(&Mag, mid_exp, prec)`** is the predicate of clause 1.4. An exponent that leaves the `i64` range follows clause 3. The subnormal clamp `eff_exp = max(pre_round_exp, MIN_NORMAL_EXPONENT)` of PLAN G3 belongs to `mid_rounding_error`, not to `ulp_mag` (open item O5).

## 15. API shapes (PLAN C5, as far as they are settled)

15.1 Ball operations take explicit `prec: u32` and `rm: RoundingMode` and return `BoundResult<Ball>`. An operation with a restricted domain (clause 10.1) takes a `DomainPolicy` as its last parameter; an operation without one (add, sub, neg, abs, mul, mul_2exp, sqr, powi, hypot, and div/inv, whose divisor-touches-0 rule is the same under both policies, clause 10.3) does not.

15.2 `Ball` has no `PartialEq`/`PartialOrd`, no `From<Ball> for ExactNum`, no `From<f64>`, and no `refine`. Equality of representations is `identical`, value questions use the `Tri` comparisons.

15.3 Whether `BoundResult<T>` is `Result<Enclosure<T>, BoundError>` or something flatter, and where "valid but uninformative" lives, is open item O1.

## Open clauses (NOT normative; questions for Jeff or Rusty; nothing here may be tested)

- **O1 Result shape.** PLAN C5 has `BoundResult<T> = Result<Enclosure<T>, BoundError>` with `Enclosure::Indeterminate { reason }`, so a function can return `Ok(Indeterminate)`, a double wrap. Options: keep it; or `Result<Ball, BoundError>` where "uninformative" is `rad = Infinity`; or `Indeterminate` becomes a `BoundError` kind. Which?
- **O2 Overflow reporting.** PLAN Part L 7 says "`Overflow` error or a ball with `rad = Infinity`, by caller choice". There is no parameter for that choice. Is it a parameter, a policy field, or one fixed behaviour? (And with zenith-float P12, a directed overflow returns a finite `max_value` flagged inexact, so detection must be "Inf, or exponent == `EXPONENT_MAX` with inexact" — handoff Q12.)
- **O3 32-bit targets.** PLAN Part L 4 says 32-bit targets refuse midpoints outside the 32-bit range "until P5 makes the ranges equal". The handoff says `EXPONENT_MAX/MIN = i32::MAX/MIN` on every target now. Is the sentence deleted?
- **O4 Half ulp vs full ulp.** PLAN Part L 6 says ε is "a half ulp under nearest modes, a full ulp under directed ones", while clause 9.4 / PLAN C6 define ε as `ulp_mag(...)` (a full ulp) whenever inexact. Which, and is the half-ulp value allowed for nearest modes?
- **O5 `pre_round_exp`, `mid_rounding_error`, P7.** PLAN C6 says `ulp_mag` uses the "leading-bit exponent" of the exact pre-round result, but G3 and clause 1.1 say the `0.m·2^e` exponent. Clause 14.7 uses the clause 1.1 convention. Is P7 (pre-round exponent from zenith-float) required, or does `ulp(exponent(rounded mid))` suffice (handoff Q5)? Where is the subnormal clamp?
- **O6 Default policy.** PLAN says the default is `Strict`. Rust has no default arguments; is `DomainPolicy` an explicit parameter everywhere (clause 15.1) with `Strict` only as `Default::default()`?
- **O7 Mag corner operands.** Results of the round-down operations with an `Infinity` operand; `div_up`/`div_lower` with a zero or `Infinity` divisor; `sqrt_lower(Infinity)`; `mul_lower(0, Infinity)`. And whether `Mag::sub_up` exists (handoff Q11: needed by pin 3 variants and by `sqr`/`abs` hull math; PLAN lists only `sub_lower`), including its result when `a < b`.
- **O8 `Infinity` ordering.** Clause 14.5 orders `Infinity` above every finite value and equal to itself. Confirm.
- **O9 Ownership.** Ball-op ownership (`&Ball` vs consuming, zenith-float `add_ref`/`mul_ref` forms) is handoff Q1 and is not settled here; neither is `Ball.prec: u32` vs `usize` (Q6).
- **O10 `smaller_than_half_ulp` corner operands and signatures.** Clause 1.4 states the predicate for a finite nonzero `Mag`. For the canonical zero (value 0, so smaller than any half ulp, but `exp = 0` would give a `mid_exp`-dependent answer by the formula) and for `Infinity`, the result is not stated. The parameter lists of `half_ulp(mid_exp, prec)` and the inherent-vs-`Ord` form of `cmp` (an inherent `cmp(&self, &Mag)` trips clippy's `should_implement_trait` and `#[allow]` is banned, so `Ord` is the natural form) are assumed by the red tests and need a decision.

## Concordance with PLAN Part L numbering

| PLAN Part L | Topic | Clause here |
|---|---|---|
| 1 | Value model | 6 |
| 2 | Invariant | 7 |
| 3 | Precision model | 8.1 and 5 (LOCKED) |
| 4a, "clause 4a" | Exponent convention | **1** (LOCKED) |
| 4 | Exponent range | 8.2, 8.3 |
| 5 | Rounding | 9.1 and 5 |
| 6 | Radius rules | 9.2 to 9.4, and 4 for division |
| 6–7 in older plan text ("clauses 6–7", C6 zero/underflow) | Zero Mag, underflow | 2, 3 |
| "clause 6 (division)" | Division radius | 4 |
| 7 | Domains | 10 |
| 8 | Exactness | 11 |
| 9 | Correctness vs tightness | 12 |
| 10 | Determinism | 13 |
