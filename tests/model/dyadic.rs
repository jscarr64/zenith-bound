use super::nat::Nat;
use core::cmp::Ordering;

/// Model of the radius type: Finite{exp, mant} denotes mant * 2^(exp - 64); mant == 0 (with exp == 0) is the canonical zero; Infinity is unbounded.
#[derive(Debug, PartialEq, Eq)]
pub enum MagV {
    Finite { exp: i64, mant: u64 },
    Infinity,
}

/// A nonnegative dyadic rational n * 2^e.
pub struct Dy {
    n: Nat,
    e: i128,
}

pub const MAX_ALIGN: i128 = 4096;

impl Dy {
    pub fn from_mag(exp: i64, mant: u64) -> Dy {
        Dy {
            n: Nat::from_u64(mant),
            e: i128::from(exp) - 64,
        }
    }

    pub fn from_magv(m: &MagV) -> Option<Dy> {
        match m {
            MagV::Finite { exp, mant } => Some(Dy::from_mag(*exp, *mant)),
            MagV::Infinity => None,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.n.is_zero()
    }

    pub fn mul(&self, other: &Dy) -> Dy {
        Dy {
            n: self.n.mul(&other.n),
            e: self.e + other.e,
        }
    }

    pub fn add(&self, other: &Dy) -> Option<Dy> {
        if self.is_zero() {
            return Some(Dy {
                n: other.n.shl(0),
                e: other.e,
            });
        }
        if other.is_zero() {
            return Some(Dy {
                n: self.n.shl(0),
                e: self.e,
            });
        }

        if (self.e - other.e).abs() > MAX_ALIGN {
            return None;
        }

        let lowest = self.e.min(other.e);
        let d1 = self.e - lowest;
        let d2 = other.e - lowest;

        let shift1 = match usize::try_from(d1) {
            Ok(v) => v,
            Err(_) => return None,
        };
        let shift2 = match usize::try_from(d2) {
            Ok(v) => v,
            Err(_) => return None,
        };

        let n1_shifted = self.n.shl(shift1);
        let n2_shifted = other.n.shl(shift2);
        let sum = n1_shifted.add(&n2_shifted);

        Some(Dy { n: sum, e: lowest })
    }

    pub fn cmp(&self, other: &Dy) -> Ordering {
        if self.is_zero() && other.is_zero() {
            return Ordering::Equal;
        }
        if self.is_zero() {
            return Ordering::Less;
        }
        if other.is_zero() {
            return Ordering::Greater;
        }

        let top1 = match i128::try_from(self.n.bit_len()) {
            Ok(v) => v + self.e,
            Err(_) => 0,
        };
        let top2 = match i128::try_from(other.n.bit_len()) {
            Ok(v) => v + other.e,
            Err(_) => 0,
        };

        match top1.cmp(&top2) {
            Ordering::Equal => {
                let d = self.e - other.e;
                if d >= 0 {
                    let shift = match usize::try_from(d) {
                        Ok(v) => v,
                        Err(_) => return Ordering::Equal,
                    };
                    self.n.shl(shift).cmp(&other.n)
                } else {
                    let shift = match usize::try_from(-d) {
                        Ok(v) => v,
                        Err(_) => return Ordering::Equal,
                    };
                    self.n.cmp(&other.n.shl(shift))
                }
            }
            ord => ord,
        }
    }
}
