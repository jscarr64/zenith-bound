//! zenith-bound: guaranteed-bounds (ball arithmetic) on zenith-float.
//!
//! Pre-release. Nothing in this crate is guaranteed yet.
#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    clippy::allow_attributes,
    clippy::arithmetic_side_effects
)]

pub mod array;
pub mod ball;
pub mod enclosure;
pub mod error;
pub mod mag;
pub mod policy;
pub mod precision;
pub mod tri;

pub use precision::{Precision, MAX_PRECISION, MIN_PRECISION};
