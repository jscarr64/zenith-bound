//! Shared test support for zenith-bound (checklist item S1): deterministic generator and fixed seeds. Integer-only, no dependencies.
//! Include it from a test target with `#[path = "../common/mod.rs"] mod common;` (the way tests/mag pulls in tests/model).
pub mod rng;
pub mod seeds;

use core::fmt::Debug;

/// A failing check returns `Err` (never a panic): `label` names the check, `got` and `want` are printed with `{:?}`.
pub fn expect_eq<T: PartialEq + Debug>(label: &str, got: &T, want: &T) -> Result<(), String> {
    if got == want {
        Ok(())
    } else {
        Err(format!("{}: expected {:?}, got {:?}", label, want, got))
    }
}
