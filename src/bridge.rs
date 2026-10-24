//! Slice S2: The `Mag` to `ExactNum` bridge: exact comparisons of a `Mag` radius against zenith-float values that never
//! build the radius as an `ExactNum` (CONTRACT 4.2, 10.2c, 15.2, 16.S2), and (pending zenith-float R14) the remaining
//! conversions.
//!
//! Owner: slice S2 (Spence). Only that slice edits this file.

use core::cmp::Ordering;

use zenith_float_num::ExactNum;

use crate::mag::Mag;

impl Mag {
    /// `self` against `|x|`, decided exactly (16.S2.1). RED STUB.
    pub fn cmp_exact_num(&self, x: &ExactNum) -> Option<Ordering> {
        let _ = (self, x);
        None
    }

    /// `self` against `|a - b|`, decided exactly (16.S2.2). RED STUB.
    pub fn cmp_exact_diff(&self, a: &ExactNum, b: &ExactNum) -> Option<Ordering> {
        let _ = (self, a, b);
        None
    }
}
