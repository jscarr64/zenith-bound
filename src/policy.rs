//! Domain policy of restricted-domain operations (CONTRACT 10.1, O6).

/// How a restricted-domain operation treats input outside its domain (CONTRACT 10.2).
#[derive(Debug, PartialEq, Eq, Default)]
pub enum DomainPolicy {
    /// `Domain` if any point of the input lies outside the domain.
    #[default]
    Strict,
    /// Enclose the function over the intersection of the input with the domain.
    Clip,
}
