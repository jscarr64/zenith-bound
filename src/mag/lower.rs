//! Round-down products and differences.

use super::mul::mul_parts;
use super::round::norm_u128;
use super::round::settle;
use super::Mag;
use core::cmp::Ordering;

impl Mag {
    /// Greatest `Mag` at or below `self * other` (CONTRACT 14.2).
    ///
    /// An `Infinity` operand yields zero. A zero operand yields zero. Exponent
    /// underflow yields zero (CONTRACT 3.2). Exponent overflow yields the largest
    /// finite `Mag`.
    pub fn mul_lower(&self, other: &Mag) -> Mag {
        if self.is_infinity() || other.is_infinity() {
            Mag::zero()
        } else {
            match (self.nonzero(), other.nonzero()) {
                (Some((ea, ma)), Some((eb, mb))) => mul_parts(ea, ma, eb, mb, false),
                _ => Mag::zero(),
            }
        }
    }

    /// Greatest `Mag` at or below `self - other` (CONTRACT 14.2).
    ///
    /// The result is zero when `self <= other`, and when either operand is `Infinity`.
    /// Subtracting zero from a finite value gives that value (CONTRACT 2.3), built from its
    /// integer parts by `Mag::from_canonical`. A
    /// positive result whose exponent would fall below `i64::MIN` is zero.
    pub fn sub_lower(&self, other: &Mag) -> Mag {
        if self.is_infinity() || other.is_infinity() {
            return Mag::zero();
        }
        if self.cmp(other) != Ordering::Greater {
            return Mag::zero();
        }
        match (self.nonzero(), other.nonzero()) {
            (Some((ea, ma)), Some((eb, mb))) => sub_parts(ea, ma.get(), eb, mb.get()),
            (Some((e, m)), None) => Mag::from_canonical(e, m.get()),
            _ => Mag::zero(),
        }
    }
}

fn sub_parts(ea: i64, ma: u64, eb: i64, mb: u64) -> Mag {
    let gap = match i128::from(ea).checked_sub(i128::from(eb)) {
        Some(gap) => gap,
        None => return Mag::zero(),
    };
    if gap >= 64 {
        return sub_wide(ea, ma, gap, mb);
    }
    let gap_u = match u32::try_from(gap) {
        Ok(gap) => gap,
        Err(_) => return Mag::zero(),
    };
    let shifted = match u128::from(ma).checked_shl(gap_u) {
        Some(shifted) => shifted,
        None => return Mag::zero(),
    };
    let diff = match shifted.checked_sub(u128::from(mb)) {
        Some(diff) => diff,
        None => return Mag::zero(),
    };
    let scale = match i128::from(eb).checked_sub(64) {
        Some(scale) => scale,
        None => return Mag::zero(),
    };
    settle(norm_u128(diff, scale, false, false), false)
}

/// Difference when `other` sits at least 64 exponents below `self`.
///
/// The smaller value is strictly below one ulp of `self`, so the round-down result is
/// `self` minus a fraction of that ulp. The subtraction is carried at 63 extra bits and
/// then truncated (CONTRACT 14.2).
fn sub_wide(ea: i64, ma: u64, gap: i128, mb: u64) -> Mag {
    let wide = match u128::from(ma).checked_shl(63) {
        Some(wide) => wide,
        None => return Mag::zero(),
    };
    let shift = match gap.checked_sub(63) {
        Some(shift) => shift,
        None => return Mag::zero(),
    };
    let (subtrahend, frac) = if shift >= 128 {
        (0u128, true)
    } else {
        let shift_u = match u32::try_from(shift) {
            Ok(shift) if shift < 128 => shift,
            _ => return Mag::zero(),
        };
        let subtrahend = match u128::from(mb).checked_shr(shift_u) {
            Some(subtrahend) => subtrahend,
            None => return Mag::zero(),
        };
        let mask = match 1u128
            .checked_shl(shift_u)
            .and_then(|bit| bit.checked_sub(1))
        {
            Some(mask) => mask,
            None => return Mag::zero(),
        };
        let frac = (u128::from(mb) & mask) != 0;
        (subtrahend, frac)
    };
    let mut diff = match wide.checked_sub(subtrahend) {
        Some(diff) => diff,
        None => return Mag::zero(),
    };
    if frac {
        diff = match diff.checked_sub(1) {
            Some(diff) => diff,
            None => return Mag::zero(),
        };
    }
    let scale = match i128::from(ea).checked_sub(127) {
        Some(scale) => scale,
        None => return Mag::zero(),
    };
    settle(norm_u128(diff, scale, false, false), false)
}
