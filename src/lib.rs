//! zenith-bound: guaranteed-bounds (ball arithmetic) on zenith-float.
//!
//! Pre-release. Nothing in this crate is guaranteed yet.
#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    clippy::allow_attributes,
    clippy::arithmetic_side_effects
)]

pub mod ball;
pub mod error;
pub mod mag;
pub mod policy;
pub mod tri;
