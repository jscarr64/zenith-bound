//! TEST_PLAN 8.1 "Carry across the exponent" (L0a-2): `add_up` where the mantissa sum, or its round-up, reaches `2^64` renormalises to
//! `mant = 2^63`, `exp + 1`, including at the zf exponent bound `ZF_EXPONENT_MAX` (whose `+ 1` is still a `Mag` exponent) and at
//! `exp = i64::MAX`, where the carry overflows to `Infinity` (CONTRACT 1.3, 3.2, 14.3). Oracle: hand-derived exact values. A `Mag` is
//! `mant * 2^(exp - 64)`; `u64::MAX` at `e` plus `2^63` at `e - 63` (one unit of the first) is exactly `2^e` = `2^63` at `e + 1`; with
//! the second operand at `e - 100` instead the exact sum lies strictly between `u64::MAX` at `e` and `2^e`, so rounding up gives `2^e`;
//! `u64::MAX` at `e` doubled is exactly `u64::MAX` at `e + 1` (no carry, the control).
//! COVERS: Mag::add_up

use crate::common::expect_eq;
use crate::common::generators::{TOP, ZF_EXPONENT_MAX};
use zenith_bound::mag::Mag;

/// The exponents the rows run at: an ordinary one, one below the zf bound, the zf bound, and the top of `i64`.
const CARRY_EXPONENTS: [i64; 5] = [
    0,
    ZF_EXPONENT_MAX - 1,
    ZF_EXPONENT_MAX,
    i64::MAX - 1,
    i64::MAX,
];

fn mag(mant: u64, exp: i64) -> Result<Mag, String> {
    Mag::finite(mant, exp).map_err(|e| format!("Mag::finite({:#x}, {}) failed: {:?}", mant, exp, e))
}

/// `mant` at `e + 1`, or `Infinity` when `e + 1` is past `i64::MAX`.
fn at_next(mant: u64, e: i64) -> Result<Mag, String> {
    match e.checked_add(1) {
        Some(n) => mag(mant, n),
        None => Ok(Mag::infinity()),
    }
}

/// Runs every carry row through `add` (both argument orders); returns the number of rows checked.
fn check_carry_rows(add: fn(&Mag, &Mag) -> Mag) -> Result<usize, String> {
    let mut rows = 0usize;
    for e in CARRY_EXPONENTS {
        let a = mag(u64::MAX, e)?;
        let cases = [
            ("exact carry", mag(TOP, e - 63)?, at_next(TOP, e)?),
            ("round-up carry", mag(TOP, e - 100)?, at_next(TOP, e)?),
            (
                "no carry (doubling)",
                mag(u64::MAX, e)?,
                at_next(u64::MAX, e)?,
            ),
        ];
        for (name, b, want) in cases {
            expect_eq(&format!("{} at {}: a + b", name, e), &add(&a, &b), &want)?;
            expect_eq(&format!("{} at {}: b + a", name, e), &add(&b, &a), &want)?;
            rows += 1;
        }
    }
    Ok(rows)
}

/// An `add_up` that saturates the mantissa at `u64::MAX` instead of carrying into the exponent.
fn add_up_without_the_carry(a: &Mag, b: &Mag) -> Mag {
    let r = Mag::add_up(a, b);
    match (r.mant(), a.exp(), b.exp()) {
        (Some(TOP), Some(ea), Some(eb)) => Mag::finite(u64::MAX, ea.max(eb)).unwrap_or(r),
        (None, Some(ea), Some(eb)) if r.is_infinity() => {
            Mag::finite(u64::MAX, ea.max(eb)).unwrap_or(r)
        }
        _ => r,
    }
}

#[test]
fn add_up_carries_into_the_exponent_and_overflows_to_infinity_at_i64_max() -> Result<(), String> {
    expect_eq("carry rows", &check_carry_rows(Mag::add_up)?, &15)
}

#[test]
fn the_carry_rows_reject_an_add_up_that_saturates_instead_of_carrying() -> Result<(), String> {
    match check_carry_rows(add_up_without_the_carry) {
        Ok(n) => Err(format!("the saturating add_up passed all {} carry rows", n)),
        Err(_) => Ok(()),
    }
}
