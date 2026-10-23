//! The sealed `Enclosure` trait (PLAN C5, 2026-10-23): what every enclosure type answers.
//!
//! Implemented by [`Ball`]; `ComplexBall` implements it when it lands (L2a). The trait is sealed:
//! no type outside zenith-bound can implement it, so methods may be added in a minor release.

use zenith_float_num::ExactNum;

use crate::ball::Ball;
use crate::tri::Tri;

mod sealed {
    /// Only zenith-bound types implement `Enclosure`.
    pub trait Sealed {}
}

/// A set-valued number with a midpoint precision.
pub trait Enclosure: sealed::Sealed {
    /// The point type this enclosure can contain.
    type Point;

    /// The requested midpoint precision in bits.
    fn prec(&self) -> u32;

    /// Structural identity: same midpoint value, same radius, same `prec` (CONTRACT 6.5 b, O20).
    fn identical(&self, other: &Self) -> bool;

    /// Whether `x` lies in the enclosure: `True`/`False` only when decided, else `Unknown`.
    fn contains(&self, x: &Self::Point) -> Tri;
}

impl sealed::Sealed for Ball {}

impl Enclosure for Ball {
    type Point = ExactNum;

    fn prec(&self) -> u32 {
        Ball::prec(self)
    }

    fn identical(&self, other: &Ball) -> bool {
        let _ = other;
        false
    }

    fn contains(&self, x: &ExactNum) -> Tri {
        let _ = x;
        Tri::Unknown
    }
}
