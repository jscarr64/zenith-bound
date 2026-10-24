//! Slice CB harness rows (`src/cball.rs`). Owned by slice CB alone.
//! CONTRACT 16.CB.1 to 16.CB.11. Written red, against S1's borrowing ops (`&self, &Ball`) and the `CBall` API of 16.CB.1;
//! this file does not compile until S1's borrow change is merged and `src/cball.rs` exists.
//! Oracles: hand-verified exact rows (worked in the comment beside each) and containment of exact images computed with
//! zenith-float's `add_full_prec`/`sub_full_prec`/`mul_full_prec` at sample points (corners and midpoints) of the inputs.
//! The mpmath.mpc golden rows and the RREF tightness factors of 16.CB.10 come with scripts/gen_cball_goldens.py.
//! COVERS: CBall::new, CBall::from_real, CBall::all_complex, CBall::neg, CBall::conj, CBall::add, CBall::sub, CBall::mul, CBall::sqr, CBall::abs, CBall::div

use zenith_bound::ball::Ball;
use zenith_bound::cball::CBall;
use zenith_bound::enclosure::Enclosure;
use zenith_bound::error::BoundError;
use zenith_bound::mag::Mag;
use zenith_bound::tri::Tri;
use zenith_float_num::ExactNum;

const TOP: u64 = 0x8000_0000_0000_0000;
const PRECS: [usize; 7] = [1, 2, 53, 64, 113, 256, 1024];

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

fn ok<T>(what: &str, r: Result<T, BoundError>) -> Result<T, String> {
    r.map_err(|e| format!("{}: unexpected {:?}", what, e))
}

fn ball(v: i64, rad: Mag, prec: usize) -> Result<Ball, String> {
    ok("from_mid_rad", Ball::from_mid_rad(num(v), rad, prec))
}

fn pt(v: i64, prec: usize) -> Result<Ball, String> {
    ok("exact", Ball::exact(num(v), prec))
}

/// The exact point `re + i·im`.
fn cpt(re: i64, im: i64, prec: usize) -> Result<CBall, String> {
    ok("CBall::new", CBall::new(pt(re, prec)?, pt(im, prec)?))
}

/// `(re ± 2^-k) + i(im ± 2^-k)`; Mag 2^-k is finite(TOP, 1 - k).
fn cbox(re: i64, im: i64, k: i64, prec: usize) -> Result<CBall, String> {
    let r = 1i64.checked_sub(k).ok_or("cbox k")?;
    ok("CBall::new", CBall::new(ball(re, mag(TOP, r)?, prec)?, ball(im, mag(TOP, r)?, prec)?))
}

fn want_point(what: &str, z: &CBall, re: i64, im: i64) -> Result<(), String> {
    if z.re().mid() != &num(re) || z.im().mid() != &num(im) || !z.re().rad().is_zero() || !z.im().rad().is_zero() {
        return Err(format!("{}: got {:?}, want exact {} + {}i", what, z, re, im));
    }
    Ok(())
}

fn want_contains(what: &str, b: &Ball, x: &ExactNum) -> Result<(), String> {
    match b.contains(x) {
        Tri::True => Ok(()),
        t @ (Tri::False | Tri::Unknown) => Err(format!("{}: {:?} does not contain {:?} ({:?})", what, b, x, t)),
    }
}

fn want_all_reals_both(what: &str, z: &CBall) -> Result<(), String> {
    if z.re().is_informative() || z.im().is_informative() || !z.re().rad().is_infinity() || !z.im().rad().is_infinity() {
        return Err(format!("{}: want both parts all reals, got {:?}", what, z));
    }
    Ok(())
}

fn want_finite_both(what: &str, z: &CBall) -> Result<(), String> {
    if !z.re().is_informative() || !z.im().is_informative() {
        return Err(format!("{}: want both parts finite, got {:?}", what, z));
    }
    Ok(())
}

/// Sample points (corners and midpoint) of `m ± 2^-k` for integer `m`.
fn samples(m: i64, k: i64) -> Result<[ExactNum; 3], String> {
    let r = pow2(-k)?;
    let c = num(m);
    Ok([c.sub_full_prec(&r), num(m), c.add_full_prec(&r)])
}

// ---- 16.CB.1 construction and invariants ----------------------------------------------------------------------

#[test]
fn cball_construct_invariants() -> Result<(), String> {
    // I2: parts of different prec are InvalidArgument.
    match CBall::new(pt(1, 53)?, pt(2, 64)?) {
        Err(BoundError::InvalidArgument { .. }) => {}
        other => return Err(format!("prec mismatch: want InvalidArgument, got {:?}", other)),
    }
    let z = ok("new", CBall::new(pt(3, 53)?, pt(-4, 53)?))?;
    if z.prec() != 53 {
        return Err(format!("prec: got {}", z.prec()));
    }
    want_point("new", &z, 3, -4)?;
    want_point("from_real", &ok("from_real", CBall::from_real(pt(7, 53)?))?, 7, 0)?;
    want_all_reals_both("all_complex", &ok("all_complex", CBall::all_complex(53))?)?;
    // I3: an all-reals part leaves the other informative.
    let h = ok("new", CBall::new(ok("all_reals", Ball::all_reals(53))?, pt(1, 53)?))?;
    if h.re().is_informative() || !h.im().is_informative() {
        return Err(format!("I3: got {:?}", h));
    }
    if !z.identical(&ok("new", CBall::new(pt(3, 53)?, pt(-4, 53)?))?) || z.identical(&h) {
        return Err("identical".into());
    }
    Ok(())
}

// ---- 16.CB.3 neg, conj, add, sub -----------------------------------------------------------------------------

#[test]
fn cball_add_sub_neg_conj_rows() -> Result<(), String> {
    let a = cpt(3, 5, 53)?;
    let b = cpt(-1, 2, 53)?;
    want_point("add", &ok("add", a.add(&b, 53))?, 2, 7)?; // (3+5i)+(-1+2i) = 2+7i
    want_point("sub", &ok("sub", a.sub(&b, 53))?, 4, 3)?; // (3+5i)-(-1+2i) = 4+3i
    want_point("neg", &a.neg(), -3, -5)?;
    want_point("conj", &a.conj(), 3, -5)?;
    if !a.conj().conj().identical(&a) {
        return Err("conj∘conj".into());
    }
    // conj keeps radii: (1 ± 2^-4) + (2 ± 2^-4)i → im radius unchanged, mid negated.
    let c = cbox(1, 2, 4, 53)?;
    let cc = c.conj();
    if cc.im().mid() != &num(-2) || cc.im().rad() != c.im().rad() || !cc.re().identical(c.re()) {
        return Err(format!("conj radii: {:?}", cc));
    }
    // containment on samples of a box sum.
    let s = ok("add", c.add(&cbox(-3, 5, 6, 53)?, 53))?;
    for x in samples(1, 4)?.iter() {
        for y in samples(-3, 6)?.iter() {
            want_contains("add re", s.re(), &x.add_full_prec(y))?;
        }
    }
    Ok(())
}

// ---- 16.CB.4 mul ---------------------------------------------------------------------------------------------

#[test]
fn cball_mul_rows() -> Result<(), String> {
    // (1+2i)(3+4i) = (3-8) + (4+6)i = -5+10i.
    want_point("mul", &ok("mul", cpt(1, 2, 53)?.mul(&cpt(3, 4, 53)?, 53))?, -5, 10)?;
    // cancellation: (1+i)(1-i) = 2 + 0i.
    want_point("mul cancel", &ok("mul", cpt(1, 1, 53)?.mul(&cpt(1, -1, 53)?, 53))?, 2, 0)?;
    // i·i = -1.
    want_point("i*i", &ok("mul", cpt(0, 1, 53)?.mul(&cpt(0, 1, 53)?, 53))?, -1, 0)?;
    // boxes, parts containing 0 and mixed signs: containment at every sample corner pair.
    for &prec in PRECS.iter() {
        let (ar, ai, br, bi) = (0i64, -3i64, 5i64, 0i64);
        let p = ok("mul", cbox(ar, ai, 2, prec)?.mul(&cbox(br, bi, 3, prec)?, prec))?;
        for x in samples(ar, 2)?.iter() {
            for y in samples(ai, 2)?.iter() {
                for u in samples(br, 3)?.iter() {
                    for v in samples(bi, 3)?.iter() {
                        let re = x.mul_full_prec(u).sub_full_prec(&y.mul_full_prec(v));
                        let im = x.mul_full_prec(v).add_full_prec(&y.mul_full_prec(u));
                        want_contains("mul re", p.re(), &re)?;
                        want_contains("mul im", p.im(), &im)?;
                    }
                }
            }
        }
    }
    Ok(())
}

// ---- 16.CB.5 sqr ---------------------------------------------------------------------------------------------

#[test]
fn cball_sqr_rows() -> Result<(), String> {
    // (3+4i)^2 = 9-16 + 24i = -7+24i.
    want_point("sqr", &ok("sqr", cpt(3, 4, 53)?.sqr(53))?, -7, 24)
}

#[test]
fn cball_sqr_not_wider_than_mul() -> Result<(), String> {
    // Every row: each part of sqr(a) is no wider than the same part of mul(a, a) (16.CB.5).
    for &prec in PRECS.iter() {
        for &(re, im, k) in [(0i64, 0i64, 1i64), (0, 3, 2), (1, -1, 4), (-7, 2, 10), (5, 0, 0)].iter() {
            let a = cbox(re, im, k, prec)?;
            let s = ok("sqr", a.sqr(prec))?;
            let m = ok("mul", a.mul(&a, prec))?;
            if s.re().rad() > m.re().rad() || s.im().rad() > m.im().rad() {
                return Err(format!("sqr wider than mul at prec {} for {:?}: {:?} vs {:?}", prec, a, s, m));
            }
            for x in samples(re, k)?.iter() {
                for y in samples(im, k)?.iter() {
                    want_contains("sqr re", s.re(), &x.mul_full_prec(x).sub_full_prec(&y.mul_full_prec(y)))?;
                    let xy = x.mul_full_prec(y);
                    want_contains("sqr im", s.im(), &xy.add_full_prec(&xy))?;
                }
            }
        }
    }
    Ok(())
}

// ---- 16.CB.6 abs ---------------------------------------------------------------------------------------------

#[test]
fn cball_abs_rows() -> Result<(), String> {
    // |3+4i| = 5 exactly (hypot of exact points is exact when the result fits).
    let r = ok("abs", cpt(3, 4, 53)?.abs(53))?;
    want_contains("|3+4i|", &r, &num(5))?;
    // lower endpoint ≥ 0: a box around 0 gives a ball whose mid ≥ rad.
    let z = ok("abs", cbox(0, 0, 3, 53)?.abs(53))?;
    if let Tri::True = z.lt(&ok("exact", Ball::exact(num(0), 53))?) {
        return Err(format!("abs below 0: {:?}", z));
    }
    want_contains("|0|", &z, &num(0))?;
    // axis: |x + 0i| contains |x|.
    want_contains("|-6|", &ok("abs", cpt(-6, 0, 53)?.abs(53))?, &num(6))
}

#[test]
fn cball_abs_i64_extremes() -> Result<(), String> {
    // 16.CB.6: CB uses S1 hypot as is. If this fails, it is a red test for S1's hypot range, not a CB fix.
    // (3·2^K) + (4·2^K)i has |z| = 5·2^K, representable at every K below.
    for &k in [1i64 << 40, -(1i64 << 40), (1i64 << 62) - (1i64 << 10), -((1i64 << 62) - (1i64 << 10))].iter() {
        let s = pow2(k)?;
        let re = ok("exact", Ball::exact(num(3).mul_full_prec(&s), 64))?;
        let im = ok("exact", Ball::exact(num(4).mul_full_prec(&s), 64))?;
        let r = ok("abs", ok("new", CBall::new(re, im))?.abs(64))?;
        if !r.is_informative() {
            return Err(format!("abs at 2^{}: not informative: {:?}", k, r));
        }
        want_contains("abs extreme", &r, &num(5).mul_full_prec(&s))?;
    }
    Ok(())
}

// ---- 16.CB.7 div ---------------------------------------------------------------------------------------------

#[test]
fn cball_div_rows() -> Result<(), String> {
    // (2+4i)/(1+i) = (2+4i)(1-i)/2 = (2+4 + (4-2)i)/2 = 3+i.
    let q = ok("div", cpt(2, 4, 53)?.div(&cpt(1, 1, 53)?, 53))?;
    want_contains("div re", q.re(), &num(3))?;
    want_contains("div im", q.im(), &num(1))?;
    // by an exact real: (6-9i)/3 = 2-3i.
    let q = ok("div", cpt(6, -9, 53)?.div(&cpt(3, 0, 53)?, 53))?;
    want_contains("div real re", q.re(), &num(2))?;
    want_contains("div real im", q.im(), &num(-3))?;
    // one axis of the divisor touches 0, the other is clear: finite, not all reals.
    let q = ok("div", cpt(1, 0, 53)?.div(&cbox(0, 4, 1, 53)?, 53))?;
    want_finite_both("one axis touches 0", &q)
}

#[test]
fn cball_div_rectangle_containing_zero_is_all_reals() -> Result<(), String> {
    // R1: both parts reach 0 → Ok with both parts all reals (never Domain).
    let exact_zero = cpt(0, 0, 53)?;
    want_all_reals_both("0/0", &ok("div", cpt(1, 1, 53)?.div(&exact_zero, 53))?)?;
    want_all_reals_both("box at 0", &ok("div", cpt(1, 1, 53)?.div(&cbox(0, 0, 3, 53)?, 53))?)?;
    // edge |m| = r on both axes: 1 ± 1 touches 0 exactly.
    let edge = ok("new", CBall::new(ball(1, mag(TOP, 1)?, 53)?, ball(-1, mag(TOP, 1)?, 53)?))?;
    want_all_reals_both("edge", &ok("div", cpt(1, 1, 53)?.div(&edge, 53))?)?;
    // just clear on one axis: 2 ± 1 excludes 0, so R1 does not hold.
    let clear = ok("new", CBall::new(ball(2, mag(TOP, 1)?, 53)?, ball(-1, mag(TOP, 1)?, 53)?))?;
    want_finite_both("clear on re", &ok("div", cpt(1, 1, 53)?.div(&clear, 53))?)
}

#[test]
fn cball_div_uncertified_all_reals_clears_at_higher_prec() -> Result<(), String> {
    // R2 (16.CB.7): R1 does not hold, but |b|^2 is not certified > 0 at this prec. The exact |b|^2 here is
    // [2^-82, 9·2^-82], clear of 0, so R2 can only come from the width of S1's sqr/add enclosure of d, not from b.
    // OPEN (for Spence): if S1's sqr is endpoint-tight this row never reaches 0 at any prec, and R2 is then unreachable
    // for rectangles; the row is kept red until that is settled against S1's borrowing sqr.
    // Strict R2 row: b = (2^-40 ± 2^-41) + 0i: re excludes 0 (|m| > r), d = [2^-82, 9·2^-82]; at prec 1 the Ball sqr
    // of 2^-40 ± 2^-41 rounds its mid and the radius reaches 0 → all reals; at prec 1024 d is certified → finite.
    let b_lo = ok("new", CBall::new(ok("from_mid_rad", Ball::from_mid_rad(pow2(-40)?, mag(TOP, -40)?, 1))?, pt(0, 1)?))?;
    let lo = ok("div", cpt(1, 0, 1)?.div(&b_lo, 1))?;
    let b_hi = ok("new", CBall::new(ok("from_mid_rad", Ball::from_mid_rad(pow2(-40)?, mag(TOP, -40)?, 1024))?, pt(0, 1024)?))?;
    let hi = ok("div", cpt(1, 0, 1024)?.div(&b_hi, 1024))?;
    want_all_reals_both("R2 low prec", &lo)?;
    want_finite_both("R2 high prec", &hi)
}

#[test]
fn cball_div_scaling_exact() -> Result<(), String> {
    // The common 2^-k on both operands leaves the quotient unchanged: (2+4i)·2^K / ((1+i)·2^K) contains 3+i at
    // K = ±2^40, where |b|^2 unscaled would leave the midpoint range.
    for &k in [1i64 << 40, -(1i64 << 40)].iter() {
        let s = pow2(k)?;
        let mk = |v: i64| ok("exact", Ball::exact(num(v).mul_full_prec(&s), 64));
        let a = ok("new", CBall::new(mk(2)?, mk(4)?))?;
        let b = ok("new", CBall::new(mk(1)?, mk(1)?))?;
        let q = ok("div", a.div(&b, 64))?;
        want_finite_both("scaled div", &q)?;
        want_contains("scaled re", q.re(), &num(3))?;
        want_contains("scaled im", q.im(), &num(1))?;
    }
    Ok(())
}

// ---- 16.CB.9 all-reals propagation and precision ----------------------------------------------------------------

#[test]
fn cball_all_reals_propagation() -> Result<(), String> {
    let h = ok("new", CBall::new(ok("all_reals", Ball::all_reals(53))?, pt(1, 53)?))?;
    // add keeps I3: im stays finite.
    let s = ok("add", h.add(&cpt(1, 1, 53)?, 53))?;
    if s.re().is_informative() || !s.im().is_informative() {
        return Err(format!("add I3: {:?}", s));
    }
    // mul: both output parts use the all-reals re part.
    want_all_reals_both("mul", &ok("mul", h.mul(&cpt(1, 1, 53)?, 53))?)
}

#[test]
fn cball_prec_sweep() -> Result<(), String> {
    // Both parts carry the op's prec; a 3-bit-or-wider midpoint rounds to prec with the error in rad.
    for &prec in PRECS.iter() {
        let a = cbox(7, -5, 3, 1024)?;
        let b = cbox(3, 2, 5, 1024)?;
        for z in [ok("add", a.add(&b, prec))?, ok("mul", a.mul(&b, prec))?, ok("sqr", a.sqr(prec))?, ok("div", a.div(&b, prec))?].iter() {
            if z.prec() != prec || z.re().prec() != prec || z.im().prec() != prec {
                return Err(format!("prec {}: got {:?}", prec, z));
            }
        }
    }
    match cpt(1, 1, 53)?.add(&cpt(1, 1, 53)?, 0) {
        Err(BoundError::InvalidArgument { .. }) => Ok(()),
        other => Err(format!("prec 0: want InvalidArgument, got {:?}", other)),
    }
}
