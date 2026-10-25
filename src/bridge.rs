//! Slice S2: The `Mag` to `ExactNum` bridge: exact and directed conversions between a `Mag` radius and a zenith-float `ExactNum`, with the range rules of CONTRACT 8 and O21 (`Representation` outside the zf exponent range, `Infinity` for an upward bound above it).
//!
//! Owner: slice S2 (owner to be assigned). Only that slice edits this file.
//! The conversions stay in `src/mag/convert.rs` (CONTRACT 16.S2.0); the two exact comparisons of 16.S2.5 and 16.S2.6 live here.
//! RED STUB (commit B): both comparisons return a fixed wrong answer so the slice_bridge rows fail; the implementation follows.

use core::cmp::Ordering;

use zenith_float_num::ExactNum;

use crate::mag::Mag;

impl Mag {
    /// `value(self)` against `|x|`, decided exactly (CONTRACT 16.S2.5). RED STUB: always `Some(Equal)`.
    pub fn cmp_exact_num(&self, x: &ExactNum) -> Option<Ordering> {
        let _ = (self, x);
        Some(Ordering::Equal)
    }

    /// `a − b` against `r + s`, decided exactly (CONTRACT 16.S2.6). RED STUB: always `Some(Equal)`.
    pub fn cmp_exact_diff(a: &ExactNum, b: &ExactNum, r: &Mag, s: &Mag) -> Option<Ordering> {
        let _ = (a, b, r, s);
        Some(Ordering::Equal)
    }
}
