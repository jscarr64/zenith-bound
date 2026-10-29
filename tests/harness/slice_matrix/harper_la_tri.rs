//! Harper wide-range rows for 16.S6.12 triangular solves (2026-10-29), written test-first against the stub API: 32×32
//! unit-lower and scaled dyadic systems at 16384 bits with exact dyadic solutions, diagonals from 2^0 down to 2^-992
//! (condition numbers 10^1 to 10^300), diagonals 2^±203, overflow and underflow crossings near EXPONENT_MAX/MIN, signed
//! zero, non-square and length errors, `n = 0`, and the untouched other triangle. Also the green self-tests of the shared
//! row builders in tests/common/la_rows.rs.
//! COVERS: BallMat::solve_lower, BallMat::solve_upper, BallMat::solve_lower_mat, BallMat::solve_upper_mat
//! COVERS: common::la_rows::at, common::la_rows::cell, common::la_rows::diag, common::la_rows::dy, common::la_rows::fuzzy, common::la_rows::int, common::la_rows::k_ln2, common::la_rows::ldlt, common::la_rows::mat, common::la_rows::mat_with, common::la_rows::mids, common::la_rows::mul_exact, common::la_rows::neg_zero, common::la_rows::ok, common::la_rows::pow2, common::la_rows::pows, common::la_rows::pt, common::la_rows::transpose, common::la_rows::unit_lower, common::la_rows::vat, common::la_rows::vecb, common::la_rows::want_all_reals, common::la_rows::want_contains, common::la_rows::want_encloses, common::la_rows::want_err, common::la_rows::want_k_ln2, common::la_rows::want_mat, common::la_rows::want_pos_zero, common::la_rows::want_vec

use zenith_bound::array::BallVec;
use zenith_bound::ball::Ball;
use zenith_bound::enclosure::Enclosure;
use zenith_bound::error::BoundError;
use zenith_bound::tri::Tri;
use zenith_float_num::{ExactNum, EXPONENT_MAX, EXPONENT_MIN};

use crate::common::la_rows::{
    at, cell, diag, dy, fuzzy, i64_of, int, k_ln2, ldlt, mat, mat_with, mids, mul_exact, neg_zero,
    ok, pow2, pows, pt, transpose, unit_lower, vat, vecb, want_all_reals, want_contains,
    want_encloses, want_err, want_k_ln2, want_mat, want_pos_zero, want_vec, P,
};

const N: usize = 32;

/// `x_i = (i mod 7 − 3)·2^(−i)`: exact dyadic solutions with mixed signs and zeros.
fn xs(n: usize) -> Result<Vec<ExactNum>, String> {
    let mut v = Vec::new();
    for i in 0..n {
        let k = i64_of(i)?;
        let m = k
            .checked_rem(7)
            .and_then(|r| r.checked_sub(3))
            .ok_or("xs")?;
        v.push(dy(m, k.checked_neg().ok_or("neg")?)?);
    }
    Ok(v)
}

/// The exponents `−⌊span·i/(n−1)⌋`, a diagonal spread of `span` bits.
fn spread(n: usize, span: i64) -> Result<Vec<i64>, String> {
    let last = i64_of(n)?.checked_sub(1).ok_or("n")?;
    let mut v = Vec::new();
    for i in 0..n {
        let e = span
            .checked_mul(i64_of(i)?)
            .and_then(|x| x.checked_div(last))
            .and_then(i64::checked_neg)
            .ok_or("spread")?;
        v.push(e);
    }
    Ok(v)
}

// ---- green self-tests of the shared builders --------------------------------------------------------------------------

#[test]
fn la_rows_ldlt_matches_zb_matmul() -> Result<(), String> {
    let n = 8;
    let l = unit_lower(n, 2)?;
    let d = pows(&[203, -203, 0, 101, -101, 7, -7, 1])?;
    let c = ldlt(n, &l, &d)?;
    let lb = mat(n, n, unit_lower(n, 2)?)?;
    let db = mat(n, n, diag(&d))?;
    let ltb = mat(n, n, transpose(n, n, &l)?)?;
    let got = ok("matmul", ok("matmul", lb.matmul(&db, P))?.matmul(&ltb, P))?;
    want_mat("L·D·Lᵀ", &got, n, n, &c)?;
    let x = xs(n)?;
    let b = mul_exact(n, n, 1, &c, &x)?;
    let bz = ok("matvec", mat(n, n, c)?.matvec(&vecb(x)?, P))?;
    want_vec("C·x", &bz, &b)
}

#[test]
fn la_rows_k_ln2_helper_decides_both_ways() -> Result<(), String> {
    for k in [0, 1, -4060, 813, EXPONENT_MAX, EXPONENT_MIN] {
        want_k_ln2(&format!("{k}·ln2"), &k_ln2(k, P)?, k)?;
    }
    match want_k_ln2("5·ln2 as 6·ln2", &k_ln2(5, P)?, 6) {
        Ok(()) => Err("want_k_ln2 accepted 5·ln2 for 6·ln2".to_string()),
        Err(_) => Ok(()),
    }
}

#[test]
fn la_rows_pow2_spans_the_exponent_range() -> Result<(), String> {
    let top = pow2(EXPONENT_MAX.checked_sub(1).ok_or("emax")?)?;
    let low = pow2(EXPONENT_MIN.checked_sub(1).ok_or("emin")?)?;
    want_contains(
        "2^(EMAX-1)",
        &pt(top)?,
        &dy(2, EXPONENT_MAX.checked_sub(2).ok_or("emax")?)?,
    )?;
    want_contains(
        "2^(EMIN-1)",
        &pt(low)?,
        &dy(1, EXPONENT_MIN.checked_sub(1).ok_or("emin")?)?,
    )?;
    match pow2(EXPONENT_MAX) {
        Ok(v) => Err(format!("2^EMAX should not be representable, got {v:?}")),
        Err(_) => Ok(()),
    }
}

#[test]
fn la_rows_neg_zero_ball_is_pos_zero() -> Result<(), String> {
    want_pos_zero("Ball::exact(-0)", &pt(neg_zero()?)?)
}

// ---- 16.S6.12 red rows --------------------------------------------------------------------------------------------------

#[test]
#[ignore = "red: CONTRACT 16.S6.12: solve_lower 32x32 dyadic unit and 2^±203 diagonal, other triangle unread"]
fn la_tri_solve_lower_32_dyadic_unit_and_scaled() -> Result<(), String> {
    let x = xs(N)?;
    let l = unit_lower(N, 1)?;
    let b = mul_exact(N, N, 1, &l, &x)?;
    // unit_diag = true: the diagonal (here 0 ± 0) and the upper triangle (all-reals) are never read
    let mut garbage = unit_lower(N, 1)?;
    if let Some(d0) = garbage.get_mut(0) {
        *d0 = int(0);
    }
    let lg = mat_with(
        N,
        N,
        garbage,
        vec![(0, 5, ok("all_reals", Ball::all_reals(P))?)],
    )?;
    want_vec(
        "solve_lower unit",
        &ok("solve_lower", lg.solve_lower(&vecb(b)?, true, P))?,
        &x,
    )?;
    let mut ks = Vec::new();
    for i in 0..N {
        ks.push(if i.checked_rem(2) == Some(0) {
            203
        } else {
            -203
        });
    }
    let t = mul_exact(N, N, N, &l, &diag(&pows(&ks)?))?;
    let b2 = mul_exact(N, N, 1, &t, &x)?;
    let tg = mat_with(N, N, t, vec![(3, 30, ok("all_reals", Ball::all_reals(P))?)])?;
    want_vec(
        "solve_lower 2^±203",
        &ok("solve_lower", tg.solve_lower(&vecb(b2)?, false, P))?,
        &x,
    )
}

#[test]
#[ignore = "red: CONTRACT 16.S6.12: solve_upper 32x32 with diagonal spreads for cond 1e1, 1e16, 1e100, 1e300"]
fn la_tri_solve_upper_32_cond_1e1_to_1e300() -> Result<(), String> {
    let x = xs(N)?;
    for span in [3, 53, 333, 992] {
        let d = pows(&spread(N, span)?)?;
        let u = mul_exact(N, N, N, &diag(&d), &transpose(N, N, &unit_lower(N, 4)?)?)?;
        let b = mul_exact(N, N, 1, &u, &x)?;
        let got = ok(
            "solve_upper",
            mat(N, N, u)?.solve_upper(&vecb(b)?, false, P),
        )?;
        want_vec(&format!("solve_upper span 2^{span}"), &got, &x)?;
    }
    Ok(())
}

#[test]
#[ignore = "red: CONTRACT 16.S6.12: solve_lower_mat and solve_upper_mat solve column j of b, equal to the vector form"]
fn la_tri_solve_lower_mat_and_solve_upper_mat_columns() -> Result<(), String> {
    let n = 16;
    let l = mul_exact(n, n, n, &unit_lower(n, 3)?, &diag(&pows(&spread(n, 120)?)?))?;
    let u = transpose(n, n, &l)?;
    // X: column 0 is xs, column 1 is −xs·2^200
    let x0 = xs(n)?;
    let mut x = Vec::new();
    for v in &x0 {
        x.push(v.mul_full_prec(&dy(-1, 0)?));
        x.push(v.mul_full_prec(&pow2(200)?));
    }
    let xs2 = transpose(n, 2, &x)?;
    let (lm, um) = (mat(n, n, l)?, mat(n, n, u)?);
    for (name, m, lower) in [
        ("solve_lower_mat", &lm, true),
        ("solve_upper_mat", &um, false),
    ] {
        let mut gridded = Vec::new();
        for i in 0..n {
            for j in 0..2 {
                gridded.push(int(0).add_full_prec(cell(&x, 2, i, j)?));
            }
        }
        let gm = mul_exact(n, n, 2, &mids(m)?, &gridded)?;
        let bm = mat(n, 2, gm)?;
        let got = if lower {
            ok(name, m.solve_lower_mat(&bm, false, P))?
        } else {
            ok(name, m.solve_upper_mat(&bm, false, P))?
        };
        want_mat(name, &got, n, 2, &x)?;
        for j in 0..2 {
            let mut col = Vec::new();
            for i in 0..n {
                col.push(int(0).add_full_prec(cell(&mids(&bm)?, 2, i, j)?));
            }
            let bv = vecb(col)?;
            let v = if lower {
                ok("solve_lower", m.solve_lower(&bv, false, P))?
            } else {
                ok("solve_upper", m.solve_upper(&bv, false, P))?
            };
            for i in 0..n {
                if !at(&got, i, j)?.identical(vat(&v, i)?) {
                    return Err(format!(
                        "{name}: column {j} row {i} differs from the vector form"
                    ));
                }
            }
            want_vec(
                &format!("{name} col {j}"),
                &v,
                xs2.get(j.checked_mul(n).ok_or("j")?..)
                    .and_then(|s| s.get(..n))
                    .ok_or("col")?,
            )?;
        }
    }
    Ok(())
}

#[test]
#[ignore = "red: CONTRACT 16.S6.12: solve_lower/solve_upper errors in order: prec, shape, b_len, singular diagonal"]
fn la_tri_errors_in_order() -> Result<(), String> {
    let wide = mat(2, 3, vec![int(1), int(0), int(0), int(1), int(1), int(0)])?;
    let b2 = vecb(vec![int(1), int(1)])?;
    want_err(
        "prec 0 before shape",
        wide.solve_lower(&b2, false, 0),
        |e| matches!(e, BoundError::InvalidArgument { .. }),
        "InvalidArgument",
    )?;
    want_err(
        "solve_lower non-square",
        wide.solve_lower(&b2, false, P),
        |e| matches!(e, BoundError::DimensionMismatch { .. }),
        "DimensionMismatch",
    )?;
    want_err(
        "solve_upper non-square",
        wide.solve_upper(&b2, false, P),
        |e| matches!(e, BoundError::DimensionMismatch { .. }),
        "DimensionMismatch",
    )?;
    let sq = mat(2, 2, vec![int(1), int(0), int(1), int(1)])?;
    let b3 = vecb(vec![int(1), int(1), int(1)])?;
    want_err(
        "solve_lower b_len",
        sq.solve_lower(&b3, false, P),
        |e| {
            matches!(
                e,
                BoundError::DimensionMismatch {
                    detail: "b_len",
                    ..
                }
            )
        },
        "DimensionMismatch b_len",
    )?;
    want_err(
        "solve_upper b_len",
        sq.solve_upper(&b3, true, P),
        |e| {
            matches!(
                e,
                BoundError::DimensionMismatch {
                    detail: "b_len",
                    ..
                }
            )
        },
        "DimensionMismatch b_len",
    )?;
    let bm3 = mat(3, 1, vec![int(1), int(1), int(1)])?;
    want_err(
        "solve_lower_mat b_len",
        sq.solve_lower_mat(&bm3, false, P),
        |e| {
            matches!(
                e,
                BoundError::DimensionMismatch {
                    detail: "b_len",
                    ..
                }
            )
        },
        "DimensionMismatch b_len",
    )?;
    want_err(
        "solve_upper_mat b_len",
        sq.solve_upper_mat(&bm3, false, P),
        |e| {
            matches!(
                e,
                BoundError::DimensionMismatch {
                    detail: "b_len",
                    ..
                }
            )
        },
        "DimensionMismatch b_len",
    )?;
    // |m| = r touches 0: a certified Singular at index 1 (mutants 920, 921, 929)
    let touch = mat_with(
        2,
        2,
        vec![int(2), int(0), int(1), int(0)],
        vec![(1, 1, fuzzy(int(1), 1)?)],
    )?;
    want_err(
        "touching diagonal",
        touch.solve_lower(&b2, false, P),
        |e| {
            matches!(
                e,
                BoundError::Singular {
                    detail: "singular",
                    ..
                }
            )
        },
        "Singular singular",
    )?;
    let straddle = mat_with(
        2,
        2,
        vec![int(0), int(1), int(0), int(2)],
        vec![(0, 0, fuzzy(dy(1, -1)?, 1)?)],
    )?;
    want_err(
        "straddling diagonal",
        straddle.solve_upper(&b2, false, P),
        |e| {
            matches!(
                e,
                BoundError::Singular {
                    detail: "singular",
                    ..
                }
            )
        },
        "Singular singular",
    )?;
    let inf = mat_with(
        2,
        2,
        vec![int(1), int(0), int(0), int(7)],
        vec![(1, 1, ok("all_reals", Ball::all_reals(P))?)],
    )?;
    want_err(
        "all-reals diagonal",
        inf.solve_lower(&b2, false, P),
        |e| {
            matches!(
                e,
                BoundError::Singular {
                    detail: "singular",
                    ..
                }
            )
        },
        "Singular singular",
    )?;
    // unit_diag = true does not read the zero diagonal: x = (1, 0)
    let zero_diag = mat(2, 2, vec![int(0), int(0), int(1), int(0)])?;
    want_vec(
        "unit_diag ignores the diagonal",
        &ok("solve_lower", zero_diag.solve_lower(&b2, true, P))?,
        &[int(1), int(0)],
    )
}

#[test]
#[ignore = "red: CONTRACT 16.S6.12: n = 0 solves return empty results"]
fn la_tri_n0_is_empty_ok() -> Result<(), String> {
    let e = mat(0, 0, Vec::new())?;
    let v = vecb(Vec::new())?;
    for (name, r) in [
        ("solve_lower", e.solve_lower(&v, false, P)),
        ("solve_upper", e.solve_upper(&v, true, P)),
    ] {
        let got = ok(name, r)?;
        if !got.is_empty() {
            return Err(format!("{name}: length {}", got.len()));
        }
    }
    let b = mat(0, 3, Vec::new())?;
    for (name, r) in [
        ("solve_lower_mat", e.solve_lower_mat(&b, false, P)),
        ("solve_upper_mat", e.solve_upper_mat(&b, false, P)),
    ] {
        let got = ok(name, r)?;
        if got.nrows() != 0 || got.ncols() != 3 {
            return Err(format!("{name}: shape {}x{}", got.nrows(), got.ncols()));
        }
    }
    Ok(())
}

#[test]
#[ignore = "red: CONTRACT 16.S6.12: overflow gives all-reals Ok, underflow stays a sound finite enclosure"]
fn la_tri_overflow_and_underflow_crossings() -> Result<(), String> {
    let big = EXPONENT_MAX.checked_sub(2).ok_or("emax")?;
    let t = mat(2, 2, vec![pow2(-8)?, int(0), int(1), int(1)])?;
    let x = ok(
        "solve_lower overflow",
        t.solve_lower(&vecb(vec![pow2(big)?, int(1)])?, false, P),
    )?;
    want_all_reals("x0 = 2^(EMAX+6)", vat(&x, 0)?)?;
    want_all_reals("x1 reads x0", vat(&x, 1)?)?;
    let tiny = EXPONENT_MIN.checked_add(2).ok_or("emin")?;
    let t2 = mat(2, 2, vec![pow2(8)?, int(1), int(0), int(1)])?;
    let y = ok(
        "solve_lower underflow",
        t2.solve_lower(&vecb(vec![pow2(tiny)?, int(1)])?, false, P),
    )?;
    let y0 = vat(&y, 0)?;
    // the exact x0 = 2^(EMIN-6) lies in (0, 2^(EMIN-1)): the ball can be certified neither below 0 nor above 2^(EMIN-1)
    let floor = pow2(EXPONENT_MIN.checked_sub(1).ok_or("emin")?)?;
    if y0.rad().is_infinity() || y0.lt(&pt(int(0))?) == Tri::True || y0.gt(&pt(floor)?) == Tri::True
    {
        return Err(format!("underflowed x0 {y0:?} cannot hold 2^(EMIN-6)"));
    }
    want_encloses("x1 = 1 (upper triangle unread)", vat(&y, 1)?, &int(1))
}

#[test]
#[ignore = "red: CONTRACT 16.S6.12: signed zero inputs give +0 solution entries"]
fn la_tri_signed_zero() -> Result<(), String> {
    let nz = neg_zero()?;
    let t = mat(
        3,
        3,
        vec![
            int(2),
            int(0),
            int(0),
            dy(1, -1)?,
            int(2),
            int(0),
            neg_zero()?,
            int(-1),
            int(2),
        ],
    )?;
    let b = vecb(vec![neg_zero()?, nz, int(4)])?;
    let x = ok("solve_lower", t.solve_lower(&b, false, P))?;
    want_pos_zero("x0 = -0/2", vat(&x, 0)?)?;
    want_pos_zero("x1", vat(&x, 1)?)?;
    want_encloses("x2 = 2", vat(&x, 2)?, &int(2))?;
    let u = ok(
        "solve_upper",
        mat(1, 1, vec![int(-3)])?.solve_upper(&vecb(vec![neg_zero()?])?, false, P),
    )?;
    want_pos_zero("-0/-3", vat(&u, 0)?)
}

#[test]
#[ignore = "red: CONTRACT 16.S6.12: radii of b and of earlier x_j reach later entries"]
fn la_tri_radii_propagate() -> Result<(), String> {
    let t = mat(2, 2, vec![int(1), int(0), int(1), int(1)])?;
    let b = BallVec::from_balls(vec![fuzzy(int(1), -9)?, pt(int(0))?]);
    let x = ok("solve_lower", t.solve_lower(&b, false, P))?;
    // b0 = 1 ± 2^-10, so x0 ⊇ [1 − 2^-10, 1 + 2^-10] and x1 = −x0 ⊇ [−1 − 2^-10, −1 + 2^-10]
    for (i, ends) in [
        (0, [dy(1023, -10)?, dy(1025, -10)?]),
        (1, [dy(-1025, -10)?, dy(-1023, -10)?]),
    ] {
        for e in &ends {
            want_contains(&format!("x{i} end"), vat(&x, i)?, e)?;
        }
    }
    Ok(())
}
