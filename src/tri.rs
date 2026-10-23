//! Three-valued comparison result (CONTRACT 15.2, O15-W, D14).

/// Result of a ball comparison; `True`/`False` only when decided exactly.
#[derive(Debug, PartialEq, Eq)]
pub enum Tri {
    /// Decided true.
    True,
    /// Decided false.
    False,
    /// Not decided.
    Unknown,
}
