//! Harper S2: cmp_exact_diff wide-gap path with the radius one binade above zf EXPONENT_MAX (CONTRACT 16.S2.1 to
//! 16.S2.3: a radius outside zf's exponent range still compares). Hand-worked, E = EXPONENT_MAX:
//! a = 2^E (1 - 2^-200) (exponent E), b = -2^(E-130) (exponent E-129), gap 129 > EXACT_GAP so cmp_far_diff runs.
//! |a - b| = 2^E (1 - 2^-200 + 2^-130). Scaled by 2^(200-E) (an exact integer comparison): |a - b| -> 2^200 - 1 + 2^70,
//! r = 2^E = Mag(TOP, E+1) -> 2^200, so |a - b| - r -> 2^70 - 1 > 0 and r vs |a - b| is Less.
//! With b = +2^(E-130): |a - b| -> 2^200 - 1 - 2^70 < 2^200, so Greater. Negating both operands keeps |a - b|.
//! Neighbour: r = Mag(u64::MAX, E) = 2^E (1 - 2^-64) -> 2^200 - 2^136 < 2^200 - 1 + 2^70, so Less.
//! COVERS: Mag::cmp_exact_diff

use core::cmp::Ordering;

use zenith_bound::mag::Mag;
use zenith_float_num::{ExactNum, EXPONENT_MAX};

const TOP: u64 = 0x8000_0000_0000_0000;

fn mag(mant: u64, exp: i64) -> Result<Mag, String> {
    Mag::finite(mant, exp).map_err(|e| format!("Mag::finite({:#x}, {}): {:?}", mant, exp, e))
}

/// `0.mant * 2^exp` exactly at 64 bits.
fn parts(mant: u64, exp: i64) -> Result<ExactNum, String> {
    ExactNum::from_mag_parts(mant, exp, 64)
        .map_err(|e| format!("from_mag_parts({:#x}, {}): {:?}", mant, exp, e))
}

fn neg(x: &ExactNum) -> ExactNum {
    ExactNum::from(0).sub_full_prec(x)
}

fn want(what: &str, got: Option<Ordering>, w: Ordering) -> Result<(), String> {
    if got == Some(w) {
        Ok(())
    } else {
        Err(format!("{}: got {:?}, want {:?}", what, got, w))
    }
}

/// (a, 2^(E-130)) with a = 2^E (1 - 2^-200) = 2^(E-1) + (2^(E-1) - 2^(E-200)), built exactly.
fn operands() -> Result<(ExactNum, ExactNum), String> {
    let e = EXPONENT_MAX;
    let half = parts(TOP, e)?; // 2^(E-1)
    let tiny = parts(TOP, e.checked_sub(199).ok_or("E-199")?)?; // 2^(E-200)
    let rest = half.sub_full_prec(&tiny);
    let a = half.add_full_prec(&rest);
    if a.inexact() || rest.inexact() {
        return Err("a not exact".into());
    }
    let s = parts(TOP, e.checked_sub(129).ok_or("E-129")?)?; // 2^(E-130)
    Ok((a, s))
}

fn r_above() -> Result<Mag, String> {
    mag(TOP, EXPONENT_MAX.checked_add(1).ok_or("E+1")?) // 2^E, exponent E+1
}

#[test]
#[ignore = "red: CONTRACT 16.S2.1/16.S2.2: cmp_far_diff returns None (to_exact_num fails above zf EXPONENT_MAX), bridge.rs:137"]
fn s2_diff_edge_radius_above_emax_less() -> Result<(), String> {
    let (a, s) = operands()?;
    want(
        "2^E vs |a - -s|",
        r_above()?.cmp_exact_diff(&a, &neg(&s)),
        Ordering::Less,
    )
}

#[test]
#[ignore = "red: CONTRACT 16.S2.1/16.S2.2: cmp_far_diff returns None (to_exact_num fails above zf EXPONENT_MAX), bridge.rs:137"]
fn s2_diff_edge_radius_above_emax_mirrored_less() -> Result<(), String> {
    let (a, s) = operands()?;
    want(
        "2^E vs |-a - s|",
        r_above()?.cmp_exact_diff(&neg(&a), &s),
        Ordering::Less,
    )
}

#[test]
#[ignore = "red: CONTRACT 16.S2.1/16.S2.2: cmp_far_diff returns None (to_exact_num fails above zf EXPONENT_MAX), bridge.rs:137"]
fn s2_diff_edge_radius_above_emax_greater() -> Result<(), String> {
    let (a, s) = operands()?;
    want(
        "2^E vs |a - s|",
        r_above()?.cmp_exact_diff(&a, &s),
        Ordering::Greater,
    )
}

#[test]
fn s2_diff_edge_radius_at_emax_less() -> Result<(), String> {
    let (a, s) = operands()?;
    let r = mag(u64::MAX, EXPONENT_MAX)?; // 2^E (1 - 2^-64), exponent E
    want(
        "2^E(1-2^-64) vs |a - -s|",
        r.cmp_exact_diff(&a, &neg(&s)),
        Ordering::Less,
    )?;
    want(
        "2^E(1-2^-64) vs |-a - s|",
        r.cmp_exact_diff(&neg(&a), &s),
        Ordering::Less,
    )
}
