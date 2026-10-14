# Gemini review of the zenith-bound plan (2026-10-14)

Five items, as relayed to Jeff on 2026-10-14. The dispositions are also in `docs/PLAN.md` Part R (R6).

## Items

- **G1. Precision is quantised to 64 bits.** zf rounds the precision up to a multiple of 64 bits. So `prec = 53` gives about 2^11 ≈ 2048× radius overestimation, and `is_unique_at` actually tests at 64 bits. Either add a zf 2.1 P-item for exact-bit rounding (linked to zf defect 6, which is being fixed in 2.0.0) or add an arbitrary-bit chop in zenith-bound. Recommend one and lock it before L0.
- **G2. Radius math in O(1).** The sqrt, div and ln radius paths should use `Mag::sub_lower`, `sqrt_up` and `div_up` on u64/u128, never the full `Ball.prec`.
- **G3. The `ulp.rs` clamp.** `eff_exp = max(pre_round_exp, MIN_NORMAL_EXPONENT)`. Check it against the exponent definition.
- **G4. Split `set_prec` from `refine`.** In `set_prec`, lowering rounds the mid and adds an ulp, and raising never shrinks the radius. `refine` belongs in a `Refinable` trait or on exact constructors.
- **G5. Ziv remainders.** Verify that the Ziv loops in zf `ops/*.rs` bound the truncation remainder rigorously, so that `Down` and `Up` can't both land on one side of a hard-to-round value.

## Dispositions

| # | Disposition | Plan change |
|---|---|---|
| G1 | **Accepted. Recommended: zf 2.1 P15, exact-bit rounding**, built on the zf defect-6 fix. A second chop in zenith-bound would be double rounding (wrong under nearest modes unless it starts from the exact value), and P15 keeps one MPFR-tested rounding path. A chop from exact values stays as the fallback only if P15 slips. P15 gates L0; Jeff locks the choice before L0 starts | C1 P15, C5, Part L clause 3, L0 gate, Summary, K5 |
| G2 | **Accepted.** New O(1) Mag helpers (`from_exact_num_lower`, `sub_lower`, `mul_lower`, `sqrt_lower`, `div_up`, `div_lower`) in `mag/lower.rs`, `mag/div.rs` and `mag/sqrt.rs`. The sqrt, div and ln radii never touch `Ball.prec` | C6, L1, L0 file plan |
| G3 | **Accepted, checked.** The clamp is sound when both exponents use the one `0.m·2^e` convention, so `MIN_NORMAL_EXPONENT` must be zf `EXPONENT_MIN` in that convention. A leading-bit constant (`EXPONENT_MIN − 1`) would halve the subnormal ulp, which is unsound. Pinned in P5, with a boundary set at `MIN_NORMAL_EXPONENT − 1, 0, +1` under all modes | C6 |
| G4 | **Accepted.** `set_prec`: lowering rounds the mid and adds the rounding error, raising leaves the mid and radius unchanged. `Ball` has no `refine`. `trait Refinable` is implemented by exact constructors and constants (later, by expression nodes) | L0 file plan (`precision.rs`, `refine.rs`), L2a, Part E, Part M |
| G5 | **Accepted.** Folded into P14, now a scoped audit that gates L1. Each L1 kernel gets a written finding (Rigorous / Fixable / Not rigorous → ToEven + proven ≤ 1 ulp, or a certified kernel) plus its MPFR Up/Down set, including the exact cases | C1 P14, L1 gate |

Estimate effect: zf 2.1 14–24 → 18–32 and L0 22–34 → 24–37, so the total goes from ~195–345 to **~201–356** (relative runs, rechecked against the column sums).
