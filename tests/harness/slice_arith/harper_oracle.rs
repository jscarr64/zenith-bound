//! Harper L0b: the slice oracle rows (tests/goldens/slices/arith/arith_oracle_rows.txt, scripts/gen_arith_oracle.py).
//! Each row is an input ball pair at PREC and the exact image rounded outward to W = 2·PREC + 64 bits ([LO, HI],
//! cross-checked with mpmath.iv). Checks per row: [LO, HI] ⊆ result (containment); rad ≤ τ·RREF (tight); rad ≤ 2^10·RREF
//! (loose), RREF = hull spread + U (TEST_PLAN 9, placeholder τ). Radii compare as exact numbers; a radius that does not
//! convert, or a row that does not parse, fails the test (never skipped).

use core::cmp::Ordering;

use zenith_bound::ball::Ball;
use zenith_bound::mag::Mag;
use zenith_bound::policy::DomainPolicy;
use zenith_float_num::ExactNum;

use super::{num, ok, row_val, want_contains};
use crate::common::goldens::golden_dir;

/// Rows the generator writes: 11 ops × 124 (add sub mul mul_exact sqr powi sqrt neg div inv hypot). A short file fails, so the check cannot pass on a truncated golden.
const WANT_ROWS: usize = 1364;
const LAMBDA: i64 = 1024;

fn field<'a>(v: &[&'a str], i: usize) -> Result<&'a str, String> {
    match v.get(i) {
        Some(&s) => Ok(s),
        None => Err(format!("missing field {}", i)),
    }
}

fn value(v: &[&str], at: usize) -> Result<ExactNum, String> {
    let s = field(v, at)?;
    let e = field(v, at.checked_add(1).ok_or("at")?)?;
    let m = field(v, at.checked_add(2).ok_or("at")?)?;
    row_val(s, e, m)
}

/// Placeholder τ (TEST_PLAN 9): add/sub 2, mul/mul_exact/sqr/sqrt/div/inv/hypot 4, powi 2n, neg 1.
fn tau(op: &str, n: u64) -> Result<i64, String> {
    match op {
        "add" | "sub" => Ok(2),
        "mul" | "mul_exact" | "sqr" | "sqrt" | "div" | "inv" | "hypot" => Ok(4),
        "neg" => Ok(1),
        "powi" => i64::try_from(n)
            .ok()
            .and_then(|k| k.checked_mul(2))
            .map(|k| k.max(1))
            .ok_or_else(|| format!("powi n {}", n)),
        other => Err(format!("unknown op {}", other)),
    }
}

/// `rad <= k·rref`, exactly. An infinite radius or one that does not convert is never within a bound.
fn within(rad: &Mag, k: i64, rref: &ExactNum) -> Result<bool, String> {
    if rad.is_infinity() {
        return Ok(false);
    }
    let r = rad
        .to_exact_num()
        .map_err(|e| format!("rad to exact: {:?}", e))?;
    match r.partial_cmp(&rref.mul_full_prec(&num(k))) {
        Some(Ordering::Less | Ordering::Equal) => Ok(true),
        Some(Ordering::Greater) => Ok(false),
        None => Err("rad vs bound unordered".into()),
    }
}

fn run(op: &str, prec: usize, n: u64, a: Ball, b: &Ball, bmid: &ExactNum) -> Result<Ball, String> {
    match op {
        "add" => ok("add", a.add(b, prec)),
        "sub" => ok("sub", a.sub(b, prec)),
        "mul" => ok("mul", a.mul(b, prec)),
        "mul_exact" => ok("mul_exact", a.mul_exact(bmid, prec)),
        "sqr" => ok("sqr", a.sqr(prec)),
        "powi" => ok("powi", a.powi(n, prec)),
        "neg" => Ok(-a),
        "sqrt" => ok("sqrt", a.sqrt(prec, &DomainPolicy::Clip)),
        "div" => ok("div", a.div(b, prec)),
        "inv" => ok("inv", a.inv(prec)),
        "hypot" => ok("hypot", a.hypot(b, prec)),
        other => Err(format!("unknown op {}", other)),
    }
}

fn check_row(line: &str) -> Result<(), String> {
    let v: Vec<&str> = line.split_whitespace().collect();
    if v.len() != 25 {
        return Err(format!("{} fields", v.len()));
    }
    let op = field(&v, 0)?;
    let prec: usize = field(&v, 1)?.parse().map_err(|_| "prec")?;
    let n: u64 = field(&v, 3)?.parse().map_err(|_| "n")?;
    let amid = value(&v, 4)?;
    let arad = Mag::from_exact_num(&value(&v, 7)?);
    let bmid = value(&v, 10)?;
    let brad = Mag::from_exact_num(&value(&v, 13)?);
    let lo = value(&v, 16)?;
    let hi = value(&v, 19)?;
    let rref = value(&v, 22)?;
    let a = ok(
        "a",
        Ball::from_mid_rad(amid.add_full_prec(&num(0)), arad, prec),
    )?;
    let b = ok(
        "b",
        Ball::from_mid_rad(bmid.add_full_prec(&num(0)), brad, prec),
    )?;
    // Strict must agree with Clip wherever the input ball lies in [0, inf).
    let strict_twin = if op == "sqrt" {
        let rn = a
            .rad()
            .to_exact_num()
            .map_err(|e| format!("rad: {:?}", e))?;
        match amid.sub_full_prec(&rn).partial_cmp(&num(0)) {
            Some(Ordering::Less) => None,
            Some(Ordering::Equal | Ordering::Greater) => Some(ok(
                "a2",
                Ball::from_mid_rad(amid.add_full_prec(&num(0)), Mag::from_exact_num(&rn), prec),
            )?),
            None => return Err("sqrt row: cannot order m - r against 0".into()),
        }
    } else {
        None
    };
    let res = run(op, prec, n, a, &b, &bmid)?;
    want_contains("LO", &res, &lo)?;
    want_contains("HI", &res, &hi)?;
    let t = tau(op, n)?;
    if !within(res.rad(), t.checked_mul(LAMBDA).ok_or("loose k")?, &rref)? {
        return Err(format!(
            "LOOSE: rad {:?} > 2^10·{}·RREF {:?}",
            res.rad(),
            t,
            rref
        ));
    }
    if !within(res.rad(), t, &rref)? {
        return Err(format!(
            "TIGHT: rad {:?} > {}·RREF {:?}",
            res.rad(),
            t,
            rref
        ));
    }
    if let Some(a2) = strict_twin {
        let s = ok("sqrt Strict", a2.sqrt(prec, &DomainPolicy::Strict))?;
        if s.mid() != res.mid() || s.rad() != res.rad() {
            return Err(format!(
                "Strict {:?} != Clip {:?} on a nonnegative ball",
                s, res
            ));
        }
    }
    Ok(())
}

#[test]
fn oracle_rows_contained_tight_loose() -> Result<(), String> {
    let path = golden_dir()
        .join("slices")
        .join("arith")
        .join("arith_oracle_rows.txt");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{:?}: {}", path, e))?;
    let mut rows = 0usize;
    let mut bad: Vec<String> = Vec::new();
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        rows = rows.checked_add(1).ok_or("rows")?;
        if let Err(e) = check_row(line) {
            bad.push(format!(
                "row {}: {}\n    row: {}",
                rows,
                e,
                line.get(..160).unwrap_or(line)
            ));
        }
    }
    if rows != WANT_ROWS {
        return Err(format!("{} rows, want {}", rows, WANT_ROWS));
    }
    if !bad.is_empty() {
        let shown: Vec<&String> = bad.iter().take(12).collect();
        return Err(format!(
            "{} of {} oracle rows fail:\n{:#?}",
            bad.len(),
            rows,
            shown
        ));
    }
    Ok(())
}

/// The two hypot rows (data rows 1313 and 1343, 1-based, comments skipped) whose radius exceeds 4·RREF under the
/// 1-Lipschitz `r + s` radius: the second argument's ball straddles 0 widely, so the true spread is far below `s`.
/// Spence's monotone enclosure (hypot(min|X|, min|Y|) .. hypot(max|X|, max|Y|)) is to make them pass; RED until then.
const HYPOT_WIDE_ROWS: [usize; 2] = [1313, 1343];

#[test]
fn hypot_wide_rows_1313_1343_tight() -> Result<(), String> {
    let path = golden_dir()
        .join("slices")
        .join("arith")
        .join("arith_oracle_rows.txt");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{:?}: {}", path, e))?;
    let data: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    let mut bad: Vec<String> = Vec::new();
    for id in HYPOT_WIDE_ROWS {
        let line = data
            .get(id.checked_sub(1).ok_or("row id 0")?)
            .ok_or_else(|| format!("row {} missing", id))?;
        if !line.starts_with("hypot ") {
            return Err(format!("row {} is not a hypot row: {}", id, line));
        }
        if let Err(e) = check_row(line) {
            bad.push(format!("row {}: {}", id, e));
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!("{:#?}", bad))
    }
}
