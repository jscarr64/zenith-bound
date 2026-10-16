use core::cmp::Ordering;

#[derive(Debug, PartialEq, Eq)]
pub struct Nat {
    limbs: Vec<u64>,
}

fn lo(x: u128) -> u64 {
    u64::try_from(x & u128::from(u64::MAX)).unwrap_or_default()
}

fn hi(x: u128) -> u64 {
    u64::try_from(x >> 64).unwrap_or_default()
}

fn add_limb_at(limbs: &mut Vec<u64>, pos: usize, x: u64) {
    let mut pos = pos;
    let mut x = x;
    while x != 0 {
        while limbs.len() <= pos {
            limbs.push(0);
        }
        if let Some(slot) = limbs.get_mut(pos) {
            let s = u128::from(*slot) + u128::from(x);
            *slot = lo(s);
            x = hi(s);
        }
        pos += 1;
    }
}

fn add_u128_at(limbs: &mut Vec<u64>, pos: usize, v: u128) {
    add_limb_at(limbs, pos, lo(v));
    add_limb_at(limbs, pos + 1, hi(v));
}

impl Nat {
    pub fn zero() -> Nat {
        Nat { limbs: vec![] }
    }

    pub fn from_u64(x: u64) -> Nat {
        if x == 0 {
            Nat::zero()
        } else {
            Nat::normalize(vec![x])
        }
    }

    pub fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    pub fn bit_len(&self) -> usize {
        match self.limbs.last() {
            None => 0,
            Some(top) => {
                let lz = usize::try_from(top.leading_zeros()).unwrap_or_default();
                self.limbs.len() * 64 - lz
            }
        }
    }

    pub fn shl(&self, bits: usize) -> Nat {
        if self.is_zero() {
            return Nat::zero();
        }
        let limb_shift = bits / 64;
        let mut limbs: Vec<u64> = vec![0; limb_shift];
        let sh = u32::try_from(bits % 64).unwrap_or_default();
        let factor: u128 = 1u128 << sh;
        let mut carry: u128 = 0;
        for limb in self.limbs.iter() {
            let v = u128::from(*limb) * factor + carry;
            limbs.push(lo(v));
            carry = u128::from(hi(v));
        }
        if carry != 0 {
            limbs.push(lo(carry));
        }
        Nat::normalize(limbs)
    }

    pub fn add(&self, other: &Nat) -> Nat {
        let max_len = self.limbs.len().max(other.limbs.len());
        let mut carry = 0u64;
        let mut result = Vec::with_capacity(max_len);
        for i in 0..max_len {
            let a = self.limbs.get(i).copied().unwrap_or(0);
            let b = other.limbs.get(i).copied().unwrap_or(0);
            let s = u128::from(a) + u128::from(b) + u128::from(carry);
            result.push(lo(s));
            carry = hi(s);
        }
        if carry != 0 {
            result.push(carry);
        }
        Nat::normalize(result)
    }

    pub fn mul(&self, other: &Nat) -> Nat {
        if self.is_zero() || other.is_zero() {
            return Nat::zero();
        }
        let mut limbs: Vec<u64> = Vec::new();
        for (i, a) in self.limbs.iter().enumerate() {
            for (j, b) in other.limbs.iter().enumerate() {
                add_u128_at(&mut limbs, i + j, u128::from(*a) * u128::from(*b));
            }
        }
        Nat::normalize(limbs)
    }

    pub fn cmp(&self, other: &Nat) -> Ordering {
        if self.limbs.len() < other.limbs.len() {
            Ordering::Less
        } else if self.limbs.len() > other.limbs.len() {
            Ordering::Greater
        } else {
            for (a, b) in self.limbs.iter().rev().zip(other.limbs.iter().rev()) {
                match a.cmp(b) {
                    Ordering::Equal => continue,
                    other => return other,
                }
            }
            Ordering::Equal
        }
    }

    fn normalize(limbs: Vec<u64>) -> Nat {
        let mut limbs = limbs;
        while limbs.last() == Some(&0) && !limbs.is_empty() {
            limbs.pop();
        }
        Nat { limbs }
    }
}
