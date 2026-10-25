//! Slice S2 harness rows (`src/bridge.rs`, `src/mag/convert.rs`). Owned by slice S2 alone.
//! Goldens: tests/goldens/slices/bridge/ (its own SHA256SUMS and GENERATORS). Mutants: tools/mutants/manifest.d/bridge.txt, ids 300..=399.
//! Golden file headers for this slice are declared here, not in tests/common/goldens.rs.
//! CONTRACT 16.S2.1 to 16.S2.6, tested per 16.S2.8. Oracles: the exact integer model tests/model/bridge.rs (M1 extension: signed
//! dyadic terms, exponent-gap clustering, top-64 + sticky rounding), itself cross-checked against Python `fractions.Fraction` on every
//! row of tests/goldens/slices/bridge/bridge_rows.txt (scripts/gen_bridge_rows.py); zenith-float's exact `abs_cmp` for P9.
//! The tests/mag/convert.rs rows (zero, NaN/Inf, sticky, carry, huge and subnormal exponents, to_exact_num rejects) stay there and are
//! cited, not duplicated; the conversion rows here add the bit-width, range-edge and model-checked rows.
//! COVERS: Mag::cmp_exact_num, Mag::cmp_exact_diff

use core::cmp::Ordering;

use zenith_bound::error::BoundError;
use zenith_bound::mag::Mag;
use zenith_float_num::{ExactNum, EXPONENT_MAX, EXPONENT_MIN, INF_NEG, INF_POS, NAN};

use crate::common::goldens::golden_dir;
use crate::common::rng::SplitMix64;

#[path = "../model/bridge.rs"]
mod model_bridge;

use model_bridge::{cmp_diff, cmp_num, round_down, round_up, top64, Term};

const TOP: u64 = 0x8000_0000_0000_0000;
const ONES: u64 = u64::MAX;
/// Property cases per test (gate tier).
const CASES: u32 = 2000;

/// `±Σ_k words[k] · 2^(exp − 64(k+1))`; no words is zero (the golden-row number token).
struct Num {
    neg: bool,
    words: Vec<u64>,
    exp: i64,
}

fn num(neg: bool, words: &[u64], exp: i64) -> Num {
    let mut w = Vec::with_capacity(words.len());
    for x in words {
        w.push(*x);
    }
    Num { neg, words: w, exp }
}

fn zero_num() -> Num {
    num(false, &[], 0)
}

/// `n` as an `ExactNum` at precision `p`, built exactly from one `from_mag_parts` per word.
fn exact_p(n: &Num, p: usize) -> Result<ExactNum, String> {
    let mut acc = ExactNum::new(p);
    let mut e = n.exp;
    for w in &n.words {
        if *w != 0 {
            let t = ExactNum::from_mag_parts(*w, e, p)
                .map_err(|err| format!("from_mag_parts({:x}, {}, {}): {:?}", w, e, p, err))?;
            acc = if acc.is_zero() {
                t
            } else {
                acc.add_full_prec(&t)
            };
            if acc.is_zero() {
                return Err(format!(
                    "zf could not hold the exact sum at word exp {} (subnormal flush)",
                    e
                ));
            }
        }
        e = e.checked_sub(64).ok_or("word exponent underflow")?;
    }
    if n.neg {
        acc.inv_sign();
    }
    Ok(acc)
}

fn exact(n: &Num) -> Result<ExactNum, String> {
    exact_p(n, 64 * (n.words.len() + 1))
}

fn term(n: &Num) -> Term {
    let (neg, exp) = (n.neg, n.exp);
    Term::words(neg, &n.words, i128::from(exp))
}

fn mag(mant: u64, exp: i64) -> Result<Mag, String> {
    Mag::finite(mant, exp).map_err(|e| format!("Mag::finite({}, {}): {:?}", mant, exp, e))
}

/// The model term of a finite `Mag` (`None` for `Infinity`).
fn mag_term(m: &Mag) -> Option<Term> {
    let mant = m.mant()?;
    let e = m.exp()?;
    Some(Term::mag(mant, i128::from(e)))
}

fn mag_str(m: &Mag) -> String {
    format!("{:?}/{:?}", m.mant(), m.exp())
}

fn want_parts(what: &str, m: &Mag, want: Option<(u64, i128)>) -> Result<(), String> {
    let got = match (m.mant(), m.exp()) {
        (Some(0), _) => None,
        (Some(a), Some(e)) => Some((a, i128::from(e))),
        (None, _) | (_, None) => return Err(format!("{}: got Infinity, want {:?}", what, want)),
    };
    if got == want {
        Ok(())
    } else {
        Err(format!("{}: got {}, want {:?}", what, mag_str(m), want))
    }
}

// ---------------------------------------------------------------------------------------------------------------- golden rows

fn parse_i64(s: &str) -> Result<i64, String> {
    s.parse::<i64>()
        .map_err(|e| format!("bad integer {:?}: {}", s, e))
}

fn parse_num(tok: &str) -> Result<Num, String> {
    if tok == "0" {
        return Ok(zero_num());
    }
    let (sign, rest) = tok.split_at(1);
    let neg = match sign {
        "+" => false,
        "-" => true,
        _ => return Err(format!("bad number token {:?}", tok)),
    };
    let rest = rest
        .strip_prefix('e')
        .ok_or(format!("bad number token {:?}", tok))?;
    let (e, ws) = rest
        .split_once(':')
        .ok_or(format!("bad number token {:?}", tok))?;
    let mut words = Vec::new();
    for w in ws.split('.') {
        words.push(u64::from_str_radix(w, 16).map_err(|err| format!("bad word {:?}: {}", w, err))?);
    }
    Ok(Num {
        neg,
        words,
        exp: parse_i64(e)?,
    })
}

fn parse_mag(tok: &str) -> Result<Mag, String> {
    if tok == "0" {
        return Ok(Mag::zero());
    }
    let rest = tok
        .strip_prefix('m')
        .ok_or(format!("bad mag token {:?}", tok))?;
    let (m, e) = rest
        .split_once('e')
        .ok_or(format!("bad mag token {:?}", tok))?;
    mag(
        m.parse::<u64>()
            .map_err(|err| format!("bad mant {:?}: {}", m, err))?,
        parse_i64(e)?,
    )
}

fn parse_ord(tok: &str) -> Result<Ordering, String> {
    match tok {
        "L" => Ok(Ordering::Less),
        "E" => Ok(Ordering::Equal),
        "G" => Ok(Ordering::Greater),
        _ => Err(format!("bad ordering {:?}", tok)),
    }
}

fn parse_parts(m: &str, e: &str) -> Result<Option<(u64, i128)>, String> {
    let mant = m
        .parse::<u64>()
        .map_err(|err| format!("bad mant {:?}: {}", m, err))?;
    let exp = i128::from(parse_i64(e)?);
    Ok(if mant == 0 { None } else { Some((mant, exp)) })
}

/// Which side a golden row is checked against.
#[derive(PartialEq)]
enum Side {
    Model,
    Code,
}

fn check_row(line: &str, side: &Side) -> Result<(), String> {
    let f: Vec<&str> = line.split_whitespace().collect();
    match f.as_slice() {
        ["CONV", x, um, ue, lm, le] => {
            let (n, up, lo) = (parse_num(x)?, parse_parts(um, ue)?, parse_parts(lm, le)?);
            if *side == Side::Model {
                let t = term(&n);
                return if round_up(&t) == up && round_down(&t) == lo {
                    Ok(())
                } else {
                    Err(format!(
                        "model up {:?} down {:?}",
                        round_up(&t),
                        round_down(&t)
                    ))
                };
            }
            let xe = exact(&n)?;
            want_parts("from_exact_num", &Mag::from_exact_num(&xe), up)?;
            want_parts("from_exact_num_lower", &Mag::from_exact_num_lower(&xe), lo)
        }
        ["CMPN", r, x, o] => {
            let (r, n, want) = (parse_mag(r)?, parse_num(x)?, parse_ord(o)?);
            let got = if *side == Side::Model {
                cmp_num(mag_term(&r).ok_or("Infinity in a golden row")?, term(&n))
            } else {
                r.cmp_exact_num(&exact(&n)?)
            };
            if got == Some(want) {
                Ok(())
            } else {
                Err(format!("got {:?}, want {:?}", got, want))
            }
        }
        ["CMPD", a, b, r, s, o] => {
            let (na, nb, r, s, want) = (
                parse_num(a)?,
                parse_num(b)?,
                parse_mag(r)?,
                parse_mag(s)?,
                parse_ord(o)?,
            );
            let got = if *side == Side::Model {
                cmp_diff(
                    term(&na),
                    term(&nb),
                    mag_term(&r).ok_or("Infinity in a golden row")?,
                    mag_term(&s).ok_or("Infinity in a golden row")?,
                )
            } else {
                Mag::cmp_exact_diff(&exact(&na)?, &exact(&nb)?, &r, &s)
            };
            if got == Some(want) {
                Ok(())
            } else {
                Err(format!("got {:?}, want {:?}", got, want))
            }
        }
        _ => Err("unknown row".to_string()),
    }
}

fn check_golden(side: &Side) -> Result<(), String> {
    let path = golden_dir()
        .join("slices")
        .join("bridge")
        .join("bridge_rows.txt");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let (mut rows, mut bad, mut first) = (0u32, 0u32, Vec::new());
    for (i, line) in text.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        rows += 1;
        if let Err(e) = check_row(line, side) {
            bad += 1;
            if first.len() < 8 {
                first.push(format!("line {}: {}: {}", i + 1, line, e));
            }
        }
    }
    if rows < 600 {
        return Err(format!("only {} golden rows", rows));
    }
    if bad == 0 {
        Ok(())
    } else {
        Err(format!(
            "{} of {} rows disagree:\n{}",
            bad,
            rows,
            first.join("\n")
        ))
    }
}

/// Model selftest (16.S2.8): the exact model reproduces every Fraction-computed row.
#[test]
fn golden_rows_match_the_model() -> Result<(), String> {
    check_golden(&Side::Model)
}

/// The code (conversions, cmp_exact_num, cmp_exact_diff) reproduces every Fraction-computed row.
#[test]
fn golden_rows_match_cmp_exact_num_and_cmp_exact_diff() -> Result<(), String> {
    check_golden(&Side::Code)
}

// ---------------------------------------------------------------------------------------------------------------- edge rows

fn ord_or_none(got: Option<Ordering>, want: Option<Ordering>, what: &str) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("{}: got {:?}, want {:?}", what, got, want))
    }
}

/// One cmp_exact_num row: the code and (for a finite radius) the model must both give `want`.
fn case_n(r: &Mag, x: &Num, p: usize, want: Option<Ordering>) -> Result<(), String> {
    let what = format!(
        "cmp_exact_num({}, {}e{}:{:x?})",
        mag_str(r),
        if x.neg { "-" } else { "+" },
        x.exp,
        x.words
    );
    ord_or_none(r.cmp_exact_num(&exact_p(x, p)?), want, &what)?;
    if let Some(t) = mag_term(r) {
        ord_or_none(cmp_num(t, term(x)), want, &format!("model {}", what))?;
    }
    Ok(())
}

/// One cmp_exact_diff row, also with `(r, s)` swapped; the model is checked when both radii are finite.
fn case_d(a: &Num, b: &Num, r: &Mag, s: &Mag, want: Option<Ordering>) -> Result<(), String> {
    let (ae, be) = (exact(a)?, exact(b)?);
    let what = format!(
        "cmp_exact_diff(e{}:{:x?}, e{}:{:x?}, {}, {})",
        a.exp,
        a.words,
        b.exp,
        b.words,
        mag_str(r),
        mag_str(s)
    );
    ord_or_none(Mag::cmp_exact_diff(&ae, &be, r, s), want, &what)?;
    ord_or_none(
        Mag::cmp_exact_diff(&ae, &be, s, r),
        want,
        &format!("swapped {}", what),
    )?;
    if let (Some(tr), Some(ts)) = (mag_term(r), mag_term(s)) {
        ord_or_none(
            cmp_diff(term(a), term(b), tr, ts),
            want,
            &format!("model {}", what),
        )?;
    }
    Ok(())
}

fn representation(m: &Mag) -> bool {
    matches!(m.to_exact_num(), Err(BoundError::Representation { .. }))
}

const L: Option<Ordering> = Some(Ordering::Less);
const E: Option<Ordering> = Some(Ordering::Equal);
const G: Option<Ordering> = Some(Ordering::Greater);

#[test]
fn cmp_exact_num_edges() -> Result<(), String> {
    let one = num(false, &[TOP], 1);
    let neg_one = num(true, &[TOP], 1);
    // zero radius (2.3): ±0 Equal, nonzero x Less; a nonzero radius against ±0 is Greater
    case_n(&Mag::zero(), &zero_num(), 64, E)?;
    case_n(&Mag::zero(), &one, 64, L)?;
    case_n(&mag(TOP, 1)?, &zero_num(), 64, G)?;
    let mut neg_zero = ExactNum::new(64);
    neg_zero.inv_sign();
    ord_or_none(Mag::zero().cmp_exact_num(&neg_zero), E, "zero against -0")?;
    ord_or_none(mag(TOP, 1)?.cmp_exact_num(&neg_zero), G, "1 against -0")?;
    // ties: x = ±value(self)
    case_n(&mag(TOP, 1)?, &one, 64, E)?;
    case_n(&mag(TOP, 1)?, &neg_one, 64, E)?;
    case_n(&mag(ONES, 5)?, &num(true, &[ONES], 5), 64, E)?;
    // one ulp either side at 64 bits, and at 65+ bits (x wider than a Mag)
    case_n(&mag(TOP, 1)?, &num(false, &[TOP + 1], 1), 64, L)?;
    case_n(&mag(TOP + 1, 1)?, &one, 64, G)?;
    case_n(&mag(TOP, 1)?, &num(false, &[TOP, TOP], 1), 64, L)?;
    case_n(&mag(TOP + 1, 1)?, &num(true, &[TOP, TOP], 1), 64, G)?;
    case_n(&mag(ONES, 1)?, &num(false, &[ONES, 1], 1), 64, L)?;
    case_n(&mag(TOP, 2)?, &num(false, &[ONES, 1], 1), 64, G)?;
    // Infinity radius and NaN/Inf x: None (16.S2.5)
    ord_or_none(
        Mag::infinity().cmp_exact_num(&exact(&one)?),
        None,
        "Infinity radius",
    )?;
    for (name, x) in [("NAN", NAN), ("INF_POS", INF_POS), ("INF_NEG", INF_NEG)] {
        ord_or_none(mag(TOP, 1)?.cmp_exact_num(&x), None, name)?;
        ord_or_none(Mag::zero().cmp_exact_num(&x), None, name)?;
    }
    // radius exponents above zf's range, decided without building the radius
    let big = num(false, &[ONES], EXPONENT_MAX);
    let big_neg = num(true, &[ONES, ONES], EXPONENT_MAX);
    for e in [
        EXPONENT_MAX + 1,
        EXPONENT_MAX + 2,
        EXPONENT_MAX + 1000,
        i64::MAX - 1,
        i64::MAX,
    ] {
        let r = mag(TOP, e)?;
        if !representation(&r) {
            return Err(format!("radius exp {} is in range", e));
        }
        case_n(&r, &big, 64, G)?;
        case_n(&r, &big_neg, 192, G)?;
        case_n(&r, &one, 64, G)?;
    }
    // ... and below it
    let tiny = num(false, &[TOP], EXPONENT_MIN - 63);
    for e in [
        i64::MIN,
        i64::MIN + 1,
        EXPONENT_MIN - 2000,
        EXPONENT_MIN - 65,
    ] {
        let r = mag(ONES, e)?;
        if !representation(&r) {
            return Err(format!("radius exp {} is in range", e));
        }
        case_n(&r, &tiny, 64, L)?;
        case_n(&r, &neg_one, 64, L)?;
    }
    // a radius outside to_exact_num's 64-bit grid that equals a 1024-bit subnormal: Equal, and one ulp either side
    let sub = num(false, &[TOP], EXPONENT_MIN - 1023);
    let r = mag(TOP, EXPONENT_MIN - 1023)?;
    if !representation(&r) {
        return Err("2^(EXPONENT_MIN - 1024) is inside to_exact_num's grid".into());
    }
    case_n(&r, &sub, 1024, E)?;
    case_n(&mag(TOP + 1, EXPONENT_MIN - 1023)?, &sub, 1024, G)?;
    case_n(&mag(ONES, EXPONENT_MIN - 1024)?, &sub, 1024, L)?;
    // single-word subnormals: zf 829f112 add_full_prec flushes a subnormal sum to zero, so exact_p cannot build two words here
    let sub900 = num(false, &[TOP | 1], EXPONENT_MIN - 900);
    case_n(&r, &sub900, 1024, L)?;
    case_n(&mag(TOP + 2, EXPONENT_MIN - 900)?, &sub900, 1024, G)?;
    case_n(&mag(TOP, EXPONENT_MIN - 900)?, &sub900, 1024, L)?;
    Ok(())
}

#[test]
fn cmp_exact_diff_edges() -> Result<(), String> {
    let z = Mag::zero();
    let one = num(false, &[TOP], 1);
    let half = num(false, &[TOP], 0);
    let r1 = mag(TOP, 1)?;
    // a = b
    case_d(&one, &one, &z, &z, E)?;
    case_d(&one, &one, &r1, &z, L)?;
    case_d(&zero_num(), &zero_num(), &z, &z, E)?;
    // touching edges a − b = r + s
    let three = num(false, &[3 << 62], 2);
    case_d(&three, &one, &r1, &r1, E)?;
    case_d(&three, &one, &r1, &z, G)?;
    case_d(&three, &one, &mag(TOP + 1, 1)?, &r1, L)?;
    case_d(&three, &one, &mag(ONES, 0)?, &r1, G)?;
    // cancellation with 1000-bit operands: a − b = 2^(e − 1024) exactly
    let mut wa = Vec::new();
    let mut rng = SplitMix64::new(0x5332_0001);
    for _ in 0..15 {
        wa.push(rng.next_u64() | 1);
    }
    let mut wb = Vec::with_capacity(16);
    for w in &wa {
        wb.push(*w);
    }
    wa.push(5);
    wb.push(4);
    let (ca, cb) = (num(false, &wa, 40), num(false, &wb, 40));
    let unit = mag(TOP, 40 - 1023)?;
    case_d(&ca, &cb, &unit, &z, E)?;
    case_d(&cb, &ca, &unit, &z, L)?;
    case_d(&ca, &cb, &mag(TOP, 40 - 1024)?, &mag(TOP, 40 - 1024)?, E)?;
    case_d(&ca, &cb, &mag(ONES, 40 - 1024)?, &z, G)?;
    // mixed signs
    let neg_one = num(true, &[TOP], 1);
    case_d(&one, &neg_one, &mag(TOP, 2)?, &z, E)?;
    case_d(&neg_one, &one, &z, &z, L)?;
    case_d(&neg_one, &neg_one, &z, &z, E)?;
    // 10.2c: |m| + r = 1 as cmp_exact_diff(1, |m|, r, 0) with m = −0.75 read as 0.75; m − r = 1 with m = 1.5, r = 0.5
    let three_q = num(false, &[3 << 62], 0);
    case_d(&one, &three_q, &mag(TOP, -1)?, &z, E)?;
    case_d(&one, &three_q, &mag(TOP + 1, -1)?, &z, L)?;
    case_d(&num(false, &[3 << 62], 1), &one, &mag(TOP, 0)?, &z, E)?;
    case_d(&num(false, &[3 << 62], 1), &one, &mag(ONES, -1)?, &z, G)?;
    // near the top of zf's range: a − b = 2·max overflows zf, but the exact order is still decided
    let max = num(false, &[ONES], EXPONENT_MAX);
    let neg_max = num(true, &[ONES], EXPONENT_MAX);
    let rmax = mag(ONES, EXPONENT_MAX)?;
    case_d(&max, &neg_max, &rmax, &rmax, E)?;
    case_d(&max, &neg_max, &rmax, &z, G)?;
    case_d(&neg_max, &max, &z, &z, L)?;
    // far-apart operands: decided by bounds, no 2^62-bit exact sum
    let deep = num(false, &[TOP], EXPONENT_MIN + 200);
    case_d(&max, &deep, &rmax, &z, L)?;
    case_d(&max, &deep, &z, &z, G)?;
    case_d(&deep, &max, &z, &z, L)?;
    case_d(&max, &deep, &rmax, &mag(TOP, EXPONENT_MIN + 199)?, L)?;
    // one out-of-range radius against an in-range one
    let r_hi = mag(TOP, EXPONENT_MAX + 1)?;
    let r_lo = mag(TOP, i64::MIN)?;
    case_d(&one, &zero_num(), &r_hi, &r1, L)?;
    case_d(&max, &neg_max, &r_lo, &z, G)?;
    case_d(&max, &neg_max, &r_lo, &r1, G)?;
    case_d(&one, &one, &r_lo, &z, L)?;
    case_d(&neg_one, &one, &r_hi, &z, L)?;
    case_d(&half, &neg_max, &r_hi, &mag(TOP, i64::MAX)?, L)?;
    // Infinity radius, NaN/Inf operands: None
    case_d(&one, &zero_num(), &Mag::infinity(), &z, None)?;
    case_d(&one, &zero_num(), &r1, &Mag::infinity(), None)?;
    let x = exact(&one)?;
    for (name, y) in [("NAN", NAN), ("INF_POS", INF_POS), ("INF_NEG", INF_NEG)] {
        ord_or_none(Mag::cmp_exact_diff(&y, &x, &z, &z), None, name)?;
        ord_or_none(Mag::cmp_exact_diff(&x, &y, &r1, &z), None, name)?;
    }
    Ok(())
}

#[test]
fn conversion_edges_against_the_model() -> Result<(), String> {
    // 63, 64, 65 and 128 significant bits, all ones with and without sticky, and the carry
    let rows = [
        num(false, &[0x7FFF_FFFF_FFFF_FFFF], 3),
        num(true, &[ONES], 3),
        num(false, &[TOP, TOP], 3),
        num(false, &[ONES, ONES], -3),
        num(false, &[TOP + 1], 0),
        num(false, &[ONES, 1], 7),
        num(true, &[ONES], EXPONENT_MAX),
        num(false, &[TOP, 1], EXPONENT_MAX),
        num(false, &[TOP], EXPONENT_MIN + 1),
    ];
    for n in &rows {
        let t = term(n);
        let x = exact(n)?;
        want_parts("up", &Mag::from_exact_num(&x), round_up(&t))?;
        want_parts("down", &Mag::from_exact_num_lower(&x), round_down(&t))?;
    }
    // subnormals down to 2^(EXPONENT_MIN − p): exact both ways
    for p in [64usize, 128, 1024] {
        let pi = i64::try_from(p).map_err(|e| e.to_string())?;
        let n = num(false, &[TOP], EXPONENT_MIN - pi + 1);
        let x = exact_p(&n, p)?;
        let want = Some((TOP, i128::from(EXPONENT_MIN - pi + 1)));
        want_parts("subnormal up", &Mag::from_exact_num(&x), want)?;
        want_parts("subnormal down", &Mag::from_exact_num_lower(&x), want)?;
        if exact_p(&num(false, &[TOP], EXPONENT_MIN - pi), p).is_ok() {
            return Err(format!("zf accepted a value below the p = {} grid", p));
        }
    }
    // to_exact_num at EXPONENT_MAX (Ok, exact) and EXPONENT_MAX + 1, at the grid edge and one below, i64 extremes, Infinity
    for (m, ok) in [
        (mag(ONES, EXPONENT_MAX)?, true),
        (mag(TOP, EXPONENT_MAX + 1)?, false),
        (mag(TOP, EXPONENT_MIN - 63)?, true),
        (mag(TOP, EXPONENT_MIN - 64)?, false),
        (mag(TOP, i64::MIN)?, false),
        (mag(ONES, i64::MAX)?, false),
        (Mag::infinity(), false),
    ] {
        match (m.to_exact_num(), ok) {
            (Ok(x), true) => {
                if Mag::from_exact_num(&x) != m || Mag::from_exact_num_lower(&x) != m || x.inexact()
                {
                    return Err(format!("{}: round trip not exact", mag_str(&m)));
                }
            }
            (Err(BoundError::Representation { .. }), false) => {}
            (got, _) => {
                return Err(format!(
                    "{}: to_exact_num gave {:?}, want ok = {}",
                    mag_str(&m),
                    got.is_ok(),
                    ok
                ))
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------- properties

fn below(rng: &mut SplitMix64, n: u64) -> i64 {
    i64::try_from(rng.below(n)).unwrap_or(0)
}

/// 1 to 4 words (first nonzero), random sign; 1 in 16 is zero. Exponents: small, near EXPONENT_MAX, near EXPONENT_MIN, wide.
fn draw_num(rng: &mut SplitMix64) -> Num {
    if rng.below(16) == 0 {
        return zero_num();
    }
    let n = 1 + rng.below(4);
    let mut words = Vec::new();
    for _ in 0..n {
        words.push(rng.next_u64());
    }
    if let Some(w) = words.first_mut() {
        *w |= 1u64 << rng.below(64);
    }
    let exp = match rng.below(4) {
        0 => below(rng, 601) - 300,
        1 => EXPONENT_MAX - below(rng, 100),
        2 => EXPONENT_MIN + 400 + below(rng, 100),
        _ => below(rng, 1 << 41) - (1 << 40),
    };
    Num {
        neg: rng.below(2) == 1,
        words,
        exp,
    }
}

/// A finite `Mag` (1 in 16 zero) with exponents small, at both zf range edges, at the i64 extremes and wide.
fn draw_mag(rng: &mut SplitMix64) -> Result<Mag, String> {
    if rng.below(16) == 0 {
        return Ok(Mag::zero());
    }
    let mant = rng.next_u64() | TOP;
    let exp = match rng.below(6) {
        0 => below(rng, 601) - 300,
        1 => EXPONENT_MAX - 70 + below(rng, 141),
        2 => EXPONENT_MIN - 70 + below(rng, 141),
        3 => i64::MAX - below(rng, 100),
        4 => i64::MIN + below(rng, 100),
        _ => below(rng, 1 << 41) - (1 << 40),
    };
    mag(mant, exp)
}

/// A `Mag` near `|x|` (its bounds, one ulp out), so the comparisons see ties and near-ties, not just far-apart values.
fn near_mag(rng: &mut SplitMix64, x: &ExactNum) -> Result<Mag, String> {
    let base = if rng.below(2) == 0 {
        Mag::from_exact_num(x)
    } else {
        Mag::from_exact_num_lower(x)
    };
    match (base.mant(), base.exp(), rng.below(3)) {
        (Some(m), Some(e), 0) if m != 0 && m < ONES => mag(m + 1, e),
        (Some(m), Some(e), 1) if m > TOP => mag(m - 1, e),
        _ => Ok(base),
    }
}

#[test]
fn p9_round_trip_monotone_and_one_ulp() -> Result<(), String> {
    let mut rng = SplitMix64::new(0x5332_0009);
    for case in 0..CASES {
        let n = draw_num(&mut rng);
        let x = exact(&n)?;
        let t = term(&n);
        let (up, lo) = (Mag::from_exact_num(&x), Mag::from_exact_num_lower(&x));
        want_parts("up", &up, round_up(&t)).map_err(|e| format!("case {}: {}", case, e))?;
        want_parts("down", &lo, round_down(&t)).map_err(|e| format!("case {}: {}", case, e))?;
        let sticky = top64(&t).is_some_and(|(_, s, _)| s);
        if lo > up || (lo == up) == sticky {
            return Err(format!(
                "case {}: lower {} up {} sticky {}",
                case,
                mag_str(&lo),
                mag_str(&up),
                sticky
            ));
        }
        // P9: the round trip is exact and on the right side
        let (u, l) = (
            up.to_exact_num().map_err(|e| format!("{:?}", e))?,
            lo.to_exact_num().map_err(|e| format!("{:?}", e))?,
        );
        if u.abs_cmp(&x).is_none_or(|c| c < 0) || l.abs_cmp(&x).is_none_or(|c| c > 0) {
            return Err(format!("case {}: P9 failed", case));
        }
        // 16.S2.4 (a): lossless for an in-range Mag
        let m = draw_mag(&mut rng)?;
        if let Ok(v) = m.to_exact_num() {
            if Mag::from_exact_num(&v) != m || Mag::from_exact_num_lower(&v) != m {
                return Err(format!(
                    "case {}: {} does not round-trip",
                    case,
                    mag_str(&m)
                ));
            }
        }
        // 16.S2.4 (c): monotone
        let y = exact(&draw_num(&mut rng))?;
        let (small, large) = match x.abs_cmp(&y) {
            Some(c) if c <= 0 => (&x, &y),
            _ => (&y, &x),
        };
        if Mag::from_exact_num(small) > Mag::from_exact_num(large)
            || Mag::from_exact_num_lower(small) > Mag::from_exact_num_lower(large)
        {
            return Err(format!("case {}: not monotone", case));
        }
    }
    Ok(())
}

#[test]
fn cmp_exact_num_matches_model() -> Result<(), String> {
    let mut rng = SplitMix64::new(0x5332_0005);
    for case in 0..CASES {
        let n = draw_num(&mut rng);
        let x = exact(&n)?;
        let r = if rng.below(2) == 0 {
            draw_mag(&mut rng)?
        } else {
            near_mag(&mut rng, &x)?
        };
        let want = cmp_num(mag_term(&r).ok_or("Infinity drawn")?, term(&n));
        let got = r.cmp_exact_num(&x);
        if got != want || got.is_none() {
            return Err(format!(
                "case {}: cmp_exact_num({}, {:?}) = {:?}, model {:?}",
                case,
                mag_str(&r),
                n.words,
                got,
                want
            ));
        }
        // consistent with Mag: Ord when x is a Mag value (antisymmetric through from_exact_num)
        let r2 = draw_mag(&mut rng)?;
        if let Ok(v) = r2.to_exact_num() {
            if r.cmp_exact_num(&v) != Some(r.cmp(&r2))
                || r2.cmp_exact_num(&x) != cmp_num(mag_term(&r2).ok_or("Infinity drawn")?, term(&n))
            {
                return Err(format!(
                    "case {}: disagrees with Mag: Ord ({} vs {})",
                    case,
                    mag_str(&r),
                    mag_str(&r2)
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn cmp_exact_diff_matches_model() -> Result<(), String> {
    let mut rng = SplitMix64::new(0x5332_0006);
    for case in 0..CASES {
        let (na, nb) = (draw_num(&mut rng), draw_num(&mut rng));
        let (a, b) = (exact(&na)?, exact(&nb)?);
        // radii: random, or near |a − b| when that difference is cheap to form exactly
        let close = match (a.exponent(), b.exponent()) {
            (Some(ea), Some(eb)) => ea.checked_sub(eb).is_some_and(|g| g.abs() < 4096),
            _ => true,
        };
        let d = if close {
            Some(a.sub_full_prec(&b))
        } else {
            None
        };
        let r = match (&d, rng.below(2)) {
            (Some(dv), 0) if !dv.is_inf() && !dv.is_nan() => near_mag(&mut rng, dv)?,
            _ => draw_mag(&mut rng)?,
        };
        let s = if rng.below(3) == 0 {
            Mag::zero()
        } else {
            draw_mag(&mut rng)?
        };
        let want = cmp_diff(
            term(&na),
            term(&nb),
            mag_term(&r).ok_or("Infinity drawn")?,
            mag_term(&s).ok_or("Infinity drawn")?,
        );
        let got = Mag::cmp_exact_diff(&a, &b, &r, &s);
        let in_range = r.to_exact_num().is_ok() && s.to_exact_num().is_ok();
        if want.is_none() || (got.is_none() && in_range) || (got.is_some() && got != want) {
            return Err(format!(
                "case {}: cmp_exact_diff = {:?}, model {:?} (r {}, s {})",
                case,
                got,
                want,
                mag_str(&r),
                mag_str(&s)
            ));
        }
        // swapping (r, s) changes nothing
        if Mag::cmp_exact_diff(&a, &b, &s, &r) != got {
            return Err(format!("case {}: (r, s) swap changed the answer", case));
        }
        // with s = 0 and a − b ≥ 0, cmp_exact_diff agrees with cmp_exact_num
        if let Some(dv) = &d {
            let nonneg = !dv.is_nan() && !dv.is_inf() && (dv.is_zero() || !dv.is_negative());
            let g0 = Mag::cmp_exact_diff(&a, &b, &r, &Mag::zero());
            if nonneg && g0.is_some() && g0 != r.cmp_exact_num(dv).map(Ordering::reverse) {
                return Err(format!(
                    "case {}: cmp_exact_diff(a, b, r, 0) disagrees with cmp_exact_num",
                    case
                ));
            }
        }
    }
    Ok(())
}
