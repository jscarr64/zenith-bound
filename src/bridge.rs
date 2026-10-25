//! Slice S2: The `Mag` to `ExactNum` bridge: exact and directed conversions between a `Mag` radius and a zenith-float `ExactNum`, with the range rules of CONTRACT 8 and O21 (`Representation` outside the zf exponent range, `Infinity` for an upward bound above it).
//!
//! Owner: slice S2 (owner to be assigned). Only that slice edits this file.
//! The conversions stay in `src/mag/convert.rs` (CONTRACT 16.S2.0); the two exact comparisons of 16.S2.5 and 16.S2.6 live here.

use core::cmp::Ordering;

use zenith_float_num::ExactNum;

use crate::mag::Mag;

impl Mag {
    /// `value(self)` against `|x|`, decided exactly (CONTRACT 16.S2.5). `None` only for a NaN or infinite `x` or an
    /// `Infinity` `self`. A `self` outside zenith-float's range is never built as an `ExactNum`: it is compared with the
    /// round-down and round-up `Mag` bounds of `|x|`.
    pub fn cmp_exact_num(&self, x: &ExactNum) -> Option<Ordering> {
        if x.is_nan() || x.is_inf() || self.is_infinity() {
            return None;
        }
        match (self.is_zero(), x.is_zero()) {
            (true, true) => return Some(Ordering::Equal),
            (true, false) => return Some(Ordering::Less),
            (false, true) => return Some(Ordering::Greater),
            (false, false) => {}
        }
        if let Ok(sn) = self.to_exact_num() {
            return sn.abs_cmp(x).map(|c| c.cmp(&0));
        }
        // `self` is off zenith-float's grid, so `self = |x|` only when `|x|` is a `Mag` (lower == upper bound).
        let lo = Mag::from_exact_num_lower(x);
        let up = Mag::from_exact_num(x);
        Some(match self.cmp(&lo) {
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal if lo < up => Ordering::Less,
            Ordering::Equal => Ordering::Equal,
        })
    }

    /// `a − b` against `r + s` (the sign of `a − b − r − s`), decided exactly (CONTRACT 16.S2.6). `None` for a NaN or
    /// infinite `a` or `b`, an `Infinity` radius, a radius outside zenith-float's range that the `Mag` bounds do not
    /// decide, or an exact partial sum in zenith-float's subnormal range (zf 829f112 flushes it), never a wrong order.
    pub fn cmp_exact_diff(a: &ExactNum, b: &ExactNum, r: &Mag, s: &Mag) -> Option<Ordering> {
        if a.is_nan()
            || a.is_inf()
            || b.is_nan()
            || b.is_inf()
            || r.is_infinity()
            || s.is_infinity()
        {
            return None;
        }
        match (r.to_exact_num(), s.to_exact_num()) {
            (Ok(rn), Ok(sn)) => {
                let terms = vec![
                    Term {
                        val: Val::Ref(a),
                        neg: false,
                    },
                    Term {
                        val: Val::Ref(b),
                        neg: true,
                    },
                    Term {
                        val: Val::Own(rn),
                        neg: true,
                    },
                    Term {
                        val: Val::Own(sn),
                        neg: true,
                    },
                ];
                sign_of_sum(terms)
            }
            (Err(_), _) | (_, Err(_)) => bound_diff(a, b, r, s),
        }
    }
}

/// An `ExactNum` borrowed from the caller or formed here.
enum Val<'a> {
    Ref(&'a ExactNum),
    Own(ExactNum),
}

impl Val<'_> {
    fn get(&self) -> &ExactNum {
        match self {
            Val::Ref(x) => x,
            Val::Own(x) => x,
        }
    }
}

/// The signed summand `(−1)^neg · val`.
struct Term<'a> {
    val: Val<'a>,
    neg: bool,
}

impl Term<'_> {
    fn sign(&self) -> Ordering {
        let x = self.val.get();
        if x.is_zero() {
            Ordering::Equal
        } else if x.is_negative() == self.neg {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }
}

/// The exact sign of `Σ terms`. Each round first tries the `Mag` bounds (the positive side's largest lower bound against
/// the negative side's rounded-up sum, and the reverse), and only when they overlap adds the largest positive and the
/// largest negative term exactly. Overlapping bounds keep those two within a few binades of each other, so no exact sum
/// spans a huge exponent gap.
fn sign_of_sum(terms: Vec<Term<'_>>) -> Option<Ordering> {
    let mut ts: Vec<Term<'_>> = terms
        .into_iter()
        .filter(|t| t.sign() != Ordering::Equal)
        .collect();
    loop {
        let (mut p_lo, mut p_up, mut n_lo, mut n_up) =
            (Mag::zero(), Mag::zero(), Mag::zero(), Mag::zero());
        let (mut ip, mut in_) = (None::<usize>, None::<usize>);
        for (i, t) in ts.iter().enumerate() {
            let x = t.val.get();
            let (lo, up) = (Mag::from_exact_num_lower(x), Mag::from_exact_num(x));
            let positive = t.sign() == Ordering::Greater;
            let (side_lo, side_up, best) = if positive {
                (&mut p_lo, &mut p_up, &mut ip)
            } else {
                (&mut n_lo, &mut n_up, &mut in_)
            };
            if lo > *side_lo {
                *side_lo = lo;
            }
            *side_up = side_up.add_up(&up);
            let larger = match best {
                Some(j) => ts
                    .get(*j)
                    .is_some_and(|b| x.abs_cmp(b.val.get()).is_some_and(|c| c > 0)),
                None => true,
            };
            if larger {
                *best = Some(i);
            }
        }
        let (Some(i), Some(j)) = (ip, in_) else {
            return Some(match (ip, in_) {
                (Some(_), None) => Ordering::Greater,
                (None, Some(_)) => Ordering::Less,
                _ => Ordering::Equal,
            });
        };
        if p_lo > n_up {
            return Some(Ordering::Greater);
        }
        if n_lo > p_up {
            return Some(Ordering::Less);
        }
        let (first, second) = if i > j { (i, j) } else { (j, i) };
        let t1 = ts.swap_remove(first);
        let t2 = ts.swap_remove(second);
        let sum = combine(&t1, &t2)?;
        if sum.sign() != Ordering::Equal {
            ts.push(sum);
        }
    }
}

/// `t1 + t2` exactly, as `(−1)^neg1 · (v1 ± v2)`.
fn combine<'a>(t1: &Term<'_>, t2: &Term<'_>) -> Option<Term<'a>> {
    let (v1, v2) = (t1.val.get(), t2.val.get());
    let val = if t1.neg == t2.neg {
        v1.add_full_prec(v2)
    } else {
        v1.sub_full_prec(v2)
    };
    // zf 829f112 flushes a subnormal sum to zero: give up rather than answer from a wrong sum.
    // sign_of_sum only combines terms of opposite sign, so a true zero needs |v1| = |v2|.
    if val.is_zero() && v1.abs_cmp(v2) != Some(0) {
        return None;
    }
    Some(Term {
        val: Val::Own(val),
        neg: t1.neg,
    })
}

/// The out-of-range radius case of 16.S2.6: decide by `Mag` bounds of `a − b` and `r + s`, or `None`.
fn bound_diff(a: &ExactNum, b: &ExactNum, r: &Mag, s: &Mag) -> Option<Ordering> {
    // r + s > 0 here (a zero radius is in range), so a ≤ b decides Less.
    if a.compare(b).is_some_and(|c| c <= 0) {
        return Some(Ordering::Less);
    }
    // a > b: |a − b| = a − b. Bound it from the parts with the signs of a and b.
    let (a_lo, a_up) = (Mag::from_exact_num_lower(a), Mag::from_exact_num(a));
    let (b_lo, b_up) = (Mag::from_exact_num_lower(b), Mag::from_exact_num(b));
    let a_neg = a.is_negative() && !a.is_zero();
    let b_neg = b.is_negative() && !b.is_zero();
    let (d_lo, d_up) = match (a_neg, b_neg || b.is_zero()) {
        // a ≥ 0 ≥ b: a − b = |a| + |b|
        (false, true) => (a_lo.max(b_lo), a_up.add_up(&b_up)),
        // a > b > 0: a − b = |a| − |b|
        (false, false) => (a_lo.sub_lower(&b_up), a_up),
        // 0 > a > b: a − b = |b| − |a|
        (true, true) => (b_lo.sub_lower(&a_up), b_up),
        // a < 0 < b cannot have a > b
        (true, false) => return Some(Ordering::Less),
    };
    let t_up = r.add_up(s);
    let t_lo = if *r > *s { r } else { s };
    if d_lo > t_up {
        Some(Ordering::Greater)
    } else if d_up < *t_lo {
        Some(Ordering::Less)
    } else {
        None
    }
}
