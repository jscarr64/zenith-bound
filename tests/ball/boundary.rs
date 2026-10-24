//! CONTRACT 3.2 (round-up underflow to the smallest positive `Mag`), 6.4 and 15.3 (`is_informative`), 14.7 (ulp terms);
//! TEST_PLAN L0b-14 (degenerate and all-reals) at the i64 exponent limits (zf 2.0 `Exponent` is i64, Jeff 2026-10-23
//! 12:36 PM CT). Spence's rule (2026-10-23 7:51 PM CT): an ulp exponent below what `Mag` holds rounds outward to the
//! smallest positive `Mag`, never to zero. Oracle: the exponent field `mid_exp - prec + 1` (full ulp) or `mid_exp - prec`
//! (half ulp) computed here in i128 and clamped up to `i64::MIN`; the mantissa is always the top bit.
//! COVERS: Mag::ulp_mag, Mag::half_ulp, Mag::smaller_than_half_ulp, Ball::from_mid_rad, Ball::is_informative, Ball::is_exact

use zenith_bound::ball::Ball;
use zenith_bound::mag::Mag;
use zenith_float_num::ExactNum;

const TOP: u64 = 1u64 << 63;

fn precisions() -> Result<Vec<usize>, String> {
    let wide = usize::try_from(u32::MAX).map_err(|_| String::from("u32::MAX as usize"))?;
    Ok(vec![0, 1, 2, 53, 4096, wide])
}

/// The power of two with exponent field `field`, clamped up to `i64::MIN` (outward rounding).
fn expect_pow2(what: &str, got: &Mag, field: i128) -> Result<(), String> {
    let want = i64::try_from(field.max(i128::from(i64::MIN)))
        .map_err(|_| format!("{}: field {} above i64::MAX", what, field))?;
    if got.exp() == Some(want) && got.mant() == Some(TOP) {
        Ok(())
    } else {
        Err(format!(
            "{}: want 2^({} - 1) (field {}), got {:?}",
            what, want, field, got
        ))
    }
}

fn mid_exponents() -> [i64; 3] {
    [i64::MIN, i64::MIN + 1, i64::MIN + 2]
}

#[test]
fn ulp_below_the_mag_range_rounds_out_to_the_smallest_positive_mag() -> Result<(), String> {
    for e in mid_exponents() {
        for p in precisions()? {
            let pi = i128::try_from(p).map_err(|_| String::from("prec"))?;
            let field = i128::from(e) - pi + 1;
            let what = format!("ulp_mag({}, {})", e, p);
            expect_pow2(&what, &Mag::ulp_mag(e, p), field)?;
        }
    }
    Ok(())
}

#[test]
fn half_ulp_below_the_mag_range_rounds_out_to_the_smallest_positive_mag() -> Result<(), String> {
    for e in mid_exponents() {
        for p in precisions()? {
            let pi = i128::try_from(p).map_err(|_| String::from("prec"))?;
            let what = format!("half_ulp({}, {})", e, p);
            expect_pow2(&what, &Mag::half_ulp(e, p), i128::from(e) - pi)?;
        }
    }
    Ok(())
}

#[test]
fn the_clamped_radius_is_not_below_half_an_ulp() -> Result<(), String> {
    // smaller_than_half_ulp compares against the unclamped threshold, so a radius rounded up to the smallest
    // positive Mag is never reported as below half an ulp (that would let a caller drop it).
    for e in mid_exponents() {
        for p in precisions()? {
            if Mag::half_ulp(e, p).smaller_than_half_ulp(e, p) {
                return Err(format!(
                    "half_ulp({}, {}) reported as smaller than half an ulp",
                    e, p
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn balls_with_a_clamped_ulp_radius_are_informative_and_not_exact() -> Result<(), String> {
    for e in mid_exponents() {
        for p in [1usize, 53, 4096] {
            for rad in [Mag::ulp_mag(e, p), Mag::half_ulp(e, p)] {
                let b = Ball::from_mid_rad(ExactNum::from(1i64), rad, p)
                    .map_err(|err| format!("from_mid_rad at {}, {}: {:?}", e, p, err))?;
                if !b.is_informative() || b.is_exact() {
                    return Err(format!(
                        "ulp radius at mid exponent {}, prec {}: is_informative {} is_exact {}",
                        e,
                        p,
                        b.is_informative(),
                        b.is_exact()
                    ));
                }
            }
        }
    }
    Ok(())
}

#[test]
fn is_informative_flips_exactly_where_the_ulp_overflows() -> Result<(), String> {
    // The top end: ulp_mag(i64::MAX, 0) has field i64::MAX + 1 and is Infinity (not informative); every other
    // precision at i64::MAX, and prec 0 one below it, stays finite (informative).
    let cases = [
        (i64::MAX, 0usize, false),
        (i64::MAX, 1, true),
        (i64::MAX - 1, 0, true),
        (i64::MAX, 4096, true),
    ];
    for (e, p, informative) in cases {
        let rad = Mag::ulp_mag(e, p);
        let b = Ball::from_mid_rad(ExactNum::from(1i64), rad, 1)
            .map_err(|err| format!("from_mid_rad ulp_mag({}, {}): {:?}", e, p, err))?;
        if b.is_informative() != informative {
            return Err(format!(
                "ulp_mag({}, {}): is_informative {}, want {}",
                e,
                p,
                b.is_informative(),
                informative
            ));
        }
    }
    Ok(())
}
