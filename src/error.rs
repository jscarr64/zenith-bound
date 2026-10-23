//! Error type of zenith-bound (variant set: CONTRACT O11-W; struct form and the two
//! linear-algebra variants: 2026-10-23 rulings a and b).

use core::fmt;

/// Why an operation returned no value (CONTRACT 6.4, O11-W).
///
/// Every variant carries `op`, the operation that refused, and `detail`, a short reason.
/// The enum is `#[non_exhaustive]`: new variants may be added in a minor release.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundError {
    /// The value is not representable (a `Mag::finite` rejection; a NaN or infinite midpoint).
    Representation {
        /// The operation that refused.
        op: &'static str,
        /// A short reason.
        detail: &'static str,
    },
    /// An argument is invalid (`prec == 0`; a shape mismatch).
    InvalidArgument {
        /// The operation that refused.
        op: &'static str,
        /// A short reason.
        detail: &'static str,
    },
    /// The input lies outside the operation's domain (CONTRACT 4.2, 10.2).
    Domain {
        /// The operation that refused.
        op: &'static str,
        /// A short reason.
        detail: &'static str,
    },
    /// The engine gave up (budget, iteration, precision or memory cap; "cannot certify at this prec").
    Resource {
        /// The operation that refused.
        op: &'static str,
        /// A short reason.
        detail: &'static str,
    },
    /// The matrix is certified not positive definite (ruling b).
    NotPositiveDefinite {
        /// The operation that refused.
        op: &'static str,
        /// A short reason.
        detail: &'static str,
    },
    /// The matrix is certified singular (ruling b).
    Singular {
        /// The operation that refused.
        op: &'static str,
        /// A short reason.
        detail: &'static str,
    },
}

impl BoundError {
    /// The operation that refused.
    pub fn op(&self) -> &'static str {
        STUB
    }

    /// The short reason given by the operation.
    pub fn detail(&self) -> &'static str {
        STUB
    }

    fn kind_text(&self) -> &'static str {
        match self {
            BoundError::Representation { .. } => "value not representable",
            BoundError::InvalidArgument { .. } => "invalid argument",
            BoundError::Domain { .. } => "input outside the domain",
            BoundError::Resource { .. } => "resource limit reached",
            BoundError::NotPositiveDefinite { .. } => "matrix not positive definite",
            BoundError::Singular { .. } => "matrix singular",
        }
    }
}

const STUB: &str = "";

impl fmt::Display for BoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.kind_text())
    }
}

impl core::error::Error for BoundError {}

/// Result of a zenith-bound operation (CONTRACT 15.3).
pub type BoundResult<T> = Result<T, BoundError>;
