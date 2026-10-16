//! CONTRACT 14.5, 2.1; oracle: dyadic-rational model (tests/model)

use crate::convert::{ctx, dy_of, from_magv};
use crate::model::dyadic::MagV;
use crate::model::sets::{extreme_exponents, magv_set, mid_exponents};

fn check_pair(a: &MagV, b: &MagV) -> Result<(), String> {
    let got = from_magv(a).cmp(&from_magv(b));
    let want = dy_of(a)?.cmp(&dy_of(b)?);
    if got == want {
        Ok(())
    } else {
        Err(format!("expected {:?}, got {:?}", want, got))
    }
}

fn check_set(set: &[MagV]) -> Result<(), String> {
    for a in set.iter() {
        for b in set.iter() {
            ctx("cmp", &[a, b], check_pair(a, b))?;
        }
    }
    Ok(())
}

#[test]
fn cmp_mid_exponent_set_matches_model() -> Result<(), String> {
    check_set(&magv_set(&mid_exponents()))
}

#[test]
fn cmp_extreme_exponent_set_matches_model() -> Result<(), String> {
    check_set(&magv_set(&extreme_exponents()))
}

#[test]
fn cmp_is_antisymmetric_on_mid_exponent_set() -> Result<(), String> {
    let set = magv_set(&mid_exponents());
    for a in set.iter() {
        for b in set.iter() {
            let ord1 = from_magv(a).cmp(&from_magv(b));
            let ord2 = from_magv(b).cmp(&from_magv(a));
            if ord1 == ord2.reverse() {
                continue;
            } else {
                return Err(format!("expected {:?}, got {:?}", ord2.reverse(), ord1));
            }
        }
    }
    Ok(())
}
