//! Slice S1: Ball core arithmetic: add, sub, neg, mul, mul by an exact scalar, sqr, powi and sqrt on `Ball`, with the
//! radius rules of CONTRACT clauses 2, 3, 9 and 10 and the slice clauses in CONTRACT section 16 "Slice S1".
//!
//! Owner: slice S1 (Spence). Only that slice edits this file.
//! Every midpoint is the zenith-float operation at the target `prec` rounded `ToEven` (O19); the radius is the propagated
//! radius plus the midpoint rounding error ε (9.4, 16.S1.2), all in `Mag` rounding upward. A midpoint overflow gives the
//! all-reals ball (10.4, O2). div, inv and a directed hypot are pending zenith-float R13 (16.S1.8).

use core::cmp::Ordering;
use core::ops::Neg;

use zenith_float_num::{Error, ExactNum, RoundingMode, EXPONENT_MAX, EXPONENT_MIN};

use crate::ball::Ball;
use crate::error::{BoundError, BoundResult};
use crate::mag::Mag;
use crate::policy::DomainPolicy;
use crate::precision::{Precision, MAX_PRECISION, MIN_PRECISION};

/// Every Ball midpoint rounds to nearest, ties to even (CONTRACT 15.1, O19).
const MID_RM: RoundingMode = RoundingMode::ToEven;
/// Extra bits `powi` carries through its squaring chain before the one final rounding to `prec` (16.S1.6).
const POWI_GUARD_BITS: Precision = 64;
/// `Mag` mantissa of one half: `0.1b * 2^0`.
const HALF_MANT: u64 = 0x8000_0000_0000_0000;

/// A classified zenith-float midpoint result (CONTRACT 10.4).
enum Mid {
    Value(ExactNum),
    Overflow,
}

impl Neg for Ball {
    type Output = Ball;

    /// `-(m ± r) = (-m) ± r`: exact and infallible; a −0 midpoint becomes +0 (CONTRACT 6.3, 15.1, 16.S1.3).
    fn neg(self) -> Ball {
        self.negated()
    }
}

impl Ball {
    /// `(m ± r) + (n ± s) = round(m + n) ± (r + s + ε)` (CONTRACT 9.3, 16.S1.1).
    ///
    /// # Errors
    /// `InvalidArgument` when `prec` is outside `MIN_PRECISION..=MAX_PRECISION`. A midpoint overflow is `Ok(all reals)`.
    pub fn add(self, other: &Ball, prec: Precision) -> BoundResult<Ball> {
        check_prec("Ball::add", prec)?;
        let (m, r, _) = self.into_parts();
        let rad = r.add_up(other.rad());
        finish(
            "Ball::add",
            m.add(other.mid(), prec, MID_RM),
            rad,
            prec,
            false,
        )
    }

    /// `(m ± r) - (n ± s) = round(m - n) ± (r + s + ε)` (CONTRACT 9.3, 16.S1.1).
    ///
    /// # Errors
    /// `InvalidArgument` when `prec` is outside `MIN_PRECISION..=MAX_PRECISION`. A midpoint overflow is `Ok(all reals)`.
    pub fn sub(self, other: &Ball, prec: Precision) -> BoundResult<Ball> {
        check_prec("Ball::sub", prec)?;
        let (m, r, _) = self.into_parts();
        let rad = r.add_up(other.rad());
        finish(
            "Ball::sub",
            m.sub(other.mid(), prec, MID_RM),
            rad,
            prec,
            false,
        )
    }

    /// `(m ± r)(n ± s) = round(m n) ± (|m| s + |n| r + r s + ε)` (CONTRACT 9.3, 16.S1.1).
    ///
    /// # Errors
    /// `InvalidArgument` when `prec` is outside `MIN_PRECISION..=MAX_PRECISION`. A midpoint overflow is `Ok(all reals)`.
    pub fn mul(self, other: &Ball, prec: Precision) -> BoundResult<Ball> {
        check_prec("Ball::mul", prec)?;
        let (m, r, _) = self.into_parts();
        let (n, s) = (other.mid(), other.rad());
        let rad = Mag::from_exact_num(&m)
            .mul_up(s)
            .add_up(&Mag::from_exact_num(n).mul_up(&r))
            .add_up(&r.mul_up(s));
        finish("Ball::mul", m.mul(n, prec, MID_RM), rad, prec, false)
    }

    /// `(m ± r) · c = round(m c) ± (|c| r + ε)` for an exact scalar `c` (16.S1.4).
    ///
    /// # Errors
    /// `InvalidArgument` for a bad `prec`; `Representation` when `c` is NaN or infinite. A midpoint overflow is
    /// `Ok(all reals)`.
    pub fn mul_exact(self, c: &ExactNum, prec: Precision) -> BoundResult<Ball> {
        check_prec("Ball::mul_exact", prec)?;
        if c.is_nan() || c.is_inf() {
            return Err(BoundError::Representation {
                op: "Ball::mul_exact",
                detail: "NaN or infinite scalar",
            });
        }
        let (m, r, _) = self.into_parts();
        let rad = Mag::from_exact_num(c).mul_up(&r);
        finish("Ball::mul_exact", m.mul(c, prec, MID_RM), rad, prec, false)
    }

    /// `(m ± r)^2 = round(m^2) ± (2|m| r + r^2 + ε)` (16.S1.5): tighter than `mul` with itself, which would also
    /// bound the same set but needs a second copy of the ball.
    ///
    /// # Errors
    /// `InvalidArgument` when `prec` is outside `MIN_PRECISION..=MAX_PRECISION`. A midpoint overflow is `Ok(all reals)`.
    pub fn sqr(self, prec: Precision) -> BoundResult<Ball> {
        check_prec("Ball::sqr", prec)?;
        let (mut m, r, _) = self.into_parts();
        let t = Mag::from_exact_num(&m).mul_up(&r);
        let rad = t.add_up(&t).add_up(&r.mul_up(&r));
        // The exact square, then one rounding to `prec`. The flag is cleared first so the accumulated flag of the square
        // reports only a loss inside the square itself (an exponent underflow), which forces ε (16.S1.5).
        m.set_inexact(false);
        let sq = m.mul_full_prec(&m);
        let lost = sq.inexact_accumulated();
        finish("Ball::sqr", round_to(&sq, prec), rad, prec, lost)
    }

    /// `(m ± r)^n` by left-to-right binary exponentiation with this module's `sqr` and `mul` at `prec + 64` bits and
    /// one final rounding to `prec`; each step is a rigorous enclosure, so the chain is (16.S1.6). `n == 0` is exactly
    /// `1 ± 0` for every ball, including one containing 0 (CONTRACT 10.2e).
    ///
    /// # Errors
    /// `InvalidArgument` when `prec` is outside `MIN_PRECISION..=MAX_PRECISION`. A midpoint overflow is `Ok(all reals)`.
    pub fn powi(self, n: u64, prec: Precision) -> BoundResult<Ball> {
        check_prec("Ball::powi", prec)?;
        if n == 0 {
            return Ball::exact(ExactNum::from(1i64), prec);
        }
        let wp = match prec.checked_add(POWI_GUARD_BITS) {
            Some(w) if w <= MAX_PRECISION => w,
            Some(_) | None => prec,
        };
        let mut acc = Ball::exact(ExactNum::from(1i64), wp)?;
        let mut started = false;
        for i in (0..u64::BITS).rev() {
            let bit = n.checked_shr(i).map_or(0, |v| v & 1) == 1;
            if started {
                acc = acc.sqr(wp)?;
            }
            if bit {
                acc = acc.mul(&self, wp)?;
                started = true;
            }
            if !acc.is_informative() {
                return Ball::all_reals(prec);
            }
        }
        let (m, r, _) = acc.into_parts();
        finish("Ball::powi", round_to(&m, prec), r, prec, false)
    }

    /// `sqrt(m ± r)` on the closed domain `[0, inf)` (CONTRACT 10.2b, D15d, 16.S1.7). With `lo = m - r` and
    /// `hi = m + r`, decided exactly: `lo > 0` gives `round(sqrt m) ± (r / sqrt(m) + ε)`; `lo == 0` gives the endpoint ball
    /// `[0, sqrt_up(hi)]`; `lo < 0` is `Domain` under `Strict`. Under `Clip`, `hi < 0` is `Domain`, `hi == 0` is exactly
    /// `0`, and `hi > 0` gives the endpoint ball. An all-reals input is `Domain` (`Strict`) or all reals (`Clip`, 10.2a).
    ///
    /// # Errors
    /// `InvalidArgument` for a bad `prec`; `Domain` as above; `Resource` if the sign of `m - r` cannot be decided.
    pub fn sqrt(self, prec: Precision, policy: &DomainPolicy) -> BoundResult<Ball> {
        const OP: &str = "Ball::sqrt";
        check_prec(OP, prec)?;
        let strict = match policy {
            DomainPolicy::Strict => true,
            DomainPolicy::Clip => false,
        };
        let (m, r, _) = self.into_parts();
        if r.is_infinity() {
            return if strict {
                Err(domain(OP, "the all-reals ball reaches below 0"))
            } else {
                Ball::all_reals(prec)
            };
        }
        let Some(ord) = cmp_abs_rad(&m, &r) else {
            return Err(BoundError::Resource {
                op: OP,
                detail: "cannot decide the sign of m - r",
            });
        };
        let neg = m.is_negative() && !m.is_zero();
        let pos = !m.is_negative() && !m.is_zero();
        // (sign of lo, sign of hi) as Ordering against 0.
        let (lo, hi) = match (neg, pos, ord) {
            (false, true, Ordering::Greater) => (Ordering::Greater, Ordering::Greater),
            (false, true, Ordering::Equal) => (Ordering::Equal, Ordering::Greater),
            (true, false, Ordering::Greater) => (Ordering::Less, Ordering::Less),
            (true, false, Ordering::Equal) => (Ordering::Less, Ordering::Equal),
            (false, false, Ordering::Equal | Ordering::Greater) => {
                (Ordering::Equal, Ordering::Equal)
            }
            (_, _, Ordering::Less) | (true, true, _) => (Ordering::Less, Ordering::Greater),
        };
        match (lo, strict, hi) {
            (Ordering::Greater, _, _) => {
                let root_lower = Mag::from_exact_num_lower(&m).sqrt_lower();
                let rad = r.div_up(&root_lower);
                finish(OP, m.sqrt(prec, MID_RM), rad, prec, false)
            }
            (Ordering::Equal, _, _) | (Ordering::Less, false, Ordering::Greater) => {
                let hi_up = if neg {
                    r
                } else {
                    Mag::from_exact_num(&m).add_up(&r)
                };
                endpoint_ball(OP, hi_up.sqrt_up(), prec)
            }
            (Ordering::Less, true, _) => Err(domain(OP, "m - r < 0 under Strict")),
            (Ordering::Less, false, Ordering::Less) => {
                Err(domain(OP, "m + r < 0: the ball misses [0, inf)"))
            }
            (Ordering::Less, false, Ordering::Equal) => Ball::exact(ExactNum::from(0i64), prec),
        }
    }
}

/// The ball `[0, u]` as `round(u / 2) ± (u / 2 + ε)`; when `u` is outside zenith-float's range, `0 ± u` (16.S1.7).
fn endpoint_ball(op: &'static str, u: Mag, prec: Precision) -> BoundResult<Ball> {
    let half = Mag::finite(HALF_MANT, 0)?;
    let u_half = u.mul_up(&half);
    match (u.to_exact_num(), ExactNum::from_mag_parts(HALF_MANT, 0, 64)) {
        (Ok(un), Ok(h)) => finish(op, un.mul(&h, prec, MID_RM), u_half, prec, false),
        (Err(_), _) | (_, Err(_)) => Ball::from_mid_rad(ExactNum::from(0i64), u, prec),
    }
}

/// `|m|` against `r`, decided exactly; `None` only if neither the exact path nor the `Mag` bounds decide it.
fn cmp_abs_rad(m: &ExactNum, r: &Mag) -> Option<Ordering> {
    match r.to_exact_num() {
        Ok(rn) => m.abs_cmp(&rn).map(|c| c.cmp(&0)),
        // r is outside zenith-float's exponent range, so it differs from every zenith-float value; bound |m| by Mag.
        Err(_) => {
            if *r < Mag::from_exact_num_lower(m) {
                Some(Ordering::Greater)
            } else if *r > Mag::from_exact_num(m) {
                Some(Ordering::Less)
            } else {
                None
            }
        }
    }
}

/// `x` rounded once to `prec`, `ToEven`; `inexact()` reports this rounding only.
fn round_to(x: &ExactNum, prec: Precision) -> ExactNum {
    ExactNum::from(1i64).mul(x, prec, MID_RM)
}

fn domain(op: &'static str, detail: &'static str) -> BoundError {
    BoundError::Domain { op, detail }
}

fn check_prec(op: &'static str, prec: Precision) -> BoundResult<()> {
    if (MIN_PRECISION..=MAX_PRECISION).contains(&prec) {
        Ok(())
    } else {
        Err(BoundError::InvalidArgument {
            op,
            detail: "prec outside MIN_PRECISION..=MAX_PRECISION",
        })
    }
}

/// CONTRACT 10.4 / O2: overflow detection and the NaN error-kind mapping.
fn classify(op: &'static str, raw: ExactNum) -> BoundResult<Mid> {
    if raw.is_inf() {
        return Ok(Mid::Overflow);
    }
    if raw.is_nan() {
        return match raw.err() {
            Some(Error::ExponentOverflow(_)) => Ok(Mid::Overflow),
            Some(Error::DivisionByZero) => Err(domain(op, "zf division by zero")),
            Some(Error::InvalidArgument) => Err(BoundError::InvalidArgument {
                op,
                detail: "zf invalid argument",
            }),
            Some(Error::MemoryAllocation) => Err(BoundError::Resource {
                op,
                detail: "zf memory allocation",
            }),
            Some(Error::PrecisionRetryExhausted) => Err(BoundError::Resource {
                op,
                detail: "zf precision retry exhausted",
            }),
            None => Err(BoundError::Representation {
                op,
                detail: "zf NaN without an error kind",
            }),
        };
    }
    if raw.inexact() && raw.exponent() == Some(EXPONENT_MAX) {
        return Ok(Mid::Overflow);
    }
    Ok(Mid::Value(raw))
}

/// ε of a rounded midpoint (CONTRACT 9.4, O19, 16.S1.2): zero when exact; `half_ulp(e, prec)` for a result with exponent
/// `e > EXPONENT_MIN`; and `2^EXPONENT_MIN` for a zero or bottom-binade (subnormal) result, which bounds any nearest
/// rounding there (both the exact value and the result lie in `[-2^EXPONENT_MIN, 2^EXPONENT_MIN]` with one sign).
fn eps(mid: &ExactNum, prec: Precision, lost: bool) -> Mag {
    if !mid.inexact() && !lost {
        return Mag::zero();
    }
    match mid.exponent() {
        Some(e) if e > EXPONENT_MIN && !mid.is_zero() => Mag::half_ulp(i64::from(e), prec),
        Some(_) | None => Mag::ulp_mag(i64::from(EXPONENT_MIN), 0),
    }
}

/// Classify the midpoint, add ε to the propagated radius and build the ball.
fn finish(
    op: &'static str,
    raw: ExactNum,
    prop: Mag,
    prec: Precision,
    lost: bool,
) -> BoundResult<Ball> {
    match classify(op, raw)? {
        Mid::Overflow => Ball::all_reals(prec),
        Mid::Value(m) => {
            let rad = prop.add_up(&eps(&m, prec, lost));
            Ball::from_mid_rad(m, rad, prec)
        }
    }
}
