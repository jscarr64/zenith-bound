//! Unit in the last place, half an ulp, and the half-ulp predicate.
//!
//! `half_ulp` is a sound rounding-error term only for nearest modes. Directed modes
//! need a full ulp from [`Mag::ulp_mag`].

use super::Mag;
use super::TOP;

impl Mag {
    /// `2^(pre_round_exp - prec)`, placed as a canonical power of two (CONTRACT 14.7).
    ///
    /// The exponent field is computed in `i128` as `pre_round_exp - prec + 1` and then
    /// placed. `prec` is the full `u32` value. An exponent field above `i64::MAX`
    /// (only `ulp_mag(i64::MAX, 0)`) is `Infinity`. An exponent field below `i64::MIN`
    /// rounds up to the smallest positive `Mag` (CONTRACT 3.2).
    pub fn ulp_mag(pre_round_exp: i64, prec: u32) -> Mag {
        let mag_exp = match i128::from(pre_round_exp).checked_sub(i128::from(prec)) {
            Some(diff) => match diff.checked_add(1) {
                Some(exp) => exp,
                None => return Mag::infinity(),
            },
            None => return Mag::min_pos(),
        };
        place_pow2(mag_exp)
    }

    /// `2^(mid_exp - prec - 1)`, placed as a canonical power of two (CONTRACT 14.7).
    ///
    /// At `prec == 0` this is `2^(mid_exp - 1)`. The exponent field is `mid_exp - prec`,
    /// computed in `i128` over the full `u32` precision. Underflow rounds up to the
    /// smallest positive `Mag`; overflow is `Infinity`.
    ///
    /// This value is the right midpoint-rounding term only under a nearest mode. A
    /// directed mode needs the full ulp from [`Mag::ulp_mag`].
    pub fn half_ulp(mid_exp: i64, prec: u32) -> Mag {
        match i128::from(mid_exp).checked_sub(i128::from(prec)) {
            Some(mag_exp) => place_pow2(mag_exp),
            None => Mag::min_pos(),
        }
    }

    /// Whether `self` is strictly below half an ulp of a midpoint at `mid_exp` and `prec`.
    ///
    /// For a finite nonzero `Mag` this is `exp <= mid_exp - prec - 1`, evaluated in
    /// `i128` so a threshold below `i64::MIN` is not rounded up (CONTRACT 1.4, 14.7).
    /// The canonical zero is below every half ulp. `Infinity` is not.
    pub fn smaller_than_half_ulp(&self, mid_exp: i64, prec: u32) -> bool {
        if self.is_zero() {
            return true;
        }
        if self.is_infinity() {
            return false;
        }
        let Some(exp) = self.exp() else {
            return false;
        };
        let threshold = match i128::from(mid_exp).checked_sub(i128::from(prec)) {
            Some(diff) => match diff.checked_sub(1) {
                Some(threshold) => threshold,
                None => return false,
            },
            None => return false,
        };
        i128::from(exp) <= threshold
    }
}

/// `2^(mag_exp - 1)` as `Finite { exp: mag_exp, mant: 2^63 }`, or the range clamp.
fn place_pow2(mag_exp: i128) -> Mag {
    match i64::try_from(mag_exp) {
        Ok(exp) => Mag::from_canonical(exp, TOP),
        Err(_) => {
            if mag_exp > i128::from(i64::MAX) {
                Mag::infinity()
            } else {
                Mag::min_pos()
            }
        }
    }
}
