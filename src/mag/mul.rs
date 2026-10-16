//! `mul_up` and the shared product used by `mul_lower`.

use super::round::loose;
use super::round::norm;
use super::Mag;

impl Mag {
    /// Least `Mag` at or above `self * other` (CONTRACT 1.3, 14.1).
    ///
    /// An `Infinity` operand yields `Infinity`, including zero times `Infinity`
    /// (CONTRACT 2.4, 14.6). A zero operand with a finite partner yields zero
    /// (CONTRACT 2.3). The 128-bit product is normalised to 64 bits; a nonzero
    /// residue rounds up. Exponent underflow rounds up to the smallest positive
    /// `Mag` (CONTRACT 3.2). Exponent overflow is `Infinity` (CONTRACT 3.3).
    pub fn mul_up(&self, other: &Mag) -> Mag {
        if self.is_infinity() || other.is_infinity() {
            Mag::infinity()
        } else {
            match (self.nonzero(), other.nonzero()) {
                (Some((ea, ma)), Some((eb, mb))) => mul_parts(ea, ma, eb, mb, true),
                _ => Mag::zero(),
            }
        }
    }
}

pub(super) fn mul_parts(ea: i64, ma: u64, eb: i64, mb: u64, up: bool) -> Mag {
    let prod = match u128::from(ma).checked_mul(u128::from(mb)) {
        Some(prod) => prod,
        None => return loose(up),
    };
    let scale = match i128::from(ea)
        .checked_add(i128::from(eb))
        .and_then(|sum| sum.checked_sub(128))
    {
        Some(scale) => scale,
        None => return loose(up),
    };
    norm(prod, scale, false, up)
}
