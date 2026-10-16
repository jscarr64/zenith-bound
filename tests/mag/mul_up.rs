//! CONTRACT 1.3, 14.1, 2.3, 2.4, 3.2, 3.3; oracle: dyadic-rational model (tests/model)

use crate::convert::{ctx, dy_of, from_magv, to_magv};
use crate::model::dyadic::MagV;
use crate::model::oracle::{check_up, cmp_dy};
use crate::model::sets::{extreme_exponents, magv_set, mid_exponents};
use zenith_bound::mag::Mag;

pub fn verify(a: &MagV, b: &MagV, r: &MagV) -> Result<(), String> {
    let da = dy_of(a)?;
    let db = dy_of(b)?;
    let exact = da.mul(&db);
    check_up(r, da.is_zero() || db.is_zero(), &|m| cmp_dy(m, &exact))
}

fn check_pair(a: &MagV, b: &MagV) -> Result<(), String> {
    let r = to_magv(&from_magv(a).mul_up(&from_magv(b)));
    verify(a, b, &r)
}

fn check_set(set: &[MagV]) -> Result<(), String> {
    for a in set.iter() {
        for b in set.iter() {
            ctx("mul_up", &[a, b], check_pair(a, b))?;
        }
    }
    Ok(())
}

#[test]
fn mul_up_mid_exponent_set_is_smallest_upper_bound() -> Result<(), String> {
    check_set(&magv_set(&mid_exponents()))
}

#[test]
fn mul_up_extreme_exponent_set_covers_underflow_and_overflow() -> Result<(), String> {
    check_set(&magv_set(&extreme_exponents()))
}

#[test]
fn mul_up_infinity_operand_gives_infinity_even_for_zero() -> Result<(), String> {
    let set = magv_set(&mid_exponents());
    for x in set.iter() {
        // x * Infinity
        let r1 = to_magv(&from_magv(x).mul_up(&Mag::Infinity));
        if r1 != MagV::Infinity {
            return Err("x * Infinity != Infinity".to_string());
        }

        // Infinity * x
        let r2 = to_magv(&Mag::Infinity.mul_up(&from_magv(x)));
        if r2 != MagV::Infinity {
            return Err("Infinity * x != Infinity".to_string());
        }
    }

    // Infinity * Infinity
    let r3 = to_magv(&Mag::Infinity.mul_up(&Mag::Infinity));
    if r3 != MagV::Infinity {
        return Err("Infinity * Infinity != Infinity".to_string());
    }

    Ok(())
}
