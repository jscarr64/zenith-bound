//! CONTRACT 1.3, 14.1, 14.2, 14.3, 14.4; oracle: mpmath.libmp directed rounding (scripts/gen_goldens.py -> tests/goldens/mag_ops.txt); the model checkers are validated against the goldens (accepts them, rejects perturbed ones)
use crate::model::direct;
use crate::model::dyadic::MagV;
use crate::model::oracle::{max_mag, min_pos_mag, pred, succ};
use crate::support::{from_magv, to_magv};
use core::str::SplitWhitespace;

const GOLDEN: &str = include_str!("../goldens/mag_ops.txt");

const OPS: [&str; 8] = [
    "add_up",
    "mul_up",
    "mul_lower",
    "sub_lower",
    "div_up",
    "div_lower",
    "sqrt_up",
    "sqrt_lower",
];

struct Row {
    op: String,
    args: Vec<MagV>,
    want: MagV,
}

fn parse_row(line: &str) -> Result<Option<Row>, String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }

    let mut it = line.split_whitespace();
    let op = it
        .next()
        .ok_or_else(|| String::from("unexpected end of line"))?;
    if !OPS.contains(&op) {
        return Err(format!("unknown op {}", op));
    }

    let a = next_magv(&mut it)?;
    let args = match op {
        "add_up" | "mul_up" | "mul_lower" | "sub_lower" | "div_up" | "div_lower" => {
            let b = next_magv(&mut it)?;
            vec![a, b]
        }
        "sqrt_up" | "sqrt_lower" => {
            vec![a]
        }
        _ => return Err(format!("unknown op {}", op)),
    };

    let want = next_result(&mut it)?;
    Ok(Some(Row {
        op: String::from(op),
        args,
        want,
    }))
}

fn next_i64(it: &mut SplitWhitespace) -> Result<i64, String> {
    it.next()
        .ok_or_else(|| String::from("unexpected end of line"))?
        .parse::<i64>()
        .map_err(|e| format!("failed to parse i64: {}", e))
}

fn next_u64(it: &mut SplitWhitespace) -> Result<u64, String> {
    it.next()
        .ok_or_else(|| String::from("unexpected end of line"))?
        .parse::<u64>()
        .map_err(|e| format!("failed to parse u64: {}", e))
}

fn next_magv(it: &mut SplitWhitespace) -> Result<MagV, String> {
    let exp = next_i64(it)?;
    let mant = next_u64(it)?;
    Ok(MagV::Finite { exp, mant })
}

fn next_result(it: &mut SplitWhitespace) -> Result<MagV, String> {
    let peek = it
        .next()
        .ok_or_else(|| String::from("unexpected end of line"))?;
    if peek == "inf" {
        Ok(MagV::Infinity)
    } else {
        let exp = peek
            .parse::<i64>()
            .map_err(|e| format!("failed to parse i64: {}", e))?;
        let mant = next_u64(it)?;
        Ok(MagV::Finite { exp, mant })
    }
}

fn run_line(line: &str) -> Result<(), String> {
    match parse_row(line)? {
        None => Ok(()),
        Some(row) => {
            let a = row
                .args
                .first()
                .ok_or_else(|| String::from("missing argument a"))?;
            let got_and_want: (MagV, MagV) = match row.op.as_str() {
                "add_up" | "mul_up" | "mul_lower" | "sub_lower" | "div_up" | "div_lower" => {
                    let b = row
                        .args
                        .get(1)
                        .ok_or_else(|| String::from("missing argument b"))?;
                    let ma = from_magv(a)?;
                    let mb = from_magv(b)?;
                    let r = match row.op.as_str() {
                        "add_up" => ma.add_up(&mb),
                        "mul_up" => ma.mul_up(&mb),
                        "mul_lower" => ma.mul_lower(&mb),
                        "sub_lower" => ma.sub_lower(&mb),
                        "div_up" => ma.div_up(&mb),
                        "div_lower" => ma.div_lower(&mb),
                        _ => return Err(format!("unknown op {}", row.op)),
                    };
                    (to_magv(&r)?, row.want)
                }
                "sqrt_up" | "sqrt_lower" => {
                    let ma = from_magv(a)?;
                    let r = if row.op == "sqrt_up" {
                        ma.sqrt_up()
                    } else {
                        ma.sqrt_lower()
                    };
                    (to_magv(&r)?, row.want)
                }
                _ => return Err(format!("unknown op {}", row.op)),
            };

            if got_and_want.0 != got_and_want.1 {
                Err(format!(
                    "{}: expected {:?}, got {:?}",
                    line, got_and_want.1, got_and_want.0
                ))
            } else {
                Ok(())
            }
        }
    }
}

fn verify_row(op: &str, args: &[MagV], r: &MagV) -> Result<(), String> {
    let a = args
        .first()
        .ok_or_else(|| String::from("missing argument a"))?;
    match op {
        "sqrt_up" => crate::sqrt_up::verify(a, r),
        "sqrt_lower" => crate::lower_sqrt::verify(a, r),
        _ => {
            let b = args
                .get(1)
                .ok_or_else(|| String::from("missing argument b"))?;
            match op {
                "add_up" => crate::add_up::verify(a, b, r),
                "mul_up" => crate::mul_up::verify(a, b, r),
                "div_up" => crate::div_up::verify(a, b, r),
                "mul_lower" => crate::lower_mul::verify(a, b, r),
                "sub_lower" => crate::lower_sub::verify(a, b, r),
                "div_lower" => crate::lower_div::verify(a, b, r),
                _ => Err(format!("unknown op {}", op)),
            }
        }
    }
}

fn is_up(op: &str) -> bool {
    matches!(op, "add_up" | "mul_up" | "div_up" | "sqrt_up")
}

fn perturb(op: &str, want: &MagV) -> Option<MagV> {
    if is_up(op) {
        match want {
            MagV::Infinity => Some(max_mag()),
            _ => Some(succ(want)),
        }
    } else {
        match want {
            MagV::Infinity => None,
            MagV::Finite { mant: 0, .. } => Some(min_pos_mag()),
            _ => pred(want),
        }
    }
}

#[test]
fn model_accepts_every_mpmath_golden() -> Result<(), String> {
    for line in GOLDEN.lines() {
        if let Some(row) = parse_row(line)? {
            verify_row(&row.op, &row.args, &row.want).map_err(|e| format!("{}: {}", line, e))?;
        }
    }
    Ok(())
}

#[test]
fn model_rejects_every_perturbed_mpmath_golden() -> Result<(), String> {
    for line in GOLDEN.lines() {
        if let Some(row) = parse_row(line)? {
            if let Some(p) = perturb(&row.op, &row.want) {
                if verify_row(&row.op, &row.args, &p).is_ok() {
                    return Err(format!(
                        "{}: model accepted the perturbed result {:?}",
                        line, p
                    ));
                }
            }
        }
    }
    Ok(())
}

#[test]
fn mag_ops_match_mpmath_goldens() -> Result<(), String> {
    for line in GOLDEN.lines() {
        run_line(line)?;
    }
    Ok(())
}

#[test]
fn golden_file_has_all_eight_ops() -> Result<(), String> {
    for op in OPS.iter() {
        let prefix = format!("{} ", op);
        if !GOLDEN.lines().any(|l| l.starts_with(prefix.as_str())) {
            return Err(format!("missing {}", op));
        }
    }
    Ok(())
}

fn direct_result(op: &str, args: &[MagV]) -> Result<MagV, String> {
    let a = args
        .first()
        .ok_or_else(|| String::from("missing argument a"))?;
    let r = match op {
        "sqrt_up" => direct::sqrt_up(a),
        "sqrt_lower" => direct::sqrt_lower(a),
        _ => {
            let b = args
                .get(1)
                .ok_or_else(|| String::from("missing argument b"))?;
            match op {
                "add_up" => direct::add_up(a, b),
                "mul_up" => direct::mul_up(a, b),
                "mul_lower" => direct::mul_lower(a, b),
                "sub_lower" => direct::sub_lower(a, b),
                "div_up" => direct::div_up(a, b),
                "div_lower" => direct::div_lower(a, b),
                _ => return Err(format!("unknown op {}", op)),
            }
        }
    };
    r.ok_or_else(|| format!("direct oracle failed on {} {:?}", op, args))
}

#[test]
fn direct_integer_oracle_matches_every_mpmath_golden() -> Result<(), String> {
    for line in GOLDEN.lines() {
        if let Some(row) = parse_row(line)? {
            let got = direct_result(&row.op, &row.args)?;
            if got != row.want {
                return Err(format!(
                    "{}: direct oracle gave {:?}, mpmath golden is {:?}",
                    line, got, row.want
                ));
            }
        }
    }
    Ok(())
}
