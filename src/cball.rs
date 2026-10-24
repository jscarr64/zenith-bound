//! Slice CB: complex balls `CBall { re, im }`, a closed rectangle of two independent real [`Ball`] parts (CONTRACT 16.CB).
//! Built only on the public `Ball` API and S1; no radius arithmetic of its own. Ownership is O9 (16.CB.0): an op takes
//! `self` and borrows its second operand; a part is consumed where the inner `Ball` op consumes it.
//! `sqr` waits on S1's borrowing `sqr`; `mul` and `div` wait on borrowing once-rounded zf ops (Rusty, after R9). Until
//! then they return `Resource` with a `detail` naming the wait, and their tests are ignored red.

use zenith_float_num::ExactNum;

use crate::ball::Ball;
use crate::enclosure::Enclosure;
use crate::error::{BoundError, BoundResult};
use crate::precision::Precision;

/// A complex ball: the rectangle `re × im` (16.CB.1). Both parts share one prec (I2); the parts are independent (I3).
#[derive(Debug)]
pub struct CBall {
    re: Ball,
    im: Ball,
}

impl CBall {
    /// `re + i·im`. `InvalidArgument` when the parts' precs differ (I2).
    pub fn new(re: Ball, im: Ball) -> BoundResult<CBall> {
        if re.prec() != im.prec() {
            return Err(BoundError::InvalidArgument {
                op: "cball_new",
                detail: "re and im must have the same prec",
            });
        }
        Ok(CBall { re, im })
    }

    /// `re + 0i`, the imaginary part exact zero at `re`'s prec.
    pub fn from_real(re: Ball) -> BoundResult<CBall> {
        let im = Ball::exact(ExactNum::from(0i64), re.prec())?;
        Ok(CBall { re, im })
    }

    /// Both parts all reals.
    pub fn all_complex(prec: Precision) -> BoundResult<CBall> {
        Ok(CBall {
            re: Ball::all_reals(prec)?,
            im: Ball::all_reals(prec)?,
        })
    }

    /// The real part.
    pub fn re(&self) -> &Ball {
        &self.re
    }

    /// The imaginary part.
    pub fn im(&self) -> &Ball {
        &self.im
    }

    /// The shared prec of both parts.
    pub fn prec(&self) -> Precision {
        self.re.prec()
    }

    /// Both parts identical (same mid, rad and prec).
    pub fn identical(&self, other: &CBall) -> bool {
        self.re.identical(&other.re) && self.im.identical(&other.im)
    }

    /// `(−re, −im)`, exact.
    pub fn neg(self) -> CBall {
        CBall {
            re: -self.re,
            im: -self.im,
        }
    }

    /// `(re, −im)`, exact.
    pub fn conj(self) -> CBall {
        CBall {
            re: self.re,
            im: -self.im,
        }
    }

    /// Part-wise `Ball::add`.
    pub fn add(self, other: &CBall, prec: Precision) -> BoundResult<CBall> {
        Ok(CBall {
            re: self.re.add(&other.re, prec)?,
            im: self.im.add(&other.im, prec)?,
        })
    }

    /// Part-wise `Ball::sub`.
    pub fn sub(self, other: &CBall, prec: Precision) -> BoundResult<CBall> {
        Ok(CBall {
            re: self.re.sub(&other.re, prec)?,
            im: self.im.sub(&other.im, prec)?,
        })
    }

    /// `|a|` as a real ball: `Ball::hypot(re, im)` as is (16.CB.6).
    pub fn abs(self, prec: Precision) -> BoundResult<Ball> {
        self.re.hypot(&self.im, prec)
    }

    /// 16.CB.5. Not implemented: each part is used twice, which needs S1's borrowing `sqr` (SHA pending).
    pub fn sqr(self, _prec: Precision) -> BoundResult<CBall> {
        Err(BoundError::Resource {
            op: "cball_sqr",
            detail: "not implemented: waits S1 borrowing sqr",
        })
    }

    /// 16.CB.4. Not implemented: waits on borrowing once-rounded zf ops (Rusty, after R9).
    pub fn mul(self, _other: &CBall, _prec: Precision) -> BoundResult<CBall> {
        Err(BoundError::Resource {
            op: "cball_mul",
            detail: "not implemented: waits zf borrowing ops",
        })
    }

    /// 16.CB.7. Not implemented: waits on borrowing once-rounded zf ops (Rusty, after R9).
    pub fn div(self, _other: &CBall, _prec: Precision) -> BoundResult<CBall> {
        Err(BoundError::Resource {
            op: "cball_div",
            detail: "not implemented: waits zf borrowing ops",
        })
    }
}
