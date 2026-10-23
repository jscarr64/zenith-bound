//! Error type of zenith-bound (variant set decided: CONTRACT O11-W).

use core::fmt;

/// Why an operation returned no value (CONTRACT 6.4, O11-W).
#[derive(Debug, PartialEq, Eq)]
pub enum BoundError {
    /// The value is not representable (a `Mag::finite` rejection; a NaN or infinite midpoint).
    Representation,
    /// An argument is invalid (`prec == 0`; `RoundingMode::None`).
    InvalidArgument,
    /// The input lies outside the operation's domain (CONTRACT 4.2, 10.2).
    Domain,
    /// The engine gave up (budget, iteration, precision or memory cap).
    Resource,
}

impl fmt::Display for BoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BoundError::Representation => "value not representable",
            BoundError::InvalidArgument => "invalid argument",
            BoundError::Domain => "input outside the domain",
            BoundError::Resource => "resource limit reached",
        })
    }
}

impl core::error::Error for BoundError {}

/// Result of a zenith-bound operation (CONTRACT 15.3).
pub type BoundResult<T> = Result<T, BoundError>;
