use crate::model::dyadic::{Dy, MagV};
use zenith_bound::mag::Mag;

pub fn to_magv(m: &Mag) -> MagV {
    match m {
        Mag::Finite { exp, mant } => MagV::Finite {
            exp: *exp,
            mant: *mant,
        },
        Mag::Infinity => MagV::Infinity,
    }
}

pub fn from_magv(m: &MagV) -> Mag {
    match m {
        MagV::Finite { exp, mant } => Mag::Finite {
            exp: *exp,
            mant: *mant,
        },
        MagV::Infinity => Mag::Infinity,
    }
}

pub fn dy_of(m: &MagV) -> Result<Dy, String> {
    Dy::from_magv(m).ok_or_else(|| format!("no finite value: {:?}", m))
}

pub fn ctx(op: &str, args: &[&MagV], r: Result<(), String>) -> Result<(), String> {
    r.map_err(|e| format!("{}{:?}: {}", op, args, e))
}
