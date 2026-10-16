//! CONTRACT 14.7, 1.4, 3.2; oracle: dyadic-rational model (tests/model)

use crate::model::dyadic::{Dy, MagV};
use crate::model::oracle::{min_pos_mag, TWO63};
use crate::model::sets::mantissas;
use crate::support::{dcmp, dy_of, to_magv};
use core::cmp::Ordering;
use zenith_bound::mag::Mag;

const PRECS: [u32; 10] = [1, 2, 3, 4, 24, 53, 64, 65, 113, 127];
const EXPS: [i64; 11] = [-1000, -65, -64, -1, 0, 1, 2, 63, 64, 65, 1000];
/// The three mantissas CONTRACT 1.4 names for its boundary set.
const CONTRACT_MANTS: [u64; 3] = [TWO63, TWO63 + 1, u64::MAX];

#[test]
fn ulp_mag_is_two_to_the_pre_round_exp_minus_prec() -> Result<(), String> {
    for e in EXPS.iter() {
        for p in PRECS.iter() {
            let k = *e - i64::from(*p);
            let r = to_magv(&Mag::ulp_mag(*e, *p));
            if dcmp(&dy_of(&r)?, &Dy::pow2(i128::from(k)))? != Ordering::Equal {
                return Err(format!(
                    "ulp_mag({},{}) value differs from 2^{}, got {:?}",
                    e, p, k, r
                ));
            }
            if r != (MagV::Finite {
                exp: k + 1,
                mant: TWO63,
            }) {
                return Err(format!(
                    "ulp_mag({},{}) expected finite with exp={}, mant={}, got {:?}",
                    e,
                    p,
                    k + 1,
                    TWO63,
                    r
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn ulp_mag_exponent_below_i64_min_rounds_up_to_smallest_positive_mag() -> Result<(), String> {
    let cases = [
        (i64::MIN, 2u32, min_pos_mag()),
        (i64::MIN, 3u32, min_pos_mag()),
        (i64::MIN, 64u32, min_pos_mag()),
        (i64::MIN, 127u32, min_pos_mag()),
        (i64::MIN + 1, 3u32, min_pos_mag()),
        (i64::MIN + 1, 64u32, min_pos_mag()),
        (i64::MIN + 1, 127u32, min_pos_mag()),
    ];
    for (e, p, want) in cases.iter() {
        let r = to_magv(&Mag::ulp_mag(*e, *p));
        if r != *want {
            return Err(format!(
                "ulp_mag({},{}) expected {:?}, got {:?}",
                e, p, want, r
            ));
        }
    }

    for p in [1u32, 2, 64, 127] {
        let e = i64::MIN + i64::from(p) - 1;
        let r = to_magv(&Mag::ulp_mag(e, p));
        if r != min_pos_mag() {
            return Err(format!(
                "ulp_mag({},{}) expected {:?}, got {:?}",
                e,
                p,
                min_pos_mag(),
                r
            ));
        }
    }

    for p in [1u32, 2, 64, 127] {
        let e = i64::MIN + i64::from(p);
        let r = to_magv(&Mag::ulp_mag(e, p));
        let want = MagV::Finite {
            exp: i64::MIN + 1,
            mant: TWO63,
        };
        if r != want {
            return Err(format!(
                "ulp_mag({},{}) expected {:?}, got {:?}",
                e, p, want, r
            ));
        }
    }

    Ok(())
}

#[test]
fn half_ulp_is_two_to_the_mid_exp_minus_prec_minus_one() -> Result<(), String> {
    for e in EXPS.iter() {
        for p in PRECS.iter() {
            let k = *e - i64::from(*p) - 1;
            let r = to_magv(&Mag::half_ulp(*e, *p));
            if dcmp(&dy_of(&r)?, &Dy::pow2(i128::from(k)))? != Ordering::Equal {
                return Err(format!(
                    "half_ulp({},{}) value differs from 2^{}, got {:?}",
                    e, p, k, r
                ));
            }
            if r != (MagV::Finite {
                exp: k + 1,
                mant: TWO63,
            }) {
                return Err(format!(
                    "half_ulp({},{}) expected finite with exp={}, mant={}, got {:?}",
                    e,
                    p,
                    k + 1,
                    TWO63,
                    r
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn smaller_than_half_ulp_matches_model_on_boundary_grid() -> Result<(), String> {
    for &mid_exp in [-70i64, 0, 10, 130].iter() {
        for p in PRECS.iter() {
            let h = mid_exp - i64::from(*p) - 1;
            for exp in (h - 2)..=(h + 2) {
                for &mant in mantissas().iter() {
                    let x = Mag::Finite { exp, mant };
                    let want = dcmp(
                        &dy_of(&MagV::Finite { exp, mant })?,
                        &Dy::pow2(i128::from(h)),
                    )? == Ordering::Less;
                    let got = x.smaller_than_half_ulp(mid_exp, *p);
                    if got != want {
                        return Err(format!("smaller_than_half_ulp(Finite {{ exp: {}, mant: {} }}, {}, {}) expected {}, got {}", exp, mant, mid_exp, p, want, got));
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn smaller_than_half_ulp_contract_1_4_boundary_set() -> Result<(), String> {
    let mid_exp = 10;
    let prec = 4;
    for exp in 3..=8 {
        for &mant in CONTRACT_MANTS.iter() {
            let x = Mag::Finite { exp, mant };
            let want = exp <= 5;
            let got = x.smaller_than_half_ulp(mid_exp, prec);
            if got != want {
                return Err(format!("smaller_than_half_ulp(Finite {{ exp: {}, mant: {} }}, {}, {}) expected {}, got {}", exp, mant, mid_exp, prec, want, got));
            }
        }
    }
    Ok(())
}

#[test]
fn half_ulp_underflow_near_i64_min_rounds_up_to_smallest_positive_mag() -> Result<(), String> {
    for p in PRECS.iter() {
        let p64 = i64::from(*p);
        for e in [i64::MIN, i64::MIN + p64 - 1, i64::MIN + p64] {
            let r = to_magv(&Mag::half_ulp(e, *p));
            if r != min_pos_mag() {
                return Err(format!(
                    "half_ulp({},{}) expected {:?}, got {:?}",
                    e,
                    p,
                    min_pos_mag(),
                    r
                ));
            }
        }
        let e = i64::MIN + p64 + 1;
        let r = to_magv(&Mag::half_ulp(e, *p));
        let want = MagV::Finite {
            exp: i64::MIN + 1,
            mant: TWO63,
        };
        if r != want {
            return Err(format!(
                "half_ulp({},{}) expected {:?}, got {:?}",
                e, p, want, r
            ));
        }
    }
    Ok(())
}

#[test]
fn smaller_than_half_ulp_near_i64_min_matches_model_and_is_false_below_the_range(
) -> Result<(), String> {
    for p in PRECS.iter() {
        let p64 = i64::from(*p);
        for mid_exp in [
            i64::MIN,
            i64::MIN + 1,
            i64::MIN + p64 - 1,
            i64::MIN + p64,
            i64::MIN + p64 + 1,
            i64::MIN + p64 + 2,
        ] {
            let h = i128::from(mid_exp) - i128::from(*p) - 1;
            for exp in [
                i64::MIN,
                i64::MIN + 1,
                i64::MIN + 2,
                i64::MIN + 3,
                i64::MIN + 64,
                i64::MIN + 130,
            ] {
                for &mant in mantissas().iter() {
                    let x = Mag::Finite { exp, mant };
                    let want =
                        dcmp(&dy_of(&MagV::Finite { exp, mant })?, &Dy::pow2(h))? == Ordering::Less;
                    let got = x.smaller_than_half_ulp(mid_exp, *p);
                    if got != want {
                        return Err(format!("smaller_than_half_ulp(Finite {{ exp: {}, mant: {} }}, {}, {}) expected {}, got {}", exp, mant, mid_exp, p, want, got));
                    }
                    if h < i128::from(i64::MIN) - 1 && got {
                        return Err(format!("smaller_than_half_ulp(Finite {{ exp: {}, mant: {} }}, {}, {}) must be false: the threshold 2^{} is below every nonzero Mag", exp, mant, mid_exp, p, h));
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn smaller_than_half_ulp_zero_is_true_and_infinity_is_false() -> Result<(), String> {
    for p in PRECS.iter() {
        for mid_exp in [i64::MIN, -1000, 0, 1000, i64::MAX] {
            if !(Mag::Finite { exp: 0, mant: 0 }).smaller_than_half_ulp(mid_exp, *p) {
                return Err(format!(
                    "zero must be smaller than half an ulp: mid_exp {}, prec {}",
                    mid_exp, p
                ));
            }
            if Mag::Infinity.smaller_than_half_ulp(mid_exp, *p) {
                return Err(format!(
                    "Infinity must not be smaller than half an ulp: mid_exp {}, prec {}",
                    mid_exp, p
                ));
            }
        }
    }
    Ok(())
}
