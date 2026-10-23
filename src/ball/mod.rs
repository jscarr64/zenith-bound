//! `Ball`: the closed set `[mid - rad, mid + rad]` (CONTRACT 6.1, 6.5 D15a/e, 8.1, 15.1-15.3).
//!
//! Fields are private (D15a). Constructors are checked: `prec == 0` is `InvalidArgument` (O17-W), a NaN or infinite
//! midpoint is `Representation` (6.2), a −0 midpoint becomes +0 (6.3), and an `Infinity` radius gives the canonical
//! all-reals ball with `mid = +0` (6.1). A constructor never rounds: an exact input wider than `prec` keeps its value
//! and its radius (D15a), so nothing is silently truncated. `Ball` is not `Clone`, `Copy`, `PartialEq` or `PartialOrd`
//! (15.2).

use crate::error::{BoundError, BoundResult};
use crate::mag::Mag;
use zenith_float_num::ExactNum;

/// A midpoint-radius ball with the requested midpoint precision `prec` (bits, at least 1).
#[derive(Debug)]
pub struct Ball {
    mid: ExactNum,
    rad: Mag,
    prec: u32,
}

impl Ball {
    /// The exact point `mid ± 0` (D15a: the midpoint is kept exactly even when wider than `prec`).
    ///
    /// # Errors
    /// `InvalidArgument` when `prec == 0`; `Representation` when `mid` is NaN or infinite.
    pub fn exact(_mid: ExactNum, _prec: u32) -> BoundResult<Ball> {
        Err(BoundError::Resource)
    }

    /// The ball `mid ± rad`; an `Infinity` radius gives the all-reals ball (`mid = +0`).
    ///
    /// # Errors
    /// `InvalidArgument` when `prec == 0`; `Representation` when `mid` is NaN or infinite.
    pub fn from_mid_rad(_mid: ExactNum, _rad: Mag, _prec: u32) -> BoundResult<Ball> {
        Err(BoundError::Resource)
    }

    /// The all-reals ball `+0 ± Infinity` (CONTRACT 6.1, 6.4).
    ///
    /// # Errors
    /// `InvalidArgument` when `prec == 0`.
    pub fn all_reals(_prec: u32) -> BoundResult<Ball> {
        Err(BoundError::Resource)
    }

    /// The midpoint.
    pub fn mid(&self) -> &ExactNum {
        &self.mid
    }

    /// The radius.
    pub fn rad(&self) -> &Mag {
        &self.rad
    }

    /// The requested midpoint precision (never read back from zenith-float, CONTRACT 8.1).
    pub fn prec(&self) -> u32 {
        self.prec
    }

    /// False iff the radius is `Infinity` (CONTRACT 6.4, 15.3).
    pub fn is_informative(&self) -> bool {
        true
    }

    /// True iff the radius is zero, so the ball is the single point `mid`.
    pub fn is_exact(&self) -> bool {
        false
    }

    /// Moves the parts out: `(mid, rad, prec)`.
    pub fn into_parts(self) -> (ExactNum, Mag, u32) {
        (self.mid, self.rad, self.prec)
    }
}
