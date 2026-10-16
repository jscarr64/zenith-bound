//! `sqrt_up` and `sqrt_lower` by an integer square root, exponent parity first.

use super::round::loose;
use super::round::norm;
use super::Mag;

impl Mag {
    /// Least `Mag` at or above `sqrt(self)` (CONTRACT 14.1, 14.4).
    ///
    /// `sqrt(Infinity)` is `Infinity`. `sqrt(0)` is zero. The exponent parity is fixed
    /// first so the radicand is a `u128`, then the integer square root is rounded up
    /// when the square is not exact.
    pub fn sqrt_up(&self) -> Mag {
        if self.is_infinity() {
            Mag::infinity()
        } else {
            sqrt_parts(self, true)
        }
    }

    /// Greatest `Mag` at or below `sqrt(self)` (CONTRACT 14.2, 14.4).
    ///
    /// `sqrt(Infinity)` and `sqrt(0)` are zero. A nonzero finite value uses the same
    /// integer square root as [`Mag::sqrt_up`], truncated toward zero.
    pub fn sqrt_lower(&self) -> Mag {
        if self.is_infinity() {
            Mag::zero()
        } else {
            sqrt_parts(self, false)
        }
    }
}

fn sqrt_parts(mag: &Mag, up: bool) -> Mag {
    let Some((exp, mant)) = mag.nonzero() else {
        return Mag::zero();
    };
    let exp = i128::from(exp);
    let parity = match exp.checked_sub(128).and_then(|adj| adj.checked_rem(2)) {
        Some(rem) => rem,
        None => return loose(up),
    };
    let (radicand, scale) = if parity == 0 {
        let radicand = match u128::from(mant.get()).checked_shl(64) {
            Some(radicand) => radicand,
            None => return loose(up),
        };
        let scale = match exp.checked_sub(128) {
            Some(scale) => scale,
            None => return loose(up),
        };
        (radicand, scale)
    } else {
        let radicand = match u128::from(mant.get()).checked_shl(63) {
            Some(radicand) => radicand,
            None => return loose(up),
        };
        let scale = match exp.checked_sub(127) {
            Some(scale) => scale,
            None => return loose(up),
        };
        (radicand, scale)
    };
    let root = isqrt(radicand);
    let sticky = match root.checked_mul(root) {
        Some(square) => square != radicand,
        None => true,
    };
    let scale = match scale.checked_div(2) {
        Some(scale) => scale,
        None => return loose(up),
    };
    norm(root, scale, sticky, up)
}

/// Floor of the square root of `n`. The result always fits in `u128` and, for every
/// `n`, its square fits in `u128` as well (`(2^64 - 1)^2 < 2^128`).
fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut lo = 1u128;
    let mut hi = SQRT_EXCLUSIVE_HI;
    loop {
        let Some(width) = hi.checked_sub(lo) else {
            return lo;
        };
        if width <= 1 {
            return lo;
        }
        let Some(half) = width.checked_shr(1) else {
            return lo;
        };
        let Some(mid) = lo.checked_add(half) else {
            return lo;
        };
        match mid.checked_mul(mid) {
            Some(square) if square <= n => lo = mid,
            _ => hi = mid,
        }
    }
}

/// Exclusive upper bound on `isqrt` of any `u128` (`2^64`).
const SQRT_EXCLUSIVE_HI: u128 = 0x1_0000_0000_0000_0000;
