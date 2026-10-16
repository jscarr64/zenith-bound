//! CONTRACT 14.1, 14.3, 14.6, 2.3; oracle: dyadic-rational model (tests/model)
use crate::convert::{ctx, dcmp, dy_of, from_magv, to_magv};
use crate::model::dyadic::MagV;
use crate::model::oracle::{check_up, cmp_dy, succ};
use crate::model::sets::{extreme_exponents, magv_set, mid_exponents};
use core::cmp::Ordering;
use zenith_bound::mag::Mag;

pub fn verify(a: &MagV, b: &MagV, r: &MagV) -> Result<(), String> {
    let da = dy_of(a)?;
    let db = dy_of(b)?;
    match da.add(&db) {
        Some(exact) => check_up(r, da.is_zero() && db.is_zero(), &|m| cmp_dy(m, &exact)),
        None => {
            // exponent gap above the model's limit; both operands nonzero; CONTRACT 14.3: the result is the larger operand plus one unit in its last place = succ(larger)
            let larger = match dcmp(&da, &db)? {
                Ordering::Less => b,
                _ => a,
            };
            let want = succ(larger);
            if *r == want {
                Ok(())
            } else {
                Err(format!("expected {:?}, got {:?}", want, r))
            }
        }
    }
}

fn check_pair(a: &MagV, b: &MagV) -> Result<(), String> {
    let r = to_magv(&from_magv(a).add_up(&from_magv(b)));
    verify(a, b, &r)
}

fn check_set(set: &[MagV]) -> Result<(), String> {
    for a in set.iter() {
        for b in set.iter() {
            ctx("add_up", &[a, b], check_pair(a, b))?;
        }
    }
    Ok(())
}

#[test]
fn add_up_mid_exponent_set_is_smallest_upper_bound() -> Result<(), String> {
    check_set(&magv_set(&mid_exponents()))
}

#[test]
fn add_up_extreme_exponent_set_is_smallest_upper_bound() -> Result<(), String> {
    check_set(&magv_set(&extreme_exponents()))
}

#[test]
fn add_up_infinity_operand_gives_infinity() -> Result<(), String> {
    for x in magv_set(&mid_exponents()).iter() {
        let r1 = to_magv(&from_magv(x).add_up(&Mag::Infinity));
        let r2 = to_magv(&Mag::Infinity.add_up(&from_magv(x)));
        if r1 != MagV::Infinity {
            return Err(format!("expected Infinity, got {:?}", r1));
        }
        if r2 != MagV::Infinity {
            return Err(format!("expected Infinity, got {:?}", r2));
        }
    }
    let r3 = to_magv(&Mag::Infinity.add_up(&Mag::Infinity));
    if r3 != MagV::Infinity {
        return Err(format!("expected Infinity, got {:?}", r3));
    }
    Ok(())
}
