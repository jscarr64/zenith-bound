//! Normalise a positive dyadic integer to a canonical `Mag`.
//!
//! `norm(m, scale, sticky, up)` rounds the real value `m * 2^scale`, plus an unresolved
//! positive residue when `sticky` is set. The `Mag` exponent field of a 64-bit mantissa
//! `mant` is `scale + 64` after the significand has been aligned to `[2^63, 2^64)`.

use super::Mag;
use super::TOP;

/// Sound stand-in when a checked arithmetic step cannot be completed.
/// Round-up overflows to `Infinity`; round-down underflows to zero.
pub(super) fn loose(up: bool) -> Mag {
    if up {
        Mag::infinity()
    } else {
        Mag::zero()
    }
}

/// Place an already aligned exponent field, clamping to the `Mag` range.
pub(super) fn pack(exp: i128, mant: u64, up: bool) -> Mag {
    if mant == 0 {
        return if up { Mag::min_pos() } else { Mag::zero() };
    }
    if mant < TOP {
        return loose(up);
    }
    match i64::try_from(exp) {
        Ok(e) => Mag::from_canonical(e, mant),
        Err(_) => {
            if exp > i128::from(i64::MAX) {
                if up {
                    Mag::infinity()
                } else {
                    Mag::max_finite()
                }
            } else if up {
                Mag::min_pos()
            } else {
                Mag::zero()
            }
        }
    }
}

/// Round `m * 2^scale` toward `+∞` when `up` is set, otherwise toward zero.
pub(super) fn norm(m: u128, scale: i128, sticky: bool, up: bool) -> Mag {
    if m == 0 {
        return if up && sticky {
            Mag::infinity()
        } else {
            Mag::zero()
        };
    }
    let bit_len = match 128u32.checked_sub(m.leading_zeros()) {
        Some(n) => n,
        None => return loose(up),
    };
    let mut sig = m;
    let mut scale = scale;
    let mut sticky = sticky;
    if bit_len > 64 {
        let sh = match bit_len.checked_sub(64) {
            Some(sh) => sh,
            None => return loose(up),
        };
        let (hi, low) = match shr_parts(sig, sh) {
            Some(parts) => parts,
            None => return loose(up),
        };
        sig = hi;
        scale = match scale.checked_add(i128::from(sh)) {
            Some(v) => v,
            None => return loose(up),
        };
        if low {
            sticky = true;
        }
    } else if bit_len < 64 {
        let sh = match 64u32.checked_sub(bit_len) {
            Some(sh) => sh,
            None => return loose(up),
        };
        sig = match sig.checked_shl(sh) {
            Some(v) => v,
            None => return loose(up),
        };
        scale = match scale.checked_sub(i128::from(sh)) {
            Some(v) => v,
            None => return loose(up),
        };
    }
    let mut mant = match u64::try_from(sig) {
        Ok(mant) => mant,
        Err(_) => return loose(up),
    };
    let mut exp = match scale.checked_add(64) {
        Some(exp) => exp,
        None => return loose(up),
    };
    if up && sticky {
        if mant == u64::MAX {
            mant = TOP;
            exp = match exp.checked_add(1) {
                Some(exp) => exp,
                None => return Mag::infinity(),
            };
        } else {
            mant = match mant.checked_add(1) {
                Some(mant) => mant,
                None => return Mag::infinity(),
            };
        }
    }
    pack(exp, mant, up)
}

/// Right-shift `m` by `sh` and report whether any shifted-out bit was set.
fn shr_parts(m: u128, sh: u32) -> Option<(u128, bool)> {
    let mask = 1u128.checked_shl(sh)?.checked_sub(1)?;
    let low = (m & mask) != 0;
    let hi = m.checked_shr(sh)?;
    Some((hi, low))
}
