//! Slice S2: The `Mag` to `ExactNum` bridge: exact comparisons of a `Mag` radius against zenith-float values that never
//! build the radius as an `ExactNum` unless it lies within a factor 2 of the value (CONTRACT 4.2, 10.2c, 15.2, 16.S2),
//! and (pending zenith-float R14, 16.S2.6) the remaining conversions.
//!
//! Owner: slice S2 (Spence). Only that slice edits this file.

use core::cmp::Ordering;

use zenith_float_num::ExactNum;

use crate::mag::Mag;

/// Operands whose exponents differ by at most this are subtracted exactly; wider gaps are decided by exponents first
/// (16.S2.3). Any value of at least 66 is correct (the wide-gap cutoffs need `|small| < 2^(e - 65)`); it only trades
/// exact-subtraction cost for case analysis.
const EXACT_GAP: i64 = 128;

/// The sign of `x` as an ordering against zero (`Less` negative, `Equal` zero, `Greater` positive).
fn sign(x: &ExactNum) -> Ordering {
    if x.is_zero() {
        Ordering::Equal
    } else if x.is_negative() {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

/// `s` if `keep`, otherwise its opposite.
fn signed(s: Ordering, keep: bool) -> Ordering {
    if keep {
        s
    } else {
        s.reverse()
    }
}

fn not_finite(x: &ExactNum) -> bool {
    x.is_nan() || x.is_inf()
}

/// The exponent `e` of `x` (`|x|` in `[2^(e-1), 2^e)`), as i64; `None` when `x` is zero or not finite.
fn exp_of(x: &ExactNum) -> Option<i64> {
    if x.is_zero() {
        return None;
    }
    x.exponent().map(i64::from)
}

impl Mag {
    /// `self` against `|x|`, decided exactly from `x`'s top 64 mantissa bits, its sticky bit and both i64 exponents,
    /// so a radius outside zenith-float's exponent range still compares (16.S2.1). `None` when `x` is NaN or infinite.
    pub fn cmp_exact_num(&self, x: &ExactNum) -> Option<Ordering> {
        if not_finite(x) {
            return None;
        }
        if self.is_infinity() {
            return Some(Ordering::Greater);
        }
        if x.is_zero() {
            return Some(if self.is_zero() {
                Ordering::Equal
            } else {
                Ordering::Greater
            });
        }
        let (me, mm) = match (self.exp(), self.mant()) {
            (Some(e), Some(m)) => (e, m),
            (None, _) | (_, None) => return Some(Ordering::Less),
        };
        let (xm, sticky, xe) = x.top_u64_mantissa_and_sticky()?;
        let tail = if sticky {
            Ordering::Less
        } else {
            Ordering::Equal
        };
        Some(me.cmp(&xe).then(mm.cmp(&xm)).then(tail))
    }

    /// `self` against `|a - b|`, decided exactly (16.S2.2). Operands within [`EXACT_GAP`] binades are subtracted
    /// exactly; otherwise the exponents decide unless the radius is within a factor 2 of the larger operand, and then
    /// only the exact `|big| - self` and one short exact sum are formed (16.S2.3). `None` when an operand is NaN or
    /// infinite, or when zenith-float reports an exact operation inexact.
    pub fn cmp_exact_diff(&self, a: &ExactNum, b: &ExactNum) -> Option<Ordering> {
        if not_finite(a) || not_finite(b) {
            return None;
        }
        if self.is_infinity() {
            return Some(Ordering::Greater);
        }
        let (ea, eb) = match (exp_of(a), exp_of(b)) {
            (Some(ea), Some(eb)) => (ea, eb),
            (None, _) => return self.cmp_exact_num(b),
            (_, None) => return self.cmp_exact_num(a),
        };
        let gap = if ea >= eb {
            ea.checked_sub(eb)?
        } else {
            eb.checked_sub(ea)?
        };
        if gap <= EXACT_GAP {
            let d = a.sub_full_prec(b);
            if d.inexact() {
                return None;
            }
            return self.cmp_exact_num(&d);
        }
        self.cmp_far_diff(a, b, ea >= eb)
    }

    /// The wide-gap case of [`Mag::cmp_exact_diff`]: `a_big` says whether `a` is the operand of larger magnitude.
    fn cmp_far_diff(&self, a: &ExactNum, b: &ExactNum, a_big: bool) -> Option<Ordering> {
        let (big, small) = if a_big { (a, b) } else { (b, a) };
        let e = exp_of(big)?;
        let es = exp_of(small)?;
        // |a - b| lies in (2^(e-1) - 2^(e-65), 2^e + 2^(e-65)) since |small| < 2^es <= 2^(e - EXACT_GAP); a radius below
        // 2^(e-1) has mant <= u64::MAX so it is at most 2^(e-1) - 2^(e-65).
        let me = match self.exp() {
            Some(me) => me,
            None => return Some(Ordering::Less),
        };
        if me <= e.checked_sub(1)? {
            return Some(Ordering::Less);
        }
        if me >= e.checked_add(2)? {
            return Some(Ordering::Greater);
        }
        let m = self.to_exact_num().ok()?;
        // a - b = tb + ts with tb = ±big, ts = ±small; |a - b| = sb (a - b) where sb is the sign of tb.
        let big_neg = big.is_negative();
        let tb_pos = big_neg != a_big;
        // u = tb - sb m, kept as u = (u_pos ? +1 : -1) * ur with ur formed from big and m.
        let (ur, u_keep) = if big_neg {
            (big.add_full_prec(&m), a_big)
        } else {
            (big.sub_full_prec(&m), a_big)
        };
        if ur.inexact() {
            return None;
        }
        // Sign of ts: small enters with + when it is a, with - when it is b.
        let ts_pos = small.is_negative() == a_big;
        let ts_sign = if ts_pos {
            Ordering::Greater
        } else {
            Ordering::Less
        };
        // y = u + ts; sign(|a - b| - self) = sb * sign(y).
        let y_sign = match exp_of(&ur) {
            None => ts_sign,
            Some(eu) => {
                let u_sign = signed(sign(&ur), u_keep);
                if es <= eu.checked_sub(2)? {
                    u_sign
                } else if eu <= es.checked_sub(2)? {
                    ts_sign
                } else {
                    // y = fu ur + fs small with fu = u_keep, fs = (sign of small as ±) == ts_pos.
                    let small_pos = !small.is_negative();
                    let fs_plus = small_pos == ts_pos;
                    let z = if u_keep == fs_plus {
                        ur.add_full_prec(small)
                    } else {
                        ur.sub_full_prec(small)
                    };
                    if z.inexact() {
                        return None;
                    }
                    signed(sign(&z), u_keep)
                }
            }
        };
        Some(signed(y_sign, tb_pos).reverse())
    }
}
