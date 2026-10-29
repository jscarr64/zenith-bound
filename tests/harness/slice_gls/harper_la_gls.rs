//! Harper cosmology-scale GLS rows (2026-10-29) for CONTRACT 16.LA4, written test-first against the stub API at 16384 bits.
//! The exact row: `C = G·Gᵀ` with `G = L·diag(2^±101)` (covariance scales 2^±202), `X = G·Q·S` with `Q` four orthonormal
//! Hadamard columns of order 16 and `S = diag(2^s)`, and `y = G·(Q·S·β₀ + 3·q₄)` with `q₄ ⟂ range(Q)`. Then exactly
//! `β = β₀`, `cov = S⁻²`, `chi2 = 9` and `logdet_c = 404·ln 2` (ln 2 from zb's rigorous `constant_ball`).
//! COVERS: GlsResult::beta, GlsResult::cov, GlsResult::chi2, GlsResult::logdet_c, gls::gls
//! COVERS: common::la_rows::want_finite

use zenith_bound::array::BallVec;
use zenith_bound::error::BoundError;
use zenith_bound::gls::gls;
use zenith_float_num::ExactNum;

use crate::common::la_rows::{
    diag, dy, fuzzy, hadamard, int, mat, mat_with, mul_exact, ok, pow2, pows, unit_lower, vecb,
    want_encloses, want_err, want_finite, want_k_ln2, want_mat, want_vec, P,
};

const M: usize = 16;
const NP: usize = 4;
const S: [i64; 4] = [0, 3, -5, 60];

struct Row {
    x: Vec<ExactNum>,
    c: Vec<ExactNum>,
    y: Vec<ExactNum>,
    beta: Vec<ExactNum>,
    cov: Vec<ExactNum>,
    k: i64,
}

/// The exact row; `gk` is the per-row exponent of `G`'s diagonal.
fn row(gk: &[i64]) -> Result<Row, String> {
    let g = mul_exact(M, M, M, &unit_lower(M, 8)?, &diag(&pows(gk)?))?;
    let c = mul_exact(M, M, M, &g, &crate::common::la_rows::transpose(M, M, &g)?)?;
    let qs = mul_exact(M, NP, NP, &hadamard(M, &[0, 1, 2, 3])?, &diag(&pows(&S)?))?;
    let x = mul_exact(M, M, NP, &g, &qs)?;
    let beta = vec![int(1), dy(-1, -1)?, int(3), pow2(-7)?];
    let mut yt = mul_exact(M, NP, 1, &qs, &beta)?;
    for (v, h) in yt.iter_mut().zip(hadamard(M, &[4])?.iter()) {
        *v = v.add_full_prec(&h.mul_full_prec(&int(3)));
    }
    let y = mul_exact(M, M, 1, &g, &yt)?;
    let mut inv = Vec::new();
    for s in S {
        inv.push(s.checked_mul(-2).ok_or("cov")?);
    }
    let mut k = 0i64;
    for e in gk {
        k = k.checked_add(e.checked_mul(2).ok_or("k")?).ok_or("k")?;
    }
    Ok(Row {
        x,
        c,
        y,
        beta,
        cov: diag(&pows(&inv)?),
        k,
    })
}

fn gk() -> Vec<i64> {
    let mut v = Vec::new();
    for i in 0..M {
        v.push(if i < 9 { 101 } else { -101 });
    }
    v
}

#[test]
#[ignore = "red: CONTRACT 16.LA4.1: the LA4.2 logdet API on the GLS covariance gives 404·ln2"]
fn la4_logdet_of_the_covariance() -> Result<(), String> {
    let r = row(&gk())?;
    let ld = ok("logdet", mat(M, M, r.c)?.logdet(P))?;
    if ld.sign() != 1 {
        return Err(format!("sign {}", ld.sign()));
    }
    want_k_ln2("ln det C", ld.ln_abs_det(), r.k)
}

#[test]
#[ignore = "red: CONTRACT 16.LA4.2: gls beta/cov/chi2/logdet_c exact on the 16x4 cosmology-scale row"]
fn la4_gls_exact_row() -> Result<(), String> {
    let r = row(&gk())?;
    let g = ok(
        "gls",
        gls(&mat(M, NP, r.x)?, &mat(M, M, r.c)?, &vecb(r.y)?, P),
    )?;
    want_vec("beta", g.beta(), &r.beta)?;
    want_mat("cov", g.cov(), NP, NP, &r.cov)?;
    want_encloses("chi2", g.chi2(), &int(9))?;
    want_k_ln2("logdet_c", g.logdet_c(), r.k)
}

fn is_w(e: &BoundError) -> bool {
    matches!(e, BoundError::DimensionMismatch { detail: "wide", .. })
}
fn is_dm(e: &BoundError) -> bool {
    matches!(e, BoundError::DimensionMismatch { .. })
}
fn is_ns(e: &BoundError) -> bool {
    matches!(
        e,
        BoundError::NotSymmetric {
            detail: "sigma",
            ..
        }
    )
}
fn is_npd(e: &BoundError) -> bool {
    matches!(
        e,
        BoundError::NotPositiveDefinite {
            detail: "sigma",
            ..
        }
    )
}
fn is_rank(e: &BoundError) -> bool {
    matches!(
        e,
        BoundError::Singular {
            detail: "rank_deficient",
            ..
        }
    )
}

#[test]
#[ignore = "red: CONTRACT 16.LA4.3: gls errors in order: prec, wide, c/y shape, NotSymmetric sigma, NotPositiveDefinite sigma"]
fn la4_errors_in_order() -> Result<(), String> {
    let r = row(&gk())?;
    let (x, c, y) = (mat(M, NP, r.x)?, mat(M, M, r.c)?, vecb(r.y)?);
    want_err(
        "prec 0",
        gls(&x, &c, &y, 0),
        |e| matches!(e, BoundError::InvalidArgument { .. }),
        "InvalidArgument",
    )?;
    let wide = mat(2, 3, vec![int(1), int(0), int(0), int(0), int(1), int(0)])?;
    let i2 = mat(2, 2, vec![int(1), int(0), int(0), int(1)])?;
    let y2 = vecb(vec![int(1), int(1)])?;
    want_err(
        "wide",
        gls(&wide, &i2, &y2, P),
        is_w,
        "DimensionMismatch wide",
    )?;
    want_err("c not m×m", gls(&x, &i2, &y, P), is_dm, "DimensionMismatch")?;
    want_err("y length", gls(&x, &c, &y2, P), is_dm, "DimensionMismatch")?;
    let x2 = mat(2, 1, vec![int(1), int(1)])?;
    let nsym_neg = mat(2, 2, vec![int(-1), int(5), int(1), int(-1)])?;
    want_err(
        "NotSymmetric first",
        gls(&x2, &nsym_neg, &y2, P),
        is_ns,
        "NotSymmetric sigma",
    )?;
    let neg = mat(2, 2, vec![int(-1), int(0), int(0), int(1)])?;
    want_err(
        "C = diag(−1, 1)",
        gls(&x2, &neg, &y2, P),
        is_npd,
        "NotPositiveDefinite sigma",
    )
}

#[test]
#[ignore = "red: CONTRACT 16.LA4.4: a duplicate column of X maps the Gram pivot to Singular rank_deficient"]
fn la4_algorithm_rank_deficient() -> Result<(), String> {
    let x = mat(3, 2, vec![int(1), int(1), int(2), int(2), int(-1), int(-1)])?;
    let c = mat(3, 3, diag(&pows(&[203, -203, 0])?))?;
    let y = vecb(vec![int(1), int(2), int(3)])?;
    want_err(
        "duplicate column",
        gls(&x, &c, &y, P),
        is_rank,
        "Singular rank_deficient",
    )
}

#[test]
#[ignore = "red: CONTRACT 16.LA4.5: gls on a fuzzy y (2^-41) and C still encloses the midpoint's beta, cov, chi2"]
fn la4_enclosure_fuzzy() -> Result<(), String> {
    let small = [3, 3, 3, 3, 3, 3, 3, 3, -3, -3, -3, -3, -3, -3, -3, -3];
    let r = row(&small)?;
    let mut ys = Vec::new();
    for v in &r.y {
        ys.push(fuzzy(int(0).add_full_prec(v), -40)?);
    }
    let c = mat_with(M, M, r.c, vec![(0, 0, fuzzy(pow2(6)?, -40)?)])?;
    let g = ok(
        "gls",
        gls(&mat(M, NP, r.x)?, &c, &BallVec::from_balls(ys), P),
    )?;
    want_vec("beta", g.beta(), &r.beta)?;
    want_mat("cov", g.cov(), NP, NP, &r.cov)?;
    want_encloses("chi2", g.chi2(), &int(9))
}

#[test]
#[ignore = "red: CONTRACT 16.LA4.6: post-S3 logdet_c is a finite ball holding 2·Σg·ln2 (strict reading)"]
fn la4_logdet_c_post_s3() -> Result<(), String> {
    let mut gk2 = Vec::new();
    for i in 0..M {
        gk2.push(if i < 2 { 4000 } else { -1 });
    }
    let r = row(&gk2)?;
    let g = ok(
        "gls",
        gls(&mat(M, NP, r.x)?, &mat(M, M, r.c)?, &vecb(r.y)?, P),
    )?;
    want_finite("logdet_c", g.logdet_c())?;
    want_k_ln2("logdet_c", g.logdet_c(), r.k)
}

#[test]
#[ignore = "red: CONTRACT 16.LA4.7: mutant killers 1300 1305 1306 (whitened y, cov = Z·Zᵀ, whitened residual)"]
fn la4_mutant_killers() -> Result<(), String> {
    // C = diag(4, 1/4), G = diag(2, 1/2), X = (2, 1/2)ᵀ: X̃ = (1, 1), W = 2, cov = 1/2. ỹ = X̃·1 + (1, −1) = (2, 0), so
    // β = 1 and chi2 = 2, while raw y = G·ỹ = (4, 0) gives β = 2 (1300) and an unwhitened residual gives 17/4 (1306).
    let c = mat(2, 2, vec![int(4), int(0), int(0), dy(1, -2)?])?;
    let x = mat(2, 1, vec![int(2), dy(1, -1)?])?;
    let y = vecb(vec![int(4), int(0)])?;
    let g = ok("gls", gls(&x, &c, &y, P))?;
    want_vec("1300 beta", g.beta(), &[int(1)])?;
    want_mat("1305 cov", g.cov(), 1, 1, &[dy(1, -1)?])?;
    want_encloses("1306 chi2", g.chi2(), &int(2))
}
