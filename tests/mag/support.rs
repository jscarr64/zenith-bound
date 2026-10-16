//! Test support: conversions between the library `Mag` and the model `MagV`, and error-context helpers. Not a test module.

use crate::model::dyadic::{Dy, MagV};
use core::cmp::Ordering;
use zenith_bound::mag::Mag;

pub fn to_magv(m: &Mag) -> MagV {
    match (m.exp(), m.mant()) {
        (Some(exp), Some(mant)) => MagV::Finite { exp, mant },
        _ => MagV::Infinity,
    }
}

pub fn from_magv(m: &MagV) -> Result<Mag, String> {
    match m {
        MagV::Finite { exp, mant } => Mag::finite(*mant, *exp)
            .map_err(|e| format!("Mag::finite({}, {}) failed: {:?}", mant, exp, e)),
        MagV::Infinity => Ok(Mag::infinity()),
    }
}

pub fn dy_of(m: &MagV) -> Result<Dy, String> {
    Dy::from_magv(m).ok_or_else(|| format!("no finite value: {:?}", m))
}

pub fn ctx(op: &str, args: &[&MagV], r: Result<(), String>) -> Result<(), String> {
    r.map_err(|e| format!("{}{:?}: {}", op, args, e))
}

pub fn dcmp(a: &Dy, b: &Dy) -> Result<Ordering, String> {
    a.cmp(b)
        .ok_or_else(|| String::from("model cannot compare the two values"))
}
