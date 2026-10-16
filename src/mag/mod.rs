//! Integer-only magnitude, an upper bound on a nonnegative real.
//!
//! A finite nonzero value with exponent `exp` and mantissa `mant` (top bit set) denotes
//! `mant * 2^(exp - 64)`, so the value lies in `[2^(exp - 1), 2^exp)` (CONTRACT 1.2).
//! The canonical zero is `exp == 0`, `mant == 0`, and it is the only finite value whose
//! mantissa is below `2^63` (CONTRACT 2.1). `Infinity` means that no finite bound exists.
//!
//! Fields are private. Values are built only with [`Mag::finite`], [`Mag::zero`] and
//! [`Mag::infinity`]. `Mag` is not `Clone`, `Copy` or `Hash`. Ordering is a total order:
//! zero is least, `Infinity` is greatest, and canonical finite values compare by `(exp, mant)`.

mod add;
mod cmp;
mod div;
mod lower;
mod mul;
mod round;
mod sqrt;
mod ulp;

use crate::error::BoundError;

/// Least canonical nonzero mantissa, `2^63`.
const TOP: u64 = 0x8000_0000_0000_0000;

#[derive(Debug)]
enum Inner {
    Finite { exp: i64, mant: u64 },
    Infinity,
}

/// Upper bound on a nonnegative real, or `Infinity` when no finite bound exists.
#[derive(Debug)]
pub struct Mag {
    inner: Inner,
}

impl Mag {
    /// The canonical zero (`exp == 0`, `mant == 0`).
    pub fn zero() -> Mag {
        Mag {
            inner: Inner::Finite { exp: 0, mant: 0 },
        }
    }

    /// The infinite bound: every finite `Mag` is strictly smaller.
    pub fn infinity() -> Mag {
        Mag {
            inner: Inner::Infinity,
        }
    }

    /// Builds the finite magnitude denoted by `mant * 2^(exp - 64)`.
    ///
    /// A mantissa in `1..2^63` is normalised by a left shift of `lz` leading zeros, and the
    /// exponent becomes `exp - lz`. That adjustment is exact. `mant == 0` is accepted only
    /// together with `exp == 0`. If `exp - lz` would be below `i64::MIN`, the value is not
    /// representable and this returns [`BoundError::Representation`] without rounding.
    pub fn finite(mant: u64, exp: i64) -> Result<Mag, BoundError> {
        if mant == 0 {
            if exp == 0 {
                Ok(Mag::zero())
            } else {
                Err(BoundError::Representation)
            }
        } else {
            let shift = mant.leading_zeros();
            let adjusted = i128::from(exp)
                .checked_sub(i128::from(shift))
                .ok_or(BoundError::Representation)?;
            let exp = i64::try_from(adjusted).map_err(|_| BoundError::Representation)?;
            let mant = mant.checked_shl(shift).ok_or(BoundError::Representation)?;
            Ok(Mag::from_canonical(exp, mant))
        }
    }

    /// Exponent of a finite value. `None` for `Infinity`.
    pub fn exp(&self) -> Option<i64> {
        match &self.inner {
            Inner::Finite { exp, .. } => Some(*exp),
            Inner::Infinity => None,
        }
    }

    /// Mantissa of a finite value. `None` for `Infinity`.
    pub fn mant(&self) -> Option<u64> {
        match &self.inner {
            Inner::Finite { mant, .. } => Some(*mant),
            Inner::Infinity => None,
        }
    }

    /// True for the canonical zero.
    pub fn is_zero(&self) -> bool {
        matches!(&self.inner, Inner::Finite { mant: 0, .. })
    }

    /// True for `Infinity`.
    pub fn is_infinity(&self) -> bool {
        matches!(&self.inner, Inner::Infinity)
    }

    fn from_canonical(exp: i64, mant: u64) -> Mag {
        Mag {
            inner: Inner::Finite { exp, mant },
        }
    }

    fn min_pos() -> Mag {
        Mag::from_canonical(i64::MIN, TOP)
    }

    fn max_finite() -> Mag {
        Mag::from_canonical(i64::MAX, u64::MAX)
    }

    /// A new `Mag` with the same exponent and mantissa, or `Infinity`.
    fn replicate(&self) -> Mag {
        match &self.inner {
            Inner::Infinity => Mag::infinity(),
            Inner::Finite { exp, mant } => Mag::from_canonical(*exp, *mant),
        }
    }

    /// `(exp, mant)` when this value is finite and nonzero.
    fn nonzero(&self) -> Option<(i64, u64)> {
        match &self.inner {
            Inner::Finite { exp, mant } if *mant != 0 => Some((*exp, *mant)),
            _ => None,
        }
    }

    /// Placeholder until `sub_lower` is implemented: `Infinity`.
    fn wrong_down() -> Mag {
        Mag::infinity()
    }

    /// Placeholder: `Infinity` for every input.
    pub fn sub_lower(&self, _other: &Mag) -> Mag {
        Mag::wrong_down()
    }
}
