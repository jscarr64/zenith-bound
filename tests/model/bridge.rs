//! Slice S2 exact model (CONTRACT 16.S2.8, the M1 extension for the bridge comparisons): pure integer arithmetic, independent of the
//! library and of zenith-float. A value is a sum of signed dyadic terms `±n · 2^low` with `n` an unbounded natural (little-endian
//! `u64` limbs) and `low` an `i128`, so any `Mag` (exponents to `i64::MAX`) and any `ExactNum` are exact here.
//! The sign of a sum is decided by exponent-gap clustering: terms sorted by top bit, a term joins the running cluster while it can
//! reach the cluster's lowest bit, and the first nonzero cluster decides, because everything after it is below its lowest bit.
//! Cross-checked against Python `fractions.Fraction` on tests/goldens/slices/bridge/bridge_rows.txt (scripts/gen_bridge_rows.py).
//! Included by tests/harness/slice_bridge.rs only (not in tests/model/mod.rs), so every item here is used by that slice.

use core::cmp::Ordering;

const LOW64: u128 = 0xFFFF_FFFF_FFFF_FFFF;
const TOP: u64 = 0x8000_0000_0000_0000;

/// A signed dyadic term `±n · 2^low` (`n` little-endian, no high zero limbs; empty is zero).
pub struct Term {
    neg: bool,
    limbs: Vec<u64>,
    low: i128,
}

fn lo64(x: u128) -> u64 {
    u64::try_from(x & LOW64).unwrap_or(0)
}

fn hi64(x: u128) -> u64 {
    u64::try_from(x >> 64).unwrap_or(0)
}

fn trim(mut v: Vec<u64>) -> Vec<u64> {
    while v.last() == Some(&0) {
        v.pop();
    }
    v
}

fn bit_len(v: &[u64]) -> i128 {
    match v.last() {
        None => 0,
        Some(t) => {
            let n = i128::try_from(v.len()).unwrap_or(0);
            64 * n - i128::from(t.leading_zeros())
        }
    }
}

fn cmp_nat(a: &[u64], b: &[u64]) -> Ordering {
    a.len()
        .cmp(&b.len())
        .then_with(|| a.iter().rev().cmp(b.iter().rev()))
}

fn add_nat(a: &[u64], b: &[u64]) -> Vec<u64> {
    let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
    let mut carry = 0u128;
    for i in 0..a.len().max(b.len()) {
        let s = u128::from(*a.get(i).unwrap_or(&0)) + u128::from(*b.get(i).unwrap_or(&0)) + carry;
        out.push(lo64(s));
        carry = s >> 64;
    }
    out.push(lo64(carry));
    trim(out)
}

/// `a − b` for `a ≥ b`.
fn sub_nat(a: &[u64], b: &[u64]) -> Vec<u64> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = 0u128;
    for (i, x) in a.iter().enumerate() {
        let y = u128::from(*b.get(i).unwrap_or(&0)) + borrow;
        let x = u128::from(*x);
        if x >= y {
            out.push(lo64(x - y));
            borrow = 0;
        } else {
            out.push(lo64((x + (1u128 << 64)) - y));
            borrow = 1;
        }
    }
    trim(out)
}

/// `a · 2^k`, `k ≥ 0` (bounded by the caller: only cluster-internal shifts reach here).
fn shl_nat(a: &[u64], k: i128) -> Option<Vec<u64>> {
    if a.iter().all(|w| *w == 0) {
        return Some(Vec::new());
    }
    let k = usize::try_from(k).ok()?;
    let (words, bits) = (k / 64, k % 64);
    let mut out = vec![0u64; words];
    let mut carry = 0u64;
    for x in a {
        let wide = u128::from(*x) << bits;
        out.push(lo64(wide) | carry);
        carry = hi64(wide);
    }
    out.push(carry);
    Some(trim(out))
}

/// `(a >> k, any dropped bit nonzero)`, `k ≥ 0`.
fn shr_nat(a: &[u64], k: i128) -> Option<(Vec<u64>, bool)> {
    let k = usize::try_from(k).ok()?;
    let (words, bits) = (k / 64, k % 64);
    let mut sticky = a.iter().take(words).any(|w| *w != 0);
    let mut rest: Vec<u64> = Vec::with_capacity(a.len());
    for w in a.iter().skip(words) {
        rest.push(*w);
    }
    if bits == 0 {
        return Some((rest, sticky));
    }
    let mut out = Vec::with_capacity(rest.len());
    for (i, w) in rest.iter().enumerate() {
        let cur = u128::from(*w) | (u128::from(*rest.get(i + 1).unwrap_or(&0)) << 64);
        out.push(lo64(cur >> bits));
    }
    if let Some(first) = rest.first() {
        sticky = sticky || (u128::from(*first) & ((1u128 << bits) - 1)) != 0;
    }
    Some((trim(out), sticky))
}

impl Term {
    /// `±Σ_k w_k · 2^(exp − 64k)` for the words `w_1, w_2, ..` most significant first (the golden-row number token).
    pub fn words(neg: bool, words: &[u64], exp: i128) -> Term {
        let n = i128::try_from(words.len()).unwrap_or(0);
        let mut limbs = Vec::with_capacity(words.len());
        for w in words.iter().rev() {
            limbs.push(*w);
        }
        Term {
            neg,
            limbs: trim(limbs),
            low: exp - 64 * n,
        }
    }

    /// `+mant · 2^(exp − 64)`: the value of a finite `Mag` (CONTRACT 1.2).
    pub fn mag(mant: u64, exp: i128) -> Term {
        Term::words(false, &[mant], exp)
    }

    /// The same magnitude with the sign flipped.
    pub fn negated(mut self) -> Term {
        self.neg = !self.neg;
        self
    }

    /// The same magnitude, positive.
    pub fn abs(mut self) -> Term {
        self.neg = false;
        self
    }

    fn top(&self) -> i128 {
        self.low + bit_len(&self.limbs)
    }
}

/// A signed natural at a fixed lowest exponent: the running cluster of `sign_of_sum`.
struct Acc {
    neg: bool,
    mag: Vec<u64>,
    low: i128,
}

impl Acc {
    fn add(self, t: Term) -> Option<Acc> {
        let low = if self.mag.is_empty() {
            t.low
        } else {
            self.low.min(t.low)
        };
        let a = shl_nat(&self.mag, self.low - low)?;
        let b = shl_nat(&t.limbs, t.low - low)?;
        if self.neg == t.neg {
            return Some(Acc {
                neg: self.neg,
                mag: add_nat(&a, &b),
                low,
            });
        }
        Some(match cmp_nat(&a, &b) {
            Ordering::Less => Acc {
                neg: t.neg,
                mag: sub_nat(&b, &a),
                low,
            },
            Ordering::Equal | Ordering::Greater => Acc {
                neg: self.neg,
                mag: sub_nat(&a, &b),
                low,
            },
        })
    }

    fn sign(&self) -> Ordering {
        match (self.mag.is_empty(), self.neg) {
            (true, _) => Ordering::Equal,
            (false, true) => Ordering::Less,
            (false, false) => Ordering::Greater,
        }
    }
}

/// The exact sign of `Σ terms` (`Greater` for positive). At most 4 terms carry weight below a cluster, and each of them lies
/// below `2^(cluster low − 3)`, so their total is below one unit of the cluster's lowest bit. `None` only if a shift overflows.
pub fn sign_of_sum(terms: Vec<Term>) -> Option<Ordering> {
    let mut ts: Vec<Term> = terms.into_iter().filter(|t| !t.limbs.is_empty()).collect();
    if ts.len() > 4 {
        return None;
    }
    ts.sort_by_key(|x| std::cmp::Reverse(x.top()));
    let mut acc: Option<Acc> = None;
    for t in ts {
        acc = match acc {
            None => {
                let low = t.low;
                Some(
                    Acc {
                        neg: false,
                        mag: Vec::new(),
                        low,
                    }
                    .add(t)?,
                )
            }
            Some(a) if t.top() + 3 > a.low || a.mag.is_empty() => Some(a.add(t)?),
            Some(a) => return Some(a.sign()),
        };
    }
    Some(acc.map_or(Ordering::Equal, |a| a.sign()))
}

/// `(mant, sticky, exp)`: the top 64 bits of `|t|` in the `0.mant · 2^exp` convention, `None` for zero.
pub fn top64(t: &Term) -> Option<(u64, bool, i128)> {
    let len = bit_len(&t.limbs);
    if len == 0 {
        return None;
    }
    let (m, sticky) = if len >= 64 {
        shr_nat(&t.limbs, len - 64)?
    } else {
        (shl_nat(&t.limbs, 64 - len)?, false)
    };
    Some((*m.first()?, sticky, t.low + len))
}

/// Least `(mant, exp)` at or above `|t|` (CONTRACT 16.S2.1), `None` for zero.
pub fn round_up(t: &Term) -> Option<(u64, i128)> {
    let (m, sticky, e) = top64(t)?;
    if !sticky {
        return Some((m, e));
    }
    Some(match m.checked_add(1) {
        Some(m1) => (m1, e),
        None => (TOP, e + 1),
    })
}

/// Greatest `(mant, exp)` at or below `|t|` (CONTRACT 16.S2.2), `None` for zero.
pub fn round_down(t: &Term) -> Option<(u64, i128)> {
    top64(t).map(|(m, _, e)| (m, e))
}

/// `value(r)` against `|x|` (16.S2.5) with `r` a `Mag` term.
pub fn cmp_num(r: Term, x: Term) -> Option<Ordering> {
    sign_of_sum(vec![r, x.abs().negated()])
}

/// `a − b` against `r + s` (16.S2.6) with `r`, `s` `Mag` terms.
pub fn cmp_diff(a: Term, b: Term, r: Term, s: Term) -> Option<Ordering> {
    sign_of_sum(vec![a, b.negated(), r.negated(), s.negated()])
}
