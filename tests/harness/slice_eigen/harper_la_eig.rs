//! Harper wide-range eigenvalue rows (2026-10-29) for CONTRACT 16.LA5a (`eig_sym`) and 16.LA5b (`eig`, Gershgorin
//! baseline), written test-first against the stub API at 16384 bits: 32×32 unsorted diagonals spanning −2^203 to 2^-8,
//! the exact 2x2 `[[2,1],[1,2]]` and its fuzzy ball, a non-diagonal input with `max_sweeps = 0`, triangular and companion
//! inputs with known integer spectra, overlapping Gershgorin discs, all-reals entries, and non-square input.
//! COVERS: BallMat::eig, BallMat::eig_sym

use zenith_bound::ball::Ball;
use zenith_bound::cball::CBall;
use zenith_bound::error::BoundError;
use zenith_bound::policy::EigOpts;
use zenith_float_num::ExactNum;

use crate::common::la_rows::{
    diag, dy, fuzzy, i64_of, int, mat, mat_with, ok, pow2, vat, want_all_reals, want_contains,
    want_encloses, want_err, want_vec, P,
};

const N: usize = 32;

/// The ascending values `−2^(203−13j)` for `j < 16`, then `2^(−203+13t)` for `t < 16`.
fn ascending() -> Result<Vec<ExactNum>, String> {
    let mut v = Vec::new();
    for j in 0..16 {
        let e = 203i64
            .checked_sub(i64_of(j)?.checked_mul(13).ok_or("e")?)
            .ok_or("e")?;
        v.push(dy(-1, e)?);
    }
    for t in 0..16 {
        let e = i64_of(t)?
            .checked_mul(13)
            .and_then(|x| x.checked_sub(203))
            .ok_or("e")?;
        v.push(pow2(e)?);
    }
    Ok(v)
}

/// `ascending()` shuffled: entry `i` is value `(5·i) mod 32`.
fn shuffled() -> Result<Vec<ExactNum>, String> {
    let a = ascending()?;
    let mut v = Vec::new();
    for i in 0..N {
        let k = i.checked_mul(5).and_then(|x| x.checked_rem(N)).ok_or("k")?;
        v.push(int(0).add_full_prec(a.get(k).ok_or("a")?));
    }
    Ok(v)
}

fn sweeps(n: usize) -> EigOpts {
    let mut o = EigOpts::default();
    o.max_sweeps = n;
    o
}

fn is_rect(e: &BoundError) -> bool {
    matches!(
        e,
        BoundError::DimensionMismatch {
            detail: "rectangular",
            ..
        }
    )
}

// ---- 16.LA5a ----------------------------------------------------------------------------------------------------------

#[test]
#[ignore = "red: CONTRACT 16.LA5a.1: eig_sym of a 32x32 unsorted diagonal −2^203..2^-8 returns ascending enclosures"]
fn la5a_eig_sym_unsorted_diagonal_32() -> Result<(), String> {
    let a = mat(N, N, diag(&shuffled()?))?;
    want_vec(
        "eig_sym",
        &ok("eig_sym", a.eig_sym(&EigOpts::default(), P))?,
        &ascending()?,
    )
}

#[test]
#[ignore = "red: CONTRACT 16.LA5a.2: rectangular, NotSymmetric \"a\", NotConverged after max_sweeps = 0 on a non-diagonal input"]
fn la5a_errors() -> Result<(), String> {
    let r = mat(2, 3, vec![int(1), int(0), int(0), int(0), int(1), int(0)])?;
    want_err(
        "rectangular",
        r.eig_sym(&EigOpts::default(), P),
        is_rect,
        "DimensionMismatch rectangular",
    )?;
    let ns = mat(2, 2, vec![int(1), int(5), int(1), int(1)])?;
    want_err(
        "NotSymmetric",
        ns.eig_sym(&EigOpts::default(), P),
        |e| matches!(e, BoundError::NotSymmetric { detail: "a", .. }),
        "NotSymmetric a",
    )?;
    let a = mat(2, 2, vec![int(2), int(1), int(1), int(2)])?;
    want_err(
        "max_sweeps 0",
        a.eig_sym(&sweeps(0), P),
        |e| matches!(e, BoundError::NotConverged { .. }),
        "NotConverged",
    )?;
    // a diagonal input meets the stopping rule before any sweep
    want_vec(
        "diagonal, 0 sweeps",
        &ok(
            "eig_sym",
            mat(2, 2, diag(&[int(3), int(-1)]))?.eig_sym(&sweeps(0), P),
        )?,
        &[int(-1), int(3)],
    )
}

#[test]
#[ignore = "red: CONTRACT 16.LA5a.3: [[2,1],[1,2]] gives 1, 3; a 2^-10 diagonal radius widens both by at least 2^-10"]
fn la5a_bound_exact_2x2_and_fuzzy() -> Result<(), String> {
    let a = mat(2, 2, vec![int(2), int(1), int(1), int(2)])?;
    want_vec(
        "exact 2x2",
        &ok("eig_sym", a.eig_sym(&EigOpts::default(), P))?,
        &[int(1), int(3)],
    )?;
    let f = mat_with(
        2,
        2,
        vec![int(2), int(1), int(1), int(2)],
        vec![(0, 0, fuzzy(int(2), -9)?), (1, 1, fuzzy(int(2), -9)?)],
    )?;
    let e = ok("eig_sym fuzzy", f.eig_sym(&EigOpts::default(), P))?;
    // 2·I ± 2^-10·I lies in the ball, so 1 ± 2^-10 and 3 ± 2^-10 are eigenvalues of members
    for (i, c) in [(0, 1), (1, 3)] {
        want_contains("lower", vat(&e, i)?, &int(c).sub_full_prec(&pow2(-10)?))?;
        want_contains("upper", vat(&e, i)?, &int(c).add_full_prec(&pow2(-10)?))?;
    }
    Ok(())
}

#[test]
#[ignore = "red: CONTRACT 16.LA5a.4: an all-reals entry gives all-reals balls, Ok"]
fn la5a_all_reals_entry() -> Result<(), String> {
    let ar = ok("all_reals", Ball::all_reals(P))?;
    let a = mat_with(3, 3, diag(&[int(1), int(2), int(3)]), vec![(2, 0, ar)])?;
    let e = ok("eig_sym", a.eig_sym(&EigOpts::default(), P))?;
    for i in 0..3 {
        want_all_reals(&format!("ball {i}"), vat(&e, i)?)?;
    }
    Ok(())
}

#[test]
#[ignore = "red: CONTRACT 16.LA5a.5: permuted-diagonal rows have exact spectra the replay can decide"]
fn la5a_permuted_diagonal() -> Result<(), String> {
    // P·diag(d)·Pᵀ is diagonal again; with d holding a repeated value, ball i still encloses the i-th smallest
    let d = [int(7), dy(-3, 200)?, int(7), pow2(-203)?, int(0)];
    let a = mat(5, 5, diag(&d))?;
    want_vec(
        "eig_sym",
        &ok("eig_sym", a.eig_sym(&EigOpts::default(), P))?,
        &[dy(-3, 200)?, int(0), pow2(-203)?, int(7), int(7)],
    )
}

#[test]
#[ignore = "red: CONTRACT 16.LA5a.6: mutant killers 1400 1404 1405 1409 1412"]
fn la5a_mutant_killers() -> Result<(), String> {
    // 1400: ρ dropped, an off-diagonal radius 2^-4 around [[1,0],[0,1]] moves eigenvalues to 1 ± 2^-4
    let f = mat_with(
        2,
        2,
        diag(&[int(1), int(1)]),
        vec![(1, 0, fuzzy(int(0), -3)?), (0, 1, fuzzy(int(0), -3)?)],
    )?;
    let e = ok("eig_sym", f.eig_sym(&EigOpts::default(), P))?;
    want_contains("1400 lower", vat(&e, 0)?, &int(1).sub_full_prec(&pow2(-4)?))?;
    want_contains("1400 upper", vat(&e, 1)?, &int(1).add_full_prec(&pow2(-4)?))?;
    // 1404/1405: order is ascending
    want_vec(
        "1404",
        &ok(
            "eig_sym",
            mat(3, 3, diag(&[int(5), int(-5), int(0)]))?.eig_sym(&EigOpts::default(), P),
        )?,
        &[int(-5), int(0), int(5)],
    )?;
    // 1409: max_sweeps = 1 cannot be ignored on a non-diagonal 3x3 that needs more than one sweep
    let a = mat(
        3,
        3,
        vec![
            int(4),
            int(1),
            int(2),
            int(1),
            int(3),
            int(1),
            int(2),
            int(1),
            int(5),
        ],
    )?;
    want_err(
        "1409",
        a.eig_sym(&sweeps(0), P),
        |e| matches!(e, BoundError::NotConverged { .. }),
        "NotConverged",
    )?;
    // 1412: non-square is refused
    want_err(
        "1412",
        mat(1, 2, vec![int(1), int(1)])?.eig_sym(&EigOpts::default(), P),
        is_rect,
        "DimensionMismatch rectangular",
    )
}

// ---- 16.LA5b ----------------------------------------------------------------------------------------------------------

fn want_c(what: &str, z: &CBall, re: &ExactNum) -> Result<(), String> {
    want_encloses(&format!("{what} re"), z.re(), re)?;
    want_encloses(&format!("{what} im"), z.im(), &int(0))
}

fn eigs(a: &zenith_bound::array::BallMat, opts: &EigOpts) -> Result<Box<[CBall]>, String> {
    ok("eig", a.eig(opts, P))
}

fn get(v: &[CBall], i: usize) -> Result<&CBall, String> {
    v.get(i)
        .ok_or_else(|| format!("missing eigenvalue {i} of {}", v.len()))
}

#[test]
#[ignore = "red: CONTRACT 16.LA5b.1: eig of a triangular and a companion matrix encloses each root, sorted by real part"]
fn la5b_triangular_and_companion() -> Result<(), String> {
    let t = mat(
        3,
        3,
        vec![
            int(5),
            pow2(-8)?,
            int(0),
            int(0),
            int(-2),
            pow2(-8)?,
            int(0),
            int(0),
            dy(1, 7)?,
        ],
    )?;
    let e = eigs(&t, &EigOpts::default())?;
    if e.len() != 3 {
        return Err(format!("{} eigenvalues", e.len()));
    }
    for (i, r) in [int(-2), int(5), dy(1, 7)?].iter().enumerate() {
        want_c(&format!("triangular {i}"), get(&e, i)?, r)?;
    }
    // companion of x³ − 6x² + 11x − 6 = (x − 1)(x − 2)(x − 3)
    let c = mat(
        3,
        3,
        vec![
            int(6),
            int(-11),
            int(6),
            int(1),
            int(0),
            int(0),
            int(0),
            int(1),
            int(0),
        ],
    )?;
    let z = eigs(&c, &EigOpts::default())?;
    for (i, r) in [1, 2, 3].iter().enumerate() {
        want_c(&format!("companion {i}"), get(&z, i)?, &int(*r))?;
    }
    Ok(())
}

#[test]
#[ignore = "red: CONTRACT 16.LA5b.2: Gershgorin discs with off-diagonal radii: [[1, 0±1/4],[0±1/4, 1]] holds 3/4 and 5/4"]
fn la5b_gershgorin_baseline() -> Result<(), String> {
    let a = mat_with(
        2,
        2,
        diag(&[int(1), int(1)]),
        vec![(0, 1, fuzzy(int(0), -1)?), (1, 0, fuzzy(int(0), -1)?)],
    )?;
    let e = eigs(&a, &EigOpts::default())?;
    for i in 0..2 {
        want_c(&format!("ball {i} ∋ 3/4"), get(&e, i)?, &dy(3, -2)?)?;
        want_c(&format!("ball {i} ∋ 5/4"), get(&e, i)?, &dy(5, -2)?)?;
    }
    Ok(())
}

#[test]
#[ignore = "red: CONTRACT 16.LA5b.3: a spent iteration budget still returns the baseline, Ok, never NotConverged"]
fn la5b_budget_spent_is_ok() -> Result<(), String> {
    let c = mat(
        3,
        3,
        vec![
            int(6),
            int(-11),
            int(6),
            int(1),
            int(0),
            int(0),
            int(0),
            int(1),
            int(0),
        ],
    )?;
    let z = eigs(&c, &sweeps(0))?;
    for (i, r) in [1, 2, 3].iter().enumerate() {
        want_c(&format!("companion {i}"), get(&z, i)?, &int(*r))?;
    }
    Ok(())
}

#[test]
#[ignore = "red: CONTRACT 16.LA5b.4: rectangular refused; an all-reals entry gives all-complex balls, Ok"]
fn la5b_errors_and_all_reals() -> Result<(), String> {
    want_err(
        "rectangular",
        mat(2, 1, vec![int(1), int(1)])?.eig(&EigOpts::default(), P),
        is_rect,
        "DimensionMismatch rectangular",
    )?;
    let ar = ok("all_reals", Ball::all_reals(P))?;
    let a = mat_with(2, 2, diag(&[int(1), int(2)]), vec![(0, 1, ar)])?;
    let e = eigs(&a, &EigOpts::default())?;
    for (i, z) in e.iter().enumerate() {
        want_all_reals(&format!("re {i}"), z.re())?;
        want_all_reals(&format!("im {i}"), z.im())?;
    }
    if e.len() != 2 {
        return Err(format!("{} eigenvalues", e.len()));
    }
    Ok(())
}

#[test]
#[ignore = "red: CONTRACT 16.LA5b.5: mutant killers 1450 1452 1454 (r_ii, centre sign, Jordan multiplicity)"]
fn la5b_mutant_killers() -> Result<(), String> {
    let a = mat_with(
        2,
        2,
        diag(&[int(-3), int(9)]),
        vec![(0, 0, fuzzy(int(-3), -3)?)],
    )?;
    let e = eigs(&a, &EigOpts::default())?;
    want_c("1452 centre −3", get(&e, 0)?, &int(-3))?;
    want_c(
        "1450 −3 − 2^-4",
        get(&e, 0)?,
        &int(-3).sub_full_prec(&pow2(-4)?),
    )?;
    want_c(
        "1450 −3 + 2^-4",
        get(&e, 0)?,
        &int(-3).add_full_prec(&pow2(-4)?),
    )?;
    let j = mat(2, 2, vec![int(2), int(1), int(0), int(2)])?;
    let z = eigs(&j, &EigOpts::default())?;
    want_c("1454 first", get(&z, 0)?, &int(2))?;
    want_c("1454 second", get(&z, 1)?, &int(2))
}
