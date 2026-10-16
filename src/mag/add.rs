//! `add_up`: the least `Mag` at or above the sum of two magnitudes.

use super::round::loose;
use super::round::norm_u128;
use super::round::settle;
use super::Mag;
use super::TOP;
use core::num::NonZeroU64;

impl Mag {
    /// Least `Mag` at or above `self + other` (CONTRACT 14.1, 14.3).
    ///
    /// An `Infinity` operand yields `Infinity` (CONTRACT 14.6). A zero operand yields the
    /// other value, before any alignment (CONTRACT 2.3); the result is built from that
    /// operand's integer parts by `Mag::from_canonical`, as for every other result. The
    /// operands are aligned to the larger exponent; bits shifted out become sticky, and a
    /// sticky sum rounds up one unit in the last place. When the exponent gap is at least 64
    /// and the smaller operand is nonzero, the result is the larger operand plus one unit.
    pub fn add_up(&self, other: &Mag) -> Mag {
        if self.is_infinity() || other.is_infinity() {
            Mag::infinity()
        } else {
            match (self.nonzero(), other.nonzero()) {
                (Some((ea, ma)), Some((eb, mb))) => add_parts(ea, ma, eb, mb),
                (Some((e, m)), None) | (None, Some((e, m))) => Mag::from_canonical(e, m.get()),
                (None, None) => Mag::zero(),
            }
        }
    }
}

fn add_parts(ea: i64, ma: NonZeroU64, eb: i64, mb: NonZeroU64) -> Mag {
    let (ea, ma, eb, mb) = if ea >= eb {
        (ea, ma, eb, mb)
    } else {
        (eb, mb, ea, ma)
    };
    let gap = match i128::from(ea).checked_sub(i128::from(eb)) {
        Some(gap) => gap,
        None => return Mag::infinity(),
    };
    if gap >= 64 {
        return successor(ea, ma.get());
    }
    let gap_u = match u32::try_from(gap) {
        Ok(gap) => gap,
        Err(_) => return Mag::infinity(),
    };
    let shifted = match u128::from(ma.get()).checked_shl(gap_u) {
        Some(shifted) => shifted,
        None => return Mag::infinity(),
    };
    let sum = match shifted.checked_add(u128::from(mb.get())) {
        Some(sum) => sum,
        None => return Mag::infinity(),
    };
    let scale = match i128::from(eb).checked_sub(64) {
        Some(scale) => scale,
        None => return loose(true),
    };
    settle(norm_u128(sum, scale, false, true), true)
}

/// The next `Mag` strictly above the canonical finite value `(exp, mant)`.
fn successor(exp: i64, mant: u64) -> Mag {
    if mant == u64::MAX {
        match exp.checked_add(1) {
            Some(exp) => Mag::from_canonical(exp, TOP),
            None => Mag::infinity(),
        }
    } else {
        match mant.checked_add(1) {
            Some(mant) => Mag::from_canonical(exp, mant),
            None => Mag::infinity(),
        }
    }
}
