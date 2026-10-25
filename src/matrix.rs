//! Slice S6: Ball array and matrix operations on `BallArray`, `BallVec` and `BallMat` (the containers stay in `array`): matmul, norms, elementwise ops and N-dimensional enclosures.
//!
//! Owner: slice S6 (Cassie, assigned by Spence 2026-10-25). Only that slice edits this file.
//! CONTRACT 16.S6.0 to 16.S6.11. RED STUB: every computing op returns `Resource` ("not implemented") and the infallible ones
//! return a placeholder, so `tests/harness/slice_matrix.rs` compiles and fails until the green unit lands.

use crate::array::{BallArray, BallMat, BallVec};
use crate::ball::Ball;
use crate::error::{BoundError, BoundResult};
use crate::mag::Mag;
use crate::precision::Precision;

/// The error every unimplemented S6 op returns in the red stub.
const fn todo_err(op: &'static str) -> BoundError {
    BoundError::Resource {
        op,
        detail: "not implemented (S6 red stub)",
    }
}

/// A rank-N array of magnitudes, row-major (CONTRACT 16.S6.0, 16.S6.1).
#[derive(Debug)]
pub struct MagArray {
    shape: Box<[usize]>,
    data: Box<[Mag]>,
}

impl MagArray {
    /// Builds an array of the given shape from `mags` in row-major order.
    ///
    /// # Errors
    /// `InvalidArgument` when the product of `shape` overflows `usize` or differs from `mags.len()`.
    pub fn from_mags(shape: Box<[usize]>, mags: Vec<Mag>) -> BoundResult<MagArray> {
        let _unused = (shape, mags);
        Err(todo_err("MagArray::from_mags"))
    }

    /// The shape.
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// The number of axes.
    pub fn rank(&self) -> usize {
        self.shape.len()
    }

    /// The number of elements.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether there are no elements.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The element at a full row-major index; `None` for a wrong rank or an index out of range.
    pub fn get(&self, index: &[usize]) -> Option<&Mag> {
        let _unused = index;
        None
    }

    /// The elements in row-major order.
    pub fn as_slice(&self) -> &[Mag] {
        &self.data
    }

    /// The shape and the elements.
    pub fn into_parts(self) -> (Box<[usize]>, Box<[Mag]>) {
        (self.shape, self.data)
    }
}

impl BallArray {
    /// Elementwise `add` (CONTRACT 16.S6.4).
    ///
    /// # Errors
    /// `InvalidArgument` for a bad `prec`, `DimensionMismatch` for unequal shapes, `Resource` on allocation failure.
    pub fn add(self, other: &BallArray, prec: Precision) -> BoundResult<BallArray> {
        let _unused = (self, other, prec);
        Err(todo_err("BallArray::add"))
    }

    /// Elementwise `sub` (CONTRACT 16.S6.4).
    ///
    /// # Errors
    /// As [`BallArray::add`].
    pub fn sub(self, other: &BallArray, prec: Precision) -> BoundResult<BallArray> {
        let _unused = (self, other, prec);
        Err(todo_err("BallArray::sub"))
    }

    /// Splits into exact midpoints and radii (CONTRACT 16.S6.5).
    pub fn mid_rad(self) -> (BallArray, MagArray) {
        let empty = MagArray {
            shape: Box::new([0]),
            data: Box::new([]),
        };
        (self, empty)
    }

    /// Rebuilds balls from midpoints and extra radii, enlarging only (CONTRACT 16.S6.5).
    ///
    /// # Errors
    /// `InvalidArgument` for a bad `prec`, `DimensionMismatch` for unequal shapes, `Resource` on allocation failure.
    pub fn from_mid_rad(mids: BallArray, rads: &MagArray, prec: Precision) -> BoundResult<BallArray> {
        let _unused = (mids, rads, prec);
        Err(todo_err("BallArray::from_mid_rad"))
    }
}

impl BallVec {
    /// Dot product (CONTRACT 16.S6.7 to 16.S6.9).
    ///
    /// # Errors
    /// `InvalidArgument` for a bad `prec`, `DimensionMismatch` for unequal lengths.
    pub fn dot(&self, other: &BallVec, prec: Precision) -> BoundResult<Ball> {
        let _unused = (self, other, prec);
        Err(todo_err("BallVec::dot"))
    }
}

impl BallMat {
    /// Matrix-vector product (CONTRACT 16.S6.7 to 16.S6.9).
    ///
    /// # Errors
    /// `InvalidArgument` for a bad `prec`, `DimensionMismatch` when `ncols != x.len()`, `Resource` on allocation failure.
    pub fn matvec(&self, x: &BallVec, prec: Precision) -> BoundResult<BallVec> {
        let _unused = (self, x, prec);
        Err(todo_err("BallMat::matvec"))
    }

    /// Matrix product (CONTRACT 16.S6.7 to 16.S6.9).
    ///
    /// # Errors
    /// `InvalidArgument` for a bad `prec`, `DimensionMismatch` when `self.ncols() != other.nrows()`, `Resource` on allocation failure.
    pub fn matmul(&self, other: &BallMat, prec: Precision) -> BoundResult<BallMat> {
        let _unused = (self, other, prec);
        Err(todo_err("BallMat::matmul"))
    }

    /// Upper bound of the max row sum of `|m| + r` (CONTRACT 16.S6.6).
    pub fn norm_inf(&self) -> Mag {
        let _unused = self;
        Mag::zero()
    }

    /// Upper bound of the max column sum of `|m| + r` (CONTRACT 16.S6.6).
    pub fn norm_1(&self) -> Mag {
        let _unused = self;
        Mag::zero()
    }

    /// Upper bound of the Frobenius norm of every matrix in the ball (CONTRACT 16.S6.6).
    pub fn norm_frob_up(&self) -> Mag {
        let _unused = self;
        Mag::zero()
    }
}
