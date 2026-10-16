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

mod cmp;

use crate::error::BoundError;

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

    /// Placeholder until `add_up` is implemented: the canonical zero.
    fn wrong_up() -> Mag {
        Mag::zero()
    }

    /// Placeholder until the round-down operations are implemented: `Infinity`.
    fn wrong_down() -> Mag {
        Mag::infinity()
    }

    /// Placeholder: zero for every input.
    pub fn add_up(&self, _other: &Mag) -> Mag {
        Mag::wrong_up()
    }

    /// Placeholder: zero for every input.
    pub fn mul_up(&self, _other: &Mag) -> Mag {
        Mag::wrong_up()
    }

    /// Placeholder: zero for every input.
    pub fn div_up(&self, _other: &Mag) -> Mag {
        Mag::wrong_up()
    }

    /// Placeholder: zero for every input.
    pub fn sqrt_up(&self) -> Mag {
        Mag::wrong_up()
    }

    /// Placeholder: `Infinity` for every input.
    pub fn sub_lower(&self, _other: &Mag) -> Mag {
        Mag::wrong_down()
    }

    /// Placeholder: `Infinity` for every input.
    pub fn mul_lower(&self, _other: &Mag) -> Mag {
        Mag::wrong_down()
    }

    /// Placeholder: `Infinity` for every input.
    pub fn div_lower(&self, _other: &Mag) -> Mag {
        Mag::wrong_down()
    }

    /// Placeholder: `Infinity` for every input.
    pub fn sqrt_lower(&self) -> Mag {
        Mag::wrong_down()
    }

    /// Placeholder: zero.
    pub fn ulp_mag(_pre_round_exp: i64, _prec: u32) -> Mag {
        Mag::wrong_up()
    }

    /// Placeholder: zero.
    pub fn half_ulp(_mid_exp: i64, _prec: u32) -> Mag {
        Mag::wrong_up()
    }

    /// Placeholder: always false.
    pub fn smaller_than_half_ulp(&self, _mid_exp: i64, _prec: u32) -> bool {
        false
    }
}
