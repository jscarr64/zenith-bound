//! Slice LA3: QR factorisation (`BallQr`) of a `BallMat` by Householder reflections, and least-squares solve.
//!
//! Owner: slice LA3 (owner to be assigned). Only that slice edits this file.
//! S0 stub (2026-10-24): no items yet. The slice adds its code here, its contract text in
//! `docs/CONTRACT.md` section "Slice LA3", its harness rows in `tests/harness/slice_qr.rs`, its
//! goldens under `tests/goldens/slices/qr/` and its mutants in `tools/mutants/manifest.d/qr.txt`.

use crate::array::{BallMat, BallVec};
use crate::error::{BoundError, BoundResult};
use crate::precision::Precision;

/// The skeleton refusal of an op whose algorithm has not landed yet.
fn not_implemented(op: &'static str) -> BoundError {
    BoundError::Resource {
        op,
        detail: "not_implemented",
    }
}

/// Thin QR: `Q` is `m×p`, `R` is `p×p` upper with `diag(R) > 0` (CONTRACT 16.LA3.1).
/// Not implemented yet: `BallMat::qr` returns Resource "not_implemented", so no value exists.
#[derive(Debug)]
pub struct BallQr {
    q: BallMat,
    r: BallMat,
}

impl BallMat {
    /// Householder QR (CONTRACT 16.LA3.1).
    /// Not implemented yet: returns Resource "not_implemented".
    ///
    /// # Errors
    /// `Resource` "not_implemented".
    pub fn qr(&self, _prec: Precision) -> BoundResult<BallQr> {
        Err(not_implemented("BallMat::qr"))
    }

    /// `argmin ‖A·x − b‖₂` (CONTRACT 16.LA3.1).
    /// Not implemented yet: returns Resource "not_implemented".
    ///
    /// # Errors
    /// `Resource` "not_implemented".
    pub fn lstsq(&self, _b: &BallVec, _prec: Precision) -> BoundResult<BallVec> {
        Err(not_implemented("BallMat::lstsq"))
    }
}

impl BallQr {
    /// The factor `Q` (CONTRACT 16.LA3.1).
    /// Not implemented yet: no `BallQr` can be built (`qr` returns Resource "not_implemented").
    pub fn q(&self) -> &BallMat {
        &self.q
    }

    /// The factor `R` (CONTRACT 16.LA3.1).
    /// Not implemented yet: no `BallQr` can be built (`qr` returns Resource "not_implemented").
    pub fn r(&self) -> &BallMat {
        &self.r
    }

    /// `(Q, R)` by value (CONTRACT 16.LA3.1).
    /// Not implemented yet: no `BallQr` can be built (`qr` returns Resource "not_implemented").
    pub fn into_parts(self) -> (BallMat, BallMat) {
        (self.q, self.r)
    }

    /// `solve_upper(R, Qᵀb)` (CONTRACT 16.LA3.1).
    /// Not implemented yet: returns Resource "not_implemented".
    ///
    /// # Errors
    /// `Resource` "not_implemented".
    pub fn lstsq(&self, _b: &BallVec, _prec: Precision) -> BoundResult<BallVec> {
        Err(not_implemented("BallQr::lstsq"))
    }
}
