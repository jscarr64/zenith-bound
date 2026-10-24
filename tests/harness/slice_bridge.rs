//! Slice S2 harness rows (`src/bridge.rs`). Owned by slice S2 alone.
//! Goldens: tests/goldens/slices/bridge/ (its own SHA256SUMS and GENERATORS). Mutants: tools/mutants/manifest.d/bridge.txt, ids 300..=399.
//! Golden file headers for this slice are declared here, not in tests/common/goldens.rs.
//! CONTRACT 4.2, 10.2c, 15.2 and 16.S2.1 to 16.S2.6 (TEST_PLAN L0a-4, moved here 2026-10-24). Oracles: hand-worked rows
//! (value in the comment beside each), and for random in-range cases zenith-float's exact `sub_full_prec` plus `abs_cmp`
//! against the radius converted by `to_exact_num` (in range only; the out-of-range rows are worked by hand).
//! COVERS: Mag::cmp_exact_num, Mag::cmp_exact_diff, Mag::from_exact_num, Mag::from_exact_num_lower, Mag::to_exact_num

use core::cmp::Ordering;

use zenith_bound::mag::Mag;
use zenith_float_num::{ExactNum, RawKind, RawParts, EXPONENT_MAX, EXPONENT_MIN};

use crate::common::rng::SplitMix64;

const TOP: u64 = 0x8000_0000_0000_0000;

fn mag(mant: u64, exp: i64) -> Result<Mag, String> {
    Mag::finite(mant, exp).map_err(|e| format!("Mag::finite({:#x}, {}): {:?}", mant, exp, e))
}

fn num(v: i64) -> ExactNum {
    ExactNum::from(v)
}

/// `0.mant * 2^exp` exactly, at `prec` bits (at least 64).
fn parts(mant: u64, exp: i64, prec: usize) -> Result<ExactNum, String> {
    ExactNum::from_mag_parts(mant, exp, prec)
        .map_err(|e| format!("from_mag_parts({:#x}, {}): {:?}", mant, exp, e))
}

/// `2^k` exactly.
fn pow2(k: i64) -> Result<ExactNum, String> {
    parts(TOP, k.checked_add(1).ok_or("k")?, 64)
}

fn neg(x: &ExactNum) -> ExactNum {
    num(0).sub_full_prec(x)
}

fn want(what: &str, got: Option<Ordering>, w: Ordering) -> Result<(), String> {
    if got == Some(w) {
        Ok(())
    } else {
        Err(format!("{}: got {:?}, want {:?}", what, got, w))
    }
}

fn nan() -> ExactNum {
    num(0).div(&num(0), 64, zenith_float_num::RoundingMode::ToEven)
}

fn inf() -> ExactNum {
    num(1).div(&num(0), 64, zenith_float_num::RoundingMode::ToEven)
}

// ---- cmp_exact_num ------------------------------------------------------------------------------------------------

#[test]
fn cmp_exact_num_zero_and_infinity() -> Result<(), String> {
    want(
        "0 vs 0",
        Mag::zero().cmp_exact_num(&num(0)),
        Ordering::Equal,
    )?;
    want("0 vs 1", Mag::zero().cmp_exact_num(&num(1)), Ordering::Less)?;
    want(
        "0 vs -1",
        Mag::zero().cmp_exact_num(&num(-1)),
        Ordering::Less,
    )?;
    // Zero radius against values below 1 (Mag zero stores exponent 0; it must not compare by exponent).
    want(
        "0 vs 2^-10",
        Mag::zero().cmp_exact_num(&pow2(-10)?),
        Ordering::Less,
    )?;
    want(
        "0 vs -2^-5000",
        Mag::zero().cmp_exact_num(&neg(&pow2(-5000)?)),
        Ordering::Less,
    )?;
    want(
        "1 vs 0",
        mag(TOP, 1)?.cmp_exact_num(&num(0)),
        Ordering::Greater,
    )?;
    // Infinity is above every finite |x|.
    want(
        "inf vs 0",
        Mag::infinity().cmp_exact_num(&num(0)),
        Ordering::Greater,
    )?;
    want(
        "inf vs 2^1000",
        Mag::infinity().cmp_exact_num(&pow2(1000)?),
        Ordering::Greater,
    )?;
    // NaN and ±Inf have no finite magnitude: undecided.
    for (w, x) in [("nan", nan()), ("inf", inf())] {
        if Mag::zero().cmp_exact_num(&x).is_some() || mag(TOP, 1)?.cmp_exact_num(&x).is_some() {
            return Err(format!("{}: want None", w));
        }
    }
    Ok(())
}

#[test]
fn cmp_exact_num_ties_and_sticky() -> Result<(), String> {
    want(
        "1 vs 1",
        mag(TOP, 1)?.cmp_exact_num(&num(1)),
        Ordering::Equal,
    )?;
    want(
        "1 vs -1",
        mag(TOP, 1)?.cmp_exact_num(&num(-1)),
        Ordering::Equal,
    )?;
    want(
        "3 vs 3",
        mag(0xC000_0000_0000_0000, 2)?.cmp_exact_num(&num(-3)),
        Ordering::Equal,
    )?;
    // x = 1 + 2^-100: same top 64 bits as 1, then a sticky bit; just above Mag 1, below Mag 1 + 2^-63.
    let x = num(1).add_full_prec(&pow2(-100)?);
    want(
        "1 vs 1+2^-100",
        mag(TOP, 1)?.cmp_exact_num(&x),
        Ordering::Less,
    )?;
    want(
        "1+2^-63 vs 1+2^-100",
        mag(TOP | 1, 1)?.cmp_exact_num(&x),
        Ordering::Greater,
    )?;
    // x = 1 - 2^-100 has top bits all ones below exponent 0: Mag(u64::MAX, 0) = 1 - 2^-64 is below it.
    let y = num(1).sub_full_prec(&pow2(-100)?);
    want(
        "1-2^-64 vs 1-2^-100",
        mag(u64::MAX, 0)?.cmp_exact_num(&y),
        Ordering::Less,
    )?;
    want(
        "1 vs 1-2^-100",
        mag(TOP, 1)?.cmp_exact_num(&y),
        Ordering::Greater,
    )?;
    // Exponent decides before the mantissa.
    want(
        "2 vs 3",
        mag(TOP, 2)?.cmp_exact_num(&num(3)),
        Ordering::Less,
    )?;
    want(
        "4 vs 3",
        mag(TOP, 3)?.cmp_exact_num(&num(3)),
        Ordering::Greater,
    )?;
    Ok(())
}

#[test]
fn cmp_exact_num_radius_outside_the_zf_range() -> Result<(), String> {
    // Radii whose exponent lies far outside zenith-float's range compare by exponent, never via to_exact_num.
    let hi = EXPONENT_MAX;
    let lo = EXPONENT_MIN;
    let big_x = parts(u64::MAX, hi, 64)?; // just below 2^EXPONENT_MAX
    let tiny_x = parts(TOP, lo, 64)?; // 2^(EXPONENT_MIN - 1)
    for e in [
        hi.checked_add(1).ok_or("e")?,
        hi.checked_add(1000).ok_or("e")?,
        (1i64 << 62) + (1i64 << 61),
        i64::MAX,
    ] {
        let m = mag(TOP, e)?;
        if m.to_exact_num().is_ok() && e > hi {
            return Err(format!("exp {}: expected outside the zf range", e));
        }
        want(
            &format!("2^{} vs big", e),
            m.cmp_exact_num(&big_x),
            Ordering::Greater,
        )?;
        want(
            &format!("2^{} vs -big", e),
            m.cmp_exact_num(&neg(&big_x)),
            Ordering::Greater,
        )?;
    }
    for e in [
        lo.checked_sub(1).ok_or("e")?,
        lo.checked_sub(1000).ok_or("e")?,
        -((1i64 << 62) + (1i64 << 61)),
        i64::MIN,
    ] {
        let m = mag(u64::MAX, e)?;
        want(
            &format!("~2^{} vs tiny", e),
            m.cmp_exact_num(&tiny_x),
            Ordering::Less,
        )?;
        want(
            &format!("~2^{} vs 0", e),
            m.cmp_exact_num(&num(0)),
            Ordering::Greater,
        )?;
    }
    Ok(())
}

/// A random nonzero `ExactNum` near `2^e` with `extra` bits below the top 64, random sign.
fn rand_num(rng: &mut SplitMix64, e: i64, extra: i64) -> Result<ExactNum, String> {
    let top = parts(rng.next_u64() | TOP, e, 64)?;
    let x = if extra > 0 && rng.below(2) == 0 {
        let low = parts(rng.next_u64() | 1, e.checked_sub(extra).ok_or("e")?, 64)?;
        top.add_full_prec(&low)
    } else {
        top
    };
    Ok(if rng.below(2) == 0 { neg(&x) } else { x })
}

/// Oracle: `m` (in the zf range) against `|x|` by zenith-float's exact `abs_cmp`.
fn oracle_num(m: &Mag, x: &ExactNum) -> Result<Ordering, String> {
    let r = m
        .to_exact_num()
        .map_err(|e| format!("oracle to_exact_num: {:?}", e))?;
    let c = r.abs_cmp(x).ok_or("abs_cmp None")?;
    Ok(c.cmp(&0))
}

#[test]
fn cmp_exact_num_matches_the_exact_oracle() -> Result<(), String> {
    let mut rng = SplitMix64::new(0x5200_0001);
    for case in 0..4000 {
        let e = i64::try_from(rng.below(4001))
            .map_err(|_| "e")?
            .checked_sub(2000)
            .ok_or("e")?;
        let x = rand_num(&mut rng, e, 70)?;
        // The radius: near |x| (same exponent, mantissa equal / ±1 / random), or one exponent away.
        let (xm, _, xe) = x.top_u64_mantissa_and_sticky().ok_or("x parts")?;
        let pick = rng.below(6);
        let (mm, me) = match pick {
            0 => (xm, xe),
            1 => (xm.checked_add(1).unwrap_or(xm), xe),
            2 => (xm.checked_sub(1).filter(|v| *v >= TOP).unwrap_or(xm), xe),
            3 => (rng.next_u64() | TOP, xe),
            4 => (rng.next_u64() | TOP, xe.checked_add(1).ok_or("e")?),
            _ => (rng.next_u64() | TOP, xe.checked_sub(1).ok_or("e")?),
        };
        let m = mag(mm, me)?;
        let w = oracle_num(&m, &x)?;
        want(
            &format!("case {} pick {} m {:?} x {:?}", case, pick, m, x),
            m.cmp_exact_num(&x),
            w,
        )?;
    }
    Ok(())
}

// ---- cmp_exact_diff -----------------------------------------------------------------------------------------------

#[test]
fn cmp_exact_diff_rows() -> Result<(), String> {
    let two = mag(TOP, 2)?;
    // |3 - 1| = 2.
    want(
        "2 vs |3-1|",
        two.cmp_exact_diff(&num(3), &num(1)),
        Ordering::Equal,
    )?;
    want(
        "2 vs |1-3|",
        two.cmp_exact_diff(&num(1), &num(3)),
        Ordering::Equal,
    )?;
    want(
        "2+ vs |3-1|",
        mag(TOP | 1, 2)?.cmp_exact_diff(&num(3), &num(1)),
        Ordering::Greater,
    )?;
    want(
        "2- vs |3-1|",
        mag(u64::MAX, 1)?.cmp_exact_diff(&num(3), &num(1)),
        Ordering::Less,
    )?;
    // |1 - (-1)| = 2, |-1 - 1| = 2.
    want(
        "2 vs |1+1|",
        two.cmp_exact_diff(&num(1), &num(-1)),
        Ordering::Equal,
    )?;
    want(
        "2 vs |-1-1|",
        two.cmp_exact_diff(&num(-1), &num(1)),
        Ordering::Equal,
    )?;
    // Gap of one binade with cancellation: |2 - (2 - 2^-50)| = 2^-50, far below 1/4.
    let near2 = num(2).sub_full_prec(&pow2(-50)?);
    want(
        "1/4 vs |2-(2-2^-50)|",
        mag(TOP, -1)?.cmp_exact_diff(&num(2), &near2),
        Ordering::Greater,
    )?;
    want(
        "2^-50 vs |2-(2-2^-50)|",
        mag(TOP, -49)?.cmp_exact_diff(&near2, &num(2)),
        Ordering::Equal,
    )?;
    // a == b.
    want(
        "0 vs |5-5|",
        Mag::zero().cmp_exact_diff(&num(5), &num(5)),
        Ordering::Equal,
    )?;
    want(
        "2 vs |5-5|",
        two.cmp_exact_diff(&num(5), &num(5)),
        Ordering::Greater,
    )?;
    // A zero operand.
    want(
        "2 vs |0-2|",
        two.cmp_exact_diff(&num(0), &num(-2)),
        Ordering::Equal,
    )?;
    want(
        "2 vs |3-0|",
        two.cmp_exact_diff(&num(3), &num(0)),
        Ordering::Less,
    )?;
    want(
        "0 vs |0-0|",
        Mag::zero().cmp_exact_diff(&num(0), &num(0)),
        Ordering::Equal,
    )?;
    // Infinity radius; NaN / Inf operands.
    want(
        "inf vs |3-1|",
        Mag::infinity().cmp_exact_diff(&num(3), &num(1)),
        Ordering::Greater,
    )?;
    for (w, a, b) in [("nan a", nan(), num(1)), ("inf b", num(1), inf())] {
        if two.cmp_exact_diff(&a, &b).is_some() {
            return Err(format!("{}: want None", w));
        }
    }
    Ok(())
}

#[test]
fn cmp_exact_diff_huge_exponent_gap() -> Result<(), String> {
    // |2^1000 - 2^-1000| is just below 2^1000; |2^1000 + 2^-1000| just above.
    let a = pow2(1000)?;
    let b = pow2(-1000)?;
    let m = mag(TOP, 1001)?; // 2^1000
    want(
        "2^1000 vs |a-b|",
        m.cmp_exact_diff(&a, &b),
        Ordering::Greater,
    )?;
    want(
        "2^1000 vs |b-a|",
        m.cmp_exact_diff(&b, &a),
        Ordering::Greater,
    )?;
    want(
        "2^1000 vs |a+b|",
        m.cmp_exact_diff(&a, &neg(&b)),
        Ordering::Less,
    )?;
    want(
        "2^1000 vs |-a-b|",
        m.cmp_exact_diff(&neg(&a), &b),
        Ordering::Less,
    )?;
    let under = mag(u64::MAX, 1000)?; // 2^1000 (1 - 2^-64)
    want(
        "2^1000- vs |a-b|",
        under.cmp_exact_diff(&a, &b),
        Ordering::Less,
    )?;
    // Deep bits decide: a = 2^1000 + 2^-500 (1501 bits), b = 2^-500, so |a - b| = 2^1000 exactly.
    let deep = a.add_full_prec(&pow2(-500)?);
    let b2 = pow2(-500)?;
    want(
        "2^1000 vs |deep-b2|",
        m.cmp_exact_diff(&deep, &b2),
        Ordering::Equal,
    )?;
    want(
        "2^1000 vs |deep+b2|",
        m.cmp_exact_diff(&deep, &neg(&b2)),
        Ordering::Less,
    )?;
    // a = 2^1000 + 2^-500, b = 2^-400: |a - b| = 2^1000 - 2^-400 + 2^-500 < 2^1000.
    want(
        "2^1000 vs |deep-2^-400|",
        m.cmp_exact_diff(&deep, &pow2(-400)?),
        Ordering::Greater,
    )?;
    // a = 2^1000 + 2^-500, b = 2^-600: |a - b| > 2^1000.
    want(
        "2^1000 vs |deep-2^-600|",
        m.cmp_exact_diff(&deep, &pow2(-600)?),
        Ordering::Less,
    )?;
    // Zero radius against a wide-gap difference (never zero).
    want(
        "0 vs |a-b|",
        Mag::zero().cmp_exact_diff(&a, &b),
        Ordering::Less,
    )?;
    want(
        "0 vs |2^-100-2^-1000|",
        Mag::zero().cmp_exact_diff(&pow2(-100)?, &b),
        Ordering::Less,
    )?;
    want(
        "0 vs |2^-1000-2^-100|",
        Mag::zero().cmp_exact_diff(&b, &pow2(-100)?),
        Ordering::Less,
    )?;
    // |big| just under 2^1000 plus a same-sign small term crosses 2^1000: a = 2^1000 - 2^700, b = -2^800.
    let under_big = a.sub_full_prec(&pow2(700)?);
    want(
        "2^1000 vs |(2^1000-2^700)+2^800|",
        m.cmp_exact_diff(&under_big, &neg(&pow2(800)?)),
        Ordering::Less,
    )?;
    want(
        "2^1000 vs |(2^1000-2^700)-2^800|",
        m.cmp_exact_diff(&under_big, &pow2(800)?),
        Ordering::Greater,
    )?;
    // The whole zf range apart: decided without an exact subtraction across ~2^32 bits.
    let hi = EXPONENT_MAX;
    let lo = EXPONENT_MIN;
    let big = parts(TOP, hi, 64)?; // 2^(EXPONENT_MAX - 1)
    let tiny = parts(TOP, lo, 64)?;
    want(
        "2^(max-1) vs |big-tiny|",
        mag(TOP, hi)?.cmp_exact_diff(&big, &tiny),
        Ordering::Greater,
    )?;
    want(
        "2^(max-1) vs |big+tiny|",
        mag(TOP, hi)?.cmp_exact_diff(&big, &neg(&tiny)),
        Ordering::Less,
    )?;
    // Radius exponents outside the zf range.
    want(
        "2^(max+5) vs |big-tiny|",
        mag(TOP, hi.checked_add(5).ok_or("e")?)?.cmp_exact_diff(&big, &tiny),
        Ordering::Greater,
    )?;
    want(
        "2^(min-5) vs |tiny-0|",
        mag(TOP, lo.checked_sub(5).ok_or("e")?)?.cmp_exact_diff(&tiny, &num(0)),
        Ordering::Less,
    )?;
    want(
        "2^(min-5) vs |tiny-2tiny|",
        mag(TOP, lo.checked_sub(5).ok_or("e")?)?.cmp_exact_diff(&tiny, &tiny.add_full_prec(&tiny)),
        Ordering::Less,
    )?;
    Ok(())
}

#[test]
fn cmp_exact_diff_matches_the_exact_oracle() -> Result<(), String> {
    let mut rng = SplitMix64::new(0x5200_0002);
    for case in 0..4000 {
        let ea = i64::try_from(rng.below(2001))
            .map_err(|_| "e")?
            .checked_sub(1000)
            .ok_or("e")?;
        // Gaps from 0 to 600: both the exact path and the large-gap path.
        let gap = i64::try_from(rng.below(601)).map_err(|_| "g")?;
        let eb = if rng.below(2) == 0 {
            ea.checked_sub(gap)
        } else {
            ea.checked_add(gap)
        }
        .ok_or("eb")?;
        let a = rand_num(&mut rng, ea, 90)?;
        let b = if rng.below(8) == 0 {
            a.add_full_prec(&num(0))
        } else {
            rand_num(&mut rng, eb, 90)?
        };
        let d = a.sub_full_prec(&b);
        let m = match d.top_u64_mantissa_and_sticky() {
            None => mag(rng.next_u64() | TOP, ea)?,
            Some((dm, _, de)) => match rng.below(5) {
                0 => mag(dm, de)?,
                1 => mag(dm.checked_add(1).unwrap_or(dm), de)?,
                2 => mag(dm.checked_sub(1).filter(|v| *v >= TOP).unwrap_or(dm), de)?,
                3 => mag(rng.next_u64() | TOP, de)?,
                _ => mag(rng.next_u64() | TOP, de.checked_add(1).ok_or("e")?)?,
            },
        };
        let w = oracle_num(&m, &d)?;
        want(
            &format!("case {} gap {} m {:?}", case, gap, m),
            m.cmp_exact_diff(&a, &b),
            w,
        )?;
    }
    Ok(())
}

// ---- pending zenith-float R14 -------------------------------------------------------------------------------------

#[test]
#[ignore = "pending zenith-float R14 Consts::get_exact_prec: 16.S2.6"]
fn bridge_constant_bound_pending_r14() -> Result<(), String> {
    Err("pending zenith-float R14: the constant bridge is stubbed (16.S2.6)".into())
}

/// An `ExactNum` from raw parts: `(-1)^negative * M * 2^(exponent - 64 * words.len())`, `M` little-endian words.
fn raw(words: &[u64], exponent: i64, negative: bool) -> Result<ExactNum, String> {
    let all_zero = words.iter().all(|w| *w == 0);
    let parts = RawParts {
        kind: if all_zero {
            RawKind::Zero
        } else {
            RawKind::Finite
        },
        words,
        precision: words.len().checked_mul(64).ok_or("prec")?,
        negative,
        exponent,
        inexact: false,
    };
    ExactNum::from_raw_parts(&parts)
        .map_err(|e| format!("from_raw_parts({:x?}, {}): {:?}", words, exponent, e))
}

/// `Mag` from_exact_num / from_exact_num_lower of `x` are exactly `up` / `lo`, for `x` and `-x`.
fn want_up_lo(what: &str, x: &ExactNum, up: &Mag, lo: &Mag) -> Result<(), String> {
    for (sign, v) in [("+", num(0).add_full_prec(x)), ("-", neg(x))] {
        let (gu, gl) = (Mag::from_exact_num(&v), Mag::from_exact_num_lower(&v));
        if gu != *up || gl != *lo {
            return Err(format!(
                "{}{}: got up {:?} lo {:?}, want {:?} {:?}",
                sign, what, gu, gl, up, lo
            ));
        }
    }
    Ok(())
}

/// 16.S2.6 (R14 RawParts half): `from_exact_num(_lower)` read `as_raw_parts`, `to_exact_num` builds through
/// `from_raw_parts`. Exponents across ±(2^62−1), multiword mantissas, sticky words, carries and subnormals. The stored
/// precision is not assumed to round-trip (only `precision == 64 * words.len()`, zf's contract).
#[test]
fn bridge_export_round_trip() -> Result<(), String> {
    let (hi, lo) = (EXPONENT_MAX, EXPONENT_MIN);
    for e in [hi, hi - 1, 1000, 0, -1000, lo + 1, lo] {
        let w = TOP | 5;
        // Normal, sticky only in the lowest word: up is one ulp above.
        want_up_lo(
            "sticky",
            &raw(&[1, 0, w], e, false)?,
            &mag(TOP | 6, e)?,
            &mag(w, e)?,
        )?;
        // Normal, exact in the top word.
        want_up_lo(
            "exact",
            &raw(&[0, 0, w], e, false)?,
            &mag(w, e)?,
            &mag(w, e)?,
        )?;
        // All ones with a tail: up carries into the next binade (Mag is wider than zf, so even at EXPONENT_MAX).
        want_up_lo(
            "carry",
            &raw(&[1, u64::MAX], e, false)?,
            &mag(TOP, e + 1)?,
            &mag(u64::MAX, e)?,
        )?;
    }
    // Subnormals (exponent == EXPONENT_MIN, top bit clear). M = 2^74 in three words: 2^(EMIN - 118) = 0.1b * 2^(EMIN - 117).
    want_up_lo(
        "sub 2^74",
        &raw(&[0, 1 << 10, 0], lo, false)?,
        &mag(TOP, lo - 117)?,
        &mag(TOP, lo - 117)?,
    )?;
    // M = 2^64 + 3: the window spans two words; the last bit of 3 is sticky. Top bit at 2^(EMIN - 128).
    want_up_lo(
        "sub span",
        &raw(&[3, 1, 0], lo, false)?,
        &mag(TOP | 2, lo - 127)?,
        &mag(TOP | 1, lo - 127)?,
    )?;
    // M = 1 in one word: 2^(EMIN - 64) = 0.1b * 2^(EMIN - 63).
    want_up_lo(
        "sub one",
        &raw(&[1], lo, false)?,
        &mag(TOP, lo - 63)?,
        &mag(TOP, lo - 63)?,
    )?;
    // Zero, both signs.
    want_up_lo("zero", &raw(&[0], 0, true)?, &Mag::zero(), &Mag::zero())?;

    // to_exact_num through from_raw_parts: exact, Finite, positive, exponent kept; normal across the range.
    for e in [hi, hi - 1, 0, lo + 1, lo] {
        let m = mag(TOP | 0x1234_5678_9abc_def1, e)?;
        let x = m
            .to_exact_num()
            .map_err(|err| format!("to_exact_num exp {}: {:?}", e, err))?;
        let p = x.as_raw_parts();
        if p.kind != RawKind::Finite
            || p.negative
            || p.exponent != e
            || p.precision != p.words.len() * 64
        {
            return Err(format!("exp {}: raw parts {:?}", e, p));
        }
        want_up_lo("to/from", &x, &m, &m)?;
    }
    // Subnormal images: below EXPONENT_MIN the value is exact only while no mantissa bit falls off the 64-bit word.
    for (mant, e, ok) in [
        (TOP, lo - 63, true),
        (0xC000_0000_0000_0000, lo - 62, true),
        (0xC000_0000_0000_0000, lo - 63, false),
        (TOP | 1, lo - 1, false),
        (TOP, lo - 64, false),
        (TOP, hi + 1, false),
    ] {
        let m = mag(mant, e)?;
        match (m.to_exact_num(), ok) {
            (Ok(x), true) => {
                if x.as_raw_parts().exponent != lo {
                    return Err(format!(
                        "{:#x}@{}: subnormal exponent {:?}",
                        mant,
                        e,
                        x.as_raw_parts()
                    ));
                }
                want_up_lo("subnormal to/from", &x, &m, &m)?;
            }
            (Err(zenith_bound::error::BoundError::Representation { .. }), false) => {}
            (r, _) => {
                return Err(format!(
                    "{:#x}@{}: want ok={}, got {:?}",
                    mant,
                    e,
                    ok,
                    r.map(|_| ())
                ))
            }
        }
    }
    // Random three-word mantissas across the full range (and the subnormal floor): lo <= |x| <= up, up - lo <= 1 ulp.
    let mut rng = SplitMix64::new(0x5200_0016);
    for case in 0..3000 {
        let sub = case % 5 == 0;
        let mut w = [rng.next_u64(), rng.next_u64(), rng.next_u64()];
        let e = if sub {
            if let Some(top) = w.last_mut() {
                *top >>= 1 + rng.below(63);
            }
            lo
        } else {
            if let Some(top) = w.last_mut() {
                *top |= TOP;
            }
            let span = u64::try_from(hi).map_err(|_| "hi")?;
            i64::try_from(rng.below(span)).map_err(|_| "e")? * if case % 2 == 0 { 1 } else { -1 }
        };
        let x = raw(&w, e, case % 3 == 0)?;
        let (up, low) = (Mag::from_exact_num(&x), Mag::from_exact_num_lower(&x));
        let c_up = up.cmp_exact_num(&x);
        let c_lo = low.cmp_exact_num(&x);
        if !matches!(c_up, Some(Ordering::Greater | Ordering::Equal))
            || !matches!(c_lo, Some(Ordering::Less | Ordering::Equal))
        {
            return Err(format!(
                "case {}: {:x?}@{}: up {:?} {:?}, lo {:?} {:?}",
                case, w, e, up, c_up, low, c_lo
            ));
        }
        let next = match (low.mant(), low.exp()) {
            (Some(u64::MAX), Some(le)) => mag(TOP, le + 1)?,
            (Some(lm), Some(le)) => mag(lm + 1, le)?,
            _ => return Err(format!("case {}: lo {:?}", case, low)),
        };
        if up != low && up != next {
            return Err(format!(
                "case {}: up {:?} more than one ulp above lo {:?}",
                case, up, low
            ));
        }
    }
    Ok(())
}

// ---- from_exact_num / from_exact_num_lower / to_exact_num sets and P9 (16.S2.4) ----------------------------------

#[test]
fn from_to_exact_num_sets_and_p9() -> Result<(), String> {
    // Carry at all ones: 1 - 2^-100 rounds up to exactly 1 = Mag(TOP, 1), down to Mag(u64::MAX, 0).
    let y = num(1).sub_full_prec(&pow2(-100)?);
    want(
        "up(1-2^-100) == 1",
        Mag::from_exact_num(&y).cmp_exact_num(&num(1)),
        Ordering::Equal,
    )?;
    if Mag::from_exact_num_lower(&y) != mag(u64::MAX, 0)? {
        return Err(format!(
            "lower(1-2^-100) = {:?}",
            Mag::from_exact_num_lower(&y)
        ));
    }
    // A radius beyond zf's ±(2^62−1) exponent range is an error, never a wrap.
    let hi = EXPONENT_MAX;
    for e in [hi.checked_add(1).ok_or("e")?, i64::MAX] {
        if mag(TOP, e)?.to_exact_num().is_ok() {
            return Err(format!("to_exact_num(2^{}) should be Representation", e));
        }
    }
    // P9: from_exact_num(x) >= |x| >= from_exact_num_lower(x), and the round trip contains the original.
    let mut rng = SplitMix64::new(0x5200_0009);
    for case in 0..3000 {
        let e = i64::try_from(rng.below(4001))
            .map_err(|_| "e")?
            .checked_sub(2000)
            .ok_or("e")?;
        let x = rand_num(&mut rng, e, 80)?;
        let up = Mag::from_exact_num(&x);
        let down = Mag::from_exact_num_lower(&x);
        let c_up = up.cmp_exact_num(&x);
        let c_down = down.cmp_exact_num(&x);
        if c_up == Some(Ordering::Less)
            || c_up.is_none()
            || c_down == Some(Ordering::Greater)
            || c_down.is_none()
        {
            return Err(format!(
                "case {}: up {:?} down {:?} for {:?}",
                case, c_up, c_down, x
            ));
        }
        let back = up
            .to_exact_num()
            .map_err(|e| format!("case {}: {:?}", case, e))?;
        if back.abs_cmp(&x).ok_or("abs_cmp")? < 0 {
            return Err(format!("case {}: round trip lost containment", case));
        }
    }
    Ok(())
}
