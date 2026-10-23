//! The `Mag` ⇄ `ExactNum` bridge (CONTRACT 2.3, 3.3, 3.4; PLAN C5).
//!
//! `ExactNum::top_u64_mantissa_and_sticky` uses the same `0.mant * 2^exp` convention as `Mag`, so its parts are
//! the round-down result, and one ulp more is the round-up result when bits were dropped.

use super::{Mag, TOP};
use crate::error::BoundError;
use zenith_float_num::{Error, ExactNum};

/// zf precision for `to_exact_num`: a `Mag` mantissa has 64 bits, so 64 bits are always exact.
const EXACT_PREC: usize = 64;

impl Mag {
    /// Least `Mag` at or above `|x|` (CONTRACT 3.3). NaN and ±Inf give `Infinity`; zero gives zero.
    pub fn from_exact_num(x: &ExactNum) -> Mag {
        if x.is_nan() || x.is_inf() {
            return Mag::infinity();
        }
        let Some((mant, sticky, exp)) = x.top_u64_mantissa_and_sticky() else {
            return Mag::zero();
        };
        if !sticky {
            return Mag::from_canonical(exp, mant);
        }
        match mant.checked_add(1) {
            Some(m) => Mag::from_canonical(exp, m),
            None => match exp.checked_add(1) {
                Some(e) => Mag::from_canonical(e, TOP),
                None => Mag::infinity(),
            },
        }
    }

    /// Greatest `Mag` at or below `|x|` (CONTRACT 3.4). NaN and ±Inf give zero; zero gives zero.
    pub fn from_exact_num_lower(x: &ExactNum) -> Mag {
        if x.is_nan() || x.is_inf() {
            return Mag::zero();
        }
        match x.top_u64_mantissa_and_sticky() {
            Some((mant, _, exp)) => Mag::from_canonical(exp, mant),
            None => Mag::zero(),
        }
    }

    /// The exact value of `self` as an `ExactNum` (CONTRACT 2.3).
    ///
    /// # Errors
    /// `Representation` when `self` is `Infinity` or its exponent lies outside zf's exponent range.
    pub fn to_exact_num(&self) -> Result<ExactNum, BoundError> {
        let Some((exp, mant)) = self.finite_parts() else {
            return Err(BoundError::Representation);
        };
        if mant == 0 {
            return Ok(ExactNum::new(EXACT_PREC));
        }
        ExactNum::from_mag_parts(mant, exp, EXACT_PREC).map_err(map_zf_error)
    }

    /// `(exp, mant)` of a finite `Mag`, including the canonical zero.
    fn finite_parts(&self) -> Option<(i64, u64)> {
        match (self.exp(), self.mant()) {
            (Some(e), Some(m)) => Some((e, m)),
            (None, Some(_)) | (Some(_), None) | (None, None) => None,
        }
    }
}

/// zf errors in zb terms (CONTRACT O2 table; PLAN: `PrecisionRetryExhausted` is `Resource`).
fn map_zf_error(e: Error) -> BoundError {
    match e {
        Error::ExponentOverflow(_) | Error::InvalidArgument => BoundError::Representation,
        Error::DivisionByZero => BoundError::Domain,
        Error::PrecisionRetryExhausted | Error::MemoryAllocation => BoundError::Resource,
    }
}
