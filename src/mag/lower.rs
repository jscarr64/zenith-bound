//! Round-down products.

use super::mul::mul_parts;
use super::Mag;

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
}
