//! PLAN C5 (2026-10-23): sealed `Enclosure` with `prec`, `identical -> bool`, `contains -> Tri`;
//! CONTRACT 6.5 b (identical includes prec), O20. Oracle: hand-built balls of small dyadics.
//! COVERS: Enclosure for Ball, Sealed for Ball

use zenith_bound::ball::Ball;
use zenith_bound::enclosure::Enclosure;
use zenith_bound::mag::Mag;
use zenith_bound::tri::Tri;
use zenith_float_num::{ExactNum, INF_POS, NAN};

const TOP: u64 = 1u64 << 63;

fn n(v: i64) -> ExactNum {
    ExactNum::from(v)
}

/// `v ± 2^k` at `prec`.
fn ball(v: i64, k: i64, prec: usize) -> Result<Ball, String> {
    let rad = Mag::finite(TOP, k.checked_add(1).ok_or("k")?).map_err(|e| format!("{:?}", e))?;
    Ball::from_mid_rad(n(v), rad, prec).map_err(|e| format!("{:?}", e))
}

fn point(v: i64, prec: usize) -> Result<Ball, String> {
    Ball::exact(n(v), prec).map_err(|e| format!("{:?}", e))
}

fn generic_prec<E: Enclosure>(e: &E) -> usize {
    e.prec()
}

#[test]
fn prec_through_the_trait() -> Result<(), String> {
    if generic_prec(&point(3, 77)?) != 77 {
        return Err("prec".into());
    }
    Ok(())
}

#[test]
fn identical_compares_mid_rad_and_prec() -> Result<(), String> {
    let a = ball(5, 0, 64)?;
    let cases = [
        (ball(5, 0, 64)?, true),
        (ball(5, 0, 65)?, false),
        (ball(5, 1, 64)?, false),
        (ball(6, 0, 64)?, false),
        (point(5, 64)?, false),
    ];
    for (i, (b, want)) in cases.iter().enumerate() {
        if a.identical(b) != *want || b.identical(&a) != *want {
            return Err(format!("case {}: identical want {}", i, want));
        }
    }
    let z = Ball::exact(-n(0), 64).map_err(|e| format!("{:?}", e))?;
    if !z.identical(&point(0, 64)?) {
        return Err("-0 and +0 points must be identical after normalisation".into());
    }
    if !Ball::all_reals(9)
        .map_err(|e| format!("{:?}", e))?
        .identical(&Ball::all_reals(9).map_err(|e| format!("{:?}", e))?)
    {
        return Err("all-reals".into());
    }
    Ok(())
}

#[test]
fn contains_decides_exact_cases() -> Result<(), String> {
    // 5 ± 2 = [3, 7]
    let b = ball(5, 1, 64)?;
    let cases: [(i64, Tri); 7] = [
        (5, Tri::True),
        (3, Tri::True),
        (7, Tri::True),
        (2, Tri::False),
        (8, Tri::False),
        (-7, Tri::False),
        (1 << 40, Tri::False),
    ];
    for (x, want) in cases {
        let got = b.contains(&n(x));
        if got != want {
            return Err(format!(
                "5±2 contains {}: want {:?}, got {:?}",
                x, want, got
            ));
        }
    }
    Ok(())
}

#[test]
fn contains_on_points_and_all_reals() -> Result<(), String> {
    let p = point(-9, 64)?;
    if p.contains(&n(-9)) != Tri::True || p.contains(&n(9)) != Tri::False {
        return Err("point ball".into());
    }
    let wide = Ball::exact(ExactNum::from(u64::MAX), 8).map_err(|e| format!("{:?}", e))?;
    if wide.contains(&ExactNum::from(u64::MAX)) != Tri::True {
        return Err("exact wide point must contain itself".into());
    }
    let all = Ball::all_reals(64).map_err(|e| format!("{:?}", e))?;
    if all.contains(&n(i64::MIN)) != Tri::True {
        return Err("all-reals contains every real".into());
    }
    if all.contains(&NAN) != Tri::False || p.contains(&INF_POS) != Tri::False {
        return Err("NaN/Inf are never contained".into());
    }
    Ok(())
}
