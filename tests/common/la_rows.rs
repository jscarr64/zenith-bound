//! Harper wide-range LA rows (2026-10-29): exact dyadic builders and rigorous checks shared by the `harper_la_*` harness
//! modules (slice_matrix, slice_lu, slice_gls, slice_eigen). Every value a row expects is built exactly here (dyadic
//! products and sums at full precision), or is enclosed rigorously by zb's own `bridge::constant_ball(Ln2)`. Checks return
//! `Err`, never panic.
use core::fmt::Debug;

use zenith_bound::array::{BallMat, BallVec};
use zenith_bound::ball::Ball;
use zenith_bound::bridge::{constant_ball, Constant};
use zenith_bound::enclosure::Enclosure;
use zenith_bound::error::BoundError;
use zenith_bound::mag::Mag;
use zenith_bound::tri::Tri;
use zenith_float_num::{Consts, ExactNum, RawKind, RawParts};

use crate::common::generators::TOP;

/// The working precision of the wide rows (16384 bits).
pub const P: usize = 16384;

/// `r` as a `String` error naming `what`.
pub fn ok<T>(what: &str, r: Result<T, BoundError>) -> Result<T, String> {
    r.map_err(|e| format!("{what}: unexpected {e:?}"))
}

/// `2^k` exactly, for `k` in `EXPONENT_MIN - 1 ..= EXPONENT_MAX - 1`.
pub fn pow2(k: i64) -> Result<ExactNum, String> {
    let e = k.checked_add(1).ok_or("pow2: exponent overflows")?;
    ExactNum::from_mag_parts(TOP, e, 64).map_err(|err| format!("pow2({k}): {err:?}"))
}

/// `m·2^k` exactly.
pub fn dy(m: i64, k: i64) -> Result<ExactNum, String> {
    Ok(ExactNum::from(m).mul_full_prec(&pow2(k)?))
}

/// The integer `v` as an exact number.
pub fn int(v: i64) -> ExactNum {
    ExactNum::from(v)
}

/// `-0` as an exact number (CONTRACT 6.3: a ball built on it has the midpoint `+0`).
pub fn neg_zero() -> Result<ExactNum, String> {
    let words = [0u64];
    ExactNum::from_raw_parts(&RawParts {
        kind: RawKind::Zero,
        words: &words,
        precision: 64,
        negative: true,
        exponent: 0,
        inexact: false,
        op_inexact: false,
    })
    .map_err(|e| format!("neg_zero: {e:?}"))
}

/// `i` as an `i64`.
pub fn i64_of(i: usize) -> Result<i64, String> {
    i64::try_from(i).map_err(|e| format!("index {i}: {e:?}"))
}

/// The exact ball `x` at `P` bits.
pub fn pt(x: ExactNum) -> Result<Ball, String> {
    ok("Ball::exact", Ball::exact(x, P))
}

/// The ball `x ± 2^(k-1)` at `P` bits.
pub fn fuzzy(x: ExactNum, k: i64) -> Result<Ball, String> {
    let r = Mag::finite(TOP, k).map_err(|e| format!("Mag::finite(2^{k}): {e:?}"))?;
    ok("Ball::from_mid_rad", Ball::from_mid_rad(x, r, P))
}

/// An `n×m` matrix of exact balls from row-major exact values.
pub fn mat(n: usize, m: usize, vals: Vec<ExactNum>) -> Result<BallMat, String> {
    let mut balls = Vec::new();
    for v in vals {
        balls.push(pt(v)?);
    }
    ok("BallMat::from_balls", BallMat::from_balls(n, m, balls))
}

/// A vector of exact balls.
pub fn vecb(vals: Vec<ExactNum>) -> Result<BallVec, String> {
    let mut balls = Vec::new();
    for v in vals {
        balls.push(pt(v)?);
    }
    Ok(BallVec::from_balls(balls))
}

/// Row-major cell `(i, j)` of an `·×m` value grid.
pub fn cell(v: &[ExactNum], m: usize, i: usize, j: usize) -> Result<&ExactNum, String> {
    let k = i
        .checked_mul(m)
        .and_then(|x| x.checked_add(j))
        .ok_or("cell index overflows")?;
    v.get(k)
        .ok_or_else(|| format!("cell ({i},{j}) outside {}", v.len()))
}

/// The exact product of the row-major `n×k` grid `a` and the `k×m` grid `b`.
pub fn mul_exact(
    n: usize,
    k: usize,
    m: usize,
    a: &[ExactNum],
    b: &[ExactNum],
) -> Result<Vec<ExactNum>, String> {
    let mut c = Vec::new();
    for i in 0..n {
        for j in 0..m {
            let mut s = int(0);
            for t in 0..k {
                s = s.add_full_prec(&cell(a, k, i, t)?.mul_full_prec(cell(b, m, t, j)?));
            }
            c.push(s);
        }
    }
    Ok(c)
}

/// The transpose of the row-major `n×m` grid `a`.
pub fn transpose(n: usize, m: usize, a: &[ExactNum]) -> Result<Vec<ExactNum>, String> {
    let mut t = Vec::new();
    for j in 0..m {
        for i in 0..n {
            t.push(ExactNum::from(0).add_full_prec(cell(a, m, i, j)?));
        }
    }
    Ok(t)
}

/// `diag(d)` as an `n×n` grid.
pub fn diag(d: &[ExactNum]) -> Vec<ExactNum> {
    let mut g = Vec::new();
    for (i, _) in d.iter().enumerate() {
        for (j, dj) in d.iter().enumerate() {
            g.push(if i == j {
                int(0).add_full_prec(dj)
            } else {
                int(0)
            });
        }
    }
    g
}

/// A unit-lower dyadic `n×n` grid: `l_ij ∈ {−1, −1/2, 0, 1/2, 1}` below the diagonal, from `(3i + 5j + seed) mod 5`.
pub fn unit_lower(n: usize, seed: usize) -> Result<Vec<ExactNum>, String> {
    let mut g = Vec::new();
    for i in 0..n {
        for j in 0..n {
            let v = if i == j {
                int(1)
            } else if j > i {
                int(0)
            } else {
                let s = i
                    .checked_mul(3)
                    .and_then(|x| j.checked_mul(5).and_then(|y| x.checked_add(y)))
                    .and_then(|x| x.checked_add(seed))
                    .ok_or("unit_lower index")?;
                let r = i64_of(s.checked_rem(5).ok_or("rem")?)?;
                dy(r.checked_sub(2).ok_or("sub")?, -1)?
            };
            g.push(v);
        }
    }
    Ok(g)
}

/// `L·diag(d)·Lᵀ` exactly, for an `n×n` grid `l`.
pub fn ldlt(n: usize, l: &[ExactNum], d: &[ExactNum]) -> Result<Vec<ExactNum>, String> {
    let ld = mul_exact(n, n, n, l, &diag(d))?;
    mul_exact(n, n, n, &ld, &transpose(n, n, l)?)
}

/// `Σ k_i` with overflow checked.
pub fn sum_i64(ks: &[i64]) -> Result<i64, String> {
    let mut s = 0i64;
    for k in ks {
        s = s.checked_add(*k).ok_or("exponent sum overflows")?;
    }
    Ok(s)
}

/// `2^k` for each `k`.
pub fn pows(ks: &[i64]) -> Result<Vec<ExactNum>, String> {
    let mut v = Vec::new();
    for k in ks {
        v.push(pow2(*k)?);
    }
    Ok(v)
}

/// Entry `(i, j)` of `a`.
pub fn at(a: &BallMat, i: usize, j: usize) -> Result<&Ball, String> {
    a.get(i, j)
        .ok_or_else(|| format!("missing entry ({i},{j}) of a {}x{}", a.nrows(), a.ncols()))
}

/// Entry `i` of `v`.
pub fn vat(v: &BallVec, i: usize) -> Result<&Ball, String> {
    v.get(i)
        .ok_or_else(|| format!("missing entry {i} of a length-{} vector", v.len()))
}

/// `b` contains `x` (decided `True`).
pub fn want_contains(what: &str, b: &Ball, x: &ExactNum) -> Result<(), String> {
    match b.contains(x) {
        Tri::True => Ok(()),
        t @ (Tri::False | Tri::Unknown) => {
            Err(format!("{what}: {b:?} does not contain {x:?} ({t:?})"))
        }
    }
}

/// `b` has a finite radius.
pub fn want_finite(what: &str, b: &Ball) -> Result<(), String> {
    if b.rad().is_infinity() {
        return Err(format!(
            "{what}: {b:?} is all-reals, want a finite enclosure"
        ));
    }
    Ok(())
}

/// `b` is a finite ball that contains `x`.
pub fn want_encloses(what: &str, b: &Ball, x: &ExactNum) -> Result<(), String> {
    want_finite(what, b)?;
    want_contains(what, b, x)
}

/// `b` is the all-reals ball (clause 10.4).
pub fn want_all_reals(what: &str, b: &Ball) -> Result<(), String> {
    if b.rad().is_infinity() {
        return Ok(());
    }
    Err(format!("{what}: {b:?} should be all-reals"))
}

/// `b` contains 0 and its midpoint is `+0` (CONTRACT 6.3).
pub fn want_pos_zero(what: &str, b: &Ball) -> Result<(), String> {
    want_encloses(what, b, &int(0))?;
    if b.mid().is_negative() {
        return Err(format!("{what}: {b:?} has a -0 midpoint"));
    }
    Ok(())
}

/// `v` has length `xs.len()` and entry `i` encloses `xs[i]`.
pub fn want_vec(what: &str, v: &BallVec, xs: &[ExactNum]) -> Result<(), String> {
    if v.len() != xs.len() {
        return Err(format!("{what}: length {} want {}", v.len(), xs.len()));
    }
    for (i, x) in xs.iter().enumerate() {
        want_encloses(&format!("{what}[{i}]"), vat(v, i)?, x)?;
    }
    Ok(())
}

/// `a` is `n×m` and entry `(i, j)` encloses the row-major `xs`.
pub fn want_mat(
    what: &str,
    a: &BallMat,
    n: usize,
    m: usize,
    xs: &[ExactNum],
) -> Result<(), String> {
    if a.nrows() != n || a.ncols() != m {
        return Err(format!(
            "{what}: shape {}x{} want {n}x{m}",
            a.nrows(),
            a.ncols()
        ));
    }
    for i in 0..n {
        for j in 0..m {
            want_encloses(
                &format!("{what}[{i},{j}]"),
                at(a, i, j)?,
                cell(xs, m, i, j)?,
            )?;
        }
    }
    Ok(())
}

/// `r` is an error that `want` accepts (variant and detail); `label` names it.
pub fn want_err<T: Debug>(
    what: &str,
    r: Result<T, BoundError>,
    want: fn(&BoundError) -> bool,
    label: &str,
) -> Result<(), String> {
    if let Err(e) = &r {
        if want(e) {
            return Ok(());
        }
    }
    Err(format!("{what}: want {label}, got {r:?}"))
}

/// An enclosure of `k·ln 2` at `prec` bits from zb's rigorous ln 2 (`bridge::constant_ball`, 16.S2.6).
pub fn k_ln2(k: i64, prec: usize) -> Result<Ball, String> {
    let mut cc = Consts::new().map_err(|e| format!("Consts::new: {e:?}"))?;
    let l = ok(
        "constant_ball(Ln2)",
        constant_ball(&mut cc, &Constant::Ln2, prec),
    )?;
    let kb = ok("Ball::exact(k)", Ball::exact(int(k), prec))?;
    ok("k·ln2", l.mul(&kb, prec))
}

/// Whether `b` covers both ends of `e`.
fn covers(b: &Ball, e: &Ball) -> Result<Tri, String> {
    let r = e
        .rad()
        .to_exact_num()
        .map_err(|err| format!("ln2 radius: {err:?}"))?;
    let lo = e.mid().sub_full_prec(&r);
    let hi = e.mid().add_full_prec(&r);
    Ok(match (b.contains(&lo), b.contains(&hi)) {
        (Tri::True, Tri::True) => Tri::True,
        (Tri::False, _) | (_, Tri::False) => Tri::False,
        (Tri::Unknown, _) | (_, Tri::Unknown) => Tri::Unknown,
    })
}

/// `b` is a finite ball holding `k·ln 2`, decided under 16.LA.5: `b` must cover a rigorous enclosure `E` of `k·ln 2` at
/// `2·P + 64` bits; a `b` that overlaps `E` without covering it is re-decided once against `E` at `8·P` bits, and a `b`
/// disjoint from `E` fails.
pub fn want_k_ln2(what: &str, b: &Ball, k: i64) -> Result<(), String> {
    want_finite(what, b)?;
    let first = P
        .checked_mul(2)
        .and_then(|p| p.checked_add(64))
        .ok_or("prec")?;
    let second = P.checked_mul(8).ok_or("prec")?;
    for prec in [first, second] {
        let e = k_ln2(k, prec)?;
        if covers(b, &e)? == Tri::True {
            return Ok(());
        }
        if b.overlaps(&e) == Tri::False {
            return Err(format!("{what}: {b:?} is disjoint from {k}·ln2 = {e:?}"));
        }
    }
    Err(format!(
        "{what}: {b:?} does not cover {k}·ln2 even at {second} bits"
    ))
}

/// The `n×m` ball matrix of the exact `grid`, with the listed entries `(i, j, b)` replaced by balls.
pub fn mat_with(
    n: usize,
    m: usize,
    grid: Vec<ExactNum>,
    repl: Vec<(usize, usize, Ball)>,
) -> Result<BallMat, String> {
    let mut balls = Vec::new();
    for v in grid {
        balls.push(pt(v)?);
    }
    for (i, j, b) in repl {
        let k = i
            .checked_mul(m)
            .and_then(|x| x.checked_add(j))
            .ok_or("mat_with index")?;
        let slot = balls
            .get_mut(k)
            .ok_or_else(|| format!("mat_with: ({i},{j}) outside {n}x{m}"))?;
        *slot = b;
    }
    ok("BallMat::from_balls", BallMat::from_balls(n, m, balls))
}

/// Columns `cols` of the Sylvester Hadamard matrix of order `m` (a power of two) scaled to orthonormal columns: entry
/// `(i, j)` is `(−1)^popcount(i & j)·2^(−log2(m)/2)`; `m` must be a power of four. Row-major `m×cols.len()`.
pub fn hadamard(m: usize, cols: &[usize]) -> Result<Vec<ExactNum>, String> {
    let half = i64::from(m.trailing_zeros())
        .checked_div(2)
        .and_then(i64::checked_neg)
        .ok_or("hadamard")?;
    let mut g = Vec::new();
    for i in 0..m {
        for j in cols {
            let s = if (i & *j).count_ones().checked_rem(2) == Some(0) {
                1
            } else {
                -1
            };
            g.push(dy(s, half)?);
        }
    }
    Ok(g)
}

/// The exact midpoints of `a`, row-major.
pub fn mids(a: &BallMat) -> Result<Vec<ExactNum>, String> {
    let mut g = Vec::new();
    for i in 0..a.nrows() {
        for j in 0..a.ncols() {
            g.push(int(0).add_full_prec(at(a, i, j)?.mid()));
        }
    }
    Ok(g)
}
