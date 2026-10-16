//! Red stub for the integer-only `Mag` (L0): every operation is total and deliberately wrong.
//! The fields are private; values are built only through `Mag::finite`, `Mag::zero` and `Mag::infinity`.
use crate::error::BoundError;
use core::cmp::Ordering;

#[derive(Debug)]
enum Inner {
    Finite { exp: i64, mant: u64 },
    Infinity,
}

/// An upper bound on a nonnegative real (`Infinity`: no finite bound).
#[derive(Debug)]
pub struct Mag {
    inner: Inner,
}

impl Mag {
    /// Wrong on purpose: the canonical zero.
    fn wrong_up() -> Mag {
        Mag::zero()
    }

    /// Wrong on purpose: `Infinity`.
    fn wrong_down() -> Mag {
        Mag::infinity()
    }

    /// Stub: always the canonical zero, whatever the input (the real constructor normalises or rejects).
    pub fn finite(_mant: u64, _exp: i64) -> Result<Mag, BoundError> {
        Ok(Mag::zero())
    }

    /// The canonical zero (`exp == 0`, `mant == 0`).
    pub fn zero() -> Mag {
        Mag {
            inner: Inner::Finite { exp: 0, mant: 0 },
        }
    }

    /// The infinite bound.
    pub fn infinity() -> Mag {
        Mag {
            inner: Inner::Infinity,
        }
    }

    /// The exponent of a finite value, `None` for `Infinity`.
    pub fn exp(&self) -> Option<i64> {
        match &self.inner {
            Inner::Finite { exp, .. } => Some(*exp),
            Inner::Infinity => None,
        }
    }

    /// The mantissa of a finite value, `None` for `Infinity`.
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

    /// Stub: rounds up to zero.
    pub fn add_up(&self, _other: &Mag) -> Mag {
        Mag::wrong_up()
    }

    /// Stub: rounds up to zero.
    pub fn mul_up(&self, _other: &Mag) -> Mag {
        Mag::wrong_up()
    }

    /// Stub: rounds up to zero.
    pub fn div_up(&self, _other: &Mag) -> Mag {
        Mag::wrong_up()
    }

    /// Stub: rounds up to zero.
    pub fn sqrt_up(&self) -> Mag {
        Mag::wrong_up()
    }

    /// Stub: rounds down to `Infinity`.
    pub fn sub_lower(&self, _other: &Mag) -> Mag {
        Mag::wrong_down()
    }

    /// Stub: rounds down to `Infinity`.
    pub fn mul_lower(&self, _other: &Mag) -> Mag {
        Mag::wrong_down()
    }

    /// Stub: rounds down to `Infinity`.
    pub fn div_lower(&self, _other: &Mag) -> Mag {
        Mag::wrong_down()
    }

    /// Stub: rounds down to `Infinity`.
    pub fn sqrt_lower(&self) -> Mag {
        Mag::wrong_down()
    }

    /// Stub: zero.
    pub fn ulp_mag(_pre_round_exp: i64, _prec: u32) -> Mag {
        Mag::wrong_up()
    }

    /// Stub: zero.
    pub fn half_ulp(_mid_exp: i64, _prec: u32) -> Mag {
        Mag::wrong_up()
    }

    /// Stub: always false.
    pub fn smaller_than_half_ulp(&self, _mid_exp: i64, _prec: u32) -> bool {
        false
    }
}

impl PartialEq for Mag {
    fn eq(&self, other: &Mag) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Mag {}

impl PartialOrd for Mag {
    fn partial_cmp(&self, other: &Mag) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Mag {
    /// Stub: always `Less` (violates reflexivity and antisymmetry).
    fn cmp(&self, _other: &Mag) -> Ordering {
        Ordering::Less
    }
}
