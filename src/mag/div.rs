//! `div_up` and `div_lower` on a 128/64 division with a sticky remainder.

use super::round::loose;
use super::round::norm_u128;
use super::round::settle;
use super::Mag;

impl Mag {
    /// Least `Mag` at or above `self / other` (CONTRACT 14.1, 14.4).
    ///
    /// `Infinity` is checked before a zero divisor, and both are checked before the
    /// zero-numerator shortcut, so `0 / 0` is `Infinity`. Any `Infinity` operand or a
    /// zero divisor yields `Infinity`. A zero numerator with a finite nonzero divisor
    /// yields zero. Exponent underflow rounds up to the smallest positive `Mag`;
    /// exponent overflow is `Infinity`.
    pub fn div_up(&self, other: &Mag) -> Mag {
        if self.is_infinity() || other.is_infinity() || other.is_zero() {
            Mag::infinity()
        } else {
            div_parts(self, other, true)
        }
    }

    /// Greatest `Mag` at or below `self / other` (CONTRACT 14.2, 14.4).
    ///
    /// An `Infinity` operand or a zero divisor yields zero. A zero numerator yields
    /// zero. Exponent underflow yields zero. Exponent overflow yields the largest
    /// finite `Mag`.
    pub fn div_lower(&self, other: &Mag) -> Mag {
        if self.is_infinity() || other.is_infinity() || other.is_zero() {
            Mag::zero()
        } else {
            div_parts(self, other, false)
        }
    }
}

fn div_parts(num: &Mag, den: &Mag, up: bool) -> Mag {
    let Some((ea, ma)) = num.nonzero() else {
        return Mag::zero();
    };
    let Some((eb, mb)) = den.nonzero() else {
        return loose(up);
    };
    let numerator = match u128::from(ma.get()).checked_shl(64) {
        Some(numerator) => numerator,
        None => return loose(up),
    };
    let divisor = u128::from(mb.get());
    let quot = match numerator.checked_div(divisor) {
        Some(quot) => quot,
        None => return loose(up),
    };
    let sticky = match quot.checked_mul(divisor) {
        Some(prod) => match numerator.checked_sub(prod) {
            Some(rem) => rem != 0,
            None => true,
        },
        None => true,
    };
    let scale = match i128::from(ea)
        .checked_sub(i128::from(eb))
        .and_then(|diff| diff.checked_sub(64))
    {
        Some(scale) => scale,
        None => return loose(up),
    };
    settle(norm_u128(quot, scale, sticky, up), up)
}
