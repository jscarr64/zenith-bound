//! Slice S1 harness rows (`src/arith.rs`). Owned by slice S1 alone.
//! Goldens: tests/goldens/slices/arith/ (its own SHA256SUMS and GENERATORS). Mutants: tools/mutants/manifest.d/arith.txt, ids 200..=299.
//! Golden file headers for this slice are declared here, not in tests/common/goldens.rs.
//! CONTRACT 2.2, 3.2, 6.3, 9.3, 9.4, 10.2b, 10.2e, 10.4, 15.1 and 16.S1.1 to 16.S1.8. Oracles: hand-verified exact rows
//! (each value worked by hand in the comment beside it); containment of exact images computed with zenith-float's exact
//! `add_full_prec`/`mul_full_prec` (and directed `sqrt` brackets) at sample points of the input balls; the committed L0b
//! enclosure rows tests/goldens/ball_enclosure_rows.txt (exact images rounded outward by scripts/gen_ball_enclosure.py).
//! COVERS: Ball::add, Ball::sub, Ball::mul, Ball::mul_exact, Ball::sqr, Ball::powi, Ball::sqrt, Neg for Ball

use zenith_bound::ball::Ball;
use zenith_bound::enclosure::Enclosure;
use zenith_bound::error::BoundError;
use zenith_bound::mag::Mag;
use zenith_bound::policy::DomainPolicy;
use zenith_bound::tri::Tri;
use zenith_float_num::{ExactNum, RoundingMode, EXPONENT_MAX, EXPONENT_MIN};

use crate::common::goldens::golden_dir;
use crate::common::rng::SplitMix64;

const TOP: u64 = 0x8000_0000_0000_0000;
const PRECS: [usize; 9] = [1, 2, 53, 64, 65, 113, 1024, 4096, 16384];
/// Mid exponents: 0, small, 1e±300 (2^±997), and far beyond (to 2^±2^28, so a product stays inside zf's i32 range).
const EXPS: [i64; 13] = [
    0, 10, -10, 997, -997, 1000, -1000, 100_000, -100_000, 1 << 20, -(1 << 20), 1 << 28, -(1 << 28),
];

fn num(v: i64) -> ExactNum {
    ExactNum::from(v)
}

fn mag(mant: u64, exp: i64) -> Result<Mag, String> {
    Mag::finite(mant, exp).map_err(|e| format!("Mag::finite({:#x}, {}): {:?}", mant, exp, e))
}

/// `2^k` exactly.
fn pow2(k: i64) -> Result<ExactNum, String> {
    let e = k.checked_add(1).ok_or("pow2 exponent")?;
    ExactNum::from_mag_parts(TOP, e, 64).map_err(|e| format!("pow2({}): {:?}", k, e))
}

/// `v * 2^k` exactly.
fn dy(v: i64, k: i64) -> Result<ExactNum, String> {
    Ok(num(v).mul_full_prec(&pow2(k)?))
}

fn ok(what: &str, r: Result<Ball, BoundError>) -> Result<Ball, String> {
    r.map_err(|e| format!("{}: unexpected {:?}", what, e))
}

fn ball(mid: ExactNum, rad: Mag, prec: usize) -> Result<Ball, String> {
    ok("from_mid_rad", Ball::from_mid_rad(mid, rad, prec))
}

fn pt(v: i64, prec: usize) -> Result<Ball, String> {
    ok("exact", Ball::exact(num(v), prec))
}

fn want_exact(what: &str, b: &Ball, mid: &ExactNum, rad: &Mag) -> Result<(), String> {
    if b.mid() != mid || b.rad() != rad {
        return Err(format!("{}: got {:?}, want mid {:?} rad {:?}", what, b, mid, rad));
    }
    Ok(())
}

fn want_domain(what: &str, r: Result<Ball, BoundError>) -> Result<(), String> {
    match r {
        Err(BoundError::Domain { .. }) => Ok(()),
        other => Err(format!("{}: want Domain, got {:?}", what, other)),
    }
}

fn want_contains(what: &str, b: &Ball, x: &ExactNum) -> Result<(), String> {
    match b.contains(x) {
        Tri::True => Ok(()),
        t => Err(format!("{}: {:?} does not contain {:?} ({:?})", what, b, x, t)),
    }
}

// ---- hand-verified exact rows ------------------------------------------------------------------------------------

#[test]
fn add_exact_points() -> Result<(), String> {
    // 3 + 5 = 8, both exact: rad 0 (CONTRACT 2.2).
    let b = ok("add", pt(3, 53)?.add(&pt(5, 53)?, 53))?;
    want_exact("3+5", &b, &num(8), &Mag::zero())?;
    // mixed signs: -3 + 5 = 2.
    let b = ok("add", pt(-3, 53)?.add(&pt(5, 53)?, 53))?;
    want_exact("-3+5", &b, &num(2), &Mag::zero())
}

#[test]
fn add_radii_sum_exactly() -> Result<(), String> {
    // (1 ± 2^-10) + (2 ± 2^-10) = 3 ± 2^-9; Mag 2^-10 is finite(TOP, -9), 2^-9 is finite(TOP, -8).
    let a = ball(num(1), mag(TOP, -9)?, 53)?;
    let b = ball(num(2), mag(TOP, -9)?, 53)?;
    want_exact("radii", &ok("add", a.add(&b, 53))?, &num(3), &mag(TOP, -8)?)
}

#[test]
fn add_rounding_adds_half_ulp() -> Result<(), String> {
    // 1 + 2^-60 at 53 bits rounds to 1 (exponent 1); ε = half_ulp(1, 53) = 2^-53 = finite(TOP, -52).
    let b = ok("add", pt(1, 53)?.add(&ok("exact", Ball::exact(pow2(-60)?, 53))?, 53))?;
    want_exact("1+2^-60", &b, &num(1), &mag(TOP, -52)?)?;
    want_contains("1+2^-60", &b, &num(1).add_full_prec(&pow2(-60)?))
}

#[test]
fn add_at_prec_one() -> Result<(), String> {
    // 1 + 2 = 3 at 1 bit: ties-to-even of 3 = 0.11b·2^2 gives 4 (0.1b·2^3); exponent 3, ε = half_ulp(3, 1) = 2^1 = finite(TOP, 2).
    let b = ok("add", pt(1, 1)?.add(&pt(2, 1)?, 1))?;
    want_exact("1+2 @1", &b, &num(4), &mag(TOP, 2)?)?;
    want_contains("1+2 @1", &b, &num(3))
}

#[test]
fn sub_rows() -> Result<(), String> {
    // (5 ± 2^-10) - (3 ± 2^-10) = 2 ± 2^-9.
    let a = ball(num(5), mag(TOP, -9)?, 53)?;
    let b = ball(num(3), mag(TOP, -9)?, 53)?;
    want_exact("5-3", &ok("sub", a.sub(&b, 53))?, &num(2), &mag(TOP, -8)?)?;
    // 3 - 3 = 0 exactly, +0 midpoint.
    let z = ok("sub", pt(3, 53)?.sub(&pt(3, 53)?, 53))?;
    want_exact("3-3", &z, &num(0), &Mag::zero())?;
    if z.mid().is_negative() {
        return Err("3-3 gave -0".into());
    }
    Ok(())
}

#[test]
fn neg_rows() -> Result<(), String> {
    // -(3 ± 2^-10) = -3 ± 2^-10; -(0 ± 0) = +0 ± 0 (CONTRACT 6.3); prec kept.
    let b = -ball(num(3), mag(TOP, -9)?, 53)?;
    want_exact("-(3)", &b, &num(-3), &mag(TOP, -9)?)?;
    let z = -pt(0, 7)?;
    if !z.mid().is_zero() || z.mid().is_negative() || z.prec() != 7 || !z.rad().is_zero() {
        return Err(format!("-(0) = {:?}", z));
    }
    let back = -(-ball(num(-5), mag(TOP, 1)?, 53)?);
    want_exact("--(-5)", &back, &num(-5), &mag(TOP, 1)?)
}

#[test]
fn mul_rows() -> Result<(), String> {
    // (3 ± 2^-10)(5 ± 2^-10) = 15 ± (3·2^-10 + 5·2^-10 + 2^-20) = 15 ± (2^-7 + 2^-20).
    // 2^-7 + 2^-20 = 2^-7 (1 + 2^-13): mant TOP | TOP >> 13, exp -6.
    let a = ball(num(3), mag(TOP, -9)?, 53)?;
    let b = ball(num(5), mag(TOP, -9)?, 53)?;
    let want_rad = mag(TOP | (TOP >> 13), -6)?;
    want_exact("3*5", &ok("mul", a.mul(&b, 53))?, &num(15), &want_rad)?;
    // mixed signs: (-3)(5) = -15 exact.
    want_exact("-3*5", &ok("mul", pt(-3, 53)?.mul(&pt(5, 53)?, 53))?, &num(-15), &Mag::zero())?;
    // zero times a huge radius (beyond zf's range) with no radius on the zero: 0 ± (0·R + 0·0 + 0·R) = 0 ± 0.
    let huge = ball(num(1), mag(TOP, 1 << 40)?, 53)?;
    want_exact("0*huge", &ok("mul", pt(0, 53)?.mul(&huge, 53))?, &num(0), &Mag::zero())
}

#[test]
fn mul_exact_rows() -> Result<(), String> {
    // (3 ± 2^-10) · (-4) = -12 ± 2^-8.
    let a = ball(num(3), mag(TOP, -9)?, 53)?;
    want_exact("3*-4", &ok("mul_exact", a.mul_exact(&num(-4), 53))?, &num(-12), &mag(TOP, -7)?)?;
    // NaN and Inf scalars are Representation.
    for c in [ExactNum::nan(Some(zenith_float_num::Error::InvalidArgument)), ExactNum::from(1i64).div(&num(0), 64, RoundingMode::ToEven)] {
        match pt(1, 53)?.mul_exact(&c, 53) {
            Err(BoundError::Representation { .. }) => {}
            other => return Err(format!("mul_exact by {:?}: {:?}", c, other)),
        }
    }
    Ok(())
}

#[test]
fn sqr_rows() -> Result<(), String> {
    // (3 ± 2^-10)^2 = 9 ± (2·3·2^-10 + 2^-20) = 9 ± (3·2^-9 + 2^-20); value 2^-7 (0.75 + 2^-13): mant (3<<62)|(1<<51), exp -7.
    let a = ball(num(3), mag(TOP, -9)?, 53)?;
    want_exact("3^2", &ok("sqr", a.sqr(53))?, &num(9), &mag(0xC000_0000_0000_0000 | (1 << 51), -7)?)?;
    // (-3)^2 = 9 exact.
    want_exact("(-3)^2", &ok("sqr", pt(-3, 53)?.sqr(53))?, &num(9), &Mag::zero())?;
    // 3^2 = 9 at 2 bits rounds to 8 (ties: 9 = 1001b, nearest 2-bit values 8 and 12): ε = half_ulp(4, 2) = 2^1.
    let b = ok("sqr", pt(3, 2)?.sqr(2))?;
    want_exact("3^2 @2", &b, &num(8), &mag(TOP, 2)?)?;
    want_contains("3^2 @2", &b, &num(9))
}

#[test]
fn powi_rows() -> Result<(), String> {
    // 3^5 = 243, 2^10 = 1024, (-2)^3 = -8: exact.
    want_exact("3^5", &ok("powi", pt(3, 53)?.powi(5, 53))?, &num(243), &Mag::zero())?;
    want_exact("2^10", &ok("powi", pt(2, 53)?.powi(10, 53))?, &num(1024), &Mag::zero())?;
    want_exact("(-2)^3", &ok("powi", pt(-2, 53)?.powi(3, 53))?, &num(-8), &Mag::zero())?;
    want_exact("7^1", &ok("powi", pt(7, 53)?.powi(1, 53))?, &num(7), &Mag::zero())?;
    // x^0 = 1 ± 0 for every ball, including 0 and a ball containing 0 (CONTRACT 10.2e).
    want_exact("0^0", &ok("powi", pt(0, 53)?.powi(0, 53))?, &num(1), &Mag::zero())?;
    let wide = ball(num(0), mag(TOP, 5)?, 53)?;
    want_exact("(0±16)^0", &ok("powi", wide.powi(0, 53))?, &num(1), &Mag::zero())?;
    // 3^41 > 2^53: inexact at 53 bits, still encloses 3^41 exactly.
    let mut p = num(1);
    for _ in 0..41 {
        p = p.mul_full_prec(&num(3));
    }
    let b = ok("powi", pt(3, 53)?.powi(41, 53))?;
    if b.is_exact() {
        return Err("3^41 at 53 bits cannot be exact".into());
    }
    want_contains("3^41", &b, &p)?;
    // (1 ± 2^-10)^2 = 1 ± (2^-9 + 2^-20) by squaring: contains 0.999^2 and 1.001^2 endpoints exactly.
    let a = ball(num(1), mag(TOP, -9)?, 53)?;
    let b = ok("powi", a.powi(2, 53))?;
    let lo = num(1).add_full_prec(&-pow2(-10)?);
    let hi = num(1).add_full_prec(&pow2(-10)?);
    want_contains("(1-r)^2", &b, &lo.mul_full_prec(&lo))?;
    want_contains("(1+r)^2", &b, &hi.mul_full_prec(&hi))
}

#[test]
fn sqrt_rows() -> Result<(), String> {
    let strict = DomainPolicy::Strict;
    let clip = DomainPolicy::Clip;
    // sqrt(4) = 2, sqrt(0) = 0: exact.
    want_exact("sqrt 4", &ok("sqrt", pt(4, 53)?.sqrt(53, &strict))?, &num(2), &Mag::zero())?;
    want_exact("sqrt 0", &ok("sqrt", pt(0, 53)?.sqrt(53, &strict))?, &num(0), &Mag::zero())?;
    // sqrt(2) at 53 bits brackets the true root.
    let b = ok("sqrt", pt(2, 53)?.sqrt(53, &strict))?;
    want_contains("sqrt2 lo", &b, &num(2).sqrt(256, RoundingMode::Down))?;
    want_contains("sqrt2 hi", &b, &num(2).sqrt(256, RoundingMode::Up))?;
    // (1 ± 2): lo = -1 < 0: Strict is Domain; Clip encloses sqrt over [0, 3].
    want_domain("strict 1±2", ball(num(1), mag(TOP, 2)?, 53)?.sqrt(53, &strict))?;
    let c = ok("clip 1±2", ball(num(1), mag(TOP, 2)?, 53)?.sqrt(53, &clip))?;
    want_contains("clip 0", &c, &num(0))?;
    want_contains("clip sqrt3", &c, &num(3).sqrt(256, RoundingMode::Up))?;
    // (1 ± 1): lo = 0 exactly, inside the closed domain: the endpoint ball [0, sqrt_up(2)] (D15d).
    let e = ok("strict 1±1", ball(num(1), mag(TOP, 1)?, 53)?.sqrt(53, &strict))?;
    want_contains("endpoint 0", &e, &num(0))?;
    want_contains("endpoint sqrt2", &e, &num(2).sqrt(256, RoundingMode::Up))?;
    if !e.is_informative() {
        return Err("endpoint ball must be finite".into());
    }
    // (-3 ± 1): misses [0, inf): Domain under both. (-1 ± 1): touches 0 only: Strict Domain, Clip exactly 0.
    want_domain("strict -3±1", ball(num(-3), mag(TOP, 1)?, 53)?.sqrt(53, &strict))?;
    want_domain("clip -3±1", ball(num(-3), mag(TOP, 1)?, 53)?.sqrt(53, &clip))?;
    want_domain("strict -1±1", ball(num(-1), mag(TOP, 1)?, 53)?.sqrt(53, &strict))?;
    want_exact("clip -1±1", &ok("clip -1±1", ball(num(-1), mag(TOP, 1)?, 53)?.sqrt(53, &clip))?, &num(0), &Mag::zero())?;
    // (0 ± 2^-10): Strict Domain, Clip [0, 2^-5].
    want_domain("strict 0±r", ball(num(0), mag(TOP, -9)?, 53)?.sqrt(53, &strict))?;
    let z = ok("clip 0±r", ball(num(0), mag(TOP, -9)?, 53)?.sqrt(53, &clip))?;
    want_contains("clip 0±r top", &z, &pow2(-5)?)?;
    // the all-reals ball: Strict Domain, Clip all reals (10.2a).
    want_domain("strict all", ok("all", Ball::all_reals(53))?.sqrt(53, &strict))?;
    if ok("clip all", ok("all", Ball::all_reals(53))?.sqrt(53, &clip))?.is_informative() {
        return Err("sqrt Clip of all reals must be all reals".into());
    }
    // a radius below zf's range on a positive mid: lo > 0, decided through the Mag bounds.
    let t = ok("tiny r", ball(num(4), mag(TOP, -(1 << 40))?, 53)?.sqrt(53, &strict))?;
    want_contains("tiny r", &t, &num(2))
}

#[test]
fn sqrt_lo_zero_is_the_endpoint_ball() -> Result<(), String> {
    // 4 ± 4 has lo == 0: Strict accepts it, and the result is the endpoint ball [0, sqrt_up(8)] (16.S1.7), whose top
    // stays below 3. The lo > 0 formula would give 2 ± 2, reaching 4.
    let b = ok("sqrt", ball(num(4), mag(TOP, 3)?, 53)?.sqrt(53, &DomainPolicy::Strict))?;
    want_contains("sqrt lo 0", &b, &num(0))?;
    let top = Mag::from_exact_num(b.mid()).add_up(b.rad());
    if top >= Mag::from_exact_num(&num(3)) {
        return Err(format!("sqrt(4 ± 4) reaches {:?}, want below 3: {:?}", top, b));
    }
    Ok(())
}

#[test]
fn prec_out_of_range_is_invalid_argument() -> Result<(), String> {
    let checks = [
        pt(1, 53)?.add(&pt(1, 53)?, 0),
        pt(1, 53)?.sub(&pt(1, 53)?, 0),
        pt(1, 53)?.mul(&pt(1, 53)?, 0),
        pt(1, 53)?.mul_exact(&num(1), 0),
        pt(1, 53)?.sqr(0),
        pt(1, 53)?.powi(0, 0),
        pt(1, 53)?.sqrt(0, &DomainPolicy::Strict),
        pt(1, 53)?.add(&pt(1, 53)?, zenith_bound::MAX_PRECISION.checked_add(1).ok_or("max+1")?),
    ];
    for (i, r) in checks.into_iter().enumerate() {
        match r {
            Err(BoundError::InvalidArgument { .. }) => {}
            other => return Err(format!("case {}: want InvalidArgument, got {:?}", i, other)),
        }
    }
    Ok(())
}

#[test]
fn overflow_is_all_reals() -> Result<(), String> {
    // 2^(EMAX-1) + 2^(EMAX-1) = 2^EMAX overflows zf; so do its product and square, and powi of 2^(EMAX/2+1).
    let big = || -> Result<Ball, String> { ok("big", Ball::exact(pow2(i64::from(EXPONENT_MAX) - 1)?, 53)) };
    let results = [
        big()?.add(&big()?, 53),
        big()?.sub(&-big()?, 53),
        big()?.mul(&big()?, 53),
        big()?.mul_exact(&num(4), 53),
        big()?.sqr(53),
        ok("half", Ball::exact(pow2(i64::from(EXPONENT_MAX / 2) + 1)?, 53))?.powi(2, 53),
    ];
    for (i, r) in results.into_iter().enumerate() {
        let b = ok("overflow", r)?;
        if b.is_informative() {
            return Err(format!("case {}: want all reals, got {:?}", i, b));
        }
    }
    Ok(())
}

#[test]
fn underflow_is_exact_or_enclosed() -> Result<(), String> {
    // 2^(2k) for k near EMIN/2 lies in or below zf's bottom binade. zf keeps a shallow one exactly (EMIN with leading
    // zero bits: rad 0 is then right, and the midpoint must be exactly 2^(2k)); a deeper one is lost, and then the radius
    // must be positive and the ball must reach from 0 up past its midpoint (16.S1.2: bottom ε). Never a silent flush.
    let half = i64::from(EXPONENT_MIN / 2);
    for k in [half - 2, half - 40, half - 200, half - 5000] {
        let x = || -> Result<Ball, String> { ok("tiny", Ball::exact(pow2(k)?, 53)) };
        for (what, r) in [("mul", x()?.mul(&x()?, 53)), ("sqr", x()?.sqr(53))] {
            let what = format!("{} k {}", what, k);
            let b = ok(&what, r)?;
            if b.rad().is_zero() {
                let mut exact = pow2(k)?;
                exact.set_inexact(false);
                let sq = exact.mul_full_prec(&exact);
                if sq.inexact_accumulated() || sq.is_zero() || b.mid().abs_cmp(&sq) != Some(0) || b.mid().is_negative() {
                    return Err(format!("{}: rad 0 but the midpoint is not exactly 2^(2k): {:?}", what, b));
                }
            } else {
                want_contains(&what, &b, &num(0))?;
                want_contains(&what, &b, b.mid())?;
            }
        }
    }
    Ok(())
}

#[test]
fn huge_and_infinite_radius() -> Result<(), String> {
    // a radius far beyond zf's range survives add (no overflow of the midpoint) and contains 1e300-scale points.
    let h = ball(num(1), mag(TOP, 1 << 40)?, 53)?;
    let b = ok("huge add", h.add(&pt(1, 53)?, 53))?;
    if !b.is_informative() || b.rad() < &mag(TOP, 1 << 40)? {
        return Err(format!("huge add: {:?}", b));
    }
    want_contains("huge add", &b, &pow2(997)?)?;
    // an all-reals operand gives all reals.
    for r in [
        ok("all", Ball::all_reals(53))?.add(&pt(1, 53)?, 53),
        pt(1, 53)?.mul(&ok("all", Ball::all_reals(53))?, 53),
        ok("all", Ball::all_reals(53))?.sqr(53),
        ok("all", Ball::all_reals(53))?.powi(3, 53),
    ] {
        if ok("all", r)?.is_informative() {
            return Err("all-reals operand must give all reals".into());
        }
    }
    Ok(())
}

#[test]
fn zero_radius_inexact_mid_gets_eps() -> Result<(), String> {
    // 1/3-like: (2^60 + 1) · 3 at 53 bits is inexact, so rad > 0 even with exact inputs (CONTRACT 2.2).
    let x = num(1).add_full_prec(&pow2(60)?);
    let b = ok("mul", ok("x", Ball::exact(x, 53))?.mul(&pt(3, 53)?, 53))?;
    if b.rad().is_zero() {
        return Err("inexact product with rad 0".into());
    }
    want_contains("inexact product", &b, &num(3).add_full_prec(&dy(3, 60)?))
}

// ---- containment property tests ----------------------------------------------------------------------------------

/// A random midpoint near `2^e`: a 64-bit top chunk plus a 64-bit chunk `k` bits further down (so wide mids round at
/// high precisions too), random sign.
fn rand_mid(rng: &mut SplitMix64, e: i64, prec: usize) -> Result<ExactNum, String> {
    let span = if prec > 1000 { 20_000 } else { 200 };
    let k = 64i64.checked_add(i64::try_from(rng.below(span)).map_err(|_| "span")?).ok_or("k")?;
    let hi = ExactNum::from_mag_parts(rng.next_u64() | TOP, e, 64).map_err(|e| format!("{:?}", e))?;
    let lo = ExactNum::from_mag_parts(rng.next_u64(), e.checked_sub(k).ok_or("e-k")?, 64).map_err(|e| format!("{:?}", e))?;
    let m = hi.add_full_prec(&lo);
    Ok(if rng.below(2) == 1 { -m } else { m })
}

/// A random radius: zero, tiny, comparable, or larger than the midpoint.
fn rand_rad(rng: &mut SplitMix64, e: i64) -> Result<Mag, String> {
    let off = match rng.below(4) {
        0 => return Ok(Mag::zero()),
        1 => -300 - i64::try_from(rng.below(300)).map_err(|_| "off")?,
        2 => -i64::try_from(rng.below(60)).map_err(|_| "off")?,
        _ => i64::try_from(rng.below(8)).map_err(|_| "off")?,
    };
    mag(rng.next_u64() | TOP, e.checked_add(off).ok_or("rad exp")?)
}

fn rand_ball(rng: &mut SplitMix64, prec: usize) -> Result<Ball, String> {
    let i = usize::try_from(rng.below(13)).map_err(|_| "i")?;
    let e = EXPS[i];
    let m = rand_mid(rng, e, prec)?;
    let r = rand_rad(rng, e)?;
    ball(m, r, prec)
}

/// A random ball whose midpoint exponent lies within 200 of `e` (add/sub operands, see run_binary).
fn near_ball(rng: &mut SplitMix64, e: i64, prec: usize) -> Result<Ball, String> {
    let off = i64::try_from(rng.below(401)).map_err(|_| "off")?.checked_sub(200).ok_or("off")?;
    let e = e.checked_add(off).ok_or("e")?;
    let m = rand_mid(rng, e, prec)?;
    let r = rand_rad(rng, e)?;
    ball(m, r, prec)
}

/// Exact sample points of a ball: m, m - r, m + r, m + r/2 (only m when r is outside zf's range).
fn points(b: &Ball) -> Result<Vec<ExactNum>, String> {
    let m = b.mid();
    let mut v = vec![m.add_full_prec(&num(0))];
    if let Ok(r) = b.rad().to_exact_num() {
        v.push(m.add_full_prec(&-r.add_full_prec(&num(0))));
        v.push(m.add_full_prec(&r));
        v.push(m.add_full_prec(&r.mul_full_prec(&pow2(-1)?)));
    }
    Ok(v)
}

fn check(what: &str, res: &Ball, x: &ExactNum) -> Result<(), String> {
    if x.is_nan() || x.is_inf() {
        return Ok(()); // the exact image left zf's range; nothing to compare
    }
    want_contains(what, res, x)
}

#[derive(Debug)]
enum Op {
    Add,
    Sub,
    Mul,
    MulExact,
    Sqr,
    Powi,
}

fn run_binary(op: &Op, seed: u64, cases: usize) -> Result<(), String> {
    let mut rng = SplitMix64::new(seed);
    for case in 0..cases {
        let prec = PRECS[usize::try_from(rng.below(9)).map_err(|_| "p")?];
        let a = rand_ball(&mut rng, prec)?;
        // add/sub: b's exponent stays within 200 of a's. A gap of 2^29 made the exact oracle sum a 2^29-bit number
        // (minutes per case) without testing anything new: past prec + 64 bits the smaller operand only feeds ε.
        let b = match op {
            Op::Add | Op::Sub => near_ball(&mut rng, a.mid().exponent().map_or(0, i64::from), prec)?,
            Op::Mul | Op::MulExact | Op::Sqr | Op::Powi => rand_ball(&mut rng, prec)?,
        };
        let (pa, pb) = (points(&a)?, points(&b)?);
        let n = rng.below(7).checked_add(1).ok_or("n")?;
        let what = format!("{:?} case {} prec {} a {:?} b {:?} n {}", op, case, prec, a, b, n);
        let bm = b.mid().add_full_prec(&num(0));
        let res = ok(&what, match op {
            Op::Add => a.add(&b, prec),
            Op::Sub => a.sub(&b, prec),
            Op::Mul => a.mul(&b, prec),
            Op::MulExact => a.mul_exact(&bm, prec),
            Op::Sqr => a.sqr(prec),
            Op::Powi => a.powi(n, prec),
        })?;
        for x in &pa {
            match op {
                Op::Add | Op::Sub | Op::Mul => {
                    for y in &pb {
                        let img = match op {
                            Op::Add => x.add_full_prec(y),
                            Op::Sub => x.add_full_prec(&-y.add_full_prec(&num(0))),
                            Op::Mul | Op::MulExact | Op::Sqr | Op::Powi => x.mul_full_prec(y),
                        };
                        check(&what, &res, &img)?;
                    }
                }
                Op::MulExact => check(&what, &res, &x.mul_full_prec(&bm))?,
                Op::Sqr => check(&what, &res, &x.mul_full_prec(x))?,
                Op::Powi => {
                    let mut p = num(1);
                    for _ in 0..n {
                        p = p.mul_full_prec(x);
                    }
                    check(&what, &res, &p)?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn containment_add() -> Result<(), String> {
    run_binary(&Op::Add, 0x5101, 300)
}

#[test]
fn containment_sub() -> Result<(), String> {
    run_binary(&Op::Sub, 0x5102, 300)
}

#[test]
fn containment_mul() -> Result<(), String> {
    run_binary(&Op::Mul, 0x5103, 300)
}

#[test]
fn containment_mul_exact() -> Result<(), String> {
    run_binary(&Op::MulExact, 0x5104, 300)
}

#[test]
fn containment_sqr() -> Result<(), String> {
    run_binary(&Op::Sqr, 0x5105, 300)
}

#[test]
fn containment_powi() -> Result<(), String> {
    run_binary(&Op::Powi, 0x5106, 200)
}

#[test]
fn containment_neg() -> Result<(), String> {
    let mut rng = SplitMix64::new(0x5107);
    for case in 0..200 {
        let prec = PRECS[usize::try_from(rng.below(9)).map_err(|_| "p")?];
        let a = rand_ball(&mut rng, prec)?;
        let pa = points(&a)?;
        let res = -a;
        for x in &pa {
            check(&format!("neg case {}", case), &res, &-x.add_full_prec(&num(0)))?;
        }
    }
    Ok(())
}

#[test]
fn containment_sqrt() -> Result<(), String> {
    let mut rng = SplitMix64::new(0x5108);
    for case in 0..300 {
        let prec = PRECS[usize::try_from(rng.below(9)).map_err(|_| "p")?];
        let a = rand_ball(&mut rng, prec)?;
        let pa = points(&a)?;
        let lo_neg = match a.rad().to_exact_num() {
            Ok(r) => a.mid().add_full_prec(&-r).is_negative(),
            Err(_) => a.mid().is_negative() || a.rad() > &Mag::from_exact_num(a.mid()),
        };
        let strict_pick = rng.below(2) == 1;
        let policy = if strict_pick { DomainPolicy::Strict } else { DomainPolicy::Clip };
        let what = format!("sqrt case {} prec {} {:?} strict {}", case, prec, a, strict_pick);
        let mid_was_zero = a.mid().is_zero();
        match a.sqrt(prec, &policy) {
            Err(BoundError::Domain { .. }) => {
                // Strict refuses iff m - r < 0; Clip only when no point is >= 0.
                if !lo_neg || (!strict_pick && pa.iter().any(|x| !x.is_negative() || x.is_zero()) && !mid_was_zero) {
                    return Err(format!("{}: unexpected Domain", what));
                }
            }
            Err(e) => return Err(format!("{}: {:?}", what, e)),
            Ok(res) => {
                if strict_pick && lo_neg {
                    return Err(format!("{}: Strict accepted m - r < 0", what));
                }
                for x in pa.iter().filter(|x| !x.is_negative() || x.is_zero()) {
                    // The oracle brackets sqrt(x) 128 bits finer than BOTH the input and the target precision (case 69: a
                    // 3264-bit x at prec 16384 was bracketed at only 3392 bits, far coarser than the ball's half ulp).
                    let p = x.precision().unwrap_or(64).max(prec).checked_add(128).ok_or("p")?;
                    check(&what, &res, &x.sqrt(p, RoundingMode::Down))?;
                    check(&what, &res, &x.sqrt(p, RoundingMode::Up))?;
                }
            }
        }
    }
    Ok(())
}

// ---- the L0b enclosure rows (tests/goldens/ball_enclosure_rows.txt) ------------------------------------------------

/// A row value `SIGN EXP MANT` (mantissa hex, odd): `(-1)^SIGN * 0.MANT * 2^EXP`.
fn row_val(s: &str, e: &str, m: &str) -> Result<ExactNum, String> {
    if m == "0" {
        return Ok(num(0));
    }
    let mut x = num(0);
    let digits = m.as_bytes();
    for chunk in digits.chunks(15) {
        let text = core::str::from_utf8(chunk).map_err(|e| e.to_string())?;
        let v = u64::from_str_radix(text, 16).map_err(|e| format!("{}: {}", m, e))?;
        let shift = i64::try_from(chunk.len()).map_err(|_| "len")?.checked_mul(4).ok_or("shift")?;
        x = x.mul_full_prec(&pow2(shift)?).add_full_prec(&ExactNum::from(v));
    }
    let exp: i32 = e.parse().map_err(|_| format!("exp {}", e))?;
    x.set_exponent(exp);
    Ok(if s == "1" { -x } else { x })
}

#[test]
fn enclosure_rows_contained() -> Result<(), String> {
    let path = golden_dir().join("ball_enclosure_rows.txt");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{:?}: {}", path, e))?;
    let mut checked = 0usize;
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let f: Vec<&str> = line.split_whitespace().collect();
        let op = f.first().copied().ok_or("empty row")?;
        let prec: usize = f.get(1).ok_or("prec")?.parse().map_err(|_| "prec")?;
        let v = |i: usize| -> Result<ExactNum, String> {
            match (f.get(i), f.get(i + 1), f.get(i + 2)) {
                (Some(s), Some(e), Some(m)) => row_val(s, e, m),
                _ => Err(format!("short row: {}", line)),
            }
        };
        let rad = |i: usize| -> Result<Mag, String> { Ok(Mag::from_exact_num(&v(i)?)) };
        let (res, lo_at) = match op {
            "sqrt" => (ball(v(3)?, rad(6)?, prec)?.sqrt(prec, &DomainPolicy::Clip), 9),
            "add" | "sub" | "mul" => {
                let a = ball(v(3)?, rad(6)?, prec)?;
                let b = ball(v(9)?, rad(12)?, prec)?;
                let r = match op {
                    "add" => a.add(&b, prec),
                    "sub" => a.sub(&b, prec),
                    _ => a.mul(&b, prec),
                };
                (r, 15)
            }
            _ => continue, // div: pending zf R13 (CONTRACT 16.S1.8)
        };
        let res = ok(line, res)?;
        want_contains(line, &res, &v(lo_at)?)?;
        want_contains(line, &res, &v(lo_at + 3)?)?;
        checked += 1;
    }
    if checked != 948 {
        return Err(format!("checked {} rows, want 948 (237 each of add, sub, mul, sqrt)", checked));
    }
    Ok(())
}
