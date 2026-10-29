//! Slice S4b: Carlson symmetric elliptic integrals (RF, RD, RJ, RC), the polylogarithm Li_s and the Lambert W function (principal and -1 branches).
//!
//! Owner: slice S4b (Cassie). Only that slice edits this file.
//! Part A (CONTRACT 16.S4b.1 to 16.S4b.3): `Ball::lambert_w0` and `Ball::lambert_wm1`. Each endpoint value of W is a
//! Newton estimate turned into a certified bracket by a directed-rounding sign test (16.S4b.2); the Carlson integrals
//! and Li_s are later S4b parts. Contract text: `docs/CONTRACT.md` section "Slice S4b"; harness rows:
//! `tests/harness/slice_carlson.rs`; goldens: `tests/goldens/slices/carlson/`; mutants: `tools/mutants/manifest.d/carlson.txt`.

use crate::ball::Ball;
use crate::error::{BoundError, BoundResult};
use crate::policy::DomainPolicy;
use crate::precision::Precision;

impl Ball {
    /// Principal-branch Lambert W, `W0`, on the ball (CONTRACT 16.S4b.1, 16.S4b.2): domain `[-1/e, inf)`.
    ///
    /// # Errors
    /// Red stub: always `Resource` (not implemented).
    pub fn lambert_w0(&self, _prec: Precision, _policy: &DomainPolicy) -> BoundResult<Ball> {
        Err(BoundError::Resource {
            op: "Ball::lambert_w0",
            detail: "red: not implemented",
        })
    }

    /// Branch -1 of Lambert W, `W-1`, on the ball (CONTRACT 16.S4b.1, 16.S4b.2): domain `[-1/e, 0)`.
    ///
    /// # Errors
    /// Red stub: always `Resource` (not implemented).
    pub fn lambert_wm1(&self, _prec: Precision, _policy: &DomainPolicy) -> BoundResult<Ball> {
        Err(BoundError::Resource {
            op: "Ball::lambert_wm1",
            detail: "red: not implemented",
        })
    }
}
