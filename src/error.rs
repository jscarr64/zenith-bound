//! Error type of zenith-bound (minimal; the variant set is an open item, see CONTRACT O11).

/// Why a value could not be built.
#[derive(Debug)]
pub enum BoundError {
    /// The requested value is not representable as a canonical `Mag`.
    Representation,
}
